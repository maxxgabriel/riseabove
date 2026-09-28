//! Conversations between people (09 §3–4). A meeting is a real interaction:
//! someone decided to have it for reasons (causes), the other side responds,
//! and one resolver — the same for every person — decides what it leads to.
//! Nobody's choice has a fixed consequence; the outcome comes from the two
//! people, their history and their circumstances.

use pw_core::{ClubId, Date, DecisionId, EventId, MeetingId, PersonId, PlayerId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::event::Causes;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Topic {
    /// Player wants more minutes.
    PlayingTime,
    /// Player asks what they must improve.
    Feedback,
    /// Player wants to play elsewhere on the pitch.
    Position,
    /// Player wants a new contract.
    NewContract,
    /// Player asks to go out on loan.
    LoanRequest,
    /// Player wants to leave.
    WantAway,
    /// Manager raises poor training or attitude.
    Attitude,
    /// Manager raises a disciplinary matter.
    Discipline,
    /// Manager explains a decision to drop or bench.
    Dropped,
    /// Manager praises and encourages.
    Encouragement,
    /// Either side follows up on an earlier promise.
    PromiseFollowUp,
    /// Player complains about a teammate.
    TeammateIssue,
    /// Clearing the air after a fall-out.
    Apology,
    /// Agent and client review the market.
    AgentReview,
}

impl Topic {
    pub const fn label(self) -> &'static str {
        match self {
            Topic::PlayingTime => "playing time",
            Topic::Feedback => "what to improve",
            Topic::Position => "playing position",
            Topic::NewContract => "a new contract",
            Topic::LoanRequest => "a loan move",
            Topic::WantAway => "leaving the club",
            Topic::Attitude => "attitude in training",
            Topic::Discipline => "discipline",
            Topic::Dropped => "being left out",
            Topic::Encouragement => "recent progress",
            Topic::PromiseFollowUp => "an earlier promise",
            Topic::TeammateIssue => "a problem with a teammate",
            Topic::Apology => "clearing the air",
            Topic::AgentReview => "the market",
        }
    }

    /// Topics a player may raise with their manager.
    pub const PLAYER_RAISES: [Topic; 9] =
        [Topic::PlayingTime, Topic::Feedback, Topic::Position, Topic::NewContract, Topic::LoanRequest, Topic::WantAway, Topic::PromiseFollowUp, Topic::TeammateIssue, Topic::Apology];
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Tone {
    Calm,
    Assertive,
    Aggressive,
    Humble,
    Joking,
}

impl Tone {
    pub const ALL: [Tone; 5] = [Tone::Calm, Tone::Assertive, Tone::Aggressive, Tone::Humble, Tone::Joking];

    pub const fn label(self) -> &'static str {
        match self {
            Tone::Calm => "calm",
            Tone::Assertive => "assertive",
            Tone::Aggressive => "aggressive",
            Tone::Humble => "humble",
            Tone::Joking => "light-hearted",
        }
    }
}

/// What a meeting led to. Several can happen at once.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Outcome {
    PromiseMade {
        promise: u32,
    },
    Refused,
    /// "Show me in training first."
    Deferred,
    Praised,
    Warned,
    Fined {
        weeks: u8,
    },
    /// Left out of the next squad.
    Dropped,
    /// Transfer-listed at their request or as punishment.
    Listed,
    /// Squad status changed.
    StatusChanged,
    /// The two people ended on worse terms.
    FellOut,
    /// Cleared the air.
    Reconciled,
    /// Agreed to open contract talks.
    TalksOpened,
    /// The conversation leaked to the press.
    Leaked,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MeetingState {
    /// Waiting for the responder (an external mind answers through a decision).
    Pending,
    Held,
    /// Did not take place (one side left the club, retired…).
    Lapsed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meeting {
    pub id: MeetingId,
    pub requested: Date,
    /// When it takes place (the responder's deadline).
    pub date: Date,
    pub initiator: PersonId,
    pub with: PersonId,
    /// The player the conversation is about (usually one of the two).
    pub player: PlayerId,
    pub club: ClubId,
    pub topic: Topic,
    /// Tone the initiator opened with.
    pub opening: Tone,
    /// Tone the responder answered with.
    pub response: Option<Tone>,
    pub state: MeetingState,
    pub outcomes: SmallVec<[Outcome; 3]>,
    /// Why this meeting was called.
    pub causes: Causes,
    /// Decision raised for an external responder, if any.
    pub decision: DecisionId,
    pub event: EventId,
    /// Whether the initiator came away satisfied, -100..=100.
    pub satisfaction: i8,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Meetings {
    pub list: pw_core::IdVec<MeetingId, Meeting>,
}

impl Meetings {
    pub fn pending(&self) -> impl Iterator<Item = (MeetingId, &Meeting)> {
        self.list.iter_enumerated().filter(|(_, m)| m.state == MeetingState::Pending)
    }

    /// Most recent meeting between two people on a topic.
    pub fn last_between(&self, a: PersonId, b: PersonId, topic: Topic) -> Option<&Meeting> {
        self.list.iter().rev().find(|m| m.topic == topic && ((m.initiator == a && m.with == b) || (m.initiator == b && m.with == a)))
    }

    /// Days since `a` last met `b` about anything, or `None`.
    pub fn days_since_any(&self, a: PersonId, b: PersonId, today: Date) -> Option<i32> {
        self.list.iter().rev().find(|m| (m.initiator == a && m.with == b) || (m.initiator == b && m.with == a)).map(|m| m.date.days_until(today))
    }

    pub fn involving(&self, p: PersonId) -> impl Iterator<Item = (MeetingId, &Meeting)> {
        self.list.iter_enumerated().filter(move |(_, m)| m.initiator == p || m.with == p)
    }

    pub fn has_pending(&self, a: PersonId, b: PersonId) -> bool {
        self.list.iter().rev().take(4096).any(|m| m.state == MeetingState::Pending && ((m.initiator == a && m.with == b) || (m.initiator == b && m.with == a)))
    }
}
