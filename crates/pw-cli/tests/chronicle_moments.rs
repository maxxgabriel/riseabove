//! The lived career's smaller moments, written by the simulation for a chronicled person: the way back from an injury (back with the
//! group, the comeback match), rumours that came to nothing, stories answered in public, the miles of a season, and what the
//! manager asked before the medical room had cleared you.

use pw_core::{Date, EventId, PlayerId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::World;
use pw_world::chronicle::Line;

fn lines(w: &World, who: pw_core::PersonId) -> Vec<Line> {
    w.ext.chronicle.of(who).map(|l| l.entries.iter().map(|e| e.line).collect()).unwrap_or_default()
}

/// A small world, a regular first-team player at the first club, chronicled from today.
fn chronicled(seed: u64) -> (Sim, PlayerId, pw_core::PersonId) {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), seed, Scale::SMALL));
    sim.run(30);
    let w = &sim.world;
    // The player with the most minutes of the last month: someone who plays.
    let p = w.players.ids().filter(|&q| w.players.hot[q].club.is_some() && w.players.hot[q].injury == 0).max_by_key(|&q| (w.players.hot[q].minutes_4w, std::cmp::Reverse(q))).unwrap();
    let who = w.players.cold[p].person;
    pw_sim::chronicle::begin(&mut sim.world, who);
    (sim, p, who)
}

#[test]
fn the_way_back_from_an_injury_is_told_back_with_the_group_then_the_comeback() {
    let (mut sim, p, who) = chronicled(41);
    let w = &mut sim.world;
    let today = w.date;
    // An injury of six weeks, a little over half of it behind him: training with the group again.
    let injury = (0..w.data.injuries.len()).find(|&i| w.data.injuries[i].days[2] >= 30).unwrap() as u16 + 1;
    w.ext.chronicle.lives.get_mut(&who).unwrap().push(today.add_days(-25), Line::Injury { injury, days: 42 }, EventId::NONE);
    {
        let h = &mut w.players.hot[p];
        h.injury = injury;
        h.injury_total = 42;
        h.injury_days = 17;
    }
    sim.run(1);
    let l = lines(&sim.world, who);
    assert_eq!(l.iter().filter(|x| matches!(x, Line::BackWithGroup)).count(), 1, "back with the group, once: {l:?}");
    sim.run(3);
    assert_eq!(lines(&sim.world, who).iter().filter(|x| matches!(x, Line::BackWithGroup)).count(), 1, "not again the next days");
    // Fit again: the first match he plays is the comeback, and only that one.
    {
        let h = &mut sim.world.players.hot[p];
        h.injury = 0;
        h.injury_days = 0;
        h.injury_total = 0;
    }
    let played = |w: &World| w.perf.recent.get(&p).map_or(0, |v| v.iter().filter(|a| a.date > today).count());
    for _ in 0..60 {
        if played(&sim.world) >= 2 {
            break;
        }
        sim.run(1);
    }
    assert!(played(&sim.world) >= 1, "he played in two months");
    let backs: Vec<Line> = lines(&sim.world, who).into_iter().filter(|x| matches!(x, Line::Comeback { .. })).collect();
    assert_eq!(backs.len(), 1, "one comeback line: {backs:?}");
    let Line::Comeback { days, club, uid, .. } = backs[0] else { unreachable!() };
    assert!(days >= 25, "out since the injury: {days} days");
    // The club he played for that day (on loan, his registration stays with the parent club).
    assert!(sim.world.perf.recent.get(&p).is_some_and(|v| v.iter().any(|a| a.club == club && a.date > today)), "the comeback is for the side he played for");
    assert_ne!(uid, u64::MAX);
}

#[test]
fn a_club_the_press_linked_you_with_that_never_came_is_told_once() {
    let (mut sim, p, who) = chronicled(42);
    let w = &mut sim.world;
    let mine = w.players.hot[p].club;
    let other = w.clubs.ids().find(|&k| k != mine).unwrap();
    // A rumour four months old that the chronicle kept as the first link with that club.
    let story = w.media.stories.iter().next().map(|s| s.id).expect("a story to borrow");
    let old = w.date.add_days(-130);
    {
        let s = w.media.stories.get_mut(story).unwrap();
        s.kind = pw_world::media::StoryKind::TransferRumour;
        s.player = p;
        s.person = who;
        s.other_club = other;
        s.date = old;
    }
    w.ext.chronicle.lives.get_mut(&who).unwrap().push(old, Line::Press { story, layer: pw_world::chronicle::Layer::National, first: false }, EventId::NONE);
    // To the first of next month, and a month beyond.
    let (y, m, _) = w.date.ymd();
    let first = if m == 12 { Date::from_ymd(y + 1, 1, 1) } else { Date::from_ymd(y, m + 1, 1) };
    let days = sim.world.date.days_until(first) as u32 + 35;
    sim.run(days);
    let told: Vec<Line> = lines(&sim.world, who).into_iter().filter(|x| matches!(x, Line::NothingCameOfIt { .. })).collect();
    if sim.world.players.hot[p].club == other {
        return; // he went there after all
    }
    assert_eq!(told.len(), 1, "told once: {told:?}");
    assert!(matches!(told[0], Line::NothingCameOfIt { club, .. } if club == other));
}

#[test]
fn a_correction_of_a_story_about_you_is_part_of_the_story() {
    let (mut sim, p, who) = chronicled(43);
    let w = &mut sim.world;
    let mut ids = w.media.stories.iter().map(|s| s.id);
    let (a, b) = (ids.next().unwrap(), ids.next().unwrap());
    {
        let s = w.media.stories.get_mut(a).unwrap();
        s.player = p;
        s.person = who;
    }
    {
        let s = w.media.stories.get_mut(b).unwrap();
        s.kind = pw_world::media::StoryKind::Correction;
        s.player = pw_core::PlayerId::NONE;
        s.person = pw_core::PersonId::NONE;
        s.refs.clear();
        s.refs.push(a);
    }
    let today = w.date;
    w.events.push(today, pw_world::event::Visibility::Public, pw_world::EventKind::Published { story: b });
    sim.run(1);
    let l = lines(&sim.world, who);
    assert!(l.iter().any(|x| matches!(x, Line::Answered { story, answer, corrected: true } if *story == a && *answer == b)), "{l:?}");
}

#[test]
fn away_trips_are_counted_by_year_and_club_when_the_clubs_are_on_the_map() {
    // The India route puts clubs on a map of regions; a synthetic world has none, and counts nothing.
    let mut w = pw_import::india::build(DataPack::builtin(), 7, pw_import::india::IndiaScale::TINY);
    let p = w.players.ids().find(|&q| w.players.hot[q].club.is_some() && w.ext.ecosystem.club_region.contains_key(&w.players.hot[q].club) && w.teams[w.players.hot[q].team].kind == pw_world::TeamKind::First).expect("a first-team player on the map");
    let who = w.players.cold[p].person;
    pw_sim::chronicle::begin(&mut w, who);
    let mut sim = Sim::new(w);
    sim.run(200);
    let w = &sim.world;
    let away = w.perf.seasons.get(&p).map_or(0, |v| v.iter().map(|l| u32::from(l.apps)).sum::<u32>());
    let trips: Vec<pw_world::chronicle::Travel> = w.ext.journeys.of.get(&who).cloned().unwrap_or_default();
    if away >= 4 {
        assert!(!trips.is_empty(), "{away} appearances and no away trip counted");
        assert!(trips.iter().any(|t| t.km > 0), "some away trip goes somewhere: {trips:?}");
    }
    for t in &trips {
        // A trip across town is a trip of a few km, or none on the map's scale.
        assert!(t.trips > 0, "{t:?}");
        assert!(t.km / u32::from(t.trips) < 4_000, "a trip inside the country: {t:?}");
    }
}

#[test]
fn playing_abroad_the_language_comes_in_steps_each_told_once() {
    let (mut sim, p, who) = chronicled(44);
    let w = &mut sim.world;
    let club = w.players.hot[p].club;
    let here = w.clubs[club].nation;
    let elsewhere = w.nations.ids().find(|&n| n != here && w.nations[n].env.language != w.nations[here].env.language);
    let elsewhere = elsewhere.expect("the small world has nations of different languages");
    // He is from somewhere else, arrived three weeks ago, and gets by.
    w.people[who].nation = elsewhere;
    let today = w.date;
    w.ext.chronicle.lives.get_mut(&who).unwrap().push(today.add_days(-21), Line::Joined { club, how: pw_world::chronicle::Join::Transfer }, EventId::NONE);
    w.lives[who].languages.retain(|(n, _)| *n != here);
    w.lives[who].languages.push((here, 40));
    let told = |w: &World| lines(w, who).into_iter().filter_map(|l| if let Line::Language { level, .. } = l { Some(level) } else { None }).collect::<Vec<u8>>();
    sim.run(8);
    assert_eq!(told(&sim.world), vec![1], "getting by, told on a Monday");
    sim.run(14);
    assert_eq!(told(&sim.world), vec![1], "not again");
    if let Some(l) = sim.world.lives[who].languages.iter_mut().find(|(n, _)| *n == here) {
        l.1 = 90;
    }
    sim.run(8);
    assert_eq!(told(&sim.world), vec![1, 3], "fluent, told once");
}

#[test]
fn at_a_trial_verdict_the_club_says_when_its_coach_and_its_scout_saw_you_differently() {
    use pw_world::dossier::{Opinion, Span};
    use pw_world::staff::StaffRole;
    let (mut sim, p, who) = chronicled(45);
    let w = &mut sim.world;
    let club = w.clubs.ids().find(|&k| k != w.players.hot[p].club && w.clubs[k].manager.is_some()).unwrap();
    let coach = w.clubs[club].manager.get().unwrap();
    let scout = w.staff.ids().find(|&s| s != coach && w.staff[s].person.is_some()).unwrap();
    let mut d = w.dossiers.map.values().next().cloned().expect("a dossier to borrow");
    d.player = p;
    d.club = club;
    d.opinions.clear();
    d.opinions.push(Opinion { by: coach, role: StaffRole::Manager, ca: Span { mid: 120.0, band: 5.0 }, pa: Span { mid: 140.0, band: 8.0 }, weight: 0.6, trust: 0.7 });
    d.opinions.push(Opinion { by: scout, role: StaffRole::Scout, ca: Span { mid: 104.0, band: 5.0 }, pa: Span { mid: 125.0, band: 8.0 }, weight: 0.4, trust: 0.6 });
    w.dossiers.map.insert((club, p), d);
    let today = w.date;
    w.events.push(today, pw_world::event::Visibility::Person(who), pw_world::EventKind::TrialEnded { player: p, club, offered: true });
    sim.run(1);
    let views: Vec<Line> = lines(&sim.world, who).into_iter().filter(|x| matches!(x, Line::TrialViews { .. })).collect();
    assert_eq!(views.len(), 1, "{views:?}");
    let Line::TrialViews { keen, doubtful, club: k } = views[0] else { unreachable!() };
    assert_eq!(k, club);
    assert_eq!(keen, sim.world.staff[coach].person, "the coach rated him higher");
    assert_eq!(doubtful, sim.world.staff[scout].person);
}

fn renowned(local: u16, wide: u16) -> pw_world::renown::Renown {
    pw_world::renown::Renown { local, continental: wide, fame: wide, ..Default::default() }
}

#[test]
fn being_recognised_in_public_grows_with_renown_and_a_trip_is_when_the_cameras_are_there() {
    use pw_world::chronicle::Spot;
    let (mut sim, p, who) = chronicled(46);
    let d0 = sim.world.date;
    // Over a few years of days, with no trips: how often someone asks for an autograph.
    let asked = |sim: &mut Sim, r: Option<pw_world::renown::Renown>| -> usize {
        match r {
            Some(r) => sim.world.renown.people.insert(who, r),
            None => sim.world.renown.people.remove(&who),
        };
        (0..2000).filter(|&i| pw_sim::chronicle::spotted(&sim.world, who, p, d0.add_days(i), &[]).is_some()).count()
    };
    assert_eq!(asked(&mut sim, None), 0, "nobody stops someone nobody has heard of");
    assert_eq!(asked(&mut sim, Some(renowned(1500, 1500))), 0, "nor someone known only to a few");
    let (some, many) = (asked(&mut sim, Some(renowned(5000, 0))), asked(&mut sim, Some(renowned(9500, 0))));
    assert!(some > 0 && many > some, "rarer for the less known: {some} against {many} in 2000 days");
    assert!(many < 120, "never every day: {many} in 2000 days");
    // On the way to join the national squad: the photographers, more often for the famous, never for the unknown.
    let nation = sim.world.people[who].nation;
    let trip = |d: Date| pw_world::event::Event {
        id: EventId(0),
        date: d,
        vis: pw_world::event::Visibility::Public,
        kind: pw_world::EventKind::NationalSquad { player: p, nation, level: pw_world::intl::Level::Senior },
        causes: Default::default(),
    };
    let photos = |sim: &mut Sim, wide: u16| -> usize {
        sim.world.renown.people.insert(who, renowned(0, wide));
        (0..400).filter(|&i| matches!(pw_sim::chronicle::spotted(&sim.world, who, p, d0.add_days(i), &[trip(d0.add_days(i))]), Some((Spot::SquadTrip, _, n)) if n == nation)).count()
    };
    let (none, few, lots) = (photos(&mut sim, 1000), photos(&mut sim, 4000), photos(&mut sim, 9500));
    assert_eq!(none, 0);
    assert!(few > 0 && lots > few, "{few} against {lots} trips photographed of 400");
    // The same day asks the same question and gets the same answer.
    let again = pw_sim::chronicle::spotted(&sim.world, who, p, d0.add_days(3), &[trip(d0.add_days(3))]);
    assert_eq!(again, pw_sim::chronicle::spotted(&sim.world, who, p, d0.add_days(3), &[trip(d0.add_days(3))]));
}

#[test]
fn the_first_autograph_and_the_first_airport_photo_are_told_once_each_and_someone_hears_of_it() {
    use pw_world::chat::Said;
    let (mut sim, _p, who) = chronicled(47);
    for _ in 0..200 {
        sim.world.renown.people.insert(who, renowned(9800, 9800));
        sim.run(1);
    }
    let spots: Vec<pw_world::chronicle::Spot> = lines(&sim.world, who).into_iter().filter_map(|l| if let Line::Spotted { spot, .. } = l { Some(spot) } else { None }).collect();
    assert!(!spots.is_empty(), "two hundred days as the best-known player in the land and nobody asked for an autograph");
    for (i, a) in spots.iter().enumerate() {
        assert!(!spots[i + 1..].iter().any(|b| b.same_kind(*a)), "each kind of moment is told once: {spots:?}");
    }
    let inbox = &sim.world.ext.chats.of[&who];
    let heard = inbox.chats.iter().flat_map(|c| c.msgs.iter()).filter(|m| matches!(m.said, Said::Spotted { .. })).count();
    assert!(heard >= 1, "the family or the squad hears of it");
    // Teasing about it after the first time stays rare: a few months apart at least.
    let firsts: Vec<Date> = sim.world.ext.chronicle.of(who).unwrap().entries.iter().filter(|e| matches!(e.line, Line::Spotted { .. })).map(|e| e.date).collect();
    let mut again: Vec<Date> = inbox.chats.iter().flat_map(|c| c.msgs.iter()).filter(|m| matches!(m.said, Said::Spotted { .. }) && !firsts.contains(&m.date)).map(|m| m.date).collect();
    again.sort();
    assert!(again.windows(2).all(|x| x[0].days_until(x[1]) > 120), "{again:?}");
}

#[test]
fn a_move_abroad_brings_a_place_to_live_the_family_deciding_and_a_clock_hours_apart() {
    use pw_world::chat::{Room, Said};
    let (mut sim, p, who) = chronicled(48);
    // A day first, so the chats are open before the move (they start from what happens after they open).
    sim.run(1);
    let w = &mut sim.world;
    let seller = w.players.hot[p].club;
    let own = w.people[who].nation;
    let lives = w.lives[who].home;
    let buyer = w.clubs.ids().find(|&k| k != seller && w.clubs[k].nation != own && w.clubs[k].nation != lives && w.clubs[k].first_team().is_some()).expect("a club in another country");
    let there = w.clubs[buyer].nation;
    // The family at home, five hours behind the new club's country.
    w.lives[who].household.parents.alive = 2;
    w.lives[who].household.parents.nation = own;
    let tz = w.nations[own].env.tz;
    w.nations[there].env.tz = tz + 5;
    let contract = pw_sim::market::new_contract(w, p, buyer, 1.0);
    pw_sim::market::execute_transfer(w, p, buyer, seller, 0, contract);
    sim.run(1);
    let w = &sim.world;
    assert_eq!(pw_world::chat::home_ahead(w, who).map(|x| x.0), Some(-5), "home is five hours behind");
    let chats = &w.ext.chats.of[&who].chats;
    let family: Vec<Said> = chats.iter().filter(|c| c.room == Room::Family).flat_map(|c| c.msgs.iter().map(|m| m.said)).collect();
    assert!(family.iter().any(|s| matches!(s, Said::FamilyMove { abroad: true, .. })), "the family decides whether someone comes: {family:?}");
    // A partner, if there is one, says whether they come too.
    if let Some(pt) = w.lives[who].household.partner {
        let said: Vec<Said> = chats.iter().filter(|c| c.room == Room::Direct { with: pt.person }).flat_map(|c| c.msgs.iter().map(|m| m.said)).collect();
        assert!(said.iter().any(|s| matches!(s, Said::PartnerMove { .. })), "{said:?}");
    }
    assert!(lines(w, who).iter().any(|l| matches!(l, Line::Life { kind: pw_world::event::LifeEventKind::Relocated { nation } } if *nation == there)));
    sim.run(12);
    let places: Vec<Line> = lines(&sim.world, who).into_iter().filter(|l| matches!(l, Line::NewPlace { .. })).collect();
    assert_eq!(places.len(), 1, "where you live, told once: {places:?}");
    assert!(matches!(places[0], Line::NewPlace { club, .. } if club == buyer));
    sim.run(20);
    assert_eq!(lines(&sim.world, who).iter().filter(|l| matches!(l, Line::NewPlace { .. })).count(), 1, "not again");
}
