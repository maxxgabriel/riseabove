//! The shared semantic layer: what happened (an [`Event`]), what a speaker knows about it ([`Speaker`]) and how sure they are ([`Certainty`]).
//! Every renderer (news, social, inbox, scout report, press, UI copy) reads the same event through a speaker's knowledge, so a channel cannot say
//! more than its speaker was told and cannot say it more firmly than it was told.

use pw_core::Date;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How firmly a speaker holds a fact. `Fact..Speculation` form a strength ladder; the other three are stances about a claim.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    /// Stated flatly: the speaker holds first-hand or official knowledge.
    Fact,
    /// Reported by a reliable outlet or verified against a named source.
    VerifiedReport,
    /// Attributed to a named or described source the speaker cannot check.
    SourceClaim,
    /// Circulating without a checkable origin.
    Rumour,
    /// The speaker's own inference.
    Speculation,
    /// The speaker (or a source) says it is not true.
    Denial,
    /// A previously published claim is being put right.
    Correction,
    /// The speaker knows this is not known.
    Unknown,
}

impl Certainty {
    pub const ALL: [Certainty; 8] =
        [Certainty::Fact, Certainty::VerifiedReport, Certainty::SourceClaim, Certainty::Rumour, Certainty::Speculation, Certainty::Denial, Certainty::Correction, Certainty::Unknown];
    /// Position on the strength ladder; stances sit outside it.
    pub fn rank(self) -> Option<u8> {
        match self {
            Certainty::Fact => Some(5),
            Certainty::VerifiedReport => Some(4),
            Certainty::SourceClaim => Some(3),
            Certainty::Rumour => Some(2),
            Certainty::Speculation => Some(1),
            _ => None,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Certainty::Fact => "fact",
            Certainty::VerifiedReport => "verified_report",
            Certainty::SourceClaim => "source_claim",
            Certainty::Rumour => "rumour",
            Certainty::Speculation => "speculation",
            Certainty::Denial => "denial",
            Certainty::Correction => "correction",
            Certainty::Unknown => "unknown",
        }
    }
    pub fn parse(s: &str) -> Option<Certainty> {
        Certainty::ALL.into_iter().find(|c| c.key() == s)
    }
    /// The certainty of a statement built from several facts. Unknown wins (a sentence cannot be firmer than a fact nobody has), then the two
    /// stances, then the weakest rung of the ladder. Nothing is ever promoted.
    pub fn combine(all: impl IntoIterator<Item = Certainty>) -> Certainty {
        let mut worst: Option<Certainty> = None;
        let (mut unknown, mut denial, mut correction) = (false, false, false);
        for c in all {
            match c {
                Certainty::Unknown => unknown = true,
                Certainty::Denial => denial = true,
                Certainty::Correction => correction = true,
                other => {
                    if worst.is_none_or(|w| other.rank() < w.rank()) {
                        worst = Some(other);
                    }
                }
            }
        }
        if unknown {
            Certainty::Unknown
        } else if denial {
            Certainty::Denial
        } else if correction {
            Certainty::Correction
        } else {
            worst.unwrap_or(Certainty::Fact)
        }
    }
}

/// A true descriptor of an entity, supplied by the caller from world state (never invented by the language layer).
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Descriptor {
    /// What sort of descriptor: `role` ("winger"), `age_role` ("19-year-old winger"), `club_role`, `origin`, `title`, ...
    pub kind: String,
    /// A noun phrase that can precede the name: "19-year-old winger".
    pub text: String,
}

/// A reference to a real entity in the world with its canonical names.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Ref {
    /// World id, e.g. `club.mohun-bagan-sg`. Two refs with the same id are the same entity.
    pub id: String,
    pub full: String,
    /// The name used on later mentions ("Mohun Bagan", "Chhetri"). Falls back to `full`.
    #[serde(default)]
    pub short: String,
    #[serde(default)]
    pub descriptors: Vec<Descriptor>,
    /// True when a name takes a plural verb ("Mohun Bagan have", but "Chennaiyin FC have" too in British usage). Used for agreement.
    #[serde(default = "yes")]
    pub plural: bool,
}

fn yes() -> bool {
    true
}

impl Ref {
    pub fn new(id: &str, full: &str, short: &str) -> Ref {
        Ref { id: id.into(), full: full.into(), short: short.into(), descriptors: Vec::new(), plural: true }
    }
    pub fn with_desc(mut self, kind: &str, text: &str) -> Ref {
        self.descriptors.push(Descriptor { kind: kind.into(), text: text.into() });
        self
    }
    pub fn short_name(&self) -> &str {
        if self.short.is_empty() { &self.full } else { &self.short }
    }
}

/// The value of one fact of an event.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Variant order matters: `untagged` tries them in turn, and a struct like `Ref` would happily accept an array as a sequence, so `List` comes first.
pub enum Value {
    Bool(bool),
    Num(i64),
    /// An amount in rupees.
    Money {
        money: i64,
    },
    Date {
        date: String,
    },
    List(Vec<String>),
    Ent(Ref),
    Text(String),
}

impl Value {
    pub fn date(d: Date) -> Value {
        let (y, m, dd) = d.ymd();
        Value::Date { date: format!("{y:04}-{m:02}-{dd:02}") }
    }
    /// Text, entity or number as a plain comparable string for `when` conditions.
    pub fn as_cmp(&self) -> Cmp {
        match self {
            Value::Num(n) => Cmp::N(*n as f64),
            Value::Money { money } => Cmp::N(*money as f64),
            Value::Bool(b) => Cmp::S(b.to_string()),
            Value::Text(t) => Cmp::S(t.clone()),
            Value::Ent(r) => Cmp::S(r.id.clone()),
            Value::Date { date } => Cmp::S(date.clone()),
            Value::List(l) => Cmp::N(l.len() as f64),
        }
    }
    /// Every string form under which this value could appear in rendered text (used to detect leaks of facts a speaker does not know).
    pub fn surface_forms(&self) -> Vec<String> {
        match self {
            Value::Num(n) => vec![n.to_string()],
            Value::Money { money } => {
                let mut v = vec![money.to_string(), crate::text::money(*money, false), crate::text::money(*money, true)];
                v.retain(|s| s.chars().any(|c| c.is_ascii_digit()));
                v
            }
            Value::Text(t) => vec![t.clone()],
            Value::Ent(r) => vec![r.full.clone()],
            Value::List(l) => l.clone(),
            Value::Date { .. } | Value::Bool(_) => Vec::new(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Cmp {
    N(f64),
    S(String),
}

/// One thing that happened. Facts are keyed by the names the event definition declares.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    pub date: Date,
    #[serde(default)]
    pub facts: BTreeMap<String, Value>,
}

impl Event {
    pub fn new(kind: &str, date: Date) -> Event {
        Event { kind: kind.into(), date, facts: BTreeMap::new() }
    }
    pub fn with(mut self, key: &str, v: Value) -> Event {
        self.facts.insert(key.into(), v);
        self
    }
    pub fn ent(self, key: &str, r: Ref) -> Event {
        self.with(key, Value::Ent(r))
    }
    pub fn text(self, key: &str, t: &str) -> Event {
        self.with(key, Value::Text(t.into()))
    }
    pub fn num(self, key: &str, n: i64) -> Event {
        self.with(key, Value::Num(n))
    }
    pub fn money(self, key: &str, rupees: i64) -> Event {
        self.with(key, Value::Money { money: rupees })
    }
}

/// What a speaker holds about one fact.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Know {
    pub certainty: Certainty,
    /// Key into the `source` table ("club_statement", "unnamed", "agent", ...). Needed for every certainty below `Fact`.
    #[serde(default)]
    pub source: Option<String>,
}

impl Know {
    pub fn fact() -> Know {
        Know { certainty: Certainty::Fact, source: None }
    }
    pub fn of(c: Certainty, source: &str) -> Know {
        Know { certainty: c, source: Some(source.into()) }
    }
    /// Parses `"fact"` or `"source_claim:unnamed"`.
    pub fn parse(s: &str) -> Option<Know> {
        let (c, src) = match s.split_once(':') {
            Some((c, s)) => (c, Some(s.to_string())),
            None => (s, None),
        };
        Some(Know { certainty: Certainty::parse(c)?, source: src })
    }
}

/// The thirteen dimensions along which speakers differ. All are 0..=1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Voice {
    pub formality: f32,
    /// 0 = short clipped sentences, 1 = long ones.
    pub sentence_length: f32,
    pub confidence: f32,
    pub technical: f32,
    pub emotion: f32,
    pub humour: f32,
    pub history: f32,
    pub stats: f32,
    pub tactical: f32,
    pub local: f32,
    pub slang: f32,
    pub skepticism: f32,
    pub optimism: f32,
}

impl Default for Voice {
    fn default() -> Voice {
        Voice {
            formality: 0.5,
            sentence_length: 0.5,
            confidence: 0.5,
            technical: 0.5,
            emotion: 0.3,
            humour: 0.1,
            history: 0.3,
            stats: 0.3,
            tactical: 0.3,
            local: 0.3,
            slang: 0.1,
            skepticism: 0.4,
            optimism: 0.5,
        }
    }
}

pub const VOICE_DIMS: [&str; 13] = ["formality", "sentence_length", "confidence", "technical", "emotion", "humour", "history", "stats", "tactical", "local", "slang", "skepticism", "optimism"];

impl Voice {
    pub fn get(&self, dim: &str) -> Option<f32> {
        Some(match dim {
            "formality" => self.formality,
            "sentence_length" => self.sentence_length,
            "confidence" => self.confidence,
            "technical" => self.technical,
            "emotion" => self.emotion,
            "humour" => self.humour,
            "history" => self.history,
            "stats" => self.stats,
            "tactical" => self.tactical,
            "local" => self.local,
            "slang" => self.slang,
            "skepticism" => self.skepticism,
            "optimism" => self.optimism,
            _ => return None,
        })
    }
}

/// Who is speaking and what they were told.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Speaker {
    pub id: String,
    /// `journalist`, `fan`, `club_official`, `scout`, `manager`, `agent`, `system`, ... Frames may restrict themselves to roles.
    pub role: String,
    pub voice: Voice,
    #[serde(default)]
    pub knows: BTreeMap<String, Know>,
    /// House style: writes "Rs 2 crore" rather than "₹2 crore".
    #[serde(default)]
    pub rs_words: bool,
}

impl Speaker {
    pub fn new(id: &str, role: &str, voice: Voice) -> Speaker {
        Speaker { id: id.into(), role: role.into(), voice, knows: BTreeMap::new(), rs_words: false }
    }
    pub fn know(mut self, key: &str, k: Know) -> Speaker {
        self.knows.insert(key.into(), k);
        self
    }
    /// Knows every fact in `keys` as plain fact.
    pub fn knows_all(mut self, keys: &[&str]) -> Speaker {
        for k in keys {
            self.knows.insert((*k).into(), Know::fact());
        }
        self
    }
}

/// Requests a rendering of one event through one channel.
#[derive(Clone, Debug)]
pub struct Request<'a> {
    pub event: &'a Event,
    pub speaker: &'a Speaker,
    pub channel: &'a str,
    /// The simulation date the text is published on; every "yesterday" is resolved against it.
    pub now: Date,
    pub seed: u64,
    /// Social behaviour or inbox move family ("reaction", "hype", "correction", ...); selects `post:$behaviour` topics.
    pub behaviour: Option<&'a str>,
    /// What the speaker is trying to do: "inform" (default), "persuade", "reassure", "warn", "ask", ...
    pub intent: Option<&'a str>,
}

impl<'a> Request<'a> {
    pub fn new(event: &'a Event, speaker: &'a Speaker, channel: &'a str, now: Date, seed: u64) -> Request<'a> {
        Request { event, speaker, channel, now, seed, behaviour: None, intent: None }
    }
    pub fn behaviour(mut self, b: &'a str) -> Self {
        self.behaviour = Some(b);
        self
    }
    pub fn intent(mut self, i: &'a str) -> Self {
        self.intent = Some(i);
        self
    }
}

/// One rendered slot of a text.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub slot: String,
    pub topic: String,
    pub frame: String,
    pub certainty: Certainty,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedOption {
    pub id: String,
    pub label: String,
    /// Plain statement of what choosing it does; comes from the option's `effect`, which the simulation implements.
    pub consequence: String,
    pub effect: String,
}

#[derive(Clone, Debug, Default)]
pub struct Rendered {
    pub parts: Vec<Part>,
    pub options: Vec<RenderedOption>,
    /// Why items were skipped or the rendering is incomplete. Empty on a clean rendering.
    pub notes: Vec<String>,
}

impl Rendered {
    /// The parts as running text: headline-like slots stand on their own line, body slots run on.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.parts.iter().enumerate() {
            if i > 0 {
                out.push(if is_display_slot(&self.parts[i - 1].slot) || is_display_slot(&p.slot) { '\n' } else { ' ' });
            }
            out.push_str(&p.text);
        }
        out
    }
    pub fn part(&self, slot: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.slot == slot)
    }
    /// The weakest certainty among the parts that assert something.
    pub fn certainty(&self) -> Option<Certainty> {
        self.parts.iter().map(|p| p.certainty).reduce(|a, b| Certainty::combine([a, b]))
    }
}

/// Slots that are display lines rather than sentences of running prose.
pub fn is_display_slot(slot: &str) -> bool {
    matches!(slot, "headline" | "subject" | "heading" | "label" | "standfirst" | "status")
}
