use pw_core::{ClubId, CompId, Date, Money, NationId, PlayerId, PosGroup, StaffId, Tactics, TeamId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum TeamKind {
    First,
    Reserve,
    U21,
    U19,
    U18,
    /// Academy age groups below the scholarship years.
    U16,
    U14,
    U12,
}

impl TeamKind {
    pub const fn label(self) -> &'static str {
        match self {
            TeamKind::First => "First Team",
            TeamKind::Reserve => "Reserves",
            TeamKind::U21 => "U21",
            TeamKind::U19 => "U19",
            TeamKind::U18 => "U18",
            TeamKind::U16 => "U16",
            TeamKind::U14 => "U14",
            TeamKind::U12 => "U12",
        }
    }

    /// Oldest age allowed at the start of the season, if restricted.
    pub const fn max_age(self) -> Option<u32> {
        match self {
            TeamKind::First | TeamKind::Reserve => None,
            TeamKind::U21 => Some(21),
            TeamKind::U19 => Some(19),
            TeamKind::U18 => Some(18),
            TeamKind::U16 => Some(16),
            TeamKind::U14 => Some(14),
            TeamKind::U12 => Some(12),
        }
    }

    pub const fn is_youth(self) -> bool {
        !matches!(self, TeamKind::First | TeamKind::Reserve)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Team {
    pub club: ClubId,
    pub kind: TeamKind,
    pub squad: Vec<PlayerId>,
    pub tactics: Tactics,
    pub captain: PlayerId,
    /// Consecutive weeks with the same formation (tactical familiarity).
    pub familiarity_weeks: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Facilities {
    pub training: u8,
    pub youth: u8,
    /// Youth recruitment network quality.
    pub academy: u8,
    pub medical: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Finance {
    pub balance: Money,
    pub transfer_budget: Money,
    /// Weekly wage budget.
    pub wage_budget: Money,
    /// Weekly wage bill (cached).
    pub wage_bill: Money,
    pub season_income: Money,
    pub season_spend: Money,
    pub debt: Money,
    /// How the club's wage pool scales the player wage curve (cached monthly; 0 = not computed yet).
    pub wage_scale: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum Ownership {
    #[default]
    Private,
    MemberOwned,
    Benefactor,
    InvestmentGroup,
    StateBacked,
}

/// Board expectations and patience (07 §10).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Board {
    /// 0–100; the manager is sacked when this collapses.
    pub satisfaction: u8,
    pub patience: u8,
    /// Expected league finishing position.
    pub target_position: u8,
    pub warnings: u8,
}

impl Default for Board {
    fn default() -> Self {
        Self { satisfaction: 60, patience: 50, target_position: 10, warnings: 0 }
    }
}

/// Recruitment need produced by squad planning (07 §5).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Need {
    pub group: PosGroup,
    pub pos: pw_core::Pos,
    /// Minimum ability sought on the CA scale.
    pub min_ability: u8,
    pub max_age: u8,
    pub urgency: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ClubMarket {
    pub needs: SmallVec<[Need; 4]>,
    pub signed_this_window: u8,
    pub last_search: Date,
    pub listed: Vec<PlayerId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Club {
    pub name: String,
    pub short_name: String,
    pub nation: NationId,
    pub city: String,
    /// 0–10,000.
    pub reputation: u16,
    pub colors: [u32; 2],
    /// First team first.
    pub teams: SmallVec<[TeamId; 3]>,
    /// Current domestic league of the first team.
    pub league: CompId,
    pub finance: Finance,
    pub facilities: Facilities,
    pub stadium: String,
    pub capacity: u32,
    pub staff: Vec<StaffId>,
    pub manager: StaffId,
    pub board: Board,
    pub ownership: Ownership,
    pub founded: u16,
    pub market: ClubMarket,
    /// Average attendance-demand multiplier from recent success.
    pub fan_mood: u8,
}

impl Club {
    #[inline]
    pub fn first_team(&self) -> TeamId {
        self.teams[0]
    }
}
