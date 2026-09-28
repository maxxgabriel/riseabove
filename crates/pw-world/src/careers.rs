//! Manager careers (07 §2). Beyond the numeric philosophy each manager has a
//! football identity — the kinds of players they love and distrust, how much
//! they rotate, how loyal they are to veterans and to players they've had
//! before, how they deal with the press, which staff follow them — and a CV
//! of jobs that ended in success, sackings, resignations or being poached.

use pw_core::{ClubId, Date, PersonId, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::staff::ManagerRecord;

/// Player profiles managers tend to have strong views about.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Archetype2 {
    TallDefender,
    BallPlayingDefender,
    AttackingFullBack,
    Destroyer,
    Playmaker,
    BoxToBox,
    PacyWinger,
    Technician,
    TargetMan,
    Poacher,
    SweeperKeeper,
    Workhorse,
}

impl Archetype2 {
    pub const ALL: [Archetype2; 12] = [
        Archetype2::TallDefender,
        Archetype2::BallPlayingDefender,
        Archetype2::AttackingFullBack,
        Archetype2::Destroyer,
        Archetype2::Playmaker,
        Archetype2::BoxToBox,
        Archetype2::PacyWinger,
        Archetype2::Technician,
        Archetype2::TargetMan,
        Archetype2::Poacher,
        Archetype2::SweeperKeeper,
        Archetype2::Workhorse,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Archetype2::TallDefender => "tall, commanding defenders",
            Archetype2::BallPlayingDefender => "ball-playing defenders",
            Archetype2::AttackingFullBack => "attacking full-backs",
            Archetype2::Destroyer => "ball-winning midfielders",
            Archetype2::Playmaker => "playmakers",
            Archetype2::BoxToBox => "box-to-box midfielders",
            Archetype2::PacyWinger => "pacy wingers",
            Archetype2::Technician => "technicians",
            Archetype2::TargetMan => "target men",
            Archetype2::Poacher => "poachers",
            Archetype2::SweeperKeeper => "sweeper keepers",
            Archetype2::Workhorse => "hard-working team players",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MediaStyle {
    Guarded,
    Combative,
    Charming,
    Candid,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum JobEnd {
    Sacked,
    Resigned,
    Poached,
    ContractExpired,
    Retired,
}

impl JobEnd {
    pub const fn label(self) -> &'static str {
        match self {
            JobEnd::Sacked => "sacked",
            JobEnd::Resigned => "resigned",
            JobEnd::Poached => "left for another club",
            JobEnd::ContractExpired => "contract expired",
            JobEnd::Retired => "retired",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub club: ClubId,
    pub from: Date,
    pub to: Option<Date>,
    pub ended: Option<JobEnd>,
    /// Record at the start of the job, to compute the spell's own record.
    pub record_at_start: ManagerRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManagerProfile {
    pub staff: StaffId,
    pub likes: SmallVec<[Archetype2; 2]>,
    pub dislikes: Option<Archetype2>,
    /// 0–100 each.
    pub rotation: u8,
    pub veteran_loyalty: u8,
    /// Preference for players he has managed before.
    pub favouritism: u8,
    pub adaptability: u8,
    pub ambition: u8,
    /// How much say he demands in recruitment.
    pub recruitment_influence: u8,
    pub media_style: MediaStyle,
    /// Staff who follow him from job to job.
    pub entourage: SmallVec<[StaffId; 3]>,
    /// Managerial reputation, 0–10,000 (separate from reputation as a player).
    pub reputation: u16,
    pub jobs: Vec<Job>,
    /// Formations he has used, most recent last.
    pub systems: SmallVec<[u8; 4]>,
    /// Players he rates from previous jobs.
    pub favourites: SmallVec<[PersonId; 8]>,
}

impl ManagerProfile {
    pub fn current_job(&self) -> Option<&Job> {
        self.jobs.last().filter(|j| j.to.is_none())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Careers {
    pub managers: FxHashMap<StaffId, ManagerProfile>,
}
