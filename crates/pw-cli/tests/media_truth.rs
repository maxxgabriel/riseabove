//! Media truth, intent and relationships (locked design 2.2-2.4, 2.13-2.21): what a story was, why it was written and why it was given
//! are kept apart; relationships are directional and remember why; access, tone and rivalry follow from them.

use pw_core::{ClubId, PersonId, StoryId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, mediarel};
use pw_world::media::{BondCause, FeudCause, Intent, Party, SourceAim, Truth};
use pw_world::{StoryKind, World};

fn world() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 81, Scale::SMALL));
    sim.run(300);
    sim
}

/// A story with a subject and a journalist, to turn into whatever a test needs.
fn a_story(w: &World) -> StoryId {
    w.media.stories.iter().find(|s| s.person.is_some() && s.club.is_some() && s.outlet.is_some() && !w.media.journalists.contains_key(&s.person)).map(|s| s.id).expect("a story about a person")
}

#[test]
fn every_story_says_what_it_was_and_the_categories_stay_consistent() {
    let sim = world();
    let w = &sim.world;
    let n = w.media.stories.len();
    assert!(n > 200, "the press was busy: {n}");
    let (mut accurate, mut other) = (0, 0);
    for s in w.media.stories.iter() {
        match s.truth {
            Truth::False => assert!(!s.grounded, "a false story was not so when it ran"),
            Truth::Accurate | Truth::AccurateAtTime | Truth::Misleading => assert!(s.grounded, "{:?} is made of true facts", s.truth),
            Truth::Manipulated => assert!(s.leaker.is_some() || s.info != u32::MAX, "a planted story came from someone"),
        }
        if s.aim != SourceAim::Genuine {
            assert!(s.info != u32::MAX && s.leaker.is_some(), "a source's aim needs a source");
        }
        if s.truth == Truth::Accurate { accurate += 1 } else { other += 1 }
    }
    assert!(accurate > 0 && other > 0, "the press is neither perfect nor hopeless: {accurate} accurate, {other} not");
    // Public record stories are not planted, and rumours are the ones that turn out false.
    assert!(w.media.stories.iter().filter(|s| s.kind == StoryKind::MatchReport).all(|s| s.truth != Truth::Manipulated));
}

#[test]
fn the_writers_reasons_follow_the_relationship_and_the_outlet_not_the_facts() {
    let mut sim = world();
    let w = &mut sim.world;
    let id = a_story(w);
    let mut s = w.media.stories[id].clone();
    let (j, subject) = (Party::Person(s.journalist), Party::Person(s.person));
    s.leaker = PersonId::NONE;
    // With nothing between them and a sober outlet: information.
    w.media.bonds.clear();
    let o = s.outlet;
    w.media.outlets[o].sensationalism = 3;
    s.tone = -40;
    assert_eq!(mediarel::intent_for(w, &s, 2.0), Intent::Inform);
    // A grievance: a harsh piece is to punish.
    mediarel::adjust(w, j, subject, BondCause::RefusedInterview, StoryId::NONE, -5, -30, -5, 40);
    assert_eq!(mediarel::intent_for(w, &s, 2.0), Intent::Punish);
    // A friend: a warm piece is a favour.
    w.media.bonds.clear();
    mediarel::adjust(w, j, subject, BondCause::Exclusive, StoryId::NONE, 5, 40, 20, 0);
    s.tone = 45;
    assert_eq!(mediarel::intent_for(w, &s, 2.0), Intent::Favour);
    // A tabloid dressing the same facts up louder: to draw a crowd.
    w.media.bonds.clear();
    w.media.outlets[o].sensationalism = 18;
    assert_eq!(mediarel::intent_for(w, &s, 20.0), Intent::Engage);
}

#[test]
fn coverage_leaves_a_directional_record_of_who_did_what_and_why() {
    let mut sim = world();
    let w = &mut sim.world;
    w.media.bonds.clear();
    let id = a_story(w);
    let (jn, subject, club, outlet) = {
        let s = &w.media.stories[id];
        (s.journalist, s.person, s.club, s.outlet)
    };
    let (j, sp) = (Party::Person(jn), Party::Person(subject));

    // A harsh piece that was false: the subject holds a grudge and thinks less of the writer; the club cools on the outlet.
    {
        let s = &mut w.media.stories[id];
        s.tone = -60;
        s.truth = Truth::False;
    }
    mediarel::on_story(w, id);
    let b = mediarel::bond(w, sp, j).expect("he now has a view of the journalist");
    assert!(b.grudge > 0 && b.warmth < 0 && b.respect < 0, "{b:?}");
    assert_eq!(b.history.last().map(|r| r.cause), Some(BondCause::FalseStory), "and it remembers why");
    assert!(mediarel::bond(w, Party::Club(club), Party::Outlet(outlet)).is_some_and(|c| c.warmth < 0));
    // It is a one-way relationship: the journalist has no such record of him.
    assert!(mediarel::bond(w, j, sp).is_none());

    // A harsh piece that was true: disliked, and respected for being right (section 2.20).
    w.media.bonds.clear();
    {
        let s = &mut w.media.stories[id];
        s.truth = Truth::Accurate;
    }
    mediarel::on_story(w, id);
    let b = mediarel::bond(w, sp, j).unwrap();
    assert!(b.warmth < 0 && b.respect > 0, "professionally respected, publicly disliked: {b:?}");

    // The same offence twice in two years lands harder the second time.
    w.media.bonds.clear();
    {
        let s = &mut w.media.stories[id];
        s.truth = Truth::False;
    }
    mediarel::on_story(w, id);
    let first = mediarel::bond(w, sp, j).unwrap().grudge;
    mediarel::on_story(w, id);
    let second = mediarel::bond(w, sp, j).unwrap().grudge;
    assert!(second - first > first, "an old grievance reignites: {first} then +{}", second - first);
    assert!(mediarel::bond(w, sp, j).unwrap().history.len() <= 4, "the history is bounded");
}

#[test]
fn a_grudge_closes_doors_and_bends_the_pen_and_time_softens_moods_before_grudges() {
    let mut sim = world();
    let w = &mut sim.world;
    w.media.bonds.clear();
    let id = a_story(w);
    let (jn, subject) = (w.media.stories[id].journalist, w.media.stories[id].person);
    let (j, sp) = (Party::Person(jn), Party::Person(subject));

    let grants = |w: &World| (0..60).filter(|&d| { let mut x = w.clone(); x.date = w.date.add_days(d); mediarel::grants_access(&x, subject, jn) }).count();
    let open = grants(w);
    mediarel::adjust(w, sp, j, BondCause::FalseStory, id, -10, -40, -10, 90);
    let shut = grants(w);
    assert!(open >= 54 && shut <= 20, "{open} interviews granted with no history, {shut} with a grudge");

    // Writes about him more harshly, and about a friend more kindly.
    assert_eq!(mediarel::tone_bias(w, jn, subject), 0.0, "he has no view of the subject yet");
    mediarel::adjust(w, j, sp, BondCause::RefusedInterview, id, -5, -30, -5, 50);
    assert!(mediarel::tone_bias(w, jn, subject) < -10.0);
    w.media.bonds.remove(&(j, sp));
    mediarel::adjust(w, j, sp, BondCause::Exclusive, id, 5, 50, 30, 0);
    assert!(mediarel::tone_bias(w, jn, subject) > 5.0);

    // A club that has frozen an outlet out: its people stop talking to it.
    w.media.bonds.clear();
    let club = w.club_of_person(subject);
    if club.is_some() {
        let outlet = w.media.journalists[&jn].outlet;
        mediarel::adjust(w, Party::Club(club), Party::Outlet(outlet), BondCause::FalseStory, id, 0, -80, 0, 60);
        assert!(grants(w) <= 40, "{}", grants(w));
    }

    // Time: moods fade faster than grudges.
    w.media.bonds.clear();
    mediarel::adjust(w, sp, j, BondCause::FalseStory, id, -10, -40, -10, 60);
    let before = mediarel::bond(w, sp, j).unwrap().clone();
    w.date = pw_core::Date::from_ymd(2027, 3, 2);
    for _ in 0..6 {
        mediarel::weekly(w);
    }
    let after = mediarel::bond(w, sp, j).unwrap();
    let mood = f32::from(after.warmth - before.warmth) / f32::from(-before.warmth);
    let grudge = f32::from(before.grudge - after.grudge) / f32::from(before.grudge);
    assert!(mood > grudge * 2.0, "warmth recovers {mood:.2}, the grudge only {grudge:.2}");
}

#[test]
fn journalists_fall_out_over_scoops_and_exposure_and_feuds_cool_and_flare() {
    let mut sim = world();
    let w = &mut sim.world;
    w.media.feuds.clear();
    let js: Vec<PersonId> = w.media.journalists.keys().copied().take(2).collect();
    let (a, b) = if js[0] < js[1] { (js[0], js[1]) } else { (js[1], js[0]) };
    mediarel::heat(w, a, b, FeudCause::Scooped, 30);
    assert_eq!(w.media.feuds.len(), 1);
    let f0 = w.media.feuds[0].heat;
    assert_eq!(f0, 30);
    // It cools.
    w.date = w.date.add_days(1);
    for _ in 0..10 {
        w.date = w.date.add_days(7);
        mediarel::weekly(w);
    }
    assert!(w.media.feuds.first().map_or(0, |f| f.heat) < f0, "time cools it");
    // Quiet for months, then something new: it flares faster than it began.
    let cooled = w.media.feuds.first().map_or(0, |f| f.heat);
    if let Some(f) = w.media.feuds.first_mut() {
        f.last = w.date.add_days(-200);
    }
    mediarel::heat(w, b, a, FeudCause::Discredited, 8);
    let after = w.media.feuds[0].heat;
    assert!(after >= cooled + 16, "a dormant feud flares twice as fast: {cooled} -> {after}");
    assert!(w.media.feuds[0].causes.len() <= 4);
    assert_eq!(w.media.feuds.len(), 1, "one feud per pair, whichever way round");
}

#[test]
fn being_first_on_a_story_makes_enemies_of_those_who_were_not() {
    let mut sim = world();
    let w = &mut sim.world;
    w.media.feuds.clear();
    // A thread with a story already on it from one journalist; another outlet follows within a few days.
    let (first_id, thread) = w.media.stories.iter().find(|s| s.thread != u32::MAX).map(|s| (s.id, s.thread)).expect("a running story");
    let first = w.media.stories[first_id].clone();
    let other_journalist = w.media.journalists.iter().find(|&(&p, j)| p != first.journalist && j.outlet != first.outlet && j.outlet.is_some()).map(|(&p, j)| (p, j.outlet)).unwrap();
    let mut second = first.clone();
    second.id = w.media.stories.next_id();
    second.journalist = other_journalist.0;
    second.outlet = other_journalist.1;
    second.date = first.date.add_days(2);
    second.claim_type = pw_world::media::ClaimType::Report;
    let sid = second.id;
    w.media.stories.push(second);
    w.media.threads[thread as usize].stories.retain(|&x| x == first_id);
    w.media.threads[thread as usize].stories.push(sid);
    mediarel::scooped(w, sid);
    assert!(w.media.feuds.iter().any(|f| f.causes.iter().any(|c| c.0 == FeudCause::Scooped) && ((f.a == other_journalist.0 && f.b == first.journalist) || (f.b == other_journalist.0 && f.a == first.journalist))), "the one beaten to it resents the other");
}

#[test]
fn a_run_of_the_world_builds_relationships_and_keeps_them_bounded() {
    let sim = world();
    let w = &sim.world;
    assert!(!w.media.bonds.is_empty(), "coverage leaves relationships behind");
    assert!(w.media.bonds.values().all(|b| b.history.len() <= 4 && (0..=100).contains(&b.grudge)));
    assert!(w.media.bonds.keys().any(|(from, to)| matches!((from, to), (Party::Person(_), Party::Person(_)))), "person to journalist");
    assert!(w.media.bonds.keys().any(|(from, to)| matches!((from, to), (Party::Club(_), Party::Outlet(_)))), "club to outlet");
    // Nobody has a relationship with themselves, and a club's view of an outlet is not the outlet's view of the club.
    assert!(w.media.bonds.keys().all(|(a, b)| a != b));
    let _ = ClubId::NONE;
}
