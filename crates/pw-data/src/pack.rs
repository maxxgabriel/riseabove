use std::path::Path;

use pw_core::attr::{Attr, CurveGroup, N_ATTR, N_CURVE_GROUPS};
use pw_core::math::interp;
use pw_core::pos::{N_POS, Pos, Role};
use pw_core::tactics::Slot;
use serde::{Deserialize, Serialize};

use crate::tuning::Tuning;

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("{file}: {source}")]
    Parse { file: &'static str, source: toml::de::Error },
    #[error("{file}: {message}")]
    Invalid { file: &'static str, message: String },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

const FILES: [&str; 7] = ["tuning.toml", "formations.toml", "weights.toml", "curves.toml", "injuries.toml", "calendar.toml", "rules.toml"];

const BUILTIN: [&str; 7] = [
    include_str!("../../../data/engine/tuning.toml"),
    include_str!("../../../data/engine/formations.toml"),
    include_str!("../../../data/engine/weights.toml"),
    include_str!("../../../data/engine/curves.toml"),
    include_str!("../../../data/engine/injuries.toml"),
    include_str!("../../../data/engine/calendar.toml"),
    include_str!("../../../data/engine/rules.toml"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataPack {
    pub tuning: Tuning,
    pub formations: Vec<Formation>,
    pub weights: PositionWeights,
    pub curves: AgeCurves,
    pub injuries: Vec<InjuryDef>,
    pub calendars: Vec<CalendarDef>,
    pub international_windows: Vec<DateSpan>,
    /// Football rules profiles (registration, labour, loans, discipline).
    #[serde(default)]
    pub rules: crate::rules::RulesFile,
}

impl DataPack {
    pub fn builtin() -> Self {
        Self::from_sources(BUILTIN).expect("builtin engine data pack is valid")
    }

    /// Load from a directory; files missing there fall back to the builtin pack.
    pub fn load_dir(dir: &Path) -> Result<Self, DataError> {
        let mut texts: [String; 7] = Default::default();
        for (i, name) in FILES.iter().enumerate() {
            let p = dir.join(name);
            texts[i] = if p.exists() { std::fs::read_to_string(p)? } else { BUILTIN[i].to_string() };
        }
        Self::from_sources(texts.each_ref().map(String::as_str))
    }

    fn from_sources(src: [&str; 7]) -> Result<Self, DataError> {
        fn parse<T: for<'de> Deserialize<'de>>(file: &'static str, s: &str) -> Result<T, DataError> {
            toml::from_str(s).map_err(|source| DataError::Parse { file, source })
        }

        let tuning: Tuning = parse(FILES[0], src[0])?;
        let formations: FormationsFile = parse(FILES[1], src[1])?;
        let weights: WeightsFile = parse(FILES[2], src[2])?;
        let curves: CurvesFile = parse(FILES[3], src[3])?;
        let injuries: InjuriesFile = parse(FILES[4], src[4])?;
        let calendar: CalendarFile = parse(FILES[5], src[5])?;
        let rules: crate::rules::RulesFile = parse(FILES[6], src[6])?;

        let formations = formations.formation.into_iter().map(Formation::try_from).collect::<Result<Vec<_>, _>>()?;
        if formations.is_empty() {
            return Err(DataError::Invalid { file: FILES[1], message: "no formations".into() });
        }
        Ok(Self {
            tuning,
            formations,
            weights: PositionWeights::build(&weights)?,
            curves: AgeCurves::build(curves)?,
            injuries: injuries.injury,
            calendars: calendar.calendar,
            international_windows: calendar.international_windows,
            rules,
        })
    }

    pub fn calendar(&self, key: &str) -> &CalendarDef {
        self.calendars.iter().find(|c| c.key == key).unwrap_or(&self.calendars[0])
    }

    pub fn formation_index(&self, key: &str) -> Option<u8> {
        self.formations.iter().position(|f| f.key == key).map(|i| i as u8)
    }
}

// ---------------------------------------------------------------- formations

#[derive(Deserialize)]
struct FormationsFile {
    formation: Vec<FormationRaw>,
}

#[derive(Deserialize)]
struct FormationRaw {
    key: String,
    name: String,
    slots: Vec<(Pos, Role)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Formation {
    pub key: String,
    pub name: String,
    pub slots: [Slot; 11],
}

impl TryFrom<FormationRaw> for Formation {
    type Error = DataError;

    fn try_from(r: FormationRaw) -> Result<Self, DataError> {
        let invalid = |message: String| DataError::Invalid { file: FILES[1], message };
        if r.slots.len() != 11 {
            return Err(invalid(format!("{}: {} slots", r.key, r.slots.len())));
        }
        if r.slots.iter().filter(|(p, _)| *p == Pos::GK).count() != 1 {
            return Err(invalid(format!("{}: needs exactly one GK", r.key)));
        }
        if let Some((p, role)) = r.slots.iter().find(|(p, role)| !role.fits(*p)) {
            return Err(invalid(format!("{}: role {role:?} cannot play {p:?}", r.key)));
        }
        let slots = std::array::from_fn(|i| Slot { pos: r.slots[i].0, role: r.slots[i].1 });
        Ok(Formation { key: r.key, name: r.name, slots })
    }
}

// ----------------------------------------------------------- position weights

#[derive(Deserialize)]
struct WeightsFile {
    #[serde(flatten)]
    positions: std::collections::BTreeMap<String, std::collections::BTreeMap<String, f32>>,
}

/// CA weights per position (17 §1). Each row sums to 1, so a player with every
/// attribute at 20 has CA 200 in that position.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionWeights {
    rows: Vec<Vec<f32>>,
}

impl PositionWeights {
    fn build(f: &WeightsFile) -> Result<Self, DataError> {
        let invalid = |message: String| DataError::Invalid { file: FILES[2], message };
        let mut rows = vec![vec![0.0f32; N_ATTR]; N_POS];
        for pos in Pos::ALL {
            let listed = f.positions.get(pos.code()).ok_or_else(|| invalid(format!("missing position {}", pos.code())))?;
            let row = &mut rows[pos.idx()];
            let mut sum = 0.0;
            for (k, &w) in listed {
                let a = Attr::from_key(k).ok_or_else(|| invalid(format!("{}: unknown attribute {k}", pos.code())))?;
                row[a.idx()] = w;
                sum += w;
            }
            if sum > 1.0 + 1e-4 {
                return Err(invalid(format!("{}: weights sum to {sum}", pos.code())));
            }
            let eligible = |a: Attr| {
                row[a.idx()] == 0.0
                    && a != Attr::Eccentricity
                    && (pos == Pos::GK || !a.is_goalkeeping())
            };
            let rest: Vec<Attr> = Attr::ALL.into_iter().filter(|&a| eligible(a)).collect();
            let share = (1.0 - sum) / rest.len().max(1) as f32;
            for a in rest {
                row[a.idx()] = share;
            }
        }
        Ok(Self { rows })
    }

    #[inline]
    pub fn row(&self, pos: Pos) -> &[f32] {
        &self.rows[pos.idx()]
    }

    #[inline]
    pub fn weight(&self, pos: Pos, a: Attr) -> f32 {
        self.rows[pos.idx()][a.idx()]
    }
}

// ----------------------------------------------------------------- age curves

#[derive(Deserialize)]
struct CurvesFile {
    speed: Vec<(f32, f32)>,
    power: Vec<(f32, f32)>,
    technical: Vec<(f32, f32)>,
    mental: Vec<(f32, f32)>,
    goalkeeping: Vec<(f32, f32)>,
}

/// Growth (>0) / decline (<0) factor by age for each curve group (04 §2).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgeCurves {
    knots: Vec<Vec<(f32, f32)>>,
}

impl AgeCurves {
    fn build(f: CurvesFile) -> Result<Self, DataError> {
        let knots = vec![f.speed, f.power, f.technical, f.mental, f.goalkeeping];
        debug_assert_eq!(knots.len(), N_CURVE_GROUPS);
        for k in &knots {
            if k.windows(2).any(|w| w[1].0 <= w[0].0) {
                return Err(DataError::Invalid { file: FILES[3], message: "curve ages must increase".into() });
            }
        }
        Ok(Self { knots })
    }

    #[inline]
    pub fn factor(&self, g: CurveGroup, age: f32) -> f32 {
        interp(&self.knots[g as usize], age)
    }
}

// ------------------------------------------------------------------ injuries

#[derive(Deserialize)]
struct InjuriesFile {
    injury: Vec<InjuryDef>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyRegion {
    Head,
    Shoulder,
    Back,
    Groin,
    Hamstring,
    Quadriceps,
    Knee,
    Calf,
    Ankle,
    Foot,
    Systemic,
}

pub const N_BODY_REGIONS: usize = 10;

impl BodyRegion {
    /// Index into per-player wear arrays; `Systemic` (illness) has no wear slot.
    pub const fn wear_slot(self) -> Option<usize> {
        match self {
            BodyRegion::Systemic => None,
            r => Some(r as usize),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mechanism {
    Contact,
    NonContact,
    Overuse,
    Illness,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InjuryDef {
    pub key: String,
    pub name: String,
    pub region: BodyRegion,
    pub mechanism: Mechanism,
    /// Days out: min, mode, max (triangular).
    pub days: [u16; 3],
    /// Relative incidence within its mechanism.
    pub weight: f32,
    /// Wear added to the region (0–100 scale).
    #[serde(default)]
    pub wear: u8,
    /// Can play on at reduced condition.
    #[serde(default)]
    pub play_through: bool,
    /// Possible permanent loss: attribute and maximum points lost.
    #[serde(default)]
    pub permanent: Vec<(Attr, f32)>,
}

// ------------------------------------------------------------------- calendar

#[derive(Deserialize)]
struct CalendarFile {
    calendar: Vec<CalendarDef>,
    international_windows: Vec<DateSpan>,
}

/// Month/day, independent of year.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct MonthDay(pub u8, pub u8);

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DateSpan {
    pub start: MonthDay,
    pub end: MonthDay,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarDef {
    pub key: String,
    pub season_start: MonthDay,
    pub season_end: MonthDay,
    /// Registration windows. A window may wrap the new year.
    pub windows: Vec<DateSpan>,
    /// Midseason break with no league fixtures.
    #[serde(default)]
    pub winter_break: Option<DateSpan>,
}

impl CalendarDef {
    /// Season spans two calendar years (Aug–May) rather than one (Mar–Nov).
    pub fn crosses_year(&self) -> bool {
        self.season_end < self.season_start
    }
}
