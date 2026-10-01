//! QA: damage the world in ways the simulation itself might never reach, then let it run. It must not panic and must stay structurally
//! valid: a club with no manager, one buried in debt, one with no staff at all, one with no governance record, one whose whole squad
//! is out of contract tomorrow.

mod qa_common;

use pw_core::{ClubId, StaffId};
use pw_import::synthetic::Scale;
use qa_common::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn a_world_with_wrecked_clubs_keeps_running_and_stays_valid() {
    let mut sim = ran(Scale::TINY, 153, 200);
    let clubs: Vec<ClubId> = sim.world.clubs.ids().take(5).collect();
    for (i, &c) in clubs.iter().enumerate() {
        let w = &mut sim.world;
        match i {
            0 => w.clubs[c].manager = StaffId::NONE,
            1 => {
                w.clubs[c].finance.balance = -2_000_000_000;
                w.clubs[c].finance.debt = 5_000_000_000;
            }
            2 => {
                let staff = w.clubs[c].staff.clone();
                for s in staff {
                    w.staff[s].club = ClubId::NONE;
                }
                w.clubs[c].staff.clear();
                w.clubs[c].manager = StaffId::NONE;
            }
            3 => {
                w.governance.remove(&c);
            }
            _ => {
                let squad = w.teams[w.clubs[c].first_team()].squad.clone();
                for p in squad {
                    w.players.cold[p].contract.end = w.date.add_days(1);
                }
            }
        }
    }
    let r = catch_unwind(AssertUnwindSafe(|| {
        for _ in 0..4 {
            sim.run(100);
            let p = pw_sim::validate::problems(&sim.world);
            assert!(p.is_empty(), "on {}: {:?}", sim.world.date, &p[..p.len().min(6)]);
        }
    }));
    assert!(r.is_ok(), "the world panicked or became invalid after damage to five clubs");
}
