//! A two-legged tie whose first leg is postponed past the second must not be decided on one leg, and must not crash the round after.
//! Found on the real archive: a postponed Champions League first leg was decided by the second leg alone, the round moved on, and the
//! orphaned first leg later referred to a tie that no longer existed.

use pw_core::rng::Rng;
use pw_data::DataPack;
use pw_import::builder;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, schedule};
use pw_world::{CompKind, Format, TeamKind};

#[test]
fn a_postponed_first_leg_delays_the_decision_and_the_round() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 51, Scale::TINY));
    // Mid-season, so the world's own season rollover does not redraw the cup under the test.
    sim.run(70);
    let w = &mut sim.world;
    let nation = w.nations.ids().next().unwrap();
    let cup = builder::add_comp(w, "Test Cup", "TC", nation, None, CompKind::Cup, 1, TeamKind::First, 4, 0, 0, 5000, Format::Knockout { legs: 2, final_legs: 1 }, 0);
    let teams: Vec<_> = w.clubs.ids().take(4).map(|c| w.clubs[c].first_team()).collect();
    let start = w.date.add_days(3);
    schedule::draw_round(w, cup, teams.clone(), start, 2, &mut Rng::keyed(&[1, 2, 3]));
    assert_eq!(w.comps[cup].state.ties.len(), 2);
    // Postpone tie 1's first leg well beyond the second leg of both ties.
    let first_leg = w.fixtures.iter().find(|(_, f)| f.comp == cup && f.tie == 1 && f.leg == 1).map(|(id, _)| id).expect("first leg of tie 1");
    let postponed_to = start.add_days(25);
    w.fixtures.reschedule(first_leg, postponed_to);

    // Play through both scheduled leg dates.
    sim.run(3 + 7 + 1);
    let ties = &sim.world.comps[cup].state.ties;
    assert_eq!(ties.len(), 2, "the round has not moved on");
    assert!(ties[0].is_decided(), "the tie whose two legs were played is decided");
    assert_eq!(ties[1].played, 1, "only the second leg of tie 1 has been played");
    assert!(!ties[1].is_decided(), "one leg does not decide a two-legged tie");
    // The second leg that was played first was not treated as the decider.
    let second = sim.world.fixtures.iter().find(|(_, f)| f.comp == cup && f.tie == 1 && f.leg == 2).map(|(_, f)| f.clone()).unwrap();
    assert!(second.score.is_some_and(|s| s.pens.is_none()), "no shootout for a match that was not the last leg");

    // Now the postponed leg is played: it decides the tie and the final follows without any panic.
    sim.run(25);
    let w = &sim.world;
    let t1 = &w.comps[cup].state.ties;
    assert!(t1.iter().all(|t| t.is_decided()) || w.comps[cup].state.ties.len() == 1, "the round completed");
    assert!(w.fixtures.iter().filter(|(_, f)| f.comp == cup).all(|(_, f)| f.score.is_some() || f.date > w.date), "no fixture was orphaned");
    let ended = w.comps[cup].state.winner.is_some() || matches!(w.comps[cup].state.stage, pw_world::comp::Stage::Knockout(2));
    assert!(ended, "the cup went on to its final: stage {:?}", w.comps[cup].state.stage);
}
