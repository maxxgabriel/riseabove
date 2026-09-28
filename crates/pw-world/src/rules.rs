//! Explainable rule checks (02 §3): every refusal carries reasons the UI can
//! show ("why can't I play/sign?").

use pw_core::{ClubId, Date, PlayerId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::comp::CompRules;
use crate::player::PlayerStatus;
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Reason {
    WindowClosed,
    SquadFull { max: u8 },
    Suspended { matches: u8 },
    Injured { days: u16 },
    Retired,
    TooYoung { min: u8 },
    MinorAbroad,
    AlreadyAtClub,
}

impl Reason {
    pub fn text(&self) -> String {
        match *self {
            Reason::WindowClosed => "The registration window is closed.".into(),
            Reason::SquadFull { max } => format!("The squad is at its limit of {max} players."),
            Reason::Suspended { matches } => format!("Suspended for {matches} more match(es)."),
            Reason::Injured { days } => format!("Injured, about {days} days out."),
            Reason::Retired => "Retired.".into(),
            Reason::TooYoung { min } => format!("Players must be {min} to sign a professional contract."),
            Reason::MinorAbroad => "International transfers of players under 18 are not permitted.".into(),
            Reason::AlreadyAtClub => "Already registered with this club.".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuleOutcome {
    pub reasons: SmallVec<[Reason; 2]>,
}

impl RuleOutcome {
    #[inline]
    pub fn allowed(&self) -> bool {
        self.reasons.is_empty()
    }

    #[inline]
    fn deny(&mut self, r: Reason) {
        self.reasons.push(r);
    }
}

pub const MIN_PRO_AGE: u8 = 16;
pub const MIN_ACADEMY_AGE: u8 = 14;

pub fn max_contract_years(age: u32) -> u8 {
    if age < 18 { 3 } else { 5 }
}

pub fn can_play(w: &World, p: PlayerId, _rules: &CompRules) -> RuleOutcome {
    let h = &w.players.hot[p];
    let mut o = RuleOutcome::default();
    if h.status == PlayerStatus::Retired {
        o.deny(Reason::Retired);
    }
    if h.injury != 0 {
        o.deny(Reason::Injured { days: h.injury_days });
    }
    if h.ban > 0 {
        o.deny(Reason::Suspended { matches: h.ban });
    }
    o
}

/// Can `club` register `p` today (permanent or loan move)?
pub fn can_sign(w: &World, club: ClubId, p: PlayerId, today: Date) -> RuleOutcome {
    let mut o = RuleOutcome::default();
    let h = &w.players.hot[p];
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    let buyer = &w.clubs[club];
    if h.club == club {
        o.deny(Reason::AlreadyAtClub);
    }
    if h.status == PlayerStatus::Retired {
        o.deny(Reason::Retired);
    }
    let free_agent = h.status == PlayerStatus::FreeAgent;
    if !free_agent && !w.nations[buyer.nation].season.window_open(today) {
        o.deny(Reason::WindowClosed);
    }
    let age = person.age(today);
    if age < u32::from(MIN_PRO_AGE) {
        o.deny(Reason::TooYoung { min: MIN_PRO_AGE });
    }
    if age < 18 && person.nation != buyer.nation {
        let from_nation = if h.club.is_some() { w.clubs[h.club].nation } else { person.nation };
        if from_nation != buyer.nation {
            o.deny(Reason::MinorAbroad);
        }
    }
    let max = w.data.tuning.squad.first_team_max;
    let size = w.clubs[club].teams.first().map_or(0, |&t| w.teams[t].squad.len());
    if size >= usize::from(max) + 4 {
        o.deny(Reason::SquadFull { max });
    }
    o
}
