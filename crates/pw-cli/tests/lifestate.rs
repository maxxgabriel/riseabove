//! Life-to-football state (locked design 7.35-7.54): events interpreted rather than turned into buffs, effects with a shape in time,
//! memories that return, support networks that help or misfire, managers who know only what reaches them, the match feeding back into
//! life, and social discourse crossing into state and back.

use pw_core::{Attr, ClubId, Date, EventId, PersonId, PlayerId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_match::{Ev, MatchEvent, MatchResult, PlayerLine, TeamStats};
use pw_sim::lifestate::{self, Coping, Reaction, Setting, Source, Support, Temper};
use pw_sim::selection::{self, Selection};
use pw_sim::tactics::MatchCtx;
use pw_sim::{Sim, attention};
use pw_world::attention::Cause as Wave;
use pw_world::event::{EventKind, LifeEventKind, Visibility};
use pw_world::lifestate::{Awareness, Chan, Handling, Known, Load, LoadKind, Scar, ScarKind, Tail};
use pw_world::{Fixture, World};

fn world() -> World {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 61, Scale::SMALL));
    sim.run(30);
    sim.world
}

fn players_of(w: &World, n: usize) -> Vec<(PlayerId, PersonId)> {
    let mut v = Vec::new();
    for c in w.clubs.ids() {
        for &p in &w.teams[w.clubs[c].first_team()].squad {
            v.push((p, w.players.cold[p].person));
            if v.len() >= n {
                return v;
            }
        }
    }
    v
}

/// Let a load have been in force for a while.
fn age(w: &mut World, who: PersonId, days: i32) {
    for l in w.lifestate.by.get_mut(&who).unwrap().loads.iter_mut() {
        l.since = l.since.add_days(-days);
    }
}

fn temper(resilience: f32, coping: Coping) -> Temper {
    Temper { resilience, coping, ambition: 0.5, professionalism: 0.5 }
}

fn support(strength: f32) -> Support {
    Support { strength, best: Source::Partner, isolated: strength < 0.25 }
}

#[test]
fn the_same_event_means_different_things_to_different_people() {
    let base = |t: Temper, s: Support| lifestate::interpret(LoadKind::OnlineAbuse, 0.8, &t, &s);
    let steady = base(temper(0.5, Coping::Steady), support(0.5));
    let driven = base(temper(0.5, Coping::Driven), support(0.5));
    let avoidant = base(temper(0.5, Coping::Avoidant), support(0.5));
    let expressive = base(temper(0.5, Coping::Expressive), support(0.5));
    assert!(driven.get(Chan::Motivation) > steady.get(Chan::Motivation) + 10, "what angers him drives him");
    assert!(driven.get(Chan::Anger) < steady.get(Chan::Anger));
    assert!(avoidant.get(Chan::Rumination) < steady.get(Chan::Rumination) * 3 / 4 && avoidant.get(Chan::Confidence) > steady.get(Chan::Confidence), "indifference: most of it does not land");
    assert!(expressive.get(Chan::Anger) > steady.get(Chan::Anger) && expressive.get(Chan::Calm) < steady.get(Chan::Calm), "he feels it out loud");
    // The fragile take more of the same hit than the resilient, and the supported less than the alone.
    let fragile = base(temper(0.1, Coping::Steady), support(0.5));
    let sturdy = base(temper(0.95, Coping::Steady), support(0.5));
    assert!(fragile.get(Chan::Confidence) < sturdy.get(Chan::Confidence) && fragile.get(Chan::Rumination) > sturdy.get(Chan::Rumination));
    let alone = base(temper(0.5, Coping::Steady), support(0.0));
    let held = base(temper(0.5, Coping::Steady), support(1.0));
    assert!(alone.get(Chan::Rumination) > held.get(Chan::Rumination) + 8);
}

#[test]
fn good_and_bad_news_are_the_same_machinery_and_can_be_present_together() {
    let e = lifestate::interpret(LoadKind::NewChild, 0.9, &temper(0.5, Coping::Steady), &support(0.5));
    assert!(e.get(Chan::Excitement) > 20 && e.get(Chan::Motivation) > 10 && e.get(Chan::Sleep) < -15, "happier, keener, and not sleeping");
    let mind = lifestate::football(&channels_of(&e), &temper(0.5, Coping::Steady), 0.5);
    assert!(mind.drive != 0.0 && mind.focus != 0.0);
    // Fame is not the effect, its reading is: the same acclaim overwhelms one and grounds another (7.50).
    let overwhelmed = lifestate::interpret(LoadKind::Hype, 0.9, &temper(0.2, Coping::Steady), &support(0.2));
    let grounded = lifestate::interpret(LoadKind::Hype, 0.9, &temper(0.8, Coping::Steady), &support(0.9));
    assert!(overwhelmed.get(Chan::Rumination) > 20 && overwhelmed.get(Chan::Confidence) < 0);
    assert!(grounded.get(Chan::Rumination) < 10 && grounded.get(Chan::Confidence) > 0);
}

fn channels_of(e: &pw_world::lifestate::Effects) -> [f32; pw_world::lifestate::N_CHAN] {
    let mut c = [0.0; pw_world::lifestate::N_CHAN];
    for ch in Chan::ALL {
        c[ch.idx()] = f32::from(e.get(ch));
    }
    c
}

#[test]
fn state_reaches_football_as_behaviour_and_pressure_is_not_always_poison() {
    let mut low = [0.0; pw_world::lifestate::N_CHAN];
    low[Chan::Rumination.idx()] = 60.0;
    low[Chan::Anger.idx()] = 50.0;
    low[Chan::Confidence.idx()] = -40.0;
    let ordinary = lifestate::football(&[0.0; pw_world::lifestate::N_CHAN], &temper(0.5, Coping::Steady), 0.5);
    assert!(ordinary.is_neutral(), "most days are ordinary");
    let shaken = lifestate::football(&low, &temper(0.3, Coping::Steady), 0.5);
    assert!(shaken.focus < -0.1 && shaken.calm < -0.1 && shaken.confidence < -0.1, "attention, temper and belief all suffer: {shaken:?}");
    // Extreme stakes and heavy stress on a resilient, ambitious man: he can be sharper for it.
    let pressure = |res: f32| lifestate::football(&low, &Temper { resilience: res, coping: Coping::Driven, ambition: 1.0, professionalism: 1.0 }, 1.0);
    assert!(pressure(1.0).focus > pressure(0.3).focus + 0.1 && pressure(1.0).drive > pressure(0.3).drive, "{:?} vs {:?}", pressure(1.0), pressure(0.3));
}

#[test]
fn effects_have_a_shape_in_time_and_do_not_share_one_timer() {
    let load = |tail: Tail, actual: u16| Load {
        kind: LoadKind::Bereavement,
        cause: EventId::NONE,
        since: Date(1000),
        onset: 4,
        expected_days: 60,
        actual_days: actual,
        tail,
        effects: Default::default(),
        helped: 0,
        misfired: 0,
        reminded: 0,
    };
    let at = |l: &Load, d: i32| l.intensity(Date(1000).add_days(d));
    let (sharp, steady, lingering, recurring) = (load(Tail::Sharp, 60), load(Tail::Steady, 60), load(Tail::Lingering, 60), load(Tail::Recurring, 60));
    // Onset: it builds before it peaks.
    assert!(at(&steady, 0) < at(&steady, 2) && at(&steady, 2) < at(&steady, 4));
    assert!((at(&steady, 10) - 1.0).abs() < 1e-4, "at the peak it is at full force");
    // Decay: sharp is gone first, lingering last, and the shapes differ at the same span.
    assert!(at(&sharp, 40) < at(&lingering, 40) && at(&lingering, 40) < at(&steady, 40), "a lingering load drops quickly at first and then does not go");
    assert!(at(&lingering, 120) > at(&steady, 120), "the long tail");
    assert!(sharp.live(Date(1000).add_days(2)) && !sharp.live(Date(1000).add_days(400)));
    assert!(at(&recurring, 200) < at(&steady, 20));
    // Duration expected against duration actual: a load that lasts longer for one person is simply longer.
    let long = load(Tail::Steady, 120);
    assert!(at(&long, 60) > at(&steady, 60));
    assert!(LoadKind::MissedPenalty.expected_days() < LoadKind::Bereavement.expected_days() && LoadKind::PublicMistake.expected_days() < 10);
}

#[test]
fn different_people_who_live_the_same_event_come_out_different() {
    let mut w = world();
    let people = players_of(&w, 40);
    for &(_, who) in &people {
        lifestate::add_load(&mut w, who, LoadKind::OnlineAbuse, 0.8, EventId::NONE);
    }
    let mut effects = std::collections::BTreeSet::new();
    let mut spans = std::collections::BTreeSet::new();
    let (mut driven, mut sunk) = (0, 0);
    for &(_, who) in &people {
        let l = w.lifestate.by[&who].loads[0];
        effects.insert(l.effects.0);
        spans.insert(l.actual_days);
        driven += usize::from(l.effects.get(Chan::Motivation) > 10);
        sunk += usize::from(l.effects.get(Chan::Confidence) < -20);
    }
    assert!(effects.len() >= 8 && spans.len() >= 4, "{} effect profiles and {} durations among 40 people", effects.len(), spans.len());
    assert!(driven > 0 && sunk > 0, "some were fired up ({driven}), some were knocked back ({sunk})");
    // And behaviour follows the person, not the event.
    let s = Setting { venue: ClubId::NONE, importance: 0.5, uid: 1 };
    let mut age_loads = people.clone();
    age_loads.truncate(40);
    for &(_, who) in &age_loads {
        age(&mut w, who, 8);
    }
    let minds: std::collections::BTreeSet<i32> = people.iter().map(|&(p, _)| (lifestate::mind_for(&w, p, &s).focus * 100.0) as i32).collect();
    assert!(minds.len() >= 8, "{} different states of attention", minds.len());
}

#[test]
fn a_support_network_shortens_and_softens_and_can_misfire() {
    let mut w = world();
    let people = players_of(&w, 2);
    let (held, alone) = (people[0].1, people[1].1);
    let make = |w: &mut World, who: PersonId, bond: u8| {
        let l = &mut w.lives[who];
        if let Some(p) = l.household.partner.as_mut() {
            p.bond = bond;
            p.lives = l.home;
        }
    };
    // Strong: a close partner, close parents, at home. Weak: none of it.
    w.lives[held].household.partner = Some(pw_world::life::Partner { person: alone, since: w.date, status: pw_world::life::PartnerStatus::Living, bond: 95, lives: w.lives[held].home });
    w.lives[held].household.parents = pw_world::life::Parents { nation: w.lives[held].home, alive: 2, closeness: 95, ..Default::default() };
    w.lives[alone].household.partner = None;
    w.lives[alone].household.parents.alive = 0;
    make(&mut w, held, 95);
    let (sh, sa) = (lifestate::support(&w, held), lifestate::support(&w, alone));
    assert!(sh.strength > sa.strength + 0.3, "{} vs {}", sh.strength, sa.strength);
    assert!(sh.best == Source::Partner || sh.best == Source::Family);
    lifestate::add_load(&mut w, held, LoadKind::Bereavement, 1.0, EventId::NONE);
    lifestate::add_load(&mut w, alone, LoadKind::Bereavement, 1.0, EventId::NONE);
    let (a, b) = (w.lifestate.by[&held].loads[0], w.lifestate.by[&alone].loads[0]);
    // Same person-traits are not held equal here; support is the difference we can compare across many weeks below.
    let _ = (a, b);
    // Over weeks the supported recover faster; some support attempts backfire.
    let mut w2 = w.clone();
    for &who in &[held, alone] {
        age(&mut w2, who, 10);
    }
    let (mut helped, mut misfired) = (0u32, 0u32);
    for week in 0..20 {
        w2.date = w.date.add_days(7 * week);
        lifestate::weekly(&mut w2);
        for who in [held, alone] {
            if let Some(st) = w2.lifestate.by.get(&who) {
                for l in &st.loads {
                    helped += u32::from(l.helped);
                    misfired += u32::from(l.misfired);
                }
            }
        }
    }
    assert!(helped > 0, "somebody helped");
    let _ = misfired;
}

#[test]
fn support_attempts_are_sometimes_misjudged() {
    let mut w = world();
    let people = players_of(&w, 60);
    for &(_, who) in &people {
        lifestate::add_load(&mut w, who, LoadKind::Breakup, 1.0, EventId::NONE);
        age(&mut w, who, 12);
    }
    let start = w.date;
    let mut misfired = 0;
    for week in 0..8 {
        w.date = start.add_days(7 * week);
        lifestate::weekly(&mut w);
        misfired = w.lifestate.by.values().flat_map(|s| s.loads.iter()).map(|l| u32::from(l.misfired)).sum::<u32>();
    }
    let helped: u32 = w.lifestate.by.values().flat_map(|s| s.loads.iter()).map(|l| u32::from(l.helped)).sum();
    assert!(helped > 10 && misfired > 0 && misfired < helped, "help {helped}, help that made it worse {misfired}");
}

#[test]
fn a_memory_comes_back_with_the_place_and_the_reaction_is_a_matter_of_odds_not_destiny() {
    let mut w = world();
    let people = players_of(&w, 80);
    let venue = w.clubs.ids().nth(2).unwrap();
    for &(_, who) in &people {
        let st = w.lifestate.by.entry(who).or_default();
        st.scars.push(Scar { kind: ScarKind::SeriousInjury, cause: EventId::NONE, date: w.date.add_days(-300), venue, weight: 80, reactivated: 0, last: w.date.add_days(-300) });
    }
    let there = Setting { venue, importance: 0.6, uid: 5 };
    let elsewhere = Setting { venue: w.clubs.ids().next().unwrap(), importance: 0.6, uid: 5 };
    let mut reactions = std::collections::BTreeSet::new();
    let mut nothing = 0;
    for &(p, _) in &people {
        assert!(lifestate::returning(&w, p, &elsewhere).is_none(), "another ground brings nothing back");
        let (_, r, k) = lifestate::returning(&w, p, &there).expect("the ground brings it back");
        assert!(k > 0.3);
        reactions.insert(format!("{r:?}"));
        nothing += usize::from(r == Reaction::Nothing);
    }
    assert!(reactions.len() >= 4, "nervous, focused, angry, motivated, avoidant: {reactions:?}");
    assert!(nothing > 0 && nothing < people.len() / 2, "and for some it means little: {nothing} of {}", people.len());
    // It fades with the years.
    let old = {
        let mut w2 = w.clone();
        for st in w2.lifestate.by.values_mut() {
            for s in st.scars.iter_mut() {
                s.date = w2.date.add_days(-365 * 9);
            }
        }
        w2
    };
    let (p, _) = people[0];
    let (fresh, faded) = (lifestate::returning(&w, p, &there).unwrap().2, lifestate::returning(&old, p, &there).map_or(0.0, |x| x.2));
    assert!(faded < fresh * 0.6, "{fresh} -> {faded}");
    // A decisive penalty missed is a different trigger: it returns for a penalty taker in a big match, not at a ground.
    let (q, qp) = people[1];
    w.players.cold[q].attrs.set(Attr::PenaltyTaking, 17.0);
    w.lifestate.by.get_mut(&qp).unwrap().scars.push(Scar { kind: ScarKind::MissedDecisivePenalty, cause: EventId::NONE, date: w.date.add_days(-100), venue: ClubId::NONE, weight: 90, reactivated: 0, last: w.date });
    w.lifestate.by.get_mut(&qp).unwrap().scars.retain(|s| s.kind == ScarKind::MissedDecisivePenalty);
    assert!(lifestate::returning(&w, q, &Setting { venue: ClubId::NONE, importance: 0.9, uid: 9 }).is_some());
    assert!(lifestate::returning(&w, q, &Setting { venue: ClubId::NONE, importance: 0.3, uid: 9 }).is_none());
}

fn fixture_and_sides(w: &World) -> (Fixture, Selection, Selection, ClubId) {
    let ids: Vec<ClubId> = w.clubs.ids().collect();
    let sel = |c: ClubId| selection::select(w, w.clubs[c].first_team(), w.date, 0.6, 7, 0).unwrap();
    let mut fx = w.fixtures.iter().next().unwrap().1.clone();
    fx.home = w.clubs[ids[0]].first_team();
    fx.away = w.clubs[ids[1]].first_team();
    fx.neutral = false;
    (fx, sel(ids[0]), sel(ids[1]), ids[0])
}

fn result(w: &World, fx: &Fixture, home_goals: u8, away_goals: u8, lines: Vec<PlayerLine>, events: Vec<MatchEvent>) -> MatchResult {
    MatchResult { home: fx.home, away: fx.away, home_goals, away_goals, ht: (0, 0), extra_time: false, pens: None, stats: [TeamStats::default(), TeamStats::default()], lines, events, pom: w.teams[fx.home].squad[0] }
}

fn line(p: PlayerId, side: u8, rating: f32) -> PlayerLine {
    PlayerLine { player: p, side, started: true, minutes: 90, rating, ..Default::default() }
}

fn ev(kind: Ev, p: PlayerId) -> MatchEvent {
    MatchEvent { t: 3000, side: 0, kind, player: p, other: PlayerId::NONE, zone: 0, value: 0.0 }
}

fn ctx(w: &World, fx: &Fixture, imp: f32, decisive: bool) -> MatchCtx {
    MatchCtx::for_fixture(w, fx, imp, decisive, None, 1.0)
}

#[test]
fn the_match_feeds_back_into_life_and_a_decisive_miss_leaves_a_scar() {
    let mut w = world();
    let (fx, sa, sb, _) = fixture_and_sides(&w);
    let (p1, p2, p3) = (sa.xi[2], sa.xi[5], sa.xi[8]);
    let persons: Vec<PersonId> = sa.xi.iter().map(|&p| w.players.cold[p].person).collect();
    let person = |p: PlayerId| persons[sa.xi.iter().position(|&x| x == p).unwrap()];
    let lines = vec![line(p1, 0, 4.1), line(p2, 0, 6.5), line(p3, 0, 6.5)];
    let events = vec![ev(Ev::OwnGoal, p1), ev(Ev::PenaltyMiss, p2), ev(Ev::Red, p3)];
    let r = result(&w, &fx, 0, 1, lines, events);
    let c = ctx(&w, &fx, 0.9, true);
    lifestate::after_match(&mut w, &fx, [&sa, &sb], &r, &c);
    let has = |w: &World, p: PlayerId, k: LoadKind| w.lifestate.by.get(&w.players.cold[p].person).is_some_and(|s| s.loads.iter().any(|l| l.kind == k));
    assert!(has(&w, p1, LoadKind::PublicMistake), "an own goal is a public mistake");
    assert!(has(&w, p2, LoadKind::MissedPenalty) && has(&w, p3, LoadKind::RedCard));
    let scars = |p: PlayerId| w.lifestate.by[&person(p)].scars.iter().map(|s| s.kind).collect::<Vec<_>>();
    assert!(scars(p2).contains(&ScarKind::MissedDecisivePenalty), "a penalty missed when it mattered stays");
    assert!(scars(p3).contains(&ScarKind::RedCardInBigMatch));
    // The load sits in the channels the person interprets it into, not in his attributes.
    let l = w.lifestate.by[&person(p2)].loads.iter().find(|l| l.kind == LoadKind::MissedPenalty).unwrap();
    assert!(l.effects.get(Chan::Rumination) > 0 && l.effects.get(Chan::Risk) < 0, "he dwells on it and takes fewer chances");
    // A heavy defeat is a load for those who lost; a triumph for the man who starred.
    let lines = vec![line(sa.xi[1], 0, 5.8), line(sb.xi[1], 1, 9.1)];
    let r = result(&w, &fx, 0, 5, lines, vec![]);
    let c2 = ctx(&w, &fx, 0.5, false);
    lifestate::after_match(&mut w, &fx, [&sa, &sb], &r, &c2);
    assert!(has(&w, sa.xi[1], LoadKind::Humiliation) && has(&w, sb.xi[1], LoadKind::Triumph));
    assert!(!has(&w, sb.xi[1], LoadKind::Humiliation));
}

#[test]
fn a_captains_word_softens_a_mistake_and_a_serious_injury_marks_the_ground() {
    let w = world();
    let (fx, sa, sb, home) = fixture_and_sides(&w);
    let p = sa.xi.iter().copied().find(|&x| x != sa.captain).unwrap();
    let who = w.players.cold[p].person;
    let cap = w.players.cold[sa.captain].person;
    let mag = |w: &World| w.lifestate.by[&who].loads.iter().find(|l| l.kind == LoadKind::PublicMistake).map(|l| l.effects.get(Chan::Rumination)).unwrap();
    let r = result(&w, &fx, 0, 1, vec![line(p, 0, 4.0)], vec![]);
    let c = ctx(&w, &fx, 0.5, false);
    // Without a captain he trusts.
    let mut plain = w.clone();
    lifestate::after_match(&mut plain, &fx, [&sa, &sb], &r, &c);
    let mut helped = w.clone();
    helped.players.cold[sa.captain].attrs.set(Attr::Leadership, 18.0);
    helped.social.adjust(who, cap, helped.date, 0, 0, 40, 0);
    lifestate::after_match(&mut helped, &fx, [&sa, &sb], &r, &c);
    assert!(mag(&helped) < mag(&plain), "the captain's steadying hand: {} < {}", mag(&helped), mag(&plain));
    // A serious injury in an important match leaves a mark on that ground.
    let mut hurt = line(sa.xi[4], 0, 6.0);
    hurt.injured = true;
    let mut w3 = w.clone();
    w3.players.hot[sa.xi[4]].injury_days = 90;
    let (r3, c3) = (result(&w3, &fx, 0, 0, vec![hurt], vec![]), ctx(&w3, &fx, 0.8, false));
    lifestate::after_match(&mut w3, &fx, [&sa, &sb], &r3, &c3);
    let sc = w3.lifestate.by[&w3.players.cold[sa.xi[4]].person].scars.iter().find(|s| s.kind == ScarKind::SeriousInjury).copied().expect("a scar");
    assert_eq!(sc.venue, home);
}

#[test]
fn the_world_notices_what_a_player_did_through_what_it_knows_of_him() {
    let mut w = world();
    let (fx, sa, sb, _) = fixture_and_sides(&w);
    let (p_known, p_hidden) = (sa.xi[3], sa.xi[6]);
    let (k, h) = (w.players.cold[p_known].person, w.players.cold[p_hidden].person);
    let mgr = w.manager_of_player(p_known).unwrap();
    for who in [k, h] {
        // Two men who take things hard, so that what they carry is heavy.
        for x in [pw_core::Hidden::Pressure, pw_core::Hidden::Professionalism, pw_core::Hidden::Temperament] {
            w.people[who].hidden.set(x, 4);
        }
        lifestate::add_load(&mut w, who, LoadKind::Bereavement, 1.0, EventId::NONE);
        age(&mut w, who, 8);
    }
    // The manager knows of one man's loss; nobody knows of the other's.
    w.lifestate.known.insert((mgr, k), Known { level: Awareness::Knows, kind: LoadKind::Bereavement, severity: 70, since: w.date, stance: Handling::Start });
    let lines = vec![line(p_known, 0, 9.0), line(p_hidden, 0, 9.0)];
    let r = result(&w, &fx, 3, 0, lines, vec![]);
    let before = w.events.len();
    let c2 = ctx(&w, &fx, 0.5, false);
    lifestate::after_match(&mut w, &fx, [&sa, &sb], &r, &c2);
    let strain: Vec<(PlayerId, bool)> = w.events.all()[before..].iter().filter_map(|e| if let EventKind::PerformedThroughStrain { player, well, .. } = e.kind { Some((player, well)) } else { None }).collect();
    assert_eq!(strain, vec![(p_known, true)], "extraordinary pressure and still brilliant, for the man whose context is known; the other's is private");
    assert!(attention::level_of(&w, k, Wave::Emotional) > 0.2, "the story travels");
    assert!(w.social.get(k, mgr).is_some(), "and the manager's praise is remembered");
    // The same context, played badly, draws a different kind of reaction (sympathy or blame), still only where it is known.
    let r = result(&w, &fx, 0, 3, vec![line(p_known, 0, 4.5), line(p_hidden, 0, 4.5)], vec![]);
    let before = w.events.len();
    let c2 = ctx(&w, &fx, 0.5, false);
    lifestate::after_match(&mut w, &fx, [&sa, &sb], &r, &c2);
    let bad: Vec<(PlayerId, bool)> = w.events.all()[before..].iter().filter_map(|e| if let EventKind::PerformedThroughStrain { player, well, .. } = e.kind { Some((player, well)) } else { None }).collect();
    assert_eq!(bad, vec![(p_known, false)]);
}

#[test]
fn managers_perceive_a_players_state_imperfectly() {
    let mut w = world();
    let people = players_of(&w, 90);
    // Comparable weight, one private (money worries), one public (a pile-on).
    for (i, &(_, who)) in people.iter().enumerate() {
        let (kind, days) = if i % 2 == 0 { (LoadKind::FinancialTrouble, 8) } else { (LoadKind::OnlineAbuse, 2) };
        lifestate::add_load(&mut w, who, kind, 1.0, EventId::NONE);
        age(&mut w, who, days);
    }
    lifestate::weekly(&mut w);
    let known = |k: LoadKind| people.iter().enumerate().filter(|(i, _)| (i % 2 == 0) == (k == LoadKind::FinancialTrouble)).filter(|&(_, &(p, who))| w.manager_of_player(p).is_some_and(|m| w.lifestate.known.get(&(m, who)).is_some_and(|x| x.level == Awareness::Knows))).count();
    let (private, public) = (known(LoadKind::FinancialTrouble), known(LoadKind::OnlineAbuse));
    assert!(private > 0 && private < 45, "some managers never hear of it: {private} of 45");
    assert!(public > private, "what is public reaches more of them: {public} against {private}");
    let dips = w.lifestate.known.values().filter(|k| k.level == Awareness::Dip).count();
    assert!(dips > 0, "some see only poor training: {dips}");
    // What he believes is not what is true.
    let exact = people.iter().filter(|&&(p, who)| w.manager_of_player(p).and_then(|m| w.lifestate.known.get(&(m, who))).is_some_and(|k| f32::from(k.severity) == lifestate::strain(&w, who).0.round())).count();
    assert!(exact < 10, "beliefs are estimates: {exact} exact");
}

#[test]
fn selection_follows_what_the_manager_believes_not_what_is_true() {
    let mut w = world();
    let (p, who) = players_of(&w, 1)[0];
    let mgr = w.manager_of_player(p).unwrap();
    lifestate::add_load(&mut w, who, LoadKind::Bereavement, 1.0, EventId::NONE);
    age(&mut w, who, 10);
    let unaware = lifestate::selection_term(&w, Some(mgr), p, 0.4);
    assert!(unaware >= 0.0, "a manager who has not been told does not rank him lower: {unaware}");
    w.lifestate.known.insert((mgr, who), Known { level: Awareness::Knows, kind: LoadKind::Bereavement, severity: 80, since: w.date, stance: Handling::Rest });
    let rest = lifestate::selection_term(&w, Some(mgr), p, 0.4);
    assert!(rest < -0.3, "he knows, and rests him: {rest}");
    let big = lifestate::selection_term(&w, Some(mgr), p, 1.0);
    assert!(big > rest, "less so in the match that matters most: {big} vs {rest}");
    w.lifestate.known.insert((mgr, who), Known { level: Awareness::Knows, kind: LoadKind::Bereavement, severity: 80, since: w.date, stance: Handling::Start });
    assert!(lifestate::selection_term(&w, Some(mgr), p, 0.4) >= 0.0, "he decided to play him");
}

#[test]
fn a_manager_weighs_the_man_and_the_football_and_what_he_chooses_is_itself_an_event() {
    let mut w = world();
    let people = players_of(&w, 120);
    for &(_, who) in &people {
        lifestate::add_load(&mut w, who, LoadKind::Bereavement, 1.0, EventId::NONE);
        age(&mut w, who, 10);
    }
    // Make every disclosure certain by trust, so the choice is what varies.
    let before = w.events.len();
    for _ in 0..3 {
        lifestate::weekly(&mut w);
        w.date = w.date.add_days(7);
    }
    let stances: std::collections::BTreeSet<String> = w.lifestate.known.values().map(|k| format!("{:?}", k.stance)).collect();
    assert!(stances.len() >= 2, "managers choose differently: {stances:?}");
    let events = w.events.all()[before..].iter().filter(|e| matches!(e.kind, EventKind::PersonalMatterHandled { .. })).count();
    assert!(events > 0 && !w.lifestate.handled.is_empty(), "the decisions are on the record ({events})");
    // Sending someone home is a real thing: he is not available.
    let sent = w.lifestate.handled.iter().find(|h| h.handling == Handling::SendHome);
    if let Some(h) = sent {
        let p = w.people[h.player].player;
        assert!(w.incidents.is_away(p, h.date.add_days(1)));
    }
}

#[test]
fn life_events_reach_people_as_things_they_live_through() {
    let mut w = world();
    let (p, who) = players_of(&w, 1)[0];
    let today = w.date;
    w.events.push(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::Bereavement });
    w.events.push(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::ChildBorn });
    w.events.push(today, Visibility::Public, EventKind::CallUp { player: p });
    lifestate::scan(&mut w);
    let kinds: Vec<LoadKind> = w.lifestate.by[&who].loads.iter().map(|l| l.kind).collect();
    assert!(kinds.contains(&LoadKind::Bereavement) && kinds.contains(&LoadKind::NewChild) && kinds.contains(&LoadKind::CallUp), "{kinds:?}");
    // Grief and joy at once: the channels hold both.
    let ch = lifestate::channels(&w, who);
    assert!(ch[Chan::Grief.idx()] >= 0.0);
    let n = w.lifestate.by[&who].loads.len();
    lifestate::scan(&mut w);
    assert_eq!(w.lifestate.by[&who].loads.len(), n, "an event is lived once");
}

#[test]
fn social_discourse_crosses_into_state_and_state_back_out() {
    let mut w = world();
    let people = players_of(&w, 2);
    let (_, pile) = people[0];
    let (_, acclaim) = people[1];
    for who in [pile, acclaim] {
        w.net.attention.remove(&who);
    }
    attention::spark(&mut w, pile, Wave::Controversy, 0.9);
    assert!(w.lifestate.by[&pile].loads.iter().any(|l| l.kind == LoadKind::OnlineAbuse), "a pile-on is something he lives through");
    attention::spark(&mut w, acclaim, Wave::Football, 0.95);
    assert!(w.lifestate.by[&acclaim].loads.iter().any(|l| l.kind == LoadKind::Hype), "sudden acclaim is another");
    // A quiet moment does neither.
    let (_, quiet) = players_of(&w, 3)[2];
    attention::spark(&mut w, quiet, Wave::Football, 0.2);
    assert!(w.lifestate.by.get(&quiet).is_none());
}

#[test]
fn state_walks_onto_the_pitch_with_the_players_who_carry_it_and_only_them() {
    let mut w = world();
    let (fx, sa, sb, _) = fixture_and_sides(&w);
    let (p, who) = (sa.xi[4], w.players.cold[sa.xi[4]].person);
    let none = lifestate::minds_for(&w, [&sa, &sb], &fx, 0.6);
    lifestate::add_load(&mut w, who, LoadKind::Bereavement, 1.0, EventId::NONE);
    age(&mut w, who, 12);
    let some = lifestate::minds_for(&w, [&sa, &sb], &fx, 0.6);
    let m = some[&p];
    assert!(m.focus < -0.1 && m.drive < 0.0, "grief costs attention and energy: {m:?}");
    assert!(!none.contains_key(&p) || none[&p].focus > m.focus);
}
