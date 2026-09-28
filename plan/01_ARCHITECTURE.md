# 01 — Architecture

## 1. Technology stack

| Layer | Choice | Reason |
|-------|--------|--------|
| World simulation | Rust, fork of open-football (Apache-2) | Native headless daily tick, rayon parallelism, FM-style skill model |
| Career layer | Rust crate `pathway_career` | Same process, zero-copy access to perception views |
| Persistence | SQLite (WAL) for queryable history + zstd-compressed binary snapshots for hot state | History must be queryable for UI; hot state must load fast |
| Client | Tauri 2 + React + TypeScript | Desktop-native, small footprint, OFM precedent |
| Charts | ECharts or Visx | Heatmaps, radar, time series |
| Scripting/mod hooks | Data packs (TOML/JSON) + optional Rhai scripts for rule edge cases | Moddability without recompiling |
| Tooling | `pathway-sim` headless CLI, `pathway-lab` calibration notebooks (Python, read-only on exported Parquet) | Balance work outside the game |

## 2. Crate layout

```
pathway/
├── crates/
│   ├── world_core/        # forked open-football core (entities, tick)
│   ├── world_rules/       # data-driven rule engine (registration, eligibility, windows, discipline)
│   ├── world_market/      # transfers, loans, contracts, agents (extends open-football pipeline)
│   ├── world_match/       # event-chain match engine (see 06)
│   ├── world_health/      # load, fatigue, injury, rehab (see 05)
│   ├── world_dev/         # development & training (see 04)
│   ├── world_media/       # media, fans, reputation, awards (see 11)
│   ├── world_history/     # archival, records, milestones
│   ├── world_perception/  # observers, estimates, knowledge decay
│   ├── decision_port/     # Mind trait, DecisionRequest/Response types
│   ├── career_core/       # protagonist systems (09, 10)
│   ├── persistence/       # SQLite schema, snapshots, migrations
│   ├── viewmodels/        # read-only projections for UI (perceived only)
│   └── sim_cli/           # headless runner, benchmarks, exports
├── data/                  # data packs (rules, nations, names, weights)
├── client/                # Tauri + React app
└── lab/                   # calibration notebooks, reference distributions
```

## 3. Entity model (core types)

```
World
 ├── Calendar (date, season map per competition, international windows)
 ├── Nation[]      (id, confederation, rules_profile, economy, climate, language[], football_culture)
 ├── Region[]      (sub-national; talent density, travel distance)
 ├── Competition[] (league/cup/continental/international; format; rules_ref; stages)
 ├── Club[]        (board, finances, facilities, stadium, reputation, philosophy, teams[])
 │    └── Team[]   (first team, B/reserve, U23/U21, U19/U18, U16...)
 ├── Person[]      (shared core: identity, nationality[], languages, personality, mind)
 │    ├── PlayerRole  (attributes, positions, contract, health, development state)
 │    ├── StaffRole   (coaching/medical/scouting/management attributes, contract)
 │    ├── AgentRole   (agency, network, clients)
 │    └── (a Person can hold several roles over life: player → coach → manager)
 ├── Agency[]      (agents, reach, reputation)
 ├── Media[]       (outlets with reach, bias, tone)
 ├── Contract[]    (party A/B, type, terms, clauses, registrations)
 ├── Relationship[] (person↔person, person↔club, person↔fanbase; typed, weighted, with memory)
 ├── Perception[]  (observer → subject → estimate + confidence + timestamp)
 └── History       (append-only archive)
```

Key rule: **a Person is permanent**. Players retire into coaches, scouts, pundits, agents or civilians. The same ID persists for history, relationships and "former teammate" links.

## 4. Time model

- **World tick = 1 day.** Every system registers a phase in the daily pipeline.
- **Protagonist day phases:** Morning, Afternoon, Evening (and Night = sleep/recovery). Only the protagonist's schedule is resolved at phase granularity; AI players get the same phases resolved statistically by the same model (fairness: identical inputs → identical distributions).
- **Match windows:** matches are resolved within the day in kick-off order (time zones respected for broadcast/news ordering, not for outcomes).

### Daily pipeline (ordered)

| Order | Phase | Systems |
|-------|-------|---------|
| 1 | Calendar | Season rollovers, window open/close, registration deadlines, birthdays, contract expiries at midnight |
| 2 | Governance | Rule changes effective today, sanctions, licensing checks |
| 3 | Club management (morning) | Board meetings, budget updates, staff hiring/firing, manager job market |
| 4 | Training | Team sessions, individual plans, load accounting |
| 5 | Health | Injury progression, rehab, illness, fitness update |
| 6 | Market | Scouting reports, shortlists, bids, negotiations, agent actions, loan recalls |
| 7 | Decisions | DecisionPort queue: AI minds resolve instantly; HumanMind requests surface to UI (with deadline + default) |
| 8 | Matches | Selection → match engine → post-match (ratings, injuries, discipline, fitness) |
| 9 | Aftermath | Morale, relationships, media, fan sentiment, perception updates, reputation |
| 10 | Life (evening) | Career core: life activities, finances, family, education (AI players: statistical life model) |
| 11 | Night | Sleep/recovery, stress decay, mood update |
| 12 | Archive | History writes, milestones, records, telemetry counters |

### Advance modes (UI)

| Mode | Behaviour |
|------|-----------|
| Next phase | Protagonist-level granular play |
| Next day | Default |
| Until event | Runs until anything needs the player (decision request, match, message flagged important) |
| Until date | Holidays / injuries / off-season |
| Autopilot period | Delegates decisions to the protagonist's own AI mind with user-set policies (e.g. "accept renewals ≥ current wage, refuse loans abroad") |

## 5. The Mind / DecisionPort contract

```rust
trait Mind {
    fn decide(&mut self, req: &DecisionRequest, ctx: &PerceivedContext) -> DecisionResponse;
}

enum DecisionRequest {
    ContractOffer { offer, deadline },
    TransferInterest { club, stage },            // "do you want to talk to X?"
    LoanProposal { terms },
    TrainingFocus { options },
    AgentChoice { candidates },
    MediaQuestion { question, tone_options },
    CaptaincyOffer, InternationalCallUp, TrialInvite,
    RetirementPrompt, LifeEvent { kind, options }, ... // full catalogue in 09/10
}
```

- AI persons: `AiMind` uses personality, goals, perceived context and seeded noise.
- Protagonist: `HumanMind` queues the request to UI. Every request has a **deadline** and a **default** (the protagonist's own AiMind answer, based on the protagonist's personality). If the user doesn't answer by deadline, the default applies. This keeps the world flowing and prevents the protagonist from "stalling" negotiations in ways AI players can't.
- The `PerceivedContext` given to both minds is built by the same perception system (P3).

## 6. Event bus

- Typed, append-only, ordered by `(date, phase, sequence)`.
- Every event carries: `event_id`, `date`, `phase`, `actors[]`, `subject`, `payload`, `visibility` (public / club-internal / private to person / secret), `seed_path`.
- Consumers: history, media, perception, career core, UI inbox, telemetry.
- Visibility enforces P3: the protagonist inbox only receives events whose visibility includes him (e.g. a club-internal shortlist is invisible unless leaked through media or agent).

## 7. Determinism

- RNG: counter-based (e.g. Philox / ChaCha8) keyed by `hash(world_seed, system_id, entity_id, date, sequence)`.
- No global RNG; no iteration-order dependence (sort entities by ID before any seeded loop; rayon work split by deterministic chunks and merged in ID order).
- Human decisions are recorded in the save as a `DecisionLog`. Replaying the log from the initial seed reproduces the world exactly.
- Floating point: use fixed-point or deterministic float ops for anything that affects outcomes (no platform-dependent math intrinsics in outcome paths).

## 8. Persistence

| Store | Contents | Format |
|-------|----------|--------|
| Hot snapshot | Current world state | bincode + zstd, one file per save slot, versioned |
| History DB | Matches, player seasons, transfers, honours, records, events flagged archival | SQLite |
| Decision log | Protagonist decisions | append-only in save |
| Media archive | Articles (template id + params, not rendered text) | SQLite |

- **Compaction:** after N seasons (default 5), match-level detail for matches not involving the protagonist's clubs/national team is compacted into season aggregates. Protagonist's matches keep full event logs forever.
- **Migrations:** every save carries schema version; migrations are forward-only and tested on golden saves.
- **Autosave:** configurable (daily / weekly / before decisions); rotating 3 slots; crash-safe (write temp → fsync → rename).

## 9. Performance budget (reference PC: 6-core, 16 GB)

| Scenario | Target |
|----------|--------|
| World size "Large" | ~60 nations active, 180 leagues, 3,500 clubs, 110k players, 25k staff |
| Non-match day advance | < 0.6 s |
| Weekend with ~1,800 matches | < 6 s |
| Full season headless | < 3 min |
| 20-season autonomy run | < 60 min headless |
| Save/load | < 4 s / < 3 s |
| Memory | < 3 GB at Large |

Techniques: struct-of-arrays for hot player fields; rayon over clubs; match engine allocation-free per match; lazy history queries; background compaction thread.

Level-of-detail rule (fairness): all matches use the same outcome model. LOD only controls **what is recorded** (full event log vs aggregates). Leagues can be set "inactive" (world-gen option) in which case they use a documented statistical engine for the *entire* league (all its clubs equally), never selectively.

## 10. Interfaces to the client

- Client never touches entities. It calls `viewmodels` commands (Tauri IPC), which return serialisable, perceived-only DTOs.
- Commands are split into **queries** (read) and **intents** (protagonist decisions, UI preferences). Intents go through DecisionPort or career-core actions only.
- Streaming: live match events streamed over an IPC channel with backpressure; the engine pre-computes the match and the UI "plays" it at chosen speed (no mid-match state is exposed that the viewer couldn't see).

## 11. Observability

- Structured logs (`tracing`), per-system timers, counters (transfers/day, injuries/week, goals/match).
- `sim_cli export --parquet` for calibration notebooks.
- Debug overlay (dev builds only) that shows *true* values next to perceived values — compiled out of release builds.

## 12. Security & integrity

- Saves are local files; no online component in v1.
- Mod scripts sandboxed (Rhai with op limits, no filesystem/network).
- Editor mode flags saves as "edited" (shown in legacy screen), never silently.
