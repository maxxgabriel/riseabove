# Implementation status

**What exists right now**, measured against `docs/design/LOCKED_DESIGN_DECISIONS.md` (the intent). This file is deliberately short on
prose: it says what is done, what is partial, what is missing, and where the evidence is. Code and tests decide; this file is
updated when they change.

* Branch `local/pathway-integration` (remote `integration/pathway-data-import`), audited 2026-09-30 (sections 1-6 rewritten after the second implementation pass).
* Labels: **IMPLEMENTED** · **PARTIAL** · **NOT IMPLEMENTED** · **NEEDS AUDIT**. "Tested" says whether a test enforces it.
* Older reports (`INTEGRATION_REPORT.md`, `FINAL_DEPTH_PASS.md`, `WORLD_SYSTEMS.md`) describe earlier commits and may be stale.

## 1. Knowledge, perception, decisions - PARTIAL (core implemented)

| Rule | Status | Evidence / gap | Tested |
| --- | --- | --- | --- |
| Actors read beliefs, not hidden truth (1.1) for team selection | **IMPLEMENTED** | `selection.rs`: ability, role fit, edge, leadership, personality read through the club's perception; risk and discipline from observable evidence | `manager_ai.rs` |
| Exposure changes certainty (1.9) | **IMPLEMENTED** | `knowledge.rs::sigma`, weekly observation in `perception.rs`; a dossier's band widens for players no one has seen | `dossier.rs` |
| Belief dossier, not noisy PA (1.5) | **IMPLEMENTED** | `pw-world/dossier.rs`, `pw-sim/dossier.rs`: current level, one-year projection, ceiling, direction, risks (unknown kept apart from absent), roles, evidence, per-evaluator opinions, revision history with reasons; `scouting::view` reads it | `pw-cli/tests/dossier.rs` |
| Several evaluators disagree (1.6), departments (1.8) | **IMPLEMENTED** | manager, assistant, coaches, head of youth, director, analyst and scouts each read the player with their own competence and exposure | `dossier.rs` |
| Manager weighs staff by trust and history (1.7) | **IMPLEMENTED** | `dossier::trust` (years together, inherited or hired, standing, track record, regard, ego); a year on, readings are checked against the market and move the evaluator's record | `dossier.rs` |
| Multi-domain judging (1.4); licences are competence, not truth (1.3) | **PARTIAL** | twelve `Domain`s from staff attributes; licences add to tactical and technical competence only, never to judging ability. Licences and schools do not yet feed manager priors | `dossier.rs` |
| Philosophy and context change decisions (1.11-1.12) | **IMPLEMENTED** | `dossier::worth` (youth trust, patience), manager style from traits, opposition, next fixture, promises | `dossier.rs`, `manager_ai.rs` |
| Players and agents act on beliefs (1.13) | **IMPLEMENTED** | `consider::self_view`, expected minutes from public standing, agents read the public view | `truth_guard.rs` |
| Decision modules do not read true ability (1.15) | **IMPLEMENTED** | guard covers selection, planning, deals, market, negotiation, contracts, board, staffing, managers, decisions, consider, mind, agents, youth, intl, social, talk; a ratchet fails on any unclassified module | `truth_guard.rs`, `qa_truth_scan.rs`, `qa_truth.rs` |

## 2. Media - PARTIAL (core implemented)

* Claim-time truth versus what audiences saw (2.3, 2.11); contextual credibility per club and topic (2.7); belief separate from sharing (2.10): **IMPLEMENTED** (`media_belief.rs`).
* Truth categories (accurate, accurate at the time, misleading, manipulated, false) kept apart from framing, the writer's intent and the source's aim (2.2, 2.4-2.6): **IMPLEMENTED** (`Story::truth/intent/aim`; `newsroom.rs`, `mediarel.rs`); spin and being fooled recorded on the journalist (`media_truth.rs`).
* Persistent directional relationships with causes, respect apart from warmth, grudges that outlast moods, access refused, tone bent (2.13-2.17, 2.20): **IMPLEMENTED** (`mediarel.rs`).
* Media-versus-media feuds that cool and flare, scoops, exposure, poaching sources (2.18-2.19): **IMPLEMENTED**.
* Gaps: prior belief about the subject in source inference (2.22), semantic history of corrections and denials beyond the existing story refs (2.23), organisations as several minds (2.12).

## 3. Transfers - PARTIAL (core implemented)

| Rule | Status | Evidence | Tested |
| --- | --- | --- | --- |
| Public value is an estimate, not the price (3.25); fair value and reservation from each side's own reading (3.8-3.10) | **IMPLEMENTED** | `market.rs` (`fair_value` shades by uncertainty), `deals.rs` | `manager_ai.rs` |
| Governance decides whose opinion counts (3.6); disagreement persists (3.7); causal memory (3.24) | **IMPLEMENTED** | `boardroom.rs`: seven voices with stances, power by club structure, authority earned and lost, case files | `boardroom.rs` |
| Missed risk versus accepted risk (3.23) | **IMPLEMENTED** | verdicts judged on process and outcome a season on | `boardroom.rs` |
| Dynamic club risk appetite (3.21-3.22) | **IMPLEMENTED** | `boardroom::appetite` with named drivers; individual tendencies | `boardroom.rs` |
| Neither side sees the other's limit (3.13); bluffing and signalling (3.14); rival-bid information with provenance (3.28); information shocks (3.27) | **IMPLEMENTED** | `bargaining.rs`: ranges, signals, honesty records, agent tips, need premium | `boardroom.rs` |
| Replacement chains (3.11), alternatives as leverage (3.12), knowing gambles (3.29) | **IMPLEMENTED** | `deals.rs::chain_wait`, replan after a departure, sellers hold out on young players they rate above the market | `boardroom.rs` |
| Sponsor and commercial pressure, senior players opposing, supporters (3.17-3.20) | **PARTIAL** | owner commercial appeal, captain and supporter voices; sponsor pressure is not a separate driver | |
| Player decisions consider the whole move (3.16) | **PARTIAL** | `move_utility`, package utility including relocation, family and language | |

## 4. Squad planning and adaptation - PARTIAL (core implemented)

* Position-level needs, projections, departures, homegrown gap (`planning.rs`): **IMPLEMENTED**. Planned versus opportunistic recruitment (4.10) and tactical fit before signing (4.11): **IMPLEMENTED** (`system_fit`, `notice_opportunity`).
* Adaptation in six channels with their own clocks (environment, routine, football, tactical, social, mental), distance from real differences and never from nationality names, experience and support, traits (4.12-4.18): **IMPLEMENTED** (`adaptation.rs`, `Nation::env`; environments are inferred and marked `known: false` until a source provides them).
* Time to usefulness in planning (4.19, 4.22), the manager's integration plan (4.20), early-use loops (4.21): **IMPLEMENTED**.
* Failed planning becomes history (4.23): **IMPLEMENTED** (`PlanFailed` events, scramble marks).
* Gaps: scenarios by horizon (4.3-4.4), player intentions in the plan (4.6), manager/director conflict over construction (4.7) beyond the boardroom stances.

## 5. Contracts - PARTIAL (core implemented)

* Packages shaped by club strategy (5.1-5.2, 5.7-5.9), player priorities (5.10), agent stakes (5.13), trade-offs between dimensions and refused clauses paid another way (5.4, 5.15), tax and cost of living (5.12), relocation demands (5.11): **IMPLEMENTED** (`package.rs`, `negotiation.rs`).
* Options, automatic extensions, bonuses (appearance, goal, assist, clean sheet, loyalty, title, promotion, continental, caps), relegation cut and relegation release, release-clause bids in force (5.3, 5.14): **IMPLEMENTED** (`clauses.rs`).
* Wage hierarchy ripples (5.6), promises remembered (5.5, 5.16), overcommitment and contract files judged on what was known (5.17-5.19): **IMPLEMENTED**.
* Gaps: image rights and other exotic clauses (5.3); sell-on and buy-back exist only club to club.

## 6. Social opinion - PARTIAL (core implemented)

* Eight opinion dimensions, events that move different ones, summaries that depend on the account (6.1-6.2, 6.7): **IMPLEMENTED** (`socialnet.rs::apply/impact_for/summarise`).
* Memories that return (6.3), belief coloured by trust (6.5, 6.12), expectation first-class (6.9), audiences (6.11), allegiance changes interpretation (6.6), opinions revised and called out (6.13): **IMPLEMENTED** (`social_opinion.rs`).
* Attention waves with causes, half-lives and reach (6.20-6.22, 6.28); fame apart from football reputation (6.18); bounded brand premium (6.19, 6.24); anti-hype (6.26-6.27); audience-specific looks (6.16-6.17, 6.36-6.38); nicknames with lives (6.34); folklore that grows in the telling (6.29-6.33): **IMPLEMENTED** (`attention.rs`, `pw-cli/tests/attention.rs`).
* Gaps: population aggregates beneath the representative accounts (6.14), players perceiving opinion imperfectly (6.15), jokes that become club culture (6.35), price and wages as public information (6.10), follower quality in transfers (6.23).

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
observer without the truth; **where every new or loaded world opens**) and `observer` = the **omniscient debug view** (`omniscient: true`),
reached only by naming it: `persp.observe {"omniscient": true}` (`{"public": false}` is the older spelling; `persp.observe {}` goes to the
public view). Every read of engine truth is gated on `Ctx::observer()`, which is true in the omniscient view alone; the only door to mutate
a world from outside is `Api::debug_mutate_world` (`pw-view/src/debug.rs`, doc-hidden, for tests).

| Rule | Status | Evidence |
| --- | --- | --- |
| Sort/filter/search cannot leak hidden truth (§8.6) | **IMPLEMENTED** | fixed: position sort used true CA as tie-break (`tables/players.rs`); `min_ca`/`expiring_days` filters and truth sorts are refused outside the omniscient view | `crates/pw-view/tests/firewall.rs` (`player_lists_are_not_ordered_by_true_ability`, `filters_and_sorts_by_hidden_truth_are_refused...`, `a_search_cannot_be_used...`) |
| Non-interference: hidden truth changes, nothing a public or inhabited viewer sees moves (§8.10) | **IMPLEMENTED** for what is reachable through the API | `audit_public_view`, `audit_inhabited_view`: ability, potential, others' personality, how everyone feels about the viewer, journalists' and referees' private numbers changed; ~640 lists (every table, every sortable column both ways, all columns) and pages compared | firewall.rs |
| Relationships as evidence and tone, not soul meters (§8.7) | **IMPLEMENTED** (`me.people`: label, tone, trust/respect words, evidence memories; no affinity number) | `relationship_pages_show_evidence_and_tone...` (also: how others feel about the viewer moves nothing he can read) |
| Uncertain knowledge in words, not false percentages (§8.5) | **IMPLEMENTED** for the viewer's own pages | rumours, told-by-others and grapevine tells carry `sureness`; a coach's selection outlook is words ("thinks you will probably start"), the medical team's certainty uses the sureness words; condition, sharpness, morale, confidence, wellbeing, legs, stress, sleep, fulfilment, bond, fluency and agent satisfaction are a `Band` (word, `step` of `steps`), never the 0-100 number; feelings carry a direction (`pull`), not a value | `firewall.rs` `the_viewers_own_state_is_in_words_and_a_forecast_is_not_a_percentage` (moving the number inside one word changes nothing) |
| Injury diagnosis private to the club (§8.10) | **IMPLEMENTED** in the player list | strangers see “Injured”, not the diagnosis or days (`Ctx::sees_medical`) |
| Debug omniscience separate and labelled (§8.8) | **IMPLEMENTED** | separate labelled view and a single gate; a world opens in the public view (new or loaded, whatever view it was saved from); the app's perspective chip shows Public view / Observer (debug) / the inhabited name and switches between public and observer; the source browser asks for the debug view by name | `firewall.rs` `omniscience_is_its_own_labelled_view...`, `contract.rs` `a_world_opens_in_the_public_view_and_omniscience_is_asked_for_by_name` |
| Reviewed list of files that read engine truth (§8.9) | **IMPLEMENTED** | `engine_truth_is_read_only_in_the_files_that_gate_it` fails when a new file reads true ability, personality, relationship internals, private books or medical state |
| Enforced by types (`VisiblePlayer`, ... §8.9) | **NOT IMPLEMENTED** | pages still build JSON from `&World`; the gate is `Ctx`, the guard is the test above |

Provenance on the person page is omniscient-view only.

## 9. Typed API contracts — IMPLEMENTED for responses (page payloads checked at run time), PARTIAL for query arguments

`crates/pw-view/src/contract.rs` is the contract layer. Tests: `crates/pw-view/tests/contract.rs` (12 tests).

| Rule | Status | Evidence |
| --- | --- | --- |
| Contract drift fails early (§9.1, 9.3) | **IMPLEMENTED** for the declared types | payload types are declared once with `contract!` (Rust struct + TypeScript interface); `contract::typescript()` renders them; `app/src/contract.generated.ts` must equal it (test fails otherwise; `UPDATE_CONTRACT=1 cargo test -p pw-view --test contract` regenerates); `app/src/types.ts` re-exports the shared wire types (Ref, Named, Cell, Col, Row, TableReq/Resp, Perspective, ErrorKind) from the generated file; `npx tsc --noEmit` clean |
| Typed payloads (§9.2) | **IMPLEMENTED** (every method), the page payloads checked at run time | typed through `contract!` (built from the declarations): `app.info`, `world.status`, `table.query`, `person.attributes`, `me.people`, `me.rumours`, `me.promises`, `me.journal`, `me.agent`, `me.story`, `news.feed`, `news.story`, the `insight.*` views, the pathway views and every command's reply. The other 36 methods (person, club, comp, nation, match, overview, search, the `me.*` pages, the inbox, the social feed, saves, datasets and the source-database browser) are declared in `crates/pw-view/src/contract_pages.rs` (`response!`, 210 types): written from the responses of both worlds over a playthrough and a sweep of every page, then checked against the app's own interfaces for where an object may be missing. Those pages still assemble JSON, so the declarations are enforced at run time: `contract_pages::check_response` walks a response against the generated TypeScript all the way down (a missing, renamed, mistyped or undeclared field anywhere is an error naming its path; generic aliases and intersections included) and the playthrough runs it on every response at every stop (India), `page_payloads_match_their_declarations_in_a_synthetic_world` on the synthetic world. It found two real faults in an older typed method (`person.attributes` left out declared fields) and three gaps in the first declarations (a concealed result has no score or outcome; the pulse links stories). `every_method_answers_with_a_declared_type` keeps every method typed. The app compiles against the generated types for the club, district, start-of-career, calendar and diagnostics pages; the other pages keep their own interfaces, which are narrower (string unions such as `"win" \| "loss"` the generated types write as `string`) |
| Structured errors (§9.5) | **IMPLEMENTED** | `ErrorKind` (NotFound, UnauthorizedPerspective, InvalidRequest, StateConflict, UnavailableInformation, SaveIncompatible, SimulationBusy, InternalError), `ApiError::kind()`, `ErrorBody {kind, code, message, retryable}` used by `pw-serve` (with HTTP statuses) and the Tauri command; the legacy `code()` (`bad_request`/`not_found`/`state`) is unchanged so nothing old breaks; `app/src/api.ts` `ApiError` carries `kind` and `retryable`. `UnavailableInformation` is raised for a match that was played and later compacted (an id never issued is `NotFound`; `Fixtures::issued`) and for a story that would spoil a result the viewer has not revealed | `contract.rs` `what_the_viewer_cannot_have_is_unavailable_not_missing` |
| Unknown / hidden / estimated / known distinct (§9.6) | **PARTIAL** | `Knowledge<T>` (exact, range, reported, unknown, hidden; no value for unknown or hidden), attribute rows use its tags, sureness in words; table cells carry the same tags (`Cell.k`: `CellKnow`), replacing the bare `u` flag and `range` (no table produces one today: a column the viewer cannot know is left out of that viewer's list). Untyped pages still use `null` for some unknowns |
| Queries separate from commands (§9.7) | **IMPLEMENTED** as a manifest | `contract::manifest()` lists every method as query or command; a test keeps it equal to the dispatcher in `lib.rs` (it caught the newsroom methods added by a merge), every argument-free query is shown not to change the revision or the clock and to repeat exactly, and a client-supplied wage/fee on a command changes nothing |
| View types, not world types (§9.4) | **PARTIAL** | the typed payloads are view types; the page payloads are assembled as JSON from `Ctx` (see §8) and checked against their declarations at run time |
| Contract tests cover major surfaces (§9.9) | **IMPLEMENTED** | every method's response, field by field and all the way down, in both kinds of world; the manifest, errors, perspective and request validation | `crates/pw-view/tests/contract.rs` (14 tests), the playthrough |
| Request validation (§9.9) | **IMPLEMENTED** for commands | every command with arguments reads a declared request type that refuses fields it does not name (`request!`/`request_enum!`, `contract::request` → `InvalidRequest` naming the field): `me.act` is `ActReq` (28 actions, each with exactly its choices), and `SettingsReq`, `ObserveReq`, `InhabitReq`, `SaveReq`, `LoadReq`, `FileReq`, `AnswerReq`, `ReplyReq`, `IdReq`, `GoalReq`, `GoalDoneReq`, `NoteReq`, `IndexReq`, `PlanReq`, `FollowReq`, `RevealReq`, `RouteReq`, `CreatePersonReq`; the app's `queueAction` is typed by `ActReq`, so an unknown action or field fails `tsc`. Query arguments are still read by hand | `contract.rs` `commands_are_read_through_declared_requests_that_refuse_what_they_do_not_name` |

## 10. Saves — PARTIAL (framework implemented, one real layout migration)

Current schema is **5** and `OLDEST_SUPPORTED` is 5: schemas 1-3 were development formats and 4 was a one-day development format (the
world model changed between them without steps), so they are refused with a plain message, untouched (tested: `fixture.rs`, unit tests in
`save.rs`). From 5 on every serialised change bumps the number and registers a `Step`.
**IMPLEMENTED**: explicit schema version, sequential migration steps (`pw-sim::save::Step`), backup before upgrade, atomic write,
checksum, metadata, clear too-new / unsupported errors, compatibility in the listing, `stable_seed`, post-load validation (`validate.rs`).
**Extension envelope** (`pw-world/src/ext.rs`): `World::ext` is written as `(EXT_VERSION, bytes)` with its own isolated steps, so a new
domain never needs a schema step. Current layout **3**: layout 2 added `scenario` (tuning, calendar, markets, club data origin), `recog`
(organisation knowledge, recommendations, referral records, market regard, watches) and `pathway` (why each step, how each player was
created); layout 3 adds `known_derbies` and `reference` at the end of `Scenario` (named derbies, and the report of reading the reference
data). `Scenario` sits in the middle of `Extensions`, so the 2 to 3 step decodes the whole layout-2 state with a frozen `ScenarioV2`,
adds the two fields empty and re-encodes (`ext.rs::v2_to_v3`); the 1 to 2 step appends the frozen layout-2 scenario too, so a layout-1
save chains through both. Rules are at the top of `ext.rs`: append fields, register a step, never invent history. Anything that needs the rest of the world
(a present baseline) is `pw-sim/src/legacy.rs::finish`, run once after load when the envelope reports an older layout: deterministic,
derived from existing state only, marked legacy wherever provenance exists (a sponsor count becomes a `Legacy` recommendation, the old
export number becomes `legacy` regard, an old story becomes an `Unrecorded` creation record with unknown age and institution).
Tested: `ext.rs` unit tests (a real layout-1 byte stream opens at the current layout, a real layout-2 byte stream upgrades to layout 3 with
the new fields empty and everything else intact, a truncated one is refused, refuses newer or missing, unbroken step chain),
`india_ecosystem.rs::an_older_layout_gets_a_deterministic_present_baseline_and_no_invented_history`, and the golden fixture
(`golden_micro.pws`, schema 5, ext layout 1) which now passes through the 1 to 2 and 2 to 3 steps on every run (not regenerated).
**Trap found the hard way**: `World::data` (the whole `DataPack`, including `RuleProfile`) is part of the positional save, so adding a field to a pack
struct (not only to `World`) changes the bytes and breaks every save. New per-scenario rules go in `Scenario` (versioned ext state), as national-side
eligibility does (`Scenario::national`); the golden fixture test is what catches a slip.
**Missing**: save size still grows without bound over long runs (tiny world ~40% a year; retention checkpoint `60cc389` is unverified
against a 3-year archive); a census check that no id moves *during simulation* is not possible (people who leave the game are removed).

## 11. Imported data — IMPLEMENTED (with stated limits)

Pipeline parse → validate → resolve → assemble (`pw-import`); provenance per person and fact group, source ids searchable and
saved; unresolved rows kept with reasons; ability from combined evidence sampled from a posterior, never from price alone;
population calibration and determinism tested (`real_archive.rs` for the local archive, `archive.rs` for fixtures).
Limits: the club-standing prior is still derived from squad value plus last league position (correlated evidence, §11.14);
single source per fact so no cross-source disagreement resolution (§11.16); the final real-archive first-month price check
has median new/recorded price 1.20 (p10 0.30, p90 4.58) — economy calibration is still open; personality is generated, never inferred (§11.10).

Local database follow-up (`pw-import::database`, `pw-view::database`, `app/src/pages/Database.tsx`): all 13 archive CSVs and
48 FM23 exports are accessible through a paged observer source browser, connected automatically after archive import or manually.
External lazy indexes support exact ID relationships, name search and world-profile links without putting raw source rows in saves.
Missing values use dated valuation and lineup evidence where available; unsupported history stays unknown and literal zero remains
a recorded value. The real archive passes all three structural/calibration/first-month tests. FM23 binary player, contract and
club-league schemas remain unresolved; FM name exports are source records, not a playable imported FM world.
Coverage, estimation rules, measured index costs and verification are in [DATABASE_INTEGRATION.md](DATABASE_INTEGRATION.md).

India reference data loader (`pw-import::india_ref`): reads the 57 TOML files of `data/worlds/india/**` (embedded at compile time; `load_dir`
reads a folder) into typed rows with provenance (states, clubs, stadiums, competitions, memberships, rivalries, derby-name aliases) and
validates every table (provenance present and consistent, duplicate and dangling ids, malformed or unknown records, stadium capacity
without a date). Bad records are reported as findings and not loaded; absent fields stay `None`; fields no row reads are counted
(`unread_fields`), and the tables with no typed row (media, academies, universities, schools, rules, national teams, languages ...) are only
validated and counted (`unread_tables`). `india::build` uses it in three ways only: a pack club with an exact name and state match in the
reference is labelled by the record (`Imported` only for a verified or imported record graded A or B that names a source; everything else
inferred, seed or unknown is a `ScenarioSeed`; builder-made is `Generated`); a place the builder would fill with a made-up club goes to a real
club the reference lists in that tier's competition or the state's top league for the start season (strength and everything else is the
place's); a number (stadium capacity with its ground name, founding year, city) replaces the made-up one only when `Prov::allows_value`
(sourced fact, or an inference graded C or better with no recorded conflict, never a seed or unknown). Real derbies become
`Scenario::known_derbies` (labels); no rivalry is created or given intensity by the loader. Tested in `india_ref.rs` and
`india_ecosystem.rs::reference`.

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

## 14. India ecosystem: recognition, pathway, export — PARTIAL (core implemented)

Implemented in `pw-sim/src/{recognition,export,ecosystem,statepath,university,youth,legacy}.rs`, world types in
`pw-world/src/{scenario,recog,pathway,eligibility}.rs`, tuning in `data/worlds/india/pack.toml`. Tested in `crates/pw-cli/tests/india_ecosystem.rs`
(28 tests, 8 of them in `mod reference` for the reference data) plus the older `recognition.rs`.

* **Tuning is data** (IMPLEMENTED, tested): tier weights, sample sizes, attention, academy need, gates, vouch trust, scouting reach,
  camp sizes, foreign parameters are `Scenario::{recognition,scouting}`, read from the pack's `[recognition]` and `[scouting]`. The defaults
  are the numbers the code always used. They are initial values, not football truths.
* **Calendar is data** (IMPLEMENTED, tested): `[[calendar]]` (district selection, school and university scouting, camps, state
  championship day, university review). `ecosystem::monthly` and `statepath::daily` ask `Scenario::due*`, never the month.
* **Organisation-specific knowledge** (IMPLEMENTED, tested): `Recog::acquaint[(Org, player)]`, one entry per club, institution, state panel,
  federation or foreign market that has actually looked. `recognised_by(club, p)` reads that club's own looks, not a global count.
  State selectors pick from their own league, their district and championship acquaintances, and a small chance of report.
* **Causal recommendations** (IMPLEMENTED, tested): yearly, a coach (school, local club or district selectors) recommends a child who is in
  the top tenth of the group they coach and has real evidence. It records the cause (`Trained{months}` / `Watched{games}`), strength (rank
  in the group), credibility (coaching quality, record). Each organisation decides its own trust (`vouch_weight`), moved by referral outcomes
  recorded at trial decisions and when university places end. No random roll.
* **Physical maturity is not quality** (IMPLEMENTED, not separately tested): attention and scout readings of young players lean on build
  (`scouting::judge`, `recognition::aspects`), fading to nothing by nineteen, scaled by the scout's physical bias.
* **Talk only sends people to look** (IMPLEMENTED, tested): a spectacular game can start `Recog::watching`; each game credited is one game
  watched; only when they are done does a scout form a judgement, from the play. Buzz never enters the judgement.
* **Export regard is contextual** (IMPLEMENTED, tested): `Recog::export[(market, segment)]` for youth, senior, league and university
  football, per market (`[[market]]`); nations in no market never look; seeds are labelled, earned regard follows exports and successes.
* **Eligibility as data** (IMPLEMENTED, tested): `Judgement{body, rule, evidence, outcome, reason}` for state and national sides.
* **Pathway remembers why** (IMPLEMENTED, tested): `PathwayExt::why` records the reason at every `note`/`note_why` call site, transfers and
  promotions; `created` records how every ecosystem player came to exist (age, region, provider, first environment, first finder, why drawn).
  Steps and players from an older save say `Unrecorded`, never a guess.
* **Records** (IMPLEMENTED, tested): every stat has a provenance; top speed and distance are estimates and are never announced.
* **Club data origin** (IMPLEMENTED, tested): `Scenario::club_origin` (Imported / ScenarioSeed / Generated). `Imported` is now used: clubs built
  from a verified, sourced reference record (West Bengal's Calcutta Premier Division lineup) are Imported; the rest of the named clubs are seeds.
  `Scenario::reference` keeps the loader's report (records by status, findings, matches), and the Development > Scenario page shows it.
  The simulation's own `prepare` still generates a labelled (Generated) thirty-year past and seeds generic derbies for clubs sharing a city;
  the loader adds neither.
* **Rivalries** start empty and grow from state championship meetings, weighted by neighbourliness (IMPLEMENTED, tested). Real derbies from
  the reference data (`culture/rivalries.toml`, `derby_name` aliases) are only `Scenario::known_derbies`: names for the news and UI, no intensity.
* **Region output** is measured by quality (top tier, internationals, senior appearances, value): `ecosystem::region_output` (tested).
* **UI** (IMPLEMENTED, typed contract, contract test, reviewed in a browser): `Development` page (regions, abroad, scenario with the reference data's
  associations, press, broadcasters, programmes, partnerships, coaching ladder, representative sides of the states in the world, rules and languages) and a
  pathway panel on player pages (`pathway.player`, `ecosystem.regions|export|scenario`); recognition internals are omniscient-view only. `app/e2e/india.mjs`
  (all 58 pages of a lived-as India career in Chromium, no console or API error, no broken value) passes; the review fixed unstyled tables, empty columns,
  maintainers' notes and duplicated sides on the scenario page, foreign nations named by their codes ("The KOR Sun", "ESP Real Oeste"), a regional paper
  preferring another state's clubs and a private story whose body repeated its headline. In this container Chromium has no Indic font, so the native-script
  words draw as boxes there; the page names system Indic fonts ("Nirmala UI", "Noto Sans") for real machines.
* **Universities compete** (IMPLEMENTED, tested): recruiting runs in up to three rounds (a place turned down goes to the next name on the list), a player
  holding several offers hears counter-offers (a programme raises its scholarship once, within its resources, for a player it rates highly), and a
  contested recruit is public news (`RecruitWon`). Two Septembers of a tiny India world: 27 contested recruits, 23 settled in a later round, 1 by a raised offer.
* **Reference data read into the world** (IMPLEMENTED, tested: `reference::the_reference_gives_the_world_its_names_press_and_institutions_and_nothing_else`):
  `pw-import/src/india_ref.rs` types every table it reads (23 of them; an untyped table is reported, not skipped), and `india_lore.rs` turns
  the records into `World::ext.lore` (`pw-world/src/lore.rs`, ext layout 4). Each entry keeps its record id and standing (Imported /
  ScenarioSeed). What it gives the world is names and words, never strengths or results:
  - state associations name each state's football body; the pyramid and the state premier leagues take their real competition names;
  - real schools, sports hostels, SAI centres and universities become the world's institutions (resources from their kind, not their record);
    clubs' academies and nicknames, supporters' groups and native-script names are attached to the clubs;
  - newsrooms are real outlets (national, state and club media chosen by football emphasis and reach), staffed by the world's journalists;
    broadcasters holding rights in the starting season are outlets too;
  - programmes, partnerships (world clubs and the federation), men's representative sides, the coaching ladder, the referees' ladder,
    rules in words and football terms in each language are recorded for the views and the text.
  The Development > Scenario page lists them with their standing; a club page shows what the club is known as, its academy, partners and media.
  Supporters and local papers call a club by its nickname (`pw-lang` render, tested in `pw-lang/tests/nickname.rs`).
* **Not done**: women's football (deliberately later); the referee pathway (the reference's referee grades are empty); districts and grassroots
  programmes are listed, not simulated.
* **Balance on the India world** (MEASURED, 6 years, tiny scale, seeds 1-2; `pathway-sim balance india-tiny`): mean ability had risen
  11-14 points in six years. Two causes, both fixed. Academy intakes in a world of regions drew potential from the European formula (mean ~106
  against India's ~70): an academy now recruits from its region's pool (`ecosystem::pool_talent`) with a selection margin, and no minors
  come from abroad. Children grew at a youth rate from eight: growth now brakes near an age ceiling (`generate::maturity_ceiling`). Drift
  is now +5 to +6 (a warning, from more and better-selected youngsters at the professional clubs). Clubs in debt fell from 23 to 11 and the
  median wage bill from 0.87 to 0.67 of revenue (see section 16). Not yet repeated on the full-scale India world in this container.

## 15. Language engine (pw-lang) in the news, inbox and social text — IMPLEMENTED for the news and the inbox, PARTIAL for social

`crates/pw-lang` (data-driven: events, certainty, lexicon, channel grammars, voices, lint and fuzz over every event x channel x voice x knowledge state) is
wired into the simulation **in every world** through `pw-narrate/src/lang.rs` (`docs/LANGUAGE.md` has the table of what maps to what). Money takes the
world's form (`pw_lang::Currency`: the Indian system in a world of Indian regions, `£` and short units elsewhere).
* **News**: in a 500-day India world all but 87 stories are written by the engine: transfers, rumours at the stage reached, injuries, manager changes,
  match reports and the morning-after analysis (naming the derby where the reference names one), interviews, milestones and records, unhappy and praise
  stories, features and data pieces (`player.reading`, the media's labels), incidents made public (18 kinds), pressure on a manager (as speculation),
  discipline, criticism and supporters' reactions. Left with the older text: season wrap-ups and manager changes (they rest on the event log, which
  forgets), denials and private matters (family, pregnancy, relationships; their body now says where it came from instead of repeating the headline).
* **Inbox**: trial invitations, talks about a move and university scholarships are worded by the engine, and so are their options: each engine option is
  tied to one of the decision's own choices by its effect (`lang::effect_choice`) and shows what choosing it does; the answer is applied by the decision as
  before. A scholarship offered to the person someone lives as is their decision (`DecisionKind::Scholarship`), not the AI's.
* **Social**: a post that relays a covered story is written by the engine in the account's voice; supporters and local papers call clubs by their
  nicknames; a local supporter celebrates in the language of the club's state (the reference terms, in their own script). Opinion, banter, chants and
  memes stay with the personality-driven text.
* **Tested** (`crates/pw-cli/tests/lang_bridge.rs`, 18 tests; `pw-lang/tests/*`): clean text, no firmer than the story, rumours never read as bids,
  nothing from the club's own business in public stories, old stories unchanged as the world ages, same story same words, the inbox's options are the
  decision's own choices, money in the world's form, the new story kinds written in a running world, nicknames, local words and derby names.
* **Not done**: a second language for the text itself (the reference words appear inside English text); engine effects not tied to a decision
  (`transfer.open_bid`, `transfer.drop_target`, `medical.request_report`) are not offered.

## 16. Economy calibration — PARTIAL (four causes of drift fixed; late-run fee prices and imported youth sides open)

Measured with `pathway-sim balance` (release) on the synthetic small world (64 clubs, two tiers), seeds 1-3, eight to ten years, and the tiny
world, seed 1, ten years; the real archive is not in this container, so its numbers below are the earlier report's.

| Finding | Cause | State | Evidence |
| --- | --- | --- | --- |
| Club balances grew ~45%/yr in the archive, x3 in the small world's first years | Placeholder cash was endowed at 0.45 of revenue; the reserve rule kept 0.75 of revenue plus a surplus drained at ~45%/yr | **FIXED**: clubs start at the reserve (`reserve_years`), owners take their share of the excess quarterly | small: median balance year 0 -> 3 x1.5 (was x3.4); `economy_long.rs` bound (within x2 over three years) |
| Imported first-team wages 22k -> 44k -> 56k -> 68k | Builders priced every generated wage and salary before the economy existed: revenue had no broadcast share | **FIXED**: `economy::ensure_pools` at the end of `builder::finalize` | `archive.rs` `wages_the_archive_lacks_are_set_at_the_level_the_running_world_pays` (0.60 of the running level before, now within 0.8-1.25); small: mean first-team wage 187k -> 235-259k over eight years (3.5-4%/yr) |
| Prime-age professionals released into the amateur game, first teams years younger | Graduates stepped up while the first team was under its hard limit; the monthly trim then shed senior players | **FIXED**: graduates step up best-first into the planned room (`first_team_target`) | tier-1 first-team age 24.3 -> 25.3-25.6 in year 6 (was falling from 28) |
| Median transfer fee +27-35%/yr (small world, `balance` flags a PROBLEM) | Mostly not the price: the mix changes (early deals are cheap youth sales from big clubs to small ones, later ones top-flight moves) | **MEASURED, partly open**: `balance` now reports fee over the public value (`fee/val`) and flags a problem only when that rises; a rising median at steady prices is a warning. Small world, seed 1: fee/value 0.70 for four years, then 1.03-1.10 in years 5-6 (warning: +12%/yr) | `metrics::tests::dearer_players_moving_is_not_price_inflation_but_paying_more_for_the_same_player_is` |
| Clubs over their wage budget kept signing and renewing; India: clubs in debt 0 -> 23 in six years (tiny), 128-182 (full), wage bill 0.87 of revenue | Only the transfer search read the wage budget; renewals, first professional deals, trials, enquiries and pre-contracts did not, and talks stretched past it. The going rate priced the first team at the whole budget, leaving nothing for staff, reserves and scholars | **FIXED**: one check (`market::wage_fits`) on every route that adds a wage; key players are renewed over budget, and anyone while the first team is at a matchday squad; the going rate prices the first team at 85% of the budget (`FIRST_TEAM_PART`) | India tiny: clubs in debt 11, wage/revenue 0.67; small: wage/revenue 0.60, first-team squads steady at 25 |
| Archive: active players +19%/yr, state growth ~31%/yr | Imported worlds carry no youth sides, which fill over the first seasons; the amateur pool grows until its age structure fills | **OPEN** (warm-up, not measured here) | `LOCAL_INTEGRATION_REPORT.md` |

A reloaded world diverged from an uninterrupted one after the change (`qa_retention`): `growth::drift` pushed `CharacterChanged` events in
hash-map order, which differs after a reload. Fixed by visiting players in id order, as `stagnation` already did.
