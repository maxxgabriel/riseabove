//! What academies have actually produced. Identity and pathway reputation are
//! statistics of graduates, not modifiers: a club is known for what its
//! players became, and families read that record when they choose.

use pw_core::ClubId;
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

/// A club's record of players it developed, weighted toward recent years.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Pathway {
    /// Players it developed who reached 22 (decayed).
    pub total: f32,
    /// Of those, how many made the senior game.
    pub made: f32,
    /// Those who made it, by line: keeper, defence, midfield, attack.
    pub by_line: [f32; 4],
}

impl Pathway {
    /// Share who made it, smoothed toward a modest prior so a tiny sample says little.
    pub fn rate(&self) -> f32 {
        (self.made + 1.0) / (self.total + 4.0)
    }

    /// What the academy is known for: each line's share of its graduates, if it has had any.
    pub fn identity(&self) -> Option<[f32; 4]> {
        let s: f32 = self.by_line.iter().sum();
        (s >= 3.0).then(|| self.by_line.map(|x| x / s))
    }
}

/// Owned by `pw_sim::pathway`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AcademyExt {
    pub pathway: FxHashMap<ClubId, Pathway>,
}
