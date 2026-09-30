//! The realiser: one event, one speaker's knowledge, one channel → text. Selection is deterministic in `seed`, fitted to the speaker's voice,
//! damped by what was said recently, and can never use a fact the speaker does not hold or state it more firmly than they hold it.

use crate::cond;
use crate::data::{Frame, Lang, Source, Variant};
use crate::derive;
use crate::model::*;
use crate::template::Node;
use crate::text;
use std::collections::{BTreeMap, BTreeSet};

/// Remembers what was said recently so the same phrase is not the first choice twice running.
#[derive(Default, Clone, Debug)]
pub struct Tracker {
    frames: Vec<String>,
    words: Vec<String>,
}

const FRAME_WINDOW: usize = 40;
const WORD_WINDOW: usize = 80;

impl Tracker {
    pub fn new() -> Tracker {
        Tracker::default()
    }
    fn penalty_frame(&self, id: &str) -> f32 {
        (self.frames.iter().filter(|f| *f == id).count() as f32 * 0.3).min(0.9)
    }
    fn penalty_word(&self, w: &str) -> f32 {
        (self.words.iter().filter(|x| *x == w).count() as f32 * 0.12).min(0.5)
    }
    fn note_frame(&mut self, id: &str) {
        self.frames.push(id.into());
        if self.frames.len() > FRAME_WINDOW {
            self.frames.remove(0);
        }
    }
    fn note_word(&mut self, w: &str) {
        self.words.push(w.into());
        if self.words.len() > WORD_WINDOW {
            self.words.remove(0);
        }
    }
}

pub struct Engine {
    pub lang: Lang,
}

fn hash(s: &str, seed: u64) -> u64 {
    let mut h = 0xcbf29ce484222325u64 ^ seed.wrapping_mul(0x9E3779B97F4A7C15);
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h ^ (h >> 29)
}

fn register_targets(reg: &str) -> Vec<(&'static str, f32)> {
    match reg {
        "formal" => vec![("formality", 0.85), ("slang", 0.0)],
        "neutral" => vec![("formality", 0.5), ("slang", 0.1)],
        "informal" => vec![("formality", 0.25), ("slang", 0.4)],
        "slang" => vec![("formality", 0.1), ("slang", 0.9)],
        "tabloid" => vec![("formality", 0.3), ("emotion", 0.8)],
        "technical" => vec![("technical", 0.9)],
        "local" => vec![("local", 0.9)],
        "hype" => vec![("emotion", 0.9), ("optimism", 0.9)],
        "sceptical" => vec![("skepticism", 0.9)],
        "wry" => vec![("humour", 0.7)],
        _ => Vec::new(),
    }
}

/// How well a voice fits a register tag plus explicit per-dimension targets. 0..=1; 0.5 when nothing is asked.
pub fn fit(voice: &Voice, register: &str, dims: &BTreeMap<String, f32>) -> f32 {
    let mut targets: Vec<(String, f32)> = register_targets(register).into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    for (k, v) in dims {
        targets.retain(|(x, _)| x != k);
        targets.push((k.clone(), *v));
    }
    if targets.is_empty() {
        return 0.5;
    }
    let sum: f32 = targets.iter().map(|(k, t)| (voice.get(k).unwrap_or(0.5) - t).abs()).sum();
    1.0 - sum / targets.len() as f32
}

/// The facts a speaker actually holds, with derived facts added.
struct View {
    facts: BTreeMap<String, Value>,
    know: BTreeMap<String, Know>,
}

impl View {
    fn new(ev: &Event, sp: &Speaker) -> View {
        let mut facts = BTreeMap::new();
        let mut know = BTreeMap::new();
        for (k, kn) in &sp.knows {
            if let Some(v) = ev.facts.get(k) {
                if kn.certainty != Certainty::Unknown {
                    facts.insert(k.clone(), v.clone());
                }
                know.insert(k.clone(), kn.clone());
            }
        }
        facts.insert("date".into(), Value::date(ev.date));
        know.insert("date".into(), Know::fact());
        derive::apply(&ev.kind, &mut facts, &mut know);
        View { facts, know }
    }
    fn cert_of(&self, k: &str) -> Option<Certainty> {
        self.know.get(k).map(|k| k.certainty)
    }
}

struct Cx<'a> {
    req: &'a Request<'a>,
    view: &'a View,
    mentioned: &'a mut BTreeSet<String>,
    tr: &'a mut Tracker,
    cert: Certainty,
    source: Option<&'a Source>,
    tag: String,
    n: u32,
    pending: Vec<String>,
    /// Fact keys already stated in this text, so a later slot does not say the same thing again.
    stated: &'a mut BTreeSet<String>,
    /// Noun choices made so far ("bid" stays "bid" through an article).
    memo: &'a mut BTreeMap<String, String>,
}

impl Cx<'_> {
    fn key(&mut self, what: &str) -> String {
        self.n += 1;
        format!("{}:{}:{}:{}:{}", self.req.speaker.id, self.req.event.kind, self.tag, what, self.n)
    }
}

fn choose(scores: &[f32], key: &str, seed: u64) -> usize {
    let best = scores.iter().cloned().fold(f32::MIN, f32::max);
    let pool: Vec<usize> = (0..scores.len()).filter(|&i| scores[i] >= best - 0.12).collect();
    pool[(hash(key, seed) % pool.len() as u64) as usize]
}

/// A sentence stated at `cx` may carry an optional fact only if the speaker holds that fact at least as firmly.
fn firm_enough(k: Certainty, cx: Certainty) -> bool {
    if k == Certainty::Unknown {
        return false;
    }
    match (k.rank(), cx.rank()) {
        (Some(a), Some(b)) => a >= b,
        (Some(_), None) => k == Certainty::Fact,
        (None, _) => k == cx,
    }
}

impl Engine {
    pub fn new(lang: Lang) -> Engine {
        Engine { lang }
    }
    pub fn builtin() -> Engine {
        Engine::new(Lang::builtin())
    }

    pub fn voice(&self, id: &str) -> Voice {
        self.lang.voices.get(id).map(|v| v.dims).unwrap_or_default()
    }

    /// A speaker that knows every non-hidden fact of `ev` first hand.
    pub fn witness(&self, id: &str, role: &str, voice: &str, ev: &Event) -> Speaker {
        let mut sp = Speaker::new(id, role, self.voice(voice));
        sp.rs_words = self.lang.voices.get(voice).is_some_and(|v| v.rs_words);
        if let Some(def) = self.lang.events.get(&ev.kind) {
            for f in &def.facts {
                if !f.hidden && ev.facts.contains_key(&f.key) {
                    sp.knows.insert(f.key.clone(), Know::fact());
                }
            }
        }
        sp
    }

    pub fn render(&self, req: &Request, tr: &mut Tracker) -> Rendered {
        let mut out = Rendered::default();
        let Some(chan) = self.lang.channels.get(req.channel) else {
            out.notes.push(format!("unknown channel {}", req.channel));
            return out;
        };
        let view = View::new(req.event, req.speaker);
        let (structure, required) = match self.lang.articles.iter().find(|a| a.event == req.event.kind && a.channel == req.channel) {
            Some(a) => (a.structure.clone(), a.required.clone()),
            None if !chan.default_structure.is_empty() => (chan.default_structure.clone(), Vec::new()),
            None => (chan.slots.iter().map(|s| format!("{}:*", s.slot)).collect(), Vec::new()),
        };
        let mut mentioned = BTreeSet::new();
        let mut used: BTreeSet<usize> = BTreeSet::new();
        let mut running = Certainty::Fact;
        let mut last_attr: Option<(Certainty, Option<String>)> = None;
        let mut used_wrappers: BTreeSet<usize> = BTreeSet::new();
        let mut stated: BTreeSet<String> = BTreeSet::new();
        let mut memo: BTreeMap<String, String> = BTreeMap::new();
        for (i, item) in structure.iter().enumerate() {
            let (slot, topic) = item.split_once(':').unwrap_or((item, "*"));
            let topic = if topic == "$behaviour" {
                match req.behaviour {
                    Some(b) => b,
                    None => continue,
                }
            } else {
                topic
            };
            match self.realise_slot(
                req,
                &view,
                chan.slots.iter().find(|s| s.slot == slot),
                slot,
                topic,
                i,
                &mut used,
                &mut mentioned,
                tr,
                running,
                &mut stated,
                &mut memo,
                &mut last_attr,
                &mut used_wrappers,
            ) {
                Some(p) => {
                    running = Certainty::combine([running, p.certainty]);
                    if is_display_slot(&p.slot) {
                        // A headline is read on its own: the body introduces everyone again.
                        mentioned.clear();
                    }
                    out.parts.push(p)
                }
                None => {
                    let msg = format!("no frame for {slot}:{topic}");
                    if required.iter().any(|r| r == item) {
                        out.notes.push(format!("required {msg}"));
                    } else {
                        out.notes.push(msg);
                    }
                }
            }
        }
        // Options only make sense as choices in a channel that declares them.
        for o in &self.lang.options {
            if o.event != req.event.kind || !o.channel.allows(req.channel) || !o.needs.iter().all(|k| view.facts.contains_key(k)) {
                continue;
            }
            if !o.when.is_empty() && !cond::eval(&o.when, &view.facts) {
                continue;
            }
            let mut cx = Cx {
                req,
                view: &view,
                mentioned: &mut mentioned,
                tr,
                cert: Certainty::Fact,
                source: None,
                tag: format!("opt:{}", o.id),
                n: 0,
                pending: Vec::new(),
                stated: &mut stated,
                memo: &mut memo,
            };
            let (Ok(l), Ok(c)) = (crate::template::parse(&o.label), crate::template::parse(&o.consequence)) else { continue };
            let (Ok(l), Ok(c)) = (self.realise(&l, &mut cx), self.realise(&c, &mut cx)) else {
                out.notes.push(format!("option {} could not be written", o.id));
                continue;
            };
            out.options.push(RenderedOption { id: o.id.clone(), label: finish(&l, "label"), consequence: finish(&c, "sentence"), effect: o.effect.clone() });
        }
        if !out.parts.is_empty() && !required.is_empty() && out.notes.iter().any(|n| n.starts_with("required")) {
            // A partial article that lacks a required item is reported, not silently shipped.
            out.notes.push("incomplete".into());
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn realise_slot(
        &self,
        req: &Request,
        view: &View,
        slot_def: Option<&crate::data::SlotDef>,
        slot: &str,
        topic: &str,
        idx: usize,
        used: &mut BTreeSet<usize>,
        mentioned: &mut BTreeSet<String>,
        tr: &mut Tracker,
        running: Certainty,
        stated: &mut BTreeSet<String>,
        memo: &mut BTreeMap<String, String>,
        last_attr: &mut Option<(Certainty, Option<String>)>,
        used_wrappers: &mut BTreeSet<usize>,
    ) -> Option<Part> {
        let delta = text::delta(req.event.date, req.now);
        let intent = req.intent.unwrap_or("inform");
        let mut cands: Vec<(usize, Certainty, Option<String>, f32)> = Vec::new();
        for (fi, f) in self.lang.frames.iter().enumerate() {
            if used.contains(&fi) || !event_matches(&f.event, &req.event.kind) || !f.channel.has(req.channel) || f.slot != slot {
                continue;
            }
            if topic != "*" && f.topic != topic {
                continue;
            }
            if !f.roles.allows(&req.speaker.role) || !f.intent.allows(intent) {
                continue;
            }
            if !f.behaviour.is_empty() && !req.behaviour.is_some_and(|b| f.behaviour.has(b)) {
                continue;
            }
            match f.time.as_str() {
                "past" if delta > 0 => continue,
                "future" if delta < 0 => continue,
                _ => {}
            }
            if !f.needs.iter().all(|k| view.know.contains_key(k)) {
                continue;
            }
            // Later slots must add something: a detail that only repeats facts already stated is dropped.
            if matches!(slot, "detail" | "context") && !f.needs.is_empty() && f.needs.iter().all(|k| stated.contains(k)) {
                continue;
            }
            if !f.when.is_empty() && !cond::eval(&f.when, &view.facts) {
                continue;
            }
            let cert = if f.needs.is_empty() { running } else { Certainty::combine(f.needs.iter().filter_map(|k| view.cert_of(k))) };
            let ok_cert = if f.certainty.is_empty() { cert != Certainty::Unknown } else { f.certainty.contains(&cert) };
            if !ok_cert {
                continue;
            }
            let source = self.source_for(f, cert, view, req);
            if f.form == "clause" && self.wrapper_for(cert, slot, req, source.as_deref().and_then(|k| self.lang.sources.get(k)), f).is_empty() {
                continue;
            }
            if f.form == "full" && self.needs_source(&self.lang.frame_nodes[fi]) && source.is_none() {
                continue;
            }
            let score = fit(&req.speaker.voice, &f.register, &f.voice) - tr.penalty_frame(&f.id) + if f.topic == topic { 0.02 } else { 0.0 };
            cands.push((fi, cert, source, score));
        }
        if cands.is_empty() {
            return None;
        }
        let max_chars = slot_def.map(|s| s.max_chars).unwrap_or(0);
        let style = slot_def.map(|s| s.style.as_str()).unwrap_or("sentence");
        loop {
            if cands.is_empty() {
                return None;
            }
            let scores: Vec<f32> = cands.iter().map(|c| c.3).collect();
            let key = format!("{}:{}:{}:{}:{}:{}", req.speaker.id, req.event.kind, slot, topic, idx, cands.len());
            let pick = choose(&scores, &key, req.seed);
            let (fi, cert, source_key, _) = cands[pick].clone();
            // Work on copies of the shared state so a rejected attempt leaves no trace.
            let (mut m2, mut st2, mut memo2, mut tr2) = (mentioned.clone(), stated.clone(), memo.clone(), tr.clone());
            let f = &self.lang.frames[fi];
            let source = source_key.as_deref().and_then(|k| self.lang.sources.get(k));
            let mut cx = Cx { req, view, mentioned: &mut m2, tr: &mut tr2, cert, source, tag: f.id.clone(), n: 0, pending: Vec::new(), stated: &mut st2, memo: &mut memo2 };
            let attempt = self.realise(&self.lang.frame_nodes[fi], &mut cx).ok().and_then(|clause| {
                if f.form != "clause" {
                    return Some((clause, None));
                }
                let mut ws = self.wrapper_for(cert, slot, req, source, f);
                let attr = (cert, source_key.clone());
                // The same attribution is not stated twice: not when the grade and source match, and not when a named source has just been
                // cited for a differently graded fact (the continuation wording carries the difference: "it is also claimed that ...").
                let repeated = cert != Certainty::Fact && last_attr.as_ref().is_some_and(|l| (l.0 == cert && (l.1.is_none() || l.1 == attr.1)) || (l.1.is_some() && l.1 == attr.1));
                let cont: Vec<usize> = ws.iter().copied().filter(|&i| self.lang.wrappers[i].continuation).collect();
                let plain: Vec<usize> = ws.iter().copied().filter(|&i| !self.lang.wrappers[i].continuation).collect();
                ws = if repeated && !cont.is_empty() { cont } else { plain };
                if ws.is_empty() {
                    return None;
                }
                let wscores: Vec<f32> = ws
                    .iter()
                    .map(|&wi| {
                        let w = &self.lang.wrappers[wi];
                        fit(&req.speaker.voice, &w.register, &w.voice) - if used_wrappers.contains(&wi) { 0.5 } else { 0.0 } - cx.tr.penalty_frame(&w.id) * 0.3
                    })
                    .collect();
                let wi = ws[choose(&wscores, &format!("{key}:wrap"), req.seed)];
                let clause = clause.trim_end_matches(['.', ' ']).to_string();
                let clause = if clause_lowerable(&clause) { text::lower_first(&clause) } else { clause };
                cx.pending.push(clause);
                cx.tag = format!("{}+{}", f.id, self.lang.wrappers[wi].id);
                self.realise(&self.lang.wrapper_nodes[wi], &mut cx).ok().map(|t| (t, Some(wi)))
            });
            let Some((raw, wrapper)) = attempt else {
                cands.remove(pick);
                continue;
            };
            let final_text = finish(&raw, style);
            if max_chars > 0 && final_text.chars().count() > max_chars {
                cands.remove(pick);
                continue;
            }
            // Commit.
            *mentioned = m2;
            *stated = st2;
            *memo = memo2;
            *tr = tr2;
            if let Some(wi) = wrapper {
                used_wrappers.insert(wi);
                tr.note_frame(&self.lang.wrappers[wi].id);
            }
            if cert != Certainty::Fact && f.form == "clause" && !is_display_slot(slot) {
                // Remember the source only if the wording actually named it.
                let cited = wrapper.is_some_and(|wi| self.needs_source(&self.lang.wrapper_nodes[wi])) || self.needs_source(&self.lang.frame_nodes[fi]);
                *last_attr = Some((cert, if cited { source_key } else { None }));
            }
            used.insert(fi);
            tr.note_frame(&f.id);
            return Some(Part { slot: slot.into(), topic: f.topic.clone(), frame: f.id.clone(), certainty: cert, text: final_text });
        }
    }

    fn needs_source(&self, nodes: &[Node]) -> bool {
        nodes.iter().any(|n| match n {
            Node::Cert(_) => true,
            Node::Alt(a) => a.iter().any(|x| self.needs_source(x)),
            Node::Opt { body, .. } => self.needs_source(body),
            _ => false,
        })
    }

    /// The source of the weakest facts behind a sentence, if the speaker has one that may stand behind that certainty.
    fn source_for(&self, f: &Frame, cert: Certainty, view: &View, req: &Request) -> Option<String> {
        let k = f.needs.iter().filter_map(|k| view.know.get(k)).find(|k| k.certainty == cert && k.source.is_some())?;
        let s = self.lang.sources.get(k.source.as_deref()?)?;
        if (!s.certainty.is_empty() && !s.certainty.contains(&cert)) || !s.roles.allows(&req.speaker.role) {
            return None;
        }
        Some(s.kind.clone())
    }

    fn wrapper_for(&self, cert: Certainty, slot: &str, req: &Request, source: Option<&Source>, _f: &Frame) -> Vec<usize> {
        (0..self.lang.wrappers.len())
            .filter(|&i| {
                let w = &self.lang.wrappers[i];
                w.certainty == cert && w.slot.allows(slot) && w.channel.allows(req.channel) && (source.is_some() || !self.needs_source(&self.lang.wrapper_nodes[i]))
            })
            .collect()
    }

    fn realise(&self, nodes: &[Node], cx: &mut Cx) -> Result<String, String> {
        let mut s = String::new();
        for n in nodes {
            match n {
                Node::Text(t) => s.push_str(t),
                Node::Field { key, mode } => {
                    s.push_str(&self.field(key, mode, cx)?);
                    cx.stated.insert(key.clone());
                }
                Node::Lex { concept, form } => s.push_str(&self.lex(concept, form, cx)?),
                Node::Col { head, slot, form } => s.push_str(&self.col(head, slot, form, cx)?),
                Node::Cert(_) => s.push_str(&cx.source.ok_or("no source")?.phrase),
                Node::Clause => s.push_str(&cx.pending.pop().ok_or("no clause")?),
                Node::Alt(alts) => {
                    let k = cx.key("alt");
                    let i = (hash(&k, cx.req.seed) % alts.len() as u64) as usize;
                    s.push_str(&self.realise(&alts[i], cx)?);
                }
                Node::Opt { key, body } => {
                    if cx.view.cert_of(key).is_some_and(|c| firm_enough(c, cx.cert)) {
                        cx.stated.insert(key.clone());
                        s.push_str(&self.realise(body, cx)?);
                    }
                }
            }
        }
        Ok(s)
    }

    fn field(&self, key: &str, mode: &str, cx: &mut Cx) -> Result<String, String> {
        let v = cx.view.facts.get(key).ok_or_else(|| format!("fact {key} not known"))?.clone();
        Ok(match v {
            Value::Ent(r) => match mode {
                "has" => (if r.plural { "have" } else { "has" }).into(),
                "is" => (if r.plural { "are" } else { "is" }).into(),
                "was" => (if r.plural { "were" } else { "was" }).into(),
                "poss" => {
                    let n = if cx.mentioned.contains(&r.id) { r.short_name() } else { &r.full };
                    cx.mentioned.insert(r.id.clone());
                    if n.ends_with('s') { format!("{n}'") } else { format!("{n}'s") }
                }
                "short" => r.short_name().to_string(),
                "role" => r.descriptors.iter().find(|d| d.kind == "role").map(|d| d.text.clone()).unwrap_or_else(|| r.short_name().to_string()),
                "full" => {
                    cx.mentioned.insert(r.id.clone());
                    r.full.clone()
                }
                _ => {
                    let first = cx.mentioned.insert(r.id.clone());
                    if !first {
                        r.short_name().to_string()
                    } else if mode == "desc" {
                        let pref = ["age_role", "club_role", "role", "title", "origin"];
                        match pref.iter().find_map(|k| r.descriptors.iter().find(|d| d.kind == *k)) {
                            Some(d) => format!("{} {}", d.text, r.full),
                            None => r.full.clone(),
                        }
                    } else {
                        r.full.clone()
                    }
                }
            },
            Value::Money { money } => match mode {
                "words" => text::money(money, true),
                _ => text::money(money, cx.req.speaker.rs_words),
            },
            Value::Num(n) => match mode {
                "words" => text::number_words(n),
                "ord" => text::ordinal(n),
                "ordw" => text::ordinal_word(n),
                "grouped" => text::group_indian(n),
                m if m.starts_with("n=") => format!("{n} {}", text::plural(n, &m[2..])),
                _ => n.to_string(),
            },
            Value::Date { date } => {
                let d = text::parse_date(&date).ok_or("bad date")?;
                if mode == "abs" { text::absolute(d) } else { text::when(d, cx.req.now) }
            }
            Value::List(l) => text::list(&l),
            Value::Text(t) => t,
            Value::Bool(b) => (if b { "yes" } else { "no" }).into(),
        })
    }

    fn variant_ok(&self, v: &Variant, cx: &Cx) -> bool {
        let sp = &cx.req.speaker;
        if !v.channels.allows(cx.req.channel) || !v.roles.allows(&sp.role) || v.not_certainty.contains(&cx.cert) {
            return false;
        }
        if let Some(min) = v.min_certainty {
            match (cx.cert.rank(), min.rank()) {
                (Some(a), Some(b)) if a >= b => {}
                _ => return false,
            }
        }
        v.requires.iter().all(|k| cx.view.facts.contains_key(k)) && (v.when.is_empty() || cond::eval(&v.when, &cx.view.facts))
    }

    fn lex(&self, concept: &str, form: &str, cx: &mut Cx) -> Result<String, String> {
        let c = self.lang.concepts.get(concept).ok_or_else(|| format!("no concept {concept}"))?;
        let form = if form == "agr" { if cx.source.is_some_and(|s| !s.plural) { "third" } else { "base" } } else { form };
        let cands: Vec<&Variant> = c.variants.iter().filter(|v| v.forms.contains_key(form) && self.variant_ok(v, cx)).collect();
        if cands.is_empty() {
            return Err(format!("no usable variant of {concept}.{form}"));
        }
        let ident = |v: &Variant| v.forms.values().cloned().collect::<Vec<_>>().join("/");
        let remembered = if c.pos == "noun" { cx.memo.get(concept).and_then(|m| cands.iter().find(|v| &ident(v) == m)) } else { None };
        let v = match remembered {
            Some(v) => *v,
            None => {
                let scores: Vec<f32> = cands.iter().map(|v| fit(&cx.req.speaker.voice, &v.register, &v.voice) - cx.tr.penalty_word(&v.forms[form])).collect();
                let k = cx.key(concept);
                let v = cands[choose(&scores, &k, cx.req.seed)];
                if c.pos == "noun" {
                    cx.memo.insert(concept.to_string(), ident(v));
                }
                v
            }
        };
        let w = v.forms[form].clone();
        cx.tr.note_word(&w);
        Ok(w)
    }

    fn col(&self, head: &str, slot: &str, form: &str, cx: &mut Cx) -> Result<String, String> {
        let c = self.lang.collocations.iter().find(|c| c.head == head && c.slot == slot).ok_or("no collocation")?;
        let cands: Vec<_> =
            c.items.iter().filter(|i| (i.when.is_empty() || cond::eval(&i.when, &cx.view.facts)) && i.channels.allows(cx.req.channel) && (form == "base" || i.forms.contains_key(form))).collect();
        if cands.is_empty() {
            return if c.optional { Ok(String::new()) } else { Err(format!("no collocate for {head}.{slot}")) };
        }
        let word = |i: &crate::data::CollocItem| if form == "base" { i.text.clone() } else { i.forms[form].clone() };
        let scores: Vec<f32> = cands.iter().map(|i| fit(&cx.req.speaker.voice, &i.register, &i.voice) - cx.tr.penalty_word(&word(i))).collect();
        let k = cx.key(head);
        let i = cands[choose(&scores, &k, cx.req.seed)];
        // A collocate that rests on a fact (`second` on `round`) has stated it.
        if i.states {
            for key in cond::keys(&i.when) {
                cx.stated.insert(key);
            }
        }
        let w = word(i);
        cx.tr.note_word(&w);
        Ok(w)
    }
}

/// `event` in a frame is a comma-separated list of kinds; `*` matches anything and `group.*` matches every kind in the group.
pub fn event_matches(pattern: &str, kind: &str) -> bool {
    pattern.split(',').map(str::trim).any(|p| p == "*" || p == kind || p.strip_suffix(".*").is_some_and(|g| kind.strip_prefix(g).is_some_and(|r| r.starts_with('.'))))
}

fn clause_lowerable(s: &str) -> bool {
    let first = s.split(' ').next().unwrap_or("");
    matches!(first, "The" | "A" | "An" | "There" | "It" | "This" | "That" | "These" | "Those" | "Both" | "Neither")
}

/// Whitespace, punctuation spacing, articles, capital and (for sentences) full stop.
pub fn finish(raw: &str, style: &str) -> String {
    let mut s = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    for p in [" ,", " .", " ;", " :", " !", " ?", " )"] {
        s = s.replace(p, &p[1..]);
    }
    s = s.replace("( ", "(");
    s = text::fix_articles(&s);
    if style == "raw" {
        return s;
    }
    s = text::capitalise(&s);
    if style == "sentence" && !s.is_empty() && !s.ends_with(['.', '!', '?', '"', '”', ')']) {
        s.push('.');
    }
    s
}
