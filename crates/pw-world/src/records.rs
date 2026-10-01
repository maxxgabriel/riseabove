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

use pw_core::{ClubId, CompId, Date, LocalClubId, NationId, PersonId, RegionId};
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
    /// A state or district: what it has produced, and football played there.
    Region(RegionId),
    /// A one-off tournament series within a nation (a state championship, a national school games); `u16` names it.
    Event(NationId, u16),
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
    // ---- one match, one player (seconds, tenths of km/h, metres, rating ×10)
    /// Seconds from kick-off to a player's first goal (lower is the record).
    FastestGoal,
    /// Seconds from a player's first to third goal of one match (lower).
    FastestHatTrick,
    GoalsInMatch,
    AssistsInMatch,
    /// ESTIMATED peak speed in a match, tenths of km/h. A measurement model (pace, acceleration, age, condition), not tracked play.
    TopSpeed,
    /// ESTIMATED distance covered in a match, metres. A measurement model (stamina, work rate, role, minutes).
    DistanceCovered,
    /// Best match rating ×10.
    MatchRating,
    /// Seconds from kick-off to a dismissal (lower).
    FastestRedCard,
    /// Age in days at a hat-trick (lower).
    YoungestHatTrick,
    // ---- a season, a career
    AssistsInSeason,
    CleanSheetsInSeason,
    /// Season average rating ×100 (enough appearances only).
    AvgRatingInSeason,
    Assists,
    CleanSheets,
    HatTricks,
    YellowCards,
    RedCards,
    ManOfTheMatch,
    /// Consecutive matches scored in.
    GoalStreak,
    /// Consecutive appearances for the club side.
    AppStreak,
    /// Appearances taken to reach 1, 10, 50, 100 career goals (lower).
    AppsToFirstGoal,
    AppsTo10Goals,
    AppsTo50Goals,
    AppsTo100Goals,
    /// Days from debut to reach 1, 10, 50, 100 career goals (lower).
    DaysToFirstGoal,
    DaysTo10Goals,
    DaysTo50Goals,
    DaysTo100Goals,
    /// Goals per game ×1000 over a career of at least 100 appearances.
    GoalRate,
    /// Minutes a keeper has gone unbeaten in a row.
    KeeperMinutesUnbeaten,
    /// Golden boots (top-scorer awards) won.
    GoldenBoots,
    // ---- teams
    MostGoalsInMatch,
    MostGoalsInSeason,
    /// Goals conceded over a league season (lower).
    FewestConcededInSeason,
    BestGoalDifference,
    WinsInSeason,
    /// Matches unbeaten from the start of a league season.
    UnbeatenSeason,
    LosingRun,
    HomeUnbeaten,
    HighestAttendance,
    /// Consecutive titles in a competition.
    TitleStreak,
    // ---- regions
    PlayersProduced,
    InternationalsProduced,
}

/// What kind of evidence made a record (the India brief, item 8). An estimate must never sit beside a fact without saying so.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Provenance {
    /// Counted from what happened in a match the engine played (goals, cards, clean sheets, minutes).
    ObservedMatchStat,
    /// Computed from observed stats (a streak, a rate, a total across seasons).
    DerivedFromObservedStats,
    /// Modelled from a player's attributes, because the match engine does not record it.
    Estimated,
    /// Brought in with an imported database, from before the simulation began.
    ImportedHistorical,
    /// Generated to give a new world a past, from before the simulation began.
    SimulatedHistorical,
}

impl Provenance {
    pub const fn label(self) -> &'static str {
        match self {
            Provenance::ObservedMatchStat => "observed in a match",
            Provenance::DerivedFromObservedStats => "derived from observed matches",
            Provenance::Estimated => "estimated (not tracked in play)",
            Provenance::ImportedHistorical => "imported history",
            Provenance::SimulatedHistorical => "simulated history",
        }
    }

    /// Whether this counts as an official record that can be announced as one.
    pub const fn official(self) -> bool {
        !matches!(self, Provenance::Estimated)
    }
}

impl Stat {
    /// The evidence behind this kind of record, whenever it was set.
    pub const fn provenance(self) -> Provenance {
        match self {
            Stat::TopSpeed | Stat::DistanceCovered => Provenance::Estimated,
            Stat::WinsInRow
            | Stat::UnbeatenRun
            | Stat::PointsInSeason
            | Stat::GoalStreak
            | Stat::AppStreak
            | Stat::AppsToFirstGoal
            | Stat::AppsTo10Goals
            | Stat::AppsTo50Goals
            | Stat::AppsTo100Goals
            | Stat::DaysToFirstGoal
            | Stat::DaysTo10Goals
            | Stat::DaysTo50Goals
            | Stat::DaysTo100Goals
            | Stat::GoalRate
            | Stat::AvgRatingInSeason
            | Stat::Titles
            | Stat::GoldenBoots
            | Stat::KeeperMinutesUnbeaten => Provenance::DerivedFromObservedStats,
            _ => Provenance::ObservedMatchStat,
        }
    }

    pub const fn lower_is_better(self) -> bool {
        matches!(
            self,
            Stat::YoungestScorer
                | Stat::YoungestDebut
                | Stat::YoungestHatTrick
                | Stat::FastestGoal
                | Stat::FastestHatTrick
                | Stat::FastestRedCard
                | Stat::AppsToFirstGoal
                | Stat::AppsTo10Goals
                | Stat::AppsTo50Goals
                | Stat::AppsTo100Goals
                | Stat::DaysToFirstGoal
                | Stat::DaysTo10Goals
                | Stat::DaysTo50Goals
                | Stat::DaysTo100Goals
                | Stat::FewestConcededInSeason
        )
    }

    /// The name of the record, for headlines.
    pub const fn title(self) -> &'static str {
        match self {
            Stat::Goals => "all-time scoring record",
            Stat::Apps => "appearance record",
            Stat::GoalsInSeason => "record for goals in a season",
            Stat::BiggestWin => "record win",
            Stat::FeePaid => "record signing",
            Stat::FeeReceived => "record sale",
            Stat::Caps => "caps record",
            Stat::IntlGoals => "international scoring record",
            Stat::Titles => "record for most titles",
            Stat::YoungestScorer => "youngest-scorer record",
            Stat::YoungestDebut => "youngest-debutant record",
            Stat::OldestScorer => "oldest-scorer record",
            Stat::WinsInRow => "record winning run",
            Stat::UnbeatenRun => "record unbeaten run",
            Stat::PointsInSeason => "points record",
            Stat::FastestGoal => "fastest-goal record",
            Stat::FastestHatTrick => "fastest hat-trick record",
            Stat::GoalsInMatch => "record for goals in a match",
            Stat::AssistsInMatch => "record for assists in a match",
            Stat::TopSpeed => "estimated peak speed",
            Stat::DistanceCovered => "estimated distance covered",
            Stat::MatchRating => "best-performance record",
            Stat::FastestRedCard => "fastest-dismissal record",
            Stat::YoungestHatTrick => "youngest hat-trick record",
            Stat::AssistsInSeason => "record for assists in a season",
            Stat::CleanSheetsInSeason => "record for clean sheets in a season",
            Stat::AvgRatingInSeason => "best season-average rating",
            Stat::Assists => "all-time assists record",
            Stat::CleanSheets => "all-time clean-sheets record",
            Stat::HatTricks => "record for hat-tricks",
            Stat::YellowCards => "record for yellow cards",
            Stat::RedCards => "record for red cards",
            Stat::ManOfTheMatch => "record for man-of-the-match awards",
            Stat::GoalStreak => "record scoring streak",
            Stat::AppStreak => "record run of consecutive appearances",
            Stat::AppsToFirstGoal => "fastest-to-a-first-goal record (appearances)",
            Stat::AppsTo10Goals => "fastest-to-10-goals record (appearances)",
            Stat::AppsTo50Goals => "fastest-to-50-goals record (appearances)",
            Stat::AppsTo100Goals => "fastest-to-100-goals record (appearances)",
            Stat::DaysToFirstGoal => "quickest first goal after debut",
            Stat::DaysTo10Goals => "quickest to 10 goals",
            Stat::DaysTo50Goals => "quickest to 50 goals",
            Stat::DaysTo100Goals => "quickest to 100 goals",
            Stat::GoalRate => "best goals-per-game record",
            Stat::KeeperMinutesUnbeaten => "record for minutes without conceding",
            Stat::GoldenBoots => "record for golden boots",
            Stat::MostGoalsInMatch => "record for goals in a match by a team",
            Stat::MostGoalsInSeason => "record for goals in a season by a team",
            Stat::FewestConcededInSeason => "best defensive season",
            Stat::BestGoalDifference => "best goal difference",
            Stat::WinsInSeason => "record for wins in a season",
            Stat::UnbeatenSeason => "longest unbeaten start to a season",
            Stat::LosingRun => "longest losing run",
            Stat::HomeUnbeaten => "longest unbeaten home run",
            Stat::HighestAttendance => "record attendance",
            Stat::TitleStreak => "record run of consecutive titles",
            Stat::PlayersProduced => "record for players produced",
            Stat::InternationalsProduced => "record for internationals produced",
        }
    }

    /// A value in words.
    pub fn render(self, v: i64) -> String {
        let minutes_seconds = |s: i64| format!("{}:{:02}", s / 60, s % 60);
        match self {
            Stat::YoungestScorer | Stat::YoungestDebut | Stat::OldestScorer | Stat::YoungestHatTrick => format!("{} years {} days", v / 365, v % 365),
            Stat::FastestGoal | Stat::FastestHatTrick | Stat::FastestRedCard => format!("{} into the match", minutes_seconds(v)),
            Stat::TopSpeed => format!("about {}.{} km/h", v / 10, v % 10),
            Stat::DistanceCovered => format!("about {}.{:02} km", v / 1000, (v % 1000) / 10),
            Stat::MatchRating => format!("a {}.{} rating", v / 10, v % 10),
            Stat::AvgRatingInSeason => format!("an average of {}.{:02}", v / 100, v % 100),
            Stat::GoalRate => format!("{}.{:03} goals a game", v / 1000, v % 1000),
            Stat::AppsToFirstGoal | Stat::AppsTo10Goals | Stat::AppsTo50Goals | Stat::AppsTo100Goals => format!("{v} appearances"),
            Stat::DaysToFirstGoal | Stat::DaysTo10Goals | Stat::DaysTo50Goals | Stat::DaysTo100Goals => format!("{} days", v),
            Stat::KeeperMinutesUnbeaten => format!("{v} minutes"),
            Stat::HighestAttendance => format!("{v} spectators"),
            Stat::BestGoalDifference => format!("a goal difference of {v}"),
            Stat::FewestConcededInSeason => format!("{v} goals conceded"),
            Stat::FeePaid | Stat::FeeReceived => format!("{v}"),
            Stat::BiggestWin => format!("a {v}-goal margin"),
            Stat::WinsInRow | Stat::WinsInSeason => format!("{v} wins"),
            Stat::UnbeatenRun | Stat::UnbeatenSeason | Stat::HomeUnbeaten => format!("{v} games unbeaten"),
            Stat::LosingRun => format!("{v} defeats in a row"),
            Stat::PointsInSeason => format!("{v} points"),
            Stat::Titles | Stat::TitleStreak => format!("{v} titles"),
            Stat::Caps => format!("{v} caps"),
            Stat::Goals | Stat::GoalsInSeason | Stat::IntlGoals | Stat::GoalsInMatch | Stat::MostGoalsInMatch | Stat::MostGoalsInSeason => format!("{v} goals"),
            Stat::Assists | Stat::AssistsInSeason | Stat::AssistsInMatch => format!("{v} assists"),
            Stat::Apps | Stat::AppStreak => format!("{v} appearances"),
            Stat::CleanSheets | Stat::CleanSheetsInSeason => format!("{v} clean sheets"),
            Stat::HatTricks => format!("{v} hat-tricks"),
            Stat::YellowCards => format!("{v} yellow cards"),
            Stat::RedCards => format!("{v} red cards"),
            Stat::ManOfTheMatch => format!("{v} man-of-the-match awards"),
            Stat::GoalStreak => format!("scoring in {v} games in a row"),
            Stat::GoldenBoots => format!("{v} golden boots"),
            Stat::PlayersProduced | Stat::InternationalsProduced => format!("{v} players"),
        }
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
    Region(RegionId),
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
