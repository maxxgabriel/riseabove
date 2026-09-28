use pw_core::{ClubId, CompId, Date, Money, PersonId, PlayerId, StaffId, TeamId};
use serde::{Deserialize, Serialize};

/// Who may learn about an event (P3). The protagonist's inbox only receives
/// events whose visibility includes them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Club(ClubId),
    Person(PersonId),
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
    pub date: Date,
    pub vis: Visibility,
    pub kind: EventKind,
}

impl EventKind {
    /// Players this event is about (for per-player feeds).
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
            | CallUp { player } => Some(player),
            _ => None,
        }
    }
}

/// Append-only event stream, ordered by insertion (which follows the daily
/// pipeline order, so `(date, phase, sequence)` ordering holds).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<Event>,
}

impl EventLog {
    #[inline]
    pub fn push(&mut self, date: Date, vis: Visibility, kind: EventKind) {
        self.events.push(Event { date, vis, kind });
    }

    pub fn since(&self, from: Date) -> &[Event] {
        let i = self.events.partition_point(|e| e.date < from);
        &self.events[i..]
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
