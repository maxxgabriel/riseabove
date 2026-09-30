use pw_lang::Lang;

#[test]
fn builtin_data_loads_and_lints_clean() {
    let lang = Lang::builtin();
    let errs = lang.lint();
    assert!(errs.is_empty(), "language data problems:\n{}", errs.join("\n"));
}
