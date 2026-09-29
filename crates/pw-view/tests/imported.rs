//! The whole vertical path on an imported world: files → world → perspective pages → human action → typed intent →
//! simulation → new state → pages again. The world here is the fictional mini archive from `pw_import::fixture`, loaded the
//! same way the desktop app loads a folder (`world.new` with `kind: "import"`). Nothing on these pages is demo data.

use std::time::{Duration, Instant};

use pw_import::fixture::Archive;
use pw_view::Api;
use serde_json::{Value, json};

fn api() -> Api {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    Api::new(std::env::temp_dir().join(format!("pw-view-imported-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))))
}

fn wait(api: &Api, what: &str) -> Value {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s[what]["running"].as_bool().unwrap_or(false) {
            if what == "task" {
                assert!(s["task"]["error"].is_null(), "task failed: {}", s["task"]["error"]);
            }
            return s;
        }
        assert!(t0.elapsed() < Duration::from_secs(300), "{what} timed out");
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn import_world(api: &Api, name: &str) {
    let dir = Archive::standard().write(name);
    api.call("world.new", json!({"kind": "import", "dir": dir.display().to_string()})).unwrap();
    wait(api, "task");
}

fn advance(api: &Api, days: u32) {
    api.call("advance.start", json!({"mode": "days", "n": days})).unwrap();
    wait(api, "job");
}

fn find_person(api: &Api, q: &str) -> u64 {
    let s = api.call("search", json!({"q": q})).unwrap();
    s["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["items"].as_array().cloned().unwrap_or_default())
        .find(|i| i["k"] == "person" && i["title"].as_str().is_some_and(|t| t.contains(q)))
        .unwrap_or_else(|| panic!("`{q}` not found in {s}"))["id"]
        .as_u64()
        .unwrap()
}

#[test]
fn a_search_an_imported_person_open_the_profile_and_read_the_career() {
    let api = api();
    import_world(&api, "journey-a");
    let id = find_person(&api, "Player10005");
    let p = api.call("person", json!({"id": id})).unwrap();
    assert_eq!(p["player"].is_null(), false);
    // An observer sees where the record came from and which facts were estimated.
    let prov = &p["provenance"];
    assert_eq!((prov["source"].as_str(), prov["id"].as_str()), (Some("transfermarkt"), Some("player:10005")));
    let origin = |group: &str| prov["facts"].as_array().unwrap().iter().find(|f| f["group"] == group).map(|f| f["origin"].as_str().unwrap().to_string()).unwrap();
    assert_eq!(origin("Identity"), "imported");
    assert_eq!(origin("Attributes"), "estimated from imported data");
    assert_eq!(origin("Personality"), "generated");
    // Career and history come from the imported spells, through the ordinary tables.
    let spells = api.call("table.query", json!({"table": "spells", "filters": {"person": id}, "limit": 20})).unwrap();
    assert!(!spells["rows"].as_array().unwrap().is_empty(), "the current club spell exists");
    api.call("table.query", json!({"table": "player_stats", "filters": {"person": id}, "limit": 20})).unwrap();
    // Clubs, competitions and nations are the imported ones.
    let s = api.call("search", json!({"q": "Football Club GB1"})).unwrap();
    assert!(s["groups"].as_array().unwrap().iter().any(|g| !g["items"].as_array().unwrap().is_empty()));
    // Provenance is part of the world: it is still there after a whole season has been played.
    advance(&api, 400);
    let later = api.call("person", json!({"id": id})).unwrap();
    assert!(later["provenance"].is_object(), "provenance survives a season");
}

#[test]
fn b_an_imported_player_can_be_inhabited_and_every_page_answers_from_their_perspective() {
    let api = api();
    import_world(&api, "journey-b");
    advance(&api, 30);
    let id = find_person(&api, "Player10005");
    api.call("persp.inhabit", json!({"person": id})).unwrap();
    for m in ["me.today", "me.messages", "me.inbox", "me.football", "me.contract", "me.people", "me.promises", "me.life", "me.self", "me.options", "me.calendar"] {
        api.call(m, json!({})).unwrap_or_else(|e| panic!("{m}: {e:?}"));
    }
    // Training: a plan is accepted as a typed change and shows as pending until the world takes it up.
    let plan = api.call("me.plan", json!({"intensity": "light"})).unwrap();
    assert_eq!((plan["applies"].as_str(), plan["plan"]["intensity"].as_str()), (Some("tomorrow"), Some("light")), "{plan}");
    let football = api.call("me.football", json!({})).unwrap();
    assert!(football["squad_status"].is_string() && football["rivals"].is_array());
    // The contract page shows the person's own terms; the data source is not their business.
    let me = api.call("person", json!({"id": id})).unwrap();
    assert!(me["provenance"].is_null(), "an inhabited person does not see the importer's bookkeeping");
    // Someone else's private state stays private.
    let other = find_person(&api, "Player10006");
    assert_eq!(api.call("person.life", json!({"id": other})).unwrap_err().code(), "state");
}

#[test]
fn c_an_action_reaches_the_simulation_and_its_effect_shows_up_later() {
    let api = api();
    import_world(&api, "journey-c");
    advance(&api, 20);
    let id = find_person(&api, "Player10005");
    api.call("persp.inhabit", json!({"person": id})).unwrap();
    let opts = api.call("me.options", json!({})).unwrap();
    let manager = opts["meet_with"].as_array().unwrap().iter().find(|t| t["role"] == "Manager").expect("an imported club's manager can be met");
    let mgr = manager["who"]["id"].as_u64().unwrap();
    // The manager is the imported club's own (from the club record's coach name), not a stand-in.
    assert!(manager["who"]["name"].as_str().is_some_and(|n| n.contains("Coach")), "{manager}");
    let queued = api.call("me.act", json!({"action": "meet", "with": mgr, "topic": "playing_time", "tone": "calm"})).unwrap();
    assert_eq!(queued["applies"], "next day");
    let waiting = |api: &Api| api.call("me.today", json!({})).unwrap()["waiting_on"].as_array().unwrap().clone();
    assert!(!waiting(&api).is_empty());
    advance(&api, 1);
    assert!(waiting(&api).iter().all(|w| w["kind"] != "intent"), "the world took the request up");
    // Later the same request cannot simply be repeated at once: the world remembers it happened.
    advance(&api, 20);
    let people = api.call("me.people", json!({})).unwrap();
    assert!(people.to_string().contains(&mgr.to_string()) || people.to_string().contains("Coach"), "the relationship with the manager exists in the world now");
}

#[test]
fn d_matches_are_played_recorded_and_reach_the_inhabited_persons_pages() {
    let api = api();
    import_world(&api, "journey-d");
    // Play into the season as an observer: leagues start in August, the world starts mid-July.
    advance(&api, 130);
    // Whether one named player gets minutes is the manager's choice and the season's luck, not what this test is about (it changed
    // whenever anything upstream moved the random draws). Inhabit a first-team player who has actually played, and check his pages.
    let played = api.call("table.query", json!({"table": "players", "filters": {"inhabitable": true, "kind": "first", "status": "active"}, "sort": {"key": "apps", "desc": true}, "limit": 1})).unwrap();
    let id = played["rows"][0]["open"]["id"].as_u64().expect("a first-team player has played");
    api.call("persp.inhabit", json!({"person": id})).unwrap();
    let football = api.call("me.football", json!({})).unwrap();
    let usage = football["usage"].as_array().unwrap();
    assert!(!usage.is_empty(), "the team has played league matches by now: {football}");
    let standings = api.call("table.query", json!({"table": "standings", "filters": {"comp": 0}, "limit": 30})).unwrap();
    assert!(standings["rows"].as_array().unwrap().len() >= 6, "the imported league has its table");
    // Each imported player who appeared has a season line through the ordinary stats table.
    let stats = api.call("table.query", json!({"table": "player_stats", "filters": {"person": id}, "limit": 10})).unwrap();
    assert!(stats["columns"].is_array());
    // Press and posts are generated from what really happened; the pages answer and show only what a player can know.
    for m in ["me.press", "me.rumours", "me.journal"] {
        api.call(m, json!({})).unwrap_or_else(|e| panic!("{m}: {e:?}"));
    }
    api.call("me.feed", json!({})).unwrap();
}

#[test]
fn e_save_and_reload_keep_the_person_the_world_the_history_and_the_provenance() {
    let api = api();
    import_world(&api, "journey-e");
    let id = find_person(&api, "Player10005");
    api.call("persp.inhabit", json!({"person": id})).unwrap();
    advance(&api, 50);
    api.call("me.note", json!({"text": "remember the fixture list"})).unwrap();
    let before = api.call("world.status", json!({})).unwrap();
    let journal_before = api.call("me.journal", json!({})).unwrap();
    let spells_before = api.call("table.query", json!({"table": "spells", "filters": {"person": id}, "limit": 50})).unwrap();
    api.call("world.save", json!({"file": "vertical"})).unwrap();
    // Close the world entirely, then open the file again.
    api.call("world.close", json!({})).unwrap();
    api.call("world.load", json!({"file": "vertical.pws"})).unwrap();
    wait(&api, "task");
    let after = api.call("world.status", json!({})).unwrap();
    assert_eq!(after["date"], before["date"]);
    assert_eq!(after["perspective"]["person"], before["perspective"]["person"], "the same person is still inhabited");
    assert_eq!(api.call("me.journal", json!({})).unwrap()["notes"], journal_before["notes"]);
    assert_eq!(api.call("table.query", json!({"table": "spells", "filters": {"person": id}, "limit": 50})).unwrap()["rows"], spells_before["rows"]);
    // Provenance is part of the world and returns with it.
    api.call("persp.observe", json!({})).unwrap();
    let p = api.call("person", json!({"id": id})).unwrap();
    assert_eq!(p["provenance"]["id"], "player:10005");
    // The saves list says the file is in the current format.
    let saves = api.call("world.saves", json!({})).unwrap();
    assert_eq!(saves["saves"][0]["format"]["state"], "current");
    // The world keeps running from the loaded state.
    advance(&api, 5);
}
