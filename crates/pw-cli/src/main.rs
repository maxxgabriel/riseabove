//! Headless runner: build or load a world, simulate, report, save.
//!
//! pathway-sim synth [tiny|small|huge] [--days N] [--seed S] [--save FILE]
//! pathway-sim import DIR [--days N] [--seed S] [--save FILE]
//!
//! Without `--seed` every new world gets a fresh random seed (printed, so a
//! world can be rebuilt exactly). Seeds may be hex, decimal or any word.
//! pathway-sim run FILE --days N [--save FILE]
//! pathway-sim report FILE

use std::path::PathBuf;
use std::time::Instant;

use pw_data::DataPack;
use pw_sim::Sim;
use pw_world::comp::sort_table;
use pw_world::{CompKind, PlayerStatus, TeamKind, World};

struct Args {
    cmd: String,
    positional: Option<String>,
    days: u32,
    seed: Option<u64>,
    save: Option<PathBuf>,
}

fn parse() -> Args {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| "help".into());
    let mut a = Args { cmd, positional: None, days: 0, seed: None, save: None };
    while let Some(x) = it.next() {
        match x.as_str() {
            "--days" => a.days = it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            "--seed" => a.seed = it.next().map(|v| pw_core::rng::parse_seed(&v)),
            "--save" => a.save = it.next().map(PathBuf::from),
            _ => a.positional = Some(x),
        }
    }
    a
}

fn main() {
    let a = parse();
    let world = match a.cmd.as_str() {
        "synth" => {
            let scale = match a.positional.as_deref() {
                Some("tiny") => pw_import::synthetic::Scale::TINY,
                Some("huge") => pw_import::synthetic::Scale::HUGE,
                _ => pw_import::synthetic::Scale::SMALL,
            };
            let t = Instant::now();
            let seed = a.seed.unwrap_or_else(pw_core::rng::fresh_seed);
            println!("world seed: {}", pw_core::rng::seed_label(seed));
            let w = pw_import::synthetic::build(DataPack::builtin(), seed, scale);
            println!("built synthetic world: {} players, {} clubs in {:.2?}", w.players.len(), w.clubs.len(), t.elapsed());
            w
        }
        "import" => {
            let dir = PathBuf::from(a.positional.clone().unwrap_or_else(|| die("import needs a folder")));
            let t = Instant::now();
            let (w, rep) = pw_import::load_dir_seeded(&dir, DataPack::builtin(), a.seed).unwrap_or_else(|e| die(&e.to_string()));
            println!("world seed: {}", pw_core::rng::seed_label(w.seed));
            println!("imported {} nations, {} competitions, {} clubs, {} players, {} staff in {:.2?}", rep.nations, rep.competitions, rep.clubs, rep.players, rep.staff, t.elapsed());
            for wmsg in rep.warnings.iter().take(20) {
                println!("  warning: {wmsg}");
            }
            w
        }
        "run" | "report" => {
            let file = PathBuf::from(a.positional.clone().unwrap_or_else(|| die("needs a save file")));
            pw_sim::save::load::<World>(&file).unwrap_or_else(|e| die(&e.to_string()))
        }
        _ => {
            println!("usage: pathway-sim synth [tiny|small|huge] [--days N] [--save F] | import DIR [--days N] [--save F] | run F --days N | report F");
            return;
        }
    };

    let mut sim = Sim::new(world);
    if a.days > 0 {
        simulate(&mut sim, a.days);
    }
    report(&sim.world);
    if let Some(path) = &a.save {
        let t = Instant::now();
        pw_sim::save::save(&sim.world, path).unwrap_or_else(|e| die(&e.to_string()));
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        println!("saved {} ({:.1} MB) in {:.2?}", path.display(), size as f64 / 1e6, t.elapsed());
    }
}

fn simulate(sim: &mut Sim, days: u32) {
    let t = Instant::now();
    let (mut worst, mut matches) = (0u128, 0usize);
    for d in 0..days {
        let s = sim.step();
        worst = worst.max(s.micros);
        matches += s.matches;
        if (d + 1) % 30 == 0 {
            println!("  {}  day {:>4}  matches so far {:>7}  worst day {:>6.1} ms", sim.world.date, d + 1, matches, worst as f64 / 1000.0);
        }
    }
    let el = t.elapsed();
    println!("simulated {days} days in {el:.2?} ({:.1} ms/day avg, worst {:.1} ms), {matches} matches", el.as_secs_f64() * 1000.0 / f64::from(days.max(1)), worst as f64 / 1000.0);
}

fn report(w: &World) {
    println!("\n== {} ==", w.date);
    let active = w.players.hot.iter().filter(|h| h.status == PlayerStatus::Active).count();
    let free = w.players.hot.iter().filter(|h| h.status == PlayerStatus::FreeAgent).count();
    let retired = w.players.hot.iter().filter(|h| h.status == PlayerStatus::Retired).count();
    let injured = w.players.hot.iter().filter(|h| h.injury != 0).count();
    println!("players: {active} active, {free} free agents, {retired} retired, {injured} injured");

    let played: Vec<_> = w.fixtures.iter().filter_map(|(_, f)| f.score.map(|s| (f, s))).collect();
    if !played.is_empty() {
        let n = played.len() as f64;
        let goals: f64 = played.iter().map(|(_, s)| f64::from(s.home + s.away)).sum();
        let home = played.iter().filter(|(_, s)| s.home > s.away).count() as f64;
        let draws = played.iter().filter(|(_, s)| s.home == s.away).count() as f64;
        println!("matches: {} played, {:.2} goals/match, home {:.1}%, draw {:.1}%", played.len(), goals / n, home / n * 100.0, draws / n * 100.0);
    }

    let mut kinds = std::collections::BTreeMap::<&str, usize>::new();
    for e in w.events.all() {
        use pw_world::EventKind as E;
        let k = match e.kind {
            E::Transfer { .. } => "transfers",
            E::LoanMove { .. } => "loans",
            E::ContractSigned { renewal: true, .. } => "renewals",
            E::Released { .. } => "releases",
            E::Retired { .. } => "retirements",
            E::Injured { .. } => "injuries",
            E::ManagerSacked { .. } => "sackings",
            E::YouthIntake { .. } => "youth intakes",
            E::Champion { .. } => "titles",
            E::BidRejected { .. } => "bids rejected",
            E::BidAccepted { .. } => "bids accepted",
            _ => continue,
        };
        *kinds.entry(k).or_default() += 1;
    }
    println!("events: {kinds:?}");

    for (id, c) in w.comps.iter_enumerated().filter(|(_, c)| c.kind == CompKind::League && c.team_kind == TeamKind::First && c.tier == 1).take(3) {
        let mut rows = c.state.table.clone();
        let mut season = c.state.season;
        if rows.iter().all(|r| r.played == 0) {
            if let Some(t) = w.history.tables.iter().rev().find(|t| t.comp == id) {
                rows = t.rows.clone();
                season = t.season;
            }
        }
        sort_table(&mut rows);
        println!("\n{} {} — {}", c.name, season, w.nations.get(c.nation).map_or("", |n| n.name.as_str()));
        for (i, r) in rows.iter().take(6).enumerate() {
            println!("  {:>2}. {:<28} {:>2} {:>2} {:>2} {:>2} {:>3}:{:<3} {:>3}", i + 1, w.team_name(r.team), r.played, r.won, r.drawn, r.lost, r.gf, r.ga, r.points);
        }
        let mut scorers: Vec<_> = w.stats.for_comp(id).collect();
        scorers.sort_by_key(|l| std::cmp::Reverse(l.goals));
        for l in scorers.iter().take(3) {
            println!("     ⚽ {:<24} {:>2} goals, {:.2} avg", w.player_name(l.player), l.goals, l.avg_rating());
        }
    }
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1)
}
