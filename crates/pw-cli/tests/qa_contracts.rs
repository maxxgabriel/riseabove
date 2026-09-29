//! QA: contract and negotiation invariants, tested by opening real talks on clones of a running world and inspecting the outcome
//! against the world exactly as it stood before the talk (so signing fees, wage ripples and balance changes cannot explain a result).
//!
//! * No package the AI player accepts is below his own floor (utility of the going terms at half his wage demand, the lowest the
//!   reservation logic can ever fall to).
//! * The club's cost for the final package never exceeds `limit_cost * 1.05`; the wage never falls below the previous offer or rises
//!   above the club's private limit; rounds never exceed the maximum.
//! * An `Agreed` talk always carries a `PlayerAccepted` line, a collapsed one carries none.

mod qa_common;

use pw_core::{ClubId, PlayerId};
use pw_import::synthetic::Scale;
use pw_sim::{market, negotiation, package};
use pw_world::event::Causes;
use pw_world::negotiation::{Negotiation, TalkEnd, TalkKind, TalkLine, TalkState};
use pw_world::{PlayerStatus, World};
use qa_common::*;

fn offers(t: &Negotiation) -> Vec<pw_world::negotiation::Terms> {
    t.log
        .iter()
        .filter_map(|(_, l)| match l {
            TalkLine::ClubOffer(x) | TalkLine::ClubImproved(x) => Some(*x),
            _ => None,
        })
        .collect()
}

/// Checks that hold for any finished or paused negotiation, judged against `pre`, the world before it was opened.
fn check_talk(pre: &World, t: &Negotiation, label: &str, failures: &mut Vec<String>) {
    let (club, p) = (t.club, t.player);
    let mut bad = |m: String| failures.push(format!("{label}: {m}"));
    let cost = package::club_cost(pre, club, p, &t.offer);
    if cost > t.limit_cost as f64 * 1.05 {
        bad(format!("final package costs {cost:.0}, more than 105% of the club's limit {} ({:?})", t.limit_cost, t.end));
    }
    if t.round > t.max_rounds + 1 {
        bad(format!("round {} beyond the maximum {}", t.round, t.max_rounds));
    }
    let seq = offers(t);
    if let Some(first) = seq.first() {
        if t.offer.wage > t.limit.wage.max(first.wage) {
            bad(format!("wage {} above the club's private limit {} (first offer {})", t.offer.wage, t.limit.wage, first.wage));
        }
        for pair in seq.windows(2) {
            if pair[1].wage < pair[0].wage {
                bad(format!("the club's wage offer fell from {} to {}", pair[0].wage, pair[1].wage));
            }
        }
    }
    let accepted = t.log.iter().any(|(_, l)| matches!(l, TalkLine::PlayerAccepted));
    match t.state {
        TalkState::Agreed => {
            if !accepted {
                bad("agreed without a PlayerAccepted line".into());
            }
            if t.end != Some(TalkEnd::Signed) {
                bad(format!("agreed but ended {:?}", t.end));
            }
        }
        TalkState::Collapsed => {
            if accepted && t.end != Some(TalkEnd::Blocked) {
                bad(format!("collapsed ({:?}) after the player accepted", t.end));
            }
        }
        _ => {}
    }
    // The player's own floor. The reservation wage is `wage_demand * k` with k clamped to at least 0.5, and utility rises with wage, so
    // no accepted package can sit below the utility of the going terms at half his demand (less the 8% the last round concedes).
    if t.state == TalkState::Agreed && matches!(t.kind, TalkKind::Renewal | TalkKind::FreeAgent | TalkKind::Transfer | TalkKind::FirstPro) {
        let pr = package::priorities(pre, p);
        let floor_wage = market::wage_demand(pre, p, club) / 2;
        let u_floor = package::reservation_utility(pre, p, club, floor_wage, t.offer.years, &pr);
        let u_off = package::player_utility(pre, p, club, &t.offer, &pr);
        let slack = 0.08 * u_floor.abs() + 1e-6;
        if u_off + slack < u_floor {
            bad(format!("accepted a package worth {u_off:.1} to the player when even his floor is {u_floor:.1} ({:?}, wage {})", t.kind, t.offer.wage));
        }
    }
}

fn running() -> pw_sim::Sim {
    ran(Scale::SMALL, 91, 60)
}

#[test]
fn renewals_respect_the_clubs_limit_and_the_players_floor() {
    let sim = running();
    let w = &sim.world;
    let mut failures = Vec::new();
    let (mut n, mut agreed) = (0, 0);
    for club in w.clubs.ids().take(14) {
        let squad = w.teams[w.clubs[club].first_team()].squad.clone();
        for &p in squad.iter().step_by(4).take(5) {
            if w.players.hot[p].status != PlayerStatus::Active || negotiation::in_talks(w, p) {
                continue;
            }
            let mut c = w.clone();
            let Some(id) = negotiation::open_renewal(&mut c, p, club, Causes::new()) else { continue };
            let t = c.talks[id].clone();
            n += 1;
            if t.state == TalkState::Agreed {
                agreed += 1;
            }
            check_talk(w, &t, &format!("renewal of {p:?} at {club:?}"), &mut failures);
        }
    }
    assert!(n >= 30, "the sample is meaningful: {n}");
    assert!(agreed > 0, "some talks agree, so the floor check ran ({agreed} of {n})");
    assert!(failures.is_empty(), "{} of {n} talks broke an invariant:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn free_agent_and_transfer_talks_respect_the_same_invariants() {
    let sim = running();
    let w = &sim.world;
    let mut failures = Vec::new();
    let mut n = 0;
    // Free agents signing with clubs that have a manager.
    let free: Vec<PlayerId> = w.players.hot.iter_enumerated().filter(|(_, h)| h.status == PlayerStatus::FreeAgent).map(|(id, _)| id).take(20).collect();
    let buyers: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].manager.is_some()).take(6).collect();
    for (i, &p) in free.iter().enumerate() {
        let club = buyers[i % buyers.len()];
        let mut c = w.clone();
        if let Some(id) = negotiation::open(&mut c, p, club, TalkKind::FreeAgent, ClubId::NONE, 0, None, Causes::new()) {
            n += 1;
            check_talk(w, &c.talks[id].clone(), &format!("free agent {p:?} at {club:?}"), &mut failures);
        }
    }
    // Transfers of first-team players between two clubs of one nation.
    for &buyer in &buyers {
        let Some(seller) = w.clubs.ids().find(|&s| s != buyer && w.clubs[s].nation == w.clubs[buyer].nation) else { continue };
        let squad = w.teams[w.clubs[seller].first_team()].squad.clone();
        for &p in squad.iter().skip(2).step_by(5).take(3) {
            if w.players.hot[p].status != PlayerStatus::Active || negotiation::in_talks(w, p) {
                continue;
            }
            let mut c = w.clone();
            let fee = market::value_of(w, p);
            if let Some(id) = negotiation::open(&mut c, p, buyer, TalkKind::Transfer, seller, fee, None, Causes::new()) {
                n += 1;
                check_talk(w, &c.talks[id].clone(), &format!("transfer of {p:?} {seller:?}->{buyer:?}"), &mut failures);
            }
        }
    }
    assert!(n >= 10, "the sample is meaningful: {n}");
    assert!(failures.is_empty(), "{} of {n} talks broke an invariant:\n{}", failures.len(), failures.join("\n"));
}

/// A talk for the same player cannot be opened twice, and opening one registers it.
#[test]
fn one_talk_per_player_at_a_time() {
    let sim = running();
    let mut w = sim.world.clone();
    let club = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some()).unwrap();
    let p = w.teams[w.clubs[club].first_team()].squad[3];
    // Make him external so the talk stays open awaiting a human answer.
    let person = w.players.cold[p].person;
    w.people[person].mind = pw_world::MindKind::External;
    let first = negotiation::open_renewal(&mut w, p, club, Causes::new());
    assert!(first.is_some());
    assert!(negotiation::in_talks(&w, p));
    assert!(negotiation::open_renewal(&mut w, p, club, Causes::new()).is_none(), "a second talk for a player in talks was opened");
    assert_eq!(w.talks.iter().filter(|t| t.player == p && t.is_open()).count(), 1);
}

/// Case files kept for important contracts stay consistent with what the world shows two seasons on.
#[test]
fn contract_files_are_consistent_after_two_seasons() {
    let sim = ran(Scale::TINY, 92, 2 * 365);
    let w = &sim.world;
    let files = &w.boardroom.contracts;
    assert!(!files.is_empty(), "two seasons of a tiny world keep at least one important contract");
    for f in files {
        assert!(f.date.0 <= w.date.0, "a contract filed in the future");
        assert!((f.club.0 as usize) < w.clubs.len() && (f.player.0 as usize) < w.players.hot.len(), "{f:?}");
        assert!(f.terms.wage >= 0 && f.terms.years >= 1, "{:?}", f.terms);
        assert!(f.commitment >= f.terms.wage, "the commitment ({}) is at least a week's wage ({})", f.commitment, f.terms.wage);
        assert!(f.believed_worth >= 0);
        assert_ne!(f.priorities[0], f.priorities[1], "two priorities are two different things");
        if let Some(o) = f.outcome {
            assert!((-1.0..=1.0).contains(&o.success), "success {} out of range", o.success);
            assert!(o.date.0 >= f.date.0, "judged before it was signed");
        }
    }
}
