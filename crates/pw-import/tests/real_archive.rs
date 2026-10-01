//! Sanity checks on the user's real local archive (`archive/` in the repository folder, git-ignored). Skipped when the
//! folder is not there. Run with
//! `cargo test --release -p pw-import --test real_archive -- --ignored --nocapture`.

use std::path::PathBuf;

use pw_data::DataPack;
use pw_import::{LoadOptions, load_dir_with};

fn archive() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../archive");
    dir.join("players.csv").exists().then_some(dir)
}

#[test]
#[ignore = "reads the real local archive; run explicitly"]
fn the_real_archive_builds_a_structurally_sound_world() {
    let Some(dir) = archive() else {
        eprintln!("no archive folder; skipped");
        return;
    };
    let (w, rep) = load_dir_with(&dir, DataPack::builtin(), Some(1), LoadOptions::default()).expect("loads");
    println!("{}", rep.summary());

    let bad_clubs: Vec<_> = w.clubs.iter().filter(|c| c.nation.is_none()).map(|c| c.name.clone()).collect();
    assert!(bad_clubs.is_empty(), "clubs without a nation: {bad_clubs:?}");
    let bad_people = w.people.iter().filter(|p| p.nation.is_none()).count();
    println!("people without a nation: {bad_people}");
    assert!(w.clubs.len() >= 500, "{} clubs", w.clubs.len());
    for (id, c) in w.clubs.iter_enumerated() {
        let first = w.club_team(id, pw_world::TeamKind::First).expect("first team");
        assert!(w.teams[first].squad.len() >= 16, "{} has {} players", c.name, w.teams[first].squad.len());
        assert!(c.league.is_some(), "{} has no league", c.name);
    }
    for n in w.nations.iter().filter(|n| !n.leagues.is_empty()) {
        for &lg in &n.leagues {
            let c = &w.comps[lg];
            assert_eq!(c.state.entrants.len(), usize::from(c.size), "{} ({}) has {} entrants, expected {}", c.name, n.name, c.state.entrants.len(), c.size);
        }
    }
}

/// Population-level calibration (locked design §11.12-11.13): ability by cohort, and how far it depends on price.
#[test]
#[ignore = "reads the real local archive; run explicitly"]
fn imported_ability_is_calibrated_by_cohort_and_not_a_function_of_price() {
    use pw_world::origin::{Facet, Origin};
    let Some(dir) = archive() else {
        eprintln!("no archive folder; skipped");
        return;
    };
    let (w, _) = load_dir_with(&dir, DataPack::builtin(), Some(1), LoadOptions::default()).expect("loads");
    struct Row {
        ca: f32,
        pa: f32,
        value: f32,
        age: f32,
        minutes_rank: f32,
        club_rep: f32,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (pid, cold) in w.players.cold.iter_enumerated() {
        let person = &w.people[cold.person];
        let Some(o) = w.origins.person(cold.person) else { continue };
        // Price calibration compares actual source prices. Missing prices now receive labelled estimates;
        // feeding those model outputs back into this check would mix two different populations.
        if o.get(Facet::Attributes) != Origin::Inferred || o.get(Facet::Value) != Origin::Imported || cold.value == 0 {
            continue;
        }
        let club = w.players.hot[pid].club;
        if club.is_none() {
            continue;
        }
        let team = w.players.hot[pid].team;
        rows.push(Row { ca: f32::from(cold.ca), pa: f32::from(cold.pa), value: cold.value as f32, age: person.age(w.date) as f32, minutes_rank: if team.is_some() { 0.0 } else { 0.0 }, club_rep: f32::from(w.clubs[club].reputation) });
    }
    let _ = rows.iter().map(|r| r.minutes_rank).sum::<f32>();
    assert!(rows.len() > 10_000, "{} inferred players", rows.len());
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
    let corr = |a: &[f32], b: &[f32]| {
        let (ma, mb) = (mean(a), mean(b));
        let cov: f32 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
        let (va, vb): (f32, f32) = (a.iter().map(|x| (x - ma).powi(2)).sum(), b.iter().map(|y| (y - mb).powi(2)).sum());
        cov / (va.sqrt() * vb.sqrt())
    };
    let ln_value: Vec<f32> = rows.iter().map(|r| r.value.ln()).collect();
    let cas: Vec<f32> = rows.iter().map(|r| r.ca).collect();
    let r_value = corr(&ln_value, &cas);
    let r_club = corr(&rows.iter().map(|r| r.club_rep).collect::<Vec<_>>(), &cas);
    println!("corr(ln value, CA) = {r_value:.2}   corr(club reputation, CA) = {r_club:.2}");
    println!("{:<18}{:>7}{:>9}{:>9}", "cohort", "n", "mean CA", "mean PA");
    let cohort = |name: &str, f: &dyn Fn(&Row) -> bool| {
        let sel: Vec<&Row> = rows.iter().filter(|r| f(r)).collect();
        let (c, p) = (mean(&sel.iter().map(|r| r.ca).collect::<Vec<_>>()), mean(&sel.iter().map(|r| r.pa).collect::<Vec<_>>()));
        println!("{name:<18}{:>7}{c:>9.1}{p:>9.1}", sel.len());
        c
    };
    let young = cohort("age 16-19", &|r| r.age < 20.0);
    let prime = cohort("age 24-29", &|r| (24.0..30.0).contains(&r.age));
    let vets = cohort("age 34+", &|r| r.age >= 34.0);
    let elite = cohort("top clubs", &|r| r.club_rep >= 8000.0);
    let mid = cohort("mid clubs", &|r| (4500.0..6500.0).contains(&r.club_rep));
    let low = cohort("smaller clubs", &|r| r.club_rep < 3500.0);
    let rich = cohort("value >= 10m", &|r| r.value >= 10e6);
    let cheap = cohort("value < 300k", &|r| r.value < 300e3);
    let (min, max) = (cas.iter().cloned().fold(f32::MAX, f32::min), cas.iter().cloned().fold(f32::MIN, f32::max));
    println!("CA range {min:.0}..{max:.0}");

    // Price informs ability without dictating it: strongly related, but clearly not a function of it.
    assert!((0.45..0.93).contains(&r_value), "value/ability correlation {r_value:.2}");
    // Club standing matters on its own.
    assert!(elite > mid + 8.0 && mid > low + 8.0, "clubs: {elite:.0} {mid:.0} {low:.0}");
    // Age cohorts: the young are below the prime; veterans do not outrun it by much.
    assert!(prime > young + 8.0, "age: young {young:.0} prime {prime:.0}");
    assert!(vets < prime + 12.0, "veterans {vets:.0} vs prime {prime:.0}");
    assert!(rich > cheap + 25.0, "value cohorts: {rich:.0} vs {cheap:.0}");
    // Outliers remain possible: some cheap players are good, some expensive ones are not.
    let cheap_good = rows.iter().filter(|r| r.value < 300e3 && r.ca >= mean(&cas)).count();
    let dear_weak = rows.iter().filter(|r| r.value >= 10e6 && r.ca <= mean(&cas)).count();
    println!("cheap-but-above-average: {cheap_good}   expensive-but-below-average: {dear_weak}");
    assert!(cheap_good > 0 && dear_weak > 0, "value must not decide ability outright");
    assert!(min >= 20.0 && max <= 195.0);
}

/// The imported values are the launchpad; once the world runs, its own formula prices players. The two should agree at population level.
#[test]
#[ignore = "reads the real local archive; run explicitly"]
fn the_worlds_own_prices_stay_near_the_imported_ones_after_the_first_month() {
    let Some(dir) = archive() else {
        eprintln!("no archive folder; skipped");
        return;
    };
    let (w, _) = load_dir_with(&dir, DataPack::builtin(), Some(1), LoadOptions::default()).expect("loads");
    let imported: Vec<(pw_core::PlayerId, i64)> = w.players.cold.iter_enumerated()
        .filter(|(_, c)| c.value > 0 && w.origins.person(c.person).is_some_and(|o| o.get(pw_world::origin::Facet::Value) == pw_world::origin::Origin::Imported))
        .map(|(p, c)| (p, c.value)).collect();
    let mut sim = pw_sim::Sim::new(w);
    sim.run(35);
    let w = &sim.world;
    let mut ratios: Vec<f32> = imported.iter().map(|&(p, v)| (w.players.cold[p].value.max(1) as f32) / v as f32).collect();
    ratios.sort_by(|a, b| a.total_cmp(b));
    let q = |f: f32| ratios[((ratios.len() - 1) as f32 * f) as usize];
    println!("new value / imported value: p10 {:.2}  p25 {:.2}  median {:.2}  p75 {:.2}  p90 {:.2}", q(0.1), q(0.25), q(0.5), q(0.75), q(0.9));
    let by_band = |lo: f64, hi: f64| {
        let mut r: Vec<f32> = imported.iter().filter(|&&(_, v)| (v as f64) >= lo && (v as f64) < hi).map(|&(p, v)| (w.players.cold[p].value.max(1) as f32) / v as f32).collect();
        r.sort_by(|a, b| a.total_cmp(b));
        (r.len(), r.get(r.len() / 2).copied().unwrap_or(0.0))
    };
    for (name, lo, hi) in [("< 300k", 0.0, 3e5), ("300k-2m", 3e5, 2e6), ("2m-10m", 2e6, 1e7), ("10m-40m", 1e7, 4e7), ("40m+", 4e7, 1e12)] {
        let (n, m) = by_band(lo, hi);
        println!("  {name:<10} n={n:<6} median ratio {m:.2}");
    }
    assert!((0.6..1.7).contains(&q(0.5)), "the median price moved by a factor of {:.2} in a month", q(0.5));
}
