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

fn inhabit_one(api: &Api) -> u32 {
    new_world(api, "small");
    advance(api, 60);
    let me = pick_player(api);
    api.call("persp.inhabit", json!({"person": me})).unwrap();
    me
}

#[test]
fn the_inhabited_pages_all_answer() {
    let api = api();
    let me = inhabit_one(&api);
    for m in ["me.today", "me.messages", "me.options", "me.self", "me.life", "me.people", "me.promises", "me.rumours", "me.press", "me.agent", "me.journal", "me.contract", "me.football"] {
        api.call(m, json!({})).unwrap_or_else(|e| panic!("{m}: {e:?}"));
    }
    let inbox = api.call("me.messages", json!({})).unwrap();
    assert!(inbox["messages"].is_array());
    // Every listed message opens, whichever kind it is.
    for m in inbox["messages"].as_array().unwrap().iter().take(40) {
        let id = m["id"].as_str().unwrap();
        api.call("me.message", json!({"id": id})).unwrap_or_else(|e| panic!("message {id}: {e:?}"));
    }
    let life = api.call("me.life", json!({})).unwrap();
    assert!(life["routine"].is_object());
    // While inhabiting, only your own life is readable; someone else's stays private.
    api.call("person.life", json!({"id": me})).unwrap();
    assert_eq!(api.call("person.life", json!({"id": me + 1})).unwrap_err().code(), "state");
    api.call("club.systems", json!({"id": 3})).unwrap();
    api.call("persp.observe", json!({})).unwrap();
    api.call("person.life", json!({"id": me + 1})).unwrap();
}

#[test]
fn new_system_tables_answer() {
    let api = api();
    new_world(&api, "small");
    advance(&api, 120);
    for (name, filters) in [
        ("stories", json!({})),
        ("agents", json!({})),
        ("talks", json!({})),
        ("bids", json!({})),
        ("intl_matches", json!({})),
        ("tournaments", json!({})),
        ("boards", json!({})),
        ("sponsors", json!({})),
        ("posts", json!({})),
        ("chants", json!({})),
        ("memes", json!({})),
        ("groups", json!({})),
        ("rivalries", json!({})),
        ("incidents", json!({})),
        ("conferences", json!({})),
        ("quotes", json!({})),
        ("referees", json!({})),
        ("controversies", json!({})),
        ("charges", json!({})),
        ("record_book", json!({})),
        ("records_broken", json!({})),
        ("votes", json!({})),
        ("hall_members", json!({})),
        ("chronicle", json!({})),
        ("schools", json!({})),
        ("rule_changes", json!({})),
        ("institutions", json!({})),
        ("minor_seasons", json!({})),
        ("outlets", json!({})),
        ("journalists", json!({})),
        ("grapevine", json!({})),
    ] {
        let t = table(&api, name, filters, 20);
        assert!(t["columns"].as_array().is_some_and(|c| !c.is_empty()), "{name} has columns");
        assert!(t["total"].is_number(), "{name} reports a total");
        // Every table also answers, differently, once somebody is inhabited.
    }
    let mut lively = 0;
    for name in ["posts", "chants", "groups", "rivalries", "incidents", "conferences", "quotes", "referees", "record_book", "institutions", "outlets", "journalists", "grapevine"] {
        if table(&api, name, json!({}), 5)["total"].as_u64().unwrap_or(0) > 0 {
            lively += 1;
        }
    }
    assert!(lively >= 8, "only {lively} of the new systems have anything in them after four months");
    // Sorting and searching work on them like on any table.
    let posts = table(&api, "posts", json!({"q": "a"}), 5);
    assert!(posts["rows"].is_array());
    // Once inhabiting, what the world hides stays hidden: the grapevine is for observers alone.
    let me = pick_player(&api);
    api.call("persp.inhabit", json!({"person": me})).unwrap();
    assert_eq!(table(&api, "grapevine", json!({}), 5)["total"], 0);
    for name in ["posts", "incidents", "referees", "controversies", "outlets"] {
        let t = table(&api, name, json!({}), 5);
        assert!(t["all_columns"].as_array().unwrap().iter().all(|c| c["presets"].as_array().unwrap().iter().all(|p| p != "internal")), "{name} leaks internal columns to an inhabited view");
    }
}

#[test]
fn actions_are_checked_queued_and_applied_by_the_world() {
    let api = api();
    let _ = inhabit_one(&api);
    let err = |v: Value| api.call("me.act", v).unwrap_err().code();
    assert_eq!(err(json!({"action": "nonsense"})), "bad_request");
    assert_eq!(err(json!({"action": "meet"})), "bad_request");

    let opts = api.call("me.options", json!({})).unwrap();
    assert!(opts["routine_budget"].as_u64().unwrap() > 0);
    let manager = opts["meet_with"].as_array().unwrap().iter().find(|t| t["role"] == "Manager").expect("a first-team player has a manager");
    let mgr = manager["who"]["id"].as_u64().unwrap();
    assert_eq!(err(json!({"action": "meet", "with": mgr, "topic": "not a topic"})), "bad_request");

    let queued = api.call("me.act", json!({"action": "meet", "with": mgr, "topic": "playing_time", "tone": "calm"})).unwrap();
    assert_eq!(queued["ok"], true);
    assert_eq!(queued["applies"], "next day");
    // Asking twice while the first is still waiting is refused, with a reason.
    assert_eq!(err(json!({"action": "meet", "with": mgr, "topic": "playing_time"})), "state");

    let waiting = |api: &Api| api.call("me.today", json!({})).unwrap()["waiting_on"].as_array().unwrap().len();
    assert!(waiting(&api) >= 1, "the request is visible while it waits");
    advance(&api, 1);
    // The intent has been taken up by the world; only the meeting itself may still be waiting.
    let after = api.call("me.today", json!({})).unwrap();
    assert!(after["waiting_on"].as_array().unwrap().iter().all(|w| w["kind"] != "intent"), "{}", after["waiting_on"]);

    // The routine planner respects the budget and reports the change as queued.
    let r = api.call("me.act", json!({"action": "routine", "hours": {"rest": 56}})).unwrap();
    assert_eq!(r["ok"], true);
}

#[test]
fn acting_needs_somebody_to_act_for() {
    let api = api();
    new_world(&api, "tiny");
    let e = api.call("me.act", json!({"action": "retire"})).unwrap_err();
    assert_eq!(e.code(), "state");
    assert_eq!(api.call("me.options", json!({})).unwrap_err().code(), "state");
    assert_eq!(api.call("me.messages", json!({})).unwrap_err().code(), "state");
}

#[test]
fn journal_notes_and_goals_persist_with_the_save() {
    let api = api();
    let _ = inhabit_one(&api);
    api.call("me.goal", json!({"text": "Reach fifty appearances", "kind": "appearances", "target": 50})).unwrap();
    api.call("me.note", json!({"text": "Ask about the captaincy"})).unwrap();
    let j = api.call("me.journal", json!({})).unwrap();
    assert_eq!(j["goals"].as_array().unwrap().len(), 1);
    assert_eq!(j["notes"].as_array().unwrap().len(), 1);
    advance(&api, 3);
    api.call("world.save", json!({"file": "career"})).unwrap();
    api.call("persp.observe", json!({})).unwrap();
    api.call("world.load", json!({"file": "career.pws"})).unwrap();
    wait_task(&api);
    let today = api.call("me.today", json!({})).unwrap();
    assert!(today["me"]["person"].is_number(), "the inhabited person is restored");
    let j = api.call("me.journal", json!({})).unwrap();
    assert_eq!(j["goals"].as_array().unwrap().len(), 1, "goals survive save and load");
    assert_eq!(j["notes"].as_array().unwrap().len(), 1, "notes survive save and load");
    api.call("me.note_remove", json!({"i": 0})).unwrap();
    assert!(api.call("me.journal", json!({})).unwrap()["notes"].as_array().unwrap().is_empty());
}

#[test]
fn a_new_person_can_be_created_and_inhabited() {
    let api = api();
    new_world(&api, "small");
    advance(&api, 10);
    let bad = api.call("person.create", json!({"first": "Ada", "last": "", "pos": "ST"})).unwrap_err();
    assert_eq!(bad.code(), "bad_request");
    let r = api.call("person.create", json!({"first": "Ada", "last": "Test", "pos": "ST", "age": 16, "nation": 0})).unwrap();
    let id = r["person"].as_u64().unwrap();
    let today = api.call("me.today", json!({})).unwrap();
    assert_eq!(today["me"]["person"].as_u64().unwrap(), id);
    advance(&api, 30);
    api.call("me.self", json!({})).unwrap();
}

#[test]
fn decisions_open_and_can_be_answered() {
    let api = api();
    let _ = inhabit_one(&api);
    let mut opened = 0;
    for _ in 0..40 {
        advance(&api, 15);
        let inbox = api.call("me.messages", json!({})).unwrap();
        for m in inbox["messages"].as_array().unwrap().iter().filter(|m| m["kind"] == "decision" && m["needs_action"] == true) {
            let id = m["id"].as_str().unwrap();
            let d = api.call("me.message", json!({"id": id})).unwrap();
            let options = d["options"].as_array().expect("a decision lists its options");
            assert!(!options.is_empty(), "{id} has options");
            assert!(options.iter().all(|o| o["label"].is_string() && o["kind"].is_string()), "{d}");
            api.call("me.answer", json!({"id": id, "choice": 0})).unwrap();
            // Until the world's next day you may still change your mind; afterwards it is settled.
            api.call("me.answer", json!({"id": id, "choice": (options.len() - 1) as u64})).unwrap();
            api.call("me.answer", json!({"id": id, "choice": options.len() as u64})).unwrap_err();
            advance(&api, 1);
            assert_eq!(api.call("me.answer", json!({"id": id, "choice": 0})).unwrap_err().code(), "state");
            opened += 1;
            break;
        }
        if opened >= 3 {
            break;
        }
    }
    assert!(opened >= 1, "no decision reached the inbox in ten simulated months");
}

#[test]
fn the_world_inbox_groups_conversations_and_replies_become_actions() {
    let api = api();
    let _ = inhabit_one(&api);
    let mut replied = 0;
    let mut opened_threads = 0;
    for _ in 0..40 {
        advance(&api, 15);
        let inbox = api.call("me.inbox", json!({})).unwrap();
        for t in inbox["threads"].as_array().unwrap() {
            assert!(t["title"].is_string() && t["preview"].is_string(), "{t}");
            let id = t["id"].as_u64().unwrap();
            let th = api.call("me.thread", json!({"id": id})).unwrap();
            opened_threads += 1;
            let msgs = th["messages"].as_array().unwrap();
            assert_eq!(msgs.len() as u64, t["count"].as_u64().unwrap());
            for m in msgs {
                assert!(m["text"].as_str().is_some_and(|s| !s.is_empty()) || m["kind"] == "meeting", "{m}");
                if replied == 0 {
                    if let Some(r) = m["replies"].as_array().and_then(|r| r.first()) {
                        let key = r["key"].as_str().unwrap();
                        api.call("me.reply", json!({"message": m["id"], "key": key})).unwrap();
                        // Once replied, it is recorded and cannot be answered twice.
                        assert_eq!(api.call("me.reply", json!({"message": m["id"], "key": key})).unwrap_err().code(), "state");
                        let again = api.call("me.thread", json!({"id": id})).unwrap();
                        let mm = again["messages"].as_array().unwrap().iter().find(|x| x["id"] == m["id"]).unwrap();
                        assert!(mm["replied"]["label"].is_string(), "{mm}");
                        replied += 1;
                    }
                }
            }
            if t["unread"].as_u64().unwrap() > 0 {
                api.call("me.thread_read", json!({"id": id})).unwrap();
                let after = api.call("me.thread", json!({"id": id})).unwrap();
                assert!(after["messages"].as_array().unwrap().iter().all(|m| m["read"] == true));
            }
        }
        if replied > 0 && opened_threads > 5 {
            break;
        }
    }
    assert!(opened_threads > 0, "no conversation reached the inbox in ten simulated months");
    assert!(replied > 0, "nothing in {opened_threads} conversations could be replied to");
    // A reply that is not on offer, or for somebody else's message, is refused.
    api.call("me.reply", json!({"message": 0, "key": "nonsense"})).unwrap_err();
    api.call("me.thread", json!({"id": 999_999})).unwrap_err();
}
