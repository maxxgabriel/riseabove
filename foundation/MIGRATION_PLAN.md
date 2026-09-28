# Migration Plan

Following the recommendation: open-football as FootballWorld base + new PlayerCareerCore crate.

FM15 work is frozen, not deleted. It becomes the behavioral reference.

---

## Phase 0: Orientation (Week 1-2)

**Goal:** Understand open-football's daily tick well enough to safely add a PlayerCareerCore hook.

1. Build open-football in release mode. Run `.dev/simulate 365` — confirm it completes without panic.
2. Read `simulator/mod.rs` fully — understand phase order (Prologue → Matchday → Periodic → World → Epilogue → Honours)
3. Read `SimulatorData` struct — find where players, clubs, competitions, matches all live
4. Run `.dev/simulate 60` under a profiler to see which phases dominate
5. Identify the post-match hook point where per-player stats are computed

**Deliverable:** A written `OPEN_FOOTBALL_ORIENTATION.md` documenting tick phase order, entity locations, and the identified hook point.

---

## Phase 1: Protagonist Injection (Week 2-3)

**Goal:** A named player entity exists in open-football's world and appears on a club's squad.

1. Create a `PlayerCareerCore` Rust crate (Apache-2, our own)
2. Add a `ProtagonistConfig` struct: `player_id`, `start_club_id`, `start_date`, `attributes`
3. At world initialization, inject a standard `Player` into open-football's world — no special flags
4. Verify the player appears in the club's squad depth count
5. Advance 7 days — verify the player is selected (or not) by the AI manager using the normal selection algorithm
6. Verify the player is not treated differently from any AI player in any system

**Test:** After 30 simulated days, the protagonist's attributes, condition, and career entry should match what any player of equivalent stats would show.

---

## Phase 2: Match Observer (Week 3-4)

**Goal:** After each match the protagonist plays, capture per-player stats and emit a `CareerEvent`.

1. Add a post-matchday hook in the Epilogue phase (safe write point, after match results applied)
2. For each match the protagonist was in: extract `PlayerMatchStats` equivalent (minutes, goals, assists, position, rating)
3. Emit `CareerEvent::MatchPlayed { ... }` into PlayerCareerCore's event log
4. Write the first PlayerCareerCore query: "what was the protagonist's last match performance?"

**Test:** Simulate a full season. Protagonist's season stats (appearances, goals, minutes) should be derivable from the event log.

---

## Phase 3: Selection Forecast (Week 4-5)

**Goal:** "Will the AI manager pick me for the next match?" — answered before the match runs.

Reimplement OFM's `player_selection_outlook()` algorithm against open-football's model:
1. Identify open-football's team selection logic (confirm it exists; investigate if not)
2. Run a shadow selection at query time using the same inputs the engine will use
3. Return: `in_starting_xi: bool`, `closest_competitor: Option<String>`, `dominant_factor`
4. Expose this via PlayerCareerCore query API

**Note:** This requires understanding open-football's actual team selection algorithm — pending code investigation.

---

## Phase 4: Transfer Integration (Week 5-6)

**Goal:** AI clubs can only approach the protagonist after observing them; protagonist can accept/reject.

1. Add `observed_clubs: HashSet<club_id>` to protagonist state, populated when a club's scout watches a match
2. Gate AI transfer approaches on `observed_clubs`
3. Expose incoming offers to PlayerCareerCore for user decision
4. Handle acceptance: protagonist moves club in open-football's world normally
5. Handle rejection: rebid cooldown for approaching club

---

## Phase 5: PlayerCareerCore Depth (Weeks 6-12)

These systems are entirely new — no open-source equivalent exists:

| System | Description |
|--------|-------------|
| Career finances | Weekly wage, agent fee, signing bonus, endorsements, expenses |
| Personal life | Family events, housing (city changes on transfer), relationships |
| Agent | Negotiation proxy; agent quality affects contract outcomes |
| Media reputation | Press coverage scale of career moments; affects transfer interest |
| Health | Long-term cumulative load → burnout risk; separate from in-game injury |
| Education/youth | Pre-professional background; affects starting stats range |
| Personal history | Persistent log of milestones (debut, first goal, first cap, etc.) |
| Confidence | Short-term performance signal; affects AI selection weight for protagonist |

Each system emits `CareerEvent`s and reads from open-football world state where needed.

---

## Phase 6: Career UI (Weeks 12-20)

**Description:** The interface through which the user experiences the protagonist's career.

Built as a separate frontend — could be Tauri (like OFM), web, or terminal-first.

Key screens:
- Match day: "Are you in the starting XI? Why?" + result + your stats
- Week view: training, media, personal events, incoming offers
- Career timeline: history of clubs, seasons, milestones
- Transfer window: offers received, agent advice, club exploration
- Personal: finances, family, agent contract

---

## What is kept from FM15 work

| FM15 finding | How it's used |
|-------------|---------------|
| `select_best_team_recursive` algorithm | Behavioral benchmark for calibrating open-football's selection |
| ACTUAL_PLAYER struct layout | Understanding what "deep attributes" means; guides attribute model design |
| 342 property FourCCs | Cross-reference for PlayerCareerCore's attribute naming |
| GAME_TEAM_SELECTOR_RECURSIVE pipeline | Reference for how a mature engine dispatches team selection |
| Session 2 identity chain | Confirms player_id at ACTUAL_PLAYER+0x154; may be used if FM15 is ever integrated |

---

## Scope estimate

| Phase | Duration | Risk |
|-------|----------|------|
| 0 — Orientation | 2 weeks | Low |
| 1 — Injection | 1 week | Medium (unknown API) |
| 2 — Match observer | 1 week | Medium |
| 3 — Selection forecast | 1-2 weeks | Medium-High |
| 4 — Transfer integration | 1 week | Medium |
| 5 — Career depth | 6 weeks | Low (new code) |
| 6 — Career UI | 8 weeks | Low |
| **Total** | **~20 weeks** | |

First playable prototype (can see protagonist's career, selection status, match stats): ~6-8 weeks.
