# 14 — QA, Calibration, Fairness & Balance

## 1. Test pyramid

| Level | What | When |
|-------|------|------|
| Unit | Formulas, rule checks, contract math, tax, eligibility | Every commit |
| Property-based | Invariants (CA ≤ PA, squads meet registration, no negative money without debt, determinism) | Every commit |
| Golden seed | Fixed seed + fixed decision log → hash of world state after 30/365 days | Every commit |
| Match calibration | 10k matches per league tier → distributions vs targets | Nightly |
| Autonomy benchmark | 20 seasons headless, multiple seeds | Nightly |
| Fairness harness | Human vs AI mind swap comparisons | Nightly |
| Soak | 50-season run, memory/time growth, save/load round-trips every season | Weekly |
| UI e2e | Playwright flows (creation, week loop, match day, contract negotiation) | Per PR touching client |
| Save migration | Load golden saves from all prior versions | Per release |

## 2. Calibration targets (defaults; refine with public aggregate data)

| Metric | Target band (top tier) |
|--------|------------------------|
| Goals per match | 2.6 – 2.9 |
| Home win / draw / away win | 44–46% / 25–27% / 28–30% |
| Shots per team per match | 11 – 14 |
| Shot conversion | 9 – 12% |
| Yellow cards per match | 3.5 – 4.5 |
| Red cards per match | 0.10 – 0.20 |
| Penalties per match | 0.25 – 0.35 |
| Average player rating | 6.7 – 6.9 (σ ≈ 0.6) |
| Time-loss injuries per player-season | 1.2 – 1.8 |
| Squad turnover per season | 25 – 40% |
| Average retirement age (outfield / GK) | 33–35 / 35–38 |
| % academy players reaching top-tier senior football | 1 – 3% |
| Title winners over 20 seasons (big league) | 3–7 distinct clubs, with 1–2 dominant eras |
| Manager average tenure (top tier) | 1.5 – 2.5 seasons |

Lower tiers get their own bands (fewer goals variance by tier; more player turnover).

## 3. Autonomy benchmark (P2)

Run 20 seasons, no human input, 5 seeds. Assert:
- Every club has a legal squad at every match date; no club fields < 11 due to registration bugs.
- Finances: ≤ 3% of clubs insolvent per season; bankruptcies lead to legal resolution paths.
- Age distribution stable (no ageing world or baby world).
- CA distribution per league tier stable (no power creep/decay > 5% over 20 years).
- Transfer market liquidity within bands; free-agent pool doesn't balloon.
- Manager & staff pools replenished (retired players become coaches).
- Competition-winner diversity metrics within bands; reputations move (at least some clubs rise/fall ≥ 2 tiers in 20 years).
- Records keep being set at plausible rates.
- National teams competitive balance plausible.

## 4. Fairness harness (P1)

Method:
1. Create protagonist P with seed S.
2. Run A: P controlled by `AiMind` (his own personality). Run B: P controlled by a scripted `HumanMind` that answers with the AI's default decisions (should reproduce A exactly — determinism check).
3. Run C: many seeds, P controlled by randomised-but-reasonable human policies vs matched AI players with identical attributes/personality at the same clubs.
4. Compare outcome distributions for P vs control group: minutes, selection rate conditional on form/ability, transfer interest, injury rates, development deltas, awards.
5. Pass if differences are explained by decisions (logged) and no unexplained bias > tolerance (e.g. 2%).

Static checks: forbid reading `is_protagonist`-like flags in world crates (lint: world crates cannot depend on `career_core`; only `decision_port` types cross the boundary).

## 5. Exploit review checklist

- Stacking development inputs (capped weekly growth).
- Refusing everything to force a free transfer each year (clubs adapt: fewer offers to known hard-negotiators; reputation "difficult").
- Player Moments cherry-picking (limited frequency, same resolution model).
- Save scumming (Ironman mode; otherwise allowed but not optimised for).
- Social media spam for reputation (diminishing returns, controversy risk).
- Time budget abuse (hard caps on hours, fatigue from overloaded schedules).

## 6. Performance tests

Budget table in 01 §9; CI fails on > 10% regression in headless season time or memory.

## 7. Telemetry (local only, opt-in export)

Counters and histograms written to local files for balancing (no network). Players can export anonymised stats to share for feedback.

## 8. Bug triage categories

World-breaking (illegal state, crashes), fairness, calibration, UX, content. Every world-breaking bug gets a regression golden seed.
