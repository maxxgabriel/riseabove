//! The almanac: matches feed records and leaderboards at every level.

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::records::{Scope, Stat};

#[test]
fn a_season_fills_the_record_book_and_the_leaderboards() {
    let mut s = Sim::new(synthetic::build(DataPack::builtin(), 41, Scale::TINY));
    s.run(330);
    let w = &s.world;
    let has = |stat: Stat| w.records.records.keys().any(|k| k.stat == stat);
    for stat in [Stat::FastestGoal, Stat::TopSpeed, Stat::DistanceCovered, Stat::GoalsInMatch, Stat::MostGoalsInSeason, Stat::HighestAttendance] {
        assert!(has(stat), "no record set for {stat:?}");
    }
    let scorers = w.ext.almanac.leaders(Scope::World, Stat::Goals, 5);
    assert!(!scorers.is_empty(), "no all-time scoring table");
    assert!(scorers.windows(2).all(|p| p[0].1 >= p[1].1), "table is ordered");
    // Fastest-to-a-goal tables are ordered the other way.
    let quick = w.ext.almanac.leaders(Scope::World, Stat::AppsToFirstGoal, 5);
    assert!(quick.windows(2).all(|p| p[0].1 <= p[1].1));
    assert!(pw_sim::invariants::check(w).is_empty());
}
