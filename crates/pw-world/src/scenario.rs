//! A scenario's own tuning: the numbers and dates that make a place's football behave the way it does, kept in data so that generic
//! code implements concepts and a pack configures them (the India brief, items 2, 9 and 10).
//!
//! Every default here is the value the simulation used before it became data, so a world saved before this state existed behaves
//! as it always did. The values are **initial tuning**, not football truths: they are read from the pack at world start and
//! any of them may be recalibrated without touching code.

use pw_core::ClubId;
use serde::{Deserialize, Serialize};

use crate::FxHashMap;
use crate::ecosystem::TIERS;

/// How evidence at each level of football becomes standing, and how much an organisation needs before it acts on a child.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecognitionTuning {
    /// What a performance at each level is worth to people deciding who to look at next, indexed by `Tier`
    /// (grassroots, school, district, adult, academy, state).
    pub tier_weight: [f32; TIERS],
    /// Games at a level before half of what they showed is believed.
    pub half_sample: f32,
    /// A performance can never count for more than this in one game.
    pub game_cap: f32,
    /// Two sightings closer than this are one look.
    pub look_gap_days: i32,
    /// Attention: the base level and how far merit lifts it (logit scale).
    pub attention_base: f32,
    pub attention_merit: f32,
    /// What an academy needs before it will invite a child it has not seen at its own level: `base + sq * rep^2 + lin * rep`.
    pub need_base: f32,
    pub need_sq: f32,
    pub need_lin: f32,
    /// Feeder sides are trusted a little more (factor on the need).
    pub feeder_discount: f32,
    /// Games in all before any academy invites.
    pub min_games: f32,
    /// Sightings an academy wants: reputation from which it wants `looks_established`, else `looks_other`.
    pub established_rep: u16,
    pub looks_established: u8,
    pub looks_other: u8,
    /// Reputation from which an academy wants proof above the park, and above the school.
    pub gate_above_park: u16,
    pub gate_above_school: u16,
    /// How much of a coach's recommendation a scout believes with no history with that coach's side, and how far a record moves it.
    pub vouch_prior_trust: f32,
    pub vouch_record_weight: f32,
}

impl Default for RecognitionTuning {
    fn default() -> Self {
        Self {
            tier_weight: [0.10, 0.28, 0.55, 0.50, 0.70, 1.0],
            half_sample: 8.0,
            game_cap: 1.0,
            look_gap_days: 10,
            attention_base: -2.9,
            attention_merit: 2.6,
            need_base: 0.05,
            need_sq: 0.34,
            need_lin: 0.14,
            feeder_discount: 0.6,
            min_games: 6.0,
            established_rep: 4500,
            looks_established: 3,
            looks_other: 2,
            gate_above_park: 6500,
            gate_above_school: 8500,
            vouch_prior_trust: 0.5,
            vouch_record_weight: 0.4,
        }
    }
}

/// Things that happen on a calendar. Generic code asks whether one is due, never what month it is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CalEvent {
    /// Districts pick their sides after open trials.
    DistrictSelection,
    /// Professional clubs watch the university game.
    UniversityScouting,
    /// The national federation's identification camps.
    NationalCamp,
    /// Academy people visit school football.
    SchoolScouting,
    /// The state-team championship is drawn and begins.
    StateChampionship,
    /// Universities take stock and set budgets.
    UniversityReview,
}

impl CalEvent {
    pub fn parse(s: &str) -> Option<CalEvent> {
        Some(match s {
            "district_selection" => CalEvent::DistrictSelection,
            "university_scouting" => CalEvent::UniversityScouting,
            "national_camp" => CalEvent::NationalCamp,
            "school_scouting" => CalEvent::SchoolScouting,
            "state_championship" => CalEvent::StateChampionship,
            "university_review" => CalEvent::UniversityReview,
            _ => return None,
        })
    }
}

/// When an event is due: in these months (1-12), on this day of the month (0 = any day of the month, which the monthly pass reads as
/// "this month").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalRule {
    pub event: CalEvent,
    pub months: Vec<u8>,
    pub day: u8,
}

/// How much scouting there is and how far it reaches. Probabilities scale with the region's coverage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoutingTuning {
    /// A club's chance in a month of visiting a school: `base + coverage * near`.
    pub school_visit_base: f32,
    pub school_visit_coverage: f32,
    /// Academy people at a district side: `base + coverage * cov`.
    pub district_look_base: f32,
    pub district_look_coverage: f32,
    /// Players a district picks; camp sizes at each cut; camp age window.
    pub district_side: u8,
    pub camp_called: [u16; 3],
    pub camp_age: [u8; 2],
    /// Foreign clubs: minimum reputation to send anyone, and the chance shape `base + share * market_reputation`.
    pub foreign_min_rep: u16,
    pub foreign_base: f32,
    pub foreign_share: f32,
    /// Players a scout who has come to watch singles out.
    pub looks_per_visit: u8,
}

impl Default for ScoutingTuning {
    fn default() -> Self {
        Self {
            school_visit_base: 0.01,
            school_visit_coverage: 0.06,
            district_look_base: 0.05,
            district_look_coverage: 0.30,
            district_side: 16,
            camp_called: [100, 40, 25],
            camp_age: [14, 17],
            foreign_min_rep: 3000,
            foreign_base: 0.05,
            foreign_share: 0.5,
            looks_per_visit: 2,
        }
    }
}

/// A market abroad, a group of nations whose clubs look at the same things the same way (data: the pack names them).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketDef {
    pub key: String,
    /// Nation codes in it.
    pub nations: Vec<String>,
    /// Reputation of the home country's players there when the world begins (0-100). A scenario seed, not a fact.
    pub start: f32,
}

/// Where a piece of starting data came from. Nothing here is a fact about the real world unless it says `Imported`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DataOrigin {
    /// Verified against a source and imported as such (a database export, a licensed dataset).
    Imported,
    /// Named by the scenario's pack as an identity, with its strengths and standing set as starting values: a seed, not a record.
    ScenarioSeed,
    /// Made up by the world builder to fill a place the pack does not name.
    Generated,
}

impl DataOrigin {
    pub const fn label(self) -> &'static str {
        match self {
            DataOrigin::Imported => "Imported",
            DataOrigin::ScenarioSeed => "Scenario seed",
            DataOrigin::Generated => "Generated",
        }
    }
}

/// The tuning and calendar a scenario runs under, and where they came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub recognition: RecognitionTuning,
    pub scouting: ScoutingTuning,
    pub calendar: Vec<CalRule>,
    pub markets: Vec<MarketDef>,
    /// The pack these values came from, or empty when they are the built-in defaults.
    pub source: String,
    /// Where each club's starting data came from. A club with no entry (a save that predates this) is of unknown origin, not generated.
    pub club_origin: FxHashMap<ClubId, DataOrigin>,
}

impl Default for Scenario {
    fn default() -> Self {
        Self { recognition: RecognitionTuning::default(), scouting: ScoutingTuning::default(), calendar: default_calendar(), markets: Vec::new(), source: String::new(), club_origin: FxHashMap::default() }
    }
}

/// The calendar the simulation used before it was data.
pub fn default_calendar() -> Vec<CalRule> {
    vec![
        CalRule { event: CalEvent::DistrictSelection, months: vec![10], day: 0 },
        CalRule { event: CalEvent::UniversityScouting, months: vec![1], day: 0 },
        CalRule { event: CalEvent::NationalCamp, months: vec![5], day: 0 },
        CalRule { event: CalEvent::SchoolScouting, months: vec![8, 9, 11, 12, 2, 3, 4], day: 0 },
        CalRule { event: CalEvent::StateChampionship, months: vec![2], day: 1 },
        CalRule { event: CalEvent::UniversityReview, months: vec![7], day: 0 },
    ]
}

impl Scenario {
    /// Is `event` due on this date? A rule with a day fires on that day only; without one, throughout its months.
    pub fn due(&self, event: CalEvent, month: u32, day: u32) -> bool {
        self.calendar.iter().filter(|r| r.event == event).any(|r| r.months.contains(&(month as u8)) && (r.day == 0 || u32::from(r.day) == day))
    }

    /// The events due in a month, in the order the calendar lists them: what a monthly pass runs.
    pub fn due_in_month(&self, month: u32) -> Vec<CalEvent> {
        let mut out: Vec<CalEvent> = Vec::new();
        for r in &self.calendar {
            if r.months.contains(&(month as u8)) && !out.contains(&r.event) {
                out.push(r.event);
            }
        }
        out
    }
}
