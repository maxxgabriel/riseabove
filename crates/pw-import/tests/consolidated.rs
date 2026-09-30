use pw_core::Date;
use pw_import::{LoadOptions, fixture::{Archive, player_row}, parse_dir};

#[test]
fn configured_start_filters_evidence_before_the_archive_is_parsed() {
    let dir = Archive::standard().write("configured-start-evidence");
    std::fs::write(dir.join("world.toml"), "start_date = \"2026-06-01\"\n").unwrap();
    std::fs::write(dir.join("appearances.csv"), "player_id,date,goals,minutes_played\n10000,2026-05-01,1,90\n10000,2026-07-01,99,90\n").unwrap();
    let set = parse_dir(&dir, LoadOptions::default()).unwrap();
    let p = set.players.iter().find(|p| p.key == "10000").unwrap();
    assert_eq!(set.start, Some(Date::from_ymd(2026, 6, 1)));
    assert_eq!(p.apps, Some(1));
    assert_eq!(p.goals, Some(1), "future evidence cannot initialize a configured snapshot");
}

#[test]
fn consolidated_metadata_keeps_catalog_outside_the_world_and_validates_dates() {
    let dir = Archive::standard().write("consolidated-metadata");
    std::fs::write(dir.join("world.toml"), "start_date = \"2026-09-30\"\n").unwrap();
    let meta = serde_json::json!({"schema_version": 1, "name": "Consolidated fixture", "start_date": "2026-09-30",
        "archive_snapshot": "2026-07-06", "fm_snapshot": "2022-06-27", "registry_stamp": "fixture",
        "catalog_players": 999999, "identity_conflicts": 3, "current_squad_coverage": ["Premier League"],
        "freshness_note": "Other squads are older; attributes are inferred."});
    std::fs::write(dir.join("riseabove.database.json"), meta.to_string()).unwrap();
    std::fs::write(dir.join("catalog_players.csv"), "entity_id,name,dob\nunresolved,Unresolved,\n").unwrap();
    let set = parse_dir(&dir, LoadOptions::default()).unwrap();
    assert_eq!(set.players.len(), 264, "catalog identities must not inflate the playable population");
    assert_eq!(set.sources[0].snapshot, Some(Date::from_ymd(2026, 7, 6)));
    assert_eq!(set.issues.count("quarantined_identity_links"), 1);
    std::fs::write(dir.join("world.toml"), "start_date = \"2026-07-15\"\n").unwrap();
    assert!(parse_dir(&dir, LoadOptions::default()).unwrap_err().to_string().contains("differs"));
    let mut wrong = meta;
    wrong["schema_version"] = 99.into();
    std::fs::write(dir.join("riseabove.database.json"), wrong.to_string()).unwrap();
    assert!(parse_dir(&dir, LoadOptions::default()).is_err());
}

#[test]
fn verified_nonleague_clubs_admit_rosters_without_inventing_a_competition() {
    let mut archive = Archive::standard();
    archive.players.push(player_row("990001", "9900", "2000-01-01 00:00:00", |_| {}));
    let dir = archive.write("verified-nonleague-clubs");
    std::fs::write(dir.join("verified_unmodeled_clubs.csv"), concat!(
        "club_id,club_name,country,reep_id,registry_status,gender,crosswalk_validation,evidence_players,tm_snapshot,reep_snapshot,snapshot\n",
        "9900,Verified Town,England,rt123,active,men,unique_provider_claim,1,2026-07-06,20260926T145536Z,2026-09-26\n",
        "9901,Unverified Town,England,rt456,retired,men,unique_provider_claim,1,2026-07-06,20260926T145536Z,2026-09-26\n",
    )).unwrap();

    let set = parse_dir(&dir, LoadOptions::default()).unwrap();
    let verified = set.clubs.iter().find(|c| c.key == "9900").expect("verified club imported");
    assert_eq!(verified.nation.as_deref(), Some("England"));
    assert!(verified.league.is_none(), "competition membership must remain unknown");
    assert!(verified.extra_teams.is_empty());
    assert!(!set.clubs.iter().any(|c| c.key == "9901"), "retired bridge must be rejected");
    assert!(set.players.iter().any(|p| p.key == "990001" && p.club.as_deref() == Some("9900")));
}
