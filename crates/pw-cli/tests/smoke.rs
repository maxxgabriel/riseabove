//! SMOKE tier: a four-club world, run for a season or less, checked for the invariants that break first.
//! Runs in a few seconds; `cargo xtask smoke`. It does not replace `world.rs` (tiny/small scale, longer runs), it is the cheap check
//! after a coherent block of work.

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::{PlayerStatus, World};

const SEED: u64 = 23;

fn micro() -> Sim {
    Sim::new(synthetic::build(DataPack::builtin(), SEED, Scale::MICRO))
}

/// Structural invariants: ids resolve, squads and registrations agree, contracts point at real clubs.
fn check_consistent(w: &World) {
    let v = pw_sim::audit::audit(w);
    assert!(v.is_empty(), "audit violations: {v:?}");
    let mut seen = std::collections::HashSet::new();
    for (tid, team) in w.teams.iter_enumerated() {
        assert!((team.club.0 as usize) < w.clubs.len(), "team {tid:?} names a club that does not exist");
        for &p in &team.squad {
            assert!((p.0 as usize) < w.players.hot.len(), "squad of {tid:?} holds unknown player {p:?}");
            assert!(seen.insert((p, team.kind)), "player {p:?} listed twice among {:?} squads", team.kind);
        }
    }
    for (id, h) in w.players.hot.iter_enumerated() {
        if h.status == PlayerStatus::Active {
            assert!((h.club.0 as usize) < w.clubs.len(), "player {id:?} registered to unknown club");
            assert!(w.teams[h.team].squad.contains(&id), "active player {id:?} is missing from the squad of his team");
            let c = &w.players.cold[id].contract;
            assert!((c.club.0 as usize) < w.clubs.len(), "player {id:?} has a contract with an unknown club");
            assert!(c.end >= c.start, "player {id:?} has a contract that ends before it starts");
        }
    }
}

#[test]
fn the_micro_world_builds_and_is_consistent_before_a_day_is_played() {
    let sim = micro();
    assert_eq!(sim.world.clubs.len(), 4);
    assert!(sim.world.players.hot.len() >= 4 * 11, "every club can field a team");
    check_consistent(&sim.world);
}

#[test]
fn a_season_of_four_clubs_completes_with_matches_and_consistent_state() {
    let mut sim = micro();
    let start = sim.world.date;
    sim.run(400);
    let w = &sim.world;
    assert!(w.date > start.add_days(399), "the calendar advanced");
    let played = w.fixtures.iter().filter(|(_, f)| f.score.is_some()).count();
    assert!(played >= 12, "a four-club double round-robin was played: {played} matches");
    assert!(!w.events.all().is_empty(), "things happened");
    check_consistent(w);
}

#[test]
fn transfers_contracts_or_releases_happen_within_two_seasons() {
    use pw_world::EventKind as E;
    let mut sim = micro();
    sim.run(2 * 365);
    let moves = sim.world.events.all().iter().filter(|e| matches!(e.kind, E::Transfer { .. } | E::ContractSigned { .. } | E::Released { .. } | E::Retired { .. } | E::YouthIntake { .. })).count();
    assert!(moves > 0, "two seasons pass with no market, contract or intake activity at all");
    check_consistent(&sim.world);
}

#[test]
fn the_same_seed_replays_and_a_saved_world_continues_exactly() {
    let dir = std::env::temp_dir().join(format!("pw-smoke-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("m.sav");

    let mut a = micro();
    a.run(90);
    pw_sim::save::save(&a.world, &path).unwrap();
    let mut b = Sim::new(pw_sim::save::load::<World>(&path).unwrap());
    check_consistent(&b.world);

    let mut c = micro();
    c.run(90);
    a.run(60);
    b.run(60);
    c.run(60);
    let fp = |w: &World| {
        let mut parts: Vec<u64> = w.events.all().iter().map(|e| pw_core::rng::hash_key(&[u64::from(e.id.0), e.date.0 as u64, format!("{:?}", e.kind).len() as u64])).collect();
        for comp in w.comps.iter() {
            for r in &comp.state.table {
                parts.push(pw_core::rng::hash_key(&[u64::from(r.team.0), u64::from(r.points as u16), u64::from(r.gf)]));
            }
        }
        pw_core::rng::hash_key(&parts)
    };
    assert_eq!(fp(&a.world), fp(&b.world), "a loaded save continues exactly like the world it was saved from");
    assert_eq!(fp(&a.world), fp(&c.world), "the same seed replays exactly");
    let _ = std::fs::remove_dir_all(&dir);
}
