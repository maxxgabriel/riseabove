# Notes for agents and developers

Project: Rise Above / Pathway, a Rust workspace. Design intent is `docs/design/LOCKED_DESIGN_DECISIONS.md`; current state is
`docs/IMPLEMENTATION_STATUS.md`. Full workflow details and timings: `docs/DEV_WORKFLOW.md`.

## Test workflow (fast iteration)

1. **Small change**: run the narrowest test and continue: `cargo xtask quick` (unit tests of the crates you changed) or
   `cargo xtask quick -p CRATE FILTER`, or one target with `cargo xtask test -p pw-cli --test manager_ai NAME`.
2. **After a coherent block of work**: `cargo xtask smoke` (micro world, a few seconds).
3. **When the feature is stable**: `cargo xtask full` (all tests, checks, short multi-seed run; minutes).
4. **Soak only** when a change can affect long-term balance, population, finance, progression or save growth, or when asked:
   `cargo xtask soak ...` (release binary run directly; `--data DIR` changes tuning without rebuilding).
5. Do not run FULL or SOAK after small edits, do not run `cargo run` repeatedly for a binary that is built, and do not run simulations
   in the `quick` profile.
6. Never skip, ignore, filter or weaken a test to go faster. If a test fails, that is the result: fix the code, or fix a wrong test premise and say so.

## Repository rules

* Never commit the local data folders (`archive/`, `fm23_extracted/`, `fm23_test_extract_*`): they are git-ignored.
* Decision code reads beliefs, not hidden truth (`scouting::view`); `crates/pw-sim/tests/truth_guard.rs` enforces it (`// truth-ok: why` marks a legitimate read).
* Imported facts are labelled Imported / Inferred / Generated / Unknown; missing is never zero; people are never merged by name alone.
* Do not push, force-push or rewrite history without being asked.
