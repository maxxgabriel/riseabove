//! Rendering inbox messages and replies from the sources they refer to.

use pw_core::PersonId;
use pw_world::World;
use pw_world::inbox::{Message, MsgSource, Reply, Thread, ThreadKey};

use crate::fmt::{club, person};

/// A thread's title.
pub fn thread_title(w: &World, t: &Thread) -> String {
    match t.key {
        ThreadKey::With(p) => person(w, p),
        ThreadKey::Press(th) => w.media.threads.get(th as usize).and_then(|x| x.stories.first()).map_or_else(|| "Press".to_string(), |&s| crate::press::headline(w, &w.media.stories[s])),
        ThreadKey::Post(_) => "Replies to your post".to_string(),
        ThreadKey::Club(c) => {
            if c.is_some() {
                club(w, c)
            } else {
                "Club".to_string()
            }
        }
        ThreadKey::Decision(d) => w.decisions.all.get(d).map_or_else(|| "Decision".to_string(), |d| crate::choices::title(w, d)),
    }
}

/// One message, in the recipient's view.
pub fn message(w: &World, m: &Message) -> String {
    let me = m.to;
    match m.source {
        MsgSource::Decision { decision } => w.decisions.all.get(decision).map_or_else(String::new, |d| crate::choices::title(w, d)),
        MsgSource::Tell { info, from } => {
            let version = crate::grapevine::version(w, info, me).unwrap_or_else(|| crate::grapevine::what(w, info));
            format!("{} told you: {version}", person(w, from))
        }
        MsgSource::Meeting { event } => w.events.get(event).and_then(|e| crate::events::line(w, e, me)).unwrap_or_default(),
        MsgSource::Story { story } => {
            let s = &w.media.stories[story];
            format!("{} ({}): {}", crate::press::outlet_name(w, s), person(w, s.journalist), crate::press::headline(w, s))
        }
        MsgSource::Mention { post } => w.net.post(post).map_or_else(String::new, |p| {
            let a = &w.net.accounts[p.author as usize];
            format!("@{}: {}", a.handle, crate::social::post(w, p))
        }),
        MsgSource::Private { event } => w.events.get(event).and_then(|e| crate::events::line(w, e, me)).unwrap_or_default(),
        MsgSource::Question { conference, question } => crate::press::question(w, conference, question),
    }
}

/// A reply option, as a label.
pub fn reply(w: &World, m: &Message, r: Reply) -> String {
    match r {
        Reply::Answer(k) => {
            let d = match m.source {
                MsgSource::Decision { decision } => w.decisions.all.get(decision),
                MsgSource::Question { conference, question } => w
                    .decisions
                    .pending_for(m.to)
                    .find(|(_, d)| matches!(d.kind, pw_world::decision::DecisionKind::PressQuestion { conference: c, question: q } if c == conference && q == question))
                    .map(|(_, d)| d),
                _ => None,
            };
            d.and_then(|d| d.options.get(usize::from(k))).map_or_else(|| format!("option {k}"), crate::choices::option)
        }
        Reply::Thank => "Thank them".into(),
        Reply::AskToMeet { tone } => format!("Ask to meet ({})", format!("{tone:?}").to_lowercase()),
        Reply::KeepQuiet => "Keep it to yourself".into(),
        Reply::PassOn { to } => format!("Tell {}", person(w, to)),
        Reply::Respond { stance } => format!("Respond on the record: {}", crate::press::stance_label(stance)),
        Reply::PostReply { concept } => format!("Reply: {}", format!("{concept:?}").to_lowercase()),
        Reply::Ignore => "Ignore".into(),
    }
}

/// What someone replied, for the thread history.
pub fn replied(w: &World, m: &Message, viewer: PersonId) -> Option<String> {
    let r = m.replied?;
    let who = if viewer == m.to { "You".to_string() } else { person(w, m.to) };
    Some(format!("{} {} — {}", r.date, who, reply(w, m, r.reply)))
}
