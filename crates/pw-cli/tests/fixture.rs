//! The golden save: a schema-4 world saved once and kept in the repository (`tests/fixtures/golden_micro.pws`). Every later build must
//! open it (directly, or through the migration steps that then exist), find it valid, simulate on from it, and save and reload it
//! with every persistent identity intact (locked design 10).
//!
//! When the schema changes, this test fails until a migration step upgrades the fixture; it is regenerated only for a deliberate
//! break, with `WRITE_GOLDEN=1 cargo test -p pw-cli --test fixture write_the_golden_fixture`.

use std::path::{Path, PathBuf};

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::save::{self, Compat};
use pw_sim::{Sim, validate};
use pw_world::World;

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_micro.pws")
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("pw-golden-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d.join("w.pws")
}

/// How the fixture is made: a micro world, ninety days in, so it holds fixtures played, events, contracts, life and media.
#[test]
fn write_the_golden_fixture() {
    if std::env::var_os("WRITE_GOLDEN").is_none() {
        return;
    }
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 4242, Scale::MICRO));
    sim.run(90);
    assert!(validate::problems(&sim.world).is_empty());
    std::fs::create_dir_all(golden().parent().unwrap()).unwrap();
    save::save_with(&sim.world, &golden(), &save::Info::of_world(&sim.world)).unwrap();
}

#[test]
fn the_golden_save_opens_validates_simulates_saves_and_reloads() {
    let path = golden();
    assert!(path.exists(), "the golden fixture is missing: {}", path.display());
    let info = save::inspect(&path).unwrap();
    assert_eq!(info.compat, Compat::Current, "the golden save is schema {}, this build writes {}: add a migration step (do not regenerate the fixture)", info.schema, save::SCHEMA_VERSION);
    assert_eq!(info.meta.as_ref().and_then(|m| m.world_seed), Some(4242));

    // Opens, and is valid.
    let world: World = save::load_world(&path).unwrap();
    assert_eq!(validate::problems(&world), Vec::<String>::new());
    let start = world.date;
    let before = validate::census(&world);
    assert!(before.preserved_in(&before).is_ok());
    assert!(world.days_simulated >= 90 && !world.events.is_empty());

    // Simulates thirty days on from it, still valid and audit-clean.
    let mut sim = Sim::new(world);
    sim.run(30);
    assert_eq!(sim.world.date, start.add_days(30));
    assert_eq!(validate::problems(&sim.world), Vec::<String>::new());
    let audit = pw_sim::audit::audit(&sim.world);
    assert!(audit.is_empty(), "audit violations: {audit:?}");

    // Saves, reloads, and the reloaded world is the same world and carries on identically.
    let out = temp("roundtrip");
    save::save_with(&sim.world, &out, &save::Info::of_world(&sim.world)).unwrap();
    let back: World = save::load_world(&out).unwrap();
    assert_eq!(validate::problems(&back), Vec::<String>::new());
    assert_eq!(validate::census(&back), validate::census(&sim.world));
    assert_eq!((back.date, back.days_simulated), (sim.world.date, sim.world.days_simulated));
    let mut again = Sim::new(back);
    again.run(10);
    sim.run(10);
    // Not compared as bytes: a reloaded map iterates in another order, which changes the compressed size and nothing else.
    assert_eq!(validate::census(&sim.world), validate::census(&again.world), "a reloaded save continues with the same people, clubs and events");
    assert_eq!(digest(&sim.world), digest(&again.world), "a reloaded save continues exactly as the original");
}

/// What happened, in order, and where the money and the players' bodies stand: the observable state of a world.
fn digest(w: &World) -> (Vec<String>, i64, u64, u64) {
    let events = w.events.all().iter().rev().take(300).map(|e| format!("{:?}", e)).collect();
    let money: i64 = w.clubs.iter().map(|c| c.finance.balance).sum();
    let bodies: u64 = w.players.hot.iter().map(|h| u64::from(h.condition) * 3 + u64::from(h.morale) + u64::from(h.fatigue) * 7 + u64::from(h.injury_days)).sum();
    let goals: u64 = w.fixtures.iter().filter_map(|(_, f)| f.score).map(|s| u64::from(s.home) * 31 + u64::from(s.away)).sum();
    (events, money, bodies, goals)
}

#[test]
fn an_older_development_schema_is_refused_with_a_plain_message_and_left_untouched() {
    let sim = {
        let mut s = Sim::new(synthetic::build(DataPack::builtin(), 5, Scale::MICRO));
        s.run(5);
        s
    };
    let p = temp("old");
    save::save_with(&sim.world, &p, &save::Info::of_world(&sim.world)).unwrap();
    // A file from the last development format: the same container with the schema field of 3.
    let mut bytes = std::fs::read(&p).unwrap();
    bytes[8..12].copy_from_slice(&3u32.to_le_bytes());
    std::fs::write(&p, &bytes).unwrap();
    let info = save::inspect(&p).unwrap();
    assert_eq!((info.schema, info.compat), (3, Compat::Unsupported));
    let err = save::load_world(&p).err().expect("refused").to_string();
    assert!(err.contains("can no longer upgrade") && err.contains("oldest readable format is 4"), "{err}");
    assert_eq!(std::fs::read(&p).unwrap(), bytes, "the file was not touched");
}
