//! Contracts (08 §5): expiry, loan ends, renewals, release.

use pw_core::PlayerId;
use pw_world::event::{EventKind, Visibility};
use pw_world::{Contract, PlayerStatus, SquadStatus, World};

use crate::decisions::{self, Proposal};
use crate::market;

pub fn daily(w: &mut World) {
    let today = w.date;
    let mut expired = Vec::new();
    let mut loans_over = Vec::new();
    for p in w.players.ids() {
        let h = &w.players.hot[p];
        if h.status != PlayerStatus::Active {
            continue;
        }
        let c = &w.players.cold[p];
        if c.contract.club.is_some() && today > c.contract.end {
            expired.push(p);
        } else if c.loan.as_ref().is_some_and(|l| today > l.end) {
            loans_over.push(p);
        }
    }
    for p in loans_over {
        market::end_loan(w, p);
    }
    for p in expired {
        if w.players.cold[p].loan.is_some() {
            market::end_loan(w, p);
            // The loan club may have made it permanent on a new contract.
            if w.players.cold[p].contract.end >= today {
                continue;
            }
        }
        // A pre-contract signed months ago takes effect now.
        if crate::deals::honour_pre_contract(w, p) {
            continue;
        }
        crate::boardroom::on_expiry(w, p);
        release(w, p);
    }
}

/// Contract over (expiry, mutual termination, liquidation): player becomes a free agent.
pub fn release(w: &mut World, p: PlayerId) {
    let today = w.date;
    let club = w.players.hot[p].club;
    let team = w.players.hot[p].team;
    if team.is_some() {
        w.teams[team].squad.retain(|&x| x != p);
    }
    let h = &mut w.players.hot[p];
    h.club = pw_core::ClubId::NONE;
    h.team = pw_core::TeamId::NONE;
    h.status = PlayerStatus::FreeAgent;
    let c = &mut w.players.cold[p];
    c.contract = Contract::default();
    c.loan = None;
    c.status = SquadStatus::Squad;
    w.history.end_spell(p, today);
    if club.is_some() {
        w.events.push(today, Visibility::Public, EventKind::Released { player: p, club });
    }
}

pub fn renew(w: &mut World, p: PlayerId, contract: Contract) {
    let today = w.date;
    let club = contract.club;
    w.events.push(today, Visibility::Public, EventKind::ContractSigned { player: p, club, wage: contract.wage, until: contract.end, renewal: true });
    w.players.cold[p].contract = contract;
    let h = &mut w.players.hot[p];
    h.morale = (h.morale + 5).min(100);
}

/// Eleven and seven substitutes.
const MATCHDAY_SQUAD: usize = 18;

/// Clubs approach players entering the final stretch of their deals (07 §8).
pub fn weekly(w: &mut World) {
    let today = w.date;
    let t = w.data.tuning.market.clone();
    let mut offers = Vec::new();
    for p in w.players.ids() {
        let h = &w.players.hot[p];
        if h.status != PlayerStatus::Active || h.club.is_none() {
            continue;
        }
        let c = &w.players.cold[p];
        let left = c.contract.days_left(today);
        let key = matches!(c.status, SquadStatus::Star | SquadStatus::Important);
        let lead = i32::from(if key { t.renewal_lead_days_key } else { t.renewal_lead_days_regular });
        if left > lead || left < 0 {
            continue;
        }
        let age = w.age(p);
        let wanted = if c.contract.kind == pw_world::ContractKind::Youth {
            // Scholars earn a first professional deal only if the club believes in them.
            crate::youth::worth_pro_contract(w, p)
        } else {
            // The club decides on how it rates him, not on his hidden ability.
            let (ca, _, pa, _) = crate::scouting::view(w, h.club, p);
            match c.status {
                SquadStatus::NotNeeded | SquadStatus::Backup => false,
                SquadStatus::Fringe => age <= 21 && pa >= ca + 15.0,
                _ => age < 33 || ca >= 130.0,
            }
        };
        // A rise has to fit the wage budget, unless the club counts on him or cannot do without: a key player is kept even over budget,
        // and so is anyone wanted while the first team could not otherwise fill a matchday squad. Over budget, the rest are let go.
        let rise = market::wage_demand(w, p, h.club) - c.contract.current_wage(today);
        let short = w.club_team(h.club, pw_world::TeamKind::First).is_none_or(|t| w.teams[t].squad.len() <= MATCHDAY_SQUAD);
        let wanted = wanted && (key || short || market::wage_fits(w, h.club, rise));
        if !wanted || w.market.on_cooldown(h.club, p, today) {
            continue;
        }
        if crate::negotiation::in_talks(w, p) {
            continue;
        }
        // Stagger: each player is looked at once a month.
        if !(p.0 + (today.0 / 7) as u32).is_multiple_of(4) {
            continue;
        }
        offers.push((p, h.club));
    }
    for (p, _club) in offers {
        // Talks open; a breakdown puts the club on cooldown inside the talks system.
        decisions::propose(w, p, Proposal::Renewal);
    }
}
