//! Turning world events into sentences the viewer is allowed to read.

use pw_core::{ClubId, CompId, PlayerId};
use pw_world::event::{Event, EventKind as E, Visibility};

use crate::ctx::Ctx;
use crate::model::{Part, Ref};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Transfers,
    Career,
    Health,
    Club,
    Competition,
    Life,
    Media,
    Board,
    International,
    Incidents,
}

impl Group {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "transfers" => Group::Transfers,
            "career" => Group::Career,
            "health" => Group::Health,
            "club" => Group::Club,
            "competition" => Group::Competition,
            "life" => Group::Life,
            "media" => Group::Media,
            "board" => Group::Board,
            "international" => Group::International,
            "incidents" => Group::Incidents,
            _ => return None,
        })
    }
}

pub fn group_of(k: &E) -> Group {
    use E::*;
    match k {
        Transfer { .. } | LoanMove { .. } | LoanReturn { .. } | ContractSigned { .. } | Released { .. } | Interest { .. } | BidRejected { .. } | BidAccepted { .. } | TransferListed { .. } => {
            Group::Transfers
        }
        Retired { .. } | Debut { .. } | FirstGoal { .. } | Award { .. } | CallUp { .. } => Group::Career,
        Injured { .. } | Recovered { .. } | Suspended { .. } => Group::Health,
        ManagerSacked { .. } | ManagerAppointed { .. } | YouthIntake { .. } => Group::Club,
        Champion { .. } | Promoted { .. } | Relegated { .. } | ManagerOfSeason { .. } => Group::Competition,
        TransferRequested { .. }
        | TransferRequestWithdrawn { .. }
        | TalksOpened { .. }
        | TalksCollapsed { .. }
        | AgentHired { .. }
        | AgentLeft { .. }
        | AgentPitch { .. }
        | DealCollapsed { .. }
        | PreContractSigned { .. }
        | TrialStarted { .. }
        | TrialEnded { .. }
        | LoanRecalled { .. }
        | OptionExercised { .. }
        | AddOnPaid { .. }
        | SellOnPaid { .. } => Group::Transfers,
        Diagnosed { .. } | InjurySetback { .. } | RushedBack { .. } | ChronicCondition { .. } => Group::Health,
        Life { .. }
        | ExamsSat { .. }
        | EnrolledCourse { .. }
        | Qualified { .. }
        | MovedHome { .. }
        | HiredHelper { .. }
        | GaveBack { .. }
        | Investment { .. }
        | NewCareer { .. }
        | CareerEnded { .. }
        | JoinedStaff { .. }
        | CameOutOfRetirement { .. } => Group::Life,
        Published { .. } | Endorsed { .. } | EndorsementEnded { .. } | SponsorClash { .. } | ClubSponsor { .. } => Group::Media,
        Takeover { .. }
        | Administration { .. }
        | PointsDeducted { .. }
        | Austerity { .. }
        | OwnerInvestment { .. }
        | ProjectStarted { .. }
        | ProjectCompleted { .. }
        | BroadcastDeal { .. }
        | ManagerResigned { .. }
        | ManagerPoached { .. }
        | TacticalChange { .. }
        | StaffFollowed { .. }
        | StaffLeft { .. } => Group::Board,
        NationalSquad { .. }
        | InternationalDebut { .. }
        | InternationalResult { .. }
        | TournamentWon { .. }
        | ChoseNation { .. }
        | RetiredFromInternational { .. }
        | NationalManagerAppointed { .. }
        | NationalManagerLeft { .. }
        | WithdrewFromSquad { .. } => Group::International,
        Incident { .. } | IncidentResponse { .. } | CaptainMediated { .. } | InvestigationCleared { .. } | LeakSuspected { .. } => Group::Incidents,
        JournalistMoved { .. } | JournalistLeft { .. } | JournalistHired { .. } | SupporterAction { .. } => Group::Media,
        BoardWarning { .. } | BoardQuery { .. } => Group::Board,
        EnrolledUniversity { .. } | Graduated { .. } | RecruitWon { .. } => Group::Life,
        Record { .. } | Voted { .. } | HallInduction { .. } => Group::Career,
        MinorTitle { .. } | Chronicle { .. } | RefereeControversy { .. } | AppealDecided { .. } | Charged { .. } | SchoolFounded { .. } | RuleChanged { .. } | RivalryKindled { .. } => Group::Competition,
        OpinionTurned { .. } => Group::Media,
        AgentExploring { .. } => Group::Transfers,
        Milestone { .. }
        | RecordBroken { .. }
        | BecameLegend { .. }
        | InductedHallOfFame { .. }
        | AcademyJoined { .. }
        | AcademyReleased { .. }
        | ScholarshipOffered { .. }
        | JoinedLocalClub { .. }
        | AcademyTrialStarted { .. }
        | AssessmentVindicated { .. }
        | SigningReviewed { .. }
        | PlanFailed { .. }
        | PersonalMatterHandled { .. }
        | AdaptationEnded { .. }
        | AdaptationStruggling { .. }
        | PerformedThroughStrain { .. }
        | MemoryReturned { .. }
        | ContractOption { .. }
        | Stagnated { .. }
        | CharacterChanged { .. } => Group::Career,
        _ => Group::Club,
    }
}

pub fn label(k: &E) -> &'static str {
    use E::*;
    match k {
        Transfer { .. } => "Transfer",
        LoanMove { .. } => "Loan",
        LoanReturn { .. } => "Loan return",
        ContractSigned { renewal: true, .. } => "Renewal",
        ContractSigned { .. } => "Signing",
        Released { .. } => "Release",
        Retired { .. } => "Retirement",
        Injured { .. } => "Injury",
        Recovered { .. } => "Recovery",
        Suspended { .. } => "Suspension",
        ManagerSacked { .. } => "Dismissal",
        ManagerAppointed { .. } => "Appointment",
        YouthIntake { .. } => "Youth intake",
        Debut { .. } => "Debut",
        FirstGoal { .. } => "First goal",
        Champion { .. } => "Title",
        Promoted { .. } => "Promotion",
        Relegated { .. } => "Relegation",
        Interest { .. } => "Interest",
        BidRejected { .. } | BidAccepted { .. } => "Bid",
        TransferListed { .. } => "Listing",
        Award { .. } => "Award",
        CallUp { .. } => "Call-up",
        Meeting { .. } => "Meeting",
        PromiseMade { .. } => "Promise",
        PromiseKept { .. } => "Promise kept",
        PromiseBroken { .. } => "Promise broken",
        TransferRequested { .. } => "Transfer request",
        TransferRequestWithdrawn { .. } => "Request withdrawn",
        Fined { .. } => "Fine",
        Unrest { .. } => "Unrest",
        TalksOpened { .. } => "Talks",
        TalksCollapsed { .. } => "Talks ended",
        Published { .. } => "Press",
        AgentHired { .. } | AgentLeft { .. } | AgentPitch { .. } => "Agent",
        Life { .. } => "Life",
        JoinedStaff { .. } => "New job",
        CameOutOfRetirement { .. } => "Comeback",
        CoachNote { .. } => "Coach's note",
        StatusChanged { .. } => "Squad status",
        Captaincy { .. } => "Captaincy",
        Takeover { .. } => "Takeover",
        Administration { .. } => "Administration",
        PointsDeducted { .. } => "Points deducted",
        Austerity { .. } => "Austerity",
        OwnerInvestment { .. } => "Investment",
        ProjectStarted { .. } | ProjectCompleted { .. } => "Project",
        BroadcastDeal { .. } => "Broadcast deal",
        ManagerResigned { .. } => "Resignation",
        ManagerPoached { .. } => "Poached",
        TacticalChange { .. } => "Tactics",
        MatchTacticsChanged { .. } => "Tactical change",
        PersonalMatterHandled { .. } => "Personal matter",
        AdaptationEnded { .. } => "Settling in",
        AdaptationStruggling { .. } => "Struggling to settle",
        MediaGrudge { .. } => "Grudge",
        AttentionSurge { .. } => "Attention",
        PerformedThroughStrain { .. } => "Under strain",
        MemoryReturned { .. } => "A memory returns",
        StaffFollowed { .. } | StaffLeft { .. } => "Staff",
        DealCollapsed { .. } => "Deal collapsed",
        PreContractSigned { .. } => "Pre-contract",
        TrialStarted { .. } | TrialEnded { .. } => "Trial",
        LoanRecalled { .. } => "Loan recalled",
        OptionExercised { .. } => "Option",
        AddOnPaid { .. } | SellOnPaid { .. } => "Clause paid",
        AcademyJoined { .. } | AcademyReleased { .. } | ScholarshipOffered { .. } | AcademyTrialStarted { .. } => "Academy",
        AssessmentVindicated { .. } => "Judgement",
        SigningReviewed { .. } => "Signing review",
        PlanFailed { .. } => "Plan failed",
        ContractOption { .. } => "Contract option",
        JoinedLocalClub { .. } => "Local club",
        ExamsSat { .. } => "Exams",
        NationalSquad { .. } => "Squad named",
        InternationalDebut { .. } => "International debut",
        InternationalResult { .. } => "International result",
        TournamentWon { .. } => "Tournament",
        ChoseNation { .. } => "Allegiance",
        RetiredFromInternational { .. } => "International retirement",
        NationalManagerAppointed { .. } | NationalManagerLeft { .. } => "National manager",
        WithdrewFromSquad { .. } => "Withdrew",
        Diagnosed { .. } => "Diagnosis",
        InjurySetback { .. } => "Setback",
        RushedBack { .. } => "Rushed back",
        Ruling { .. } => "Decision",
        PathwayStep { .. } => "Pathway",
        Breakout { .. } => "Breakout",
        ChronicCondition { .. } => "Chronic condition",
        PlayerSettled { .. } => "Settled",
        DressingRoomSplit { .. } => "Dressing room",
        LeaderEmerged { .. } => "Leader",
        TookUnderWing { .. } => "Mentoring",
        CharacterChanged { .. } => "Character",
        Stagnated { .. } => "Stagnation",
        Milestone { .. } => "Milestone",
        RecordBroken { .. } => "Record",
        BecameLegend { .. } => "Legend",
        InductedHallOfFame { .. } => "Hall of fame",
        ManagerOfSeason { .. } => "Manager of the season",
        EnrolledCourse { .. } | Qualified { .. } => "Study",
        MovedHome { .. } => "Home",
        HiredHelper { .. } => "Help",
        GaveBack { .. } => "Giving back",
        Endorsed { .. } | EndorsementEnded { .. } => "Endorsement",
        SponsorClash { .. } => "Sponsor clash",
        ClubSponsor { .. } => "Sponsorship",
        NewCareer { .. } | CareerEnded { .. } => "Career",
        Investment { .. } => "Investment",
        BoardWarning { .. } => "Board warning",
        BoardQuery { .. } => "Board query",
        LeakSuspected { .. } => "Leak",
        AgentExploring { .. } => "Agent",
        Incident { .. } => "Incident",
        IncidentResponse { .. } => "Response",
        CaptainMediated { .. } => "Captain stepped in",
        InvestigationCleared { .. } => "Investigation",
        JournalistMoved { .. } | JournalistLeft { .. } | JournalistHired { .. } => "Press corps",
        SupporterAction { .. } => "Supporters",
        EnrolledUniversity { .. } => "University",
        RecruitWon { .. } => "Recruitment",
        Graduated { .. } => "Graduation",
        MinorTitle { .. } => "Title",
        Record { .. } => "Record",
        Voted { .. } => "Vote",
        HallInduction { .. } => "Hall of fame",
        Chronicle { .. } => "Chronicle",
        RefereeControversy { .. } => "Referee",
        AppealDecided { .. } => "Appeal",
        Charged { .. } => "Charge",
        SchoolFounded { .. } => "Tactical school",
        RuleChanged { .. } => "Rule change",
        RivalryKindled { .. } => "Rivalry",
        OpinionTurned { .. } => "Opinion",
    }
}

pub fn clubs_of(k: &E) -> Vec<ClubId> {
    k.clubs().into_iter().collect()
}

pub fn comp_of(k: &E) -> Option<CompId> {
    match *k {
        E::Debut { comp, .. } | E::FirstGoal { comp, .. } | E::Champion { comp, .. } | E::Promoted { comp, .. } | E::Relegated { comp, .. } | E::Award { comp, .. } => Some(comp),
        _ => None,
    }
}

/// May this viewer read this event?
pub fn visible(c: &Ctx, e: &Event) -> bool {
    let vis = match e.vis {
        Visibility::Public => true,
        Visibility::Club(cl) => c.observer() || c.same_club(cl),
        Visibility::Person(p) => c.observer() || Some(p) == c.me(),
        Visibility::Between(a, b) => c.observer() || Some(a) == c.me() || Some(b) == c.me(),
    };
    vis && !spoils(c, e)
}

/// Events that would give away a result the viewer has chosen not to see yet.
fn spoils(c: &Ctx, e: &Event) -> bool {
    if c.s.meta.concealed.is_empty() {
        return false;
    }
    if let E::Published { story } = e.kind {
        return c.w.media.stories.get(story).is_some_and(|s| c.story_spoils(s));
    }
    let team_of_event = match e.kind {
        E::Debut { team, .. } | E::FirstGoal { team, .. } => Some(team),
        E::Injured { player, .. } | E::Suspended { player, .. } => Some(c.w.players.hot[player].team).filter(|t| t.is_some()),
        _ => None,
    };
    let Some(team) = team_of_event else { return false };
    c.concealed_fixtures().iter().any(|f| f.date == e.date && f.involves(team))
}

fn pl(c: &Ctx, p: PlayerId) -> Part {
    Part::l(c.player_ref(p), c.player_name(p))
}

fn cl(c: &Ctx, x: ClubId) -> Part {
    if x.is_none() { Part::t("no club") } else { Part::l(Ref::club(x), c.club_name(x)) }
}

fn cp(c: &Ctx, x: CompId) -> Part {
    Part::l(Ref::comp(x), c.comp_name(x))
}

fn t(s: &str) -> Part {
    Part::t(s)
}

/// The main thing the event is about, for row navigation.
pub fn primary(c: &Ctx, k: &E) -> Option<Ref> {
    if let Some(p) = k.player() {
        return Some(c.player_ref(p));
    }
    match *k {
        E::Retired { person } => return Some(Ref::person(person)),
        E::ManagerSacked { staff, .. } | E::ManagerAppointed { staff, .. } => return Some(Ref::person(c.w.staff[staff].person)),
        E::Champion { team, .. } | E::Promoted { team, .. } | E::Relegated { team, .. } => return Some(c.team_ref(team)),
        _ => {}
    }
    if let Some(&p) = k.people().iter().find(|p| p.is_some()) {
        return Some(Ref::person(p));
    }
    k.clubs().iter().find(|x| x.is_some()).map(|&x| Ref::club(x))
}

/// The sentence for an event, as parts that link to what it names. Older kinds are built here with
/// links; everything else is the narration crate's line, so text always comes from recorded state.
pub fn describe(c: &Ctx, e: &Event) -> Vec<Part> {
    use E::*;
    let k = &e.kind;
    match *k {
        Transfer { player, from, to, fee } => {
            let mut v = vec![pl(c, player), t(" joined "), cl(c, to)];
            if from.is_some() {
                v.push(t(" from "));
                v.push(cl(c, from));
            }
            if fee > 0 {
                v.push(t(" for "));
                v.push(Part::money(fee));
            } else {
                v.push(t(if from.is_some() { " on a free transfer" } else { " as a free agent" }));
            }
            v
        }
        LoanMove { player, from, to, until } => {
            vec![pl(c, player), t(" went on loan to "), cl(c, to), t(" from "), cl(c, from), t(" until "), Part::date(until)]
        }
        LoanReturn { player, to } => vec![pl(c, player), t(" returned from loan to "), cl(c, to)],
        ContractSigned { player, club, wage, until, renewal } => {
            let mut v = vec![pl(c, player), t(if renewal { " signed a new contract with " } else { " signed with " }), cl(c, club), t(" until "), Part::date(until)];
            if c.sees_contract(player) {
                v.push(t(" on "));
                v.push(Part::money(wage));
                v.push(t(" a week"));
            }
            v
        }
        Released { player, club } => vec![pl(c, player), t(" left "), cl(c, club), t(" when the contract ended")],
        Retired { person } => vec![Part::l(Ref::person(person), c.person_name(person)), t(" retired from playing")],
        Injured { player, injury, days } => {
            let name = pw_sim::health::injury_name(c.w, injury);
            vec![pl(c, player), t(&crate::fmt::singulars(format!(" was injured ({}), out for about {} days", name.to_lowercase(), days)))]
        }
        Recovered { player } => vec![pl(c, player), t(" recovered from injury")],
        Suspended { player, matches } => vec![pl(c, player), t(&format!(" was suspended for {matches} match{}", if matches == 1 { "" } else { "es" }))],
        ManagerSacked { staff, club } => {
            let person = c.w.staff[staff].person;
            vec![cl(c, club), t(" dismissed manager "), Part::l(Ref::person(person), c.person_name(person))]
        }
        ManagerAppointed { staff, club } => {
            let person = c.w.staff[staff].person;
            vec![cl(c, club), t(" appointed "), Part::l(Ref::person(person), c.person_name(person)), t(" as manager")]
        }
        YouthIntake { club, count } => vec![cl(c, club), t(&format!(" took in {count} new youth players"))],
        Debut { player, team, comp } => vec![pl(c, player), t(" made a first appearance for "), Part::l(c.team_ref(team), c.team_short(team)), t(" in "), cp(c, comp)],
        FirstGoal { player, team, comp } => vec![pl(c, player), t(" scored a first goal for "), Part::l(c.team_ref(team), c.team_short(team)), t(" in "), cp(c, comp)],
        Champion { comp, team, season } => vec![Part::l(c.team_ref(team), c.team_name(team)), t(" won "), cp(c, comp), t(&format!(" ({})", c.season_label(comp, season)))],
        Promoted { comp, team } => vec![Part::l(c.team_ref(team), c.team_name(team)), t(" earned promotion from "), cp(c, comp)],
        Relegated { comp, team } => vec![Part::l(c.team_ref(team), c.team_name(team)), t(" were relegated from "), cp(c, comp)],
        Interest { player, club } => vec![cl(c, club), t(" showed interest in "), pl(c, player)],
        BidRejected { player, club, fee } => vec![cl(c, club), t(" had a bid of "), Part::money(fee), t(" for "), pl(c, player), t(" rejected")],
        BidAccepted { player, club, fee } => vec![cl(c, club), t(" had a bid of "), Part::money(fee), t(" for "), pl(c, player), t(" accepted")],
        TransferListed { player, club } => vec![cl(c, club), t(" listed "), pl(c, player), t(" for transfer")],
        Award { player, comp, award, season } => {
            vec![pl(c, player), t(&format!(" won {} in ", award.label())), cp(c, comp), t(&format!(" ({})", c.season_label(comp, season)))]
        }
        CallUp { player } => vec![pl(c, player), t(" was called up by the national team")],
        _ => {
            let viewer = c.me().unwrap_or(pw_core::PersonId::NONE);
            match pw_narrate::events::line(c.w, e, viewer) {
                Some(line) => linkify(&line, &entities(c, k)),
                None => vec![Part::t(label(k))],
            }
        }
    }
}

/// The people, players and clubs an event names, with the exact text the narration prints for them.
fn entities(c: &Ctx, k: &E) -> Vec<(String, Ref)> {
    let mut v: Vec<(String, Ref)> = Vec::new();
    for p in k.people() {
        v.push((c.person_name(p), Ref::person(p)));
    }
    if let Some(p) = k.player() {
        v.push((c.player_name(p), c.player_ref(p)));
    }
    for x in k.clubs() {
        if x.is_some() {
            v.push((c.club_name(x), Ref::club(x)));
        }
    }
    v
}

/// Split narrated text into plain runs and links, wherever a known name appears as a whole word.
pub fn linkify(text: &str, ents: &[(String, Ref)]) -> Vec<Part> {
    let mut out: Vec<Part> = Vec::new();
    let mut rest = text;
    loop {
        let mut best: Option<(usize, usize, Ref)> = None;
        for (name, r) in ents {
            if name.chars().count() < 3 {
                continue;
            }
            let mut from = 0;
            while let Some(i) = rest[from..].find(name.as_str()) {
                let at = from + i;
                let end = at + name.len();
                let before = rest[..at].chars().next_back().is_none_or(|ch| !ch.is_alphanumeric());
                let after = rest[end..].chars().next().is_none_or(|ch| !ch.is_alphanumeric());
                if before && after {
                    if best.is_none_or(|(b, l, _)| at < b || (at == b && name.len() > l)) {
                        best = Some((at, name.len(), *r));
                    }
                    break;
                }
                from = at + 1;
                if from >= rest.len() {
                    break;
                }
            }
        }
        match best {
            None => {
                if !rest.is_empty() {
                    out.push(Part::t(rest));
                }
                break;
            }
            Some((at, len, r)) => {
                if at > 0 {
                    out.push(Part::t(&rest[..at]));
                }
                out.push(Part::l(r, &rest[at..at + len]));
                rest = &rest[at + len..];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_whole_names_only() {
        let ents = vec![("Ann Lee".to_string(), Ref { k: "person", id: 1 }), ("Ann".to_string(), Ref { k: "person", id: 2 })];
        let parts = linkify("Ann Lee signed. Joanna met Ann.", &ents);
        let flat: Vec<(&str, Option<u32>)> = parts.iter().map(|p| (p.t.as_str(), p.r.map(|r| r.id))).collect();
        assert_eq!(flat, vec![("Ann Lee", Some(1)), (" signed. Joanna met ", None), ("Ann", Some(2)), (".", None)]);
    }

    #[test]
    fn plain_text_is_kept() {
        assert_eq!(linkify("Nothing to link.", &[]).len(), 1);
    }
}
