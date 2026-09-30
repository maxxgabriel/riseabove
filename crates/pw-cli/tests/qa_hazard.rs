//! QA: the incident engine reads its pressures only while a roll could still hit (`incidents::hazard_under`). The shortcut must never
//! change a decision: for every kind of incident, every subject and a sweep of rolls, it either returns exactly the plain hazard or
//! gives up on a roll the plain hazard would have refused too.
mod qa_common;

use pw_import::synthetic::Scale;
use pw_sim::incidents::{Ctx, hazard_agrees};
use pw_world::incident::IncidentKind;
use qa_common::*;

#[test]
fn the_grapevine_ceiling_is_never_below_the_inclination_it_stands_in_for() {
    let s = ran(Scale::TINY, 22, 300);
    let checked = pw_sim::grapevine::check_ceiling(&s.world, 4000).unwrap_or_else(|e| panic!("{e}"));
    assert!(checked > 2000, "only {checked} contacts checked");
}

#[test]
fn the_bounded_hazard_never_disagrees_with_the_plain_one() {
    let s = ran(Scale::TINY, 21, 400);
    let w = &s.world;
    let rolls: Vec<f32> = (0..64).map(|i| i as f32 / 64.0).chain([0.0, 0.0001, 0.4999, 0.5, 0.9999]).collect();
    let mut checked = 0;
    for club in w.clubs.ids() {
        let squad = &w.teams[w.clubs[club].first_team()].squad;
        for (i, &p) in squad.iter().enumerate() {
            let other = squad[(i + 1) % squad.len()];
            let c = Ctx { a: w.players.cold[p].person, b: w.players.cold[other].person, pa: p, pb: other, club, nation: w.clubs[club].nation, ..Ctx::default() };
            for kind in IncidentKind::ALL {
                for &roll in &rolls {
                    if let Err(e) = hazard_agrees(w, kind, &c, roll) {
                        panic!("{e}");
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 50_000, "only {checked} combinations checked");
}
