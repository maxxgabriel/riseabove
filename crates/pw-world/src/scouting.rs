//! Scouting as an information network (03 §11, 08 §1). Scouts are people
//! sent to places; they can only watch so many matches, they know some
//! countries better than others, they carry biases, and what they file is
//! their opinion on a date. Clubs decide from reports, which go stale.
//! Two scouts watching the same player can — and do — disagree.

use pw_core::{Attr, ClubId, CompId, Date, NationId, PlayerId, PosGroup, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Brief {
    /// Cover a country's football generally.
    Nation(NationId),
    /// Cover a specific competition.
    Competition(CompId),
    /// Youth and grassroots football in a country.
    Youth(NationId),
    /// Watch one named player.
    Player(PlayerId),
    /// Find options for a position group.
    Need(PosGroup),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Assignment {
    pub scout: StaffId,
    pub club: ClubId,
    pub brief: Brief,
    pub since: Date,
    pub until: Date,
}

/// A scout's leanings, −10..10 each.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct ScoutBias {
    /// Over-rates athletes (+) or technicians (−).
    pub physical: i8,
    /// Over-rates flair and dribblers.
    pub flair: i8,
    /// Over-rates youthful potential.
    pub youth: i8,
    /// Over-rates players from their own country.
    pub home: i8,
    /// Over-rates what they see in weak competitions.
    pub context_blind: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoutProfile {
    pub staff: StaffId,
    /// Where the scout is based now (travel takes time).
    pub based: NationId,
    /// Familiarity with each nation's football, 0–100.
    pub knows: SmallVec<[(NationId, u8); 4]>,
    pub bias: ScoutBias,
    /// Matches they can attend in a week.
    pub capacity: u8,
}

impl ScoutProfile {
    pub fn familiarity(&self, n: NationId) -> u8 {
        self.knows.iter().find(|(x, _)| *x == n).map_or(0, |&(_, f)| f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Verdict {
    /// Sign now.
    Sign,
    /// Keep watching.
    Monitor,
    /// Not for us.
    Pass,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Note {
    Strength(Attr),
    Weakness(Attr),
    GoodAttitude,
    PoorAttitude,
    InjuryConcern,
    LateDeveloper,
    PhysicallyAhead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub scout: StaffId,
    pub date: Date,
    /// Minutes of football this report rests on (cumulative for this scout).
    pub minutes: u16,
    /// Current ability as estimated (CA scale) and its band.
    pub ca: u8,
    pub pa: u8,
    pub band: u8,
    /// Recommendation grade 1 (poor) – 5 (outstanding) for this club.
    pub grade: u8,
    pub verdict: Verdict,
    pub notes: SmallVec<[Note; 3]>,
    /// Reputation of the competition it was seen in.
    pub context: u16,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Scouting {
    pub profiles: FxHashMap<StaffId, ScoutProfile>,
    pub assignments: Vec<Assignment>,
    /// Latest reports per (club, player), newest last (a few kept).
    pub reports: FxHashMap<(ClubId, PlayerId), SmallVec<[Report; 3]>>,
}

impl Scouting {
    pub fn file(&mut self, club: ClubId, player: PlayerId, r: Report) {
        let list = self.reports.entry((club, player)).or_default();
        list.push(r);
        if list.len() > 3 {
            list.remove(0);
        }
    }

    pub fn of(&self, club: ClubId, player: PlayerId) -> &[Report] {
        self.reports.get(&(club, player)).map_or(&[], |v| v.as_slice())
    }

    pub fn assignments_of(&self, scout: StaffId) -> impl Iterator<Item = &Assignment> {
        self.assignments.iter().filter(move |a| a.scout == scout)
    }

    /// Drop reports nobody would still trust.
    pub fn forget(&mut self, before: Date) {
        for l in self.reports.values_mut() {
            l.retain(|r| r.date >= before);
        }
        self.reports.retain(|_, l| !l.is_empty());
    }
}
