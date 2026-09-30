//! Automated checks on rendered text. Every check reports a plain-language issue so a failing corpus case says what is wrong and where.

use crate::data::Lang;
use crate::model::*;
use crate::text;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub check: &'static str,
    pub detail: String,
}

fn issue(check: &'static str, detail: impl Into<String>) -> Issue {
    Issue { check, detail: detail.into() }
}

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '₹').to_lowercase()).filter(|w| !w.is_empty()).collect()
}

const ALLOWED_DOUBLES: [&str; 3] = ["had", "that", "very"];
const AUX_START: [&str; 12] = ["have", "has", "had", "is", "are", "was", "were", "will", "would", "been", "being", "be"];
const DANGLING_END: [&str; 17] = ["and", "of", "the", "a", "an", "for", "to", "with", "that", "by", "from", "at", "on", "in", "but", "or", "as"];
const STRONG: [&str; 6] = ["confirmed", "officially", "announced", "it is official", "has been confirmed", "official:"];
const HEDGE: [&str; 9] = ["reportedly", "rumour", "allegedly", "apparently", "according to", "talk that", "claimed", "sources", "it is understood"];

/// Structural checks that need no knowledge of the event.
pub fn check_text(t: &str) -> Vec<Issue> {
    let mut out = Vec::new();
    if t.trim().is_empty() {
        return vec![issue("empty", "no text")];
    }
    if t.contains(['{', '}']) || t.contains("[?") || t.contains('$') {
        out.push(issue("placeholder", format!("unresolved placeholder in {t:?}")));
    }
    if t.contains("  ") {
        out.push(issue("spacing", "double space"));
    }
    for bad in [" ,", " .", " ;", " :", ",,", "..", ",.", ".,", "?.", "!.", "( ", " )", ";;", "::", ", ,", "—-"] {
        if t.contains(bad) && !t.contains("...") {
            out.push(issue("punctuation", format!("`{bad}` in {t:?}")));
        }
    }
    if t.matches('(').count() != t.matches(')').count() || t.matches('"').count() % 2 == 1 {
        out.push(issue("punctuation", "unbalanced brackets or quotes"));
    }
    let w = words(t);
    for pair in w.windows(2) {
        if pair[0] == pair[1] && !ALLOWED_DOUBLES.contains(&pair[0].as_str()) && pair[0].chars().any(|c| c.is_alphabetic()) {
            out.push(issue("doubled_word", format!("`{} {}`", pair[0], pair[1])));
        }
    }
    let raw: Vec<&str> = t.split(' ').collect();
    for (i, pair) in raw.windows(2).enumerate() {
        let (a, n) = (pair[0], pair[1]);
        let al = a.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
        if (al == "a" || al == "an") && a.chars().all(|c| c.is_alphabetic()) {
            let want_an = text::vowel_sound(n);
            if want_an != (al == "an") {
                out.push(issue("article", format!("`{a} {n}` at word {i}")));
            }
        }
    }
    // Sentences.
    for s in t.split_inclusive(['.', '!', '?']) {
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        let first = s.split(' ').next().unwrap_or("");
        let fl = first.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
        if AUX_START.contains(&fl.as_str()) && !s.ends_with('?') {
            out.push(issue("missing_subject", format!("sentence begins with `{first}`: {s:?}")));
        }
        if first.chars().next().is_some_and(|c| c.is_lowercase()) && s != t.trim() {
            out.push(issue("capitalisation", format!("sentence starts lowercase: {s:?}")));
        }
    }
    if t.chars().next().is_some_and(|c| c.is_lowercase()) {
        out.push(issue("capitalisation", "text starts with a lowercase letter"));
    }
    if let Some(last) = w.last()
        && DANGLING_END.contains(&last.as_str())
    {
        out.push(issue("empty_clause", format!("ends with `{last}`")));
    }
    for empty in ["that ,", "for .", "of .", "by ,", "with .", "and ,", "to .", "in ."] {
        if t.contains(empty) {
            out.push(issue("empty_clause", format!("`{empty}`")));
        }
    }
    if let Some(dup) = repeated_fragment(&w, 4) {
        out.push(issue("repeated_fragment", format!("`{dup}` appears twice")));
    }
    out
}

fn repeated_fragment(w: &[String], n: usize) -> Option<String> {
    let mut seen = BTreeSet::new();
    for g in w.windows(n) {
        let k = g.join(" ");
        if !seen.insert(k.clone()) {
            return Some(k);
        }
    }
    None
}

/// Checks one part against its channel, the event date and its certainty.
pub fn check_part(lang: &Lang, req: &Request, part: &Part) -> Vec<Issue> {
    let mut out = check_text(&part.text);
    let low = part.text.to_lowercase();
    if let Some(ch) = lang.channels.get(req.channel) {
        for f in &ch.forbid {
            if part.text.contains(f.as_str()) {
                out.push(issue("channel_fit", format!("`{f}` does not belong in {}", req.channel)));
            }
        }
        if let Some(sd) = ch.slots.iter().find(|s| s.slot == part.slot)
            && sd.max_chars > 0
            && part.text.chars().count() > sd.max_chars
        {
            out.push(issue("channel_fit", format!("{} is {} chars, limit {}", part.slot, part.text.chars().count(), sd.max_chars)));
        }
    }
    let delta = text::delta(req.event.date, req.now);
    let has = |needle: &str| low.split(|c: char| !c.is_alphanumeric()).any(|w| w == needle);
    if delta < 0
        && (has("tomorrow")
            || low.contains("next monday")
            || low.contains("next tuesday")
            || low.contains("next wednesday")
            || low.contains("next thursday")
            || low.contains("next friday")
            || low.contains("next saturday")
            || low.contains("next sunday"))
    {
        out.push(issue("date", "future wording for a past event"));
    }
    if delta > 0
        && (has("yesterday")
            || low.contains("last monday")
            || low.contains("last tuesday")
            || low.contains("last wednesday")
            || low.contains("last thursday")
            || low.contains("last friday")
            || low.contains("last saturday")
            || low.contains("last sunday"))
    {
        out.push(issue("date", "past wording for a future event"));
    }
    if delta != -1 && has("yesterday") {
        out.push(issue("date", "`yesterday` but the event is not from yesterday"));
    }
    if delta != 0 && has("today") && part.certainty != Certainty::Unknown {
        out.push(issue("date", "`today` but the event is not from today"));
    }
    if delta != 1 && has("tomorrow") {
        out.push(issue("date", "`tomorrow` but the event is not tomorrow"));
    }
    match part.certainty {
        Certainty::Rumour | Certainty::Speculation | Certainty::SourceClaim => {
            if let Some(s) = STRONG.iter().find(|s| low.contains(*s)) {
                out.push(issue("certainty", format!("firm wording `{s}` at {}", part.certainty.key())));
            }
        }
        Certainty::Fact => {
            if let Some(h) = HEDGE.iter().find(|h| low.contains(*h)) {
                out.push(issue("certainty", format!("hedge `{h}` on a stated fact")));
            }
        }
        _ => {}
    }
    out
}

/// Everything: per-part checks, entity repetition across the whole text, and facts the speaker did not know appearing anyway.
pub fn check_rendered(lang: &Lang, req: &Request, r: &Rendered) -> Vec<Issue> {
    let mut out = Vec::new();
    for p in &r.parts {
        for i in check_part(lang, req, p) {
            out.push(Issue { check: i.check, detail: format!("[{}] {}", p.slot, i.detail) });
        }
    }
    let all = r.text();
    let event = req.event;
    for v in event.facts.values() {
        if let Value::Ent(e) = v {
            for p in &r.parts {
                if whole_word_count(&p.text, &e.full) > 1 {
                    out.push(issue("duplicate_entity", format!("`{}` twice in [{}] {:?}", e.full, p.slot, p.text)));
                }
            }
        }
    }
    // A headline legitimately shares wording with the lede; only body sentences must not repeat each other.
    let mut body: String = r.parts.iter().filter(|p| !is_display_slot(&p.slot)).map(|p| p.text.as_str()).collect::<Vec<_>>().join(" ");
    // The same source may be named more than once; that is attribution, not a repeated phrase.
    for src in lang.sources.values() {
        body = body.replace(&src.phrase, "").replace(&crate::text::capitalise(&src.phrase), "");
    }
    let w = words(&body);
    if let Some(dup) = repeated_fragment(&w, 5) {
        out.push(issue("repeated_fragment", format!("`{dup}` repeated across parts")));
    }
    // Leaks: any fact the speaker does not know must not surface in any form.
    if lang.events.contains_key(&event.kind) {
        let low = all.to_lowercase();
        for (k, v) in &event.facts {
            let known = req.speaker.knows.get(k).is_some_and(|k| k.certainty != Certainty::Unknown);
            if known {
                continue;
            }
            for form in v.surface_forms() {
                let short = match v {
                    Value::Num(n) => n.abs() < 100,
                    _ => form.chars().count() < 4,
                };
                if !short && low.contains(&form.to_lowercase()) {
                    out.push(issue("leak", format!("fact `{k}` ({form}) appears but the speaker does not know it")));
                }
            }
        }
    }
    out
}

/// Occurrences of `needle` that are not part of a longer word ("India" inside "Indian" does not count).
fn whole_word_count(hay: &str, needle: &str) -> usize {
    let mut n = 0;
    let mut from = 0;
    while let Some(i) = hay[from..].find(needle) {
        let start = from + i;
        let end = start + needle.len();
        let before_ok = hay[..start].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let after_ok = hay[end..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            n += 1;
        }
        from = end;
    }
    n
}
