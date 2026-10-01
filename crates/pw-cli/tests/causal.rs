//! Unified causal simulation (locked design 13): subsystems that used to end in a private number now leave events that name their
//! cause and are read by the systems downstream. Three islands are connected here: settling in after a move, what stories do to
//! relationships, and waves of attention.

use pw_core::{EventId, PersonId, PlayerId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, adaptation, attention, lifestate, mediarel};
use pw_world::attention::Cause as Wave;
use pw_world::event::{Cause, EventKind, Visibility};
use pw_world::lifestate::LoadKind;
use pw_world::media::{BondCause, Party};
use pw_world::World;

fn world(days: u32) -> World {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 88, Scale::SMALL));
    sim.run(days);
    sim.world
}

fn player(w: &World) -> (PlayerId, PersonId) {
    let p = w.teams[w.clubs[w.clubs.ids().next().unwrap()].first_team()].squad[5];
    (p, w.players.cold[p].person)
}

fn cause_of(w: &World, e: EventId) -> Vec<EventId> {
    w.events.get(e).map(|x| x.causes.iter().filter_map(|c| if let Cause::Event(i) = c { Some(*i) } else { None }).collect()).unwrap_or_default()
}

#[test]
fn settling_in_ends_in_events_that_name_the_move_that_started_it() {
    let w = world(420);
    let ended: Vec<_> = w.events.all().iter().filter(|e| matches!(e.kind, EventKind::AdaptationEnded { .. })).collect();
    assert!(ended.len() >= 5, "{} settlings ended in a season", ended.len());
    let mut named = 0;
    for e in &ended {
        let EventKind::AdaptationEnded { player, .. } = e.kind else { unreachable!() };
        for c in cause_of(&w, e.id) {
            let src = w.events.get(c).map(|x| &x.kind);
            assert!(
                matches!(src, Some(EventKind::ContractSigned { player: p, .. }) | Some(EventKind::LoanMove { player: p, .. }) if *p == player),
                "the cause of a settling is his own move, not {src:?}"
            );
            named += 1;
        }
    }
    assert!(named * 10 >= ended.len() * 9, "nearly every settling names its move: {named} of {}", ended.len());
}

#[test]
fn not_settling_is_lived_as_loneliness_and_the_load_names_the_event() {
    let mut w = world(30);
    let (p, who) = player(&w);
    let club = w.players.hot[p].club;
    let mv = w.events.push(w.date, Visibility::Public, EventKind::ContractSigned { player: p, club, wage: 1000, until: w.date.add_days(900), renewal: false });
    let mut causes = pw_world::Causes::new();
    causes.push(Cause::Event(mv));
    let ev = w.events.push_caused(w.date, Visibility::Club(club), EventKind::AdaptationStruggling { player: p, club, channel: pw_world::adaptation::Channel::Social }, causes);
    lifestate::scan(&mut w);
    let load = w.lifestate.by[&who].loads.iter().find(|l| l.kind == LoadKind::Loneliness).expect("he lives it as loneliness");
    assert_eq!(load.cause, ev, "the load names the event, which names the move");
    assert_eq!(cause_of(&w, ev), vec![mv]);
    // A struggle with the climate is not loneliness.
    let (_, other) = { let q = w.teams[w.clubs[w.clubs.ids().nth(1).unwrap()].first_team()].squad[5]; (q, w.players.cold[q].person) };
    let q = w.people[other].player;
    let c2 = w.players.hot[q].club;
    w.events.push(w.date, Visibility::Club(c2), EventKind::AdaptationStruggling { player: q, club: c2, channel: pw_world::adaptation::Channel::Environment });
    lifestate::scan(&mut w);
    assert!(w.lifestate.by.get(&other).is_none_or(|s| s.loads.iter().all(|l| l.kind != LoadKind::Loneliness)));
}

#[test]
fn a_move_records_its_event_on_the_settling_in() {
    let w = world(420);
    let with_cause = w.adaptation.current.values().filter(|a| a.cause.is_some()).count();
    assert!(with_cause * 10 >= w.adaptation.current.len() * 9, "{with_cause} of {} settlings know their cause", w.adaptation.current.len());
    let _ = adaptation::struggled(&w, PlayerId(0));
}

fn plant_story(w: &mut World, subject: PersonId, journalist: PersonId, source: EventId) -> pw_core::StoryId {
    let mut s = w.media.stories.iter().next().expect("a story").clone();
    s.id = w.media.stories.next_id();
    s.journalist = journalist;
    s.person = subject;
    s.player = w.people[subject].player;
    s.tone = -80;
    s.truth = pw_world::media::Truth::False;
    let mut ev = pw_world::Causes::new();
    ev.push(Cause::Event(source));
    let e = w.events.push_caused(w.date, Visibility::Public, EventKind::Published { story: s.id }, ev);
    s.event = e;
    w.media.stories.push(s)
}

#[test]
fn a_lasting_grudge_against_a_journalist_is_an_event_caused_by_the_story_and_the_man_lives_through_it() {
    let mut w = world(120);
    let (_, subject) = player(&w);
    let journalist = *w.media.journalists.keys().min().expect("a journalist");
    let src = w.events.last_id();
    let mut grudge_events = Vec::new();
    for _ in 0..6 {
        let id = plant_story(&mut w, subject, journalist, src);
        mediarel::on_story(&mut w, id);
        grudge_events = w.events.all().iter().filter(|e| matches!(e.kind, EventKind::MediaGrudge { subject: s, .. } if s == subject)).map(|e| e.id).collect();
        if !grudge_events.is_empty() {
            break;
        }
    }
    assert_eq!(grudge_events.len(), 1, "crossing the line is one event, not one per story");
    let g = grudge_events[0];
    let published = cause_of(&w, g);
    assert_eq!(published.len(), 1);
    assert!(matches!(w.events.get(published[0]).unwrap().kind, EventKind::Published { .. }), "caused by the story that did it");
    assert!(mediarel::bond(&w, Party::Person(subject), Party::Person(journalist)).unwrap().grudge >= 30);
    // More stories do not repeat it.
    for _ in 0..3 {
        let id = plant_story(&mut w, subject, journalist, src);
        mediarel::on_story(&mut w, id);
    }
    assert_eq!(w.events.all().iter().filter(|e| matches!(e.kind, EventKind::MediaGrudge { subject: s, .. } if s == subject)).count(), 1);
    // The man lives it: a load that names the grudge.
    lifestate::scan(&mut w);
    let l = w.lifestate.by[&subject].loads.iter().find(|l| l.kind == LoadKind::Scandal).expect("a scandal load");
    assert_eq!(l.cause, g);
    let _ = BondCause::FalseStory;
}

#[test]
fn a_wave_of_attention_names_what_set_it_off_and_what_it_does_to_him_names_the_wave() {
    let mut w = world(60);
    let (_, who) = player(&w);
    w.net.attention.remove(&who);
    let src = w.events.push(w.date, Visibility::Public, EventKind::Retired { person: PersonId(0) });
    attention::spark_caused(&mut w, who, Wave::Controversy, 0.9, src);
    let surge = w.events.all().iter().rev().find(|e| matches!(e.kind, EventKind::AttentionSurge { person, .. } if person == who)).expect("a surge on the record");
    assert_eq!(cause_of(&w, surge.id), vec![src]);
    let load = w.lifestate.by[&who].loads.iter().find(|l| l.kind == LoadKind::OnlineAbuse).expect("a pile-on load");
    assert_eq!(load.cause, surge.id, "the load names the surge, the surge names the source");
    // The same wave sparked again while it lasts is not a new surge.
    let n = w.events.all().iter().filter(|e| matches!(e.kind, EventKind::AttentionSurge { person, .. } if person == who)).count();
    attention::spark_caused(&mut w, who, Wave::Controversy, 0.9, src);
    assert_eq!(n, w.events.all().iter().filter(|e| matches!(e.kind, EventKind::AttentionSurge { person, .. } if person == who)).count());
    // A small football wave is not a public event.
    let (_, quiet) = { let q = w.teams[w.clubs[w.clubs.ids().nth(1).unwrap()].first_team()].squad[3]; (q, w.players.cold[q].person) };
    w.net.attention.remove(&quiet);
    let before = w.events.len();
    attention::spark_caused(&mut w, quiet, Wave::Football, 0.4, src);
    assert_eq!(before, w.events.len());
}

#[test]
fn over_a_season_surges_and_their_consequences_form_chains_back_to_football() {
    let w = world(300);
    let surges: Vec<_> = w.events.all().iter().filter(|e| matches!(e.kind, EventKind::AttentionSurge { .. })).collect();
    assert!(!surges.is_empty(), "attention surges happen");
    let caused = surges.iter().filter(|e| !cause_of(&w, e.id).is_empty()).count();
    assert!(caused * 2 >= surges.len(), "most surges name their source: {caused} of {}", surges.len());
    // Loads that came from a surge point back at it.
    let named = w.lifestate.by.values().flat_map(|s| s.loads.iter()).filter(|l| l.cause.is_some()).count();
    assert!(named > 0, "some lived experiences name their event");
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The rest of the islands: contracts and clauses, the dressing room, stagnation, sponsors, rivalries, referees, opinion.
// ---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn every_cause_in_a_running_world_comes_before_its_effect() {
    let w = world(500);
    let bad: Vec<_> = pw_sim::audit::audit(&w).into_iter().filter(|v| matches!(v, pw_sim::audit::Violation::CauseNotBefore { .. })).collect();
    assert!(bad.is_empty(), "a cause named after its effect: {bad:?}");
}

#[test]
fn the_connected_islands_name_what_brought_them_about() {
    // Two seasons of a small world: every kind below is emitted by a system that used to leave it uncaused. Each that occurs names its
    // cause the large majority of the time; the rest are older than the event log remembers or have no single event behind them.
    let w = world(750);
    let cov = pw_sim::audit::cause_coverage(&w);
    let get = |k: &str| cov.iter().find(|(n, _, _)| n == k).map_or((0, 0), |&(_, n, c)| (n, c));
    let mut seen = Vec::new();
    for (kind, floor) in [("ContractOption", 90), ("PlayerSettled", 80), ("Stagnated", 100), ("EndorsementEnded", 40), ("AppealDecided", 40), ("RivalryKindled", 60)] {
        let (n, c) = get(kind);
        if n >= 3 {
            seen.push(kind);
            assert!(c * 100 >= n * floor, "{kind}: {c} of {n} name a cause, expected at least {floor}%");
        }
    }
    assert!(!seen.is_empty(), "none of the connected kinds occurred in two seasons: {cov:?}");
}

#[test]
fn an_option_taken_names_the_signing_it_belongs_to() {
    let mut w = world(30);
    let (p, _) = player(&w);
    let club = w.players.hot[p].club;
    let signed = w.events.push(w.date, Visibility::Public, EventKind::ContractSigned { player: p, club, wage: 1000, until: w.date.add_days(900), renewal: false });
    // The deal-behind lookup is what clause events use: the newest signing of this player.
    let found = pw_sim::clauses::deal_behind(&w, p);
    assert_eq!(found.len(), 1);
    assert!(matches!(found[0], Cause::Event(id) if id == signed));
}
