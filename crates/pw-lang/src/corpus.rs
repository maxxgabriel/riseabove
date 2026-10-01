//! The test corpus: representative event states with several valid outputs each, checked for consistency, grammar, certainty, leaks and channel fit.
//! Cases live in `data/lang/en/corpus/*.toml`.

use crate::check;
use crate::model::*;
use crate::render::{Engine, Tracker};
use crate::text;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize, Debug, Clone)]
pub struct CaseEvent {
    pub kind: String,
    pub date: String,
    #[serde(default)]
    pub facts: BTreeMap<String, Value>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct CaseSpeaker {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub voice: String,
    /// `key = "fact"` or `key = "source_claim:unnamed"`. Facts not listed are not known.
    #[serde(default)]
    pub knows: BTreeMap<String, String>,
    /// When true every non-hidden fact is known as plain fact before `knows` is applied.
    #[serde(default)]
    pub witness: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Expect {
    /// Every output contains each of these.
    #[serde(default)]
    pub must_contain: Vec<String>,
    /// No output contains any of these.
    #[serde(default)]
    pub must_not_contain: Vec<String>,
    /// The overall certainty of every output.
    #[serde(default)]
    pub certainty: Option<Certainty>,
    /// At least this many different outputs across the seeds ("multiple valid outputs").
    #[serde(default)]
    pub min_distinct: usize,
    #[serde(default)]
    pub min_parts: usize,
    /// Nothing should be renderable (the speaker knows too little).
    #[serde(default)]
    pub empty: bool,
    /// Options that must be offered (inbox).
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Case {
    pub id: String,
    #[serde(default)]
    pub summary: String,
    pub now: String,
    pub channel: String,
    #[serde(default)]
    pub behaviour: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default = "eight")]
    pub seeds: u64,
    pub event: CaseEvent,
    pub speaker: CaseSpeaker,
    #[serde(default)]
    pub expect: Expect,
}

fn eight() -> u64 {
    8
}

#[derive(Deserialize, Default)]
struct CaseFile {
    #[serde(default)]
    case: Vec<Case>,
}

pub fn builtin_cases() -> Result<Vec<Case>, Vec<String>> {
    let mut out = Vec::new();
    let mut errs = Vec::new();
    for (path, src) in crate::embedded::FILES {
        if !path.contains("/corpus/") {
            continue;
        }
        match toml::from_str::<CaseFile>(src) {
            Ok(f) => out.extend(f.case),
            Err(e) => errs.push(format!("{path}: {e}")),
        }
    }
    if errs.is_empty() { Ok(out) } else { Err(errs) }
}

#[derive(Debug, Default)]
pub struct CaseReport {
    pub id: String,
    pub outputs: Vec<String>,
    pub problems: Vec<String>,
}

pub fn run_case(eng: &Engine, c: &Case) -> CaseReport {
    let mut rep = CaseReport { id: c.id.clone(), ..Default::default() };
    let (Some(date), Some(now)) = (text::parse_date(&c.event.date), text::parse_date(&c.now)) else {
        rep.problems.push("bad date".into());
        return rep;
    };
    let ev = Event { kind: c.event.kind.clone(), date, facts: c.event.facts.clone() };
    let mut sp = if c.speaker.witness { eng.witness("s", &c.speaker.role, &c.speaker.voice, &ev) } else { Speaker::new("s", &c.speaker.role, eng.voice(&c.speaker.voice)) };
    sp.rs_words = eng.lang.voices.get(&c.speaker.voice).is_some_and(|v| v.rs_words);
    for (k, v) in &c.speaker.knows {
        match Know::parse(v) {
            Some(kn) => {
                sp.knows.insert(k.clone(), kn);
            }
            None => rep.problems.push(format!("bad knowledge {k}={v}")),
        }
    }
    let mut distinct = BTreeSet::new();
    for seed in 0..c.seeds {
        let mut req = Request::new(&ev, &sp, &c.channel, now, seed);
        req.behaviour = c.behaviour.as_deref();
        req.intent = c.intent.as_deref();
        let r = eng.render(&req, &mut Tracker::new());
        if c.expect.empty {
            if !r.parts.is_empty() {
                rep.problems.push(format!("seed {seed}: expected nothing, got {:?}", r.text()));
            }
            continue;
        }
        let t = r.text();
        if r.parts.len() < c.expect.min_parts.max(1) {
            rep.problems.push(format!("seed {seed}: only {} parts ({})", r.parts.len(), r.notes.join("; ")));
        }
        for i in check::check_rendered(&eng.lang, &req, &r) {
            rep.problems.push(format!("seed {seed}: {} — {}", i.check, i.detail));
        }
        for m in &c.expect.must_contain {
            if !t.to_lowercase().contains(&m.to_lowercase()) {
                rep.problems.push(format!("seed {seed}: missing {m:?} in {t:?}"));
            }
        }
        for m in &c.expect.must_not_contain {
            if t.to_lowercase().contains(&m.to_lowercase()) {
                rep.problems.push(format!("seed {seed}: contains forbidden {m:?} in {t:?}"));
            }
        }
        if let (Some(want), Some(got)) = (c.expect.certainty, r.certainty())
            && want != got
        {
            rep.problems.push(format!("seed {seed}: certainty {} but expected {}", got.key(), want.key()));
        }
        for o in &c.expect.options {
            if !r.options.iter().any(|x| &x.id == o) {
                rep.problems.push(format!("seed {seed}: option {o} not offered"));
            }
        }
        if r.notes.iter().any(|n| n.starts_with("required")) {
            rep.problems.push(format!("seed {seed}: {}", r.notes.join("; ")));
        }
        distinct.insert(t.clone());
        rep.outputs.push(t);
    }
    if distinct.len() < c.expect.min_distinct {
        rep.problems.push(format!("only {} distinct outputs over {} seeds, wanted {}", distinct.len(), c.seeds, c.expect.min_distinct));
    }
    rep
}

pub fn run_all(eng: &Engine, cases: &[Case]) -> Vec<CaseReport> {
    cases.iter().map(|c| run_case(eng, c)).collect()
}
