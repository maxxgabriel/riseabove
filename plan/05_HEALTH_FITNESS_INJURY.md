# 05 — Health, Fitness, Load & Injury

## 1. Load model

Every training session and match adds **load** (arbitrary units, AU):

```
session_load = intensity(session_type) × duration × individual_multiplier
match_load   = minutes × (pressing_intensity × role_demand × weather_mod × altitude_mod)
```

Two rolling windows (acute:chronic workload):

- Acute load = 7-day sum; Chronic load = 28-day exponentially weighted average.
- **ACWR** = acute / chronic. Safe band 0.8–1.3. Above 1.5 → injury hazard multiplier up to ×2.5. Below 0.6 (undertrained) → also elevated risk on return + poor sharpness.

Sports-science staff quality determines how well clubs keep players in the safe band (AI managers with low Fitness knowledge ignore it more).

## 2. Condition, fatigue debt, sharpness

| Quantity | Behaviour |
|----------|-----------|
| Condition (0–100) | Drops during matches (stamina, role, intensity); recovers ~15–35/day depending on Natural Fitness, age, sleep, recovery sessions |
| Fatigue debt (0–100) | Accumulates when recovery < load over weeks; lowers max condition, raises injury risk, lowers performance; cleared by rest weeks |
| Match sharpness (0–100) | Rises with minutes, decays without; low sharpness = performance penalty and slight injury risk increase |
| Jet lag / travel fatigue | International travel across time zones: condition penalty for 1–3 days |

## 3. Injury model

### 3.1 Hazard

Per exposure event (sprint, tackle, landing, collision in match; high-intensity drills in training):

```
p_injury = base_rate(event_type)
         × proneness_mult(Injury Proneness)
         × load_mult(ACWR, fatigue_debt)
         × body_wear_mult(region)
         × age_mult
         × condition_mult
         × pitch_weather_mult
         × opponent_dirtiness_mult (contact events)
         × recent_return_mult (first 4 weeks after injury)
```

Calibration target: ~1.2–1.8 time-loss injuries per player per season at top level, match injury rate ~7–10× training rate per hour, hamstring/ankle/knee/groin dominate (14_QA references public epidemiology averages).

### 3.2 Injury catalogue (data pack)

Each injury type: body region, mechanism (contact/non-contact/overuse), severity distribution (days out: min/mode/max), recurrence risk window, permanent-effect table, surgery option, "play through" possibility.

Examples of categories: muscle strains (grades 1–3), ligament sprains, ligament ruptures, fractures, tendinopathies, contusions, concussion (special protocol), illness (viral, food poisoning), dental, cuts requiring stitches, overuse/stress reactions, back problems, groin/pubalgia.

### 3.3 Concussion protocol

- Head-impact events trigger assessment; concussion substitute rules apply (02).
- Mandatory graduated return (typically ≥ 6 days, configurable per rule profile).
- Repeated concussions raise long-term risk flags; medical staff may advise career changes (handled sensitively, see 10 §12).

## 4. Medical process

1. Injury occurs → immediate on-pitch decision: continue (if "play through" allowed & minor), substitute.
2. Diagnosis: medical staff quality affects accuracy of expected return date (UI shows range).
3. Treatment plan: conservative vs surgery (surgery: longer out, lower recurrence), specialist choice (club or player-funded).
4. Rehab phases: Rest → Rehab → Light training → Full training → Match fitness. Each phase has daily progress; setbacks possible.
5. Return decisions: manager can rush a player back (higher recurrence risk); the player can push to return early or ask for caution (DecisionPort).
6. Long-term effects: body wear increase; possible permanent attribute reduction; injury proneness drift.

## 5. Playing through pain

- Minor knocks: player may play at reduced condition; risk of aggravation.
- Painkiller/injection decisions are abstracted as "Play through (risk ↑)" vs "Rest" choices; no medical detail beyond that.

## 6. Illness, mental fatigue & well-being

- Seasonal illness probabilities (winter), squad outbreaks.
- Mental fatigue from stress (10) reduces Concentration/Decisions effectiveness in matches (bounded), raises burnout risk.
- Burnout state: long-term motivation drop; needs rest/support; can be prevented by balance.

## 7. Body wear & career length

Per body region wear 0–100 increases with load and injuries; recovers slowly with rest and good care. High wear accelerates physical decline and raises injury risk. This is how a player's choices (rushing back, overtraining, poor recovery) shorten careers realistically — and how good care extends them.

## 8. Pre-season & off-season

- Off-season: fitness decays; players with high Professionalism keep fitness (individual programme); poor lifestyle → return unfit.
- Pre-season: high loads; injury risk band; friendlies build sharpness.

## 9. Medical staff & facilities

| Staff | Effect |
|-------|--------|
| Head of medical / physios (Physiotherapy attr) | Rehab speed, diagnosis accuracy |
| Sports scientists (Sports Science attr) | ACWR management, injury prevention (−10…−25% hazard) |
| Fitness coaches | Condition recovery, fitness growth |
| Psychologist (optional role) | Stress/burnout reduction |

## 10. Protagonist-facing health UI

- Body map with regions coloured by wear/injury history (perceived; not exact numbers).
- Load chart (acute vs chronic) — only if club has sports-science staff that shares data, otherwise vague "feeling tired" signals.
- Rehab timeline with phases and expected return window.
- Choices: rest request, extra recovery, second opinion (costs money), return-timing preference.
