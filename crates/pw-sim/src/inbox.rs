//! Filling inboxes from real communications, and turning replies into
//! intents. See `pw_world::inbox`.

use pw_core::{DecisionId, PersonId};
use pw_world::event::{EventKind, Visibility};
use pw_world::inbox::{MsgSource, Reply, ReplyRecord, ThreadKey};
use pw_world::info::InfoKind;
use pw_world::interaction::{Tone, Topic};
use pw_world::socialnet::{Concept, NO_POST};
use pw_world::{Intent, MindKind, World};
use smallvec::SmallVec;

/// Daily, after everything else has happened: deliver today's
/// communications to the people humans control.
pub fn daily(w: &mut World) {
    let today = w.date;
    let humans: Vec<PersonId> = w.people.iter_enumerated().filter(|(_, p)| p.mind == MindKind::External).map(|(id, _)| id).collect();
    if humans.is_empty() {
        return;
    }
    for &me in &humans {
        // Decisions raised today.
        let ds: Vec<(DecisionId, PersonId)> = w.decisions.pending_for(me).filter(|(_, d)| d.created == today).map(|(id, d)| (id, counterpart(w, &d.kind))).collect();
        for (id, from) in ds {
            let key = if from.is_some() { ThreadKey::With(from) } else { ThreadKey::Decision(id) };
            w.inbox.deliver(me, from, today, MsgSource::Decision { decision: id }, key, 80);
        }
    }
    // Tellings today.
    let tells: Vec<(u32, PersonId, PersonId)> = w.grapevine.tells.iter().rev().take_while(|t| t.date == today).filter(|t| humans.contains(&t.to)).map(|t| (t.info, t.from, t.to)).collect();
    for (info, from, to) in tells {
        w.inbox.deliver(to, from, today, MsgSource::Tell { info, from }, ThreadKey::With(from), 60);
    }
    // Events: meetings, private notes, stories about them.
    let evs: Vec<(pw_core::EventId, EventKind, Visibility)> = w.events.since(today).iter().map(|e| (e.id, e.kind.clone(), e.vis)).collect();
    for (id, kind, vis) in evs {
        match kind {
            EventKind::Meeting { from, with, .. } => {
                for (me, other) in [(from, with), (with, from)] {
                    if humans.contains(&me) {
                        w.inbox.deliver(me, other, today, MsgSource::Meeting { event: id }, ThreadKey::With(other), 50);
                    }
                }
            }
            EventKind::Published { story } => {
                let s = &w.media.stories[story];
                if humans.contains(&s.person) {
                    let (to, from, thread) = (s.person, s.journalist, s.thread);
                    let key = if thread != u32::MAX { ThreadKey::Press(thread) } else { ThreadKey::With(from) };
                    w.inbox.deliver(to, from, today, MsgSource::Story { story }, key, 55);
                }
            }
            _ => match vis {
                Visibility::Person(p) if humans.contains(&p) => {
                    let from = kind.people().into_iter().find(|&x| x != p).unwrap_or(PersonId::NONE);
                    let club = kind.clubs().first().copied().unwrap_or(pw_core::ClubId::NONE);
                    let key = if from.is_some() { ThreadKey::With(from) } else { ThreadKey::Club(club) };
                    w.inbox.deliver(p, from, today, MsgSource::Private { event: id }, key, 65);
                }
                _ => {}
            },
        }
    }
    // Posts that reply to, quote or call out a human's posts.
    let mine: Vec<(PersonId, u32)> = humans.iter().filter_map(|&h| w.net.account_of(h).map(|a| (h, a))).collect();
    if !mine.is_empty() {
        let mentions: Vec<(PersonId, u32, u32, PersonId)> = w
            .net
            .posts
            .iter()
            .rev()
            .take_while(|p| p.date == today)
            .filter_map(|p| {
                let parent = if p.reply_to != NO_POST { p.reply_to } else { p.quote_of };
                let target = w.net.post(parent).map(|x| x.author);
                let about_me = mine.iter().find(|(h, a)| Some(*a) == target || (p.about == *h && p.concept == Concept::CallOut));
                about_me.map(|&(h, _)| (h, p.id, if parent != NO_POST { parent } else { p.id }, w.net.accounts[p.author as usize].person))
            })
            .collect();
        for (h, post, root, from) in mentions {
            w.inbox.deliver(h, from, today, MsgSource::Mention { post }, ThreadKey::Post(root), 30);
        }
    }
    // Questions waiting at today's press conferences.
    let qs: Vec<(PersonId, u32, u8, PersonId)> = w
        .pressroom
        .conferences
        .iter()
        .rev()
        .take_while(|c| c.date == today)
        .filter(|c| humans.contains(&c.speaker))
        .flat_map(|c| c.questions.iter().enumerate().filter(|(i, _)| c.answers[*i].is_none()).map(move |(i, q)| (c.speaker, c.id, i as u8, q.journalist)))
        .collect();
    for (me, conference, question, j) in qs {
        w.inbox.deliver(me, j, today, MsgSource::Question { conference, question }, ThreadKey::With(j), 70);
    }
}

/// The other person in a decision, if it has one.
fn counterpart(w: &World, k: &pw_world::decision::DecisionKind) -> PersonId {
    use pw_world::decision::DecisionKind as D;
    match *k {
        D::Meeting { meeting } => w.meetings.list.get(meeting).map_or(PersonId::NONE, |m| m.initiator),
        D::PressQuestion { conference, question } => w.pressroom.conferences.get(conference as usize).and_then(|c| c.questions.get(usize::from(question))).map_or(PersonId::NONE, |q| q.journalist),
        _ => PersonId::NONE,
    }
}

/// The replies that make sense for a message, from what it refers to.
pub fn options(w: &World, msg: u32) -> SmallVec<[Reply; 6]> {
    let Some(m) = w.inbox.messages.get(msg as usize) else { return SmallVec::new() };
    let mut v: SmallVec<[Reply; 6]> = SmallVec::new();
    match m.source {
        MsgSource::Decision { decision } => {
            let n = w.decisions.all.get(decision).map_or(0, |d| d.options.len());
            v.extend((0..n as u8).map(Reply::Answer));
        }
        MsgSource::Question { conference, question } => {
            // The question's decision carries the options.
            if let Some((id, _)) = w.decisions.pending_for(m.to).find(|(_, d)| matches!(d.kind, pw_world::decision::DecisionKind::PressQuestion { conference: c, question: q } if c == conference && q == question)) {
                let n = w.decisions.all[id].options.len();
                v.extend((0..n as u8).map(Reply::Answer));
            }
        }
        MsgSource::Tell { from, .. } => {
            v.push(Reply::Thank);
            v.push(Reply::AskToMeet { tone: Tone::Calm });
            v.push(Reply::AskToMeet { tone: Tone::Assertive });
            v.push(Reply::KeepQuiet);
            if let Some(pt) = w.lives.get(m.to).and_then(|l| l.household.partner) {
                if pt.person != from {
                    v.push(Reply::PassOn { to: pt.person });
                }
            }
        }
        MsgSource::Story { .. } => {
            for s in [pw_world::media::Stance::Deny, pw_world::media::Stance::Deflect, pw_world::media::Stance::Loyalty, pw_world::media::Stance::Complain] {
                v.push(Reply::Respond { stance: s });
            }
            v.push(Reply::Ignore);
        }
        MsgSource::Mention { .. } => {
            for c in [Concept::Agree, Concept::Disagree, Concept::Defend, Concept::Mock] {
                v.push(Reply::PostReply { concept: c });
            }
            v.push(Reply::Ignore);
        }
        MsgSource::Meeting { .. } | MsgSource::Private { .. } => {
            if m.from.is_some() {
                v.push(Reply::AskToMeet { tone: Tone::Calm });
                v.push(Reply::AskToMeet { tone: Tone::Humble });
            }
            v.push(Reply::Ignore);
        }
    }
    v
}

/// The topic a follow-up meeting would be about.
fn topic_for(w: &World, m: &pw_world::inbox::Message) -> Topic {
    match m.source {
        MsgSource::Tell { info, .. } => match w.grapevine.get(info).kind {
            InfoKind::Interest { .. } | InfoKind::Bid { .. } | InfoKind::ContractTalks { .. } | InfoKind::Exploring { .. } => {
                if w.agents.list.iter().any(|a| a.person == m.from) { Topic::AgentReview } else { Topic::NewContract }
            }
            InfoKind::Unhappy { .. } | InfoKind::DressingRoom { .. } => Topic::TeammateIssue,
            InfoKind::Discipline { .. } => Topic::Discipline,
            InfoKind::Incident { .. } => Topic::Apology,
            _ => Topic::Feedback,
        },
        MsgSource::Private { event } => match w.events.get(event).map(|e| e.kind.clone()) {
            Some(EventKind::BoardWarning { .. } | EventKind::BoardQuery { .. }) => Topic::Feedback,
            _ => Topic::Feedback,
        },
        _ => Topic::Feedback,
    }
}

/// Reply to a message. The reply is recorded and becomes an intent or an
/// answer; any response from the other side comes from their own mind,
/// later, through the same systems. Returns false if the reply is not valid.
pub fn reply(w: &mut World, who: PersonId, msg: u32, r: Reply) -> bool {
    let today = w.date;
    let Some(m) = w.inbox.messages.get(msg as usize).copied() else { return false };
    if m.to != who || m.replied.is_some() || !options(w, msg).contains(&r) {
        return false;
    }
    let ok = match (m.source, r) {
        (MsgSource::Decision { decision }, Reply::Answer(k)) => w.decisions.answer(decision, k),
        (MsgSource::Question { conference, question }, Reply::Answer(k)) => {
            let id = w.decisions.pending_for(who).find(|(_, d)| matches!(d.kind, pw_world::decision::DecisionKind::PressQuestion { conference: c, question: q } if c == conference && q == question)).map(|(id, _)| id);
            id.is_some_and(|id| w.decisions.answer(id, k))
        }
        (MsgSource::Tell { from, .. }, Reply::Thank) => {
            w.intents.submit(who, Intent::Thank { to: from }, today);
            true
        }
        (MsgSource::Tell { info, .. }, Reply::PassOn { to }) => {
            w.intents.submit(who, Intent::Tell { to, info }, today);
            true
        }
        (_, Reply::AskToMeet { tone }) => {
            let topic = topic_for(w, &m);
            w.intents.submit(who, Intent::RequestMeeting { with: m.from, topic, tone }, today);
            true
        }
        (MsgSource::Story { .. }, Reply::Respond { stance }) => {
            w.intents.submit(who, Intent::SpeakToPress { about: who, stance }, today);
            true
        }
        (MsgSource::Mention { post }, Reply::PostReply { concept }) => {
            let about = w.net.post(post).map_or(PersonId::NONE, |p| if p.about.is_some() { p.about } else { who });
            w.intents.submit(who, Intent::Post { about, concept, reply_to: post, quote_of: NO_POST }, today);
            true
        }
        (_, Reply::KeepQuiet | Reply::Ignore) => true,
        _ => false,
    };
    if ok {
        let mm = &mut w.inbox.messages[msg as usize];
        mm.replied = Some(ReplyRecord { reply: r, date: today });
        mm.read = true;
    }
    ok
}

/// Mark a thread read.
pub fn read(w: &mut World, who: PersonId, thread: u32) {
    let Some(t) = w.inbox.threads.get(thread as usize) else { return };
    if t.owner != who {
        return;
    }
    let ids: SmallVec<[u32; 4]> = t.messages.clone();
    for m in ids {
        w.inbox.messages[m as usize].read = true;
    }
}
