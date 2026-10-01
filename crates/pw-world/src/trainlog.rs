//! The training ground, week by week, for the people a human inhabits: which group you trained with, which coach worked with you,
//! whether you were pulled aside, and the little traces the week left on the squad (who was flying, who stayed behind, who was
//! fined, who came back, who arrived). Written on Mondays by `pw_sim::trainlog` from what the week recorded; nothing decides from it.

use pw_core::{Date, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// Weeks kept per person.
pub const KEEP: usize = 104;

/// Where you trained.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Group {
    FirstTeam,
    /// Back with the group for parts of sessions after an injury.
    Partial,
    /// Inside with the physio.
    Rehab,
}

/// How your own week went.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Week {
    Sharp,
    Ordinary,
    Flat,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Trace {
    /// The manager took you aside: good (praise for your work) or not.
    PulledAside { by: PersonId, good: bool },
    Flying { who: PersonId },
    OffThePace { who: PersonId },
    StayedBehind { who: PersonId },
    Fined { who: PersonId },
    BackInTraining { who: PersonId },
    Arrived { who: PersonId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainingWeek {
    /// The Monday the week ended on.
    pub date: Date,
    pub group: Group,
    pub week: Week,
    /// The coach who worked with you most (`NONE` when the club has none).
    pub coach: PersonId,
    pub traces: SmallVec<[Trace; 6]>,
}

/// Owned by `pw_sim::trainlog`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TrainLogs {
    pub of: FxHashMap<PersonId, Vec<TrainingWeek>>,
}
