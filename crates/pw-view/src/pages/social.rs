//! Social media as a person sees it: a thin, personal feed of a much bigger world, and how they
//! can join in. Every post is rendered by narration from what the author recorded; the sender's
//! own reasons and opinions stay in the world.

use pw_world::socialnet::{AccountKind, NO_POST, Post};
use serde_json::{Value, json};

use super::me::named;
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Ref};

pub fn kind_label(k: AccountKind) -> &'static str {
    match k {
        AccountKind::Supporter | AccountKind::Casual => "Supporter",
        AccountKind::Hardcore => "Lifelong supporter",
        AccountKind::Ultra => "Ultra",
        AccountKind::Local => "Local",
        AccountKind::International => "Overseas fan",
        AccountKind::Stats => "Stats account",
        AccountKind::FanNews => "Fan news",
        AccountKind::AcademyWatcher => "Academy watcher",
        AccountKind::RumourMill => "Rumours",
        AccountKind::Neutral => "Neutral",
        AccountKind::Provocateur => "Provocateur",
        AccountKind::Person => "Verified",
        AccountKind::ClubOfficial => "Club",
    }
}

/// One post, with the post it answers or quotes shown briefly.
pub fn post_json(c: &Ctx, p: &Post, depth: u8) -> Value {
    if c.post_spoils(p) {
        return json!({
            "id": p.id, "date": p.date.0,
            "author": {"handle": "", "display": "A post", "kind": "Held back", "followers": 0, "person": Value::Null, "you": false},
            "text": "A post about an unrevealed result", "about": Value::Null,
            "likes": 0, "reposts": 0, "replies": 0, "parent": Value::Null, "quoted": Value::Null,
        });
    }
    let w = c.w;
    let a = &w.net.accounts[p.author as usize];
    let author_person = if a.person.is_some() { Some(named(Ref::person(a.person), c.person_name(a.person))) } else { None };
    let about = if p.about.is_some() && p.about != a.person { Some(named(Ref::person(p.about), c.person_name(p.about))) } else { None };
    let parent = |id: u32| -> Value {
        if depth > 0 {
            return Value::Null;
        }
        w.net.post(id).filter(|q| !c.post_spoils(q)).map_or(Value::Null, |q| post_json(c, q, depth + 1))
    };
    json!({
        "id": p.id, "date": p.date.0,
        "author": {"handle": a.handle, "display": a.display, "kind": kind_label(a.kind), "followers": a.followers, "person": author_person, "you": Some(a.person) == c.me()},
        "text": c.post_text(p),
        "about": about, "likes": p.likes, "reposts": p.reposts, "replies": p.replies,
        "reply_to": if p.reply_to != NO_POST { json!(p.reply_to) } else { Value::Null },
        "quote_of": if p.quote_of != NO_POST { json!(p.quote_of) } else { Value::Null },
        "parent": if p.reply_to != NO_POST { parent(p.reply_to) } else { Value::Null },
        "quoted": if p.quote_of != NO_POST { parent(p.quote_of) } else { Value::Null },
    })
}

/// The inhabited person's feed.
pub fn feed(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let me = c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world. Inhabit someone to read their feed.".into()))?;
    let n = args.get("limit").and_then(Value::as_u64).map_or(40, |n| n.clamp(5, 100) as usize);
    let ids = pw_sim::socialnet::feed(c.w, me, n);
    // One account saying the same words twice on one day is shown once.
    let mut said: std::collections::HashSet<(u32, i32, String)> = std::collections::HashSet::new();
    let posts: Vec<Value> = ids
        .into_iter()
        .filter_map(|id| c.w.net.post(id))
        .filter(|p| !c.post_spoils(p) && !c.post_text(p).trim().is_empty())
        .filter(|p| said.insert((p.author, p.date.0, c.post_text(p))))
        .map(|p| post_json(c, p, 0))
        .collect();
    let mine = c.w.net.account_of(me).map(|a| {
        let acc = &c.w.net.accounts[a as usize];
        json!({"handle": acc.handle, "followers": acc.followers})
    });
    Ok(json!({"posts": posts, "account": mine}))
}

/// Replies under a post, newest last.
pub fn thread(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing post".into()))? as u32;
    let p = c.w.net.post(id).ok_or_else(|| ApiError::NotFound("post".into()))?;
    if c.post_spoils(p) { return Err(ApiError::NotFound("post".into())); }
    let replies: Vec<Value> = c.w.net.posts.iter().filter(|r| r.reply_to == id && !c.post_spoils(r) && !c.post_text(r).trim().is_empty()).take(30).map(|r| post_json(c, r, 1)).collect();
    Ok(json!({"post": post_json(c, p, 0), "replies": replies}))
}
