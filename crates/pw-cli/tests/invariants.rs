//! Structural invariants and player provenance (Wave 0 of the simulation
//! expansion). Kept in its own file so it does not touch the shared world tests.

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::player::PlayerSource;

fn ran(seed: u64, days: u32) -> Sim {
    let mut s = Sim::new(synthetic::build(DataPack::builtin(), seed, Scale::TINY));
    s.run(days);
    s
}

#[test]
fn a_fresh_world_satisfies_every_invariant() {
    let s = Sim::new(synthetic::build(DataPack::builtin(), 21, Scale::TINY));
    let b = pw_sim::invariants::check(&s.world);
    assert!(b.is_empty(), "{b:#?}");
}

#[test]
fn invariants_hold_through_a_season() {
    let mut s = Sim::new(synthetic::build(DataPack::builtin(), 22, Scale::TINY));
    // Checked at intervals so a breach is caught near its cause.
    for _ in 0..8 {
        s.run(45);
        let b = pw_sim::invariants::check(&s.world);
        assert!(b.is_empty(), "day {}: {b:#?}", s.world.days_simulated);
    }
}

#[test]
fn every_player_has_a_recorded_origin() {
    let s = ran(23, 420);
    let w = &s.world;
    let all = w.players.created_by_source(pw_core::Date(i32::MIN), pw_core::Date(i32::MAX));
    assert_eq!(all.iter().sum::<u32>() as usize, w.players.len());
    // The starting population is the synthetic fixture; later players are named by the youth domain.
    let fixture = all[PlayerSource::ALL.iter().position(|&x| x == PlayerSource::SyntheticFixture).unwrap()];
    assert!(fixture > 0);
    let youth: u32 = [PlayerSource::AcademyIntake, PlayerSource::GrassrootsCohort].iter().map(|s| all[PlayerSource::ALL.iter().position(|x| x == s).unwrap()]).sum();
    assert!(youth > 0, "a year of play created no youth players: {all:?}");
}
