use pw_core::{ClubId, Date, Money};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum ContractKind {
    #[default]
    None,
    Youth,
    Professional,
    Amateur,
}

/// Squad status (03 §9). Sets expectations for minutes and wage bracket.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord, Default)]
pub enum SquadStatus {
    Star,
    Important,
    Regular,
    #[default]
    Squad,
    ImpactSub,
    Fringe,
    Backup,
    Youngster,
    NotNeeded,
}

impl SquadStatus {
    pub const ALL: [SquadStatus; 9] = [
        SquadStatus::Star,
        SquadStatus::Important,
        SquadStatus::Regular,
        SquadStatus::Squad,
        SquadStatus::ImpactSub,
        SquadStatus::Fringe,
        SquadStatus::Backup,
        SquadStatus::Youngster,
        SquadStatus::NotNeeded,
    ];

    /// Expected share of available league minutes.
    pub const fn expected_minutes(self) -> f32 {
        match self {
            SquadStatus::Star => 0.85,
            SquadStatus::Important => 0.75,
            SquadStatus::Regular => 0.6,
            SquadStatus::Squad => 0.35,
            SquadStatus::ImpactSub => 0.25,
            SquadStatus::Fringe => 0.12,
            SquadStatus::Backup => 0.05,
            SquadStatus::Youngster => 0.1,
            SquadStatus::NotNeeded => 0.0,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            SquadStatus::Star => "Star Player",
            SquadStatus::Important => "Important Player",
            SquadStatus::Regular => "Regular Starter",
            SquadStatus::Squad => "Squad Player",
            SquadStatus::ImpactSub => "Impact Sub",
            SquadStatus::Fringe => "Fringe Player",
            SquadStatus::Backup => "Emergency Backup",
            SquadStatus::Youngster => "Hot Prospect",
            SquadStatus::NotNeeded => "Not Needed",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Contract {
    pub club: ClubId,
    pub kind: ContractKind,
    /// Weekly gross wage.
    pub wage: Money,
    pub start: Date,
    pub end: Date,
    /// 0 = no release clause.
    pub release_clause: Money,
    pub promised_status: Option<SquadStatus>,
    /// Percent wage rise each contract year.
    pub yearly_rise: u8,
    /// Percent wage cut on relegation.
    pub relegation_cut: u8,
    pub appearance_bonus: Money,
    pub goal_bonus: Money,
}

impl Contract {
    #[inline]
    pub fn is_active(&self, today: Date) -> bool {
        self.kind != ContractKind::None && self.club.is_some() && today <= self.end
    }

    #[inline]
    pub fn days_left(&self, today: Date) -> i32 {
        today.days_until(self.end)
    }

    /// Wage paid this week, including yearly rises.
    pub fn current_wage(&self, today: Date) -> Money {
        let years = (self.start.days_until(today) / 365).max(0);
        let mut w = self.wage as f64;
        for _ in 0..years {
            w *= 1.0 + f64::from(self.yearly_rise) / 100.0;
        }
        w as Money
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loan {
    pub parent: ClubId,
    pub club: ClubId,
    pub start: Date,
    pub end: Date,
    /// Percent of wage paid by the loan club.
    pub wage_share: u8,
    pub fee: Money,
    /// 0 = no option.
    pub buy_option: Money,
    pub recall: bool,
}
