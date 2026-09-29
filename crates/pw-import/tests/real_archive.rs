//! Sanity checks on the user's real local archive (`archive/` in the repository folder, git-ignored). Skipped when the
//! folder is not there. Run with
//! `cargo test --release -p pw-import --test real_archive -- --ignored --nocapture`.

use std::path::PathBuf;

use pw_data::DataPack;
use pw_import::{LoadOptions, load_dir_with};

fn archive() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../archive");
    dir.join("players.csv").exists().then_some(dir)
}

#[test]
#[ignore = "reads the real local archive; run explicitly"]
fn the_real_archive_builds_a_structurally_sound_world() {
    let Some(dir) = archive() else {
        eprintln!("no archive folder; skipped");
        return;
    };
    let (w, rep) = load_dir_with(&dir, DataPack::builtin(), Some(1), LoadOptions::default()).expect("loads");
    println!("{}", rep.summary());

    let bad_clubs: Vec<_> = w.clubs.iter().filter(|c| c.nation.is_none()).map(|c| c.name.clone()).collect();
    assert!(bad_clubs.is_empty(), "clubs without a nation: {bad_clubs:?}");
    let bad_people = w.people.iter().filter(|p| p.nation.is_none()).count();
    println!("people without a nation: {bad_people}");
    assert!(w.clubs.len() >= 500, "{} clubs", w.clubs.len());
    for (id, c) in w.clubs.iter_enumerated() {
        let first = w.club_team(id, pw_world::TeamKind::First).expect("first team");
        assert!(w.teams[first].squad.len() >= 16, "{} has {} players", c.name, w.teams[first].squad.len());
        assert!(c.league.is_some(), "{} has no league", c.name);
    }
    for n in w.nations.iter().filter(|n| !n.leagues.is_empty()) {
        for &lg in &n.leagues {
            let c = &w.comps[lg];
            assert_eq!(c.state.entrants.len(), usize::from(c.size), "{} ({}) has {} entrants, expected {}", c.name, n.name, c.state.entrants.len(), c.size);
        }
    }
}
