//! The development workflow, one command per tier. See `docs/DEV_WORKFLOW.md` and `cargo xtask help`.
//!
//! Every step runs with its output visible and the whole command stops at the first failure. Nothing is filtered, retried or ignored.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Instant;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".into());
    let rest: Vec<String> = args.collect();
    let ok = match cmd.as_str() {
        "quick" => quick(&rest),
        "test" => step("cargo test (targeted)", cargo(&["test"], &rest)),
        "smoke" => smoke(&rest),
        "full" => full(),
        "soak" => soak(&rest),
        "changed" => {
            println!("{}", changed_crates().join(" "));
            true
        }
        _ => {
            print!("{}", include_str!("help.txt"));
            cmd == "help"
        }
    };
    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn cargo(head: &[&str], tail: &[String]) -> Command {
    let mut c = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.args(head).args(tail);
    c
}

/// Run one step with its output on screen and report how long it took. Returns false on failure.
fn step(label: &str, mut c: Command) -> bool {
    println!("\n>> {label}\n   {}", describe(&c));
    let t = Instant::now();
    let status = c.status();
    let secs = t.elapsed().as_secs_f32();
    match status {
        Ok(s) if s.success() => {
            println!("<< ok: {label} ({secs:.1}s)");
            true
        }
        Ok(s) => {
            println!("<< FAILED: {label} ({secs:.1}s, {s})");
            false
        }
        Err(e) => {
            println!("<< FAILED to start {label}: {e}");
            false
        }
    }
}

fn describe(c: &Command) -> String {
    let mut s = c.get_program().to_string_lossy().to_string();
    for a in c.get_args() {
        s.push(' ');
        s.push_str(&a.to_string_lossy());
    }
    s
}

fn has_nextest() -> bool {
    Command::new("cargo").args(["nextest", "--version"]).output().is_ok_and(|o| o.status.success())
}

// ---------------------------------------------------------------------------------------------------------------------- quick

/// Crates with uncommitted changes (or, on a clean tree, those changed by the last commit).
fn changed_crates() -> Vec<String> {
    let lines = |args: &[&str]| -> Vec<String> {
        Command::new("git").args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect()).unwrap_or_default()
    };
    let mut files: Vec<String> = lines(&["status", "--porcelain=v1", "-uall"])
        .into_iter()
        .filter_map(|l| l.get(3..).map(|p| p.rsplit(" -> ").next().unwrap_or(p).trim_matches('"').to_string()))
        .collect();
    if files.is_empty() {
        files = lines(&["diff", "--name-only", "HEAD~1", "HEAD"]);
    }
    let mut out: Vec<String> = Vec::new();
    for f in files {
        let f = f.replace('\\', "/");
        let krate = if let Some(rest) = f.strip_prefix("crates/") {
            rest.split('/').next().map(str::to_string)
        } else if f.starts_with("data/engine/") {
            Some("pw-data".to_string())
        } else if f.starts_with("xtask/") {
            Some("xtask".to_string())
        } else {
            None
        };
        if let Some(k) = krate
            && (k == "xtask" || Path::new("crates").join(&k).join("Cargo.toml").exists())
            && !out.contains(&k)
        {
            out.push(k);
        }
    }
    out
}

fn quick(args: &[String]) -> bool {
    let (mut crates, mut filters, mut all) = (Vec::<String>::new(), Vec::<String>::new(), false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-p" | "--package" => crates.extend(it.next().cloned()),
            "--all" => all = true,
            _ => filters.push(a.clone()),
        }
    }
    if crates.is_empty() && !all {
        crates = changed_crates();
        if crates.is_empty() {
            println!("no changed crate found; testing every crate's unit tests (use -p CRATE to choose)");
            all = true;
        } else {
            println!("changed crates: {}", crates.join(", "));
        }
    }
    let mut c = cargo(&["test", "--profile", "quick", "--lib"], &[]);
    if all {
        c.arg("--workspace");
    }
    for k in &crates {
        c.args(["-p", k]);
    }
    if !filters.is_empty() {
        c.arg("--").args(&filters);
    }
    step("QUICK: unit tests, unoptimised build", c)
}

// ---------------------------------------------------------------------------------------------------------------------- smoke

/// Integration checks on the micro world and small fixtures: (package, test target).
const SMOKE: &[(&str, &str)] = &[("pw-cli", "smoke"), ("pw-cli", "knockouts"), ("pw-import", "archive"), ("pw-view", "imported")];

fn smoke(args: &[String]) -> bool {
    for (pkg, test) in SMOKE {
        let mut c = cargo(&["test", "-p", pkg, "--test", test], &[]);
        if !args.is_empty() {
            c.arg("--").args(args);
        }
        if !step(&format!("SMOKE: {pkg} {test}"), c) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------------------------------------------------- full

fn full() -> bool {
    let tests = if has_nextest() {
        step("FULL: all workspace tests (nextest)", cargo(&["nextest", "run", "--workspace"], &[]))
    } else {
        println!("(cargo-nextest not installed: running `cargo test`; see docs/DEV_WORKFLOW.md)");
        step("FULL: all workspace tests", cargo(&["test", "--workspace", "--no-fail-fast"], &[]))
    };
    tests
        && step("FULL: type-check every target", cargo(&["check", "--workspace", "--all-targets"], &[]))
        && step(
            "FULL: short multi-seed simulation (3 seeds x 3 seasons, tiny)",
            cargo(&["run", "--quiet", "-p", "pw-cli", "--bin", "pathway-sim", "--", "balance", "tiny", "--years", "3", "--seeds", "1,2,3"], &[]),
        )
}

// ---------------------------------------------------------------------------------------------------------------------- soak

fn soak(args: &[String]) -> bool {
    // Build the release binary once; cargo does nothing when the code has not changed.
    if !step("SOAK: release binary (no-op when up to date)", cargo(&["build", "--release", "-p", "pw-cli", "--bin", "pathway-sim"], &[])) {
        return false;
    }
    let real = args.iter().any(|a| a == "--real");
    let rest: Vec<&String> = args.iter().filter(|a| *a != "--real").collect();
    // A leading word that is not a flag is the world to run; default to the tiny one.
    let mut c = Command::new(if cfg!(windows) { "target/release/pathway-sim.exe" } else { "target/release/pathway-sim" });
    c.arg("balance");
    if rest.first().is_none_or(|a| a.starts_with("--")) {
        c.arg("tiny");
    }
    c.args(rest);
    // Run the binary directly: no cargo, no relink; tuning can change with `--data data/engine` without rebuilding.
    let ok = step("SOAK: long-run balance analysis", c);
    if ok && real {
        return step(
            "SOAK: real-archive checks (needs the local archive/ folder)",
            cargo(&["test", "--release", "-p", "pw-import", "--test", "real_archive", "--", "--ignored", "--nocapture"], &[]),
        );
    }
    ok
}
