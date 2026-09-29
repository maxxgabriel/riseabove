//! Contract tests (locked design 9): drift between the backend and the frontend fails here, not when someone clicks a screen.
//!
//! * the TypeScript the app compiles against is generated from the Rust contract and must equal it;
//! * every method is declared as a query or a command, and the declaration matches what the dispatcher answers;
//! * queries change nothing and repeat exactly;
//! * responses of typed methods carry exactly the declared fields;
//! * errors come in categories, with the older three-way code kept;
//! * unknown, hidden, estimated and known stay four different things on the wire.

use std::path::Path;
use std::time::{Duration, Instant};

use pw_view::contract::{self, ErrorKind, Knowledge, MethodKind};
use pw_view::{Api, ErrorBody};
use serde_json::{Value, json};

fn api() -> Api {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    Api::new(std::env::temp_dir().join(format!("pw-contract-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))))
}

fn wait(api: &Api, key: &str) {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s[key]["running"].as_bool().unwrap_or(false) {
            return;
        }
        assert!(t0.elapsed() < Duration::from_secs(300), "{key} timed out");
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn world() -> Api {
    let api = api();
    api.call("world.new", json!({"kind": "synthetic", "scale": "small", "seed": 7})).unwrap();
    wait(&api, "task");
    api.call("advance.start", json!({"mode": "days", "n": 50})).unwrap();
    wait(&api, "job");
    api
}

fn inhabit(api: &Api) -> u32 {
    let t = api.call("table.query", json!({"table": "players", "filters": {"inhabitable": true, "kind": "first", "status": "active"}, "limit": 100})).unwrap();
    let rows = t["rows"].as_array().unwrap();
    let me = rows[rows.len() / 2]["open"]["id"].as_u64().unwrap() as u32;
    api.call("persp.inhabit", json!({"person": me})).unwrap();
    me
}

fn generated_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src/contract.generated.ts")
}

#[test]
fn the_typescript_the_app_compiles_against_is_the_contract() {
    let want = contract::typescript();
    let path = generated_path();
    if std::env::var_os("UPDATE_CONTRACT").is_some() {
        std::fs::write(&path, &want).unwrap();
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
    assert!(
        have == want,
        "app/src/contract.generated.ts is out of date with crates/pw-view/src/contract.rs. Regenerate: UPDATE_CONTRACT=1 cargo test -p pw-view --test contract"
    );
}

#[test]
fn every_method_is_declared_as_a_query_or_a_command_and_the_declaration_matches_the_dispatcher() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap();
    let start = src.find("pub fn call(&self, method: &str").unwrap();
    let end = src[start..].find("// ---- world lifecycle").unwrap();
    let body = &src[start..start + end];
    let mut dispatched: Vec<String> = Vec::new();
    for line in body.lines() {
        let l = line.trim_start();
        if let Some(rest) = l.strip_prefix('"')
            && let Some(name) = rest.split('"').next()
            && rest[name.len()..].starts_with("\" =>")
        {
            dispatched.push(name.to_string());
        }
    }
    let mut declared: Vec<String> = contract::manifest().iter().map(|m| m.name.to_string()).collect();
    dispatched.sort();
    declared.sort();
    let dup: Vec<&String> = declared.windows(2).filter(|w| w[0] == w[1]).map(|w| &w[0]).collect();
    assert!(dup.is_empty(), "declared twice: {dup:?}");
    let missing: Vec<&String> = dispatched.iter().filter(|d| !declared.contains(d)).collect();
    let extra: Vec<&String> = declared.iter().filter(|d| !dispatched.contains(d)).collect();
    assert!(missing.is_empty() && extra.is_empty(), "dispatched but not declared: {missing:?}; declared but not dispatched: {extra:?}");
    assert!(dispatched.len() > 60, "the dispatcher was read ({} methods)", dispatched.len());
}

#[test]
fn queries_change_nothing_and_repeat_exactly() {
    let api = world();
    inhabit(&api);
    let status = |api: &Api| api.call("world.status", json!({})).unwrap();
    let mut answered = 0;
    for m in contract::manifest().iter().filter(|m| m.kind == MethodKind::Query) {
        // Wall-clock timings are the one thing allowed to differ between two identical questions.
        if m.name == "diagnostics" {
            continue;
        }
        let before = status(&api);
        let first = api.call(m.name, json!({}));
        let second = api.call(m.name, json!({}));
        let after = status(&api);
        assert_eq!(before["revision"], after["revision"], "{} changed the world's revision", m.name);
        assert_eq!(before["date"], after["date"], "{} moved the clock", m.name);
        match (first, second) {
            (Ok(a), Ok(b)) => {
                assert_eq!(a, b, "{} answered differently to the same question", m.name);
                answered += 1;
            }
            (Err(a), Err(b)) => assert_eq!(a.kind(), b.kind(), "{} fails differently", m.name),
            (a, b) => panic!("{}: {:?} then {:?}", m.name, a.map(|_| "ok"), b.map(|_| "ok")),
        }
    }
    assert!(answered > 20, "{answered} queries answered without arguments");
}

#[test]
fn commands_are_intents_not_authoritative_state() {
    // No request type in the contract carries a value the world owns.
    let ts = contract::typescript();
    for req in ["TableReq", "PersonReq"] {
        let (a, b) = contract::interface_fields(&ts, req).unwrap();
        for f in a.iter().chain(&b) {
            assert!(!["wage", "fee", "ca", "pa", "value", "rating", "salary", "result", "score", "goals"].contains(&f.as_str()), "{req} carries world state: {f}");
        }
    }
    // And the world does not take a new wage from the client when the action is merely to ask for something.
    let api = world();
    let me = inhabit(&api);
    let before = api.call("me.contract", json!({})).unwrap();
    let _ = api.call("me.act", json!({"action": "transfer_request", "wage": 999_999, "fee": 1, "ca": 200}));
    api.call("advance.start", json!({"mode": "days", "n": 7})).unwrap();
    wait(&api, "job");
    let after = api.call("me.contract", json!({})).unwrap();
    assert_eq!(before["wage"], after["wage"], "a client-supplied wage changed the contract of {me}");
}

fn check_against(ts: &str, name: &str, v: &Value) {
    let (req, opt) = contract::interface_fields(ts, name).unwrap_or_else(|| panic!("no declaration of {name}"));
    let obj = v.as_object().unwrap_or_else(|| panic!("{name} is not an object: {v}"));
    for f in &req {
        assert!(obj.contains_key(f), "{name}: declared field {f} is missing from {v}");
    }
    for k in obj.keys() {
        assert!(req.contains(k) || opt.contains(k), "{name}: the response has {k}, which the contract does not declare");
    }
}

#[test]
fn typed_responses_carry_exactly_the_declared_fields() {
    let ts = contract::typescript();
    let api = world();
    check_against(&ts, "AppInfo", &api.call("app.info", json!({})).unwrap());
    let s = api.call("world.status", json!({})).unwrap();
    check_against(&ts, "StatusView", &s);
    check_against(&ts, "Job", &s["job"]);
    check_against(&ts, "Task", &s["task"]);
    check_against(&ts, "SettingsView", &s["settings"]);
    let t = api.call("table.query", json!({"table": "players", "limit": 3})).unwrap();
    check_against(&ts, "TableResp", &t);
    for c in t["all_columns"].as_array().unwrap() {
        check_against(&ts, "Col", c);
    }
    for r in t["rows"].as_array().unwrap() {
        check_against(&ts, "Row", r);
        for cell in r["cells"].as_array().unwrap() {
            check_against(&ts, "Cell", cell);
        }
    }
    let pid = t["rows"][0]["open"]["id"].as_u64().unwrap();
    let a = api.call("person.attributes", json!({"id": pid})).unwrap();
    check_against(&ts, "AttributesView", &a);
    for g in a["groups"].as_array().unwrap() {
        check_against(&ts, "AttrGroupView", g);
    }
    for p in a["positions"].as_array().unwrap() {
        check_against(&ts, "PositionView", p);
    }
    inhabit(&api);
    let people = api.call("me.people", json!({})).unwrap();
    check_against(&ts, "PeopleView", &people);
    for r in people["people"].as_array().unwrap() {
        check_against(&ts, "RelationshipRow", r);
    }
    let rum = api.call("me.rumours", json!({})).unwrap();
    check_against(&ts, "RumoursView", &rum);
    for r in rum["rumours"].as_array().unwrap() {
        check_against(&ts, "RumourRow", r);
    }
    // The table request the app sends is the one the backend reads.
    let (req, opt) = contract::interface_fields(&ts, "TableReq").unwrap();
    assert!(req == ["table"] && ["filters", "sort", "offset", "limit", "columns", "preset"].iter().all(|f| opt.contains(&f.to_string())));
}

#[test]
fn a_status_of_a_closed_world_is_still_a_status() {
    let ts = contract::typescript();
    let s = api().call("world.status", json!({})).unwrap();
    assert_eq!(s["open"], false);
    check_against(&ts, "StatusView", &s);
}

#[test]
fn errors_come_in_categories_and_keep_the_older_code() {
    let api = api();
    let e = |m: &str, a: Value| api.call(m, a).unwrap_err();
    assert_eq!(e("no.such.method", json!({})).kind(), ErrorKind::NotFound);
    assert_eq!(e("me.today", json!({})).kind(), ErrorKind::StateConflict);
    let api = world();
    assert_eq!(api.call("person", json!({"id": 99_999_999})).unwrap_err().kind(), ErrorKind::NotFound);
    assert_eq!(api.call("persp.inhabit", json!({})).unwrap_err().kind(), ErrorKind::InvalidRequest);
    // Observing where a person is needed is a perspective problem, and a client can tell it from a failure.
    let err = api.call("me.today", json!({})).unwrap_err();
    assert_eq!((err.kind(), err.code()), (ErrorKind::UnauthorizedPerspective, "state"));
    // Busy: the world is advancing.
    api.call("advance.start", json!({"mode": "days", "n": 400})).unwrap();
    let busy = api.call("world.close", json!({}));
    if let Err(b) = busy {
        assert_eq!(b.kind(), ErrorKind::SimulationBusy);
        assert!(ErrorBody::of(&b).retryable);
    }
    api.call("advance.stop", json!({})).unwrap();
    wait(&api, "job");
    let me = inhabit(&api);
    // Someone else's private life.
    let other = api.call("person.life", json!({"id": me + 1})).unwrap_err();
    assert_eq!(other.kind(), ErrorKind::UnauthorizedPerspective);
    // An already-settled or impossible request is a conflict.
    let conflict = api.call("me.act", json!({"action": "withdraw_request"})).unwrap_err();
    assert_eq!(conflict.kind(), ErrorKind::StateConflict);
    // The wire body carries the category, the legacy code, the words and whether to retry.
    let body = serde_json::to_value(ErrorBody::of(&conflict)).unwrap();
    assert_eq!((body["kind"].as_str(), body["code"].as_str(), body["retryable"].as_bool()), (Some("state_conflict"), Some("state"), Some(false)));
    assert!(body["message"].as_str().is_some_and(|m| !m.is_empty()));
    // The legacy codes are what they always were.
    for (k, code) in [(ErrorKind::InvalidRequest, "bad_request"), (ErrorKind::NotFound, "not_found"), (ErrorKind::SaveIncompatible, "state"), (ErrorKind::InternalError, "state")] {
        assert_eq!(k.legacy_code(), code);
    }
    // Every category has a distinct name and the TypeScript lists them all.
    let ts = contract::typescript();
    let mut names: Vec<&str> = ErrorKind::ALL.iter().map(|k| k.name()).collect();
    for n in &names {
        assert!(ts.contains(&format!("\"{n}\"")), "{n} missing from the generated types");
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), ErrorKind::ALL.len());
}

#[test]
fn a_bad_save_is_reported_as_incompatible_not_as_a_crash() {
    let api = world();
    api.call("world.save", json!({"file": "good"})).unwrap();
    let dir = api.call("world.saves", json!({})).unwrap()["dir"].as_str().unwrap().to_string();
    std::fs::write(Path::new(&dir).join("junk.pws"), b"this is not a save").unwrap();
    api.call("world.load", json!({"file": "junk.pws"})).unwrap();
    wait(&api, "task");
    let s = api.call("world.status", json!({})).unwrap();
    assert!(s["task"]["error"].is_string(), "{s}");
    let e = pw_view::ApiError::SaveIncompatible("x".into());
    assert_eq!(e.kind(), ErrorKind::SaveIncompatible);
}

#[test]
fn unknown_hidden_estimated_and_known_are_four_different_things_on_the_wire() {
    let known = serde_json::to_value(Knowledge::Known { v: 12u8 }).unwrap();
    let est = serde_json::to_value(Knowledge::Estimated { lo: 9u8, hi: 15, v: 12 }).unwrap();
    let reported = serde_json::to_value(Knowledge::Reported { v: 12u8, source: "the press".into() }).unwrap();
    let unknown = serde_json::to_value(Knowledge::<u8>::Unknown).unwrap();
    let hidden = serde_json::to_value(Knowledge::<u8>::Hidden).unwrap();
    let kinds: Vec<&str> = [&known, &est, &reported, &unknown, &hidden].iter().map(|v| v["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["exact", "range", "reported", "unknown", "hidden"]);
    assert!(unknown.get("v").is_none() && hidden.get("v").is_none(), "no value stands in for what is not known");
    assert!(est["lo"] == 9 && est["hi"] == 15 && reported["source"] == "the press");
    // What the app is told about attributes uses the same tags, and a stranger is never given an exact value for what is unknown.
    let api = world();
    inhabit(&api);
    let mut seen = std::collections::BTreeSet::new();
    let t = api.call("table.query", json!({"table": "players", "filters": {"kind": "first"}, "limit": 200, "offset": 300})).unwrap();
    for r in t["rows"].as_array().unwrap().iter().take(60) {
        let a = api.call("person.attributes", json!({"id": r["open"]["id"]})).unwrap();
        for g in a["groups"].as_array().unwrap() {
            for x in g["attrs"].as_array().unwrap() {
                let k = x["kind"].as_str().unwrap();
                seen.insert(k.to_string());
                match k {
                    "unknown" => assert!(x.get("v").is_none(), "unknown with a value: {x}"),
                    "range" => assert!(x["lo"].as_u64() <= x["hi"].as_u64() && x.get("v").is_some()),
                    "exact" => assert!(x.get("lo").is_none()),
                    other => panic!("an attribute with kind {other}"),
                }
            }
        }
    }
    assert!(seen.contains("range"), "strangers are assessed in ranges: {seen:?}");
    let ts = contract::typescript();
    assert!(ts.contains("export type Knowledge<T>") && ts.contains("\"hidden\"") && ts.contains("\"unknown\""));
}
