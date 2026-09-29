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

| Idea | Status | Evidence | Tested |
| --- | --- | --- | --- |
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
(`Session::act`). Three views exist and are labelled (`world.status.perspective`, `Ctx::view_mode`): `inhabit`, `public` (a non-inhabiting
observer without the truth: `persp.observe {"public": true}`) and `observer` = the **omniscient debug view** (`omniscient: true`, the
default, unchanged for the app). Every read of engine truth is gated on `Ctx::observer()`, which is true in the omniscient view alone; the
only door to mutate a world from outside is `Api::debug_mutate_world` (`pw-view/src/debug.rs`, doc-hidden, for tests).

| Rule | Status | Evidence |
| --- | --- | --- |
| Sort/filter/search cannot leak hidden truth (§8.6) | **IMPLEMENTED** | fixed: position sort used true CA as tie-break (`tables/players.rs`); `min_ca`/`expiring_days` filters and truth sorts are refused outside the omniscient view | `crates/pw-view/tests/firewall.rs` (`player_lists_are_not_ordered_by_true_ability`, `filters_and_sorts_by_hidden_truth_are_refused...`, `a_search_cannot_be_used...`) |
| Non-interference: hidden truth changes, nothing a public or inhabited viewer sees moves (§8.10) | **IMPLEMENTED** for what is reachable through the API | `audit_public_view`, `audit_inhabited_view`: ability, potential, others' personality, how everyone feels about the viewer, journalists' and referees' private numbers changed; ~640 lists (every table, every sortable column both ways, all columns) and pages compared | firewall.rs |
| Relationships as evidence and tone, not soul meters (§8.7) | **IMPLEMENTED** (`me.people`: label, tone, trust/respect words, evidence memories; no affinity number) | `relationship_pages_show_evidence_and_tone...` (also: how others feel about the viewer moves nothing he can read) |
| Uncertain knowledge in words, not false percentages (§8.5) | **PARTIAL** | rumours, told-by-others and grapevine tells now carry `sureness`; some staff statements still quote a stated forecast (“about 60%”), and `Word.value` numbers for the viewer's own condition remain |
| Injury diagnosis private to the club (§8.10) | **IMPLEMENTED** in the player list | strangers see “Injured”, not the diagnosis or days (`Ctx::sees_medical`) |
| Debug omniscience separate and labelled (§8.8) | **PARTIAL** | separate labelled view and a single gate; observer is still the default view the desktop app opens in, and the app has no switch for the public view |
| Reviewed list of files that read engine truth (§8.9) | **IMPLEMENTED** | `engine_truth_is_read_only_in_the_files_that_gate_it` fails when a new file reads true ability, personality, relationship internals, private books or medical state |
| Enforced by types (`VisiblePlayer`, ... §8.9) | **NOT IMPLEMENTED** | pages still build JSON from `&World`; the gate is `Ctx`, the guard is the test above |

Provenance on the person page is omniscient-view only.

## 9. Typed API contracts — PARTIAL

`crates/pw-view/src/contract.rs` is the contract layer. Tests: `crates/pw-view/tests/contract.rs` (9 tests).

| Rule | Status | Evidence |
| --- | --- | --- |
| Contract drift fails early (§9.1, 9.3) | **IMPLEMENTED** for the declared types | payload types are declared once with `contract!` (Rust struct + TypeScript interface); `contract::typescript()` renders them; `app/src/contract.generated.ts` must equal it (test fails otherwise; `UPDATE_CONTRACT=1 cargo test -p pw-view --test contract` regenerates); `app/src/types.ts` re-exports the shared wire types (Ref, Named, Cell, Col, Row, TableReq/Resp, Perspective, ErrorKind) from the generated file; `npx tsc --noEmit` clean |
| Typed payloads (§9.2) | **PARTIAL** | typed and checked field-by-field against real responses: `app.info`, `world.status` (+Job, Task, settings), `table.query` (TableReq/TableResp/Col/Row/Cell), `person.attributes`, `me.people`, `me.rumours` (the last three are *built* through the contract structs). About 55 other methods still return `serde_json::Value` (listed `unknown` in `ApiMethods`); the frontend `call<any>` fallback remains for them |
| Structured errors (§9.5) | **IMPLEMENTED** | `ErrorKind` (NotFound, UnauthorizedPerspective, InvalidRequest, StateConflict, UnavailableInformation, SaveIncompatible, SimulationBusy, InternalError), `ApiError::kind()`, `ErrorBody {kind, code, message, retryable}` used by `pw-serve` (with HTTP statuses) and the Tauri command; the legacy `code()` (`bad_request`/`not_found`/`state`) is unchanged so nothing old breaks; `app/src/api.ts` `ApiError` carries `kind` and `retryable`. `UnavailableInformation` exists but no page raises it yet |
| Unknown / hidden / estimated / known distinct (§9.6) | **PARTIAL** | `Knowledge<T>` (exact, range, reported, unknown, hidden; no value for unknown or hidden), attribute rows use its tags, sureness in words. Cells still use `u: true` for unknown; most pages use `null` |
| Queries separate from commands (§9.7) | **IMPLEMENTED** as a manifest | `contract::manifest()` lists every method as query or command; a test keeps it equal to the dispatcher in `lib.rs` (it caught the newsroom methods added by a merge), every argument-free query is shown not to change the revision or the clock and to repeat exactly, and a client-supplied wage/fee on a command changes nothing |
| View types, not world types (§9.4) | **PARTIAL** | the typed payloads are view types; the untyped pages still assemble JSON from `Ctx` (see §8) |
| Contract tests cover major surfaces (§9.9) | **PARTIAL** | the surfaces above; no request validation schema for commands yet (`me.act` still parses `Value` by hand) |

## 10. Saves — PARTIAL

Current schema is **4**, and `OLDEST_SUPPORTED` is 4: schemas 1-3 were development formats (the world model changed between them without
migration steps), so they are refused with a plain message, untouched (tested: `fixture.rs::an_older_development_schema_is_refused...`, unit
tests in `save.rs`). From 4 on every serialised change bumps the number and registers a `Step`.
**IMPLEMENTED**: explicit schema version, sequential migration steps (`pw-sim::save::Step`), backup before upgrade, atomic write,
checksum, metadata (created schema, migration history, seed, provenance), clear too-new / unsupported errors, listing shows compatibility,
`stable_seed` for migrations, post-load validation and census (`validate.rs`); unit tests for the framework.
**Golden fixture**: `crates/pw-cli/tests/fixtures/golden_micro.pws` (schema 4, micro world, 90 days, 430 KB) with
`crates/pw-cli/tests/fixture.rs`: it must load (or upgrade), validate, simulate 30 days audit-clean, save and reload with the same
identities and continue exactly as the original (compared semantically: a reloaded map iterates in another order, so compressed bytes differ).
The test fails when the schema moves without a migration for the fixture; regenerate only for a deliberate break (`WRITE_GOLDEN=1`).
**Missing**: no real migration exists yet (nothing has changed since 4); a census check that no id moves *during simulation* is not
possible (people who leave the game are removed, 532 -> 525 in 30 days on the fixture), so ID preservation is checked across save/load
and migration only; the app's saves folder from before this bump lists as unsupported.

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

Foundation: events carry causes (`Cause`, `causes!`), incidents, promises, grapevine of who-knows-what, persistent relationships with
memories. Islands connected in this pass (`crates/pw-cli/tests/causal.rs`, 6 tests):

* **Settling in (§4.12-4.21 → §13):** a signing or loan stamps its event on the `Adapting` record; struggling and ending are now events
  (`AdaptationStruggling`, `AdaptationEnded`) caused by that move, and `lifestate::scan` reads a struggle on the social or mental front
  as a loneliness load that names the event.
* **Media outcomes (§2.13-2.19):** a story that leaves a lasting grievance (grudge line) emits `MediaGrudge`, caused by the `Published`
  event of the story; the subject lives it as a scandal load that names the grudge.
* **Attention (§6.16-6.24, 7.49):** `attention::spark_caused` takes the event that set it off; a wave breaking beyond football, or a big
  one, is an `AttentionSurge` caused by that event; the pile-on or hype load a person lives through names the surge (chain: match/quote/
  incident event → surge → load → manager's handling event).
* Life state and tactics (this pass) were built causal from the start: handling decisions, memory returns and tactical changes carry causes.

Still islands (state changes without an event of their own): contract package negotiation and clause reviews (`clauses.rs` pushes
uncaused events), social opinion drift (`socialnet` opinions move without events), dressing-room group mood, development stagnation,
sponsor/commerce moves, culture rivalries, referee season reviews. `audit.rs` checks only that transfer news has a transfer behind it;
there is no general audit that every consequence has a cause.

## Test inventory (mechanical)

Run `cargo test -p pw-import -p pw-sim -p pw-view -p pw-cli` for the fast suites. Long and data-dependent runs are `#[ignore]`d:
`cargo test --release -p pw-import --test real_archive -- --ignored --nocapture` needs the local `archive/` folder.
