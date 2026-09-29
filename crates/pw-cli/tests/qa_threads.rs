//! QA: the result of a simulation does not depend on how many threads the machine gives it. The simulation uses rayon in several places;
//! any reduction whose order depends on scheduling would make two machines disagree about the same seed. The binary is run twice, with
//! one thread and with several, and the year-end rows of its balance table (population, money, market, fame, save size) must match.

use std::process::Command;

fn table(threads: &str, world: &str, years: &str, seed: &str) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_pathway-sim")).args(["balance", world, "--years", years, "--seeds", seed]).env("RAYON_NUM_THREADS", threads).output().expect("the binary runs");
    assert!(out.status.success(), "pathway-sim failed: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    // Rows of the yearly table start with the year number; timings ("done (12ms)") and the heading's elapsed time are left out.
    text.lines().filter(|l| l.trim_start().chars().next().is_some_and(|c| c.is_ascii_digit()) && !l.contains("done (")).map(str::to_string).collect()
}

#[test]
fn one_thread_and_many_threads_give_the_same_world_micro() {
    let one = table("1", "micro", "3", "5");
    let many = table("6", "micro", "3", "5");
    assert!(one.len() >= 3, "the table has its rows: {one:?}");
    assert_eq!(one, many, "the year-end table depends on the thread count");
}

#[test]
fn one_thread_and_many_threads_give_the_same_world_tiny() {
    let one = table("1", "tiny", "3", "2");
    let many = table("8", "tiny", "3", "2");
    assert!(one.len() >= 3, "{one:?}");
    assert_eq!(one, many, "the year-end table depends on the thread count");
}
