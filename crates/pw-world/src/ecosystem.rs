//! A nation's football ecosystem below and beside the professional game:
//! regions and districts, the associations that run them, the mass of children
//! who play (as aggregates, never as people), the institutions that develop
//! them, and the route each materialised player has taken.
//!
//! Layers, cheapest first:
//! 1. `Pool`: participants per district and age, plain numbers.
//! 2. Prospects: when a child becomes competitive or noticed, they become a
//!    real player (`PlayerSource::RegionalPool`) with a `PlayerStory`.
//! 3. Registered and professional players use the ordinary machinery.
//!
//! Everything regional is a *state* that moves: infrastructure follows
//! investment with a lag, culture remembers what a region has produced, and
//! nothing is a permanent bonus for a place.

use pw_core::{ClubId, Date, IdVec, LocalClubId, NationId, PersonId, PlayerId, RegionId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum RegionKind {
    /// A state or union territory (or the equivalent): has an association.
    State,
    /// A district or cluster of districts inside a state.
    District,
}

/// Coarse environment: adaptation, travel, pitch condition and scheduling, not weather.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Climate {
    HumidCoastal,
    HotDry,
    HighAltitude,
    CoolHill,
    HeavyMonsoon,
    NorthEast,
    Temperate,
}

/// A place football is played and developed. Every quality runs 0–100 and moves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub nation: NationId,
    pub kind: RegionKind,
    /// The state, for a district; `NONE` for a state.
    pub parent: RegionId,
    /// Coarse map position on a 0–100 grid (west→east, south→north). Not geography, only distance.
    pub x: u8,
    pub y: u8,
    pub climate: Climate,
    /// Main language, an index into the nation's language list (adaptation, settling).
    pub language: u8,
    /// University zone.
    pub zone: u8,
    pub population_k: u32,
    /// Share of children who play organised football.
    pub participation: f32,
    pub facilities: f32,
    pub coach_density: f32,
    pub academy_access: f32,
    /// How affordable it is to take part.
    pub economic_access: f32,
    /// Closeness of professional clubs (visibility, role models, trials).
    pub pro_proximity: f32,
    pub competition_density: f32,
    /// How much of the region's football is watched by people who can move a player on.
    pub scouting_coverage: f32,
    /// Investment received this year (public and private), the flow that infrastructure lags behind.
    pub invest_public: f32,
    pub invest_private: f32,
    /// Football memory: what the region has produced lately. Decays; feeds participation and pride.
    pub culture: f32,
}

/// A state (or national) football association as an institution.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Association {
    pub admin: f32,
    pub youth_invest: f32,
    pub coach_ed: f32,
    pub comp_quality: f32,
    pub scouting: f32,
    pub finance: f32,
    pub grassroots_reach: f32,
    pub academy_coord: f32,
    pub referee_dev: f32,
    pub facilities: f32,
    pub commercial: f32,
    pub governance: f32,
}

/// Participants by age, ages 4–17, for one district.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pool {
    pub part: [f32; 14],
    /// Prospects drawn out of the pool so far (for the development metrics).
    pub materialised: u32,
}

pub const POOL_FIRST_AGE: usize = 4;

/// Who runs the football a child first plays.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Provider {
    /// The AIFF's grassroots programme run through clubs and associations.
    BlueCubs,
    School,
    Community,
    Municipal,
    Foundation,
    PrivateSchool,
}

/// What a step on a player's route was.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum StageKind {
    Grassroots,
    School,
    District,
    StateYouth,
    Academy,
    Released,
    University,
    StateLeague,
    StateTeam,
    Trial,
    SemiPro,
    Professional,
    NationalCamp,
}

impl StageKind {
    pub const fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(c: u8) -> StageKind {
        use StageKind::*;
        [Grassroots, School, District, StateYouth, Academy, Released, University, StateLeague, StateTeam, Trial, SemiPro, Professional, NationalCamp].get(usize::from(c)).copied().unwrap_or(Grassroots)
    }

    pub const fn label(self) -> &'static str {
        match self {
            StageKind::Grassroots => "grassroots football",
            StageKind::School => "school football",
            StageKind::District => "district football",
            StageKind::StateYouth => "state youth football",
            StageKind::Academy => "academy",
            StageKind::Released => "released",
            StageKind::University => "university",
            StageKind::StateLeague => "state league",
            StageKind::StateTeam => "state team",
            StageKind::Trial => "trial",
            StageKind::SemiPro => "semi-professional",
            StageKind::Professional => "professional",
            StageKind::NationalCamp => "national camp",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Stage {
    pub date: Date,
    pub kind: StageKind,
    /// The club, institution or competition (meaning depends on `kind`).
    pub target: u32,
    pub region: RegionId,
}

/// Where a player comes from and who found them. Only for materialised players.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerStory {
    /// Where they grew up.
    pub home: RegionId,
    /// Where they are developing now (moves with school, academy, university).
    pub dev: RegionId,
    pub provider: Provider,
    /// The first person outside the family who took them seriously (a scout, coach or selector).
    pub found_by: PersonId,
    pub found_club: ClubId,
    pub found_on: Date,
}

/// What a university gives a player.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Scholarship {
    pub inst: u32,
    /// 0 none, 1 tuition, 2 tuition and hostel, 3 full support with a stipend.
    pub tier: u8,
    pub from: Date,
}

/// Extra state for a school, sports school or university (`minor::Institution` keeps the basics).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstProfile {
    pub region: RegionId,
    /// Sports budget, 0–100. Moves with results, visibility and sponsors.
    pub resources: f32,
    pub facilities: f32,
    /// Scholarship places it can fund each year.
    pub scholarships: u8,
    /// Sports school or residential programme (better environment, but it means moving).
    pub residential: bool,
    /// Rolling football success, 0–100.
    pub success: f32,
    /// A real institution (named from the seed data) rather than a generated one.
    pub real: bool,
}

/// The state-team championship in progress (the Santosh Trophy's shape: state and
/// association sides, groups, then knockouts).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tournament {
    pub year: i32,
    pub nation: NationId,
    /// (state, squad).
    pub squads: Vec<(RegionId, Vec<PlayerId>)>,
    /// Groups of indices into `squads`.
    pub groups: Vec<Vec<usize>>,
    /// Scheduled matches not yet played: (date, home, away, knockout).
    pub fixtures: Vec<(Date, usize, usize, bool)>,
    /// Points and goals per squad in the groups: (played, points, for, against).
    pub table: Vec<(u8, u8, u16, u16)>,
    /// Squads still alive in the knockout.
    pub alive: Vec<usize>,
    pub stage: u8,
    pub winner: RegionId,
    pub scorers: FxHashMap<PlayerId, u16>,
}

/// Owned by `pw_sim::ecosystem`. Empty unless a nation has been given an ecosystem.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Ecosystem {
    pub regions: IdVec<RegionId, Region>,
    pub zones: Vec<String>,
    pub languages: Vec<String>,
    /// State associations, keyed by the state's region.
    pub assoc: FxHashMap<RegionId, Association>,
    /// Each association's name (the pack's real names).
    pub assoc_name: FxHashMap<RegionId, String>,
    /// The national federation's own attributes.
    pub federation: Option<Association>,
    pub pools: FxHashMap<RegionId, Pool>,
    pub inst: FxHashMap<u32, InstProfile>,
    pub scholarship: FxHashMap<PlayerId, Scholarship>,
    pub story: FxHashMap<PlayerId, PlayerStory>,
    pub stages: FxHashMap<PlayerId, SmallVec<[Stage; 8]>>,
    pub local_region: FxHashMap<LocalClubId, RegionId>,
    pub club_region: FxHashMap<ClubId, RegionId>,
    /// The state-team championship of the current year, while it runs.
    pub tournament: Option<Tournament>,
    /// Winners of the state championship so far: (state, year).
    pub tournament_titles: Vec<(RegionId, i32)>,
    /// Year the last yearly update ran.
    pub last_year: i32,
}

impl Ecosystem {
    pub fn is_configured(&self) -> bool {
        !self.regions.is_empty()
    }

    /// The state a region belongs to (itself if it is one).
    pub fn state_of(&self, r: RegionId) -> RegionId {
        if r.is_none() {
            return r;
        }
        let reg = &self.regions[r];
        if reg.kind == RegionKind::State { r } else { reg.parent }
    }

    /// Coarse travel burden between two regions, 0 (same place) to 1 (across the country).
    pub fn travel_burden(&self, a: RegionId, b: RegionId) -> f32 {
        if a.is_none() || b.is_none() {
            return 0.3;
        }
        let (ra, rb) = (&self.regions[a], &self.regions[b]);
        let dx = f32::from(ra.x) - f32::from(rb.x);
        let dy = f32::from(ra.y) - f32::from(rb.y);
        ((dx * dx + dy * dy).sqrt() / 100.0).min(1.0)
    }

    pub fn region_of_club(&self, c: ClubId) -> RegionId {
        self.club_region.get(&c).copied().unwrap_or(RegionId::NONE)
    }

    /// A player's route so far, oldest first.
    pub fn route(&self, p: PlayerId) -> &[Stage] {
        self.stages.get(&p).map_or(&[], |v| v.as_slice())
    }
}
