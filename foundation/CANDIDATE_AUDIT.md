# Candidate Foundation Audit

Conducted: 2026-09-28. Code-level inspection — not README claims.

---

## Candidate Status

| Project | Found | Audited |
|---------|-------|---------|
| OpenFootManager (OFM) | ✓ E:\pers\openfootmanager | ✓ Full |
| open-football (ZOXEXIVO) | ✓ E:\pers\open-football-zox | ✓ Full |
| Agentic FC | ✗ Not found | Repo does not exist on GitHub under any plausible name |
| League-Soccer | ✗ Not found | Repo does not exist on GitHub |

---

## 1. OpenFootManager (OFM)

**Repo:** E:\pers\openfootmanager  
**Version:** 0.3.0  
**License:** GPL-3 ⚠️ — derivative works must also be GPL  
**Language:** Rust (backend) + TypeScript/React (UI) via Tauri desktop  
**Scale:** ~102 core Rust files (ofm_core), ~50 engine files, ~25 domain files

### Architecture

- **Crates:** `domain` (types), `engine` (match sim), `ofm_core` (world logic), `db` (SQLite persistence), `ofm-cli`
- **Entity model:** `Game` holds `teams`, `players`, `managers`, `staff`, `competitions`, `player_career`
- **Persistence:** SQLite via `db` crate with migrations; JSON save/load via serde
- **Time model:** Turn-based daily advance; player clicks "Continue"; async matchday processing
- **Event architecture:** `MatchEvent` per minute during live match; `Message`/`News` system for world events
- **Determinism:** Seeded RNG (player_id + season + salt) for aging, retirement — deterministic replay possible
- **Player Career mode:** Explicitly built-in via `Game::is_player_career()`, `controlled_player()`, `HumanController::Player(id)`

### World Model

| System | Implementation | Status |
|--------|---------------|--------|
| Clubs | `domain/team.rs` — finances, formation, training groups, player roles, staff | ✓ Real |
| Competitions | `domain/league.rs` — fixtures, standings, knockouts, group stages | ✓ Real |
| Player aging & retirement | `ofm_core/aging.rs` — seeded annual: pace decline 30+, technical growth <32, retirement chance 33-37+ | ✓ Real |
| Manager careers | `ai_hiring.rs`, `firing.rs` — satisfaction thresholds, warnings, firing, vacant club replacement | ✓ Real |
| Season progression | `end_of_season/mod.rs` — promotions, relegations, berths | ✓ Real |
| Historical stats | `domain/world_history.rs`, `db/stats_repo.rs` — career entries with attribute snapshots | ✓ Real |
| National teams | `domain/national_team.rs`, `national_team.rs` — squad selection, international matches | ✓ Real |
| Finances | `finances/mod.rs` — wages, transfer fees, sponsorship, marketing, board support, debt | ✓ Real |

**FM15 gap:** No facilities/stadium investment system. Club reputation changes but no explicit infrastructure progression. Domestic finances only (no TV deal variation by division tier beyond reputation proxy).

### Player Model

| Field | OFM Implementation | FM15 Equivalent |
|-------|------------------|----------------|
| Attributes | 19 attributes (pace, stamina, strength, agility, passing, shooting, tackling, dribbling, defending, positioning, vision, decisions, composure, aggression, teamwork, leadership, handling, reflexes, aerial) all 0-100 | ~70 attributes, 1-20 scale |
| Potential | `potential: u8` (1-99) set at generation; higher than ovr for youth | ✓ Hidden PA/CA |
| Fitness/condition | `condition: u8` (0-100 short-term), `fitness: u8` (0-100 long-term) | ✓ Condition + fitness |
| Injury | `injury: Option<Injury>` with `days_remaining` | Basic (name + days); no type/severity |
| Morale | `morale: u8` + `morale_core: PlayerMoraleCore` (manager_trust, issues, promises, renewal_state) | ✓ Deep |
| Contract | `contract_end`, `wage`, `market_value`, `transfer_listed`, `loan_listed` | ✓ |
| Career history | `career: Vec<CareerEntry>` with attribute snapshot per season + awards | ✓ |
| Transfer history | `movement_history: Vec<PlayerMovementEntry>` with fee/kind/clubs | ✓ |
| Active loan | `active_loan: Option<ActiveLoan>` with development tracking | ✓ |

**FM15 gap:** 19 attributes vs FM15's ~70. Personality (determination, professionalism, ambition) exists only as `morale_core.manager_trust`. No explicit positions weighting beyond 4 broad groups in positional fit scoring.

### Manager/Club AI

**Team selection** (`live_match_manager/team_builder.rs`):

Real algorithm for AI clubs:
```
ai_select_starting_xi_for_team():
  Step 1: positional_fit_for_assignment + form + fitness + trust + tactical_fit + coach_assessment_adjustment
  Step 2: load management — rest tired player if fresher same-group player within fit_tolerance
```

Coach assessment adjustment uses recent match/training/trial/scouting observations (up to 180 days old), with recency and confidence weighting. Multiple coaches can hold independent views of the same player.

`player_selection_outlook()` — computes whether the protagonist would be selected by their AI club manager, identifying the dominant factor (ability, form, fitness, workload, trust, tactical_fit). **This is directly useful for the Player Career UI.**

**Manager AI:**
- Substitutions: fatigue threshold (condition < 50-75 depending on quality), tactical subs when losing after 65'
- Tactical changes: style shifts (attacking/defensive/highpress) based on score + minute + personality
- AiPersonality: Pragmatist, Visionary, Reactive — distinct decision profiles

**Recruitment AI** (`transfers/market.rs`):
- `buyer_has_genuine_interest()` — reputation deficit check + squad depth check
- `incoming_interest_score()` — contract expiry, transfer listed, value, morale modifiers
- `controlled_player_is_known_to_buyer()` — requires real match/scouting observation within 120 days before a controlled player can receive interest — prevents fantasy transfer spam

**FM15 gap:** No explicit scouting radius/assignment system (OFM has assignments but simpler than FM). No manager attributes affecting negotiation directly.

### Transfer System (full path)

1. **Need detection:** `squad_position_depths()` per club; `buyer_has_genuine_interest()` filters
2. **Candidate discovery:** Daily sweep scores all players; transfers/market.rs
3. **Evaluation:** `minimum_acceptable_fee()` (market value × multiplier based on listed/contract status/importance)
4. **Interest:** `incoming_interest_score()` drives likelihood of approach
5. **Negotiation:** `transfer_negotiation_metrics()` in bids.rs; multi-round with counter-offers; `rebid_cooldown` prevents harassment
6. **Player decision:** Controlled player sees and accepts/rejects; AI player accepts based on wage/wage_offered
7. **Contract:** `execution.rs` — fee debited, player moves team, jersey assigned, loan ended if active
8. **Registration:** `registration.rs` — competition registration deadlines enforced

**Status: B** — usable with adaptation. Missing: release clauses, agent negotiations, chairman approval, rival club bidding wars.

### Development System

`training.rs` — runs daily per training day:
- `compute_coaching_bonus()` from coaching staff rating + specialization match
- `physio_mult` from physio staff for recovery
- Per-player focus override > group focus > team default
- `AI_FATIGUE_GUARD_CONDITION = 40` — auto-rests exhausted AI players
- Controlled player development: `unlock_growth_room` — coaching + manageable load + psychology can gradually unlock more of seeded growth range
- Seasonal: `apply_seasonal_aging()` — pace decay 30+, technical growth <32, `should_retire()` seeded

**Status: B** — real mechanics. Missing: training injury risk, morale impact on training gain, facility-level bonus is single multiplier.

### Match Output

`engine/report.rs` — `PlayerMatchStats`:
- `minutes_played: u8`
- `goals: u8`, `assists: u8`
- `shots: u8`, `shots_on_target: u8`
- `passes_completed: u8`, `passes_attempted: u8`
- `tackles_won: u8`, `interceptions: u8`
- `fouls_committed: u8`, `yellow_cards: u8`, `red_cards: u8`
- `rating: f32` (0.0–10.0)

Match events: `EventType` includes Goal, OwnGoal, YellowCard, RedCard, Penalty, FreeKick, Corner, KickOff, FullTime, Substitution, Injury. All per-minute with player+side.

**Status: A** — directly provides everything Player Career needs.

### World Autonomy

OFM is designed as a manager game where the human controls one team. For player career use, AI clubs handle all their own affairs autonomously. Confirmed real implementations:
- AI manager hiring/firing with satisfaction threshold → warning → fire → replacement cycle
- AI transfer market runs daily
- Seasonal aging and retirement
- Competition scheduling and promotion/relegation
- AI training decisions

**Multi-season autonomous test:** The `catchup.rs` mid-season fill and `dormant.rs` suggest off-screen seasons are supported. The `ofm-cli` crate suggests headless operation is possible. However, the architecture is primarily built for interactive use — the "Continue" button advances one or more days.

**Key gap:** No headless multi-season benchmark run exists. Would need verification.

---

## 2. open-football (ZOXEXIVO)

**Repo:** E:\pers\open-football-zox  
**License:** Apache 2.0 ✓ — permissive, can use in any project  
**Language:** Rust (all backend); web frontend exists but is secondary  
**Scale:** 1094 Rust files in core — approximately 5× OFM's size

### Architecture

- **Crates:** `core` (everything), `database` (data loading), `web` (optional HTTP API), match engine embedded in core
- **Simulation model:** Daily tick via `FootballSimulator::simulate()` — truly headless world simulation
- **Parallelism:** Rayon thread pool for parallel continent/club processing; mimalloc for perf
- **Entity model:** `SimulatorData` holds all world state; continents → countries → clubs → players
- **Time model:** Date-driven daily tick; designed for continuous multi-year simulation
- **Purpose:** "Watch the game evolve by itself" — zero human management assumed

### World Model — much deeper than OFM

| System | Files/modules | Status |
|--------|--------------|--------|
| Clubs | club/core/, club/board/, club/finance/ — board chairman, mandate, estate, personality, ownership/benefactor | ✓ Very deep |
| Competitions | competitions/ + continent/competitions/ + continent/tournaments/ — global/continental/national tiers | ✓ Very deep |
| Academies | club/academy/ — intake, training, graduation, tuning, callups | ✓ Real |
| Manager careers | club/board/manager/ — search, shortlist, candidate scoring, market, seat, approach | ✓ Deep |
| Player development | club/player/development.rs — CoachingEffect; ability curves | ✓ Real |
| Finances | club/finance/ — complex multi-source | ✓ Real |
| Country/economy | country/ — media, economy, national teams | ✓ |
| Continent rules | continent/regulations/, continent/rankings/ | ✓ |
| Awards | awards/ — monthly, season, POTY, team of week/year | ✓ |

### Player Model

Skills on 1-20 FM-style scale (unlike OFM's 0-100):
- Technical (15 skills: corners, crossing, dribbling, finishing, first touch, flair, free kicks, heading, long shots, long throws, marking, passing, penalty taking, tackling, technique)
- Mental (14 skills: aggression, anticipation, bravery, composure, concentration, decisions, determination, leadership, off_the_ball, positioning, teamwork, vision, work_rate + others)
- Physical (8 skills: acceleration, agility, balance, jumping, natural_fitness, pace, stamina, strength)
- Goalkeeping (GK-specific)

`PlayerAttributes` (separate from skills): condition (0-10000 int), fitness, jadedness, weight, height, value, current_ability (1-200), hidden `potential_ability`, ability_marker for observable progression, international reputation.

**Hidden potential:** `potential_ability` is private — clubs can only estimate via `PotentialEstimator`. More realistic than OFM's transparent `potential` field.

**Mind system:** `PlayerMind` — full neural-network-style agent with episodes, encoding inputs, situation assessment. Much deeper than OFM's morale_core.

### Transfer System

`src/core/src/club/transfers/strategy.rs`:
- `RecruitmentPolicy` — philosophy, financial_stance, signing_preference, youth_focus, age_preference, min_scouting_confidence, resale_value_sensitivity, domestic_bias
- `NegotiationPolicy` — buying_aggressiveness, wage_discipline, fee_discipline, max_overpay_ratio, installment_preference, addon_preference, sell_on_preference, loan_preference, risk_appetite
- Full transfer pipeline: `TransferRequest` → `TransferApproach` → `PersonalTermsOffer` → `TransferClause` → `TransferOffer`
- Board mandate, ledger, promises system

**Status: B** — very deep but requires integration work.

### Development System

`club/player/development.rs` — `CoachingEffect`: real attribute progression with coaching influence. `club/academy/training.rs` — youth development. Age-based curves with position-weighted importance.

### Match Output

Match engine is embedded in core; produces per-player statistics. The `.dev/simulate` binary uses `match-stub` feature that collapses to 0-0 scorelines for profiling without engine overhead — meaning the match engine IS separable from the world sim.

`FieldSquad` feeds the engine; `Score` comes back plus per-player stats (implied by competition tracking).

**Status: A for autonomous world; B for player-level data extraction** — match output exists but reading individual player stats from the world sim requires more investigation.

### World Autonomy

**This is the primary use case.** The simulator is designed for zero human input:
- `FootballSimulator::simulate()` daily tick with no user interaction points
- Parallel rayon processing across clubs and continents
- Full manager hiring/firing cycle
- Academy intake → development → graduation → senior squad
- Transfer market operates continuously
- Seasons progress, promotions/relegations fire
- Awards distributed

**Multi-season test:** `.dev/simulate` binary accepts `[days]` parameter — verified to be a real headless driver. Can simulate 60, 365, 1825 days without human input.

**Performance:** mimalloc + rayon = genuinely fast headless sim. Expected: full European world over one season in minutes on modern hardware.

---

## 3 & 4. Agentic FC / League-Soccer

**Not found.** No repository exists under these names on GitHub. The user may have been referring to private projects, projects under different names, or future-planned work. Cannot audit.

---

## FM15 Benchmark Comparison

What a mature FM-style implementation has that neither candidate fully covers:

| System | FM15 depth | OFM | open-football |
|--------|-----------|-----|--------------|
| Attributes | ~70 (1-20) | 19 (0-100) | ~40 (1-20) |
| Personality | determination, professionalism, ambition, loyalty, etc. | morale_core (simplified) | Mind system (deep) |
| Position specificity | ~25 granular positions | 17 positions | PlayerPositionType (many) |
| Tactical depth | full FM roles/duties per position | roles system (20+ roles) | position_weights (detailed) |
| Staff attributes | 20+ per staff member | coaching, judging, physiotherapy | full staff hierarchy |
| Competition structure | continental → national → domestic cups | competitions + national | continent → country → club |
| Injury system | 100+ injury types, recovery rates | name + days_remaining | severity, type, recovery speed |
| Scouting radius | country/region budgets, scout ability | assignment system | ScoutingRegion system |
| Club facilities | training ground, youth facilities | not modeled | estate system |
| Media/reputation | press conferences, media handling | news system | country/media system |
| Youth intake | realistic regional generation | youth scouting assignments | academy intake + tuning |
