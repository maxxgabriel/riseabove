//! Voted awards with ballots, halls of fame at every level, and a chronicle
//! of the world's achievements (V in the media brief; 18 in the final
//! brief).
//!
//! **Votes.** Awards decided by people are decided by real people in the
//! world — national team managers and captains, journalists, the players of
//! a league — each ranking candidates through their own lens (performance,
//! trophies, fame, goals, what they have seen) with their own biases (a
//! journalist sees more of their own league; nobody may vote for their own
//! nation's or team's players where the rules say so). Every ballot is
//! kept with the reason each pick was made, so "third in the vote" and
//! "voted for by 61 of 180 journalists" are facts, not flavour.
//!
//! **Halls of fame.** The world's, each nation's, each club's and each
//! school's or university's, each inducting a class chosen by a committee
//! vote with a threshold. Candidates are judged on what the scope cares
//! about: a club remembers apps, goals, trophies and love; a nation its caps
//! and tournaments; a school the alumni who went on to play.
//!
//! **Chronicle.** Firsts and world-level feats: the first double in a
//! nation, an unbeaten league season, the first player from a nation to win
//! the world award, a first tournament win, a third world award — each
//! entry dated and linked to the facts that made it.

use pw_core::{ClubId, CompId, Date, NationId, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// A voted award (what is being decided).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Ballot {
    WorldPlayer {
        young: bool,
    },
    /// Chosen by the players of a league.
    PlayersPlayer {
        comp: CompId,
    },
    /// Best university or school player in a nation, by its journalists.
    MinorPlayer {
        nation: NationId,
        university: bool,
    },
    /// Induction into a hall of fame.
    Hall {
        hall: u32,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum VoterKind {
    NationalManager,
    Journalist,
    Player,
    /// A hall committee member (a legend, a journalist, an official).
    Committee,
}

/// Why a voter ranked a candidate where they did (the largest component of
/// their score for that candidate).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Why {
    Performances,
    Trophies,
    Fame,
    Goals,
    /// Saw them often (same league, same country).
    Familiarity,
    Longevity,
    Loyalty,
    International,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cast {
    pub voter: PersonId,
    pub kind: VoterKind,
    /// Ranked picks with the reason for each.
    pub picks: SmallVec<[(PersonId, Why); 3]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vote {
    pub id: u32,
    pub ballot: Ballot,
    pub year: i32,
    pub date: Date,
    pub casts: Vec<Cast>,
    /// Final tally, highest first.
    pub result: Vec<(PersonId, u32)>,
    /// Voters in total (for "named on N of M ballots").
    pub voters: u32,
}

impl Vote {
    /// How many ballots named someone at all.
    pub fn named_by(&self, p: PersonId) -> usize {
        self.casts.iter().filter(|c| c.picks.iter().any(|x| x.0 == p)).count()
    }

    /// The most common reason voters gave for someone.
    pub fn main_reason(&self, p: PersonId) -> Option<Why> {
        let mut counts: SmallVec<[(Why, u16); 8]> = SmallVec::new();
        for c in &self.casts {
            for &(q, why) in &c.picks {
                if q == p {
                    if let Some(x) = counts.iter_mut().find(|x| x.0 == why) {
                        x.1 += 1;
                    } else {
                        counts.push((why, 1));
                    }
                }
            }
        }
        counts.iter().max_by_key(|x| x.1).map(|x| x.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum HallScope {
    World,
    Nation(NationId),
    Club(ClubId),
    Institution(u32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Member {
    pub person: PersonId,
    pub year: i32,
    /// The vote that elected them.
    pub vote: u32,
    /// Share of the committee that voted for them, 0–100.
    pub share: u8,
    pub score: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hall {
    pub id: u32,
    pub scope: HallScope,
    pub founded: i32,
    pub members: Vec<Member>,
    /// Share of ballots needed, 0–100.
    pub threshold: u8,
    /// Most inducted in one year.
    pub class_size: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Feat {
    /// League and cup in the same season.
    Double { club: ClubId, season: i32 },
    /// A league season without defeat.
    Unbeaten { club: ClubId, comp: CompId, season: i32 },
    /// A nation's first international tournament win.
    FirstTournament { nation: NationId, tournament: u32 },
    /// First player from a nation to win the world award.
    FirstWorldPlayerFrom { nation: NationId, person: PersonId, year: i32 },
    /// Someone won the world award for the Nth time (N ≥ 2).
    WorldPlayerAgain { person: PersonId, times: u8, year: i32 },
    /// A career milestone nobody alive in the world had reached.
    FirstTo { person: PersonId, what: FirstTo },
    /// A club's first league title.
    FirstTitle { club: ClubId, comp: CompId, season: i32 },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum FirstTo {
    SeniorApps(u16),
    CareerGoals(u16),
    Caps(u16),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: u32,
    pub date: Date,
    pub feat: Feat,
    /// Whether it is the first of its kind in its scope ("first double in
    /// the nation's history"), and how many came before.
    pub first: bool,
    pub before: u16,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Acclaim {
    pub votes: Vec<Vote>,
    pub halls: Vec<Hall>,
    pub hall_index: FxHashMap<HallScope, u32>,
    pub chronicle: Vec<Entry>,
    /// Counts per kind of feat and scope, for "first"/"Nth".
    pub counts: FxHashMap<(u8, u32), u16>,
    /// Progress through history for the chronicle scan.
    pub honours_seen: usize,
    pub tournaments_seen: usize,
    pub awards_seen: usize,
    /// "First to" thresholds already claimed.
    pub claimed: SmallVec<[FirstTo; 8]>,
    pub tables_seen: usize,
    /// The latest vote for each ballot.
    pub last_votes: FxHashMap<Ballot, u32>,
}

impl Acclaim {
    pub fn hall(&self, s: HallScope) -> Option<&Hall> {
        self.hall_index.get(&s).map(|&i| &self.halls[i as usize])
    }

    pub fn in_hall(&self, s: HallScope, p: PersonId) -> bool {
        self.hall(s).is_some_and(|h| h.members.iter().any(|m| m.person == p))
    }

    pub fn halls_of(&self, p: PersonId) -> Vec<(HallScope, Member)> {
        self.halls.iter().flat_map(|h| h.members.iter().filter(move |m| m.person == p).map(move |m| (h.scope, *m))).collect()
    }

    /// The last vote for a ballot kind.
    pub fn last(&self, b: Ballot) -> Option<&Vote> {
        self.last_votes.get(&b).map(|&i| &self.votes[i as usize])
    }
}
