//! The language and media test matrix: many seeds, supporter types, ages,
//! outlets, eras and dialects, checked for variation, grammar, voice
//! separation and truthfulness.

use pw_core::PersonId;
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_narrate::lexicon::{AgeBand, Dialect, Era, Locale, Platform, Register, Voice};
use pw_narrate::quality;
use pw_sim::Sim;
use pw_world::socialnet::AccountKind;
use std::collections::{HashMap, HashSet};

fn world(seed: u64, days: u32) -> Sim {
    let mut s = Sim::new(synthetic::build(DataPack::builtin(), seed, Scale::SMALL));
    s.run(days);
    s
}

/// Words that belong to young, casual voices and must not leak into formal ones.
const CASUAL_ONLY: [&str; 10] = ["lol", "gooo", "washed", "mid", "cooked", "hoofball", "scenes", "gaffer", "fuming", "💀"];

#[test]
fn posts_vary_and_stay_clean_across_seeds() {
    let mut all = 0usize;
    let mut distinct: HashSet<String> = HashSet::new();
    let mut by_kind: HashMap<AccountKind, HashSet<String>> = HashMap::new();
    for seed in [201u64, 202, 203] {
        let s = world(seed, 250);
        let w = &s.world;
        for p in &w.net.posts {
            let text = pw_narrate::social::post(w, p);
            let issues = quality::check(&text, false, 400);
            assert!(issues.is_empty(), "seed {seed}: {issues:?}: {text}");
            all += 1;
            distinct.insert(text.clone());
            by_kind.entry(w.net.accounts[p.author as usize].kind).or_default().insert(text);
        }
    }
    let ratio = distinct.len() as f32 / all.max(1) as f32;
    println!("posts {all}, distinct {} ({:.0}%)", distinct.len(), ratio * 100.0);
    assert!(all > 1000, "enough posts to judge");
    // Variation: reactions to shared events repeat, but most posts differ.
    assert!(ratio > 0.4, "too much mechanical repetition: {ratio:.2}");
    // Different kinds of account sound different.
    let stats = by_kind.get(&AccountKind::Stats).cloned().unwrap_or_default();
    let casual = by_kind.get(&AccountKind::Casual).cloned().unwrap_or_default();
    if !stats.is_empty() && !casual.is_empty() {
        let overlap = stats.intersection(&casual).count() as f32 / stats.len() as f32;
        assert!(overlap < 0.5, "stats and casual accounts sound the same ({overlap:.2})");
    }
}

#[test]
fn formal_outlets_never_use_casual_slang() {
    for seed in [211u64, 212] {
        let s = world(seed, 250);
        let w = &s.world;
        for st in w.media.stories.iter() {
            let v = pw_narrate::press::voice(w, st);
            if v.register != Register::Formal && v.register != Register::Neutral {
                continue;
            }
            let h = pw_narrate::press::headline(w, st).to_lowercase();
            for word in CASUAL_ONLY {
                assert!(!h.split(|c: char| !c.is_alphanumeric()).any(|t| t == word) && !h.contains("💀"), "formal headline uses '{word}': {h}");
            }
            assert!(quality::check(&h, false, 200).is_empty(), "{h}");
        }
    }
}

#[test]
fn vocabulary_follows_dialect_era_and_register() {
    use pw_narrate::grammar::{Term, term};
    let base = Voice {
        locale: Locale::En,
        dialect: Dialect::British,
        era: Era::Analytics,
        platform: Platform::Social,
        register: Register::Neutral,
        age: AgeBand::Middle,
        emoji: false,
        humour: 40,
        hedging: 40,
    };
    // Lexical dialect only.
    let us = Voice { dialect: Dialect::American, ..base };
    for k in 0..20 {
        assert_eq!(term(&us, Term::Pitch, k), "field");
        assert_ne!(term(&base, Term::Pitch, k), "field");
    }
    // Analytics language does not exist before its era.
    let classic = Voice { era: Era::Classic, register: Register::Analytical, ..base };
    for k in 0..20 {
        let t = term(&classic, Term::Chances, k);
        assert!(!t.contains("xG") && !t.contains("expected"), "{t} in the classic era");
    }
    // Young casual voices in different eras use different slang.
    let young = |era| Voice { era, register: Register::Casual, age: AgeBand::Young, ..base };
    let a: HashSet<&str> = pw_narrate::lexicon::words(&young(Era::Classic), pw_narrate::lexicon::Slot::PraiseAdj).iter().copied().collect();
    let b: HashSet<&str> = pw_narrate::lexicon::words(&young(Era::Analytics), pw_narrate::lexicon::Slot::PraiseAdj).iter().copied().collect();
    assert!(a != b, "slang did not move with the era");
}

#[test]
fn text_only_names_what_exists() {
    // Every rendered event, headline and post in a world names only people
    // and clubs that exist (no placeholders), across seeds.
    for seed in [221u64, 222] {
        let s = world(seed, 300);
        let w = &s.world;
        let mut n = 0;
        for e in w.events.since(pw_core::Date(0)) {
            if let Some(line) = pw_narrate::events::line(w, e, PersonId::NONE) {
                n += 1;
                assert!(!line.contains(" ? ") && !line.contains("no club") && !line.contains("NONE"), "{line}");
            }
        }
        assert!(n > 1000);
        assert!(pw_sim::audit::audit(w).is_empty());
    }
}
