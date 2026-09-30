//! Read-only editorial views of the recorded world, filtered through the viewer's knowledge.

use pw_core::{ClubId, Date, StoryId};
use pw_world::event::EventKind as E;
use pw_world::media::{Story, StoryLink};
use std::collections::HashMap;
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};
use crate::narrative;

fn named(r: Ref, name: String) -> Value {
    serde_json::to_value(Named::new(r, name)).unwrap_or(Value::Null)
}

fn story_clubs(s: &Story, club: ClubId) -> bool {
    club.is_some() && (s.club == club || s.other_club == club)
}

fn following(c: &Ctx, s: &Story) -> bool {
    c.w.followed.iter().any(|&t| story_clubs(s, c.w.teams[t].club))
}

fn personal(c: &Ctx, s: &Story) -> bool {
    c.me().is_some_and(|me| s.person == me) || story_clubs(s, c.my_club())
}

fn graphic(c: &Ctx, s: &Story) -> Value {
    match c.w.media.links.get(&s.id) {
        Some(StoryLink::Fixture { uid, home, away, hg, ag, .. }) => json!({
            "kind": "result", "match": Ref::fixture(*uid),
            "home": named(Ref::club(*home), c.club_short(*home)),
            "away": named(Ref::club(*away), c.club_short(*away)),
            "score": [hg, ag],
        }),
        _ if s.club.is_some() => json!({"kind": "club", "club": named(Ref::club(s.club), c.club_name(s.club))}),
        _ if s.person.is_some() => json!({"kind": "person", "person": named(Ref::person(s.person), c.person_name(s.person))}),
        _ => json!({"kind": "type"}),
    }
}

fn summary(c: &Ctx, s: &Story) -> Value {
    json!({
        "id": s.id.0, "date": s.date.0, "outlet": pw_narrate::press::outlet_name(c.w, s),
        "headline": c.headline(s), "kind": format!("{:?}", s.kind), "claim": s.claim_type.label(),
        "about_you": c.me().is_some_and(|me| s.person == me),
        "following": following(c, s), "graphic": graphic(c, s),
        "subject": if s.person.is_some() { named(Ref::person(s.person), c.person_name(s.person)) }
            else if s.club.is_some() { named(Ref::club(s.club), c.club_name(s.club)) }
            else { Value::Null },
    })
}

/// Ranked public stories; no hidden-result story enters the response at all.
pub fn feed(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let filter = args.get("filter").and_then(Value::as_str).unwrap_or("for_you");
    if !matches!(filter, "for_you" | "following" | "world") {
        return Err(ApiError::Bad("unknown news filter".into()));
    }
    let limit = args.get("limit").and_then(Value::as_u64).map_or(30, |n| n.clamp(5, 60) as usize);
    let cutoff = c.w.date.add_days(-60);
    let mut stories: Vec<(&Story, i32)> = c.w.media.stories.iter().rev().take_while(|s| s.date >= cutoff)
        .filter(|s| !c.story_spoils(s))
        .filter(|s| filter != "following" || following(c, s))
        .map(|s| {
            let age = (c.w.date.0 - s.date.0).max(0);
            let relevance = if filter == "world" { 0 } else { 40 * i32::from(personal(c, s)) + 20 * i32::from(following(c, s)) };
            (s, i32::from(s.news) * 2 + relevance - age * 3)
        })
        .collect();
    stories.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.date.cmp(&a.0.date)).then(b.0.id.0.cmp(&a.0.id.0)));
    let mut picked = Vec::with_capacity(limit);
    let mut kinds = HashMap::new();
    let mut subjects = HashMap::new();
    // Two outlets running the same headline on the same day are one piece of news to a reader.
    let mut seen_headlines: std::collections::HashSet<(i32, String)> = std::collections::HashSet::new();
    for (s, _) in &stories {
        if !seen_headlines.insert((s.date.0, c.headline(s))) {
            continue;
        }
        let kind_count = *kinds.get(&s.kind).unwrap_or(&0);
        let subject = if s.person.is_some() { Some((true, s.person.0)) } else if s.club.is_some() { Some((false, s.club.0)) } else { None };
        let subject_count = subject.and_then(|key| subjects.get(&key).copied()).unwrap_or(0);
        if kind_count >= if matches!(s.kind, pw_world::media::StoryKind::Interview) { 2 } else { 5 }
            || (subject.is_some() && subject_count >= 2) {
            continue;
        }
        *kinds.entry(s.kind).or_insert(0usize) += 1;
        if let Some(key) = subject { *subjects.entry(key).or_insert(0usize) += 1; }
        picked.push(summary(c, s));
        if picked.len() == limit { break; }
    }
    Ok(json!({"stories": picked, "filter": filter}))
}

/// The full article is shared by the News page and editorial cards.
pub fn story(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing story".into()))? as u32;
    let s = c.w.media.stories.get(StoryId(id)).ok_or_else(|| ApiError::NotFound("story".into()))?;
    if c.story_spoils(s) {
        return Err(ApiError::NotFound("story".into()));
    }
    let mut out = summary(c, s);
    out["body"] = json!(c.story_body(s));
    Ok(out)
}

/// A compact chronology of visible events and articles for the portal and chrome.
pub fn pulse(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let limit = args.get("limit").and_then(Value::as_u64).map_or(12, |n| n.clamp(3, 30) as usize);
    let cutoff = Date(c.w.date.0 - 21);
    let mut items = Vec::new();
    for e in c.w.events.since(cutoff).iter().rev() {
        if !narrative::visible(c, e) {
            continue;
        }
        let (target, label) = match e.kind {
            E::Published { story } => {
                let Some(s) = c.w.media.stories.get(story) else { continue };
                if c.story_spoils(s) { continue; }
                (json!({"k": "news", "id": story.0}), "News")
            }
            _ => (json!(narrative::primary(c, &e.kind)), narrative::label(&e.kind)),
        };
        if target.is_null() { continue; }
        items.push(json!({"id": e.id.0, "date": e.date.0, "label": label, "parts": narrative::describe(c, e), "target": target}));
        if items.len() >= limit { break; }
    }
    Ok(json!({"items": items}))
}
