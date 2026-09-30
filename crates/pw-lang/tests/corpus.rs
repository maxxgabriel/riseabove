use pw_lang::{Engine, corpus};

#[test]
fn corpus_passes() {
    let cases = corpus::builtin_cases().expect("corpus parses");
    assert!(cases.len() >= 30, "corpus shrank to {} cases", cases.len());
    let eng = Engine::builtin();
    let mut bad = Vec::new();
    for r in corpus::run_all(&eng, &cases) {
        for p in &r.problems {
            bad.push(format!("{}: {p}", r.id));
        }
    }
    assert!(bad.is_empty(), "{} problems:\n{}", bad.len(), bad.join("\n"));
}
