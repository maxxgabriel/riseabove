//! Minds and the DecisionPort (01 §5). Every offer to a player goes through
//! `propose`: AI minds answer now; external minds receive a decision request
//! whose default is what their own AI mind would have chosen (P1).

use pw_core::rng::{hash_key, noise, stream};
use pw_core::{ClubId, Hidden, Money, PlayerId};
use pw_world::decision::{Decision, DecisionKind};
use pw_world::event::{EventKind, Visibility};
use pw_world::world::PendingDeal;
use pw_world::{Contract, Loan, MindKind, TeamKind, World};

use crate::{contracts, market};

#[derive(Clone, Debug)]
pub enum Proposal {
    Transfer { buyer: ClubId, seller: ClubId, fee: Money, contract: Contract },
    Renewal { contract: Contract },
    Loan { loan: Loan },
    FreeAgent { club: ClubId, contract: Contract },
}

/// Share of first-team minutes a player of ability `ca` could expect at `club`.
pub fn expected_share(w: &World, club: ClubId, ca: u8, exclude: PlayerId) -> f32 {
    let Some(team) = w.club_team(club, TeamKind::First) else { return 0.1 };
    let rank = w.teams[team].squad.iter().filter(|&&p| p != exclude && w.players.cold[p].ca > ca).count();
    match rank {
        0..=10 => 0.9 - rank as f32 * 0.03,
        11..=15 => 0.35,
        16..=20 => 0.15,
        _ => 0.05,
    }
}

/// Player-side utility of moving to `club` for `wage` vs staying put (08 §4, 17 §6).
pub fn move_utility(w: &World, p: PlayerId, club: ClubId, wage: Money) -> f32 {
    let h = &w.players.hot[p];
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    let hid = |x: Hidden| person.hidden.f(x);
    let cur = h.club;
    let rep = |cl: ClubId| if cl.is_some() { f32::from(w.clubs[cl].reputation) / 10_000.0 } else { 0.0 };
    let level = rep(club) - rep(cur);
    let pt = expected_share(w, club, c.ca, p) - if cur.is_some() { expected_share(w, cur, c.ca, p) } else { 0.0 };
    let cur_wage = if cur.is_some() { c.contract.current_wage(w.date) } else { 0 };
    let money = (pw_core::math::ln((wage as f32 + 50.0) / (cur_wage as f32 + 50.0)) / 2.0).clamp(-1.0, 1.0);
    let years_here = if cur.is_some() { (c.joined.days_until(w.date) as f32 / 365.0).min(10.0) } else { 0.0 };
    let loyalty = years_here * hid(Hidden::Loyalty) / 20.0 * 0.08;
    let home_nation = person.nation;
    let abroad = |cl: ClubId| cl.is_some() && w.clubs[cl].nation != home_nation;
    let life_cost = |cl: ClubId| if abroad(cl) { (1.0 - hid(Hidden::Adaptability) / 20.0) * 0.3 } else { 0.0 };
    let life = life_cost(cur) - life_cost(club);

    let (wl, wp, wm, wt, wy, wf) = if hid(Hidden::Ambition) >= 15.0 {
        (0.30, 0.20, 0.15, 0.20, 0.05, 0.10)
    } else if hid(Hidden::Loyalty) >= 15.0 {
        (0.15, 0.20, 0.10, 0.10, 0.30, 0.15)
    } else if hid(Hidden::Adaptability) <= 7.0 {
        (0.15, 0.20, 0.15, 0.05, 0.10, 0.35)
    } else if hid(Hidden::Professionalism) <= 8.0 {
        (0.15, 0.10, 0.40, 0.10, 0.05, 0.20)
    } else {
        (0.22, 0.22, 0.2, 0.12, 0.1, 0.14)
    };
    wl * level * 3.0 + wp * pt * 2.0 + wm * money + wt * level * 2.0 - wy * loyalty * 3.0 + wf * life * 2.0
}

/// What this player's own AI mind would do with an offer.
pub fn ai_accepts(w: &World, p: PlayerId, prop: &Proposal) -> bool {
    let jitter = 0.05 * noise(&[w.seed, stream::MIND, u64::from(p.0), w.date.0 as u64]);
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    match prop {
        Proposal::Transfer { buyer, contract, .. } => move_utility(w, p, *buyer, contract.wage) + jitter > 0.04,
        Proposal::FreeAgent { club, contract } => move_utility(w, p, *club, contract.wage) + jitter > -0.35,
        Proposal::Loan { loan } => {
            let gain = expected_share(w, loan.club, c.ca, p) - expected_share(w, loan.parent, c.ca, p);
            gain + jitter > 0.1
        }
        Proposal::Renewal { contract } => {
            let demand = market::wage_demand(w, p, contract.club) as f32;
            let loyal = person.hidden.f(Hidden::Loyalty) / 100.0;
            let age = person.age(w.date);
            contract.wage as f32 >= demand * (0.92 - loyal) + jitter * demand || age >= 32
        }
    }
}

/// Put an offer to a player. Returns `Some(accepted)` when resolved now, `None` when pending.
pub fn propose(w: &mut World, p: PlayerId, prop: Proposal) -> Option<bool> {
    let accept = ai_accepts(w, p, &prop);
    let person_id = w.players.cold[p].person;
    if w.people[person_id].mind != MindKind::External {
        if accept {
            apply(w, p, &prop);
        } else {
            reject(w, p, &prop);
        }
        return Some(accept);
    }
    let today = w.date;
    let kind = match &prop {
        Proposal::Transfer { buyer, contract, .. } => DecisionKind::ContractOffer { club: *buyer, contract: contract.clone(), renewal: false },
        Proposal::Renewal { contract } => DecisionKind::ContractOffer { club: contract.club, contract: contract.clone(), renewal: true },
        Proposal::Loan { loan } => DecisionKind::LoanOffer { loan: loan.clone() },
        Proposal::FreeAgent { club, contract } => DecisionKind::FreeAgentOffer { club: *club, contract: contract.clone() },
    };
    let deadline_days = match &prop {
        Proposal::Renewal { .. } => 14,
        _ => 5,
    };
    let id = w.decisions.push(Decision {
        person: person_id,
        player: p,
        kind,
        created: today,
        deadline: today.add_days(deadline_days),
        default: if accept { 0 } else { 1 },
        answer: None,
        resolved: false,
    });
    match prop {
        Proposal::Transfer { buyer, seller, fee, contract } => {
            w.market.pending.push(PendingDeal { player: p, buyer, seller, fee, contract, loan: None, decision: id });
            w.events.push(today, Visibility::Person(person_id), EventKind::Interest { player: p, club: buyer });
        }
        Proposal::Loan { loan } => {
            let (buyer, seller) = (loan.club, loan.parent);
            w.market.pending.push(PendingDeal { player: p, buyer, seller, fee: loan.fee, contract: w.players.cold[p].contract.clone(), loan: Some(loan), decision: id });
        }
        _ => {}
    }
    None
}

fn apply(w: &mut World, p: PlayerId, prop: &Proposal) {
    match prop {
        Proposal::Transfer { buyer, seller, fee, contract } => market::execute_transfer(w, p, *buyer, *seller, *fee, contract.clone()),
        Proposal::Renewal { contract } => contracts::renew(w, p, contract.clone()),
        Proposal::Loan { loan } => market::execute_loan(w, p, loan.clone()),
        Proposal::FreeAgent { club, contract } => market::execute_transfer(w, p, *club, ClubId::NONE, 0, contract.clone()),
    }
}

fn reject(w: &mut World, p: PlayerId, prop: &Proposal) {
    let until = w.date.add_days(90);
    let club = match prop {
        Proposal::Transfer { buyer, .. } | Proposal::FreeAgent { club: buyer, .. } => *buyer,
        Proposal::Loan { loan } => loan.club,
        Proposal::Renewal { contract } => contract.club,
    };
    w.market.cooldown.insert((club, p), until);
}

/// Apply every answered or expired decision (01 §5: defaults at deadlines).
pub fn resolve_due(w: &mut World) {
    for id in w.decisions.due(w.date) {
        let (p, kind, choice) = {
            let d = &mut w.decisions.all[id];
            d.resolved = true;
            (d.player, d.kind.clone(), d.answer.unwrap_or(d.default))
        };
        let accepted = choice == 0;
        let pending_idx = w.market.pending.iter().position(|d| d.decision == id);
        let deal = pending_idx.map(|i| w.market.pending.remove(i));
        let prop = match (kind, deal) {
            (DecisionKind::ContractOffer { renewal: true, contract, .. }, _) => Proposal::Renewal { contract },
            (DecisionKind::ContractOffer { .. }, Some(d)) => Proposal::Transfer { buyer: d.buyer, seller: d.seller, fee: d.fee, contract: d.contract },
            (DecisionKind::LoanOffer { loan }, _) => Proposal::Loan { loan },
            (DecisionKind::FreeAgentOffer { club, contract }, _) => Proposal::FreeAgent { club, contract },
            _ => continue,
        };
        if accepted && still_valid(w, p, &prop) {
            apply(w, p, &prop);
        } else {
            reject(w, p, &prop);
        }
    }
}

/// The world may have moved on while a decision was pending.
fn still_valid(w: &World, p: PlayerId, prop: &Proposal) -> bool {
    let h = &w.players.hot[p];
    match prop {
        Proposal::Transfer { seller, .. } => h.club == *seller && h.status != pw_world::PlayerStatus::Retired,
        Proposal::Renewal { contract } => h.club == contract.club,
        Proposal::Loan { loan } => h.club == loan.parent && w.players.cold[p].loan.is_none(),
        Proposal::FreeAgent { .. } => h.status == pw_world::PlayerStatus::FreeAgent,
    }
}

/// Stable per-(player, key) coin for AI choices that aren't utility-driven.
pub fn coin(w: &World, p: PlayerId, key: u64) -> f32 {
    (hash_key(&[w.seed, stream::MIND, u64::from(p.0), key]) % 10_000) as f32 / 10_000.0
}
