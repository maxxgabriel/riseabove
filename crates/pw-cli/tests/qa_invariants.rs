//! QA: long-run structural invariants, checked every 100 simulated days for three seasons on the micro and tiny worlds.
//!
//! Everything here is a property that must hold at every moment, whatever the economy is doing: references resolve, squads are legal
//! sizes, contracts belong to real clubs and have not silently lapsed, events only cite earlier events, negotiations are filed
//! consistently, and no number has left its range.

mod qa_common;

use pw_core::{ClubId, Date};
use pw_import::synthetic::Scale;
use pw_sim::{audit, validate};
use pw_world::event::Cause;
use pw_world::negotiation::{TalkEnd, TalkState};
use pw_world::{PlayerStatus, TeamKind, World};
use qa_common::*;

/// Every problem found in the world today, as text. Empty when the world is sound.
fn problems(w: &World) -> Vec<String> {
    let mut out: Vec<String> = validate::problems(w);
    for v in audit::audit(w) {
        out.push(format!("audit: {v:?}"));
    }
    let today = w.date;
    let n_clubs = w.clubs.len();

    // Squads: a first team can field a side and none grows without bound; nobody is listed twice within a kind.
    for (tid, t) in w.teams.iter_enumerated() {
        if t.kind == TeamKind::First {
            if t.squad.len() < 11 {
                out.push(format!("first team {tid:?} of club {:?} has only {} players", t.club, t.squad.len()));
            }
            if t.squad.len() > 70 {
                out.push(format!("first team {tid:?} of club {:?} has {} players", t.club, t.squad.len()));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for &p in &t.squad {
            if !seen.insert(p) {
                out.push(format!("player {p:?} is listed twice in {tid:?}"));
            }
        }
    }

    // Players: statuses agree with clubs; contracts have not lapsed unnoticed; bytes stay in range.
    for (id, h) in w.players.hot.iter_enumerated() {
        let c = &w.players.cold[id];
        if h.status == PlayerStatus::Active {
            if c.contract.end.0 + 2 < today.0 && c.loan.is_none() {
                out.push(format!("active player {id:?} has been out of contract since {:?} (today {:?})", c.contract.end, today));
            }
            if c.contract.wage < 0 {
                out.push(format!("player {id:?} has a negative wage"));
            }
            if c.contract.club.0 as usize >= n_clubs {
                out.push(format!("player {id:?} contracted to unknown club"));
            }
        }
        if c.ca == 0 || c.ca > 200 || c.pa > 200 {
            out.push(format!("player {id:?} has impossible ability {}/{}", c.ca, c.pa));
        }
        if c.value < 0 {
            out.push(format!("player {id:?} has negative value {}", c.value));
        }
        for (name, v) in [("condition", h.condition), ("morale", h.morale), ("confidence", h.confidence), ("fatigue", h.fatigue)] {
            if v > 100 {
                out.push(format!("player {id:?} has {name} {v} > 100"));
            }
        }
    }

    // Money: finite and sane; the wage bill is never negative.
    for (id, c) in w.clubs.iter_enumerated() {
        let f = &c.finance;
        if f.wage_bill < 0 || f.wage_budget < 0 || f.debt < 0 {
            out.push(format!("club {id:?} has a negative wage bill/budget/debt: {f:?}"));
        }
        if c.board.satisfaction > 100 {
            out.push(format!("club {id:?} board satisfaction {}", c.board.satisfaction));
        }
    }

    // Events: strictly increasing ids, causes only look backwards, the date never runs ahead of the world.
    let mut last = None;
    let mut known: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for e in w.events.all() {
        if let Some(prev) = last
            && e.id.0 <= prev
        {
            out.push(format!("event id {} follows {prev}", e.id.0));
        }
        last = Some(e.id.0);
        if e.date.0 > today.0 {
            out.push(format!("event {} is dated in the future ({:?})", e.id.0, e.date));
        }
        for c in &e.causes {
            if let Cause::Event(cid) = c
                && cid.is_some()
                && cid.0 >= e.id.0
            {
                out.push(format!("event {} cites cause {} which is not earlier", e.id.0, cid.0));
            }
        }
        known.insert(e.id.0);
    }

    // Negotiations: open talks are registered and the player is registered to them; finished talks say how they ended.
    for (tid, t) in w.talks.iter_enumerated() {
        if t.club.0 as usize >= n_clubs || t.player.0 as usize >= w.players.hot.len() {
            out.push(format!("talk {tid:?} names an unknown club or player"));
            continue;
        }
        if t.round == 0 || t.round > t.max_rounds + 1 {
            out.push(format!("talk {tid:?} is in round {} of {}", t.round, t.max_rounds));
        }
        if t.is_open() {
            if w.market.talking.get(&t.player) != Some(&tid) {
                out.push(format!("open talk {tid:?} for {:?} is not the one registered for him", t.player));
            }
        } else {
            if t.end.is_none() {
                out.push(format!("closed talk {tid:?} has no ending"));
            }
            if w.market.talking.get(&t.player) == Some(&tid) {
                out.push(format!("closed talk {tid:?} is still registered as active"));
            }
            if (t.state == TalkState::Agreed) != (t.end == Some(TalkEnd::Signed)) {
                out.push(format!("talk {tid:?} state {:?} does not match ending {:?}", t.state, t.end));
            }
        }
        if t.offer.wage < 0 || t.offer.years == 0 {
            out.push(format!("talk {tid:?} offers a wage of {} for {} years", t.offer.wage, t.offer.years));
        }
    }
    for (p, tid) in &w.market.talking {
        match w.talks.get(*tid) {
            Some(t) if t.is_open() && t.player == *p => {}
            _ => out.push(format!("market lists {p:?} as in talks {tid:?} but that talk is not open for him")),
        }
    }
    let _ = (Date(0), ClubId::NONE);
    out
}

fn assert_sound_every_100_days(scale: Scale, seed: u64, days: u32) {
    let mut s = sim(scale, seed);
    let mut d = 0;
    while d <= days {
        let p = problems(&s.world);
        assert!(p.is_empty(), "seed {seed}, day {d} ({}): {} problems, first: {:?}", s.world.date, p.len(), &p[..p.len().min(8)]);
        s.run(100);
        d += 100;
    }
}

#[test]
fn micro_world_stays_sound_for_three_seasons() {
    assert_sound_every_100_days(Scale::MICRO, 71, 1100);
}

#[test]
fn micro_world_stays_sound_for_three_seasons_second_seed() {
    assert_sound_every_100_days(Scale::MICRO, 72, 1100);
}

#[test]
fn tiny_world_stays_sound_for_three_seasons() {
    assert_sound_every_100_days(Scale::TINY, 73, 1100);
}

/// Heavier: the small world for three seasons and four seeds of tiny. Run with
/// `cargo test --release -p pw-cli --test qa_invariants -- --ignored --nocapture`.
#[test]
#[ignore = "heavy: small world for three seasons plus three more tiny seeds, about a minute in release"]
fn small_and_more_tiny_worlds_stay_sound() {
    assert_sound_every_100_days(Scale::SMALL, 74, 1100);
    for seed in 75..78 {
        assert_sound_every_100_days(Scale::TINY, seed, 1100);
    }
}
