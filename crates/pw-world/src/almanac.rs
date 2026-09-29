//! The almanac: careers as counters, all-time leaderboards and streaks.
//!
//! The record book (`records`) keeps the best mark and who held it before.
//! The almanac keeps everything underneath: each player's running career
//! (goals, assists, clean sheets, cards, streaks, how fast they reached their
//! first goals), per-competition tallies and the top of every table, so any
//! question of the form "who has scored most, fastest, most often" has an
//! answer from data that was written when it happened.

use pw_core::{ClubId, CompId, Date, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::records::{Scope, Stat};

/// The goal milestones tracked for "fastest to N".
pub const MILESTONES: [u32; 4] = [1, 10, 50, 100];

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct CareerLine {
    pub apps: u32,
    pub goals: u32,
    pub assists: u32,
    pub clean_sheets: u32,
    pub yellows: u16,
    pub reds: u16,
    pub hat_tricks: u16,
    pub motm: u16,
    pub debut: Date,
    /// Appearances when each milestone was reached (0 = not yet).
    pub apps_to: [u16; 4],
    /// Days since debut when each milestone was reached (0 = not yet).
    pub days_to: [u16; 4],
    pub goal_streak: u16,
    pub best_goal_streak: u16,
    pub app_streak: u16,
    pub best_app_streak: u16,
    /// Best single-match rating ×10.
    pub best_rating: u8,
    /// Best top speed, tenths of km/h.
    pub top_speed: u16,
    pub best_distance_m: u16,
    /// Keeper minutes since last conceding, and the best such run.
    pub gk_run: u32,
    pub best_gk_run: u32,
}

/// The top of one table.
pub type Board = SmallVec<[(PersonId, i64); 8]>;

pub const BOARD_LEN: usize = 20;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Almanac {
    pub career: FxHashMap<PersonId, CareerLine>,
    /// (apps, goals, assists) per player per competition, all time.
    pub comp_tally: FxHashMap<(CompId, PersonId), (u16, u16, u16)>,
    pub boards: FxHashMap<(Scope, Stat), Board>,
    /// Per club: (current losing run, current home unbeaten run).
    pub runs: FxHashMap<ClubId, (u16, u16)>,
    /// Per competition: the club on a run of titles and the run.
    pub title_run: FxHashMap<CompId, (ClubId, u16)>,
    pub golden_boots: FxHashMap<PersonId, u16>,
}

impl Almanac {
    /// Offer a value to a table; keeps the best `BOARD_LEN`, one entry per person.
    pub fn offer(&mut self, scope: Scope, stat: Stat, who: PersonId, value: i64) {
        let lower = stat.lower_is_better();
        let b = self.boards.entry((scope, stat)).or_default();
        if let Some(e) = b.iter_mut().find(|e| e.0 == who) {
            if if lower { value < e.1 } else { value > e.1 } {
                e.1 = value;
            }
        } else if b.len() < BOARD_LEN || b.last().is_some_and(|l| if lower { value < l.1 } else { value > l.1 }) {
            b.push((who, value));
        } else {
            return;
        }
        b.sort_by(|x, y| if lower { x.1.cmp(&y.1) } else { y.1.cmp(&x.1) }.then(x.0.cmp(&y.0)));
        b.truncate(BOARD_LEN);
    }

    /// The top `n` of a table.
    pub fn leaders(&self, scope: Scope, stat: Stat, n: usize) -> &[(PersonId, i64)] {
        self.boards.get(&(scope, stat)).map_or(&[], |b| &b[..n.min(b.len())])
    }
}
