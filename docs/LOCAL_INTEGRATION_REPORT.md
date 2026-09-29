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

- Review scouting, event, and social history retention before changing their visibility or historical behavior.
- Calibrate the balance, fee, wage, and population trends with multiple seeds and compare imported and synthetic worlds.
- Add the new measured timings to `DEV_WORKFLOW.md` (done in this pass); decide separately whether the slow India tests belong in a different workflow tier.
