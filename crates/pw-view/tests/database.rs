use std::time::{Duration, Instant};
use pw_import::fixture::Archive;
use pw_view::Api;
use serde_json::json;

#[test]
fn sources_are_connected_remembered_and_linked_only_by_the_correct_source_namespace() {
    let root = std::env::temp_dir().join(format!("pw-view-database-{}", std::process::id()));
    let source = Archive::standard().write("database-view");
    let api = Api::new(&root);
    api.call("world.new", json!({"kind": "import", "dir": source})).unwrap();
    let begin = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if s["task"]["running"] == false { assert!(s["task"]["error"].is_null(), "{s}"); break; }
        assert!(begin.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(15));
    }
    // The world opens in the public view, and source records are for the observer (debug) view only.
    assert!(api.call("database.query", json!({"table": "players.csv"})).is_err(), "the public view reads no source records");
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    let sources = api.call("database.sources", json!({})).unwrap();
    assert!(sources["sources"][0]["tables"].as_array().unwrap().iter().any(|t| t["name"] == "players.csv"));
    let p = api.call("database.query", json!({"table": "players.csv", "column": "player_id", "value": "10005"})).unwrap();
    assert_eq!(p["matched"], 1);
    let id = p["rows"][0]["refs"]["player_id"]["id"].as_u64().unwrap();
    assert_eq!(p["rows"][0]["refs"]["player_id"]["k"], "person");
    assert!(api.call("database.query", json!({"table": "../players.csv"})).is_err());
    api.call("persp.inhabit", json!({"person": id})).unwrap();
    assert!(matches!(api.call("database.query", json!({"table": "players.csv"})), Err(pw_view::ApiError::Unauthorized(_))));
    api.call("persp.observe", json!({"public": true})).unwrap();
    assert!(api.call("database.query", json!({"table": "players.csv"})).is_err());
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    let fm = root.join("fm/usable");
    std::fs::create_dir_all(&fm).unwrap();
    std::fs::write(fm.join("fm23_export_catalog.csv"), "file,status\npeople_names_usable.csv,name_only\n").unwrap();
    std::fs::write(fm.join("people_names_usable.csv"), "name,player_id,person_id_candidate\nSame Name,10005,10005\n").unwrap();
    let attached = api.call("database.attach", json!({"dir": root.join("fm")})).unwrap();
    let p = api.call("database.query", json!({"source": attached["id"], "table": "people_names_usable.csv"})).unwrap();
    assert!(p["rows"][0]["refs"].is_null(), "FM IDs are never Transfermarkt person IDs");
    // A consolidated folder can hold several providers. Determine the namespace
    // from the selected table, not from another table in that folder.
    std::fs::write(source.join("riseabove.database.json"), "{}").unwrap();
    std::fs::write(source.join("fm23_legacy_people.csv"), "name,player_id\nSame Name,10005\n").unwrap();
    api.call("database.attach", json!({"dir": source})).unwrap();
    let legacy = api.call("database.query", json!({"table": "fm23_legacy_people.csv"})).unwrap();
    assert!(legacy["rows"][0]["refs"].is_null(), "a legacy FM ID must not link to a TM player in a mixed folder");
    drop(api);
    let reopened = Api::new(&root);
    assert_eq!(reopened.call("database.sources", json!({})).unwrap()["sources"].as_array().unwrap().len(), 2);
    let p = reopened.call("database.query", json!({"table": "players.csv", "column": "player_id", "value": "10005"})).unwrap();
    assert_eq!(p["rows"][0]["fields"]["player_id"], "10005");
    assert_eq!(p, reopened.call("database.query", json!({"table": "players.csv", "column": "player_id", "value": "10005"})).unwrap(), "read-only responses repeat exactly");
    drop(reopened);
    std::fs::remove_dir_all(&root).unwrap();
}
