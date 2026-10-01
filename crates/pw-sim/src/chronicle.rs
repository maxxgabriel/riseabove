//! The career chronicle (`pw_world::chronicle`): written as things happen, for the people a human has inhabited.
//!
//! `begin` opens a chronicle with what the world already kept of the person (how they came to exist, the pathway steps and their
//! reasons, the events still in the log); `daily` runs at the end of each simulated day and adds the day's events, matches and
//! coverage, and what people from the person's past did next. Ties to people (teammates, classmates, coaches, scouts, the managers
//! who let them go) are kept up to date weekly.
//!
//! Decision code never reads this: it is a record of what the person lived and learned, for them and the views.

use pw_core::{ClubId, Date, EventId, PersonId, PlayerId};
use pw_world::FxHashMap;
use pw_world::World;
use pw_world::chronicle::{Big, Join, Layer, Life, Line, Then, Tie, TieKind};
use pw_world::ecosystem::StageKind;
use pw_world::event::{Event, EventKind as E};
use pw_world::media::{OutletKind, StoryKind};
use pw_world::pathway::Why;
use pw_world::recog::Org;
use pw_world::staff::StaffRole;

/// Shortest shared spell (days) before a teammate's later moves are worth telling.
const TEAMMATE_DAYS: i32 = 120;

/// Start chronicling `who` (idempotent): what the world already kept of them, and the people around them now.
pub fn begin(w: &mut World, who: PersonId) {
    if w.people.get(who).is_none() || w.ext.chronicle.lives.contains_key(&who) {
        return;
    }
    let today = w.date;
    let p = w.people[who].player;
    let mut life = Life { since: today, ..Life::default() };
    if p.is_some() {
        backfill(w, &mut life, p);
    }
    for e in w.events.all() {
        on_event(w, &mut life, who, p, e, today);
    }
    life.entries.sort_by_key(|e| e.date);
    life.cursor = w.events.last_id();
    if p.is_some() {
        seasons(w, &mut life, p);
        refresh_ties(w, &mut life, p, today);
    }
    life.entries.sort_by_key(|e| e.date);
    w.ext.chronicle.lives.insert(who, life);
}

/// Each finished calendar year of senior football, once.
fn seasons(w: &World, life: &mut Life, p: PlayerId) {
    let year = w.date.year();
    let Some(lines) = w.perf.seasons.get(&p) else { return };
    for l in lines.iter().filter(|l| l.year < year && l.apps + l.benched + l.omitted > 0) {
        if life.seasons.contains(&(l.year, l.club)) {
            continue;
        }
        life.seasons.push((l.year, l.club));
        let rating10 = if l.apps == 0 { 0 } else { (l.rating_sum / u32::from(l.apps)) as u16 };
        let end = Date::from_ymd(l.year, 12, 31);
        life.push(
            end,
            Line::Season { year: l.year, club: l.club, apps: l.apps, starts: l.starts, goals: l.goals, assists: l.assists, rating10, benched: l.benched, omitted: l.omitted },
            EventId::NONE,
        );
    }
}

/// Where the person came from and the pathway steps whose reasons were kept.
fn backfill(w: &World, life: &mut Life, p: PlayerId) {
    let eco = &w.ext.ecosystem;
    if let Some(c) = w.ext.pathway.created.get(&p) {
        life.push(c.date, Line::Began { region: c.region, institution: c.institution, finder: c.first_finder }, EventId::NONE);
        add_tie(life, c.first_finder, TieKind::Finder, c.date, c.date);
    } else if let Some(s) = eco.story.get(&p) {
        life.push(s.found_on, Line::Began { region: s.home, institution: None, finder: s.found_by }, EventId::NONE);
        add_tie(life, s.found_by, TieKind::Finder, s.found_on, s.found_on);
    }
    for r in w.ext.pathway.of(p) {
        // Joining clubs and universities is told by the events of the move; the pathway adds what only it knows.
        if matches!(r.why, Why::ChosenStart | Why::Emerged | Why::Signed { .. } | Why::Transfer { .. }) {
            continue;
        }
        let line = Line::Step { stage: r.kind, why: r.why };
        if !life.has(&line) {
            life.push(r.date, line, EventId::NONE);
        }
    }
    if let Some(s) = eco.scholarship.get(&p) {
        let line = Line::Scholarship { inst: s.inst, tier: s.tier, contested: false };
        if !life.entries.iter().any(|e| matches!(e.line, Line::Scholarship { inst, .. } | Line::Step { why: Why::UniversityScholarship { inst, .. }, .. } if inst == s.inst)) {
            life.push(s.from, line, EventId::NONE);
        }
    }
}

/// The end of a simulated day: events since the last pass, today's matches, and on Mondays the people around each person.
pub fn daily(w: &mut World) {
    if w.ext.chronicle.lives.is_empty() {
        return;
    }
    let today = w.date;
    let mut who: Vec<PersonId> = w.ext.chronicle.lives.keys().copied().collect();
    who.sort_unstable();
    for id in who {
        let Some(mut life) = w.ext.chronicle.lives.remove(&id) else { continue };
        let p = w.people[id].player;
        let fresh: Vec<Event> = w.events.after(life.cursor).to_vec();
        for e in &fresh {
            on_event(w, &mut life, id, p, e, today);
        }
        life.cursor = w.events.last_id();
        if p.is_some() {
            seasons(w, &mut life, p);
            if let Some(apps) = w.perf.recent.get(&p) {
                for a in apps.iter().filter(|a| a.date == today) {
                    on_app(w, &mut life, p, a.club, a.comp, a.goals, a.assists, today);
                }
            }
            if today.weekday() == pw_core::Weekday::Mon {
                refresh_ties(w, &mut life, p, today);
            }
        }
        w.ext.chronicle.lives.insert(id, life);
    }
}

fn add_tie(life: &mut Life, person: PersonId, kind: TieKind, from: Date, to: Date) -> usize {
    match life.tie_of(person, kind) {
        Some(i) => {
            life.ties[i].to = life.ties[i].to.max(to);
            i
        }
        None => {
            life.ties.push(Tie { person, kind, from, to });
            life.ties.len() - 1
        }
    }
}

/// Teammates, the manager and classmates of today.
fn refresh_ties(w: &World, life: &mut Life, p: PlayerId, today: Date) {
    let me = w.players.cold[p].person;
    let h = &w.players.hot[p];
    if h.club.is_some() {
        if h.team.is_some() {
            for &q in &w.teams[h.team].squad {
                if q != p {
                    add_tie(life, w.players.cold[q].person, TieKind::Teammate { club: h.club }, today, today);
                }
            }
        }
        let m = w.clubs[h.club].manager;
        if m.is_some() && w.staff[m].person != me {
            add_tie(life, w.staff[m].person, TieKind::Coach { club: h.club }, today, today);
        }
    }
    if let Some(&inst) = w.minor.member_of.get(&p) {
        for &q in &w.minor.institutions[inst as usize].members {
            if q != p {
                add_tie(life, w.players.cold[q].person, TieKind::Classmate { inst }, today, today);
            }
        }
    }
}

/// What the person learns when an organisation says it has been watching them: who first took them seriously there, and when.
fn told(w: &World, life: &mut Life, p: PlayerId, org: Org, today: Date) {
    if life.told.contains(&org) {
        return;
    }
    let Some(a) = w.ext.recog.known(org, p) else { return };
    life.told.push(org);
    life.entries.push(pw_world::chronicle::Entry { date: a.first, line: Line::Noticed { org, by: a.first_by, how: a.how }, event: EventId::NONE, learned: (a.first < today).then_some(today) });
    if a.first_by.is_some() && a.first_by != w.players.cold[p].person {
        add_tie(life, a.first_by, TieKind::Scout { org }, a.first, a.first);
    }
}

fn club_of_team(w: &World, t: pw_core::TeamId) -> ClubId {
    w.teams.get(t).map_or(ClubId::NONE, |t| t.club)
}

/// The fixture of `club` in `comp` on `date`, if its facts are still kept.
fn uid_on(w: &World, date: Date, club: ClubId, comp: pw_core::CompId) -> u64 {
    w.recent_matches.on(date).find(|m| m.involves(club) && m.comp == comp).map_or(u64::MAX, |m| m.uid)
}

/// How far an outlet reaches from the person's own country.
pub fn layer_of(w: &World, outlet: pw_core::OutletId, home: pw_core::NationId) -> Layer {
    let Some(o) = w.media.outlets.get(outlet) else { return Layer::Local };
    if o.nation != home && home.is_some() {
        Layer::Abroad
    } else if matches!(o.kind, OutletKind::Local | OutletKind::FanChannel) {
        Layer::Local
    } else {
        Layer::National
    }
}

fn on_event(w: &World, life: &mut Life, who: PersonId, p: PlayerId, e: &Event, today: Date) {
    let d = e.date;
    let id = e.id;
    let mine = |q: PlayerId| p.is_some() && q == p;
    let push = |life: &mut Life, line: Line| {
        if !life.entries.iter().any(|x| x.event == id && x.line == line) {
            life.push(d, line, id);
        }
    };
    match e.kind {
        E::Transfer { player, to, .. } if mine(player) => push(life, Line::Joined { club: to, how: Join::Transfer }),
        E::LoanMove { player, to, .. } if mine(player) => push(life, Line::Joined { club: to, how: Join::Loan }),
        E::LoanReturn { player, to } if mine(player) => push(life, Line::Joined { club: to, how: Join::LoanReturn }),
        E::ContractSigned { player, club, until, renewal, .. } if mine(player) => {
            let first = !renewal && !life.entries.iter().any(|x| matches!(x.line, Line::Contract { .. }));
            push(life, Line::Contract { club, first, renewal, until });
            told(w, life, p, Org::Club(club), today);
        }
        E::Released { player, club } | E::AcademyReleased { player, club } if mine(player) => {
            push(life, Line::Released { club });
            let m = w.clubs.get(club).map_or(pw_core::StaffId::NONE, |c| c.manager);
            if m.is_some() {
                add_tie(life, w.staff[m].person, TieKind::LetGo { club }, d, d);
            }
        }
        E::AcademyJoined { player, club } if mine(player) => {
            push(life, Line::Joined { club, how: Join::Academy });
            told(w, life, p, Org::Club(club), today);
        }
        E::TrialStarted { player, club } | E::AcademyTrialStarted { player, club } if mine(player) => {
            push(life, Line::Trial { club });
            told(w, life, p, Org::Club(club), today);
        }
        E::TrialEnded { player, club, offered } if mine(player) => {
            push(life, Line::TrialOutcome { club, offered });
            if !offered {
                let m = w.clubs.get(club).map_or(pw_core::StaffId::NONE, |c| c.manager);
                if m.is_some() {
                    add_tie(life, w.staff[m].person, TieKind::LetGo { club }, d, d);
                }
            }
        }
        E::RecruitWon { person, institution, .. } if person == who => {
            let tier = w.ext.ecosystem.scholarship.get(&p).filter(|s| s.inst == institution).map_or(0, |s| s.tier);
            push(life, Line::Scholarship { inst: institution, tier, contested: true });
        }
        E::EnrolledUniversity { person, institution } if person == who => {
            push(life, Line::Enrolled { inst: institution });
            if p.is_some() {
                told(w, life, p, Org::Institution(institution), today);
            }
        }
        E::Graduated { person, institution, .. } if person == who => push(life, Line::Graduated { inst: institution }),
        E::ExamsSat { person, passed } if person == who => push(life, Line::Exams { passed }),
        E::MinorTitle { history } if p.is_some() => {
            let Some(s) = w.minor.history.get(history as usize) else { return };
            let Some(l) = w.minor.careers.get(&p).and_then(|v| v.iter().rev().find(|l| l.season == s.season && l.kind == s.kind && l.apps > 0)) else { return };
            push(
                life,
                Line::MinorSeason { history, apps: l.apps, goals: l.goals, won: s.winner == l.entrant, runner_up: s.runner_up == l.entrant && s.winner != l.entrant, top_scorer: s.top_scorer == p, best: s.best == p },
            );
        }
        E::Debut { player, team, comp } if mine(player) => {
            let club = club_of_team(w, team);
            push(life, Line::Debut { club, comp, uid: uid_on(w, d, club, comp) });
        }
        E::FirstGoal { player, team, comp } if mine(player) => {
            let club = club_of_team(w, team);
            push(life, Line::FirstGoal { club, comp, uid: uid_on(w, d, club, comp) });
        }
        E::PathwayStep { player, kind, target } if mine(player) => {
            let stage = StageKind::from_code(kind);
            match stage {
                StageKind::StateTeam => push(life, Line::StateSide { state: pw_core::RegionId(target) }),
                StageKind::NationalCamp | StageKind::District | StageKind::StateYouth | StageKind::StateLeague | StageKind::SemiPro => {
                    if let Some(why) = w.ext.pathway.for_step(p, stage, d) {
                        push(life, Line::Step { stage, why });
                    }
                }
                _ => {}
            }
        }
        E::NationalSquad { player, nation, .. } if mine(player) => push(life, Line::NationalSquad { nation }),
        E::InternationalDebut { player, nation, .. } if mine(player) => push(life, Line::Capped { nation }),
        E::WithdrewFromSquad { player, nation } if mine(player) => push(life, Line::WithdrewFromSquad { nation }),
        E::Injured { player, injury, days } if mine(player) && days >= 7 => push(life, Line::Injury { injury, days }),
        E::InjurySetback { player, days } if mine(player) => push(life, Line::Setback { days }),
        E::Recovered { player } if mine(player) => {
            let open = life.entries.iter().rev().find(|x| matches!(x.line, Line::Injury { .. } | Line::Recovered)).is_some_and(|x| matches!(x.line, Line::Injury { .. }));
            if open {
                push(life, Line::Recovered);
            }
        }
        E::Award { player, comp, award, season } if mine(player) => push(life, Line::Honour { award, comp, season }),
        E::Champion { comp, team, season } if p.is_some() && w.players.hot[p].team == team => push(life, Line::Title { comp, club: club_of_team(w, team), season }),
        E::Promoted { comp, team } if p.is_some() && w.players.hot[p].team == team => push(life, Line::Promoted { comp, club: club_of_team(w, team) }),
        E::Relegated { comp, team } if p.is_some() && w.players.hot[p].team == team => push(life, Line::Relegated { comp, club: club_of_team(w, team) }),
        E::Captaincy { player, team } if mine(player) => push(life, Line::Captain { club: club_of_team(w, team) }),
        E::TookUnderWing { mentor, mentee } if mentee == who => {
            push(life, Line::Mentor { mentor });
            add_tie(life, mentor, TieKind::Mentor, d, d);
        }
        E::Milestone { player, kind, count, club } if mine(player) => push(life, Line::Milestone { kind, count, club }),
        E::RecordBroken { player, kind, club, value } if mine(player) => push(life, Line::Record { kind, club, value }),
        E::Breakout { player, .. } if mine(player) && !life.entries.iter().any(|x| x.line == Line::Breakout) => push(life, Line::Breakout),
        E::Published { story } => press(w, life, who, p, story, d, id),
        E::Endorsed { person, brand, .. } if person == who => push(life, Line::Endorsed { brand }),
        E::Life { person, kind } if person == who => push(life, Line::Life { kind }),
        E::MovedHome { person, bought } if person == who => push(life, Line::MovedHome { bought }),
        E::Retired { person } if person == who => push(life, Line::Retired),
        _ => meanwhile(w, life, who, p, e),
    }
}

/// Coverage of the person: the first piece at each reach, and the pieces that looked at them in depth.
fn press(w: &World, life: &mut Life, who: PersonId, p: PlayerId, story: pw_core::StoryId, d: Date, id: EventId) {
    let Some(s) = w.media.stories.get(story) else { return };
    if s.person != who && !(p.is_some() && s.player == p) {
        return;
    }
    let layer = layer_of(w, s.outlet, w.people[who].nation);
    let first = !life.reached.contains(&layer);
    let deep = matches!(s.kind, StoryKind::Feature | StoryKind::WonderkidList | StoryKind::Retrospective | StoryKind::Interview) || s.news >= 80;
    // The first time each club is said to want you is part of the story, whether or not anything came of it.
    let linked = s.kind == StoryKind::TransferRumour
        && s.other_club.is_some()
        && !life.entries.iter().any(|x| matches!(x.line, Line::Press { story, .. } if w.media.stories.get(story).is_some_and(|o| o.kind == StoryKind::TransferRumour && o.other_club == s.other_club)));
    if first || deep || linked {
        if first {
            life.reached.push(layer);
        }
        life.push(d, Line::Press { story, layer, first }, id);
    }
}

/// What people from the past did next: only what is public, and only for crossings that meant something.
fn meanwhile(w: &World, life: &mut Life, who: PersonId, p: PlayerId, e: &Event) {
    if life.ties.is_empty() {
        return;
    }
    let my_club = if p.is_some() { w.players.hot[p].club } else { ClubId::NONE };
    let person_of = |q: PlayerId| w.players.cold.get(q).map(|c| c.person);
    let (other, then) = match e.kind {
        E::ManagerAppointed { staff, club } => {
            let Some(s) = w.staff.get(staff) else { return };
            (s.person, if club == my_club { Then::ManagesYou { club } } else { Then::BecameManager { club } })
        }
        E::JoinedStaff { person, staff, club } => {
            let role = w.staff.get(staff).map_or(StaffRole::Coach, |s| s.role);
            (person, if role == StaffRole::Manager { if club == my_club { Then::ManagesYou { club } } else { Then::BecameManager { club } } } else { Then::JoinedStaff { club, role } })
        }
        E::Transfer { player, to, .. } => {
            let Some(q) = person_of(player) else { return };
            (q, if to == my_club { Then::JoinedYourClub { club: to } } else { Then::Moved { club: to } })
        }
        E::InternationalDebut { player, nation, .. } => {
            let Some(q) = person_of(player) else { return };
            (q, Then::Capped { nation })
        }
        E::Award { player, award, .. } => {
            let Some(q) = person_of(player) else { return };
            (q, Then::Honoured { award })
        }
        E::Retired { person } => (person, Then::Retired),
        _ => return,
    };
    if other == who {
        return;
    }
    let Some(t) = life.best_tie(other) else { return };
    let tie = life.ties[t];
    // A brief crossing with a teammate or classmate is not a shared past; a move inside the club you share is not news to you.
    let shared = match tie.kind {
        TieKind::Teammate { club } => tie.days() >= TEAMMATE_DAYS && club != my_club,
        TieKind::Classmate { .. } => tie.days() >= TEAMMATE_DAYS || !matches!(then, Then::Moved { .. }),
        _ => true,
    };
    let notable = shared || matches!(then, Then::JoinedYourClub { .. } | Then::ManagesYou { .. });
    if !notable {
        return;
    }
    let line = Line::Meanwhile { who: other, tie: t as u16, then };
    if !life.entries.iter().any(|x| x.event == e.id && x.line == line) {
        life.push(e.date, line, e.id);
    }
}

/// A match of the person's: worth a line when it was an occasion or they made it one, and when it brought someone from their past.
#[allow(clippy::too_many_arguments)]
fn on_app(w: &World, life: &mut Life, p: PlayerId, club: ClubId, comp: pw_core::CompId, goals: u8, assists: u8, today: Date) {
    let Some(m) = w.recent_matches.on(today).find(|m| m.involves(club) && m.comp == comp) else { return };
    let opp = if m.home == club { m.away } else { m.home };
    let side_result = if m.home == club { m.result() } else { -m.result() };
    let decisive = w.fixtures.on(today).iter().map(|&f| w.fixtures.get(f)).find(|f| f.uid == m.uid).is_some_and(|f| f.decisive);
    let big = if m.hat_tricks.contains(&p) {
        Some(Big::HatTrick)
    } else if m.late_winner.is_some_and(|g| g.player == p) {
        Some(Big::Winner)
    } else if decisive && goals > 0 {
        Some(Big::Decisive)
    } else if m.derby && (goals > 0 || m.pom == p) {
        Some(Big::Derby)
    } else if goals >= 2 {
        Some(Big::Brace)
    } else if m.significance >= 80 && goals > 0 {
        Some(Big::BigOccasion)
    } else if m.pom == p && m.significance >= 60 {
        Some(Big::BestOnPitch)
    } else {
        None
    };
    if let Some(big) = big {
        life.push(today, Line::Match { uid: m.uid, club, opp, comp, goals, assists, result: side_result, big }, EventId::NONE);
    }
    faced(w, life, opp, m.uid, today);
}

/// People from the past on the other side today, told the first time only.
fn faced(w: &World, life: &mut Life, opp: ClubId, uid: u64, today: Date) {
    let mut seen: FxHashMap<PersonId, ()> = FxHashMap::default();
    let mut lines = Vec::new();
    let manager = w.clubs.get(opp).map(|c| c.manager).filter(|m| m.is_some()).map(|m| w.staff[m].person);
    for (i, t) in life.ties.iter().enumerate() {
        if seen.contains_key(&t.person) || life.entries.iter().any(|x| matches!(x.line, Line::Faced { who, .. } if who == t.person)) {
            continue;
        }
        let meant = match t.kind {
            TieKind::Teammate { club } => t.days() >= TEAMMATE_DAYS && club != opp,
            TieKind::Classmate { .. } => true,
            TieKind::Coach { club } | TieKind::LetGo { club } => club != opp,
            TieKind::Mentor | TieKind::Finder | TieKind::Scout { .. } => false,
        };
        if !meant {
            continue;
        }
        let on_pitch = || {
            let q = w.people.get(t.person).map_or(PlayerId::NONE, |x| x.player);
            q.is_some() && w.perf.recent.get(&q).is_some_and(|apps| apps.iter().any(|a| a.date == today && a.club == opp))
        };
        if manager == Some(t.person) || on_pitch() {
            seen.insert(t.person, ());
            lines.push(Line::Faced { who: t.person, tie: i as u16, uid, club: opp });
        }
    }
    for l in lines {
        life.push(today, l, EventId::NONE);
    }
}
