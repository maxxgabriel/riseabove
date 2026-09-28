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

#[test]
fn narration_never_changes_the_world() {
    // Rendering is a pure function of state: a world whose every line is
    // rendered each day evolves exactly like one nobody reads.
    let mut a = sim(Scale::TINY, 31);
    let mut b = sim(Scale::TINY, 31);
    for _ in 0..150 {
        a.step();
        b.step();
        let w = &b.world;
        for e in w.events.since(w.date.add_days(-1)) {
            let _ = pw_narrate::events::line(w, e, PersonId::NONE);
        }
        for p in w.net.posts.iter().rev().take(50) {
            let _ = pw_narrate::social::post(w, p);
        }
        for s in w.media.stories.iter().rev().take(20) {
            let _ = pw_narrate::press::headline(w, s);
        }
    }
    assert_eq!(digest(&a.world), digest(&b.world));
}

#[test]
fn identities_are_stable_within_a_save() {
    let mut a = ran(Scale::TINY, 37, 90);
    let handles: Vec<(u32, String)> = a.world.net.accounts.iter().map(|x| (x.id, x.handle.clone())).collect();
    let dir = std::env::temp_dir().join(format!("pw-test-ids-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("w.sav");
    pw_sim::save::save(&a.world, &path).unwrap();
    let mut b = Sim::new(pw_sim::save::load::<World>(&path).unwrap());
    a.run(200);
    b.run(200);
    for (id, h) in &handles {
        assert_eq!(&a.world.net.accounts[*id as usize].handle, h, "a handle changed over time");
        assert_eq!(&b.world.net.accounts[*id as usize].handle, h, "a handle changed through save/load");
    }
    // History survives save and load.
    assert_eq!(a.world.records.records.len(), b.world.records.records.len());
    assert_eq!(a.world.acclaim.votes.len(), b.world.acclaim.votes.len());
    assert_eq!(a.world.minor.history.len(), b.world.minor.history.len());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn suspended_players_are_not_selected() {
    let mut s = ran(Scale::TINY, 41, 40);
    let w = &mut s.world;
    let team = w.teams.ids().find(|&t| w.teams[t].kind == pw_world::TeamKind::First).unwrap();
    let comp = w.clubs[w.teams[team].club].league;
    let best = *w.teams[team].squad.iter().max_by_key(|&&p| w.players.cold[p].ca).unwrap();
    w.players.hot[best].ban = 2;
    let sel = pw_sim::selection::select_in(w, team, comp, w.date, 0.5, 9, 0).expect("a side");
    assert!(!sel.xi.contains(&best) && !sel.bench.contains(&best), "a suspended player was picked");
}

#[test]
fn record_ties_do_not_break_records() {
    use pw_world::minor::Level;
    use pw_world::records::{Holder, Mark, RecordKey, Scope, Stat};
    let mut s = sim(Scale::TINY, 43);
    let w = &mut s.world;
    let key = RecordKey { scope: Scope::World, stat: Stat::Goals, level: Level::Professional };
    let people: Vec<PersonId> = w.people.ids().take(3).collect();
    let mark = |p: PersonId, v: i64| Mark { holder: Holder::Person(p), value: v, date: pw_core::Date(0), against: None };
    let before = w.records.broken.len();
    pw_sim::records::note(w, key, mark(people[0], 10), 1, false);
    assert!(pw_sim::records::note(w, key, mark(people[1], 10), 1, false).is_none(), "a tie is not a new record");
    assert!(pw_sim::records::note(w, key, mark(people[2], 11), 1, false).is_some(), "a better mark is");
    assert_eq!(w.records.get(&key).unwrap().current.holder, Holder::Person(people[2]));
    assert_eq!(w.records.get(&key).unwrap().previous.last().unwrap().holder, Holder::Person(people[0]));
    assert_eq!(w.records.broken.len(), before + 1);
}

#[test]
fn a_world_without_a_protagonist_is_alive() {
    let s = ran(Scale::SMALL, 47, 730);
    let w = &s.world;
    assert!(w.people.iter().all(|p| p.mind != MindKind::External));
    use pw_world::EventKind as E;
    let mut seen: std::collections::BTreeMap<&str, usize> = Default::default();
    for e in w.events.since(pw_core::Date(0)) {
        let k = match e.kind {
            E::Incident { .. } => "incidents",
            E::Published { .. } => "news",
            E::Transfer { .. } => "transfers",
            E::ManagerSacked { .. } => "sackings",
            E::ManagerAppointed { .. } => "appointments",
            E::Retired { .. } => "retirements",
            E::NewCareer { .. } => "post-playing careers",
            E::Award { .. } | E::Voted { .. } => "awards",
            E::Record { .. } | E::RecordBroken { .. } => "records",
            E::JournalistMoved { .. } | E::JournalistHired { .. } | E::JournalistLeft { .. } => "journalist careers",
            E::MinorTitle { .. } => "minor football",
            E::RefereeControversy { .. } => "controversies",
            _ => continue,
        };
        *seen.entry(k).or_default() += 1;
    }
    let rumours = w.media.stories.iter().filter(|s| s.kind == pw_world::StoryKind::TransferRumour).count();
    println!("{seen:?} rumours {rumours} posts {}", w.net.posts.len());
    for k in ["incidents", "news", "transfers", "retirements", "awards", "records", "minor football", "controversies", "appointments"] {
        assert!(seen.get(k).copied().unwrap_or(0) > 0, "no {k} in two autonomous seasons: {seen:?}");
    }
    assert!(rumours > 0 && !w.net.posts.is_empty());
    // Players develop and decline.
    let grew = w.players.cold.iter().filter(|c| c.senior_apps > 20).count();
    assert!(grew > 0);
}

#[test]
fn incidents_become_causal_chains_that_differ_by_seed() {
    let mut shapes = Vec::new();
    for seed in [51u64, 52, 53] {
        let s = ran(Scale::SMALL, seed, 300);
        let w = &s.world;
        // A contextual incident: caused by recorded pressures, learned by
        // people, responded to by someone with authority.
        let mut chains = 0;
        let mut shape: Vec<String> = Vec::new();
        for inc in w.incidents.list.iter() {
            let Some(ev) = w.events.get(inc.event) else { continue };
            let pressured = ev.causes.iter().any(|c| matches!(c, pw_world::event::Cause::Fact(pw_world::event::Fact::Pressure { .. })));
            let known = w.grapevine.items.iter().rev().take(20_000).any(|it| matches!(it.kind, pw_world::info::InfoKind::Incident { incident } if incident == inc.id) && it.holders.len() >= 2);
            if pressured && known && !inc.responses.is_empty() {
                chains += 1;
                if shape.len() < 5 {
                    shape.push(format!("{:?}->{:?}", inc.kind, inc.responses[0].response));
                }
            }
        }
        println!("seed {seed}: {chains} chains, e.g. {shape:?}");
        assert!(chains > 0, "seed {seed}: no incident went pressure -> knowledge -> response");
        shapes.push(shape);
    }
    assert!(shapes[0] != shapes[1] || shapes[1] != shapes[2], "chains identical across seeds");
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
