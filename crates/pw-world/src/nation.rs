use pw_core::{CompId, Date};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::names::NameId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Confed {
    Uefa,
    Conmebol,
    Concacaf,
    Caf,
    Afc,
    Ofc,
}

impl Confed {
    pub const ALL: [Confed; 6] = [Confed::Uefa, Confed::Conmebol, Confed::Concacaf, Confed::Caf, Confed::Afc, Confed::Ofc];

    pub fn from_code(s: &str) -> Option<Self> {
        Some(match s.to_ascii_uppercase().as_str() {
            "UEFA" => Confed::Uefa,
            "CONMEBOL" => Confed::Conmebol,
            "CONCACAF" => Confed::Concacaf,
            "CAF" => Confed::Caf,
            "AFC" => Confed::Afc,
            "OFC" => Confed::Ofc,
            _ => return None,
        })
    }

    pub const fn code(self) -> &'static str {
        match self {
            Confed::Uefa => "UEFA",
            Confed::Conmebol => "CONMEBOL",
            Confed::Concacaf => "CONCACAF",
            Confed::Caf => "CAF",
            Confed::Afc => "AFC",
            Confed::Ofc => "OFC",
        }
    }
}

/// Current season dates for a nation's domestic football.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NationSeason {
    /// Calendar year the season starts in.
    pub year: i32,
    pub start: Date,
    pub end: Date,
    pub windows: SmallVec<[(Date, Date); 2]>,
    pub winter_break: Option<(Date, Date)>,
}

impl NationSeason {
    pub fn window_open(&self, d: Date) -> bool {
        self.windows.iter().any(|&(a, b)| d >= a && d <= b)
    }

    /// Season label, e.g. "2026/27" or "2026".
    pub fn label(&self) -> String {
        if self.end.year() != self.year {
            format!("{}/{:02}", self.year, (self.year + 1) % 100)
        } else {
            self.year.to_string()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Nation {
    pub code: String,
    pub name: String,
    pub confed: Confed,
    /// Football strength, 0–10,000.
    pub reputation: u16,
    /// Wage/revenue level relative to the richest economy (1.0).
    pub economy: f32,
    /// Youth development quality 1–20 (talent density of regens).
    pub youth_rating: u8,
    /// Index into the data pack's calendars.
    pub calendar: u8,
    /// Domestic leagues, top tier first.
    pub leagues: Vec<CompId>,
    pub cups: SmallVec<[CompId; 2]>,
    pub season: NationSeason,
    /// Name pools harvested from imported people, used for regens.
    pub first_names: Vec<NameId>,
    pub last_names: Vec<NameId>,
}
