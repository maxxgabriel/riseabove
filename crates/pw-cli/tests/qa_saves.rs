//! QA: a save file that has been damaged in any way is refused with an error, never loaded and never a panic; a good save loaded through
//! the checked path passes validation; the failed load leaves the file untouched.

mod qa_common;

use pw_import::synthetic::Scale;
use pw_sim::save;
use pw_world::World;
use qa_common::*;

fn saved() -> (std::path::PathBuf, Vec<u8>) {
    let sim = ran(Scale::MICRO, 61, 120);
    let p = temp_path("dmg");
    save::save_with(&sim.world, &p, &save::Info::of_world(&sim.world)).unwrap();
    let bytes = std::fs::read(&p).unwrap();
    (p, bytes)
}

#[test]
fn any_single_flipped_byte_is_refused() {
    let (p, good) = saved();
    let n = good.len();
    // Header, checksum, metadata and payload: a spread of positions including both ends.
    let mut positions: Vec<usize> = (0..64.min(n)).collect();
    positions.extend((1..=200).map(|i| i * n / 201));
    positions.extend(n.saturating_sub(16)..n);
    let mut accepted = Vec::new();
    for &i in &positions {
        let mut bad = good.clone();
        bad[i] ^= 0x40;
        std::fs::write(&p, &bad).unwrap();
        let r = std::panic::catch_unwind(|| save::load_world(&p));
        match r {
            Err(_) => panic!("loading a save with byte {i} flipped panicked"),
            Ok(Ok(_)) => accepted.push(i),
            Ok(Err(_)) => {}
        }
    }
    assert!(accepted.is_empty(), "damaged saves were accepted with byte(s) {accepted:?} flipped (of {n})");
    cleanup(&p);
}

#[test]
fn any_truncation_is_refused() {
    let (p, good) = saved();
    let n = good.len();
    let cuts: Vec<usize> = [0, 1, 7, 8, 11, 12, 19, 20, 21, 30, 100].into_iter().chain((1..40).map(|i| i * n / 40)).chain([n - 1]).filter(|&c| c < n).collect();
    for c in cuts {
        std::fs::write(&p, &good[..c]).unwrap();
        let r = std::panic::catch_unwind(|| save::load_world(&p));
        match r {
            Err(_) => panic!("loading a save truncated at {c} of {n} bytes panicked"),
            Ok(Ok(_)) => panic!("a save truncated at {c} of {n} bytes was accepted"),
            Ok(Err(_)) => {}
        }
    }
    cleanup(&p);
}

#[test]
fn garbage_and_empty_files_are_refused_with_an_error() {
    let p = temp_path("garbage");
    for content in [&b""[..], b"hello", &[0u8; 64][..], &[0xffu8; 4096][..], b"PWSAVE\x00\x00\x03\x00\x00\x00"] {
        std::fs::write(&p, content).unwrap();
        assert!(save::load_world(&p).is_err(), "{} bytes of junk loaded", content.len());
        assert!(std::panic::catch_unwind(|| save::inspect(&p)).is_ok(), "inspect panicked on junk");
    }
    assert!(save::load_world(&p.with_file_name("missing.sav")).is_err());
    cleanup(&p);
}

/// Note: `load_checked` runs its validator only when a save is *upgraded* (the validator guards what a migration wrote); a current-schema
/// save is decoded as it is, so a structurally broken world that was saved by a buggy build is not caught on load (`pathway-sim check`
/// and `validate::check` are the tools for that). This test pins what does hold: a good save passes, and loading never rewrites it.
#[test]
fn a_good_save_passes_the_checked_load_and_loading_changes_nothing() {
    let (p, good) = saved();
    let ok: World = save::load_checked(&p, &|w: &World| pw_sim::validate::check(w)).expect("a valid world loads through the validator");
    assert_eq!(ok.seed, 61);
    assert!(pw_sim::validate::check(&ok).is_ok());
    assert_eq!(std::fs::read(&p).unwrap(), good, "loading rewrote the file");
    cleanup(&p);
}
