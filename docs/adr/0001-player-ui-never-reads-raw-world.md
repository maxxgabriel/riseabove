# ADR 0001: The player-facing UI never reads the raw world

**Status:** accepted (locked design §8, §9). **Implemented:** `pw-view::Ctx`, `Session::act`.

**Decision.** Every page is produced by `pw-view` from a read-only view scoped to a perspective (observer or one inhabited person).
Actions are typed intents queued through the session; the frontend never sends world state.

**Why.** Actors in this game act on beliefs, and the person playing must only know what their character could. A screen that reads
the world and hides fields is one refactor away from leaking hidden ability, private trust or unreported interest, and sorting or
filtering can leak even when a value is not shown.

**Consequences.** New pages go through `Ctx`; lists must say whose belief they sort by; debug omniscience should be a separate,
labelled path (today observer mode fills that role). Do not "simplify" by handing the frontend a world type.
