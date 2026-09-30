//! Conversations and contract talks as text. A meeting's lines are built from
//! the two people, the topic, the tones actually used and the outcomes the
//! simulation produced — never the other way round.

use pw_core::PersonId;
use pw_world::World;
use pw_world::interaction::{Meeting, MeetingState, Outcome, Tone, Topic};
use pw_world::negotiation::{Negotiation, TalkLine, Terms};

use crate::fmt::{person, wage};
use crate::pick;

fn say(w: &World, p: PersonId, viewer: PersonId) -> String {
    if p == viewer { "You".into() } else { person(w, p) }
}

pub fn opener(topic: Topic, tone: Tone, key: u64) -> &'static str {
    match (topic, tone) {
        (Topic::PlayingTime, Tone::Aggressive) => pick(key, &["I should be playing. This is a joke.", "I'm not sitting on the bench any more."]),
        (Topic::PlayingTime, Tone::Humble) => pick(key, &["I'd like to understand what I need to do to get more minutes.", "I just want a chance to show what I can do."]),
        (Topic::PlayingTime, _) => pick(key, &["I want more game time.", "I need to be playing regularly."]),
        (Topic::Feedback, _) => pick(key, &["What do I need to work on?", "Where do you see me right now?"]),
        (Topic::Position, _) => "I think I could offer more in a different position.",
        (Topic::NewContract, Tone::Assertive) => "My contract doesn't reflect what I bring. We need to talk.",
        (Topic::NewContract, _) => "I'd like to talk about my contract.",
        (Topic::LoanRequest, _) => "I think a loan would do me good right now.",
        (Topic::WantAway, Tone::Aggressive) => "I want out. Sell me.",
        (Topic::WantAway, _) => "I think it's time for me to move on.",
        (Topic::Attitude, Tone::Aggressive) => "Your training has been a disgrace. Sort it out.",
        (Topic::Attitude, _) => pick(key, &["Your standards in training have dropped.", "I'm not seeing what I need from you in training."]),
        (Topic::Discipline, _) => "What happened can't happen again.",
        (Topic::Dropped, _) => "I want to explain why you've not been playing.",
        (Topic::Encouragement, _) => pick(key, &["I've been impressed with you lately.", "Keep doing what you're doing."]),
        (Topic::PromiseFollowUp, Tone::Aggressive) => "You gave me your word. What happened?",
        (Topic::PromiseFollowUp, _) => "I wanted to follow up on what we agreed.",
        (Topic::TeammateIssue, _) => "I've got a problem with one of the lads.",
        (Topic::Apology, _) => "I was out of order the other day. I'm sorry.",
        (Topic::AgentReview, _) => "Let's go over where things stand.",
    }
}

pub fn reply(tone: Tone, key: u64) -> &'static str {
    match tone {
        Tone::Calm => pick(key, &["I hear you.", "Let's talk about it properly.", "Okay. Go on."]),
        Tone::Assertive => pick(key, &["I'll be straight with you.", "Here's how I see it."]),
        Tone::Aggressive => pick(key, &["Don't come in here and talk to me like that.", "You've got a nerve."]),
        Tone::Humble => pick(key, &["You're right. I'll listen.", "Fair enough. I want to get this right."]),
        Tone::Joking => pick(key, &["Come on, it's not that bad.", "Easy — sit down, have a coffee."]),
    }
}

pub fn outcome(w: &World, o: &Outcome, m: &Meeting, viewer: PersonId) -> String {
    match *o {
        Outcome::PromiseMade { promise } => {
            let p = w.social.promise(promise);
            let from = p.map_or(m.with, |p| p.from);
            let what = p.map_or("something".into(), |p| p.kind.text());
            let due = p.map_or(String::new(), |p| format!(" by {}", p.due));
            format!("{} promised {what}{due}.", say(w, from, viewer))
        }
        Outcome::Refused => "The request was turned down.".into(),
        Outcome::Deferred => "\"Show me in training first.\"".into(),
        Outcome::Praised => "Words of praise were shared.".into(),
        Outcome::Warned => "It ended with a warning.".into(),
        Outcome::Fined { weeks } => if weeks == 1 { "A fine of a week's wages was imposed.".to_string() } else { format!("A fine of {weeks} weeks' wages was imposed.") },
        Outcome::Dropped => "Left out of the next squad.".into(),
        Outcome::Listed => "Placed on the transfer list.".into(),
        Outcome::StatusChanged => "Squad status changed.".into(),
        Outcome::FellOut => "Voices were raised. It ended badly.".into(),
        Outcome::Reconciled => "The air was cleared.".into(),
        Outcome::TalksOpened => "Contract talks will begin.".into(),
        Outcome::Leaked => "Details later reached the press.".into(),
    }
}

/// A meeting as a short transcript plus outcomes.
pub fn meeting(w: &World, m: &Meeting, viewer: PersonId) -> Vec<String> {
    let key = u64::from(m.id.0);
    let mut out = vec![format!("{} → {}: {} ({})", say(w, m.initiator, viewer), person(w, m.with), m.topic.label(), m.date)];
    out.push(format!("  {}: \"{}\" [{}]", say(w, m.initiator, viewer), opener(m.topic, m.opening, key), m.opening.label()));
    match (m.state, m.response) {
        (MeetingState::Pending, _) => out.push("  (awaiting a response)".into()),
        (MeetingState::Lapsed, _) => out.push("  (the meeting never happened)".into()),
        (MeetingState::Held, Some(r)) => {
            out.push(format!("  {}: \"{}\" [{}]", say(w, m.with, viewer), reply(r, key ^ 0x99), r.label()));
            for o in &m.outcomes {
                out.push(format!("  → {}", outcome(w, o, m, viewer)));
            }
        }
        _ => {}
    }
    if !m.causes.is_empty() {
        let why: Vec<String> = m.causes.iter().map(|c| crate::events::cause(w, c, viewer)).collect();
        out.push(format!("  why: {}", why.join("; ")));
    }
    out
}

fn years(n: u32) -> String {
    if n == 1 { "1 year".to_string() } else { format!("{n} years") }
}

pub fn terms(t: &Terms) -> String {
    let mut s = format!("{} for {}", wage(t.wage), years(u32::from(t.years)));
    if t.signing_fee > 0 {
        s += &format!(", signing fee {}", crate::fmt::money(t.signing_fee));
    }
    if let Some(st) = t.status {
        s += &format!(", status {}", st.label());
    }
    if t.release_clause > 0 {
        s += &format!(", release clause {}", crate::fmt::money(t.release_clause));
    }
    if t.loyalty_bonus > 0 {
        s += &format!(", loyalty bonus {}", crate::fmt::money(t.loyalty_bonus));
    }
    if t.options.club_years > 0 {
        s += &format!(", club option +{}", years(u32::from(t.options.club_years)));
    }
    if t.options.player_years > 0 {
        s += &format!(", player option +{}", years(u32::from(t.options.player_years)));
    }
    if let Some((trigger, extra)) = t.options.auto {
        s += &format!(", +{} on {}", years(u32::from(extra)), trigger.label());
    }
    s
}

pub fn talk_line(l: &TalkLine) -> String {
    match l {
        TalkLine::ClubOffer(t) => format!("Club offered {}", terms(t)),
        TalkLine::PlayerCounter(t) => format!("Player asked for {}", terms(t)),
        TalkLine::ClubImproved(t) => format!("Club improved to {}", terms(t)),
        TalkLine::ClubHeldFirm => "Club held firm".into(),
        TalkLine::PlayerAccepted => "Player accepted".into(),
        TalkLine::PlayerRejected => "Player rejected the offer".into(),
        TalkLine::ClubWalkedAway => "Club walked away".into(),
        TalkLine::AgentPressed => "Agent pressed for more".into(),
    }
}

pub fn negotiation(w: &World, n: &Negotiation) -> Vec<String> {
    let mut v = vec![format!("{} with {} — round {} of {}", n.kind.label(), crate::fmt::club(w, n.club), n.round, n.max_rounds)];
    if n.agent.is_some() {
        v.push(format!("  Agent: {}", person(w, w.agents.list[n.agent].person)));
    }
    for (d, l) in &n.log {
        v.push(format!("  {d}  {}", talk_line(l)));
    }
    v
}
