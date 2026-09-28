use serde::{Deserialize, Serialize};

use crate::pos::{Pos, Role};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord, Default)]
pub enum Mentality {
    VeryDefensive,
    Defensive,
    #[default]
    Balanced,
    Positive,
    Attacking,
}

impl Mentality {
    /// Signed level, -2..=2.
    pub const fn level(self) -> i32 {
        self as i32 - 2
    }

    pub const fn from_level(l: i32) -> Self {
        match l {
            i32::MIN..=-2 => Mentality::VeryDefensive,
            -1 => Mentality::Defensive,
            0 => Mentality::Balanced,
            1 => Mentality::Positive,
            _ => Mentality::Attacking,
        }
    }

    pub const fn shift(self, by: i32) -> Self {
        Self::from_level(self.level() + by)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Slot {
    pub pos: Pos,
    pub role: Role,
}

/// Team instructions. Sliders are 0–100 with 50 as neutral.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Tactics {
    /// Index into the data pack's formation list.
    pub formation: u8,
    pub mentality: Mentality,
    pub tempo: u8,
    pub width: u8,
    pub directness: u8,
    pub line: u8,
    pub press: u8,
}

impl Default for Tactics {
    fn default() -> Self {
        Self { formation: 0, mentality: Mentality::Balanced, tempo: 50, width: 50, directness: 50, line: 50, press: 50 }
    }
}

impl Tactics {
    #[inline]
    pub fn unit(v: u8) -> f32 {
        (f32::from(v) - 50.0) / 50.0
    }
}
