//! International football (07 §13, 09 §9). Nations field senior and youth
//! sides picked by their own managers from what their federations can see;
//! they play friendlies and qualifiers in the international windows and
//! continental championships and a world tournament in the summers. Caps
//! lock a player to a nation once they are competitive and senior.

use pw_core::{Date, NationId, PlayerId, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::FxHashSet;
use crate::nation::Confed;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Level {
    Senior,
    U21,
    U19,
    U17,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Senior, Level::U21, Level::U19, Level::U17];

    /// Oldest age allowed (on 1 January of the cycle), if limited.
    pub const fn max_age(self) -> Option<u32> {
        match self {
            Level::Senior => None,
            Level::U21 => Some(21),
            Level::U19 => Some(19),
            Level::U17 => Some(17),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Level::Senior => "senior",
            Level::U21 => "U21",
            Level::U19 => "U19",
            Level::U17 => "U17",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NationalSide {
    pub nation: NationId,
    pub level: Level,
    pub manager: StaffId,
    pub squad: Vec<PlayerId>,
    pub captain: PlayerId,
    pub selected: Date,
    /// When the current manager took over.
    pub since: Date,
    /// Consecutive squads each player has been in (for "dropped" and loyalty).
    pub streak: FxHashMap<PlayerId, u8>,
    /// Competitive results under the current manager (w, d, l).
    pub record: (u16, u16, u16),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Cap {
    pub nation: NationId,
    pub level: Level,
    pub caps: u16,
    pub goals: u16,
    /// Competitive (non-friendly) appearances.
    pub competitive: u16,
    pub debut: Date,
    pub last: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum TournamentKind {
    Continental(Confed),
    World,
}

impl TournamentKind {
    pub fn label(self) -> String {
        match self {
            TournamentKind::Continental(c) => format!("{} Championship", c.code()),
            TournamentKind::World => "World Tournament".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MatchKind {
    Friendly,
    Qualifier { tournament: u32 },
    Group { tournament: u32 },
    Knockout { tournament: u32, round: u8 },
}

impl MatchKind {
    pub fn competitive(self) -> bool {
        !matches!(self, MatchKind::Friendly)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntlLine {
    pub player: PlayerId,
    pub minutes: u8,
    pub rating: f32,
    pub goals: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntlMatch {
    pub date: Date,
    pub level: Level,
    pub home: NationId,
    pub away: NationId,
    pub kind: MatchKind,
    pub home_goals: u8,
    pub away_goals: u8,
    pub pens: Option<(u8, u8)>,
    pub lines: Vec<IntlLine>,
}

/// A scheduled international.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct IntlFixture {
    pub date: Date,
    pub level: Level,
    pub home: NationId,
    pub away: NationId,
    pub kind: MatchKind,
    pub neutral: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Standing {
    pub played: u8,
    pub points: u8,
    pub gf: u8,
    pub ga: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Stage {
    Qualifying,
    /// Finalists drawn into groups; waiting for the summer.
    Drawn,
    Groups,
    Knockout,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tournament {
    pub id: u32,
    pub kind: TournamentKind,
    pub year: i32,
    pub stage: Stage,
    /// Qualifying groups (nation, standing).
    pub qual_groups: Vec<Vec<(NationId, Standing)>>,
    /// Finals groups.
    pub groups: Vec<Vec<(NationId, Standing)>>,
    /// Remaining nations in the knockout bracket, in bracket order.
    pub bracket: Vec<NationId>,
    /// Nations that reached the finals.
    pub finalists: Vec<NationId>,
    /// Qualifying slots per confederation (world tournament) or in total.
    pub slots: Vec<(Confed, u8)>,
    pub round: u8,
    pub start: Date,
    pub winner: NationId,
    pub runner_up: NationId,
    /// Best player of the finals (by ratings).
    pub best_player: PlayerId,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Intl {
    pub sides: FxHashMap<(NationId, Level), NationalSide>,
    pub managers: FxHashSet<StaffId>,
    pub caps: FxHashMap<PlayerId, SmallVec<[Cap; 2]>>,
    pub matches: Vec<IntlMatch>,
    pub tournaments: Vec<Tournament>,
    /// Players who have retired from international football.
    pub retired: FxHashSet<PlayerId>,
    /// Last window processed (start date).
    pub last_window: Date,
    pub fixtures: Vec<IntlFixture>,
    /// Players who have committed to one of two eligible nations.
    pub declared: FxHashMap<PlayerId, NationId>,
    /// Pending dual-nationality call-ups awaiting the player's answer.
    pub asking: FxHashMap<PlayerId, (NationId, NationId)>,
    /// Players currently away on international duty (not available to clubs).
    pub duty: FxHashSet<PlayerId>,
}

impl Intl {
    /// The nation a player is tied to by competitive senior caps, if any.
    pub fn locked_to(&self, p: PlayerId) -> Option<NationId> {
        self.caps.get(&p)?.iter().find(|c| c.level == Level::Senior && c.competitive > 0).map(|c| c.nation)
    }

    pub fn caps_for(&self, p: PlayerId, n: NationId, level: Level) -> u16 {
        self.caps.get(&p).and_then(|v| v.iter().find(|c| c.nation == n && c.level == level)).map_or(0, |c| c.caps)
    }

    pub fn senior_caps(&self, p: PlayerId) -> u16 {
        self.caps.get(&p).map_or(0, |v| v.iter().filter(|c| c.level == Level::Senior).map(|c| c.caps).sum())
    }

    pub fn in_squad(&self, p: PlayerId) -> Option<(NationId, Level)> {
        self.sides.values().find(|s| s.squad.contains(&p)).map(|s| (s.nation, s.level))
    }

    /// The side a national manager runs.
    pub fn side_of_manager(&self, m: StaffId) -> Option<(NationId, Level)> {
        self.sides.values().find(|s| s.manager == m).map(|s| (s.nation, s.level))
    }

    /// Whether a nation is still playing at an ongoing finals tournament.
    pub fn at_finals(&self, n: NationId) -> bool {
        self.tournaments.iter().any(|t| matches!(t.stage, Stage::Groups | Stage::Knockout) && t.finalists.contains(&n) && (t.stage == Stage::Groups || t.bracket.contains(&n)))
    }
}
