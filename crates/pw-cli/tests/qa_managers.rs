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
    for seed in 1..=3 {
        check_every(Scale::TINY, seed, 6, 30);
    }
}

#[test]
fn the_manager_market_stays_straight_in_the_micro_world() {
    for seed in 1..=4 {
        check_every(Scale::MICRO, seed, 8, 60);
    }
}

/// Heavier: the small world (64 clubs, many poachings) for six years. `--ignored`.
#[test]
#[ignore = "heavy: six years of the small world, checked every 60 days"]
fn the_manager_market_stays_straight_on_the_small_world() {
    check_every(Scale::SMALL, 1, 6, 60);
}
