//! Football below the professional game (04 §6, 09 §2).
//!
//! Local clubs are where most footballers start and where many end up:
//! grassroots sides for children, amateur and semi-professional sides for
//! adults. Professional academies recruit from them — some from their own
//! town, some from across the world — through feeder links and youth scouts,
//! invite children on trials, review every age group each summer, offer
//! scholarships at sixteen and expect schoolwork to be done. Most children
//! are released at some point; some are found again.

use pw_core::{ClubId, Date, LocalClubId, Money, NationId, PersonId, PlayerId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LocalLevel {
    /// Children's football (roughly 8–15).
    Grassroots,
    /// Adult amateur / semi-professional football.
    Amateur,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalClub {
    pub name: String,
    pub nation: NationId,
    pub city: String,
    pub level: LocalLevel,
    /// Quality of volunteer/part-time coaching, 1–20.
    pub coaching: u8,
    pub facilities: u8,
    /// Professional club this side feeds (`NONE` = independent).
    pub feeder_of: ClubId,
    /// Local standing, 0–1000 (how visible its players are to scouts).
    pub standing: u16,
    pub members: Vec<PlayerId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Reach {
    Local,
    Regional,
    National,
    International,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum AcademyStyle {
    /// Technique first.
    Technical,
    /// Size, speed and power.
    Athletic,
    Balanced,
    /// Local lads first.
    Community,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Academy {
    pub club: ClubId,
    pub reach: Reach,
    pub style: AcademyStyle,
    /// 0–100: share of each age group released at review.
    pub strictness: u8,
    /// −10..10: how much physically advanced children are over-rated.
    pub maturity_bias: i8,
    /// Season budget for recruitment and staffing.
    pub budget: Money,
    pub feeders: SmallVec<[LocalClubId; 4]>,
    /// Weekly study hours required of academy players and scholars.
    pub study_required: u8,
    /// Most children the academy keeps per age group.
    pub places: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AcademyTrial {
    pub player: PlayerId,
    pub club: ClubId,
    pub from: Date,
    pub until: Date,
}

/// Schooling for children and teenagers (10 §3).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct School {
    /// 0–100.
    pub grades: u8,
    pub attendance: u8,
    /// Key exams sat (at 16) and passed.
    pub exams_passed: bool,
}

impl Default for School {
    fn default() -> Self {
        Self { grades: 55, attendance: 92, exams_passed: false }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Release {
    pub club: ClubId,
    pub date: Date,
    /// Age at release.
    pub age: u8,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Youth {
    pub local: pw_core::IdVec<LocalClubId, LocalClub>,
    pub member_of: FxHashMap<PlayerId, LocalClubId>,
    pub academies: FxHashMap<ClubId, Academy>,
    pub trials: Vec<AcademyTrial>,
    pub school: FxHashMap<PersonId, School>,
    /// Every release from an academy, for rediscovery and for the history books.
    pub released: FxHashMap<PlayerId, SmallVec<[Release; 2]>>,
    pub last_cohort: i32,
}

impl Youth {
    pub fn join(&mut self, p: PlayerId, club: LocalClubId) {
        self.leave(p);
        self.local[club].members.push(p);
        self.member_of.insert(p, club);
    }

    pub fn leave(&mut self, p: PlayerId) {
        if let Some(l) = self.member_of.remove(&p) {
            self.local[l].members.retain(|&x| x != p);
        }
    }
}
