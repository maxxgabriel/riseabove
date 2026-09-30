use std::collections::BTreeMap;

use pw_core::{CompId, Date, FixtureId, IdVec, Money, NationId, TeamId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::club::TeamKind;
use crate::nation::Confed;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CompKind {
    League,
    Cup,
    Continental,
    SuperCup,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Format {
    League { rounds: u8 },
    Knockout { legs: u8, final_legs: u8 },
    Groups { groups: u8, size: u8, advance: u8, legs: u8, ko_legs: u8, final_legs: u8 },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CompRules {
    /// Yellow cards that trigger a one-match ban (0 = never).
    pub yellow_limit: u8,
    pub bench: u8,
    pub subs: u8,
    pub extra_time: bool,
    pub away_goals: bool,
    /// Max non-domestic players in the match squad (0 = unlimited).
    pub foreigner_limit: u8,
}

impl Default for CompRules {
    fn default() -> Self {
        Self { yellow_limit: 5, bench: 9, subs: 5, extra_time: true, away_goals: false, foreigner_limit: 0 }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TableRow {
    pub team: TeamId,
    pub group: u8,
    pub played: u8,
    pub won: u8,
    pub drawn: u8,
    pub lost: u8,
    pub gf: u16,
    pub ga: u16,
    pub points: i16,
}

impl TableRow {
    pub fn new(team: TeamId, group: u8) -> Self {
        Self { team, group, ..Default::default() }
    }

    #[inline]
    pub fn gd(&self) -> i32 {
        i32::from(self.gf) - i32::from(self.ga)
    }

    pub fn record(&mut self, scored: u8, conceded: u8) {
        self.played += 1;
        self.gf += u16::from(scored);
        self.ga += u16::from(conceded);
        match scored.cmp(&conceded) {
            std::cmp::Ordering::Greater => {
                self.won += 1;
                self.points += 3;
            }
            std::cmp::Ordering::Equal => {
                self.drawn += 1;
                self.points += 1;
            }
            std::cmp::Ordering::Less => self.lost += 1,
        }
    }
}

/// Standard ordering: points, goal difference, goals for, then id (stable).
pub fn sort_table(rows: &mut [TableRow]) {
    rows.sort_by(|a, b| (a.group).cmp(&b.group).then(b.points.cmp(&a.points)).then(b.gd().cmp(&a.gd())).then(b.gf.cmp(&a.gf)).then(a.team.cmp(&b.team)));
}

/// A knockout pairing across one or two legs.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tie {
    pub a: TeamId,
    pub b: TeamId,
    pub goals_a: u8,
    pub goals_b: u8,
    pub away_a: u8,
    pub away_b: u8,
    pub legs: u8,
    pub played: u8,
    pub winner: TeamId,
}

impl Tie {
    pub fn new(a: TeamId, b: TeamId, legs: u8) -> Self {
        Self { a, b, goals_a: 0, goals_b: 0, away_a: 0, away_b: 0, legs, played: 0, winner: TeamId::NONE }
    }

    pub fn is_decided(&self) -> bool {
        self.winner.is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Stage {
    #[default]
    NotStarted,
    League,
    Groups,
    /// Knockout round with this many teams remaining (32, 16, …, 2).
    Knockout(u16),
    Finished,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CompState {
    pub season: i32,
    pub entrants: Vec<TeamId>,
    pub table: Vec<TableRow>,
    pub ties: Vec<Tie>,
    pub stage: Stage,
    pub winner: TeamId,
    pub runner_up: TeamId,
    pub start: Date,
    pub end: Date,
    /// Planned date of each knockout round (first leg), set when the season is scheduled.
    pub round_dates: Vec<Date>,
    /// Knockout round currently being played (index into `round_dates`).
    pub round: u8,
    /// Teams relegated/promoted at the last season end (for news and UI).
    pub last_moves: Vec<(TeamId, i8)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Competition {
    pub name: String,
    pub short_name: String,
    /// `NONE` for continental competitions.
    pub nation: NationId,
    pub confed: Option<Confed>,
    pub kind: CompKind,
    pub tier: u8,
    pub reputation: u16,
    pub format: Format,
    pub rules: CompRules,
    /// Intended number of entrants.
    pub size: u16,
    pub promote: u8,
    pub relegate: u8,
    pub above: CompId,
    pub below: CompId,
    /// Continental qualification from league position: (competition, places).
    pub continental: SmallVec<[(CompId, u8); 3]>,
    pub team_kind: TeamKind,
    pub prize_pool: Money,
    pub state: CompState,
}

impl Competition {
    pub fn is_league(&self) -> bool {
        matches!(self.format, Format::League { .. })
    }

    pub fn sorted_table(&self) -> Vec<TableRow> {
        let mut t = self.state.table.clone();
        sort_table(&mut t);
        t
    }

    pub fn position_of(&self, team: TeamId) -> Option<usize> {
        self.sorted_table().iter().position(|r| r.team == team).map(|p| p + 1)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Score {
    pub home: u8,
    pub away: u8,
    pub ht_home: u8,
    pub ht_away: u8,
    pub extra_time: bool,
    pub pens: Option<(u8, u8)>,
}

impl Score {
    /// Winner side after extra time and penalties: Some(true) = home.
    pub fn home_won(&self) -> Option<bool> {
        match self.home.cmp(&self.away) {
            std::cmp::Ordering::Greater => Some(true),
            std::cmp::Ordering::Less => Some(false),
            std::cmp::Ordering::Equal => self.pens.map(|(h, a)| h > a),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fixture {
    /// Stable identifier that survives compaction (match reports key on it).
    pub uid: u64,
    pub comp: CompId,
    /// League matchday or knockout round index.
    pub round: u8,
    pub leg: u8,
    pub group: u8,
    pub date: Date,
    pub home: TeamId,
    pub away: TeamId,
    /// Index into the competition's `ties`, or `u16::MAX`.
    pub tie: u16,
    /// Knockout match needing a winner on the day (single leg or second leg).
    pub decisive: bool,
    pub neutral: bool,
    pub score: Option<Score>,
}

impl Fixture {
    #[inline]
    pub fn involves(&self, t: TeamId) -> bool {
        self.home == t || self.away == t
    }

    #[inline]
    pub fn opponent(&self, t: TeamId) -> TeamId {
        if self.home == t { self.away } else { self.home }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Fixtures {
    list: IdVec<FixtureId, Fixture>,
    by_date: BTreeMap<Date, Vec<FixtureId>>,
    /// Each team's fixtures (so per-team questions cost that team's
    /// schedule, not the world's).
    by_team: crate::FxHashMap<TeamId, Vec<FixtureId>>,
    next_uid: u64,
}

impl Fixtures {
    pub fn add(&mut self, mut f: Fixture) -> FixtureId {
        f.uid = self.next_uid;
        self.next_uid += 1;
        let (date, home, away) = (f.date, f.home, f.away);
        let id = self.list.push(f);
        self.by_date.entry(date).or_default().push(id);
        self.by_team.entry(home).or_default().push(id);
        self.by_team.entry(away).or_default().push(id);
        id
    }

    /// A team's fixtures between two dates (inclusive), in no particular order.
    pub fn of_team_between(&self, team: TeamId, from: Date, to: Date) -> impl Iterator<Item = FixtureId> + '_ {
        self.by_team.get(&team).into_iter().flatten().copied().filter(move |&id| {
            let d = self.list[id].date;
            d >= from && d <= to
        })
    }

    #[inline]
    pub fn get(&self, id: FixtureId) -> &Fixture {
        &self.list[id]
    }

    #[inline]
    pub fn get_mut(&mut self, id: FixtureId) -> &mut Fixture {
        &mut self.list[id]
    }

    pub fn on(&self, d: Date) -> &[FixtureId] {
        self.by_date.get(&d).map_or(&[], Vec::as_slice)
    }

    pub fn between(&self, from: Date, to: Date) -> impl Iterator<Item = FixtureId> + '_ {
        // An empty window (a competition that has not started yet asks for `start - 1 ..= today`) is empty, not a panic.
        let window = (from <= to).then(|| self.by_date.range(from..=to));
        window.into_iter().flatten().flat_map(|(_, v)| v.iter().copied())
    }

    pub fn iter(&self) -> impl Iterator<Item = (FixtureId, &Fixture)> {
        self.list.iter_enumerated()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Whether `team` has no fixture (other than `ignore`) on `date`.
    pub fn team_free(&self, team: TeamId, date: Date, ignore: Option<FixtureId>) -> bool {
        self.of_team_between(team, date, date).all(|id| Some(id) == ignore)
    }

    /// The first date on or after `from` when neither team is playing
    /// (a team never plays twice in a day). Gives up after `limit` days and returns `from`.
    pub fn first_free_date(&self, a: TeamId, b: TeamId, from: Date, limit: i32, ignore: Option<FixtureId>) -> Date {
        (0..=limit).map(|d| from.add_days(d)).find(|&d| self.team_free(a, d, ignore) && self.team_free(b, d, ignore)).unwrap_or(from)
    }

    /// Move a fixture to another date (postponement / rescheduling).
    pub fn reschedule(&mut self, id: FixtureId, date: Date) {
        let old = self.list[id].date;
        if let Some(v) = self.by_date.get_mut(&old) {
            v.retain(|&x| x != id);
        }
        self.list[id].date = date;
        self.by_date.entry(date).or_default().push(id);
    }

    pub fn next_for(&self, team: TeamId, from: Date, horizon: i32) -> Option<FixtureId> {
        self.of_team_between(team, from, from.add_days(horizon)).filter(|&id| self.list[id].score.is_none()).min_by_key(|&id| (self.list[id].date, id))
    }

    /// Drop played fixtures older than `before`. Invalidates `FixtureId`s;
    /// callers keep only `uid`s across compaction.
    pub fn compact(&mut self, before: Date) {
        let kept: Vec<Fixture> = std::mem::take(&mut self.list).into_vec().into_iter().filter(|f| f.date >= before || f.score.is_none()).collect();
        self.by_date.clear();
        self.by_team.clear();
        self.list = IdVec::with_capacity(kept.len());
        for f in kept {
            let (d, home, away) = (f.date, f.home, f.away);
            let id = self.list.push(f);
            self.by_date.entry(d).or_default().push(id);
            self.by_team.entry(home).or_default().push(id);
            self.by_team.entry(away).or_default().push(id);
        }
    }
}
