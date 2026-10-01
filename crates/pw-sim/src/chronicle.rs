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
        let mut trips: Vec<(ClubId, f32)> = Vec::new();
        if p.is_some() {
            seasons(w, &mut life, p);
            if let Some(apps) = w.perf.recent.get(&p) {
                for a in apps.iter().filter(|a| a.date == today) {
                    on_app(w, &mut life, p, a.club, a.comp, a.goals, a.assists, today);
                    if let Some(km) = w.recent_matches.on(today).find(|m| m.involves(a.club) && m.comp == a.comp && m.away == a.club).and_then(|m| club_km(w, a.club, m.home)) {
                        trips.push((a.club, km));
                    }
                }
            }
            back_with_group(w, &mut life, p, today);
            talked_about(w, &mut life, id, p, today);
            if let Some((spot, club, nation)) = spotted(w, id, p, today, &fresh)
                && !life.entries.iter().any(|x| matches!(x.line, Line::Spotted { spot: s, .. } if s.same_kind(spot)))
            {
                life.push(today, Line::Spotted { spot, club, nation }, EventId::NONE);
            }
            new_place(w, &mut life, id, p, today);
            if today.day() == 1 {
                nothing_came_of_it(w, &mut life, p, today);
            }
            if today.weekday() == pw_core::Weekday::Mon {
                refresh_ties(w, &mut life, p, today);
                language(w, &mut life, id, p, today);
            }
        }
        w.ext.chronicle.lives.insert(id, life);
        for (club, km) in trips {
            let v = w.ext.journeys.of.entry(id).or_default();
            let year = today.year();
            match v.iter_mut().find(|t| t.year == year && t.club == club) {
                Some(t) => {
                    t.km += km.round() as u32;
                    t.trips += 1;
                }
                None => v.push(pw_world::chronicle::Travel { year, club, km: km.round() as u32, trips: 1 }),
            }
        }
    }
}

/// Map units of the regions' map in km (`ecosystem::Region::x`, `y`).
pub const KM_PER_UNIT: f32 = 30.0;

/// Distance in km between two clubs' home regions, when both are on the map.
pub fn club_km(w: &World, a: ClubId, b: ClubId) -> Option<f32> {
    let eco = &w.ext.ecosystem;
    let ra = eco.regions.get(*eco.club_region.get(&a)?)?;
    let rb = eco.regions.get(*eco.club_region.get(&b)?)?;
    let (dx, dy) = (f32::from(ra.x) - f32::from(rb.x), f32::from(ra.y) - f32::from(rb.y));
    Some((dx * dx + dy * dy).sqrt() * KM_PER_UNIT)
}

/// Smallest distance (km) of an away trip that is a flight.
pub const FLIGHT_KM: f32 = 500.0;

/// Recognised in public today: asked for an autograph in the street or outside training, or photographed at an airport on the
/// way to an away match far off, to join the national squad, or to a new club far from the last. How likely grows with how well
/// known the person is (`renown`): the city's standing for autographs, celebrity and continental standing for the photographers;
/// below a floor of standing, never. Pure and keyed on the day and the person: the chronicle and the chats ask it the same
/// question and get the same answer. `fresh` is the day's new events.
pub fn spotted(w: &World, who: PersonId, p: PlayerId, today: Date, fresh: &[Event]) -> Option<(pw_world::chronicle::Spot, ClubId, pw_core::NationId)> {
    use pw_world::chronicle::Spot;
    let r = w.renown.people.get(&who)?;
    let club = w.players.hot[p].club;
    let mut rng = pw_core::Rng::keyed(&[w.seed, 0x7370_6f74, u64::from(today.0 as u32), u64::from(who.0)]);
    // Photographers at the airport: only on a trip, more often the better known.
    let wide = f32::from(r.fame.max(r.continental)) / 10_000.0;
    let lens = if wide < 0.2 { 0.0 } else { ((wide - 0.2) * 1.2).min(0.8) };
    let mut trip = None;
    for e in fresh.iter().filter(|e| e.date == today) {
        match e.kind {
            E::Transfer { player, from, to, .. } if player == p && far_move(w, p, from, to).is_some() => trip = Some((Spot::Moving, to, pw_core::NationId::NONE)),
            E::NationalSquad { player, nation, .. } if player == p && trip.is_none() => trip = Some((Spot::SquadTrip, ClubId::NONE, nation)),
            _ => {}
        }
    }
    let played_today = w.perf.recent.get(&p).is_some_and(|v| v.iter().any(|a| a.date == today));
    if trip.is_none() && played_today && club.is_some() {
        let here = w.clubs.get(club).map(|k| k.nation);
        if let Some(m) = w.recent_matches.on(today).find(|m| m.away == club) {
            let abroad = w.clubs.get(m.home).map(|k| k.nation) != here;
            if abroad || club_km(w, club, m.home).is_some_and(|km| km >= FLIGHT_KM) {
                trip = Some((Spot::AwayTrip, m.home, pw_core::NationId::NONE));
            }
        }
    }
    let flash = rng.chance(lens);
    if let Some(t) = trip
        && flash
    {
        return Some(t);
    }
    // Autographs: the city's standing, on an ordinary day.
    let local = f32::from(r.local) / 10_000.0;
    let ask = if local < 0.25 { 0.0 } else { (local - 0.25) * 0.03 };
    let roll = rng.f32();
    let training_day = club.is_some() && !played_today && !matches!(today.weekday(), pw_core::Weekday::Sat | pw_core::Weekday::Sun);
    if training_day && roll < ask {
        Some((Spot::Training, club, pw_core::NationId::NONE))
    } else if roll >= ask && roll < ask * 1.7 {
        Some((Spot::Street, club, pw_core::NationId::NONE))
    } else {
        None
    }
}

/// Whether a move from club `from` to club `to` takes the person to another country (`Some(true)`) or another state of their own
/// (`Some(false)`); `None` for a move nearby. With no club before, home is where the person grew up.
pub fn far_move(w: &World, p: PlayerId, from: ClubId, to: ClubId) -> Option<bool> {
    let eco = &w.ext.ecosystem;
    let person_nation = w.players.cold.get(p).and_then(|c| w.people.get(c.person)).map(|x| x.nation);
    let nation = |k: ClubId| if k.is_some() { w.clubs.get(k).map(|c| c.nation) } else { person_nation };
    let (a, b) = (nation(from)?, nation(to)?);
    if a != b {
        return Some(true);
    }
    let state = |k: ClubId| if k.is_some() { eco.club_region.get(&k).map(|&r| eco.state_of(r)) } else { eco.story.get(&p).map(|s| eco.state_of(s.home)) };
    match (state(from), state(to)) {
        (Some(x), Some(y)) if x != y => Some(false),
        _ => None,
    }
}

/// Where someone lives after arriving on `since`: the home they chose since then (rented or bought, with its quality), or, until
/// they do, what the club found: a host family for someone under eighteen, digs for the rest.
pub fn place_after_move(w: &World, who: PersonId, since: Date) -> (pw_world::affairs::HomeKind, u8) {
    use pw_world::affairs::HomeKind;
    let home = w.affairs.of(who).map(|a| a.home).unwrap_or_default();
    if home.since >= since && matches!(home.kind, HomeKind::Rented | HomeKind::Owned) {
        return (home.kind, home.quality);
    }
    let young = w.people.get(who).is_some_and(|x| x.age(w.date) < 18);
    (if young { HomeKind::Family } else { HomeKind::Digs }, 0)
}

/// Days after a move far from home when where you live is told.
const NEW_PLACE_DAYS: i32 = 10;

/// A few days after a permanent move to another state or country: where you ended up living, once a move.
fn new_place(w: &World, life: &mut Life, who: PersonId, p: PlayerId, today: Date) {
    let Some(i) = life.entries.iter().rposition(|x| matches!(x.line, Line::Joined { how: Join::Transfer | Join::Signed, .. })) else { return };
    let (arrived, Line::Joined { club, .. }) = (life.entries[i].date, life.entries[i].line) else { return };
    let days = arrived.days_until(today);
    if !(NEW_PLACE_DAYS..=NEW_PLACE_DAYS + 60).contains(&days) || w.players.hot[p].club != club {
        return;
    }
    if life.entries[i..].iter().any(|x| matches!(x.line, Line::NewPlace { club: c, .. } if c == club)) {
        return;
    }
    // Where you came from: the club of the spell before this one, or the last club the story knows.
    let before = w.history.spells.get(&p).and_then(|v| v.iter().rev().find(|s| s.club != club && s.from <= arrived)).map(|s| s.club).or_else(|| {
        life.entries[..i].iter().rev().find_map(|x| match x.line {
            Line::Joined { club: c, .. } => Some(c),
            _ => None,
        })
    });
    if far_move(w, p, before.unwrap_or(ClubId::NONE), club).is_none() {
        return;
    }
    let (home, quality) = place_after_move(w, who, arrived);
    life.push(today, Line::NewPlace { club, home, quality }, EventId::NONE);
}

/// The first time each wider circle talks about you online: your own supporters, another club's, another state, another country.
fn talked_about(w: &World, life: &mut Life, who: PersonId, p: PlayerId, today: Date) {
    use pw_world::chronicle::FanReach;
    if life.entries.iter().filter(|x| matches!(x.line, Line::Talked { .. })).count() >= 4 {
        return;
    }
    let home = w.people[who].nation;
    let mine = w.players.hot[p].club;
    let eco = &w.ext.ecosystem;
    let state = |c: ClubId| eco.club_region.get(&c).map(|&r| eco.state_of(r));
    let my_state = state(mine).or_else(|| eco.story.get(&p).map(|s| eco.state_of(s.home)));
    for post in w.net.posts.iter().rev().take_while(|x| x.date >= today) {
        if post.about != who && post.about2 != who {
            continue;
        }
        let Some(acc) = w.net.accounts.get(post.author as usize) else { continue };
        if acc.person == who || acc.kind == pw_world::socialnet::AccountKind::Person {
            continue;
        }
        let their_state = if acc.club.is_some() { state(acc.club) } else { None };
        let reach = if acc.nation.is_some() && home.is_some() && acc.nation != home {
            FanReach::Abroad
        } else if acc.club.is_some() && acc.club == mine {
            FanReach::OwnClub
        } else if their_state.is_some() && my_state.is_some() && their_state != my_state {
            FanReach::OtherState
        } else if acc.club.is_some() {
            FanReach::OtherClub
        } else {
            continue;
        };
        if life.entries.iter().any(|x| matches!(x.line, Line::Talked { reach: r, .. } if r == reach)) {
            continue;
        }
        let region = their_state.unwrap_or(pw_core::RegionId::NONE);
        life.push(today, Line::Talked { reach, club: acc.club, nation: acc.nation, region }, EventId::NONE);
    }
}

/// Weekly, while playing in another country: the language of that country reaching a new level, each level once.
fn language(w: &World, life: &mut Life, who: PersonId, p: PlayerId, today: Date) {
    let club = w.players.hot[p].club;
    let Some(k) = w.clubs.get(club) else { return };
    let (here, home) = (k.nation, w.people[who].nation);
    if here == home || here.is_none() || w.nations.get(here).is_none() {
        return;
    }
    let target = w.nations[here].env.language;
    let Some(life_state) = w.lives.get(who) else { return };
    let best = life_state.languages.iter().map(|&(n, f)| if n == here || w.nations.get(n).is_some_and(|x| x.env.language == target) { f } else { 0 }).max().unwrap_or(0);
    let level = match best {
        85.. => 3,
        55..=84 => 2,
        25..=54 => 1,
        _ => 0,
    };
    let told = life.entries.iter().filter_map(|x| match x.line {
        Line::Language { nation, level } if nation == here => Some(level),
        _ => None,
    });
    let max_told = told.max().unwrap_or(0);
    // Someone who already spoke it when he arrived has nothing to tell.
    let arrived = life.entries.iter().rev().find(|x| matches!(x.line, Line::Joined { club: c, .. } if c == club)).map(|x| x.date);
    if level > max_told && arrived.is_some_and(|d| d.days_until(today) >= 14) {
        life.push(today, Line::Language { nation: here, level }, EventId::NONE);
    }
}

/// The index of the last injury line, if the person is still on the way back from it (no recovery line after it).
fn open_injury(life: &Life) -> Option<usize> {
    let i = life.entries.iter().rposition(|x| matches!(x.line, Line::Injury { .. }))?;
    Some(i)
}

/// Part of the way back: the first day training with the group again (the medical stage the training ground already uses).
fn back_with_group(w: &World, life: &mut Life, p: PlayerId, today: Date) {
    let h = &w.players.hot[p];
    if h.injury == 0 {
        return;
    }
    let stage = pw_world::medical::ReturnStage::of(f32::from(h.injury_days) / f32::from(h.injury_total.max(1)));
    if stage < pw_world::medical::ReturnStage::PartialTeam {
        return;
    }
    let Some(i) = open_injury(life) else { return };
    if life.entries[i + 1..].iter().any(|x| matches!(x.line, Line::BackWithGroup | Line::Recovered)) {
        return;
    }
    life.push(today, Line::BackWithGroup, EventId::NONE);
}

/// Monthly: clubs the press linked you with that never came. Four months after the first link, with no move to that club, the
/// story says so once.
fn nothing_came_of_it(w: &World, life: &mut Life, p: PlayerId, today: Date) {
    let mine = w.players.hot[p].club;
    let mut found: Vec<(ClubId, pw_core::StoryId)> = Vec::new();
    for (i, e) in life.entries.iter().enumerate() {
        let Line::Press { story, .. } = e.line else { continue };
        let Some(s) = w.media.stories.get(story) else { continue };
        if s.kind != StoryKind::TransferRumour || s.other_club.is_none() || e.date.days_until(today) < 120 {
            continue;
        }
        let club = s.other_club;
        let came = club == mine || life.entries[i..].iter().any(|x| matches!(x.line, Line::Joined { club: c, .. } if c == club));
        let told = life.entries.iter().any(|x| matches!(x.line, Line::NothingCameOfIt { club: c, .. } if c == club));
        if !came && !told && !found.iter().any(|(c, _)| *c == club) {
            found.push((club, story));
        }
    }
    for (club, story) in found {
        life.push(today, Line::NothingCameOfIt { club, story }, EventId::NONE);
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
        E::Transfer { player, to, .. } if mine(player) => {
            push(life, Line::Joined { club: to, how: Join::Transfer });
            reunion(w, life, who, to, d, id);
        }
        E::LoanMove { player, to, .. } if mine(player) => {
            push(life, Line::Joined { club: to, how: Join::Loan });
            reunion(w, life, who, to, d, id);
        }
        E::LoanReturn { player, to } if mine(player) => push(life, Line::Joined { club: to, how: Join::LoanReturn }),
        E::ContractSigned { player, club, until, renewal, wage } if mine(player) => {
            let first = !renewal && !life.entries.iter().any(|x| matches!(x.line, Line::Contract { .. }));
            push(life, Line::Contract { club, first, renewal, until });
            push(life, Line::Terms { club, until, wage });
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
            if let Some(line) = trial_views(w, club, p) {
                push(life, line);
            }
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
        E::AskedIfReady { player, manager } if mine(player) => push(life, Line::AskedIfReady { by: manager }),
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
        _ => {
            if let Some((club, news)) = club_news(w, p, &e.kind) {
                push(life, Line::AtClub { club, news });
            }
            meanwhile(w, life, who, p, e);
        }
    }
}

/// What a club tells you at a trial's verdict about how its people saw you: only that the coaching side and the scouting side
/// disagreed, and which way, when they did by a clear margin. Never a number.
fn trial_views(w: &World, club: ClubId, p: PlayerId) -> Option<Line> {
    use pw_world::staff::StaffRole as R;
    let d = w.dossiers.get(club, p)?;
    let best = |roles: &[R]| d.opinions.iter().filter(|o| roles.contains(&o.role)).max_by(|a, b| a.weight.total_cmp(&b.weight).then(b.by.cmp(&a.by)));
    let coach = best(&[R::Manager, R::Assistant, R::Coach, R::HeadOfYouth])?;
    let scout = best(&[R::Scout, R::Analyst])?;
    let gap = coach.ca.mid - scout.ca.mid; // truth-ok: two evaluators' readings (beliefs, not truth), told only as who rated you higher
    if gap.abs() < 6.0 {
        return None;
    }
    let person = |s: pw_core::StaffId| w.staff.get(s).map(|x| x.person).filter(|q| q.is_some());
    let (keen, doubtful) = if gap > 0.0 { (person(coach.by)?, person(scout.by)?) } else { (person(scout.by)?, person(coach.by)?) };
    Some(Line::TrialViews { club, keen, doubtful })
}

/// Arriving at a club whose manager is someone from the past: the coach who let you go elsewhere, a former teammate.
fn reunion(w: &World, life: &mut Life, who: PersonId, club: ClubId, d: Date, id: EventId) {
    let Some(m) = w.clubs.get(club).map(|k| k.manager).filter(|m| m.is_some()) else { return };
    let boss = w.staff[m].person;
    if boss == who {
        return;
    }
    let Some(t) = life.best_tie(boss) else { return };
    if matches!(life.ties[t].kind, TieKind::Coach { club: c } if c == club) {
        return;
    }
    let line = Line::Meanwhile { who: boss, tie: t as u16, then: Then::ManagesYou { club } };
    if !life.entries.iter().any(|x| x.event == id && x.line == line) {
        life.push(d, line, id);
    }
}

/// Coverage of the person: the first piece at each reach, and the pieces that looked at them in depth.
fn press(w: &World, life: &mut Life, who: PersonId, p: PlayerId, story: pw_core::StoryId, d: Date, id: EventId) {
    let Some(s) = w.media.stories.get(story) else { return };
    // A correction or a denial of something written about you: the earlier piece was answered in public.
    if matches!(s.kind, StoryKind::Correction | StoryKind::Denial) {
        for &r in &s.refs {
            let about_me = w.media.stories.get(r).is_some_and(|o| o.person == who || (p.is_some() && o.player == p));
            let line = Line::Answered { story: r, answer: story, corrected: s.kind == StoryKind::Correction };
            if about_me && !life.has(&line) {
                life.push(d, line, id);
            }
        }
    }
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
    } else if decider(m, club, p) {
        Some(Big::Decider)
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
    // The first goal for a new club, after scoring for another first (the very first goal is its own line).
    if goals > 0 && !life.entries.iter().any(|x| matches!(x.line, Line::FirstGoal { club: c, .. } | Line::FirstGoalFor { club: c, .. } if c == club)) {
        let lines = w.perf.seasons.get(&p);
        let here: u32 = lines.map_or(0, |v| v.iter().filter(|l| l.club == club).map(|l| u32::from(l.goals)).sum());
        let elsewhere = lines.is_some_and(|v| v.iter().any(|l| l.club != club && l.goals > 0)) || life.entries.iter().any(|x| matches!(x.line, Line::FirstGoal { .. }));
        if elsewhere && here <= u32::from(goals) {
            life.push(today, Line::FirstGoalFor { club, uid: m.uid }, EventId::NONE);
        }
    }
    // The first match back after an injury that kept you out.
    if let Some(i) = open_injury(life) {
        let since = life.entries[i].date;
        let played_since = w.perf.recent.get(&p).is_some_and(|v| v.iter().any(|a| a.date > since && a.date < today));
        let told = life.entries[i..].iter().any(|x| matches!(x.line, Line::Comeback { .. }));
        if !played_since && !told {
            let days = since.days_until(today).clamp(0, i32::from(u16::MAX)) as u16;
            life.push(today, Line::Comeback { uid: m.uid, club, opp, days }, EventId::NONE);
        }
    }
    faced(w, life, opp, m.uid, today);
}

/// Whether `p` scored the goal that won `club` the match by one: the side's goal that took it past the other side's final total.
fn decider(m: &pw_world::matchfacts::MatchFacts, club: ClubId, p: PlayerId) -> bool {
    let side = u8::from(m.away == club);
    let (gf, ga) = if side == 0 { (m.hg, m.ag) } else { (m.ag, m.hg) };
    if m.pens.is_some() || gf != ga + 1 {
        return false;
    }
    let mut ours: Vec<&pw_world::matchfacts::Goal> = m.goals.iter().filter(|g| g.side == side).collect();
    ours.sort_by_key(|g| g.minute);
    ours.get(usize::from(ga)).is_some_and(|g| g.player == p && !g.own_goal)
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

/// What changed at the person's own club: a new manager or a sacking, a takeover, administration, a points deduction, an owner's
/// money, a new facility.
pub fn club_news(w: &World, p: PlayerId, kind: &E) -> Option<(ClubId, pw_world::chronicle::ClubNews)> {
    use pw_world::chronicle::ClubNews as N;
    if p.is_none() {
        return None;
    }
    let mine = w.players.hot[p].club;
    let staff_person = |s: pw_core::StaffId| w.staff.get(s).map_or(PersonId::NONE, |x| x.person);
    let (club, news) = match *kind {
        E::ManagerAppointed { staff, club } => (club, N::NewManager { who: staff_person(staff) }),
        E::ManagerSacked { staff, club } => (club, N::ManagerSacked { who: staff_person(staff) }),
        E::Takeover { club, owner, .. } => (club, N::Takeover { owner }),
        E::Administration { club } => (club, N::Administration),
        E::PointsDeducted { club, points } => (club, N::PointsDeducted { points }),
        E::OwnerInvestment { club, .. } => (club, N::Investment),
        E::ProjectCompleted { club, kind } => (club, N::Facility { kind }),
        _ => return None,
    };
    (club.is_some() && club == mine).then_some((club, news))
}

#[cfg(test)]
mod tests {
    use super::decider;
    use pw_core::{ClubId, CompId, Date, FixtureId, PlayerId, TeamId};
    use pw_world::matchfacts::{Goal, MatchFacts};

    fn facts(hg: u8, ag: u8, goals: &[(u32, u8, u8, bool)]) -> MatchFacts {
        MatchFacts {
            uid: 1,
            fixture: FixtureId(0),
            date: Date(0),
            comp: CompId(0),
            home_team: TeamId(0),
            away_team: TeamId(1),
            home: ClubId(0),
            away: ClubId(1),
            hg,
            ag,
            pens: None,
            goals: goals.iter().map(|&(p, minute, side, own_goal)| Goal { player: PlayerId(p), assist: PlayerId::NONE, minute, side, penalty: false, own_goal }).collect(),
            reds: Default::default(),
            late_winner: None,
            hat_tricks: Default::default(),
            comeback: false,
            pom: PlayerId::NONE,
            pom_rating: 0,
            debut_goals: Default::default(),
            significance: 0,
            derby: false,
        }
    }

    #[test]
    fn the_winner_is_the_goal_that_took_the_side_past_the_other_sides_total() {
        // 2-1: home goals by 7 (10') and 8 (60'), away by 9 (30'). The second home goal won it.
        let m = facts(2, 1, &[(7, 10, 0, false), (9, 30, 1, false), (8, 60, 0, false)]);
        assert!(decider(&m, ClubId(0), PlayerId(8)));
        assert!(!decider(&m, ClubId(0), PlayerId(7)), "the opener did not win it");
        assert!(!decider(&m, ClubId(1), PlayerId(9)), "the losing side has no winner");
        // 1-0 away: the only goal.
        assert!(decider(&facts(0, 1, &[(5, 80, 1, false)]), ClubId(1), PlayerId(5)));
        // 3-1 is not won by one goal; a draw has no winner; an own goal is nobody's winner on the side it helped.
        assert!(!decider(&facts(3, 1, &[(7, 1, 0, false), (7, 2, 0, false), (9, 3, 1, false), (7, 4, 0, false)]), ClubId(0), PlayerId(7)));
        assert!(!decider(&facts(1, 1, &[(7, 1, 0, false), (9, 3, 1, false)]), ClubId(0), PlayerId(7)));
        assert!(!decider(&facts(1, 0, &[(9, 50, 0, true)]), ClubId(0), PlayerId(9)));
    }
}
