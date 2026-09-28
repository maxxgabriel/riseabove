//! Whole-world tests: a world runs, stays consistent, replays exactly,
//! tells the truth in words, and lets a human act through the same systems.
//!
//! Fast tests run on the tiny and small synthetic worlds. Long runs are
//! `#[ignore]`d; run them with
//! `cargo test --release -p pw-cli --test world -- --ignored --nocapture`.

use pw_core::PersonId;
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::{Intent, MindKind, World};

fn sim(scale: Scale, seed: u64) -> Sim {
    Sim::new(synthetic::build(DataPack::builtin(), seed, scale))
}

fn ran(scale: Scale, seed: u64, days: u32) -> Sim {
    let mut s = sim(scale, seed);
    s.run(days);
    s
}

/// A digest of what happened (event kinds in order, league tables, counts).
fn digest(w: &World) -> u64 {
    let mut parts: Vec<u64> = Vec::new();
    for e in w.events.since(pw_core::Date(0)) {
        parts.push(pw_core::rng::hash_key(&[u64::from(e.id.0), format!("{:?}", e.kind).len() as u64, e.date.0 as u64]));
    }
    for c in w.comps.iter() {
        for r in &c.state.table {
            parts.push(pw_core::rng::hash_key(&[u64::from(r.team.0), u64::from(r.points as u16), u64::from(r.gf)]));
        }
    }
    parts.push(w.net.posts.len() as u64);
    parts.push(w.media.stories.len() as u64);
    parts.push(w.incidents.list.len() as u64);
    parts.push(w.records.records.len() as u64);
    pw_core::rng::hash_key(&parts)
}

#[test]
fn a_season_runs_and_the_audit_is_clean() {
    let s = ran(Scale::TINY, 11, 400);
    let w = &s.world;
    assert!(w.fixtures.iter().filter(|f| f.1.score.is_some()).count() > 40, "matches were played");
    let v = pw_sim::audit::audit(w);
    assert!(v.is_empty(), "audit violations: {v:?}");
}

#[test]
fn same_seed_replays_exactly_and_seeds_differ() {
    let a = ran(Scale::TINY, 5, 200);
    let b = ran(Scale::TINY, 5, 200);
    assert_eq!(digest(&a.world), digest(&b.world), "same seed, same world");
    let c = ran(Scale::TINY, 6, 200);
    assert_ne!(digest(&a.world), digest(&c.world), "different seeds, different worlds");
    // Supporters' identities vary by seed.
    let ha: Vec<&str> = a.world.net.accounts.iter().filter(|x| x.person.is_none()).map(|x| x.handle.as_str()).collect();
    let hc: Vec<&str> = c.world.net.accounts.iter().filter(|x| x.person.is_none()).map(|x| x.handle.as_str()).collect();
    assert_ne!(ha, hc, "accounts differ between seeds");
}

#[test]
fn saving_and_loading_continues_the_same_world() {
    let dir = std::env::temp_dir().join(format!("pw-test-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("w.sav");
    let mut a = ran(Scale::TINY, 9, 120);
    pw_sim::save::save(&a.world, &path).unwrap();
    let loaded: World = pw_sim::save::load(&path).unwrap();
    let mut b = Sim::new(loaded);
    a.run(60);
    b.run(60);
    assert_eq!(digest(&a.world), digest(&b.world), "a loaded save continues exactly");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_rendered_line_passes_quality_checks() {
    let s = ran(Scale::SMALL, 3, 300);
    let w = &s.world;
    let mut bad: Vec<(String, Vec<pw_narrate::quality::Issue>)> = Vec::new();
    for e in w.events.since(pw_core::Date(0)) {
        if let Some(line) = pw_narrate::events::line(w, e, PersonId::NONE) {
            let issues = pw_narrate::quality::check(&line, true, 600);
            if !issues.is_empty() {
                bad.push((line, issues));
            }
        }
    }
    for p in &w.net.posts {
        let text = pw_narrate::social::post(w, p);
        let issues = pw_narrate::quality::check(&text, false, 400);
        if !issues.is_empty() {
            bad.push((text, issues));
        }
    }
    for st in w.media.stories.iter() {
        let h = pw_narrate::press::headline(w, st);
        let issues = pw_narrate::quality::check(&h, false, 200);
        if !issues.is_empty() {
            bad.push((h, issues));
        }
    }
    bad.truncate(40);
    assert!(bad.is_empty(), "text issues:\n{}", bad.iter().map(|(t, i)| format!("{i:?}: {t}")).collect::<Vec<_>>().join("\n"));
}

#[test]
fn the_new_systems_are_alive() {
    let s = ran(Scale::SMALL, 21, 400);
    let w = &s.world;
    let report = format!(
        "accounts {} posts {} groups {} chants {} stories {} threads {} incidents {} grapevine {} referees {} controversies {} minor seasons {} records {} votes {} halls {} past seasons {}",
        w.net.accounts.len(),
        w.net.posts.len() + w.net.kept.len(),
        w.net.groups.len(),
        w.net.chants.len(),
        w.media.stories.len(),
        w.media.threads.len(),
        w.incidents.list.len(),
        w.grapevine.items.len(),
        w.officials.referees.len(),
        w.officials.controversies.len(),
        w.minor.history.len(),
        w.records.records.len(),
        w.acclaim.votes.len(),
        w.acclaim.halls.len(),
        w.backfill.seasons.len(),
    );
    println!("{report}");
    assert!(!w.net.accounts.is_empty(), "{report}");
    assert!(!w.net.posts.is_empty() || !w.net.kept.is_empty(), "{report}");
    assert!(!w.net.groups.is_empty(), "{report}");
    assert!(!w.media.stories.is_empty(), "{report}");
    assert!(!w.grapevine.items.is_empty(), "{report}");
    assert!(!w.officials.referees.is_empty(), "{report}");
    assert!(!w.minor.institutions.is_empty(), "{report}");
    assert!(!w.minor.history.is_empty(), "{report}");
    assert!(!w.records.records.is_empty(), "{report}");
    assert!(w.backfill.done && !w.backfill.seasons.is_empty(), "{report}");
}

#[test]
fn a_human_posts_and_replies_through_the_same_systems() {
    let mut s = ran(Scale::TINY, 13, 60);
    // Take control of a first-team player.
    let who = {
        let w = &s.world;
        let p = w.teams.iter().find(|t| t.kind == pw_world::TeamKind::First).and_then(|t| t.squad.first().copied()).expect("a player");
        w.players.cold[p].person
    };
    s.world.people[who].mind = MindKind::External;
    let before = s.world.net.next_post_id();
    s.world.intents.submit(
        who,
        Intent::Post { about: who, concept: pw_world::socialnet::Concept::Statement, reply_to: pw_world::socialnet::NO_POST, quote_of: pw_world::socialnet::NO_POST },
        s.world.date,
    );
    s.run(1);
    let w = &s.world;
    let acc = w.net.account_of(who).expect("an account");
    assert!(w.net.posts.iter().any(|p| p.id >= before && p.author == acc), "the human's post exists");
    // The inbox holds only real communications.
    assert!(pw_sim::audit::audit(w).iter().all(|v| !matches!(v, pw_sim::audit::Violation::InboxWithoutSource { .. })));
}

#[test]
fn referees_are_not_biased_even_when_supporters_think_so() {
    let s = ran(Scale::SMALL, 17, 700);
    let w = &s.world;
    // Correctness must not depend on which side a call went against.
    let (mut home, mut away) = ((0u32, 0u32), (0u32, 0u32));
    for c in &w.officials.controversies {
        let Some(m) = w.recent_matches.by_uid(c.uid) else { continue };
        let slot = if c.against == m.home { &mut home } else { &mut away };
        slot.0 += 1;
        slot.1 += u32::from(c.correct);
    }
    if home.0 >= 30 && away.0 >= 30 {
        let (rh, ra) = (home.1 as f32 / home.0 as f32, away.1 as f32 / away.0 as f32);
        assert!((rh - ra).abs() < 0.15, "correct-call rate home {rh:.2} vs away {ra:.2}");
    }
}

#[test]
fn history_before_the_start_is_marked_and_never_names_real_people() {
    let s = sim(Scale::SMALL, 23);
    let w = &s.world;
    assert!(w.backfill.done);
    assert!(w.backfill.seasons.iter().all(|x| x.provenance == pw_world::backfill::Provenance::Generated));
    let real: std::collections::HashSet<String> = w.people.iter().map(|p| p.display_name(&w.names).into_owned()).collect();
    assert!(w.backfill.figures.iter().all(|f| !real.contains(&f.name)));
}

// ---------------------------------------------------------------------------
// Long runs (ignored by default)
// ---------------------------------------------------------------------------

fn long_run(scale: Scale, seed: u64, seasons: u32) {
    let mut s = sim(scale, seed);
    let t = std::time::Instant::now();
    for year in 0..seasons {
        s.run(365);
        let w = &s.world;
        let v = pw_sim::audit::audit(w);
        println!(
            "season {:>2}: {} players, {} posts kept {}, stories {}, records {}, votes {}, schools {}, rule changes {}, audit {} ({:.1?})",
            year + 1,
            w.players.len(),
            w.net.posts.len(),
            w.net.kept.len(),
            w.media.stories.len(),
            w.records.records.len(),
            w.acclaim.votes.len(),
            w.evolution.schools.len(),
            w.evolution.changes.len(),
            v.len(),
            t.elapsed()
        );
        assert!(v.is_empty(), "season {}: {v:?}", year + 1);
    }
}

#[test]
#[ignore = "long run"]
fn five_seasons_small() {
    long_run(Scale::SMALL, 101, 5);
}

#[test]
#[ignore = "long run"]
fn twenty_seasons_small() {
    long_run(Scale::SMALL, 102, 20);
}

#[test]
#[ignore = "long run"]
fn fifty_seasons_tiny() {
    long_run(Scale::TINY, 103, 50);
}
