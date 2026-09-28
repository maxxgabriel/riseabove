//! Contextual incidents (S9, S11): things that happen *because the world made
//! them plausible*, with a roll deciding only whether they happen now.
//!
//! Each kind of incident has a definition: who or what it can happen to
//! (scope), a baseline hazard, the pressures that raise it (with weights),
//! where it happens, how visible and sensitive it is. When one happens it is
//! recorded with its parties, witnesses, severity and the pressures that
//! made it likely — so `why` can always answer — and it becomes information
//! that travels, a response from whoever has authority, memories, and
//! follow-up pressure (unresolved tension makes the next row likelier).

use pw_core::{ClubId, Date, EventId, FixtureId, NationId, PersonId, PlayerId, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum IncidentKind {
    // In the squad.
    TrainingConfrontation,
    TacticalDisagreement,
    StormedOut,
    LateArrival,
    // Around the club and its fixtures.
    EquipmentProblem,
    PitchDamage,
    TravelDelay,
    Postponement,
    VisaProblem,
    RegistrationError,
    PaperworkProblem,
    CoachResigned,
    StaffPoached,
    // Personal.
    FamilyEmergency,
    RelationshipConflict,
    Pregnancy,
    MovingProblem,
    Burglary,
    ExamClash,
    ChildcareClash,
    UnexpectedBill,
    // Club and world.
    OwnershipControversy,
    SponsorCollapse,
    EconomicDownturn,
    FacilityDamage,
    TransportDisruption,
    SevereWeather,
    FederationDispute,
    Investigation,
    SupporterUnrest,
}

/// What an incident can happen to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Scope {
    /// Two members of one squad (or a player and the manager).
    SquadPair,
    Player,
    Staff,
    Person,
    Club,
    Fixture,
    Nation,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Location {
    TrainingGround,
    DressingRoom,
    Stadium,
    Travel,
    Home,
    Office,
    City,
    Nationwide,
}

/// Who may learn of it directly.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Exposure {
    /// Only those involved (and whoever they tell).
    Private,
    /// The club's people.
    Club,
    /// Everyone.
    Public,
}

/// Pressures that make incidents more or less likely. Every one is computed
/// from state; the contributions are stored on the incident for `why`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Pressure {
    /// Grievances and bad memories between the parties.
    Resentment,
    /// Competing for the same place.
    PositionRivalry,
    /// A short temper.
    Temper,
    LowMorale,
    /// Recently criticised in public.
    PublicCriticism,
    /// Heavy training load and fatigue.
    TrainingLoad,
    /// A divided dressing room.
    RoomTension,
    /// Something between them never got resolved.
    Unresolved,
    Unprofessional,
    Nightlife,
    Stress,
    /// Newly arrived from abroad.
    NewAbroad,
    Winter,
    ClubFinances,
    OwnerMeddling,
    PoorResults,
    Fame,
    ManagerPressure,
    /// Staff unhappy with the manager.
    StaffDiscontent,
    /// Staff better than their club.
    Overqualified,
    /// Young children at home.
    Household,
    Studying,
    WeakEconomy,
    /// Ageing, under-invested facilities.
    OldFacilities,
    /// Long trips and busy calendars.
    Congestion,
    /// A partner relationship under strain.
    StrainedRelationship,
    /// A recent move.
    RecentMove,
    Wealth,
    /// Supporters' anger at the owner.
    FanAnger,
}

impl Pressure {
    pub const fn label(self) -> &'static str {
        match self {
            Pressure::Resentment => "existing resentment",
            Pressure::PositionRivalry => "competition for the same place",
            Pressure::Temper => "a short temper",
            Pressure::LowMorale => "low morale",
            Pressure::PublicCriticism => "recent public criticism",
            Pressure::TrainingLoad => "a heavy training load",
            Pressure::RoomTension => "tension in the dressing room",
            Pressure::Unresolved => "something left unresolved",
            Pressure::Unprofessional => "a lax attitude",
            Pressure::Nightlife => "late nights",
            Pressure::Stress => "stress",
            Pressure::NewAbroad => "being new in a foreign country",
            Pressure::Winter => "winter weather",
            Pressure::ClubFinances => "the club's finances",
            Pressure::OwnerMeddling => "an interfering owner",
            Pressure::PoorResults => "poor results",
            Pressure::Fame => "fame",
            Pressure::ManagerPressure => "pressure on the manager",
            Pressure::StaffDiscontent => "staff unhappy with the manager",
            Pressure::Overqualified => "better offers elsewhere",
            Pressure::Household => "a young family",
            Pressure::Studying => "studying alongside football",
            Pressure::WeakEconomy => "a weak economy",
            Pressure::OldFacilities => "ageing facilities",
            Pressure::Congestion => "a crowded calendar and long trips",
            Pressure::StrainedRelationship => "a strained relationship",
            Pressure::RecentMove => "a recent move",
            Pressure::Wealth => "visible wealth",
            Pressure::FanAnger => "supporters' anger",
        }
    }
}

/// The static description of an incident kind.
#[derive(Clone, Copy, Debug)]
pub struct IncidentDef {
    pub kind: IncidentKind,
    pub scope: Scope,
    /// Baseline probability per eligible subject per evaluation period.
    pub hazard: f32,
    pub pressures: &'static [(Pressure, f32)],
    pub location: Location,
    pub exposure: Exposure,
    /// Base sensitivity when it becomes information, 0–100.
    pub sensitivity: u8,
}

/// Every incident kind's definition. Weights multiply `exp(Σ wᵢ·fᵢ)` with
/// each pressure fᵢ in 0..1, so a fully pressured subject is e^Σw times
/// likelier than a calm one.
pub fn def(kind: IncidentKind) -> IncidentDef {
    use IncidentKind as K;
    use Pressure as P;
    let d = |scope: Scope, hazard: f32, pressures: &'static [(Pressure, f32)], location: Location, exposure: Exposure, sensitivity: u8| IncidentDef { kind, scope, hazard, pressures, location, exposure, sensitivity };
    match kind {
        K::TrainingConfrontation => d(
            Scope::SquadPair,
            0.0006,
            &[(P::Resentment, 2.2), (P::PositionRivalry, 1.0), (P::Temper, 1.4), (P::LowMorale, 0.8), (P::PublicCriticism, 0.7), (P::TrainingLoad, 0.5), (P::RoomTension, 0.9), (P::Unresolved, 1.8)],
            Location::TrainingGround,
            Exposure::Club,
            65,
        ),
        K::TacticalDisagreement => d(Scope::Player, 0.0015, &[(P::Resentment, 1.6), (P::Temper, 1.0), (P::LowMorale, 1.0), (P::PoorResults, 0.8), (P::RoomTension, 0.6)], Location::TrainingGround, Exposure::Club, 50),
        K::StormedOut => d(Scope::Player, 0.0008, &[(P::Temper, 1.8), (P::LowMorale, 1.2), (P::Resentment, 1.2), (P::PublicCriticism, 0.8), (P::Unprofessional, 0.6)], Location::TrainingGround, Exposure::Club, 60),
        K::LateArrival => d(Scope::Player, 0.004, &[(P::Unprofessional, 1.8), (P::Nightlife, 1.4), (P::Stress, 0.6), (P::Household, 0.5), (P::LowMorale, 0.5)], Location::TrainingGround, Exposure::Club, 25),
        K::EquipmentProblem => d(Scope::Club, 0.01, &[(P::OldFacilities, 2.0), (P::ClubFinances, 1.0)], Location::TrainingGround, Exposure::Club, 10),
        K::PitchDamage => d(Scope::Club, 0.01, &[(P::Winter, 1.8), (P::OldFacilities, 1.5), (P::ClubFinances, 0.6)], Location::Stadium, Exposure::Public, 15),
        K::TravelDelay => d(Scope::Fixture, 0.01, &[(P::Winter, 1.0), (P::Congestion, 1.4)], Location::Travel, Exposure::Public, 10),
        K::Postponement => d(Scope::Fixture, 0.002, &[(P::Winter, 2.5)], Location::Stadium, Exposure::Public, 5),
        K::VisaProblem => d(Scope::Player, 0.02, &[(P::NewAbroad, 2.5), (P::ClubFinances, 0.5)], Location::Office, Exposure::Club, 30),
        K::RegistrationError => d(Scope::Club, 0.004, &[(P::ClubFinances, 1.2), (P::ManagerPressure, 0.5)], Location::Office, Exposure::Public, 35),
        K::PaperworkProblem => d(Scope::Club, 0.01, &[(P::ClubFinances, 1.0)], Location::Office, Exposure::Private, 20),
        K::CoachResigned => d(Scope::Staff, 0.003, &[(P::StaffDiscontent, 2.2), (P::Overqualified, 1.0), (P::Stress, 0.8), (P::ClubFinances, 0.6)], Location::Office, Exposure::Club, 35),
        K::StaffPoached => d(Scope::Staff, 0.002, &[(P::Overqualified, 2.4), (P::StaffDiscontent, 0.8)], Location::Office, Exposure::Public, 30),
        K::FamilyEmergency => d(Scope::Person, 0.004, &[(P::Household, 1.2), (P::Stress, 0.4)], Location::Home, Exposure::Private, 55),
        K::RelationshipConflict => d(Scope::Person, 0.01, &[(P::StrainedRelationship, 2.2), (P::Stress, 1.0), (P::Nightlife, 0.8), (P::Fame, 0.4)], Location::Home, Exposure::Private, 60),
        K::Pregnancy => d(Scope::Person, 0.0, &[], Location::Home, Exposure::Private, 40),
        K::MovingProblem => d(Scope::Person, 0.05, &[(P::RecentMove, 2.5), (P::NewAbroad, 1.0)], Location::Home, Exposure::Private, 10),
        K::Burglary => d(Scope::Person, 0.002, &[(P::Wealth, 1.8), (P::Fame, 1.2)], Location::Home, Exposure::Public, 30),
        K::ExamClash => d(Scope::Person, 0.05, &[(P::Studying, 3.0), (P::Congestion, 0.8)], Location::City, Exposure::Private, 10),
        K::ChildcareClash => d(Scope::Person, 0.01, &[(P::Household, 2.2), (P::StrainedRelationship, 0.8)], Location::Home, Exposure::Private, 15),
        K::UnexpectedBill => d(Scope::Person, 0.01, &[(P::Wealth, -0.5), (P::Stress, 0.5)], Location::Home, Exposure::Private, 10),
        K::OwnershipControversy => d(Scope::Club, 0.004, &[(P::OwnerMeddling, 2.0), (P::FanAnger, 1.4), (P::ClubFinances, 1.0)], Location::Office, Exposure::Public, 50),
        K::SponsorCollapse => d(Scope::Club, 0.0008, &[(P::WeakEconomy, 2.5)], Location::City, Exposure::Public, 30),
        K::EconomicDownturn => d(Scope::Nation, 0.003, &[(P::WeakEconomy, 2.0)], Location::Nationwide, Exposure::Public, 20),
        K::FacilityDamage => d(Scope::Club, 0.0015, &[(P::OldFacilities, 2.0), (P::Winter, 1.0)], Location::TrainingGround, Exposure::Public, 20),
        K::TransportDisruption => d(Scope::Nation, 0.01, &[(P::Winter, 1.0), (P::WeakEconomy, 1.0)], Location::Nationwide, Exposure::Public, 10),
        K::SevereWeather => d(Scope::Nation, 0.01, &[(P::Winter, 3.0)], Location::Nationwide, Exposure::Public, 10),
        K::FederationDispute => d(Scope::Nation, 0.004, &[(P::Congestion, 1.2), (P::WeakEconomy, 0.6)], Location::Nationwide, Exposure::Public, 30),
        K::Investigation => d(Scope::Club, 0.002, &[(P::ClubFinances, 2.6), (P::OwnerMeddling, 0.8)], Location::Office, Exposure::Public, 60),
        K::SupporterUnrest => d(Scope::Club, 0.01, &[(P::FanAnger, 2.8), (P::PoorResults, 1.4), (P::OwnerMeddling, 0.8)], Location::Stadium, Exposure::Public, 45),
    }
}

impl IncidentKind {
    pub const ALL: [IncidentKind; 30] = [
        IncidentKind::TrainingConfrontation,
        IncidentKind::TacticalDisagreement,
        IncidentKind::StormedOut,
        IncidentKind::LateArrival,
        IncidentKind::EquipmentProblem,
        IncidentKind::PitchDamage,
        IncidentKind::TravelDelay,
        IncidentKind::Postponement,
        IncidentKind::VisaProblem,
        IncidentKind::RegistrationError,
        IncidentKind::PaperworkProblem,
        IncidentKind::CoachResigned,
        IncidentKind::StaffPoached,
        IncidentKind::FamilyEmergency,
        IncidentKind::RelationshipConflict,
        IncidentKind::Pregnancy,
        IncidentKind::MovingProblem,
        IncidentKind::Burglary,
        IncidentKind::ExamClash,
        IncidentKind::ChildcareClash,
        IncidentKind::UnexpectedBill,
        IncidentKind::OwnershipControversy,
        IncidentKind::SponsorCollapse,
        IncidentKind::EconomicDownturn,
        IncidentKind::FacilityDamage,
        IncidentKind::TransportDisruption,
        IncidentKind::SevereWeather,
        IncidentKind::FederationDispute,
        IncidentKind::Investigation,
        IncidentKind::SupporterUnrest,
    ];

    /// Incidents where one party behaved badly toward another/the club.
    pub const fn is_conduct(self) -> bool {
        matches!(self, IncidentKind::TrainingConfrontation | IncidentKind::TacticalDisagreement | IncidentKind::StormedOut | IncidentKind::LateArrival)
    }

    /// Incidents where the person may need time away.
    pub const fn needs_leave(self) -> bool {
        matches!(self, IncidentKind::FamilyEmergency | IncidentKind::ChildcareClash | IncidentKind::ExamClash)
    }
}

/// Something the people involved are asked to do.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Ask {
    /// Ask for time away (family, childcare, exams).
    RequestLeave,
    /// Apologise to the other party.
    Apologise,
}

/// What someone with authority did about it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Response {
    Fine,
    Drop,
    DemandApology,
    Mediate,
    InvolveCaptain,
    KeepPrivate,
    Ignore,
    /// Shield one party (and so, implicitly, blame the other).
    Protect(PersonId),
    Delay,
    /// Speak about it publicly.
    Statement,
    GrantLeave,
    RefuseLeave,
    /// Club-level: apologise to supporters / promise change.
    Apologise,
    /// Club-level: dig in and dismiss the criticism.
    Defy,
    /// Club-level: hold an internal inquiry.
    Inquiry,
}

impl Response {
    pub const fn label(self) -> &'static str {
        match self {
            Response::Fine => "fined those involved",
            Response::Drop => "dropped the player",
            Response::DemandApology => "demanded an apology",
            Response::Mediate => "sat them down together",
            Response::InvolveCaptain => "asked the captain to sort it out",
            Response::KeepPrivate => "kept it in-house",
            Response::Ignore => "let it go",
            Response::Protect(_) => "took one side",
            Response::Delay => "put off dealing with it",
            Response::Statement => "spoke about it publicly",
            Response::GrantLeave => "granted time away",
            Response::RefuseLeave => "refused time away",
            Response::Apologise => "apologised",
            Response::Defy => "dismissed the criticism",
            Response::Inquiry => "opened an internal inquiry",
        }
    }
}

/// Why an authority chose a response (for audits).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Reason {
    Discipline,
    Empathy,
    Results,
    Favouritism,
    Avoidance,
    Authority,
    ClubCulture,
    BoardPressure,
    Hierarchy,
    Evidence,
    MediaStyle,
    Temper,
}

impl Reason {
    pub const fn label(self) -> &'static str {
        match self {
            Reason::Discipline => "a disciplinarian streak",
            Reason::Empathy => "a feel for people",
            Reason::Results => "the next result mattering more",
            Reason::Favouritism => "favouritism",
            Reason::Avoidance => "avoiding conflict",
            Reason::Authority => "their standing in the club",
            Reason::ClubCulture => "the club's traditions",
            Reason::BoardPressure => "pressure from the board",
            Reason::Hierarchy => "who was involved",
            Reason::Evidence => "how much they really knew",
            Reason::MediaStyle => "how they deal with the press",
            Reason::Temper => "temper",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ResponseRecord {
    pub by: PersonId,
    pub response: Response,
    pub date: Date,
    pub reasons: [(Reason, u8); 3],
    pub event: EventId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Incident {
    pub id: u32,
    pub kind: IncidentKind,
    pub date: Date,
    pub club: ClubId,
    pub nation: NationId,
    pub location: Location,
    /// Principal people (instigator first where there is one).
    pub parties: SmallVec<[PersonId; 3]>,
    pub players: SmallVec<[PlayerId; 3]>,
    pub staff: StaffId,
    pub fixture: FixtureId,
    pub witnesses: SmallVec<[PersonId; 6]>,
    /// 0–100.
    pub severity: u8,
    /// The pressures that made it plausible, with their contribution (0–255).
    pub pressures: SmallVec<[(Pressure, u8); 4]>,
    pub event: EventId,
    /// The information item it became.
    pub info: u32,
    pub responses: SmallVec<[ResponseRecord; 2]>,
    /// Those who know were told to keep quiet.
    pub hushed: bool,
    pub resolved: bool,
    /// A later incident this one led to.
    pub led_to: Option<u32>,
    /// An earlier incident that led to this one.
    pub follows: Option<u32>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Incidents {
    pub list: Vec<Incident>,
    /// Unresolved tension between two people (ordered pair), 0–100.
    pub tension: FxHashMap<(PersonId, PersonId), u8>,
    /// Players away (leave) or left out for discipline until a date.
    pub away: FxHashMap<PlayerId, Date>,
    pub dropped: FxHashMap<PlayerId, Date>,
    /// Decisions deferred: (incident, authority, reconsider on).
    pub deferred: Vec<(u32, PersonId, Date)>,
    /// Pregnancies: (parent, other parent, due date, incident).
    pub expecting: Vec<(PersonId, PersonId, Date, u32)>,
    /// Open investigations: club → (verdict date, incident).
    pub investigations: FxHashMap<ClubId, (Date, u32)>,
    /// Nationwide conditions in force: (nation, kind, until, incident).
    pub national: Vec<(NationId, IncidentKind, Date, u32)>,
}

impl Incidents {
    pub fn get(&self, id: u32) -> Option<&Incident> {
        self.list.get(id as usize)
    }

    fn key(a: PersonId, b: PersonId) -> (PersonId, PersonId) {
        if a <= b { (a, b) } else { (b, a) }
    }

    pub fn tension(&self, a: PersonId, b: PersonId) -> u8 {
        self.tension.get(&Self::key(a, b)).copied().unwrap_or(0)
    }

    pub fn add_tension(&mut self, a: PersonId, b: PersonId, by: i16) {
        let k = Self::key(a, b);
        let v = i16::from(self.tension.get(&k).copied().unwrap_or(0)) + by;
        if v <= 0 {
            self.tension.remove(&k);
        } else {
            self.tension.insert(k, v.min(100) as u8);
        }
    }

    pub fn is_away(&self, p: PlayerId, today: Date) -> bool {
        self.away.get(&p).is_some_and(|&d| d > today) || self.dropped.get(&p).is_some_and(|&d| d > today)
    }

    /// A nationwide condition (weather, transport) in force today.
    pub fn national_active(&self, n: NationId, kind: IncidentKind, today: Date) -> Option<u32> {
        self.national.iter().find(|x| x.0 == n && x.1 == kind && x.2 >= today).map(|x| x.3)
    }
}
