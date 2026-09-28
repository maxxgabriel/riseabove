# 17 — Formulas & Tuning Appendix

All constants are defaults in data packs (`data/core/*.toml`); values are starting points for calibration, not final.

## 1. Current Ability

```
CA_pos(p, pos) = clamp( Σ_a  W[pos][a] · attr_scaled(a) , 1, 200 )
attr_scaled(a) = (attr(a) − 1) / 19          // 0..1
W[pos][*]      normalised so a player with all 20s = 200
CA(p)          = max_pos CA_pos(p, pos) weighted by familiarity(pos) ≥ Accomplished
```

Example position weights (ST): Finishing 0.14, Off the Ball 0.09, Composure 0.08, First Touch 0.07, Anticipation 0.07, Pace 0.07, Acceleration 0.07, Dribbling 0.05, Technique 0.05, Heading 0.05, Strength 0.04, Decisions 0.04, Balance 0.03, Agility 0.03, others share remainder.

## 2. Development step (weekly)

```
room      = max(0, PA − CA) / 200
age_f     = AgeCurve(group, age + bio_offset)            // −0.6 .. 1.0
train_f   = 0.4 + 0.6 · coaching_mult · facility_mult · session_share(group)
match_f   = min(1.2, minutes_last_4w / 360) · level_fit · (0.8 + 0.04·(rating−6))
wellness  = 0.8 + 0.2 · wellbeing
prof_mult = 0.7 + 0.03 · Professionalism + 0.015 · Determination
Δattr     = G · room · max(age_f,0) · (train_f + match_f) · wellness · prof_mult
          + min(age_f,0) · D · decline_scale(natural_fitness, body_wear)
          + N(0, σ=0.02)
weekly cap: |Δattr| ≤ 0.12 (≤ 0.20 age ≤ 18)
G = 0.085, D = 0.06
```

## 3. Injury hazard

```
p = base(event) · (0.6 + 0.04·InjuryProneness) · acwr_mult · (1 + 0.8·fatigue_debt/100)
    · (1 + 0.6·wear_region/100) · age_mult(age) · (1.3 if condition < 60) · env_mult
acwr_mult = 1.0 for 0.8–1.3; 1 + 1.5·(acwr−1.3) above; 1 + 0.8·(0.8−acwr) below
age_mult  = 1.0 at 22–28, +3%/yr beyond 28, +2%/yr below 20
```

## 4. Selection score weights (defaults by manager archetype)

| Weight | Pragmatist | Developer | Rotator | Loyalist |
|--------|-----------|-----------|---------|----------|
| ability | 0.40 | 0.30 | 0.35 | 0.35 |
| form | 0.20 | 0.15 | 0.20 | 0.10 |
| fitness | 0.15 | 0.15 | 0.20 | 0.10 |
| role_fit | 0.10 | 0.10 | 0.10 | 0.10 |
| trust | 0.05 | 0.05 | 0.05 | 0.20 |
| youth | 0.00 | 0.15 | 0.03 | 0.00 |
| rotation penalty | 0.05 | 0.05 | 0.15 | 0.02 |

## 5. Market value

```
base = 1e6 · exp(0.042 · (CA − 100)) · potential_bonus
potential_bonus = 1 + max(0, PA_est − CA) / 100 · youth_factor(age)     // youth_factor 1.0 at ≤ 21 → 0 at 27
value = base · pos_mult · rep_mult · league_mult · contract_mult · form_mult · injury_mult · inflation
contract_mult = 0.35 + 0.65 · min(1, years_left / 3)
```

## 6. Player move utility weights (by dominant personality)

| Weight | Ambitious | Loyal | Money-driven | Family-first |
|--------|-----------|-------|--------------|--------------|
| level | 0.30 | 0.15 | 0.15 | 0.15 |
| playing time | 0.20 | 0.20 | 0.10 | 0.20 |
| money | 0.15 | 0.10 | 0.40 | 0.15 |
| trophies | 0.20 | 0.10 | 0.10 | 0.05 |
| loyalty (stay) | 0.05 | 0.30 | 0.05 | 0.10 |
| life fit | 0.10 | 0.15 | 0.20 | 0.35 |

## 7. Rating model

```
value_added = Σ ΔxT(actions) + 0.9·xG_created_as_shooter_beyond_expectation + 0.8·xA
            + 0.015·(tackles_won + interceptions + blocks) + 0.01·aerials_won
            − 0.02·turnovers_in_own_half − 0.25·errors_to_goal − 0.3·red − 0.03·yellow
rating = clamp(6.0 + 2.2 · value_added · (90 / max(minutes, 20)) ^ 0.5 + 0.2·result_sign, 3.0, 10.0)
```

## 8. Reputation update (weekly)

```
rep_target = f(CA, competition_prestige_weighted_minutes, honours, caps, media_exposure)
rep += α · (rep_target − rep)   with α = 0.03 (0.08 during tournaments), decay 0.2%/week without exposure
```

## 9. Well-being effect bounds

| Effect | Range |
|--------|-------|
| Recovery rate | ±10% |
| Match Concentration/Decisions effectiveness | ±3% |
| Training rating | ±0.3 |
| Injury hazard | ±10% |

## 10. Perception uncertainty

```
σ_attr = σ0 · (1 − min(0.9, observed_minutes / 3000)) · (1.3 − 0.03 · JudgingAbility) + decay · weeks_since_observed
σ0 = 4.0 (on 1–20 scale), decay = 0.05/week
```
