//! The validated import representation. Every adapter (pack CSV, Transfermarkt archive, staff list) parses its
//! files into this, and only this reaches world building. `None` means the source had no usable value
//! (UNKNOWN); the assembler decides what stands in for it and labels it in the world's origin book.

use std::collections::BTreeMap;

use pw_core::{Attr, Date, Foot, Hidden, Pos, PosGroup, StaffAttr};
use pw_world::nation::Confed;
use pw_world::{CompKind, SquadStatus, StaffRole, TeamKind};

/// A record's id inside its source (`club_id`, `player_id`, ...).
pub type Key = String;

#[derive(Clone, Debug)]
pub struct SourceInfo {
    pub name: String,
    pub snapshot: Option<Date>,
    pub note: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Severity {
    /// The row was dropped or a field discarded.
    Error,
    /// The row was kept; something was fixed, defaulted or is doubtful.
    Warning,
    /// Worth counting, no action taken (out of scope, unsupported).
    Info,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    pub file: String,
    /// 1-based data row, 0 when not tied to a row.
    pub row: usize,
    pub key: Key,
    pub message: String,
}

pub const KEPT_ISSUES: usize = 5_000;

/// Findings from parsing and resolving: a bounded sample of them plus exact counts per code.
#[derive(Default, Debug)]
pub struct Issues {
    pub sample: Vec<Issue>,
    pub counts: BTreeMap<&'static str, (Severity, u32)>,
}

impl Issues {
    pub fn add(&mut self, severity: Severity, code: &'static str, file: &str, row: usize, key: &str, message: impl Into<String>) {
        let e = self.counts.entry(code).or_insert((severity, 0));
        e.1 += 1;
        if self.sample.len() < KEPT_ISSUES {
            self.sample.push(Issue { severity, code, file: file.to_string(), row, key: key.to_string(), message: message.into() });
        }
    }

    pub fn count(&self, code: &str) -> u32 {
        self.counts.get(code).map_or(0, |c| c.1)
    }

    pub fn total(&self, severity: Severity) -> u32 {
        self.counts.values().filter(|c| c.0 == severity).map(|c| c.1).sum()
    }

    /// Codes with their counts, largest first.
    pub fn by_count(&self) -> Vec<(&'static str, u32)> {
        let mut v: Vec<_> = self.counts.iter().map(|(k, c)| (*k, c.1)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        v
    }
}

/// How firmly a link was established.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Link {
    /// The source names the target by id.
    Stated,
    /// Derived from two or more independent pieces of evidence.
    Inferred,
}

#[derive(Clone, Debug, Default)]
pub struct ImpNation {
    pub source: u8,
    pub key: Key,
    pub code: String,
    pub name: String,
    pub confed: Option<Confed>,
    pub reputation: Option<u16>,
    pub economy: Option<f32>,
    pub youth_rating: Option<u8>,
    pub calendar: Option<String>,
    /// Not listed as a country by the source; created because people hold its citizenship.
    pub minor: bool,
}

#[derive(Clone, Debug)]
pub struct ImpComp {
    pub source: u8,
    pub key: Key,
    pub name: String,
    pub short: String,
    pub nation: Option<Key>,
    pub confed: Option<Confed>,
    pub kind: CompKind,
    pub tier: Option<u8>,
    pub team_kind: TeamKind,
    pub size: Option<u16>,
    pub promote: Option<u8>,
    pub relegate: Option<u8>,
    pub reputation: Option<u16>,
    pub format: Option<String>,
    pub legs: Option<u8>,
    pub groups: Option<u8>,
    pub group_size: Option<u8>,
    pub advance: Option<u8>,
    pub prize_pool: Option<i64>,
    /// The source has no record of this competition; it was created from clubs that reference it.
    pub derived: bool,
}

impl ImpComp {
    pub fn new(key: impl Into<Key>, name: impl Into<String>, kind: CompKind) -> Self {
        Self {
            source: 0,
            key: key.into(),
            name: name.into(),
            short: String::new(),
            nation: None,
            confed: None,
            kind,
            tier: None,
            team_kind: TeamKind::First,
            size: None,
            promote: None,
            relegate: None,
            reputation: None,
            format: None,
            legs: None,
            groups: None,
            group_size: None,
            advance: None,
            prize_pool: None,
            derived: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImpClub {
    pub source: u8,
    pub key: Key,
    pub name: String,
    pub short: String,
    pub nation: Option<Key>,
    pub city: String,
    pub league: Option<Key>,
    pub reputation: Option<u16>,
    pub balance: Option<i64>,
    pub transfer_budget: Option<i64>,
    pub wage_budget: Option<i64>,
    pub stadium: String,
    pub capacity: Option<u32>,
    pub training: Option<u8>,
    pub youth: Option<u8>,
    pub academy: Option<u8>,
    pub colours: [Option<u32>; 2],
    pub founded: Option<u16>,
    pub extra_teams: Vec<TeamKind>,
    /// The manager named by the source, for linking staff records.
    pub manager_name: Option<String>,
    /// Sum of the squad's market values, when the source states one per club.
    pub squad_value: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct ImpPlayer {
    pub source: u8,
    pub key: Key,
    pub row: usize,
    pub first: String,
    pub last: String,
    pub common: String,
    pub dob: Option<Date>,
    pub nationality: Option<Key>,
    pub nationality2: Option<Key>,
    pub club: Option<Key>,
    pub team: TeamKind,
    pub positions: Vec<Pos>,
    /// Role estimated from recent lineup evidence when the identity row did not state it.
    pub position_inferred: bool,
    /// Coarse role when the source gives no specific position.
    pub position_group: Option<PosGroup>,
    pub foot: Option<Foot>,
    pub feet: Option<(u8, u8)>,
    pub height: Option<u8>,
    pub weight: Option<u8>,
    pub attrs: Vec<(Attr, f32)>,
    pub hidden: Vec<(Hidden, u8)>,
    pub ca: Option<f32>,
    pub pa: Option<i32>,
    pub wage: Option<i64>,
    pub contract_end: Option<Date>,
    pub value: Option<i64>,
    /// Latest dated valuation carried forward to the snapshot, not a stated current valuation.
    pub value_inferred: bool,
    pub rep_current: Option<u16>,
    pub rep_home: Option<u16>,
    pub rep_world: Option<u16>,
    pub status: Option<SquadStatus>,
    pub loan_from: Option<Key>,
    pub loan_end: Option<Date>,
    pub shirt: Option<u8>,
    pub caps: Option<u16>,
    pub intl_goals: Option<u16>,
    pub apps: Option<u32>,
    pub goals: Option<u32>,
    /// Minutes played in the twelve months before the start date, when the source covers matches.
    pub minutes_12m: Option<u32>,
    pub agent: Option<String>,
    /// When the player joined the current club, if the source says.
    pub joined: Option<Date>,
}

impl ImpPlayer {
    pub fn new(key: impl Into<Key>, row: usize) -> Self {
        Self {
            source: 0,
            key: key.into(),
            row,
            first: String::new(),
            last: String::new(),
            common: String::new(),
            dob: None,
            nationality: None,
            nationality2: None,
            club: None,
            team: TeamKind::First,
            positions: Vec::new(),
            position_inferred: false,
            position_group: None,
            foot: None,
            feet: None,
            height: None,
            weight: None,
            attrs: Vec::new(),
            hidden: Vec::new(),
            ca: None,
            pa: None,
            wage: None,
            contract_end: None,
            value: None,
            value_inferred: false,
            rep_current: None,
            rep_home: None,
            rep_world: None,
            status: None,
            loan_from: None,
            loan_end: None,
            shirt: None,
            caps: None,
            intl_goals: None,
            apps: None,
            goals: None,
            minutes_12m: None,
            agent: None,
            joined: None,
        }
    }

    pub fn display(&self) -> String {
        if !self.common.is_empty() { self.common.clone() } else { format!("{} {}", self.first, self.last).trim().to_string() }
    }
}

#[derive(Clone, Debug)]
pub struct ImpStaff {
    pub source: u8,
    pub key: Key,
    pub row: usize,
    pub first: String,
    pub last: String,
    pub dob: Option<Date>,
    /// The source gives an age, not a birth date.
    pub age: Option<u8>,
    pub nationality: Option<Key>,
    pub nationality2: Option<Key>,
    pub club: Option<Key>,
    pub club_link: Option<Link>,
    /// The source's own text for the team (kept for the unresolved report).
    pub team_text: Option<String>,
    pub role: Option<StaffRole>,
    pub attrs: Vec<(StaffAttr, u8)>,
    pub wage: Option<i64>,
    pub reputation: Option<u16>,
    pub formation: Option<String>,
    pub contract_end: Option<Date>,
}

impl ImpStaff {
    pub fn new(key: impl Into<Key>, row: usize) -> Self {
        Self {
            source: 0,
            key: key.into(),
            row,
            first: String::new(),
            last: String::new(),
            dob: None,
            age: None,
            nationality: None,
            nationality2: None,
            club: None,
            club_link: None,
            team_text: None,
            role: None,
            attrs: Vec::new(),
            wage: None,
            reputation: None,
            formation: None,
            contract_end: None,
        }
    }
}

/// A finished league season computed from imported results, or given directly.
#[derive(Clone, Debug)]
pub struct ImpSeason {
    pub comp: Key,
    pub season: i32,
    pub champion: Key,
    pub runner_up: Option<Key>,
    pub top_scorer: Option<String>,
    pub top_goals: Option<u16>,
}

/// One period a player belonged to a club before the start date.
#[derive(Clone, Debug)]
pub struct ImpSpell {
    pub player: Key,
    pub club: Key,
    pub from: Date,
    pub to: Option<Date>,
    pub fee: Option<i64>,
    pub loan: bool,
}

#[derive(Clone, Debug)]
pub struct UnresolvedRow {
    pub source: u8,
    pub id: Key,
    pub what: &'static str,
    pub reason: String,
}

#[derive(Default, Debug)]
pub struct ImportSet {
    pub sources: Vec<SourceInfo>,
    pub start: Option<Date>,
    pub seed: Option<u64>,
    pub nations: Vec<ImpNation>,
    pub comps: Vec<ImpComp>,
    pub clubs: Vec<ImpClub>,
    pub players: Vec<ImpPlayer>,
    pub staff: Vec<ImpStaff>,
    pub seasons: Vec<ImpSeason>,
    pub spells: Vec<ImpSpell>,
    pub referees: Vec<String>,
    /// Explicit (competition, club) entries, by source key.
    pub entrants: Vec<(Key, Key)>,
    /// Rows that could not be placed safely: kept for the report, never guessed into the world.
    pub unresolved: Vec<UnresolvedRow>,
    /// Player ids whose source rows describe the same person as another id (reported, never merged).
    pub possible_duplicates: Vec<(Key, Key, String)>,
    pub issues: Issues,
}

impl ImportSet {
    pub fn add_source(&mut self, name: &str, snapshot: Option<Date>, note: &str) -> u8 {
        self.sources.push(SourceInfo { name: name.into(), snapshot, note: note.into() });
        (self.sources.len() - 1) as u8
    }
}
