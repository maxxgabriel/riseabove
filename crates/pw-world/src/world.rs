use pw_core::{ClubId, CompId, Date, IdVec, Money, NationId, PersonId, PlayerId, StaffId, TeamId};
use pw_data::DataPack;
use pw_match::MatchResult;
use serde::{Deserialize, Serialize};

use crate::agent::Agents;
use crate::beliefs::Beliefs;
use crate::club::{Club, Team, TeamKind};
use crate::intent::Intents;
use crate::interaction::Meetings;
use crate::life::Life;
use crate::media::Media;
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
    /// Players who have handed in a transfer request, and when.
    pub requests: FxHashMap<PlayerId, Date>,
    /// Players their club has explicitly made available for transfer, and when.
    pub listed: FxHashMap<PlayerId, Date>,
    /// Players their club has agreed to loan out, and when.
    pub loan_listed: FxHashMap<PlayerId, Date>,
    /// Players currently in contract talks (one set of talks at a time).
    pub talking: FxHashMap<PlayerId, pw_core::TalkId>,
}

impl MarketBook {
    pub fn on_cooldown(&self, club: ClubId, p: PlayerId, today: Date) -> bool {
        self.cooldown.get(&(club, p)).is_some_and(|&d| today < d)
    }

    pub fn is_pending(&self, p: PlayerId) -> bool {
        self.pending.iter().any(|d| d.player == p)
    }

    pub fn has_requested(&self, p: PlayerId) -> bool {
        self.requests.contains_key(&p)
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
    /// What individual people believe (S15).
    pub beliefs: Beliefs,
    /// Life off the pitch, one per person (aligned with `people`).
    pub lives: IdVec<PersonId, Life>,
    pub agents: Agents,
    pub media: Media,
    pub meetings: Meetings,
    pub intents: Intents,
    /// Owners, boards, policies and projects, per club.
    pub governance: FxHashMap<ClubId, crate::governance::Governance>,
    pub economy: crate::governance::Economy,
    /// Manager identities and CVs.
    pub careers: crate::careers::Careers,
    /// Scouts, their assignments and reports.
    pub scouting: crate::scouting::Scouting,
    /// Full match results (events, per-player lines) for watched teams, keyed by fixture uid.
    pub reports: FxHashMap<u64, MatchResult>,
    pub days_simulated: u64,
    /// Mixed into the seed when a playthrough begins, so two playthroughs of
    /// the same starting world diverge while one save replays exactly (S22).
    pub playthrough: u64,
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
            beliefs: Beliefs::default(),
            lives: IdVec::new(),
            agents: Agents::default(),
            media: Media::default(),
            meetings: Meetings::default(),
            intents: Intents::default(),
            governance: FxHashMap::default(),
            economy: Default::default(),
            careers: Default::default(),
            scouting: Default::default(),
            reports: FxHashMap::default(),
            days_simulated: 0,
            playthrough: 0,
        }
    }

    /// Inhabit a person: from now on their decisions come from outside the
    /// simulation. Nothing else about the world changes (S4).
    pub fn take_control(&mut self, person: PersonId) -> bool {
        match self.people.get_mut(person) {
            Some(p) => {
                p.mind = MindKind::External;
                true
            }
            None => false,
        }
    }

    /// Hand a person back to their own AI mind, which carries on from their
    /// personality, values and memories (S4).
    pub fn release_control(&mut self, person: PersonId) {
        if let Some(p) = self.people.get_mut(person) {
            p.mind = MindKind::Ai;
        }
    }

    /// Start a new playthrough: future randomness diverges from any other
    /// playthrough of the same world, while this one stays reproducible (S22).
    pub fn begin_playthrough(&mut self, salt: u64) {
        self.playthrough = pw_core::rng::hash_key(&[self.playthrough, salt]);
        self.seed = pw_core::rng::hash_key(&[self.seed, pw_core::rng::stream::PLAYTHROUGH, self.playthrough]);
    }

    #[inline]
    pub fn life(&self, person: PersonId) -> &Life {
        &self.lives[person]
    }

    #[inline]
    pub fn life_mut(&mut self, person: PersonId) -> &mut Life {
        &mut self.lives[person]
    }

    /// The club a person currently works or plays for, if any.
    pub fn club_of_person(&self, person: PersonId) -> ClubId {
        let p = &self.people[person];
        if p.player.is_some() {
            let h = &self.players.hot[p.player];
            if h.status == PlayerStatus::Active {
                return h.club;
            }
        }
        if p.staff.is_some() && self.staff[p.staff].employed() {
            return self.staff[p.staff].club;
        }
        ClubId::NONE
    }

    /// The team a player currently plays for (loan club during a loan).
    pub fn playing_club(&self, p: PlayerId) -> ClubId {
        let t = self.players.hot[p].team;
        if t.is_some() { self.teams[t].club } else { ClubId::NONE }
    }

    /// The manager (as a person) of the club a player trains with.
    pub fn manager_of_player(&self, p: PlayerId) -> Option<PersonId> {
        let club = self.playing_club(p);
        if club.is_none() {
            return None;
        }
        self.clubs[club].manager.get().map(|m| self.staff[m].person)
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
