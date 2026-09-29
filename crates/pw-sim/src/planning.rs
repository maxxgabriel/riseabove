//! Multi-season squad planning (07 §5). Each month a club looks at every
//! position group through its own eyes: how deep it is, how good its likely
//! starters are against what the club wants to be — now and, projected through
//! development and ageing, in one and two seasons — how old they are, whose
//! contracts run out and who is likely to leave when they do, who is injured
//! long-term, which academy players will be ready, whether the homegrown quota
//! holds and whether the wage budget has room. Within a group it finds the
//! position the manager's own system is thinnest at, so a need is for a left
//! back or a holding midfielder, not "a defender". Needs come out of that — a
//! starter, rotation, cover, or a successor for an ageing starter — each with
//! a wage band from the club's structure and a fee band from its budget.
//! Recruitment works only from these needs.

use pw_core::math::{exp, interp};
use pw_core::{ClubId, Money, PlayerId, Pos, PosGroup};
use pw_world::club::Need;
use pw_world::deals::{GroupPlan, NeedRole, PlanNeed, SquadPlan};
use pw_world::governance::TransferStyle;
use pw_world::player::familiarity_factor;
use pw_world::{SquadStatus, TeamKind, World};
use smallvec::SmallVec;

use crate::market::ideal_ca;
use crate::scouting;

/// Group, starters wanted in a typical system, base squad depth, and a representative position.
const GROUPS: [(PosGroup, usize, u8, Pos); 4] = [(PosGroup::Gk, 1, 2, Pos::GK), (PosGroup::Def, 4, 7, Pos::DC), (PosGroup::Mid, 4, 7, Pos::MC), (PosGroup::Att, 2, 4, Pos::ST)];

/// Expected ability `years` from now: growth toward potential while young, decline after thirty.
pub fn project(ca: f32, pa: f32, age: f32, years: f32) -> f32 {
    let k = interp(&[(17.0, 0.55), (21.0, 0.45), (24.0, 0.30), (27.0, 0.10), (29.0, 0.0)], age);
    let grown = ca + (pa - ca).max(0.0) * (1.0 - exp(-k * years));
    let decline = ((age + years - 30.0).max(0.0) - (age - 30.0).max(0.0)) * 1.8;
    (grown - decline).max(1.0)
}

/// Chance a player with an expiring contract stays: what the club thinks of him, and how old he is.
fn renewal_chance(status: SquadStatus, age: f32) -> f32 {
    let base = match status {
        SquadStatus::Star => 0.9,
        SquadStatus::Important => 0.85,
        SquadStatus::Regular => 0.75,
        SquadStatus::Youngster => 0.6,
        SquadStatus::Squad => 0.55,
        SquadStatus::ImpactSub => 0.45,
        SquadStatus::Fringe => 0.25,
        SquadStatus::Backup => 0.15,
        SquadStatus::NotNeeded => 0.02,
    };
    if age >= 33.0 { base * 0.7 } else { base }
}

/// Positions of a group, and how many of each the manager's formation fields.
fn slots_needed(w: &World, club: ClubId, group: PosGroup) -> Vec<(Pos, u8)> {
    let team = w.clubs[club].first_team();
    let f = crate::selection::philosophy_of(w, team).formations[0];
    let Some(formation) = w.data.formations.get(usize::from(f)) else { return Vec::new() };
    let mut out: Vec<(Pos, u8)> = Vec::new();
    for s in formation.slots.iter().filter(|s| s.pos.group() == group) {
        match out.iter_mut().find(|(p, _)| *p == s.pos) {
            Some(e) => e.1 += 1,
            None => out.push((s.pos, 1)),
        }
    }
    out
}

/// The position within a group the manager's system is thinnest at: (position, quality of who fills it, cover beyond those
/// needed). Quality counts a player at a position by how well he knows it; a missing body counts as a poor one.
fn weak_spot(w: &World, club: ClubId, group: PosGroup, members: &[(PlayerId, f32)], ideal: f32, rep_pos: Pos) -> (Pos, f32, i8) {
    let mut worst: Option<(f32, Pos, f32, i8)> = None;
    for (pos, need) in slots_needed(w, club, group) {
        let mut able: Vec<f32> = members.iter().filter(|(p, _)| w.players.hot[*p].injury_days <= 60 && w.players.cold[*p].familiarity[pos.idx()] >= 14).map(|(p, ca)| ca * familiarity_factor(w.players.cold[*p].familiarity[pos.idx()])).collect();
        able.sort_by(|a, b| b.total_cmp(a));
        let need = usize::from(need);
        let missing = need.saturating_sub(able.len());
        let quality = (able.iter().take(need).sum::<f32>() + missing as f32 * ideal * 0.6) / need.max(1) as f32;
        let cover = able.len() as i8 - need as i8;
        // How badly this spot needs help: a quality shortfall, no cover behind the starters, or a body missing outright.
        let pressure = (ideal - quality).max(0.0) + if cover <= 0 { 10.0 } else { 0.0 } + missing as f32 * 8.0;
        if worst.is_none_or(|b| pressure > b.0) {
            worst = Some((pressure, pos, quality, cover));
        }
    }
    worst.map_or((rep_pos, 0.0, 0), |(_, p, q, c)| (p, q, c))
}

pub fn plan(w: &mut World, club: ClubId) {
    let today = w.date;
    let Some(team) = w.club_team(club, TeamKind::First) else { return };
    let rep = w.clubs[club].reputation;
    let ideal = ideal_ca(rep);
    let big = rep >= 5000;
    let style = w.governance.get(&club).map_or(TransferStyle::Balanced, |g| g.policy.transfer_style);
    let max_age_policy = w.governance.get(&club).map_or(0, |g| g.policy.max_signing_age);
    let squad: Vec<PlayerId> = w.teams[team].squad.clone();
    let academy: Vec<PlayerId> = w.clubs[club].teams.iter().filter(|&&t| t != team).flat_map(|&t| w.teams[t].squad.clone()).collect();
    let wages: Vec<Money> = {
        let mut v: Vec<Money> = squad.iter().map(|&p| w.players.cold[p].contract.current_wage(today)).filter(|&x| x > 0).collect();
        v.sort_unstable();
        v
    };
    let median_wage = wages.get(wages.len() / 2).copied().unwrap_or(500);
    let budget = w.clubs[club].finance.transfer_budget;
    let wage_headroom = w.clubs[club].finance.wage_budget - w.clubs[club].finance.wage_bill;
    let prof = pw_world::rules::profile(w, w.clubs[club].nation);
    let homegrown = squad.iter().filter(|&&p| w.age(p) > u32::from(prof.u21_exempt_age) && pw_world::rules::is_homegrown(w, p, club)).count() as u8;
    let homegrown_gap = prof.homegrown_min.saturating_sub(homegrown);

    let mut groups: SmallVec<[GroupPlan; 4]> = SmallVec::new();
    let mut needs: SmallVec<[PlanNeed; 6]> = SmallVec::new();
    let mut sell: SmallVec<[PlayerId; 6]> = SmallVec::new();
    let mut promote: SmallVec<[PlayerId; 4]> = SmallVec::new();
    // Everyone as the club sees him, for the wage and resale passes.
    let mut seen: Vec<(PlayerId, f32)> = Vec::with_capacity(squad.len());

    for (group, starters, depth_base, rep_pos) in GROUPS {
        let target_depth = depth_base + u8::from(big);
        let mut members: Vec<(PlayerId, f32)> = squad.iter().copied().filter(|&p| w.players.cold[p].best_pos.group() == group).map(|p| (p, scouting::view(w, club, p).0)).collect();
        members.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        seen.extend(members.iter().copied());
        let starters_v: Vec<(PlayerId, f32)> = members.iter().take(starters).copied().collect();
        let quality = starters_v.iter().map(|x| x.1).sum::<f32>() / starters_v.len().max(1) as f32;
        let avg_age = members.iter().map(|(p, _)| w.age_years(*p)).sum::<f32>() / members.len().max(1) as f32;
        let expiring = members.iter().filter(|(p, _)| (0..365).contains(&w.players.cold[*p].contract.days_left(today))).count() as u8;
        // Expiring contracts the club will probably lose: the expected number of departures, not a flat guess.
        let departures = members
            .iter()
            .filter(|(p, _)| (0..365).contains(&w.players.cold[*p].contract.days_left(today)))
            .map(|(p, _)| 1.0 - renewal_chance(w.players.cold[*p].status, w.age_years(*p)))
            .sum::<f32>()
            .round() as u8;
        let injured = members.iter().filter(|(p, _)| w.players.hot[*p].injury_days > 60).count() as u8;
        let ageing = starters_v.iter().filter(|(p, _)| w.age_years(*p) >= 31.0).count() as u8;
        // The starters' expected level in one and two seasons.
        let projected = |years: f32| -> f32 {
            let v: Vec<f32> = starters_v.iter().map(|(p, ca)| project(*ca, scouting::view(w, club, *p).2.max(*ca), w.age_years(*p), years)).collect();
            v.iter().sum::<f32>() / v.len().max(1) as f32
        };
        let (quality_next, quality_in_two) = (projected(1.0), projected(2.0));
        // Academy players who look ready within two seasons (as the club sees them).
        let prospects: Vec<PlayerId> = academy
            .iter()
            .copied()
            .filter(|&p| w.players.cold[p].best_pos.group() == group && w.age(p) <= 21)
            .filter(|&p| {
                let (ca, _, pa, _) = scouting::view(w, club, p);
                pa >= quality - 5.0 && ca >= quality - 25.0
            })
            .collect();
        for &p in prospects.iter().take(2) {
            if promote.len() < 4 {
                promote.push(p);
            }
        }
        let available = members.len() as u8 - injured.min(members.len() as u8);
        let (weak_pos, _, weak_cover) = weak_spot(w, club, group, &members, ideal, rep_pos);
        groups.push(GroupPlan {
            group,
            depth: members.len() as u8,
            target_depth,
            quality,
            target_quality: ideal,
            avg_age,
            expiring,
            injured,
            prospects: prospects.len() as u8,
            ageing_starters: ageing,
            quality_next,
            quality_in_two,
            expected_departures: departures,
            weak_pos,
            weak_cover,
        });
        let wage_for = |role: NeedRole| -> Money {
            let k = match role {
                NeedRole::Starter => 1.8,
                NeedRole::Successor => 1.3,
                NeedRole::Rotation => 1.0,
                NeedRole::Backup => 0.6,
            };
            (median_wage as f32 * k) as Money
        };
        let fee_for = |role: NeedRole| -> Money {
            let share = match role {
                NeedRole::Starter => 0.6,
                NeedRole::Successor => 0.35,
                NeedRole::Rotation => 0.25,
                NeedRole::Backup => 0.1,
            };
            (budget as f32 * share) as Money
        };
        let max_age = |role: NeedRole| -> u8 {
            let base = match role {
                NeedRole::Successor => 24,
                NeedRole::Starter => 30,
                NeedRole::Rotation => 31,
                NeedRole::Backup => 34,
            };
            let base = match style {
                TransferStyle::Develop | TransferStyle::Value => base.min(26),
                TransferStyle::WinNow => base.max(32),
                _ => base,
            };
            if max_age_policy > 0 { base.min(max_age_policy) } else { base }
        };
        let pushed = std::cell::Cell::new(false);
        let mut push = |role: NeedRole, min_ability: f32, urgency: u8| {
            pushed.set(true);
            needs.push(PlanNeed {
                group,
                pos: weak_pos,
                role,
                min_ability: min_ability.clamp(1.0, 200.0) as u8,
                max_age: max_age(role),
                homegrown: homegrown_gap > 0,
                wage_band: wage_for(role),
                fee_band: fee_for(role),
                urgency,
            });
        };
        // Short of bodies, counting the departures the club expects and long injuries.
        let effective = available.saturating_sub(departures) + (prospects.len() as u8).min(1);
        if effective < target_depth.saturating_sub(1) || weak_cover < 0 {
            push(if members.len() < starters || weak_cover < 0 { NeedRole::Starter } else { NeedRole::Rotation }, (ideal - 14.0).max(quality - 12.0), 2);
        } else if quality < ideal - 8.0 {
            push(NeedRole::Starter, quality + 4.0, if quality < ideal - 18.0 { 3 } else { 2 });
        } else if effective < target_depth || weak_cover == 0 {
            push(NeedRole::Backup, ideal - 22.0, 1);
        }
        // The future: starters who will have declined below what the club wants in two seasons, with no academy answer.
        if ageing > 0 && prospects.is_empty() {
            push(NeedRole::Successor, quality - 15.0, 1);
        } else if quality_in_two < ideal - 12.0 && quality_in_two < quality - 6.0 && prospects.is_empty() && !pushed.get() {
            push(NeedRole::Successor, quality_in_two + 4.0, 1);
        }
        // Surplus: deepest players beyond target depth, the unwanted.
        for (p, _) in members.iter().skip(usize::from(target_depth) + 1) {
            if sell.len() < 6 {
                sell.push(*p);
            }
        }
        for (p, _) in &members {
            let c = &w.players.cold[*p];
            if c.status == SquadStatus::NotNeeded && !sell.contains(p) && sell.len() < 6 {
                sell.push(*p);
            }
        }
    }
    // Money: over the wage budget, the club cannot add without shedding. The best paid players outside the first fourteen
    // by ability are offered first.
    if wage_headroom < 0 {
        seen.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let core: Vec<PlayerId> = seen.iter().take(14).map(|x| x.0).collect();
        let mut paid: Vec<PlayerId> = squad.iter().copied().filter(|p| !core.contains(p)).collect();
        paid.sort_by(|a, b| w.players.cold[*b].contract.current_wage(today).cmp(&w.players.cold[*a].contract.current_wage(today)).then(a.cmp(b)));
        for p in paid.into_iter().take(3) {
            if sell.len() < 6 && !sell.contains(&p) {
                sell.push(p);
            }
        }
    }
    // Resale: clubs that trade on value sell players at the top of their price while a decline is coming.
    if matches!(style, TransferStyle::Value | TransferStyle::Develop) {
        let mut values: Vec<Money> = squad.iter().map(|&p| w.players.cold[p].value).collect();
        values.sort_unstable();
        let median_value = values.get(values.len() / 2).copied().unwrap_or(0);
        for &p in &squad {
            let c = &w.players.cold[p];
            let keeps = matches!(c.status, SquadStatus::Star | SquadStatus::Important);
            if w.age_years(p) >= 28.0 && !keeps && c.value > median_value * 3 / 2 && sell.len() < 6 && !sell.contains(&p) {
                sell.push(p);
            }
        }
    }
    needs.sort_by_key(|n| std::cmp::Reverse(n.urgency));
    // The simple view other systems read.
    let simple: SmallVec<[Need; 4]> = needs.iter().take(4).map(|n| Need { group: n.group, pos: n.pos, min_ability: n.min_ability, max_age: n.max_age, urgency: n.urgency }).collect();
    w.clubs[club].market.needs = simple;
    for &p in &sell {
        if !w.clubs[club].market.listed.contains(&p) {
            w.clubs[club].market.listed.push(p);
        }
    }
    w.deals.plans.insert(club, SquadPlan { built: today, groups, needs, sell, promote, homegrown_gap, wage_headroom });
}

/// The full need behind a simple need, if the club has a plan.
pub fn need_detail(w: &World, club: ClubId, group: PosGroup) -> Option<PlanNeed> {
    w.deals.plans.get(&club).and_then(|p| p.needs.iter().find(|n| n.group == group).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn young_players_are_expected_to_grow_toward_potential_and_veterans_to_decline() {
        let (ca, pa) = (100.0, 150.0);
        assert!(project(ca, pa, 18.0, 2.0) > project(ca, pa, 18.0, 1.0) && project(ca, pa, 18.0, 1.0) > ca);
        assert!(project(ca, pa, 18.0, 2.0) <= pa);
        assert!((project(ca, ca, 26.0, 2.0) - ca).abs() < 1.0, "a prime player with no room holds his level");
        assert!(project(ca, ca, 32.0, 2.0) < project(ca, ca, 32.0, 1.0) && project(ca, ca, 32.0, 1.0) < ca);
        // Only the years past thirty count.
        assert!((project(ca, ca, 28.0, 3.0) - (ca - 1.8)).abs() < 1e-3);
    }

    #[test]
    fn who_is_expected_to_stay_depends_on_what_the_club_thinks_of_him() {
        assert!(renewal_chance(SquadStatus::Star, 26.0) > renewal_chance(SquadStatus::Squad, 26.0));
        assert!(renewal_chance(SquadStatus::Squad, 26.0) > renewal_chance(SquadStatus::NotNeeded, 26.0));
        assert!(renewal_chance(SquadStatus::Regular, 34.0) < renewal_chance(SquadStatus::Regular, 26.0));
    }
}
