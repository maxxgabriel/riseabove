//! Where does simulation time go, and does it grow as the world ages?
//!
//!   cargo run --release -p pw-view --example profile -- [tiny|small] [years]
//!
//! Prints, for each simulated year, the time per day split by the kind of day (a Monday, the first
//! of a month, an ordinary day), and the size of the structures that grow with history.

use std::time::Instant;

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let scale = match args.first().map(String::as_str) {
        Some("tiny") => Scale::TINY,
        _ => Scale::SMALL,
    };
    let years: u32 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(4);
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let t0 = Instant::now();
    let mut sim = if let Some(path) = flag("--load") {
        let w: pw_world::World = pw_sim::save::load(std::path::Path::new(&path)).expect("load");
        Sim::new(w)
    } else {
        Sim::new(synthetic::build(DataPack::builtin(), 42, scale))
    };
    println!("ready in {:.1}s", t0.elapsed().as_secs_f32());
    if args.iter().any(|a| a == "--systems") {
        // The cost of each daily system on this world as it stands now (state is mutated slightly by repeats).
        let w = &mut sim.world;
        macro_rules! time {
            ($name:expr, $call:expr) => {{
                let t = Instant::now();
                for _ in 0..5 {
                    $call;
                }
                println!("{:>28}: {:7.1} ms", $name, t.elapsed().as_secs_f64() * 200.0);
            }};
        }
        time!("life::sync", pw_sim::life::sync(w));
        time!("intents::process", pw_sim::intents::process(w));
        time!("market::daily", pw_sim::market::daily(w));
        time!("deals::daily", pw_sim::deals::daily(w));
        time!("negotiation::daily", pw_sim::negotiation::daily(w));
        time!("decisions::resolve_due", pw_sim::decisions::resolve_due(w));
        time!("talk::daily", pw_sim::talk::daily(w));
        time!("incidents::daily", pw_sim::incidents::daily(w));
        time!("intl::daily", pw_sim::intl::daily(w));
        time!("grapevine::daily", pw_sim::grapevine::daily(w));
        time!("pressroom::daily", pw_sim::pressroom::daily(w));
        time!("newsroom::daily", pw_sim::newsroom::daily(w));
        time!("socialnet::persons_post", pw_sim::socialnet::persons_post(w));
        time!("socialnet::daily", pw_sim::socialnet::daily(w));
        time!("season::daily", pw_sim::season::daily(w));
        time!("contracts::daily", pw_sim::contracts::daily(w));
        time!("people::daily", pw_sim::people::daily(w));
        time!("--- weekly ---", ());
        time!("development::weekly", pw_sim::development::weekly(w));
        time!("medical::weekly", pw_sim::medical::weekly(w));
        time!("grapevine::feelings", pw_sim::grapevine::feelings(w));
        time!("incidents::weekly", pw_sim::incidents::weekly(w));
        time!("newsroom::weekly", pw_sim::newsroom::weekly(w));
        time!("socialnet::weekly", pw_sim::socialnet::weekly(w));
        time!("dressing::weekly", pw_sim::dressing::weekly(w));
        time!("perception::weekly", pw_sim::perception::weekly(w));
        time!("social::weekly", pw_sim::social::weekly(w));
        time!("morale::weekly", pw_sim::morale::weekly(w));
        time!("talk::manager_summons", pw_sim::talk::manager_summons(w));
        time!("mind::weekly", pw_sim::mind::weekly(w));
        time!("youth::weekly", pw_sim::youth::weekly(w));
        time!("agents::weekly", pw_sim::agents::weekly(w));
        time!("media::weekly", pw_sim::media::weekly(w));
        time!("reputation::weekly", pw_sim::reputation::weekly(w));
        time!("finance::weekly", pw_sim::finance::weekly(w));
        time!("contracts::weekly", pw_sim::contracts::weekly(w));
        time!("--- monthly ---", ());
        time!("market::monthly", pw_sim::market::monthly(w));
        time!("deals::monthly", pw_sim::deals::monthly(w));
        time!("perception::monthly", pw_sim::perception::monthly(w));
        time!("life::monthly", pw_sim::life::monthly(w));
        time!("mind::monthly", pw_sim::mind::monthly(w));
        time!("social::monthly", pw_sim::social::monthly(w));
        time!("staffing::monthly", pw_sim::staffing::monthly(w));
        time!("governance::monthly", pw_sim::governance::monthly(w));
        time!("managers::monthly", pw_sim::managers::monthly(w));
        time!("scouting::assign", pw_sim::scouting::assign(w));
        time!("medical::monthly", pw_sim::medical::monthly(w));
        time!("growth::monthly", pw_sim::growth::monthly(w));
        time!("dressing::monthly", pw_sim::dressing::monthly(w));
        time!("interpret::monthly", pw_sim::interpret::monthly(w));
        time!("honours::monthly", pw_sim::honours::monthly(w));
        time!("renown::monthly", pw_sim::renown::monthly(w));
        time!("incidents::monthly", pw_sim::incidents::monthly(w));
        time!("affairs::monthly", pw_sim::affairs::monthly(w));
        time!("commerce::monthly", pw_sim::commerce::monthly(w));
        time!("socialnet::monthly", pw_sim::socialnet::monthly(w));
        println!("active info items {}, all {}", w.grapevine.active.len(), w.grapevine.items.len());
        let mut kinds: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
        for &i in &w.grapevine.active {
            let it = &w.grapevine.items[i as usize];
            let name = format!("{:?}", it.kind);
            let name = name.split([' ', '{']).next().unwrap_or("").to_string();
            let e = kinds.entry(name).or_default();
            e.0 += 1;
            e.1 += it.holders.len();
        }
        for (k, (n, h)) in kinds {
            println!("  active {k}: {n} items, {:.1} holders each", h as f64 / n as f64);
        }
        let mut inc: std::collections::BTreeMap<String, usize> = Default::default();
        for i in w.incidents.list.iter() {
            *inc.entry(format!("{:?}", i.kind)).or_default() += 1;
        }
        for (k, n) in inc {
            println!("  incidents {k}: {n}");
        }
        return;
    }
    if let Some(n) = flag("--days").and_then(|v| v.parse::<u32>().ok()) {
        // A short run for a sampling profiler: no per-year report.
        let t = Instant::now();
        for _ in 0..n {
            sim.step();
        }
        println!("{n} days in {:.2}s", t.elapsed().as_secs_f32());
        if args.iter().any(|a| a == "--hash") {
            // A fingerprint of the whole world, to check that a speed-up changed nothing.
            use std::hash::{Hash, Hasher};
            let path = std::env::temp_dir().join("pw-profile-hash.pws");
            pw_sim::save::save(&sim.world, &path).expect("save");
            let bytes = std::fs::read(&path).expect("read");
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut h);
            println!("world fingerprint {:016x} ({} bytes)", h.finish(), bytes.len());
        }
        return;
    }
    let save_at = flag("--save");
    for year in 1..=years {
        let (mut ord, mut mon, mut first) = ((0u128, 0u32), (0u128, 0u32), (0u128, 0u32));
        let mut worst: (u128, i32) = (0, 0);
        let t = Instant::now();
        for _ in 0..365 {
            let d = sim.world.date;
            let s = sim.step();
            let bucket = if d.day() == 1 { &mut first } else if d.weekday() == pw_core::Weekday::Mon { &mut mon } else { &mut ord };
            bucket.0 += s.micros;
            bucket.1 += 1;
            if s.micros > worst.0 {
                worst = (s.micros, d.0);
            }
        }
        let w = &sim.world;
        println!(
            "year {year}: {:.1}s | ordinary {:.0}ms/day, monday {:.0}ms, first-of-month {:.0}ms, worst {:.0}ms | events {} stories {} posts {} kept {} accounts {} grapevine {} incidents {} people {}",
            t.elapsed().as_secs_f32(),
            ord.0 as f64 / f64::from(ord.1.max(1)) / 1000.0,
            mon.0 as f64 / f64::from(mon.1.max(1)) / 1000.0,
            first.0 as f64 / f64::from(first.1.max(1)) / 1000.0,
            worst.0 as f64 / 1000.0,
            w.events.all().len(),
            w.media.stories.len(),
            w.net.posts.len(),
            w.net.kept.len(),
            w.net.accounts.len(),
            w.grapevine.items.len(),
            w.incidents.list.len(),
            w.people.len(),
        );
    }
    if let Some(path) = save_at {
        pw_sim::save::save(&sim.world, std::path::Path::new(&path)).expect("save");
        println!("saved {path}");
    }
}
