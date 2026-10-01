//! Saves and world validity (locked design section 10): a simulated world is structurally valid, survives a save/load with every
//! persistent identity intact and its metadata recorded, and the validator catches the damage a bad migration would do.

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, save, validate};
use pw_world::World;

fn world_after(days: u32) -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 31, Scale::MICRO));
    sim.run(days);
    sim
}

fn temp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("pw-saves-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d.join("w.sav")
}

#[test]
fn a_simulated_world_is_structurally_valid_at_every_stage() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 31, Scale::MICRO));
    for _ in 0..4 {
        assert_eq!(validate::problems(&sim.world), Vec::<String>::new(), "on {}", sim.world.date);
        sim.run(100);
    }
}

#[test]
fn saving_records_metadata_and_loading_preserves_every_identity() {
    let sim = world_after(150);
    let p = temp("census");
    save::save_with(&sim.world, &p, &save::Info::of_world(&sim.world)).unwrap();
    let info = save::inspect(&p).unwrap();
    let meta = info.meta.expect("saves carry metadata");
    assert_eq!((meta.created_schema, meta.world_seed), (Some(save::SCHEMA_VERSION), Some(sim.world.seed)));

    let before = validate::census(&sim.world);
    let back: World = save::load_world(&p).unwrap();
    validate::census(&back).preserved_in(&before).expect("no id moved");
    assert_eq!(before, validate::census(&back));
    assert!(validate::problems(&back).is_empty());
}

#[test]
fn the_census_notices_a_renumbered_or_dropped_entity() {
    let sim = world_after(30);
    let before = validate::census(&sim.world);
    let mut w = sim.world.clone();
    // Swap two clubs' identities (what a migration that regenerated ids would do).
    let (a, b) = (pw_core::ClubId(0), pw_core::ClubId(1));
    let name = w.clubs[a].name.clone();
    w.clubs[a].name = w.clubs[b].name.clone();
    w.clubs[b].name = name;
    assert!(before.preserved_in(&validate::census(&w)).unwrap_err().contains("clubs"));
    // New entities are fine; fewer are not.
    let mut fewer = sim.world.clone();
    fewer.staff = pw_core::IdVec::new();
    assert!(before.preserved_in(&validate::census(&fewer)).unwrap_err().contains("staff"));
}

#[test]
fn the_validator_catches_dangling_references_and_double_registrations() {
    let sim = world_after(30);

    let mut w = sim.world.clone();
    let team = w.clubs[pw_core::ClubId(0)].first_team();
    w.teams[team].squad.push(pw_core::PlayerId(u32::MAX - 1));
    assert!(validate::problems(&w).iter().any(|p| p.contains("unknown player")), "{:?}", validate::problems(&w));

    let mut w = sim.world.clone();
    let (t0, t1) = (w.clubs[pw_core::ClubId(0)].first_team(), w.clubs[pw_core::ClubId(1)].first_team());
    let p = w.teams[t0].squad[0];
    w.teams[t1].squad.push(p);
    assert!(validate::problems(&w).iter().any(|m| m.contains("two squads")), "{:?}", validate::problems(&w));

    let mut w = sim.world.clone();
    let p = w.teams[t0].squad[0];
    w.players.cold[p].contract.club = pw_core::ClubId(9999);
    assert!(validate::problems(&w).iter().any(|m| m.contains("unknown club")), "{:?}", validate::problems(&w));

    let mut w = sim.world.clone();
    w.clubs[pw_core::ClubId(0)].finance.wage_scale = f32::NAN;
    assert!(validate::problems(&w).iter().any(|m| m.contains("finances")), "{:?}", validate::problems(&w));
}

/// The census of live ids: nobody is listed twice, and every id a roster, contract or back-reference holds exists and agrees.
#[test]
fn the_validator_catches_duplicated_live_ids_and_dangling_roster_references() {
    let sim = world_after(30);
    let has = |w: &World, what: &str| validate::problems(w).iter().any(|m| m.contains(what));
    let c0 = pw_core::ClubId(0);

    let mut w = sim.world.clone();
    let t = w.clubs[c0].first_team();
    let p = w.teams[t].squad[0];
    w.teams[t].squad.push(p);
    assert!(has(&w, "two squads"), "one team listing a player twice: {:?}", validate::problems(&w));

    let mut w = sim.world.clone();
    let team = w.clubs[c0].first_team();
    w.clubs[c0].teams.push(team);
    assert!(has(&w, "twice"), "{:?}", validate::problems(&w));

    let mut w = sim.world.clone();
    let st = w.clubs[c0].staff[0];
    w.clubs[pw_core::ClubId(1)].staff.push(st);
    assert!(has(&w, "both"), "{:?}", validate::problems(&w));

    let mut w = sim.world.clone();
    w.clubs[c0].staff.push(pw_core::StaffId(u32::MAX - 1));
    assert!(has(&w, "unknown staff"), "{:?}", validate::problems(&w));

    // Two people claiming one staff record, or a staff record naming someone who does not name it back.
    let mut w = sim.world.clone();
    let st = w.clubs[c0].staff[0];
    let owner = w.staff[st].person;
    let other = w.people.iter_enumerated().map(|(id, _)| id).find(|&id| id != owner && w.people[id].staff.is_none()).expect("a person without a staff record");
    w.people[other].staff = st;
    assert!(has(&w, "names someone else"), "{:?}", validate::problems(&w));
    let mut w = sim.world.clone();
    w.staff[st].person = other;
    assert!(has(&w, "do not name each other"), "{:?}", validate::problems(&w));

    // A player registered to one club but playing for another's team.
    let mut w = sim.world.clone();
    let p = w.teams[w.clubs[c0].first_team()].squad[0];
    w.players.hot[p].club = pw_core::ClubId(1);
    assert!(has(&w, "not his club"), "{:?}", validate::problems(&w));

    // A retired player must have left his squad.
    let mut w = sim.world.clone();
    let p = w.teams[w.clubs[c0].first_team()].squad[0];
    w.players.hot[p].status = pw_world::PlayerStatus::Retired;
    assert!(has(&w, "retired player"), "{:?}", validate::problems(&w));
}

#[test]
fn validation_reports_damage_in_a_loaded_world() {
    // Upgrades validate before writing (unit-tested in save.rs); this checks the validator finds real damage in a loaded world.
    let sim = world_after(30);
    let mut w = sim.world.clone();
    let t = w.clubs[pw_core::ClubId(0)].first_team();
    w.teams[t].squad.push(pw_core::PlayerId(u32::MAX - 1));
    let p = temp("damaged");
    save::save(&w, &p).unwrap();
    let loaded: World = save::load(&p).unwrap();
    assert!(validate::check(&loaded).unwrap_err().contains("unknown player"));
}
