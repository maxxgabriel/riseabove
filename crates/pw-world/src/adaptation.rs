//! Settling in after a move (locked design 4.12-4.21). Signed is not available, is not acclimatised, is not tactically integrated,
//! is not performing at the expected level: each channel below has its own distance, its own timeline and its own supports, and
//! two players making the same move do not settle alike.

use pw_core::{ClubId, Date, PlayerId};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::dossier::Confidence;

pub const N_CHANNELS: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Channel {
    /// Climate, humidity, altitude, daylight, the physicality of the league.
    Environment,
    /// Timezone, sleep, meals, training times, travel.
    Routine,
    /// The league's pace and style, his role.
    Football,
    /// This manager's system: pressing cues, rotations, set pieces.
    Tactical,
    /// Language, culture, housing, family, teammates.
    Social,
    /// Confidence, homesickness, pressure, the supporters and the press.
    Mental,
}

impl Channel {
    pub const ALL: [Channel; N_CHANNELS] = [Channel::Environment, Channel::Routine, Channel::Football, Channel::Tactical, Channel::Social, Channel::Mental];

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            Channel::Environment => "climate and surroundings",
            Channel::Routine => "body clock and routine",
            Channel::Football => "the football itself",
            Channel::Tactical => "the manager's system",
            Channel::Social => "language, culture and life outside the club",
            Channel::Mental => "confidence and pressure",
        }
    }

    /// Weeks a typical player takes to settle a distance of 0.5 on this channel; each channel has its own clock.
    pub const fn base_weeks(self) -> f32 {
        match self {
            Channel::Environment => 5.0,
            Channel::Routine => 3.0,
            Channel::Football => 10.0,
            Channel::Tactical => 14.0,
            Channel::Social => 20.0,
            Channel::Mental => 12.0,
        }
    }
}

/// One number per channel.
pub type Levels = [f32; N_CHANNELS];

/// What a club does to ease a newcomer in (section 4.17), as bit flags.
pub mod support {
    pub const HOUSING: u8 = 1;
    pub const FAMILY: u8 = 2;
    pub const LANGUAGE: u8 = 4;
    pub const NUTRITION: u8 = 8;
    pub const LIAISON: u8 = 16;
    pub const PSYCHOLOGY: u8 = 32;
    pub const CONDITIONING: u8 = 64;
    pub const MENTOR: u8 = 128;
}

/// How the manager brings a signing into the side (section 4.20).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Integration {
    StartImmediately,
    /// Starts the lesser matches first.
    Gradual,
    /// Comes off the bench for a spell.
    Substitute,
    /// Trains with the group and does not play for a while.
    TrainingOnly,
    /// Starts, but is taken off early.
    ReducedMinutes,
    /// Given a simpler job in the system than the rest.
    SimplifiedRole,
    /// Builds up his body before anything else.
    ConditioningFirst,
}

impl Integration {
    pub const fn label(self) -> &'static str {
        match self {
            Integration::StartImmediately => "straight into the side",
            Integration::Gradual => "the lesser matches first",
            Integration::Substitute => "from the bench at first",
            Integration::TrainingOnly => "training only for now",
            Integration::ReducedMinutes => "starts, comes off early",
            Integration::SimplifiedRole => "a simpler role",
            Integration::ConditioningFirst => "conditioning first",
        }
    }
}

/// A player still settling in.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Adapting {
    pub since: Date,
    pub club: ClubId,
    /// How far this move takes him from what he knows, per channel, 0..1 (his experience already taken off).
    pub distance: Levels,
    /// Weeks this player, at this club, needs to settle each channel completely.
    pub weeks: Levels,
    /// How settled he is, per channel, 0..1.
    pub progress: Levels,
    pub support: u8,
    pub plan: Integration,
    pub plan_until: Date,
    /// Heavy minutes played while body and routine were still unsettled (feeds the negative loop, section 4.21).
    pub strain: f32,
    /// Running level of recent performances against his own norm (feeds the positive loop).
    pub form: f32,
}

/// How a settling-in ended, kept so a later review can say whether adaptation was the trouble.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Settled {
    pub date: Date,
    pub weeks: u16,
    pub slowest: Channel,
    /// Took far longer than his club expected, or never did.
    pub struggled: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Adaptations {
    pub current: FxHashMap<PlayerId, Adapting>,
    pub done: FxHashMap<PlayerId, Settled>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Level {
    Low,
    Moderate,
    High,
}

impl Level {
    pub fn of(x: f32) -> Level {
        if x < 0.3 {
            Level::Low
        } else if x < 0.6 {
            Level::Moderate
        } else {
            Level::High
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Level::Low => "low",
            Level::Moderate => "moderate",
            Level::High => "high",
        }
    }
}

/// What a club believes about how a signing will settle (section 4.19): risks per channel, how soon he is likely to be useful, and
/// how sure it is.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Readiness {
    pub risks: [Level; N_CHANNELS],
    /// Weeks until he is expected to be playing near his level.
    pub weeks_to_useful: f32,
    pub immediate_availability: Level,
    pub immediate_effectiveness: Level,
    pub confidence: Confidence,
}
