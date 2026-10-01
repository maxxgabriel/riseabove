//! QA: the registries that used to grow with every year (information items, incidents, meetings, the event log) forget what nothing
//! needs any more, keep addressing stable ids, and a saved world reloads and continues exactly like the world that was never saved.
mod qa_common;

use pw_import::synthetic::Scale;
use qa_common::*;

/// Long enough for every window to have started forgetting (the longest is three years).
const DAYS: u32 = 4 * 365 + 60;

#[test]
fn old_information_incidents_and_meetings_are_forgotten_and_ids_stay_stable() {
    let s = ran(Scale::TINY, 11, DAYS);
    let w = &s.world;
    assert!(w.grapevine.items.base() > 0, "no information item was forgotten in four years");
    assert!(w.incidents.list.base() > 0, "no incident was forgotten in four years");
    assert!(w.meetings.list.base() > 0, "no meeting was forgotten in four years");
    // Ids are never reused: the next id is everything ever issued, and what is held is only the recent part.
    assert_eq!(w.grapevine.items.len(), w.grapevine.items.base() as usize + w.grapevine.items.held());
    assert!(w.grapevine.items.held() < w.grapevine.items.len());
    // Everything that points at an information item, an incident or a meeting either still resolves or points at a forgotten id.
    for (person, ids) in &w.grapevine.by_person {
        for &id in ids {
            assert!(w.grapevine.items.get(id as usize).is_some_and(|it| it.knows(*person)), "{person:?} is indexed under item {id}, which is gone or does not list them");
        }
    }
    for &id in &w.grapevine.active {
        assert!(w.grapevine.items.get(id as usize).is_some(), "active item {id} was forgotten");
    }
    for t in &w.grapevine.tells {
        assert!(w.grapevine.items.get(t.info as usize).is_some(), "a telling refers to forgotten item {}", t.info);
    }
    for i in w.incidents.list.iter() {
        assert!(w.incidents.get(i.id).is_some());
    }
    for &(id, ..) in &w.incidents.deferred {
        assert!(w.incidents.get(id).is_some(), "a deferred decision points at forgotten incident {id}");
    }
    for &(_, incident) in w.incidents.investigations.values() {
        assert!(w.incidents.get(incident).is_some(), "an open investigation points at forgotten incident {incident}");
    }
    for (_, m) in w.meetings.pending() {
        assert!(w.meetings.list.get(m.id).is_some());
    }
    // A forgotten information item reads as one nobody knows and nobody is telling; it never panics.
    let gone = w.grapevine.get(0);
    assert!(gone.holders.is_empty() && gone.closed);
    // The world is still sound.
    assert!(pw_sim::validate::problems(w).is_empty(), "{:?}", pw_sim::validate::problems(w));
    assert!(pw_sim::audit::audit(w).is_empty(), "{:?}", pw_sim::audit::audit(w));
}

#[test]
fn a_world_that_has_forgotten_things_reloads_and_continues_identically() {
    let mut a = ran(Scale::TINY, 12, DAYS);
    assert!(a.world.grapevine.items.base() > 0 && a.world.incidents.list.base() > 0 && a.world.meetings.list.base() > 0);
    let p = temp_path("retention");
    pw_sim::save::save_with(&a.world, &p, &pw_sim::save::Info::of_world(&a.world)).unwrap();
    let back = pw_sim::save::load_world(&p).unwrap();
    cleanup(&p);
    // The forgotten prefixes are recovered from the saved rows.
    assert_eq!(back.grapevine.items.base(), a.world.grapevine.items.base());
    assert_eq!(back.grapevine.items.len(), a.world.grapevine.items.len());
    assert_eq!(back.incidents.list.base(), a.world.incidents.list.base());
    assert_eq!(back.incidents.list.len(), a.world.incidents.list.len());
    assert_eq!(back.meetings.list.base(), a.world.meetings.list.base());
    assert_eq!(back.meetings.list.len(), a.world.meetings.list.len());
    assert_eq!(digest(&a.world), digest(&back), "a reloaded world differs from the one saved");
    let mut b = pw_sim::Sim::new(back);
    a.run(200);
    b.run(200);
    assert_eq!(digest(&a.world), digest(&b.world), "a reloaded world diverged from the uninterrupted one after forgetting: {}", explain_divergence(&a.world, &b.world));
}

/// A world reloaded at any age continues exactly like the one that was never stopped, day by day for a hundred days. This is the check
/// that found the systems that walked an unordered map and recorded events in its order (growth, interpretation, captains).
#[test]
#[ignore = "heavy: about a minute in the test profile (tiny worlds of 4, 8.5, 10 and 11.5 years, each continued day by day)"]
fn a_reload_at_any_age_continues_identically_day_by_day() {
    for days in [1500u32, 3100, 3600, 4200] {
        let mut a = ran(Scale::TINY, 12, days);
        let p = temp_path("retention2");
        pw_sim::save::save_with(&a.world, &p, &pw_sim::save::Info::of_world(&a.world)).unwrap();
        let back = pw_sim::save::load_world(&p).unwrap();
        cleanup(&p);
        let mut b = pw_sim::Sim::new(back);
        for d in 0..100 {
            a.run(1);
            b.run(1);
            if digest(&a.world) != digest(&b.world) {
                let first = a.world.events.all().iter().zip(b.world.events.all().iter()).find(|(e, f)| format!("{:?}{:?}", e.kind, e.causes) != format!("{:?}{:?}", f.kind, f.causes));
                panic!("reloaded at day {days}, the worlds differ {d} days on ({}); first differing event: {:?}", a.world.date, first.map(|(e, f)| (&e.kind, &f.kind)));
            }
        }
    }
}

#[test]
fn nothing_is_kept_past_its_window() {
    use pw_world::EventKind as E;
    let s = ran(Scale::TINY, 13, 6 * 365);
    let w = &s.world;
    let today = w.date;
    let age = |d: pw_core::Date| d.days_until(today);
    // Information items are forgotten two years after they were created (monthly pass, so a month of slack).
    let oldest_item = w.grapevine.items.iter().map(|it| age(it.date)).max().unwrap_or(0);
    assert!(oldest_item <= 730 + 31, "an information item is {oldest_item} days old");
    // Incidents and meetings after three years (yearly pass, so a year of slack); the ones something waits on may stay.
    let waited_on: std::collections::HashSet<u32> = w.incidents.deferred.iter().map(|x| x.0).chain(w.incidents.expecting.iter().map(|x| x.3)).chain(w.incidents.investigations.values().map(|x| x.1)).chain(w.incidents.national.iter().map(|x| x.3)).collect();
    let oldest_incident = w.incidents.list.iter().filter(|i| !waited_on.contains(&i.id)).map(|i| age(i.date)).max().unwrap_or(0);
    assert!(oldest_incident <= 3 * 365 + 365, "an incident nothing waits on is {oldest_incident} days old");
    let pinned: std::collections::HashSet<_> = w.decisions.all.iter().filter_map(|d| if let pw_world::DecisionKind::Meeting { meeting } = d.kind { Some(meeting) } else { None }).collect();
    let oldest_meeting = w.meetings.list.iter().filter(|m| !pinned.contains(&m.id)).map(|m| age(m.date)).max().unwrap_or(0);
    assert!(oldest_meeting <= 3 * 365 + 365, "a meeting is {oldest_meeting} days old");
    // The event log: working detail for a little over a year, what people talk about for two, headlines for good.
    let mut headlines = 0;
    for e in w.events.all() {
        let old = age(e.date);
        let working = matches!(e.kind, E::AgentPitch { .. } | E::Injured { .. } | E::Recovered { .. } | E::Diagnosed { .. } | E::CoachNote { .. } | E::Fined { .. } | E::TalksOpened { .. });
        assert!(!working || old <= 400 + 31, "{:?} is {old} days old", e.kind);
        let talked = matches!(e.kind, E::Meeting { .. } | E::PromiseMade { .. } | E::Published { .. });
        assert!(!talked || old <= 730 + 31, "{:?} is {old} days old", e.kind);
        headlines += usize::from(old > 731 && matches!(e.kind, E::Transfer { .. } | E::Debut { .. } | E::Retired { .. } | E::Champion { .. }));
    }
    // History stays: the old headlines are still there.
    assert!(headlines > 50, "only {headlines} old headline events were kept");
}

/// People who left the game (a computer-run player retired for years, with no job in it) stop producing news and private rumours: their
/// month-by-month life is no longer stepped. Without this a world's retired population grows for ever and each of them keeps adding events.
#[test]
fn people_who_left_the_game_stop_generating_life_events() {
    use pw_world::{EventKind as E, LifeEventKind as L};
    let s = ran(Scale::TINY, 14, 8 * 365);
    let w = &s.world;
    let gone = w.people.ids().filter(|&p| pw_sim::retention::left_the_game(w, p, w.date)).count();
    assert!(gone > 20, "only {gone} people had left the game after eight years");
    // When each person retired, from the log: `left_the_game` reads today's status, so a person who retired after an event (an amateur
    // who stopped playing years after his last professional match) must not be counted as gone at that event.
    let mut retired_on: std::collections::HashMap<pw_core::PersonId, pw_core::Date> = std::collections::HashMap::new();
    for e in w.events.all() {
        if let E::Retired { person } = e.kind {
            retired_on.entry(person).or_insert(e.date);
        }
    }
    let mut after_leaving = Vec::new();
    for e in w.events.all() {
        // The events of a person's own month (money, parents): a partner's step can still mark a shared event such as a wedding.
        if let E::Life { person, kind: kind @ (L::FinancialTrouble | L::ParentUnwell | L::ParentRecovered | L::Bereavement) } = &e.kind {
            let player = w.people[*person].player;
            // `gone` is read at the event's date, for someone who really had played and had retired before it: status is today's, so
            // the retirement itself must predate the event (a player who went years without a match but retired later had not left
            // the game yet). Retirements are never forgotten by the log.
            let retired_before = retired_on.get(person).is_some_and(|&d| d < e.date);
            if player.is_some() && w.players.hot[player].last_match.0 > 0 && retired_before && pw_sim::retention::left_the_game(w, *person, e.date) {
                after_leaving.push((e.date, *person, format!("{kind:?}")));
            }
        }
    }
    assert!(after_leaving.is_empty(), "life events for people who had left the game: {:?}", &after_leaving[..after_leaving.len().min(5)]);
}
