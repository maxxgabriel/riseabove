//! Referees, discipline, appeals and stadium atmosphere (11–13 in the final
//! brief).
//!
//! **Referees are people.** Each has a nation, a tier, and tendencies: how
//! strict they are (which the match engine uses), how accurate their big
//! calls are, and how composed they stay under a hostile crowd. They build
//! careers: promotion, demotion, retirement.
//!
//! **Controversies** are the big calls of a match — red cards and penalties.
//! Each has a hidden truth (was it correct?) drawn from the referee's
//! accuracy, independent of which side it favoured: there is no corruption.
//! Supporters of the side it went against feel wronged anyway, more when it
//! was wrong, more still when they are tribal; grievances against one
//! referee accumulate into a *perceived* bias that is not real.
//!
//! **Discipline.** Clubs may appeal red cards; a panel sees the incident
//! imperfectly and rescinds wrong calls more often than right ones, and a
//! frivolous appeal can add a match. Mass confrontations and comments about
//! referees bring misconduct charges and fines.
//!
//! **Atmosphere.** Each senior fixture has an atmosphere from the home
//! supporters' groups (size, mood, voice), the occasion (derby, stakes) and
//! the songs they sing. Its psychological effects are small and bounded.

use pw_core::{ClubId, Date, NationId, PersonId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Referee {
    pub id: u32,
    pub person: PersonId,
    pub nation: NationId,
    /// 1 = top flight.
    pub tier: u8,
    /// 0–100: card-happy.
    pub strictness: u8,
    /// 0–100: chance a big call is right is 0.80 + 0.17 × accuracy / 100.
    pub accuracy: u8,
    /// 0–100: steadiness in a hostile ground (affects only how contested
    /// their calls feel, never which side they favour).
    pub composure: u8,
    pub matches: u32,
    pub reds: u32,
    pub penalties: u32,
    pub big_calls: u32,
    /// Big calls later judged wrong (by reviews and appeals).
    pub wrong: u32,
    pub active: bool,
    pub since: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CallKind {
    RedCard,
    SecondYellow,
    Penalty,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Controversy {
    pub id: u32,
    pub uid: u64,
    pub date: Date,
    pub referee: u32,
    pub kind: CallKind,
    /// The player sent off, or the scorer's side for a penalty.
    pub player: PlayerId,
    /// The club the call went against.
    pub against: ClubId,
    pub benefited: ClubId,
    pub minute: u8,
    /// Hidden truth: was the call correct?
    pub correct: bool,
    /// Whether the call went against the home side.
    pub against_home: bool,
    /// How wronged the losing side's supporters feel, 0–100.
    pub grievance: u8,
    pub appeal: Option<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum AppealOutcome {
    Rescinded,
    Upheld,
    /// A frivolous appeal: the ban grows.
    Extended,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Appeal {
    pub id: u32,
    pub controversy: u32,
    pub club: ClubId,
    pub player: PlayerId,
    pub lodged: Date,
    pub decided: Option<Date>,
    pub outcome: Option<AppealOutcome>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ChargeKind {
    /// Failing to control players (a mass confrontation or a card count).
    FailingToControl,
    /// Public comments questioning a referee's integrity.
    RefereeComments { person: PersonId },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Charge {
    pub id: u32,
    pub club: ClubId,
    pub kind: ChargeKind,
    pub date: Date,
    pub uid: u64,
    pub fine: pw_core::Money,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Atmosphere {
    /// 0–100.
    pub level: u8,
    /// 0–100: how hostile to the visitors.
    pub hostility: u8,
    /// Chants sung (up to three chant ids; `u32::MAX` = none).
    pub chants: [u32; 3],
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Officials {
    pub referees: Vec<Referee>,
    pub by_nation: FxHashMap<NationId, Vec<u32>>,
    pub by_person: FxHashMap<PersonId, u32>,
    /// Fixture uid → referee.
    pub assigned: FxHashMap<u64, u32>,
    pub controversies: Vec<Controversy>,
    pub appeals: Vec<Appeal>,
    pub charges: Vec<Charge>,
    /// A club's supporters' grievance against a referee, 0–1000.
    pub grievance: FxHashMap<(ClubId, u32), u16>,
    /// Fixture uid → atmosphere (recent fixtures only).
    pub atmosphere: FxHashMap<u64, Atmosphere>,
}

impl Officials {
    /// Whether a club's supporters believe a referee is against them
    /// (a perception; referees have no bias).
    pub fn perceived_bias(&self, club: ClubId, referee: u32) -> bool {
        self.grievance.get(&(club, referee)).is_some_and(|&g| g >= 400)
    }
}
