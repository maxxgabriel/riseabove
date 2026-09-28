# 07 — Club, Manager, Staff & Board AI

## 1. People who run clubs

| Role | Decides |
|------|---------|
| Owner / Chairman | Budget, ambition, manager hiring/firing, facilities investment, sale of club |
| Board | Expectations, patience, style mandate ("play attacking football", "develop youth") |
| Director of Football (optional) | Recruitment strategy, contract policy, manager shortlist |
| Manager / Head Coach | Selection, tactics, training, squad status, transfer requests |
| Assistant manager | Delegated tasks (friendlies, press, renewals), opposition reports |
| Coaches (first team, youth, GK, set-piece) | Training effect, player assessments |
| Scouts / Chief scout | Player discovery, reports |
| Analysts | Opposition analysis, data (improves tactics, perception quality) |
| Medical & sports science | Health (05) |
| Head of Youth Development | Academy intake, youth philosophy |

All are Persons with attributes, personality, contracts and careers.

## 2. Manager model

Attributes: Tactical Knowledge, Man Management, Motivating, Discipline, Judging Ability, Judging Potential, Youth Trust, Adaptability, Determination, Media Handling, Attacking/Defending/Fitness/Technical/Mental coaching.

Philosophy profile (numeric, drives defaults):
- Preferred formations (ranked), style (possession ↔ direct, pressing high ↔ low block), rotation tendency, youth trust, loyalty to experienced players, favouritism towards former players, tolerance of indiscipline, preferred player archetypes (e.g. tall centre-backs), risk appetite, tactical flexibility.

Job-security state: board satisfaction, fan satisfaction, media pressure → warnings → sacking. Manager career: reputation, CV, job applications, nationality/language fit, availability.

## 3. Team selection AI (the heart of the protagonist's experience)

Deterministic core + seeded tie-breaking. Evaluated before every match:

```
for each candidate player p, for each slot s in chosen formation:
  score(p, s) =
      w_ability   * perceived_position_ability(p, s)       // from coach's perception, not truth
    + w_form      * form(p)                                // last 5 matches + training ratings
    + w_fitness   * fitness_readiness(p)                   // condition, sharpness, fatigue debt
    + w_role      * role_fit(p, role(s))                   // attribute weights × familiarity × traits
    + w_trust     * manager_trust(p)                       // relationship memory
    + w_status    * status_commitment(p)                   // promises, squad status, captain
    + w_tactic    * tactical_familiarity(p)
    + w_youth     * youth_policy_bonus(p)                  // youth trust × age × board mandate
    + w_opp       * opponent_specific_fit(p, s)            // e.g. aerial threat opponent → tall CB
    − w_rotation  * rotation_pressure(p, congestion)       // load management
    − w_risk      * card_or_injury_risk(p)                 // one yellow from suspension before derby
    + noise_seeded(σ ~ manager Consistency)
weights w_* = f(manager philosophy, match importance, competition)
assignment = max-weight bipartite matching (Hungarian) over players × slots,
             with constraints (registration, suspension, injuries, foreigners on pitch rule, GK exactly 1)
bench     = next best by coverage (GK backup, positional cover, impact sub profile)
```

This mirrors FM15's recursive best-XI search (reference lab) but uses an exact assignment solver for speed.

### Explainability

The selection result stores the top factors for each borderline decision. The protagonist's manager can *tell* him (in conversation or via coach) the dominant reason he was dropped: "form", "fitness", "tactical fit", "trust", "competition for places" — only what the manager is willing to share (Man Management / personality).

### Selection forecast (protagonist UI)

Runs the same selector with the manager's current perceptions a few days before the match; displays a probability band ("likely to start", "fighting for a place"). Uses no information the protagonist couldn't plausibly sense (coach feedback, training reports, recent line-ups). Uncertainty is added proportional to manager unpredictability.

## 4. Tactics AI

- Picks formation & style from philosophy × squad fit (best XI score across candidate formations) × opponent analysis.
- In-match adaptation (06 §8).
- Tactical familiarity grows with weeks using the same system; changing systems resets part of it (incentive for consistency).

## 5. Squad planning (season & multi-season)

Every club maintains a **Squad Plan** (updated monthly):

```
for each position group:
  depth_now, quality_now (perceived), age_profile, contract_expiry_profile,
  target_depth, target_quality (from ambition/budget),
  needs = [(position, profile, urgency, budget_band, age_band)]
```

Needs feed recruitment (08). Plan horizons: now (this window), next season, 3-year (succession for ageing stars, homegrown quota compliance).

Actions derived from the plan:
- Buy / loan in / promote youth / convert position / sell / loan out / release / renew / extend key players early.
- Wage structure discipline: tiers by squad status; avoid breaking structure unless star signing.

## 6. Youth policy

- Academy intake each year (open-football academy system): quality from youth facilities, youth recruitment rating, Head of Youth Development, nation talent density.
- Promotion decisions: youth players with perceived CA close to squad's lower bound and high perceived PA get U21/first-team exposure.
- Release policy: annual review (04 §6).

## 7. Loans (outgoing) logic

A player is loaned out when: perceived PA high, expected minutes low, age 17–23, and a loan club offers minutes at the right level. Loan club selection weighs: guaranteed minutes clause, league level vs player CA, coaching quality, style fit, distance/language, wage contribution.

Recall conditions: injury crisis at parent club, loanee not playing (clause), window rules.

## 8. Contract policy

- Renewal timing: key players 18–24 months before expiry; average players 12 months; ageing players year-by-year.
- Offer generation: wage band by status & market; bonuses; length by age & injury history; release clause policy by league norms.
- Let-go decisions: declining ability, wage/value mismatch, surplus.

## 9. Board & finances

- Budgets: wage budget, transfer budget (with instalment rules), facilities budget; set yearly, revised after windows or takeovers.
- Revenue: matchday (attendance × ticket price × demand), broadcast (league distribution model: equal share + merit + facility fees), commercial (reputation-driven, kit/sponsor cycles), prize money, player sales, owner injections.
- Costs: wages, amortisation (fee / contract years), staff, facilities upkeep, travel, youth, agent fees, loan fees, debt interest.
- Sustainability rules (02) → board constrains spending; sanctions possible.
- Ownership types & behaviours: member-owned (conservative, fan-sensitive), local benefactor (spiky spending), investment group (value-focused, sells high), state-backed (high budgets, reputation goals). Takeovers happen via ownership market events.

## 10. Board expectations & manager evaluation

Season objectives (league position band, cup progress, youth minutes, style, finances). Satisfaction updated weekly with patience by owner personality. Sacking triggers: sustained under-performance, relegation zone at key dates, fan protests, dressing room collapse. Replacement search: shortlist by style fit, reputation, availability, cost, nationality/language; caretaker phase.

Protagonist consequence: new manager = new perceptions (partially reset trust; knowledge transferred via assistant/staff notes), possibly new philosophy that suits him more or less.

## 11. Dressing room dynamics

- Relationships graph (players ↔ players, players ↔ staff): friendship, respect, rivalry, conflict; formed by nationality/language, age, shared time, personality compatibility, events (arguments, celebrating together, competition for position).
- Hierarchy: Team leaders (captain, vice, influential players) with influence scores; leaders shape reactions to manager decisions.
- Social groups (cliques). Group morale; unrest events (e.g. influential player unhappy → others' morale dips).
- Captaincy selection: manager choice among leaders (Leadership, seniority, reputation, language).

## 12. Staff AI

Hiring/firing staff by role gaps and budget; staff careers (move to bigger clubs, retire, ex-players become coaches). Staff attribute development with experience and badges.

## 13. National team AI

- National coach with same manager model; squad selection from all eligible players using perception (scouting coverage per nation varies — smaller nations know less).
- Call-up rules: window squad size, provisional lists, injury replacements, club release obligations.
- Youth national teams (U17, U19, U21, Olympic-style age-limited) with same logic.
- Eligibility checks via rules engine, including one-time switch; the national federation may *approach* dual-eligible players (DecisionPort request to them).
- Tournament management: qualifiers, final tournaments, squad 23–26 lists, pre-tournament friendlies, player fitness concerns with clubs.

## 14. AI personalities & variety guarantee

To avoid homogeneous worlds: sample manager/owner personalities from data-pack distributions per nation culture; enforce diversity metrics in 14_QA (not every club plays the same style or buys the same profile).
