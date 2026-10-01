//! Property test: every event × channel × voice × seed × random knowledge state renders without a single checker issue, never states a fact more
//! firmly than the speaker holds it, and never shows a fact the speaker does not know.

use pw_core::Date;
use pw_lang::model::{Descriptor, Know, Ref, Speaker, Value};
use pw_lang::{Certainty, Engine, Event, Request, Tracker, check};
use std::collections::BTreeMap;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn club(i: usize) -> Value {
    const C: [(&str, &str, &str, bool); 5] = [
        ("club.mohun-bagan-sg", "Mohun Bagan SG", "Mohun Bagan", true),
        ("club.kerala-blasters", "Kerala Blasters", "Blasters", true),
        ("club.bengaluru-fc", "Bengaluru FC", "Bengaluru", true),
        ("club.east-bengal", "East Bengal", "East Bengal", true),
        ("club.sudeva-delhi", "Sudeva Delhi FC", "Sudeva", true),
    ];
    let (id, full, short, plural) = C[i % C.len()];
    Value::Ent(Ref { id: id.into(), full: full.into(), short: short.into(), descriptors: vec![], plural })
}

fn person(i: usize) -> Value {
    const P: [(&str, &str, &str, &str, &str); 3] = [
        ("person.rahul-kumar", "Rahul Kumar", "Rahul", "19-year-old winger", "winger"),
        ("person.asha-devi", "Asha Devi", "Asha", "22-year-old midfielder", "midfielder"),
        ("person.manolo-gomez", "Manolo Gomez", "Gomez", "Spanish", "head coach"),
    ];
    let (id, full, short, d1, d2) = P[i % P.len()];
    Value::Ent(Ref {
        id: id.into(),
        full: full.into(),
        short: short.into(),
        descriptors: vec![Descriptor { kind: "age_role".into(), text: d1.into() }, Descriptor { kind: "role".into(), text: d2.into() }],
        plural: false,
    })
}

/// Alternatives for facts whose value changes the wording; everything else has one sample.
fn alternatives(key: &str) -> Vec<Value> {
    let t = |s: &str| Value::Text(s.into());
    match key {
        "manner" => vec![t("sacked"), t("mutual"), t("resigned")],
        "offer_kind" => vec![t("trial"), t("admission"), t("scholarship"), t("sports quota")],
        "first_call" | "ours" | "to_player" | "in_match" | "improved" => vec![Value::Bool(true), Value::Bool(false)],
        "fee" => vec![Value::Money { money: 25_000_000 }, Value::Money { money: 0 }, Value::Money { money: 8_500_000 }],
        "round" => vec![Value::Num(1), Value::Num(3)],
        "home_goals" => vec![Value::Num(0), Value::Num(3), Value::Num(1)],
        "away_goals" => vec![Value::Num(0), Value::Num(2), Value::Num(4)],
        "weeks_out" => vec![Value::Num(1), Value::Num(9), Value::Num(20)],
        "matches_seen" => vec![Value::Num(0), Value::Num(2), Value::Num(6)],
        "contract_years" => vec![Value::Num(1), Value::Num(4)],
        "caps" => vec![Value::Num(0), Value::Num(12)],
        _ => vec![],
    }
}

fn sample(ty: &str, key: &str, idx: usize) -> Value {
    let t = |s: &str| Value::Text(s.into());
    match (ty, key) {
        ("club", _) => club(idx),
        ("person", _) => person(idx),
        ("team", _) => Value::Ent(Ref { id: "team.india-men".into(), full: "India".into(), short: "India".into(), descriptors: vec![], plural: true }),
        ("money", "seller_minimum") => Value::Money { money: 45_000_000 },
        ("money", "wage") => Value::Money { money: 2_400_000 },
        ("money", _) => Value::Money { money: 25_000_000 },
        ("num", "position") => Value::Num(8),
        ("num", _) => Value::Num(3),
        ("bool", _) => Value::Bool(true),
        ("list", "scorers") => Value::List(vec!["R. Kumar 12'".into(), "A. Devi 55'".into()]),
        ("list", "concerns") => Value::List(vec!["end product".into(), "aerial duels".into()]),
        ("list", _) => Value::List(vec!["pace".into(), "first touch".into()]),
        ("date", _) => Value::date(Date::from_ymd(2026, 8, 1)),
        ("competition", _) => t("Indian Super League"),
        ("place", _) => t("Kozhikode"),
        (_, "injury") => t("hamstring strain"),
        (_, "camp") => t("preparatory camp"),
        (_, "to_tier") => t("I-League"),
        (_, "level") => t("state league starter"),
        (_, "position") => t("winger"),
        (_, "course") => t("BA"),
        (_, "programme") => t("exchange programme"),
        (_, "stage") => t("probables list"),
        (_, "record") => t("record for most goals in a season"),
        (_, "value") => t("21 goals"),
        (_, "label") => t("Potential"),
        (_, "reason") => t("private reason"),
        (_, "board_reason") => t("private reason"),
        _ => t("sample"),
    }
}

fn facts_for(eng: &Engine, kind: &str, rng: &mut Rng) -> BTreeMap<String, Value> {
    let def = &eng.lang.events[kind];
    let mut out = BTreeMap::new();
    for (i, f) in def.facts.iter().enumerate() {
        // A match moment's `kind` picks which of its frames can be used, so the fuzz tries each.
        let alts = if kind == "match.moment" && f.key == "kind" { vec![Value::Text("late_winner".into()), Value::Text("hat_trick".into()), Value::Text("red_card".into())] } else { alternatives(&f.key) };
        let v = if alts.is_empty() { sample(&f.ty, &f.key, i + rng.below(3)) } else { alts[rng.below(alts.len())].clone() };
        out.insert(f.key.clone(), v);
    }
    // Distinct entities in different roles: two clubs in one event must not be the same club.
    let mut seen = std::collections::BTreeSet::new();
    let mut bump = 0;
    for (k, v) in out.iter_mut() {
        let Value::Ent(r) = v else { continue };
        let is_club = r.id.starts_with("club");
        let mut cur = r.clone();
        while !seen.insert(cur.id.clone()) {
            bump += 1;
            let Value::Ent(n) = (if is_club { club(bump + 1 + k.len()) } else { person(bump + 1) }) else { unreachable!() };
            cur = n;
        }
        *v = Value::Ent(cur);
    }
    out
}

fn random_knowledge(eng: &Engine, ev: &Event, rng: &mut Rng) -> BTreeMap<String, Know> {
    let sources: Vec<&str> = eng.lang.sources.keys().map(String::as_str).collect();
    let mut knows = BTreeMap::new();
    for key in ev.facts.keys() {
        if rng.below(100) < 22 {
            continue;
        }
        let c = match rng.below(100) {
            0..=39 => Certainty::Fact,
            40..=54 => Certainty::VerifiedReport,
            55..=64 => Certainty::SourceClaim,
            65..=74 => Certainty::Rumour,
            75..=79 => Certainty::Speculation,
            80..=84 => Certainty::Denial,
            85..=89 => Certainty::Correction,
            _ => Certainty::Unknown,
        };
        let source = if c == Certainty::Fact || rng.below(5) == 0 { None } else { Some(sources[rng.below(sources.len())].to_string()) };
        knows.insert(key.clone(), Know { certainty: c, source });
    }
    knows
}

#[test]
fn every_state_renders_clean() {
    let eng = Engine::builtin();
    let kinds: Vec<String> = eng.lang.events.keys().cloned().collect();
    let voices: Vec<String> = eng.lang.voices.keys().cloned().collect();
    let channels: Vec<String> = eng.lang.channels.keys().cloned().collect();
    let behaviours = [
        "reaction", "hype", "antihype", "skepticism", "analysis", "joke", "anger", "praise", "nostalgia", "correction", "quote", "cheer", "groan", "grumble", "jibe", "sarcasm", "worry", "ask", "defend", "concede", "reluctant", "hold", "overrated",
        "agree", "disagree",
    ];
    let mut rng = Rng(0x5eed);
    let (mut renders, mut nonempty) = (0usize, 0usize);
    let mut bad: Vec<String> = Vec::new();
    let mut per_pair: BTreeMap<(String, String), usize> = BTreeMap::new();
    for kind in &kinds {
        for _variant in 0..100 {
            let facts = facts_for(&eng, kind, &mut rng);
            let date = Date::from_ymd(2026, 8, 1);
            let ev = Event { kind: kind.clone(), date, facts };
            for _trial in 0..3 {
                let knows = random_knowledge(&eng, &ev, &mut rng);
                let now = Date(date.0 + rng.below(21) as i32 - 10);
                for ch in &channels {
                    let vname = &voices[rng.below(voices.len())];
                    let mut sp = Speaker::new("s", &eng.lang.voices[vname].role, eng.voice(vname));
                    sp.rs_words = eng.lang.voices[vname].rs_words;
                    sp.knows = knows.clone();
                    let beh = behaviours[rng.below(behaviours.len())];
                    let req = Request::new(&ev, &sp, ch, now, rng.next()).behaviour(beh);
                    let r = eng.render(&req, &mut Tracker::new());
                    renders += 1;
                    if !r.parts.is_empty() {
                        nonempty += 1;
                        *per_pair.entry((kind.clone(), ch.clone())).or_default() += 1;
                    }
                    for i in check::check_rendered(&eng.lang, &req, &r) {
                        bad.push(format!("{kind}/{ch} voice={vname} seed={} now={}: {} — {}\n      {:?}", req.seed, now.0 - date.0, i.check, i.detail, r.text()));
                    }
                    // A part is never firmer than the weakest fact it rests on.
                    for p in &r.parts {
                        let f = eng.lang.frames.iter().find(|f| f.id == p.frame).unwrap();
                        if !f.needs.is_empty() {
                            let weakest = Certainty::combine(f.needs.iter().filter_map(|k| sp.knows.get(k).map(|k| k.certainty)));
                            if weakest != p.certainty && !pw_lang_derived(&f.needs) {
                                bad.push(format!("{kind}/{ch}: part {} says {:?} but its facts give {:?}", p.frame, p.certainty, weakest));
                            }
                        }
                    }
                    for o in &r.options {
                        for i in check::check_text(&o.label).into_iter().chain(check::check_text(&o.consequence)) {
                            bad.push(format!("{kind}/{ch} option {}: {} — {}", o.id, i.check, i.detail));
                        }
                    }
                }
            }
        }
    }
    bad.sort();
    bad.dedup();
    let shown: Vec<&String> = bad.iter().take(40).collect();
    assert!(bad.is_empty(), "{} distinct problems in {renders} renders ({nonempty} non-empty):\n{}", bad.len(), shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    assert!(nonempty * 4 > renders / 3, "too few renderings produced text: {nonempty}/{renders}");
}

fn pw_lang_derived(needs: &[String]) -> bool {
    needs.iter().any(|k| matches!(k.as_str(), "winner" | "loser" | "winner_goals" | "loser_goals" | "margin" | "total" | "result_kind"))
}
