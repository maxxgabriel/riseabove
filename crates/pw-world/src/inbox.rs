//! Inbox and conversation threads (S in the media brief; 19 in the final
//! brief).
//!
//! An inbox holds only communications that exist elsewhere in the world: a
//! decision someone must make, a person who told them something (a
//! grapevine telling), a meeting that took place, a story written about
//! them, a post that replied to or called them out, a private event aimed at
//! them (a board warning, a board query). The message stores the reference,
//! never text; narration renders it from the source.
//!
//! Messages are grouped into threads by counterpart and subject, so a
//! conversation that runs over weeks reads as one. Replies are not text
//! either: each reply becomes an intent (or a decision answer) processed by
//! the same systems as an AI's choice, and the counterpart's answer, if any,
//! arrives only when their own mind produces one — nothing is fabricated.
//!
//! Inboxes are kept for people a human controls (and anyone who was
//! controlled, until trimmed); AI minds read the same sources directly.

use pw_core::{ClubId, Date, DecisionId, EventId, PersonId, StoryId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MsgSource {
    /// A decision raised for this person.
    Decision { decision: DecisionId },
    /// Someone told them something (an index into the grapevine's items and
    /// the teller).
    Tell { info: u32, from: PersonId },
    /// A meeting they took part in (event carries the outcome).
    Meeting { event: EventId },
    /// A story about them.
    Story { story: StoryId },
    /// A post that replied to, quoted or called them out.
    Mention { post: u32 },
    /// A private event addressed to them.
    Private { event: EventId },
    /// A press conference question waiting for them.
    Question { conference: u32, question: u8 },
}

/// What a thread is about.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ThreadKey {
    /// A conversation with one person.
    With(PersonId),
    /// A running press story (the media thread id).
    Press(u32),
    /// Social replies around one of their posts.
    Post(u32),
    /// Club business (board, directors).
    Club(ClubId),
    /// Everything else, one per decision.
    Decision(DecisionId),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: u32,
    pub to: PersonId,
    /// The counterpart (the teller, the other side of a meeting, the
    /// journalist, the post's author's person if any).
    pub from: PersonId,
    pub date: Date,
    pub source: MsgSource,
    pub thread: u32,
    pub read: bool,
    /// What they replied (the intent or answer it produced), if any.
    pub replied: Option<ReplyRecord>,
    /// How important, 0–100 (sorting only).
    pub weight: u8,
}

/// A reply someone can make. Each maps onto a real intent or answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Reply {
    /// Pick an option of a decision.
    Answer(u8),
    /// Thank the teller (warmth toward them).
    Thank,
    /// Ask to meet the counterpart about it.
    AskToMeet { tone: crate::interaction::Tone },
    /// Keep it to yourself (explicitly: nothing happens, it is recorded).
    KeepQuiet,
    /// Pass it on to someone else.
    PassOn { to: PersonId },
    /// Answer a story on the record.
    Respond { stance: crate::media::Stance },
    /// Reply to a post.
    PostReply { concept: crate::socialnet::Concept },
    /// Nothing.
    Ignore,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ReplyRecord {
    pub reply: Reply,
    pub date: Date,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    pub id: u32,
    pub owner: PersonId,
    pub key: ThreadKey,
    pub messages: SmallVec<[u32; 4]>,
    pub opened: Date,
    pub last: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Inbox {
    pub messages: Vec<Message>,
    pub threads: Vec<Thread>,
    pub by_person: FxHashMap<PersonId, SmallVec<[u32; 16]>>,
    /// (owner, key) → thread id.
    pub thread_index: FxHashMap<(PersonId, ThreadKey), u32>,
}

impl Inbox {
    pub fn thread(&mut self, owner: PersonId, key: ThreadKey, date: Date) -> u32 {
        if let Some(&t) = self.thread_index.get(&(owner, key)) {
            return t;
        }
        let id = self.threads.len() as u32;
        self.threads.push(Thread { id, owner, key, messages: SmallVec::new(), opened: date, last: date });
        self.thread_index.insert((owner, key), id);
        self.by_person.entry(owner).or_default().push(id);
        id
    }

    /// Deliver a message (at most once per source per person).
    pub fn deliver(&mut self, to: PersonId, from: PersonId, date: Date, source: MsgSource, key: ThreadKey, weight: u8) -> Option<u32> {
        let t = self.thread(to, key, date);
        if self.threads[t as usize].messages.iter().any(|&m| self.messages[m as usize].source == source) {
            return None;
        }
        let id = self.messages.len() as u32;
        self.messages.push(Message { id, to, from, date, source, thread: t, read: false, replied: None, weight });
        let th = &mut self.threads[t as usize];
        th.messages.push(id);
        th.last = date;
        Some(id)
    }

    /// A person's threads, most recent first.
    pub fn threads_of(&self, p: PersonId) -> Vec<&Thread> {
        let mut v: Vec<&Thread> = self.by_person.get(&p).map_or_else(Vec::new, |ids| ids.iter().map(|&t| &self.threads[t as usize]).collect());
        v.sort_by(|a, b| b.last.cmp(&a.last).then(b.id.cmp(&a.id)));
        v
    }

    pub fn unread(&self, p: PersonId) -> usize {
        self.by_person.get(&p).map_or(0, |ids| ids.iter().flat_map(|&t| self.threads[t as usize].messages.iter()).filter(|&&m| !self.messages[m as usize].read).count())
    }
}
