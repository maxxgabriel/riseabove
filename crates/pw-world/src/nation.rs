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
        if self.end.year() != self.year { format!("{}/{:02}", self.year, (self.year + 1) % 100) } else { self.year.to_string() }
    }
}

/// A setting inferred from the confederation and the country's code, when nothing better is known. Deterministic, varied from country
/// to country, and marked `known: false` so nobody mistakes it for data. Real values, where a source has them, replace it.
pub fn inferred_environment(code: &str, confed: Confed) -> Environment {
    let h = code.bytes().fold(0x9e37_79b9u64, |a, b| pw_core::rng::hash_key(&[a, u64::from(b)]));
    let pick = |salt: u64, lo: i64, hi: i64| -> i64 { lo + (pw_core::rng::hash_key(&[h, salt]) % ((hi - lo + 1) as u64)) as i64 };
    let (climate, tz, language, culture, pace, physical) = match confed {
        Confed::Uefa => (pick(1, -1, 1), pick(2, -1, 3), pick(3, 0, 11), pick(4, 0, 4), pick(5, 42, 62), pick(6, 42, 68)),
        Confed::Conmebol => (pick(1, 1, 3), pick(2, -5, -3), pick(3, 20, 21), 10, pick(5, 52, 72), pick(6, 42, 60)),
        Confed::Concacaf => (pick(1, 1, 3), pick(2, -8, -5), pick(3, 20, 30), 11, pick(5, 50, 70), pick(6, 48, 64)),
        Confed::Caf => (pick(1, 1, 3), pick(2, -1, 3), pick(3, 40, 44), 12, pick(5, 50, 70), pick(6, 48, 68)),
        Confed::Afc => (pick(1, -2, 3), pick(2, 3, 9), pick(3, 50, 58), pick(4, 13, 15), pick(5, 48, 66), pick(6, 38, 60)),
        Confed::Ofc => (pick(1, 1, 2), pick(2, 10, 12), 60, 16, 50, 55),
    };
    let altitude = match confed {
        Confed::Conmebol if pick(7, 0, 9) < 3 => pick(8, 1500, 3600),
        _ => pick(8, 5, 600),
    };
    Environment {
        climate: climate as i8,
        humidity: pick(9, 35, 88) as u8,
        altitude: altitude as u16,
        tz: tz as i8,
        language: language as u8,
        culture: culture as u8,
        pace: pace as u8,
        physical: physical as u8,
        tempo: pick(10, 40, 68) as u8,
        tax: pick(11, 10, 45) as u8,
        living: pick(12, 65, 140) as u8,
        known: false,
    }
}

/// The physical and cultural setting of a country's football: what a newcomer has to get used to (locked design 4.13, 4.14).
/// Differences are compared, never the nations by name.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Environment {
    /// -3 very cold .. +3 very hot.
    pub climate: i8,
    /// 0..100.
    pub humidity: u8,
    /// Metres above sea level where most football is played.
    pub altitude: u16,
    /// Hours from UTC.
    pub tz: i8,
    /// Language family: countries sharing a value share (most of) a language.
    pub language: u8,
    /// Cultural cluster.
    pub culture: u8,
    /// How the league plays, 0..100 each.
    pub pace: u8,
    pub physical: u8,
    pub tempo: u8,
    /// Income tax on wages, percent, and the cost of living against a typical place (100 = typical): what the same gross wage is worth.
    pub tax: u8,
    pub living: u8,
    /// Whether this came from real data. `false` is an inference from region and must be treated as one.
    pub known: bool,
}

impl Default for Environment {
    fn default() -> Self {
        Self { climate: 0, humidity: 55, altitude: 100, tz: 0, language: 0, culture: 0, pace: 50, physical: 50, tempo: 50, tax: 30, living: 100, known: false }
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
    pub env: Environment,
}
