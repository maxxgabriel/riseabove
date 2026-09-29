//! India's football ecosystem: the world builds, children come out of district
//! pools with recorded origins, and structural invariants hold as it runs.

use pw_data::DataPack;
use pw_import::india::{self, IndiaScale};
use pw_sim::Sim;
use pw_world::player::PlayerSource;

fn world(seed: u64) -> Sim {
    Sim::new(india::build(DataPack::builtin(), seed, IndiaScale::TINY))
}

#[test]
fn india_builds_with_regions_pools_and_a_full_pyramid() {
    let s = world(5);
    let w = &s.world;
    assert!(w.ext.ecosystem.is_configured());
    assert!(w.ext.ecosystem.pools.values().any(|p| p.part[0] > 0.0));
    let india = w.nations.iter_enumerated().find(|(_, n)| n.code == "IND").map(|x| x.0).unwrap();
    assert_eq!(w.nations[india].leagues.len(), 4, "four national tiers");
    let by = w.players.created_by_source(pw_core::Date(i32::MIN), pw_core::Date(i32::MAX));
    let pool = by[PlayerSource::ALL.iter().position(|&x| x == PlayerSource::RegionalPool).unwrap()];
    assert!(pool > 50, "children came out of the district pools: {by:?}");
    assert_eq!(by.iter().sum::<u32>() as usize, w.players.len());
    let b = pw_sim::invariants::check(w);
    assert!(b.is_empty(), "{b:#?}");
}

#[test]
fn india_runs_a_season_and_stays_consistent() {
    let mut s = world(6);
    for _ in 0..4 {
        s.run(100);
        let b = pw_sim::invariants::check(&s.world);
        assert!(b.is_empty(), "day {}: {b:#?}", s.world.days_simulated);
    }
    let w = &s.world;
    // Some children have been noticed and some have a route recorded.
    assert!(w.ext.ecosystem.stages.values().any(|v| v.len() >= 2), "no one has a route of more than one step");
}
