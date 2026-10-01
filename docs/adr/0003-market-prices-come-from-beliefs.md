# ADR 0003: Market prices come from beliefs, not the true ability

**Status:** accepted (locked design §3.5, §3.25-3.30). **Implemented (first step):** `market.rs::public_view`, `fair_value`,
`seller_reservation`; `deals.rs`.

**Decision.** `Player::value` is the *public market estimate*: a stable noisy consensus, narrower for famous players. A club values
a player from its own reading of him (scouting reports and its own coaches' eyes). A seller's reservation adds its circumstances:
how the player is rated there, how easily he is replaced, cash need, contract, board stance. A buyer's ceiling adds urgency and how many
credible alternatives it has. No side sees the other's limit; the public estimate is context, not an anchor.

**Why.** Pricing from true ability makes every club omniscient about every player and makes every fee a lookup.

**Consequences.** `market::true_worth` exists for audits only. Remaining truth reads in recruitment are listed in
`IMPLEMENTATION_STATUS.md`. The price curve is calibrated to the imported scale (`tuning.rs`, round-trip tested).
