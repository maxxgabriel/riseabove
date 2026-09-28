# 08 — Market: Transfers, Loans, Contracts, Agents

## 1. Full transfer pipeline (every club, every player)

```
Need (squad plan 07§5)
 → Discovery   : scouting network + agent pitches + media buzz + past observations
 → Evaluation  : perceived ability/potential, fit to role, age, wage demand estimate, fee estimate, character reports
 → Shortlist   : ranked by value-for-need within budget
 → Approach    : informal enquiry to club (and/or agent) — can leak to media
 → Club negotiation : bid → accept/reject/counter (multi-round, rebid cooldown)
 → Player negotiation: personal terms via agent (wage, length, bonuses, clauses, status, promises)
 → Medical     : health check (05) — can fail or renegotiate
 → Registration: rules engine check (window, quotas, work permit)
 → Completion  : contract active, fee instalments scheduled, sell-on obligations recorded
 → Aftermath   : morale shifts (selling club dressing room, buying club competition), media, fan reaction
```

Every step is an event; most are club-internal visibility. The protagonist learns about interest in him only through the agent, media leaks, or official approaches (P3).

## 2. Valuation

```
market_value = base(perceived_CA, perceived_PA, age curve)
             × position_multiplier × reputation_multiplier × league_market_multiplier
             × contract_length_factor (≤ 1y left → steep discount)
             × form_factor × injury_factor × nationality/homegrown premium
             × market_inflation_index(date)
asking_price = market_value × selling_club_stance (untouchable/key/available/listed) × negotiation personality
```

Seller stance: untouchable players can still be sold at "unrefusable" multiples; release clauses override stance.

## 3. Club-to-club negotiation

- Offer components: fee (upfront + instalments), add-ons (appearances, goals, promotion, international caps, trophies), sell-on %, buy-back clause, player-exchange, loan-back.
- Negotiation personalities (DoF/chairman): hardball, fair, desperate (financial trouble), relationship-based (friendly clubs discount).
- Deadline-day behaviour: urgency raises acceptance probability for both sides.

## 4. Player-side decision (the same model for the protagonist's AI default)

A player evaluates an interested club with a utility model:

```
U(club) = w_level     * competitive_level(club)            // league & club strength
        + w_play      * expected_playing_time(club, player) // perceived by agent/player
        + w_money     * after_tax_income_increase
        + w_trophy    * trophy_chances
        + w_dev       * development_environment
        + w_rep       * reputation_gain
        + w_loyalty   * (− leaving_cost_current_club)
        + w_home      * hometown/nation affinity
        + w_life      * life_fit (language, climate, family willingness 10§5, partner career)
        + w_manager   * manager_relationship/style fit
        − w_risk      * uncertainty
weights from personality (Ambition, Loyalty, Professionalism) + life situation
```

Player actions: refuse to talk, open to talks, push for move (transfer request), agree personal terms, reject.

## 5. Contracts

### 5.1 Contract types

Youth/scholarship, professional, amateur, non-contract, trial, loan agreement, pre-contract (signed with new club when ≤ 6 months left per rules), coaching/staff contracts later in life.

### 5.2 Terms

| Term | Notes |
|------|-------|
| Wage (weekly/monthly) | Club currency, gross |
| Length | Max by age/nation rules |
| Signing-on fee | Lump sum or spread |
| Loyalty bonus | Paid if player stays through term |
| Bonuses | Appearance, starting, goal, assist, clean sheet, win, unused sub, promotion, avoiding relegation, trophies, international cap |
| Yearly wage rise | % per year |
| Clauses | Release clause (domestic/foreign split), relegation wage drop, promotion wage rise, minimum-fee release to higher-division clubs, optional extension (club/player), top-goalscorer rise, match-highest-earner, sell-on to player (rare) |
| Squad status promise | Written status (starting player etc.) |
| Image rights | % split; tax relevant |
| Other promises (non-contractual) | playing time, new signings, captaincy, future transfer permission — tracked in promise ledger |

### 5.3 Renewals

Club-initiated or player-requested; agent negotiates; multi-round with counter-offers; walk-away thresholds; "running down contract" strategy for free transfer (player choice, affects relationship).

### 5.4 Terminations

Mutual consent (payout), unilateral with just cause (e.g. unpaid wages for N months per rules), club releases (payout of remaining or negotiated), disciplinary termination, retirement.

## 6. Loans

Types: season-long, half-season, emergency (where allowed); with/without option/obligation to buy (obligation conditions: appearances, promotion). Terms: wage contribution %, loan fee, playing-time clause, recall clause, can-play-against-parent flag. Domestic loan limits per rules.

Loan-list decisions for the protagonist: club may *propose* a loan; the player can accept, refuse, or ask the agent to find alternatives. Refusal affects relationship with manager depending on personality.

## 7. Agents

### 7.1 Agent model

Person with: Negotiation, Network (per region), Reputation, Greed (fee %), Loyalty to client, Honesty, Workload (number of clients), Specialisation (youth, big-money, lower leagues, specific countries).

### 7.2 What agents do (for all players)

- Pitch clients to clubs within their network (creates discovery for the club).
- Filter and present interest (honest agents present all; greedy agents push highest-fee deals).
- Negotiate personal terms and renewals.
- Advise on loans, image rights, sponsorship (connects to 10/11).
- Manage media (limited).
- Can be fired/hired; representation contracts with duration and exit fees (rules may cap agent fees).

### 7.3 Protagonist-agent interface

- Instructions: priorities (money/playing time/league level/country preferences), minimum acceptable terms, red lines (countries, clubs), "keep me informed of all interest ≥ level X", autonomy level.
- Monthly agent meeting report: interest summary (as the agent perceives/chooses to present it), market value estimate range, advice.

## 8. Protagonist transfer gating (fairness, not favouritism)

A club can only act on the protagonist if it has *perception* of him from real sources: scouts watched him, he played against them, agent pitched him, national team exposure, media exposure. This rule applies to **every player** (open-football's perception model enforces it for everyone). The OFM "known to buyer" idea is re-implemented as a universal rule.

## 9. Regulations in the market

- Work permits (points model), foreigner quotas, homegrown quotas → clubs account for these in evaluation.
- Minors: international moves under 18 blocked except rule exceptions (02).
- Solidarity & training compensation: small % of fees distributed to youth clubs (by years trained 12–23); training compensation for out-of-contract youth moving abroad. This matters because your boyhood club benefits from your transfers (life/story hook).
- Third-party ownership prohibited (not modeled as allowed).
- Transfer bans / registration embargoes as sanctions.

## 10. Free agents & unattached players

Pool of released players; clubs pick up by need & budget; outside-window signing rules; trial periods (1–4 weeks) with evaluation; unattached protagonist can train with clubs on trial, join a training camp for free agents, or drop to semi-pro.

## 11. Market health metrics (QA 14)

Transfers per window per league tier, fee distributions vs reputation, % of squad turnover per season (typ. 25–40%), free-agent re-employment rates by age, loan counts by age.
