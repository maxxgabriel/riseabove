# Architecture Options

Six options evaluated against the Player Career goal:
- FootballWorld runs autonomously
- Protagonist is an ordinary Player entity — no hidden favoritism
- Match simulation produces per-player data (no 3D graphics needed)
- Long-term world evolution (5-20 seasons)

---

## Option A: OpenFootManager as base, port deeper systems

**Description:** Fork OFM. Add open-football's skill model (40 attrs → OFM's 19). Deepen transfer, injury, youth systems. Build PlayerCareerCore as a new OFM module.

**Pros:**
- Player career mode already started (`is_player_career()`, `controlled_player()`, `player_selection_outlook()`)
- AI selection forecasting built in
- Transfer gating for protagonist (`controlled_player_is_known_to_buyer()`)
- Tauri desktop app ready to extend
- Active development, real tests

**Cons:**
- GPL-3: entire project must be open source
- 19 attribute model is thin for a deep career sim
- Not designed for headless multi-season simulation
- UI is manager-focused; career UI would be a major rebuild

**Score for goal:** 7/10

---

## Option B: open-football as base, graft PlayerCareerCore

**Description:** Use open-football's headless world simulator (Apache-2) as the autonomous football world. Build PlayerCareerCore separately on top. Build a new UI.

**Pros:**
- Apache-2: no licensing constraint
- Deepest autonomous world simulation (1094 files, parallel rayon, proven headless)
- FM-style 1-20 skill model with hidden potential
- Full academy, board, competition, club hierarchy
- Designed to "watch the game evolve by itself" — exactly what we need
- Position weights, staff perception, scouting estimation all built

**Cons:**
- No player career concept: protagonist must be grafted in from scratch
- No "will the AI select me?" forecast — must build
- No transfer gating for protagonist — must build
- Match output per-player API needs verification and wrapping
- Large codebase (1094 files) — steep onboarding

**Score for goal:** 7.5/10

---

## Option C: Agentic FC as base

**Not applicable.** Repository not found. Cannot evaluate.

---

## Option D: Another candidate (FM15 as reference lab)

**Description:** Continue RE-ing FM15 to understand its exact algorithms, then reimplement them independently. Use FM15 as the behavioral oracle.

**Pros:**
- Perfect football simulation (the gold standard)
- RE work already done: team selection pipeline, player attributes, property system

**Cons:**
- FM15 cannot be redistributed or directly used as the world engine
- Player career protagonist would require FM15 running alongside the sim
- Already decided: FM15 is the reference lab, not the runtime

**Score for goal:** 2/10 (reference value only)

---

## Option E: Build new core, reuse selected open-source modules

**Description:** New Rust project. Take specific algorithms from OFM (aging, selection, morale) and open-football (skill model, hidden potential, transfer policies, academy). Build FootballWorld and PlayerCareerCore from scratch.

**Pros:**
- Clean design, no legacy constraints
- Pick the best algorithms from each project
- Apache-2 or MIT license from day one
- Can model the protagonist from the start without retrofitting

**Cons:**
- Longest path — months of foundational work before anything runs
- Must reimplement everything that isn't directly copied (GPL-limited algorithms must be reimplemented)
- High risk of implementing stub systems that "exist" but don't participate

**Score for goal:** 6/10

---

## Option F: Hybrid — open-football for world, OFM for career layer

**Description:** Run open-football headless as FootballWorld. Build PlayerCareerCore as a separate service that reads world state and generates career events. Use OFM's player career patterns (selection_outlook, transfer gating, morale core) reimplemented against open-football's data model.

**Pros:**
- World simulation depth from open-football (Apache-2)
- Career concepts from OFM (GPL-3 algorithms reimplemented, not copied)
- Clean separation: FootballWorld ↔ PlayerCareerCore interface
- Protagonist is literally an entry in open-football's player list

**Cons:**
- Two different Rust codebases to integrate
- open-football's entity model is not designed for protagonist injection — must understand and extend
- open-football is async (tokio/rayon) — integration surface is complex
- No OFM code can be directly used due to GPL-3

**Score for goal:** 8/10

---

## Summary

| Option | World depth | Career support | License | Build cost | Recommended? |
|--------|------------|----------------|---------|------------|--------------|
| A (OFM base) | Medium | High | GPL-3 ⚠️ | Low | If open-source OK |
| B (open-football base) | Very High | None (build) | Apache-2 ✓ | Medium | Strong candidate |
| C (Agentic FC) | Unknown | Unknown | Unknown | N/A | Cannot evaluate |
| D (FM15 ref) | N/A | N/A | Illegal | N/A | Reference only |
| E (new core) | You build | You build | Free choice | High | Long-term bet |
| **F (hybrid)** | **Very High** | **Build on good foundation** | **Apache-2 ✓** | **Medium-High** | **Recommended** |
