//! The language data model. Everything a writer can change lives in `data/lang/<locale>/**/*.toml`; nothing here holds vocabulary.

use crate::model::{Certainty, VOICE_DIMS, Voice};
use crate::template::{self, Node};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum Strs {
    One(String),
    Many(Vec<String>),
}

impl Default for Strs {
    fn default() -> Strs {
        Strs::Many(Vec::new())
    }
}

impl Strs {
    pub fn list(&self) -> Vec<&str> {
        match self {
            Strs::One(s) => vec![s.as_str()],
            Strs::Many(v) => v.iter().map(String::as_str).collect(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.list().is_empty()
    }
    pub fn has(&self, s: &str) -> bool {
        self.list().contains(&s)
    }
    /// Empty means "any".
    pub fn allows(&self, s: &str) -> bool {
        self.is_empty() || self.has(s)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct FactDef {
    pub key: String,
    /// `club` `person` `team` `competition` `place` `text` `num` `money` `date` `list` `bool`.
    #[serde(rename = "type")]
    pub ty: String,
    /// True facts that exist in the world but that outsiders do not know (a seller's minimum, a player's private motive). They never appear in
    /// text unless the speaker's knowledge lists them, and the leak check looks for them.
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct EventDef {
    pub kind: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, rename = "fact")]
    pub facts: Vec<FactDef>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Variant {
    /// Form name → word: `{ base = "turn down", third = "turns down", past = "turned down", pp = "turned down", ing = "turning down" }`.
    pub forms: BTreeMap<String, String>,
    #[serde(default)]
    pub register: String,
    #[serde(default)]
    pub voice: BTreeMap<String, f32>,
    #[serde(default)]
    pub channels: Strs,
    #[serde(default)]
    pub roles: Strs,
    /// Weakest certainty at which this word may be used (e.g. "confirm" needs `verified_report` or firmer).
    #[serde(default)]
    pub min_certainty: Option<Certainty>,
    /// Certainties this word must NOT be used at ("official" never at rumour).
    #[serde(default)]
    pub not_certainty: Vec<Certainty>,
    /// Fact keys that must be known for the word to be honest ("lodge a bid" needs nothing, "improved bid" needs `round`).
    #[serde(default)]
    pub requires: Vec<String>,
    /// Semantic condition over known facts (`margin >= 3`, `team == 'team.india-men'`): the word says only what the facts support.
    #[serde(default)]
    pub when: String,
    /// Free-text grammar/usage note kept with the word ("takes a fee object", "not for injuries to under-18s"). Not interpreted.
    #[serde(default)]
    pub note: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Concept {
    pub id: String,
    /// What the concept means, in one line.
    #[serde(default)]
    pub meaning: String,
    #[serde(default)]
    pub pos: String,
    #[serde(default, rename = "variant")]
    pub variants: Vec<Variant>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct CollocItem {
    pub text: String,
    #[serde(default)]
    pub forms: BTreeMap<String, String>,
    #[serde(default)]
    pub when: String,
    #[serde(default)]
    pub register: String,
    #[serde(default)]
    pub voice: BTreeMap<String, f32>,
    #[serde(default)]
    pub channels: Strs,
    /// True when using the word says the facts in `when` outright ("second" states the round); vague words ("serious") leave them unstated.
    #[serde(default)]
    pub states: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Collocation {
    pub head: String,
    pub slot: String,
    /// When true the slot may be empty (no collocate passes) without it being an error.
    #[serde(default)]
    pub optional: bool,
    #[serde(default, rename = "item")]
    pub items: Vec<CollocItem>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Frame {
    pub id: String,
    pub event: String,
    #[serde(default)]
    pub channel: Strs,
    pub slot: String,
    #[serde(default = "main_topic")]
    pub topic: String,
    #[serde(default)]
    pub needs: Vec<String>,
    /// `clause` frames are wrapped by the certainty grammar; `full` frames carry their own certainty wording and list `certainty`.
    #[serde(default = "clause_form")]
    pub form: String,
    #[serde(default)]
    pub certainty: Vec<Certainty>,
    #[serde(default)]
    pub when: String,
    /// `past`, `future` or `any` relative to the publication date.
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub roles: Strs,
    #[serde(default)]
    pub intent: Strs,
    #[serde(default)]
    pub behaviour: Strs,
    #[serde(default)]
    pub register: String,
    #[serde(default)]
    pub voice: BTreeMap<String, f32>,
    pub text: String,
}

fn main_topic() -> String {
    "main".into()
}
fn clause_form() -> String {
    "clause".into()
}

#[derive(Deserialize, Clone, Debug)]
pub struct Wrapper {
    pub id: String,
    pub certainty: Certainty,
    /// Used only when the previous part already attributed a statement of the same certainty to the same source, so attribution is not repeated.
    #[serde(default)]
    pub continuation: bool,
    #[serde(default)]
    pub slot: Strs,
    #[serde(default)]
    pub channel: Strs,
    #[serde(default)]
    pub register: String,
    #[serde(default)]
    pub voice: BTreeMap<String, f32>,
    pub text: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Source {
    pub kind: String,
    pub phrase: String,
    #[serde(default = "yes")]
    pub plural: bool,
    /// Certainties this source can stand behind.
    #[serde(default)]
    pub certainty: Vec<Certainty>,
    #[serde(default)]
    pub roles: Strs,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize, Clone, Debug)]
pub struct SlotDef {
    pub slot: String,
    /// Slots that are sentences get a capital and a full stop; labels and headlines do not get the full stop.
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub max_chars: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ChannelDef {
    pub id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, rename = "slot")]
    pub slots: Vec<SlotDef>,
    /// Substrings that never belong in this channel ("#" and "@" in news).
    #[serde(default)]
    pub forbid: Vec<String>,
    /// Default structure when an event has no article entry: `slot:topic` items.
    #[serde(default)]
    pub default_structure: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Article {
    pub event: String,
    pub channel: String,
    /// `slot:topic` items in order. Topic `$behaviour` takes the request's behaviour. Items with no satisfiable frame are skipped.
    pub structure: Vec<String>,
    /// Items that must be present or the whole rendering is reported as failed.
    #[serde(default)]
    pub required: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OptionDef {
    pub event: String,
    #[serde(default)]
    pub channel: Strs,
    pub id: String,
    pub label: String,
    /// The simulation effect the option triggers; the consequence line is written from it.
    pub effect: String,
    pub consequence: String,
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub when: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct VoicePreset {
    pub id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub role: String,
    #[serde(flatten)]
    pub dims: Voice,
    /// Writes money as "Rs 2 crore" instead of "₹2 crore".
    #[serde(default)]
    pub rs_words: bool,
}

/// House style of an outlet or account class. Safe metadata only: no scores, no claims about real people.
#[derive(Deserialize, Clone, Debug)]
pub struct Profile {
    pub id: String,
    pub voice: String,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Deserialize, Default, Debug)]
pub struct FileData {
    #[serde(default)]
    pub event: Vec<EventDef>,
    #[serde(default)]
    pub concept: Vec<Concept>,
    #[serde(default)]
    pub collocation: Vec<Collocation>,
    #[serde(default)]
    pub frame: Vec<Frame>,
    #[serde(default)]
    pub wrapper: Vec<Wrapper>,
    #[serde(default)]
    pub source: Vec<Source>,
    #[serde(default)]
    pub channel: Vec<ChannelDef>,
    #[serde(default)]
    pub article: Vec<Article>,
    #[serde(default)]
    pub option: Vec<OptionDef>,
    #[serde(default)]
    pub voice: Vec<VoicePreset>,
    #[serde(default)]
    pub profile: Vec<Profile>,
}

/// Parsed, indexed language data.
#[derive(Default)]
pub struct Lang {
    pub events: BTreeMap<String, EventDef>,
    pub concepts: BTreeMap<String, Concept>,
    pub collocations: Vec<Collocation>,
    pub frames: Vec<Frame>,
    pub frame_nodes: Vec<Vec<Node>>,
    pub wrappers: Vec<Wrapper>,
    pub wrapper_nodes: Vec<Vec<Node>>,
    pub sources: BTreeMap<String, Source>,
    pub channels: BTreeMap<String, ChannelDef>,
    pub articles: Vec<Article>,
    pub options: Vec<OptionDef>,
    pub voices: BTreeMap<String, VoicePreset>,
    pub profiles: BTreeMap<String, Profile>,
}

impl Lang {
    /// Loads and merges the given `(path, toml)` files. Duplicate ids are errors.
    pub fn load(files: &[(&str, &str)]) -> Result<Lang, Vec<String>> {
        let mut errs = Vec::new();
        let mut l = Lang::default();
        for (path, src) in files {
            if path.contains("/corpus/") {
                continue;
            }
            let fd: FileData = match toml::from_str(src) {
                Ok(f) => f,
                Err(e) => {
                    errs.push(format!("{path}: {e}"));
                    continue;
                }
            };
            for e in fd.event {
                if l.events.insert(e.kind.clone(), e.clone()).is_some() {
                    errs.push(format!("{path}: duplicate event {}", e.kind));
                }
            }
            for c in fd.concept {
                if let Some(old) = l.concepts.get_mut(&c.id) {
                    // Several files may add variants to one concept; the meaning stays with the first definition.
                    old.variants.extend(c.variants);
                } else {
                    l.concepts.insert(c.id.clone(), c);
                }
            }
            l.collocations.extend(fd.collocation);
            for f in fd.frame {
                match template::parse(&f.text) {
                    Ok(n) => {
                        l.frame_nodes.push(n);
                        l.frames.push(f);
                    }
                    Err(e) => errs.push(format!("{path}: frame {}: {e}", f.id)),
                }
            }
            for w in fd.wrapper {
                match template::parse(&w.text) {
                    Ok(n) => {
                        l.wrapper_nodes.push(n);
                        l.wrappers.push(w);
                    }
                    Err(e) => errs.push(format!("{path}: wrapper {}: {e}", w.id)),
                }
            }
            for s in fd.source {
                l.sources.insert(s.kind.clone(), s);
            }
            for c in fd.channel {
                l.channels.insert(c.id.clone(), c);
            }
            l.articles.extend(fd.article);
            l.options.extend(fd.option);
            for v in fd.voice {
                l.voices.insert(v.id.clone(), v);
            }
            for p in fd.profile {
                l.profiles.insert(p.id.clone(), p);
            }
        }
        if errs.is_empty() { Ok(l) } else { Err(errs) }
    }

    /// The language data shipped with the crate.
    pub fn builtin() -> Lang {
        match Lang::load(crate::embedded::FILES) {
            Ok(l) => l,
            Err(e) => panic!("built-in language data is invalid: {}", e.join("; ")),
        }
    }

    /// Cross-checks the data set: unknown events, facts, concepts, slots; templates asking for facts their frame does not require; forms missing.
    pub fn lint(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for (f, nodes) in self.frames.iter().zip(&self.frame_nodes) {
            let id = &f.id;
            if !seen.insert(id.clone()) {
                errs.push(format!("frame {id}: duplicate id"));
            }
            let patterns: Vec<&str> = f.event.split(',').map(str::trim).collect();
            let exact = patterns.len() == 1 && !patterns[0].contains('*');
            let ev_facts: Vec<&str> = if exact {
                match self.events.get(&f.event) {
                    Some(ev) => ev.facts.iter().map(|x| x.key.as_str()).collect(),
                    None => {
                        errs.push(format!("frame {id}: unknown event {}", f.event));
                        continue;
                    }
                }
            } else {
                if !self.events.keys().any(|k| patterns.iter().any(|p| crate::render::event_matches(p, k))) {
                    errs.push(format!("frame {id}: event pattern {} matches nothing", f.event));
                }
                if !f.needs.is_empty() {
                    errs.push(format!("frame {id}: frames for several events cannot need facts"));
                }
                Vec::new()
            };
            let mut fact_keys = ev_facts;
            fact_keys.push("date");
            for n in &f.needs {
                if !fact_keys.contains(&n.as_str()) && !crate::derive::derived_keys(&f.event).contains(&n.as_str()) {
                    errs.push(format!("frame {id}: needs unknown fact {n}"));
                }
            }
            let mut req = Vec::new();
            template::required_keys(nodes, &mut req);
            for k in req {
                if !f.needs.contains(&k) && k != "date" {
                    errs.push(format!("frame {id}: uses {{{k}}} outside an optional segment but does not list it in needs"));
                }
            }
            let mut all = Vec::new();
            template::all_keys(nodes, &mut all);
            for k in all {
                if !fact_keys.contains(&k.as_str()) && !crate::derive::derived_keys(&f.event).contains(&k.as_str()) {
                    errs.push(format!("frame {id}: unknown fact {k} in text"));
                }
            }
            for c in f.channel.list() {
                if !self.channels.contains_key(c) {
                    errs.push(format!("frame {id}: unknown channel {c}"));
                } else if !self.channels[c].slots.iter().any(|s| s.slot == f.slot) {
                    errs.push(format!("frame {id}: channel {c} has no slot {}", f.slot));
                }
            }
            if f.channel.is_empty() {
                errs.push(format!("frame {id}: needs a channel"));
            }
            if f.form != "full" && f.form != "clause" {
                errs.push(format!("frame {id}: form must be clause or full"));
            }
            if f.form == "clause" && nodes.iter().any(|n| matches!(n, Node::Clause)) {
                errs.push(format!("frame {id}: {{clause}} belongs in wrappers"));
            }
            self.lint_nodes(&format!("frame {id}"), nodes, &mut errs);
            self.lint_dims(&format!("frame {id}"), &f.voice, &mut errs);
            if !f.when.is_empty()
                && let Err(e) = crate::cond::parse(&f.when)
            {
                errs.push(format!("frame {id}: bad when: {e}"));
            }
        }
        for (w, nodes) in self.wrappers.iter().zip(&self.wrapper_nodes) {
            let id = &w.id;
            if w.certainty != Certainty::Fact && w.certainty != Certainty::Unknown && !nodes.iter().any(|n| matches!(n, Node::Clause)) {
                errs.push(format!("wrapper {id}: must contain {{clause}}"));
            }
            self.lint_nodes(&format!("wrapper {id}"), nodes, &mut errs);
            self.lint_dims(&format!("wrapper {id}"), &w.voice, &mut errs);
            let mut ks = Vec::new();
            template::all_keys(nodes, &mut ks);
            if !ks.is_empty() {
                errs.push(format!("wrapper {id}: wrappers cannot use facts, only {{clause}} and {{cert:source}}"));
            }
        }
        for a in &self.articles {
            if !self.events.contains_key(&a.event) {
                errs.push(format!("article {}/{}: unknown event", a.event, a.channel));
            }
            let Some(ch) = self.channels.get(&a.channel) else {
                errs.push(format!("article {}/{}: unknown channel", a.event, a.channel));
                continue;
            };
            for item in a.structure.iter().chain(&a.required) {
                let slot = item.split(':').next().unwrap_or("");
                if !ch.slots.iter().any(|s| s.slot == slot) {
                    errs.push(format!("article {}/{}: slot {slot} not in channel", a.event, a.channel));
                }
            }
        }
        for o in &self.options {
            if !self.events.contains_key(&o.event) {
                errs.push(format!("option {}: unknown event {}", o.id, o.event));
            }
            for (label, t) in [("label", &o.label), ("consequence", &o.consequence)] {
                match template::parse(t) {
                    Ok(n) => self.lint_nodes(&format!("option {} {label}", o.id), &n, &mut errs),
                    Err(e) => errs.push(format!("option {} {label}: {e}", o.id)),
                }
            }
        }
        for c in &self.collocations {
            for it in &c.items {
                self.lint_dims(&format!("collocation {}.{}", c.head, c.slot), &it.voice, &mut errs);
                if !it.when.is_empty()
                    && let Err(e) = crate::cond::parse(&it.when)
                {
                    errs.push(format!("collocation {}.{}: bad when: {e}", c.head, c.slot));
                }
            }
        }
        for c in self.concepts.values() {
            for v in &c.variants {
                self.lint_dims(&format!("concept {}", c.id), &v.voice, &mut errs);
                if v.forms.is_empty() {
                    errs.push(format!("concept {}: variant with no forms", c.id));
                }
            }
        }
        errs
    }

    fn lint_dims(&self, what: &str, dims: &BTreeMap<String, f32>, errs: &mut Vec<String>) {
        for (k, v) in dims {
            if !VOICE_DIMS.contains(&k.as_str()) {
                errs.push(format!("{what}: unknown voice dimension {k}"));
            } else if !(0.0..=1.0).contains(v) {
                errs.push(format!("{what}: voice {k} out of range"));
            }
        }
    }

    fn lint_nodes(&self, what: &str, nodes: &[Node], errs: &mut Vec<String>) {
        for n in nodes {
            match n {
                Node::Lex { concept, form } => match self.concepts.get(concept) {
                    None => errs.push(format!("{what}: unknown concept {concept}")),
                    Some(c) => {
                        let ok = |f: &str| c.variants.iter().any(|v| v.forms.contains_key(f));
                        let found = if form == "agr" { ok("base") && ok("third") } else { ok(form) };
                        if !found {
                            errs.push(format!("{what}: concept {concept} has no form {form}"));
                        }
                    }
                },
                Node::Col { head, slot, .. } => {
                    if !self.collocations.iter().any(|c| &c.head == head && &c.slot == slot) {
                        errs.push(format!("{what}: unknown collocation {head}.{slot}"));
                    }
                }
                Node::Cert(what_) if what_ != "source" => errs.push(format!("{what}: unknown {{cert:{what_}}}")),
                Node::Alt(alts) => {
                    for a in alts {
                        self.lint_nodes(what, a, errs);
                    }
                }
                Node::Opt { body, .. } => self.lint_nodes(what, body, errs),
                _ => {}
            }
        }
    }
}
