//! The causal event log (S11). Every consequential change to the world is an
//! event with an id, a visibility (who may learn of it, P3) and the causes that
//! produced it, so "why did this happen?" is answerable from data and text can
//! be rendered from state rather than invented (S17).

use pw_core::{AgentId, ClubId, CompId, Date, EventId, MeetingId, Money, NationId, PersonId, PlayerId, StaffId, StoryId, TalkId, TeamId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::social::MemoryKind;

/// Who may learn about an event (P3). Knowledge still has to travel: a
/// club-internal event reaches the press only through a person who leaks it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Club(ClubId),
    Person(PersonId),
    /// A private exchange between two people (a meeting, a phone call).
    Between(PersonId, PersonId),
}

impl Visibility {
    /// Can `person`, currently attached to `club` (or `NONE`), see this directly?
    pub fn reaches(&self, person: PersonId, club: ClubId) -> bool {
        match *self {
            Visibility::Public => true,
            Visibility::Club(c) => club.is_some() && c == club,
            Visibility::Person(p) => p == person,
            Visibility::Between(a, b) => a == person || b == person,
        }
    }
}

/// A recorded circumstance that contributed to an event. Facts are small,
/// typed references into state that existed at the time, never prose.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Fact {
    /// Training ratings below the player's own norm for `weeks`.
    TrainingSlump { player: PlayerId, weeks: u8 },
    /// Training ratings well above norm for `weeks`.
    TrainingSurge { player: PlayerId, weeks: u8 },
    /// Recent match ratings well below the player's norm.
    FormSlump { player: PlayerId },
    FormSurge { player: PlayerId },
    /// Share of available minutes against what the player's status promises.
    MinutesShortfall { player: PlayerId, share_pct: u8, expected_pct: u8 },
    /// A club has been watching a player (scouting, analysis, agent pitch).
    Tracking { club: ClubId, player: PlayerId, minutes: u16 },
    /// A club's squad plan has a hole in this player's position group.
    SquadNeed { club: ClubId },
    BoardPressure { club: ClubId, warnings: u8 },
    Congestion { team: TeamId, matches: u8 },
    /// A remembered episode between two people.
    Memory { from: PersonId, about: PersonId, kind: MemoryKind },
    PromiseDue { promise: u32 },
    /// Weekly wage relative to squad peers of similar standing, percent.
    WageGap { player: PlayerId, pct_of_peers: u16 },
    /// Family and partner circumstances weighed in a decision.
    Household { person: PersonId },
    Injury { player: PlayerId, days: u16 },
    /// Living abroad without the language or roots yet.
    Unsettled { person: PersonId, nation: NationId },
    /// Low trust between two people.
    LowTrust { from: PersonId, about: PersonId, trust: u8 },
    ContractRunningDown { player: PlayerId, days: u16 },
    PublicCriticism { story: StoryId },
    /// A football rule stood in the way.
    Rule { reason: crate::rules::Reason },
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Cause {
    Event(EventId),
    Fact(Fact),
}

pub type Causes = SmallVec<[Cause; 3]>;

/// Things that happen in people's private lives (10). These are outcomes of
/// the life simulation for every person, not flavour cards.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LifeEventKind {
    StartedDating { partner: PersonId },
    MovedIn { partner: PersonId },
    Married { partner: PersonId },
    Separated { partner: PersonId },
    ChildBorn,
    ParentUnwell,
    ParentRecovered,
    Bereavement,
    Relocated { nation: NationId },
    PartnerJoinedMove { partner: PersonId },
    PartnerStayedBehind { partner: PersonId },
    FinancialTrouble,
    Graduated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventKind {
    Transfer { player: PlayerId, from: ClubId, to: ClubId, fee: Money },
    LoanMove { player: PlayerId, from: ClubId, to: ClubId, until: Date },
    LoanReturn { player: PlayerId, to: ClubId },
    ContractSigned { player: PlayerId, club: ClubId, wage: Money, until: Date, renewal: bool },
    Released { player: PlayerId, club: ClubId },
    Retired { person: PersonId },
    Injured { player: PlayerId, injury: u16, days: u16 },
    Recovered { player: PlayerId },
    Suspended { player: PlayerId, matches: u8 },
    ManagerSacked { staff: StaffId, club: ClubId },
    ManagerAppointed { staff: StaffId, club: ClubId },
    YouthIntake { club: ClubId, count: u8 },
    Debut { player: PlayerId, team: TeamId, comp: CompId },
    FirstGoal { player: PlayerId, team: TeamId, comp: CompId },
    Champion { comp: CompId, team: TeamId, season: i32 },
    Promoted { comp: CompId, team: TeamId },
    Relegated { comp: CompId, team: TeamId },
    Interest { player: PlayerId, club: ClubId },
    BidRejected { player: PlayerId, club: ClubId, fee: Money },
    BidAccepted { player: PlayerId, club: ClubId, fee: Money },
    TransferListed { player: PlayerId, club: ClubId },
    Award { player: PlayerId, comp: CompId, award: AwardKind, season: i32 },
    CallUp { player: PlayerId },

    /// A conversation took place (details in `World::meetings`).
    Meeting { meeting: MeetingId, from: PersonId, with: PersonId },
    PromiseMade { promise: u32, from: PersonId, to: PersonId },
    PromiseKept { promise: u32, from: PersonId, to: PersonId },
    PromiseBroken { promise: u32, from: PersonId, to: PersonId },
    TransferRequested { player: PlayerId, club: ClubId },
    TransferRequestWithdrawn { player: PlayerId, club: ClubId },
    Fined { player: PlayerId, club: ClubId, amount: Money },
    /// Dressing-room unrest spreading from an unhappy, influential player.
    Unrest { club: ClubId, player: PlayerId },
    /// Contract talks opened, moved, or ended (details in `World::talks`).
    TalksOpened { talk: TalkId, player: PlayerId, club: ClubId },
    TalksCollapsed { talk: TalkId, player: PlayerId, club: ClubId },
    /// A journalist published something (details in `World::stories`).
    Published { story: StoryId },
    AgentHired { player: PlayerId, agent: AgentId },
    AgentLeft { player: PlayerId, agent: AgentId },
    /// A club heard about a player through their agent.
    AgentPitch { player: PlayerId, agent: AgentId, club: ClubId },
    Life { person: PersonId, kind: LifeEventKind },
    /// A person took a football job after (or instead of) playing.
    JoinedStaff { person: PersonId, staff: StaffId, club: ClubId },
    CameOutOfRetirement { person: PersonId },
    /// A coach's note on a player's training (club-internal).
    CoachNote { player: PlayerId, by: PersonId, note: CoachNote },
    /// A manager changed a player's squad status.
    StatusChanged { player: PlayerId, club: ClubId, from: crate::contract::SquadStatus, to: crate::contract::SquadStatus },
    /// A manager named a new captain.
    Captaincy { player: PlayerId, team: TeamId },
    /// A club changed hands.
    Takeover { club: ClubId, owner: PersonId, previous: PersonId },
    /// A club entered administration.
    Administration { club: ClubId },
    PointsDeducted { club: ClubId, points: u8 },
    /// The board cut budgets and put earners up for sale.
    Austerity { club: ClubId },
    /// The owner put money in.
    OwnerInvestment { club: ClubId, amount: Money },
    ProjectStarted { club: ClubId, kind: crate::governance::ProjectKind },
    ProjectCompleted { club: ClubId, kind: crate::governance::ProjectKind },
    /// A nation's top flight signed a new broadcast deal.
    BroadcastDeal { nation: NationId, pool: Money },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CoachNote {
    PoorTraining,
    ExcellentTraining,
    Improving,
    Declining,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum AwardKind {
    PlayerOfSeason,
    YoungPlayerOfSeason,
    TopScorer,
    TeamOfSeason,
    PlayerOfMonth,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub date: Date,
    pub vis: Visibility,
    pub kind: EventKind,
    pub causes: Causes,
}

impl EventKind {
    /// The player this event is about (for per-player feeds).
    pub fn player(&self) -> Option<PlayerId> {
        use EventKind::*;
        match *self {
            Transfer { player, .. }
            | LoanMove { player, .. }
            | LoanReturn { player, .. }
            | ContractSigned { player, .. }
            | Released { player, .. }
            | Injured { player, .. }
            | Recovered { player }
            | Suspended { player, .. }
            | Debut { player, .. }
            | FirstGoal { player, .. }
            | Interest { player, .. }
            | BidRejected { player, .. }
            | BidAccepted { player, .. }
            | TransferListed { player, .. }
            | Award { player, .. }
            | CallUp { player }
            | TransferRequested { player, .. }
            | TransferRequestWithdrawn { player, .. }
            | Fined { player, .. }
            | Unrest { player, .. }
            | TalksOpened { player, .. }
            | TalksCollapsed { player, .. }
            | AgentHired { player, .. }
            | AgentLeft { player, .. }
            | AgentPitch { player, .. }
            | CoachNote { player, .. }
            | StatusChanged { player, .. }
            | Captaincy { player, .. } => Some(player),
            _ => None,
        }
    }

    /// People (not via a player role) this event is about.
    pub fn people(&self) -> SmallVec<[PersonId; 2]> {
        use EventKind::*;
        let mut v = SmallVec::new();
        match *self {
            Retired { person } | Life { person, .. } | JoinedStaff { person, .. } | CameOutOfRetirement { person } => v.push(person),
            Meeting { from, with, .. } => {
                v.push(from);
                v.push(with);
            }
            PromiseMade { from, to, .. } | PromiseKept { from, to, .. } | PromiseBroken { from, to, .. } => {
                v.push(from);
                v.push(to);
            }
            _ => {}
        }
        v
    }

    /// Clubs this event is about.
    pub fn clubs(&self) -> SmallVec<[ClubId; 2]> {
        use EventKind::*;
        let mut v = SmallVec::new();
        match *self {
            Transfer { from, to, .. } | LoanMove { from, to, .. } => {
                if from.is_some() {
                    v.push(from);
                }
                v.push(to);
            }
            LoanReturn { to: club, .. }
            | ContractSigned { club, .. }
            | Released { club, .. }
            | ManagerSacked { club, .. }
            | ManagerAppointed { club, .. }
            | YouthIntake { club, .. }
            | Interest { club, .. }
            | BidRejected { club, .. }
            | BidAccepted { club, .. }
            | TransferListed { club, .. }
            | TransferRequested { club, .. }
            | TransferRequestWithdrawn { club, .. }
            | Fined { club, .. }
            | Unrest { club, .. }
            | TalksOpened { club, .. }
            | TalksCollapsed { club, .. }
            | AgentPitch { club, .. }
            | StatusChanged { club, .. }
            | Takeover { club, .. }
            | Administration { club }
            | PointsDeducted { club, .. }
            | Austerity { club }
            | OwnerInvestment { club, .. }
            | ProjectStarted { club, .. }
            | ProjectCompleted { club, .. }
            | JoinedStaff { club, .. } => v.push(club),
            _ => {}
        }
        v
    }
}

/// Append-only event stream. Ids are assigned in insertion order, which
/// follows the daily pipeline, so both id and `(date, sequence)` order hold.
/// Compaction removes old events but never reuses ids.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<Event>,
    next: u32,
}

impl EventLog {
    #[inline]
    pub fn push(&mut self, date: Date, vis: Visibility, kind: EventKind) -> EventId {
        self.push_caused(date, vis, kind, Causes::new())
    }

    pub fn push_caused(&mut self, date: Date, vis: Visibility, kind: EventKind, causes: Causes) -> EventId {
        let id = EventId(self.next);
        self.next += 1;
        self.events.push(Event { id, date, vis, kind, causes });
        id
    }

    /// Look up an event by id (binary search; ids are monotonic).
    pub fn get(&self, id: EventId) -> Option<&Event> {
        self.events.binary_search_by_key(&id, |e| e.id).ok().map(|i| &self.events[i])
    }

    pub fn since(&self, from: Date) -> &[Event] {
        let i = self.events.partition_point(|e| e.date < from);
        &self.events[i..]
    }

    /// Events recorded after `id` (exclusive), for incremental feeds.
    pub fn after(&self, id: EventId) -> &[Event] {
        if id.is_none() {
            return &self.events;
        }
        let i = self.events.partition_point(|e| e.id <= id);
        &self.events[i..]
    }

    pub fn last_id(&self) -> EventId {
        self.events.last().map_or(EventId::NONE, |e| e.id)
    }

    pub fn all(&self) -> &[Event] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Keep recent events plus those `keep` marks as archival.
    pub fn compact(&mut self, before: Date, keep: impl Fn(&Event) -> bool) {
        self.events.retain(|e| e.date >= before || keep(e));
    }
}

/// Convenience for building cause lists inline.
#[macro_export]
macro_rules! causes {
    ($($c:expr),* $(,)?) => {{
        let mut v: $crate::event::Causes = $crate::event::Causes::new();
        $(v.push($c);)*
        v
    }};
}
