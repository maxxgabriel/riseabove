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

/// What sets off an automatic extension (locked design 5.14).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Trigger {
    /// Appearances since signing.
    Appearances(u16),
    Promotion,
    Title,
    Continental,
    /// International caps since signing.
    Caps(u16),
}

impl Trigger {
    pub fn label(self) -> String {
        match self {
            Trigger::Appearances(n) => format!("{n} appearances"),
            Trigger::Promotion => "promotion".into(),
            Trigger::Title => "winning the title".into(),
            Trigger::Continental => "continental qualification".into(),
            Trigger::Caps(n) => format!("{n} international caps"),
        }
    }
}

/// Options and extensions on a contract. Who holds an option changes what it is worth (locked design 5.14).
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Options {
    /// The club may add this many years at the same wage.
    pub club_years: u8,
    /// The player may add this many years.
    pub player_years: u8,
    /// Both must want it.
    pub mutual_years: u8,
    /// Extra years added automatically when the trigger is met.
    pub auto: Option<(Trigger, u8)>,
    /// One of the above has been used.
    pub used: bool,
}

impl Options {
    pub fn any(&self) -> bool {
        self.club_years > 0 || self.player_years > 0 || self.mutual_years > 0 || self.auto.is_some()
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
    pub assist_bonus: Money,
    pub clean_sheet_bonus: Money,
    /// Paid each year on the anniversary of signing while he is still at the club.
    pub loyalty_bonus: Money,
    pub title_bonus: Money,
    pub promotion_bonus: Money,
    /// Paid when the club qualifies for a continental competition.
    pub continental_bonus: Money,
    /// Per senior international cap.
    pub cap_bonus: Money,
    /// A release clause that only exists after relegation (0 = none).
    pub relegation_release: Money,
    pub options: Options,
    /// His senior appearances and caps when he signed, so triggers can count from there.
    pub apps_base: u16,
    pub caps_base: u16,
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
