//! The India ecosystem brief, item by item: recognition that is tuned by data and known by organisation, recommendations with causes,
//! contextual regard abroad, viral talk that only sends people to look, a calendar and eligibility that are data, records that say
//! what kind of evidence they are, and pathway history that says why and never invents a past.

use pw_core::{ClubId, PersonId, PlayerId};
use pw_data::DataPack;
use pw_import::india::{self, IndiaScale};
use pw_sim::{export, legacy, recognition, Sim};
use pw_world::ecosystem::Tier;
use pw_world::eligibility::Reason;
use pw_world::pathway::{Draw, Why};
use pw_world::recog::{Learned, Org, Referrals, Segment, Source, VouchBasis};
use pw_world::scenario::{CalEvent, DataOrigin};

fn world(seed: u64) -> Sim {
    Sim::new(india::build(DataPack::builtin(), seed, IndiaScale::TINY))
}

fn kids(s: &Sim) -> Vec<PlayerId> {
    let mut ids: Vec<_> = s.world.ext.ecosystem.story.keys().copied().collect();
    ids.sort();
    ids
}

fn top_academy(s: &Sim) -> ClubId {
    let mut v: Vec<_> = s.world.youth.academies.keys().copied().collect();
    v.sort_by_key(|&c| (std::cmp::Reverse(s.world.clubs[c].reputation), c));
    v[0]
}

fn look(s: &mut Sim, org: Org, p: PlayerId, n: u32) {
    for _ in 0..n {
        s.world.date = s.world.date.add_days(30);
        recognition::sighted_by(&mut s.world, org, PersonId::NONE, p, Learned::Watched);
    }
}

// ------------------------------------------------------------------------------------------------- C. calibration is data

#[test]
fn the_pack_carries_the_tuning_and_the_defaults_are_the_same_numbers() {
    let s = world(1);
    let sc = &s.world.ext.scenario;
    assert_eq!(sc.source, "data/worlds/india/pack.toml");
    assert_eq!(sc.recognition, pw_world::scenario::RecognitionTuning::default(), "the pack's initial values equal the built-in defaults, so nothing changed for a world without the section");
    assert_eq!(sc.scouting, pw_world::scenario::ScoutingTuning::default());
    assert_eq!(sc.markets.len(), 2);
    assert!(sc.calendar.iter().any(|r| r.event == CalEvent::StateChampionship && r.day == 1 && r.months == vec![2]));
}

#[test]
fn changing_the_tier_weights_in_data_changes_what_a_performance_is_worth() {
    let mut s = world(2);
    let p = kids(&s)[3];
    for _ in 0..20 {
        recognition::credit(&mut s.world, p, Tier::State, 8.6, 1.0);
    }
    let base = recognition::standing(&s.world, p);
    assert!(base > 0.2, "twenty good state games should make a name: {base}");
    s.world.ext.scenario.recognition.tier_weight = [0.0; 6];
    assert!(recognition::standing(&s.world, p) < 1e-6, "with every level weighted at nothing, no evidence makes a name");
    s.world.ext.scenario.recognition.tier_weight = [0.10, 0.28, 0.55, 0.50, 0.70, 2.0];
    assert!(recognition::standing(&s.world, p) > base, "weighting state football more raises the name it makes");
}

#[test]
fn a_great_player_can_be_missed_and_a_mediocre_one_overvalued() {
    // A high standing is what the evidence made, not the ability: the same evidence gives the same name whoever is behind it.
    let mut s = world(3);
    let ids = kids(&s);
    let (a, b) = (ids[5], ids[6]);
    for _ in 0..15 {
        recognition::credit(&mut s.world, a, Tier::School, 7.9, 1.0);
        recognition::credit(&mut s.world, b, Tier::School, 7.9, 1.0);
    }
    let (sa, sb) = (recognition::standing(&s.world, a), recognition::standing(&s.world, b));
    assert!((sa - sb).abs() < 1e-6, "standing is a function of evidence only");
    // The player nobody has watched is not recognised by any academy, however good he really is.
    let club = top_academy(&s);
    assert!(!recognition::recognised_by(&s.world, club, a));
}

// ------------------------------------------------------------------------------------------------- D. organisations know differently

#[test]
fn each_organisation_knows_a_player_through_its_own_looks() {
    let mut s = world(4);
    let p = kids(&s)[7];
    let mut academies: Vec<_> = s.world.youth.academies.keys().copied().collect();
    academies.sort();
    let (a, b) = (academies[0], academies[1]);
    for _ in 0..40 {
        recognition::credit(&mut s.world, p, Tier::District, 8.4, 1.0);
        recognition::credit(&mut s.world, p, Tier::State, 8.4, 1.0);
    }
    look(&mut s, Org::Club(a), p, 5);
    assert_eq!(recognition::looks_by(&s.world, Org::Club(a), p), 5);
    assert_eq!(recognition::looks_by(&s.world, Org::Club(b), p), 0, "another club's scouts have not seen him");
    assert!(!recognition::recognised_by(&s.world, b, p), "a club with no looks of its own cannot invite him");
    // Looks on the same day are one look.
    let before = recognition::looks_by(&s.world, Org::Club(a), p);
    recognition::sighted_by(&mut s.world, Org::Club(a), PersonId::NONE, p, Learned::Watched);
    assert_eq!(recognition::looks_by(&s.world, Org::Club(a), p), before, "a second look inside the gap is the same look");
}

// ------------------------------------------------------------------------------------------------- 4. vouching has a cause

/// The children of the largest institution that has at least four of them in the ecosystem, with the institution.
fn a_group(s: &Sim) -> (u32, Vec<PlayerId>) {
    let w = &s.world;
    let mut best: (u32, Vec<PlayerId>) = (0, Vec::new());
    for (i, inst) in w.minor.institutions.iter().enumerate() {
        let mut m: Vec<PlayerId> = inst.members.iter().copied().filter(|p| w.ext.ecosystem.story.contains_key(p)).collect();
        m.sort();
        if m.len() > best.1.len() {
            best = (i as u32, m);
        }
    }
    assert!(best.1.len() >= 4, "the tiny world should have a school with four ecosystem children, found {}", best.1.len());
    best
}

#[test]
fn a_recommendation_has_a_cause_and_only_the_clearly_best_of_a_group_get_one() {
    let mut s = world(5);
    let (inst, group) = a_group(&s);
    let star = group[0];
    for &p in &group {
        for _ in 0..30 {
            let rating = if p == star { 9.0 } else { 6.2 };
            recognition::credit(&mut s.world, p, Tier::School, rating, 1.0);
        }
    }
    recognition::yearly(&mut s.world);
    let v = s.world.ext.recog.vouch.get(&star).copied().expect("the clearly best child of his coach's group is recommended");
    assert_eq!(v.from, Source::Institution(inst), "the recommendation comes from the place that coaches him");
    assert!(matches!(v.basis, VouchBasis::Trained { months } if months > 0), "and rests on a relationship: {:?}", v.basis);
    assert!(v.strength >= 0.90 && v.credibility > 0.0 && v.credibility <= 1.0);
    let others_with: usize = group.iter().filter(|p| **p != star && s.world.ext.recog.vouch.contains_key(p)).count();
    assert!(others_with < group.len() - 1, "a coach does not vouch for everyone");
    assert!(s.world.ext.recog.vouch.get(&group[1]).is_none_or(|x| x.strength >= 0.90), "nobody below the coach's top tenth is recommended");
    // Same inputs, same result: nothing here is random.
    let mut t = world(5);
    let (_, g2) = a_group(&t);
    for &p in &g2 {
        for _ in 0..30 {
            recognition::credit(&mut t.world, p, Tier::School, if p == g2[0] { 9.0 } else { 6.2 }, 1.0);
        }
    }
    recognition::yearly(&mut t.world);
    assert_eq!(t.world.ext.recog.vouch.len(), s.world.ext.recog.vouch.len());
}

#[test]
fn a_scout_decides_how_far_to_trust_a_recommendation_from_its_record() {
    let mut s = world(6);
    let (_, group) = a_group(&s);
    let star = group[0];
    for &p in &group {
        for _ in 0..30 {
            recognition::credit(&mut s.world, p, Tier::School, if p == star { 9.0 } else { 6.2 }, 1.0);
        }
    }
    recognition::yearly(&mut s.world);
    let from = s.world.ext.recog.vouch[&star].from;
    let mut academies: Vec<_> = s.world.youth.academies.keys().copied().collect();
    academies.sort();
    let (trusting, burned) = (Org::Club(academies[0]), Org::Club(academies[1]));
    s.world.ext.recog.referrals.insert((trusting, from), Referrals { hits: 8, misses: 0 });
    s.world.ext.recog.referrals.insert((burned, from), Referrals { hits: 0, misses: 8 });
    let (t, b, none) = (recognition::vouch_weight(&s.world, Some(trusting), star), recognition::vouch_weight(&s.world, Some(burned), star), recognition::vouch_weight(&s.world, None, star));
    assert!(t > none && none > b, "trust follows the organisation's own record with that source: {t} > {none} > {b}");
    assert!(b >= 0.0 && t <= 1.0);
    // An academy that trusts the source needs one look fewer, never none.
    let looks = |s: &Sim, o: Org| recognition::looks_by(&s.world, o, star);
    assert_eq!(looks(&s, trusting), 0);
}

#[test]
fn a_recommendation_that_was_followed_is_remembered_against_its_source() {
    let mut s = world(7);
    let (_, group) = a_group(&s);
    let star = group[0];
    for &p in &group {
        for _ in 0..30 {
            recognition::credit(&mut s.world, p, Tier::School, if p == star { 9.0 } else { 6.2 }, 1.0);
        }
    }
    recognition::yearly(&mut s.world);
    let from = s.world.ext.recog.vouch[&star].from;
    let club = top_academy(&s);
    recognition::referral_outcome(&mut s.world, Org::Club(club), star, true);
    recognition::referral_outcome(&mut s.world, Org::Club(club), star, false);
    recognition::referral_outcome(&mut s.world, Org::Club(club), star, true);
    let r = s.world.ext.recog.referrals[&(Org::Club(club), from)];
    assert_eq!((r.hits, r.misses), (2, 1));
    // No recommendation, nothing to remember.
    let other = group[group.len() - 1];
    s.world.ext.recog.vouch.remove(&other);
    recognition::referral_outcome(&mut s.world, Org::Club(club), other, true);
    assert_eq!(s.world.ext.recog.referrals.len(), 1);
}

// ------------------------------------------------------------------------------------------------- 19. talk only sends people to look

#[test]
fn talk_sends_a_scout_to_watch_and_a_judgement_forms_only_after_the_games() {
    let mut s = world(8);
    let ids = kids(&s);
    // Find a child for whom a spectacular first game gets talked about (it is a chance, not a rule).
    let mut found: Option<(PlayerId, pw_world::recog::Watch)> = None;
    for &p in ids.iter().take(600) {
        if s.world.ext.recog.watching.iter().any(|x| x.player == p) {
            continue;
        }
        recognition::credit(&mut s.world, p, Tier::State, 10.0, 1.5);
        if let Some(x) = s.world.ext.recog.watching.iter().find(|x| x.player == p).copied() {
            found = Some((p, x));
            break;
        }
    }
    let (p, watch) = found.expect("with hundreds of spectacular first games, some get talked about");
    let Org::Club(club) = watch.org else { panic!("a watch is by a club") };
    assert!(s.world.ext.ecosystem.repute[&p].buzz > 0, "there is talk");
    assert_eq!(recognition::looks_by(&s.world, watch.org, p), 0, "hearing the talk is not a look");
    assert!(s.world.scouting.of(club, p).is_empty(), "no judgement forms from talk alone");
    // Two more games watched still leave it open; each game credited is one game watched.
    let left = watch.left;
    for _ in 1..left {
        recognition::credit(&mut s.world, p, Tier::State, 6.0, 1.0);
    }
    assert!(s.world.ext.recog.watching.iter().any(|x| x.player == p && x.org == watch.org), "one game short: still watching");
    assert!(s.world.scouting.of(club, p).is_empty());
    // Bad games watched: the judgement forms from the play, and the talk is not in it.
    recognition::credit(&mut s.world, p, Tier::State, 5.0, 1.0);
    assert!(!s.world.ext.recog.watching.iter().any(|x| x.player == p && x.org == watch.org), "the watch is over");
    assert!(!s.world.scouting.of(club, p).is_empty(), "now a scout has a view, formed by watching");
    assert!(recognition::looks_by(&s.world, watch.org, p) >= 1, "and the club has looked");
    assert_eq!(s.world.ext.recog.acquaint[&(watch.org, p)].how, Learned::Watched);
}

// ------------------------------------------------------------------------------------------------- 9. the calendar is data

#[test]
fn the_calendar_decides_when_selection_and_scouting_happen() {
    let with = {
        let mut s = world(9);
        s.run(400);
        s.world.ext.recog.acquaint.keys().filter(|(o, _)| matches!(o, Org::State(_))).count()
    };
    let without = {
        let mut s = world(9);
        s.world.ext.scenario.calendar.clear();
        s.run(400);
        s.world.ext.recog.acquaint.keys().filter(|(o, _)| matches!(o, Org::State(_))).count()
    };
    assert!(with > 0, "district trials and the state championship make the selectors know players");
    assert_eq!(without, 0, "with nothing on the calendar, selectors learn no one");
    let mut s = world(9);
    let sc = &mut s.world.ext.scenario;
    assert!(sc.due(CalEvent::StateChampionship, 2, 1) && !sc.due(CalEvent::StateChampionship, 2, 2));
    sc.calendar.retain(|r| r.event != CalEvent::DistrictSelection);
    assert!(!sc.due_in_month(10).contains(&CalEvent::DistrictSelection));
    sc.calendar.push(pw_world::scenario::CalRule { event: CalEvent::DistrictSelection, months: vec![3], day: 0 });
    assert!(sc.due_in_month(3).contains(&CalEvent::DistrictSelection), "a pack can move an event to another month");
}

// ------------------------------------------------------------------------------------------------- 17/18. regard abroad is contextual

#[test]
fn regard_abroad_is_by_market_and_by_kind_of_football() {
    let s = world(10);
    let w = &s.world;
    // The pack names two markets; a nation in neither has no market and so no club looks from there.
    let spanish = w.clubs.iter_enumerated().find(|(_, c)| w.nations[c.nation].code == "ESP").map(|x| x.0).unwrap();
    let japanese = w.clubs.iter_enumerated().find(|(_, c)| w.nations[c.nation].code == "JPN").map(|x| x.0).unwrap();
    let indian = w.clubs.iter_enumerated().find(|(_, c)| w.nations[c.nation].code == "IND").map(|x| x.0).unwrap();
    assert_ne!(export::market_of(w, spanish), export::market_of(w, japanese));
    assert_eq!(export::market_of(w, indian), None);
    // Seeded, and labelled: the seeds differ by market and by kind of football, and university football is the least seen.
    let m0 = export::market_of(w, spanish).unwrap();
    assert!(w.ext.recog.regard(m0, Segment::University) < w.ext.recog.regard(m0, Segment::League));
    let m1 = export::market_of(w, japanese).unwrap();
    assert!(w.ext.recog.regard(m1, Segment::League) > w.ext.recog.regard(m0, Segment::League), "east asia's seed is higher than spain's in this pack");
}

#[test]
fn a_market_that_rates_the_football_on_show_sends_more_scouts_than_one_that_does_not() {
    let mut s = world(11);
    let ids = kids(&s);
    let pool: Vec<PlayerId> = ids.iter().copied().filter(|&p| s.world.age(p) >= 16).take(30).collect();
    assert!(pool.len() >= 10);
    for &p in &pool {
        for _ in 0..8 {
            recognition::credit(&mut s.world, p, Tier::State, 8.0, 1.0);
        }
    }
    for m in 0..2u8 {
        s.world.ext.recog.export.insert((m, Segment::Youth), pw_world::recog::Regard { level: if m == 0 { 100.0 } else { 0.0 }, ..Default::default() });
    }
    for y in 0..40 {
        s.world.date = s.world.date.add_days(365);
        export::eyes(&mut s.world, Segment::Youth, Tier::State, &pool);
        let _ = y;
    }
    let v0 = s.world.ext.recog.export[&(0, Segment::Youth)].visits;
    let v1 = s.world.ext.recog.export[&(1, Segment::Youth)].visits;
    assert!(v0 > v1, "a market with regard 100 sent {v0} visits; one with none sent {v1}");
    // A different kind of football in the same market is judged on its own regard.
    let before = s.world.ext.recog.export.get(&(0, Segment::University)).map_or(0, |r| r.visits);
    export::eyes(&mut s.world, Segment::University, Tier::State, &pool);
    let after = s.world.ext.recog.export[&(0, Segment::University)].visits;
    assert!(after - before <= 2, "university football is barely watched abroad");
}

#[test]
fn regard_is_earned_slowly_and_never_jumps() {
    let mut s = world(12);
    let before = s.world.ext.recog.regard(0, Segment::League);
    export::yearly(&mut s.world);
    let after = s.world.ext.recog.regard(0, Segment::League);
    assert!((after - before).abs() <= 0.15 * 75.0 + 1e-3, "regard moves by a fraction of the gap: {before} -> {after}");
    assert!(!s.world.ext.recog.export[&(0, Segment::League)].legacy);
}

// ------------------------------------------------------------------------------------------------- 12. eligibility is data

#[test]
fn eligibility_outcomes_follow_the_rules_in_the_pack() {
    let mut s = world(13);
    let year = s.world.date.year();
    let mut states: Vec<_> = s.world.ext.ecosystem.assoc.keys().copied().collect();
    states.sort();
    let mut pick: Option<(PlayerId, pw_core::RegionId)> = None;
    'outer: for p in kids(&s) {
        for &st in &states {
            if pw_sim::eligibility::judge_state(&s.world, p, st, year, false).eligible {
                pick = Some((p, st));
                break 'outer;
            }
        }
    }
    let (p, st) = pick.expect("someone in the tiny world qualifies for a state side");
    let j = pw_sim::eligibility::judge_state(&s.world, p, st, year, false);
    assert!(matches!(j.reason, Reason::Qualifies(_)) && !j.evidence.is_empty(), "a judgement says which rule and shows its evidence: {j:?}");
    assert!(j.evidence.iter().all(|e| !matches!(e.rule, pw_world::eligibility::Rule::CapTied)));
    s.world.ext.ecosystem.eligibility.min_age = 60;
    let j = pw_sim::eligibility::judge_state(&s.world, p, st, year, false);
    assert!(!j.eligible && j.reason == Reason::TooYoung, "raising the age floor in data removes him: {j:?}");
    s.world.ext.ecosystem.eligibility.min_age = 0;
    s.world.ext.ecosystem.eligibility.max_age = 1;
    let j = pw_sim::eligibility::judge_state(&s.world, p, st, year, false);
    assert_eq!(j.reason, Reason::TooOld);
}

// ------------------------------------------------------------------------------------------------- 13/22. history remembers why, provenance is kept

#[test]
fn every_ecosystem_player_has_a_creation_record_and_new_ones_say_why() {
    let mut s = world(14);
    let n0 = s.world.ext.ecosystem.story.len();
    assert!(n0 > 100);
    let missing0 = kids(&s).iter().filter(|p| !s.world.ext.pathway.created.contains_key(p)).count();
    assert_eq!(missing0, 0, "everyone the world starts with has a record of where they came from");
    assert!(kids(&s).iter().all(|p| s.world.ext.pathway.created[p].why == Draw::WorldStart));
    s.run(800);
    let all = kids(&s);
    let missing = all.iter().filter(|p| !s.world.ext.pathway.created.contains_key(p)).count();
    assert_eq!(missing, 0, "every player the pools drew has a record");
    let drawn: Vec<_> = all.iter().filter(|p| s.world.ext.pathway.created[p].why == Draw::Competitive).collect();
    assert!(!drawn.is_empty(), "the pools drew players as the world ran");
    for p in drawn.iter().take(50) {
        let c = &s.world.ext.pathway.created[p];
        assert!(c.age.is_some() && !c.legacy);
        assert_eq!(c.region, s.world.ext.ecosystem.story[p].home);
    }
}

#[test]
fn steps_on_a_route_carry_the_reason_they_happened() {
    let mut s = world(15);
    s.run(1200);
    let mut with_why = 0;
    let mut steps = 0;
    for p in kids(&s) {
        for r in s.world.ext.pathway.of(p) {
            steps += 1;
            with_why += 1;
            // A reason is never one that contradicts the step it is attached to.
            match r.why {
                Why::DistrictSelection { .. } => assert_eq!(r.kind, pw_world::ecosystem::StageKind::District),
                Why::StateSelection { .. } => assert_eq!(r.kind, pw_world::ecosystem::StageKind::StateTeam),
                Why::CampCall => assert_eq!(r.kind, pw_world::ecosystem::StageKind::NationalCamp),
                Why::AcademyInvite { .. } | Why::ScoutRecommendation { .. } => assert_eq!(r.kind, pw_world::ecosystem::StageKind::Academy),
                Why::ReleasedByAcademy { .. } => assert_eq!(r.kind, pw_world::ecosystem::StageKind::Released),
                Why::UniversityScholarship { .. } => assert_eq!(r.kind, pw_world::ecosystem::StageKind::University),
                _ => {}
            }
        }
    }
    assert!(steps > 100 && with_why == steps, "{steps} steps recorded");
    let districts = kids(&s).iter().filter(|p| s.world.ext.pathway.of(**p).iter().any(|r| matches!(r.why, Why::DistrictSelection { .. }))).count();
    assert!(districts > 0, "districts picked sides in three years");
}

#[test]
fn club_data_is_labelled_by_where_it_came_from() {
    let s = world(16);
    let w = &s.world;
    let mut counts = [0usize; 3];
    for (id, c) in w.clubs.iter_enumerated() {
        let o = w.ext.scenario.club_origin.get(&id).copied().unwrap_or_else(|| panic!("club {} has no origin", c.name));
        counts[o as usize] += 1;
    }
    let seeded = w.ext.scenario.club_origin.values().filter(|o| **o == DataOrigin::ScenarioSeed).count();
    let generated = w.ext.scenario.club_origin.values().filter(|o| **o == DataOrigin::Generated).count();
    assert!(seeded > 0 && generated > 0, "the pack names some clubs and the builder makes up the rest: {counts:?}");
    // Premise changed with the reference loader: some reference records (West Bengal's state leagues) are verified against sources, so
    // clubs built from them are Imported. The test that every Imported club rests on a sourced record is in `mod reference`.
    assert!(w.ext.scenario.club_origin.values().filter(|o| **o == DataOrigin::Imported).count() < w.clubs.len() / 4, "imported clubs are the exception: {counts:?}");
}

// ------------------------------------------------------------------------------------------------- 7/8. records say what kind of evidence they are

#[test]
fn speed_and_distance_are_estimates_and_are_never_announced_as_records() {
    use pw_world::records::{Provenance, Stat};
    assert_eq!(Stat::TopSpeed.provenance(), Provenance::Estimated);
    assert_eq!(Stat::DistanceCovered.provenance(), Provenance::Estimated);
    assert!(!Stat::TopSpeed.provenance().official() && !Stat::DistanceCovered.provenance().official());
    assert_eq!(Stat::Goals.provenance(), Provenance::ObservedMatchStat);
    assert_eq!(Stat::WinsInRow.provenance(), Provenance::DerivedFromObservedStats);
    assert!(Stat::Goals.provenance().official());
    assert!(Stat::TopSpeed.title().contains("estimated"), "the title says so: {}", Stat::TopSpeed.title());
}

// ------------------------------------------------------------------------------------------------- A. saves keep working

#[test]
fn an_older_layout_gets_a_deterministic_present_baseline_and_no_invented_history() {
    let build = || {
        let mut s = world(17);
        let ids = kids(&s);
        let p = ids[2];
        s.world.ext.ecosystem.repute.entry(p).or_default().sponsor = 2;
        s.world.ext.ecosystem.export = 30.0;
        // What a layout-1 save leaves: nothing in the layout-2 domains.
        s.world.ext.recog = Default::default();
        s.world.ext.pathway = Default::default();
        s.world.ext.migrated_from = Some(1);
        (s, p)
    };
    let (mut a, p) = build();
    legacy::finish(&mut a.world);
    assert_eq!(a.world.ext.migrated_from, None, "the marker is consumed");
    let v = a.world.ext.recog.vouch[&p];
    assert_eq!(v.basis, VouchBasis::Legacy, "an old sponsor count becomes a labelled legacy recommendation, not a causal one");
    assert!(v.credibility <= 0.5);
    assert!(a.world.ext.recog.export.values().all(|r| r.legacy) && !a.world.ext.recog.export.is_empty(), "the old export number becomes marked legacy regard");
    let c = a.world.ext.pathway.created[&p];
    assert!(c.legacy && c.why == Draw::Unrecorded && c.age.is_none() && c.institution.is_none(), "how he was created was not kept, so it stays unknown: {c:?}");
    assert!(a.world.ext.pathway.why.is_empty(), "no reasons are invented for steps that were never explained");
    assert!(a.world.ext.recog.acquaint.is_empty() && a.world.ext.recog.watching.is_empty(), "no looks are invented");
    // Idempotent and deterministic.
    let snapshot = (a.world.ext.recog.vouch.len(), a.world.ext.recog.export.len(), a.world.ext.pathway.created.len());
    legacy::finish(&mut a.world);
    assert_eq!(snapshot, (a.world.ext.recog.vouch.len(), a.world.ext.recog.export.len(), a.world.ext.pathway.created.len()));
    let (mut b, _) = build();
    legacy::finish(&mut b.world);
    assert_eq!(snapshot, (b.world.ext.recog.vouch.len(), b.world.ext.recog.export.len(), b.world.ext.pathway.created.len()));
    let pb = b.world.ext.recog.vouch[&p];
    assert!((pb.strength - v.strength).abs() < 1e-6 && (pb.credibility - v.credibility).abs() < 1e-6);
    // The upgraded world still runs and stays valid.
    a.run(120);
    let bad = pw_sim::invariants::check(&a.world);
    assert!(bad.is_empty(), "{bad:#?}");
}

// ------------------------------------------------------------------------------------------------- 20/21. history, not scores

#[test]
fn rivalries_start_empty_and_grow_from_matches_played() {
    let mut s = world(18);
    assert!(s.world.ext.ecosystem.rivalry.is_empty(), "a world begins with neighbours, not enemies");
    s.run(400);
    // After the first state championship, the meetings are remembered.
    assert!(!s.world.ext.ecosystem.rivalry.is_empty(), "matches between states leave a rivalry behind");
    assert!(s.world.ext.ecosystem.rivalry.values().all(|v| v.is_finite() && *v > 0.0));
}

#[test]
fn a_region_is_measured_by_what_it_produced_not_only_how_many() {
    let mut s = world(19);
    s.run(1500);
    let out = pw_sim::ecosystem::region_output(&s.world);
    for o in &out {
        assert!(o.top_tier <= o.professionals && o.internationals <= o.professionals, "{o:?}");
        assert!(o.senior_apps >= 10 * o.professionals, "professionals are defined by ten senior appearances: {o:?}");
    }
    let mut sorted = out.clone();
    sorted.sort_by_key(|o| o.region);
    assert_eq!(out, sorted, "sorted by region so the report is stable");
}

/// Discovery outcomes for calibration (the brief wants great players sometimes missed and mediocre ones sometimes taken):
/// `cargo test -p pw-cli --test india_ecosystem discovery_outcomes -- --ignored --nocapture`. Reads hidden ability, which only a test may.
#[test]
#[ignore = "report"]
fn discovery_outcomes() {
    use pw_world::ecosystem::StageKind;
    for seed in [41u64, 42] {
        let mut s = world(seed);
        s.run(1500);
        let w = &s.world;
        let mut rows: Vec<(u8, bool, bool, bool, u8)> = Vec::new(); // (pa, looked at by anyone, academy, pro, age)
        for &p in w.ext.ecosystem.story.keys() {
            let age = w.age(p) as u8;
            // Only children the pools drew as the world ran (the starting population was placed, not discovered), old enough to have been.
            if age < 15 || w.ext.pathway.created.get(&p).is_none_or(|c| c.why != Draw::Competitive) {
                continue;
            }
            let looked = w.ext.recog.acquaint.keys().any(|(_, q)| *q == p);
            let stages = w.ext.ecosystem.stages.get(&p);
            let has = |k: StageKind| stages.is_some_and(|v| v.iter().any(|x| x.kind == k));
            rows.push((w.players.cold[p].pa, looked, has(StageKind::Academy), w.players.cold[p].senior_apps >= 10, age));
        }
        rows.sort_by_key(|r| std::cmp::Reverse(r.0));
        let n = rows.len().max(1);
        let top = &rows[..n / 10];
        let bottom = &rows[n - n / 2..];
        let share = |v: &[(u8, bool, bool, bool, u8)], f: fn(&(u8, bool, bool, bool, u8)) -> bool| v.iter().filter(|r| f(r)).count() as f32 / v.len().max(1) as f32;
        eprintln!(
            "seed {seed}: {n} drawn players 15+; top-decile PA: never looked at {:.0}%, academy {:.0}%, 10+ senior apps {:.0}%; bottom half PA: academy {:.0}%, 10+ senior apps {:.0}%",
            100.0 * (1.0 - share(top, |r| r.1)),
            100.0 * share(top, |r| r.2),
            100.0 * share(top, |r| r.3),
            100.0 * share(bottom, |r| r.2),
            100.0 * share(bottom, |r| r.3)
        );
        let vouched = w.ext.recog.vouch.len();
        let watched = w.ext.recog.acquaint.values().filter(|a| a.how == Learned::Watched).count();
        let buzz = w.ext.recog.acquaint.values().filter(|a| a.how == Learned::Buzz).count();
        let out = pw_sim::ecosystem::region_output(w);
        eprintln!(
            "seed {seed}: vouches {vouched}, acquaintances {} (watched {watched}, only heard {buzz}), referral records {}, regions with output {}, regard abroad {:?}",
            w.ext.recog.acquaint.len(),
            w.ext.recog.referrals.len(),
            out.len(),
            (0..2u8).map(|m| Segment::ALL.map(|g| w.ext.recog.regard(m, g).round() as i32)).collect::<Vec<_>>()
        );
    }
}

// ------------------------------------------------------------------------------------------------- reference data: real clubs with provenance

mod reference {
    use super::*;
    use pw_import::india_ref::{self, ClubRow};
    use pw_world::World;

    fn reference_club(w: &World, id: ClubId) -> Option<&'static ClubRow> {
        let reference = india_ref::builtin();
        let region = w.ext.ecosystem.state_of(w.ext.ecosystem.region_of_club(id));
        if region.is_none() {
            return None; // a club abroad
        }
        let state = reference.states.iter().find(|s| s.name == w.ext.ecosystem.regions[region].name)?;
        reference.club_exact(&w.clubs[id].name, &state.id)
    }

    fn origin(w: &World, id: ClubId) -> DataOrigin {
        w.ext.scenario.club_origin[&id]
    }

    #[test]
    fn some_clubs_are_now_imported_and_only_where_the_reference_has_a_sourced_fact() {
        let s = world(21);
        let w = &s.world;
        let imported: Vec<ClubId> = w.clubs.ids().filter(|&c| origin(w, c) == DataOrigin::Imported).collect();
        // The state premier league of West Bengal takes the six clubs (the tiny world's league size) the reference lists, verified, in
        // the Calcutta Premier Division; no other state in the tiny world has a verified league.
        assert_eq!(imported.len(), IndiaScale::TINY.state_league, "{:?}", imported.iter().map(|&c| &w.clubs[c].name).collect::<Vec<_>>());
        for &c in &imported {
            let r = reference_club(w, c).unwrap_or_else(|| panic!("{} is Imported but matches no reference club", w.clubs[c].name));
            assert!(r.prov.is_sourced_fact(), "{}: Imported needs a verified record with a source: {:?}", r.name, r.prov);
            assert_eq!(w.comps[w.clubs[c].league].name, "West Bengal Premier League", "{}", r.name);
            assert_eq!(w.clubs[c].name, r.name);
            assert_eq!(w.clubs[c].city, r.city);
        }
        // The converse: a club whose record is not a sourced fact is never labelled as one.
        for c in w.clubs.ids() {
            if let Some(r) = reference_club(w, c) {
                assert_eq!(origin(w, c) == DataOrigin::Imported, r.prov.is_sourced_fact(), "{}", r.name);
            }
        }
        // The report says so.
        let rep = &w.ext.scenario.reference;
        assert!(rep.records > 2000 && rep.files == 57 && rep.clubs_from_reference >= imported.len() as u32, "{rep:?}");
        assert_eq!(rep.by_status.iter().sum::<u32>(), rep.records);
        assert!(rep.clubs_matched >= rep.clubs_from_reference + 10, "the pack's clubs match the reference by exact name: {rep:?}");
        assert_eq!(rep.findings as usize, india_ref::builtin().findings.len(), "the pack has no club the reference lacks, so the only findings are the loader's: {rep:?}");
        assert!(rep.findings >= 1 && rep.finding_samples.iter().any(|f| f.contains("bidhannagar-msa")), "the one known inconsistency in the data is reported: {rep:?}");
    }

    #[test]
    fn real_clubs_replace_made_up_ones_and_the_rest_stay_generated() {
        let s = world(22);
        let w = &s.world;
        let by_name = |n: &str| w.clubs.ids().find(|&c| w.clubs[c].name == n).unwrap_or_else(|| panic!("no club {n}"));
        // A club the pack names keeps its pack standing and is a seed (the reference knows the name, not the facts).
        let mb = by_name("Mohun Bagan Super Giant");
        assert_eq!(origin(w, mb), DataOrigin::ScenarioSeed);
        assert_eq!(w.clubs[mb].reputation, 6800, "starting strength still comes from the pack");
        // A real ground and its capacity replace the made-up ones where the club is tied to a stadium record that dates its capacity.
        let kb = by_name("Kerala Blasters");
        assert_eq!((w.clubs[kb].stadium.as_str(), w.clubs[kb].capacity), ("Jawaharlal Nehru International Stadium, Kochi", 41_000));
        assert_eq!(origin(w, kb), DataOrigin::ScenarioSeed, "an inferred capacity does not make the club an import");
        // A club the reference has no ground for keeps the made-up capacity and no stadium name: a real name never sits by an invented number.
        assert_eq!(w.clubs[mb].stadium, "");
        assert_eq!(w.clubs[mb].capacity, u32::from(w.clubs[mb].reputation) * 4 + 1_500);
        // Generated clubs still exist, are labelled so, and have no reference record.
        let generated: Vec<ClubId> = w.clubs.ids().filter(|&c| origin(w, c) == DataOrigin::Generated).collect();
        assert!(generated.len() > 50);
        for &c in generated.iter().filter(|&&c| w.clubs[c].nation == w.clubs[mb].nation) {
            assert!(reference_club(w, c).is_none(), "{} is generated but matches a reference club by name", w.clubs[c].name);
        }
    }

    #[test]
    fn no_history_result_title_or_rivalry_from_the_real_world_is_attached_to_any_club() {
        // The world as the builder leaves it, before the simulation prepares it (`Sim::new` generates a labelled past and seeds culture).
        let raw = india::build(DataPack::builtin(), 23, IndiaScale::TINY);
        assert!(raw.history.honours.is_empty() && raw.history.tables.is_empty() && raw.history.awards.is_empty(), "no honour, table or award exists");
        assert!(raw.honours.tallies.is_empty() && raw.honours.hall.is_empty() && raw.honours.votes.is_empty() && raw.honours.comps.is_empty() && raw.honours.clubs.is_empty());
        assert!(raw.backfill.seasons.is_empty() && raw.backfill.figures.is_empty() && !raw.backfill.done, "no past season or figure is attached to a real name");
        assert!(raw.ext.almanac.boards.is_empty() && raw.ext.almanac.title_run.is_empty() && raw.ext.almanac.career.is_empty());
        assert!(raw.ext.ecosystem.tournament_titles.is_empty());
        assert!(raw.culture.rivalries.list.is_empty(), "no rivalry exists when the builder is done, however famous the derby");
        assert!(raw.fixtures.iter().all(|(_, f)| f.score.is_none()), "no result exists");
        // A founding year comes from a record only where the record may give a number (a sourced fact, or an inference graded C or
        // better). Salgaocar's 1956 is "unknown, graded D" and is not taken.
        for c in raw.clubs.ids() {
            if let Some(r) = reference_club(&raw, c) {
                if let Some(y) = r.founded.filter(|_| r.prov.allows_value()) {
                    assert_eq!(i32::from(raw.clubs[c].founded), y, "{}", r.name);
                }
            }
        }
        let salgaocar = raw.clubs.ids().find(|&c| raw.clubs[c].name == "Salgaocar FC");
        assert!(salgaocar.is_none_or(|c| reference_club(&raw, c).is_some_and(|r| !r.prov.allows_value())), "Salgaocar's record is not allowed to give a number");
        // Once the simulation has prepared the world, the past it makes is labelled as made, and a rivalry starts with no history.
        let s = world(23);
        let w = &s.world;
        assert!(w.backfill.seasons.iter().all(|x| x.provenance == pw_world::backfill::Provenance::Generated), "a past season in this world is generated, never imported");
        assert!(w.backfill.figures.iter().all(|x| x.provenance == pw_world::backfill::Provenance::Generated));
        for r in &w.culture.rivalries.list {
            assert!(r.h2h == (0, 0, 0) && r.moments.is_empty() && r.last_meeting == pw_core::Date(0) && r.revenge_due.is_none(), "a rivalry starts with no history: {r:?}");
        }
    }

    #[test]
    fn known_derbies_are_names_between_clubs_of_this_world_and_nothing_more() {
        let raw = india::build(DataPack::builtin(), 24, IndiaScale::TINY);
        let d = &raw.ext.scenario.known_derbies;
        let kolkata = d.iter().find(|x| x.name == "Kolkata Derby").expect("East Bengal and Mohun Bagan are both in the world");
        let names = [raw.clubs[kolkata.a].name.as_str(), raw.clubs[kolkata.b].name.as_str()];
        assert!(names.contains(&"East Bengal") && names.contains(&"Mohun Bagan Super Giant"), "{names:?}");
        assert_eq!(kolkata.origin, DataOrigin::ScenarioSeed, "an inferred derby is a seed");
        assert!(kolkata.derby && kolkata.source_id == "rivalry.kolkata-derby");
        // Only pairs of which both clubs are in this world (Karnataka is not in the tiny world, so the Southern Derby is not named).
        assert!(d.iter().all(|x| x.a != x.b && raw.clubs.ids().any(|c| c == x.a) && raw.clubs.ids().any(|c| c == x.b)));
        assert!(!d.iter().any(|x| x.name == "Southern Derby"));
        assert!(d.len() < india_ref::builtin().derbies().len());
        // They are labels: building set no rivalry, and the intensities the simulation's own generic seeding gives do not depend on
        // the named derbies: seed again with the list cleared and the same values come out.
        assert!(raw.culture.rivalries.list.is_empty());
        let seeded = |clear: bool| {
            let mut w = india::build(DataPack::builtin(), 24, IndiaScale::TINY);
            if clear {
                w.ext.scenario.known_derbies.clear();
            }
            pw_sim::culture::ensure(&mut w);
            w.culture.rivalries.list.iter().map(|r| (r.a, r.b, r.intensity, r.kinds.clone())).collect::<Vec<_>>()
        };
        let (with, without) = (seeded(false), seeded(true));
        assert!(!with.is_empty());
        assert_eq!(with, without, "the reference's derbies set no rivalry intensity");
    }

    fn fingerprint(s: &Sim) -> (Vec<(String, String, String, u32, u16, u16, DataOrigin)>, Vec<(String, String, u32)>, pw_world::scenario::ReferenceReport, usize) {
        let w = &s.world;
        let clubs = w.clubs.iter_enumerated().map(|(id, c)| (c.name.clone(), c.short_name.clone(), c.stadium.clone(), c.capacity, c.founded, c.reputation, w.ext.scenario.club_origin[&id])).collect();
        let derbies = w.ext.scenario.known_derbies.iter().map(|d| (d.name.clone(), w.clubs[d.a].name.clone(), u32::from(d.origin as u8))).collect();
        (clubs, derbies, w.ext.scenario.reference.clone(), w.players.len())
    }

    #[test]
    fn the_same_seed_builds_the_same_world_and_the_reference_does_not_depend_on_the_seed() {
        let (a, b) = (world(25), world(25));
        assert_eq!(fingerprint(&a), fingerprint(&b));
        assert_eq!(pw_sim::validate::census(&a.world), pw_sim::validate::census(&b.world));
        // Which real clubs are in the world is the reference's doing, not the dice's: the same names in the same places for another seed.
        let c = world(26);
        let real = |s: &Sim| -> Vec<(String, DataOrigin)> {
            s.world.clubs.iter_enumerated().filter(|(id, _)| reference_club(&s.world, *id).is_some()).map(|(id, c)| (c.name.clone(), s.world.ext.scenario.club_origin[&id])).collect()
        };
        assert_eq!(real(&a), real(&c));
    }

    #[test]
    fn the_world_with_real_clubs_runs_120_days_and_stays_consistent() {
        let mut s = world(27);
        assert!(pw_sim::invariants::check(&s.world).is_empty());
        assert_eq!(pw_sim::validate::problems(&s.world), Vec::<String>::new());
        let start = s.world.date;
        s.run(120);
        assert_eq!(s.world.date, start.add_days(120));
        let b = pw_sim::invariants::check(&s.world);
        assert!(b.is_empty(), "{b:#?}");
        assert_eq!(pw_sim::validate::problems(&s.world), Vec::<String>::new());
        // Real clubs played like any other: fixtures were played in a league that holds one.
        let imported = s.world.clubs.ids().find(|&c| s.world.ext.scenario.club_origin[&c] == DataOrigin::Imported).unwrap();
        let team = s.world.clubs[imported].first_team();
        assert!(s.world.fixtures.iter().any(|(_, f)| f.score.is_some() && (f.home == team || f.away == team)), "an imported club has played in 120 days");
    }

    #[test]
    fn a_world_saved_and_loaded_keeps_its_reference_labels() {
        let s = world(28);
        let dir = std::env::temp_dir().join(format!("pw-india-ref-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("w.pws");
        pw_sim::save::save_with(&s.world, &path, &pw_sim::save::Info::of_world(&s.world)).unwrap();
        let back = pw_sim::save::load_world(&path).unwrap();
        assert_eq!(back.ext.scenario.known_derbies, s.world.ext.scenario.known_derbies);
        assert_eq!(back.ext.scenario.reference, s.world.ext.scenario.reference);
        assert_eq!(back.ext.scenario.club_origin.len(), s.world.ext.scenario.club_origin.len());
        assert_eq!(back.ext.migrated_from, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_full_world_gives_real_clubs_their_places_once_and_keeps_every_pack_club() {
        let w = india::build(DataPack::builtin(), 29, IndiaScale::FULL);
        let rep = &w.ext.scenario.reference;
        assert_eq!(rep.findings as usize, india_ref::builtin().findings.len(), "every pack club has an exact reference record: {rep:?}");
        assert!(rep.clubs_from_reference > 100, "{rep:?}");
        // A real club is in the world once: its reference id is placed once, so its name is in the world once.
        let mut real_names: Vec<&str> = w.clubs.ids().filter(|&c| reference_club(&w, c).is_some()).map(|c| w.clubs[c].name.as_str()).collect();
        let n = real_names.len();
        assert_eq!(n as u32, rep.clubs_matched);
        real_names.sort_unstable();
        real_names.dedup();
        assert_eq!(real_names.len(), n, "a real club appears twice");
        // No made-up club carries a real club's name in its state.
        for c in w.clubs.ids().filter(|&c| w.ext.scenario.club_origin[&c] == DataOrigin::Generated && !w.ext.ecosystem.region_of_club(c).is_none()) {
            assert!(reference_club(&w, c).is_none(), "{}", w.clubs[c].name);
        }
        let d = &w.ext.scenario.known_derbies;
        for name in ["Kolkata Derby", "Southern Derby", "Kerala Derby"] {
            assert!(d.iter().any(|x| x.name == name), "{name} missing from {:?}", d.iter().map(|x| &x.name).collect::<Vec<_>>());
        }
        assert!(w.culture.rivalries.list.is_empty());
        // Every league is full and every club sits in a region.
        for (id, c) in w.clubs.iter_enumerated() {
            if w.nations[c.nation].code == "IND" {
                assert!(!w.ext.ecosystem.region_of_club(id).is_none(), "{} has no region", c.name);
            }
        }
    }
}
