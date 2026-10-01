//! What players actually did, and what different people make of it.
//!
//! Every senior appearance (and every time a player sat on the bench or was
//! left out) is recorded. Nobody reads the record neutrally: a pragmatic
//! manager values the defensive work, a scout the underlying numbers, the
//! media the goals and the big nights, the fans effort and recent moments.
//! Interpretations are derived from the same record through each observer's
//! lens and become labels others can repeat ("big-game player", "flat-track
//! bully", "in a slump", "underrated").

use pw_core::{ClubId, CompId, Date, PlayerId, Pos};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct App {
    pub date: Date,
    pub comp: CompId,
    pub club: ClubId,
    /// Opponent club reputation (0–10,000) for context.
    pub opp_rep: u16,
    pub started: bool,
    pub pos: Option<Pos>,
    pub minutes: u8,
    /// Rating × 10.
    pub rating: u8,
    pub goals: u8,
    pub assists: u8,
    /// Expected goals + assists × 100.
    pub xgi: u16,
    pub key_passes: u8,
    pub tackles_won: u8,
    pub result: i8,
    pub importance: u8,
}

/// Season totals for one player at one club.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct SeasonLine {
    pub year: i32,
    pub club: ClubId,
    pub apps: u16,
    pub starts: u16,
    pub minutes: u32,
    pub goals: u16,
    pub assists: u16,
    pub rating_sum: u32,
    pub xgi: u32,
    pub benched: u16,
    pub omitted: u16,
    pub motm: u16,
    /// Ratings in big matches (importance ≥ 0.75 or strong opponents).
    pub big_sum: u32,
    pub big_apps: u16,
    /// Ratings against weak opponents.
    pub small_sum: u32,
    pub small_apps: u16,
}

impl SeasonLine {
    pub fn avg(&self) -> f32 {
        if self.apps == 0 { 0.0 } else { self.rating_sum as f32 / f32::from(self.apps) / 10.0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Lens {
    Manager,
    Scout,
    Media,
    Fans,
    Analyst,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Label {
    BigGamePlayer,
    FlatTrackBully,
    InForm,
    InSlump,
    /// The numbers say more than the reputation.
    Underrated,
    /// The reputation says more than the numbers.
    Overrated,
    GoalThreat,
    Workhorse,
    Unreliable,
    Breakthrough,
    FrozenOut,
    Durable,
    InjuryProne,
}

impl Label {
    pub const fn text(self) -> &'static str {
        match self {
            Label::BigGamePlayer => "a big-game player",
            Label::FlatTrackBully => "a flat-track bully",
            Label::InForm => "in form",
            Label::InSlump => "in a slump",
            Label::Underrated => "underrated",
            Label::Overrated => "overrated",
            Label::GoalThreat => "a constant goal threat",
            Label::Workhorse => "a workhorse",
            Label::Unreliable => "unreliable",
            Label::Breakthrough => "having a breakthrough",
            Label::FrozenOut => "frozen out",
            Label::Durable => "durable",
            Label::InjuryProne => "injury-prone",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Reading {
    pub lens: Lens,
    pub label: Label,
    pub since: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Perf {
    /// Last appearances, newest last (bounded).
    pub recent: FxHashMap<PlayerId, SmallVec<[App; 10]>>,
    /// Season lines, oldest first.
    pub seasons: FxHashMap<PlayerId, SmallVec<[SeasonLine; 4]>>,
    /// How each lens currently reads a player.
    pub readings: FxHashMap<PlayerId, SmallVec<[Reading; 3]>>,
    /// Newly formed public readings, waiting for the media to pick them up.
    pub fresh: Vec<(PlayerId, Label, Lens, Date)>,
}

pub const RECENT: usize = 10;

impl Perf {
    pub fn line_mut(&mut self, p: PlayerId, year: i32, club: ClubId) -> &mut SeasonLine {
        let v = self.seasons.entry(p).or_default();
        if let Some(i) = v.iter().position(|l| l.year == year && l.club == club) {
            return &mut v[i];
        }
        v.push(SeasonLine { year, club, ..Default::default() });
        v.last_mut().expect("pushed")
    }

    pub fn season(&self, p: PlayerId, year: i32) -> Option<&SeasonLine> {
        self.seasons.get(&p)?.iter().rev().find(|l| l.year == year)
    }

    pub fn has(&self, p: PlayerId, lens: Lens, label: Label) -> bool {
        self.readings.get(&p).is_some_and(|v| v.iter().any(|r| r.lens == lens && r.label == label))
    }

    pub fn labels(&self, p: PlayerId) -> &[Reading] {
        self.readings.get(&p).map_or(&[], |v| v.as_slice())
    }
}
