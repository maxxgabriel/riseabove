//! The world inbox: conversations with the people who reached the inhabited person, and the
//! replies they can send. A message stores only a reference to something that happened; every word
//! here is rendered from that source. A reply is not text: it becomes an intent or a decision
//! answer that the world applies by its own rules, and any answer from the other side comes later,
//! from their own mind, or not at all.

use pw_core::PersonId;
use pw_world::World;
use pw_world::decision::DecisionKind;
use pw_world::event::EventKind as E;
use pw_world::inbox::{Message, MsgSource, Reply, Thread, ThreadKey};
use pw_world::socialnet::Concept;
use serde_json::{Value, json};

use super::inbox::{meeting_json, options_json, state_of};
use super::me::named;
use super::social::post_json;
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Ref};
use crate::narrative;
use crate::session::Session;

fn need(c: &Ctx) -> ApiResult<PersonId> {
    c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world. Inhabit someone to read their messages.".into()))
}

fn short(s: &str, n: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= n {
        return s.to_string();
    }
    let cut: String = s.chars().take(n).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(a, _)| a).trim_end_matches([',', '.', ';', ':']);
    format!("{cut}…")
}

// ---- replies ---------------------------------------------------------------------------------------

/// A stable name for a reply, so the client can send what it saw even if the list has shifted.
fn reply_key(r: Reply) -> String {
    match r {
        Reply::Answer(k) => format!("answer:{k}"),
        Reply::Thank => "thank".into(),
        Reply::AskToMeet { tone } => format!("meet:{}", tone.label()),
        Reply::KeepQuiet => "quiet".into(),
        Reply::PassOn { to } => format!("pass:{}", to.0),
        Reply::Respond { stance } => format!("respond:{stance:?}"),
        Reply::PostReply { concept } => format!("post:{concept:?}"),
        Reply::Ignore => "ignore".into(),
    }
}

fn concept_word(c: Concept) -> &'static str {
    super::act::concept_label(c)
}

fn reply_label(c: &Ctx, m: &Message, r: Reply) -> String {
    match r {
        Reply::Answer(k) => format!("Option {}", k + 1),
        Reply::Thank => format!("Thank {}", c.person_name(m.from)),
        Reply::AskToMeet { tone } => format!("Ask {} to talk it through ({})", c.person_name(m.from), tone.label()),
        Reply::KeepQuiet => "Keep it to yourself".into(),
        Reply::PassOn { to } => format!("Tell {}", c.person_name(to)),
        Reply::Respond { stance } => format!("Answer it on the record: {}", pw_narrate::press::stance_label(stance).to_lowercase()),
        Reply::PostReply { concept } => format!("Reply online: {}", concept_word(concept).to_lowercase()),
        Reply::Ignore => "Leave it".into(),
    }
}

/// What the reply does, in the world's own terms.
fn reply_effect(r: Reply) -> Option<&'static str> {
    Some(match r {
        Reply::Answer(_) => return None,
        Reply::Thank => "Sent as a thank-you the next day. They will remember you were grateful.",
        Reply::AskToMeet { .. } => "Asks for a meeting the next day. They decide whether to take it, and how it goes depends on how they see you.",
        Reply::KeepQuiet => "Nothing happens. It is recorded that you kept it to yourself.",
        Reply::PassOn { .. } => "Tells them what you heard, as you understood it. It spreads from there by the usual ways.",
        Reply::Respond { .. } => "Speaks to the press the next day. The quote is on the record and can be repeated.",
        Reply::PostReply { .. } => "Posts the next day from your account. Others can reply, repost and quote it.",
        Reply::Ignore => "Nothing happens.",
    })
}

fn replies_json(c: &Ctx, m: &Message) -> Vec<Value> {
    if m.replied.is_some() {
        return Vec::new();
    }
    pw_sim::inbox::options(c.w, m.id)
        .into_iter()
        .filter(|r| !matches!(r, Reply::Answer(_)))
        .map(|r| json!({"key": reply_key(r), "label": reply_label(c, m, r), "effect": reply_effect(r), "quiet": matches!(r, Reply::KeepQuiet | Reply::Ignore)}))
        .collect()
}

fn replied_json(c: &Ctx, m: &Message) -> Value {
    match m.replied {
        None => Value::Null,
        Some(rr) => {
            let label = match (m.source, rr.reply) {
                (MsgSource::Decision { decision }, Reply::Answer(k)) => {
                    c.w.decisions.all.get(decision).and_then(|d| d.options.get(usize::from(k)).map(|ch| super::inbox::option_text(c, d, ch))).unwrap_or_else(|| "You answered".into())
                }
                (_, r) => reply_label(c, m, r),
            };
            json!({"label": label, "date": rr.date.0})
        }
    }
}

// ---- messages ---------------------------------------------------------------------------------------

fn pending_question(w: &World, who: PersonId, conference: u32, question: u8) -> Option<pw_core::DecisionId> {
    w.decisions.pending_for(who).find(|(_, d)| matches!(d.kind, DecisionKind::PressQuestion { conference: c, question: q } if c == conference && q == question)).map(|(id, _)| id)
}

/// The one-line form of a message, for lists.
fn line(c: &Ctx, m: &Message) -> String {
    let w = c.w;
    let me = m.to;
    match m.source {
        MsgSource::Decision { decision } => w.decisions.all.get(decision).map_or_else(String::new, |d| pw_narrate::choices::title(w, d)),
        MsgSource::Tell { info, from } => match pw_narrate::grapevine::belief(w, info, me) {
            Some((what, _)) => format!("{} told you {}.", c.person_name(from), what),
            // What was said has since been forgotten (by the world's grapevine, not only by you); the day it was said still tells two apart.
            None => format!("On {}, {} told you something you no longer remember clearly.", crate::fmt::date(m.date), c.person_name(from)),
        },
        MsgSource::Meeting { event } | MsgSource::Private { event } => w.events.get(event).map_or_else(|| "Something from earlier that is no longer on record.".to_string(), |e| {
            // An event the narration has no sentence for is still listed by what it is, never as a blank line.
            pw_narrate::events::line(w, e, me).filter(|t| !t.trim().is_empty()).unwrap_or_else(|| {
                let said: String = narrative::describe(c, e).iter().map(|p| p.t.as_str()).collect();
                if said.trim().is_empty() { narrative::label(&e.kind).to_string() } else { said }
            })
        }),
        MsgSource::Story { story } => c.headline(&w.media.stories[story]),
        MsgSource::Mention { post } => w.net.post(post).map_or_else(String::new, |p| format!("{}: {}", w.net.accounts[p.author as usize].display, c.post_text(p))),
        MsgSource::Question { conference, question } => pw_narrate::press::question(w, conference, question),
    }
}

/// Whether what a message was about is still in the records (old events are compacted away). A note that says only that something
/// is gone is not worth showing.
fn still_on_record(c: &Ctx, m: &Message) -> bool {
    let w = c.w;
    match m.source {
        MsgSource::Meeting { event } | MsgSource::Private { event } => w.events.get(event).is_some(),
        MsgSource::Mention { post } => w.net.post(post).is_some_and(|p| !c.post_text(p).trim().is_empty()),
        _ => true,
    }
}

fn kind_key(s: &MsgSource) -> &'static str {
    match s {
        MsgSource::Decision { .. } => "decision",
        MsgSource::Tell { .. } => "tell",
        MsgSource::Meeting { .. } => "meeting",
        MsgSource::Story { .. } => "story",
        MsgSource::Mention { .. } => "mention",
        MsgSource::Private { .. } => "private",
        MsgSource::Question { .. } => "question",
    }
}

fn message_json(c: &Ctx, m: &Message) -> Value {
    let w = c.w;
    let me = m.to;
    let from = if m.from.is_some() { Some(named(Ref::person(m.from), c.person_name(m.from))) } else { None };
    let mut v = json!({
        "id": m.id, "kind": kind_key(&m.source), "date": m.date.0, "from": from, "read": m.read,
        "text": line(c, m), "replied": replied_json(c, m), "replies": replies_json(c, m),
    });
    match m.source {
        MsgSource::Decision { decision } => {
            if let Some(d) = w.decisions.all.get(decision) {
                v["decision"] = json!({"id": format!("d{}", decision.0), "state": state_of(d), "title": d.kind.title(), "deadline": d.deadline.0, "options": options_json(c, d), "kind": super::inbox::kind_key(&d.kind)});
            }
        }
        MsgSource::Question { conference, question } => {
            if let Some(cf) = w.pressroom.conferences.get(conference as usize) {
                v["press"] = json!({"club": named(Ref::club(cf.club), c.club_name(cf.club)), "date": cf.date.0, "number": usize::from(question) + 1, "of": cf.questions.len()});
                if let Some(id) = pending_question(w, me, conference, question) {
                    let d = &w.decisions.all[id];
                    v["decision"] = json!({"id": format!("d{}", id.0), "state": state_of(d), "title": d.kind.title(), "deadline": d.deadline.0, "options": options_json(c, d), "kind": super::inbox::kind_key(&d.kind)});
                } else if let Some(q) = cf.answers.get(usize::from(question)).copied().flatten().and_then(|q| w.pressroom.quotes.get(q as usize)) {
                    v["answered_with"] = json!(pw_narrate::press::stance_label(q.stance));
                }
            }
        }
        MsgSource::Tell { info, .. } => {
            if let Some((_, sure)) = pw_narrate::grapevine::belief(w, info, me) {
                v["sureness"] = json!(crate::model::sureness(sure));
            }
        }
        MsgSource::Meeting { event } => {
            if let Some(E::Meeting { meeting, .. }) = w.events.get(event).map(|e| &e.kind) {
                v["meeting"] = meeting_json(c, &w.meetings.list[*meeting]);
            }
            if let Some(e) = w.events.get(event) {
                v["parts"] = json!(narrative::describe(c, e));
            }
        }
        MsgSource::Private { event } => {
            if let Some(e) = w.events.get(event) {
                v["parts"] = json!(narrative::describe(c, e));
                v["label"] = json!(narrative::label(&e.kind));
            }
        }
        MsgSource::Story { story } => {
            let s = &w.media.stories[story];
            v["story"] = json!({"id": story.0, "outlet": pw_narrate::press::outlet_name(w, s), "headline": c.headline(s), "body": c.story_body(s), "date": s.date.0});
        }
        MsgSource::Mention { post } => {
            if let Some(p) = w.net.post(post) {
                v["post"] = post_json(c, p, 0);
            }
        }
    }
    v
}

// ---- threads ----------------------------------------------------------------------------------------

fn thread_title(c: &Ctx, t: &Thread) -> (String, Option<Value>, &'static str) {
    let w = c.w;
    match t.key {
        ThreadKey::With(p) => (c.person_name(p), Some(named(Ref::person(p), c.person_name(p))), "person"),
        ThreadKey::Press(id) => {
            let title = w.media.threads.get(id as usize).and_then(|th| th.stories.last()).map_or_else(|| "In the press".to_string(), |&s| c.headline(&w.media.stories[s]));
            (short(&title, 70), None, "press")
        }
        ThreadKey::Post(root) => {
            let text = w.net.post(root).map_or_else(String::new, |p| c.post_text(p));
            (if text.is_empty() { "Online".to_string() } else { format!("Online: {}", short(&text, 50)) }, None, "post")
        }
        // A private matter that names neither a person nor a club is filed under the game itself, with nothing to link to.
        ThreadKey::Club(cl) if cl.is_none() => ("Personal".to_string(), None, "club"),
        ThreadKey::Club(cl) => (c.club_name(cl), Some(named(Ref::club(cl), c.club_name(cl))), "club"),
        ThreadKey::Decision(d) => (w.decisions.all.get(d).map_or_else(|| "Decision".to_string(), |d| d.kind.title().to_string()), None, "decision"),
    }
}

fn awaiting(w: &World, m: &Message) -> bool {
    match m.source {
        MsgSource::Decision { decision } => w.decisions.all.get(decision).is_some_and(|d| d.answer.is_none() && !d.resolved),
        MsgSource::Question { conference, question } => pending_question(w, m.to, conference, question).is_some_and(|id| w.decisions.all[id].answer.is_none()),
        _ => false,
    }
}

/// `me.inbox`: conversations, most recent first.
pub fn inbox(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    let limit = crate::contract::request::<crate::contract::LimitReq>(args.clone())?.limit.map_or(120, |n| n.clamp(10, 400) as usize);
    let mut rows: Vec<Value> = Vec::new();
    let mut unread_total = 0usize;
    let mut action_total = 0usize;
    for t in w.inbox.threads_of(me) {
        let msgs: Vec<&Message> = t.messages.iter().map(|&i| &w.inbox.messages[i as usize]).filter(|m| still_on_record(c, m)).collect();
        let unread = msgs.iter().filter(|m| !m.read).count();
        let needs = msgs.iter().any(|m| awaiting(w, m));
        let deadline = msgs.iter().rev().find_map(|m| {
            if !awaiting(w, m) { return None; }
            match m.source {
                MsgSource::Decision { decision } => w.decisions.all.get(decision).map(|d| d.deadline.0),
                MsgSource::Question { conference, question } => pending_question(w, m.to, conference, question).map(|id| w.decisions.all[id].deadline.0),
                _ => None,
            }
        });
        unread_total += unread;
        action_total += usize::from(needs);
        if rows.len() >= limit {
            continue;
        }
        let Some(last) = msgs.last() else { continue };
        let (title, with, kind) = thread_title(c, t);
        rows.push(json!({
            "id": t.id, "title": title, "with": with, "kind": kind, "last": t.last.0, "opened": t.opened.0,
            "count": msgs.len(), "unread": unread, "needs_action": needs, "deadline": deadline, "preview": short(&line(c, last), 110),
            "last_kind": kind_key(&last.source),
        }));
    }
    Ok(json!({"threads": rows, "unread": unread_total, "awaiting": action_total}))
}

/// `me.thread`: one conversation, oldest first.
pub fn thread(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let me = need(c)?;
    let id = crate::contract::request::<crate::contract::IdReq>(args.clone())?.id as usize;
    let t = c.w.inbox.threads.get(id).filter(|t| t.owner == me).ok_or_else(|| ApiError::NotFound("conversation".into()))?;
    let (title, with, kind) = thread_title(c, t);
    // The same words from the same person on the same day, with nothing to reply to, are said once.
    let mut said: std::collections::HashSet<(i32, u32, String)> = std::collections::HashSet::new();
    let msgs: Vec<Value> = t
        .messages
        .iter()
        .map(|&i| &c.w.inbox.messages[i as usize])
        .filter(|m| still_on_record(c, m))
        .filter(|m| !replies_json(c, m).is_empty() || m.replied.is_some() || said.insert((m.date.0, m.from.0, line(c, m))))
        .map(|m| message_json(c, m))
        .collect();
    Ok(json!({"id": t.id, "title": title, "with": with, "kind": kind, "opened": t.opened.0, "last": t.last.0, "messages": msgs}))
}

/// `me.thread_read`: mark a conversation read.
pub fn mark_read(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let id = crate::contract::request::<crate::contract::IdReq>(args.clone())?.id;
    if s.my_person().is_none() {
        return Err(ApiError::Unauthorized("You are observing the world; there is no inbox to read.".into()));
    }
    s.game.read_thread(id);
    Ok(crate::contract::wire(crate::contract::Done { ok: true }))
}

/// `me.reply`: send one of the replies a message offers.
pub fn reply(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let req: crate::contract::ReplyReq = crate::contract::request(args.clone())?;
    let (msg, key) = (req.message, req.key.as_str());
    let me = s.my_person().ok_or_else(|| ApiError::Unauthorized("You are observing the world; there is nobody to reply for.".into()))?;
    let w = s.w();
    let m = w.inbox.messages.get(msg as usize).copied().filter(|m| m.to == me).ok_or_else(|| ApiError::NotFound("message".into()))?;
    if m.replied.is_some() {
        return Err(ApiError::State("You have already replied to that.".into()));
    }
    let opts = pw_sim::inbox::options(w, msg);
    let idx = opts.iter().position(|&r| reply_key(r) == key).ok_or_else(|| ApiError::Bad("That reply is not available for this message.".into()))?;
    let label = {
        let c = Ctx::new(s);
        reply_label(&c, &m, opts[idx])
    };
    if !s.game.reply(msg, idx) {
        return Err(ApiError::State("That reply could not be sent.".into()));
    }
    s.revision += 1;
    let applies = if matches!(opts[idx], Reply::KeepQuiet | Reply::Ignore | Reply::Answer(_)) { "now" } else { "next day" };
    Ok(crate::contract::wire(crate::contract::ActDone { ok: true, text: label, applies: applies.into() }))
}
