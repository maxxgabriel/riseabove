//! A player's development as a story over time: who took them under their
//! wing, how their character has been shaped, stretches without football,
//! and the trajectory coaches and journalists read from it.

use pw_core::{Date, PersonId, PlayerId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DevRecord {
    /// Monthly current-ability snapshots (date, ca), newest last, bounded.
    pub ca: SmallVec<[(Date, u8); 12]>,
    pub mentor: PersonId,
    pub mentor_since: Date,
    /// Consecutive weeks with almost no senior football.
    pub idle_weeks: u16,
    /// Potential lost to stagnation so far.
    pub eroded: u8,
    /// Hidden-attribute drift applied so far (sum of absolute steps), for pacing.
    pub drift: u8,
    pub last_trait: Date,
}

impl DevRecord {
    /// CA change over roughly the last year.
    pub fn trend(&self) -> i16 {
        match (self.ca.first(), self.ca.last()) {
            (Some(a), Some(b)) => i16::from(b.1) - i16::from(a.1),
            _ => 0,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Growth {
    pub records: FxHashMap<PlayerId, DevRecord>,
    /// Mentor → mentees.
    pub mentees: FxHashMap<PersonId, SmallVec<[PersonId; 3]>>,
}
