//! Attention, virality, folklore (locked design 6.16-6.40).
//!
//! Football reputation, fame, attention and commercial appeal are different things: a player can be famous and mediocre, or excellent
//! and unnoticed. Attention arrives in waves with different causes, different reach beyond football and different half-lives; some of
//! it turns into nicknames that spread, mutate and die, and some into folklore that is retold with embellishment and corrected by
//! those who know better.

use pw_core::{Date, EventId, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::socialnet::MomentKind;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Cause {
    /// What he did on the pitch.
    Football,
    /// Who he is: a quote, a manner, a story about him.
    Personality,
    /// How he looks; the response depends on who is looking.
    Aesthetic,
    /// A row, a red card, a request.
    Controversy,
    /// Something that moves people: an injury, a comeback, a farewell.
    Emotional,
    /// Turned into a joke.
    Meme,
}

impl Cause {
    /// Days for a wave of this kind to lose half its force: controversy burns out fast, looks linger.
    pub const fn half_life(self) -> f32 {
        match self {
            Cause::Football => 14.0,
            Cause::Personality => 10.0,
            Cause::Aesthetic => 25.0,
            Cause::Controversy => 5.0,
            Cause::Emotional => 8.0,
            Cause::Meme => 3.0,
        }
    }

    /// How far beyond football audiences it carries (locked design 6.22): a goal stays in football, a look or a joke does not.
    pub const fn reach(self) -> f32 {
        match self {
            Cause::Football => 0.15,
            Cause::Personality => 0.45,
            Cause::Aesthetic => 0.70,
            Cause::Controversy => 0.50,
            Cause::Emotional => 0.40,
            Cause::Meme => 0.60,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Cause::Football => "football",
            Cause::Personality => "personality",
            Cause::Aesthetic => "looks",
            Cause::Controversy => "controversy",
            Cause::Emotional => "emotion",
            Cause::Meme => "a joke",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Wave {
    pub cause: Cause,
    pub start: Date,
    /// 0..1 at the start.
    pub peak: f32,
}

impl Wave {
    /// How strong it is on `today`.
    pub fn level(&self, today: Date) -> f32 {
        let days = self.start.days_until(today).max(0) as f32;
        self.peak * 0.5f32.powf(days / self.cause.half_life())
    }
}

/// The attention on one person.
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Attention {
    pub waves: SmallVec<[Wave; 3]>,
    /// Share of his following that comes from outside football, 0..1 (locked design 6.23: who follows matters, not only how many).
    pub general_share: f32,
    pub last_spark: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum NickKind {
    /// A familiar form of the name.
    Diminutive,
    /// From what he is like as a player (an attribute).
    Trait(u8),
    /// From a moment that people remember.
    Moment(MomentKind),
    /// A jibe that stuck.
    Mocking,
    /// Where he is from.
    Origin,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum NickState {
    Rising,
    Established,
    Fading,
    Dead,
}

/// A nickname: born from something, spread by use, changed in the retelling, and left behind when nobody cares (locked design 6.34).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Nickname {
    pub person: PersonId,
    pub kind: NickKind,
    pub born: Date,
    /// Accounts using it.
    pub users: u16,
    pub last_used: Date,
    pub state: NickState,
    /// Times it has mutated.
    pub variants: u8,
    /// A stable seed for how it is worded.
    pub key: u32,
}

/// A moment that has become a story people tell (locked design 6.29-6.33): retold more often than it is checked, so it grows.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Myth {
    pub about: PersonId,
    pub moment: MomentKind,
    pub event: EventId,
    pub born: Date,
    pub retellings: u16,
    /// How far the tale has drifted from what happened, 0..100.
    pub embellishment: u8,
    /// Times someone who knew better set it straight.
    pub corrected: u16,
    pub last: Date,
}
