//! A scripted playthrough of the India route, the way a person would play it, through the same API the interface calls.
//!
//! A human career is begun at several points on the route, weeks and seasons pass, and at every stop every query the client can make is
//! made and every response is read the way a player would read it: no error where the state permits, no `NaN`, no empty text, no
//! placeholder or debug residue, no sentence said twice, names that resolve, references that point at something that exists, dates and
//! money that make sense. Decisions are answered, intents are sent, the world is saved and reloaded, and the perspective firewall is
//! checked by non-interference (hidden truth is changed and nothing the inhabited person can read may move).
//!
//! `cargo test -p pw-view --test playthrough` is the short run. `-- --ignored` runs the long one (three seasons from every start).
//! Set `PW_TEXT_DUMP=path` to write every distinct line of generated text the run saw, for reading.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use pw_core::PersonId;
use pw_view::{Api, contract};
use serde_json::{Value, json};

// ------------------------------------------------------------------------------------------------ plumbing

fn api() -> Api {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    Api::new(std::env::temp_dir().join(format!("pw-playthrough-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))))
}

fn wait(api: &Api, key: &str) -> Value {
    let t0 = Instant::now();
    loop {
        let s = api.call("world.status", json!({})).unwrap();
        if !s[key]["running"].as_bool().unwrap_or(false) {
            assert!(s[key]["error"].is_null(), "{key} failed: {}", s[key]["error"]);
            return s[key].clone();
        }
        assert!(t0.elapsed() < Duration::from_secs(600), "{key} timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Words that mean an identifier or a code, not something a person reads.
const CODE_KEYS: &[&str] = &[
    "k", "kind", "key", "id", "dkind", "file", "code", "tone", "state", "mode", "folder", "last_kind", "fmt", "align", "presets", "preset", "columns", "table", "sort", "weekday", "perspective",
    "source", "reply", "filter", "handle", "seed", "applies", "claim", "grade", "reason_code", "style", "model",
];

/// Keys whose value is always a day number.
const DATE_KEYS: &[&str] = &["date", "since", "first", "deadline", "opened", "due", "next_due", "requested", "d", "last_viewed", "created", "end", "start", "until", "expires", "founded"];
/// Keys whose value is always an amount of money.
const MONEY_KEYS: &[&str] = &["fee", "wage", "money", "m", "amount", "cost", "price", "salary", "budget", "balance", "cash", "income", "prize", "worth", "savings", "debt", "release_clause", "base_cost"];
/// Keys whose string is sentence text a reader sees (never empty).
const TEXT_KEYS: &[&str] = &["text", "title", "headline", "body", "summary", "preview", "subject", "name", "label", "blurb"];

#[derive(Default)]
struct Bot {
    problems: Vec<String>,
    /// Kind of text -> distinct lines seen.
    lines: BTreeMap<String, BTreeSet<String>>,
    limits: Limits,
    today: i32,
    checked_matches: BTreeSet<u64>,
    calls: usize,
    at: String,
    flagged: BTreeSet<String>,
}

#[derive(Default, Clone, Copy)]
struct Limits {
    people: u32,
    clubs: u32,
    comps: u32,
    nations: u32,
    teams: u32,
}

impl Bot {
    fn problem(&mut self, what: String) {
        // One line per distinct problem shape, so a thousand copies of the same defect read as one.
        let shape: String = what.chars().filter(|c| !c.is_ascii_digit()).take(160).collect();
        if self.flagged.insert(shape) {
            self.problems.push(format!("[{}] {what}", self.at));
        }
    }

    fn refresh(&mut self, api: &Api) {
        self.limits = api
            .debug_mutate_world(|w| Limits { people: w.people.len() as u32, clubs: w.clubs.len() as u32, comps: w.comps.len() as u32, nations: w.nations.len() as u32, teams: w.teams.len() as u32 })
            .unwrap_or_default();
        self.today = api.call("world.status", json!({})).ok().and_then(|s| s["date"].as_i64()).unwrap_or(0) as i32;
    }

    /// Call a query or a command whose failure would be a defect, and read what came back.
    fn q(&mut self, api: &Api, method: &str, args: Value) -> Option<Value> {
        self.calls += 1;
        let label = if method == "table.query" { format!("table.query[{}]", args["table"].as_str().unwrap_or("?")) } else { method.to_string() };
        match api.call(method, args.clone()) {
            Ok(v) => {
                self.read(&label, &v);
                Some(v)
            }
            Err(e) => {
                self.problem(format!("{method} {args} failed: {e}"));
                None
            }
        }
    }

    /// Call something that may refuse for a reason of state; a refusal must be a plain sentence of the right kind, never a fault.
    fn maybe(&mut self, api: &Api, method: &str, args: Value) -> Option<Value> {
        self.calls += 1;
        match api.call(method, args.clone()) {
            Ok(v) => {
                self.read(method, &v);
                Some(v)
            }
            Err(e) => {
                let msg = e.to_string();
                if !matches!(e.kind(), contract::ErrorKind::StateConflict | contract::ErrorKind::UnauthorizedPerspective | contract::ErrorKind::UnavailableInformation | contract::ErrorKind::NotFound) {
                    self.problem(format!("{method} {args} refused with the wrong kind of error ({:?}): {msg}", e.kind()));
                }
                self.text(method, "error", &msg);
                None
            }
        }
    }

    fn read(&mut self, method: &str, v: &Value) {
        self.walk(method, "", v);
        self.duplicates(method, v);
    }

    fn walk(&mut self, at: &str, key: &str, v: &Value) {
        match v {
            Value::String(s) => {
                if !CODE_KEYS.contains(&key) {
                    self.text(at, key, s);
                }
            }
            Value::Number(n) => {
                let x = n.as_f64().unwrap_or(f64::NAN);
                if key == "seed" {
                    // A random seed is any 64-bit number.
                } else if !x.is_finite() || x.abs() > 1e15 {
                    self.problem(format!("{at}.{key}: absurd number {n}"));
                } else if DATE_KEYS.contains(&key) && n.is_i64() {
                    let d = n.as_i64().unwrap_or(0);
                    if d < -40_000 || d > i64::from(self.today) + 365 * 25 || d == 0 {
                        self.problem(format!("{at}.{key}: implausible date {d} (today {})", self.today));
                    }
                } else if MONEY_KEYS.contains(&key) && x.abs() > 3e10 {
                    self.problem(format!("{at}.{key}: implausible money {x}"));
                }
            }
            Value::Array(a) => {
                for x in a {
                    self.walk(at, key, x);
                }
            }
            Value::Object(o) => {
                self.reference(at, o);
                for (k, x) in o {
                    if let Value::String(t) = x
                        && t.trim().is_empty()
                        && TEXT_KEYS.contains(&k.as_str())
                        && at != "world.status"
                    {
                        let ctx: String = Value::Object(o.clone()).to_string().chars().take(240).collect();
                        self.problem(format!("{at}.{k}: empty text in {ctx}"));
                    }
                    // An `s`/`t` inside a cell or part is text; `r`, `n` and `u` are not.
                    self.walk(at, k, x);
                }
            }
            _ => {}
        }
    }

    fn reference(&mut self, at: &str, o: &serde_json::Map<String, Value>) {
        let (Some(Value::String(k)), Some(Value::Number(id))) = (o.get("k"), o.get("id")) else { return };
        let id = id.as_u64().unwrap_or(u64::MAX);
        let l = self.limits;
        let bound = match k.as_str() {
            "person" => Some(l.people),
            "club" => Some(l.clubs),
            "comp" => Some(l.comps),
            "nation" => Some(l.nations),
            "team" => Some(l.teams),
            _ => None,
        };
        if let Some(b) = bound
            && id >= u64::from(b)
        {
            self.problem(format!("{at}: a {k} reference to {id} but only {b} exist"));
        }
        if let Some(Value::String(name)) = o.get("name") {
            let n = name.trim();
            if n.is_empty() || n == "?" || n.eq_ignore_ascii_case("unknown") || n.chars().all(|c| c.is_ascii_digit() || c == '#') || n.starts_with("Person ") || n.starts_with("Club ") && n[5..].chars().all(|c| c.is_ascii_digit()) {
                self.problem(format!("{at}: a {k} {id} that does not resolve to a name ({name:?})"));
            }
        }
    }

    /// Text a reader sees, from `at`, under `key`.
    fn text(&mut self, at: &str, key: &str, s: &str) {
        if s.trim().is_empty() {
            // Empty text is reported by `walk`, which can show the object around it.
            return;
        }
        // `t` runs of a sentence made of parts are fragments: they may begin or end with a space that joins them to a link.
        let fragment = key == "t";
        let identifier_ok = key == "handle" || s.starts_with('@');
        // "None" alone is a value a row can have ("Release clause: None"), not leaked debug output.
        if s == "None" {
            return;
        }
        if let Some(why) = text_defect(if fragment { s.trim() } else { s }) {
            if !(identifier_ok && why.starts_with("an identifier")) {
                self.problem(format!("{at}.{key}: {why}: {s:?}"));
            }
        }
        if s.chars().count() > 12 && (TEXT_KEYS.contains(&key) || key == "s" || key == "t") {
            self.lines.entry(at.to_string()).or_default().insert(s.to_string());
        }
    }

    /// The same line twice in one response, where the reader would see it twice.
    fn duplicates(&mut self, at: &str, v: &Value) {
        match v {
            Value::Array(a) => {
                let mut seen: BTreeMap<(String, String), usize> = BTreeMap::new();
                for x in a {
                    if let Value::Object(o) = x {
                        if o.get("author").is_some_and(|a| a["kind"] == "Held back") {
                            continue;
                        }
                        for key in ["headline", "text", "preview", "title"] {
                            if let Some(Value::String(s)) = o.get(key)
                                && s.chars().count() > 28
                            {
                                let date = o.get("date").or_else(|| o.get("last")).or_else(|| o.get("made")).map(|d| d.to_string()).unwrap_or_default();
                                let who = o.get("from").or_else(|| o.get("with")).or_else(|| o.get("author")).map(|d| d.to_string()).unwrap_or_default();
                                *seen.entry((format!("{key}|{date}|{who}"), s.clone())).or_default() += 1;
                            }
                        }
                    }
                    self.duplicates(at, x);
                }
                for ((k, s), n) in seen {
                    if n > 1 {
                        self.problem(format!("{at}: the same {} appears {n} times: {s:?}", k.split('|').next().unwrap_or("line")));
                    }
                }
            }
            Value::Object(o) => {
                for x in o.values() {
                    self.duplicates(at, x);
                }
            }
            _ => {}
        }
    }
}

/// What is wrong with a line of text a reader sees, if anything.
fn text_defect(s: &str) -> Option<String> {
    if s != s.trim() {
        return Some("leading or trailing space".into());
    }
    for bad in ["{", "}", "$", "\u{fffd}", "Some(", "None)", "::", "{:?", "Option<", "Vec<", "Id(", "NaN", "Infinity", "[object", "\\n", "<br", "&amp;", "&#"] {
        if s.contains(bad) {
            return Some(format!("placeholder or debug residue {bad:?}"));
        }
    }
    let words: Vec<&str> = s.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '_' && c != '-').map(|w| w.trim_matches('-')).filter(|w| !w.is_empty()).collect();
    for w in &words {
        if matches!(*w, "undefined" | "null" | "None" | "NaN" | "inf" | "TODO" | "TBD" | "XXX" | "nan") {
            return Some(format!("the word {w:?}"));
        }
        if w.len() > 3 && w.contains('_') && w.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            return Some(format!("an identifier {w:?}"));
        }
    }
    for p in s.split_whitespace().collect::<Vec<_>>().windows(2) {
        if p[0].eq_ignore_ascii_case(p[1]) && p[0].len() > 1 && p[0].chars().all(char::is_alphabetic) && !matches!(p[0].to_lowercase().as_str(), "had" | "that" | "very" | "bye" | "no" | "so" | "ha") {
            return Some(format!("the word {:?} twice in a row", p[0]));
        }
    }
    if s.contains("  ") || s.contains(" ,") || s.contains(" .") || s.contains(" !") || s.contains(" ?") || s.contains(",,") || s.contains(" ;") || s.contains(" :") && !s.contains(" : ") {
        return Some("spacing or punctuation".into());
    }
    if let Some(i) = s.find("..")
        && !s[i..].starts_with("...")
    {
        return Some("a doubled full stop".into());
    }
    if s.contains("(s)") || s.contains("(es)") {
        return Some("an unresolved plural \"(s)\"".into());
    }
    // "1 days", "0 day", "2 week": a number agreeing with its noun.
    let spaced: Vec<&str> = s.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '.')).collect();
    for p in spaced.windows(2) {
        let (n, noun) = (p[0], p[1].trim_matches('.').to_lowercase());
        if !n.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let plural = ["days", "weeks", "years", "months", "matches", "games", "goals", "points", "appearances", "players", "hours", "minutes", "seasons", "clubs", "assists", "caps", "trophies"];
        // Singular nouns also work as adjectives ("a 2 match ban", "5 match ratings"), so only nouns that do not are checked after a number.
        let singular = ["day", "week", "month", "assist", "trophy", "minute", "hour"];
        if n == "1" && plural.contains(&noun.as_str()) {
            return Some(format!("\"1 {noun}\""));
        }
        if n != "1" && singular.contains(&noun.as_str()) {
            return Some(format!("\"{n} {noun}\""));
        }
    }
    // a/an.
    for p in words.windows(2) {
        let (art, next) = (p[0], p[1]);
        let first = next.chars().next().unwrap_or('x');
        if !next.chars().all(|c| c.is_lowercase() || c == '\'') {
            continue;
        }
        let lower_vowel_sound = "aeio".contains(first) || (first == 'u' && !["university", "universities", "unit", "united", "union", "useful", "usual", "uniform", "unique", "user", "use", "used", "uefa"].contains(&next));
        if art == "a" && lower_vowel_sound && !["once", "one", "eu"].contains(&next) {
            return Some(format!("\"a {next}\""));
        }
        if art == "an" && !lower_vowel_sound && !["hour", "honest", "honour", "honours", "heir", "hourly"].contains(&next) && first != 'h' {
            return Some(format!("\"an {next}\""));
        }
    }
    // The same sentence twice.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for sent in s.split(['.', '!', '?']) {
        let t = sent.trim().to_lowercase();
        if t.chars().count() >= 14 && !seen.insert(t.clone()) {
            return Some(format!("a sentence said twice ({t:?})"));
        }
    }
    None
}

// ------------------------------------------------------------------------------------------------ the tables and pages

const TABLES: &[&str] = &[
    "players", "staff", "clubs", "nations", "comps", "standings", "fixtures", "player_stats", "comp_stats", "events", "transfers", "honours", "awards", "spells", "stories", "agents", "talks", "bids",
    "intl_matches", "tournaments", "boards", "sponsors", "posts", "chants", "memes", "groups", "rivalries", "incidents", "conferences", "quotes", "referees", "controversies", "charges", "record_book",
    "records_broken", "votes", "hall_members", "chronicle", "schools", "rule_changes", "institutions", "minor_seasons", "outlets", "journalists", "grapevine",
];

fn table_filters(table: &str, comp: u32, person: u32) -> Value {
    match table {
        "standings" | "fixtures" | "comp_stats" => json!({"comp": comp}),
        "player_stats" | "spells" => json!({"person": person}),
        _ => json!({}),
    }
}

impl Bot {
    /// Every table: the default page, a second page, every column, and sorted both ways by the first sortable column.
    fn tables(&mut self, api: &Api, comp: u32, person: u32) {
        for &t in TABLES {
            let f = table_filters(t, comp, person);
            let Some(first) = self.q(api, "table.query", json!({"table": t, "filters": f, "limit": 40})) else { continue };
            let cols = first["all_columns"].as_array().cloned().unwrap_or_default();
            let ncols = first["columns"].as_array().map_or(0, Vec::len);
            if cols.is_empty() || ncols == 0 {
                self.problem(format!("table {t} has no columns"));
            }
            for row in first["rows"].as_array().cloned().unwrap_or_default() {
                if row["cells"].as_array().map_or(0, Vec::len) != ncols {
                    self.problem(format!("table {t}: a row with the wrong number of cells"));
                }
                if let Some(r) = row.get("open") {
                    self.reference(&format!("table {t} row"), r.as_object().unwrap());
                    if r["k"] == "match" {
                        self.match_pages(api, r["id"].as_u64().unwrap_or(0));
                    }
                }
            }
            if first["total"].as_u64().unwrap_or(0) > 40 {
                self.q(api, "table.query", json!({"table": t, "filters": f, "limit": 40, "offset": 40}));
            }
            let keys: Vec<Value> = cols.iter().map(|c| c["key"].clone()).collect();
            if let Some(c) = cols.iter().find(|c| c["sortable"].as_bool().unwrap_or(false)) {
                for desc in [true, false] {
                    self.q(api, "table.query", json!({"table": t, "filters": f, "limit": 25, "columns": keys, "sort": {"key": c["key"], "desc": desc}}));
                }
            }
        }
    }

    fn match_pages(&mut self, api: &Api, uid: u64) {
        if uid == 0 || !self.checked_matches.insert(uid) || self.checked_matches.len() > 400 {
            return;
        }
        self.maybe(api, "match", json!({"uid": uid}));
        self.maybe(api, "match.watch", json!({"uid": uid}));
        self.maybe(api, "insight.match", json!({"uid": uid}));
    }

    /// The pages of the world itself, whoever is looking.
    fn world_pages(&mut self, api: &Api, round: usize, deep: bool) {
        self.q(api, "app.info", json!({}));
        self.q(api, "world.status", json!({}));
        self.q(api, "overview", json!({}));
        self.q(api, "world.pulse", json!({}));
        self.q(api, "diagnostics", json!({}));
        self.q(api, "capabilities", json!({}));
        self.q(api, "crest.colors", json!({}));
        self.q(api, "world.saves", json!({}));
        for f in ["for_you", "following", "world"] {
            if let Some(feed) = self.q(api, "news.feed", json!({"filter": f, "limit": 40})) {
                let stories = feed["stories"].as_array().cloned().unwrap_or_default();
                let take = if deep { 12 } else { 4 };
                for s in stories.iter().skip(round % 3).step_by(2).take(take) {
                    if let Some(st) = self.q(api, "news.story", json!({"id": s["id"]})) {
                        let _ = st;
                    }
                    if let Some(uid) = s["graphic"]["match"]["id"].as_u64() {
                        self.match_pages(api, uid);
                    }
                }
            }
        }
        for q in ["a", "united", "ko", "raj"] {
            self.q(api, "search", json!({"q": q}));
        }
        self.q(api, "ecosystem.regions", json!({}));
        self.q(api, "ecosystem.export", json!({}));
        self.q(api, "ecosystem.scenario", json!({}));
        let l = self.limits;
        let comps: Vec<u32> = if deep { (0..l.comps).collect() } else { (0..l.comps).skip(round % 3).step_by(3).take(5).collect() };
        for c in &comps {
            self.q(api, "comp", json!({"id": c}));
            self.q(api, "comp.overview", json!({"id": c}));
            self.q(api, "insight.comp", json!({"id": c}));
        }
        let clubs: Vec<u32> = if deep { (0..l.clubs).collect() } else { (0..l.clubs).skip(round % 7).step_by(9).take(8).collect() };
        for c in &clubs {
            self.q(api, "club", json!({"id": c}));
            self.q(api, "club.systems", json!({"id": c}));
            self.q(api, "insight.club", json!({"id": c}));
        }
        let nations: Vec<u32> = if deep { (0..l.nations).collect() } else { (0..l.nations).take(3).collect() };
        for n in &nations {
            self.q(api, "nation", json!({"id": n}));
        }
    }

    /// Pages about one person, from whoever's eyes are open.
    fn person_pages(&mut self, api: &Api, id: u32) {
        self.q(api, "person", json!({"id": id}));
        self.q(api, "person.attributes", json!({"id": id}));
        self.q(api, "insight.person", json!({"id": id}));
        self.maybe(api, "pathway.player", json!({"id": id}));
        self.maybe(api, "person.life", json!({"id": id}));
    }

    /// Everything only an inhabited person has.
    fn me_pages(&mut self, api: &Api, round: usize, deep: bool) -> Option<Value> {
        let today = self.q(api, "me.today", json!({}))?;
        self.q(api, "me.self", json!({}));
        self.q(api, "me.life", json!({}));
        self.q(api, "me.people", json!({}));
        self.q(api, "me.promises", json!({}));
        self.q(api, "me.rumours", json!({}));
        self.q(api, "me.press", json!({}));
        self.q(api, "me.agent", json!({}));
        self.q(api, "me.journal", json!({}));
        self.q(api, "me.football", json!({}));
        self.q(api, "me.options", json!({}));
        self.maybe(api, "me.contract", json!({}));
        let d = self.today;
        self.q(api, "me.calendar", json!({}));
        self.q(api, "me.calendar", json!({"from": d + 30, "to": d + 100}));
        self.q(api, "me.calendar", json!({"from": d - 40, "to": d}));
        if let Some(feed) = self.q(api, "me.feed", json!({"limit": 60})) {
            let posts = feed["posts"].as_array().cloned().unwrap_or_default();
            for p in posts.iter().take(if deep { 10 } else { 3 }) {
                self.q(api, "social.thread", json!({"id": p["id"]}));
            }
        }
        if let Some(m) = self.q(api, "me.messages", json!({"limit": 200})) {
            let msgs = m["messages"].as_array().cloned().unwrap_or_default();
            for x in msgs.iter().skip(round % 4).step_by(3).take(if deep { 14 } else { 5 }) {
                self.q(api, "me.message", json!({"id": x["id"]}));
            }
        }
        if let Some(inb) = self.q(api, "me.inbox", json!({"limit": 120})) {
            let threads = inb["threads"].as_array().cloned().unwrap_or_default();
            for t in threads.iter().take(if deep { 14 } else { 6 }) {
                self.q(api, "me.thread", json!({"id": t["id"]}));
                self.q(api, "me.thread_read", json!({"id": t["id"]}));
            }
        }
        // The story behind something the press said about me.
        if let Some(p) = self.q(api, "me.press", json!({})) {
            for s in p["stories"].as_array().cloned().unwrap_or_default().iter().take(3) {
                self.maybe(api, "me.story", json!({"id": s["id"]}));
            }
        }
        for f in today["recent"].as_array().cloned().unwrap_or_default() {
            if let Some(uid) = f["uid"].as_u64() {
                self.match_pages(api, uid);
            }
        }
        if let Some(uid) = today["next_match"]["uid"].as_u64() {
            self.match_pages(api, uid);
        }
        self.q(api, "me.viewed", json!({}));
        Some(today)
    }
}

// ------------------------------------------------------------------------------------------------ deciding and acting

impl Bot {
    /// Answer every decision that is waiting and reply to messages that offer replies. Returns how many things were answered.
    fn answer_everything(&mut self, api: &Api, turn: usize) -> usize {
        let mut done = 0;
        let Some(inb) = self.q(api, "me.inbox", json!({"limit": 200})) else { return 0 };
        let threads = inb["threads"].as_array().cloned().unwrap_or_default();
        for t in threads.iter().take(40) {
            let Some(th) = self.q(api, "me.thread", json!({"id": t["id"]})) else { continue };
            for m in th["messages"].as_array().cloned().unwrap_or_default() {
                if let Some(d) = m.get("decision").filter(|d| d["state"] == "awaiting") {
                    let n = d["options"].as_array().map_or(0, Vec::len);
                    if n == 0 {
                        self.problem(format!("decision {} ({}) has no options", d["id"], d["title"]));
                        continue;
                    }
                    // Alternate between the suggested default and rotating through the others.
                    let pick = if (turn + done) % 3 == 0 { d["options"].as_array().and_then(|o| o.iter().position(|x| x["default"] == true)).unwrap_or(0) } else { (turn + done) % n };
                    if self.q(api, "me.answer", json!({"id": d["id"], "choice": pick})).is_some() {
                        done += 1;
                        // (An answer can be changed until the day ends, so answering again is allowed.)
                    }
                } else if let Some(r) = m["replies"].as_array().and_then(|r| r.get((turn + done) % r.len().max(1))).filter(|_| m["replied"].is_null()) {
                    if self.maybe(api, "me.reply", json!({"message": m["id"], "key": r["key"]})).is_some() {
                        done += 1;
                    }
                }
            }
        }
        done
    }

    /// Everything a person can do from the action list, each once in a while.
    fn act_on_options(&mut self, api: &Api, turn: usize, me: u32) {
        let Some(o) = self.q(api, "me.options", json!({})) else { return };
        let pick = |arr: &Value, i: usize| arr.as_array().and_then(|a| if a.is_empty() { None } else { Some(a[i % a.len()].clone()) });
        let mut actions: Vec<Value> = Vec::new();
        if let (Some(w), Some(t), Some(tone)) = (pick(&o["meet_with"], turn), pick(&o["topics"], turn), pick(&o["tones"], turn)) {
            actions.push(json!({"action": "meet", "with": w["who"]["id"], "topic": t["key"], "tone": tone["key"]}));
        }
        actions.push(json!({"action": "routine", "hours": {"rest": 8 + (turn % 3), "study": 2}}));
        if let Some(l) = pick(&o["lifestyles"], turn) {
            actions.push(json!({"action": "lifestyle", "value": l["key"]}));
        }
        if let Some(s) = pick(&o["stances"], turn) {
            actions.push(json!({"action": "press", "about": me, "stance": s["key"]}));
        }
        if let Some(p) = pick(&o["posts"], turn) {
            actions.push(json!({"action": "post", "concept": p["key"], "about": me}));
        }
        if let Some(c) = pick(&o["courses"], turn) {
            actions.push(json!({"action": "enrol", "course": c["key"]}));
        }
        if let Some(h) = pick(&o["helpers"], turn) {
            actions.push(json!({"action": "helper", "helper": h["key"], "quality": 6}));
            actions.push(json!({"action": "dismiss_helper", "helper": h["key"]}));
        }
        if let Some(a) = pick(&o["agents"], turn) {
            actions.push(json!({"action": "hire_agent", "agent": a["id"]}));
        }
        actions.push(json!({"action": "drop_agent"}));
        actions.push(json!({"action": "dating", "open": turn % 2 == 0}));
        actions.push(json!({"action": "pain", "on": turn % 2 == 1}));
        actions.push(json!({"action": "giving", "pct": 5, "community": 4}));
        actions.push(json!({"action": "move_home", "buy": false, "quality": 2}));
        actions.push(json!({"action": "invest", "amount": 100, "risk": 5}));
        actions.push(json!({"action": "transfer_request"}));
        actions.push(json!({"action": "withdraw_request"}));
        if let Some(r) = pick(&o["roles"], turn) {
            actions.push(json!({"action": "seek_job", "role": r["key"]}));
        }
        if let Some(n) = pick(&o["nations"], turn) {
            actions.push(json!({"action": "nation", "nation": n["id"]}));
        }
        actions.push(json!({"action": "amateur"}));
        actions.push(json!({"action": "mentor", "person": me + 1}));
        for a in actions {
            if let Some(r) = self.maybe(api, "me.act", a.clone()) {
                if r["text"].as_str().is_none_or(str::is_empty) {
                    self.problem(format!("me.act {a} queued something with no sentence"));
                }
            }
        }
        // A training plan and the notebook.
        self.maybe(api, "me.plan", json!({"intensity": INTENSITY[turn % 3], "extra": turn % 3, "recovery": 2}));
        self.q(api, "me.goal", json!({"kind": "personal", "text": format!("Play every week of block {turn}"), "target": 5}));
        self.q(api, "me.note", json!({"text": format!("Note after {turn} stops")}));
        if let Some(j) = self.q(api, "me.journal", json!({})) {
            if let Some(n) = j["notes"].as_array().map(Vec::len).filter(|n| *n > 3) {
                self.q(api, "me.note_remove", json!({"i": n - 1}));
            }
        }
        self.q(api, "settings.set", json!({"conceal_mine": false, "stops": {"decisions": true, "matches": false, "major": true}}));
        self.q(api, "club.follow", json!({"club": (turn % 5) as u32, "follow": turn % 2 == 0}));
        // The queued list answers for what was sent.
        self.q(api, "me.today", json!({}));
        // Bad requests are refused with a plain sentence and never crash.
        for (m, a) in [("me.act", json!({"action": "no_such"})), ("me.answer", json!({"id": "d99999999", "choice": 0})), ("me.thread", json!({"id": 99999999})), ("person", json!({"id": 99999999})), ("no.such", json!({}))] {
            match api.call(m, a.clone()) {
                Ok(v) => self.problem(format!("{m} {a} accepted nonsense: {v}")),
                Err(e) => {
                    if e.to_string().trim().len() < 3 {
                        self.problem(format!("{m} {a} refused without a message"));
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ the firewall by non-interference

/// Change everything hidden: true ability and potential, others' personalities, how everyone feels about the viewer, the leanings of
/// journalists and the numbers of referees.
fn perturb_hidden(api: &Api, me: PersonId) {
    api.debug_mutate_world(|w| {
        for c in w.players.cold.iter_mut() {
            let ca = c.ca;
            c.ca = 20 + (u16::from(c.pa) * 7 % 150) as u8;
            c.pa = c.ca.max(ca) + 10;
        }
        for (id, p) in w.people.iter_enumerated_mut() {
            if id != me {
                for h in pw_core::Hidden::ALL {
                    let v = p.hidden.get(h);
                    p.hidden.set(h, 20 - v.min(20));
                }
            }
        }
        let today = w.date;
        let pairs: Vec<(PersonId, PersonId)> = w.social.endpoints().filter(|&(_, b)| b == me).collect();
        for (a, b) in pairs {
            w.social.adjust(a, b, today, 0, -80, -40, -40);
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

fn snapshot(api: &Api, me: u32, others: &[u32]) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let mut put = |name: String, m: &str, args: Value| {
        if let Ok(v) = api.call(m, args) {
            out.insert(name, v);
        }
    };
    for &p in others.iter().chain([&me]) {
        for m in ["person", "person.attributes", "insight.person", "pathway.player", "person.life"] {
            put(format!("{m}/{p}"), m, json!({"id": p}));
        }
    }
    for c in 0..6u32 {
        for m in ["club", "club.systems", "insight.club"] {
            put(format!("{m}/{c}"), m, json!({"id": c}));
        }
    }
    for c in 0..3u32 {
        for m in ["comp", "comp.overview", "insight.comp"] {
            put(format!("{m}/{c}"), m, json!({"id": c}));
        }
    }
    for m in ["nation", "overview", "world.pulse", "news.feed", "ecosystem.regions", "ecosystem.export", "ecosystem.scenario"] {
        put(m.to_string(), m, json!({"id": 0}));
    }
    put("search".into(), "search", json!({"q": "a"}));
    for m in [
        "me.today", "me.self", "me.life", "me.people", "me.promises", "me.rumours", "me.press", "me.agent", "me.contract", "me.football", "me.options", "me.messages", "me.inbox", "me.feed", "me.journal",
        "me.calendar",
    ] {
        put(m.to_string(), m, json!({}));
    }
    for &t in TABLES {
        let f = table_filters(t, 0, me);
        let Ok(first) = api.call("table.query", json!({"table": t, "filters": f, "limit": 60})) else { continue };
        let cols: Vec<Value> = first["all_columns"].as_array().cloned().unwrap_or_default();
        let keys: Vec<Value> = cols.iter().map(|c| c["key"].clone()).collect();
        out.insert(format!("{t}/default"), first);
        for c in cols.iter().filter(|c| c["sortable"].as_bool().unwrap_or(false)) {
            let key = c["key"].as_str().unwrap();
            for desc in [true, false] {
                if let Ok(v) = api.call("table.query", json!({"table": t, "filters": f, "limit": 30, "columns": keys, "sort": {"key": key, "desc": desc}})) {
                    out.insert(format!("{t}/sort:{key}:{}", if desc { "desc" } else { "asc" }), v);
                }
            }
        }
    }
    out
}

fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    if out.len() >= 4 {
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
        _ if a != b => {
            let cut = |v: &Value| v.to_string().chars().take(70).collect::<String>();
            out.push(format!("{path}: {} -> {}", cut(a), cut(b)))
        }
        _ => {}
    }
}

impl Bot {
    /// Hidden truth moves; nothing the inhabited person can read may.
    fn firewall(&mut self, api: &Api, me: u32) {
        let others: Vec<u32> = vec![1, 2, me.saturating_add(1), me.saturating_add(2), 1500, 2100];
        let others: Vec<u32> = others.into_iter().filter(|&p| p < self.limits.people && p != me).collect();
        let before = snapshot(api, me, &others);
        perturb_hidden(api, PersonId(me));
        let after = snapshot(api, me, &others);
        if before.len() < 300 {
            self.problem(format!("the firewall audit covered only {} pages and lists", before.len()));
        }
        for (k, a) in &before {
            let mut d = Vec::new();
            diff(a, after.get(k).unwrap_or(&Value::Null), "", &mut d);
            if !d.is_empty() {
                self.problem(format!("firewall: {k} moved when only hidden truth did: {}", d.join(" | ")));
            }
        }
        // And the plain structural promises while inhabiting someone.
        if let Some(a) = self.q(api, "person.attributes", json!({"id": others.first().copied().unwrap_or(1)})) {
            if !a["hidden"].is_null() || !a["internal"].is_null() || !a["personality"].is_null() {
                self.problem("an inhabited person sees another's hidden attributes".into());
            }
        }
        if let Some(t) = self.q(api, "table.query", json!({"table": "players", "limit": 3})) {
            let cols: Vec<String> = t["all_columns"].as_array().cloned().unwrap_or_default().iter().map(|c| c["key"].as_str().unwrap_or("").to_string()).collect();
            for hidden in ["ca", "pa", "wage", "value", "contract_end"] {
                if cols.iter().any(|c| c == hidden) {
                    self.problem(format!("the players table offers the hidden column {hidden} to an inhabited person"));
                }
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ the playthrough

struct Run {
    api: Api,
    bot: Bot,
    me: u32,
    turn: usize,
    answered: usize,
    stops_seen: BTreeSet<String>,
}

impl Run {
    fn new(seed: u64) -> Self {
        let api = api();
        api.call("world.new", json!({"kind": "india", "scale": "tiny", "seed": seed})).unwrap();
        wait(&api, "task");
        let mut bot = Bot::default();
        bot.refresh(&api);
        Run { api, bot, me: 0, turn: 0, answered: 0, stops_seen: BTreeSet::new() }
    }

    fn begin(&mut self, start: &str, district_ix: usize) -> bool {
        self.bot.at = format!("{start} begin");
        let o = self.bot.q(&self.api, "route.options", json!({})).expect("route options");
        assert_eq!(o["available"], true);
        let districts: Vec<u64> = o["states"].as_array().unwrap().iter().flat_map(|s| s["districts"].as_array().unwrap().iter().map(|d| d["id"].as_u64().unwrap())).collect();
        let district = districts[district_ix % districts.len()];
        for (i, d) in std::iter::once(district).chain(districts.iter().copied()).enumerate() {
            if i > 12 {
                break;
            }
            match self.api.call("route.begin", json!({"start": start, "district": d})) {
                Ok(v) => {
                    self.me = v["person"].as_u64().unwrap() as u32;
                    self.bot.refresh(&self.api);
                    return true;
                }
                Err(e) => {
                    let m = e.to_string();
                    if !m.contains("no school, university or club") {
                        self.bot.problem(format!("route.begin {start} refused for another reason: {m}"));
                        return false;
                    }
                }
            }
        }
        false
    }

    /// One stop: read everything, answer and act, read again.
    fn stop(&mut self, label: &str, deep: bool) {
        self.bot.at = format!("{label} (day {})", self.api.call("world.status", json!({})).map_or(0, |s| s["date"].as_i64().unwrap_or(0)));
        self.bot.refresh(&self.api);
        self.turn += 1;
        let t = self.turn;
        let me = self.me;
        self.bot.world_pages(&self.api, t, deep);
        self.bot.me_pages(&self.api, t, deep);
        self.bot.person_pages(&self.api, me);
        let (mut others, l) = (vec![], self.bot.limits);
        for k in 0..if deep { 14 } else { 4 } {
            others.push(((t as u32 * 37 + k * 211) % l.people).max(1));
        }
        for p in others {
            self.bot.person_pages(&self.api, p);
        }
        if deep || t % 4 == 1 {
            self.bot.tables(&self.api, (t as u32) % l.comps.max(1), me);
        }
        self.answered += self.bot.answer_everything(&self.api, t);
        self.bot.act_on_options(&self.api, t, me);
        // Whatever was sent shows on the next reading.
        self.bot.me_pages(&self.api, t + 1, false);
    }

    /// Advance and report why it stopped. A decision that stops the clock is answered on the spot.
    fn advance(&mut self, args: Value) {
        self.bot.at = format!("advance {args}");
        let r = self.api.call("advance.start", args.clone());
        if let Err(e) = r {
            self.bot.problem(format!("advance.start {args} failed: {e}"));
            return;
        }
        let job = wait(&self.api, "job");
        if let Some(s) = job["stop"].as_object() {
            self.stops_seen.insert(s["kind"].as_str().unwrap_or("").to_string());
            self.bot.read("job.stop", &json!({"text": s["text"]}));
        }
        self.bot.refresh(&self.api);
    }

    fn save_and_reload(&mut self) {
        self.bot.at = "save and reload".into();
        let names = ["today", "messages", "inbox", "life", "football", "contract", "people", "journal", "press"];
        let read = |api: &Api| -> Vec<Value> { names.iter().map(|n| api.call(&format!("me.{n}"), json!({})).unwrap_or(Value::Null)).collect() };
        let status = |api: &Api| {
            let mut s = api.call("world.status", json!({})).unwrap();
            for k in ["revision", "job", "task"] {
                s.as_object_mut().unwrap().remove(k);
            }
            s
        };
        let (before, st0) = (read(&self.api), status(&self.api));
        let saved = self.bot.q(&self.api, "world.save", json!({"file": "playthrough"}));
        if saved.is_none() {
            return;
        }
        if let Some(list) = self.bot.q(&self.api, "world.saves", json!({})) {
            if !list["saves"].as_array().is_some_and(|s| s.iter().any(|x| x["file"] == "playthrough.pws")) {
                self.bot.problem("a save just made is not in the list of saves".into());
            }
        }
        self.api.call("world.load", json!({"file": "playthrough.pws"})).unwrap();
        wait(&self.api, "task");
        let (after, st1) = (read(&self.api), status(&self.api));
        if st0 != st1 {
            self.bot.problem(format!("the status changed over a save and reload: {st0} -> {st1}"));
        }
        for (n, (a, b)) in names.iter().zip(before.iter().zip(after.iter())) {
            let mut d = Vec::new();
            diff(a, b, "", &mut d);
            if !d.is_empty() {
                self.bot.problem(format!("me.{n} changed over a save and reload: {}", d.join(" | ")));
            }
        }
        self.bot.refresh(&self.api);
    }

    fn finish(mut self, dump: bool) {
        // The last look at the whole world, then the same world through the public's eyes.
        self.bot.at = "final sweep".into();
        self.bot.world_pages(&self.api, 0, true);
        self.bot.at = "public view".into();
        self.api.call("persp.observe", json!({"public": true})).unwrap();
        self.bot.world_pages(&self.api, 1, false);
        self.bot.person_pages(&self.api, self.me);
        self.bot.tables(&self.api, 0, self.me);
        self.bot.at = "observer view".into();
        self.api.call("persp.observe", json!({})).unwrap();
        self.bot.world_pages(&self.api, 2, false);
        self.bot.person_pages(&self.api, self.me);
        if let Ok(path) = std::env::var("PW_TEXT_DUMP") {
            if dump {
                let mut out = String::new();
                for (at, lines) in &self.bot.lines {
                    out.push_str(&format!("### {at}\n"));
                    for l in lines {
                        out.push_str(l);
                        out.push('\n');
                    }
                }
                let _ = std::fs::write(path, out);
            }
        }
        let n = self.bot.problems.len();
        let _ = std::fs::write(std::env::temp_dir().join("pw-playthrough-problems.txt"), self.bot.problems.join("
"));
        assert!(n == 0, "{n} defects in {} calls ({} decisions and replies answered):\n  {}", self.bot.calls, self.answered, self.bot.problems.join("\n  "));
    }
}

const INTENSITY: [&str; 3] = ["light", "normal", "high"];
const STARTS: [&str; 6] = ["school_standout", "released_academy", "university_freshman", "university_star", "state_league", "semi_pro"];

fn play(seed: u64, starts: &[&str], rounds: &[u32], deep: bool) {
    let mut run = Run::new(seed);
    let mut began = 0;
    let only = std::env::var("PW_ONLY").ok();
    for (i, &start) in starts.iter().enumerate() {
        if only.as_deref().is_some_and(|o| o != start) {
            continue;
        }
        if !run.begin(start, i * 3 + seed as usize) {
            continue;
        }
        began += 1;
        run.stop(&format!("{start} at the start"), deep);
        for (r, &days) in rounds.iter().enumerate() {
            run.advance(json!({"mode": "days", "n": days}));
            run.stop(&format!("{start} after {days} days (round {r})"), deep && r % 2 == 0);
            if r % 3 == 2 {
                run.advance(json!({"mode": "until_event", "max_days": 40}));
                run.stop(&format!("{start} at an event (round {r})"), false);
            }
            if r == 1 {
                run.advance(json!({"mode": "until_match"}));
                run.stop(&format!("{start} after a match"), false);
            }
        }
        run.bot.at = format!("{start} firewall");
        let me = run.me;
        if i % 2 == 0 {
            run.bot.firewall(&run.api, me);
        }
        run.save_and_reload();
        run.stop(&format!("{start} after reloading"), false);
    }
    assert!(only.is_some() || began >= starts.len().min(3), "only {began} of {} starts could be begun", starts.len());
    run.finish(true);
}

/// Every start, a few weeks and a season's worth of stops.
#[test]
fn a_short_career_from_every_start_reads_well() {
    play(7, &STARTS, &[6, 7, 21, 60], false);
}

/// Three seasons from every start, with deep sweeps. Slow.
#[test]
#[ignore = "long: three seasons from every start"]
fn three_seasons_from_every_start_read_well() {
    let weeks: Vec<u32> = std::iter::repeat_n(30, 36).collect();
    play(11, &STARTS, &weeks, true);
}

/// The text checks themselves: the defects they exist to catch are caught, and ordinary writing passes.
#[test]
fn the_text_checks_catch_what_they_should() {
    for bad in [
        "Signed {player} from Rovers",
        "He scored $3 goals",
        "He was out for 1 days",
        "He was out for 2 week",
        "The club is hoping for a a win",
        "an team of its own",
        "a injury kept him out",
        "Nothing happened.  Twice",
        "He said null",
        "The club signed him last week. The club signed him last week.",
        "Rovers lose , again",
        "Suspended for 2 match(es)",
        " padded",
        "see school_standout now",
    ] {
        assert!(text_defect(bad).is_some(), "not caught: {bad:?}");
    }
    for good in [
        "Kiran signed for Rovers on a three-year deal.",
        "An hour later the university side had a unique chance...",
        "Asha scored 2 goals in 1 game.",
        "She was out for 1 day; he, 2 weeks.",
        "A European tie and a one-off, a useful union.",
    ] {
        assert_eq!(text_defect(good), None, "wrongly flagged: {good:?}");
    }
}
