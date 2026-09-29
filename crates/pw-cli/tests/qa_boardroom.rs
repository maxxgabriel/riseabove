//! QA: boardroom invariants over a long run. Case files must agree with themselves, verdicts arrive only after a season, appetite is
//! exactly the sum of its stated drivers, and institutional power is always a proper distribution.

mod qa_common;

use pw_import::synthetic::Scale;
use pw_sim::{boardroom, dossier};
use pw_world::boardroom::{CaseState, Voice};
use pw_world::World;
use qa_common::*;

fn check_cases(w: &World, failures: &mut Vec<String>) -> usize {
    let mut n = 0;
    for (i, c) in w.boardroom.cases.iter().enumerate() {
        n += 1;
        let mut bad = |m: String| failures.push(format!("case {i} ({:?} for {:?} at {:?}, {:?}): {m}", c.player, c.date, c.buyer, c.state));
        if c.date.0 > w.date.0 {
            bad("filed in the future".into());
        }
        if c.buyer.0 as usize >= w.clubs.len() || c.player.0 as usize >= w.players.hot.len() {
            bad("names an unknown club or player".into());
            continue;
        }
        // Stances: valid range, one per voice, powers sum to one.
        let mut voices = std::collections::HashSet::new();
        for s in &c.stances {
            if !(-1.0001..=1.0001).contains(&s.support) {
                bad(format!("{:?} support {} out of range", s.voice, s.support));
            }
            if !(0.0..=1.0001).contains(&s.power) {
                bad(format!("{:?} power {} out of range", s.voice, s.power));
            }
            if !voices.insert(s.voice) {
                bad(format!("{:?} takes two stances", s.voice));
            }
            if s.voice != Voice::Supporters && s.who.is_none() && s.voice != Voice::Analyst {
                // The supporters have no person; everyone else who speaks is somebody.
                bad(format!("{:?} spoke but is nobody", s.voice));
            }
        }
        if !c.stances.is_empty() {
            let total: f32 = c.stances.iter().map(|s| s.power).sum();
            if (total - 1.0).abs() > 1e-3 {
                bad(format!("powers sum to {total}"));
            }
            let score: f32 = c.stances.iter().map(|s| s.support * s.power).sum();
            if (score - c.score).abs() > 1e-3 {
                bad(format!("recorded score {} but the stances give {score}", c.score));
            }
        }
        // The decision agrees with the numbers behind it.
        let vetoed = c.stances.iter().any(|s| s.voice == Voice::Owner && s.support < -0.6) && w.governance.get(&c.buyer).is_some_and(|g| g.owner.meddling >= 60);
        match c.state {
            CaseState::Declined => {
                if c.score >= c.threshold && !vetoed {
                    bad(format!("declined with score {} over the bar {} and no veto", c.score, c.threshold));
                }
                if !c.risks_accepted.is_empty() {
                    bad("declined but lists accepted risks".into());
                }
                if c.signed.is_some() || c.outcome.is_some() {
                    bad("declined yet signed or judged".into());
                }
            }
            CaseState::Pursuing | CaseState::Signed | CaseState::Collapsed => {
                if c.score + 1e-4 < c.threshold {
                    bad(format!("pursued with score {} under the bar {}", c.score, c.threshold));
                }
            }
        }
        if c.risks_accepted.iter().any(|r| !c.risks_known.contains(r)) {
            bad("accepted a risk it never knew about".into());
        }
        if c.risks_known.iter().any(|r| c.risks_unknown.contains(r)) {
            bad("a risk is both known and unknown".into());
        }
        if !(0.05 - 1e-4..=0.95 + 1e-4).contains(&c.appetite) {
            bad(format!("appetite {} outside 0.05..0.95", c.appetite));
        }
        if c.state != CaseState::Signed && (c.signed.is_some() || c.outcome.is_some() || c.price_paid != 0) {
            bad("not signed yet carries a signing date, price or outcome".into());
        }
        if c.state == CaseState::Signed && c.signed.is_none() {
            bad("signed without a date".into());
        }
        if let Some(d) = c.signed
            && d.0 < c.date.0
        {
            bad("signed before the decision".into());
        }
        // Verdicts only after a season: at least 300 days from the signing, judged from what was known at the time.
        if let Some(o) = c.outcome {
            let signed = c.signed.expect("outcome implies signed (checked above)");
            if signed.days_until(o.date) < 300 {
                bad(format!("judged {} days after signing", signed.days_until(o.date)));
            }
            if !(-1.0001..=1.0001).contains(&o.success) {
                bad(format!("success {} out of range", o.success));
            }
            if boardroom::verdict(c, o.success, &o.materialised) != o.verdict {
                bad(format!("verdict {:?} does not follow from success {} and the process on file", o.verdict, o.success));
            }
        }
    }
    n
}

fn check_book(w: &World, failures: &mut Vec<String>) {
    for (club, a) in &w.boardroom.appetite {
        let sum: f32 = a.drivers.iter().map(|d| d.1).sum();
        let want = (0.5 + sum).clamp(0.05, 0.95);
        if (a.level - want).abs() > 1e-4 {
            failures.push(format!("club {club:?}: appetite {} is not 0.5 plus its drivers ({want})", a.level));
        }
        if a.as_of.0 > w.date.0 {
            failures.push(format!("club {club:?}: appetite from the future"));
        }
    }
    for ((club, voice), v) in &w.boardroom.authority {
        if !(-0.3001..=0.3001).contains(v) {
            failures.push(format!("club {club:?}: authority of {voice:?} is {v}"));
        }
    }
    for (club, h) in &w.boardroom.honesty {
        if !(0.0..=1.0).contains(h) {
            failures.push(format!("club {club:?}: honesty {h}"));
        }
    }
    if w.boardroom.blocked.len() > 400 {
        failures.push(format!("{} blocked-youngster records", w.boardroom.blocked.len()));
    }
    // Power is a distribution for every club and every combination of voices in the room.
    for club in w.clubs.ids().take(12) {
        for mask in 0u32..128 {
            let present: [bool; 7] = std::array::from_fn(|i| mask & (1 << i) != 0);
            let p = boardroom::powers(w, club, &present);
            let total: f32 = p.iter().sum();
            let any = present.iter().any(|&x| x);
            if any && (total - 1.0).abs() > 1e-4 {
                failures.push(format!("club {club:?}, mask {mask:07b}: powers sum to {total}"));
            }
            if !any && total != 0.0 {
                failures.push(format!("club {club:?}: an empty room has power {total}"));
            }
            for i in 0..7 {
                if !present[i] && p[i] != 0.0 {
                    failures.push(format!("club {club:?}, mask {mask:07b}: absent voice {i} holds power {}", p[i]));
                }
                if p[i] < 0.0 {
                    failures.push(format!("club {club:?}: negative power"));
                }
            }
        }
    }
}

#[test]
fn case_files_and_the_book_stay_consistent_over_three_seasons() {
    let mut s = sim(Scale::TINY, 111);
    let mut failures = Vec::new();
    let (mut cases_seen, mut judged) = (0usize, 0usize);
    for _ in 0..11 {
        s.run(100);
        cases_seen = cases_seen.max(check_cases(&s.world, &mut failures));
        check_book(&s.world, &mut failures);
        judged = judged.max(s.world.boardroom.cases.iter().filter(|c| c.outcome.is_some()).count());
    }
    assert!(failures.is_empty(), "{} inconsistencies, first {:?}", failures.len(), &failures[..failures.len().min(10)]);
    assert!(cases_seen > 0, "three seasons of a tiny world produce at least one case file");
    eprintln!("qa_boardroom: max cases held {cases_seen}, judged {judged}");
}

/// Heavier: small world, four seasons, checks every 200 days. `--ignored` (about a minute in release).
#[test]
#[ignore = "heavy: four seasons of the small world"]
fn case_files_stay_consistent_on_the_small_world() {
    let mut s = sim(Scale::SMALL, 112);
    let mut failures = Vec::new();
    let mut judged = 0usize;
    for _ in 0..7 {
        s.run(200);
        check_cases(&s.world, &mut failures);
        check_book(&s.world, &mut failures);
        judged = judged.max(s.world.boardroom.cases.iter().filter(|c| c.outcome.is_some()).count());
    }
    assert!(failures.is_empty(), "{} inconsistencies, first {:?}", failures.len(), &failures[..failures.len().min(10)]);
    assert!(judged > 0, "at least one signing was judged in four seasons");
}

/// A dossier is a reading by named evaluators: the weights add up, the ceiling is not below the current level and every span is sane.
#[test]
fn dossiers_are_internally_consistent_after_a_season() {
    let s = ran(Scale::TINY, 113, 300);
    let w = &s.world;
    assert!(w.dossiers.map.len() > 50);
    let mut failures = Vec::new();
    for (&(club, p), d) in w.dossiers.map.iter() {
        let mut bad = |m: String| failures.push(format!("dossier {club:?}/{p:?}: {m}"));
        if d.club != club || d.player != p {
            bad("filed under the wrong key".into());
        }
        if d.ceiling.mid + 1e-3 < d.current.mid {
            bad(format!("ceiling {} below current {}", d.ceiling.mid, d.current.mid));
        }
        if d.current.band < 0.0 || d.ceiling.band < 0.0 || !d.current.mid.is_finite() || !d.ceiling.mid.is_finite() {
            bad("a span is negative or not finite".into());
        }
        if !d.opinions.is_empty() {
            let total: f32 = d.opinions.iter().map(|o| o.weight).sum();
            if (total - 1.0).abs() > 1e-2 {
                bad(format!("opinion weights sum to {total}"));
            }
        }
        for r in &d.risks {
            if !(0.0..=1.0001).contains(&r.level) {
                bad(format!("risk level {}", r.level));
            }
        }
        if d.spread < 0.0 || !d.spread.is_finite() {
            bad(format!("spread {}", d.spread));
        }
    }
    let _ = dossier::viewer;
    assert!(failures.is_empty(), "{} problems, first {:?}", failures.len(), &failures[..failures.len().min(10)]);
}
