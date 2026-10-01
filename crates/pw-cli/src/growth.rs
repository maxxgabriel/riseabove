//! `pathway-sim growth WORLD --years N`: save size and speed, year by year. A diagnostic for save growth and run time.
//!
//! WORLD is `micro|tiny|small|huge|india-tiny|india-regional|india-full`. Prints, per simulated year, the days per second, the
//! serialized world (raw bincode and lz4, which is what a save file holds) and every section, then the largest field paths of the
//! final year (`--depth D`, `--top K`), the average cost of an ordinary day, a first-of-month day and the July rollover, the time
//! by system (set `PW_PROFILE=1`) and the peak memory.
//!
//! `--detail SECTION` (repeatable through a comma list, `ext` and `ext.recog` style) lists the field paths inside those sections
//! for the first and last year instead of the whole world.

use std::time::Instant;

use pw_data::DataPack;
use pw_sim::Sim;
use pw_world::World;

pub struct Opts {
    pub world: String,
    pub years: u32,
    pub seed: u64,
    pub depth: usize,
    pub top: usize,
    pub detail: Vec<String>,
    pub data: Option<std::path::PathBuf>,
    /// Report every N years (default 1).
    pub every: u32,
    /// Also save the world at the end to this file and report the file size.
    pub save: Option<std::path::PathBuf>,
}

fn build(o: &Opts) -> World {
    let pack = match &o.data {
        Some(d) => DataPack::load_dir(d).unwrap_or_else(|e| crate::die(&format!("--data: {e}"))),
        None => DataPack::builtin(),
    };
    use pw_import::india::{self, IndiaScale};
    match o.world.as_str() {
        "india-tiny" => india::build(pack, o.seed, IndiaScale::TINY),
        "india-regional" => india::build(pack, o.seed, IndiaScale { states: 12, state_league: 8, squad: 20 }),
        "india-full" => india::build(pack, o.seed, IndiaScale::FULL),
        other => pw_import::synthetic::build(pack, o.seed, crate::scale_named(Some(other))),
    }
}

/// Sections by name, as (raw, lz4) bytes.
fn sections(w: &World) -> Vec<(&'static str, u64, u64)> {
    let raw = pw_sim::metrics::section_sizes(w);
    let lz = pw_sim::metrics::section_sizes_lz4(w);
    raw.into_iter().map(|(n, r)| (n, r, lz.iter().find(|x| x.0 == n).map_or(0, |x| x.1))).collect()
}

fn mb(b: u64) -> f64 {
    b as f64 / 1e6
}

/// Peak working set of this process in bytes, where the platform reports it.
pub fn peak_memory() -> Option<u64> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Counters {
            cb: u32,
            page_faults: u32,
            peak_working_set: usize,
            working_set: usize,
            a: usize,
            b: usize,
            c: usize,
            d: usize,
            pagefile: usize,
            peak_pagefile: usize,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, cb: u32) -> i32;
            fn GetCurrentProcess() -> isize;
        }
        let mut c = Counters { cb: std::mem::size_of::<Counters>() as u32, page_faults: 0, peak_working_set: 0, working_set: 0, a: 0, b: 0, c: 0, d: 0, pagefile: 0, peak_pagefile: 0 };
        // SAFETY: a correctly sized, initialised struct and the pseudo-handle of this process.
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
        if ok != 0 {
            return Some(c.peak_working_set as u64);
        }
        None
    }
    #[cfg(not(windows))]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with("VmHWM"))?;
        let kb: u64 = line.trim_start_matches("VmHWM:").trim().trim_end_matches("kB").trim().parse().ok()?;
        Some(kb * 1024)
    }
}

fn detail(label: &str, w: &World, which: &[String], depth: usize, top: usize) {
    for name in which {
        let rows = match name.as_str() {
            "ext" => pw_sim::sizes::breakdown("ext", &w.ext, depth),
            n if n.starts_with("ext.") => match &n[4..] {
                "recog" => pw_sim::sizes::breakdown(n, &w.ext.recog, depth),
                "pathway" => pw_sim::sizes::breakdown(n, &w.ext.pathway, depth),
                "ecosystem" => pw_sim::sizes::breakdown(n, &w.ext.ecosystem, depth),
                "almanac" => pw_sim::sizes::breakdown(n, &w.ext.almanac, depth),
                "medical" => pw_sim::sizes::breakdown(n, &w.ext.medical, depth),
                "decisions" => pw_sim::sizes::breakdown(n, &w.ext.decisions, depth),
                "academy" => pw_sim::sizes::breakdown(n, &w.ext.academy, depth),
                "staff" => pw_sim::sizes::breakdown(n, &w.ext.staff, depth),
                "training" => pw_sim::sizes::breakdown(n, &w.ext.training, depth),
                "scenario" => pw_sim::sizes::breakdown(n, &w.ext.scenario, depth),
                _ => continue,
            },
            _ => match pw_sim::sizes::section(w, name, depth) {
                Some(r) => r,
                None => {
                    eprintln!("no section {name}");
                    continue;
                }
            },
        };
        println!("\n-- {label}: {name}");
        for (path, bytes, count) in rows.iter().take(top) {
            println!("  {:>10.3} MB  {:>10} x  {path}", mb(*bytes), count);
        }
    }
}

pub fn run(o: &Opts) {
    let t = Instant::now();
    let world = build(o);
    println!("{}: built in {:.1?} ({} players, {} clubs, seed {})", o.world, t.elapsed(), world.players.len(), world.clubs.len(), o.seed);
    let mut sim = Sim::new(world);
    let mut table: Vec<(u32, Vec<(&'static str, u64, u64)>)> = Vec::new();
    let s0 = sections(&sim.world);
    let (raw0, lz0) = (s0.iter().map(|x| x.1).sum::<u64>(), s0.iter().map(|x| x.2).sum::<u64>());
    println!("year  days/s   raw MB   lz4 MB  | run time, peak memory, active players / all players");
    println!("{:>4} {:>7} {:>8.2} {:>8.2}  |", 0, "-", mb(raw0), mb(lz0));
    table.push((0, s0));
    if !o.detail.is_empty() {
        detail("year 0", &sim.world, &o.detail, o.depth, o.top);
    }
    let (mut ordinary, mut month_start, mut july) = ((0u128, 0u64), (0u128, 0u64), (0u128, 0u64));
    let mut worst = (0u128, String::new());
    let begin = Instant::now();
    let mut simulated = 0u64;
    for year in 1..=o.years {
        let ty = Instant::now();
        for _ in 0..365 {
            let date = sim.world.date;
            let st = sim.step();
            simulated += 1;
            let bucket = if date.month() == 7 && date.day() == 1 {
                &mut july
            } else if date.day() == 1 {
                &mut month_start
            } else {
                &mut ordinary
            };
            bucket.0 += st.micros;
            bucket.1 += 1;
            if st.micros > worst.0 {
                worst = (st.micros, format!("{date}"));
            }
        }
        let secs = ty.elapsed().as_secs_f64();
        if year % o.every == 0 || year == o.years {
            let s = sections(&sim.world);
            let (raw, lz) = (s.iter().map(|x| x.1).sum::<u64>(), s.iter().map(|x| x.2).sum::<u64>());
            let active = sim.world.players.hot.iter().filter(|h| h.status == pw_world::PlayerStatus::Active).count();
            println!("{year:>4} {:>7.1} {:>8.2} {:>8.2}  | {:.0?} elapsed, peak {}, {active} / {}", 365.0 / secs, mb(raw), mb(lz), begin.elapsed(), peak_memory().map_or("?".into(), |m| format!("{:.0} MB", mb(m))), sim.world.players.len());
            table.push((year, s));
            if pw_sim::profile::enabled() {
                println!("   time by system this year (PW_PROFILE):");
                for (name, us, calls) in pw_sim::profile::take().into_iter().take(30) {
                    println!("     {name:<30} {:>9.1} ms {calls:>7} calls {:>8.1} us/call", us as f64 / 1000.0, us as f64 / f64::from(calls.max(1)));
                }
            }
        }
    }
    let avg = |b: (u128, u64)| if b.1 == 0 { 0.0 } else { b.0 as f64 / b.1 as f64 / 1000.0 };
    println!(
        "\nsimulated {simulated} days in {:.1?} = {:.1} days/s. average day {:.1} ms; first of month {:.1} ms; July 1 {:.1} ms; worst day {:.0} ms ({})",
        begin.elapsed(),
        simulated as f64 / begin.elapsed().as_secs_f64(),
        avg(ordinary),
        avg(month_start),
        avg(july),
        worst.0 as f64 / 1000.0,
        worst.1
    );
    println!("peak memory: {}", peak_memory().map_or("unknown".into(), |m| format!("{:.0} MB", mb(m))));

    // The section matrix: raw MB per year, largest sections of the last year first.
    if let Some((_, last)) = table.last() {
        let mut names: Vec<_> = last.iter().map(|x| (x.0, x.1)).collect();
        names.sort_by_key(|x| std::cmp::Reverse(x.1));
        print!("\nraw MB by section:\n{:<14}", "section");
        for (y, _) in &table {
            print!(" {:>7}", format!("y{y}"));
        }
        println!();
        for (name, _) in names.iter().take(24) {
            print!("{name:<14}");
            for (_, s) in &table {
                print!(" {:>7.2}", s.iter().find(|x| x.0 == *name).map_or(0.0, |x| mb(x.1)));
            }
            println!();
        }
        print!("\nlz4 MB by section:\n{:<14}", "section");
        for (y, _) in &table {
            print!(" {:>7}", format!("y{y}"));
        }
        println!();
        let mut lnames: Vec<_> = last.iter().map(|x| (x.0, x.2)).collect();
        lnames.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (name, _) in lnames.iter().take(24) {
            print!("{name:<14}");
            for (_, s) in &table {
                print!(" {:>7.2}", s.iter().find(|x| x.0 == *name).map_or(0.0, |x| mb(x.2)));
            }
            println!();
        }
    }
    if o.detail.is_empty() {
        println!("\n-- final year: largest field paths of the whole world");
        for (path, bytes, count) in pw_sim::sizes::world_paths(&sim.world, o.depth).iter().take(o.top) {
            println!("  {:>10.3} MB  {:>10} x  {path}", mb(*bytes), count);
        }
    } else {
        detail(&format!("year {}", o.years), &sim.world, &o.detail, o.depth, o.top);
    }
    if let Some(path) = &o.save {
        let t = Instant::now();
        pw_sim::save::save_with(&sim.world, path, &pw_sim::save::Info::of_world(&sim.world)).unwrap_or_else(|e| crate::die(&e.to_string()));
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        println!("saved {} ({:.2} MB) in {:.1?}", path.display(), mb(size), t.elapsed());
        let t = Instant::now();
        let _ = pw_sim::save::load_world(path).unwrap_or_else(|e| crate::die(&e.to_string()));
        println!("loaded in {:.1?}", t.elapsed());
    }
}
