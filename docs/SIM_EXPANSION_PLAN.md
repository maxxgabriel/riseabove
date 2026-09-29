# Simulation expansion: agreed decisions, slices 0/1/4, Wave 0

Companion to `SIMULATION_EXPANSION_AUDIT.md`. Local branch `sim/expansion-audit`, not pushed.
Decisions below come from the owner's review of the audit and override it where they differ.

## 1. Decisions in force

| # | Decision | Consequence for the build |
|---|---|---|
| 1 | OFM injury fix allowed, tightly scoped | Adapter-only, small diff; mechanics verified before any rate calibration; calibration follows if incidence moves |
| 2 | One canonical player-creation pipeline with provenance | Done in Wave 0: `Players::push` requires an `Origin`; academy intake now created by `youth.rs`. The direct intake and the grassroots pipeline are still two *sources*, now measurable. Merging them is a later, measured change |
| 3 | Knowledge = truth + evidence + actor/organisation beliefs → perspective views | No per-audience copies and no single global estimate. `Belief { holder, about, value, spread, source, date }` kept only where disagreement changes a decision; public view derived from public evidence |
| 4 | Pain, compliance, support, mechanism may have real effects | Represent cheaply or derive; each must change a decision, probability, recovery or relationship, or it is not stored |
| 5 | Board = seats over existing people, no averaged opinion | Keep who supported, who opposed, who had authority, why |
| 6 | Sacking compensation deferred | Model the liability if useful; **off by default** until the local economy baseline is understood |
| 7 | One `Extensions` entry, organised by domain, not a god object | `medical`, `training`, `academy`, `governance`, `federation`, `decision_memory`; each with an owner and migration rule. Lands with the first slice that needs persistent state (Slice 1) |
| 8 | Measure first; no second metrics framework | Only non-overlapping read-only checks now; the rest specified in §5 |
| 9 | Decision memory for material decisions only | Ruling record with importance threshold |
| 10 | Coarse regions only | No coordinates |
| 11 | International success does not buff youth directly | Deferred |
| 12 | No fixed number of medical voices | Materiality decides how many opinions exist |
| 13 | Invariants are architecture | Started in Wave 0 |

## 2. Slice 0: foundations and measurement (not broad refactoring)

**Done (commit `ca97ac3`):** player provenance; academy intake moved under the youth domain; `pw_sim::invariants::check`.

**Remaining in Slice 0, behavioural and gated on your go-ahead:** the OFM injury-path correction (D1).

Files the D1 correction would touch:

| File | Change | Collision risk with unpushed work |
|---|---|---|
| `crates/pw-match/src/ofm.rs` | Scale OFM's contact-injury events by `injury_risk`; add a non-contact channel keyed `(seed, HEALTH, fixture uid, player)`. No other adapter code touched | Low (you report it was not recently changed) |
| `crates/pw-data/src/tuning.rs`, `data/engine/tuning.toml` | One new tuning field (non-contact base rate) | **Medium**: local runtime-tuning overrides |
| `crates/pw-sim/src/matchday.rs`, `health.rs` | `match_injury` receives minute and mechanism | Medium |
| new `crates/pw-cli/tests/…` file | Monotonicity test only | None (new file) |

Expected effect: match injuries become responsive to workload, fatigue, fragility and age. Contact injuries keep their present mean (thinning is mean-preserving); the non-contact channel *adds* incidence, so the season total rises. It must be followed by calibration against the 1.2–1.8 per player-season target, after checking that the direction is right (injury rate rises with `injury_risk` and with fatigue) and not by tuning until a number appears.

## 3. Slice 1: the rushed return

- **Input state:** an injured first-choice player; the case's true remaining fraction; a fixture of real importance (`matchday::importance` + `culture::stakes`) inside a few days; board satisfaction; manager archetype; the club's medical staff.
- **Beliefs / advice:** each *material* voice holds its own belief about remaining time: the medical lead (accurate, cautious), a physio (optimistic by role) if one exists, the player (own sense of readiness, shaped by pain, ambition, professionalism), the manager (need × importance × trust in medical). Only voices that disagree enough to matter are recorded. Pain reaches the manager only through what the player chooses to report.
- **Decision:** the manager decides unless the club's medical authority (owner/director policy) vetoes. Recorded as a `Ruling` (what was known, believed, advised, who supported what, risk accepted).
- **Event:** `Ruling` event, then `RushedBack` citing it; later `InjurySetback` or a normal return citing it.
- **Consequences:** chance resolves on the *true* remaining fraction, fragility, rehab compliance and support; a return ladder (individual → partial team → full training → bench → match) caps condition and sharpness; recurrence and body-wear feed back into `hazard_mult`.
- **Other systems:** selection (limited player in XI), match execution, press/social criticism, contract and valuation (dead `history_risk` wired), deal medicals, board.
- **Memory / history:** trust memories (medical overruled or vindicated; player pushed back); manager's learned propensity to rush, updated from outcomes only (a good decision that fails lowers it; a bad one that succeeds raises it).
- **Persistent state:** `Extensions.medical` (open-case context, beliefs) and `Extensions.decision_memory` (rulings, bounded per club, compacted after two seasons except consequential ones).

## 4. Slice 4: the board turns

- **Input state:** club position against target; owner ambition, patience, meddling, frugality; sporting director's recruits and how much they play; supporter unrest; finances; the manager's contract; quality of the available alternatives.
- **Beliefs / advice:** each seat (owner, chair, sporting director, finance director where the club has one) forms its *own* backing of the manager from its own pressures, recorded as facts. No averaged score is the source of truth; `Board.satisfaction` remains only as a cached summary.
- **Decision:** act when the seats with authority back a change, weighed against the alternative pool and the cost of leaving. Recorded as a `Ruling{Sack|Back}` with who supported, who opposed, who had authority, and why.
- **Event:** `Ruling`, then `ManagerSacked` or a public "backed" event, citing it.
- **Consequences:** the replacement is chosen by the seats' preferences; entourage and staff move; supporters and press react; the manager's CV records how it ended. **Compensation is computed and recorded but not paid** while the switch is off (decision 6).
- **Other systems:** market (new manager's tastes), academy (if the head of youth changes), culture (rivalry on manager moves), finance (liability), social.
- **Memory / history:** the ruling and its outcome; later boards can cite the precedent.
- **Persistent state:** `Extensions.governance` (seats over existing persons, backings) plus decision memory.

## 5. Wave 0 metrics and invariants (to merge into the single local framework)

Implemented now (read-only, non-overlapping): identity uniqueness both ways; squad/registration consistency; rehab bookkeeping; staff employment consistency; double-booked fixtures; origin completeness; `Players::created_by_source(from, to)` for youth-source counts.

To specify for the local framework rather than build here:
injury incidence and recurrence by mechanism and source; return-to-play stage timing; rushed-return counts and outcomes; academy graduates' share of first-team minutes; youth source counts by year (already derivable); staff quality distribution and drift by year; staff/manager turnover by role; training-load distribution; national-team call-up distribution; board ruling frequencies and who was overruled.

Further invariants to add as their systems land: no youth creation without provenance (type-enforced now), no return before an allowed stage unless marked a risky exception, no impossible contract overlap, no fixture participant outside valid registration, no competition/team membership contradiction.

## 6. Migration

Wave 0 adds one field to `Players` (`origin`). Pre-change saves do not load; this is the same break every layout change causes today (saves are unversioned positional bincode, and the save wrappers in `pw-career` and `pw-view` embed `World`, so a trailer chunk is not possible without touching them). I have not touched `save.rs`. Real migration and any version header wait until the local save architecture is visible; provenance for old saves would be backfilled as `DatabaseImport`/`SyntheticFixture` by date, never invented.

## 7. Collision map for unpushed local work

| Area | Touched so far | Later slices |
|---|---|---|
| `pw-world/src/player.rs` | yes (Origin, PlayerSource) | — |
| `pw-sim/src/people.rs`, `youth.rs` | yes (intake moved) | Slice for youth |
| `pw-import/src/{csvimport,synthetic}.rs`, `pw-career/src/create.rs` | one-line source tags | — |
| `pw-sim/src/lib.rs` | one `pub mod` line | hooks per slice |
| `pw-world/src/world.rs` | no | one `ext` field with Slice 1 |
| `pw-world/src/event.rs`, `pw-narrate/src/events.rs` | no | nested domain variants; narrate arm needed |
| `pw-core/src/rng.rs` (`stream::ALL`) | no | new stream ids |
| `data/engine/tuning.toml`, `pw-data/src/tuning.rs` | no | D1 field |
| `PROGRESS.md`, `pw-cli/tests/world.rs`, `pw-view/**`, `app/**` | no | avoided |
