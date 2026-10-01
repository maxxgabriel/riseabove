//! The club's training week and what the squad has practised.
//!
//! Resolved per team per week, never per session or per player: a week's plan
//! comes from fixture density, the manager's philosophy and staff advice; the
//! plan feeds daily load, development emphasis and tactical familiarity.

use pw_core::TeamId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// One team's week. Shares are percent of training time and sum to 100.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LoadPlan {
    /// Matches in the seven days from the plan's Monday.
    pub matches: u8,
    /// Training days left once match-day routines are taken out.
    pub days: u8,
    pub tactical: u8,
    pub physical: u8,
    pub recovery: u8,
}

impl Default for LoadPlan {
    fn default() -> Self {
        Self { matches: 1, days: 4, tactical: 40, physical: 35, recovery: 25 }
    }
}

impl LoadPlan {
    /// Multiplier on the club's standard daily training load.
    pub fn load_mult(&self) -> f32 {
        (0.75 + 0.5 * f32::from(self.physical) / 100.0 * f32::from(self.days) / 4.0).clamp(0.5, 1.25)
    }
}

/// Owned by `pw_sim::training`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TrainingExt {
    pub plans: FxHashMap<TeamId, LoadPlan>,
    /// Per team: how well drilled each formation is, (formation index, 0–100). At most three kept.
    pub familiarity: FxHashMap<TeamId, SmallVec<[(u8, u8); 3]>>,
}

impl TrainingExt {
    /// How well drilled a team is in a formation, 0–1. A shape never practised is 0.
    pub fn drilled(&self, team: TeamId, formation: u8) -> f32 {
        self.familiarity.get(&team).and_then(|v| v.iter().find(|x| x.0 == formation)).map_or(0.0, |x| f32::from(x.1) / 100.0)
    }

    pub fn load_mult(&self, team: TeamId) -> f32 {
        self.plans.get(&team).map_or(1.0, LoadPlan::load_mult)
    }
}
