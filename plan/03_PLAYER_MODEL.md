# 03 — Player Model

Everything about a footballer as an entity. Same model for every player, protagonist included.

## 1. Identity

| Field | Notes |
|-------|-------|
| id, names (first, last, common/known-as, nickname list) | Nicknames can be earned in-game (media system) |
| date_of_birth, place_of_birth (city, region, nation) | Birth nation drives eligibility |
| nationalities[] with acquisition type (birth, descent, residency, naturalisation) + date | Needed for switching rules |
| languages[] with proficiency 0–100 | Adaptation, captaincy, media |
| height, weight, body_type, preferred_foot, weak_foot 1–20 | Body model (§6) |
| hometown_club_affinity, favourite_club | Transfer preferences, fan reactions |
| family_background (for AI: summary; protagonist: full in 10_LIFE_SIM) | Relocation willingness, personality seeds |

## 2. Visible attributes (1–20 scale)

Base: open-football's attribute groups, extended to a full FM-style set.

### Technical (outfield)
Corners, Crossing, Dribbling, Finishing, First Touch, Free Kicks, Heading, Long Shots, Long Throws, Marking, Passing, Penalty Taking, Tackling, Technique.

### Mental
Aggression, Anticipation, Bravery, Composure, Concentration, Decisions, Determination, Flair, Leadership, Off the Ball, Positioning, Teamwork, Vision, Work Rate.

### Physical
Acceleration, Agility, Balance, Jumping Reach, Natural Fitness, Pace, Stamina, Strength.

### Goalkeeping (GK only; outfield have hidden low values)
Aerial Reach, Command of Area, Communication, Eccentricity, Handling, Kicking, One on Ones, Reflexes, Rushing Out, Throwing, Passing (shared), First Touch (shared).

Each attribute stored as fixed-point (e.g. 1.00–20.00) internally; UI shows integers or perceived ranges.

## 3. Hidden attributes (1–20)

| Attribute | Effect |
|-----------|--------|
| Consistency | Variance of match-to-match performance |
| Important Matches | Performance modifier in finals/derbies/decisive games |
| Injury Proneness | Base injury hazard multiplier |
| Versatility | Speed of learning new positions |
| Adaptability | Speed of settling in new country/culture |
| Ambition | Desire for bigger clubs/wages/trophies |
| Loyalty | Resistance to leaving, renewal willingness |
| Pressure | Handling media/fan pressure |
| Professionalism | Training quality, lifestyle discipline, development efficiency |
| Sportsmanship | Fouls, simulation tendency, disciplinary risk |
| Temperament | Reaction to provocation, red-card risk, dressing-room conflict |
| Controversy | Likelihood of media incidents |
| Dirtiness | Foul type severity |

Personality labels (e.g. "Model Professional", "Driven", "Temperamental") are *derived* from hidden attributes, and shown to the protagonist only as perceived (coach/agent descriptions).

## 4. Traits (player preferred moves)

Discrete tags learned via training/mentoring or natural: *Cuts inside from wing, Tries tricks, Shoots from distance, Stays back at all times, Runs with ball often, Plays one-twos, Dives into tackles, Curls ball, Places shots, Tries to beat offside trap, Likes to lob keeper, Knocks ball past opponent, Comes deep to get ball, Gets into opposition area, Hugs line, Dwells on ball, Stops play, Arrives late in box, Moves into channels, Penalty box player, Plays with back to goal, Tries killer balls often, Marks opponent tightly, Uses outside of foot, Long throw specialist*...

Each trait modifies action selection weights in the match engine (06) and can conflict with manager instructions (morale effect if repeatedly overridden).

## 5. Positions & roles

- Position grid: GK, DR/DC/DL, WBR/WBL, DM, MR/MC/ML, AMR/AMC/AML, ST (14 slots).
- `position_familiarity[slot]` 1–20 (Natural ≥ 18, Accomplished 15–17, Competent 12–14, Unconvincing 8–11, Awkward 5–7, Ineffectual < 5).
- Role familiarity (per role/duty, e.g. Inverted Winger (Support), Ball-Playing Defender, Deep-Lying Playmaker, Box-to-Box, Pressing Forward, Target Man, False Nine, Sweeper Keeper, Anchor, Mezzala, Wing-Back, Inverted Full-Back, Shadow Striker...): derived score from attribute weights × position familiarity × traits.
- Learning new positions: training focus + match minutes in that slot + Versatility → familiarity growth.

## 6. Body model

| Component | Detail |
|-----------|--------|
| Height/weight | Growth curve through adolescence (growth spurt window ~12–16, sampled) |
| Body composition | Body fat %, muscle mass index; influenced by training, diet, lifestyle |
| Biological age vs chronological | Early/late maturers; affects youth performance vs long-term development (key for "late bloomer" realism) |
| Joint/tissue health | Per body region wear score (knee L/R, ankle L/R, hamstring L/R, groin, back, shoulder...) — accumulates from load and injury history (05) |
| Natural fitness | Recovery rate, decline age modifier |

## 7. Ability model

- **CA (1–200):** weighted sum of attributes, weights per position (data pack), matching FM-style CA semantics. For a player with multiple positions, CA is computed for the best position; "position CA" computed per slot for selection.
- **PA (1–200 or negative ranges for generated youths):** the ceiling. PA is *hidden from everyone*; clubs estimate it through `PotentialEstimator` (perception).
- **Headroom:** `PA − CA` determines development room; attributes cannot push CA above PA (except small, rare over-performance mechanism for late bloomers: PA itself can be *re-rolled once* between 17–21 based on biological maturity + development environment, bounded ±15, seeded).
- **Attribute caps by body:** physical attributes bounded by body model (a 1.68 m player's Jumping Reach has a lower practical cap).

## 8. Dynamic state

| State | Range | Updated |
|-------|-------|---------|
| Condition (short-term energy) | 0–100 | Daily, per match minute |
| Match sharpness | 0–100 | Minutes played recently vs training |
| Fitness (aerobic base) | 0–100 | Weekly, training load & rest |
| Form | rolling rating average (last 5/10 matches) | Post-match |
| Morale | 0–100 with components | Daily events |
| Confidence | 0–100 (separate from morale: on-pitch belief) | Matches, errors, praise |
| Fatigue debt (accumulated) | 0–100 | Load vs recovery (05) |
| Stress | 0–100 | Life & career pressure (10) |
| Happiness components | playing time, role, wage fairness, team success, manager relationship, city/life, family, promises | Weekly |

### Morale composition

```
morale = clamp( base_personality
             + w_play*playing_time_satisfaction
             + w_role*squad_status_match
             + w_wage*wage_fairness_vs_peers
             + w_team*recent_results
             + w_mgr*manager_relationship
             + w_life*life_satisfaction          // from life sim (AI: statistical)
             + w_prom*promise_state
             + events_decay_sum, 0, 100)
```
Weights come from personality (e.g. high Ambition → higher w_team & w_wage; high Loyalty → higher w_mgr, lower w_wage).

## 9. Squad status & expectations

Statuses: Star Player, Important Player, Regular Starter, Squad Player, Impact Sub, Fringe Player, Emergency Backup, Youngster, Not Needed.

Each status sets expectations: share of minutes, big-match starts, wage bracket. The gap between expected and actual drives playing-time satisfaction. The manager assigns status (07) — the protagonist can negotiate it in contracts.

## 10. Reputation (per player)

- `current_reputation`, `home_reputation` (nation), `world_reputation` (0–10,000).
- Regional reputation map (reputation per confederation) — a star in one continent can be unknown in another.
- Derived by media coverage, competition prestige, performance, honours, international appearances; decays slowly without exposure.

## 11. Perception layer

Every observer (coach, scout, DoF, agent, journalist, fan community, protagonist himself) holds `Perception<Subject>`:

```rust
struct AttributePerception {
    estimate: [Fixed; N_ATTR],   // point estimate
    uncertainty: [Fixed; N_ATTR],// ± range
    observed_minutes: u32,       // evidence accumulated
    contexts: ContextMix,        // league level, position observed
    last_updated: Date,
    bias: Bias,                  // e.g. nationality bias, height bias, "flair" bias from observer personality
}
```

- Uncertainty shrinks with observation time, observer judging skill (Judging Ability / Judging Potential), and match context quality.
- Knowledge decays over time without observation.
- The protagonist's **self-perception** comes from coach feedback, performance data and his own "self-awareness" (Professionalism + experience); displayed as ranges in UI with a confidence meter.

## 12. Career memory (per player, compact)

- Clubs & seasons, appearances, minutes, goals, assists, average rating, trophies, awards, caps.
- Relationship ledger (key persons & sentiment).
- Injury history (feeds body model).
- "Promises made to me" ledger with outcomes.
- Notable moments (hat-tricks, debuts, finals, red cards) for media and legacy.
