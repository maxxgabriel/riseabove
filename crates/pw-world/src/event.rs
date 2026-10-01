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
    TrainingSlump {
        player: PlayerId,
        weeks: u8,
    },
    /// Training ratings well above norm for `weeks`.
    TrainingSurge {
        player: PlayerId,
        weeks: u8,
    },
    /// Recent match ratings well below the player's norm.
    FormSlump {
        player: PlayerId,
    },
    FormSurge {
        player: PlayerId,
    },
    /// Share of available minutes against what the player's status promises.
    MinutesShortfall {
        player: PlayerId,
        share_pct: u8,
        expected_pct: u8,
    },
    /// A club has been watching a player (scouting, analysis, agent pitch).
    Tracking {
        club: ClubId,
        player: PlayerId,
        minutes: u16,
    },
    /// A club's squad plan has a hole in this player's position group.
    SquadNeed {
        club: ClubId,
    },
    BoardPressure {
        club: ClubId,
        warnings: u8,
    },
    Congestion {
        team: TeamId,
        matches: u8,
    },
    /// A remembered episode between two people.
    Memory {
        from: PersonId,
        about: PersonId,
        kind: MemoryKind,
    },
    PromiseDue {
        promise: u32,
    },
    /// Weekly wage relative to squad peers of similar standing, percent.
    WageGap {
        player: PlayerId,
        pct_of_peers: u16,
    },
    /// Family and partner circumstances weighed in a decision.
    Household {
        person: PersonId,
    },
    Injury {
        player: PlayerId,
        days: u16,
    },
    /// Living abroad without the language or roots yet.
    Unsettled {
        person: PersonId,
        nation: NationId,
    },
    /// Low trust between two people.
    LowTrust {
        from: PersonId,
        about: PersonId,
        trust: u8,
    },
    ContractRunningDown {
        player: PlayerId,
        days: u16,
    },
    PublicCriticism {
        story: StoryId,
    },
    /// A football rule stood in the way.
    Rule {
        reason: crate::rules::Reason,
    },
    /// Someone said it on the record.
    Said {
        person: PersonId,
    },
    /// A match was played (fixture uid).
    Played {
        fixture: u64,
    },
    /// How the media/analysts currently read a player.
    Reading {
        player: PlayerId,
    },
    /// A published ranking.
    Ranking,
    /// An anniversary of a season.
    Anniversary {
        year: i32,
    },
    /// Someone heard something (an information item) from someone.
    Heard {
        info: u32,
        from: PersonId,
    },
    /// A pressure that made an incident plausible, and how strongly.
    Pressure {
        pressure: crate::incident::Pressure,
        level: u8,
    },
    /// A disposition that shaped someone's response.
    Disposition {
        person: PersonId,
        reason: crate::incident::Reason,
        level: u8,
    },
    /// A run of results anyone can see.
    PoorRun {
        club: ClubId,
        defeats: u8,
        games: u8,
    },
    /// How newsworthy a story was judged, and why (0–100 each).
    Newsworthy {
        importance: u8,
        relevance: u8,
        controversy: u8,
    },
    /// A supporter post that spread (see `World::net`).
    Viral {
        post: u32,
        reposts: u32,
    },
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
    Transfer {
        player: PlayerId,
        from: ClubId,
        to: ClubId,
        fee: Money,
    },
    LoanMove {
        player: PlayerId,
        from: ClubId,
        to: ClubId,
        until: Date,
    },
    LoanReturn {
        player: PlayerId,
        to: ClubId,
    },
    ContractSigned {
        player: PlayerId,
        club: ClubId,
        wage: Money,
        until: Date,
        renewal: bool,
    },
    Released {
        player: PlayerId,
        club: ClubId,
    },
    Retired {
        person: PersonId,
    },
    Injured {
        player: PlayerId,
        injury: u16,
        days: u16,
    },
    Recovered {
        player: PlayerId,
    },
    Suspended {
        player: PlayerId,
        matches: u8,
    },
    ManagerSacked {
        staff: StaffId,
        club: ClubId,
    },
    ManagerAppointed {
        staff: StaffId,
        club: ClubId,
    },
    YouthIntake {
        club: ClubId,
        count: u8,
    },
    Debut {
        player: PlayerId,
        team: TeamId,
        comp: CompId,
    },
    FirstGoal {
        player: PlayerId,
        team: TeamId,
        comp: CompId,
    },
    Champion {
        comp: CompId,
        team: TeamId,
        season: i32,
    },
    Promoted {
        comp: CompId,
        team: TeamId,
    },
    Relegated {
        comp: CompId,
        team: TeamId,
    },
    Interest {
        player: PlayerId,
        club: ClubId,
    },
    BidRejected {
        player: PlayerId,
        club: ClubId,
        fee: Money,
    },
    BidAccepted {
        player: PlayerId,
        club: ClubId,
        fee: Money,
    },
    TransferListed {
        player: PlayerId,
        club: ClubId,
    },
    Award {
        player: PlayerId,
        comp: CompId,
        award: AwardKind,
        season: i32,
    },
    CallUp {
        player: PlayerId,
    },

    /// A conversation took place (details in `World::meetings`).
    Meeting {
        meeting: MeetingId,
        from: PersonId,
        with: PersonId,
    },
    PromiseMade {
        promise: u32,
        from: PersonId,
        to: PersonId,
    },
    PromiseKept {
        promise: u32,
        from: PersonId,
        to: PersonId,
    },
    PromiseBroken {
        promise: u32,
        from: PersonId,
        to: PersonId,
    },
    TransferRequested {
        player: PlayerId,
        club: ClubId,
    },
    TransferRequestWithdrawn {
        player: PlayerId,
        club: ClubId,
    },
    Fined {
        player: PlayerId,
        club: ClubId,
        amount: Money,
    },
    /// Dressing-room unrest spreading from an unhappy, influential player.
    Unrest {
        club: ClubId,
        player: PlayerId,
    },
    /// Contract talks opened, moved, or ended (details in `World::talks`).
    TalksOpened {
        talk: TalkId,
        player: PlayerId,
        club: ClubId,
    },
    TalksCollapsed {
        talk: TalkId,
        player: PlayerId,
        club: ClubId,
    },
    /// A journalist published something (details in `World::stories`).
    Published {
        story: StoryId,
    },
    AgentHired {
        player: PlayerId,
        agent: AgentId,
    },
    AgentLeft {
        player: PlayerId,
        agent: AgentId,
    },
    /// A club heard about a player through their agent.
    AgentPitch {
        player: PlayerId,
        agent: AgentId,
        club: ClubId,
    },
    Life {
        person: PersonId,
        kind: LifeEventKind,
    },
    /// A person took a football job after (or instead of) playing.
    JoinedStaff {
        person: PersonId,
        staff: StaffId,
        club: ClubId,
    },
    CameOutOfRetirement {
        person: PersonId,
    },
    /// A coach's note on a player's training (club-internal).
    CoachNote {
        player: PlayerId,
        by: PersonId,
        note: CoachNote,
    },
    /// A manager changed a player's squad status.
    StatusChanged {
        player: PlayerId,
        club: ClubId,
        from: crate::contract::SquadStatus,
        to: crate::contract::SquadStatus,
    },
    /// A manager named a new captain.
    Captaincy {
        player: PlayerId,
        team: TeamId,
    },
    /// A club changed hands.
    Takeover {
        club: ClubId,
        owner: PersonId,
        previous: PersonId,
    },
    /// A club entered administration.
    Administration {
        club: ClubId,
    },
    PointsDeducted {
        club: ClubId,
        points: u8,
    },
    /// The board cut budgets and put earners up for sale.
    Austerity {
        club: ClubId,
    },
    /// The owner put money in.
    OwnerInvestment {
        club: ClubId,
        amount: Money,
    },
    ProjectStarted {
        club: ClubId,
        kind: crate::governance::ProjectKind,
    },
    ProjectCompleted {
        club: ClubId,
        kind: crate::governance::ProjectKind,
    },
    /// A nation's top flight signed a new broadcast deal.
    BroadcastDeal {
        nation: NationId,
        pool: Money,
    },
    ManagerResigned {
        staff: StaffId,
        club: ClubId,
    },
    /// A club lured another club's manager away, paying compensation.
    ManagerPoached {
        staff: StaffId,
        from: ClubId,
        to: ClubId,
        compensation: Money,
    },
    /// A manager switched to a new system.
    TacticalChange {
        club: ClubId,
        staff: StaffId,
        formation: u8,
    },
    /// A member of staff followed their manager to a new club.
    StaffFollowed {
        staff: StaffId,
        manager: StaffId,
        club: ClubId,
    },
    StaffLeft {
        staff: StaffId,
        club: ClubId,
    },
    /// A club-to-club deal fell through.
    DealCollapsed {
        player: PlayerId,
        buyer: ClubId,
        seller: ClubId,
        reason: crate::deals::DealEnd,
    },
    PreContractSigned {
        player: PlayerId,
        club: ClubId,
    },
    TrialStarted {
        player: PlayerId,
        club: ClubId,
    },
    TrialEnded {
        player: PlayerId,
        club: ClubId,
        offered: bool,
    },
    LoanRecalled {
        player: PlayerId,
        club: ClubId,
    },
    OptionExercised {
        player: PlayerId,
        club: ClubId,
        fee: Money,
    },
    AddOnPaid {
        player: PlayerId,
        from: ClubId,
        to: ClubId,
        amount: Money,
    },
    SellOnPaid {
        player: PlayerId,
        to: ClubId,
        amount: Money,
    },
    /// A child or teenager joined an academy after a trial.
    AcademyJoined {
        player: PlayerId,
        club: ClubId,
    },
    /// An academy let a young player go at its review.
    AcademyReleased {
        player: PlayerId,
        club: ClubId,
    },
    ScholarshipOffered {
        player: PlayerId,
        club: ClubId,
    },
    /// Signed up with a local grassroots or amateur side.
    JoinedLocalClub {
        player: PlayerId,
        local: pw_core::LocalClubId,
    },
    AcademyTrialStarted {
        player: PlayerId,
        club: ClubId,
    },
    /// A year on, a club evaluator's old reading of a young player turned out far too low (or, rarely, spot on) for someone now well known.
    AssessmentVindicated {
        staff: StaffId,
        player: PlayerId,
        club: ClubId,
        was_right: bool,
    },
    /// A club looked back on a signing: how it turned out, and whether the decision or the luck was to blame.
    /// An option on a contract was taken up, or passed on.
    ContractOption {
        player: PlayerId,
        club: ClubId,
        kind: OptionKind,
        taken: bool,
    },
    /// A plan a club made turned out wrong, and it shows.
    PlanFailed {
        club: ClubId,
        player: PlayerId,
        kind: crate::boardroom::PlanFailure,
    },
    SigningReviewed {
        player: PlayerId,
        club: ClubId,
        verdict: crate::boardroom::Verdict,
        /// Someone at the club had opposed it and been overruled.
        overruled: bool,
    },
    /// A manager changed how his side played during a match, and why he said he did.
    MatchTacticsChanged {
        club: ClubId,
        manager: StaffId,
        minute: u8,
        half_time: bool,
        response: crate::tactics::Response,
        believed: crate::tactics::Diagnosis,
    },
    /// A manager decided what to do about a player who was struggling privately.
    PersonalMatterHandled {
        player: PlayerId,
        manager: PersonId,
        handling: crate::lifestate::Handling,
        believed: crate::lifestate::LoadKind,
    },
    /// A player played well (or badly) through something heavy in his life.
    PerformedThroughStrain {
        player: PlayerId,
        kind: crate::lifestate::LoadKind,
        well: bool,
    },
    /// A major memory came back: the ground, the situation.
    MemoryReturned {
        player: PlayerId,
        scar: crate::lifestate::ScarKind,
    },
    /// A move stopped being new: he settled, or it never worked out.
    AdaptationEnded {
        player: PlayerId,
        club: ClubId,
        weeks: u16,
        struggled: bool,
    },
    /// Settling in is going badly, and on which front.
    AdaptationStruggling {
        player: PlayerId,
        club: ClubId,
        channel: crate::adaptation::Channel,
    },
    /// A story left someone with a grievance against a journalist that will not go away.
    MediaGrudge {
        subject: PersonId,
        journalist: PersonId,
        cause: crate::media::BondCause,
    },
    /// Attention broke over someone and beyond football.
    AttentionSurge {
        person: PersonId,
        cause: crate::attention::Cause,
    },
    ExamsSat {
        person: PersonId,
        passed: bool,
    },
    /// A national side named its squad (the player was in it).
    NationalSquad {
        player: PlayerId,
        nation: NationId,
        level: crate::intl::Level,
    },
    InternationalDebut {
        player: PlayerId,
        nation: NationId,
        level: crate::intl::Level,
    },
    /// Result of an international match (index into `World::intl::matches`).
    InternationalResult {
        index: u32,
    },
    TournamentWon {
        nation: NationId,
        tournament: u32,
    },
    /// A dual national committed to one country.
    ChoseNation {
        player: PlayerId,
        nation: NationId,
    },
    RetiredFromInternational {
        player: PlayerId,
        nation: NationId,
    },
    /// A federation appointed a manager for one of its sides.
    NationalManagerAppointed {
        staff: StaffId,
        nation: NationId,
        level: crate::intl::Level,
    },
    /// A national manager left (sacked after failure, poached, retired).
    NationalManagerLeft {
        staff: StaffId,
        nation: NationId,
        level: crate::intl::Level,
        sacked: bool,
    },
    /// A player was withdrawn from a national squad (injury, club pressure, refusal).
    WithdrewFromSquad {
        player: PlayerId,
        nation: NationId,
    },
    /// The medical team's verdict on an injury.
    Diagnosed {
        player: PlayerId,
        injury: u16,
        estimate: u16,
        treatment: crate::medical::Treatment,
    },
    InjurySetback {
        player: PlayerId,
        days: u16,
    },
    RushedBack {
        player: PlayerId,
    },
    /// A step on a player's route through football: joining a school, an academy, a university, a state team.
    PathwayStep {
        player: PlayerId,
        kind: u8,
        target: u32,
    },
    /// A young player's name spread beyond their own pitch: a clip that travelled (`earned` false) or a run of
    /// steady football that people finally noticed (`earned` true). `tier` is the level (`ecosystem::Tier`).
    Breakout {
        player: PlayerId,
        tier: u8,
        earned: bool,
    },
    /// A material decision was made; the ruling holds who believed and backed what.
    Ruling {
        ruling: u32,
    },
    ChronicCondition {
        player: PlayerId,
    },
    /// A newcomer has become part of the dressing room.
    PlayerSettled {
        player: PlayerId,
        club: ClubId,
    },
    /// A leader's group has turned against the manager.
    DressingRoomSplit {
        club: ClubId,
        leader: PlayerId,
    },
    LeaderEmerged {
        player: PlayerId,
        club: ClubId,
    },
    /// An experienced player took a younger one under their wing.
    TookUnderWing {
        mentor: PersonId,
        mentee: PersonId,
    },
    /// A player has visibly changed as a person/professional.
    CharacterChanged {
        person: PersonId,
        up: bool,
    },
    /// Went too long without football; the ceiling came down.
    Stagnated {
        player: PlayerId,
    },
    Milestone {
        player: PlayerId,
        kind: MilestoneKind,
        count: u16,
        club: ClubId,
    },
    RecordBroken {
        player: PlayerId,
        kind: RecordKind,
        club: ClubId,
        value: i64,
    },
    /// A club's supporters now count a player among their legends.
    BecameLegend {
        person: PersonId,
        club: ClubId,
    },
    InductedHallOfFame {
        person: PersonId,
    },
    ManagerOfSeason {
        staff: StaffId,
        comp: CompId,
        season: i32,
    },
    EnrolledCourse {
        person: PersonId,
        course: crate::affairs::Course,
    },
    Qualified {
        person: PersonId,
        course: crate::affairs::Course,
    },
    MovedHome {
        person: PersonId,
        bought: bool,
    },
    HiredHelper {
        person: PersonId,
        helper: crate::affairs::Helper,
    },
    /// Started a foundation or a visible community commitment.
    GaveBack {
        person: PersonId,
        foundation: bool,
    },
    Endorsed {
        person: PersonId,
        brand: u32,
        fee_year: Money,
    },
    EndorsementEnded {
        person: PersonId,
        brand: u32,
        why: crate::commerce::DealEnd,
    },
    /// A personal deal collides with a club partner in the same sector.
    SponsorClash {
        person: PersonId,
        brand: u32,
        club: ClubId,
    },
    ClubSponsor {
        club: ClubId,
        brand: u32,
        slot: crate::commerce::ClubSlot,
        fee_year: Money,
    },
    /// Began a working life after (or beside) playing.
    NewCareer {
        person: PersonId,
        path: crate::affairs::CareerPath,
    },
    CareerEnded {
        person: PersonId,
        path: crate::affairs::CareerPath,
    },
    Investment {
        person: PersonId,
        gain: Money,
    },
    /// The board issued a private warning to its manager.
    BoardWarning {
        club: ClubId,
        manager: StaffId,
        warnings: u8,
    },
    /// The board asked the manager to explain something it heard.
    BoardQuery {
        club: ClubId,
        manager: PersonId,
        info: u32,
    },
    /// Someone believes a colleague leaked to the press.
    LeakSuspected {
        by: PersonId,
        suspect: PersonId,
        info: u32,
    },
    /// An agent began quietly sounding out clubs for a client.
    AgentExploring {
        agent: PersonId,
        player: PlayerId,
    },
    /// An incident (see `World::incidents`).
    Incident {
        incident: u32,
        kind: crate::incident::IncidentKind,
    },
    /// Someone in authority responded to an incident.
    IncidentResponse {
        incident: u32,
        by: PersonId,
        response: crate::incident::Response,
    },
    /// A captain settled a feud on their own initiative.
    CaptainMediated {
        captain: PersonId,
        a: PersonId,
        b: PersonId,
    },
    InvestigationCleared {
        club: ClubId,
    },
    JournalistMoved {
        person: PersonId,
        from: pw_core::OutletId,
        to: pw_core::OutletId,
    },
    JournalistLeft {
        person: PersonId,
        outlet: pw_core::OutletId,
    },
    JournalistHired {
        person: PersonId,
        outlet: pw_core::OutletId,
    },
    EnrolledUniversity {
        person: PersonId,
        institution: u32,
    },
    /// Left university (`early`: to turn professional).
    Graduated {
        person: PersonId,
        institution: u32,
        early: bool,
    },
    /// A minor competition's season finished (`World::minor.history` index).
    MinorTitle {
        history: u32,
    },
    /// A record fell (`World::records.broken` index).
    Record {
        broken: u32,
        person: PersonId,
        club: ClubId,
    },
    /// A voted award was decided (`World::acclaim.votes` index).
    Voted {
        vote: u32,
        person: PersonId,
    },
    /// Someone entered a hall of fame (other than the world's, which has
    /// `InductedHallOfFame`).
    HallInduction {
        hall: u32,
        person: PersonId,
    },
    /// An entry in the world's chronicle of achievements.
    Chronicle {
        entry: u32,
    },
    /// A big refereeing call that one side's supporters dispute.
    RefereeControversy {
        controversy: u32,
    },
    AppealDecided {
        appeal: u32,
        player: PlayerId,
    },
    /// A club was charged by its federation (`World::officials.charges`).
    Charged {
        charge: u32,
        club: ClubId,
    },
    /// A tactical school was born around a manager.
    SchoolFounded {
        school: u32,
        founder: PersonId,
    },
    /// A federation changed a rule (`World::evolution.changes`).
    RuleChanged {
        change: u32,
    },
    /// A supporter group acted together (see `World::net.groups`).
    SupporterAction {
        club: ClubId,
        group: u32,
        action: crate::socialnet::GroupAction,
    },
    /// A player wanted by more than one university chose `institution` over `over` (the best rival offer); `raised` when the winner had
    /// improved its scholarship to win, `round` the recruiting round it was settled in.
    RecruitWon {
        person: PersonId,
        institution: u32,
        over: u32,
        raised: bool,
        round: u8,
    },
    /// A rivalry began or took on a new character (a cup revenge, a title race, a relegation scrap). Appended last: older saves'
    /// events keep their numbering.
    RivalryKindled {
        a: crate::culture::Side,
        b: crate::culture::Side,
        kind: crate::culture::RivalryKind,
    },
    /// An account's view of someone crossed a line: won over (`up`) or turned against him. Individual opinion moves are not events
    /// (there are millions); the crossing is, and it rests on the newest thing that happened to him.
    OpinionTurned {
        about: PersonId,
        account: u32,
        up: bool,
    },
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
    /// Most assists in a league season.
    Playmaker,
    /// Best goalkeeper of a league season.
    GoldenGlove,
    /// Voted best player in the world for the calendar year (by rank).
    WorldPlayer {
        rank: u8,
    },
    WorldYoungPlayer,
    /// Best player at clubs of a confederation.
    ContinentalPlayer(crate::nation::Confed),
    /// Voted best player of a league season by the league's players.
    PlayersPlayer,
}

impl AwardKind {
    pub fn label(self) -> String {
        match self {
            AwardKind::PlayerOfSeason => "Player of the Season".into(),
            AwardKind::YoungPlayerOfSeason => "Young Player of the Season".into(),
            AwardKind::TopScorer => "the Golden Boot".into(),
            AwardKind::TeamOfSeason => "a place in the Team of the Season".into(),
            AwardKind::PlayerOfMonth => "Player of the Month".into(),
            AwardKind::Playmaker => "the Playmaker award".into(),
            AwardKind::PlayersPlayer => "the Players' Player of the Season award".into(),
            AwardKind::GoldenGlove => "the Golden Glove".into(),
            AwardKind::WorldPlayer { rank: 1 } => "the World Player of the Year award".into(),
            AwardKind::WorldPlayer { rank } => format!("{} place in the World Player of the Year vote", crate::event::ordinal(rank)),
            AwardKind::WorldYoungPlayer => "the World Young Player of the Year award".into(),
            AwardKind::ContinentalPlayer(c) => format!("{} Player of the Year", c.code()),
        }
    }
}

pub fn ordinal(n: u8) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MilestoneKind {
    ClubApps,
    CareerGoals,
    SeniorApps,
    Caps,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum RecordKind {
    ClubTopScorer,
    ClubMostApps,
    ClubRecordSigning,
    ClubRecordSale,
    ClubBiggestWin,
    LeagueGoalsInSeason,
    NationMostCaps,
    NationTopScorer,
    WorldRecordFee,
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
            | DealCollapsed { player, .. }
            | PreContractSigned { player, .. }
            | TrialStarted { player, .. }
            | TrialEnded { player, .. }
            | LoanRecalled { player, .. }
            | OptionExercised { player, .. }
            | AcademyJoined { player, .. }
            | AcademyReleased { player, .. }
            | ScholarshipOffered { player, .. }
            | JoinedLocalClub { player, .. }
            | AcademyTrialStarted { player, .. }
            | AssessmentVindicated { player, .. }
            | SigningReviewed { player, .. }
            | PlanFailed { player, .. }
            | ContractOption { player, .. }
            | AdaptationEnded { player, .. }
            | AdaptationStruggling { player, .. }
            | PersonalMatterHandled { player, .. }
            | PerformedThroughStrain { player, .. }
            | MemoryReturned { player, .. }
            | NationalSquad { player, .. }
            | InternationalDebut { player, .. }
            | ChoseNation { player, .. }
            | RetiredFromInternational { player, .. }
            | WithdrewFromSquad { player, .. }
            | Diagnosed { player, .. }
            | InjurySetback { player, .. }
            | RushedBack { player }
            | PathwayStep { player, .. }
            | Breakout { player, .. }
            | ChronicCondition { player }
            | PlayerSettled { player, .. }
            | LeaderEmerged { player, .. }
            | Stagnated { player }
            | Milestone { player, .. }
            | RecordBroken { player, .. }
            | AgentExploring { player, .. }
            | AppealDecided { player, .. }
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
            MediaGrudge { subject, journalist, .. } => {
                v.push(subject);
                v.push(journalist);
            }
            AttentionSurge { person, .. } => v.push(person),
            Retired { person } | Life { person, .. } | JoinedStaff { person, .. } | CameOutOfRetirement { person } | ExamsSat { person, .. } | CharacterChanged { person, .. } => v.push(person),
            BecameLegend { person, .. } | InductedHallOfFame { person } => v.push(person),
            EnrolledCourse { person, .. }
            | Qualified { person, .. }
            | MovedHome { person, .. }
            | HiredHelper { person, .. }
            | GaveBack { person, .. }
            | Endorsed { person, .. }
            | EndorsementEnded { person, .. }
            | SponsorClash { person, .. }
            | NewCareer { person, .. }
            | CareerEnded { person, .. }
            | Investment { person, .. }
            | EnrolledUniversity { person, .. }
            | RecruitWon { person, .. }
            | Graduated { person, .. } => v.push(person),
            Record { person, .. } if person.is_some() => v.push(person),
            Voted { person, .. } | HallInduction { person, .. } => v.push(person),
            SchoolFounded { founder, .. } => v.push(founder),
            BoardQuery { manager, .. } => v.push(manager),
            IncidentResponse { by, .. } => v.push(by),
            JournalistMoved { person, .. } | JournalistLeft { person, .. } | JournalistHired { person, .. } => v.push(person),
            CaptainMediated { captain, a, b } => {
                v.push(captain);
                v.push(a);
                v.push(b);
            }
            LeakSuspected { by, suspect, .. } => {
                v.push(by);
                v.push(suspect);
            }
            TookUnderWing { mentor, mentee } => {
                v.push(mentor);
                v.push(mentee);
            }
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
            | PlayerSettled { club, .. }
            | ClubSponsor { club, .. }
            | BoardWarning { club, .. }
            | BoardQuery { club, .. }
            | InvestigationCleared { club }
            | SupporterAction { club, .. }
            | Record { club, .. }
            | Charged { club, .. }
            | DressingRoomSplit { club, .. }
            | LeaderEmerged { club, .. }
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
            | ManagerResigned { club, .. }
            | TacticalChange { club, .. }
            | MatchTacticsChanged { club, .. }
            | AdaptationEnded { club, .. }
            | AdaptationStruggling { club, .. }
            | StaffFollowed { club, .. }
            | StaffLeft { club, .. }
            | JoinedStaff { club, .. } => v.push(club),
            ManagerPoached { from, to, .. } | AddOnPaid { from, to, .. } => {
                v.push(from);
                v.push(to);
            }
            DealCollapsed { buyer, seller, .. } => {
                v.push(buyer);
                v.push(seller);
            }
            AcademyJoined { club, .. } | AcademyReleased { club, .. } | ScholarshipOffered { club, .. } | AcademyTrialStarted { club, .. } | AssessmentVindicated { club, .. } | SigningReviewed { club, .. } | PlanFailed { club, .. } | ContractOption { club, .. } => v.push(club),
            PreContractSigned { club, .. } | TrialStarted { club, .. } | TrialEnded { club, .. } | LoanRecalled { club, .. } | OptionExercised { club, .. } => v.push(club),
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

    /// The newest event of the last `within_days` days (up to `today`) that `f` accepts: the event a consequence found later rests on
    /// (the signing behind a clause review, the sacking behind a vacancy). `None` when it has been compacted away or never happened, in
    /// which case the consequence is recorded without a cause rather than with an invented one. Bounded by days, not by a count of
    /// events: how many events a day holds depends on the size of the world.
    pub fn latest_where(&self, today: Date, within_days: i32, f: impl Fn(&Event) -> bool) -> Option<EventId> {
        self.since(today.add_days(-within_days)).iter().rev().find(|e| f(e)).map(|e| e.id)
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

/// Who held an option on a contract.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum OptionKind {
    Club,
    Player,
    Mutual,
    /// An extension that a condition in the contract set off.
    Automatic,
}

impl OptionKind {
    pub const fn label(self) -> &'static str {
        match self {
            OptionKind::Club => "the club's option",
            OptionKind::Player => "the player's option",
            OptionKind::Mutual => "the mutual option",
            OptionKind::Automatic => "the automatic extension",
        }
    }
}
