//! The boardroom: who has a say in a signing, how willing a club is to take risk, and what the club remembers about important
//! deals (locked design 3.6, 3.7, 3.19-3.24). None of it is a score to average; institutional power decides, and the decision
//! leaves a case file that later judgements, reputations and stories can read.

use pw_core::{ClubId, Date, Money, PersonId, PlayerId};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::contract::SquadStatus;
use crate::dossier::{Confidence, RiskKind, Span};
use crate::negotiation::{Lever, Priority, Terms};

/// The people or bodies whose opinion can matter to a signing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Voice {
    Manager,
    Director,
    Recruitment,
    Owner,
    Captain,
    Analyst,
    Supporters,
}

impl Voice {
    pub const ALL: [Voice; 7] = [Voice::Manager, Voice::Director, Voice::Recruitment, Voice::Owner, Voice::Captain, Voice::Analyst, Voice::Supporters];

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            Voice::Manager => "the manager",
            Voice::Director => "the sporting director",
            Voice::Recruitment => "the head of recruitment",
            Voice::Owner => "the owner",
            Voice::Captain => "the captain",
            Voice::Analyst => "the analysts",
            Voice::Supporters => "the supporters",
        }
    }
}

/// How a club's football decisions are actually made.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Structure {
    ManagerLed,
    DirectorLed,
    OwnerLed,
    Committee,
    DataLed,
}

impl Structure {
    pub const fn label(self) -> &'static str {
        match self {
            Structure::ManagerLed => "manager-led",
            Structure::DirectorLed => "sporting-director-led",
            Structure::OwnerLed => "owner-led",
            Structure::Committee => "run by a recruitment committee",
            Structure::DataLed => "data-led",
        }
    }
}

/// What lay behind a stance.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Why {
    FootballValue,
    Price,
    Alternative,
    Commercial,
    RoleCompetition,
    WageHierarchy,
    Data,
    PublicMood,
    Risk(RiskKind),
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stance {
    pub voice: Voice,
    /// Who held it (none for the supporters).
    pub who: PersonId,
    /// -1 firmly against .. +1 firmly for.
    pub support: f32,
    /// How much this voice counts at this club for this kind of decision, 0..1 (sums to 1).
    pub power: f32,
    pub why: Why,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum CaseState {
    /// Decided in favour; the club is pursuing him.
    Pursuing,
    /// The club turned it down internally, before any approach.
    Declined,
    Signed,
    /// The deal fell through for reasons outside the club's decision.
    Collapsed,
}

/// How a signing is judged afterwards. The outcome alone never decides it (section 3.23): a risk the club saw and accepted is a
/// different decision from one it never noticed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Verdict {
    /// A sound decision that worked.
    Sound,
    /// A sound decision that did not work out.
    Unlucky,
    /// A careless decision that worked anyway.
    Lucky,
    /// A risk the club had recognised and accepted came to pass.
    AcceptedRisk,
    /// A risk nobody at the club had noticed came to pass.
    MissedRisk,
    /// Neither the process nor the result.
    Mistake,
}

impl Verdict {
    pub const fn label(self) -> &'static str {
        match self {
            Verdict::Sound => "a sound signing that worked",
            Verdict::Unlucky => "a sound signing that did not work out",
            Verdict::Lucky => "a careless signing that worked out anyway",
            Verdict::AcceptedRisk => "a known risk that came to pass",
            Verdict::MissedRisk => "a risk nobody saw",
            Verdict::Mistake => "a mistake",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub date: Date,
    /// -1 disaster .. +1 triumph, judged from what happened on the pitch and in the market.
    pub success: f32,
    pub verdict: Verdict,
    /// Risks that materialised.
    pub materialised: [Option<RiskKind>; 2],
}

/// One important transfer decision and everything needed to understand it later (section 3.24).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Case {
    pub buyer: ClubId,
    pub seller: ClubId,
    pub player: PlayerId,
    pub date: Date,
    pub structure: Structure,
    pub stances: SmallVec<[Stance; 7]>,
    /// Power-weighted support, and the bar it had to clear at the club's appetite for risk.
    pub score: f32,
    pub threshold: f32,
    pub state: CaseState,
    /// The voice that made the running.
    pub champion: Voice,
    /// Voices firmly against who were nonetheless overruled.
    pub overruled: SmallVec<[Voice; 3]>,
    /// The dossier the decision rested on.
    pub current: Span,
    pub ceiling: Span,
    pub confidence: Confidence,
    /// Risks the club's evaluators had flagged, and which of them it decided to live with.
    pub risks_known: SmallVec<[RiskKind; 4]>,
    pub risks_accepted: SmallVec<[RiskKind; 4]>,
    /// Risks it could not judge at all.
    pub risks_unknown: SmallVec<[RiskKind; 4]>,
    pub role: SquadStatus,
    pub alternatives: u8,
    /// The range the club believed the seller might accept.
    pub price_low: Money,
    pub price_high: Money,
    pub price_paid: Money,
    pub appetite: f32,
    pub signed: Option<Date>,
    pub outcome: Option<Outcome>,
    /// A replacement chain: the sale this signing was made to cover, or that it made possible.
    pub covers: PlayerId,
    /// His record when the club took him on, so the outcome can be judged from what came after.
    pub apps_at: u16,
    pub injuries_at: u16,
    /// Recruited to fill a need the plan had named (true), or an opportunity that turned up (false).
    pub planned: bool,
    /// Signed in a hurry after a plan failed (a renewal that collapsed, a prospect who was not ready).
    pub panic: bool,
}

/// Ways a club's own planning let it down (section 4.23).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PlanFailure {
    /// An academy player the club counted on was not ready.
    ProspectOverestimated,
    /// A youngster left after being blocked by a signing, and did well elsewhere.
    YouthBlocked,
    /// A key player the club expected to keep walked out on a free.
    RenewalCollapsed,
}

impl PlanFailure {
    pub const fn label(self) -> &'static str {
        match self {
            PlanFailure::ProspectOverestimated => "counted on an academy player who was not ready",
            PlanFailure::YouthBlocked => "blocked a youngster who went on to thrive elsewhere",
            PlanFailure::RenewalCollapsed => "expected to keep a key player and lost him for nothing",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Driver {
    NewOwner,
    Investment,
    Cash,
    Debt,
    RecentSuccess,
    RecentFlops,
    Tenure,
    BoardPressure,
    SupporterMood,
    LeaguePosition,
    Trophies,
    Sponsor,
    WindowTiming,
}

impl Driver {
    pub const fn label(self) -> &'static str {
        match self {
            Driver::NewOwner => "new ownership",
            Driver::Investment => "fresh investment",
            Driver::Cash => "cash reserves",
            Driver::Debt => "debt",
            Driver::RecentSuccess => "recent recruitment success",
            Driver::RecentFlops => "recent expensive flops",
            Driver::Tenure => "how long the director and manager have been in post",
            Driver::BoardPressure => "board pressure",
            Driver::SupporterMood => "supporter mood",
            Driver::LeaguePosition => "league position",
            Driver::Trophies => "recent trophies",
            Driver::Sponsor => "a new sponsor's ambitions",
            Driver::WindowTiming => "the closing window",
        }
    }
}

/// A club's present willingness to take a gamble, and why (section 3.21). Recomputed monthly, never a fixed slider.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Appetite {
    /// 0 very cautious .. 1 very aggressive; 0.5 is the club's normal.
    pub level: f32,
    pub drivers: SmallVec<[(Driver, f32); 6]>,
    pub as_of: Date,
}

impl Default for Appetite {
    fn default() -> Self {
        Self { level: 0.5, drivers: SmallVec::new(), as_of: Date(0) }
    }
}

/// What each club has learned about signing players (section 3.7).
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Record {
    pub hits: u8,
    pub flops: u8,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Boardroom {
    pub cases: Vec<Case>,
    pub appetite: FxHashMap<ClubId, Appetite>,
    /// Institutional standing gained or lost by each voice at each club through the results of past decisions, -0.3..+0.3.
    pub authority: FxHashMap<(ClubId, Voice), f32>,
    pub record: FxHashMap<ClubId, Record>,
    /// How honest a club's negotiators have been found to be, 0 (caught bluffing) .. 1; missing means unknown and is treated as 0.6.
    pub honesty: FxHashMap<ClubId, f32>,
    /// A key player left when the club had counted on renewing him: (position group, date). Signings soon after are made in a hurry.
    pub scramble: FxHashMap<ClubId, (pw_core::PosGroup, Date)>,
    /// Signings that may have blocked a promising youngster (section 4.23).
    pub blocked: Vec<Blocked>,
    /// Important contracts, kept so they can be understood and judged later (section 5.19).
    pub contracts: Vec<ContractFile>,
    /// The last event this book has read for clause payments and triggers.
    pub event_cursor: u32,
}

/// What lay behind an important contract: what the club believed, what it conceded, what the player and agent wanted, what was
/// promised (locked design 5.19). Judged a season on by what was known at the time as well as by how it turned out (5.18).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ContractFile {
    pub club: ClubId,
    pub player: PlayerId,
    pub date: Date,
    pub terms: Terms,
    /// What the club thought he was worth in fees, and what it intended him to be.
    pub believed_worth: Money,
    pub role: SquadStatus,
    /// Risks the evaluators had flagged.
    pub risks: SmallVec<[RiskKind; 4]>,
    /// Other interest in him at the time, and the alternatives the club had.
    pub competing: u8,
    pub alternatives: u8,
    /// The voice that approved terms outside the club's usual structure, if they were.
    pub exception: Option<Voice>,
    /// Levers each side moved on, in order (true = the player's side).
    pub concessions: SmallVec<[(Lever, bool); 6]>,
    pub priorities: [Priority; 2],
    /// A status or minutes promise was made.
    pub promised: Option<SquadStatus>,
    /// What the deal commits the club to in all, and that as a share of its yearly revenue, in percent.
    pub commitment: Money,
    pub burden_pct: u8,
    pub apps_at: u16,
    pub outcome: Option<Outcome>,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Blocked {
    pub club: ClubId,
    pub youngster: PlayerId,
    pub signing: PlayerId,
    pub date: Date,
    /// The club's reading of the youngster's current level when it signed over him.
    pub level_then: f32,
}

impl Boardroom {
    pub fn authority_of(&self, club: ClubId, v: Voice) -> f32 {
        self.authority.get(&(club, v)).copied().unwrap_or(0.0)
    }

    pub fn appetite_of(&self, club: ClubId) -> f32 {
        self.appetite.get(&club).map_or(0.5, |a| a.level)
    }

    pub fn honesty_of(&self, club: ClubId) -> f32 {
        self.honesty.get(&club).copied().unwrap_or(0.6)
    }

    /// The most recent case on a player for this buyer.
    pub fn case_for(&self, buyer: ClubId, p: PlayerId) -> Option<usize> {
        self.cases.iter().rposition(|c| c.buyer == buyer && c.player == p)
    }
}
