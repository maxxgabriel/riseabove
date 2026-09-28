# Reusable Systems

Specific files/modules from each project that could be ported or used directly.
License noted per project.

---

## From OpenFootManager — GPL-3 ⚠️

If the final project is GPL-3 (open source), these can be used directly.
If not GPL-3, algorithms can be studied and reimplemented (ideas only — not code).

| Module | File(s) | Reuse Class | What to take |
|--------|---------|-------------|-------------|
| Player aging & retirement | `ofm_core/src/aging.rs` | A | Seeded retirement chance table (33-37+), pace decay, technical growth; deterministic on player_id + season + salt |
| AI team selection algorithm | `ofm_core/src/live_match_manager/team_builder.rs` | A (GPL) / C (MIT project) | `ai_select_starting_xi_with_scorer()` with 5-factor scoring, load management; `player_selection_outlook()` for protagonist feedback |
| Transfer market sweep | `ofm_core/src/transfers/market.rs` | B | `buyer_has_genuine_interest()`, `incoming_interest_score()`, squad depth calculation |
| Transfer negotiation | `ofm_core/src/transfers/bids.rs` + `lifecycle.rs` | B | Multi-round negotiation, rebid cooldown logic |
| Contract expiry / free agents | `ofm_core/src/contracts/expiry.rs` + `free_agent.rs` | B | Expiry detection, AI free-agent pickup |
| Training system | `ofm_core/src/training.rs` | B | Coaching bonus formula, per-player focus override, AI fatigue guard |
| Manager firing | `ofm_core/src/firing.rs` | B | Satisfaction threshold → warning → final_warning → fire; stage tracking |
| Manager hiring | `ofm_core/src/ai_hiring.rs` | B | Club-vacant → assistant stand-in → generated manager; career record |
| Player morale core | `domain/src/player.rs` `PlayerMoraleCore` | B | manager_trust, unresolved_issue, pending_promise, renewal_state; clean state machine |
| Loan system | `ofm_core/src/transfers/loans.rs` | B | Loan offer structure, wage contribution %, buy option, development tracking |
| Match report format | `engine/src/report.rs` `PlayerMatchStats` | A | Direct: minutes, goals, assists, shots, passes, tackles, cards, rating (f32) |
| Match events | `engine/src/event.rs` `EventType` | A | 15+ event types; per-minute with player+side |
| Positional fit scoring | `ofm_core/src/player_rating.rs` | B | `effective_rating_for_assignment()`, `positional_fit_for_assignment()` |
| Tactical role fit | `team_builder.rs::tactical_role_fit()` | B | 30 roles, 3-attribute fit scoring per role+slot |
| Protagonist transfer gate | `transfers/market.rs::controlled_player_is_known_to_buyer()` | A (GPL) / C | Observation-with-evidence requirement before club can approach protagonist |

---

## From open-football — Apache-2 ✓

All code below is Apache-2 and can be used in any project, including commercial or proprietary.

| Module | File(s) | Reuse Class | What to take |
|--------|---------|-------------|-------------|
| Skill model (FM-style 1-20) | `core/src/club/player/ability/skills.rs` | B | Technical/Mental/Physical structure; weighted ability calculation per position; 40+ skills |
| Position weights system | `core/src/club/player/ability/position_weights.rs` | B | Per-position skill weight tables; maps to FM's positional importance |
| Hidden potential system | `core/src/club/player/ability/attributes.rs` | A | `potential_ability` hidden from all AI; `ability_marker` for observable progress; scouting estimate separation |
| Academy system | `core/src/club/academy/` (7 files) | B | intake.rs, training.rs, graduation.rs, tuning.rs — full youth pipeline |
| Club board/chairman | `core/src/club/board/` | C | Mandate, doctrine, personality, promises — algorithm design reference |
| Manager candidate scoring | `core/src/club/board/manager/scorer.rs` | B | How boards evaluate manager candidates |
| Transfer policies | `core/src/club/transfers/strategy.rs` | B | RecruitmentPolicy, NegotiationPolicy — all the levers FM-style AI uses |
| Headless simulator | `core/src/simulator/mod.rs` + `.dev/simulate/` | A | Daily tick architecture; async; parallel; provably runs multi-season without human |
| Competition simulation | `core/src/competitions/simulation.rs` | B | GlobalCompetitionSimulator; match result application |
| Staff perception | `core/src/club/staff/perception.rs` | B | AbilityEstimator, PotentialEstimator, CoachEye — grounded scouting |
| Player squad social view | `core/src/club/player/core/player.rs SquadSocialView` | B | Language/nationality cohesion in dressing room |
| Sell-on obligations | `core/src/club/player/core/player.rs SellOnObligation` | A | Stacked sell-on percentage records; persistent per transfer |
| Player language profile | `core/src/club/player/language.rs` | B | Language learning/adaptation model |

---

## Algorithm-only references (no code copying regardless of license)

| Algorithm | Source | Notes |
|-----------|--------|-------|
| FM-style CA/PA development | open-football/attributes.rs | The CA/PA pattern itself is a design idea, not protectable code; can implement independently |
| Retirement chance table | OFM/aging.rs | Design pattern only; implement with own values |
| Positional fit scoring | OFM/team_builder.rs | Weighted attribute average per position-group is a general algorithm |
| Transfer interest scoring | OFM/market.rs | Contract status + listed + value + morale scoring pattern |
| Coach assessment decay | OFM/team_builder.rs | Time-weighted observation with confidence; implement independently |
