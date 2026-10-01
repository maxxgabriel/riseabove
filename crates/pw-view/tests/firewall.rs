//! The perspective firewall (locked design 8): what a viewer who is not omniscient sees must not move when only hidden truth moves.
//!
//! The strongest test of a leak is non-interference: change hidden truth (true ability and potential, personality, what others feel
//! about the viewer, journalists' private leanings, referees' private numbers) and look at every list, every sort and every page again.
//! If anything a non-omniscient viewer can see has moved, the truth is reaching the screen, directly or through an ordering or filter.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use pw_core::PersonId;
use pw_view::Api;
use serde_json::{Value, json};

fn api() -> Api {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    Api::new(std::env::temp_dir().join(format!("pw-firewall-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))))
}

fn wait(api: &Api, key: &str) {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s[key]["running"].as_bool().unwrap_or(false) {
            assert!(s[key]["error"].is_null(), "{key} failed: {}", s[key]["error"]);
            return;
        }
        assert!(t0.elapsed() < Duration::from_secs(300), "{key} timed out");
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn world() -> Api {
    let api = api();
    api.call("world.new", json!({"kind": "synthetic", "scale": "small", "seed": 19})).unwrap();
    wait(&api, "task");
    api.call("advance.start", json!({"mode": "days", "n": 70})).unwrap();
    wait(&api, "job");
    api
}

const TABLES: &[&str] = &[
    "players", "staff", "clubs", "nations", "comps", "standings", "fixtures", "player_stats", "comp_stats", "events", "transfers", "honours", "awards", "spells", "stories", "agents", "talks", "bids",
    "intl_matches", "tournaments", "boards", "sponsors", "posts", "chants", "memes", "groups", "rivalries", "incidents", "conferences", "quotes", "referees", "controversies", "charges", "record_book",
    "records_broken", "votes", "hall_members", "chronicle", "schools", "rule_changes", "institutions", "minor_seasons", "outlets", "journalists", "grapevine",
];

fn filters(table: &str, person: u32) -> Value {
    match table {
        "standings" | "fixtures" | "comp_stats" => json!({"comp": 0}),
        "player_stats" | "spells" => json!({"person": person}),
        _ => json!({}),
    }
}

/// Every table, in its default form and sorted by each of its sortable columns, with all its columns.
fn tables(api: &Api, person: u32) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for &t in TABLES {
        let f = filters(t, person);
        let Ok(first) = api.call("table.query", json!({"table": t, "filters": f, "limit": 60})) else { continue };
        let cols: Vec<Value> = first["all_columns"].as_array().cloned().unwrap_or_default();
        let keys: Vec<Value> = cols.iter().map(|c| c["key"].clone()).collect();
        out.insert(format!("{t}/default"), first);
        for c in cols.iter().filter(|c| c["sortable"].as_bool().unwrap_or(false)) {
            let key = c["key"].as_str().unwrap();
            for desc in [true, false] {
                let q = json!({"table": t, "filters": f, "limit": 40, "columns": keys, "sort": {"key": key, "desc": desc}});
                if let Ok(v) = api.call("table.query", q) {
                    out.insert(format!("{t}/sort:{key}:{}", if desc { "desc" } else { "asc" }), v);
                }
            }
        }
    }
    out
}

fn pages(api: &Api, people: &[u32], inhabited: bool) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let mut put = |name: String, m: &str, args: Value| {
        if let Ok(v) = api.call(m, args) {
            out.insert(name, v);
        }
    };
    for &p in people {
        put(format!("person/{p}"), "person", json!({"id": p}));
        put(format!("person.attributes/{p}"), "person.attributes", json!({"id": p}));
        put(format!("insight.person/{p}"), "insight.person", json!({"id": p}));
    }
    for c in 0..4 {
        put(format!("club/{c}"), "club", json!({"id": c}));
        put(format!("club.systems/{c}"), "club.systems", json!({"id": c}));
        put(format!("insight.club/{c}"), "insight.club", json!({"id": c}));
    }
    put("comp".into(), "comp", json!({"id": 0}));
    put("comp.overview".into(), "comp.overview", json!({"id": 0}));
    put("insight.comp".into(), "insight.comp", json!({"id": 0}));
    put("nation".into(), "nation", json!({"id": 0}));
    put("overview".into(), "overview", json!({}));
    put("world.pulse".into(), "world.pulse", json!({}));
    put("news.feed".into(), "news.feed", json!({}));
    put("search".into(), "search", json!({"q": "a"}));
    if inhabited {
        for m in ["me.today", "me.self", "me.life", "me.people", "me.promises", "me.rumours", "me.press", "me.agent", "me.contract", "me.football", "me.options", "me.messages", "me.inbox", "me.feed"] {
            put(m.to_string(), m, json!({}));
        }
    }
    out
}

/// The paths at which two JSON values differ (first few).
fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    if out.len() >= 6 {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))).filter(|k| *k != "revision") {
                diff(x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null), &format!("{path}.{k}"), out);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                diff(p, q, &format!("{path}[{i}]"), out);
            }
        }
        _ if a != b => out.push(format!("{path}: {} -> {}", short(a), short(b))),
        _ => {}
    }
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 70 { format!("{}...", &s[..70]) } else { s }
}

/// Change everything hidden: true ability and potential, the personalities of everyone but the viewer, what everyone feels about the
/// viewer, the private leanings of journalists and the private numbers of referees.
fn perturb_hidden(api: &Api, me: Option<PersonId>) {
    api.debug_mutate_world(|w| {
        for c in w.players.cold.iter_mut() {
            let ca = c.ca;
            c.ca = 20 + (u16::from(c.pa) * 7 % 150) as u8;
            c.pa = c.ca.max(ca) + 10;
        }
        for (id, p) in w.people.iter_enumerated_mut() {
            if Some(id) != me {
                for h in pw_core::Hidden::ALL {
                    let v = p.hidden.get(h);
                    p.hidden.set(h, 20 - v.min(20));
                }
            }
        }
        if let Some(me) = me {
            let today = w.date;
            let pairs: Vec<(PersonId, PersonId)> = w.social.endpoints().filter(|&(_, b)| b == me).collect();
            for (a, b) in pairs {
                w.social.adjust(a, b, today, 0, -80, -40, -40);
            }
        }
        for j in w.media.journalist_profiles.values_mut() {
            j.knowledge = 100 - j.knowledge;
            j.risk = 100 - j.risk;
            j.ambition = 100 - j.ambition;
        }
        for r in w.officials.referees.iter_mut() {
            r.strictness = 100 - r.strictness;
            r.accuracy = 100 - r.accuracy;
        }
    })
    .unwrap();
}

fn report(before: &BTreeMap<String, Value>, after: &BTreeMap<String, Value>) -> Vec<String> {
    let mut bad = Vec::new();
    for (k, a) in before {
        let b = after.get(k).unwrap_or(&Value::Null);
        let mut d = Vec::new();
        diff(a, b, "", &mut d);
        if !d.is_empty() {
            bad.push(format!("{k}: {}", d.join(" | ")));
        }
    }
    bad
}

#[test]
fn audit_public_view() {
    let api = world();
    api.call("persp.observe", json!({"public": true})).unwrap();
    let people = [1008u32, 1500, 2100, 40];
    let (t0, p0) = (tables(&api, 1008), pages(&api, &people, false));
    perturb_hidden(&api, None);
    let (t1, p1) = (tables(&api, 1008), pages(&api, &people, false));
    let mut bad = report(&t0, &t1);
    bad.extend(report(&p0, &p1));
    assert!(t0.len() + p0.len() > 400, "the audit covered {} lists and pages", t0.len() + p0.len());
    assert!(bad.is_empty(), "{} things a public viewer sees moved when only hidden truth did:\n  {}", bad.len(), bad.join("\n  "));
}

#[test]
fn audit_inhabited_view() {
    let api = world();
    let t = api.call("table.query", json!({"table": "players", "filters": {"inhabitable": true, "kind": "first", "status": "active"}, "limit": 200})).unwrap();
    let rows = t["rows"].as_array().unwrap();
    let me = rows[rows.len() / 2]["open"]["id"].as_u64().unwrap() as u32;
    api.call("persp.inhabit", json!({"person": me})).unwrap();
    let people = [1008u32, 1500, 2100, 40, me + 1, me + 2];
    let (t0, p0) = (tables(&api, me), pages(&api, &people, true));
    perturb_hidden(&api, Some(PersonId(me)));
    let (t1, p1) = (tables(&api, me), pages(&api, &people, true));
    let mut bad = report(&t0, &t1);
    bad.extend(report(&p0, &p1));
    assert!(t0.len() + p0.len() > 400, "the audit covered {} lists and pages", t0.len() + p0.len());
    assert!(bad.is_empty(), "{} things an inhabited viewer sees moved when only hidden truth did:\n  {}", bad.len(), bad.join("\n  "));
}

fn inhabit_mid(api: &Api) -> u32 {
    let t = api.call("table.query", json!({"table": "players", "filters": {"inhabitable": true, "kind": "first", "status": "active"}, "limit": 200})).unwrap();
    let rows = t["rows"].as_array().unwrap();
    let me = rows[rows.len() / 2]["open"]["id"].as_u64().unwrap() as u32;
    api.call("persp.inhabit", json!({"person": me})).unwrap();
    me
}

fn ids(t: &Value) -> Vec<u64> {
    t["rows"].as_array().unwrap().iter().map(|r| r["open"]["id"].as_u64().unwrap()).collect()
}

/// Two players in the same position swap their true ability: a list a non-omniscient viewer sorts by position must not notice.
#[test]
fn player_lists_are_not_ordered_by_true_ability() {
    let api = world();
    inhabit_mid(&api);
    let q = || api.call("table.query", json!({"table": "players", "filters": {"kind": "first", "status": "active"}, "sort": {"key": "pos", "desc": false}, "limit": 400})).unwrap();
    let before = ids(&q());
    // Make the true ordering within every position the reverse of what it was.
    api.debug_mutate_world(|w| {
        let mut by_pos: std::collections::BTreeMap<usize, Vec<pw_core::PlayerId>> = Default::default();
        for p in w.players.ids() {
            by_pos.entry(w.players.cold[p].best_pos.idx()).or_default().push(p);
        }
        for (_, mut v) in by_pos {
            v.sort_by_key(|&p| w.players.cold[p].ca);
            let cas: Vec<(u8, u8)> = v.iter().map(|&p| (w.players.cold[p].ca, w.players.cold[p].pa)).collect();
            for (&p, &(ca, pa)) in v.iter().zip(cas.iter().rev()) {
                w.players.cold[p].ca = ca;
                w.players.cold[p].pa = pa;
            }
        }
    });
    assert_eq!(before, ids(&q()), "the order of a position-sorted list follows something the viewer cannot see");
    // And the omniscient view is allowed its truth columns, sorted by them.
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    let t = api.call("table.query", json!({"table": "players", "filters": {"kind": "first", "status": "active"}, "sort": {"key": "ca", "desc": true}, "limit": 5})).unwrap();
    assert!(t["all_columns"].as_array().unwrap().iter().any(|c| c["key"] == "ca"));
}

#[test]
fn filters_and_sorts_by_hidden_truth_are_refused_outside_the_omniscient_view() {
    let api = world();
    let total = |api: &Api, f: Value, sort: Option<&str>| {
        let mut q = json!({"table": "players", "filters": f, "limit": 5});
        if let Some(k) = sort {
            q["sort"] = json!({"key": k, "desc": true});
        }
        let t = api.call("table.query", q).unwrap();
        (t["total"].as_u64().unwrap(), ids(&t), t["sort"].clone())
    };
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    let all = total(&api, json!({"kind": "first"}), None).0;
    assert!(total(&api, json!({"kind": "first", "min_ca": 150}), None).0 < all, "the omniscient view may filter by true ability");
    for public in [true, false] {
        if public {
            api.call("persp.observe", json!({"public": true})).unwrap();
        } else {
            inhabit_mid(&api);
        }
        assert_eq!(total(&api, json!({"kind": "first", "min_ca": 150}), None).0, all, "min_ca filters on truth (public: {public})");
        assert_eq!(total(&api, json!({"kind": "first", "expiring_days": 5}), None).0, all, "contract expiry is private (public: {public})");
        for hidden in ["ca", "pa", "wage", "value", "condition", "morale", "contract_end"] {
            let (_, _, applied) = total(&api, json!({"kind": "first"}), Some(hidden));
            assert!(applied.is_null() || applied[0] != hidden, "a sort on {hidden} was honoured (public: {public})");
        }
    }
}

#[test]
fn a_search_cannot_be_used_to_find_people_by_hidden_traits() {
    let api = world();
    inhabit_mid(&api);
    for q in ["ambitious", "professional", "temperamental", "high potential", "pa:170"] {
        let s = api.call("search", json!({"q": q})).unwrap();
        let hits: usize = s["groups"].as_array().unwrap().iter().map(|g| g["items"].as_array().map_or(0, Vec::len)).sum();
        let people: usize = s["groups"].as_array().unwrap().iter().filter(|g| g["title"].as_str().is_some_and(|t| t.to_lowercase().contains("player") || t.to_lowercase().contains("people"))).map(|g| g["items"].as_array().map_or(0, Vec::len)).sum();
        assert!(people == 0 || hits == people, "'{q}' finds people: {s}");
        assert_eq!(people, 0, "'{q}' matched people by something other than a name: {s}");
    }
}

#[test]
fn relationship_pages_show_evidence_and_tone_and_not_the_internal_numbers() {
    let api = world();
    let me = inhabit_mid(&api);
    let before = api.call("me.people", json!({})).unwrap();
    let rows = before["people"].as_array().unwrap();
    assert!(!rows.is_empty(), "the inhabited player knows people");
    for r in rows {
        for k in ["affinity", "value", "score", "raw"] {
            assert!(r.get(k).is_none(), "a relationship row carries {k}: {r}");
        }
        assert!(r["label"].is_string() && r["trust"].is_string() && r["respect"].is_string() && ["pos", "warn", "neg"].contains(&r["tone"].as_str().unwrap()));
        assert!(r["evidence"].is_array());
    }
    // A person can secretly resent the viewer while appearing friendly: how everyone feels about me is not on any page I can read.
    let person = api.call("person", json!({"id": rows[0]["who"]["id"]})).unwrap();
    api.debug_mutate_world(|w| {
        let me = PersonId(me);
        let today = w.date;
        let pairs: Vec<(PersonId, PersonId)> = w.social.endpoints().filter(|&(_, b)| b == me).collect();
        for (a, b) in pairs {
            w.social.adjust(a, b, today, 0, -100, -100, -100);
        }
    });
    assert_eq!(before["people"], api.call("me.people", json!({})).unwrap()["people"], "my page moved with what others feel about me");
    assert_eq!(person, api.call("person", json!({"id": rows[0]["who"]["id"]})).unwrap());
    // What has been heard is said in words.
    let r = api.call("me.rumours", json!({})).unwrap();
    for x in r["rumours"].as_array().unwrap() {
        assert!(x["confidence"].is_null() && x["sureness"].is_string(), "a rumour with a false percentage: {x}");
    }
}

#[test]
fn the_diagnosis_of_an_injury_is_the_clubs_business() {
    let api = world();
    // Find a first-team player who is injured, in the omniscient view.
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    let t = api.call("table.query", json!({"table": "players", "filters": {"kind": "first", "status": "active", "injured": true}, "limit": 20})).unwrap();
    let hurt = t["rows"].as_array().unwrap().first().map(|r| (r["open"]["id"].as_u64().unwrap(), r["cells"].clone()));
    let (id, _) = hurt.expect("after ten weeks somebody is injured");
    let cell = |api: &Api| {
        let t = api.call("table.query", json!({"table": "players", "filters": {"ids": [id]}, "columns": ["name", "avail", "club"], "limit": 1})).unwrap();
        t["rows"][0]["cells"][1].clone()
    };
    let omniscient = cell(&api);
    assert!(omniscient["sub"].is_string(), "the omniscient view has the diagnosis: {omniscient}");
    api.call("persp.observe", json!({"public": true})).unwrap();
    let public = cell(&api);
    assert_eq!(public["s"], "Injured", "everyone can see he is out: {public}");
    assert!(public["sub"].is_null() && public["n"].is_null(), "but not the diagnosis or the days: {public}");
    // A stranger at another club is in the same position; the man's own club is not.
    let me = inhabit_mid(&api);
    let mine = api.call("person", json!({"id": me})).unwrap();
    let _ = mine;
    let stranger = cell(&api);
    let club_of_me = api.call("world.status", json!({})).unwrap()["perspective"]["club"].clone();
    let club_of_him = api.call("person", json!({"id": id})).unwrap();
    let same = club_of_him["player"]["club"]["name"] == club_of_me;
    if !same {
        assert!(stranger["sub"].is_null(), "{stranger}");
    }
}

#[test]
fn omniscience_is_its_own_labelled_view_and_the_others_do_not_reach_it() {
    let api = world();
    let status = |api: &Api| api.call("world.status", json!({})).unwrap()["perspective"].clone();
    let internal = |api: &Api| api.call("person", json!({"id": 1008})).unwrap()["player"]["internal"].clone();
    let cols = |api: &Api| api.call("table.query", json!({"table": "players", "limit": 3})).unwrap()["all_columns"].as_array().unwrap().iter().map(|c| c["key"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    // A world opens in the public view: the same world, without the truth.
    let s = status(&api);
    assert_eq!((s["mode"].as_str(), s["omniscient"].as_bool()), (Some("public"), Some(false)));
    assert!(internal(&api).is_null() && !cols(&api).contains(&"ca".to_string()));
    // The omniscient debug view has to be asked for by name, and it says what it is.
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    let s = status(&api);
    assert_eq!((s["mode"].as_str(), s["omniscient"].as_bool()), (Some("observer"), Some(true)));
    assert!(!internal(&api).is_null() && cols(&api).contains(&"ca".to_string()));
    // Leaving it without naming a view goes back to the public one.
    api.call("persp.observe", json!({})).unwrap();
    let s = status(&api);
    assert_eq!((s["mode"].as_str(), s["omniscient"].as_bool()), (Some("public"), Some(false)));
    assert!(internal(&api).is_null() && !cols(&api).contains(&"ca".to_string()));
    assert_eq!(api.call("person", json!({"id": 1008})).unwrap()["perspective"], "public");
    assert_eq!(api.call("club", json!({"id": 3})).unwrap()["economy"], Value::Null, "national finance is not the public's");
    // Inhabiting is another.
    inhabit_mid(&api);
    assert_eq!(status(&api)["mode"], "inhabit");
    assert!(internal(&api).is_null());
    // No view survives a save: reopening a world starts in the public view, even one saved from the omniscient view.
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    api.call("world.save", json!({"file": "fw"})).unwrap();
    api.call("world.load", json!({"file": "fw.pws"})).unwrap();
    wait(&api, "task");
    assert_eq!(status(&api)["mode"], "public");
    assert!(internal(&api).is_null());
}

/// The places in this crate that read the engine's private truth. Adding one anywhere else fails this test: it has to be reviewed,
/// gated on `Ctx::observer` (the omniscient view) or on the viewer's own person, and recorded here.
#[test]
fn engine_truth_is_read_only_in_the_files_that_gate_it() {
    use std::path::Path;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let patterns: [(&str, &[&str]); 5] = [
        ("true ability", &[".ca ", ".ca)", ".ca,", ".ca}", ".ca.", ".pa ", ".pa)", ".pa,", ".pa}", ".pa."]),
        ("personality", &[".hidden.get(", ".hidden.f(", "Hidden::"]),
        ("relationship internals", &[".affinity", ".respect"]),
        ("private books", &["w.boardroom", "w.dossiers", "w.governance", "w.deals."]),
        ("private medical", &["injury_days", "medical."]),
    ];
    // file -> what it may read (each gated; see the file's own comments)
    let allowed: &[(&str, &str)] = &[
        // The view-type layer: the one file that turns a player into what a viewer may know of him (`VisiblePlayer`, `VisibleAbility`).
        ("visible.rs", "true ability"),
        ("visible.rs", "personality"),
        ("visible.rs", "private medical"),
        ("pages/insights.rs", "true ability"),
        ("pages/insights.rs", "private books"),
        ("pages/insights.rs", "private medical"),
        ("pages/me.rs", "private medical"),
        // Your own trial only (`deals.trials` filtered to the inhabited player): the club and the day they decide are what a player
        // on trial is told.
        ("pages/me.rs", "private books"),
        ("pages/club.rs", "private books"),
        ("pages/life.rs", "relationship internals"),
        ("pages/life.rs", "private medical"),
        ("pages/matchp.rs", "private medical"),
        ("tables/systems.rs", "private books"),
        ("tables/society.rs", "private books"),
        ("tables/society.rs", "personality"),
        ("tables/staff.rs", "true ability"),
        ("tables/clubs.rs", "private books"),
        ("ctx.rs", "private medical"),
        ("pages/world.rs", "private books"),
        ("pages/act.rs", "private books"),
        ("pages/act.rs", "private medical"),
        ("advance.rs", "private books"),
    ];
    let mut found: Vec<(String, &str)> = Vec::new();
    let mut reads: Vec<(String, &str)> = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&root, &mut files);
    for f in files {
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&f).unwrap();
        let code: String = text.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        for (name, pats) in patterns {
            if pats.iter().any(|p| code.contains(p)) {
                reads.push((rel.clone(), name));
                if !allowed.iter().any(|&(af, an)| af == rel && an == name) {
                    found.push((rel.clone(), name));
                }
            }
        }
    }
    assert!(found.is_empty(), "engine truth is read in files that are not on the reviewed list: {found:?}");
    // The player list rows and the person page's attribute and ability readout are built from the view types, not from the world:
    // they are off the list, and they stay off it. `visible.rs` is where that truth is read and gated.
    for off in ["tables/players.rs", "pages/person.rs"] {
        let leaks: Vec<_> = reads.iter().filter(|(f, _)| f == off).collect();
        assert!(leaks.is_empty(), "{off} reads engine truth again instead of using VisiblePlayer/VisibleAbility: {leaks:?}");
    }
    for need in ["true ability", "personality", "private medical"] {
        assert!(reads.iter().any(|(f, n)| f == "visible.rs" && *n == need), "visible.rs no longer reads {need}: the layer is not where the gate is");
    }
}

#[test]
fn private_negotiations_and_interest_are_not_listed_outside_the_omniscient_view() {
    let api = world();
    let rows = |api: &Api, t: &str| api.call("table.query", json!({"table": t, "limit": 50})).unwrap()["total"].as_u64().unwrap();
    let private = ["talks", "bids", "boards", "grapevine", "sponsors"];
    assert!(private.iter().any(|t| rows(&api, t) > 0), "the omniscient view has private books to hide");
    for public in [true, false] {
        if public {
            api.call("persp.observe", json!({"public": true})).unwrap();
        } else {
            inhabit_mid(&api);
        }
        for t in private {
            let r = api.call("table.query", json!({"table": t, "limit": 50})).unwrap();
            let cols: Vec<&str> = r["all_columns"].as_array().unwrap().iter().map(|c| c["key"].as_str().unwrap()).collect();
            // Either nothing is listed, or nothing of what is listed is a private figure.
            assert!(r["total"].as_u64().unwrap() == 0 || !cols.iter().any(|k| ["ask", "reserve", "ceiling", "wealth", "meddling", "patience", "interest"].contains(k)), "{t} lists private figures ({cols:?}, public: {public})");
        }
    }
}

/// How the viewer feels is told in words (locked design 8.5): the pages about oneself carry a word and its place among the words, never
/// the engine's number, so moving the number inside one word changes nothing on any of them; and a forecast is not a percentage.
#[test]
fn the_viewers_own_state_is_in_words_and_a_forecast_is_not_a_percentage() {
    let api = world();
    let me = inhabit_mid(&api);
    let pages = |api: &Api| ["me.today", "me.self", "me.life", "me.agent"].map(|m| api.call(m, json!({})).unwrap());
    let is_band = |v: &Value| v.as_object().is_some_and(|o| o.len() == 3 && o["label"].is_string() && o["step"].as_u64() <= o["steps"].as_u64() && o["step"].as_u64() >= Some(1));
    let [today, own, life, agent] = pages(&api);
    for k in ["condition", "sharpness", "morale", "confidence", "wellbeing", "fatigue"] {
        assert!(is_band(&today["condition"][k]), "{k} is not a word band: {}", today["condition"][k]);
    }
    for k in ["stress", "sleep", "fulfilment"] {
        assert!(is_band(&own[k]), "{k} is not a word band: {}", own[k]);
    }
    for m in today["mind"].as_array().unwrap().iter().chain(own["mood"].as_array().unwrap()).chain(own["wellbeing"].as_array().unwrap()) {
        assert!(m.get("value").is_none() && ["pos", "neg", "flat"].contains(&m["pull"].as_str().unwrap()), "a feeling with a number: {m}");
    }
    for l in life["languages"].as_array().unwrap() {
        assert!(is_band(&l["level"]) && l.get("value").is_none(), "{l}");
    }
    if !agent["agent"].is_null() {
        assert!(is_band(&agent["agent"]["satisfaction"]));
    }
    for t in own["told"].as_array().unwrap() {
        assert!(!t["text"].as_str().unwrap().contains('%'), "what a coach said is quoted as a percentage: {t}");
    }
    // Inside one word, the number underneath moves nothing: fresh at 95 and at 99 reads the same.
    let set = |api: &Api, v: u8| {
        api.debug_mutate_world(|w| {
            let p = w.people[PersonId(me)].player;
            let h = &mut w.players.hot[p];
            (h.condition, h.sharpness, h.morale, h.confidence, h.wellbeing, h.fatigue) = (v, v, v, v, v, 100 - v);
        });
    };
    set(&api, 95);
    let a = pages(&api);
    set(&api, 99);
    let b = pages(&api);
    assert_eq!(a[0]["condition"], b[0]["condition"], "the condition page moved inside one word");
}

/// The people (first-team players, in list order) a view-type test looks at.
fn sample_people(api: &Api, n: usize) -> Vec<u32> {
    let t = api.call("table.query", json!({"table": "players", "filters": {"kind": "first", "status": "active"}, "limit": 400})).unwrap();
    let all: Vec<u32> = ids(&t).into_iter().map(|i| i as u32).collect();
    let step = (all.len() / n).max(1);
    all.into_iter().step_by(step).take(n).collect()
}

/// `VisiblePlayer` and `VisibleAbility` (the view types the player list rows and the attribute readout are built from) can only be
/// built through `Ctx`, so for a public or inhabited viewer they do not hold the true ability, the potential, the personality, other
/// people's private terms, their body state or their diagnosis: not as a value a page forgot to hide, but as `None`. Hidden truth is
/// then changed under them and they do not move; in the omniscient view the same types do carry it (the test is not vacuous).
#[test]
fn the_view_type_cannot_carry_hidden_truth_to_a_public_or_inhabited_viewer() {
    let api = world();
    let people = sample_people(&api, 40);
    let shown = |api: &Api| -> BTreeMap<u32, Value> { people.iter().map(|&p| (p, api.debug_visible_player(p).expect("a player"))).collect() };

    // The omniscient view is the only one that has the truth in the types.
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    for (p, v) in shown(&api) {
        for field in ["engine", "terms", "value", "body"] {
            assert!(!v["player"][field].is_null(), "the omniscient view lacks {field} for {p}: {}", v["player"]);
        }
        assert!(v["ability"]["engine"].is_object() && v["ability"]["hidden"].is_array(), "the omniscient readout lacks the truth for {p}");
        assert_eq!(v["ability"]["assessment"], "Omniscient");
    }

    for public in [true, false] {
        let me = if public {
            api.call("persp.observe", json!({"public": true})).unwrap();
            None
        } else {
            Some(inhabit_mid(&api))
        };
        let before = shown(&api);
        for (&p, v) in &before {
            let own = Some(p) == me;
            // Never, for anyone: the engine's numbers and the personality behind the readout.
            assert!(v["player"]["engine"].is_null(), "true ability or personality reached the view of {p} (public: {public}): {}", v["player"]["engine"]);
            assert!(v["ability"]["engine"].is_null() && v["ability"]["hidden"].is_null(), "the readout of {p} carries the truth (public: {public})");
            assert_ne!(v["ability"]["assessment"], "Omniscient");
            // Private terms, worth and body state: the man's own, or nothing.
            for field in ["terms", "value", "body"] {
                assert_eq!(!v["player"][field].is_null(), own, "{field} of {p} (public: {public}, own: {own}): {}", v["player"][field]);
            }
        }
        // Change everything hidden. The types for a viewer who may not know it do not move.
        perturb_hidden(&api, me.map(PersonId));
        let after = shown(&api);
        for (p, a) in &before {
            let mut d = Vec::new();
            diff(a, &after[p], "", &mut d);
            assert!(d.is_empty(), "the view of {p} moved when only hidden truth did (public: {public}): {}", d.join(" | "));
        }
    }
}

/// A person page used to give any viewer the injury and its days. It is now built from `VisiblePlayer`: strangers see that he is out.
#[test]
fn the_person_page_keeps_a_strangers_diagnosis_to_the_club() {
    let api = world();
    api.call("persp.observe", json!({"omniscient": true})).unwrap();
    let t = api.call("table.query", json!({"table": "players", "filters": {"kind": "first", "status": "active", "injured": true}, "limit": 20})).unwrap();
    let id = ids(&t).first().copied().expect("after ten weeks somebody is injured");
    let avail = |api: &Api| api.call("person", json!({"id": id})).unwrap()["player"]["availability"].clone();
    let omniscient = avail(&api);
    assert!(omniscient["label"].as_str().unwrap().contains("about") && !omniscient["detail"].as_str().unwrap().is_empty(), "{omniscient}");
    api.call("persp.observe", json!({"public": true})).unwrap();
    let public = avail(&api);
    assert_eq!((public["label"].as_str(), public["detail"].as_str()), (Some("Injured"), Some("")), "the public sees that he is out, nothing more: {public}");
    // The man himself knows.
    api.call("persp.inhabit", json!({"person": id})).unwrap();
    assert_eq!(avail(&api), omniscient, "he knows his own injury");
}
