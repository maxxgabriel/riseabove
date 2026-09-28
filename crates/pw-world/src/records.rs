//! A generic record engine (U in the media brief; 18 in the final brief).
//!
//! One book holds records at every level of the game — a school's all-time
//! scorer, a university league's goals in a season, an amateur division's
//! biggest win, a club's longest winning run, a league's points total, the
//! youngest scorer in a competition, a nation's most capped player. A
//! record is keyed by *where* it applies (scope), *what* is measured (stat)
//! and the level of the game. Each keeps its line of holders, so a new
//! record carries its history: who held it, since when, by how much it
//! was beaten, how long it stood.
//!
//! The professional records in `honours` feed this book too (without a
//! second announcement), so every record has the same history.

use pw_core::{ClubId, CompId, Date, LocalClubId, NationId, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::minor::{Entrant, Level};

/// Where a record applies.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Scope {
    World,
    Nation(NationId),
    Comp(CompId),
    Club(ClubId),
    Institution(u32),
    Local(LocalClubId),
    /// A minor competition by kind within a nation (school leagues of a
    /// nation share records; `u8` is the kind's code, `tier` for amateurs).
    Minor(NationId, u8),
}

/// What is measured.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Stat {
    /// All-time goals for the scope.
    Goals,
    Apps,
    GoalsInSeason,
    /// Margin of victory.
    BiggestWin,
    FeePaid,
    FeeReceived,
    Caps,
    IntlGoals,
    Titles,
    /// Age in days (lower is the record).
    YoungestScorer,
    YoungestDebut,
    /// Age in days (higher is the record).
    OldestScorer,
    WinsInRow,
    UnbeatenRun,
    PointsInSeason,
}

impl Stat {
    /// Whether lower values are records.
    pub const fn lower_is_better(self) -> bool {
        matches!(self, Stat::YoungestScorer | Stat::YoungestDebut)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct RecordKey {
    pub scope: Scope,
    pub stat: Stat,
    pub level: Level,
}

/// Who holds it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Holder {
    Person(PersonId),
    Club(ClubId),
    Entrant(Entrant),
    Nation(NationId),
    /// A figure of the world's past (`World::backfill.figures`).
    Past(u32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Mark {
    pub holder: Holder,
    pub value: i64,
    pub date: Date,
    /// Who was on the other end (a beaten club), if anyone.
    pub against: Option<Holder>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub key: RecordKey,
    pub current: Mark,
    /// Earlier holders, most recent last (bounded).
    pub previous: SmallVec<[Mark; 3]>,
    /// Times it has been broken.
    pub broken: u16,
}

/// A record falling, with its context (for text and for consequences).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Broken {
    pub key: RecordKey,
    pub new: Mark,
    pub old: Option<Mark>,
    /// Days the old mark stood.
    pub stood_days: i32,
    /// Whether the new holder already held it (extending their own record).
    pub own: bool,
    pub announced: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RecordBook {
    pub records: FxHashMap<RecordKey, Record>,
    pub broken: Vec<Broken>,
    /// Current runs (club → (wins in a row, unbeaten)).
    pub runs: FxHashMap<ClubId, (u16, u16)>,
    /// Minor all-time tallies (player person, entrant) → (apps, goals).
    pub minor_tallies: FxHashMap<(PersonId, Entrant), (u16, u16)>,
    /// Titles won per (entrant, minor kind code, nation).
    pub minor_titles: FxHashMap<(Entrant, u8, NationId), u16>,
}

impl RecordBook {
    pub fn get(&self, key: &RecordKey) -> Option<&Record> {
        self.records.get(key)
    }

    /// Records held by a person, club or entrant.
    pub fn held_by(&self, h: Holder) -> Vec<&Record> {
        let mut v: Vec<&Record> = self.records.values().filter(|r| r.current.holder == h).collect();
        v.sort_by_key(|r| r.key);
        v
    }

    /// Records in a scope.
    pub fn in_scope(&self, s: Scope) -> Vec<&Record> {
        let mut v: Vec<&Record> = self.records.values().filter(|r| r.key.scope == s).collect();
        v.sort_by_key(|r| r.key);
        v
    }
}
