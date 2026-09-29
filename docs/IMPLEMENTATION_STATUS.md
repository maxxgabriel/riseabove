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

* professional accuracy is judged on claim-time truth (`Story::grounded`): `newsroom.rs::close_thread` scores journalists, their source ties and
  corrections that way, while a separate public record follows what audiences saw come to pass (§2.3, §2.11) — **IMPLEMENTED**
  (three scenario tests). Outlet-level public credibility (`media.rs::settle_credibility`) stays outcome-based on purpose (it is the public's view).
* credibility is contextual: each journalist keeps a record per club and kind of story (`JournalistProfile::ledger`, `public_trust`), and
  `socialnet.rs::believes` uses it (falling back on the outlet with little evidence), together with personal outlet trust, credulity,
  knowledge, desirability and corroboration — **IMPLEMENTED**; prior belief about the subject and source inference are still missing
* belief and sharing are separate decisions: `socialnet.rs::pass_on` (spite, humour and news value add to belief rather than follow it) —
  **IMPLEMENTED**
* both are pinned by `crates/pw-cli/tests/media_belief.rs`
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

## 7. Tactics and life state — PARTIAL (core implemented)

**Tactical pipeline (§7.1-7.34).** `pw-match::Coach` is the seam: at half an hour, half-time, the hour and the last quarter each side is
handed a `Look` (football evidence only: shots, chances, possession, territory, build-up losses, per-player duels/fouls/condition/booking)
and answers with a `Call` (new instructions, role changes, substitutions). `pw-sim/src/coach.rs` runs observe → diagnose → adapt;
`pw-sim/src/tactics.rs` holds the manager as tactician, the belief dossier, response choice, execution, learning and history.

| Idea | Status | Evidence |
| --- | --- | --- |
| Pre-match belief with uncertainty, not engine parameters (§7.2) | **IMPLEMENTED** | `tactics::dossier` (history of how the opponent was seen to play + club memory + preparation quality, fuzzed; confidence Low..VeryHigh) | `tests/tactics.rs` dossier, preparation tests |
| Deliberate surprise (§7.2) | **IMPLEMENTED** | `tactics::prepare`, rate follows adaptability and preparation | tactics.rs |
| Observation ≠ diagnosis, several hypotheses, confidence, rejected explanation (§7.4) | **IMPLEMENTED** | `coach::signs`, `diag_weights`, `Trace::{believed, rejected, confidence}` | tactics.rs (same evidence read differently; man blamed for a leak in the shape) |
| Multi-dimensional tactician, philosophy, no counter table (§7.5, 7.7) | **IMPLEMENTED** (compact) | `Profile` (prep, reading, adapt, patience, stubborn, comms, daring, ego); `applicability` is one input beside philosophy fit, lessons, memory, urgency | tactics.rs |
| Staff observations that disagree, credibility (§7.12-7.13) | **IMPLEMENTED** | assistant/analyst/fitness observers, `Credit` per (manager, staff), right/ignored-right/heeded-wrong | tactics.rs |
| Thresholds for change, half-time evidence, game state, aggregate (§7.15-7.17, 7.20) | **IMPLEMENTED** | `wanted` minutes vs evidence, `MatchCtx::urgency`, `SignKind::Scoreline` | tactics.rs |
| Opponents react (§7.14) | **IMPLEMENTED** (compact) | a structural change by the other side waives patience (`Trace::reacting`) | tactics.rs |
| Player capability / familiarity limits, training menu (§7.9-7.11, 7.22, 7.32) | **IMPLEMENTED** (compact) | `execution` (drill of target style, communication, understanding, fatigue, stress, setting), `Drill` per club, weekly rehearsal, new-manager reset; partial execution and confused instructions | tactics.rs |
| Tactical memory, staff turnover (§7.23-7.24) | **IMPLEMENTED** | `OppMemory` per (club, opponent), halves when its author leaves | tactics.rs |
| Post-match learning incl. wrong lessons (§7.29) | **IMPLEMENTED** | process verdict vs result verdict, lessons weighted by reading skill, `credited_luck`, `misreads` | tactics.rs |
| Causal traces, public record with cause (§7.34) | **IMPLEMENTED** | `Trace`, `tactics::explain`, `EventKind::MatchTacticsChanged` caused by `Fact::Played` | tactics.rs |
| Accepted vs unseen risk (§7.33) | **IMPLEMENTED** | `Trace::{saw_risk, took_risk}` | tactics.rs |
| Reputation from history (§7.28) | **IMPLEMENTED** | `tactics::reputation`, counters only | tactics.rs |

Not implemented: coaching licences and tactical schools feeding manager priors (§7.6, 7.25 — schools exist in `evolution.rs` but do not
enter `Profile`), league-wide tactical evolution driven by matches (§7.26), career development of tactical traits (§7.27), media and
player interpretations of a match differing from the internal one (§7.30), the human footballer's local tactical view and player feedback
(§7.21, 7.31), set pieces, formation changes in match (only instructions, roles and substitutions change; the OFM backend maps
instructions to its six play styles), `Lod::Standard` national-team matches (`intl.rs` is not coached). The coached path exists for the OFM
backend (the default); the native backend takes only the pre-match states. Substitution *intents* are kept for the coach's own
substitutions (`Trace`), not for the engine's fatigue rotation.

**Life-to-football state (§7.35-7.54).** `pw-world/src/lifestate.rs`, `pw-sim/src/lifestate.rs`.

| Idea | Status | Evidence |
| --- | --- | --- |
| Dynamic state carried into football; interpretation, not flat buffs (§7.35-7.36) | **IMPLEMENTED** | event → `interpret` (template bent by resilience, coping style, support) → eleven psychological channels → `football` → `Mind` (focus, calm, risk, drive, confidence) → attributes the engine reads for attention, composure, flair, work rate and form | `tests/lifestate.rs`; `pw-match/tests/coach.rs` (attentive side beats unfocused one) |
| Positive and negative through one machinery, mixed effects (§7.37) | **IMPLEMENTED** | 20 `LoadKind`s (new child: excitement + motivation up, sleep down) | lifestate.rs |
| State vs trait (§7.38) | **IMPLEMENTED** | `Temper` (stable) vs `channels` (current) | lifestate.rs |
| Temporal profiles (§7.39) | **IMPLEMENTED** | onset, peak, `Tail::{Sharp, Steady, Lingering, Recurring}`, expected vs actual duration, reminders | lifestate.rs |
| Trauma and major memories reactivate, odds not destiny (§7.40) | **IMPLEMENTED** (venue and big-penalty triggers) | `Scar`, `returning`, reaction drawn from temperament, fades with years, `MemoryReturned` | lifestate.rs |
| Pressure need not hurt, the world notices (§7.41-7.42) | **IMPLEMENTED** (compact) | resilient stakes-sharpening in `football`; `PerformedThroughStrain` only where the context is public or the manager knows, attention + manager memory follow | lifestate.rs |
| Support networks, misfires (§7.43) | **IMPLEMENTED** | `support` (partner, family, teammates, captain, manager, isolation), shorter loads, `misfired` | lifestate.rs |
| Managers perceive imperfectly, choose (§7.44, 7.54) | **IMPLEMENTED** | `Known` (Unaware/Dip/Knows, believed severity), `Handling` chosen weighing care, importance, stakes; the choice is an event with memory effects | lifestate.rs |
| State influences selection (§7.45) | **IMPLEMENTED** | `Factors::state` from the manager's belief only | lifestate.rs |
| Match events feed back (§7.46) | **PARTIAL** | after each match: mistakes, own goals, missed penalties, dismissals, heavy defeats, triumphs, injuries, captain's softening, home crowd; *not* within the match (the OFM engine cannot be altered mid-play) | lifestate.rs |
| Social discourse crosses into state and back (§7.49-7.50) | **IMPLEMENTED** (compact) | `attention::spark` → `on_attention` → `OnlineAbuse` / `Hype`; strain performance → attention | lifestate.rs |
| Life events reach people (§7.37) | **IMPLEMENTED** | `lifestate::scan` reads `Life`, `CallUp`, `ContractSigned` events | lifestate.rs |

Not implemented: contextual UI for player state with time-horizon wording (§7.47-7.48; the data exists in `Known`/`LoadKind` but no page
exposes it — see the section 8 perspective firewall work), wrongdoing propagation and private-then-leaked incidents as one machinery
(§7.51-7.53 — `incidents.rs` and `grapevine.rs` exist separately and do not yet produce `Load`s), national-team matches without minds.

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

Current schema is **3**; schemas 1 and 2 were development formats and are refused with a plain message (tested).
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
