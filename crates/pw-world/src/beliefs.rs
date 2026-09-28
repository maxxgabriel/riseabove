//! What individual people believe (S15). Truth lives in the world; a person
//! holds beliefs, each with a channel it arrived through, a date and a
//! confidence. The client shows the controlled person nothing else (plus
//! public information), so uncertainty and misinformation are part of play.
//!
//! Clubs as institutions keep their evidence in `knowledge`; this store is for
//! people — players hearing about interest, a coach's opinion as it was told,
//! what a partner thinks the move would mean.

use pw_core::{ClubId, Date, EventId, Money, PersonId, StoryId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// How a belief reached its holder. Each channel has its own fidelity.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Channel {
    /// Saw it themselves.
    Witnessed,
    /// Someone told them directly.
    Told(PersonId),
    /// Their agent passed it on (filtered by the agent's honesty and interests).
    Agent(PersonId),
    /// Official club communication.
    Club(ClubId),
    /// Read it in the press.
    Media(StoryId),
    /// Common knowledge.
    Public,
    /// Worked it out from what they could see.
    Inferred,
}

impl Channel {
    /// Rough reliability of the channel on its own, 0–100.
    pub fn base_fidelity(&self) -> u8 {
        match self {
            Channel::Witnessed => 95,
            Channel::Club(_) => 85,
            Channel::Told(_) => 70,
            Channel::Agent(_) => 65,
            Channel::Public => 80,
            Channel::Media(_) => 45,
            Channel::Inferred => 50,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum BeliefKind {
    /// A club is interested in this person.
    ClubInterested { club: ClubId },
    /// A bid of roughly this size was made.
    BidMade { club: ClubId, fee: Money },
    /// How the manager rates them, as communicated (1–5).
    ManagerRating { manager: PersonId, stars: u8 },
    /// What their coaches' assessments suggest about their ability (CA scale band)
    /// and ceiling (a hint, 1–5).
    Assessment { ca_lo: u8, ca_hi: u8, ceiling: u8 },
    /// How they stand in the manager's plans for the next match (0–100 chance to start).
    SelectionOutlook { start_pct: u8 },
    /// A published rumour about them.
    Rumour { story: StoryId },
    /// Fans of a club like/dislike them (-100..=100).
    FanMood { club: ClubId, score: i8 },
}

impl BeliefKind {
    /// Two beliefs of the same slot replace one another.
    fn slot(&self) -> (u8, u32) {
        match *self {
            BeliefKind::ClubInterested { club } => (0, club.0),
            BeliefKind::BidMade { club, .. } => (1, club.0),
            BeliefKind::ManagerRating { manager, .. } => (2, manager.0),
            BeliefKind::Assessment { .. } => (3, 0),
            BeliefKind::SelectionOutlook { .. } => (4, 0),
            BeliefKind::Rumour { story } => (5, story.0),
            BeliefKind::FanMood { club, .. } => (6, club.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Belief {
    /// Whom the belief is about.
    pub about: PersonId,
    pub kind: BeliefKind,
    pub channel: Channel,
    /// 0–100.
    pub confidence: u8,
    pub date: Date,
    /// The event it ultimately stems from (`NONE` for inferences).
    pub origin: EventId,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Beliefs {
    by_holder: FxHashMap<PersonId, SmallVec<[Belief; 4]>>,
}

impl Beliefs {
    /// Learn (or refresh) a belief. A newer belief in the same slot replaces an
    /// older one unless the older one came through a clearly better channel
    /// and is still fresh.
    pub fn learn(&mut self, holder: PersonId, b: Belief) {
        if holder.is_none() {
            return;
        }
        let list = self.by_holder.entry(holder).or_default();
        if let Some(old) = list.iter_mut().find(|x| x.about == b.about && x.kind.slot() == b.kind.slot()) {
            let fresher = b.date.days_until(old.date) <= -30;
            if fresher || b.confidence + 10 >= old.confidence {
                *old = b;
            }
            return;
        }
        list.push(b);
    }

    pub fn of(&self, holder: PersonId) -> impl Iterator<Item = &Belief> {
        self.by_holder.get(&holder).into_iter().flatten()
    }

    pub fn about(&self, holder: PersonId, about: PersonId) -> impl Iterator<Item = &Belief> {
        self.of(holder).filter(move |b| b.about == about)
    }

    pub fn holds(&self, holder: PersonId, pred: impl Fn(&Belief) -> bool) -> bool {
        self.of(holder).any(pred)
    }

    /// Beliefs fade: interest heard about a year ago is not interest today.
    pub fn forget(&mut self, before: Date) {
        for l in self.by_holder.values_mut() {
            l.retain(|b| b.date >= before);
        }
        self.by_holder.retain(|_, l| !l.is_empty());
    }

    pub fn len(&self) -> usize {
        self.by_holder.values().map(|l| l.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.by_holder.is_empty()
    }
}
