//! QA: a ratchet around the truth guard. `truth_guard.rs` checks a fixed list of decision modules. This test covers the rest of `pw-sim`:
//!
//! * Every source file must either be a guarded decision module (checked by `truth_guard.rs`), be on the explicit list below of modules
//!   that legitimately touch true ability (they simulate the physics of the world or build beliefs out of truth), or read no true `ca` /
//!   `pa` at all. A new module that starts reading hidden ability therefore fails here until somebody looks at it and classifies it.
//! * Every `// truth-ok` marker must carry a reason (`// truth-ok: why`), or the marker means nothing.
//! * The list below must not name files that no longer exist (so it cannot rot into a blanket permission).

use std::path::PathBuf;

/// Modules whose business is the truth: they change abilities, generate people, form beliefs from evidence, or score the world.
/// (`incidents.rs` matches the pattern only because a `Conflict` has fields called `pa` and `pb`; `economy.rs` measures each
/// league's true strength for its macro model; `reputation.rs` drives world reputation from true ability.)
const LEGITIMATE: &[&str] = &[
    "development.rs",
    "growth.rs",
    "health.rs",
    "people.rs",
    "minor.rs",
    "metrics.rs",
    "reputation.rs",
    "perception.rs",
    "scouting.rs",
    "dossier.rs",
    "incidents.rs",
    "economy.rs",
];

/// The modules `truth_guard.rs` already polices line by line.
const GUARDED: &[&str] = &[
    "selection.rs", "planning.rs", "deals.rs", "market.rs", "negotiation.rs", "contracts.rs", "board.rs", "staffing.rs", "managers.rs", "decisions.rs",
    "consider.rs", "mind.rs", "agents.rs", "youth.rs", "intl.rs", "social.rs", "talk.rs",
];

/// Newer modules where clubs, players and the press decide. They read no true ability today; this pins it.
const MUST_STAY_CLEAN: &[&str] = &[
    "boardroom.rs", "bargaining.rs", "package.rs", "clauses.rs", "adaptation.rs", "mediarel.rs", "attention.rs", "newsroom.rs", "socialnet.rs",
    "pressroom.rs", "media.rs", "governance.rs", "finance.rs", "grapevine.rs", "commerce.rs", "culture.rs", "honours.rs", "awards.rs", "inbox.rs",
    "press.rs", "responses.rs", "interpret.rs", "renown.rs", "life.rs", "affairs.rs", "medical.rs", "dressing.rs",
];

fn src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn reads_truth(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or("");
    ["ca", "pa"].iter().any(|f| {
        let pat = format!(".{f}");
        code.match_indices(&pat).any(|(i, _)| !code[i + pat.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    })
}

/// Unmarked reads of true ability in the non-test part of a file: (line number, text).
fn unmarked_reads(file: &str) -> Vec<(usize, String)> {
    let text = std::fs::read_to_string(src().join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.contains("#[cfg(test)]") {
            break;
        }
        if reads_truth(line) && !line.contains("truth-ok") {
            out.push((n + 1, line.trim().to_string()));
        }
    }
    out
}

fn all_sources() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(src()).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".rs")).collect();
    v.sort();
    v
}

#[test]
fn no_unclassified_module_reads_hidden_ability() {
    let mut offenders = Vec::new();
    for f in all_sources() {
        if GUARDED.contains(&f.as_str()) || LEGITIMATE.contains(&f.as_str()) {
            continue;
        }
        for (n, l) in unmarked_reads(&f) {
            offenders.push(format!("{f}:{n}: {l}"));
        }
    }
    assert!(
        offenders.is_empty(),
        "these modules are neither guarded nor listed as legitimate users of true ability; mark the line `// truth-ok: why`, route it through `scouting::view`, or classify the module:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn newer_decision_modules_stay_clean() {
    let mut offenders = Vec::new();
    for f in MUST_STAY_CLEAN {
        for (n, l) in unmarked_reads(f) {
            offenders.push(format!("{f}:{n}: {l}"));
        }
    }
    assert!(offenders.is_empty(), "decision modules read hidden ability:\n{}", offenders.join("\n"));
}

#[test]
fn every_truth_ok_marker_has_a_reason_and_the_lists_name_real_files() {
    let mut bare = Vec::new();
    for f in all_sources() {
        let text = std::fs::read_to_string(src().join(&f)).unwrap();
        for (n, line) in text.lines().enumerate() {
            if let Some(i) = line.find("truth-ok") {
                let rest = line[i + "truth-ok".len()..].trim_start_matches(':').trim();
                // Only comments count: the guard tests themselves mention the marker in strings.
                let is_comment = line[..i].contains("//") && !line.trim_start().starts_with("//!") && !line.trim_start().starts_with("///");
                if is_comment && !line[i..].starts_with("truth-ok:") || is_comment && rest.len() < 8 {
                    bare.push(format!("{f}:{}: {}", n + 1, line.trim()));
                }
            }
        }
    }
    assert!(bare.is_empty(), "a truth-ok marker without a reason:\n{}", bare.join("\n"));
    let files = all_sources();
    for name in LEGITIMATE.iter().chain(GUARDED).chain(MUST_STAY_CLEAN) {
        assert!(files.iter().any(|f| f == name), "the lists name {name}, which does not exist");
    }
}
