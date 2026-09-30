//! Headless runner: build or load a world, simulate, report, save.
//!
//! pathway-sim synth [tiny|small|huge|NATIONS] [--days N] [--seed S] [--save FILE]
//! pathway-sim import DIR [--days N] [--seed S] [--save FILE]
//! pathway-sim balance <micro|tiny|small|huge|DIR> [--years N] [--seeds 1,2,3]   long-run economy, fame and growth trends
//!
//! `--data DIR` on any command loads the engine data (tuning, weights, ...) from DIR at run time instead of the compiled-in copy
//! (files missing there fall back to the built-in ones), so calibration can be iterated without rebuilding: `--data data/engine`.
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
    years: u32,
    seeds: Vec<u64>,
    data: Option<PathBuf>,
}

fn parse() -> Args {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| "help".into());
    let mut a = Args { cmd, positional: None, days: 0, seed: None, save: None, years: 5, seeds: vec![1, 2, 3], data: None };
    while let Some(x) = it.next() {
        match x.as_str() {
            "--days" => a.days = it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            "--seed" => a.seed = it.next().map(|v| pw_core::rng::parse_seed(&v)),
            "--save" => a.save = it.next().map(PathBuf::from),
            "--data" => a.data = it.next().map(PathBuf::from),
            "--years" => a.years = it.next().and_then(|v| v.parse().ok()).unwrap_or(5),
            "--seeds" => a.seeds = it.next().map(|v| v.split(',').map(pw_core::rng::parse_seed).collect()).unwrap_or_default(),
            _ => a.positional = Some(x),
        }
    }
    a
}

/// The engine data: compiled in, or read from `--data DIR` at run time.
fn pack(a: &Args) -> DataPack {
    match &a.data {
        Some(dir) => DataPack::load_dir(dir).unwrap_or_else(|e| die(&format!("--data {}: {e}", dir.display()))),
        None => DataPack::builtin(),
    }
}

/// A scale by name (`micro`, `tiny`, `small`, `huge`, or a number of nations).
fn scale_named(name: Option<&str>) -> pw_import::synthetic::Scale {
    match name {
        Some("micro") => pw_import::synthetic::Scale::MICRO,
        Some("tiny") => pw_import::synthetic::Scale::TINY,
        Some("huge") => pw_import::synthetic::Scale::HUGE,
        // `N`: N nations of four 22-club divisions (for scale runs).
        Some(n) if n.parse::<u16>().is_ok() => pw_import::synthetic::Scale { nations: n.parse().unwrap_or(1), ..pw_import::synthetic::Scale::HUGE },
        _ => pw_import::synthetic::Scale::SMALL,
    }
}

/// Run the same world for several seeds and years and say what is drifting.
fn balance(a: &Args) {
    let target = a.positional.clone().unwrap_or_else(|| "small".into());
    let dir = PathBuf::from(&target);
    let mut problems = 0;
    for &seed in &a.seeds {
        let world = if dir.is_dir() {
            pw_import::load_dir_seeded(&dir, pack(a), Some(seed)).unwrap_or_else(|e| die(&e.to_string())).0
        } else {
            pw_import::synthetic::build(pack(a), seed, scale_named(Some(target.as_str())))
        };
        let t = Instant::now();
        let mut sim = Sim::new(world);
        let run = pw_sim::metrics::observe(&mut sim, a.years, |s| eprintln!("  seed {seed}: year {} done ({:.0?})", s.year, t.elapsed()));
        println!("
== {target}, seed {} ({} years, {:.1?}) ==", pw_core::rng::seed_label(seed), a.years, t.elapsed());
        print!("{}", pw_sim::metrics::render(&run));
        let findings = pw_sim::metrics::analyse(&run);
        if findings.is_empty() {
            println!("no drift found");
        }
        for f in &findings {
            problems += usize::from(f.level == pw_sim::metrics::Level::Problem);
            println!("  {} [{}] {}", if f.level == pw_sim::metrics::Level::Problem { "PROBLEM" } else { "warn   " }, f.series, f.message);
        }
    }
    println!("
{problems} problem(s) across {} seed(s)", a.seeds.len());
}

/// `check FILE`: what a save is (schema, build, seed, migration history) and whether the world in it is structurally sound.
fn check(a: &Args) {
    let file = PathBuf::from(a.positional.clone().unwrap_or_else(|| die("check needs a save file")));
    let info = pw_sim::save::inspect(&file).unwrap_or_else(|e| die(&e.to_string()));
    println!("{}: schema {} ({:?}), {:.1} MB", file.display(), info.schema, info.compat, info.bytes as f64 / 1e6);
    match &info.meta {
        Some(m) => println!("  created with schema {}, last written by build {}, seed {:?}, import provenance {:?}, {} migration(s) {:?}", m.created_schema.map_or("unknown".into(), |s| s.to_string()), m.build, m.world_seed, m.import_provenance, m.migrations.len(), m.migrations),
        None => println!("  no metadata (written before saves carried it)"),
    }
    let w: World = pw_sim::save::load(&file).unwrap_or_else(|e| die(&e.to_string()));
    let problems = pw_sim::validate::problems(&w);
    println!("  {} people, {} players, {} clubs; {} structural problem(s)", w.people.len(), w.players.hot.len(), w.clubs.len(), problems.len());
    for p in &problems {
        println!("    {p}");
    }
    if !problems.is_empty() {
        std::process::exit(1);
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("database") {
        database();
        return;
    }
    let a = parse();
    if a.cmd == "balance" {
        balance(&a);
        return;
    }
    if a.cmd == "check" {
        check(&a);
        return;
    }
    let world = match a.cmd.as_str() {
        "synth" => {
            let scale = scale_named(a.positional.as_deref());
            let t = Instant::now();
            let seed = a.seed.unwrap_or_else(pw_core::rng::fresh_seed);
            println!("world seed: {}", pw_core::rng::seed_label(seed));
            let w = pw_import::synthetic::build(pack(&a), seed, scale);
            println!("built synthetic world: {} players, {} clubs in {:.2?}", w.players.len(), w.clubs.len(), t.elapsed());
            w
        }
        "import" => {
            let dir = PathBuf::from(a.positional.clone().unwrap_or_else(|| die("import needs a folder")));
            let t = Instant::now();
            let (w, rep) = pw_import::load_dir_seeded(&dir, pack(&a), a.seed).unwrap_or_else(|e| die(&e.to_string()));
            println!("world seed: {}", pw_core::rng::seed_label(w.seed));
            println!("imported in {:.2?}: {}", t.elapsed(), rep.summary());
            for wmsg in rep.warnings.iter().take(20) {
                println!("  warning: {wmsg}");
            }
            w
        }
        "run" | "report" => {
            let file = PathBuf::from(a.positional.clone().unwrap_or_else(|| die("needs a save file")));
            pw_sim::save::load_world(&file).unwrap_or_else(|e| die(&e.to_string()))
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
        pw_sim::save::save_with(&sim.world, path, &pw_sim::save::Info::of_world(&sim.world)).unwrap_or_else(|e| die(&e.to_string()));
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        println!("saved {} ({:.1} MB) in {:.2?}", path.display(), size as f64 / 1e6, t.elapsed());
    }
}

/// Profile or browse source records without constructing a simulated world.
/// database DIR --all | --table FILE [--column FIELD --value ID] [--search NAME] [--cache DIR]
fn database() {
    let mut it = std::env::args().skip(2);
    let root = PathBuf::from(it.next().unwrap_or_else(|| die("database needs a source folder")));
    let mut cache = std::env::temp_dir().join("riseabove-database-indexes");
    let (mut table, mut column, mut value, mut search, mut all) = (None, None, String::new(), String::new(), false);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--all" => all = true,
            "--table" => table = it.next(),
            "--column" => column = it.next(),
            "--value" => value = it.next().unwrap_or_default(),
            "--search" => search = it.next().unwrap_or_default(),
            "--cache" => cache = it.next().map(PathBuf::from).unwrap_or(cache),
            _ => die(&format!("unknown database option {flag}")),
        }
    }
    let mut db = pw_import::database::Catalog::open(&root, &cache).unwrap_or_else(|e| die(&e.to_string()));
    println!("source: {}\nindex cache: {}", db.root().display(), cache.display());
    if all {
        let tables: Vec<_> = db.tables().iter().map(|t| t.name.clone()).collect();
        let begin = Instant::now();
        let mut rows = 0;
        for table in tables {
            let t = Instant::now();
            let p = db.query(&table, None, "", "", 0, 1).unwrap_or_else(|e| die(&e.to_string()));
            let cold = t.elapsed();
            let t = Instant::now();
            let end = db.query(&table, None, "", "", p.total.saturating_sub(1), 1).unwrap_or_else(|e| die(&e.to_string()));
            assert_eq!(end.total, p.total);
            rows += p.total;
            println!("{table:<38} {:>10} rows {:>7.1} MB index {:>9.3}s first {:>8.3}ms last; malformed {}{}", p.total,
                p.index_bytes as f64 / 1e6, cold.as_secs_f64(), t.elapsed().as_secs_f64() * 1000.0, p.malformed, if p.cached { " (disk cache)" } else { "" });
        }
        println!("{rows} records across {} tables in {:.3}s", db.tables().len(), begin.elapsed().as_secs_f64());
    } else if let Some(table) = table {
        let t = Instant::now();
        let p = db.query(&table, column.as_deref(), &value, &search, 0, 10).unwrap_or_else(|e| die(&e.to_string()));
        println!("{} matching / {} total; {:.3}ms; {} malformed", p.matched, p.total, t.elapsed().as_secs_f64() * 1000.0, p.malformed);
        for row in p.rows { println!("record {}: {:?}", row.row + 1, row.fields); }
    } else {
        for table in db.tables() { println!("{}: {} bytes; {}", table.name, table.bytes, table.status); }
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
    // Peak memory, where the platform reports it.
    if let Ok(status) = std::fs::read_to_string("/proc/self/status")
        && let Some(line) = status.lines().find(|l| l.starts_with("VmHWM"))
    {
        println!("peak memory: {}", line.trim_start_matches("VmHWM:").trim());
    }
    if pw_sim::profile::enabled() {
        println!("time by system (PW_PROFILE):");
        for (name, us, calls) in pw_sim::profile::take().into_iter().take(25) {
            println!("  {name:<28} {:>10.1} ms  {calls:>6} calls", us as f64 / 1000.0);
        }
    }
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
        if rows.iter().all(|r| r.played == 0)
            && let Some(t) = w.history.tables.iter().rev().find(|t| t.comp == id)
        {
            rows = t.rows.clone();
            season = t.season;
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
