//! QA: the manager market keeps its bookkeeping straight. Every club's manager works for that club, is not retired, and runs no other
//! club; the count of employed managers equals the number of clubs that have one.
//!
//! Regression for two order-of-operations bugs found by the soak (tiny world, seed 1, first seen on 2028-07-02): `try_poach` let the club
//! the manager was leaving pick him again as an unemployed manager before the poacher had signed him, so one man ran two or three clubs;
//! and retirement flagged the manager retired only after the club had chosen a successor (who could be the same man).

mod qa_common;

use pw_import::synthetic::Scale;
use pw_world::{StaffRole, World};
use qa_common::*;

fn chair_problems(w: &World) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::new();
    for (cid, c) in w.clubs.iter_enumerated() {
        let Some(m) = c.manager.get() else { continue };
        let st = &w.staff[m];
        if st.club != cid {
            out.push(format!("club {cid:?}'s manager {m:?} works for {:?}", st.club));
        }
        if st.retired {
            out.push(format!("club {cid:?}'s manager {m:?} is retired"));
        }
        if let Some(o) = seen.insert(m, cid) {
            out.push(format!("manager {m:?} runs both {o:?} and {cid:?}"));
        }
        if st.role != StaffRole::Manager {
            out.push(format!("club {cid:?}'s manager {m:?} has the role {:?}", st.role));
        }
    }
    let with_chair = w.clubs.iter().filter(|c| c.manager.is_some()).count();
    let employed = w.staff.iter().filter(|s| s.role == StaffRole::Manager && s.employed()).count();
    if employed < with_chair {
        out.push(format!("{with_chair} clubs have a manager but only {employed} managers are employed"));
    }
    out
}

fn check_every(scale: Scale, seed: u64, years: u32, step: u32) {
    let mut s = sim(scale, seed);
    for k in 0..(years * 365 / step) {
        s.run(step);
        let p = chair_problems(&s.world);
        assert!(p.is_empty(), "seed {seed}, day {}: {} problems, first {:?}", (k + 1) * step, p.len(), &p[..p.len().min(4)]);
        let v = pw_sim::validate::problems(&s.world);
        assert!(v.is_empty(), "seed {seed}, day {}: validate says {:?}", (k + 1) * step, &v[..v.len().min(4)]);
    }
}

#[test]
fn no_manager_runs_two_clubs_and_none_is_retired_in_charge_tiny() {
    // Seed 1 is where the soak first showed it (2028-07-02, day 731); seed 2 is the entourage case (2028-03-13).
    for seed in 1..=2 {
        check_every(Scale::TINY, seed, 4, 30);
    }
}

#[test]
fn the_manager_market_stays_straight_in_the_micro_world() {
    for seed in 1..=4 {
        check_every(Scale::MICRO, seed, 8, 60);
    }
}

/// A club that has just let a manager go (sacked, resigned, contract not renewed) does not take the same man straight back: the search for
/// a successor used to find him among the unemployed. Events are read the day they are made, before compaction can drop them.
#[test]
fn a_manager_let_go_is_not_appointed_back_the_same_day() {
    use pw_world::EventKind as E;
    let (mut departures, mut rehired) = (0, Vec::new());
    for seed in 1..=6u64 {
        for scale in if seed == 1 { vec![Scale::MICRO, Scale::TINY] } else { vec![Scale::MICRO] } {
            let mut s = sim(scale, seed);
            let mut last = pw_core::EventId::NONE;
            for _ in 0..(6 * 365) {
                s.step();
                let fresh: Vec<_> = s.world.events.after(last).iter().map(|e| (e.id, e.date, e.kind.clone())).collect();
                if let Some(l) = fresh.last() {
                    last = l.0;
                }
                for (i, (_, date, kind)) in fresh.iter().enumerate() {
                    if let E::ManagerSacked { staff, club } | E::ManagerResigned { staff, club } = kind {
                        departures += 1;
                        if fresh[i + 1..].iter().any(|(_, d, k)| d == date && matches!(k, E::ManagerAppointed { staff: s2, club: c2 } if s2 == staff && c2 == club)) {
                            rehired.push(format!("seed {seed}: {staff:?} let go by {club:?} and appointed back on {date:?}"));
                        }
                    }
                }
            }
        }
    }
    assert!(departures >= 4, "the sample includes managers being let go: {departures}");
    assert!(rehired.is_empty(), "{} of {departures} managers were appointed straight back: {:?}", rehired.len(), &rehired[..rehired.len().min(4)]);
}

/// Heavier: the small world (64 clubs, many poachings) for six years. `--ignored`.
#[test]
#[ignore = "heavy: six years of the small world, checked every 60 days"]
fn the_manager_market_stays_straight_on_the_small_world() {
    check_every(Scale::SMALL, 1, 6, 60);
}
