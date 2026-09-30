//! Shared helpers for the `qa_*` test targets (adversarial and property tests written by the tester).
//! Not a test target itself: `tests/qa_common/mod.rs` is only compiled where a target says `mod qa_common;`.
#![allow(dead_code)]

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::World;
use std::sync::atomic::{AtomicU32, Ordering};

pub fn sim(scale: Scale, seed: u64) -> Sim {
    Sim::new(synthetic::build(DataPack::builtin(), seed, scale))
}

pub fn ran(scale: Scale, seed: u64, days: u32) -> Sim {
    let mut s = sim(scale, seed);
    s.run(days);
    s
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A fresh temp file path that is removed by the caller's `cleanup`.
pub fn temp_path(tag: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("pw-qa-{}-{tag}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d.join("w.sav")
}

pub fn cleanup(p: &std::path::Path) {
    if let Some(d) = p.parent() {
        let _ = std::fs::remove_dir_all(d);
    }
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The whole state of the world as one number: the world is saved and the file hashed. The container holds a schema, the build
/// version and a bincode payload with no clock or random input, so two equal worlds give the same number.
pub fn state_hash(w: &World) -> u64 {
    let p = temp_path("hash");
    pw_sim::save::save_with(w, &p, &pw_sim::save::Info::of_world(w)).unwrap();
    let bytes = std::fs::read(&p).unwrap();
    cleanup(&p);
    fnv(&bytes)
}

/// A digest of what happened, independent of the order of unordered maps: events (id, date, kind text), league tables, and totals of
/// the things the newer systems keep.
pub fn digest(w: &World) -> u64 {
    let mut parts: Vec<u64> = Vec::new();
    for e in w.events.since(pw_core::Date(0)) {
        parts.push(fnv(format!("{}|{}|{:?}|{:?}", e.id.0, e.date.0, e.kind, e.causes).as_bytes()));
    }
    for c in w.comps.iter() {
        for r in &c.state.table {
            parts.push(pw_core::rng::hash_key(&[u64::from(r.team.0), u64::from(r.points as u16), u64::from(r.gf), u64::from(r.ga)]));
        }
    }
    parts.push(w.net.posts.len() as u64);
    parts.push(w.media.stories.len() as u64);
    parts.push(w.incidents.list.len() as u64);
    parts.push(w.records.records.len() as u64);
    parts.push(w.talks.len() as u64);
    parts.push(w.boardroom.cases.len() as u64);
    parts.push(w.dossiers.map.len() as u64);
    parts.push(w.net.attention.len() as u64);
    parts.push(w.net.nicknames.len() as u64);
    parts.push(w.net.myths.len() as u64);
    parts.push(w.boardroom.contracts.len() as u64);
    parts.push(w.media.bonds.len() as u64);
    let mut money = 0i64;
    for c in w.clubs.iter() {
        money = money.wrapping_add(c.finance.balance).wrapping_add(c.finance.wage_bill);
    }
    parts.push(money as u64);
    let mut ability = 0u64;
    for c in w.players.cold.iter() {
        ability += u64::from(c.ca) * 3 + u64::from(c.pa) + c.value as u64 % 1000;
    }
    parts.push(ability);
    pw_core::rng::hash_key(&parts)
}

/// Where two worlds that should be identical first differ: the first event that is not the same in both, and the counts that differ.
/// A hash says that two worlds diverged; this says where to look.
pub fn explain_divergence(a: &World, b: &World) -> String {
    let ea: Vec<_> = a.events.since(pw_core::Date(0)).into_iter().collect();
    let eb: Vec<_> = b.events.since(pw_core::Date(0)).into_iter().collect();
    let mut out = format!("events {} vs {}", ea.len(), eb.len());
    if let Some((x, y)) = ea.iter().zip(eb.iter()).find(|(x, y)| format!("{:?}|{:?}|{:?}", x.date, x.kind, x.causes) != format!("{:?}|{:?}|{:?}", y.date, y.kind, y.causes)) {
        out += &format!("; first difference at event {}: {:?} {:?} / {:?} {:?}", x.id.0, x.date, x.kind, y.date, y.kind);
    }
    let counts = |w: &World| {
        vec![
            ("posts", w.net.posts.len()),
            ("stories", w.media.stories.len()),
            ("incidents", w.incidents.list.len()),
            ("records", w.records.records.len()),
            ("talks", w.talks.len()),
            ("cases", w.boardroom.cases.len()),
            ("dossiers", w.dossiers.map.len()),
            ("attention", w.net.attention.len()),
            ("contracts", w.boardroom.contracts.len()),
        ]
    };
    for ((name, x), (_, y)) in counts(a).into_iter().zip(counts(b)) {
        if x != y {
            out += &format!("; {name} {x} vs {y}");
        }
    }
    let money = |w: &World| w.clubs.iter().fold(0i64, |m, c| m.wrapping_add(c.finance.balance).wrapping_add(c.finance.wage_bill));
    if money(a) != money(b) {
        out += &format!("; money {} vs {}", money(a), money(b));
    }
    out
}
