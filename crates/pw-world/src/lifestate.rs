//! What a person carries onto the pitch from the rest of his life (locked design 7.35-7.54).
//!
//! An event is not a bonus or a penalty. It is *interpreted* by the person who lives it, and the interpretation sets psychological
//! channels (rumination, anger, motivation, sleep, focus...) whose *shape in time* is its own: onset, a peak, a decay that can be sharp,
//! steady or lingering. Football sees the channels only through behaviour: attention, composure, appetite for risk, energy. Major
//! moments that leave a mark are kept as scars that can come back when the place or the situation does. Positive and negative
//! experiences are the same machinery, and can be present together.

use pw_core::{ClubId, Date, EventId, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

pub const N_CHAN: usize = 11;

/// The psychological channels an experience can move. State, not trait: traits decide how these develop.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Chan {
    Rumination,
    Anger,
    Grief,
    Excitement,
    Motivation,
    /// Sleep and recovery; negative is poor sleep.
    Sleep,
    Focus,
    Confidence,
    /// Emotional control; negative is a short fuse.
    Calm,
    /// Appetite for risk; negative is playing safe.
    Risk,
    Belonging,
}

impl Chan {
    pub const ALL: [Chan; N_CHAN] =
        [Chan::Rumination, Chan::Anger, Chan::Grief, Chan::Excitement, Chan::Motivation, Chan::Sleep, Chan::Focus, Chan::Confidence, Chan::Calm, Chan::Risk, Chan::Belonging];

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            Chan::Rumination => "dwelling on it",
            Chan::Anger => "anger",
            Chan::Grief => "grief",
            Chan::Excitement => "excitement",
            Chan::Motivation => "motivation",
            Chan::Sleep => "sleep",
            Chan::Focus => "focus",
            Chan::Confidence => "confidence",
            Chan::Calm => "emotional control",
            Chan::Risk => "appetite for risk",
            Chan::Belonging => "sense of belonging",
        }
    }
}

/// The effect of one experience on each channel, at its peak, -100..100.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct Effects(pub [i8; N_CHAN]);

impl Effects {
    pub fn get(&self, c: Chan) -> i8 {
        self.0[c.idx()]
    }

    pub fn add(&mut self, c: Chan, v: i32) {
        self.0[c.idx()] = (i32::from(self.0[c.idx()]) + v).clamp(-100, 100) as i8;
    }

    pub fn scale(&mut self, k: f32) {
        for v in &mut self.0 {
            *v = (f32::from(*v) * k).round().clamp(-100.0, 100.0) as i8;
        }
    }

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&v| v == 0)
    }
}

/// What kind of thing happened, from any part of a life. The same list serves good and bad news.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LoadKind {
    Bereavement,
    ParentIll,
    Breakup,
    NewRelationship,
    NewChild,
    Marriage,
    Loneliness,
    FinancialTrouble,
    /// A pile-on: abuse, mockery, being the story for the wrong reasons.
    OnlineAbuse,
    /// Sudden attention and praise, the weight of expectation.
    Hype,
    Scandal,
    CallUp,
    NewContract,
    ContractWorry,
    LongInjury,
    PublicMistake,
    MissedPenalty,
    RedCard,
    Humiliation,
    /// Something that went right in public: a comeback, a decisive goal.
    Triumph,
}

impl LoadKind {
    pub const fn label(self) -> &'static str {
        match self {
            LoadKind::Bereavement => "a bereavement",
            LoadKind::ParentIll => "a parent's illness",
            LoadKind::Breakup => "a break-up",
            LoadKind::NewRelationship => "a new relationship",
            LoadKind::NewChild => "a new child",
            LoadKind::Marriage => "a marriage",
            LoadKind::Loneliness => "loneliness in a new place",
            LoadKind::FinancialTrouble => "money worries",
            LoadKind::OnlineAbuse => "abuse online",
            LoadKind::Hype => "sudden attention",
            LoadKind::Scandal => "a controversy",
            LoadKind::CallUp => "a national-team call-up",
            LoadKind::NewContract => "a new contract",
            LoadKind::ContractWorry => "an uncertain contract",
            LoadKind::LongInjury => "a long injury",
            LoadKind::PublicMistake => "a public mistake",
            LoadKind::MissedPenalty => "a missed penalty",
            LoadKind::RedCard => "a sending-off",
            LoadKind::Humiliation => "a heavy defeat",
            LoadKind::Triumph => "a triumph",
        }
    }

    /// Would the person ordinarily keep this to himself?
    pub const fn private(self) -> bool {
        matches!(self, LoadKind::Bereavement | LoadKind::ParentIll | LoadKind::Breakup | LoadKind::FinancialTrouble | LoadKind::Loneliness | LoadKind::ContractWorry)
    }

    /// Days it would ordinarily be expected to matter, before the person and his people change that.
    pub const fn expected_days(self) -> u16 {
        match self {
            LoadKind::Bereavement => 90,
            LoadKind::ParentIll => 60,
            LoadKind::Breakup => 45,
            LoadKind::NewRelationship => 30,
            LoadKind::NewChild => 70,
            LoadKind::Marriage => 12,
            LoadKind::Loneliness => 120,
            LoadKind::FinancialTrouble => 100,
            LoadKind::OnlineAbuse => 8,
            LoadKind::Hype => 30,
            LoadKind::Scandal => 21,
            LoadKind::CallUp => 12,
            LoadKind::NewContract => 20,
            LoadKind::ContractWorry => 50,
            LoadKind::LongInjury => 60,
            LoadKind::PublicMistake => 3,
            LoadKind::MissedPenalty => 10,
            LoadKind::RedCard => 7,
            LoadKind::Humiliation => 5,
            LoadKind::Triumph => 6,
        }
    }

    pub const fn tail(self) -> Tail {
        match self {
            LoadKind::Bereavement | LoadKind::Loneliness | LoadKind::LongInjury => Tail::Lingering,
            LoadKind::OnlineAbuse | LoadKind::PublicMistake | LoadKind::RedCard | LoadKind::Humiliation | LoadKind::Triumph | LoadKind::CallUp => Tail::Sharp,
            LoadKind::FinancialTrouble | LoadKind::ContractWorry | LoadKind::ParentIll => Tail::Recurring,
            _ => Tail::Steady,
        }
    }
}

/// The shape a load fades in.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Tail {
    /// Gone quickly once it has peaked.
    Sharp,
    /// Fades evenly.
    Steady,
    /// Fades slowly with a long tail.
    Lingering,
    /// Fades, then a reminder brings part of it back.
    Recurring,
}

/// One experience in force: its interpretation (`effects`) and its shape in time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Load {
    pub kind: LoadKind,
    pub cause: EventId,
    pub since: Date,
    /// Days to reach full force.
    pub onset: u8,
    /// What would ordinarily be expected, and what it turned out to be for this person with these people around him.
    pub expected_days: u16,
    pub actual_days: u16,
    pub tail: Tail,
    /// What the person made of it, at full force.
    pub effects: Effects,
    /// Support attempts that helped and attempts that backfired.
    pub helped: u8,
    pub misfired: u8,
    /// How many times a reminder has brought it back.
    pub reminded: u8,
}

impl Load {
    /// How much of its full force it has today, 0..1.
    pub fn intensity(&self, today: Date) -> f32 {
        let t = self.since.days_until(today);
        if t < 0 {
            return 0.0;
        }
        let t = t as f32;
        let onset = f32::from(self.onset).max(1.0);
        if t < onset {
            return t / onset;
        }
        let span = f32::from(self.actual_days).max(1.0);
        let peak = (span * 0.2).max(1.0);
        let after = (t - onset - peak).max(0.0);
        let f = match self.tail {
            Tail::Sharp => 0.5f32.powf(after / (span * 0.2).max(1.0)),
            Tail::Steady => (1.0 - after / span).max(0.0),
            Tail::Lingering => 1.0 / (1.0 + after / (span * 0.25)),
            Tail::Recurring => (1.0 - after / span).max(0.0) + 0.12,
        };
        let f = if matches!(self.tail, Tail::Lingering) && after > span * 3.0 { 0.0 } else { f };
        let f = if matches!(self.tail, Tail::Recurring) && after > span * 2.0 { 0.0 } else { f };
        // A reminder that has come back leaves it a little stronger for longer.
        (f * (1.0 + 0.08 * f32::from(self.reminded))).clamp(0.0, 1.0)
    }

    /// Whether it still matters at all.
    pub fn live(&self, today: Date) -> bool {
        let t = self.since.days_until(today);
        t < i32::from(self.onset) || self.intensity(today) >= 0.04
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ScarKind {
    SeriousInjury,
    MissedDecisivePenalty,
    RedCardInBigMatch,
    Humiliation,
    PublicMistake,
}

impl ScarKind {
    pub const fn label(self) -> &'static str {
        match self {
            ScarKind::SeriousInjury => "a serious injury there",
            ScarKind::MissedDecisivePenalty => "a penalty missed when it mattered",
            ScarKind::RedCardInBigMatch => "a sending-off in a big match",
            ScarKind::Humiliation => "a humiliating defeat",
            ScarKind::PublicMistake => "a mistake everyone saw",
        }
    }
}

/// A major moment that stays. Its acute effect ends; the memory does not, and can come back with the place or the situation.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Scar {
    pub kind: ScarKind,
    pub cause: EventId,
    pub date: Date,
    /// Whose ground it happened at (`NONE` when the place does not matter).
    pub venue: ClubId,
    /// 0..100.
    pub weight: u8,
    pub reactivated: u8,
    pub last: Date,
}

/// The lived state of one person.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct PersonState {
    pub loads: SmallVec<[Load; 4]>,
    pub scars: SmallVec<[Scar; 3]>,
    /// The day the state was last brought up to date.
    pub checked: Date,
}

/// How much a manager knows of a player's private state, and how he knows it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Awareness {
    /// Nothing has reached him.
    Unaware,
    /// He sees only that the man has been poor in training.
    Dip,
    /// He knows what is going on.
    Knows,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Known {
    pub level: Awareness,
    /// What he knows it is; only meaningful when he `Knows`.
    pub kind: LoadKind,
    /// How heavy he judges it, 0..100: his belief, not the truth.
    pub severity: u8,
    pub since: Date,
    /// What he has decided to do about it for now.
    pub stance: Handling,
}

/// What a manager did about a player who was struggling (7.44, 7.54).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Handling {
    Start,
    Rest,
    Bench,
    SendHome,
    SimplifyRole,
    ProtectPublicly,
    SayNothing,
    SeekHelp,
}

impl Handling {
    pub const fn label(self) -> &'static str {
        match self {
            Handling::Start => "picked the player anyway",
            Handling::Rest => "rested the player",
            Handling::Bench => "left the player out",
            Handling::SendHome => "sent the player home",
            Handling::SimplifyRole => "gave the player a simpler job",
            Handling::ProtectPublicly => "stood up for the player in public",
            Handling::SayNothing => "said nothing",
            Handling::SeekHelp => "asked the support staff to help",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Handled {
    pub date: Date,
    pub manager: PersonId,
    pub player: PersonId,
    pub handling: Handling,
    /// What he took it to be (his belief) and the event it followed.
    pub believed: LoadKind,
    pub cause: EventId,
    /// How much the next match mattered, 0..1, when he decided.
    pub stakes: f32,
}

/// The book of lived state.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct LifeStates {
    pub by: FxHashMap<PersonId, PersonState>,
    /// (manager, player) -> what the manager knows of the player's state.
    pub known: FxHashMap<(PersonId, PersonId), Known>,
    /// Recent handling decisions, oldest first.
    pub handled: std::collections::VecDeque<Handled>,
    /// Events up to here have been read.
    pub cursor: EventId,
}

impl LifeStates {
    pub const HANDLED_CAP: usize = 400;

    pub fn state(&self, who: PersonId) -> Option<&PersonState> {
        self.by.get(&who)
    }

    pub fn note_handled(&mut self, h: Handled) {
        if self.handled.len() >= Self::HANDLED_CAP {
            self.handled.pop_front();
        }
        self.handled.push_back(h);
    }
}
