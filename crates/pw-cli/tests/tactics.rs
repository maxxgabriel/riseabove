//! Tactical intelligence (locked design 7.1-7.34): a belief dossier that is not the opponent's instructions, observation apart from
//! diagnosis, staff who disagree and are right or wrong, managers with their own thresholds, execution limited by what the squad has
//! drilled, opponents who react, tactical memory, and lessons that can be the wrong ones.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use pw_core::{ClubId, Hidden, Mentality, PlayerId, Pos, StaffAttr, StaffId, Tactics};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_match::{Coach, Look, PlayerLook, SideLook, Tally};
use pw_sim::coach::{self, MatchCoach};
use pw_sim::selection::{self, Selection};
use pw_sim::tactics::{self, MatchCtx};
use pw_sim::Sim;
use pw_world::dossier::Confidence;
use pw_world::event::{Cause, EventKind, Fact};
use pw_world::tactics::{Credit, Diagnosis, Drill, ManagerTactics, OppMemory, Response, SignKind};
use pw_world::{Archetype, Fixture, World};

fn season() -> &'static World {
    static W: OnceLock<World> = OnceLock::new();
    W.get_or_init(|| {
        let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 77, Scale::SMALL));
        sim.run(220);
        sim.world
    })
}

fn young() -> World {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 31, Scale::MICRO));
    sim.run(20);
    sim.world
}

/// Two clubs and their selections, and a fixture between them.
fn pair(w: &World) -> (ClubId, ClubId, Selection, Selection, Fixture) {
    let ids: Vec<ClubId> = w.clubs.ids().collect();
    let (a, b) = (ids[0], ids[1]);
    let sel = |c: ClubId| selection::select(w, w.clubs[c].first_team(), w.date, 0.6, 7, 0).expect("a team can be picked");
    let (sa, sb) = (sel(a), sel(b));
    let mut fx = w.fixtures.iter().next().expect("a fixture").1.clone();
    fx.home = w.clubs[a].first_team();
    fx.away = w.clubs[b].first_team();
    (a, b, sa, sb, fx)
}

fn side(s: &Selection) -> SideLook {
    let bench_pos = [Pos::ST, Pos::DC, Pos::MC, Pos::AMR, Pos::DL, Pos::DM];
    SideLook {
        tactics: s.tactics,
        possession: 50,
        shots: 3,
        on_target: 1,
        big_chances: 0,
        corners: 1,
        fouls: 3,
        yellows: 0,
        reds: 0,
        territory: 8,
        lost_own_third: 1,
        subs_made: 0,
        subs_max: 5,
        players: s.xi.iter().zip(s.slots.iter()).map(|(&p, sl)| PlayerLook { player: p, pos: sl.pos, role: sl.role, condition: 90, booked: false, tally: Tally::default() }).collect(),
        bench: s.bench.iter().enumerate().map(|(i, &p)| (p, if i == 0 { Pos::GK } else { bench_pos[i % bench_pos.len()] })).collect(),
    }
}

fn look(a: &Selection, b: &Selection, minute: u8, half: bool) -> Look {
    Look { minute, half_time: half, goals: [0, 0], sides: [side(a), side(b)] }
}

fn ctx(w: &World, fx: &Fixture) -> MatchCtx {
    MatchCtx::for_fixture(w, fx, 0.6, false, None, 1.0)
}

/// Make the manager of a club a certain kind of tactician.
fn shape_manager(w: &mut World, club: ClubId, archetype: Archetype, skill: f32, temper: f32) -> StaffId {
    let m = w.clubs[club].manager.get().expect("a manager");
    let person = w.staff[m].person;
    w.staff[m].philosophy.archetype = archetype;
    for a in [StaffAttr::TacticalKnowledge, StaffAttr::JudgingAbility, StaffAttr::Tactical] {
        w.staff[m].attrs.set(a, skill as u8);
    }
    for h in [Hidden::Temperament, Hidden::Ambition, Hidden::Adaptability, Hidden::Consistency] {
        w.people[person].hidden.set(h, temper as u8);
    }
    m
}

fn run_call(w: &World, fx: &Fixture, a: &Selection, b: &Selection, looks: &[Look]) -> pw_sim::coach::Logs {
    let mut c = MatchCoach::new(w, fx, ctx(w, fx), [a, b], Default::default());
    for l in looks {
        c.call(l);
    }
    c.into_logs()
}

#[test]
fn the_dossier_is_a_belief_about_the_opponent_that_can_be_wrong_and_grows_surer_with_evidence() {
    let w = season();
    let clubs: Vec<ClubId> = w.clubs.ids().collect();
    let (mut n, mut exact, mut thin_now, mut sure) = (0, 0, 0, 0);
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for &c in &clubs {
        for &o in clubs.iter().filter(|&&o| o != c).take(6) {
            let Some(last) = w.tactics.styles.get(&o).and_then(|v| v.back()) else { continue };
            let d = tactics::dossier(w, c, o);
            n += 1;
            exact += usize::from(d.press == last.press);
            thin_now += usize::from(d.thin);
            sure += usize::from(d.style_confidence >= Confidence::Medium);
            xs.push(f32::from(d.press));
            ys.push(f32::from(last.press));
        }
    }
    assert!(n > 100, "{n}");
    assert!(exact * 4 < n, "a belief is not the opponent's actual instructions: {exact}/{n} matched exactly");
    let (mx, my) = (xs.iter().sum::<f32>() / n as f32, ys.iter().sum::<f32>() / n as f32);
    let cov: f32 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let corr = cov / (xs.iter().map(|x| (x - mx).powi(2)).sum::<f32>().sqrt() * ys.iter().map(|y| (y - my).powi(2)).sum::<f32>().sqrt());
    assert!(corr > 0.4, "but it follows how they were seen to play: correlation {corr}");
    assert!(sure * 2 > n && thin_now * 3 < n, "after a season most beliefs rest on real evidence: sure {sure}, thin {thin_now} of {n}");
    // With nothing to go on, he says so.
    let fresh = young();
    let ids: Vec<ClubId> = fresh.clubs.ids().collect();
    let d = tactics::dossier(&fresh, ids[0], ids[1]);
    assert!(d.thin && d.style_confidence == Confidence::Low && d.shape_confidence == Confidence::Low);
}

#[test]
fn better_preparation_gives_a_truer_picture() {
    let mut w = season().clone();
    let clubs: Vec<ClubId> = w.clubs.ids().collect();
    let (c, o) = (clubs[0], clubs[1]);
    let recs: Vec<f32> = w.tactics.styles[&o].iter().map(|r| f32::from(r.press)).collect();
    let truth = recs.iter().sum::<f32>() / recs.len() as f32;
    let mut err = |skill: f32| {
        shape_manager(&mut w, c, Archetype::Pragmatist, skill, 10.0);
        let base = w.date;
        let mut total = 0.0;
        for k in 0..60 {
            w.date = base.add_days(k);
            total += (f32::from(tactics::dossier(&w, c, o).press) - truth).abs();
        }
        total / 60.0
    };
    let (poor, good) = (err(3.0), err(20.0));
    assert!(poor > good * 1.15, "a careless preparation ({poor}) is further out than a thorough one ({good})");
}

#[test]
fn observation_is_one_thing_and_the_explanation_another_and_the_same_evidence_is_read_differently() {
    let mut w = season().clone();
    let (a, _b, sa, sb, fx) = pair(&w);
    // Vary the manager's interpretation in isolation. The season's recruited advisers can otherwise
    // supply the same strong diagnosis on every call, dominating the reading being varied here.
    let manager = w.clubs[a].manager;
    w.clubs[a].staff.retain(|&staff| staff == manager);
    // One defender is being beaten and they are getting the better chances: two true things, several possible explanations.
    let mut l = look(&sa, &sb, 45, true);
    l.sides[1].shots = 12;
    l.sides[1].on_target = 6;
    l.sides[1].big_chances = 2;
    l.sides[0].players[3].tally.duels_lost = 4;
    l.sides[0].tactics.mentality = Mentality::Positive;
    let truth = coach::signs(&l, 0);
    assert!(truth.iter().any(|s| s.kind == SignKind::ChancesAgainst) && truth.iter().any(|s| s.kind == SignKind::BeatenMan && s.who == sa.xi[3]), "both signs are there to be seen");
    let (mut diagnoses, mut responses, mut blamed_man_for_leak, mut blamed_shape) = (BTreeSet::new(), BTreeSet::new(), 0, 0);
    for skill in [1.0, 6.0, 11.0, 16.0, 20.0] {
        shape_manager(&mut w, a, Archetype::Pragmatist, skill, 10.0);
        for uid in 0..40 {
            let mut f = fx.clone();
            f.uid = 900_000 + uid;
            let logs = run_call(&w, &f, &sa, &sb, &[l.clone()]);
            for t in &logs.brains[0].as_ref().unwrap().traces {
                diagnoses.insert(t.trace.believed);
                responses.insert(t.trace.response);
                if t.trace.sign == SignKind::ChancesAgainst {
                    blamed_man_for_leak += usize::from(t.trace.believed == Diagnosis::IndividualForm);
                    blamed_shape += usize::from(t.trace.believed == Diagnosis::TooOpen);
                }
            }
        }
    }
    assert!(diagnoses.len() >= 3 && responses.len() >= 2, "the same evidence, different readings and answers: {diagnoses:?} {responses:?}");
    assert!(blamed_man_for_leak > 0 && blamed_shape > 0, "sometimes he blames the man for what is the shape, sometimes the shape: {blamed_man_for_leak} / {blamed_shape}");
}

#[test]
fn a_manager_who_reads_the_game_worse_sees_less_of_it() {
    let mut w = season().clone();
    let (a, _b, sa, sb, fx) = pair(&w);
    let mut l = look(&sa, &sb, 45, true);
    l.sides[1].shots = 8;
    l.sides[1].on_target = 3;
    let acted = |w: &World| (0..80u64).filter(|&u| { let mut f = fx.clone(); f.uid = 500_000 + u; !run_call(w, &f, &sa, &sb, &[l.clone()]).brains[0].as_ref().unwrap().traces.is_empty() }).count();
    shape_manager(&mut w, a, Archetype::Rotator, 20.0, 10.0);
    let keen = acted(&w);
    shape_manager(&mut w, a, Archetype::Rotator, 1.0, 10.0);
    let dim = acted(&w);
    assert!(keen > dim + 5, "a keen reader notices what a poor one misses: {keen} against {dim}");
}

#[test]
fn managers_have_different_thresholds_for_change() {
    let mut w = season().clone();
    let (a, _b, sa, sb, fx) = pair(&w);
    // A middling piece of evidence, first seen at half an hour, still there twenty minutes later.
    let mut l30 = look(&sa, &sb, 30, false);
    l30.sides[1].shots = 6;
    l30.sides[1].on_target = 3;
    l30.sides[0].shots = 3;
    let mut l60 = l30.clone();
    l60.minute = 50;
    let acts = |w: &World| (0..60u64).filter(|&u| { let mut f = fx.clone(); f.uid = 700_000 + u; run_call(w, &f, &sa, &sb, &[l30.clone(), l60.clone()]).brains[0].as_ref().unwrap().traces.iter().any(|t| t.trace.response != Response::Hold) }).count();
    let m = shape_manager(&mut w, a, Archetype::Rotator, 14.0, 10.0);
    let hasty = tactics::profile(&w, a).unwrap().1;
    let quick = acts(&w);
    shape_manager(&mut w, a, Archetype::Loyalist, 14.0, 10.0);
    let slow = tactics::profile(&w, a).unwrap().1;
    let steady = acts(&w);
    let _ = m;
    assert!(slow.patience > hasty.patience + 12.0 && slow.stubborn > hasty.stubborn, "temperament, apart from knowledge: {} vs {}", slow.patience, hasty.patience);
    assert!(quick > steady + 15, "the impatient manager acts on it twenty minutes on ({quick}), the patient one waits for more ({steady})");
}

#[test]
fn half_time_allows_more_evidence_and_the_scoreline_changes_what_a_manager_will_risk() {
    let w = season();
    let (a, _b, sa, sb, fx) = pair(w);
    // The same 0-0 is not the same when two goals down on aggregate in the second half.
    let level = MatchCtx { importance: 0.6, decisive: false, first_leg: None, opposition: [0.0, 0.0], referee: 1.0 };
    let two_down = MatchCtx { importance: 0.6, decisive: true, first_leg: Some((0, 2)), opposition: [0.0, 0.0], referee: 1.0 };
    // home (side 0) hosted the second leg after the away side won the first 2-0.
    assert!(level.urgency(0, [0, 0], 70).abs() < 0.05, "a level league game at 70 minutes is calm");
    assert!(two_down.urgency(1, [0, 0], 70) < 0.05, "the away side is comfortable");
    assert!(two_down.urgency(0, [0, 0], 70) > 0.2, "the home side needs goals: {}", two_down.urgency(0, [0, 0], 70));
    assert!(level.urgency(0, [2, 0], 80) < -0.2, "and a side ahead late wants to keep it");
    assert!(two_down.urgency(0, [0, 0], 85) > two_down.urgency(0, [0, 0], 50), "the clock makes it more pressing");
    // In the last quarter, two down on aggregate: the manager acts on the scoreline where a league manager level at 0-0 does nothing.
    let mut l60 = look(&sa, &sb, 75, false);
    l60.goals = [0, 0];
    let mut acted = (0, 0);
    for u in 0..60u64 {
        let mut f = fx.clone();
        f.uid = 100_000 + u;
        let mut c1 = MatchCoach::new(w, &f, two_down, [&sa, &sb], Default::default());
        c1.call(&l60);
        acted.0 += usize::from(c1.into_logs().brains[0].as_ref().unwrap().traces.iter().any(|t| t.trace.sign == SignKind::Scoreline));
        let mut c2 = MatchCoach::new(w, &f, level, [&sa, &sb], Default::default());
        c2.call(&l60);
        acted.1 += usize::from(c2.into_logs().brains[0].as_ref().unwrap().traces.iter().any(|t| t.trace.sign == SignKind::Scoreline));
    }
    let _ = a;
    assert!(acted.0 > 30 && acted.1 == 0, "chasing the tie on aggregate: {} of 60, level league game: {}", acted.0, acted.1);
}

#[test]
fn a_change_is_only_carried_out_as_far_as_the_squad_has_drilled_it_and_the_manager_can_get_it_across() {
    let mut w = season().clone();
    let (a, _b, sa, _sb, _fx) = pair(&w);
    let (m, prof) = tactics::profile(&w, a).unwrap();
    let base = sa.tactics;
    let target = tactics::shifted(base, Response::GoDirect);
    let style = pw_world::tactics::style_of(&target);
    let xi: Vec<PlayerId> = sa.xi.to_vec();
    let mut fam = |v: u8| {
        w.tactics.drill.insert(a, Drill { manager: m, fam: [v; 6], since: w.date });
        tactics::execution(&w, a, &xi, &target, &prof, 60, false)
    };
    let (rehearsed, improvised) = (fam(95), fam(8));
    assert!(rehearsed > improvised + 0.3, "well-drilled {rehearsed}, improvised {improvised}");
    // Half-time is a better place to explain than the touchline late on.
    w.tactics.drill.insert(a, Drill { manager: m, fam: [60; 6], since: w.date });
    assert!(tactics::execution(&w, a, &xi, &target, &prof, 45, true) > tactics::execution(&w, a, &xi, &target, &prof, 80, false));
    // A manager who cannot get a message across loses more of it.
    let mute = tactics::Profile { comms: 0.0, ..prof };
    assert!(tactics::execution(&w, a, &xi, &target, &mute, 60, false) < tactics::execution(&w, a, &xi, &target, &prof, 60, false));
    // The change lands partially: the sliders move only part of the way.
    let part = tactics::blend(base, target, 0.4);
    assert!(part.directness > base.directness && part.directness < target.directness);
    let _ = style;
    // A new manager finds a squad that knows someone else's system.
    let mut w2 = season().clone();
    let old = w2.tactics.drill[&a];
    let other = w2.staff.iter_enumerated().map(|(id, _)| id).find(|&s| s != old.manager).unwrap();
    w2.tactics.drill.get_mut(&a).unwrap().manager = other;
    tactics::weekly(&mut w2);
    let after = w2.tactics.drill[&a];
    assert_eq!(after.manager, old.manager);
    assert!(after.fam.iter().sum::<u8>() < old.fam.iter().sum::<u8>() || after.fam.iter().copied().max() <= Some(40), "{:?} -> {:?}", old.fam, after.fam);
}

#[test]
fn an_opponent_who_reacts_is_answered_at_once_by_a_manager_who_reads_the_game() {
    let mut w = season().clone();
    let (a, _b, sa, sb, fx) = pair(&w);
    shape_manager(&mut w, a, Archetype::Loyalist, 20.0, 10.0);
    let mut first = look(&sa, &sb, 30, false);
    first.sides[1].shots = 7;
    first.sides[1].on_target = 4;
    first.sides[0].shots = 3;
    // Twenty minutes later, the same picture, but the other side has just changed how it plays.
    let mut same = first.clone();
    same.minute = 50;
    let mut changed = same.clone();
    changed.sides[1].tactics = Tactics { press: 90, mentality: Mentality::Attacking, ..changed.sides[1].tactics };
    let count = |second: &Look| {
        let mut n = (0, 0);
        for u in 0..80u64 {
            let mut f = fx.clone();
            f.uid = 300_000 + u;
            let logs = run_call(&w, &f, &sa, &sb, &[first.clone(), second.clone()]);
            for t in &logs.brains[0].as_ref().unwrap().traces {
                n.0 += 1;
                n.1 += usize::from(t.trace.reacting);
            }
        }
        n
    };
    let (quiet, quiet_reacting) = count(&same);
    let (moved, moved_reacting) = count(&changed);
    assert_eq!(quiet_reacting, 0, "nothing to answer");
    assert!(moved_reacting > 20 && moved > quiet + 15, "he answers what they changed: {moved} decisions ({moved_reacting} in reply) against {quiet}");
}

#[test]
fn staff_disagree_with_the_manager_and_with_each_other_and_are_right_or_wrong_by_the_record() {
    let w = season();
    let tr = &w.tactics.traces;
    assert!(tr.iter().any(|t| !t.against.is_empty()), "somebody said something else");
    assert!(tr.iter().any(|t| !t.backed_by.is_empty()), "and somebody agreed");
    let (right, wrong, ignored, heeded): (u32, u32, u32, u32) = w.tactics.credit.values().fold((0, 0, 0, 0), |a, c| (a.0 + u32::from(c.right), a.1 + u32::from(c.wrong), a.2 + u32::from(c.ignored_right), a.3 + u32::from(c.heeded_wrong)));
    assert!(right > 20 && wrong > 20, "assistants were right and wrong: {right}/{wrong}");
    assert!(ignored > 0 && heeded > 0, "some who were right were ignored ({ignored}) and some who were wrong were heeded ({heeded})");
    let standings: Vec<f32> = w.tactics.credit.values().map(Credit::standing).collect();
    let (lo, hi) = (standings.iter().copied().fold(1.0f32, f32::min), standings.iter().copied().fold(0.0f32, f32::max));
    assert!(hi - lo > 0.25, "credibility separates the reliable from the rest: {lo}..{hi}");
}

#[test]
fn a_staff_members_standing_changes_how_much_the_manager_lets_him_move_his_mind() {
    let mut w = season().clone();
    let (a, _b, sa, sb, fx) = pair(&w);
    let (m, _) = tactics::profile(&w, a).unwrap();
    let assistant = w.clubs[a].staff.iter().copied().find(|&s| w.staff[s].role == pw_world::StaffRole::Assistant).expect("an assistant");
    let mut l = look(&sa, &sb, 45, true);
    l.sides[1].shots = 9;
    let followed = |w: &World| (0..100u64).map(|u| { let mut f = fx.clone(); f.uid = 800_000 + u; run_call(w, &f, &sa, &sb, &[l.clone()]) }).map(|lg| lg.brains[0].as_ref().unwrap().traces.iter().map(|t| t.trace.backed_by.len()).sum::<usize>()).sum::<usize>();
    w.tactics.credit.insert((m, assistant), Credit { right: 30, wrong: 0, ignored_right: 0, heeded_wrong: 0 });
    let trusted = followed(&w);
    w.tactics.credit.insert((m, assistant), Credit { right: 0, wrong: 30, ignored_right: 0, heeded_wrong: 30 });
    let doubted = followed(&w);
    assert!(trusted > doubted, "a man who has been right is heard ({trusted}), one who has been wrong less so ({doubted})");
}

#[test]
fn a_club_remembers_its_opponents_and_the_memory_weakens_when_the_man_who_wrote_it_leaves() {
    let mut w = season().clone();
    let (&(club, opp), mem) = w.tactics.memory.iter().find(|&(&(c, _), m)| m.meetings >= 2 && w.staff.get(m.author).is_some_and(|s| s.club == c && !s.retired)).expect("a remembered opponent");
    let mem: OppMemory = *mem;
    let (_, fresh) = tactics::memory_of(&w, club, opp).unwrap();
    w.staff[mem.author].retired = true;
    let (_, after) = tactics::memory_of(&w, club, opp).unwrap();
    assert!(after < fresh * 0.6, "turnover halves it: {fresh} -> {after}");
    assert!(mem.press <= 100 && mem.meetings >= 2);
}

#[test]
fn managers_learn_from_the_scoreboard_as_well_as_from_what_happened_and_sometimes_learn_the_wrong_lesson() {
    let w = season();
    let done = w.tactics.traces.iter().filter(|t| t.by_process.is_some() && t.by_result.is_some()).count();
    let misread = w.tactics.traces.iter().filter(|t| t.misread()).count();
    assert!(done > 150, "{done} judged decisions");
    assert!(misread * 5 > done && misread * 10 < done * 9, "process and result part company often but not always: {misread}/{done}");
    let luck: u16 = w.tactics.managers.values().map(|m| m.credited_luck).sum();
    let good: u16 = w.tactics.managers.values().map(|m| m.good_reads).sum();
    assert!(luck > 0 && good > 0, "luck credited {luck}, good reads {good}");
    assert!(w.tactics.managers.values().any(|m| m.lessons.iter().any(|&l| l > 10)) && w.tactics.managers.values().any(|m| m.lessons.iter().any(|&l| l < -10)), "lessons of both signs were learned");
}

#[test]
fn a_decision_keeps_its_reasons_and_a_public_trace_with_a_cause() {
    let w = season();
    let t = w.tactics.traces.iter().find(|t| t.response.is_structural() && t.rejected.is_some()).expect("a structural change with a rejected explanation");
    let text = tactics::explain(t);
    assert!(text.contains(t.response.label()) && text.contains(t.believed.label()) && text.contains(t.sign.label()), "{text}");
    assert!(t.wanted_minutes > 0 || t.half_time || t.sign == SignKind::Scoreline);
    let seen = w.events.all().iter().filter(|e| matches!(e.kind, EventKind::MatchTacticsChanged { .. })).count();
    assert!(seen > 20, "{seen} changes are on the public record");
    assert!(w.events.all().iter().filter(|e| matches!(e.kind, EventKind::MatchTacticsChanged { .. })).all(|e| e.causes.iter().any(|c| matches!(c, Cause::Fact(Fact::Played { .. })))), "each names the match it happened in");
    // The manager who saw the danger and took it is not the manager who never saw it.
    assert!(w.tactics.traces.iter().any(|t| t.took_risk) && w.tactics.traces.iter().any(|t| matches!(t.response, Response::GoForIt | Response::PressHigher) && !t.saw_risk));
}

#[test]
fn a_tactical_reputation_is_only_what_the_record_supports() {
    let w = season();
    let m = w.clubs.iter().find_map(|c| c.manager.get()).unwrap();
    let mut w = w.clone();
    w.tactics.managers.remove(&m);
    assert!(tactics::reputation(&w, m).is_empty(), "no history, no reputation");
    w.tactics.managers.insert(m, ManagerTactics { changes: 3, good_reads: 3, comebacks: 1, ..Default::default() });
    assert!(tactics::reputation(&w, m).is_empty(), "three changes is not a reputation");
    w.tactics.managers.insert(m, ManagerTactics { changes: 14, good_reads: 9, comebacks: 4, ..Default::default() });
    assert_eq!(tactics::reputation(&w, m), vec!["a brilliant in-game manager"]);
    w.tactics.managers.insert(m, ManagerTactics { changes: 2, stood_pat_behind: 10, ..Default::default() });
    assert_eq!(tactics::reputation(&w, m), vec!["has no Plan B"]);
    w.tactics.managers.insert(m, ManagerTactics { changes: 12, good_reads: 3, misreads: 6, ..Default::default() });
    assert_eq!(tactics::reputation(&w, m), vec!["often reads it wrong"]);
}

#[test]
fn a_manager_may_deliberately_play_against_type_and_the_more_so_the_more_adaptable_and_prepared_he_is() {
    let mut w = season().clone();
    let (a, _b, sa, sb, fx) = pair(&w);
    let c = ctx(&w, &fx);
    let rate = |w: &mut World, skill: f32, temper: f32| {
        shape_manager(w, a, Archetype::Pragmatist, skill, temper);
        let base = w.date;
        let mut n = 0;
        for k in 0..150 {
            w.date = base.add_days(k);
            let (mut x, mut y) = (sa.clone(), sb.clone());
            let preps = tactics::prepare(w, [&mut x, &mut y], &c);
            if let Some(p) = preps[0]
                && p.surprise
            {
                n += 1;
                assert!(x.tactics != sa.tactics, "a surprise is a different way of playing");
            }
        }
        n
    };
    let (open, set) = (rate(&mut w, 20.0, 20.0), rate(&mut w, 2.0, 1.0));
    assert!(open > set + 5, "adaptable and prepared {open}, rigid and careless {set}");
}

#[test]
fn the_same_season_replays_to_the_same_decisions() {
    let run = || {
        let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 41, Scale::MICRO));
        sim.run(150);
        sim.world.tactics.traces.iter().map(|t| (t.uid, t.minute, t.response, t.believed, t.by_process, t.by_result)).collect::<Vec<_>>()
    };
    let (a, b) = (run(), run());
    assert!(!a.is_empty());
    assert_eq!(a, b);
}
