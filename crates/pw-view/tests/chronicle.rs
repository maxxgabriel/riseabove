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
                "inst" => api.call("institution", json!({"id": id})).is_ok(),
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

#[test]
fn your_match_is_a_day_lived_and_a_hidden_result_stays_hidden() {
    let api = lived(4, 90);
    let today = api.call("me.today", json!({})).unwrap();
    let recent = today["recent"].as_array().unwrap().clone();
    assert!(!recent.is_empty(), "three months at a club and no match");
    let uid = recent[0]["uid"].clone();
    // The India world keeps your own results hidden until you choose to see them: the day stops at kick-off.
    let hidden = api.call("me.matchday", json!({"uid": uid})).unwrap();
    let said: String = hidden["steps"].as_array().unwrap().iter().flat_map(|s| s["parts"].as_array().unwrap().iter().map(|p| p["t"].as_str().unwrap_or("").to_string())).collect();
    assert!(!said.contains("Won ") && !said.contains("Lost ") && !said.contains("Drew ") && !said.contains("at the break"), "a hidden result is not told: {said}");
    api.call("match.reveal_all", json!({})).unwrap();
    let day = api.call("me.matchday", json!({"uid": uid})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.matchday", &day) {
        panic!("me.matchday is not its declared type: {e}");
    }
    let kinds: Vec<&str> = day["steps"].as_array().unwrap().iter().map(|s| s["kind"].as_str().unwrap()).collect();
    for k in ["squad", "half", "result"] {
        assert!(kinds.contains(&k), "a played match day has {k}: {kinds:?}");
    }
    for s in day["steps"].as_array().unwrap() {
        let t = text(s);
        assert!(!t.trim().is_empty() && !t.contains("  ") && !t.contains('{'), "clean: {t}");
    }
    // Someone else's match is not yours to relive.
    let other = api.call("table.query", json!({"table": "fixtures", "filters": {"played": true}, "limit": 50})).unwrap();
    let mine = today["me"]["club"]["name"].as_str().unwrap_or("").to_string();
    if let Some(f) = other["rows"].as_array().unwrap().iter().find(|r| !r.to_string().contains(&mine)) {
        assert!(api.call("me.matchday", json!({"uid": f["open"]["id"]})).is_err());
    }
}

#[test]
fn a_wage_becomes_payslips_bonuses_and_a_life() {
    let api = lived(4, 120);
    let m = api.call("me.money", json!({})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.money", &m) {
        panic!("me.money is not its declared type: {e}");
    }
    let months = m["months"].as_array().unwrap();
    assert!(months.len() >= 3, "four months of a contract leave payslips: {m}");
    for x in months {
        let n = |k: &str| x[k].as_i64().unwrap();
        assert!(n("wage") > 0 && n("tax") > 0, "a professional is paid and taxed: {x}");
        assert_eq!(n("net"), n("wage") + n("other") - n("tax"), "take-home is pay less tax: {x}");
        assert_eq!(n("left"), n("net") - n("living") - n("family"), "what is left is take-home less living and family: {x}");
    }
    for b in m["bonuses"].as_array().unwrap() {
        assert!(b["kept"].as_i64().unwrap() < b["gross"].as_i64().unwrap(), "tax and the agent take their share of a bonus: {b}");
    }
    assert!(!m["meaning"].as_array().unwrap().is_empty(), "the numbers are said in words");
    assert_eq!(api.call("world.status", json!({})).unwrap()["currency"], "₹", "an India world counts in rupees");
}

#[test]
fn a_club_is_a_place_and_your_club_has_a_mood() {
    let api = lived(4, 60);
    let today = api.call("me.today", json!({})).unwrap();
    let atmosphere = &today["atmosphere"];
    assert!(atmosphere["mood"].is_string(), "your club has a mood: {atmosphere}");
    let mine = today["me"]["club"]["id"].clone();
    let club = api.call("club", json!({"id": mine})).unwrap();
    let place = &club["place"];
    for k in ["region", "climate", "population", "football"] {
        assert!(place[k].as_str().is_some_and(|s| !s.is_empty()), "an India club has its {k}: {place}");
    }
    assert!(place["from_home"].is_string(), "how far from home, for the person you live as");
    assert!(!place["nearby"].as_array().unwrap().iter().any(|n| n["id"] == mine), "a club is not its own neighbour");
    // A club in a world without regions has no place to describe, and says nothing rather than guessing.
    let synth = api_synthetic();
    assert!(synth.call("club", json!({"id": 0})).unwrap()["place"].is_null());
}

fn api_synthetic() -> Api {
    let api = api();
    api.call("world.new", json!({"kind": "synthetic", "scale": "tiny"})).unwrap();
    wait(&api, "task");
    api
}

#[test]
fn the_training_ground_leaves_a_week_by_week_trace() {
    let api = lived(4, 60);
    let t = api.call("me.training", json!({})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.training", &t) {
        panic!("me.training is not its declared type: {e}");
    }
    let weeks = t["weeks"].as_array().unwrap();
    assert!(weeks.len() >= 7, "two months leave a week per Monday: {}", weeks.len());
    let dates: Vec<i64> = weeks.iter().map(|w| w["date"].as_i64().unwrap()).collect();
    assert!(dates.windows(2).all(|d| d[0] - d[1] == 7), "one line a week, newest first: {dates:?}");
    for w in weeks {
        for tr in w["traces"].as_array().unwrap() {
            let s = text(&json!({"parts": tr}));
            assert!(!s.trim().is_empty() && !s.contains("  "), "clean: {s}");
        }
    }
}

#[test]
fn today_brings_back_this_day_in_earlier_years_and_the_week_around_the_country() {
    let api = lived(23, 420);
    let t = api.call("me.today", json!({})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.today", &t) {
        panic!("today is not its declared type: {e}");
    }
    let today = t["date"].as_i64().unwrap();
    for d in t["on_this_day"].as_array().unwrap() {
        let years = d["years_ago"].as_i64().unwrap();
        assert!(years >= 1, "{d}");
        assert!(d["date"].as_i64().unwrap() < today);
        assert!(!text(d).trim().is_empty());
    }
    let kinds = ["breakout", "manager", "owner", "administration", "investment", "project", "record"];
    let around = t["around"].as_array().unwrap();
    assert!(around.len() <= 4);
    let mut seen = std::collections::HashSet::new();
    for a in around {
        let k = a["kind"].as_str().unwrap();
        assert!(kinds.contains(&k), "{a}");
        assert!(seen.insert(k.to_string()), "one per kind: {a}");
        assert!(today - a["date"].as_i64().unwrap() <= 7, "of the week: {a}");
        assert!(!text(a).trim().is_empty());
    }
    // A year on, the story has a line on this day only if something happened on it: the field is there either way.
    assert!(t["on_this_day"].is_array() && t["around"].is_array());
}

/// The `waiting_on` items of a kind on Today.
fn waits(api: &Api, kind: &str) -> Vec<Value> {
    let t = api.call("me.today", json!({})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.today", &t) {
        panic!("today is not its declared type: {e}");
    }
    t["waiting_on"].as_array().unwrap().iter().filter(|x| x["kind"] == kind).cloned().collect()
}

fn day(y: i32, m: u32, d: u32) -> i64 {
    i64::from(pw_core::Date::from_ymd(y, m, d).0)
}

#[test]
fn the_published_calendar_is_something_to_wait_for() {
    // The India world starts on 1 July 2026.
    let api = api();
    api.call("world.new", json!({"kind": "india", "scale": "tiny", "seed": 29})).unwrap();
    wait(&api, "task");
    let o = api.call("route.options", json!({})).unwrap();
    let district = o["states"][0]["districts"][0]["id"].clone();
    let begin = |start: &str| api.call("route.begin", json!({"start": start, "district": district})).unwrap();

    // Eighteen, released, free: the universities' recruiting week (the first Monday of September) is ahead, without an offer yet.
    begin("released_academy");
    let uni = waits(&api, "university");
    assert_eq!(uni.len(), 1, "{uni:?}");
    assert_eq!(uni[0]["date"].as_i64(), Some(day(2026, 9, 7)));
    assert!(uni[0]["since"].is_null(), "a calendar date was not set in motion by anyone: {}", uni[0]);
    let t = uni[0]["text"].as_str().unwrap();
    assert!(t.contains("7 September") && t.contains("in 10 weeks"), "{t}");
    assert!(waits(&api, "selection").is_empty(), "eighteen is past the district trials and the state squad is months away");

    // Sixteen at school: the district's open trials on 1 October, once they are near.
    begin("school_standout");
    assert!(waits(&api, "selection").is_empty(), "three months ahead is not yet worth a line");
    assert!(waits(&api, "university").is_empty(), "sixteen is too young for the universities");
    advance(&api, 40);
    let sel = waits(&api, "selection");
    assert_eq!(sel.len(), 1, "{sel:?}");
    assert_eq!(sel[0]["date"].as_i64(), Some(day(2026, 10, 1)));
    let t = sel[0]["text"].as_str().unwrap();
    assert!(t.contains("district side") && t.contains("1 October") && t.contains("in 7 weeks"), "{t}");

    // After the trials the item is gone; if the person went, the story says how it went.
    advance(&api, 60);
    assert!(waits(&api, "selection").iter().all(|x| x["date"].as_i64() != Some(day(2026, 10, 1))));
    let ch = api.call("me.chronicle", json!({})).unwrap();
    for e in ch["entries"].as_array().unwrap().iter().filter(|e| e["date"].as_i64() == Some(day(2026, 10, 1)) && e["cat"] == "international") {
        let t = text(e);
        assert!(t.starts_with("Went to the open trials for the ") || t.starts_with("Picked by the selectors of "), "{t}");
    }

    // Twenty, in the state league: the day the state selectors name their squad (1 February), within ten weeks of it.
    advance(&api, 45);
    begin("state_league");
    let sel = waits(&api, "selection");
    assert_eq!(sel.len(), 1, "{sel:?}");
    assert_eq!(sel[0]["date"].as_i64(), Some(day(2027, 2, 1)));
    let t = sel[0]["text"].as_str().unwrap();
    assert!(t.contains("state championship") && t.contains("1 February") && t.contains("in about 2 months"), "{t}");
}

/// Opens a link of any kind the story uses.
fn opens(api: &Api, r: &Value) -> bool {
    let id = r["id"].as_u64().unwrap();
    let m = match r["k"].as_str().unwrap() {
        "inst" => "institution",
        k => k,
    };
    api.call(m, json!({"id": id})).is_ok()
}

/// A table row's cell by column key.
fn cell<'a>(t: &Value, row: &'a Value, key: &str) -> &'a Value {
    let i = t["columns"].as_array().unwrap().iter().position(|c| c == key).unwrap_or_else(|| panic!("no column {key}"));
    &row["cells"][i]
}

#[test]
fn what_became_of_them_is_public_record_furthest_risen_first() {
    let api = lived(23, 420);
    let today = api.call("world.status", json!({})).unwrap()["date"].as_i64().unwrap();
    let ch = api.call("me.chronicle", json!({})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("me.chronicle", &ch) {
        panic!("the chronicle is not its declared type: {e}");
    }
    let became = ch["became"].as_array().unwrap();
    assert!(!became.is_empty(), "fourteen months on, someone from the start of the story has moved on: {}", ch["people"]);
    let me = ch["person"]["id"].clone();
    let mut keys = Vec::new();
    for b in became {
        assert_ne!(b["who"]["id"], me, "you are not among the people from your past");
        assert!(today - b["from"].as_i64().unwrap() >= 365, "only paths that crossed more than a year ago: {b}");
        for k in ["role", "summary"] {
            let s = b[k].as_str().unwrap();
            assert!(!s.trim().is_empty() && !s.contains("  "), "{k} says something: {b}");
        }
        assert!(opens(&api, &b["who"]), "{b}");
        for k in ["club", "league", "caps_for"] {
            if !b[k].is_null() {
                assert!(opens(&api, &b[k]), "{k} opens: {b}");
            }
        }
        // Missing is not zero: no caps is null, not 0.
        assert!(b["caps"].is_null() || b["caps"].as_u64().unwrap() > 0, "{b}");
        assert_eq!(b["caps"].is_null(), b["caps_for"].is_null(), "{b}");
        // How far they rose, from what anyone can see: the club's standing, a manager above a player, caps above both.
        let rep = if b["club"].is_null() { 0 } else { api.call("club", json!({"id": b["club"]["id"]})).unwrap()["reputation"].as_i64().unwrap() };
        let role = b["role"].as_str().unwrap();
        let mut key = rep;
        if !b["club"].is_null() && role != "Player" {
            key += if role == "Manager" { 2_500 } else { 500 };
        }
        if let Some(caps) = b["caps"].as_i64() {
            key += 3_000 + caps.min(100) * 40;
        }
        key += b["managed"].as_i64().unwrap() * 300;
        keys.push(key);
    }
    assert!(keys.windows(2).all(|k| k[0] >= k[1]), "furthest risen first: {keys:?}");
}

#[test]
fn keepsakes_are_documents_with_what_the_line_kept() {
    // This life signs a contract, wins medals and is written about in its first fourteen months.
    let api = lived(4, 420);
    let ch = api.call("me.chronicle", json!({})).unwrap();
    assert!(papers(&api, &ch) >= 3, "the contract, the medals and the clipping are documents: {}", ch["entries"]);
}

/// Checks the documents of a chronicle; returns how many there were.
fn papers(api: &Api, ch: &Value) -> usize {
    let mut n = 0;
    for e in ch["entries"].as_array().unwrap() {
        // The four keepsakes that are papers are documents of their own kind; no other line is one.
        let paper = e["keepsake"].as_str().filter(|k| ["contract", "call_up", "medal", "clipping"].contains(k));
        let d = &e["doc"];
        assert_eq!(paper, d["kind"].as_str(), "a contract, call-up, medal or clipping is a document of that kind, and nothing else is: {e}");
        if d.is_null() {
            continue;
        }
        n += 1;
        let kind = d["kind"].as_str().unwrap();
        match kind {
            "contract" => {
                assert_eq!(e["keepsake"], "contract");
                assert!(d["until"].as_i64().unwrap() > e["date"].as_i64().unwrap(), "a contract runs past the day it was signed: {e}");
                assert!(d["wage"].is_null() || d["wage"].as_i64().unwrap() > 0, "a wage is kept or not, never zero: {e}");
                assert!(d["years"].is_null() || d["years"].as_f64().unwrap() >= 0.5, "{e}");
            }
            "call_up" => {
                assert!(d["squad"].as_str().is_some_and(|s| !s.is_empty()), "a call-up names the squad: {e}");
                if let (Some(a), Some(b)) = (d["from"].as_i64(), d["to"].as_i64()) {
                    assert!(a <= b && b >= e["date"].as_i64().unwrap(), "the window is the one called up for: {e}");
                }
            }
            "medal" => {
                assert!(!d["comp"].is_null() || d["comp_name"].is_string(), "a medal names its competition: {e}");
                assert!(d["season"].is_string(), "and its season: {e}");
            }
            "clipping" => {
                assert!(d["headline"].as_str().is_some_and(|s| !s.is_empty()) && d["outlet"].as_str().is_some_and(|s| !s.is_empty()), "a clipping has its headline and outlet: {e}");
            }
            other => panic!("unexpected document kind {other}"),
        }
        for k in ["club", "nation", "comp"] {
            if !d[k].is_null() {
                assert!(opens(api, &d[k]), "{k} opens: {e}");
            }
        }
    }
    n
}

#[test]
fn an_institution_has_a_page_and_the_story_links_to_it() {
    let api = lived(21, 120);
    // The institutions list opens each one on its own page.
    let t = api.call("table.query", json!({"table": "institutions", "limit": 5})).unwrap();
    let rows = t["rows"].as_array().unwrap();
    assert!(!rows.is_empty(), "an India world has schools and universities");
    for row in rows {
        let name = cell(&t, row, "name");
        let r = &name["r"];
        assert_eq!(r["k"], "inst", "an institution's name links to its page: {row}");
        assert_eq!(row["open"], *r, "and the row opens it");
        let page = api.call("institution", json!({"id": r["id"]})).unwrap();
        if let Some(Err(e)) = pw_view::contract_pages::check_response("institution", &page) {
            panic!("institution is not its declared type: {e}");
        }
        assert_eq!(page["name"], name["s"]);
        for k in ["kind", "standing", "origin"] {
            assert!(page[k].as_str().is_some_and(|s| !s.is_empty()), "{k}: {page}");
        }
        assert!(page["founded"].is_null() || page["founded"].as_i64().unwrap() > 0, "an unknown founding year is not year zero");
        for p in page["players"].as_array().unwrap().iter().chain(page["alumni"].as_array().unwrap()) {
            assert!(opens(&api, &p["who"]), "{p}");
        }
        for x in page["titles"].as_array().unwrap() {
            assert!(x["finish"] == "won" || x["finish"] == "runner_up", "{x}");
        }
    }
    assert!(api.call("institution", json!({"id": 9_999_999})).is_err(), "an institution that does not exist is not found");
    // The universities near your club link to their pages.
    let today = api.call("me.today", json!({})).unwrap();
    let club = api.call("club", json!({"id": today["me"]["club"]["id"]})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("club", &club) {
        panic!("club is not its declared type: {e}");
    }
    for u in club["place"]["universities"].as_array().unwrap() {
        assert_eq!(u["k"], "inst");
        assert!(opens(&api, u), "{u}");
    }
    assert!(club["place"]["media"].is_array(), "a place has its local media, even if none: {}", club["place"]);
}

#[test]
fn a_club_abroad_is_a_country_to_live_in() {
    let api = lived(4, 30);
    let today = api.call("me.today", json!({})).unwrap();
    let mine = api.call("club", json!({"id": today["me"]["club"]["id"]})).unwrap();
    assert!(mine["country"].is_null(), "your own country is home, not abroad");
    let home = mine["nation"]["id"].clone();
    let mut abroad = None;
    let mut offset = 0;
    while abroad.is_none() {
        let t = api.call("table.query", json!({"table": "clubs", "limit": 500, "offset": offset})).unwrap();
        let rows = t["rows"].as_array().unwrap();
        assert!(!rows.is_empty(), "an India world has clubs abroad");
        abroad = rows.iter().find(|r| {
            let n = &cell(&t, r, "nation")["r"];
            !n.is_null() && n["id"] != home
        }).map(|r| r["id"].clone());
        offset += rows.len();
    }
    let club = api.call("club", json!({"id": abroad.unwrap()})).unwrap();
    if let Some(Err(e)) = pw_view::contract_pages::check_response("club", &club) {
        panic!("club is not its declared type: {e}");
    }
    let k = &club["country"];
    assert_eq!(k["nation"]["id"], club["nation"]["id"], "{k}");
    for f in ["climate", "language", "clock", "from_home", "football", "living"] {
        assert!(k[f].as_str().is_some_and(|s| !s.is_empty()), "a move abroad says {f}: {k}");
    }
    assert!(k["origin"] == "Imported" || k["origin"] == "Inferred", "{k}");
}
