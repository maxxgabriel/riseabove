//! Transfers and loans (08): valuation, wage demands, squad planning,
//! perception-driven searches, club-to-club negotiation, execution.

use pw_core::math::{exp, interp};
use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Money, PlayerId, Pos, PosGroup, TeamId};
use pw_world::club::Need;
use pw_world::contract::{ContractKind, Loan};
use pw_world::event::{EventKind, Visibility};
use pw_world::rules::{can_sign, max_contract_years};
use pw_world::{Contract, MemoryKind, PlayerStatus, SquadStatus, TeamKind, World};
use rayon::prelude::*;
use smallvec::SmallVec;

use crate::consider;
use crate::decisions::{self, Proposal};
use crate::finance;
use crate::perception::club_view;

// ---------------------------------------------------------------- valuation

pub fn value_of(w: &World, p: PlayerId) -> Money {
    let c = &w.players.cold[p];
    let h = &w.players.hot[p];
    let t = &w.data.tuning.market;
    let age = w.age_years(p);
    let ca = f32::from(c.ca);
    let youth = interp(&[(21.0, 1.0), (27.0, 0.0)], age);
    let potential = 1.0 + (f32::from(c.pa) - ca).max(0.0) / 100.0 * youth * 1.5;
    let age_mult = interp(&[(16.0, 0.55), (19.0, 1.0), (24.0, 1.1), (28.0, 1.0), (31.0, 0.7), (33.0, 0.45), (36.0, 0.2)], age);
    let years = (h.club.is_some()).then(|| c.contract.days_left(w.date) as f32 / 365.0).unwrap_or(0.0);
    let contract = if h.status == PlayerStatus::FreeAgent { 0.3 } else { 0.35 + 0.65 * (years / 3.0).min(1.0) };
    let rep = 0.85 + 0.3 * f32::from(c.rep.world) / 10_000.0;
    // Current internationals carry a premium buyers pay for.
    let intl = 1.0 + 0.12 * crate::intl::standing(w, p);
    let inj = if h.injury_days > 60 { 0.8 } else { 1.0 };
    let v = t.value_base * exp(t.value_exp * (ca - 100.0)) * potential * age_mult * contract * rep * intl * inj * w.economy.global();
    (v.max(5_000.0) as Money / 5_000) * 5_000
}

/// Weekly wage a player expects at `club`.
pub fn wage_demand(w: &World, p: PlayerId, club: ClubId) -> Money {
    let c = &w.players.cold[p];
    let ca = f32::from(c.ca);
    let (econ, rep) = if club.is_some() {
        let cl = &w.clubs[club];
        (w.nations[cl.nation].economy, f32::from(cl.reputation) / 10_000.0)
    } else {
        (0.5, 0.3)
    };
    let index = if club.is_some() { w.economy.wage_index(w.clubs[club].nation) } else { w.economy.global() };
    let base = 400.0 * exp(0.048 * (ca - 60.0)) * index;
    let club_scale = 0.3 + 1.3 * rep;
    let fame = 1.0 + 0.5 * f32::from(c.rep.world) / 10_000.0;
    ((base * club_scale * fame * econ.max(0.2)).max(150.0) as Money / 50) * 50
}

pub fn contract_years(age: u32) -> u8 {
    let y = match age {
        0..=23 => 5,
        24..=27 => 4,
        28..=30 => 3,
        31..=32 => 2,
        _ => 1,
    };
    y.min(max_contract_years(age))
}

pub fn new_contract(w: &World, p: PlayerId, club: ClubId, premium: f32) -> Contract {
    let age = w.age(p);
    let years = i32::from(contract_years(age));
    let wage = (wage_demand(w, p, club) as f32 * premium) as Money;
    let c = &w.players.cold[p];
    Contract {
        club,
        kind: if age < 17 { ContractKind::Youth } else { ContractKind::Professional },
        wage,
        start: w.date,
        end: w.date.add_months(12 * years),
        release_clause: 0,
        promised_status: None,
        yearly_rise: 3,
        relegation_cut: 20,
        appearance_bonus: wage / 10,
        goal_bonus: if c.best_pos.group() == PosGroup::Att { wage / 5 } else { wage / 10 },
    }
}

// ------------------------------------------------------------- monthly

pub fn monthly(w: &mut World) {
    let values: Vec<Money> = (0..w.players.len())
        .into_par_iter()
        .map(|i| {
            let p = PlayerId(i as u32);
            if w.players.hot[p].status == PlayerStatus::Retired { 0 } else { value_of(w, p) }
        })
        .collect();
    for (c, v) in w.players.cold.iter_mut().zip(values) {
        c.value = v;
    }
    for club in w.clubs.ids() {
        assign_statuses(w, club);
        plan_squad(w, club);
    }
}

/// Squad status from the manager's own view of the pecking order (03 §9):
/// perceived ability, adjusted by how much he trusts and rates each player.
fn assign_statuses(w: &mut World, club: ClubId) {
    let Some(team) = w.club_team(club, TeamKind::First) else { return };
    let today = w.date;
    let manager = w.clubs[club].manager.get().map(|m| w.staff[m].person);
    let mut ranked: Vec<(PlayerId, f32)> = w.teams[team]
        .squad
        .iter()
        .map(|&p| {
            let (ca, _, _, _) = club_view(w, club, p);
            let who = w.players.cold[p].person;
            let taste = w.clubs[club].manager.get().map_or(0.0, |s| crate::managers::preference(w, s, p) * 6.0);
            let opinion = taste
                + manager.map_or(0.0, |m| {
                    (consider::trust(w, m, who) - 0.5) * 12.0 + w.social.get(m, who).map_or(0.0, |r| (f32::from(r.respect) - 50.0) * 0.15) - consider::memory(w, m, who, MemoryKind::PoorAttitude) * 4.0
                });
            (p, ca + opinion)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let n = ranked.len();
    for (rank, (p, _)) in ranked.into_iter().enumerate() {
        let age = w.age(p);
        let c = &mut w.players.cold[p];
        if let Some(promised) = c.contract.promised_status {
            c.status = promised;
            continue;
        }
        let before = c.status;
        c.status = match rank {
            0..=1 => SquadStatus::Star,
            2..=5 => SquadStatus::Important,
            6..=10 => SquadStatus::Regular,
            _ if age <= 20 => SquadStatus::Youngster,
            11..=15 => SquadStatus::Squad,
            16..=18 => SquadStatus::ImpactSub,
            _ if rank + 2 >= n => SquadStatus::NotNeeded,
            _ => SquadStatus::Fringe,
        };
        let after = c.status;
        let big = (before as i32 - after as i32).abs() >= 2 || matches!(after, SquadStatus::Star | SquadStatus::NotNeeded) != matches!(before, SquadStatus::Star | SquadStatus::NotNeeded);
        if before != after && big && before != SquadStatus::Youngster {
            let mut causes = pw_world::Causes::new();
            if let Some(m) = manager {
                causes.push(pw_world::Cause::Fact(pw_world::Fact::LowTrust { from: m, about: w.players.cold[p].person, trust: (consider::trust(w, m, w.players.cold[p].person) * 100.0) as u8 }));
            }
            w.events.push_caused(today, Visibility::Club(club), EventKind::StatusChanged { player: p, club, from: before, to: after }, causes);
        }
    }
}

/// Re-rank a club's squad through its (possibly new) manager's eyes now.
pub fn reassess(w: &mut World, club: ClubId) {
    assign_statuses(w, club);
}

/// Target ability for a club's starters, from its reputation.
pub fn ideal_ca(rep: u16) -> f32 {
    60.0 + 110.0 * (f32::from(rep) / 10_000.0).powf(0.8)
}

fn plan_squad(w: &mut World, club: ClubId) {
    let Some(team) = w.club_team(club, TeamKind::First) else { return };
    let rep = w.clubs[club].reputation;
    let ideal = ideal_ca(rep);
    let big = rep >= 5000;
    let mut needs: SmallVec<[Need; 4]> = SmallVec::new();
    for (group, target, starters, rep_pos) in [
        (PosGroup::Gk, if big { 3 } else { 2 }, 1usize, Pos::GK),
        (PosGroup::Def, if big { 8 } else { 7 }, 4, Pos::DC),
        (PosGroup::Mid, if big { 8 } else { 7 }, 4, Pos::MC),
        (PosGroup::Att, if big { 5 } else { 4 }, 2, Pos::ST),
    ] {
        let mut cas: Vec<(u8, Pos)> =
            w.teams[team].squad.iter().map(|&p| &w.players.cold[p]).filter(|c| c.best_pos.group() == group && c.status != SquadStatus::NotNeeded).map(|c| (c.ca, c.best_pos)).collect();
        cas.sort_by(|a, b| b.0.cmp(&a.0));
        let weakest = cas.get(starters.saturating_sub(1)).map(|x| x.0).unwrap_or(0);
        let pos = cas.get(starters.saturating_sub(1)).map_or(rep_pos, |x| x.1);
        if cas.len() < target {
            needs.push(Need { group, pos, min_ability: ((ideal - 12.0).max(f32::from(weakest) - 8.0)) as u8, max_age: 29, urgency: 2 });
        } else if f32::from(weakest) < ideal - 8.0 {
            needs.push(Need { group, pos, min_ability: weakest.saturating_add(4), max_age: 30, urgency: 1 });
        }
    }
    needs.sort_by_key(|n| std::cmp::Reverse(n.urgency));
    w.clubs[club].market.needs = needs;
    // Surplus goes on the list, along with anyone the club has made available
    // or who has asked to leave; agents make sure other clubs hear about it.
    let listed: Vec<PlayerId> = w.clubs[club]
        .teams
        .iter()
        .flat_map(|&t| w.teams[t].squad.iter().copied())
        .filter(|&p| w.players.cold[p].status == SquadStatus::NotNeeded || w.market.listed.contains_key(&p) || w.market.requests.contains_key(&p))
        .collect();
    w.clubs[club].market.listed = listed;
    // The full multi-season plan refines needs and the sell list.
    crate::planning::plan(w, club);
}

// --------------------------------------------------------------- searches

pub fn daily(w: &mut World) {
    let today = w.date;
    let interval = u32::from(w.data.tuning.market.search_interval_days.max(1));
    let max = w.data.tuning.market.max_transfers_per_club_window;
    let day = today.0 as u32;
    for club in w.clubs.ids() {
        if (club.0 + day) % interval != 0 {
            continue;
        }
        let nation = w.clubs[club].nation;
        if !w.nations[nation].season.window_open(today) {
            continue;
        }
        if w.clubs[club].market.signed_this_window >= max || w.clubs[club].market.needs.is_empty() {
            continue;
        }
        // Shortlist first; a broad search only when the shortlist is exhausted.
        if !crate::deals::pursue(w, club) {
            search(w, club);
        }
    }
}

struct Target {
    player: PlayerId,
    score: f32,
    fee: Money,
}

fn search(w: &mut World, club: ClubId) {
    let today = w.date;
    let needs = w.clubs[club].market.needs.clone();
    let budget = w.clubs[club].finance.transfer_budget;
    let wage_room = (w.clubs[club].finance.wage_budget - w.clubs[club].finance.wage_bill).max(w.clubs[club].finance.wage_budget / 20);
    for need in needs {
        let mut best: Option<Target> = None;
        for (p, seen) in w.knowledge.known(club).collect::<Vec<_>>() {
            if seen.minutes < 90 {
                continue;
            }
            let h = &w.players.hot[p];
            let c = &w.players.cold[p];
            if h.club == club || !matches!(h.status, PlayerStatus::Active | PlayerStatus::FreeAgent) || c.loan.is_some() {
                continue;
            }
            if c.best_pos.group() != need.group && c.familiarity[need.pos.idx()] < 15 {
                continue;
            }
            if w.age(p) > u32::from(need.max_age) || w.market.on_cooldown(club, p, today) || w.market.is_pending(p) || crate::negotiation::in_talks(w, p) {
                continue;
            }
            let (ca, _, pa, _) = crate::scouting::view(w, club, p);
            if ca < f32::from(need.min_ability) {
                continue;
            }
            let fee = if h.status == PlayerStatus::FreeAgent { 0 } else { asking_price(w, p) };
            if fee > budget || wage_demand(w, p, club) > wage_room {
                continue;
            }
            let youth = if w.age(p) <= 23 { (pa - ca).max(0.0) * 0.3 } else { 0.0 };
            let cost = (fee as f32 / 1e6).sqrt() * 0.8;
            // Players known to be available are easier to get.
            let available = if w.market.requests.contains_key(&p) || w.market.listed.contains_key(&p) { 3.0 } else { 0.0 };
            // Managers push for their kind of player and for favourites from past jobs.
            let wanted = crate::managers::wants(w, club, p) * 6.0;
            let score = ca + youth - cost + available + wanted;
            if best.as_ref().is_none_or(|b| score > b.score) {
                best = Some(Target { player: p, score, fee });
            }
        }
        let Some(t) = best else { continue };
        if !can_sign(w, club, t.player, today).allowed() {
            continue;
        }
        approach(w, club, t.player, t.fee);
        break;
    }
}

/// What the selling club would accept (seller stance × contract situation).
pub fn asking_price(w: &World, p: PlayerId) -> Money {
    let c = &w.players.cold[p];
    let stance = match c.status {
        SquadStatus::Star => 2.0,
        SquadStatus::Important => 1.5,
        SquadStatus::Regular => 1.2,
        SquadStatus::Squad | SquadStatus::ImpactSub | SquadStatus::Youngster => 1.0,
        _ => 0.8,
    };
    let years = c.contract.days_left(w.date) as f32 / 365.0;
    let expiring = if years < 1.0 { 0.7 } else { 1.0 };
    let unsettled = if w.market.requests.contains_key(&p) || w.market.listed.contains_key(&p) { 0.75 } else { 1.0 };
    let club = w.players.hot[p].club;
    let board = if club.is_some() { crate::governance::selling_stance(w, club) } else { 1.0 };
    let v = (c.value as f32 * stance * expiring * unsettled * board) as Money;
    if c.contract.release_clause > 0 { v.min(c.contract.release_clause) } else { v }
}

/// Club-to-club talks, then personal terms through the player's mind.
fn approach(w: &mut World, buyer: ClubId, p: PlayerId, asking: Money) {
    let today = w.date;
    let seller = w.players.hot[p].club;
    // Players under contract are pursued through club-to-club deals.
    if seller.is_some() {
        let group = w.players.cold[p].best_pos.group();
        crate::deals::enquire(w, buyer, p, Some(group));
        return;
    }
    let budget = w.clubs[buyer].finance.transfer_budget;
    let value = w.players.cold[p].value;
    let mut rng = Rng::keyed(&[w.seed, stream::MARKET, u64::from(buyer.0), u64::from(p.0), today.0 as u64]);
    let willing = (value as f32 * rng.range_f32(1.05, 1.4)) as Money;
    let fee = if seller.is_none() {
        0
    } else if willing.min(budget) >= asking {
        asking
    } else {
        w.market.cooldown.insert((buyer, p), today.add_days(i32::from(w.data.tuning.market.rebid_cooldown_days) * 3));
        w.events.push(today, Visibility::Club(seller), EventKind::BidRejected { player: p, club: buyer, fee: willing.min(budget) });
        return;
    };
    if seller.is_some() {
        w.events.push(today, Visibility::Club(seller), EventKind::BidAccepted { player: p, club: buyer, fee });
    }
    let prop = if seller.is_none() { Proposal::FreeAgent { club: buyer } } else { Proposal::Transfer { buyer, seller, fee } };
    decisions::propose(w, p, prop);
}

// --------------------------------------------------------------- moves

fn remove_from_team(w: &mut World, p: PlayerId) {
    let t = w.players.hot[p].team;
    if t.is_some() {
        w.teams[t].squad.retain(|&x| x != p);
    }
}

/// Where a newly arrived player trains: first team unless young and not yet good enough.
fn landing_team(w: &World, p: PlayerId, club: ClubId) -> TeamId {
    let first = w.clubs[club].first_team();
    let age = w.age(p);
    if age >= 21 {
        return first;
    }
    let mut cas: Vec<u8> = w.teams[first].squad.iter().map(|&x| w.players.cold[x].ca).collect();
    cas.sort_by(|a, b| b.cmp(a));
    let bar = cas.get(17).copied().unwrap_or(0);
    if w.players.cold[p].ca >= bar {
        return first;
    }
    let youth = [TeamKind::U21, TeamKind::Reserve, TeamKind::U19, TeamKind::U18];
    let pick = youth.iter().filter(|k| k.max_age().is_none_or(|m| age <= m)).find_map(|&k| w.club_team(club, k));
    pick.unwrap_or(first)
}

pub fn execute_transfer(w: &mut World, p: PlayerId, buyer: ClubId, seller: ClubId, fee: Money, contract: Contract) {
    let today = w.date;
    remove_from_team(w, p);
    // Leaving the amateur game for a professional club.
    w.youth.leave(p);
    let team = landing_team(w, p, buyer);
    w.teams[team].squad.push(p);
    {
        let h = &mut w.players.hot[p];
        h.club = buyer;
        h.team = team;
        h.status = PlayerStatus::Active;
        h.morale = (h.morale + 10).min(100);
    }
    {
        let c = &mut w.players.cold[p];
        c.contract = contract.clone();
        c.loan = None;
        c.joined = today;
        c.status = SquadStatus::Squad;
    }
    finance::pay_fee(w, buyer, seller, fee);
    if seller.is_some() {
        crate::deals::on_transfer_fee(w, p, seller, fee);
    }
    crate::honours::on_transfer(w, p, buyer, seller, fee);
    w.clubs[buyer].market.signed_this_window += 1;
    w.clubs[buyer].market.needs.retain(|n| n.group != w.players.cold[p].best_pos.group());
    if seller.is_some() {
        w.clubs[seller].market.listed.retain(|&x| x != p);
        w.events.push(today, Visibility::Public, EventKind::Transfer { player: p, from: seller, to: buyer, fee });
    }
    let ev = w.events.push(today, Visibility::Public, EventKind::ContractSigned { player: p, club: buyer, wage: contract.wage, until: contract.end, renewal: false });
    w.history.start_spell(p, buyer, today, false, fee);
    w.knowledge.observe(buyer, p, 300, today);
    w.market.requests.remove(&p);
    w.market.listed.remove(&p);
    w.market.loan_listed.remove(&p);
    let who = w.players.cold[p].person;
    let nation = w.clubs[buyer].nation;
    crate::life::relocate(w, who, nation, pw_world::Cause::Event(ev));
    if seller.is_some() {
        crate::culture::on_transfer(w, p, seller, buyer, ev);
    }
    crate::media::on_move(w, p, seller, buyer, ev);
}

pub fn execute_loan(w: &mut World, p: PlayerId, loan: Loan) {
    let today = w.date;
    remove_from_team(w, p);
    let team = w.clubs[loan.club].first_team();
    w.teams[team].squad.push(p);
    w.players.hot[p].team = team;
    finance::pay_fee(w, loan.club, loan.parent, loan.fee);
    let dest_nation = w.clubs[loan.club].nation;
    let ev = w.events.push(today, Visibility::Public, EventKind::LoanMove { player: p, from: loan.parent, to: loan.club, until: loan.end });
    w.history.start_spell(p, loan.club, today, true, loan.fee);
    w.knowledge.observe(loan.club, p, 300, today);
    w.market.loan_listed.remove(&p);
    w.players.cold[p].loan = Some(loan);
    let who = w.players.cold[p].person;
    crate::life::relocate(w, who, dest_nation, pw_world::Cause::Event(ev));
}

/// Loanee goes back to the parent club.
pub fn end_loan(w: &mut World, p: PlayerId) {
    // Obligations and options can make the move permanent instead.
    if crate::deals::loan_ends(w, p) {
        return;
    }
    let Some(loan) = w.players.cold[p].loan.take() else { return };
    let today = w.date;
    remove_from_team(w, p);
    let team = landing_team(w, p, loan.parent);
    w.teams[team].squad.push(p);
    w.players.hot[p].team = team;
    let ev = w.events.push(today, Visibility::Public, EventKind::LoanReturn { player: p, to: loan.parent });
    w.history.start_spell(p, loan.parent, today, false, 0);
    let who = w.players.cold[p].person;
    let nation = w.clubs[loan.parent].nation;
    crate::life::relocate(w, who, nation, pw_world::Cause::Event(ev));
}

/// Development loans for surplus youngsters (07 §7), weekly during windows.
pub fn weekly_loans(w: &mut World) {
    let today = w.date;
    for parent in w.clubs.ids() {
        let nation = w.clubs[parent].nation;
        if !w.nations[nation].season.window_open(today) {
            continue;
        }
        let Some(first) = w.club_team(parent, TeamKind::First) else { continue };
        let parent_rep = w.clubs[parent].reputation;
        let candidate = w.clubs[parent]
            .teams
            .iter()
            .flat_map(|&t| w.teams[t].squad.iter().copied())
            .filter(|&p| {
                let c = &w.players.cold[p];
                let h = &w.players.hot[p];
                let age = w.age(p);
                let agreed = w.market.loan_listed.contains_key(&p);
                (agreed || ((18..=21).contains(&age) && h.minutes_4w < 120 && f32::from(c.pa) >= ideal_ca(parent_rep) - 10.0))
                    && age >= 17
                    && c.loan.is_none()
                    && h.available()
                    && !w.market.is_pending(p)
                    && !crate::negotiation::in_talks(w, p)
                    && !w.market.on_cooldown(ClubId::NONE, p, today)
            })
            .max_by_key(|&p| (w.players.cold[p].pa, std::cmp::Reverse(p)));
        let Some(p) = candidate else { continue };
        let ca = w.players.cold[p].ca;
        let group = w.players.cold[p].best_pos.group();
        let dest = w
            .clubs
            .iter_enumerated()
            .filter(|(id, c)| *id != parent && c.nation == nation && c.reputation + 500 < parent_rep && c.reputation * 3 > parent_rep)
            .filter(|(_, c)| c.market.needs.iter().any(|n| n.group == group && n.min_ability <= ca + 5))
            .filter(|(id, _)| pw_world::rules::can_loan(w, *id, parent, p, today).allowed())
            .max_by_key(|(id, c)| (c.reputation, std::cmp::Reverse(*id)))
            .map(|(id, _)| id);
        let Some(dest) = dest else { continue };
        let (loan, terms) = crate::deals::loan_terms(w, parent, dest, p);
        w.deals.loans.insert(p, terms);
        let _ = first;
        if decisions::propose(w, p, Proposal::Loan { loan }) == Some(false) {
            w.market.cooldown.insert((ClubId::NONE, p), today.add_days(60));
        }
    }
}

/// Unattached players: clubs with needs look at free agents they know of.
pub fn free_agent_sweep(w: &mut World) {
    let today = w.date;
    let free: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status == PlayerStatus::FreeAgent).collect();
    if free.is_empty() {
        return;
    }
    for club in w.clubs.ids() {
        if w.clubs[club].market.needs.is_empty() || (club.0 + today.0 as u32) % 7 != 0 {
            continue;
        }
        let need = w.clubs[club].market.needs[0];
        let nation = w.clubs[club].nation;
        let pick = free
            .iter()
            .copied()
            .filter(|&p| {
                let c = &w.players.cold[p];
                let person = &w.people[c.person];
                (person.nation == nation || w.knowledge.seen(club, p).is_some())
                    && (c.best_pos.group() == need.group)
                    && c.ca >= need.min_ability
                    && w.age(p) <= u32::from(need.max_age) + 3
                    && !w.market.on_cooldown(club, p, today)
                    && !w.market.is_pending(p)
                    && !crate::negotiation::in_talks(w, p)
            })
            .max_by_key(|&p| (w.players.cold[p].ca, std::cmp::Reverse(p)));
        if let Some(p) = pick {
            approach(w, club, p, 0);
        }
    }
}
