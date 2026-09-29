//! Settling in and planning that fails (locked design 4.10-4.23): channels with their own clocks, distances from real differences and
//! not nationalities, experience that shortens them, support that helps, a manager who chooses how fast to bring a signing in, and
//! plans that turn out wrong and are remembered.

use pw_core::{ClubId, PlayerId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, adaptation, boardroom};
use pw_world::adaptation::{Channel, Integration, Level, N_CHANNELS};
use pw_world::boardroom::PlanFailure;
use pw_world::nation::Environment;
use pw_world::{EventKind, SquadStatus, World};

fn world() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 61, Scale::SMALL));
    sim.run(45);
    sim
}

/// A player at a club in nation 0, and a club in nation 1 to move him to.
fn move_pair(w: &World) -> (PlayerId, ClubId, ClubId) {
    let n0 = w.nations.ids().next().unwrap();
    let n1 = w.nations.ids().nth(1).unwrap();
    let from = w.clubs.ids().find(|&c| w.clubs[c].nation == n0 && w.clubs[c].manager.is_some()).unwrap();
    let to = w.clubs.ids().find(|&c| w.clubs[c].nation == n1 && w.clubs[c].manager.is_some()).unwrap();
    let p = w.teams[w.clubs[from].first_team()].squad[4];
    (p, from, to)
}

fn far() -> Environment {
    Environment { climate: 3, humidity: 85, altitude: 2800, tz: 9, language: 77, culture: 9, pace: 80, physical: 20, tempo: 80, tax: 45, living: 140, known: true }
}

fn near_of(e: &Environment) -> Environment {
    Environment { known: true, ..*e }
}

#[test]
fn distance_comes_from_real_differences_and_experience_takes_it_off() {
    let mut sim = world();
    let (p, from, to) = move_pair(&sim.world);
    let w = &mut sim.world;
    let (n0, n1) = (w.clubs[from].nation, w.clubs[to].nation);
    let who = w.players.cold[p].person;

    // Two countries alike in every respect: the body, the clock and the football are not far apart, whatever their names.
    let base = w.nations[n0].env;
    w.nations[n1].env = near_of(&base);
    let alike = adaptation::distances(w, p, from, to);
    assert!(alike[Channel::Environment.idx()] < 0.05 && alike[Channel::Routine.idx()] < 0.05, "{alike:?}");

    // Very different ones: far on climate, clock and football, and (with no language in common) socially.
    w.nations[n1].env = far();
    w.lives[who].languages.retain(|(n, _)| *n != n1);
    let distant = adaptation::distances(w, p, from, to);
    assert!(distant[Channel::Environment.idx()] > 0.5 && distant[Channel::Routine.idx()] > 0.8 && distant[Channel::Football.idx()] > 0.4, "{distant:?}");
    assert!(distant[Channel::Social.idx()] > alike[Channel::Social.idx()] + 0.25);

    // Speaking the language shortens the social distance only.
    w.lives[who].languages.push((n1, 95));
    let speaks = adaptation::distances(w, p, from, to);
    assert!(speaks[Channel::Social.idx()] < distant[Channel::Social.idx()] - 0.2);
    assert!((speaks[Channel::Environment.idx()] - distant[Channel::Environment.idx()]).abs() < 1e-4);

    // Years spent somewhere like the destination take off climate and clock, not the manager's system.
    let before = adaptation::distances(w, p, from, to);
    let other = w.clubs.ids().find(|&c| w.clubs[c].nation == n1).unwrap();
    w.history.start_spell(p, other, w.date.add_days(-900), false, 0);
    w.history.end_spell(p, w.date.add_days(-300));
    let seasoned = adaptation::distances(w, p, from, to);
    assert!(seasoned[Channel::Environment.idx()] < before[Channel::Environment.idx()] * 0.6, "{seasoned:?} vs {before:?}");
    assert!(seasoned[Channel::Routine.idx()] < before[Channel::Routine.idx()] * 0.6);
    // Not by nationality: nothing here looked at a country's name, only at its setting.
    assert!(seasoned.iter().all(|d| (0.0..=1.0).contains(d)));
}

#[test]
fn every_channel_keeps_its_own_clock_and_two_players_do_not_settle_alike() {
    let mut sim = world();
    let (p, from, to) = move_pair(&sim.world);
    let w = &mut sim.world;
    let (n0, n1) = (w.clubs[from].nation, w.clubs[to].nation);
    w.nations[n1].env = near_of(&w.nations[n0].env);
    adaptation::begin(w, p, from, to);
    w.players.hot[p].club = to;
    let a = w.adaptation.current.get(&p).expect("he is settling").clone();
    let distinct = a.weeks.iter().fold(Vec::<u32>::new(), |mut v, x| {
        if !v.contains(&(*x as u32)) {
            v.push(*x as u32);
        }
        v
    });
    assert!(distinct.len() >= 4, "not one generic timer: {:?}", a.weeks);
    assert!(a.progress.iter().all(|&x| x == 0.0), "signed is not settled");

    // A month on, the quick channels are further along than the slow ones.
    let mut clock = w.clone();
    for _ in 0..4 {
        clock.date = clock.date.add_days(7);
        adaptation::weekly(&mut clock);
    }
    let after = &clock.adaptation.current[&p];
    assert!(after.progress[Channel::Routine.idx()] > after.progress[Channel::Social.idx()], "{:?}", after.progress);
    assert!(after.progress[Channel::Environment.idx()] > after.progress[Channel::Tactical.idx()]);

    // The same move by an adaptable, professional player and by a rigid, careless one.
    let q = w.teams[w.clubs[from].first_team()].squad[6];
    let (whop, whoq) = (w.players.cold[p].person, w.players.cold[q].person);
    for (who, v) in [(whop, 19u8), (whoq, 2u8)] {
        for h in [pw_core::Hidden::Adaptability, pw_core::Hidden::Professionalism, pw_core::Hidden::Pressure] {
            w.people[who].hidden.set(h, v);
        }
    }
    adaptation::begin(w, p, from, to);
    adaptation::begin(w, q, from, to);
    let (sp, sq) = (w.adaptation.current[&p].weeks, w.adaptation.current[&q].weeks);
    for ch in [Channel::Tactical, Channel::Social, Channel::Mental] {
        assert!(sp[ch.idx()] < sq[ch.idx()], "{ch:?}: adaptable {} vs rigid {}", sp[ch.idx()], sq[ch.idx()]);
    }
}

#[test]
fn what_a_club_does_for_a_newcomer_shortens_the_wait() {
    let mut sim = world();
    let (p, from, to) = move_pair(&sim.world);
    let w = &mut sim.world;
    w.clubs[to].reputation = 500;
    for s in w.clubs[to].staff.clone() {
        w.staff[s].role = pw_world::StaffRole::Coach;
    }
    let poor_support = adaptation::club_support(w, to, p);
    adaptation::begin(w, p, from, to);
    let poor = w.adaptation.current[&p].clone();

    w.clubs[to].reputation = 9000;
    let staff = w.clubs[to].staff.clone();
    w.staff[staff[0]].role = pw_world::StaffRole::DirectorOfFootball;
    w.staff[staff[1]].role = pw_world::StaffRole::SportsScientist;
    w.staff[staff[2]].role = pw_world::StaffRole::FitnessCoach;
    let rich_support = adaptation::club_support(w, to, p);
    assert!(rich_support.count_ones() > poor_support.count_ones(), "{rich_support:#b} vs {poor_support:#b}");
    adaptation::begin(w, p, from, to);
    let rich = w.adaptation.current[&p].clone();
    for ch in [Channel::Social, Channel::Mental] {
        assert!(rich.weeks[ch.idx()] < poor.weeks[ch.idx()], "{ch:?}: {} vs {}", rich.weeks[ch.idx()], poor.weeks[ch.idx()]);
    }
}

#[test]
fn the_manager_chooses_how_fast_to_bring_him_in_and_selection_obeys() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some()).unwrap();
    let team = w.clubs[club].first_team();
    let star = pw_sim::selection::select(w, team, w.date, 0.5, 9, 1).unwrap().xi[5];

    // Body and clock far away: he is not thrown in.
    let mut hard = [0.1f32; N_CHANNELS];
    hard[Channel::Environment.idx()] = 0.9;
    hard[Channel::Routine.idx()] = 0.9;
    let cautious = {
        w.people[w.staff[w.clubs[club].manager].person].hidden.set(pw_core::Hidden::Ambition, 2);
        w.people[w.staff[w.clubs[club].manager].person].hidden.set(pw_core::Hidden::Controversy, 2);
        adaptation::choose_integration(w, club, star, &hard)
    };
    assert!(matches!(cautious, Integration::TrainingOnly | Integration::ConditioningFirst), "{cautious:?}");
    // Nothing to get used to: not held back for his body or his clock.
    let easy = [0.05f32; N_CHANNELS];
    assert!(!matches!(adaptation::choose_integration(w, club, star, &easy), Integration::TrainingOnly | Integration::ConditioningFirst));

    // The plan is kept for a spell and it removes him from the eleven; without it he plays.
    let plays = |w: &World| pw_sim::selection::select(w, team, w.date, 0.5, 9, 1).unwrap().xi.contains(&star);
    assert!(plays(w), "a member of the eleven starts");
    adaptation::begin(w, star, ClubId::NONE, club);
    let a = w.adaptation.current.get_mut(&star).unwrap();
    a.plan = Integration::TrainingOnly;
    a.plan_until = w.date.add_days(14);
    assert_eq!(adaptation::hold(w, star), 1.0);
    assert!(!plays(w), "training only means not in the eleven");
    w.date = w.date.add_days(15);
    assert_eq!(adaptation::hold(w, star), 0.0, "and the hold ends when the plan does");
}

#[test]
fn a_player_still_settling_brings_less_to_the_pitch() {
    let mut sim = world();
    let (p, from, to) = move_pair(&sim.world);
    let w = &mut sim.world;
    let base = pw_sim::selection::player_sheet(w, p);
    adaptation::begin(w, p, from, to);
    let a = w.adaptation.current.get_mut(&p).unwrap();
    a.progress = [0.0; N_CHANNELS];
    let fresh = pw_sim::selection::player_sheet(w, p);
    assert!(fresh.sharpness < base.sharpness && fresh.morale < base.morale, "body and head are not there yet");
    assert!(fresh.familiarity.iter().zip(base.familiarity.iter()).any(|(a, b)| a < b), "thinking instead of reacting");
    w.adaptation.current.get_mut(&p).unwrap().progress = [1.0; N_CHANNELS];
    let settled = pw_sim::selection::player_sheet(w, p);
    assert_eq!((settled.sharpness, settled.morale, settled.familiarity), (base.sharpness, base.morale, base.familiarity));
    // A simpler role softens the system's part of it.
    let a = w.adaptation.current.get_mut(&p).unwrap();
    a.progress = [0.5; N_CHANNELS];
    a.plan = Integration::StartImmediately;
    let full = pw_sim::selection::player_sheet(w, p).familiarity.iter().map(|&f| u32::from(f)).sum::<u32>();
    w.adaptation.current.get_mut(&p).unwrap().plan = Integration::SimplifiedRole;
    let simple = pw_sim::selection::player_sheet(w, p).familiarity.iter().map(|&f| u32::from(f)).sum::<u32>();
    assert!(simple > full, "{simple} vs {full}");
}

#[test]
fn early_use_can_speed_things_up_or_damage_them() {
    let mut sim = world();
    let (p, from, to) = move_pair(&sim.world);
    let w = &mut sim.world;
    adaptation::begin(w, p, from, to);
    w.players.hot[p].club = to;
    let start = w.adaptation.current[&p].clone();

    // Negative loop: three full matches in a week on a body and clock that have not settled.
    let mut heavy = w.clone();
    heavy.players.hot[p].minutes_week = 270;
    let fatigue = heavy.players.hot[p].fatigue;
    heavy.date = heavy.date.add_days(7);
    adaptation::weekly(&mut heavy);
    assert!(heavy.players.hot[p].fatigue > fatigue, "recovery worsens");
    assert!(heavy.adaptation.current[&p].strain > 0.0);

    // Positive loop: playing well early speeds up the slow channels; playing badly slows them.
    let progress_with = |form_rating: f32| {
        let mut x = w.clone();
        x.players.hot[p].minutes_week = 90;
        for _ in 0..5 {
            x.players.hot[p].push_rating(form_rating);
        }
        for _ in 0..4 {
            x.date = x.date.add_days(7);
            adaptation::weekly(&mut x);
        }
        x.adaptation.current[&p].progress[Channel::Tactical.idx()]
    };
    let (good, bad) = (progress_with(8.0), progress_with(5.0));
    assert!(good > bad, "{good} vs {bad}");
    assert!(start.strain == 0.0);
}

#[test]
fn a_club_expects_the_settling_from_public_facts_and_is_not_certain() {
    let mut sim = world();
    let (p, _from, to) = move_pair(&sim.world);
    let w = &mut sim.world;
    let n1 = w.clubs[to].nation;
    let home = w.players.hot[p].club;
    let n0 = w.clubs[home].nation;
    // A short hop and a long one.
    w.nations[n1].env = near_of(&w.nations[n0].env);
    let near = adaptation::readiness(w, to, p);
    w.nations[n1].env = far();
    let distant = adaptation::readiness(w, to, p);
    assert!(distant.risks[Channel::Environment.idx()] > near.risks[Channel::Environment.idx()], "{:?} vs {:?}", distant.risks, near.risks);
    assert!(distant.risks[Channel::Routine.idx()] > near.risks[Channel::Routine.idx()]);
    assert!(distant.weeks_to_useful >= near.weeks_to_useful, "a longer move is never a shorter wait");
    assert!(distant.risks.iter().any(|r| *r >= Level::Moderate));
    assert!(distant.immediate_effectiveness <= near.immediate_effectiveness && distant.immediate_availability <= near.immediate_availability, "a far move is less useful straight away");
    // A player the club has never seen is a wider guess than one it has watched for months.
    let strange = w.teams[w.clubs[home].first_team()].squad.iter().copied().find(|&q| w.knowledge.seen(to, q).is_none()).unwrap();
    w.knowledge.observe(to, p, 900, w.date);
    let known = adaptation::readiness(w, to, p);
    assert!(adaptation::readiness(w, to, strange).confidence <= known.confidence);
    // Cheap read for scanning: nothing to settle within a country, something across a border.
    let same = w.teams[w.clubs[to].first_team()].squad[3];
    assert_eq!(adaptation::quick_weeks(w, same, to), 0.0);
    assert!(adaptation::quick_weeks(w, p, to) > 5.0);
}

#[test]
fn urgent_clubs_mind_the_wait_and_rebuilding_ones_do_not() {
    let mut sim = world();
    let (p, _, to) = move_pair(&sim.world);
    let w = &mut sim.world;
    let group = w.players.cold[p].best_pos.group();
    let need = pw_world::deals::PlanNeed {
        group,
        pos: w.players.cold[p].best_pos,
        role: pw_world::deals::NeedRole::Starter,
        min_ability: 100,
        max_age: 30,
        homegrown: false,
        wage_band: 1000,
        fee_band: 1_000_000,
        urgency: 3,
    };
    w.deals.plans.entry(to).or_insert_with(|| pw_world::deals::SquadPlan { built: w.date, groups: Default::default(), needs: Default::default(), sell: Default::default(), promote: Default::default(), homegrown_gap: 0, wage_headroom: 0 }).needs = [need].into_iter().collect();
    w.governance.get_mut(&to).unwrap().policy.transfer_style = pw_world::governance::TransferStyle::WinNow;
    let urgent = adaptation::impatience(w, to, p);
    w.governance.get_mut(&to).unwrap().policy.transfer_style = pw_world::governance::TransferStyle::Develop;
    let rebuilding = adaptation::impatience(w, to, p);
    assert!(urgent > rebuilding * 2.0, "{urgent} vs {rebuilding}");
}

#[test]
fn a_key_player_walking_out_on_a_free_is_a_failed_plan_and_the_scramble_is_marked() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some()).unwrap();
    let p = w.teams[w.clubs[club].first_team()].squad[2];
    w.players.cold[p].status = SquadStatus::Star;
    let group = w.players.cold[p].best_pos.group();
    let events = w.events.len();
    boardroom::on_expiry(w, p);
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, EventKind::PlanFailed { kind: PlanFailure::RenewalCollapsed, .. })));
    assert_eq!(w.boardroom.scramble.get(&club).map(|x| x.0), Some(group));

    // A signing soon after, in that position, is made in a hurry: it goes on the case file and counts against the process.
    let other = w.clubs.ids().find(|&c| c != club && w.clubs[c].nation == w.clubs[club].nation).unwrap();
    let target = w.teams[w.clubs[other].first_team()].squad[5];
    let revenue = pw_sim::finance::season_revenue(w, club);
    let target_group = w.players.cold[target].best_pos.group();
    w.boardroom.scramble.insert(club, (target_group, w.date));
    boardroom::consider_target(w, club, target, Some(target_group), revenue, revenue);
    let case = w.boardroom.cases.last().unwrap();
    assert!(case.panic && case.planned);
}

#[test]
fn a_prospect_the_club_counted_on_who_was_not_ready_and_a_blocked_youngster_who_thrived_elsewhere_become_history() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some()).unwrap();
    let academy = w.clubs[club].teams.iter().copied().find(|&t| w.teams[t].kind != pw_world::TeamKind::First && !w.teams[t].squad.is_empty()).map(|t| w.teams[t].squad[0]);
    let prospect = academy.unwrap_or(w.teams[w.clubs[club].first_team()].squad[20]);
    w.deals.counted_on.push(pw_world::deals::CountedOn { club, player: prospect, date: w.date.add_days(-340), expected: 190.0 });
    let events = w.events.len();
    w.date = w.date.add_days(1);
    boardroom::check_plans(w);
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, EventKind::PlanFailed { kind: PlanFailure::ProspectOverestimated, player, .. } if player == prospect)));
    assert!(!w.deals.counted_on.iter().any(|c| c.date.days_until(w.date) >= 330), "plans that have been checked are not kept");

    // A youngster signed over, gone, and now well known and far better than the club thought.
    let star = w.players.ids().max_by_key(|&p| w.players.cold[p].ca).unwrap();
    let other = w.clubs.ids().find(|&c| c != club).unwrap();
    w.players.hot[star].club = other;
    w.players.cold[star].rep.world = 8000;
    let then = pw_sim::market::public_view(w, star).0 - 30.0;
    w.boardroom.blocked.push(pw_world::boardroom::Blocked { club, youngster: star, signing: prospect, date: w.date.add_days(-320), level_then: then });
    let events = w.events.len();
    boardroom::check_plans(w);
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, EventKind::PlanFailed { kind: PlanFailure::YouthBlocked, player, .. } if player == star)));
    assert!(w.boardroom.blocked.is_empty());
}

#[test]
fn exceptional_chances_outside_the_plan_are_noticed_and_marked_as_such() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 62, Scale::SMALL));
    sim.run(400);
    let w = &sim.world;
    // Whatever a club has noticed really is a bargain by its own reading.
    for (&club, &(p, _)) in &w.deals.opportunities {
        let (ca, ..) = pw_sim::scouting::view(w, club, p);
        assert!(ca >= pw_sim::market::ideal_ca(w.clubs[club].reputation) + 4.0);
        assert!(w.players.hot[p].club.is_some() && w.players.hot[p].club != club, "an opportunity is someone at another club");
    }
    // Planned and opportunistic signings are told apart on the case files.
    let planned = w.boardroom.cases.iter().filter(|c| c.planned).count();
    assert!(planned > 0, "most decisions start from a need");
}
