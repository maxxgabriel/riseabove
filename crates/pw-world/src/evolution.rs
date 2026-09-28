//! How the game itself changes: tactical schools and the meta (14 in the
//! final brief), and rare rule changes made by institutions for reasons
//! (15).
//!
//! **Schools.** When a manager wins big with a distinctive way of playing,
//! a school of thought is born around them. People who played or coached
//! under a school's managers and later become managers carry its
//! principles, drifting a little; the school's prestige follows its
//! adherents' success and fades when nobody wins with it any more.
//!
//! **Meta.** Each nation's fashionable way to play is pulled toward what
//! its champions did (success is imitated), on top of its slow drift.
//!
//! **Rules.** A federation changes a rule rarely, and only under pressure it
//! can measure: an injury crisis (more substitutes), a red-card epidemic
//! (longer bans), a national team in decline with few homegrown players
//! (a homegrown quota), ties decided on away goals (abolishing the rule).
//! Each change is dated and applies from a season; the rules any past
//! season was played under can be reconstructed.

use pw_core::{Date, NationId, PersonId, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct School {
    pub id: u32,
    /// Generated from the founder's name and style ("the Okafor school").
    pub founder: PersonId,
    pub origin: NationId,
    pub born: Date,
    pub press: u8,
    pub tempo: u8,
    pub direct: u8,
    pub youth: u8,
    /// Managers currently working in its tradition.
    pub adherents: SmallVec<[StaffId; 8]>,
    /// 0–1000.
    pub prestige: u16,
    /// Senior trophies won by its managers.
    pub titles: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum RuleKey {
    /// Substitutes allowed per match (competition rules).
    Subs,
    /// Matches banned for a straight red.
    RedBan,
    /// Minimum homegrown players in a squad.
    HomegrownMin,
    /// Away goals as a tie-breaker (1 = used, 0 = abolished).
    AwayGoals,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum RuleCause {
    /// Injuries per club last season.
    InjuryCrisis { per_club: f32 },
    /// Straight reds per club last season.
    CardEpidemic { per_club: f32 },
    /// The national side's recent record and homegrown share of league minutes.
    NationalDecline { win_rate: f32, homegrown: f32 },
    /// Ties decided on away goals last season.
    AwayGoalsDebate { ties: u16 },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RuleChange {
    pub id: u32,
    pub nation: NationId,
    pub key: RuleKey,
    pub old: i32,
    pub new: i32,
    /// First season played under the new rule.
    pub from_season: i32,
    pub date: Date,
    pub cause: RuleCause,
}

/// A federation's temperament.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Federation {
    /// 0–100: resistance to change.
    pub conservatism: u8,
    pub last_change: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Evolution {
    pub schools: Vec<School>,
    /// A manager's school (by staff id).
    pub school_of: FxHashMap<StaffId, u32>,
    /// A person's formative school (played or coached under it).
    pub formed_by: FxHashMap<PersonId, u32>,
    pub changes: Vec<RuleChange>,
    pub federations: FxHashMap<NationId, Federation>,
}

impl Evolution {
    /// The value of a rule for a nation in a season, given its value today:
    /// undo every change that came into force after that season.
    pub fn value_in(&self, nation: NationId, key: RuleKey, season: i32, today_value: i32) -> i32 {
        let mut v = today_value;
        for c in self.changes.iter().rev().filter(|c| c.nation == nation && c.key == key && c.from_season > season) {
            v = c.old;
        }
        v
    }

    /// The current override of a profile rule, if one was ever changed.
    pub fn current(&self, nation: NationId, key: RuleKey) -> Option<i32> {
        self.changes.iter().rev().find(|c| c.nation == nation && c.key == key).map(|c| c.new)
    }

    pub fn school_of_person(&self, p: PersonId) -> Option<&School> {
        self.formed_by.get(&p).map(|&s| &self.schools[s as usize])
    }
}
