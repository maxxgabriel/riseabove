//! Real-world reference data for India (`data/worlds/india/**`), read into typed rows that keep their provenance.
//!
//! This module is a pure loader: text in, rows and findings out. It does not touch a world. `india::build` decides what a world
//! takes from it (see that module); the rules it follows are the ones written here, in [`Prov`].
//!
//! Rules this loader keeps (design: missing is never zero; imported facts are labelled; people are never merged by name alone):
//!
//! * Every record must carry a provenance (`prov`). A record without one, or with one that does not parse, is **not loaded and is
//!   reported** as a [`Finding`]. So is a record of an unknown table, a record that does not fit its table's shape, a duplicate id,
//!   a reference to an id that does not exist, and a provenance that contradicts itself (`verified` with no source).
//! * An absent optional field stays `None`. Nothing is defaulted to zero.
//! * Fields a typed row does not read are counted in [`Reference::unread_fields`], so what is not used is visible, not silent.
//! * Tables this loader has no typed row for (media, academies, rules, ...) are still checked (provenance, ids, references) and
//!   counted in [`Reference::by_table`], but their contents are not read into anything. [`READ_TABLES`] lists what is read.
//! * Matching a reference club to a club of the world is by stable id or by exact name plus state. Never fuzzy.
//!
//! The builtin world reads the files embedded at compile time ([`builtin`]); [`load_dir`] reads a directory (for a scenario edited
//! on disk, and for the test that the embedded list is complete).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use pw_world::scenario::DataOrigin;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use toml::{Table, Value};

/// Every reference file of the builtin India world, embedded. `tests::the_embedded_list_is_every_file_in_the_folder` fails when a
/// file is added to the folder and not listed here.
pub const BUILTIN_FILES: &[(&str, &str)] = &[
    ("academies/academies.toml", include_str!("../../../data/worlds/india/academies/academies.toml")),
    ("academies/academies_b.toml", include_str!("../../../data/worlds/india/academies/academies_b.toml")),
    ("associations/associations.toml", include_str!("../../../data/worlds/india/associations/associations.toml")),
    ("associations/associations_a.toml", include_str!("../../../data/worlds/india/associations/associations_a.toml")),
    ("associations/other_organisers.toml", include_str!("../../../data/worlds/india/associations/other_organisers.toml")),
    ("associations/state_assoc_b.toml", include_str!("../../../data/worlds/india/associations/state_assoc_b.toml")),
    ("broadcasters/broadcasters.toml", include_str!("../../../data/worlds/india/broadcasters/broadcasters.toml")),
    ("broadcasters/rights_b.toml", include_str!("../../../data/worlds/india/broadcasters/rights_b.toml")),
    ("clubs/national.toml", include_str!("../../../data/worlds/india/clubs/national.toml")),
    ("clubs/tier3_b.toml", include_str!("../../../data/worlds/india/clubs/tier3_b.toml")),
    ("clubs/tier4_b.toml", include_str!("../../../data/worlds/india/clubs/tier4_b.toml")),
    ("clubs/women_b.toml", include_str!("../../../data/worlds/india/clubs/women_b.toml")),
    ("competitions/inter_university_b.toml", include_str!("../../../data/worlds/india/competitions/inter_university_b.toml")),
    ("competitions/membership.toml", include_str!("../../../data/worlds/india/competitions/membership.toml")),
    ("competitions/membership_t3_b.toml", include_str!("../../../data/worlds/india/competitions/membership_t3_b.toml")),
    ("competitions/membership_t4_b.toml", include_str!("../../../data/worlds/india/competitions/membership_t4_b.toml")),
    ("competitions/membership_women_b.toml", include_str!("../../../data/worlds/india/competitions/membership_women_b.toml")),
    ("competitions/national.toml", include_str!("../../../data/worlds/india/competitions/national.toml")),
    ("competitions/national_b.toml", include_str!("../../../data/worlds/india/competitions/national_b.toml")),
    ("culture/rivalries.toml", include_str!("../../../data/worlds/india/culture/rivalries.toml")),
    ("development/coach_education.toml", include_str!("../../../data/worlds/india/development/coach_education.toml")),
    ("development/referees.toml", include_str!("../../../data/worlds/india/development/referees.toml")),
    ("geography/districts.toml", include_str!("../../../data/worlds/india/geography/districts.toml")),
    ("geography/states.toml", include_str!("../../../data/worlds/india/geography/states.toml")),
    ("grassroots/programmes.toml", include_str!("../../../data/worlds/india/grassroots/programmes.toml")),
    ("grassroots/programmes_b.toml", include_str!("../../../data/worlds/india/grassroots/programmes_b.toml")),
    ("languages/aliases_b.toml", include_str!("../../../data/worlds/india/languages/aliases_b.toml")),
    ("languages/languages.toml", include_str!("../../../data/worlds/india/languages/languages.toml")),
    ("languages/languages_b.toml", include_str!("../../../data/worlds/india/languages/languages_b.toml")),
    ("languages/terminology.toml", include_str!("../../../data/worlds/india/languages/terminology.toml")),
    ("languages/terminology_b.toml", include_str!("../../../data/worlds/india/languages/terminology_b.toml")),
    ("media/outlets.toml", include_str!("../../../data/worlds/india/media/outlets.toml")),
    ("media/outlets_national_b.toml", include_str!("../../../data/worlds/india/media/outlets_national_b.toml")),
    ("media/outlets_regional_b.toml", include_str!("../../../data/worlds/india/media/outlets_regional_b.toml")),
    ("media/outlets_specialist_b.toml", include_str!("../../../data/worlds/india/media/outlets_specialist_b.toml")),
    ("national_teams/teams_a.toml", include_str!("../../../data/worlds/india/national_teams/teams_a.toml")),
    ("national_teams/teams_b.toml", include_str!("../../../data/worlds/india/national_teams/teams_b.toml")),
    ("partnerships/partnerships.toml", include_str!("../../../data/worlds/india/partnerships/partnerships.toml")),
    ("rules/registration.toml", include_str!("../../../data/worlds/india/rules/registration.toml")),
    ("rules/registration_a.toml", include_str!("../../../data/worlds/india/rules/registration_a.toml")),
    ("rules/rules_b.toml", include_str!("../../../data/worlds/india/rules/rules_b.toml")),
    ("schools/schools_b.toml", include_str!("../../../data/worlds/india/schools/schools_b.toml")),
    ("schools/schools_c.toml", include_str!("../../../data/worlds/india/schools/schools_c.toml")),
    ("stadiums/stadiums_b.toml", include_str!("../../../data/worlds/india/stadiums/stadiums_b.toml")),
    ("stadiums/stadiums_c.toml", include_str!("../../../data/worlds/india/stadiums/stadiums_c.toml")),
    ("state_leagues/east_b.toml", include_str!("../../../data/worlds/india/state_leagues/east_b.toml")),
    ("state_leagues/goa_b.toml", include_str!("../../../data/worlds/india/state_leagues/goa_b.toml")),
    ("state_leagues/karnataka_b.toml", include_str!("../../../data/worlds/india/state_leagues/karnataka_b.toml")),
    ("state_leagues/kerala_b.toml", include_str!("../../../data/worlds/india/state_leagues/kerala_b.toml")),
    ("state_leagues/maharashtra_b.toml", include_str!("../../../data/worlds/india/state_leagues/maharashtra_b.toml")),
    ("state_leagues/north_b.toml", include_str!("../../../data/worlds/india/state_leagues/north_b.toml")),
    ("state_leagues/northeast_b.toml", include_str!("../../../data/worlds/india/state_leagues/northeast_b.toml")),
    ("state_leagues/south_b.toml", include_str!("../../../data/worlds/india/state_leagues/south_b.toml")),
    ("state_leagues/wb.toml", include_str!("../../../data/worlds/india/state_leagues/wb.toml")),
    ("universities/universities.toml", include_str!("../../../data/worlds/india/universities/universities.toml")),
    ("universities/universities_a.toml", include_str!("../../../data/worlds/india/universities/universities_a.toml")),
    ("universities/universities_b.toml", include_str!("../../../data/worlds/india/universities/universities_b.toml")),
];

/// The tables whose records are read into typed rows. All other tables are only validated and counted.
pub const READ_TABLES: [&str; 23] = [
    "state",
    "club",
    "stadium",
    "competition",
    "membership",
    "rivalry",
    "alias",
    "association",
    "university",
    "school",
    "academy",
    "outlet",
    "broadcaster",
    "rights",
    "programme",
    "partnership",
    "team",
    "licence",
    "grade",
    "rule",
    "language",
    "term",
    "district",
];

/// Every table name the schema defines (`data/worlds/india/SCHEMA.md`). A table not in this list is reported.
const SCHEMA_TABLES: &[&str] = &[
    "state",
    "district",
    "association",
    "competition",
    "club",
    "stadium",
    "academy",
    "university",
    "school",
    "outlet",
    "broadcaster",
    "programme",
    "partnership",
    "rule",
    "team",
    "licence",
    "grade",
    "rivalry",
    "language",
    "term",
    "ownership",
    "membership",
    "rights",
    "sponsorship",
    "alias",
];

// ------------------------------------------------------------------------------------------------- provenance

/// How a record came to be, as its file states it (`SCHEMA.md`, "Every record").
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvStatus {
    Imported,
    Verified,
    Inferred,
    ScenarioSeed,
    Generated,
    Unknown,
}

impl ProvStatus {
    pub const ALL: [ProvStatus; 6] = [ProvStatus::Imported, ProvStatus::Verified, ProvStatus::Inferred, ProvStatus::ScenarioSeed, ProvStatus::Generated, ProvStatus::Unknown];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            ProvStatus::Imported => "Imported",
            ProvStatus::Verified => "Verified",
            ProvStatus::Inferred => "Inferred",
            ProvStatus::ScenarioSeed => "Scenario seed",
            ProvStatus::Generated => "Generated",
            ProvStatus::Unknown => "Unknown",
        }
    }
}

/// `A` officially verified, `B` strong secondary, `C` partial or inferred, `D` scenario seed, `E` generated.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize)]
pub enum Quality {
    A,
    B,
    C,
    D,
    E,
}

/// The provenance of one record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Prov {
    pub status: ProvStatus,
    pub q: Quality,
    /// Sources actually consulted. Empty for anything that was not looked up.
    #[serde(default)]
    pub src: Vec<String>,
    /// Free text for the debug view. Never copied into a world: it may describe real history.
    #[serde(default)]
    pub note: Option<String>,
    /// Disagreeing sources recorded by the researcher (`SCHEMA.md`). A record with conflicts never overrides a value.
    #[serde(default)]
    pub conflicts: Vec<Value>,
}

impl Prov {
    /// Checked against sources: stated as verified or imported, graded A or B, and naming at least one source.
    pub fn is_sourced_fact(&self) -> bool {
        matches!(self.status, ProvStatus::Verified | ProvStatus::Imported) && self.q <= Quality::B && !self.src.is_empty()
    }

    /// The label a club (or derby) built from this record carries in `Scenario::club_origin`.
    ///
    /// A sourced fact is `Imported`. A record the builder made is `Generated`. Everything else (inferred, a scenario seed, an entity
    /// that is real but whose facts are unknown, and a `verified` that does not hold up: no source, or graded C to E) names a real
    /// thing without being a verified record of it: `ScenarioSeed`.
    pub fn origin(&self) -> DataOrigin {
        if self.is_sourced_fact() {
            DataOrigin::Imported
        } else if self.status == ProvStatus::Generated {
            DataOrigin::Generated
        } else {
            DataOrigin::ScenarioSeed
        }
    }

    /// May the record name a real entity in a world (a club, a ground)? Not if it was itself generated.
    pub fn names_real_entity(&self) -> bool {
        self.status != ProvStatus::Generated
    }

    /// May a number in the record (capacity, founded year) replace a value the builder would have made up? Only a sourced fact, or an
    /// inference graded C or better that no source contradicts. Never a scenario seed, an unknown or a generated record.
    pub fn allows_value(&self) -> bool {
        self.conflicts.is_empty() && (self.is_sourced_fact() || (self.status == ProvStatus::Inferred && self.q <= Quality::C))
    }

    /// A provenance that contradicts itself, in words (reported as a finding; the record is still loaded, but is not an `Imported` one).
    fn inconsistency(&self) -> Option<String> {
        match self.status {
            ProvStatus::Verified | ProvStatus::Imported if self.src.is_empty() => Some(format!("status {:?} but no source is listed", self.status)),
            ProvStatus::Verified | ProvStatus::Imported if self.q > Quality::B => Some(format!("status verified but quality {:?}: verified means A or B", self.q)),
            ProvStatus::Generated if self.q != Quality::E => Some(format!("status generated but quality {:?}: generated means E", self.q)),
            _ => None,
        }
    }
}

// ------------------------------------------------------------------------------------------------- rows

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClubKind {
    Professional,
    Institutional,
    Departmental,
    Academy,
    Amateur,
    University,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClubStatus {
    Active,
    Inactive,
    Defunct,
    Suspended,
    Unknown,
}

impl ClubStatus {
    /// A club that may be put in a league at the start: playing, or real with its current status not confirmed.
    pub fn is_placeable(self) -> bool {
        matches!(self, ClubStatus::Active | ClubStatus::Unknown)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateKind {
    State,
    UnionTerritory,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    National,
    State,
    District,
    Zonal,
    Continental,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Men,
    Women,
    Mixed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompKind {
    League,
    Cup,
    Tournament,
    Championship,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompStatus {
    Active,
    Suspended,
    Defunct,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Natural,
    Artificial,
    Hybrid,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RivalryKind {
    Derby,
    Rivalry,
}

/// `[[state]]`: a state or union territory. `key` is the letters `pack.toml` uses (`WB`); the id is `state.wb`.
#[derive(Clone, Debug, Deserialize)]
pub struct StateRow {
    pub id: String,
    pub key: String,
    pub name: String,
    pub kind: StateKind,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[club]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct ClubRow {
    pub id: String,
    pub name: String,
    pub city: String,
    /// A state id.
    pub state: String,
    pub kind: ClubKind,
    pub status: ClubStatus,
    #[serde(default)]
    pub short: Option<String>,
    /// Founded year. Absent is unknown, never zero.
    #[serde(default)]
    pub founded: Option<i32>,
    /// A stadium id.
    #[serde(default)]
    pub home_ground: Option<String>,
    /// A ground named as text, unverified (not in the schema; some files use it while no stadium record exists).
    #[serde(default)]
    pub home_ground_name: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[stadium]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct StadiumRow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub city: String,
    pub state: String,
    /// Absent is unknown, never zero.
    #[serde(default)]
    pub capacity: Option<u32>,
    /// The year or date the capacity describes. A capacity without it is a finding.
    #[serde(default)]
    pub capacity_as_of: Option<Value>,
    #[serde(default)]
    pub surface: Option<Surface>,
    #[serde(default)]
    pub home_clubs: Vec<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[competition]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct CompRow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub short: Option<String>,
    pub organiser: String,
    pub level: Level,
    #[serde(default)]
    pub tier: Option<u8>,
    #[serde(default)]
    pub state: Option<String>,
    /// `senior`, `u23`, `u19` ...
    pub age: String,
    pub gender: Gender,
    pub kind: CompKind,
    pub season: String,
    pub status: CompStatus,
    #[serde(default)]
    pub teams: Option<u32>,
    #[serde(default)]
    pub qualifies_to: Vec<String>,
    #[serde(default)]
    pub fed_by: Vec<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[membership]]`: a club in a competition in a season.
#[derive(Clone, Debug, Deserialize)]
pub struct MembershipRow {
    /// A club id (or an association id for a representative side).
    pub club: String,
    pub competition: String,
    /// `2026-27`, or `2026` for a calendar-year competition.
    pub season: String,
    #[serde(default)]
    pub stage: Option<String>,
    /// The club also plays in a national competition (not a second club).
    #[serde(default)]
    pub concurrent_national: bool,
    /// The record is a reserve side's entry.
    #[serde(default)]
    pub reserve: bool,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[rivalry]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct RivalryRow {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub a: String,
    pub b: String,
    pub kind: RivalryKind,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[alias]]`. Only the derby names (`kind = "derby_name"`, with an `entity_pair`) are used; other aliases are read and counted.
#[derive(Clone, Debug, Deserialize)]
pub struct AliasRow {
    pub id: String,
    #[serde(default)]
    pub entity: Option<String>,
    #[serde(default)]
    pub entity_pair: Vec<String>,
    pub alias: String,
    pub kind: String,
    #[serde(default)]
    pub lang: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

// ------------------------------------------------------------------------------------------------- identity tables
// Rows of the tables that give the world names and labels (institutions, the press, programmes, ladders). Enumerated fields are kept
// as the text the file writes: a value the schema does not list is carried through as words, not rejected.

/// `[[association]]`: a federation, a state or district association, or another organiser.
#[derive(Clone, Debug, Deserialize)]
pub struct AssociationRow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub abbr: Option<String>,
    /// `national`, `state`, `institutional`, `district`; absent when the file does not say.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub hq_city: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[university]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct UniversityRow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub short: Option<String>,
    pub city: String,
    pub state: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub zone: Option<String>,
    #[serde(default)]
    pub residential: Option<bool>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[school]]`: a school, sports school, sports hostel or SAI centre.
#[derive(Clone, Debug, Deserialize)]
pub struct SchoolRow {
    pub id: String,
    pub name: String,
    pub city: String,
    pub state: String,
    /// `school`, `sports_school`, `sports_hostel`, `sai_centre`, `military`, `academy_school`.
    pub kind: String,
    #[serde(default)]
    pub residential: Option<bool>,
    #[serde(default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub programme: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[academy]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct AcademyRow {
    pub id: String,
    pub name: String,
    /// Absent for a programme without one home (a national elite academy that moves between centres).
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    /// A club id, when the academy belongs to a club.
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub parent_name: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub residential: Option<bool>,
    #[serde(default)]
    pub age_groups: Vec<String>,
    #[serde(default)]
    pub status: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[outlet]]`: a newspaper, channel or site. The file carries no credibility or quality: the simulation grows its own.
#[derive(Clone, Debug, Deserialize)]
pub struct OutletRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub medium: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    /// `national`, `multi_state`, `state`, `local`.
    pub reach: String,
    #[serde(default)]
    pub home_state: Option<String>,
    #[serde(default)]
    pub home_city: Option<String>,
    #[serde(default)]
    pub focus: Vec<String>,
    /// `specialist`, `strong`, `general`.
    #[serde(default)]
    pub football_emphasis: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
    /// The club an official club channel belongs to.
    #[serde(default)]
    pub club: Option<String>,
    /// `large`, `medium`, `small`, `niche`: only where the researcher found it defensible.
    #[serde(default)]
    pub audience: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[broadcaster]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct BroadcasterRow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub medium: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub region: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[rights]]`: who shows a competition in a season.
#[derive(Clone, Debug, Deserialize)]
pub struct RightsRow {
    pub broadcaster: String,
    /// A competition id, or words for a foreign competition.
    pub competition: String,
    pub season: String,
    #[serde(default)]
    pub languages: Vec<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[programme]]`: a grassroots, talent-identification or development programme.
#[derive(Clone, Debug, Deserialize)]
pub struct ProgrammeRow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub operator: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub ages: Option<String>,
    /// `national`, or state ids (one or several).
    #[serde(default)]
    pub region: Option<Value>,
    #[serde(default)]
    pub description: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// A party abroad in a partnership, written inline.
#[derive(Clone, Debug, Deserialize)]
pub struct ForeignParty {
    pub name: String,
    pub nation: String,
    #[serde(default)]
    pub kind: Option<String>,
}

/// `[[partnership]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct PartnershipRow {
    pub id: String,
    #[serde(default)]
    pub indian: Vec<String>,
    #[serde(default)]
    pub foreign: Vec<ForeignParty>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub components: Vec<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[team]]`: a national or representative side.
#[derive(Clone, Debug, Deserialize)]
pub struct TeamRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub gender: String,
    pub age: String,
    #[serde(default)]
    pub association: Option<String>,
    #[serde(default)]
    pub competitions: Vec<String>,
    #[serde(default)]
    pub eligibility: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[licence]]`: one step of the coaching ladder.
#[derive(Clone, Debug, Deserialize)]
pub struct LicenceRow {
    pub id: String,
    pub name: String,
    pub body: String,
    pub order: u8,
    #[serde(default)]
    pub prerequisite: Option<String>,
    #[serde(default)]
    pub requirement: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[grade]]`: one step of the referees' ladder.
#[derive(Clone, Debug, Deserialize)]
pub struct GradeRow {
    pub id: String,
    pub name: String,
    pub body: String,
    pub order: u8,
    /// `district`, `state`, `national`, `international`.
    pub scope: String,
    #[serde(default)]
    pub requirement: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[rule]]`: a rule in words (and its numbers, when the file gives them).
#[derive(Clone, Debug, Deserialize)]
pub struct RuleRow {
    pub id: String,
    pub topic: String,
    pub statement: String,
    /// The competitions or bodies it applies to.
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub competition: Option<String>,
    #[serde(default)]
    pub season: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[language]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct LanguageRow {
    pub id: String,
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub script: Option<String>,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[term]]`: a football word in one language and register.
#[derive(Clone, Debug, Deserialize)]
pub struct TermRow {
    pub id: String,
    pub concept: String,
    pub canonical: String,
    #[serde(default)]
    pub synonyms: Vec<String>,
    pub register: String,
    #[serde(default)]
    pub region: Option<Value>,
    pub lang: String,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// `[[district]]`.
#[derive(Clone, Debug, Deserialize)]
pub struct DistrictRow {
    pub id: String,
    pub state: String,
    pub name: String,
    pub prov: Prov,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// A table with no typed row: only its id and provenance are read.
#[derive(Deserialize)]
struct Stub {
    #[serde(default)]
    id: Option<String>,
    prov: Prov,
}

/// One problem found while loading. Every one is also a record (or a file) the simulation did not get, or got flagged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub file: String,
    pub table: String,
    pub id: String,
    pub problem: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.table.is_empty(), self.id.is_empty()) {
            (true, _) => write!(f, "{}: {}", self.file, self.problem),
            (false, true) => write!(f, "{}: {}: {}", self.file, self.table, self.problem),
            (false, false) => write!(f, "{}: {} {}: {}", self.file, self.table, self.id, self.problem),
        }
    }
}

/// A named derby or rivalry between two clubs, from `culture/rivalries.toml` or a `derby_name` alias. A label for the news and the UI.
/// It carries no intensity and no history: rivalries in the simulation come from what happens in it.
#[derive(Clone, Debug)]
pub struct DerbyRef<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub a: &'a str,
    pub b: &'a str,
    /// `derby` (a local fixture) as against `rivalry`.
    pub is_derby: bool,
    pub prov: &'a Prov,
}

// ------------------------------------------------------------------------------------------------- the loaded reference

/// Everything loaded from a set of reference files.
#[derive(Default, Debug)]
pub struct Reference {
    pub files: usize,
    pub states: Vec<StateRow>,
    pub clubs: Vec<ClubRow>,
    pub stadiums: Vec<StadiumRow>,
    pub competitions: Vec<CompRow>,
    pub memberships: Vec<MembershipRow>,
    pub rivalries: Vec<RivalryRow>,
    pub aliases: Vec<AliasRow>,
    pub associations: Vec<AssociationRow>,
    pub universities: Vec<UniversityRow>,
    pub schools: Vec<SchoolRow>,
    pub academies: Vec<AcademyRow>,
    pub outlets: Vec<OutletRow>,
    pub broadcasters: Vec<BroadcasterRow>,
    pub rights: Vec<RightsRow>,
    pub programmes: Vec<ProgrammeRow>,
    pub partnerships: Vec<PartnershipRow>,
    pub teams: Vec<TeamRow>,
    pub licences: Vec<LicenceRow>,
    pub grades: Vec<GradeRow>,
    pub rules: Vec<RuleRow>,
    pub languages: Vec<LanguageRow>,
    pub terms: Vec<TermRow>,
    pub districts: Vec<DistrictRow>,
    /// Records loaded (with a valid provenance), by table, including tables this loader does not read into rows.
    pub by_table: BTreeMap<String, u32>,
    /// Records loaded, by provenance status, in [`ProvStatus::ALL`] order.
    pub by_status: [u32; 6],
    pub findings: Vec<Finding>,
    /// Fields present in records of a typed table that no row reads, as `table.field` with how many records carry them.
    pub unread_fields: BTreeMap<String, u32>,
    club_by_id: HashMap<String, usize>,
    club_by_name: HashMap<(String, String), Vec<usize>>,
    stadium_by_id: HashMap<String, usize>,
    state_by_id: HashMap<String, usize>,
}

impl Reference {
    /// Records loaded in all tables.
    pub fn records(&self) -> u32 {
        self.by_table.values().sum()
    }

    /// Tables that have records but are not read into rows: what this loader does not use yet.
    pub fn unread_tables(&self) -> Vec<(&str, u32)> {
        self.by_table.iter().filter(|(t, _)| !READ_TABLES.contains(&t.as_str())).map(|(t, n)| (t.as_str(), *n)).collect()
    }

    pub fn state(&self, id: &str) -> Option<&StateRow> {
        self.state_by_id.get(id).map(|&i| &self.states[i])
    }

    /// The id of the state `pack.toml` writes as `key` (`WB` is `state.wb`), if the reference has it.
    pub fn state_of_key(&self, key: &str) -> Option<&StateRow> {
        self.state(&format!("state.{}", key.to_ascii_lowercase()))
    }

    pub fn club(&self, id: &str) -> Option<&ClubRow> {
        self.club_by_id.get(id).map(|&i| &self.clubs[i])
    }

    /// The club with exactly this name in this state (a state id). `None` if there is none or more than one: never a guess.
    pub fn club_exact(&self, name: &str, state_id: &str) -> Option<&ClubRow> {
        match self.club_by_name.get(&(name.to_string(), state_id.to_string()))?.as_slice() {
            [one] => Some(&self.clubs[*one]),
            _ => None,
        }
    }

    /// Does the reference have any club (one or several) with exactly this name in this state? A name the builder makes up must not
    /// be one, or a generated club would carry a real club's name.
    pub fn names_club(&self, name: &str, state_id: &str) -> bool {
        self.club_by_name.contains_key(&(name.to_string(), state_id.to_string()))
    }

    pub fn stadium(&self, id: &str) -> Option<&StadiumRow> {
        self.stadium_by_id.get(id).map(|&i| &self.stadiums[i])
    }

    /// The ground a club plays at, by stable reference only: the id in its `home_ground`; else the one stadium that lists the club in
    /// `home_clubs`; else a stadium of its state whose name or alias is exactly the club's `home_ground_name`. Two candidates at the
    /// same step is no answer.
    pub fn stadium_of(&self, club: &ClubRow) -> Option<&StadiumRow> {
        if let Some(id) = &club.home_ground {
            return self.stadium(id);
        }
        let listed: Vec<&StadiumRow> = self.stadiums.iter().filter(|s| s.home_clubs.contains(&club.id)).collect();
        match listed.as_slice() {
            [one] => return Some(one),
            [] => {}
            _ => return None,
        }
        let name = club.home_ground_name.as_deref()?;
        let named: Vec<&StadiumRow> = self.stadiums.iter().filter(|s| s.state == club.state && (s.name == name || s.aliases.iter().any(|a| a == name))).collect();
        match named.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }

    /// The national senior men's league at a place of the pyramid (1 = top), when the reference has exactly one.
    pub fn national_league(&self, tier: u8) -> Option<&CompRow> {
        let mut v = self
            .competitions
            .iter()
            .filter(|c| c.level == Level::National && c.tier == Some(tier) && c.gender == Gender::Men && c.age == "senior" && c.kind == CompKind::League && c.status != CompStatus::Defunct);
        let first = v.next()?;
        v.next().is_none().then_some(first)
    }

    /// The clubs entered in a competition in the season that begins in `year` (`2026-27` and `2026` both match 2026), in the order the
    /// files list them, each once, without reserve sides. Only clubs a world may use: a known club, placeable, not itself generated,
    /// in a membership that is not generated. A club of another state than the competition's is included (clubs do cross states);
    /// callers that need the same state filter on it.
    pub fn members(&self, comp: &str, year: i32) -> Vec<&ClubRow> {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut out = Vec::new();
        for m in self.memberships.iter().filter(|m| m.competition == comp && !m.reserve && season_starts(&m.season, year) && m.prov.names_real_entity()) {
            let Some(c) = self.club(&m.club) else { continue };
            if c.status.is_placeable() && c.prov.names_real_entity() && seen.insert(c.id.as_str()) {
                out.push(c);
            }
        }
        out
    }

    /// The competition that is a state's top men's league for the season that begins in `year`: among its senior men's state leagues of
    /// tier 1, the one with the most placeable member clubs of that state (ties: the lower id). `None` if no league has a member.
    pub fn state_top_league(&self, state_id: &str, year: i32) -> Option<&CompRow> {
        let mut best: Option<(usize, &CompRow)> = None;
        for c in self.competitions.iter().filter(|c| {
            c.level == Level::State
                && c.state.as_deref() == Some(state_id)
                && c.tier == Some(1)
                && c.gender == Gender::Men
                && c.age == "senior"
                && c.kind == CompKind::League
                && c.status != CompStatus::Defunct
        }) {
            let n = self.members(&c.id, year).iter().filter(|m| m.state == state_id).count();
            if n == 0 {
                continue;
            }
            if best.is_none_or(|(bn, bc)| n > bn || (n == bn && c.id < bc.id)) {
                best = Some((n, c));
            }
        }
        best.map(|b| b.1)
    }

    /// The named derbies and rivalries between clubs: the rivalry records first, then derby names that connect a pair of clubs not
    /// already named. Each pair once.
    pub fn derbies(&self) -> Vec<DerbyRef<'_>> {
        let rivalries = self.rivalries.iter().map(|r| DerbyRef { id: &r.id, name: r.name.as_deref().unwrap_or(&r.id), a: &r.a, b: &r.b, is_derby: r.kind == RivalryKind::Derby, prov: &r.prov });
        let derby_names = self.aliases.iter().filter(|a| a.kind == "derby_name").filter_map(|a| match a.entity_pair.as_slice() {
            [x, y] => Some(DerbyRef { id: &a.id, name: &a.alias, a: x, b: y, is_derby: true, prov: &a.prov }),
            _ => None,
        });
        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        let mut out: Vec<DerbyRef<'_>> = Vec::new();
        for d in rivalries.chain(derby_names) {
            let pair = if d.a <= d.b { (d.a, d.b) } else { (d.b, d.a) };
            if d.a.starts_with("club.") && d.b.starts_with("club.") && seen.insert(pair) {
                out.push(d);
            }
        }
        out
    }
}

/// Does a season label cover the season that begins in `year`: `2026-27`, `2026`, or a range `2023-24 to 2027-28`?
pub fn season_covers(season: &str, year: i32) -> bool {
    match season.split_once(" to ") {
        Some((a, b)) => {
            let first = |s: &str| s.trim().get(..4).and_then(|y| y.parse::<i32>().ok());
            matches!((first(a), first(b)), (Some(x), Some(y)) if x <= year && year <= y)
        }
        None => season_starts(season, year),
    }
}

/// Does a season label (`2026-27`, `2026`) begin in this year?
pub fn season_starts(season: &str, year: i32) -> bool {
    let y = year.to_string();
    season == y || season.strip_prefix(&y).is_some_and(|rest| rest.starts_with('-'))
}

// ------------------------------------------------------------------------------------------------- loading

/// The builtin world's reference data, parsed once per process.
pub fn builtin() -> &'static Reference {
    static REF: OnceLock<Reference> = OnceLock::new();
    REF.get_or_init(|| load_sources(BUILTIN_FILES))
}

/// Read every reference file under `root` (a `data/worlds/<nation>` folder), as the builtin list would embed them.
pub fn load_dir(root: &Path) -> std::io::Result<Reference> {
    let mut files: Vec<(String, String)> = Vec::new();
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) -> std::io::Result<()> {
        for e in std::fs::read_dir(dir)? {
            let p = e?.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "_manifest") {
                    continue;
                }
                walk(&p, root, out)?;
            } else if p.extension().is_some_and(|x| x == "toml") && p.file_name().is_some_and(|n| n != "pack.toml") {
                let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                out.push((rel, std::fs::read_to_string(&p)?));
            }
        }
        Ok(())
    }
    walk(root, root, &mut files)?;
    let borrowed: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    Ok(load_sources(&borrowed))
}

/// Parse and check a set of `(path, text)` files. Never panics on bad data: every problem becomes a [`Finding`]. The result does not
/// depend on the order the files are given in.
pub fn load_sources(files: &[(&str, &str)]) -> Reference {
    let mut sorted: Vec<&(&str, &str)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let mut r = Reference { files: sorted.len(), ..Default::default() };
    // id -> (table, file), for duplicate and reference checks across every table.
    let mut ids: HashMap<String, (String, String)> = HashMap::new();
    let mut checks: Vec<Check> = Vec::new();

    for (path, text) in sorted {
        let file = (*path).to_string();
        let table: Table = match text.parse() {
            Ok(t) => t,
            Err(e) => {
                r.findings.push(Finding { file, table: String::new(), id: String::new(), problem: format!("does not parse as TOML: {}", e.message()) });
                continue;
            }
        };
        match table.get("meta") {
            Some(Value::Table(m)) if m.get("dataset").is_some_and(Value::is_str) => {}
            Some(_) => r.findings.push(Finding { file: file.clone(), table: "meta".into(), id: String::new(), problem: "[meta] has no dataset name".into() }),
            None => r.findings.push(Finding { file: file.clone(), table: String::new(), id: String::new(), problem: "missing [meta] table".into() }),
        }
        for (name, value) in &table {
            if name == "meta" {
                continue;
            }
            if !SCHEMA_TABLES.contains(&name.as_str()) {
                let n = value.as_array().map_or(1, Vec::len);
                r.findings.push(Finding { file: file.clone(), table: name.clone(), id: String::new(), problem: format!("unknown table (not in SCHEMA.md): {n} record(s) not loaded") });
                continue;
            }
            let Some(rows) = value.as_array() else {
                r.findings.push(Finding { file: file.clone(), table: name.clone(), id: String::new(), problem: "is not an array of records".into() });
                continue;
            };
            for row in rows {
                let Some(rec) = row.as_table() else {
                    r.findings.push(Finding { file: file.clone(), table: name.clone(), id: String::new(), problem: "a record is not a table".into() });
                    continue;
                };
                read_record(&mut r, &mut ids, &mut checks, &file, name, rec);
            }
        }
    }
    r.index();
    r.check_references(&ids, &checks);
    r
}

const STATE: &[&str] = &["state."];
const STADIUM: &[&str] = &["stadium."];
const CLUB: &[&str] = &["club."];
const ASSOC: &[&str] = &["assoc."];
const COMP: &[&str] = &["comp."];
const CLUB_OR_ASSOC: &[&str] = &["club.", "assoc."];
const CLUB_OR_STATE: &[&str] = &["club.", "state."];
const ANY: &[&str] = &[""];
const LANG: &[&str] = &["lang."];
const BCAST: &[&str] = &["bcast."];
const LICENCE: &[&str] = &["licence."];

/// What to check about a record once every id is known.
struct Check {
    file: String,
    table: &'static str,
    id: String,
    /// `(what the field is, the referenced id, the prefixes the referenced id may have)`.
    refs: Vec<(&'static str, String, &'static [&'static str])>,
}

fn read_record(r: &mut Reference, ids: &mut HashMap<String, (String, String)>, checks: &mut Vec<Check>, file: &str, table: &str, rec: &Table) {
    let table_static: &'static str = SCHEMA_TABLES.iter().copied().find(|t| *t == table).unwrap_or("");
    let label = rec.get("id").and_then(Value::as_str).map(str::to_string).or_else(|| rec.get("club").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
    let find = |problem: String| Finding { file: file.to_string(), table: table.to_string(), id: label.clone(), problem };
    if !rec.contains_key("prov") {
        r.findings.push(find("has no prov: not loaded".into()));
        return;
    }
    if let (Some(a), Some(b)) = (rec.get("effective_from"), rec.get("effective_to")) {
        if a.to_string() > b.to_string() {
            r.findings.push(find("effective_from is after effective_to".into()));
        }
    }
    // Typed rows: parse, then register. Each arm yields (prov, id, checks) for the shared bookkeeping below.
    macro_rules! typed {
        ($ty:ty) => {{
            match parse::<$ty>(rec) {
                Ok(row) => {
                    for k in row.extra.keys() {
                        *r.unread_fields.entry(format!("{table}.{k}")).or_default() += 1;
                    }
                    Some(row)
                }
                Err(e) => {
                    r.findings.push(find(format!("malformed, not loaded: {e}")));
                    None
                }
            }
        }};
    }
    let (prov, id, refs): (Prov, Option<String>, Vec<(&'static str, String, &'static [&'static str])>) = match table {
        "state" => match typed!(StateRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![]);
                r.states.push(row);
                out
            }
            None => return,
        },
        "club" => match typed!(ClubRow) {
            Some(row) => {
                let mut refs = vec![("state", row.state.clone(), STATE)];
                if let Some(g) = &row.home_ground {
                    refs.push(("home_ground", g.clone(), STADIUM));
                }
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.clubs.push(row);
                out
            }
            None => return,
        },
        "stadium" => match typed!(StadiumRow) {
            Some(row) => {
                if row.capacity.is_some() && row.capacity_as_of.is_none() {
                    r.findings.push(find("has a capacity but no capacity_as_of".into()));
                }
                let mut refs = vec![("state", row.state.clone(), STATE)];
                refs.extend(row.home_clubs.iter().map(|c| ("home_clubs", c.clone(), CLUB)));
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.stadiums.push(row);
                out
            }
            None => return,
        },
        "competition" => match typed!(CompRow) {
            Some(row) => {
                let mut refs = vec![("organiser", row.organiser.clone(), ASSOC)];
                if let Some(s) = &row.state {
                    refs.push(("state", s.clone(), STATE));
                }
                refs.extend(row.qualifies_to.iter().map(|c| ("qualifies_to", c.clone(), COMP)));
                refs.extend(row.fed_by.iter().map(|c| ("fed_by", c.clone(), COMP)));
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.competitions.push(row);
                out
            }
            None => return,
        },
        "membership" => match typed!(MembershipRow) {
            Some(row) => {
                let refs = vec![("club", row.club.clone(), CLUB_OR_ASSOC), ("competition", row.competition.clone(), COMP)];
                let out = (row.prov.clone(), None, refs);
                r.memberships.push(row);
                out
            }
            None => return,
        },
        "rivalry" => match typed!(RivalryRow) {
            Some(row) => {
                let refs = vec![("a", row.a.clone(), CLUB_OR_STATE), ("b", row.b.clone(), CLUB_OR_STATE)];
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.rivalries.push(row);
                out
            }
            None => return,
        },
        "alias" => match typed!(AliasRow) {
            Some(row) => {
                if row.entity.is_none() && row.entity_pair.is_empty() {
                    r.findings.push(find("has neither entity nor entity_pair".into()));
                }
                let mut refs: Vec<(&'static str, String, &'static [&'static str])> = Vec::new();
                refs.extend(row.entity.iter().map(|e| ("entity", e.clone(), ANY)));
                refs.extend(row.entity_pair.iter().map(|e| ("entity_pair", e.clone(), ANY)));
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.aliases.push(row);
                out
            }
            None => return,
        },
        "association" => match typed!(AssociationRow) {
            Some(row) => {
                let mut refs: Vec<(&'static str, String, &'static [&'static str])> = Vec::new();
                refs.extend(row.state.iter().map(|x| ("state", x.clone(), STATE)));
                refs.extend(row.parent.iter().map(|x| ("parent", x.clone(), ASSOC)));
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.associations.push(row);
                out
            }
            None => return,
        },
        "university" => match typed!(UniversityRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![("state", row.state.clone(), STATE)]);
                r.universities.push(row);
                out
            }
            None => return,
        },
        "school" => match typed!(SchoolRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![("state", row.state.clone(), STATE)]);
                r.schools.push(row);
                out
            }
            None => return,
        },
        "academy" => match typed!(AcademyRow) {
            Some(row) => {
                let mut refs: Vec<(&'static str, String, &'static [&'static str])> = row.state.iter().map(|x| ("state", x.clone(), STATE)).collect();
                refs.extend(row.parent.iter().map(|x| ("parent", x.clone(), CLUB)));
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.academies.push(row);
                out
            }
            None => return,
        },
        "outlet" => match typed!(OutletRow) {
            Some(row) => {
                let mut refs: Vec<(&'static str, String, &'static [&'static str])> = row.home_state.iter().map(|x| ("home_state", x.clone(), STATE)).collect();
                refs.extend(row.languages.iter().map(|x| ("languages", x.clone(), LANG)));
                refs.extend(row.club.iter().map(|x| ("club", x.clone(), CLUB)));
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.outlets.push(row);
                out
            }
            None => return,
        },
        "broadcaster" => match typed!(BroadcasterRow) {
            Some(row) => {
                let refs = row.languages.iter().map(|x| ("languages", x.clone(), LANG)).collect();
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.broadcasters.push(row);
                out
            }
            None => return,
        },
        "rights" => match typed!(RightsRow) {
            Some(row) => {
                let mut refs = vec![("broadcaster", row.broadcaster.clone(), BCAST)];
                if row.competition.starts_with("comp.") {
                    refs.push(("competition", row.competition.clone(), COMP));
                }
                let out = (row.prov.clone(), None, refs);
                r.rights.push(row);
                out
            }
            None => return,
        },
        "programme" => match typed!(ProgrammeRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![]);
                r.programmes.push(row);
                out
            }
            None => return,
        },
        "partnership" => match typed!(PartnershipRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![]);
                r.partnerships.push(row);
                out
            }
            None => return,
        },
        "team" => match typed!(TeamRow) {
            Some(row) => {
                let refs = row.association.iter().map(|x| ("association", x.clone(), ASSOC)).collect();
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.teams.push(row);
                out
            }
            None => return,
        },
        "licence" => match typed!(LicenceRow) {
            Some(row) => {
                let refs = row.prerequisite.iter().map(|x| ("prerequisite", x.clone(), LICENCE)).collect();
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.licences.push(row);
                out
            }
            None => return,
        },
        "grade" => match typed!(GradeRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![]);
                r.grades.push(row);
                out
            }
            None => return,
        },
        "rule" => match typed!(RuleRow) {
            Some(row) => {
                let refs = row.competition.iter().map(|x| ("competition", x.clone(), COMP)).collect();
                let out = (row.prov.clone(), Some(row.id.clone()), refs);
                r.rules.push(row);
                out
            }
            None => return,
        },
        "language" => match typed!(LanguageRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![]);
                r.languages.push(row);
                out
            }
            None => return,
        },
        "term" => match typed!(TermRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![("lang", row.lang.clone(), LANG)]);
                r.terms.push(row);
                out
            }
            None => return,
        },
        "district" => match typed!(DistrictRow) {
            Some(row) => {
                let out = (row.prov.clone(), Some(row.id.clone()), vec![("state", row.state.clone(), STATE)]);
                r.districts.push(row);
                out
            }
            None => return,
        },
        _ => match parse::<Stub>(rec) {
            Ok(s) => (s.prov, s.id, vec![]),
            Err(e) => {
                r.findings.push(find(format!("malformed, not loaded: {e}")));
                return;
            }
        },
    };
    if let Some(p) = prov.inconsistency() {
        r.findings.push(find(format!("provenance contradicts itself: {p} (it is not treated as a verified record)")));
    }
    r.by_status[prov.status.index()] += 1;
    *r.by_table.entry(table.to_string()).or_default() += 1;
    if let Some(id) = id {
        if let Some((t, f)) = ids.get(&id) {
            r.findings.push(find(format!("duplicate id, also in {f} ({t})")));
        } else {
            ids.insert(id.clone(), (table.to_string(), file.to_string()));
        }
    }
    if !refs.is_empty() {
        checks.push(Check { file: file.to_string(), table: table_static, id: label.clone(), refs });
    }
}

fn parse<T: DeserializeOwned>(rec: &Table) -> Result<T, String> {
    Value::Table(rec.clone()).try_into::<T>().map_err(|e| e.message().to_string())
}

impl Reference {
    fn index(&mut self) {
        for (i, s) in self.states.iter().enumerate() {
            self.state_by_id.entry(s.id.clone()).or_insert(i);
        }
        for (i, c) in self.clubs.iter().enumerate() {
            self.club_by_id.entry(c.id.clone()).or_insert(i);
            self.club_by_name.entry((c.name.clone(), c.state.clone())).or_default().push(i);
        }
        for (i, s) in self.stadiums.iter().enumerate() {
            self.stadium_by_id.entry(s.id.clone()).or_insert(i);
        }
    }

    /// Every id a record points at must be the id of a record that was loaded, and of the right kind.
    fn check_references(&mut self, ids: &HashMap<String, (String, String)>, checks: &[Check]) {
        for c in checks {
            for (what, target, prefixes) in &c.refs {
                let kind_ok = prefixes.iter().any(|p| target.starts_with(p));
                let problem = if !ids.contains_key(target) {
                    Some(format!("{what} points at {target}, which is not a loaded record"))
                } else if !kind_ok {
                    Some(format!("{what} points at {target}, which is not a {} record", prefixes.iter().map(|p| p.trim_end_matches('.')).collect::<Vec<_>>().join(" or ")))
                } else {
                    None
                };
                if let Some(problem) = problem {
                    self.findings.push(Finding { file: c.file.clone(), table: c.table.to_string(), id: c.id.clone(), problem });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
[meta]
dataset = "test.good"

[[state]]
id = "state.wb"
key = "WB"
name = "West Bengal"
kind = "state"
prov = { status = "inferred", q = "C", src = [], note = "x" }

[[state]]
id = "state.kl"
key = "KL"
name = "Kerala"
kind = "state"
prov = { status = "inferred", q = "C", src = [] }

[[association]]
id = "assoc.ifa"
name = "IFA"
prov = { status = "inferred", q = "C", src = [] }

[[ownership]]
club = "club.a"
owner = "A holding company"
kind = "corporate"
prov = { status = "inferred", q = "C", src = [] }

[[stadium]]
id = "stadium.vyb"
name = "Salt Lake Stadium"
aliases = ["VYBK"]
city = "Kolkata"
state = "state.wb"
capacity = 68000
capacity_as_of = 2023
home_clubs = ["club.a"]
ground_type = "stadium"
prov = { status = "inferred", q = "C", src = [] }

[[stadium]]
id = "stadium.nocap"
name = "Maidan Ground"
city = "Kolkata"
state = "state.wb"
prov = { status = "scenario_seed", q = "D", src = [] }

[[club]]
id = "club.a"
name = "Alpha FC"
city = "Kolkata"
state = "state.wb"
kind = "professional"
status = "active"
founded = 1921
prov = { status = "verified", q = "B", src = ["https://example.org/a"] }

[[club]]
id = "club.b"
name = "Beta SC"
city = "Kolkata"
state = "state.wb"
kind = "amateur"
status = "unknown"
prov = { status = "unknown", q = "C", src = [] }

[[club]]
id = "club.gone"
name = "Gone United"
city = "Kolkata"
state = "state.wb"
kind = "amateur"
status = "defunct"
prov = { status = "inferred", q = "C", src = [] }

[[competition]]
id = "comp.wb.top"
name = "WB Top"
organiser = "assoc.ifa"
level = "state"
tier = 1
state = "state.wb"
age = "senior"
gender = "men"
kind = "league"
season = "2026"
status = "active"
prov = { status = "verified", q = "B", src = ["https://example.org/c"] }

[[membership]]
club = "club.a"
competition = "comp.wb.top"
season = "2026"
prov = { status = "verified", q = "B", src = ["https://example.org/m"] }

[[membership]]
club = "club.b"
competition = "comp.wb.top"
season = "2026"
prov = { status = "inferred", q = "C", src = [] }

[[membership]]
club = "club.gone"
competition = "comp.wb.top"
season = "2026"
prov = { status = "inferred", q = "C", src = [] }

[[membership]]
club = "club.a"
competition = "comp.wb.top"
season = "2025"
prov = { status = "verified", q = "B", src = ["https://example.org/m"] }

[[rivalry]]
id = "rivalry.ab"
name = "The Derby"
a = "club.a"
b = "club.b"
kind = "derby"
prov = { status = "inferred", q = "C", src = [] }

[[alias]]
id = "alias.ab.01"
entity_pair = ["club.b", "club.a"]
alias = "Same pair again"
kind = "derby_name"
prov = { status = "inferred", q = "C", src = [] }
"#;

    fn load(text: &str) -> Reference {
        load_sources(&[("good.toml", text)])
    }

    fn problems(r: &Reference) -> Vec<String> {
        r.findings.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_good_file_loads_with_no_findings() {
        let r = load(GOOD);
        assert_eq!(problems(&r), Vec::<String>::new());
        assert_eq!((r.states.len(), r.clubs.len(), r.stadiums.len(), r.competitions.len(), r.memberships.len()), (2, 3, 2, 1, 4));
        assert_eq!((r.by_table["association"], r.associations.len()), (1, 1), "the association is read into a row");
        assert_eq!(r.by_table["ownership"], 1, "a table with no typed row is counted");
        assert_eq!(r.unread_tables(), vec![("ownership", 1)]);
        assert_eq!(r.club("club.a").unwrap().founded, Some(1921));
        assert_eq!(r.club("club.b").unwrap().founded, None, "absent is None, never zero");
        assert_eq!(r.stadium("stadium.nocap").unwrap().capacity, None);
        assert_eq!(r.unread_fields.get("stadium.ground_type"), Some(&1), "a field nothing reads is counted, not dropped silently");
        assert_eq!(r.by_status.iter().sum::<u32>(), r.records());
    }

    #[test]
    fn provenance_maps_to_a_data_origin_and_a_fact_is_only_what_is_sourced() {
        let r = load(GOOD);
        let a = &r.club("club.a").unwrap().prov;
        assert_eq!((a.origin(), a.allows_value()), (DataOrigin::Imported, true));
        let b = &r.club("club.b").unwrap().prov;
        assert_eq!((b.origin(), b.allows_value(), b.names_real_entity()), (DataOrigin::ScenarioSeed, false, true), "a real club whose facts are unknown is a seed");
        let cap = &r.stadium("stadium.vyb").unwrap().prov;
        assert_eq!((cap.origin(), cap.allows_value()), (DataOrigin::ScenarioSeed, true), "an inference graded C may give a number, but it is not an import");
        let seed = &r.stadium("stadium.nocap").unwrap().prov;
        assert_eq!((seed.origin(), seed.allows_value()), (DataOrigin::ScenarioSeed, false), "a scenario seed never overrides a value");
        let p = |status, q, src: &[&str]| Prov { status, q, src: src.iter().map(|s| s.to_string()).collect(), note: None, conflicts: vec![] };
        assert_eq!(p(ProvStatus::Verified, Quality::A, &["u"]).origin(), DataOrigin::Imported);
        assert_eq!(p(ProvStatus::Imported, Quality::B, &["u"]).origin(), DataOrigin::Imported);
        assert_eq!(p(ProvStatus::Verified, Quality::B, &[]).origin(), DataOrigin::ScenarioSeed, "verified with no source is not an import");
        assert_eq!(p(ProvStatus::Verified, Quality::C, &["u"]).origin(), DataOrigin::ScenarioSeed, "verified graded C is not an import");
        assert_eq!(p(ProvStatus::Inferred, Quality::C, &[]).origin(), DataOrigin::ScenarioSeed);
        assert_eq!(p(ProvStatus::ScenarioSeed, Quality::D, &[]).origin(), DataOrigin::ScenarioSeed);
        assert_eq!(p(ProvStatus::Unknown, Quality::D, &[]).origin(), DataOrigin::ScenarioSeed);
        assert_eq!(p(ProvStatus::Generated, Quality::E, &[]).origin(), DataOrigin::Generated);
        assert!(!p(ProvStatus::Generated, Quality::E, &[]).names_real_entity());
        let mut conflicted = p(ProvStatus::Verified, Quality::A, &["u"]);
        conflicted.conflicts.push(Value::Boolean(true));
        assert!(!conflicted.allows_value(), "a recorded conflict never overrides a value");
    }

    #[test]
    fn a_malformed_record_is_reported_and_not_loaded() {
        let bad = r#"
[meta]
dataset = "test.bad"

[[club]]
id = "club.nokind"
name = "No Kind"
city = "X"
state = "state.wb"
status = "active"
prov = { status = "inferred", q = "C", src = [] }

[[club]]
id = "club.badenum"
name = "Bad Enum"
city = "X"
state = "state.wb"
kind = "megaclub"
status = "active"
prov = { status = "inferred", q = "C", src = [] }

[[club]]
id = "club.noprov"
name = "No Prov"
city = "X"
state = "state.wb"
kind = "amateur"
status = "active"

[[club]]
id = "club.badprov"
name = "Bad Prov"
city = "X"
state = "state.wb"
kind = "amateur"
status = "active"
prov = { status = "certain", q = "C", src = [] }

[[club]]
id = "club.badq"
name = "Bad Q"
city = "X"
state = "state.wb"
kind = "amateur"
status = "active"
prov = { status = "inferred", q = "Z", src = [] }

[[club]]
id = "club.fine"
name = "Fine"
city = "X"
state = "state.wb"
kind = "amateur"
status = "active"
prov = { status = "inferred", q = "C", src = [] }

[[club]]
id = "club.fine"
name = "Fine Twice"
city = "X"
state = "state.wb"
kind = "amateur"
status = "active"
prov = { status = "inferred", q = "C", src = [] }

[[stadium]]
id = "stadium.cap"
name = "Cap"
city = "X"
state = "state.wb"
capacity = 5000
prov = { status = "inferred", q = "C", src = [] }

[[stadium]]
id = "stadium.negative"
name = "Neg"
city = "X"
state = "state.wb"
capacity = -5
capacity_as_of = 2023
prov = { status = "inferred", q = "C", src = [] }

[[membership]]
club = "club.ghost"
competition = "comp.nowhere"
season = "2026"
prov = { status = "inferred", q = "C", src = [] }

[[club]]
id = "club.liar"
name = "Liar"
city = "X"
state = "state.wb"
kind = "amateur"
status = "active"
prov = { status = "verified", q = "B", src = [] }

[[spaceship]]
id = "x"
"#;
        let r = load(bad);
        let p = problems(&r).join("\n");
        for id in ["club.nokind", "club.badenum", "club.noprov", "club.badprov", "club.badq", "stadium.negative"] {
            assert!(r.club(id).is_none() && r.stadium(id).is_none(), "{id} must not be loaded");
            assert!(p.contains(id), "{id} must be reported:\n{p}");
        }
        assert!(p.contains("has no prov"), "{p}");
        assert!(p.contains("megaclub"), "the unknown value is named: {p}");
        assert!(p.contains("duplicate id"), "{p}");
        assert!(p.contains("has a capacity but no capacity_as_of"), "{p}");
        assert!(p.contains("club.ghost, which is not a loaded record") && p.contains("comp.nowhere, which is not a loaded record"), "{p}");
        assert!(p.contains("unknown table") && p.contains("spaceship"), "{p}");
        assert!(p.contains("verified but no source") || p.contains("no source is listed"), "a verified record with no source is reported: {p}");
        assert_eq!(r.club("club.fine").unwrap().name, "Fine", "the first of two duplicates stays");
        assert_eq!(r.club("club.liar").unwrap().prov.origin(), DataOrigin::ScenarioSeed, "and it is not an import");
        assert!(!p.contains("missing [meta]"), "this file has a meta table");
        let r2 = load("[[club]]\nid = \"club.x\"\n");
        assert!(problems(&r2).iter().any(|f| f.contains("missing [meta]")), "{:?}", problems(&r2));
        let r3 = load("this is = = not toml");
        assert!(problems(&r3).iter().any(|f| f.contains("does not parse")), "{:?}", problems(&r3));
        assert_eq!(r3.records(), 0);
    }

    #[test]
    fn a_reference_to_the_wrong_kind_of_record_is_reported() {
        let text = GOOD.replace("organiser = \"assoc.ifa\"", "organiser = \"club.a\"");
        let p = problems(&load(&text)).join("\n");
        assert!(p.contains("organiser points at club.a, which is not a assoc record"), "{p}");
    }

    #[test]
    fn matching_is_by_stable_id_or_exact_name_and_state() {
        let r = load(GOOD);
        assert_eq!(r.club_exact("Alpha FC", "state.wb").unwrap().id, "club.a");
        assert!(r.club_exact("Alpha FC", "state.kl").is_none(), "the same name in another state is another club");
        assert!(r.club_exact("Alpha", "state.wb").is_none(), "no fuzzy merging");
        assert!(r.club_exact("alpha fc", "state.wb").is_none(), "not even by case");
        let twin = format!(
            "{GOOD}\n[[club]]\nid = \"club.a2\"\nname = \"Alpha FC\"\ncity = \"Howrah\"\nstate = \"state.wb\"\nkind = \"amateur\"\nstatus = \"active\"\nprov = {{ status = \"inferred\", q = \"C\", src = [] }}\n"
        );
        assert!(load(&twin).club_exact("Alpha FC", "state.wb").is_none(), "two clubs with one name in a state: no answer, not a guess");
        assert_eq!(r.state_of_key("WB").unwrap().id, "state.wb");
        assert!(r.state_of_key("ZZ").is_none());
    }

    #[test]
    fn grounds_are_found_by_reference_not_by_guess() {
        let r = load(GOOD);
        let a = r.club("club.a").unwrap();
        assert_eq!(r.stadium_of(a).unwrap().id, "stadium.vyb", "the stadium lists the club");
        assert!(r.stadium_of(r.club("club.b").unwrap()).is_none(), "nothing names a ground for club.b");
        let text = GOOD.replace("kind = \"amateur\"\nstatus = \"unknown\"", "kind = \"amateur\"\nstatus = \"unknown\"\nhome_ground_name = \"Maidan Ground\"");
        let r = load(&text);
        assert_eq!(r.stadium_of(r.club("club.b").unwrap()).unwrap().id, "stadium.nocap", "an exact ground name in the club's state");
    }

    #[test]
    fn members_are_the_placeable_clubs_of_the_season_in_file_order() {
        let r = load(GOOD);
        let ids: Vec<&str> = r.members("comp.wb.top", 2026).iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["club.a", "club.b"], "the defunct club and last year's entry are not members");
        assert!(season_starts("2026-27", 2026) && season_starts("2026", 2026) && !season_starts("2025-26", 2026) && !season_starts("20260", 2026));
        assert_eq!(r.state_top_league("state.wb", 2026).unwrap().id, "comp.wb.top");
        assert!(r.state_top_league("state.kl", 2026).is_none());
    }

    #[test]
    fn derbies_are_labels_each_pair_once() {
        let r = load(GOOD);
        let d = r.derbies();
        assert_eq!(d.len(), 1, "the alias names the pair the rivalry already names");
        assert_eq!((d[0].name, d[0].is_derby), ("The Derby", true));
    }

    #[test]
    fn the_result_does_not_depend_on_the_order_of_the_files() {
        let other = "[meta]\ndataset = \"o\"\n[[state]]\nid = \"state.zz\"\nkey = \"ZZ\"\nname = \"Z\"\nkind = \"state\"\nprov = { status = \"inferred\", q = \"C\", src = [] }\n";
        let a = load_sources(&[("a.toml", GOOD), ("b.toml", other)]);
        let b = load_sources(&[("b.toml", other), ("a.toml", GOOD)]);
        let ids = |r: &Reference| r.states.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&a), ids(&b));
        assert_eq!(a.findings, b.findings);
    }

    #[test]
    fn the_embedded_list_is_every_file_in_the_folder() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/worlds/india");
        let on_disk = load_dir(&root).expect("the reference folder is readable");
        let mut embedded: Vec<&str> = BUILTIN_FILES.iter().map(|f| f.0).collect();
        embedded.sort_unstable();
        let mut disk: Vec<String> = Vec::new();
        fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    if p.file_name().is_some_and(|n| n != "_manifest") {
                        walk(&p, root, out);
                    }
                } else if p.extension().is_some_and(|x| x == "toml") && p.file_name().is_some_and(|n| n != "pack.toml") {
                    out.push(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
                }
            }
        }
        walk(&root, &root, &mut disk);
        disk.sort();
        assert_eq!(embedded, disk.iter().map(String::as_str).collect::<Vec<_>>(), "a reference file was added or removed: update BUILTIN_FILES");
        // And the embedded text is the text on disk.
        assert_eq!(on_disk.records(), builtin().records());
        assert_eq!(on_disk.findings, builtin().findings);
    }

    #[test]
    fn the_builtin_reference_loads_without_findings_and_has_what_the_world_needs() {
        let r = builtin();
        let bad: Vec<String> = problems(r);
        assert!(bad.iter().all(|f| f.contains("bidhannagar-msa")), "only the one known data inconsistency is expected, found: {bad:#?}");
        assert!(r.records() > 2000, "{}", r.records());
        assert!(r.clubs.len() > 150 && r.stadiums.len() > 100 && r.competitions.len() > 200 && r.memberships.len() > 300);
        for t in 1..=4 {
            assert!(r.national_league(t).is_some(), "national tier {t}");
        }
        assert_eq!(r.national_league(1).unwrap().id, "comp.isl");
        assert_eq!(r.state_top_league("state.wb", 2026).unwrap().id, "comp.wb.cfl-premier");
        assert_eq!(r.state_top_league("state.kl", 2026).unwrap().id, "comp.kl.kerala-premier-league", "the league with the most members is the state's top league");
        assert!(r.derbies().len() >= 2);
        assert!(r.club_exact("Mohun Bagan Super Giant", "state.wb").is_some());
        // Every status is represented, so the page can show the spread.
        assert!(
            r.by_status[ProvStatus::Verified.index()] > 0
                && r.by_status[ProvStatus::Inferred.index()] > 0
                && r.by_status[ProvStatus::ScenarioSeed.index()] > 0
                && r.by_status[ProvStatus::Unknown.index()] > 0
        );
    }
}
