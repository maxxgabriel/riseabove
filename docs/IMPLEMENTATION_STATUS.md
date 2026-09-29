# Implementation status

**What exists right now**, measured against `docs/design/LOCKED_DESIGN_DECISIONS.md` (the intent). This file is deliberately short on
prose: it says what is done, what is partial, what is missing, and where the evidence is. Code and tests decide; this file is
updated when they change.

* Branch `local/pathway-integration` (remote `integration/pathway-data-import`), audited 2026-09-29.
* Labels: **IMPLEMENTED** · **PARTIAL** · **NOT IMPLEMENTED** · **NEEDS AUDIT**. "Tested" says whether a test enforces it.
* Older reports (`INTEGRATION_REPORT.md`, `FINAL_DEPTH_PASS.md`, `WORLD_SYSTEMS.md`) describe earlier commits and may be stale.

## 1. Knowledge, perception, decisions — PARTIAL

| Rule | Status | Evidence / gap | Tested |
| --- | --- | --- | --- |
| Actors read beliefs, not hidden truth (§1.1) for team selection | **IMPLEMENTED** | `selection.rs`: ability, role fit, edge, leadership, personality read through the club's perception (`perceive`); risk and discipline from observable evidence | `manager_ai.rs` (beliefs correlate with truth, never equal it) |
| Exposure changes certainty (§1.9) | **IMPLEMENTED** | `knowledge.rs::sigma`, weekly observation in `perception.rs` | world tests |
| Several scouts, reports, disagreement (§1.6) | **PARTIAL** | `scouting.rs::view`/`disagreement`; no departmental opinions (assistant, academy head, analyst) | partial |
| Belief dossier instead of noisy PA (§1.5) | **NOT IMPLEMENTED** | a belief is `(ca, band, pa, band)`; no roles, risks, evidence list, revision history | no |
| Multi-domain judging, licences (§1.3-1.4) | **NOT IMPLEMENTED** | two staff attributes (judging ability / potential); no licences | no |
| Manager weighs staff by trust and history (§1.7) | **NOT IMPLEMENTED** | | no |
| Philosophy and context change decisions (§1.11-1.12) | **PARTIAL** | manager style from traits (rotation habit, loyalty, sports science, discipline); opposition, next fixture, promises | unit + `manager_ai.rs` |
| Decision modules do not read true ability (§1.15) | **IMPLEMENTED** | selection, squad needs, newcomer placement, loans, free-agent sweeps, renewals, projections now use `scouting::view` | `pw-sim/tests/truth_guard.rs` fails on any unmarked `.ca`/`.pa` read in `selection`, `planning`, `deals`, `market`, `negotiation`, `contracts`, `board`, `staffing`, `managers` |

Other modules (`youth`, `intl`, `awards`, `newsroom`, `incidents`, ...) are not under the guard yet; extend `DECISION_MODULES` as each is audited.

## 2. Media — PARTIAL

Exists: outlets, journalists (`JournalistProfile`: knowledge, risk, bias, hits/misses, source ties), stories with claim type
(fact/report/rumour/speculation), information fidelity incl. *planted* and *outdated*, threads, press conferences.

Gaps against the locked rules (these are the design's own priority targets):

* accuracy is judged by outcome: `media.rs::settle_credibility` rewards an outlet when the event later *happened* (§2.3) — **OPEN**
* one flat `credibility` per outlet and per journalist; no topic or club-specific credibility (§2.7) — **OPEN**
* `socialnet.rs::believes` is partly contextual (personal outlet trust, credulity, knowledge, desirability, corroboration) but
  ignores journalist identity, topic and prior belief (§2.9) — **PARTIAL**
* belief and sharing as separate decisions (§2.10) — **NEEDS AUDIT**
* truth/framing/intent/manipulation kept apart (§2.4), directional persistent media relationships with causes (§2.13-15),
  media–media rivalries (§2.18) — **NOT IMPLEMENTED** as designed

## 3. Transfers — PARTIAL

| Rule | Status | Evidence | Tested |
| --- | --- | --- | --- |
| Public value is an estimate, not the truth price (§3.25) | **IMPLEMENTED** | `market.rs::public_view`/`value_of`: stable noisy consensus narrowed by fame | `manager_ai.rs` |
| Buyer fair value and seller reservation come from each side's own reading (§3.5, 3.8-3.10) | **IMPLEMENTED** | `market.rs::fair_value`, `seller_reservation` (replacement cover, cash need, contract, board stance); `deals.rs` opening bid, ceiling, alternatives as leverage | `manager_ai.rs` |
| Neither side sees the other's limit (§3.13) | **IMPLEMENTED** | negotiation compares bids only | by construction |
| Wage demand follows public reading, not hidden ability | **IMPLEMENTED** | `wage_demand` | no |
| Replacement chains (§3.11) | **PARTIAL** | replanning is monthly; no immediate replan after a departure; no measurement | market-activity test only |
| Governance decides whose opinion counts (§3.6), causal memory of deals (§3.24) | **NOT IMPLEMENTED** | owner/board exist (`governance.rs`) but do not vote on signings | no |
| Rival-bid information with provenance, bluffing, urgency signalling (§3.14, 3.27-28) | **NOT IMPLEMENTED** | | no |
| Dynamic club risk appetite (§3.21) | **NOT IMPLEMENTED** | | no |
| Planned vs opportunistic recruitment (§4.10) | **NOT IMPLEMENTED** | | no |

## 4. Squad planning and adaptation — PARTIAL

* Position-level needs from the manager's own formation, one- and two-season projections, expected departures by renewal chance,
  wage headroom, resale, homegrown gap: **IMPLEMENTED** (`planning.rs`; unit tests for projection and renewal).
* Scenarios with confidence by horizon, succession scenarios, time-to-usefulness (§4.3-4.4, 4.19, 4.22): **NOT IMPLEMENTED**.
* Adaptation (§4.12-4.21): a single settling factor after a move abroad in weekly development (`development.rs::circumstance_factor`,
  tested). Climate, timezone, language, football, social and mental channels each on their own timeline: **NOT IMPLEMENTED**.
* Manager chooses how fast to integrate a signing (§4.20): **NOT IMPLEMENTED**.

## 5. Contracts — PARTIAL

`negotiation.rs::Terms` has wage, years, signing fee, appearance/goal/clean-sheet bonuses, release clause, promised status,
yearly rise, relegation cut, sell-on to player. Missing: club/player/mutual options and automatic extensions (§5.14), loyalty,
title, promotion, continental and cap bonuses, wage-hierarchy effects on other players' demands (§5.6), package trade-offs by
agent priorities (§5.4, 5.13), causal memory of exceptional contracts (§5.19). Promises are stored and remembered
(`social.rs::Promise`): **IMPLEMENTED**.

## 6. Social opinion — NEEDS AUDIT

Persistent accounts with personas, groups, threads, chants, memes, reposts and replies (`socialnet.rs`, 1,450 lines) exist.
Whether opinion is multidimensional (§6.1-6.2), fame/attention/commercial appeal separate (§6.18), virality and appearance
audiences (§6.16-6.22), mythology and history grounding (§6.29-6.33) is not yet verified against the code.

## 7. Tactics and life state — PARTIAL / NEEDS AUDIT

Match engine is the vendored OFM engine with its own in-match managers; selection, formations, promised roles and weekly
morale/confidence/wellbeing exist. Observation → diagnosis → adaptation with staff input (§7.1-7.34) and a unified life-to-football
state with temporal profiles (§7.35-7.54) are **NOT IMPLEMENTED** as specified; life sim (`life.rs`, `affairs.rs`) and mood
factors exist and feed some state.

## 8. Perspective firewall — PARTIAL

`pw-view::Ctx` is the single reading path: pages take a read-only `&World` scoped by perspective; actions queue typed `Intent`s
(`Session::act`). Tests: hidden state and concealed results for inhabited people. Open: sort and filter leaks on lists (§8.6),
relationship pages show numeric internals (§8.7), observer mode shows true CA/PA ("Under the hood", by design as the omniscient
mode but not yet an isolated debug path). Provenance on the person page is observer-only.

## 9. Typed API contracts — NOT IMPLEMENTED

Requests and responses are `serde_json::Value`; the frontend keeps hand-written TypeScript types (`app/src/types.ts`). Errors
carry a code (`bad_request`, `state`, `not_found`) but not the design's categories. No generated bindings, no contract tests
beyond behaviour tests.

## 10. Saves — PARTIAL

**IMPLEMENTED**: explicit schema version, sequential migration steps (`pw-sim::save::Step`), backup before upgrade, atomic write,
checksum, clear too-new / unsupported errors, listing shows compatibility; unit tests for the framework. **Missing**: post-migration
validation hook, save metadata (created schema, migration history), a deterministic legacy-init helper (`stable_seed(world, migration,
entity)`), old-save fixtures in CI, ID-preservation checks. No real migration exists yet (schema 2 is the first versioned one).

## 11. Imported data — IMPLEMENTED (with stated limits)

Pipeline parse → validate → resolve → assemble (`pw-import`); provenance per person and fact group, source ids searchable and
saved; unresolved rows kept with reasons; ability from combined evidence sampled from a posterior, never from price alone;
population calibration and determinism tested (`real_archive.rs` for the local archive, `archive.rs` for fixtures).
Limits: the club-standing prior is still derived from squad value plus last league position (correlated evidence, §11.14);
single source per fact so no disagreement resolution (§11.16); after the first month prices sit within a factor of about 1.7 of
the imported ones (median 1.3) — economy calibration is still open; personality is generated, never inferred (§11.10).

## 12. Documentation — IMPLEMENTED

`docs/README.md` states the hierarchy; this file is the current-state record; `docs/adr/` holds the architectural decisions.

## 13. Unified causal simulation — PARTIAL

Foundation exists: events carry causes (`Cause`, `causes!`), incidents, promises, grapevine of who-knows-what, persistent
relationships with memories. Not every subsystem consumes and emits through it (media outcomes, adaptation, contracts, social
opinion are the main islands).

## Test inventory (mechanical)

Run `cargo test -p pw-import -p pw-sim -p pw-view -p pw-cli` for the fast suites. Long and data-dependent runs are `#[ignore]`d:
`cargo test --release -p pw-import --test real_archive -- --ignored --nocapture` needs the local `archive/` folder.
