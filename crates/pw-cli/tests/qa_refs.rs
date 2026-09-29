//! QA: two-way references. `validate::problems` checks that ids resolve; these check that the things they point at point back, because a
//! one-way reference is how a person ends up in two places (the manager bug found by the soak was of this kind).
//!
//! * a member of a club's staff list works for that club and for no other; a person with a club is in its list
//! * a first-team captain who has left is replaced within a month
//! * a player's team belongs to the club he is registered with, or to the club he is on loan at
//! * a loanee's loan record agrees with his registration
//! * a staff record's person names the staff record back
//! * journalists are people with a record, and an account's owner is a person who names that account

mod qa_common;

use pw_import::synthetic::Scale;
use pw_world::{PlayerStatus, TeamKind, World};
use qa_common::*;

fn ref_problems(w: &World) -> Vec<String> {
    let mut out = Vec::new();
    let mut listed = std::collections::HashMap::new();
    for (cid, c) in w.clubs.iter_enumerated() {
        let mut dup = std::collections::HashSet::new();
        for &s in &c.staff {
            if !dup.insert(s) {
                out.push(format!("club {cid:?} lists staff {s:?} twice"));
            }
            if let Some(o) = listed.insert(s, cid) {
                out.push(format!("staff {s:?} is on the lists of both {o:?} and {cid:?}"));
            }
            if w.staff[s].club != cid {
                out.push(format!("club {cid:?} lists staff {s:?} who works for {:?}", w.staff[s].club));
            }
        }
    }
    for (sid, s) in w.staff.iter_enumerated() {
        if s.club.is_some() && !s.retired && !w.clubs[s.club].staff.contains(&sid) {
            out.push(format!("staff {sid:?} works for {:?} but is not on its list", s.club));
        }
        if w.people[s.person].staff != sid {
            out.push(format!("staff {sid:?} and person {:?} do not name each other", s.person));
        }
    }
    for (tid, t) in w.teams.iter_enumerated() {
        for &p in &t.squad {
            let h = &w.players.hot[p];
            if h.team != tid {
                out.push(format!("player {p:?} is in the squad of {tid:?} but his team is {:?}", h.team));
            }
        }
    }
    for (id, h) in w.players.hot.iter_enumerated() {
        if h.status != PlayerStatus::Active {
            continue;
        }
        let c = &w.players.cold[id];
        let team_club = w.teams[h.team].club;
        match &c.loan {
            None => {
                if team_club != h.club {
                    out.push(format!("player {id:?} is registered with {:?} but plays for a team of {team_club:?}", h.club));
                }
                if c.contract.club != h.club {
                    out.push(format!("player {id:?} is registered with {:?} but his contract is with {:?}", h.club, c.contract.club));
                }
            }
            Some(l) => {
                if l.parent != h.club {
                    out.push(format!("loanee {id:?}: loan parent {:?} but registered with {:?}", l.parent, h.club));
                }
                if team_club != l.club {
                    out.push(format!("loanee {id:?}: loan club {:?} but plays for a team of {team_club:?}", l.club));
                }
                if l.parent == l.club {
                    out.push(format!("loanee {id:?} is on loan to his own club"));
                }
            }
        }
    }
    for (&p, j) in &w.media.journalists {
        if w.people[p].player.is_some() && w.players.hot[w.people[p].player].status != PlayerStatus::Retired {
            out.push(format!("journalist {p:?} is also a player who has not retired"));
        }
        if j.person != p {
            out.push(format!("journalist record {p:?} names {:?}", j.person));
        }
    }
    for (i, a) in w.net.accounts.iter().enumerate() {
        if a.person.is_some() && w.net.by_person.get(&a.person) != Some(&(i as u32)) {
            out.push(format!("account {i} ({}) belongs to {:?} who is filed under another account", a.handle, a.person));
        }
    }
    out
}

/// Captains are re-chosen on the 1st of each month, so a first-team captain who has left can be stale for up to a month; never longer.
fn stale_captains(w: &World, since: &mut std::collections::HashMap<pw_core::TeamId, pw_core::Date>, out: &mut Vec<String>) {
    for (tid, t) in w.teams.iter_enumerated() {
        let stale = t.kind == TeamKind::First && t.captain.is_some() && !t.squad.contains(&t.captain) && t.squad.iter().any(|&p| w.players.hot[p].status == PlayerStatus::Active);
        if !stale {
            since.remove(&tid);
            continue;
        }
        let from = *since.entry(tid).or_insert(w.date);
        if from.days_until(w.date) > 32 {
            out.push(format!("first team {tid:?} has had captain {:?} who is not in the squad since {from:?}", t.captain));
        }
    }
}

fn check(scale: Scale, seed: u64, days: u32, step: u32) {
    let mut s = sim(scale, seed);
    let mut since = std::collections::HashMap::new();
    for d in 0..=days {
        let mut p = Vec::new();
        stale_captains(&s.world, &mut since, &mut p);
        if d % step == 0 {
            p.extend(ref_problems(&s.world));
        }
        assert!(p.is_empty(), "seed {seed}, day {d} ({}): {} problems, first {:#?}", s.world.date, p.len(), &p[..p.len().min(6)]);
        s.step();
    }
}

#[test]
fn references_point_both_ways_in_tiny_worlds() {
    for seed in 1..=2 {
        check(Scale::TINY, seed, 4 * 365, 60);
    }
}

#[test]
fn references_point_both_ways_in_micro_worlds() {
    for seed in 1..=3 {
        check(Scale::MICRO, seed, 4 * 365, 45);
    }
}

/// Heavier: more seeds and years. `--ignored` (about a minute).
#[test]
#[ignore = "heavy: eight worlds for six years, references checked daily"]
fn references_point_both_ways_over_many_seeds() {
    for seed in 10..14 {
        check(Scale::TINY, seed, 6 * 365, 30);
        check(Scale::MICRO, seed, 8 * 365, 30);
    }
}
