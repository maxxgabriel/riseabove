//! A mechanical guard for locked design §1.15: modules where clubs and managers *decide* must not read a player's true current or
//! potential ability. They ask what the club believes (`scouting::view`, `perception::club_view`) or what the market shows
//! (`market::value_of`). A line that legitimately touches the truth carries a `// truth-ok: <why>` comment.

use std::path::PathBuf;

const DECISION_MODULES: &[&str] = &["selection.rs", "planning.rs", "deals.rs", "market.rs", "negotiation.rs", "contracts.rs", "board.rs", "staffing.rs", "managers.rs"];

fn reads_truth(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or("");
    ["ca", "pa"].iter().any(|f| {
        let pat = format!(".{f}");
        code.match_indices(&pat).any(|(i, _)| !code[i + pat.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    })
}

#[test]
fn decision_modules_do_not_read_hidden_ability() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    for m in DECISION_MODULES {
        let Ok(text) = std::fs::read_to_string(src.join(m)) else { continue };
        let mut in_tests = false;
        for (n, line) in text.lines().enumerate() {
            if line.contains("#[cfg(test)]") {
                in_tests = true;
            }
            if !in_tests && reads_truth(line) && !line.contains("truth-ok") {
                offenders.push(format!("{m}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(offenders.is_empty(), "decision code reads true ability (use the club's belief, or mark `// truth-ok: why`):\n{}", offenders.join("\n"));
}

#[test]
fn the_guard_recognises_reads_and_ignores_lookalikes() {
    assert!(reads_truth("let x = w.players.cold[p].ca;"));
    assert!(reads_truth("if c.pa >= 3 {}"));
    assert!(!reads_truth("let x = w.data.cache;"));
    assert!(!reads_truth("let r = report.ca_view();"));
    assert!(!reads_truth("let x = 1; // c.ca is noted here"));
}
