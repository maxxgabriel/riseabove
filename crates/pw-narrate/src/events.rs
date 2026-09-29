//! Rendering events and their causes. Every line names its source event.

use pw_core::PersonId;
use pw_world::event::{Cause, Event, EventKind, Fact, LifeEventKind};
use pw_world::{MemoryKind, World};

use crate::fmt::{club, money, nation, person, player};
use crate::pick;

/// One line describing an event, told from `viewer`'s side where it matters
/// ("you" for the viewer). `None` when the event has nothing to say to text.
pub fn line(w: &World, e: &Event, viewer: PersonId) -> Option<String> {
    raw_line(w, e, viewer).map(agree)
}

/// Verb agreement when the viewer is the subject: "You was injured" reads as "You were injured".
fn agree(s: String) -> String {
    if !s.contains("You ") {
        return s;
    }
    s.replace("You is ", "You are ").replace("You was ", "You were ").replace("You has ", "You have ")
}

fn raw_line(w: &World, e: &Event, viewer: PersonId) -> Option<String> {
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
            let a = award.label();
            if comp.is_some() { format!("{} won {a} in the {} ({season}).", pl(p), w.comps[comp].name) } else { format!("{} won {a} ({season}).", pl(p)) }
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
                pw_world::event::CoachNote::PoorTraining => "has been below standard in training",
                pw_world::event::CoachNote::ExcellentTraining => "has been outstanding in training",
                pw_world::event::CoachNote::Improving => "is improving",
                pw_world::event::CoachNote::Declining => "is slipping",
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
        TacticalChange { club: c, staff, formation } => {
            format!("{} switched {} to a {}.", w.staff_name(staff), club(w, c), w.data.formations.get(usize::from(formation)).map_or("new system", |f| f.name.as_str()))
        }
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
        NationalSquad { player: p, nation: n, level } => {
            format!("{} {} named in the {} {} squad.", pl(p), if w.players.cold[p].person == viewer { "were" } else { "was" }, nation(w, n), level.label())
        }
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
        Diagnosed { player: p, injury, estimate, treatment } => format!(
            "The medical team expect {} to miss around {} with a {} ({}).",
            pl(p),
            crate::fmt::duration_days(estimate),
            w.data.injuries.get(usize::from(injury).saturating_sub(1)).map_or("problem", |d| d.name.as_str()),
            treatment.label()
        ),
        InjurySetback { player: p, days } => format!("{} suffered a setback in rehabilitation: another {} out.", pl(p), crate::fmt::duration_days(days)),
        Ruling { ruling } => match w.ext.decisions.get(ruling) {
            Some(r) => match r.kind {
                pw_world::ruling::RulingKind::ReturnFromInjury => format!("The club cleared {} to play before the injury had fully healed.", pl(r.subject)),
                pw_world::ruling::RulingKind::SackManager => format!("{} decided to part with the manager.", club(w, r.club)),
                pw_world::ruling::RulingKind::BackManager => format!("{} decided to stand by the manager for now.", club(w, r.club)),
            },
            None => "A decision was made.".to_string(),
        },
        RushedBack { player: p } => format!("{} was passed fit ahead of schedule.", pl(p)),
        ChronicCondition { player: p } => format!("{} now has a condition that will need managing.", pl(p)),
        PlayerSettled { player: p, club: c } => format!("{} has settled in at {}.", pl(p), club(w, c)),
        DressingRoomSplit { club: c, leader } => format!("A group around {} at {} has lost faith in the manager.", pl(leader), club(w, c)),
        LeaderEmerged { player: p, club: c } => format!("{} has become one of the voices of the {} dressing room.", pl(p), club(w, c)),
        TookUnderWing { mentor, mentee } => format!("{} has taken {} under their wing.", me(mentor), me_lc(mentee)),
        CharacterChanged { person: x, up } => {
            if up {
                format!("People around {} have noticed a new maturity.", me(x))
            } else {
                format!("People around {} worry about their attitude lately.", me(x))
            }
        }
        Stagnated { player: p } => format!("Without football, {}'s development has stalled.", pl(p)),
        Milestone { player: p, kind, count, club: c } => match kind {
            pw_world::event::MilestoneKind::ClubApps => format!("{} made appearance number {count} for {}.", pl(p), club(w, c)),
            pw_world::event::MilestoneKind::CareerGoals => format!("{} scored career goal number {count}.", pl(p)),
            pw_world::event::MilestoneKind::SeniorApps => format!("{} reached {count} senior appearances.", pl(p)),
            pw_world::event::MilestoneKind::Caps => format!("{} won cap number {count}.", pl(p)),
        },
        RecordBroken { player: p, kind, club: c, value } => {
            use pw_world::event::RecordKind as R;
            let base = match kind {
                R::ClubTopScorer => format!("{} became {}'s all-time top scorer ({value} goals).", pl(p), club(w, c)),
                R::ClubMostApps => format!("{} now has more appearances for {} than anyone ({value}).", pl(p), club(w, c)),
                R::ClubRecordSigning => format!("{} became {}'s record signing ({}).", pl(p), club(w, c), money(value)),
                R::ClubRecordSale => format!("{} became {}'s record sale ({}).", pl(p), club(w, c), money(value)),
                R::ClubBiggestWin => format!("{} recorded their biggest ever win (by {value}).", club(w, c)),
                R::LeagueGoalsInSeason => format!("{} set a new league record of {value} goals in a season.", pl(p)),
                R::NationMostCaps => format!("{} became their country's most-capped player ({value}).", pl(p)),
                R::NationTopScorer => format!("{} became their country's all-time top scorer ({value}).", pl(p)),
                R::WorldRecordFee => format!("{} became the most expensive player in history ({}).", pl(p), money(value)),
            };
            format!("{base}{}", crate::history::pro_context(w, e.date, kind, c))
        }
        BecameLegend { person: x, club: c } => format!("{} {} now spoken of as a legend at {}.", me(x), if x == viewer { "are" } else { "is" }, club(w, c)),
        InductedHallOfFame { person: x } => format!("{} {} inducted into the Hall of Fame.", me(x), if x == viewer { "were" } else { "was" }),
        EnrolledCourse { person: x, course } => format!("{} enrolled on a {}.", me(x), course.label()),
        Qualified { person: x, course } => format!("{} completed the {}.", me(x), course.label()),
        MovedHome { person: x, bought } => format!("{} {} a new home.", me(x), if bought { "bought" } else { "moved into" }),
        HiredHelper { person: x, helper } => format!("{} took on a {}.", me(x), helper.label()),
        GaveBack { person: x, foundation } => {
            if foundation {
                format!("{} launched a charitable foundation.", me(x))
            } else {
                format!("{} committed time to community work.", me(x))
            }
        }
        Endorsed { person: x, brand, fee_year } => {
            let b = &w.commerce.brands[brand as usize];
            format!("{} signed an endorsement with {} ({}, {} a year).", me(x), b.name, b.sector.label(), money(fee_year))
        }
        EndorsementEnded { person: x, brand, why } => {
            let b = &w.commerce.brands[brand as usize];
            let how = match why {
                pw_world::commerce::DealEnd::Expired => "came to an end",
                pw_world::commerce::DealEnd::Scandal => "was terminated after damaging headlines",
                pw_world::commerce::DealEnd::Conflict => "was dropped because of a clash with the club's partners",
                pw_world::commerce::DealEnd::Retired => "ended with retirement",
                pw_world::commerce::DealEnd::Faded => "was not renewed",
            };
            format!("{}'s deal with {} {how}.", me(x), b.name)
        }
        SponsorClash { person: x, brand, club: c } => format!("{}'s deal with {} clashes with {}'s own partners.", me(x), w.commerce.brands[brand as usize].name, club(w, c)),
        ClubSponsor { club: c, brand, slot, fee_year } => {
            format!("{} agreed a {} sponsorship with {} worth {} a year.", club(w, c), slot.label(), w.commerce.brands[brand as usize].name, money(fee_year))
        }
        NewCareer { person: x, path } => format!("{} began a career in {}.", me(x), path.label()),
        CareerEnded { person: x, path } => format!("{} stepped away from {}.", me(x), path.label()),
        Investment { person: x, gain } => {
            if gain >= 0 {
                format!("{}'s investments returned {}.", me(x), money(gain))
            } else {
                format!("{} lost {} on investments.", me(x), money(-gain))
            }
        }
        BoardWarning { club: c, manager, warnings } => format!("The {} board privately warned {} ({} warning{}).", club(w, c), w.staff_name(manager), warnings, if warnings == 1 { "" } else { "s" }),
        BoardQuery { club: c, manager, .. } => format!("The {} board asked {} to explain what they had heard.", club(w, c), me_lc(manager)),
        LeakSuspected { by, suspect, .. } => format!("{} suspects {} of talking to the press.", me(by), me_lc(suspect)),
        AgentExploring { agent, player: p } => format!("{} began quietly sounding out clubs about {}.", me(agent), pl(p)),
        Incident { incident, .. } => crate::incidents::sentence(w, incident, viewer),
        IncidentResponse { incident, by, response } => crate::incidents::response(w, incident, by, response, viewer),
        CaptainMediated { captain, a, b } => format!("{} stepped in to settle things between {} and {}.", me(captain), me_lc(a), me_lc(b)),
        InvestigationCleared { club: c } => format!("{} were cleared by the investigation.", club(w, c)),
        JournalistMoved { person: x, from, to } => format!("{} left {} for {}.", me(x), w.media.outlets[from].name, w.media.outlets[to].name),
        JournalistLeft { person: x, outlet } => format!("{} is no longer writing for {}.", me(x), w.media.outlets[outlet].name),
        JournalistHired { person: x, outlet } => format!("{} joined {}.", me(x), w.media.outlets[outlet].name),
        EnrolledUniversity { person: x, institution } => format!("{} enrolled at {}.", me(x), crate::history::institution(w, institution)),
        Graduated { person: x, institution, early } => {
            if early {
                format!("{} left {} to turn professional.", me(x), crate::history::institution(w, institution))
            } else {
                format!("{} graduated from {}.", me(x), crate::history::institution(w, institution))
            }
        }
        Record { broken, .. } => w.records.broken.get(broken as usize).map_or_else(String::new, |b| crate::history::broken(w, b)),
        Voted { vote, person: x } => w.acclaim.votes.get(vote as usize).map_or_else(String::new, |v| crate::history::vote(w, v, x)),
        HallInduction { hall, person: x } => w.acclaim.halls.get(hall as usize).map_or_else(String::new, |h| {
            let share = h.members.iter().find(|m| m.person == x).map_or(String::new(), |m| format!(" with {}% of the committee's votes", m.share));
            format!("{} {} inducted into {}{share}.", me(x), if x == viewer { "were" } else { "was" }, crate::history::hall_name(w, h.scope))
        }),
        Chronicle { entry } => w.acclaim.chronicle.get(entry as usize).map_or_else(String::new, |e| crate::history::chronicle(w, e)),
        RefereeControversy { controversy } => w.officials.controversies.get(controversy as usize).map_or_else(String::new, |c| crate::officiating::controversy(w, c)),
        AppealDecided { appeal, .. } => w.officials.appeals.get(appeal as usize).map_or_else(String::new, |a| crate::officiating::appeal(w, a)),
        Charged { charge, .. } => w.officials.charges.get(charge as usize).map_or_else(String::new, |c| crate::officiating::charge(w, c)),
        SchoolFounded { school, .. } => w.evolution.schools.get(school as usize).map_or_else(String::new, |s| crate::history::school_founded(w, s)),
        RuleChanged { change } => w.evolution.changes.get(change as usize).map_or_else(String::new, |c| crate::history::rule_change(w, c)),
        MinorTitle { history } => w.minor.history.get(history as usize).map_or_else(String::new, |s| crate::history::season_line(w, s)),
        SupporterAction { club: c, group, action } => crate::social::group_action(w, c, group, action),
        ManagerOfSeason { staff, comp, season } => format!("{} was named Manager of the Season in the {} ({season}).", w.staff_name(staff), w.comps[comp].name),
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
        Fact::Said { person: x } => format!("{} said so publicly", person(w, x)),
        Fact::Played { .. } => "the match itself".into(),
        Fact::Reading { player: p } => format!("how {} performances are being read", who(p)),
        Fact::Ranking => "a published ranking".into(),
        Fact::Anniversary { year } => format!("the anniversary of {year}"),
        Fact::Heard { info, from } => format!("{} heard it ({})", person(w, from), crate::grapevine::what(w, info)),
        Fact::Pressure { pressure, level } => format!("{} ({})", pressure.label(), crate::incidents::strength(level)),
        Fact::Disposition { person: x, reason, level } => format!("{}'s {} ({})", person(w, x), reason.label(), crate::incidents::strength(level)),
        Fact::PoorRun { club: c, defeats, games } => format!("{} had lost {defeats} of their last {games}", club(w, c)),
        Fact::Viral { reposts, .. } => format!("a supporter's post was shared {reposts} times"),
        Fact::Newsworthy { importance, relevance, controversy } => format!("editors judged it newsworthy (importance {importance}, relevance {relevance}, controversy {controversy})"),
    }
}

pub fn memory(kind: MemoryKind) -> &'static str {
    kind.text()
}
