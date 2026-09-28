//! End-to-end checks of the client API against a real (synthetic) world.

use std::time::{Duration, Instant};

use pw_view::Api;
use serde_json::{Value, json};

fn api() -> Api {
    let dir = std::env::temp_dir().join(format!("pw-view-test-{}-{}", std::process::id(), rand_suffix()));
    Api::new(dir)
}

fn rand_suffix() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::SeqCst)
}

fn wait_task(api: &Api) {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s["task"]["running"].as_bool().unwrap_or(false) {
            assert!(s["task"]["error"].is_null(), "task failed: {}", s["task"]["error"]);
            return;
        }
        assert!(t0.elapsed() < Duration::from_secs(120), "task timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn wait_job(api: &Api) -> Value {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s["job"]["running"].as_bool().unwrap_or(false) {
            return s["job"].clone();
        }
        assert!(t0.elapsed() < Duration::from_secs(300), "advance timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn new_world(api: &Api, scale: &str) {
    api.call("world.new", json!({"kind": "synthetic", "scale": scale})).unwrap();
    wait_task(api);
}

fn advance(api: &Api, days: u32) -> Value {
    api.call("advance.start", json!({"mode": "days", "n": days})).unwrap();
    wait_job(api)
}

fn table(api: &Api, name: &str, filters: Value, limit: usize) -> Value {
    api.call("table.query", json!({"table": name, "filters": filters, "limit": limit})).unwrap()
}

#[test]
fn every_table_answers() {
    let api = api();
    new_world(&api, "small");
    advance(&api, 60);
    for (name, filters) in [
        ("players", json!({})),
        ("staff", json!({})),
        ("clubs", json!({})),
        ("nations", json!({})),
        ("comps", json!({})),
        ("standings", json!({"comp": 0})),
        ("fixtures", json!({"comp": 0})),
        ("comp_stats", json!({"comp": 0})),
        ("events", json!({})),
        ("transfers", json!({})),
        ("player_stats", json!({"person": 100})),
        ("spells", json!({"person": 100})),
        ("honours", json!({})),
        ("awards", json!({})),
    ] {
        let t = table(&api, name, filters, 20);
        assert!(t["columns"].as_array().is_some_and(|c| !c.is_empty()), "{name} has columns");
        for row in t["rows"].as_array().unwrap() {
            assert_eq!(row["cells"].as_array().unwrap().len(), t["columns"].as_array().unwrap().len(), "{name} row width");
        }
    }
    // Sorting is stable and honours direction.
    let asc = api.call("table.query", json!({"table": "players", "sort": {"key": "age", "desc": false}, "limit": 5})).unwrap();
    let desc = api.call("table.query", json!({"table": "players", "sort": {"key": "age", "desc": true}, "limit": 5})).unwrap();
    let age = |t: &Value, i: usize| t["rows"][i]["cells"][1]["n"].as_f64().unwrap();
    assert!(age(&asc, 0) <= age(&asc, 4));
    assert!(age(&desc, 0) >= age(&desc, 4));
    assert!(age(&desc, 0) > age(&asc, 0));
}

#[test]
fn observer_pages_and_search() {
    let api = api();
    new_world(&api, "small");
    advance(&api, 45);
    let p = api.call("person", json!({"id": 1008})).unwrap();
    assert!(!p["player"]["internal"].is_null(), "observer sees internal state");
    let s = api.call("search", json!({"q": "tosi"})).unwrap();
    assert!(!s["groups"].as_array().unwrap().is_empty());
    for m in ["overview", "diagnostics", "capabilities"] {
        api.call(m, json!({})).unwrap();
    }
    api.call("club", json!({"id": 3})).unwrap();
    api.call("comp", json!({"id": 0})).unwrap();
    api.call("nation", json!({"id": 0})).unwrap();
    let err = api.call("me.today", json!({})).unwrap_err();
    assert_eq!(err.code(), "state");
}

/// Find an active player at a mid-table club for inhabiting.
fn pick_player(api: &Api) -> u32 {
    let t = table(api, "players", json!({"inhabitable": true, "kind": "first", "status": "active"}), 200);
    let rows = t["rows"].as_array().unwrap();
    rows[rows.len() / 2]["open"]["id"].as_u64().unwrap() as u32
}

#[test]
fn inhabiting_limits_what_is_visible() {
    let api = api();
    new_world(&api, "small");
    advance(&api, 45);
    let me = pick_player(&api);
    api.call("persp.inhabit", json!({"person": me})).unwrap();

    let cols = table(&api, "players", json!({}), 5)["all_columns"].as_array().unwrap().iter().map(|c| c["key"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    for hidden in ["ca", "pa", "wage", "value", "contract_end", "condition"] {
        assert!(!cols.contains(&hidden.to_string()), "column {hidden} must be hidden while inhabiting");
    }
    let today = api.call("me.today", json!({})).unwrap();
    assert_eq!(today["me"]["person"].as_u64().unwrap() as u32, me);
    assert!(today["condition"]["condition"]["label"].is_string());

    // Another club's player: attributes are ranges or unknown, never internal values.
    let other = api.call("table.query", json!({"table": "players", "filters": {"kind": "first"}, "limit": 60, "offset": 1500})).unwrap();
    let mut saw_non_exact = false;
    for row in other["rows"].as_array().unwrap() {
        let id = row["open"]["id"].as_u64().unwrap();
        if id as u32 == me {
            continue;
        }
        let person = api.call("person", json!({"id": id})).unwrap();
        assert!(person["player"]["internal"].is_null(), "internal state leaked");
        assert!(person["player"]["contract"].is_null() || person["is_me"] == true, "contract leaked");
        assert!(person["player"]["condition"].is_null(), "condition leaked");
        let a = api.call("person.attributes", json!({"id": id})).unwrap();
        assert!(a["hidden"].is_null() && a["internal"].is_null(), "hidden attributes leaked");
        for g in a["groups"].as_array().unwrap() {
            for at in g["attrs"].as_array().unwrap() {
                if at["kind"] != "exact" {
                    saw_non_exact = true;
                }
            }
        }
    }
    assert!(saw_non_exact, "strangers should not be assessed exactly");

    // Own contract is visible.
    let c = api.call("me.contract", json!({})).unwrap();
    assert_eq!(c["has_contract"], true);
    // Observer mode restores the full view.
    api.call("persp.observe", json!({})).unwrap();
    let cols = table(&api, "players", json!({}), 5)["all_columns"].as_array().unwrap().len();
    assert!(cols > 20);
}

#[test]
fn recording_detail_never_changes_outcomes() {
    // Same world, same days; one run records every match of one club in full detail.
    let run = |follow: bool| -> String {
        let api = api();
        new_world(&api, "small");
        if follow {
            api.call("club.follow", json!({"club": 3, "follow": true})).unwrap();
        }
        advance(&api, 120);
        let t = table(&api, "standings", json!({"comp": 0}), 50);
        let f = table(&api, "fixtures", json!({"comp": 0, "played": true}), 500);
        let p = api.call("table.query", json!({"table": "players", "sort": {"key": "goals", "desc": true}, "limit": 30})).unwrap();
        format!("{}|{}|{}", t["rows"], f["rows"], p["rows"])
    };
    assert_eq!(run(false), run(true));
}

#[test]
fn results_stay_hidden_until_revealed() {
    let api = api();
    new_world(&api, "small");
    advance(&api, 20);
    let me = pick_player(&api);
    api.call("persp.inhabit", json!({"person": me})).unwrap();
    // Advance until one of our matches has been played.
    api.call("advance.start", json!({"mode": "until_match"})).unwrap();
    let job = wait_job(&api);
    assert_eq!(job["stop"]["kind"], "match", "{job}");
    let status = api.call("world.status", json!({})).unwrap();
    assert!(status["unrevealed"].as_u64().unwrap() >= 1, "our match should be waiting: {job}");

    // Find our concealed fixture.
    let f = api.call("table.query", json!({"table": "fixtures", "filters": {"mine": true, "played": true}, "limit": 50, "sort": {"key": "date", "desc": true}})).unwrap();
    let row = &f["rows"][0];
    let cols: Vec<&str> = f["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let score_ix = cols.iter().position(|c| *c == "score").unwrap();
    let cell = &row["cells"][score_ix];
    assert_eq!(cell["s"], "? – ?", "score must be masked: {cell}");
    let uid = row["open"]["id"].as_u64().unwrap();
    let m = api.call("match", json!({"uid": uid})).unwrap();
    assert!(m["score"].is_null() && m["detail"].is_null());
    // Watching returns the recording without revealing the result elsewhere.
    let w = api.call("match.watch", json!({"uid": uid})).unwrap();
    assert!(w["score"].is_object());
    assert!(w["detail"]["events"].is_array(), "our matches are recorded in full detail");
    let still = api.call("match", json!({"uid": uid})).unwrap();
    assert!(still["score"].is_null());
    api.call("match.reveal", json!({"uid": uid})).unwrap();
    let shown = api.call("match", json!({"uid": uid})).unwrap();
    assert!(shown["score"].is_object());
}

#[test]
fn save_and_load_round_trip() {
    let api = api();
    new_world(&api, "tiny");
    advance(&api, 10);
    let before = api.call("world.status", json!({})).unwrap()["date"].clone();
    api.call("world.save", json!({"file": "roundtrip"})).unwrap();
    let saves = api.call("world.saves", json!({})).unwrap();
    assert_eq!(saves["saves"].as_array().unwrap().len(), 1);
    advance(&api, 5);
    api.call("world.load", json!({"file": "roundtrip.pws"})).unwrap();
    wait_task(&api);
    assert_eq!(api.call("world.status", json!({})).unwrap()["date"], before);
    let bad = api.call("world.load", json!({"file": "missing.pws"}));
    assert!(bad.is_ok());
    let t0 = Instant::now();
    while api.call("world.status", json!({})).unwrap()["task"]["running"] == true && t0.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(10));
    }
    let s = api.call("world.status", json!({})).unwrap();
    assert!(s["task"]["error"].is_string(), "a missing save reports an error");
    assert_eq!(s["date"], before, "the open world is untouched by a failed load");
}

#[test]
fn stop_lands_on_a_day_boundary() {
    let api = api();
    new_world(&api, "small");
    api.call("advance.start", json!({"mode": "days", "n": 3000})).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    api.call("advance.stop", json!({})).unwrap();
    let job = wait_job(&api);
    assert_eq!(job["stop"]["kind"], "user");
    let done = job["days_done"].as_u64().unwrap();
    assert!(done > 0 && done < 3000);
    let date = api.call("world.status", json!({})).unwrap()["date"].as_i64().unwrap();
    assert_eq!(date, job["from"].as_i64().unwrap() + done as i64);
}
