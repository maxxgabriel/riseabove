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
