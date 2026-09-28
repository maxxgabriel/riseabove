//! Rendering events and their causes. Every line names its source event.

use pw_core::PersonId;
use pw_world::event::{AwardKind, Cause, CoachNote, Event, EventKind, Fact, LifeEventKind};
use pw_world::{MemoryKind, World};

use crate::fmt::{club, money, nation, person, player};
use crate::pick;

/// One line describing an event, told from `viewer`'s side where it matters
/// ("you" for the viewer). `None` when the event has nothing to say to text.
pub fn line(w: &World, e: &Event, viewer: PersonId) -> Option<String> {
    let me = |p: PersonId| if p == viewer { "You".to_string() } else { person(w, p) };
    let me_lc = |p: PersonId| if p == viewer { "you".to_string() } else { person(w, p) };
    let pl = |p: pw_core::PlayerId| {
        if p.is_some() && w.players.cold[p].person == viewer { "You".to_string() } else { player(w, p) }
    };
    let key = u64::from(e.id.0);
    use EventKind::*;
    Some(match e.kind {
        Transfer { player: p, from, to, fee } => {
            let how = if fee > 0 { format!("for {}", money(fee)) } else { "on a free".into() };
            let origin = if from.is_some() { format!(" from {}", club(w, from)) } else { String::new() };
            format!("{} {} {}{origin} {how}.", pl(p), pick(key, &["joined", "signed for", "completed a move to"]), club(w, to))
        }
        LoanMove { player: p, from, to, until } => format!("{} joined {} on loan from {} until {until}.", pl(p), club(w, to), club(w, from)),
        LoanReturn { player: p, to } => format!("{} returned to {} after a loan spell.", pl(p), club(w, to)),
        ContractSigned { player: p, club: c, wage, until, renewal } => {
            if renewal {
                format!("{} signed a new contract with {} until {until} ({}).", pl(p), club(w, c), crate::fmt::wage(wage))
            } else {
                format!("{} signed a contract with {} until {until}.", pl(p), club(w, c))
            }
        }
        Released { player: p, club: c } => format!("{} left {} as a free agent.", pl(p), club(w, c)),
        Retired { person: x } => format!("{} retired from playing.", me(x)),
        Injured { player: p, injury, days } => {
            let name = if injury > 0 { w.data.injuries.get(usize::from(injury - 1)).map_or("an injury".to_string(), |d| d.name.to_lowercase()) } else { "an injury".into() };
            format!("{} {} ({name}), expected out for about {} days.", pl(p), pick(key, &["picked up an injury", "was injured", "suffered an injury"]), days)
        }
        Recovered { player: p } => format!("{} is back in full training.", pl(p)),
        Suspended { player: p, matches } => format!("{} will serve a {matches}-match suspension.", pl(p)),
        ManagerSacked { staff, club: c } => format!("{} sacked manager {}.", club(w, c), w.staff_name(staff)),
        ManagerAppointed { staff, club: c } => format!("{} appointed {} as manager.", club(w, c), w.staff_name(staff)),
        YouthIntake { club: c, count } => format!("{} welcomed {count} new players from their youth intake.", club(w, c)),
        Debut { player: p, team, .. } => format!("{} made a senior debut for {}.", pl(p), w.team_name(team)),
        FirstGoal { player: p, team, .. } => format!("{} scored a first senior goal for {}.", pl(p), w.team_name(team)),
        Champion { comp, team, season } => format!("{} won the {} ({season}).", w.team_name(team), w.comps[comp].name),
        Promoted { comp, team } => format!("{} were promoted from the {}.", w.team_name(team), w.comps[comp].name),
        Relegated { comp, team } => format!("{} were relegated from the {}.", w.team_name(team), w.comps[comp].name),
        Interest { player: p, club: c } => format!("{} are interested in {}.", club(w, c), player(w, p)),
        BidRejected { player: p, club: c, fee } => format!("A {} bid from {} for {} was rejected.", money(fee), club(w, c), player(w, p)),
        BidAccepted { player: p, club: c, fee } => format!("A {} bid from {} for {} was accepted.", money(fee), club(w, c), player(w, p)),
        TransferListed { player: p, club: c } => format!("{} placed {} on the transfer list.", club(w, c), player(w, p)),
        Award { player: p, comp, award, season } => {
            let a = match award {
                AwardKind::PlayerOfSeason => "Player of the Season",
                AwardKind::YoungPlayerOfSeason => "Young Player of the Season",
                AwardKind::TopScorer => "the Golden Boot",
                AwardKind::TeamOfSeason => "a place in the Team of the Season",
                AwardKind::PlayerOfMonth => "Player of the Month",
            };
            format!("{} won {a} in the {} ({season}).", pl(p), w.comps[comp].name)
        }
        CallUp { player: p } => format!("{} received an international call-up.", pl(p)),
        Meeting { meeting, from, with } => {
            let m = &w.meetings.list[meeting];
            format!("{} met {} about {}.", me(from), me_lc(with), m.topic.label())
        }
        PromiseMade { promise, from, to } => {
            let what = w.social.promise(promise).map_or("something".to_string(), |p| p.kind.text());
            format!("{} promised {} {what}.", me(from), me_lc(to))
        }
        PromiseKept { promise, from, to } => {
            let what = w.social.promise(promise).map_or("a promise".to_string(), |p| p.kind.text());
            format!("{} kept a promise to {}: {what}.", me(from), me_lc(to))
        }
        PromiseBroken { promise, from, to } => {
            let what = w.social.promise(promise).map_or("a promise".to_string(), |p| p.kind.text());
            format!("{} broke a promise to {}: {what}.", me(from), me_lc(to))
        }
        TransferRequested { player: p, club: c } => format!("{} handed in a transfer request at {}.", pl(p), club(w, c)),
        TransferRequestWithdrawn { player: p, club: c } => format!("{} withdrew a transfer request at {}.", pl(p), club(w, c)),
        Fined { player: p, club: c, amount } => format!("{} fined {} {}.", club(w, c), player(w, p), money(amount)),
        Unrest { club: c, player: p } => format!("Unrest in the {} dressing room, centred on {}.", club(w, c), player(w, p)),
        TalksOpened { talk, player: p, club: c } => format!("{} opened talks with {} over {}.", club(w, c), player(w, p), w.talks[talk].kind.label()),
        TalksCollapsed { talk, player: p, club: c } => format!("Talks between {} and {} over {} broke down.", club(w, c), player(w, p), w.talks[talk].kind.label()),
        Published { story } => crate::press::headline(w, &w.media.stories[story]),
        AgentHired { player: p, agent } => format!("{} is now represented by {}.", pl(p), person(w, w.agents.list[agent].person)),
        AgentLeft { player: p, agent } => format!("{} parted ways with agent {}.", pl(p), person(w, w.agents.list[agent].person)),
        AgentPitch { player: p, agent, club: c } => format!("Agent {} offered {} to {}.", person(w, w.agents.list[agent].person), player(w, p), club(w, c)),
        Life { person: x, kind } => life(w, x, kind, viewer, key),
        JoinedStaff { person: x, staff, club: c } => format!("{} joined {} as {}.", me(x), club(w, c), w.staff[staff].role.label().to_lowercase()),
        CameOutOfRetirement { person: x } => format!("{} came out of retirement.", me(x)),
        CoachNote { player: p, by, note } => {
            let n = match note {
                CoachNote::PoorTraining => "has been below standard in training",
                CoachNote::ExcellentTraining => "has been outstanding in training",
                CoachNote::Improving => "is improving",
                CoachNote::Declining => "is slipping",
            };
            format!("{} noted that {} {n}.", person(w, by), player(w, p))
        }
        StatusChanged { player: p, club: c, from, to } => format!("{} now see {} as {} (was {}).", club(w, c), player(w, p), to.label(), from.label()),
        Captaincy { player: p, team } => format!("{} was named captain of {}.", pl(p), w.team_name(team)),
        Takeover { club: c, owner, .. } => format!("{} have been taken over by {}.", club(w, c), person(w, owner)),
        Administration { club: c } => format!("{} have entered administration.", club(w, c)),
        PointsDeducted { club: c, points } => format!("{} were deducted {points} points.", club(w, c)),
        Austerity { club: c } => format!("{} cut budgets and made high earners available.", club(w, c)),
        OwnerInvestment { club: c, amount } => format!("The owner of {} invested {}.", club(w, c), money(amount)),
        ProjectStarted { club: c, kind } => format!("{} began work on a new {}.", club(w, c), kind.label()),
        ProjectCompleted { club: c, kind } => format!("{} opened their new {}.", club(w, c), kind.label()),
        BroadcastDeal { nation: n, pool } => format!("{}'s top flight signed a {} broadcast deal.", nation(w, n), money(pool)),
        ManagerResigned { staff, club: c } => format!("{} resigned as manager of {}.", w.staff_name(staff), club(w, c)),
        ManagerPoached { staff, from, to, compensation } => format!("{} left {} to take charge of {} ({} compensation).", w.staff_name(staff), club(w, from), club(w, to), money(compensation)),
        TacticalChange { club: c, staff, formation } => format!("{} switched {} to a {}.", w.staff_name(staff), club(w, c), w.data.formations.get(usize::from(formation)).map_or("new system", |f| f.name.as_str())),
        StaffFollowed { staff, manager, club: c } => format!("{} followed {} to {}.", w.staff_name(staff), w.staff_name(manager), club(w, c)),
        StaffLeft { staff, club: c } => format!("{} left the {} backroom staff.", w.staff_name(staff), club(w, c)),
        DealCollapsed { player: p, buyer, seller, reason } => format!("{}'s move from {} to {} collapsed: {}.", pl(p), club(w, seller), club(w, buyer), reason.label()),
        PreContractSigned { player: p, club: c } => format!("{} agreed a pre-contract to join {} when their deal expires.", pl(p), club(w, c)),
        TrialStarted { player: p, club: c } => format!("{} began a trial at {}.", pl(p), club(w, c)),
        TrialEnded { player: p, club: c, offered } => format!("{}'s trial at {} ended{}.", pl(p), club(w, c), if offered { " with a contract offer" } else { " without an offer" }),
        LoanRecalled { player: p, club: c } => format!("{} recalled {} from loan.", club(w, c), player(w, p)),
        OptionExercised { player: p, club: c, fee } => format!("{} made {}'s loan permanent for {}.", club(w, c), player(w, p), money(fee)),
        AddOnPaid { player: p, from, to, amount } => format!("{} paid {} {} in add-ons for {}.", club(w, from), club(w, to), money(amount), player(w, p)),
        SellOnPaid { player: p, to, amount } => format!("{} received a {} sell-on payment for {}.", club(w, to), money(amount), player(w, p)),
        AcademyJoined { player: p, club: c } => format!("{} joined the {} academy.", pl(p), club(w, c)),
        AcademyReleased { player: p, club: c } => format!("{} was released by the {} academy.", pl(p), club(w, c)),
        ScholarshipOffered { player: p, club: c } => format!("{} was offered a scholarship by {}.", pl(p), club(w, c)),
        JoinedLocalClub { player: p, local } => format!("{} signed up with {}.", pl(p), w.youth.local[local].name),
        AcademyTrialStarted { player: p, club: c } => format!("{} began a trial with the {} academy.", pl(p), club(w, c)),
        ExamsSat { person: x, passed } => format!("{} {} school exams.", me(x), if passed { "passed" } else { "struggled in" }),
        NationalSquad { player: p, nation: n, level } => format!("{} {} named in the {} {} squad.", pl(p), if w.players.cold[p].person == viewer { "were" } else { "was" }, nation(w, n), level.label()),
        InternationalDebut { player: p, nation: n, level } => format!("{} made a {} debut for {}.", pl(p), level.label(), nation(w, n)),
        InternationalResult { index } => {
            let m = &w.intl.matches[index as usize];
            format!("{} {}-{} {} ({}, {}).", nation(w, m.home), m.home_goals, m.away_goals, nation(w, m.away), m.level.label(), if m.kind.competitive() { "competitive" } else { "friendly" })
        }
        TournamentWon { nation: n, tournament } => {
            let t = &w.intl.tournaments[tournament as usize];
            format!("{} won the {} {}.", nation(w, n), t.kind.label(), t.year)
        }
        ChoseNation { player: p, nation: n } => format!("{} committed their international future to {}.", pl(p), nation(w, n)),
        RetiredFromInternational { player: p, nation: n } => format!("{} retired from international football with {}.", pl(p), nation(w, n)),
        NationalManagerAppointed { staff, nation: n, level } => format!("{} appointed {} to lead their {} side.", nation(w, n), w.staff_name(staff), level.label()),
        NationalManagerLeft { staff, nation: n, level, sacked } => {
            format!("{} {} the {} {} job.", w.staff_name(staff), if sacked { "was dismissed from" } else { "left" }, nation(w, n), level.label())
        }
        WithdrewFromSquad { player: p, nation: n } => format!("{} withdrew from the {} squad.", pl(p), nation(w, n)),
    })
}

fn life(w: &World, who: PersonId, kind: LifeEventKind, viewer: PersonId, key: u64) -> String {
    let you = who == viewer;
    let subj = if you { "You".to_string() } else { person(w, who) };
    let poss = if you { "your".to_string() } else { format!("{}'s", person(w, who)) };
    match kind {
        LifeEventKind::StartedDating { partner } => format!("{subj} started seeing {}.", person(w, partner)),
        LifeEventKind::MovedIn { partner } => format!("{subj} and {} moved in together.", person(w, partner)),
        LifeEventKind::Married { partner } => format!("{subj} and {} got married.", person(w, partner)),
        LifeEventKind::Separated { partner } => format!("{subj} and {} separated.", person(w, partner)),
        LifeEventKind::ChildBorn => format!("{} family welcomed a new baby.", capital(&poss)),
        LifeEventKind::ParentUnwell => format!("{} parent has been taken ill.", capital(&poss)),
        LifeEventKind::ParentRecovered => format!("{} parent is recovering well.", capital(&poss)),
        LifeEventKind::Bereavement => format!("{} family suffered a bereavement.", capital(&poss)),
        LifeEventKind::Relocated { nation: n } => format!("{subj} moved to {}.", nation(w, n)),
        LifeEventKind::PartnerJoinedMove { partner } => format!("{} {}.", person(w, partner), pick(key, &["came along for the move", "is making the move too", "packed up and came along"])),
        LifeEventKind::PartnerStayedBehind { partner } => format!("{} decided to stay behind rather than move.", person(w, partner)),
        LifeEventKind::FinancialTrouble => format!("{subj} {} money worries.", if you { "have" } else { "has" }),
        LifeEventKind::Graduated => format!("{subj} completed a qualification."),
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Why something happened, from its recorded causes.
pub fn cause(w: &World, c: &Cause, viewer: PersonId) -> String {
    match c {
        Cause::Event(id) => w.events.get(*id).and_then(|e| line(w, e, viewer)).map_or_else(|| "an earlier event".into(), |l| format!("after: {l}")),
        Cause::Fact(f) => fact(w, f, viewer),
    }
}

pub fn fact(w: &World, f: &Fact, viewer: PersonId) -> String {
    let who = |p: pw_core::PlayerId| if p.is_some() && w.players.cold[p].person == viewer { "your".to_string() } else { format!("{}'s", player(w, p)) };
    match *f {
        Fact::TrainingSlump { player: p, weeks } => format!("{} training has been below par for {weeks} weeks", who(p)),
        Fact::TrainingSurge { player: p, weeks } => format!("{} training has been excellent for {weeks} weeks", who(p)),
        Fact::FormSlump { player: p } => format!("{} recent form has been poor", who(p)),
        Fact::FormSurge { player: p } => format!("{} recent form has been superb", who(p)),
        Fact::MinutesShortfall { player: p, share_pct, expected_pct } => format!("{} share of minutes is {share_pct}% against an expected {expected_pct}%", who(p)),
        Fact::Tracking { club: c, player: p, .. } => format!("{} have been watching {}", club(w, c), player(w, p)),
        Fact::SquadNeed { club: c } => format!("{} need cover in that position", club(w, c)),
        Fact::BoardPressure { club: c, warnings } => format!("the {} board has issued {warnings} warning(s)", club(w, c)),
        Fact::Congestion { team, matches } => format!("{} face {matches} matches in a short spell", w.team_name(team)),
        Fact::Memory { from, about, kind } => format!("{} remembers that {} {}", person(w, from), person(w, about), memory(kind)),
        Fact::PromiseDue { promise } => format!("a promise ({}) has come due", w.social.promise(promise).map_or("unknown".into(), |p| p.kind.text())),
        Fact::WageGap { player: p, pct_of_peers } => format!("{} wage is {pct_of_peers}% of comparable teammates'", who(p)),
        Fact::Household { person: x } => format!("{}'s family circumstances", person(w, x)),
        Fact::Injury { player: p, days } => format!("{} injury ({days} days)", who(p)),
        Fact::Unsettled { person: x, nation: n } => format!("{} has not settled in {}", person(w, x), nation(w, n)),
        Fact::LowTrust { from, about, trust } => format!("{} trusts {} little ({trust}/100)", person(w, from), person(w, about)),
        Fact::ContractRunningDown { player: p, days } => format!("{} contract has {days} days left", who(p)),
        Fact::PublicCriticism { story } => format!("criticism in the press: {}", crate::press::headline(w, &w.media.stories[story])),
        Fact::Rule { reason } => reason.text(),
    }
}

pub fn memory(kind: MemoryKind) -> &'static str {
    kind.text()
}
