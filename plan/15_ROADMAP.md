# 15 — Roadmap, Milestones & Acceptance Criteria

Assumes a small team (1–3 developers) or a solo developer with AI assistance. Durations are rough and sequential; parallelise where marked.

## Phase 0 — Orientation & harness (3–4 weeks)

- Fork open-football; build headless; run `.dev/simulate`; write architecture notes of its tick, entities, persistence.
- Create `sim_cli` wrapper with seed control, Parquet export, timing.
- Establish determinism audit (identify non-deterministic iteration, RNG sources).
**Acceptance:** same seed → identical world hash after 365 days on two machines; baseline metrics dashboard exists.

## Phase 1 — World autonomy baseline (4–6 weeks)

- Calibration pass on match output, transfers, development, retirements using 14 §2 targets.
- Fill gaps discovered in open-football (e.g. registration rules, discipline carry-over) via `world_rules`.
**Acceptance:** 20-season autonomy benchmark passes on 3 seeds; calibration within bands for top 2 tiers.

## Phase 2 — Protagonist injection & DecisionPort (4 weeks)

- Mind trait; migrate player decisions (contracts, transfers, loans, training focus) behind DecisionPort.
- `HumanMind` with deadlines/defaults; decision log; replay.
- Fairness harness v1.
**Acceptance:** protagonist exists as ordinary Player; fairness harness A/B identical; decisions replay.

## Phase 3 — Perception layer (4 weeks, parallel with 4)

- Observer perceptions (coach, scout, agent, self), uncertainty, decay; universal "known to buyer" rule.
**Acceptance:** UI-facing viewmodels read only perceptions; lint prevents truth reads.

## Phase 4 — Match output bridge & career events (3 weeks)

- Output contract (06 §11) exposed; full LOD for protagonist; career event stream (debuts, goals, milestones).
**Acceptance:** protagonist match history with per-player stats stored and queryable across seasons.

## Phase 5 — Client vertical slice (6 weeks)

- Tauri app: Home, Calendar, Inbox, Match Day (text + radar), Performance, Career/Contract, basic Training.
- Week loop playable from age 17 at a generated club.
**Acceptance:** a tester plays one full season with debuts, selection battles, a renewal, and a loan offer without dev tools.

## Phase 6 — Selection AI & dressing room (4 weeks)

- Hungarian assignment selector with explainability; selection forecast; manager conversations; promises; relationships graph v1.
**Acceptance:** manager explanations match logged top factors ≥ 90%; forecast calibration (predicted vs actual start rate within 10%).

## Phase 7 — Market depth for the player (5 weeks)

- Agent model & UI; interest board; personal-terms negotiation; clauses; loans with clauses; pre-contracts; free agency & trials.
**Acceptance:** 10 scripted scenario tests (e.g. relegation clause trigger, recall, pre-contract, work permit refusal) pass.

## Phase 8 — Development & health depth (5 weeks)

- Training UI, individual plans, trait learning, mentoring, load/ACWR, injury catalogue, rehab flow, body wear.
**Acceptance:** injury & development distributions within bands; UI shows rehab phases correctly.

## Phase 9 — Life sim v1 (6 weeks)

- Time budget, well-being, family, friends, partner (light), housing, relocation, personal finances & taxes, sponsorships; AI statistical life model.
**Acceptance:** fairness harness still passes with life sim on; life effects within documented bounds.

## Phase 10 — Media, reputation, awards, history (5 weeks)

- Press conferences, social media, fan affinity, awards voting, records, legacy screen.
**Acceptance:** 20-season run produces believable award winners & records; legacy screen complete.

## Phase 11 — Youth start & education (5 weeks)

- Grassroots/academy stages, school, retention reviews, growth spurts, youth internationals.
**Acceptance:** start at 12 and reach a first pro-contract decision naturally in playtests.

## Phase 12 — International career (4 weeks)

- Eligibility engine completeness, federation approaches, switching, tournaments.
**Acceptance:** eligibility test matrix (30 cases) passes.

## Phase 13 — Retirement & post-career (4 weeks)

- Retirement flow, testimonial, coaching badges, punditry, epilogue.
**Acceptance:** complete life from creation to epilogue in a long playtest.

## Phase 14 — Polish, modding, localisation, performance (6+ weeks)

- Data-pack tooling, editor, accessibility audit, performance targets, save migration tests, content variety (commentary/news pools).
**Acceptance:** release candidate criteria (all budgets met, zero world-breaking bugs in 50-season soak).

## Cumulative timeline

~80–90 weeks for a small team to full scope; first playable vertical slice at ~20 weeks (end of Phase 5).

## Risk register

| Risk | Impact | Mitigation |
|------|--------|-----------|
| open-football internals harder to extend than expected | High | Phase 0 deep study; keep extensions in separate crates; upstream-friendly patches |
| Determinism issues with rayon/float | High | Early audit; fixed-point in outcome paths |
| Match engine calibration time sink | Medium | Lab notebooks, parameter sweeps, automated fitting |
| Scope creep in life sim | Medium | Bounded effects; v1 list fixed |
| Performance at Large world | Medium | SoA, profiling budget per phase, inactive-league statistical mode |
| Fairness regressions | High | Nightly harness; lint boundaries |
| GPL contamination from OFM | Legal | Clean-room policy: no code copying, design notes only |
| Burnout of solo dev | High | Vertical slices with playable value early |
