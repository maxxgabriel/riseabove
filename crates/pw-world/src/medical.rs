//! Bodies have histories (05 §4). Every injury is a case: diagnosed with
//! uncertainty by the people who happen to be in the medical room, treated
//! one way or another (sometimes by the player's choice), sometimes rushed,
//! sometimes set back. What is left behind — scar tissue, fragility in a
//! region, a chronic condition that needs managing — raises the hazard of the
//! next one and shapes how managers use the player.

use pw_core::{ClubId, Date, EventId, PlayerId, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Treatment {
    /// Rest and rehabilitation.
    Conservative,
    /// Pain-killing injection / play-through management.
    Managed,
    /// Operation: longer, but the tissue comes back sounder.
    Surgery,
}

impl Treatment {
    pub const fn label(self) -> &'static str {
        match self {
            Treatment::Conservative => "rest and rehabilitation",
            Treatment::Managed => "managed through",
            Treatment::Surgery => "surgery",
        }
    }
}

/// One injury, open or closed.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Case {
    pub player: PlayerId,
    /// Injury catalogue index + 1.
    pub injury: u16,
    /// Wear slot of the region, or `u8::MAX` for illness.
    pub region: u8,
    pub club: ClubId,
    pub date: Date,
    /// The medical team's current estimate of days out (what everyone sees).
    pub estimate: u16,
    /// How sure they are, 0–100.
    pub certainty: u8,
    pub treatment: Treatment,
    /// Came back before the body was ready.
    pub rushed: bool,
    pub setbacks: u8,
    /// Recurrence of an earlier injury in the same region.
    pub recurrence: bool,
    /// Picked up on international duty.
    pub on_duty: bool,
    /// Days actually missed (filled when closed).
    pub actual: u16,
    pub closed: Option<Date>,
}

/// A region that has been hurt and not fully forgiven.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Fragility {
    pub region: u8,
    /// Extra hazard multiplier for this region (0 = none; 1 = doubled).
    pub level: f32,
    pub since: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Chronic {
    /// Needs rest between matches (knees, backs, tendons).
    Managed { region: u8 },
    /// Recurrent illness-type condition (asthma, migraine).
    Systemic,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Medical {
    /// Open cases, one per injured player.
    pub open: FxHashMap<PlayerId, Case>,
    /// Closed cases, most recent last (trimmed to the last dozen per player).
    pub history: FxHashMap<PlayerId, SmallVec<[Case; 4]>>,
    pub fragile: FxHashMap<PlayerId, SmallVec<[Fragility; 2]>>,
    pub chronic: FxHashMap<PlayerId, SmallVec<[Chronic; 1]>>,
    /// Players the medical staff will let play through pain on request.
    pub willing_to_rush: FxHashMap<PlayerId, bool>,
}

impl Medical {
    pub fn history_of(&self, p: PlayerId) -> &[Case] {
        self.history.get(&p).map_or(&[], |v| v.as_slice())
    }

    /// Hazard multiplier from fragile regions and chronic conditions.
    pub fn fragility(&self, p: PlayerId) -> f32 {
        let f: f32 = self.fragile.get(&p).map_or(0.0, |v| v.iter().map(|x| x.level).sum());
        let c = self.chronic.get(&p).map_or(0.0, |v| v.len() as f32 * 0.25);
        1.0 + f.min(1.5) + c
    }

    pub fn needs_managing(&self, p: PlayerId) -> bool {
        self.chronic.get(&p).is_some_and(|v| v.iter().any(|c| matches!(c, Chronic::Managed { .. })))
    }

    /// Serious injuries in the last `days`.
    pub fn serious_recent(&self, p: PlayerId, today: Date, days: i32) -> usize {
        self.history_of(p).iter().filter(|c| c.actual >= 42 && c.date.days_until(today) <= days).count()
    }
}

/// Where a case sits on the way back, from how much of it is truly left.
/// Derived, never stored: no hot state per player.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ReturnStage {
    Rehab,
    Individual,
    PartialTeam,
    FullTraining,
    BenchReady,
    MatchReady,
}

impl ReturnStage {
    /// `left` is the fraction of the case still to run (1 = just injured).
    pub fn of(left: f32) -> ReturnStage {
        match left {
            x if x > 0.8 => ReturnStage::Rehab,
            x if x > 0.6 => ReturnStage::Individual,
            x if x > 0.4 => ReturnStage::PartialTeam,
            x if x > 0.2 => ReturnStage::FullTraining,
            x if x > 0.08 => ReturnStage::BenchReady,
            _ => ReturnStage::MatchReady,
        }
    }
}

/// A player cleared before the body was ready. While the window lasts, the
/// hazard is raised in proportion to how much was left, so a recurrence
/// emerges from the ordinary injury machinery rather than from a script.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RushedReturn {
    pub ruling: u32,
    pub event: EventId,
    pub from: Date,
    /// Fraction of the case that was truly left when they were cleared.
    pub left: f32,
    pub region: u8,
    pub manager: StaffId,
}

/// Owned by `pw_sim::returns`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MedicalExt {
    pub rushed: FxHashMap<PlayerId, RushedReturn>,
    /// A manager's learned readiness to rush players back, moved only by how
    /// past rushes turned out (not by whether they were sound).
    pub rush_bias: FxHashMap<StaffId, i8>,
}
