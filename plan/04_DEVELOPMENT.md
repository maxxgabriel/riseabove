# 04 — Development & Training

## 1. Principles

1. Development is the product of **inputs** (training quality, match experience, coaching, facilities, professionalism, health) acting on **capacity** (age, PA headroom, biological maturity).
2. No input is free: more training load raises injury risk and fatigue (05).
3. The protagonist uses exactly the same functions; he only chooses inputs (focus, extra sessions, lifestyle) that AI players choose via their minds.

## 2. Weekly development step (every player)

```
for each attribute a:
  potential_room = f_room(PA − CA, attribute_weight_in_position, body_cap(a))
  age_factor     = AgeCurve[a.group](age, biological_age_offset)       // growth/plateau/decline
  inputs         = training_effect(a) + match_effect(a) + mentoring_effect(a) + environment_effect
  wellness       = g(fitness, fatigue_debt, injury_state, stress, sleep_quality)
  delta_a        = potential_room * age_factor * inputs * wellness * professionalism_mult
                   + decline_term(a, age, natural_fitness, body_wear)
                   + noise(seeded, σ small)
  attribute[a]  += delta_a   (then renormalise so CA ≤ PA)
```

### Age curves (default data pack, tunable)

| Group | Growth window | Peak plateau | Decline starts | Notes |
|-------|--------------|--------------|----------------|-------|
| Physical (pace, acceleration, agility) | 14–22 | 22–27 | 27–29 | Decline accelerated by body wear & low natural fitness |
| Physical (strength, stamina, jumping) | 15–24 | 24–30 | 30–32 | |
| Technical | 13–25 | 25–31 | 31–33 | Slow decline |
| Mental | 15–29 | 29–34 | 34+ | Anticipation/Decisions/Positioning keep growing into 30s |
| Goalkeeping | 16–28 | 28–34 | 34–36 | Keepers peak later |

Late bloomers: biological_age_offset (−2…+2 years) shifts growth windows; a late maturer underperforms at 16 and overtakes at 20–22. This is the source of realistic "released at 16, star at 24" stories.

## 3. Training system

### 3.1 Team training (club-owned, manager/coach decided)

Weekly schedule grid: 7 days × 3 slots (morning/afternoon/extra). Session types:

| Session | Primary attributes | Load |
|---------|-------------------|------|
| Physical conditioning | Stamina, Natural Fitness, Strength | High |
| Speed & agility | Pace, Acceleration, Agility, Balance | High |
| Technical drills | First Touch, Passing, Technique, Dribbling | Medium |
| Finishing | Finishing, Composure, Long Shots | Medium |
| Defensive shape | Positioning, Marking, Tackling, Concentration | Medium |
| Attacking movement | Off the Ball, Anticipation, Vision | Medium |
| Set pieces | Corners, Free Kicks, Heading, Marking | Low |
| Tactical (match prep) | Decisions, Teamwork, tactical familiarity | Low |
| Match practice | All, sharpness | High |
| Recovery | Condition, fatigue debt ↓ | Negative |
| Rest | Condition, stress ↓ | None |
| GK sessions | GK attributes | Medium |

Manager personality determines the default schedule (07). Periodisation: pre-season heavy, match weeks lighter, recovery after matches (MD-1/MD+1 logic).

### 3.2 Individual training

- **Focus** (one attribute cluster or a position/role/trait): +x% development in that area, −y% elsewhere (zero-sum-ish).
- **Additional sessions** (protagonist and AI with high Professionalism/Determination): extra load and extra development; subject to club rules (some clubs restrict extra work; sports science staff can veto if fatigue high).
- **Trait learning:** requires coach with relevant skill; 4–12 weeks; can fail; can be *unlearned*.
- **Position retraining:** learning curve by Versatility.

### 3.3 Training performance

Each session produces a per-player **training rating** (5–10) from: attributes involved, condition, professionalism, mood, random. Coaches observe it → feeds selection (07) and perception (03). This is a key protagonist lever: training well visibly matters, but only through the coach's perception.

### 3.4 Coaching quality

Coach attributes (1–20): Attacking, Defending, Fitness, Mental, Tactical, Technical, Goalkeeping (shot stopping, distribution, handling), Working with Youngsters, Motivating, Discipline, Determination, Man Management, Level of Discipline, Adaptability.

`coaching_effect(session) = mean(top-k coaches assigned to that area weighted by workload) → 0.6…1.4 multiplier`. Overloaded coaches (too many areas) lose effectiveness.

### 3.5 Facilities

Training facilities (1–20), youth facilities (1–20), medical/sports-science (1–20), data/analysis (1–20). Multipliers on development (0.85…1.15), injury rehab speed, recovery. Upgraded by board decisions (multi-season projects).

## 4. Match experience

`match_effect` depends on: minutes, competition level relative to player CA (playing just above your level = best), rating, position played. Senior minutes as a teenager are highly valuable; sitting on a top club's bench is poor for development (this drives realistic loan logic in 07/08).

| Context | Relative development value (per 90) |
|---------|------------------------------------|
| Senior match at a level ≥ player CA | 1.00 |
| Senior match far below CA | 0.45 |
| Reserve/U21 match | 0.35–0.55 |
| Youth match (age-appropriate) | 0.30–0.50 |
| Bench without minutes | 0.02 |
| Training only | via training_effect |

## 5. Mentoring & dressing room influence

- Mentoring groups (manager/coach assigned or player-chosen): an experienced player with high Professionalism/Determination can transfer personality traits (Professionalism, Determination, Ambition — hidden) and on-pitch traits, slowly, if relationship is good.
- Negative influence exists: poor professionals in a friendship group can lower a youngster's Professionalism.
- Protagonist can *seek* a mentor (request) and later *be* a mentor.

## 6. Youth development stages

| Age band | Environment | Key systems |
|----------|-------------|-------------|
| 8–11 | Grassroots / pre-academy | Fun, multi-sport, school; talent ID by local scouts |
| 12–15 | Academy (foundation/youth phase) | Short training sessions, education priority, growth spurts, release decisions each year |
| 16–18 | Scholarship / youth contract | Full-time football + education, U18 league, first pro contract decision |
| 18–21 | Professional development | U21/B team, loans, senior debut window |
| 21–23 | Last development window | Breakthrough or step-down |

Academy retention decisions: each season academies evaluate players (perceived CA/PA + attitude + biological maturity) and release a fraction (realistic: most academy players are released). Released players enter the grassroots/semi-pro pool where they can still be found.

## 7. Regression & decline

- Decline term driven by age, Natural Fitness, body wear (05), and lifestyle (10) — bounded modifiers.
- Serious injuries can permanently reduce specific physical attributes (e.g. cruciate ligament → Acceleration/Agility −0…3), scaled by rehab quality and age.
- Long inactivity (no club) reduces sharpness quickly and fitness slowly; attributes only erode after months.

## 8. Coach/club development philosophy (AI side)

- Club "youth trust" level (07) → minutes allocated to young players.
- Loan strategy (08) → development loans chosen by level fit, playing-time guarantee and loan club's coaching/facilities.

## 9. Protagonist-facing development tools (UI in 12)

- Development report every 4 weeks from coach: perceived strengths/weaknesses, focus recommendation, attitude comment.
- Attribute history chart (perceived ranges tighten as knowledge grows).
- "Talk to coach" interaction: ask for focus change, extra sessions, position change, mentor.
- Personal trainer / nutritionist (paid, off-club; 10_LIFE_SIM): small bounded effects, clubs may object if it conflicts with their programme.

## 10. Guardrails

- Hard cap on weekly attribute growth (prevents exploits from stacking all inputs).
- Diminishing returns on extra training; overtraining syndrome risk (05).
- No development input exists only for the protagonist; every input he can choose, AI players can choose via personality-driven minds.
