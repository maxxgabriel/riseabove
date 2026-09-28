use pw_core::{ClubId, CompId, Date, IdVec, Money, NationId, PersonId, PlayerId, StaffId, TeamId};
use pw_data::DataPack;
use pw_match::MatchResult;
use serde::{Deserialize, Serialize};

use crate::club::{Club, Team, TeamKind};
use crate::comp::{Competition, Fixtures};
use crate::contract::{Contract, Loan};
use crate::decision::{Decisions, MindKind};
use crate::event::EventLog;
use crate::history::History;
use crate::knowledge::Knowledge;
use crate::names::Names;
use crate::nation::Nation;
use crate::person::Person;
use crate::player::{PlayerStatus, Players};
use crate::staff::Staff;
use crate::negotiation::Negotiation;
use crate::social::Social;
use crate::stats::SeasonStats;
use crate::{FxHashMap, FxHashSet};

/// A deal waiting on a player's (possibly external) decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingDeal {
    pub player: PlayerId,
    pub buyer: ClubId,
    pub seller: ClubId,
    pub fee: Money,
    pub contract: Contract,
    pub loan: Option<Loan>,
    pub decision: pw_core::DecisionId,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MarketBook {
    /// (buyer, player) → earliest date the buyer may approach again.
    pub cooldown: FxHashMap<(ClubId, PlayerId), Date>,
    pub pending: Vec<PendingDeal>,
}

impl MarketBook {
    pub fn on_cooldown(&self, club: ClubId, p: PlayerId, today: Date) -> bool {
        self.cooldown.get(&(club, p)).is_some_and(|&d| today < d)
    }

    pub fn is_pending(&self, p: PlayerId) -> bool {
        self.pending.iter().any(|d| d.player == p)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct World {
    pub seed: u64,
    pub date: Date,
    pub data: DataPack,
    pub names: Names,
    pub nations: IdVec<NationId, Nation>,
    pub people: IdVec<PersonId, Person>,
    pub players: Players,
    pub staff: IdVec<StaffId, Staff>,
    pub clubs: IdVec<ClubId, Club>,
    pub teams: IdVec<TeamId, Team>,
    pub comps: IdVec<CompId, Competition>,
    pub fixtures: Fixtures,
    pub knowledge: Knowledge,
    pub events: EventLog,
    pub history: History,
    pub stats: SeasonStats,
    pub decisions: Decisions,
    pub market: MarketBook,
    pub social: Social,
    pub talks: IdVec<pw_core::TalkId, Negotiation>,
    /// Full match results (events, per-player lines) for watched teams, keyed by fixture uid.
    pub reports: FxHashMap<u64, MatchResult>,
    pub days_simulated: u64,
    /// Teams whose matches are recorded at full detail on request (recording only, never outcomes).
    pub followed: Vec<TeamId>,
}

impl World {
    pub fn new(data: DataPack, seed: u64, date: Date) -> Self {
        Self {
            seed,
            date,
            data,
            names: Names::default(),
            nations: IdVec::new(),
            people: IdVec::new(),
            players: Players::default(),
            staff: IdVec::new(),
            clubs: IdVec::new(),
            teams: IdVec::new(),
            comps: IdVec::new(),
            fixtures: Fixtures::default(),
            knowledge: Knowledge::default(),
            events: EventLog::default(),
            history: History::default(),
            stats: SeasonStats::default(),
            decisions: Decisions::default(),
            market: MarketBook::default(),
            social: Social::default(),
            talks: IdVec::new(),
            reports: FxHashMap::default(),
            days_simulated: 0,
            followed: Vec::new(),
        }
    }

    #[inline]
    pub fn person_of(&self, p: PlayerId) -> &Person {
        &self.people[self.players.cold[p].person]
    }

    pub fn player_name(&self, p: PlayerId) -> String {
        self.person_of(p).display_name(&self.names).into_owned()
    }

    pub fn player_short(&self, p: PlayerId) -> String {
        self.person_of(p).short_name(&self.names)
    }

    pub fn staff_name(&self, s: StaffId) -> String {
        self.people[self.staff[s].person].display_name(&self.names).into_owned()
    }

    #[inline]
    pub fn age(&self, p: PlayerId) -> u32 {
        self.person_of(p).age(self.date)
    }

    #[inline]
    pub fn age_years(&self, p: PlayerId) -> f32 {
        self.person_of(p).dob.age_years(self.date)
    }

    #[inline]
    pub fn club_of(&self, t: TeamId) -> ClubId {
        self.teams[t].club
    }

    pub fn team_name(&self, t: TeamId) -> String {
        let team = &self.teams[t];
        let club = &self.clubs[team.club];
        match team.kind {
            TeamKind::First => club.name.clone(),
            k => format!("{} {}", club.name, k.label()),
        }
    }

    pub fn team_short(&self, t: TeamId) -> String {
        let team = &self.teams[t];
        let club = &self.clubs[team.club];
        match team.kind {
            TeamKind::First => club.short_name.clone(),
            k => format!("{} {}", club.short_name, k.label()),
        }
    }

    /// Club team of a given kind, if the club runs one.
    pub fn club_team(&self, club: ClubId, kind: TeamKind) -> Option<TeamId> {
        self.clubs[club].teams.iter().copied().find(|&t| self.teams[t].kind == kind)
    }

    /// Players contracted to the club across all its teams (excluding loanees out).
    pub fn club_senior_count(&self, club: ClubId) -> usize {
        self.clubs[club].teams.iter().map(|&t| self.teams[t].squad.len()).sum()
    }

    /// Teams whose matches are recorded at full detail: those with an
    /// externally-minded player, plus their opponents' view comes for free.
    pub fn watched_teams(&self) -> FxHashSet<TeamId> {
        self.people
            .iter()
            .filter(|p| p.mind == MindKind::External && p.player.is_some())
            .map(|p| self.players.hot[p.player].team)
            .filter(|t| t.is_some())
            .chain(self.followed.iter().copied())
            .collect()
    }

    pub fn external_players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.people.iter().filter(|p| p.mind == MindKind::External && p.player.is_some()).map(|p| p.player)
    }

    /// Main domestic league competition for a team this season.
    pub fn league_of(&self, t: TeamId) -> Option<CompId> {
        self.comps.iter_enumerated().find(|(_, c)| c.is_league() && c.state.entrants.contains(&t)).map(|(id, _)| id)
    }

    pub fn is_free_agent(&self, p: PlayerId) -> bool {
        self.players.hot[p].status == PlayerStatus::FreeAgent
    }

    pub fn club_manager_judging(&self, club: ClubId) -> (f32, f32) {
        let c = &self.clubs[club];
        let best = |f: fn(&Staff) -> f32| c.staff.iter().map(|&s| f(&self.staff[s])).fold(6.0f32, f32::max);
        (
            best(|s| s.attrs.f(pw_core::StaffAttr::JudgingAbility)),
            best(|s| s.attrs.f(pw_core::StaffAttr::JudgingPotential)),
        )
    }
}
