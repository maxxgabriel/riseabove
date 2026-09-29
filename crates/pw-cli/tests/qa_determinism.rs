//! QA: determinism and save round-trips, on the whole state of the world rather than a digest of events.
//!
//! * The same seed run twice must give byte-identical saved state (not just the same league tables).
//! * A world saved, loaded and continued must land where the uninterrupted world lands.
//! * Save -> load -> save is a fixed point.
//! * Different seeds and a different playthrough salt diverge.

mod qa_common;

use pw_import::synthetic::Scale;
use pw_sim::Sim;
use pw_world::World;
use qa_common::*;

#[test]
fn the_same_seed_twice_gives_byte_identical_state_micro() {
    let a = ran(Scale::MICRO, 101, 220);
    let b = ran(Scale::MICRO, 101, 220);
    assert_eq!(digest(&a.world), digest(&b.world), "event-level digest differs");
    assert_eq!(state_hash(&a.world), state_hash(&b.world), "the saved bytes differ for the same seed: some unordered or clock-driven state leaked into the run");
}

#[test]
fn the_same_seed_twice_gives_byte_identical_state_tiny() {
    let a = ran(Scale::TINY, 202, 150);
    let b = ran(Scale::TINY, 202, 150);
    assert_eq!(digest(&a.world), digest(&b.world));
    assert_eq!(state_hash(&a.world), state_hash(&b.world), "the saved bytes differ for the same seed");
}

#[test]
fn different_seeds_and_playthrough_salts_diverge() {
    let a = ran(Scale::MICRO, 1, 120);
    let b = ran(Scale::MICRO, 2, 120);
    assert_ne!(state_hash(&a.world), state_hash(&b.world));
    let w1 = pw_import::synthetic::build(pw_data::DataPack::builtin(), 7, Scale::MICRO);
    let mut w2 = w1.clone();
    w2.begin_playthrough(99);
    let (mut s1, mut s2) = (Sim::new(w1), Sim::new(w2));
    s1.run(150);
    s2.run(150);
    assert_ne!(digest(&s1.world), digest(&s2.world), "two playthroughs of one starting world should diverge (S22)");
}

/// What a save round trip must preserve: every identity, the structure, and everything the digest reads. (The bytes of a second save can
/// differ from the first, because unordered maps are written in whatever order they hold, so this compares meaning, not bytes.)
#[test]
fn a_round_trip_after_many_days_preserves_identity_structure_and_digest() {
    let sim = ran(Scale::TINY, 33, 400);
    let p = temp_path("fixed");
    pw_sim::save::save_with(&sim.world, &p, &pw_sim::save::Info::of_world(&sim.world)).unwrap();
    let first = std::fs::metadata(&p).unwrap().len();
    let back: World = pw_sim::save::load_world(&p).unwrap();
    let q = temp_path("fixed2");
    pw_sim::save::save_with(&back, &q, &pw_sim::save::Info::of_world(&back)).unwrap();
    let second = std::fs::metadata(&q).unwrap().len();
    assert_eq!(digest(&sim.world), digest(&back), "the digest of a reloaded world differs");
    pw_sim::validate::census(&back).preserved_in(&pw_sim::validate::census(&sim.world)).expect("an id moved");
    assert!(pw_sim::validate::problems(&back).is_empty());
    let drift = (first as f64 - second as f64).abs() / first as f64;
    assert!(drift < 0.01, "a second save is {second} bytes against {first}: the round trip lost or invented data");
    cleanup(&p);
    cleanup(&q);
}

/// Continuing a reloaded world reaches the same state as never having stopped, checked on the digest of all events (id, date, kind, causes) and totals.
fn continue_matches(scale: Scale, seed: u64, before: u32, after: u32) {
    let mut a = sim(scale, seed);
    a.run(before);
    let p = temp_path("cont");
    pw_sim::save::save_with(&a.world, &p, &pw_sim::save::Info::of_world(&a.world)).unwrap();
    let mut b = Sim::new(pw_sim::save::load_world(&p).unwrap());
    cleanup(&p);
    a.run(after);
    b.run(after);
    assert_eq!(digest(&a.world), digest(&b.world), "a reloaded world diverged from the uninterrupted one (seed {seed}, {before}+{after} days)");
}

#[test]
fn a_reloaded_world_continues_identically_micro_across_a_season_boundary() {
    continue_matches(Scale::MICRO, 5, 330, 120);
}

#[test]
fn a_reloaded_world_continues_identically_tiny() {
    continue_matches(Scale::TINY, 12, 200, 100);
}

#[test]
fn two_reloads_in_a_row_are_still_the_same_world() {
    let mut a = sim(Scale::MICRO, 44);
    a.run(100);
    let mut b = Sim::new(a.world.clone());
    for _ in 0..3 {
        let p = temp_path("multi");
        pw_sim::save::save(&b.world, &p).unwrap();
        b = Sim::new(pw_sim::save::load_world(&p).unwrap());
        cleanup(&p);
        a.run(40);
        b.run(40);
    }
    assert_eq!(digest(&a.world), digest(&b.world));
}

/// Reload at different points, including across the 1 July season boundary and a Monday/first-of-month mix, on several seeds.
#[test]
fn reload_continuation_holds_across_seeds_and_reload_dates() {
    for (seed, before) in [(6u64, 45u32), (7, 181), (8, 240), (9, 361)] {
        continue_matches(Scale::MICRO, seed, before, 160);
    }
    for (seed, before) in [(13u64, 100u32), (14, 330)] {
        continue_matches(Scale::TINY, seed, before, 120);
    }
}

/// A wide sweep for order-dependent code: whenever a system walks an unordered map without sorting, a reloaded world (whose maps are
/// rebuilt in another order) leaves the uninterrupted world's path. `cargo test --release -p pw-cli --test qa_determinism -- --ignored`.
#[test]
#[ignore = "heavy: 24 micro runs and 6 tiny runs with reloads, about half a minute in release"]
fn reload_sweep_finds_no_order_dependent_system() {
    for seed in 20u64..44 {
        continue_matches(Scale::MICRO, seed, 30 + (seed as u32 * 37) % 330, 200);
    }
    for seed in 50u64..56 {
        continue_matches(Scale::TINY, seed, 60 + (seed as u32 * 53) % 300, 250);
    }
}
