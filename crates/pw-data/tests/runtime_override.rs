//! Tuning can be changed at run time (`pathway-sim ... --data DIR`) without recompiling: a directory with only some files, holding
//! only some values, overrides exactly those and leaves the rest as built in. Calibration loops depend on this.

use pw_data::DataPack;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("pw-data-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_partial_tuning_file_overrides_only_what_it_names() {
    let builtin = DataPack::builtin();
    let dir = temp_dir("override");
    std::fs::write(dir.join("tuning.toml"), "[market]\nvalue_exp = 0.05\n").unwrap();
    let loaded = DataPack::load_dir(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!((loaded.tuning.market.value_exp - 0.05).abs() < 1e-9, "the named value changes");
    assert_ne!(loaded.tuning.market.value_exp, builtin.tuning.market.value_exp, "and differs from the built-in one");
    assert_eq!(loaded.tuning.market.value_base, builtin.tuning.market.value_base, "a value the file does not name keeps its default");
    assert_eq!(loaded.tuning.development.growth, builtin.tuning.development.growth, "other sections are untouched");
    assert_eq!(loaded.formations.len(), builtin.formations.len(), "files that are absent fall back to the built-in ones");
}

#[test]
fn an_empty_directory_is_the_builtin_pack_and_a_broken_file_is_an_error() {
    let dir = temp_dir("empty");
    let same = DataPack::load_dir(&dir).unwrap();
    assert_eq!(same.tuning.market.value_exp, DataPack::builtin().tuning.market.value_exp);

    std::fs::write(dir.join("tuning.toml"), "[market\nvalue_exp = ").unwrap();
    let err = DataPack::load_dir(&dir).expect_err("a malformed file is refused, never silently replaced by defaults");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(err.to_string().contains("tuning.toml"), "the error names the file: {err}");
}
