//! Careers for staff other than managers (managers have `careers::ManagerProfile`).
//! Created lazily at a person's first move; a legacy record starts with one open job
//! since `Staff.joined` and never invents earlier history.

use pw_core::{ClubId, Date, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::staff::StaffRole;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StaffEnd {
    /// Contract ran out and one side declined to renew.
    Expired,
    /// Taken by a bigger club.
    Poached,
    Retired,
    /// Promoted into another role.
    Promoted,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StaffJob {
    pub club: ClubId,
    pub role: StaffRole,
    pub from: Date,
    pub to: Option<Date>,
    pub ended: Option<StaffEnd>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StaffCareer {
    pub jobs: Vec<StaffJob>,
    /// Managers this person served under before moving up; their ideas carry.
    pub mentors: SmallVec<[StaffId; 2]>,
}

/// Owned by `pw_sim::stafflife`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct StaffExt {
    pub careers: FxHashMap<StaffId, StaffCareer>,
}
