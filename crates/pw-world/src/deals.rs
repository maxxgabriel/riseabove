//! Squad plans and the deal machinery between clubs (07 §5, 08 §1–6).
//!
//! A club's plan says where it is thin, ageing, expiring or about to lose
//! someone, what kind of player it wants and what it can pay. Recruitment
//! works from the plan through shortlists into deals: enquiries, bids and
//! counters with instalments, add-ons, sell-on and buy-back clauses; medicals;
//! then personal terms. Money owed later is tracked as payables. Loans carry
//! fees, wage splits, options, obligations and recall clauses. Pre-contracts
//! and trials sit alongside.

use pw_core::{ClubId, Date, EventId, Money, PlayerId, Pos, PosGroup, TalkId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::negotiation::Terms;

// ------------------------------------------------------------------ plans

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum NeedRole {
    /// A new first choice.
    Starter,
    /// Depth that plays regularly.
    Rotation,
    /// Emergency cover.
    Backup,
    /// Someone to take over from an ageing or departing starter.
    Successor,
}

impl NeedRole {
    pub const fn label(self) -> &'static str {
        match self {
            NeedRole::Starter => "a starter",
            NeedRole::Rotation => "a rotation player",
            NeedRole::Backup => "cover",
            NeedRole::Successor => "a long-term successor",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GroupPlan {
    pub group: PosGroup,
    pub depth: u8,
    pub target_depth: u8,
    /// Average perceived ability of the likely starters.
    pub quality: f32,
    pub target_quality: f32,
    pub avg_age: f32,
    /// Contracts ending within a year.
    pub expiring: u8,
    /// Out long-term injured.
    pub injured: u8,
    /// Academy/young players expected to step up within two seasons.
    pub prospects: u8,
    /// Starters aged 31+.
    pub ageing_starters: u8,
    /// The likely starters' expected ability one and two seasons from now (development, ageing).
    pub quality_next: f32,
    pub quality_in_two: f32,
    /// Players expected to leave within a year: expiring contracts the club is unlikely to renew.
    pub expected_departures: u8,
    /// The thinnest spot in the group for the manager's system, and how many able players cover it beyond those needed.
    pub weak_pos: Pos,
    pub weak_cover: i8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlanNeed {
    pub group: PosGroup,
    /// The position the manager's system is short at within the group.
    pub pos: Pos,
    pub role: NeedRole,
    pub min_ability: u8,
    pub max_age: u8,
    /// Must be homegrown to keep the squad legal.
    pub homegrown: bool,
    /// Weekly wage the structure allows for this role.
    pub wage_band: Money,
    /// Fee the club is prepared to spend.
    pub fee_band: Money,
    /// 1–3.
    pub urgency: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SquadPlan {
    pub built: Date,
    pub groups: SmallVec<[GroupPlan; 4]>,
    pub needs: SmallVec<[PlanNeed; 6]>,
    /// Players the club would sell this window.
    pub sell: SmallVec<[PlayerId; 6]>,
    /// Young players the plan expects to promote.
    pub promote: SmallVec<[PlayerId; 4]>,
    /// Homegrown players short of the quota.
    pub homegrown_gap: u8,
    /// Wage budget left after the current wage bill (negative: over budget).
    pub wage_headroom: Money,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shortlist {
    pub updated: Date,
    /// (player, score) best first.
    pub targets: SmallVec<[(PlayerId, f32); 5]>,
    /// Attempts that failed this window for this need.
    pub failures: u8,
}

// ------------------------------------------------------------------ deals

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum AddOnKind {
    Appearances(u16),
    Goals(u16),
    Promotion,
    Title,
    InternationalCaps(u16),
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct AddOn {
    pub kind: AddOnKind,
    pub amount: Money,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct LoanTerms {
    pub fee: Money,
    /// Percent of wages the borrowing club pays.
    pub wage_share: u8,
    /// Option to buy at this fee (0 = none).
    pub option: Money,
    /// Obligation to buy at this fee once `obligation_apps` are reached (0 = none).
    pub obligation: Money,
    pub obligation_apps: u16,
    pub recall: bool,
    /// Minimum share of minutes the loanee should get (0 = no clause).
    pub minutes_clause: u8,
    /// Appearances at the loan club when it started (for obligations).
    pub apps_at_start: u16,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DealTerms {
    /// Total guaranteed fee.
    pub fee: Money,
    /// Paid in this many yearly parts (1 = upfront).
    pub instalments: u8,
    pub add_ons: SmallVec<[AddOn; 2]>,
    /// Percent of any future fee owed back to the seller.
    pub sell_on: u8,
    /// Seller may buy back at this fee within the window below (0 = none).
    pub buyback: Money,
    pub loan: Option<LoanTerms>,
}

impl DealTerms {
    /// What the package is worth to the seller today, roughly.
    pub fn value(&self) -> f64 {
        let discount = 1.0 - 0.04 * f64::from(self.instalments.saturating_sub(1));
        let adds: f64 = self.add_ons.iter().map(|a| a.amount as f64 * 0.5).sum();
        self.fee as f64 * discount + adds + self.fee as f64 * f64::from(self.sell_on) / 100.0 * 0.4
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DealState {
    Enquiry,
    /// Waiting for the seller's answer to a bid.
    Bid,
    /// Waiting for the buyer's answer to a counter.
    Counter,
    /// Fee agreed; medical under way.
    Medical,
    /// Personal terms being negotiated.
    Terms,
    Done,
    Collapsed,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DealEnd {
    NotForSale,
    SellerRefused,
    BuyerWithdrew,
    PlayerRefused,
    FailedMedical,
    Rules,
    WindowClosed,
    Hijacked,
    /// The seller would only sell once it had a replacement, and the replacement fell through.
    ReplacementFailed,
}

impl DealEnd {
    pub const fn label(self) -> &'static str {
        match self {
            DealEnd::NotForSale => "the player is not for sale",
            DealEnd::SellerRefused => "the clubs could not agree a fee",
            DealEnd::BuyerWithdrew => "the buying club walked away",
            DealEnd::PlayerRefused => "personal terms were not agreed",
            DealEnd::FailedMedical => "the player failed a medical",
            DealEnd::Rules => "registration rules",
            DealEnd::WindowClosed => "the window closed",
            DealEnd::Hijacked => "another club beat them to it",
            DealEnd::ReplacementFailed => "the seller's replacement fell through, so it pulled out",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum DealLine {
    Enquired,
    Bid(Money),
    Countered(Money),
    Raised(Money),
    Accepted(Money),
    MedicalPassed,
    MedicalFailed,
    FeeRenegotiated(Money),
    Ended(DealEnd),
    /// The seller will only agree once its replacement is signed.
    AwaitingReplacement(PlayerId),
    /// The replacement signed, so the seller became flexible.
    ReplacementSigned,
    Signalled(Signal),
}

/// Things one side says to move the other (section 3.14). Never shown as a game, only as their effects and, later, as what turned out true.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Signal {
    /// The seller says others are bidding.
    RivalInterest,
    /// The buyer says its budget is spent.
    BudgetGone,
    /// The buyer says it will walk away.
    WalkAway,
    /// The buyer takes its time.
    Delay,
}

/// Where a piece of information came from, which bounds how far it is believed (section 3.28).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Source {
    /// The club's own scouts and staff.
    Own,
    /// Passed on by a player's agent.
    Agent,
    /// Said by the other side in the negotiation.
    Claim,
    /// Worked out from what is public (injuries, results, reputation).
    Inference,
}

/// What one side thinks the other could pay or accept: a range, never the exact figure (section 3.13).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Limit {
    pub lo: Money,
    pub hi: Money,
}

/// A belief about a competing bid, with where it came from and how far it is trusted.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct RivalInfo {
    /// The bidder, if known.
    pub club: ClubId,
    /// The fee it is said to have offered, 0 if unstated.
    pub claimed: Money,
    pub source: Source,
    /// 0..1 how much the recipient believes it.
    pub reliability: f32,
    pub date: Date,
    /// For audit and for judging bluffs afterwards: whether a rival bid really existed when this was passed on.
    pub real: bool,
}

/// Signals used in one negotiation.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Signals {
    pub rivals_claimed: bool,
    pub budget_claimed: Money,
    pub walked: bool,
    pub delays: u8,
    /// Something said was false and the other side found out.
    pub caught: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClubDeal {
    pub buyer: ClubId,
    pub seller: ClubId,
    pub player: PlayerId,
    pub need: Option<PosGroup>,
    pub terms: DealTerms,
    pub ask: Option<DealTerms>,
    pub round: u8,
    pub state: DealState,
    pub end: Option<DealEnd>,
    pub opened: Date,
    /// Next date one side acts.
    pub next: Date,
    /// How badly the buyer needs this done (0..1+), grows toward deadline day.
    pub urgency: f32,
    pub log: Vec<(Date, DealLine)>,
    pub talk: TalkId,
    pub event: EventId,
    /// The replacement the seller is waiting for before it lets him go (none = not waiting), and since when.
    pub awaiting: PlayerId,
    pub awaiting_since: Date,
    pub signals: Signals,
    /// What each side believes about the other's limit.
    pub buyer_thinks_seller_min: Limit,
    pub seller_thinks_buyer_max: Limit,
    /// What the buyer has heard about competing bids.
    pub rival_info: SmallVec<[RivalInfo; 2]>,
}

impl ClubDeal {
    pub fn is_open(&self) -> bool {
        !matches!(self.state, DealState::Done | DealState::Collapsed)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PayReason {
    Instalment,
    AddOn,
    SellOn,
    AgentFee,
    LoanFee,
    Compensation,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Payable {
    pub from: ClubId,
    pub to: ClubId,
    pub amount: Money,
    pub due: Date,
    pub reason: PayReason,
    pub player: PlayerId,
}

/// A conditional payment waiting on a player's career.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PendingAddOn {
    pub player: PlayerId,
    pub payer: ClubId,
    pub payee: ClubId,
    pub add_on: AddOn,
    pub since: Date,
    pub apps_at: u16,
    pub goals_at: u16,
    pub caps_at: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreContract {
    pub player: PlayerId,
    pub club: ClubId,
    pub terms: Terms,
    pub signed: Date,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Trial {
    pub player: PlayerId,
    pub club: ClubId,
    pub from: Date,
    pub until: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Deals {
    pub plans: FxHashMap<ClubId, SquadPlan>,
    pub shortlists: FxHashMap<(ClubId, PosGroup), Shortlist>,
    pub deals: Vec<ClubDeal>,
    pub payables: Vec<Payable>,
    pub add_ons: Vec<PendingAddOn>,
    /// Sell-on obligations: (club owed, percent).
    pub sell_ons: FxHashMap<PlayerId, SmallVec<[(ClubId, u8); 2]>>,
    /// Buy-back rights: (club, fee, until).
    pub buybacks: FxHashMap<PlayerId, (ClubId, Money, Date)>,
    pub loans: FxHashMap<PlayerId, LoanTerms>,
    pub pre_contracts: Vec<PreContract>,
    pub trials: Vec<Trial>,
    /// An exceptional chance each club has noticed outside its plan, and when (section 4.10).
    pub opportunities: FxHashMap<ClubId, (PlayerId, Date)>,
    /// Academy players the club counted on being ready, to check later whether they were (section 4.23).
    pub counted_on: Vec<CountedOn>,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct CountedOn {
    pub club: ClubId,
    pub player: PlayerId,
    pub date: Date,
    /// The level the club expected him to have reached.
    pub expected: f32,
}

impl Deals {
    pub fn open_for(&self, p: PlayerId) -> impl Iterator<Item = (usize, &ClubDeal)> {
        self.deals.iter().enumerate().filter(move |(_, d)| d.player == p && d.is_open())
    }

    pub fn active(&self, buyer: ClubId, p: PlayerId) -> bool {
        self.deals.iter().any(|d| d.buyer == buyer && d.player == p && d.is_open())
    }

    pub fn on_trial(&self, p: PlayerId) -> Option<&Trial> {
        self.trials.iter().find(|t| t.player == p)
    }
}
