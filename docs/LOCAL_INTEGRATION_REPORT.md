# Local integration report

Date: 2026-09-30

## Checkout

- Work was done in `E:\pers\riseabove` on `local/pathway-integration`.
- HEAD remained `9e64e26`; the branch was 76 commits ahead of `origin/integration/pathway-data-import` when checked.
- No commit or push was made. The retention and diagnostic changes in this report are local working-tree changes.
- The separate tester worktree was inspected; its edits were not copied wholesale.

## Changes in this pass

- Bound full dossiers to players each club has a current reason to assess, with a short grace period for recently dropped assessments.
- Age out obsolete grapevine holders and compact old closed negotiation details while preserving IDs, private terms, and audit outcomes.
- Added save-size-by-section and first-team wage metrics. The wage drift check now uses first-team wages so the large youth pool does not hide senior wage inflation.
- Added save-bound QA coverage and fixed lowercase incident headlines in press copy.
- No save format or simulation-rule migration was introduced.

## Verification

- `cargo xtask full`: 398 tests passed, 20 ignored; all-target workspace check passed. The tiny three-seed balance step completed and printed economy warnings.
- After the final metrics diagnostic edit: `cargo xtask quick -p pw-sim` passed 47/47, and `cargo xtask test -p pw-cli --test qa_save_bounds -- --nocapture` passed 3/3.
- `cargo xtask soak archive --years 3 --seeds 1 --real`: balance run completed; all 3 real-archive tests passed.
- `git diff --check` passed.

## Real archive save comparison

Same imported archive and seed, before and after retention changes; sizes are decimal MB.

| Year | Before | Current | Reduction |
| ---: | ---: | ---: | ---: |
| 0 | 40.3 | 40.3 | 0% |
| 1 | 785.0 | 454.8 | 42.1% |
| 2 | 1,304.4 | 700.8 | 46.3% |
| 3 | 1,652.0 | 800.6 | 51.5% |

At year 3, the largest serialized sections were scouting (162.8 MB), dossiers (125.1 MB), events (119.2 MB), grapevine (77.9 MB), social (47.7 MB), and media (36.6 MB). The tiny-world six-year QA also passed its sublinear-growth bound: 13.8 MB at year 6, with dossiers at 0.9 MB.

The archive save is about half its previous size, though it is still large and continues to grow. The largest remaining sections need separate retention review before further history is discarded.

## Archive drift

The initial all-player wage series was misleading because it includes youth players with no weekly wage. In this run, the median all-player wage ended at 309, while the first-team median rose from 22k at import to 44k in year 1, 56k in year 2, and 68k in year 3. The corrected first-team trend is about 24% annual growth from year 1 to year 3.

First-team players fell from 19,092 to 12,553, settling at 22.9 per club across 548 clubs. Youth-side players grew to 24,687. The imported first-team squads began above the configured target and maximum, so this run shows roster normalization as well as population movement; it does not establish whether the resulting shrinkage is desirable for every competition.

The archive run still reports median club balances growing about 45% per year, transfer fees about 43% per year, and active players about 19% per year. Mean ability moved down 5.7 points; serialized save size grew about 33% per year after the first full year. The updated wage check now flags the senior wage inflation directly. These economy and population trends remain open for calibration; no tuning adjustment was made from a single archive seed.

## Remaining work

- Scouting retention is implemented in the follow-up below; event and social histories retain their existing historical behavior.
- Calibrate the balance, fee, wage, and population trends with multiple seeds and compare imported and synthetic worlds.
- Add the new measured timings to `DEV_WORKFLOW.md` (done in this pass); decide separately whether the slow India tests belong in a different workflow tier.

## Save-growth follow-up

Scope: save growth only. The academy and wage experiments explored during this pass were removed. `market.rs` matches the
pre-pass wage calculation at `9e64e26`.

The monthly scouting pass now retains the newest 1,000 general subjects per club, with deterministic player-ID ordering for date ties.
Own players (including owned players on loan), explicit player assignments, shortlisted players, open-deal targets, and human-controlled
players are protected separately, along with pending pre-contracts and current senior and academy trials. Retired subjects leave the report book. Removing a full general report does not remove the compact
`Knowledge::Seen` exposure record, so the club still has its existing evidence for its fallback reading.

This bounds the broad scouting working set alongside the dossier limit, closed-information holder compaction, and closed-negotiation
detail compaction from the earlier checkpoint. Stable IDs, story source paths, final negotiation outcomes, and private limits remain.
Historical rows still accumulate as actual football history; this work limits the large working records attached to them.

`qa_scouting_retention` checks the report limit, deterministic insertion-order independence, named targets, shortlists, owned loans,
retired players, idempotence, and save/reload preservation. The six-year `qa_save_bounds` case now measures compressed file size as well
as raw serialized state and retains its existing growth checks.

The final targeted follow-up checks pass (4 tests). On the six-year tiny world, raw state is 9.4 MB in year 3 and 13.8 MB in year 6;
compressed saves are 3.8 MB and 5.6 MB respectively. The raw-state addition falls from 2.6 MB in year 2 to 1.6 MB in year 6.
These figures use the original academy lifecycle and wage calculation.

The follow-up full run encountered an unrelated golden-fixture decode failure (`UnexpectedEof` at `fixture.rs:49`), before any
retention code runs. Concurrent save-envelope and eligibility changes landed while this pass was running; this failure is reported
and its test remains enabled. The full run finished with 397/399 passing; its other failure was the tactics observation case, which
passes in a targeted rerun after the wage rollback. It is not a passing full-suite result. The workspace all-target check passed.

The final release archive run (seed 1, original wage calculation and academy lifecycle) completed three years in 429 seconds:

| Year | Baseline raw state (MB) | Retained raw state (MB) | Reduction |
| ---: | ---: | ---: | ---: |
| 0 | 40.3 | 40.3 | 0% |
| 1 | 785.0 | 414.5 | 47.2% |
| 2 | 1,304.4 | 601.7 | 53.9% |
| 3 | 1,652.0 | 715.2 | 56.7% |

At year 3 the largest sections are dossiers 123.8 MB, events 118.8 MB, scouting 80.1 MB, grapevine 77.1 MB,
social 47.5 MB and media 36.5 MB. These are uncompressed serialized-state measurements, not actual `.pws` file sizes.
The final additional protections for pre-contracts and trials were added after that release build.
Three six-year tiny release runs (seeds 1, 2, 3) end at 13.6, 13.6 and 13.8 MB, with no structural findings.
The archive still reports 31% annual state growth from years 1 to 3 and economy/population warnings; historical records are not bounded.

The final scouting regression passed after adding all three commitment protections (senior trial, academy trial and pre-contract),
including a compressed save/reload round trip. No save schema was changed by the retention work.

## Database and missing-value follow-up

The separate source catalog exposes all 13 archive CSVs (7,506,407 rows) and 48 FM23 exports (6,221,995 rows, including duplicate
exports/localizations). All tables were scanned; no malformed rows were reported. Source files were not modified or committed.
Raw data and disposable indexes stay outside world saves. The source browser works before opening a world and in omniscient
observer mode; inhabited/public queries are refused. Related records use exact source IDs; world links additionally require the
Transfermarkt source namespace. FM IDs and names never become Transfermarkt identities.

Missing valuations can use non-conflicting recent dated history, and missing roles can use repeated recent club lineups. Neither
uses future records. Unobserved career totals are unavailable, missing goal fields do not become zero, and recorded zero values
and wages survive assembly. Without a usable valuation, the existing public pricing model supplies a labelled initial estimate.
The final local archive recovers eight positions from lineups. Its 3,079 estimated player values are labelled; there is no recent
history valuation to recover in this dataset's remaining missing-value cases. No birth dates or unverified club links are invented.

Release indexing took 32.385 seconds for every archive table and 7.863 seconds for every FM export. Restart scans using disk
indexes took 3.357 and 1.409 seconds respectively. Normal browsing indexes only the selected table. Dense ID postings reduce
appearances from 174.1 to 99.2 MB and lineups from 192.0 to 117.3 MB. The catalog evicts older indexes toward 96 MiB, retaining a
larger active table when necessary; construction allocations and world memory are additional.

The real-archive tests pass 3/3: coherent world structure, cohort/price calibration, and first-month price stability. They import
201 nations, 54 competitions, 548 clubs, 19,092 players and 3,957 staff, with 4,593 unresolved source rows reported. Price/ability
correlation is 0.81; recorded-price players average CA 107.3 at age 16–19 and 117.9 at age 24–29. First-month median new/recorded
price is 1.20, with p10 0.30 and p90 4.58. Calibration excludes model-estimated prices, since they are not independent source evidence.

The first full run of this follow-up completed 425/427. Its two test premises were corrected with their assertions retained:
`stop_lands_on_a_day_boundary` now waits for observed progress instead of assuming a worker starts within 50 ms;
the manager interpretation case isolates the varied manager from advisers inherited from its simulated season. Both pass narrow
reruns. No production tactics or stopping rules were changed. The second full run finished 425/427, exposing appearances without
dates in the older fixture export. The importer now recovers a missing appearance date from the exact game record, excludes
conflicting/future dates, and the career fixture supplies its previously absent game records. All 19 archive regressions pass.
The third full run finished 426/427. Its binary had been compiled before a concurrent commit (`1e2e389`) corrected the imported
journey's player selection: a fixed named player was not guaranteed minutes. The committed test selects a first-team player by
recorded appearances instead. Its targeted rerun passes. The fourth full run passes **427/427**, followed by the workspace
all-target check and the three-seed, three-season tiny simulation. It uses four concurrent test cases to reduce contention;
no test assertions are removed. Nextest reports 21 existing ignored tests; the three real-archive checks were explicitly run
separately and pass. Timings: tests 540.7 seconds including compilation, all-target check 5.9 seconds, simulation 44.1 seconds.
The simulation command succeeds but reports six balance problems across its three seeds: club balances and accumulated state
growth (42–44% annually over its early years), plus wage/reputation/ability warnings. This is not a clean balance result or a claim
that all history growth has been eliminated. The separate six-year retention and compressed-save regressions pass.

The browser review passes at 1366×768, 1440×900, 1920×1080 and 390×844, including pagination, source history links, FM name search,
larger text, keyboard focus, reduced-motion mode, imported profile → source → world links and refusal in inhabited mode.
It found and fixed standalone scrolling, narrow database overflow, native input styling and narrow shell-header overflow.
`npm run build`, the final four database/missing-value regressions and the workspace all-target check pass.
The browser review also passes against the latest release server binary, including an actual archive import and the observer switch.

FM23's CSV exports are available, but its raw player/staff/contract schemas are still undecoded. This is not complete playable FM23
integration. See `DATABASE_INTEGRATION.md` for the implemented boundary and remaining schema dependency.
