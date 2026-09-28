# Systemic Simulation Rules

Concrete architectural rules that turn [`PRODUCT_NORTH_STAR.md`](PRODUCT_NORTH_STAR.md) into code
constraints. Each rule has an ID so reviews and commits can cite it (for example "violates S3").
"MUST" rules block a merge. "SHOULD" rules need a written justification when broken.

---

## A. The world and the mind

**S1. The world runs without a human. (MUST)**
`pw-sim` must advance any number of days with zero `External` minds and no UI. Every system must have
at least one headless metric in the autonomy benchmark (see checklist §3). A system that only runs
when someone is controlled is not a world system.

**S2. The mind is the only difference. (MUST)**
`Person.mind` (`Ai` or `External`) may be read in exactly three places:
1. **Decision routing.** The DecisionPort answers now (AI) or queues a request with a deadline and a
   default (External).
2. **Recording level of detail.** Store full match reports and full event detail for teams involving an
   External mind. Recording never changes outcomes.
3. **The client and view layer.**

Any other read of `mind` in `pw-world`, `pw-sim` or `pw-match` is a defect. This is enforced by an
allowlist check (checklist §2).

**S3. One entity model. (MUST)**
The controlled person is an ordinary `Person` with the ordinary role records (`Player`, `Staff`, family
member…). There is no protagonist-only record type holding world facts. The career/client layer may hold
only:
- the HumanMind adapter (pending requests, the decision log),
- UI state (notes, pinned personal goals, screen preferences, read/unread flags),
- derived views computed from the world.

Family, partner, friends, agent, money, housing, education, health, contracts, relationships and
history live in the world, for everybody.

**S4. Taking or releasing control is a single field change. (MUST)**
`take_control(person)` sets `mind = External`. `release(person)` sets `mind = Ai`. Neither function
creates, deletes or edits any other world state. When the AI resumes, it continues from the person's
persisted personality, values and memories.

**S5. Symmetric agency. (MUST)**
The action set is the same for every mind. Anything a human can do (ask for a meeting, request a
transfer, change training focus, refuse a loan, hire an agent, move house, retire), an AI person can
also do. The AI chooses these actions from the same option generator through its utility evaluation.
This covers both directions:
- **Requests** arrive at a person (offers, summons, questions).
- **Intents** start from a person (asking, requesting, initiating).

## B. Decisions and behaviour

**S6. Decisions are generated from state, then evaluated. (MUST)**
A decision point is `(situation → option set → per-option consequences model)`. The option set is
built from the current state (rules, relationships, circumstances), not from a fixed menu. The AI mind
scores options with the same consequence model the world applies after a human chooses. No outcome is
hard-coded to an option ("Argue → trust −10"). The option starts an interaction that other simulated
people resolve with their own state and personality.

**S7. Considerations, not triggers. (MUST)**
Behaviour comes from utility over reusable *considerations*: small, pure functions of state such as
`form_trend`, `promise_outstanding`, `fixture_congestion`, `language_barrier` and `wage_vs_peers`.
- Personality and role supply the weights.
- Seeded noise adds variance.
- A single consideration must not be able to decide a choice on its own unless it is a hard rule (S9).

Adding depth usually means adding considerations and wiring them into existing decisions, not adding
events.

**S8. No stage or age triggers. (MUST)**
There is no code of the form `if age == N`, `if reputation > X then <story>`, or `if stage == Scholar`
that starts content. Ages and thresholds may appear only as:
- universal rules (S9), or
- inputs to considerations.

There is no `CareerStage` enum that gates features, unlocks screens or selects events.

**S9. Rules are data and explainable. (MUST)**
Universal rules (minimum professional age, contract length limits, registration, windows, quotas,
eligibility, discipline) live in data packs per nation or competition. They are evaluated by the rules
engine, which returns reasons (`RuleOutcome`). Constants such as `MIN_PRO_AGE` in code are temporary
debt.

**S10. Retirement and role changes are decisions or transitions, never endings. (MUST)**
- Retirement is a decision that every mind makes from the same considerations: body, offers, money,
  family, motivation.
- Forced exits (no club, medical ineligibility) are world outcomes that apply to anyone.
- After retirement the person persists and can enter the staff, agent, media or private-life job
  markets through the same systems as everyone else.
- The engine has no "game over" state.

## C. Causality and memory

**S11. Every state change is an event with causes. (MUST)**
Each world event carries:
- an id, date, actors and kind,
- a visibility (who can know),
- a `causes` list referencing earlier events or recorded facts (for example
  `training_rating_trend(p, 3w)`, `promise #812 broken`, `scout report #55`).

"Why did this happen?" must be answerable from data for anything the UI shows.

**S12. Memories are typed, sourced and consumed. (MUST)**
Relationships are not a single number.
- A relationship holds typed memories: kind, date, salience, the event that caused it, and whether it
  was public.
- Summary scores (affinity, trust, respect) are derived and cached. They are not the source of truth.
- Considerations read memories directly ("broke a promise to me in the last 12 months", "argued in
  public").
- Salience decays at rates that depend on the memory kind and the person's personality.
- Formative memories (a betrayal, a life-changing favour) may never fully decay.

**S13. Reputation has audiences. (MUST)**
Reputation is stored per audience, not as one or three numbers:
- sporting reputation by region or confederation,
- public image,
- per-club fan standing,
- insider (professional) reputation among managers, agents and scouts,
- media standing with individual outlets.

Each audience updates from the events it can see.

**S14. History is permanent and queryable. (SHOULD)**
Season stats, spells, transfers, injuries, honours, records, notable events and relationships are
archived for everyone. Compaction reduces granularity, not existence. Once in the archive, a fact can
be quoted by media, recalled by people and used by considerations years later.

## D. Knowledge and information flow

**S15. Knowledge is held by observers. (MUST)**
Truth lives in the world, and each observer (person or club department) holds beliefs with a source,
a date and a confidence.
- Information moves only through channels: witnessed, told (conversation), briefed (club internal),
  leaked, published, or inferred from public data.
- Each channel has a fidelity and a delay, and can distort.
- The client shows only the controlled person's beliefs plus public information.

**S16. Information has a supply chain. (MUST)**
Rumours, interest, reports and gossip are beliefs that travel between people. A published transfer
rumour requires all of the following:
- a real source event (a scout report, a bid, an agent pitch),
- a person who knew it and had a reason to share it (discretion, relationship, gain),
- a journalist who received it,
- an outlet whose accuracy and sensationalism shape what gets printed.

A false rumour is allowed only as a distortion of something real or as a deliberate plant by a
simulated person with a motive.

## E. Text

**S17. Text is a pure function of state. (MUST)**
Renderers take `(event or state, context facts, speaker or outlet, audience)` and return text. They
never write to the world. Every inbox message, article, post and conversation line references the
event or state it describes. Text that has no source event cannot exist.

**S18. Text gets its width from composition. (SHOULD)**
Text is built from:
- a template grammar keyed by event kind,
- *context facts* derived from history and state (first goal since injury, former club, record fee,
  derby, contract expiring, journalist's bias, fan standing),
- speaker voice (personality, role, outlet style).

This gives combinatorial variety without bespoke scripts per situation.

## F. People and life

**S19. Life is made of entities for everyone. (MUST)**
Family members, partners, friends, agents, journalists and advisers are `Person`s. They have
personality, age, location, occupation, languages, money, ambitions, relationships and memories. They
run the same needs → considerations → actions loop and have their own lives: jobs, moves, illness,
children, separations. A partner refusing to relocate is that partner's own decision, made from their
career, family, language, relationship and circumstances.

**S20. Level of detail may reduce resolution, never change rules. (MUST)**
The world is large (about 300k players), so life detail may be stored at lower resolution for people
nobody interacts with. Two conditions apply:
1. **Identical materialisation.** When detail is materialised (for example a player's parents become
   full records), it is produced by a function of `(world seed, person id)` that does not depend on who
   controls whom. Taking control must not change what exists.
2. **Same model, different granularity.** The low-resolution path must be the same model run at a coarser
   time step or with aggregated inputs. It must not be a separate "statistical AI life" with its own
   formulas. If a shortcut is unavoidable, it applies to a whole population (for example every player in
   an inactive league), never to one person because of their mind.

## G. Randomness and reproducibility

**S21. Keyed randomness, no global RNG. (MUST)**
Every random draw comes from a stream keyed by `(seed, system, entities, date, sequence)`. Parallel
work merges in id order. Outcome paths use deterministic math (`pw_core::math`).

**S22. Seeds for engineering, not destiny. (MUST)**
- The world seed is mixed with a **playthrough seed**, drawn fresh when a new game or takeover begins
  unless the user or a test pins it.
- One save plus the same decisions gives the same history (debugging, replays, bug reports).
- A new playthrough of the same starting world gives a different history.
- Decisions are logged, so a session can be replayed exactly.

**S23. Nothing is decided in advance unless the real world decides it that way. (SHOULD)**
Hidden ceilings such as potential are allowed as uncertain, evolving quantities, not fixed destinies.
Development, injuries, relationships and opportunities are resolved when they happen, from the state
at that moment.

## H. Boundaries and scale

**S24. The client never mutates the world. (MUST)**
The client reads view models and submits intents or answers through the DecisionPort. There are no
direct writes and no truth reads outside a debug build.

**S25. Crate boundaries enforce fairness. (MUST)**
`pw-world`, `pw-sim` and `pw-match` must not depend on the career or client crates. Only port types
cross the boundary.

**S26. Sparse, bounded state. (SHOULD)**
Relationships, beliefs and memories are stored only for pairs that have interacted, with salience-based
pruning. Memory stays bounded over 50 simulated seasons (soak test).

**S27. Uniform fidelity for outcomes. (MUST)**
All matches use the same outcome model, and so do all decisions and all development. Level of detail
controls what is recorded and how finely time is sliced for a whole population. It never controls who
gets the better model.
