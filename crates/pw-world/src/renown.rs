//! Reputation has more than one audience (11 §5). A player's football
//! reputation (`PlayerCold::rep`: current, home nation, world) says what the
//! game thinks of them. Renown adds the rest: how their city sees them, how
//! far across their continent their name carries, how famous they are
//! outside football, and how many people follow them. Public image and
//! insider standing live in `Media`.

use pw_core::{Date, PersonId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Renown {
    /// Standing in the city/region of the club they play for, 0–10,000.
    pub local: u16,
    /// Standing across their club's confederation, 0–10,000.
    pub continental: u16,
    /// Celebrity beyond football, 0–10,000.
    pub fame: u16,
    /// Social following.
    pub followers: u32,
    /// Highest world reputation ever reached (for retrospectives).
    pub peak_world: u16,
    pub peak_date: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Renowns {
    pub people: FxHashMap<PersonId, Renown>,
}

impl Renowns {
    pub fn of(&self, p: PersonId) -> Renown {
        self.people.get(&p).copied().unwrap_or_default()
    }
}
