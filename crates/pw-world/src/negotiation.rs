//! Contract negotiations (08 §5): multi-round talks over full terms. The same
//! state machine serves AI players (resolved within a day) and external minds
//! (who answer through the UI, usually via their agent).

use pw_core::{ClubId, Date, Money, PlayerId};
use serde::{Deserialize, Serialize};

use crate::contract::{Contract, ContractKind, Loan, SquadStatus};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Terms {
    /// Weekly wage.
    pub wage: Money,
    pub years: u8,
    pub signing_fee: Money,
    pub appearance_bonus: Money,
    pub goal_bonus: Money,
    pub clean_sheet_bonus: Money,
    /// 0 = none.
    pub release_clause: Money,
    pub status: Option<SquadStatus>,
    pub yearly_rise: u8,
    pub relegation_cut: u8,
    /// Share of a future transfer fee paid to the player (rare), percent.
    pub sell_on_to_player: u8,
}

impl Terms {
    pub fn to_contract(&self, club: ClubId, kind: ContractKind, start: Date) -> Contract {
        Contract {
            club,
            kind,
            wage: self.wage,
            start,
            end: start.add_months(12 * i32::from(self.years)),
            release_clause: self.release_clause,
            promised_status: self.status,
            yearly_rise: self.yearly_rise,
            relegation_cut: self.relegation_cut,
            appearance_bonus: self.appearance_bonus,
            goal_bonus: self.goal_bonus,
        }
    }

    /// Rough total value to the player over the deal, for comparing offers.
    pub fn worth(&self) -> f64 {
        let years = f64::from(self.years.max(1));
        let wages = self.wage as f64 * 52.0 * years * (1.0 + f64::from(self.yearly_rise) / 100.0 * (years - 1.0) / 2.0);
        let bonus = (self.appearance_bonus as f64 * 30.0 + self.goal_bonus as f64 * 5.0 + self.clean_sheet_bonus as f64 * 8.0) * years;
        wages + bonus + self.signing_fee as f64
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TalkKind {
    Renewal,
    /// Personal terms after clubs agreed a fee.
    Transfer,
    FreeAgent,
    /// First professional deal for an academy player.
    FirstPro,
    /// A loan's player-side agreement (wage share is the club's business).
    Loan,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TalkState {
    /// Waiting for the player's side to respond.
    PlayerTurn,
    /// Waiting for the club to respond to a counter.
    ClubTurn,
    Agreed,
    Collapsed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Negotiation {
    pub player: PlayerId,
    pub club: ClubId,
    pub kind: TalkKind,
    /// Selling club and agreed fee for transfers.
    pub seller: ClubId,
    pub fee: Money,
    pub loan: Option<Loan>,
    pub offer: Terms,
    pub ask: Option<Terms>,
    /// Club's private ceiling; never shown to the player.
    pub limit: Terms,
    pub round: u8,
    pub max_rounds: u8,
    pub opened: Date,
    pub deadline: Date,
    pub state: TalkState,
    /// Human-readable log of the talks, newest last.
    pub log: Vec<(Date, String)>,
}

impl Negotiation {
    pub fn is_open(&self) -> bool {
        matches!(self.state, TalkState::PlayerTurn | TalkState::ClubTurn)
    }
}
