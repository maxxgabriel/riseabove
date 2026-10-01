//! The career chronicle: a life on the India route, lived through the API, becomes a dated story whose every line points at
//! something real, reads as its declared type, and survives a save and a reload unchanged.

use std::time::{Duration, Instant};

use pw_view::Api;
use serde_json::{Value, json};

fn api() -> Api {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    Api::new(std::env::temp_dir().join(format!("pw-view-chronicle-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))))
}

fn wait(api: &Api, what: &str) {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s[what]["running"].as_bool().unwrap_or(false) {
            assert!(s["task"]["error"].is_null(), "task failed: {}", s["task"]["error"]);
            return;
        }
        assert!(t0.elapsed() < Duration::from_secs(600), "{what} timed out");
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn text(e: &Value) -> String {
    e["parts"].as_array().unwrap().iter().map(|p| p["t"].as_str().unwrap_or("")).collect()
}

/// A world on the India route, someone begun on it, and `days` lived.
fn lived(seed: u64, days: u32) -> Api {
    let api = api();
    api.call("world.new", json!({"kind": "india", "scale": "tiny", "seed": seed})).unwrap();
    wait(&api, "task");
    let o = api.call("route.options", json!({})).unwrap();
    let district = o["states"][0]["districts"][0]["id"].clone();
    let begun = ["state_league", "university_freshman", "school_standout"].iter().any(|s| api.call("route.begin", json!({"start": s, "district": district})).is_ok());
    assert!(begun, "no start could be begun");
    advance(&api, days);
    api
}

/// Let `days` pass, through the stops a waiting decision or a match would make (the defaults answer what is not answered).
fn advance(api: &Api, days: u32) {
    api.call("settings.set", json!({"stops": {"decisions": false, "matches": false, "major": false}})).unwrap();
    let date = |api: &Api| api.call("world.status", json!({})).unwrap()["date"].as_i64().unwrap();
    let end = date(api) + i64::from(days);
    for _ in 0..50 {
        let left = end - date(api);
        if left <= 0 {
            return;
        }
        api.call("advance.start", json!({"mode": "days", "n": left})).unwrap();
        wait(api, "job");
    }
    panic!("the days would not pass");
}

#[test]
fn a_lived_career_becomes_a_linked_dated_story() {
    let api = lived(21, 300);
    let today = api.call("world.status", json!({})).unwrap()["date"].as_i64().unwrap();
    let ch = api.call("me.chronicle", json!({})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.chronicle", &ch) {
        panic!("the chronicle is not its declared type: {e}");
    }
    let entries = ch["entries"].as_array().unwrap();
    assert!(entries.len() >= 3, "a season of a life leaves more than this: {ch}");
    let mut last = i64::MIN;
    for e in entries {
        let d = e["date"].as_i64().unwrap();
        assert!(d >= last, "the story is told in order");
        assert!(d <= today, "nothing is told before it happens: {e}");
        last = d;
        let t = text(e);
        assert!(!t.trim().is_empty(), "every line says something: {e}");
        assert!(!t.contains("undefined") && !t.contains("NaN") && !t.contains("[object") && !t.contains("  "), "clean text: {t}");
        assert!(t.starts_with(|c: char| c.is_uppercase() || c.is_ascii_digit()), "a line is a sentence: {t}");
        if let Some(l) = e["learned"].as_i64() {
            assert!(l > d && l <= today, "a thing learned later is learned after it happened: {e}");
        }
        // Every link points at something the API can open.
        for p in e["parts"].as_array().unwrap() {
            let Some(r) = p.get("r") else { continue };
            let id = r["id"].as_u64().unwrap();
            let opened = match r["k"].as_str().unwrap() {
                "person" => api.call("person", json!({"id": id})).is_ok(),
                "club" => api.call("club", json!({"id": id})).is_ok(),
                "comp" => api.call("comp", json!({"id": id})).is_ok(),
                "nation" => api.call("nation", json!({"id": id})).is_ok(),
                other => panic!("unexpected link kind {other}"),
            };
            assert!(opened, "the link in \"{t}\" opens: {r}");
        }
        if let Some(story) = e["story"].as_u64() {
            assert!(api.call("news.story", json!({"id": story})).is_ok() || e["cat"] == "recognition", "a press line names a story");
        }
    }
    // Where the story starts is told, and the person is among nobody's "people from your past".
    let me = ch["person"]["id"].clone();
    assert!(ch["people"].as_array().unwrap().iter().all(|p| p["who"]["id"] != me));
    assert_eq!(ch["reach"].as_array().unwrap().len(), 3, "local, national and abroad");
    assert!(entries.iter().any(|e| e["cat"] == "moves" || e["cat"] == "people"), "how the career began is in it: {ch}");

    // A save and a reload keep it as it was, and the days after add to it rather than starting over.
    api.call("world.save", json!({"file": "chronicle-test"})).unwrap();
    api.call("world.load", json!({"file": "chronicle-test.pws"})).unwrap();
    wait(&api, "task");
    let again = api.call("me.chronicle", json!({})).unwrap();
    assert_eq!(again["entries"], ch["entries"], "the chronicle survives a reload unchanged");
    advance(&api, 60);
    let later = api.call("me.chronicle", json!({})).unwrap();
    let (a, b) = (ch["entries"].as_array().unwrap(), later["entries"].as_array().unwrap());
    assert!(b.len() >= a.len() && b[..a.len()] == a[..], "the story grows; what was told stays told");
}

#[test]
fn two_runs_of_the_same_world_tell_the_same_story() {
    let a = lived(8, 120).call("me.chronicle", json!({})).unwrap();
    let b = lived(8, 120).call("me.chronicle", json!({})).unwrap();
    assert_eq!(a["entries"], b["entries"]);
    assert_eq!(a["people"], b["people"]);
}

#[test]
fn an_observer_has_no_chronicle_of_their_own() {
    let api = api();
    api.call("world.new", json!({"kind": "synthetic", "scale": "tiny"})).unwrap();
    wait(&api, "task");
    assert!(api.call("me.chronicle", json!({})).is_err());
}

#[test]
fn the_squad_and_the_people_close_to_you_write_and_their_words_stay_put() {
    let api = lived(11, 240);
    let rooms = api.call("me.chats", json!({})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.chats", &rooms) {
        panic!("me.chats is not its declared type: {e}");
    }
    let rooms = rooms["rooms"].as_array().unwrap().clone();
    assert!(!rooms.is_empty(), "eight months at a club and nobody has written");
    assert!(rooms.iter().any(|r| r["kind"] == "squad"), "the squad has a group: {rooms:?}");
    let mut before = Vec::new();
    for r in &rooms {
        let chat = api.call("me.chat", json!({"id": r["id"]})).unwrap();
        if let Some(Err(e)) = pw_view::contract_pages::check_response("me.chat", &chat) {
            panic!("me.chat is not its declared type: {e}");
        }
        let lines = chat["lines"].as_array().unwrap();
        assert!(!lines.is_empty());
        for l in lines {
            let t = text(&json!({"parts": l["text"]}));
            assert!(!t.trim().is_empty() && !t.contains('{') && !t.contains('}') && !t.contains("  "), "a clean message: {t}");
            assert!(!l["from"].as_str().unwrap().is_empty());
        }
        before.push((r["id"].clone(), lines.clone()));
    }
    // Reading a chat clears its unread count.
    let first = &rooms[0];
    api.call("me.chat_read", json!({"id": first["id"]})).unwrap();
    let after = api.call("me.chats", json!({})).unwrap();
    assert!(after["rooms"].as_array().unwrap().iter().any(|r| r["id"] == first["id"] && r["unread"] == 0));
    // What was said stays said, in the same words, after a reload and as more is said.
    api.call("world.save", json!({"file": "chats-test"})).unwrap();
    api.call("world.load", json!({"file": "chats-test.pws"})).unwrap();
    wait(&api, "task");
    advance(&api, 30);
    for (id, lines) in before {
        let now = api.call("me.chat", json!({"id": id})).unwrap();
        let now = now["lines"].as_array().unwrap();
        assert!(now.len() >= lines.len().min(200));
        if now.len() == lines.len() || lines.len() < 200 {
            assert_eq!(now[..lines.len()], lines[..], "messages keep their words");
        }
    }
}
