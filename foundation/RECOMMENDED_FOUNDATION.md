# Recommended Foundation

## Recommendation: Option B — open-football as base world, OFM patterns reimplemented for career layer

**Single engineering recommendation:** Use **open-football (Apache-2)** as the FootballWorld engine. Build PlayerCareerCore in a separate Rust crate that attaches to open-football's world as a standard player entity. Reimplement (not copy) key OFM career algorithms against open-football's data model.

---

## Why not OFM directly?

OFM has the best player career architecture — but it is **GPL-3**. If the project is ever closed-source or commercially licensed, GPL-3 makes OFM unusable as a foundation. The algorithms are excellent; the license is the constraint.

If the project commits to being permanently open-source under GPL-3, OFM becomes the stronger choice (Option A) due to its existing player career infrastructure.

---

## Why open-football?

Evidence from code:

1. **Designed for this exact purpose.** The simulator's documented purpose is "watch football ecosystems run, develop, and surprise you on their own." Its headless `FootballSimulator::simulate()` is a daily tick with no human interaction required. Verified via `.dev/simulate` binary (accepts `[days]` argument).

2. **World depth that matters.** 1094 Rust files. Real academy (intake → training → graduation). Real board/chairman system with personality and mandate. Real transfer pipeline (RecruitmentPolicy, NegotiationPolicy). Real continental/national/domestic competition hierarchies. Real parallel simulation (rayon + mimalloc). FM15 would still win on depth, but open-football is the closest open-source alternative.

3. **FM-style skill model.** Technical (15) + Mental (14) + Physical (8) attributes on a 1-20 scale with position-weighted ability calculation. Hidden `potential_ability` properly separated from `current_ability` — clubs estimate via `PotentialEstimator`, not direct reads. This is the right design.

4. **Apache-2 license.** No restriction on commercial use, proprietary modifications, or future licensing changes.

5. **Protagonist is an ordinary player.** Nothing in open-football has a special "human player" branch. Injecting a protagonist as a standard Player entry in a club's squad is the correct architecture. The world makes no special accommodations.

---

## What must be built regardless of choice

Even with the best foundation, these systems must be built from scratch or significantly extended:

| System | Why it must be built | Effort |
|--------|---------------------|--------|
| **Player selection forecast** | "Will the AI manager pick me for Saturday?" — Not in open-football | Medium |
| **Protagonist transfer gating** | Prevent clubs with zero observation of the player from approaching | Medium |
| **PlayerCareerCore entities** | Education, family, finances, agent, housing, media reputation, health, personal history | High |
| **Career event system** | Goal scored, injury, debut, hat-trick, transfer, international call-up, personal milestone | High |
| **Career UI** | The entire player perspective (not manager view) | High |
| **Protagonist retirement agency** | Player decides when to retire, not the simulation engine | Low |
| **Attribute mapping** | Map open-football's 40 attrs to PlayerCareerCore's attribute model | Medium |
| **Match observation API** | Read per-player stats for protagonist's match — minutes, rating, goals, position | Medium |

---

## What to build from scratch vs. port

| Component | Action |
|-----------|--------|
| FootballWorld engine | Use open-football as-is (or with light extension) |
| Protagonist Player entity | Add to open-football's player list — no forks needed |
| Match output per protagonist | Wrap open-football's match result reader; map to CareerEvent |
| Team selection forecast | Reimplement OFM's `player_selection_outlook()` algorithm against open-football's AI selection |
| Transfer gating | Reimplement OFM's `controlled_player_is_known_to_buyer()` against open-football's scouting model |
| Development (CA/PA) | Use open-football's model; add coaching observation multiplier for protagonist |
| Aging | Use open-football's age curves |
| Retirement | Use open-football for all AI players; protagonist retirement is a user action |
| PlayerCareerCore | New crate; interfaces via open-football's `SimulatorData` |
| Career events | New event stream reading open-football's match results |
| Career UI | New frontend; reads PlayerCareerCore state |

---

## On FM15

FM15 remains the **reference laboratory**. Its team selection algorithm, property system, and player struct layout are confirmed and documented. When implementing open-football's selection algorithm, FM15's backtracking branch-and-bound design is the benchmark for what "realistic" means. FM15 should be consulted for behavioral calibration, not used as runtime.

---

## Risk factors

1. **open-football's match output API**: Per-player stats exist in competition tracking but the read API at the SimulatorData level needs verification. This is the highest technical risk.

2. **Protagonist injection**: open-football has no concept of a human-controlled player. Adding one requires understanding the daily tick well enough to inject observation hooks without corrupting determinism.

3. **Simulator state size**: 1094 files is a large surface to learn. Budget 2-4 weeks for orientation before productive extension.

4. **open-football's async model**: Uses tokio + rayon. PlayerCareerCore integration must respect the async boundary.
