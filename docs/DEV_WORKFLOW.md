# Development workflow: four tiers

Iterating on this codebase used to mean minutes per edit (a `pw-sim` change relinked every dependent test binary with debug info, 123 s).
It is now seconds. **No test was removed, weakened or ignored to get there**: the same tests exist, they are just run at the right time.

| Tier | When | Command | Steady-state time |
| --- | --- | --- | --- |
| **QUICK** | after every small change | `cargo xtask quick` | 0.2 s no-op, 2-6 s after an edit |
| **SMOKE** | after a coherent block of work | `cargo xtask smoke` | 3.4 s |
| **FULL** | when the work is stable | `cargo xtask full` | 8m52 on the latest integrated tree; timings vary by cache and host |
| **SOAK** | major checkpoints, or when long-run balance may have moved | `cargo xtask soak ...` | minutes, on demand |

`cargo xtask help` lists everything. Every step prints its output and elapsed time and the command stops at the first failure.

## QUICK: unit tests of what you changed

```
cargo xtask quick                      # crates with uncommitted changes (git status), lib tests, unoptimised build
cargo xtask quick -p pw-sim            # one crate
cargo xtask quick -p pw-sim selection  # one crate, test-name filter
cargo xtask quick --all                # every crate's lib tests
cargo xtask test -p pw-cli --test manager_ai selection   # one integration test target, any cargo-test arguments
```

QUICK uses the `quick` profile: our own crates at `opt-level 0` with incremental compilation, dependencies still at 3. Unit tests do not
simulate a world, so the slower generated code does not matter and the compile is far cheaper. **Do not run simulations in this profile**
(6x slower); anything that runs days of simulation belongs to SMOKE or above and uses the normal `dev` profile.

## SMOKE: the micro world

`cargo xtask smoke` runs `pw-cli/tests/smoke.rs` (four-club `Scale::MICRO` world: builds consistent, a 400-day season with audit and
structural invariants, two seasons of market/contract/intake activity, same-seed replay and save/reload continuation), the knockout
regression, the import fixtures and the perspective checks (`pw-import --test archive`, `pw-view --test imported`). About 1 s of test time.

Worlds available to `synth` and `balance`: `micro` (4 clubs), `tiny`, `small` (default), `huge`, or a number of nations.

## FULL: regression

`cargo xtask full` = all workspace tests (`cargo nextest` if installed, otherwise `cargo test --no-fail-fast`), a type-check of every target,
and a 3-seed x 3-season balance run on the tiny world. Do not run it after each edit. Note that the balance step **prints** drift findings
but does not fail on them, because the economy calibration is still open (see `docs/IMPLEMENTATION_STATUS.md`); read its output.

## SOAK: long-run correctness

```
cargo xtask soak                                   # tiny world, 5 years, seeds 1,2,3
cargo xtask soak small --years 10 --seeds 1,2,3,4
cargo xtask soak archive --years 3 --seeds 1 --real   # the real imported world (local archive/ folder) plus the ignored real-archive tests
cargo xtask soak tiny --data data/engine           # tuning read from disk at run time
```

SOAK builds the release binary once and then runs `target/release/pathway-sim.exe` **directly**: no cargo start-up, no relink, and repeated
runs cost only the simulation (micro, 2 years: 0.57 s direct vs 0.74 s through `cargo run`; more importantly, no rebuild check).
**Tuning is runtime-loaded**: `pathway-sim ... --data DIR` reads `tuning.toml`, `weights.toml`, ... from `DIR` (files or single values that
are missing fall back to the built-in ones, malformed files are refused), so calibration loops do not recompile anything. Tested by
`crates/pw-data/tests/runtime_override.rs`. Run soak when a change can affect long-term balance, population, finance, progression or save
growth, and when asked. The metrics live in `pw-sim/src/metrics.rs` (`observe`, `analyse`, `render`), CLI `pathway-sim balance`.

## Rules of thumb

1. Small change: run the narrowest QUICK test and continue.
2. After a feature block: `cargo xtask smoke`.
3. When the feature is stable: `cargo xtask full`.
4. Major checkpoint, or a change that can move the long run: `cargo xtask soak`.
5. Never skip, ignore, reduce or filter a failing test to go faster. A failure is a result. Fix the code or, if the test's premise is wrong, fix the premise and say why.
6. Never run cargo repeatedly for a binary that is already built: run the built binary.

## Profiles (Cargo.toml)

| Profile | Purpose | Notes |
| --- | --- | --- |
| `dev` / `test` | SMOKE, FULL, ordinary `cargo test` | own crates `opt-level 1`, deps 3, **`debug = 0`** |
| `quick` | QUICK unit tests | own crates `opt-level 0`, incremental, deps 3 |
| `debugging` | when you need a debugger | `dev` + full debug info: `cargo test --profile debugging` |
| `release` | SOAK | thin LTO, 4 codegen units |

Why the test profile was optimised: simulation tests run tens of thousands of match-days, and opt-level 0 makes them about 6x slower, so the
optimisation is deliberate and stays for SMOKE and FULL. What was wasteful was debug info: Cargo builds a test's dependencies with `dev`, and the
debug info made each relink of the big test binaries dominate (123 s to 8.6 s after `debug = 0`).

## Measurements (Windows 11, 12 cores, rustc 1.98)

| Operation | Before | After |
| --- | --- | --- |
| Edit `pw-sim`, rebuild `pw-cli` tests | 123 s | 8.6 s |
| Edit `pw-sim`, rebuild all workspace tests | not measured (minutes) | 12.8 s |
| Edit `pw-world`, rebuild all workspace tests | not measured | 19.8 s |
| QUICK, first build of the `quick` profile (`pw-sim`) | n/a | 6.3 s |
| QUICK, nothing changed | n/a | 0.2 s |
| QUICK, one-line edit in `pw-sim` | 5.6 s (opt 1) | 2.2 s |
| SMOKE, first run after switching profile | n/a | 39 s (compiles four test targets) |
| SMOKE, steady state | n/a | 3.4 s |
| FULL | not measured before (no single command existed) | 3 m 38 s (all tests, type-check and the simulation; tests 148 s, check 35 s, sim 34 s) |
| Latest FULL, 398 tests plus all-target check and 3-seed tiny balance | n/a | 8 m 52 s |
| Real archive SOAK, 3 years, 1 seed, with archive checks | n/a | 13 m 02 s (9 m 48 balance; 3 m 09 archive-check step, including a 2 m 56 cold test build) |
| Release build of `pathway-sim` (cold) | 1 m 32 s | 1 m 32 s (unchanged) |
| Release build, nothing changed | 0.2 s | 0.2 s |
| `balance micro --years 2` | 0.74 s via `cargo run` | 0.57 s direct |

## Tools evaluated

* **lld / rust-lld linker**: no material improvement over the MSVC linker once debug info was removed (measured, within noise). Not adopted.
* **sccache**: not adopted. Incremental compilation (which sccache cannot cache) is what makes edit loops fast, and the remaining cost is
  linking and codegen of local crates, not dependency rebuilds. Reconsider for CI, where the target directory is cold.
* **cargo-nextest**: `cargo xtask full` uses it when `cargo nextest` exists and falls back to `cargo test` otherwise. `cargo install cargo-nextest --locked`.
  Nextest does not run doctests; there are none of consequence here.
* **just**: not needed, `cargo xtask` is the same thing without another tool (`.cargo/config.toml` alias).

## CI mapping

There is no CI configuration in the repository. If one is added: every push runs SMOKE and `cargo check --workspace --all-targets`; pull
requests run FULL; SOAK is a scheduled or manually triggered job and must not gate merges.

## Remaining bottlenecks

* Cold release build (1.5 min) and cold dev test build (about 2.3 min): dependency-bound, paid once.
* First SMOKE after changing crates in the middle of the tree recompiles several test targets (26 s for `pw-import`'s).
* The `pw-cli` `manager_ai` and `world` and `pw-view` `api` test targets run whole seasons (10-80 s); they are FULL, not SMOKE, by design.
