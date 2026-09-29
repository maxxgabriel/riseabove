//! Being seen in a huge country: one big game is not a name, grassroots evidence
//! only carries so far, and the level a performance came at decides what it is worth.

use pw_data::DataPack;
use pw_import::india::{self, IndiaScale};
use pw_sim::{recognition, Sim};
use pw_world::ecosystem::Tier;

fn world(seed: u64) -> Sim {
    Sim::new(india::build(DataPack::builtin(), seed, IndiaScale::TINY))
}

fn a_kid(s: &Sim) -> pw_core::PlayerId {
    let mut ids: Vec<_> = s.world.ext.ecosystem.story.keys().copied().collect();
    ids.sort();
    *ids.iter().find(|&&p| s.world.ext.ecosystem.repute.get(&p).is_none()).unwrap_or(&ids[0])
}

fn top_academy(s: &Sim) -> pw_core::ClubId {
    let mut v: Vec<_> = s.world.youth.academies.keys().copied().collect();
    v.sort_by_key(|&c| (std::cmp::Reverse(s.world.clubs[c].reputation), c));
    v[0]
}

#[test]
fn one_huge_game_at_grassroots_is_not_recognition() {
    let mut s = world(21);
    let p = a_kid(&s);
    let club = top_academy(&s);
    recognition::credit(&mut s.world, p, Tier::Grassroots, 9.8, 1.0);
    assert!(recognition::standing(&s.world, p) < 0.03, "standing {}", recognition::standing(&s.world, p));
    assert!(!recognition::recognised_by(&s.world, club, p));
    // Even with a scout having looked twice, one game is no sample.
    if let Some(r) = s.world.ext.ecosystem.repute.get_mut(&p) {
        r.sightings = 3;
    }
    assert!(!recognition::recognised_by(&s.world, club, p));
}

#[test]
fn a_season_at_grassroots_alone_cannot_carry_to_the_top_academies() {
    let mut s = world(22);
    let p = a_kid(&s);
    let club = top_academy(&s);
    for _ in 0..60 {
        recognition::credit(&mut s.world, p, Tier::Grassroots, 8.6, 1.0);
    }
    s.world.ext.ecosystem.repute.get_mut(&p).unwrap().sightings = 6;
    let st = recognition::standing(&s.world, p);
    assert!(st < 0.35, "grassroots-only standing should be capped, got {st}");
    assert!(!recognition::recognised_by(&s.world, club, p), "the best academy invited a child on park football alone");
}

#[test]
fn the_same_evidence_at_a_higher_level_carries_further() {
    let mut s = world(23);
    let (a, b) = {
        let mut ids: Vec<_> = s.world.ext.ecosystem.story.keys().copied().collect();
        ids.sort();
        (ids[0], ids[1])
    };
    for _ in 0..10 {
        recognition::credit(&mut s.world, a, Tier::Grassroots, 8.2, 1.0);
        recognition::credit(&mut s.world, b, Tier::State, 8.2, 1.0);
    }
    let (sa, sb) = (recognition::standing(&s.world, a), recognition::standing(&s.world, b));
    assert!(sb > 3.0 * sa, "state {sb} vs grassroots {sa}");
    assert_eq!(recognition::best_tier(&s.world, b), Some(Tier::State));
    let asp = recognition::aspects(&s.world, b);
    assert!(asp.level > 0.9 && asp.sample > 0.4 && asp.coverage >= 0.0);
}

#[test]
fn most_children_are_not_noticed_as_the_world_runs() {
    let mut s = world(24);
    s.run(400);
    let w = &s.world;
    let n = w.ext.ecosystem.story.len().max(1);
    let seen = w.ext.ecosystem.story.values().filter(|st| st.found_by.is_some()).count();
    assert!(seen * 3 < n * 2, "{seen} of {n} were found; recognition should be scarce");
    let b = pw_sim::invariants::check(w);
    assert!(b.is_empty(), "{b:#?}");
}

/// Funnel report for tuning how scarce recognition is: `cargo test --release -p pw-cli --test recognition funnel -- --ignored --nocapture`.
#[test]
#[ignore = "report"]
fn funnel() {
    use pw_world::ecosystem::StageKind::*;
    for seed in [31u64, 32] {
        let mut s = world(seed);
        {
            let eco = &s.world.ext.ecosystem;
            let a0 = eco.stages.values().filter(|v| v.iter().any(|x| x.kind == Academy)).count();
            eprintln!("seed {seed} at start: prospects {}, academy stages {a0}", eco.story.len());
        }
        s.run(1100);
        let w = &s.world;
        let eco = &w.ext.ecosystem;
        let kids: Vec<_> = eco.story.keys().copied().collect();
        let n = kids.len();
        let mut seen1 = 0;
        let mut seen2 = 0;
        let mut found = 0;
        let mut standing_hi = 0;
        for &p in &kids {
            if let Some(r) = eco.repute.get(&p) {
                seen1 += usize::from(r.sightings >= 1);
                seen2 += usize::from(r.sightings >= 2);
            }
            found += usize::from(eco.story[&p].found_by.is_some());
            standing_hi += usize::from(recognition::standing(w, p) > 0.3);
        }
        let acad: Vec<_> = eco.stages.iter().filter(|(_, v)| v.iter().any(|x| x.kind == Academy)).map(|(p, _)| *p).collect();
        let in_story = acad.iter().filter(|p| eco.story.contains_key(p)).count();
        let seen2_acad = acad.iter().filter(|p| eco.repute.get(p).is_some_and(|r| r.sightings >= 2)).count();
        eprintln!("seed {seed}: academy joiners {} (in story {in_story}, sighted>=2 {seen2_acad})", acad.len());
        let stage = |k| eco.stages.values().filter(|v| v.iter().any(|s| s.kind == k)).count();
        eprintln!(
            "seed {seed}: prospects {n}, sighted>=1 {seen1}, sighted>=2 {seen2}, found {found}, standing>0.3 {standing_hi}, district {}, trial {}, academy {}, camp {}, state team {}, univ {}, pro {}, released {}",
            stage(District), stage(Trial), stage(Academy), stage(NationalCamp), stage(StateTeam), stage(University), stage(Professional), stage(Released)
        );
    }
}
