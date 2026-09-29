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
    assert!(v.strength >= 0.75 && v.credibility > 0.0 && v.credibility <= 1.0);
    let others_with: usize = group.iter().filter(|p| **p != star && s.world.ext.recog.vouch.contains_key(p)).count();
    assert!(others_with < group.len() - 1, "a coach does not vouch for everyone");
    assert!(s.world.ext.recog.vouch.get(&group[1]).is_none_or(|x| x.strength >= 0.75), "nobody below the coach's top quarter is recommended");
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
    assert_eq!(w.ext.scenario.club_origin.values().filter(|o| **o == DataOrigin::Imported).count(), 0, "nothing in this pack is a verified import");
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
