//! QA diagnostic: after a save and reload, find the first day and the first event where a reloaded world leaves the uninterrupted one.
//! Prints the events around the divergence. Run: `cargo test -p pw-cli --test qa_reload_diag -- --ignored --nocapture`.

mod qa_common;

use pw_import::synthetic::Scale;
use pw_sim::Sim;
use qa_common::*;

fn first_divergence(scale: Scale, seed: u64, before: u32, after: u32) -> Option<String> {
    let mut a = sim(scale, seed);
    a.run(before);
    let p = temp_path("diag");
    pw_sim::save::save_with(&a.world, &p, &pw_sim::save::Info::of_world(&a.world)).unwrap();
    let mut b = Sim::new(pw_sim::save::load_world(&p).unwrap());
    cleanup(&p);
    for day in 0..after {
        a.step();
        b.step();
        let (ea, eb) = (a.world.events.all(), b.world.events.all());
        let n = ea.len().min(eb.len());
        let f = |e: &pw_world::event::Event| format!("{:?}|{:?}|{:?}|{:?}", e.date, e.vis, e.kind, e.causes);
        let first = (0..n).find(|&i| f(&ea[i]) != f(&eb[i]));
        if first.is_some() || ea.len() != eb.len() {
            let mut out = format!("day {day} after reload ({}): lens {} vs {}, first differing index {first:?}\n", a.world.date, ea.len(), eb.len());
            let i = first.unwrap_or(n);
            for j in i.saturating_sub(3)..(i + 4) {
                if let Some(e) = ea.get(j) {
                    out += &format!("A[{j}] {}\n", f(e));
                }
                if let Some(e) = eb.get(j) {
                    out += &format!("B[{j}] {}\n", f(e));
                }
            }
            return Some(out);
        }
    }
    None
}

#[test]
#[ignore = "diagnostic"]
fn where_does_the_reloaded_world_diverge() {
    let r = first_divergence(Scale::TINY, 14, 330, 120);
    eprintln!("{}", r.unwrap_or_else(|| "no divergence in events".into()));
}
