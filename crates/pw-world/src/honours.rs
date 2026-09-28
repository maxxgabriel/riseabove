//! Memory at the scale of institutions (11 §8): records, milestones, club
//! legends, a hall of fame, and the tallies they are computed from.
//! Honours (trophies) and season awards live in `history`; this is what a
//! club historian or a retrospective writer would look up.

use pw_core::{ClubId, CompId, Date, Money, NationId, PersonId, PlayerId, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// All-time appearances and goals for a player at a club (senior sides).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Tally {
    pub apps: u16,
    pub goals: u16,
    pub trophies: u8,
    pub first: Date,
    pub last: Date,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Holder {
    pub player: PlayerId,
    pub value: i64,
    pub date: Date,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ClubRecords {
    pub top_scorer: Holder,
    pub most_apps: Holder,
    pub record_signing: Holder,
    pub record_sale: Holder,
    /// Margin; `player` unused.
    pub biggest_win: Holder,
    pub biggest_win_against: ClubId,
    pub legends: SmallVec<[PersonId; 8]>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CompRecords {
    pub goals_in_season: Holder,
    pub goals_in_season_year: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NationRecords {
    pub most_caps: Holder,
    pub top_scorer: Holder,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Inductee {
    pub person: PersonId,
    pub date: Date,
    /// Career score that earned it.
    pub score: u32,
}

/// Votes in a global award, kept for season reviews ("third in the vote").
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vote {
    pub year: i32,
    pub young: bool,
    pub ranking: Vec<(PlayerId, u32)>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Honours {
    pub tallies: FxHashMap<(ClubId, PlayerId), Tally>,
    pub clubs: FxHashMap<ClubId, ClubRecords>,
    pub comps: FxHashMap<CompId, CompRecords>,
    pub nations: FxHashMap<NationId, NationRecords>,
    pub world_fee: Holder,
    pub hall: Vec<Inductee>,
    pub votes: Vec<Vote>,
    pub managers_of_season: Vec<(i32, CompId, StaffId)>,
    /// Players of the month (year, month, comp, player).
    pub monthly: Vec<(i32, u8, CompId, PlayerId)>,
    /// Fees already considered for records (transfer event ids processed up to).
    pub transfers_seen: u32,
    pub last_fee: Money,
}

impl Honours {
    pub fn tally(&self, club: ClubId, p: PlayerId) -> Tally {
        self.tallies.get(&(club, p)).copied().unwrap_or_default()
    }

    pub fn is_legend(&self, club: ClubId, person: PersonId) -> bool {
        self.clubs.get(&club).is_some_and(|r| r.legends.contains(&person))
    }

    pub fn in_hall(&self, person: PersonId) -> bool {
        self.hall.iter().any(|i| i.person == person)
    }
}
