# 06 — Match Engine (event-chain model)

Goal: every match in the world produces believable results and **per-player data** (minutes, position, role, rating, goals, assists, xG, key passes, tackles, duels, cards, injuries, substitutions) and a readable story, cheaply (target ≤ 3 ms per match, allocation-free). No physics, no 3D.

## 1. Model overview

The pitch is a grid of **zones** (default 6 lengthwise × 5 widthwise = 30 zones, plus the two penalty boxes subdivided for finishing). The match is a sequence of **possession chains**. Each chain is a sequence of **actions** by specific players, moving the ball between zones until the chain ends (shot, turnover, foul, out of play, offside, set piece).

```
match
 └─ phases (kick-off, open play, set pieces, stoppages, half-time, subs, extra time, penalties)
     └─ possession chain
         └─ action: (actor, action_type, from_zone, to_zone, target_player, pressure, outcome)
```

Time advances per action (sampled durations, e.g. short pass 2–4 s, carry 3–8 s, stoppages 20–90 s). Ball-in-play time calibrated ~55–60 minutes per 90.

## 2. Inputs

| Input | Source |
|-------|--------|
| Line-ups, formation, roles, duties | Club AI (07) or opponent AI |
| Team instructions | mentality, width, tempo, passing directness, defensive line, line of engagement, pressing intensity, counter-press, time wasting, set-piece routines, marking scheme |
| Player instructions | shoot more/less, dribble more/less, cross from byline/early, mark specific player, press more, stay wider… |
| Player state | attributes, traits, condition, sharpness, morale, confidence, form, familiarity with position/role and tactic |
| Context | home advantage, crowd, weather, pitch, referee, importance, derby, rivalry, fatigue from travel |
| Team cohesion | tactical familiarity (weeks with same system), partnerships (players who played together often) |

## 3. Action selection

At each step the ball carrier chooses an action from a candidate set:

```
candidates = {short_pass(target), long_pass(target), through_ball(target), cross(zone),
              carry(zone), dribble(opponent), shoot(type), hold_up, clear, switch_play, back_pass}

utility(a) = expected_value(a | zone, pressure, teammates' positions)      // xT / xG model
           × tactic_weight(a, team_instructions, role)
           × trait_weight(a, player.traits)
           × personality_weight(a, Flair, Decisions)
decision_quality = f(Decisions, Vision, Composure, pressure, condition)
chosen = softmax(utility / temperature(decision_quality))                    // better deciders pick best more often
```

- **Positions of teammates/opponents** are not simulated continuously; each player has a *zone occupancy distribution* derived from formation, role, phase (in/out of possession), and ball zone. Sampling from it gives who is available/pressuring.
- **Pressure** on carrier = f(opponent pressing, defender Work Rate/Anticipation/Pace, line height).

## 4. Action resolution (attribute contests)

Each action resolves as a contest with a logistic model:

```
p_success = σ( k * (attacker_skill − defender_skill) + context_bias )
attacker_skill = Σ attr weights (e.g. pass: Passing, Technique, Vision, Composure; condition-adjusted)
defender_skill = Σ attr weights of relevant defender(s) (Anticipation, Positioning, Marking, Tackling…)
```

Outcomes include partial success (misplaced but retained, deflection → corner, foul won, card). Fouls come from tackle attempts: foul probability = f(Tackling, Aggression, Dirtiness, fatigue, referee strictness).

## 5. Shooting & goals

- xG model by shot location (zone + sub-zone), angle proxy, body part (foot/head), assist type (through ball, cutback, cross), pressure, shot type (placed, powered, chip, volley), and goalkeeper position.
- Shot outcome: blocked / off target / on target → save vs goal. Save probability = f(GK Reflexes, Handling, One on Ones, Positioning, Aerial Reach for crosses, shot quality).
- Rebounds create second-chance chains.
- Calibration: ~2.6–2.9 goals/match top-flight average, home win ~44–46%, draw ~25–27%, conversion ~10–11% per shot, 0-0 ~7–8% (14_QA).

## 6. Set pieces

Corners, free kicks (direct/indirect), penalties, throw-ins (long throws), goal kicks. Routines from set-piece coach instructions; aerial duels use Jumping Reach, Heading, Strength, Bravery, marking assignment. Penalty: taker Penalty Taking + Composure + pressure vs GK; penalty shootouts with order selection and pressure escalation.

## 7. Referee model

Referees are persons with attributes: Strictness, Consistency, Advantage tendency, Home bias (tiny), Fitness. VAR availability per competition (rules). Effects: foul/card thresholds, penalty decisions, added time. Controversial decisions generate media events.

## 8. Fatigue, subs, injuries, cards inside the match

- Condition drains per action by intensity; high-pressing roles drain faster; players below thresholds lose attribute effectiveness.
- Manager AI (07) makes substitutions: tactical (losing/winning), fatigue, injury, card risk, time-wasting; human-like timing distributions (55', 65', 75', 85').
- Injuries rolled per exposure (05); injured player may continue at reduced effectiveness or be subbed.
- Cards: yellow/second yellow/red, with accumulation fed to discipline rules.
- Tactical changes mid-game: mentality shifts by score/time/personality.

## 9. Momentum & psychology (bounded)

- Momentum score per team rises with chances/goals, falls after conceding; modifies pressing and confidence slightly (±3–5% effective).
- Important Matches, Pressure and Composure affect big-game performance; crowd hostility affects away players with low Pressure.

## 10. Ratings

Per-player rating 1.0–10.0 from **action value added**:

```
rating = 6.0 + scale * (Σ ΔxT_positive + Σ ΔxG_contributions + defensive_value(tackles, interceptions, blocks, aerials won)
                        − Σ turnover_costs − errors_leading_to_shot/goal − card_penalties) / minutes_normalizer
       + role_adjustment + result_bonus(small)
```

Calibrated so the league-average rating ≈ 6.7–6.9, standard deviation ≈ 0.6, 9+ ratings rare. Goalkeeper ratings from goals prevented (PSxG − goals conceded) + distribution + claims.

## 11. Output contract (per match)

```
MatchResult {
  ids, competition, stage, date, venue, attendance, weather, referee,
  score (ht, ft, et, pens), scorers/assists (minute, type, body part, xG),
  team_stats { possession, shots, sot, xG, corners, fouls, offsides, passes, pass%, PPDA, big chances },
  player_stats[] {
    player_id, team, started, minutes, positions_played[(slot, from, to)], role,
    rating, goals, assists, xG, xA, shots, sot, key_passes, passes, pass%, progressive_passes,
    carries, dribbles_att/succ, crosses, tackles, interceptions, clearances, blocks,
    aerials_won/lost, duels_won/lost, fouls_committed/drawn, offsides, yellow, red,
    errors_to_shot/goal, saves, goals_conceded, psxg_faced, distance_proxy, sprints_proxy,
    condition_start/end, injury (if any), heatmap zone counts (30 zones)
  },
  events[] (minute-ordered; recorded fully for protagonist's matches & archival matches),
  commentary_seeds[] (template ids + params for text generation),
  player_of_the_match
}
```

## 12. Levels of recording (not outcome)

| LOD | Recorded |
|-----|----------|
| Full | All events, heatmaps, commentary seeds (protagonist's team, his national team, matches he watches, top-flight archival) |
| Standard | Player stats, key events (goals, cards, subs, injuries) |
| Aggregate | Score, scorers, cards, injuries, minutes (compacted later) |

Outcome model is identical across LODs (P1).

## 13. Presentation (see 12_UI_UX)

- Text commentary generated from templates (with variety pools, player nicknames, context awareness: "his first goal since his injury").
- 2D radar: dots placed by sampling each player's zone distribution + action positions (visual approximation, not physics).
- Key-highlights mode: only chains with xG ≥ 0.08 or cards/injuries/goals.
- Protagonist cam: filter to actions involving the protagonist + running tally.

## 14. Optional "Player Moments" mode

When enabled, at selected chains where the protagonist is the ball carrier or key defender, the UI pauses and offers 2–4 options (e.g. shoot / pass to runner / carry). The chosen option replaces the protagonist's `AiMind` action choice; **resolution uses the identical contest model**. Frequency limited (≤ 6 per match). Fairness: option set equals the candidate set the AI would have considered; expected value of human choices is tracked in QA to ensure no systematic exploitation beyond "good decision-making" (which a high-Decisions AI player also achieves). Players can disable this mode entirely.

## 15. Engine validation

- Distribution tests (goals, shots, cards, possession, ratings) per league strength tier.
- Tactical sanity tests: higher pressing → more high turnovers & more fatigue; wide play → more crosses; low block → fewer shots conceded but lower xG created.
- Attribute sensitivity tests: +2 Finishing on a striker → measurable xG over/underperformance shift.
- Mismatch tests: CA gap → win probability curve matching real-world odds models.
