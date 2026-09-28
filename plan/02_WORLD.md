# 02 — World Model: Nations, Competitions, Rules, Calendar

## 1. Geography & nations

| Field | Purpose |
|-------|---------|
| `confederation` | Continental competition access, international qualifying path |
| `football_tier` (1–5) | Talent generation density, league strength seed, youth coaching quality |
| `talent_profile` | Per-position talent bias (e.g. technical midfielders vs physical defenders), height distribution |
| `economy` | Wage levels, ticket prices, broadcast market size, tax regime ref, cost-of-living index |
| `languages[]` | Adaptation, media, relocation difficulty |
| `climate_zones[]` | Weather distributions by month (affects match engine and injury rates marginally) |
| `calendar_type` | Autumn–spring, spring–autumn, calendar-year, split-season (apertura/clausura) |
| `labour_rules_ref` | Minimum pro-contract age, max contract length, minimum wages, work-permit model |
| `youth_rules_ref` | Academy registration ages, school obligations, international transfer of minors (general prohibition + exceptions) |
| `citizenship_rules_ref` | Naturalisation years, dual nationality allowed, descent rules |
| `rivalries[]` | National and regional derbies |
| `culture` | Fan intensity, media aggressiveness, youth trust, tactical tradition |

Regions (sub-national) carry: population, talent density, rival cities, travel distance matrix (for fatigue and relocation).

## 2. Competition model

```
Competition
 ├── kind: League | Cup | SuperCup | Continental | International | Friendly | Youth | Reserve
 ├── tier, nation/confederation, gender_category (v1: men's; data model supports women's football later)
 ├── seasons[] → Stage[] (league phase, groups, knockout rounds, playoffs, relegation playoffs)
 ├── format: rounds, legs, seeding, draw rules (country protection, seeding pots)
 ├── tiebreakers: ordered list (points, GD, GF, H2H points, H2H GD, away goals, fair play, lots, playoff match)
 ├── match_rules: extra time, penalties, away-goals toggle, substitutions (count, windows, concussion subs), bench size
 ├── registration_rules: squad size, homegrown quotas, foreigner limits, U21 exemptions, list deadlines
 ├── discipline_rules: yellow accumulation thresholds, reset points, red card bans, competition-specific carry-over
 ├── prize_money / broadcast distribution model
 ├── qualification links: → continental slots, promotion/relegation, playoffs
 └── licensing: stadium capacity, finances, youth setup requirements (can block promotion)
```

### Supported formats (data-driven)

- Double round-robin; triple/quadruple round-robin; split league (top-half/bottom-half after N rounds, points carried or halved).
- Apertura/Clausura with combined table for relegation/continental spots.
- Group stage + knockout; Swiss-model league phase (N matches vs pot-seeded opponents) + knockout playoff.
- Single-leg knockouts with/without replays; two-legged ties; neutral-venue finals.
- Promotion playoffs (n-team ladder), relegation playoffs vs lower-tier teams, averages-based relegation (points-per-game over 3 seasons).
- Reserve/B-team leagues with "B team can't be promoted to the same tier as A team" rule.
- Youth competitions with age cut-off dates.

## 3. Rules engine (`world_rules`)

All rules are evaluated through a single engine with explainable results:

```rust
fn check(rule_set, context) -> RuleOutcome { allowed: bool, reasons: Vec<RuleReason>, remedies: Vec<Remedy> }
```

The UI can therefore always answer "why can't I play/sign/register?".

### Rule families

| Family | Examples |
|--------|----------|
| Registration | Squad list max N, min homegrown club-trained / nation-trained, foreign player cap, non-bloc cap, U21 unlimited, goalkeeper emergency registration, list edits only in windows |
| Eligibility (club) | Cup-tied players, loan players can't face parent club (toggle), max clubs registered per season (e.g. 3 registrations, play for 2), suspension carry-over |
| Eligibility (international) | Birth, parent, grandparent, residency years, one-time switch conditions (e.g. no competitive senior caps above threshold, age limit), naturalisation |
| Labour | Min professional age, max contract years (under 18: shorter), min wage, work permit (points-based: international caps %, fee, wage, league strength), EU-style free movement blocs |
| Minors | International moves under 18 generally prohibited with documented exceptions (family relocation for non-football reasons, bloc exceptions 16–18, cross-border proximity) — generic, configurable |
| Transfer windows | Summer/winter dates per nation, emergency GK signings, free-agent signing outside windows (toggle) |
| Discipline | Yellow thresholds (e.g. 5 → 1 match), second yellow = 1 match, straight red base bans, violent conduct extensions, appeal outcome probabilities, suspension served across competitions per rules |
| Sustainability (finance) | Squad-cost ratio caps, break-even rule over 3 seasons, sanctions: fines, registration embargo, points deduction |
| Licensing | Stadium capacity/lighting per tier, youth academy requirement, license denial blocks promotion |
| Integrity | Anti-doping testing (random selection per match, whereabouts obligation as a calendar duty), betting prohibition for participants — modeled only as rules and sanctions, no gameplay loop encourages breaking them |
| Match | Subs count & windows, concussion substitutes, bench size, kit clash, abandoned match policy (replay / result stands / forfeit) |
| Home-grown formation credit | Years at club between ages 15–21 (configurable) |

## 4. Calendar

- Global date; each competition season has start/end dates; world "season id" per nation calendar.
- International windows (FIFA-style): 4–5 per year, plus tournament summers.
- Winter breaks (nation-specific), summer off-season, pre-season tours, friendlies.
- Scheduling engine: fixture generator respecting stadium sharing, derby spacing, continental midweeks, TV slots (for news ordering + fatigue: kick-off time vs travel), congestion limits (min 2 full days between matches unless rules allow).
- Postponements: weather, stadium unavailability, international call-ups (some nations postpone when ≥N players called), cup replays; rescheduling algorithm fills free midweeks.
- Southern hemisphere and calendar-year leagues interleave correctly with international windows (e.g. a transfer from a spring–autumn league to an autumn–spring league mid-year is a key edge case: contract dates, registration windows and "season" statistics must split cleanly).

## 5. Stadiums & venues

| Field | Use |
|-------|-----|
| capacity, standing ratio | Revenue, atmosphere |
| pitch_type (grass/hybrid/artificial), pitch_quality | Match engine (passing error, injury rate small modifier) |
| dimensions | Small effect on tactics (wide play) |
| altitude | Fatigue modifier for visiting teams |
| roof/climate | Weather exposure |
| ownership (club/municipal/rented), shared_with | Finances, scheduling conflicts |
| expansion projects | Board decisions, multi-season construction, temporary capacity cuts |

## 6. Weather & climate

- Monthly distributions per climate zone: temperature, precipitation, wind, snow.
- Match-day weather sampled per venue per match.
- Effects (small, bounded): heat → faster fatigue accumulation; heavy rain → passing error ↑, long balls ↑; wind → crossing/long-pass variance; snow/frozen pitch → postponement probability; extreme heat → cooling breaks rule.

## 7. Governance & world evolution

- **Rule change events:** every season the governing bodies may change a rule (e.g. substitution count, foreigner limits, league expansion). Governed by data-pack "evolution tables" with probabilities, plus optional fixed timeline mode.
- **League restructuring:** expansion/contraction with transitional relegation counts; rebranding; new continental competitions.
- **Coefficients:** club and nation coefficients (5-season rolling) decide continental slots and seeding.
- **Club lifecycle:** foundation (phoenix clubs), mergers, bankruptcy → administration → points deduction → liquidation → reformation in a low tier (players become free agents; contracts voided per rules).

## 8. Economy model (world-level)

- Per-nation inflation index (wages, fees, ticket prices), default ~2–4%/yr with variance.
- Broadcast deal cycles (3-year renewals; big step changes create market booms).
- Currency: every nation has a currency; exchange rates drift slowly (random walk with mean reversion); contracts denominated in club currency; protagonist's personal finances convert on payment.
