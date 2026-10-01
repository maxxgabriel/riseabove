//! History before the start date (17 in the final brief).
//!
//! A world begins with a past: champions and runners-up of each top
//! flight for decades before day one, the goals of each season's top
//! scorer, club legends, and the records they set. Some worlds import
//! their past (`history.csv`); otherwise it is generated from the world
//! seed and the clubs' standing. **Every entry says which it is.** Text
//! about generated history says so, and nothing generated is ever
//! attributed to a real, imported person: past figures in generated
//! history are generated people with names from their nation's pool.

use pw_core::{ClubId, CompId, NationId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Provenance {
    /// From the data pack (`history.csv`).
    Imported,
    /// Generated for this world from its seed.
    Generated,
}

/// A figure of the past (not a live person in the world).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PastFigure {
    pub id: u32,
    pub name: String,
    pub nation: NationId,
    pub club: ClubId,
    pub born: i32,
    pub apps: u16,
    pub goals: u16,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PastSeason {
    pub comp: CompId,
    /// The year the season started.
    pub season: i32,
    pub champion: ClubId,
    pub runner_up: ClubId,
    /// A past figure (`u32::MAX` if unknown).
    pub top_scorer: u32,
    pub top_goals: u16,
    pub provenance: Provenance,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Backfill {
    pub figures: Vec<PastFigure>,
    pub seasons: Vec<PastSeason>,
    /// The first and last seasons covered.
    pub from: i32,
    pub to: i32,
    pub done: bool,
    /// People in the world when the past was generated (ids below this are
    /// the world's own people; generated names must not match theirs).
    pub people_at: u32,
}

impl Backfill {
    /// Past titles of a club in a competition.
    pub fn titles(&self, club: ClubId, comp: CompId) -> usize {
        self.seasons.iter().filter(|s| s.champion == club && s.comp == comp).count()
    }

    pub fn seasons_of(&self, comp: CompId) -> impl Iterator<Item = &PastSeason> {
        self.seasons.iter().filter(move |s| s.comp == comp)
    }

    pub fn last_title(&self, club: ClubId) -> Option<i32> {
        self.seasons.iter().filter(|s| s.champion == club).map(|s| s.season).max()
    }
}
