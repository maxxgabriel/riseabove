# PATHWAY — Master Design & Engineering Plan

Codename: **Pathway**. A deep, autonomous football-world simulator in which the human controls exactly one footballer, from academy trialist to retirement and beyond, while every club, manager, agent, board, scout and player in the world lives its own life.

Status: DESIGN (no implementation started). Foundation decision: see `docs/foundation/RECOMMENDED_FOUNDATION.md` (open-football as the FootballWorld engine, with a new PlayerCareerCore layer built on top).

---

## 1. Vision in one paragraph

You are one player in a living world. You don't pick the team. The manager does, and he has a philosophy, a job to keep, favourites, and a memory of how you trained on Tuesday. Clubs scout you only if someone actually watched you. Your agent may be brilliant or lazy. Your partner may not want to move to another country. Your knee remembers the injury you rushed back from. In fifteen seasons the league looks different: clubs rise, managers move on, your youth teammates are coaches, and your name is either in the record books or forgotten. None of it is scripted for you, and none of it bends in your favour.

## 2. Design pillars (non-negotiable)

| # | Pillar | What it means in practice | How we verify it |
|---|--------|---------------------------|------------------|
| P1 | **One ordinary player** | The protagonist is a plain `Player` entity. No system reads a "this is the human" flag to change an outcome. The only difference is *who makes his decisions*. | Fairness harness (14_QA): replace the human with an AI mind; outcome distributions must match within tolerance. |
| P2 | **The world runs itself** | 20+ seasons with zero human input produce believable squads, markets, careers, dynasties and declines. | Autonomy benchmark suite (14_QA), run nightly. |
| P3 | **You only know what your player would know** | Attributes, club intentions, market interest and opponent quality come as perceptions with uncertainty, not truth. | UI data layer only reads from `Perception` views, never raw entities (lint + test). |
| P4 | **Depth over spectacle** | No 3D, no ball physics. Match simulation is an event model that produces credible football data and a readable story. | Match calibration vs real-world statistical distributions. |
| P5 | **Consequences persist** | Injuries, relationships, reputations, promises and history have memory. | Save/replay tests; long-memory systems covered by edge-case catalogue. |
| P6 | **Deterministic and replayable** | Same seed + same decisions = same world. | Golden-seed regression tests. |
| P7 | **Moddable and data-driven** | Rules, competitions, nations, attributes weights, calendars, and names are data, not code. | All rule tables loaded from versioned data packs. |
| P8 | **Respectful life simulation** | Off-pitch life is rich but grounded and non-exploitative: no gambling mechanics, no pay-to-win, sensitive topics handled with care. | Content guidelines (10_LIFE_SIM §12). |

## 3. How to read this plan

| File | Topic |
|------|-------|
| `01_ARCHITECTURE.md` | Engine architecture, time model, event bus, determinism, persistence, performance budget |
| `02_WORLD.md` | Nations, leagues, competitions, rules engine, calendars, registration, stadiums, climate |
| `03_PLAYER_MODEL.md` | Every player attribute, hidden traits, personality, body model, positions, CA/PA, perception |
| `04_DEVELOPMENT.md` | Growth curves, training, coaching, match experience, mentoring, regression, youth stages |
| `05_HEALTH_FITNESS_INJURY.md` | Load, fatigue, fitness, injury model, medical staff, rehab, long-term body wear |
| `06_MATCH_ENGINE.md` | Event-chain match engine, tactics, ratings, stats, referees, output contract |
| `07_CLUB_AI.md` | Managers, selection, tactics, squad planning, staff, boards, finances, national-team AI |
| `08_MARKET.md` | Transfers, loans, contracts, clauses, agents, valuation, negotiation, regulation |
| `09_CAREER_CORE.md` | The protagonist's football career: creation, stages, playing time, dressing room, milestones |
| `10_LIFE_SIM.md` | Off-pitch life: time budget, education, family, relationships, finances, housing, health, identity |
| `11_MEDIA_REPUTATION_HISTORY.md` | Media, fans, social media, reputation layers, awards, records, history, legacy |
| `12_UI_UX.md` | Screens, flows, information design, live match view, notifications, accessibility |
| `13_DATA_AND_WORLDGEN.md` | Database, procedural world generation, editor, data packs, licensing |
| `14_QA_BALANCE.md` | Calibration, fairness harness, autonomy benchmarks, soak tests, telemetry |
| `15_ROADMAP.md` | Milestones, vertical slices, acceptance criteria, risks, staffing |
| `16_EDGE_CASES.md` | Catalogue of edge cases every system must handle |
| `17_FORMULAS_APPENDIX.md` | Reference formulas, constants, tuning tables |

## 4. Top-level system map

```
                         ┌──────────────────────────────────────────────┐
                         │                 FootballWorld                 │
                         │  (open-football core + extensions, Apache-2)  │
                         │                                              │
  Data packs ──────────► │ Nations · Leagues · Competitions · Rules      │
  (rules, names,         │ Clubs · Boards · Finances · Facilities        │
   calendars, weights)   │ People: Players · Managers · Staff · Agents   │
                         │ Market: Transfers · Loans · Contracts         │
                         │ Development · Health · Injuries               │
                         │ Match Engine (event-chain)                    │
                         │ Media · Reputation · History                  │
                         └───────────────┬──────────────────────────────┘
                                         │  WorldEvent stream (typed, ordered, seeded)
                                         │  Perception queries (fogged views)
                                         │  DecisionPort (player decisions in/out)
                         ┌───────────────▼──────────────────────────────┐
                         │               PlayerCareerCore                │
                         │ Protagonist mind (HumanMind) · Life sim       │
                         │ Education · Family · Relationships · Agent UI │
                         │ Personal finances · Housing · Health & mind   │
                         │ Media persona · Personal history · Goals      │
                         └───────────────┬──────────────────────────────┘
                                         │  ViewModels (read-only, perceived)
                         ┌───────────────▼──────────────────────────────┐
                         │          Client (Tauri + React/TS)            │
                         └──────────────────────────────────────────────┘
```

The arrows are one-way contracts. PlayerCareerCore never writes to FootballWorld state directly. It submits *player decisions* through the same `DecisionPort` every AI player uses, and it can *record life facts* (e.g. "sleep quality this week") that FootballWorld consumes through documented, bounded modifiers that also exist for AI players (AI players get simulated life facts from the same model).

## 5. The protagonist lifecycle (overview)

```
Creation (age 8–17 start options)
 → Grassroots / school football (optional start ≤12)
 → Academy trial → Academy scholar (U12–U18)  + school/education
 → First professional contract (age rules per nation)
 → Reserve/U21/B-team · loans · breakthrough
 → Senior career: selection battles, transfers, contracts, international
 → Peak · leadership · captaincy
 → Decline · role change · lower leagues / abroad
 → Retirement decision (player-driven)
 → Post-career: coaching badges, punditry, scouting, business, ambassador, family life
 → Legacy screen (records, honours, reputation curve, relationships)
```

## 6. Glossary

| Term | Meaning |
|------|---------|
| CA / PA | Current Ability / Potential Ability (hidden numeric totals, 1–200 scale as in FM-style models) |
| Mind | The decision-maker for a person. `AiMind` for everyone except the protagonist, who has `HumanMind`. |
| DecisionPort | The single interface through which any person's decisions enter the world. |
| Perception | A fogged, possibly biased estimate of a fact, owned by an observer (coach, scout, fan, protagonist). |
| Tick | One simulated day (world). Days have phases for the protagonist (morning / afternoon / evening). |
| Data pack | Versioned set of data files (rules, nations, names, weights). |
| LOD | Level of detail for *recording*, never for *outcome modelling* (fairness). |
| Fairness harness | Test that swaps the human with an AI mind to verify no hidden favouritism. |
| Window | Transfer registration window. |
| Homegrown | Registration category based on years trained in a nation/club before a threshold age. |

## 7. Scope guardrails

In scope: everything listed in the chapters. Out of scope for v1 (listed so they're not forgotten): online multiplayer careers, real-licensed names/badges shipped by default (user data packs only), VR/3D, esports modes, real-money anything.
