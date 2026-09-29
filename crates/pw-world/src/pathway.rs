//! Why a player went where he went, and where a player came from (the India brief, items 13 and 22). A route that says only
//! "2028 Club X" cannot be explained; this keeps the cause, and it never invents one for a step that happened before it was kept.

use pw_core::{ClubId, Date, PersonId, PlayerId, RegionId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::ecosystem::{Provider, StageKind};

/// What brought a player to a step.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Why {
    /// Drawn from the district's pool as a child who plays.
    Emerged,
    /// An academy invited him.
    AcademyInvite { club: ClubId },
    /// An academy let him go.
    ReleasedByAcademy { club: ClubId },
    /// A university (institution id) gave him a place, on this scholarship tier (0 none).
    UniversityScholarship { inst: u32, tier: u8 },
    /// Picked for a state side on this ground.
    StateSelection { state: RegionId },
    /// The district picked him.
    DistrictSelection { district: RegionId },
    /// Called to a national camp.
    CampCall,
    /// Invited to trial by a club.
    ProfessionalTrial { club: ClubId },
    /// A scout took him to a club.
    ScoutRecommendation { scout: PersonId, club: ClubId },
    /// A coach or teacher recommended him, and this is who.
    CoachRecommendation { from: crate::recog::Source },
    /// Moved with his school or family, or enrolled where he lived.
    Enrolled,
    /// Signed as a professional.
    Signed { club: ClubId },
    /// Moved for a fee.
    Transfer { to: ClubId },
    /// His club went up a division.
    Promotion { club: ClubId },
    /// Began here when the person was created for the player to inhabit.
    ChosenStart,
}

impl Why {
    pub fn text(self) -> &'static str {
        match self {
            Why::Emerged => "began playing organised football",
            Why::AcademyInvite { .. } => "was invited by an academy",
            Why::ReleasedByAcademy { .. } => "was released by an academy",
            Why::UniversityScholarship { .. } => "was given a university scholarship",
            Why::StateSelection { .. } => "was selected for the state side",
            Why::DistrictSelection { .. } => "was picked by the district",
            Why::CampCall => "was called to a national identification camp",
            Why::ProfessionalTrial { .. } => "was invited to a professional trial",
            Why::ScoutRecommendation { .. } => "was recommended to a club by a scout",
            Why::CoachRecommendation { .. } => "was recommended by his coach",
            Why::Enrolled => "enrolled where he lived",
            Why::Signed { .. } => "signed as a professional",
            Why::Transfer { .. } => "was transferred",
            Why::Promotion { .. } => "went up with his club",
            Why::ChosenStart => "started his story here",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Reason {
    pub date: Date,
    pub kind: StageKind,
    pub why: Why,
}

/// How a real player came to exist (item 22): no player appears unexplained.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Creation {
    pub date: Date,
    /// The district he grew up in.
    pub region: RegionId,
    pub provider: Provider,
    /// The institution (school or university id) he first played for, if any.
    pub institution: Option<u32>,
    /// His age when he became a real player; unknown for a save that did not keep it (never zero).
    pub age: Option<u8>,
    /// The first competitive setting he played in.
    pub first_env: StageKind,
    /// Who first took him seriously.
    pub first_finder: PersonId,
    /// Why he was drawn out of the pool now.
    pub why: Draw,
    /// True when this record was derived from an older save's player story rather than kept at the time.
    pub legacy: bool,
}

/// Why a child from the aggregate pool became a real player.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Draw {
    /// Reached competitive age and joined organised football.
    Competitive,
    /// Noticed early by someone with a reason to look.
    Noticed,
    /// Placed in the world as a person to inhabit.
    Chosen,
    /// A save from before creation was recorded.
    Unrecorded,
}

/// Owned by `pw_sim::pathway`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct PathwayExt {
    pub why: FxHashMap<PlayerId, SmallVec<[Reason; 8]>>,
    pub created: FxHashMap<PlayerId, Creation>,
}

impl PathwayExt {
    /// The reasons for a player's steps, oldest first.
    pub fn of(&self, p: PlayerId) -> &[Reason] {
        self.why.get(&p).map_or(&[], |v| v.as_slice())
    }

    /// The reason recorded for a step of this kind on this date, if one was kept.
    pub fn for_step(&self, p: PlayerId, kind: StageKind, date: Date) -> Option<Why> {
        self.of(p).iter().rev().find(|r| r.kind == kind && r.date == date).map(|r| r.why)
    }
}
