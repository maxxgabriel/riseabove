//! Import pipeline tests on a small generated archive in the Transfermarkt layout. No real data is used: every club, person
//! and number is made here, so the tests exercise the rules (parsing, validation, resolution, labelling) rather than a dataset.

use pw_data::DataPack;
use pw_import::fixture::{Archive, player_row, q};
use pw_import::{LoadOptions, Severity, build_world, load_dir_with, parse_dir};
use pw_world::origin::{Facet, Origin};

fn load(a: &Archive, name: &str) -> (pw_world::World, pw_import::ImportReport) {
    load_dir_with(&a.write(name), DataPack::builtin(), Some(7), LoadOptions::default()).expect("loads")
}

// ---- tests ------------------------------------------------------------------------------------------------------

#[test]
fn a_clean_archive_builds_a_coherent_world_with_sources_kept() {
    let (w, rep) = load(&Archive::standard(), "clean");
    assert_eq!((rep.nations >= 4, rep.competitions, rep.clubs, rep.players), (true, 4, 12, 12 * 22), "{}", rep.summary());
    // The qualifier is out of scope; two leagues, a cup and the Champions League exist.
    assert_eq!(w.comps.len(), 4);
    assert_eq!(rep.unresolved, 0);
    // Squads and leagues link up.
    for club in w.clubs.ids() {
        let first = w.club_team(club, pw_world::TeamKind::First).expect("first team");
        assert_eq!(w.teams[first].squad.len(), 22);
        assert!(w.clubs[club].league.is_some());
        assert!(w.clubs[club].manager.is_some(), "a manager exists for every club");
    }
    for lg in w.nations.iter().flat_map(|n| n.leagues.clone()) {
        assert_eq!(w.comps[lg].state.entrants.len(), 6);
    }
    // Every imported record keeps its source id, and can be found by it.
    let p = w.origins.find("transfermarkt", "player:10000").expect("player found by source id");
    assert_eq!(w.people[p].display_name(&w.names), "Ana Player10000");
    assert_eq!(w.origins.clubs.len(), 12);
    assert!(w.origins.person(p).is_some());
    // Short names are derived, not the whole legal name.
    assert!(w.clubs.iter().any(|c| c.short_name == "GB1 Number 0" || c.short_name.len() < c.name.len()));
    // Country identity: Brazil is a minor nation with a real code and confederation; England is listed.
    let brazil = w.nations.iter().find(|n| n.name == "Brazil").expect("minor nation created from citizenship");
    assert_eq!((brazil.code.as_str(), brazil.confed), ("BRA", pw_world::Confed::Conmebol));
    assert_eq!(w.nations.iter().filter(|n| n.name == "England").count(), 1);
}

#[test]
fn facts_the_source_lacks_are_labelled_and_never_zero() {
    let (w, _) = load(&Archive::standard(), "labels");
    let p = w.origins.find("transfermarkt", "player:10000").unwrap();
    let o = w.origins.person(p).unwrap();
    assert_eq!(o.get(Facet::Identity), Origin::Imported);
    assert_eq!(o.get(Facet::Position), Origin::Imported);
    assert_eq!(o.get(Facet::Attributes), Origin::Inferred, "attributes come from market value");
    assert_eq!(o.get(Facet::Hidden), Origin::Generated);
    assert_eq!(o.get(Facet::Potential), Origin::Inferred);
    assert_eq!(o.get(Facet::Contract), Origin::Inferred, "contract end imported, wage estimated");
    assert_eq!(o.get(Facet::Value), Origin::Imported);
    assert_eq!(o.get(Facet::Career), Origin::Unknown, "no appearance data in the fixture: unknown, not zero");
    let pid = w.people[p].player;
    let c = &w.players.cold[pid];
    assert!(c.ca > 30 && c.pa >= c.ca && c.contract.wage > 0);
    // A richer player is better than a poorer one at the same age.
    let rich = w.people[w.origins.find("transfermarkt", "player:10000").unwrap()].player;
    let poor = w.people[w.origins.find("transfermarkt", "player:10521").unwrap()].player;
    assert!(w.players.cold[rich].ca > w.players.cold[poor].ca, "{} vs {}", w.players.cold[rich].ca, w.players.cold[poor].ca);
    // Club facts the source lacks are recorded as gaps.
    let gaps = w.origins.club_gaps.values().next().unwrap();
    assert!(gaps & pw_world::origin::gap::FINANCES != 0 && gaps & pw_world::origin::gap::CITY != 0);
}

#[test]
fn bad_rows_are_reported_and_the_good_rows_survive() {
    let mut a = Archive::standard();
    a.players.push(player_row("900", "100", "1990-02-30", |_| {})); // impossible date
    a.players.push(player_row("901", "100", "", |_| {})); // no birth date
    a.players.push(player_row("902", "100", "1995-05-05", |c| {
        c[1] = "Władysław".into();
        c[2] = "Żmuda-Ñandú 山田太郎".into();
    }));
    a.players.push(player_row("903", "100", "1995-05-06", |c| c[2] = "L".repeat(300)));
    a.players.push(player_row("10000", "100", "1980-01-01", |c| c[2] = "Duplicate".into())); // reuses an existing id
    a.players.push(player_row("905", "100", "1996-01-01", |c| c[2] = "Twin".into()));
    a.players.push(player_row("906", "100", "1996-01-01", |c| c[2] = "Twin".into())); // same name and birth date, different id
    a.players.push(player_row("907", "100", "1996-02-01", |c| c[14] = "17".into())); // height 17 cm
    a.players.push(player_row("908", "100", "1996-03-01", |c| c[9] = "Atlantis".into())); // unknown country
    a.players.push(player_row("909", "100", "1996-04-01", |c| {
        c[24] = "-500".into();
        c[15] = "2020-01-01".into();
    }));
    a.players.push(player_row("910", "77777", "1996-05-01", |_| {})); // club outside the modelled world
    a.players.push(player_row("911", "100", "1996-06-01", |c| c[4] = "2019".into())); // stale
    a.players.push(player_row("912", "100", "1996-07-01", |c| c[15] = "not-a-date".into()));
    let (w, rep) = load(&a, "bad");
    let issues = parse_dir(&a.write("bad2"), LoadOptions::default()).unwrap().issues;
    let by = |code: &str| issues.count(code);
    assert_eq!(by("missing_dob"), 2, "the impossible date and the empty date both leave a person without a birth date: {}", rep.summary());
    assert_eq!(by("bad_date"), 2, "the impossible birth date and the unparseable contract date");
    assert_eq!(by("duplicate_id"), 1);
    assert_eq!(by("possible_duplicate"), 1);
    assert_eq!(by("out_of_range"), 1);
    assert_eq!(by("unknown_country"), 1);
    assert_eq!(by("contract_expired"), 1, "only the 2020 contract; the unparseable one is a bad_date");
    assert_eq!(by("stale_player"), 1);
    assert_eq!(by("club_not_modelled"), 1);
    // Unicode and very long names survive intact.
    let p = w.origins.find("transfermarkt", "player:902").expect("unicode player");
    assert_eq!(w.people[p].display_name(&w.names), "Władysław Żmuda-Ñandú 山田太郎");
    let p = w.origins.find("transfermarkt", "player:903").unwrap();
    assert!(w.people[p].display_name(&w.names).len() >= 300);
    // First id wins.
    let p = w.origins.find("transfermarkt", "player:10000").unwrap();
    assert_eq!(w.people[p].display_name(&w.names), "Ana Player10000");
    // The twins are two people.
    assert!(w.origins.find("transfermarkt", "player:905").is_some() && w.origins.find("transfermarkt", "player:906").is_some());
    // Dropped rows are in the unresolved ledger, with reasons.
    assert!(w.origins.unresolved.iter().any(|u| u.id == "900" && u.reason.contains("date of birth")));
    assert!(w.origins.find("transfermarkt", "player:900").is_none() && w.origins.find("transfermarkt", "player:910").is_none() && w.origins.find("transfermarkt", "player:911").is_none());
    // Unknown values are labelled, not zero: height, contract, value, nationality.
    let o = |id: &str| w.origins.person(w.origins.find("transfermarkt", id).unwrap()).unwrap().clone();
    assert_eq!(o("player:907").get(Facet::Physique), Origin::Generated);
    assert!((150..=215).contains(&w.players.cold[w.people[w.origins.find("transfermarkt", "player:907").unwrap()].player].height));
    assert_eq!(o("player:909").get(Facet::Contract), Origin::Generated);
    assert_eq!(o("player:909").get(Facet::Value), Origin::Inferred);
    assert_eq!(o("player:908").get(Facet::Identity), Origin::Inferred, "nationality taken from the club, and said so");
    let end = w.players.cold[w.people[w.origins.find("transfermarkt", "player:909").unwrap()].player].contract.end;
    assert!(end > w.date, "a generated contract runs past the start date");
}

#[test]
fn transfers_build_history_only_from_known_players_and_clubs() {
    let mut a = Archive::standard();
    // Player 10000 (club 100): joined 101 in 2019 for 5m, moved to 100 in 2022 for 12m. A move to an unknown club, a future move,
    // an unparseable date, a negative fee and an unknown player are all handled.
    for row in [
        ["10000", "2019-07-01", "19/20", "999", "101", "x", "y", "5000000", "", "p"],
        ["10000", "2020-01-01", "19/20", "101", "8888", "x", "y", "100", "", "p"],
        ["10000", "2022-07-01", "22/23", "8888", "100", "x", "y", "12000000", "", "p"],
        ["10000", "2031-07-01", "30/31", "100", "101", "x", "y", "1", "", "p"],
        ["10000", "31/31/2020", "20/21", "100", "101", "x", "y", "1", "", "p"],
        ["10001", "2021-07-01", "21/22", "100", "101", "x", "y", "-5", "", "p"],
        ["424242", "2021-07-01", "21/22", "100", "101", "x", "y", "1", "", "p"],
    ] {
        a.transfers.push(q(&row));
    }
    let (w, _) = load(&a, "transfers");
    let issues = parse_dir(&a.write("transfers2"), LoadOptions::default()).unwrap().issues;
    assert_eq!(issues.count("future_transfer"), 1);
    assert_eq!(issues.count("bad_date"), 1);
    assert_eq!(issues.count("bad_amount"), 1);
    let p = w.origins.find("transfermarkt", "player:10000").unwrap();
    let pid = w.people[p].player;
    let spells = &w.history.spells[&pid];
    // Closed: 101 (2019-07-01 → 2020-01-01). Move to the unknown club leaves a gap. Open: current club since 2022 with the 12m fee.
    assert_eq!(spells.len(), 2, "{spells:?}");
    assert_eq!((spells[0].fee, spells[0].to.is_some()), (5_000_000, true));
    assert_eq!((spells[1].fee, spells[1].to), (12_000_000, None));
    assert_eq!(w.players.cold[pid].joined.year(), 2022, "joined date comes from the source");
}

#[test]
fn a_league_missing_from_the_competition_file_is_derived_from_the_country_code() {
    let mut a = Archive::standard();
    a.clubs.push(q(&["300", "club-300", "Argentine Club", "AR1", "", "22", "25", "5", "20", "3", "G", "20000", "", "", "2025", "", "u"]));
    for k in 0..20 {
        a.players.push(player_row(&format!("30{k:03}"), "300", "1995-01-01", |c| c[9] = "Argentina".into()));
    }
    let (w, _) = load(&a, "derived");
    let lg = w.comps.iter().find(|c| c.name == "Argentina First Division").expect("derived league");
    assert_eq!(w.nations[lg.nation].name, "Argentina");
    let set = parse_dir(&a.write("derived2"), LoadOptions::default()).unwrap();
    assert_eq!(set.issues.count("derived_competition"), 1);
    assert!(set.comps.iter().any(|c| c.derived));
}

#[test]
fn a_missing_league_is_placed_by_citizenship_only_when_the_declared_club_count_agrees() {
    let build = |declared: &str| {
        let mut a = Archive::standard();
        // Argentina's country code does not match the league id, so the code cannot place it.
        a.countries[2] = q(&["3", "Argentina", "ARZZ", "amerika", declared, "0", "25", "u"]);
        a.clubs.push(q(&["300", "club-300", "Argentine Club", "AR9", "", "22", "25", "5", "20", "3", "G", "20000", "", "", "2025", "", "u"]));
        for k in 0..20 {
            a.players.push(player_row(&format!("30{k:03}"), "300", "1995-01-01", |c| c[9] = "Argentina".into()));
        }
        a
    };
    // One club names the league and Argentina declares one club, and its players are Argentine.
    let set = parse_dir(&build("1").write("cit-ok"), LoadOptions::default()).unwrap();
    let lg = set.comps.iter().find(|c| c.key == "AR9").expect("league derived");
    assert_eq!((lg.nation.as_deref(), lg.derived), (Some("Argentina"), true));
    // The declared count disagrees: one piece of evidence is not enough, and the league (and its club) is left out.
    let set = parse_dir(&build("18").write("cit-no"), LoadOptions::default()).unwrap();
    assert!(set.comps.iter().all(|c| c.key != "AR9"));
    assert!(set.clubs.iter().all(|c| c.key != "300"));
    assert_eq!(set.issues.count("unknown_league"), 2);
}

#[test]
fn nobody_in_the_world_is_left_without_a_nation() {
    let a = Archive::standard();
    let dir = a.write("nonation");
    let head = "\"Name\";\"Nation\";\"Team\";\"Job\";\"Age\";\"Wage\";\"Def. Tact.\";\"Def. Tech.\";\"Att. Tact.\";\"Att. Tech.\";\"Poss. Tact.\";\"Poss. Tech.\";\"Strength\";\"Quickness\";\"GK Shot Stopping\";\"GK Handling\";\"Best Rating\"\n";
    let row = |name: &str, nation: &str| format!("\"{name}\";\"{nation}\";\"-\";\"Scout\";\"44\";\"0\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"61% (M)\"\n");
    std::fs::write(dir.join("Staff list.csv"), format!("{head}{}{}", row("Known, Kim", "France"), row("Lost, Lee", "Vietnam du Sud"))).unwrap();
    let (w, _) = load_dir_with(&dir, DataPack::builtin(), Some(7), LoadOptions::default()).unwrap();
    assert!(w.people.iter().all(|p| p.nation.is_some()), "every person has a nation");
    let names: Vec<String> = w.staff.iter().map(|s| w.people[s.person].display_name(&w.names).to_string()).collect();
    assert!(names.contains(&"Kim Known".to_string()) && !names.contains(&"Lee Lost".to_string()));
    assert!(w.origins.unresolved.iter().any(|u| u.id == "row2" && u.reason.contains("nationality")));
}

#[test]
fn a_league_no_file_describes_drops_its_clubs_and_says_so() {
    let mut a = Archive::standard();
    a.clubs.push(q(&["301", "club-301", "Lost Club", "ZZ9", "", "22", "25", "5", "20", "3", "G", "20000", "", "", "2025", "", "u"]));
    let set = parse_dir(&a.write("lost"), LoadOptions::default()).unwrap();
    assert_eq!(set.issues.count("unknown_league"), 2, "once when created, once when the club is dropped");
    assert!(set.clubs.iter().all(|c| c.key != "301"));
}

#[test]
fn unknown_competition_country_drops_only_that_competition() {
    let mut a = Archive::standard();
    a.comps.push(q(&["XX", "x", "some-league", "first_tier", "domestic_league", "9", "Freedonia", "XX", "europa", "6", "u"]));
    let set = parse_dir(&a.write("freedonia"), LoadOptions::default()).unwrap();
    assert_eq!(set.issues.count("unknown_country"), 1);
    assert!(set.comps.iter().all(|c| c.key != "XX"));
}

#[test]
fn competition_names_that_collide_are_told_apart_by_country() {
    let mut a = Archive::standard();
    a.comps.push(q(&["X1", "x", "premier-league", "first_tier", "domestic_league", "2", "Spain", "X1", "europa", "6", "u"]));
    let set = parse_dir(&a.write("collide"), LoadOptions::default()).unwrap();
    let names: Vec<_> = set.comps.iter().filter(|c| c.kind == pw_world::CompKind::League).map(|c| c.name.clone()).collect();
    assert!(names.contains(&"Premier League (England)".to_string()) && names.contains(&"Premier League (Spain)".to_string()), "{names:?}");
}

#[test]
fn past_seasons_come_only_from_complete_tables_and_carry_their_provenance() {
    let mut a = Archive::standard();
    // 2022 is incomplete (a single game); 2023 and 2024 are complete.
    a.games.push(q(&["9001", "GB1", "2022", "1", "2022-09-01", "100", "101", "1", "0"]));
    // A player who scores in 2024 makes a top scorer; one goal each in 2023 for two players is a tie and names nobody.
    a.appearances.push(q(&["1", "31", "10000", "GB1", "2", "90", "Ana Player10000"]));
    a.appearances.push(q(&["2", "31", "10001", "GB1", "1", "90", "Ana Player10001"]));
    a.appearances.push(q(&["3", "1", "10000", "GB1", "1", "90", "Ana Player10000"]));
    a.appearances.push(q(&["4", "1", "10001", "GB1", "1", "90", "Ana Player10001"]));
    let (w, _) = load(&a, "seasons");
    let mut seasons: Vec<_> = w.backfill.seasons.iter().map(|s| (s.season, w.clubs[s.champion].name.clone(), s.top_goals)).collect();
    seasons.sort();
    assert_eq!(seasons.len(), 2, "{seasons:?}");
    assert!(seasons.iter().all(|s| s.1.contains("Number 0")), "the club that won every table");
    assert!(w.backfill.seasons.iter().all(|s| s.provenance == pw_world::backfill::Provenance::Imported));
    let set = parse_dir(&a.write("seasons2"), LoadOptions::default()).unwrap();
    assert!(set.issues.count("season_not_derived") >= 1, "the one-game season is not a table");
    let s24 = set.seasons.iter().find(|s| s.season == 2024).unwrap();
    assert_eq!(s24.top_scorer.as_deref(), Some("Ana Player10000"));
    let s23 = set.seasons.iter().find(|s| s.season == 2023).unwrap();
    assert_eq!(s23.top_scorer, None, "a shared top spot is not named");
}

#[test]
fn career_counters_and_shirts_come_from_match_files_and_are_marked() {
    let mut a = Archive::standard();
    for (i, mins) in ["90", "45", "0"].iter().enumerate() {
        a.appearances.push(q(&[&i.to_string(), &(500 + i).to_string(), "10001", "GB1", "1", mins, "n"]));
    }
    // National-team games are not club career.
    a.comps.push(q(&["EURO", "euro", "uefa-euro", "uefa_euro", "national_team_competition", "-1", "", "", "europa", "", "u"]));
    a.appearances.push(q(&["9", "600", "10001", "EURO", "5", "90", "n"]));
    a.lineups.push(q(&["a", "2026-05-01", "1", "10001", "100", "7"]));
    a.lineups.push(q(&["b", "2026-05-08", "2", "10001", "100", "9"]));
    a.lineups.push(q(&["c", "2026-05-15", "3", "10001", "999", "77"])); // a different club: not this club's shirt
    let (w, _) = load(&a, "counters");
    let p = w.origins.find("transfermarkt", "player:10001").unwrap();
    let c = &w.players.cold[w.people[p].player];
    assert_eq!((c.senior_apps, c.senior_goals), (2, 2), "the unused sub and the international are not club appearances");
    assert_eq!(c.shirt, 9, "latest number at the current club");
    assert_eq!(w.origins.person(p).unwrap().get(Facet::Career), Origin::Imported);
}

#[test]
fn staff_list_joins_a_club_only_through_its_manager_and_keeps_unemployed_staff() {
    let a = Archive::standard();
    let dir = a.write("staff");
    let head = "\"Name\";\"Nation\";\"Team\";\"Job\";\"Age\";\"Wage\";\"Def. Tact.\";\"Def. Tech.\";\"Att. Tact.\";\"Att. Tech.\";\"Poss. Tact.\";\"Poss. Tech.\";\"Strength\";\"Quickness\";\"GK Shot Stopping\";\"GK Handling\";\"Best Rating\"\n";
    let mut bytes = head.as_bytes().to_vec();
    let row = |name: &str, nation: &str, team: &str, job: &str| format!("\"{name}\";\"{nation}\";\"{team}\";\"{job}\";\"44\";\"0\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"57% (3.0)\";\"61% (M)\"\n");
    // Club 100's manager is "Coach100 Surname100" in clubs.csv.
    bytes.extend_from_slice(row("Surname100, Coach100", "Angleterre", "The Hundred", "Manager").as_bytes());
    bytes.extend_from_slice(row("Nobody, Pat", "Espagne", "The Hundred", "Coach").as_bytes());
    bytes.extend_from_slice(row("Stranger, Sam", "Espagne", "Elsewhere FC", "Coach").as_bytes());
    bytes.extend_from_slice(row("Free, Una", "Br\u{e9}sil", "-", "Scout").as_bytes());
    bytes.extend_from_slice(row("Rich, Ann", "Qatar", "The Hundred", "Owner").as_bytes());
    // Windows-1252 for the accent.
    let text: Vec<u8> = String::from_utf8(bytes).unwrap().chars().map(|c| if c == '\u{e9}' { 0xe9 } else { c as u8 }).collect();
    std::fs::write(dir.join("Staff list.csv"), text).unwrap();
    let (w, rep) = load_dir_with(&dir, DataPack::builtin(), Some(7), LoadOptions::default()).unwrap();
    let club100 = w.origins.clubs.iter().find(|(_, r)| r.id == "club:100").map(|(c, _)| *c).unwrap();
    let mgr = w.clubs[club100].manager;
    assert!(mgr.is_some());
    let m = &w.staff[mgr];
    assert_eq!(w.people[m.person].display_name(&w.names), "Coach100 Surname100");
    assert_eq!(w.names.get(w.people[m.person].last), "Surname100");
    assert_eq!(w.nations[w.people[m.person].nation].name, "England", "French nation resolved");
    assert_eq!(w.origins.person(m.person).unwrap().get(Facet::Club), Origin::Inferred, "two pieces of evidence, not a stated link");
    let names: Vec<String> = w.staff.iter().map(|s| w.people[s.person].display_name(&w.names).to_string()).collect();
    assert!(names.iter().any(|n| n == "Pat Nobody"), "placed through the manager's team spelling");
    assert!(!names.iter().any(|n| n == "Sam Stranger"), "team not matched: reported, not placed");
    assert!(names.iter().any(|n| n == "Una Free"), "no team in the list: unemployed, kept");
    assert!(!names.iter().any(|n| n == "Ann Rich"), "an owner is not staff");
    assert!(w.origins.unresolved.iter().any(|u| u.what == "staff" && u.reason.contains("Elsewhere FC")));
    assert!(rep.findings.iter().any(|(k, _)| k == "board_role"));
    let free = w.staff.iter().find(|s| w.people[s.person].display_name(&w.names) == "Una Free").unwrap();
    assert!(free.club.is_none());
    assert_eq!(w.nations[w.people[free.person].nation].name, "Brazil");
}

#[test]
fn save_and_load_keep_provenance_and_the_world_still_runs() {
    let (w, _) = load(&Archive::standard(), "save");
    let path = std::env::temp_dir().join(format!("pw-archive-save-{}.pws", std::process::id()));
    pw_sim::save::save(&w, &path).unwrap();
    let back: pw_world::World = pw_sim::save::load(&path).unwrap();
    assert_eq!(back.origins.people.len(), w.origins.people.len());
    assert_eq!(back.origins.clubs.len(), 12);
    assert_eq!(back.origins.by_ref.len(), w.origins.by_ref.len());
    assert!(back.origins.find("transfermarkt", "player:10000").is_some());
    let mut sim = pw_sim::Sim::new(back);
    sim.run(3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn imported_world_advances_stays_audit_clean_and_a_player_can_be_inhabited() {
    let (mut w, _) = load(&Archive::standard(), "sim");
    let p = w.origins.find("transfermarkt", "player:10005").unwrap();
    assert!(!w.people[p].is_external(), "imported players start under the game's own mind");
    assert!(w.take_control(p));
    let mut sim = pw_sim::Sim::new(w);
    sim.run(200);
    let w = &sim.world;
    assert!(w.people[p].is_external(), "inhabiting is the only change of who decides");
    assert!(w.fixtures.iter().any(|f| f.1.score.is_some()), "matches were played in the imported competitions");
    let v = pw_sim::audit::audit(w);
    assert!(v.is_empty(), "audit violations after the imported world ran: {v:?}");
    // The rules bind imported people like any other: nobody in a squad is banned from being registered twice.
    let mut seen = std::collections::HashSet::new();
    for t in w.teams.iter() {
        for &pl in &t.squad {
            assert!(seen.insert(pl), "player in two squads");
        }
    }
    assert!(w.origins.person(p).is_some(), "provenance survives the simulation");
}

#[test]
fn parsed_set_can_be_adjusted_before_building() {
    let a = Archive::standard();
    let mut set = parse_dir(&a.write("adjust"), LoadOptions::default()).unwrap();
    set.players.truncate(100);
    let (w, rep) = build_world(&set, DataPack::builtin(), Some(1));
    assert_eq!(rep.players, 100);
    assert_eq!(w.players.len(), 100);
    assert_eq!(set.issues.total(Severity::Error), 0);
}
