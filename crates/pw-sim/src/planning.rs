//! Multi-season squad planning (07 §5). Each month a club looks at every
//! position group through its own eyes: how deep it is, how good its likely
//! starters are against what the club wants to be, how old they are, whose
//! contracts run out, who is injured long-term, which academy players will
//! be ready, and whether the homegrown quota holds. Needs come out of that —
//! a starter, rotation, cover, or a successor for an ageing starter — each
//! with a wage band from the club's structure and a fee band from its budget.
//! Recruitment works only from these needs.

use pw_core::{ClubId, Money, PlayerId, Pos, PosGroup};
use pw_world::club::Need;
use pw_world::deals::{GroupPlan, NeedRole, PlanNeed, SquadPlan};
use pw_world::governance::TransferStyle;
use pw_world::{SquadStatus, TeamKind, World};
use smallvec::SmallVec;

use crate::market::ideal_ca;
use crate::scouting;

const GROUPS: [(PosGroup, usize, u8, Pos); 4] = [
    (PosGroup::Gk, 1, 2, Pos::GK),
    (PosGroup::Def, 4, 7, Pos::DC),
    (PosGroup::Mid, 4, 7, Pos::MC),
    (PosGroup::Att, 2, 4, Pos::ST),
];

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
    let prof = pw_world::rules::profile(w, w.clubs[club].nation);
    let homegrown = squad.iter().filter(|&&p| w.age(p) > u32::from(prof.u21_exempt_age) && pw_world::rules::is_homegrown(w, p, club)).count() as u8;
    let homegrown_gap = prof.homegrown_min.saturating_sub(homegrown);

    let mut groups: SmallVec<[GroupPlan; 4]> = SmallVec::new();
    let mut needs: SmallVec<[PlanNeed; 6]> = SmallVec::new();
    let mut sell: SmallVec<[PlayerId; 6]> = SmallVec::new();
    let mut promote: SmallVec<[PlayerId; 4]> = SmallVec::new();

    for (group, starters, depth_base, _) in GROUPS {
        let target_depth = depth_base + u8::from(big);
        let mut members: Vec<(PlayerId, f32)> = squad
            .iter()
            .copied()
            .filter(|&p| w.players.cold[p].best_pos.group() == group)
            .map(|p| (p, scouting::view(w, club, p).0))
            .collect();
        members.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let starters_v: Vec<(PlayerId, f32)> = members.iter().take(starters).copied().collect();
        let quality = starters_v.iter().map(|x| x.1).sum::<f32>() / starters_v.len().max(1) as f32;
        let avg_age = members.iter().map(|(p, _)| w.age_years(*p)).sum::<f32>() / members.len().max(1) as f32;
        let expiring = members.iter().filter(|(p, _)| (0..365).contains(&w.players.cold[*p].contract.days_left(today))).count() as u8;
        let injured = members.iter().filter(|(p, _)| w.players.hot[*p].injury_days > 60).count() as u8;
        let ageing = starters_v.iter().filter(|(p, _)| w.age_years(*p) >= 31.0).count() as u8;
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
        let mut push = |role: NeedRole, min_ability: f32, urgency: u8| {
            needs.push(PlanNeed {
                group,
                role,
                min_ability: min_ability.clamp(1.0, 200.0) as u8,
                max_age: max_age(role),
                homegrown: homegrown_gap > 0,
                wage_band: wage_for(role),
                fee_band: fee_for(role),
                urgency,
            });
        };
        // Short of bodies (counting expected departures and long injuries).
        let effective = available.saturating_sub(expiring.min(1)) + (prospects.len() as u8).min(1);
        if effective < target_depth.saturating_sub(1) {
            push(if members.len() < starters { NeedRole::Starter } else { NeedRole::Rotation }, (ideal - 14.0).max(quality - 12.0), 2);
        } else if quality < ideal - 8.0 {
            push(NeedRole::Starter, quality + 4.0, if quality < ideal - 18.0 { 3 } else { 2 });
        } else if effective < target_depth {
            push(NeedRole::Backup, ideal - 22.0, 1);
        }
        if ageing > 0 && prospects.is_empty() {
            push(NeedRole::Successor, quality - 15.0, 1);
        }
        // Surplus: deepest players beyond target depth, ageing high earners, the unwanted.
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
    needs.sort_by_key(|n| std::cmp::Reverse(n.urgency));
    // The simple view other systems read.
    let simple: SmallVec<[Need; 4]> = needs
        .iter()
        .take(4)
        .map(|n| Need {
            group: n.group,
            pos: GROUPS.iter().find(|g| g.0 == n.group).map_or(Pos::MC, |g| g.3),
            min_ability: n.min_ability,
            max_age: n.max_age,
            urgency: n.urgency,
        })
        .collect();
    w.clubs[club].market.needs = simple;
    for &p in &sell {
        if !w.clubs[club].market.listed.contains(&p) {
            w.clubs[club].market.listed.push(p);
        }
    }
    w.deals.plans.insert(club, SquadPlan { built: today, groups, needs, sell, promote, homegrown_gap });
}

/// The full need behind a simple need, if the club has a plan.
pub fn need_detail(w: &World, club: ClubId, group: PosGroup) -> Option<PlanNeed> {
    w.deals.plans.get(&club).and_then(|p| p.needs.iter().find(|n| n.group == group).copied())
}
