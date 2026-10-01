//! Prints every corpus case's outputs: `cargo run -p pw-lang --example show [case-prefix]`.
use pw_lang::{Engine, corpus};

fn main() {
    let prefix = std::env::args().nth(1).unwrap_or_default();
    let eng = Engine::builtin();
    for c in corpus::builtin_cases().expect("corpus") {
        if !c.id.starts_with(&prefix) {
            continue;
        }
        let r = corpus::run_case(&eng, &c);
        println!("== {} ({})", c.id, c.summary);
        for (i, o) in r.outputs.iter().enumerate().take(4) {
            println!("  [{i}] {o}");
        }
        for p in &r.problems {
            println!("  !! {p}");
        }
    }
}
