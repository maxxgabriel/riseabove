//! Belief dossiers (locked design section 1.5): what a club's people think of a player, why, and how sure they are.
//!
//! A dossier is not a noisy copy of hidden ability. It holds a judgement (current level, short-term projection, long-term ceiling,
//! direction, risks, best roles) each with a confidence, the evidence it rests on, every staff member's own opinion, how the
//! manager weighed them, and a short history that explains why the judgement moved (section 1.14).

use pw_core::{ClubId, Date, PlayerId, Pos, StaffId};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::staff::StaffRole;

/// Kinds of football judgement (section 1.4). A staff member can be strong in one and weak in another.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Domain {
    CurrentAbility,
    Potential,
    TacticalFit,
    Role,
    Technical,
    Physical,
    Mental,
    Personality,
    Durability,
    Youth,
    Adaptability,
    Trajectory,
}

impl Domain {
    pub const ALL: [Domain; 12] = [
        Domain::CurrentAbility,
        Domain::Potential,
        Domain::TacticalFit,
        Domain::Role,
        Domain::Technical,
        Domain::Physical,
        Domain::Mental,
        Domain::Personality,
        Domain::Durability,
        Domain::Youth,
        Domain::Adaptability,
        Domain::Trajectory,
    ];
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Confidence {
    Low,
    Medium,
    High,
    VeryHigh,
}

impl Confidence {
    pub const fn label(self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
            Confidence::VeryHigh => "very high",
        }
    }
}

/// A belief about a quantity on the ability scale: a best guess and how far it could be out.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Span {
    pub mid: f32,
    pub band: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum RiskKind {
    /// Body still developing, or judged slight for the level.
    Physical,
    /// Decision-making judged inconsistent.
    Decisions,
    /// A record of injuries.
    Injuries,
    /// Attitude or professionalism doubted.
    Attitude,
    /// Level judged to swing from match to match.
    Consistency,
}

/// A risk the evaluators see. `None` in the dossier's risk list means nothing is flagged; a risk that could not be assessed at all is
/// recorded as unknown rather than as absent (section 10.4 in spirit: unknown is not zero).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Risk {
    pub kind: RiskKind,
    /// 0..1, how worrying it is believed to be.
    pub level: f32,
    /// False when the evaluators have too little exposure to judge it.
    pub known: bool,
}

/// How much of each kind of evidence stands behind a judgement (section 1.9).
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Evidence {
    /// Sees him in training every week (he is in one of the club's teams).
    pub training: bool,
    /// Minutes the club has watched, in matches and otherwise.
    pub minutes_seen: u16,
    /// Scout reports on file.
    pub reports: u8,
    /// An analyst covers the club's reading of him.
    pub analytics: bool,
    /// Medical history is available (he is the club's player).
    pub medical: bool,
    /// Days since anyone last saw him.
    pub days_since_seen: u16,
}

/// One evaluator's own reading, made with their own attributes and exposure.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Opinion {
    pub by: StaffId,
    pub role: StaffRole,
    pub ca: Span,
    pub pa: Span,
    /// How much the manager weighed this opinion, 0..1 (sums to 1 over the dossier).
    pub weight: f32,
    /// The manager's trust in this person, 0..1.
    pub trust: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Field {
    Current,
    Ceiling,
    Direction,
}

/// Why a judgement moved.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Reason {
    /// More football watched or trained with.
    MoreEvidence,
    /// A new scouting report changed the picture.
    NewReport,
    /// A different manager reads the same evidence differently.
    NewManager,
    /// Staff arrived or left.
    NewStaff,
    /// An injury record grew.
    Injury,
    /// The player grew, physically or in ability.
    Development,
    /// Old evidence faded and the estimate drifted.
    Stale,
    /// Reassessed with the same evidence and new weighting.
    Reweighed,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Revision {
    pub date: Date,
    pub field: Field,
    pub from: f32,
    pub to: f32,
    pub reason: Reason,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Dossier {
    pub player: PlayerId,
    pub club: ClubId,
    pub date: Date,
    /// The manager (or deciding person) whose weighing this is; a change of manager changes the reading.
    pub viewer: StaffId,
    pub current: Span,
    pub current_confidence: Confidence,
    /// A year from now, at the pace the evaluators expect.
    pub short_term: Span,
    /// Where he is thought to be able to reach.
    pub ceiling: Span,
    pub ceiling_confidence: Confidence,
    /// -1 declining, 0 level, +1 improving, with how sure.
    pub direction: i8,
    pub direction_confidence: Confidence,
    pub risks: SmallVec<[Risk; 4]>,
    /// Best projected positions, best first.
    pub roles: SmallVec<[Pos; 2]>,
    pub evidence: Evidence,
    pub opinions: SmallVec<[Opinion; 6]>,
    /// How far apart the evaluators' readings of his current level are.
    pub spread: f32,
    /// Newest last; a handful kept.
    pub history: SmallVec<[Revision; 4]>,
}

/// How often an evaluator's readings have held up (section 1.7: previous accuracy matters).
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Track {
    pub right: u16,
    pub wrong: u16,
}

impl Track {
    /// -1..1 with little evidence pulling towards 0.
    pub fn record(self) -> f32 {
        (f32::from(self.right) - f32::from(self.wrong)) / (f32::from(self.right) + f32::from(self.wrong) + 4.0)
    }
}

/// An old reading kept until it can be checked against what happened.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub by: StaffId,
    pub club: ClubId,
    pub player: PlayerId,
    pub date: Date,
    pub ca: f32,
    pub pa: f32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Dossiers {
    pub map: FxHashMap<(ClubId, PlayerId), Dossier>,
    pub track: FxHashMap<StaffId, Track>,
    pub pending: Vec<Pending>,
}

impl Dossiers {
    pub fn get(&self, club: ClubId, p: PlayerId) -> Option<&Dossier> {
        self.map.get(&(club, p))
    }
}
