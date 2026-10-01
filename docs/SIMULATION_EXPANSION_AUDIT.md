# Simulation Expansion Audit

Snapshot audited: `origin/ui/desktop-client` at `1ce1531` (2026-09-29). Local branch: `sim/expansion-audit`. Nothing pushed.

Status: **audit only.** No file under `crates/`, `data/`, `app/` or `vendor/` was changed. Nothing was compiled, run or
benchmarked for this report. Every claim below comes from reading code and tracing call paths; where I only inferred something,
it says so. Line numbers refer to this snapshot and will drift.

Out of scope on purpose: the desktop UI redesign, the testing workflow, broad refactors. Where a design would need API or
screen work, it is listed as a requirement (§13), not designed.

The code calls the product *Pathway*; the brief calls it *Rise Above*. Same project.

---

## 1. Summary

1. **The premise needs one correction.** Every system in the brief already has a module in the remote: medical cases, staff
   careers, youth pipeline, owners and boards, registration rules, rule evolution, national teams, referees, agents, scouting.
   About 24k lines of `pw-sim` and 11k of `pw-world`. The weakness is not absence. It is that many of these modules are
   **only weakly connected to each other, and several connections are declared but dead** (§3).
2. **Fifteen wiring defects were verified** (§3). The most important: the default match backend (OFM) never reads the
   injury-risk value, so fragility, fatigue and workload do not change in-match injury odds, and the penalty for rushing a
   player back never reaches a match. That one defect makes the medical system look deeper than it behaves.
3. **Four structural gaps** matter more than any missing feature:
   - The board is one number (`Board.satisfaction`). Sacking costs nothing. Poaching a manager does cost money.
   - Only managers have careers. Every other staff member is permanent once hired, and improves every year up to a cap.
   - Club money is a stateless function of reputation, and facilities can only ratchet upward.
   - There is no shared record of "who advised what, who decided, why". So "why was the physio ignored?" cannot be answered.
4. **Saves are positional bincode of the whole `World`** (`crates/pw-sim/src/save.rs`). Any change to any serialized struct breaks
   old saves. Nothing persisted can be added safely until there is a sidecar block and a version header (§6.3).
5. **Determinism is in good shape for draws** (keyed streams) **but not for spawns.** Creating a person shifts every later
   `PersonId`, and every person gets a full `Life` record. Designs that add "board members as people" or "doctors as people"
   are more expensive than they look (§6.2).
6. **Recommended order:** Wave 0 (measure, fix the dead wiring, add the sidecar and one shared decision-record primitive) →
   staff careers and board seats → medical and training → club economy and competition fixes → youth regions and pathway →
   international depth (§9).
7. **Cut list** in §12. **My critique of the brief** is in §14. The largest point: expanding before measuring is the main risk,
   because the repo's own reports say balance over decades is unmeasured.

---

## 2. What the remote already gives us to build on

Reuse these. Building parallel versions is the main way this work goes wrong (§3, D5 is the example).

| Primitive | Where | Use it for |
|---|---|---|
| Events with `visibility` and `causes` (`Cause::Event` / `Cause::Fact`), `push_caused` | `pw-world/src/event.rs` | Every new consequence; "why" chains already render |
| Typed, sourced, decaying memories between persons | `pw-world/src/social.rs`, `consider.rs` | Trust after outcomes (doctor/manager/player) |
| Beliefs and information items with fidelity, knowers, leaks (`InfoKind`) | `pw-world/src/info.rs`, `pw-sim/src/grapevine.rs` | "Who knows": injury worse than said, board turning |
| Perception: `Observer`, `perceive`, `sigma`, per-club evidence | `pw-world/src/knowledge.rs`, `perception.rs` | Any estimate with uncertainty |
| Considerations: small pure functions of state | `pw-sim/src/consider.rs` (30 fns) | Decision inputs; add here, not new triggers |
| Symmetric intents and decisions (AI path = human path) | `intents.rs`, `decisions.rs`, `mind.rs` | Any new choice a person can make |
| Incident hazard from named pressures, each recorded as a cause | `pw-sim/src/incidents.rs` | Training incidents, board unrest, congestion |
| Keyed RNG: `w.rng(stream, keys)`, `w.roll(...)`, 39 named streams | `pw-core/src/rng.rs` | All new randomness |
| Agenda (things due later) | `pw-world/src/agenda.rs` | Deferred follow-ups |
| Records engine, honours, chronicle | `records.rs`, `honours.rs` | Historical continuity |
| Semantic audit + digest tests | `pw-sim/src/audit.rs`, `pw-cli/tests/world.rs` | Invariants; observe-only digest checks (§6.2) |
| Data packs (`data/engine/*.toml`), rule profiles, rule change log with `value_in(season)` | `pw-data`, `pw-world/src/rules.rs`, `evolution.rs` | Any new rule or template |

The daily pipeline order is in `crates/pw-sim/src/lib.rs:150-300`. Selection, matches and health all run from a snapshot of the world;
matches are simulated in parallel (`matchday.rs:51`) and applied in fixture order. Injuries are therefore decided inside the
engine call, before they touch world state. That matters for the fix in D1.

---

## 3. Verified wiring defects

Each was traced through callers. "Dead" means no caller reads or writes it.

| # | Defect | Evidence | Effect |
|---|---|---|---|
| D1 | The default backend ignores per-player injury risk. `PlayerSheet.injury_risk` (fragility × load × wear × age × proneness from `health::hazard_mult`) is read only by the native engine. OFM injures the *fouled* player with a flat 3% per foul. | `pw-match/src/engine.rs:630` uses it; `pw-match/src/ofm.rs` never does; `pw-data/src/tuning.rs:95` default `MatchBackend::Ofm`; `vendor/ofm-engine/src/live_match/zone_resolution.rs:466` | Match injuries are contact-only and blind to workload, fatigue, fragility and age. Only training injuries respond to the body. |
| D2 | Rushing back is instant and mostly inert. A rush sets `injury_days = 0`, condition ≤ 75, sharpness ≤ 40. Its only lasting cost is `+0.5` fragility, which multiplies a hazard that (D1) match injuries ignore. The rush is a weekly dice roll (`roll < 0.6`), not tied to a coming fixture. | `pw-sim/src/medical.rs:178-205` | "Rushed return, recurrence, criticism" cannot happen in a match. |
| D3 | Declared, never used: `Treatment::Managed`; `InjuryDef.play_through` (set in the catalogue); `medical::history_risk`; `Fact::Injury` (only rendered, never built); `stream::TRAINING`. | `pw-world/src/medical.rs:14`, `pw-data/src/pack.rs:286`, `pw-sim/src/medical.rs:264`, `pw-world/src/event.rs:98` | "Playing through pain" has no state. Valuation ignores injury history. |
| D4 | Tactical familiarity is a stub. `Team.familiarity_weeks` is only ever set to 0. | `pw-world/src/club.rs:58`, 3 constructors | No practised-shape concept. |
| D5 | **Two youth generators run at once.** `people::youth_intake` spawns 15–16-year-olds straight into every club's youth team each year. `youth::new_cohort` spawns 8-year-olds into grassroots clubs that feed trials → academy → scholarship. | `pw-sim/src/people.rs:140`, `youth.rs:124`, `lib.rs:172,214` | The grassroots pipeline is optional supply. A club's intake quality comes mostly from facilities and reputation, not from where kids came from. Geography is a `city` string. |
| D6 | Academy identity is inert. `AcademyStyle` is a random draw at creation that only shifts `maturity_bias`. `Reach` is fixed at creation. Local-club `coaching` and `facilities` are read once (cohort creation) and never affect growth. Players outside pro teams train in a default environment. | `youth.rs:86-101`, `development.rs:61` (`team.is_some()` else defaults) | Rags-to-riches players get no local coaching effect; "clubs known for X" cannot emerge. |
| D7 | Non-manager staff never turn over. `contract_end` is written and never enforced; `retired` is set only for managers ≥ 66; only managers are poached; only entourage moves. Yearly growth gives +1 to every key attribute below 18 for anyone with Professionalism ≥ 10, and nobody declines. | `staffing.rs:178-194`, `managers.rs:355-365`, `grep retired = true` | Backroom quality only ratchets up over decades; academy and medical "identity" freeze. |
| D8 | Sacking is free. `board::weekly` removes the manager and pays nothing. `try_poach` pays `wage × 52`. | `board.rs:70-84` vs `managers.rs:414-450` | "Kept because compensation is high" cannot happen. |
| D9 | Continental places are hard-coded by nation reputation rank. `Competition.continental` (places per league) is read only by `pw-view`. The 5-year `coefficient` feeds broadcast deals but not qualification. | `season.rs:373-395`; `grep .continental` | League decline does not cost European places. |
| D10 | Club revenue has no state: base `rep^2.2` + broadcast merit + squad-fame term, recomputed every week. Shirt/kit sponsorship (`rep²`, locked 3 years) is paid on top, straight into `balance`. Reputation moves only at season end by league finish. | `economy.rs:118-141`, `commerce.rs:83-112`, `reputation.rs:37-53` | Possible double counting (WORLD_SYSTEMS risk 5). Sponsors do not respond to squad fame, followers or supporters. |
| D11 | Facility levels only rise. `commission` raises a level; only the rare `FacilityDamage` incident lowers `training` by 1. | `governance.rs:300-365`, `incidents.rs:706-712` | No decline through neglect; a ratchet against "clubs rise and fall". |
| D12 | `Finance.debt = max(0, -balance)`. No principal, interest or lender. Transfer instalments exist (`Payable`) but budgets and board reviews do not see committed payments. | `finance.rs:60-70`, `deals.rs:386,441` | Cash vs accounting vs committed cannot be told apart. |
| D13 | Manager reputation is stored twice (`Staff.reputation`, `ManagerProfile.reputation`) and synced by hand in 3 places. Only managers' copy is ever updated meaningfully. | `managers.rs:288,467` | Minor. One source of truth needed before "contextual reputation". |
| D14 | Rule constants next to data: `MIN_PRO_AGE`, `max_contract_years()` in code although `RuleProfile` has the fields. | `pw-world/src/rules.rs:75-80`, `negotiation.rs:92` | S9 debt; small. |
| D15 | Selection reads training rating as truth (`h.training`), while ability is perceived. | `selection.rs:~165` | Small. Contradicts "manager belief". |

Two suspected issues I could not prove by reading, and want a metric for (§9 Wave 0):
- **Double-booked days.** `schedule_league` falls back to Wednesday, and cups prefer Wednesday. Nothing I read stops one team having two fixtures on one day. Matches select from a pre-day snapshot, so a double-booked team would field the same XI twice.
- **Amateur build-up.** Grassroots creates about 2–4 children per grassroots club per year; amateurs are never removed
  (`PlayerStatus::Amateur` persists, `INTEGRATION_REPORT` §10 notes "retired people and amateurs accumulate"). Weekly youth loops will grow with time. Not measured.

---

## 4. System-by-system audit

Legend per system: **Strong** (implemented and connected), **Shallow**, **Scaffolded** (declared, not wired), **Absent**,
**Conflicts** (with the brief's direction).

### A. Injury, medical, rehab, body wear

**Strong**
- Injuries are cases with an *estimate and a certainty* that the world sees, distinct from the true remaining days
  (`medical.rs on_injury`, σ falls with medical quality). Estimates converge weekly.
- Diagnosis quality depends on the best physio / sports scientist plus facilities (`medical_quality`).
- Surgery vs rehab is a real choice: AI decides by professionalism, age, contract years, club quality; a human gets a `Decision`.
- Setbacks, recurrence within a year (×1.3), fragility left behind (decays 7%/month), chronic conditions, systemic illness.
- Training-injury hazard is built from workload ratio, fatigue, wear, age, well-being, proneness and fragility (`health::hazard_mult`).
- A medical room that knows more than the public: `InjuryWorse` becomes an information item known by the player, manager and
  physios, and can leak (`grapevine.rs:174`).
- Same intent/decision path for AI and human (`Intent::PlayThroughPain`, `DecisionKind::Treatment`).

**Shallow**
- Availability is binary: `available()` requires `injury == 0`; `rules::can_play` denies an injured player outright
  (`pw-world/src/rules.rs:82-93`). No stages between "out" and "fit".
- Rehab speed depends only on club medical facility level. Staff skill shifts *diagnosis noise* and *setback chance*.
  Player professionalism, compliance and recovery habits (`plan.recovery`, sleep) affect condition, not injury progress.
- Mechanism comes from the catalogue category only (contact / non-contact / overuse / illness) and is not stored on the case.
- Public knowledge: `Injured` is a `Visibility::Club` event; the press publishes only if ≥ 28 days, a big club and a senior
  player (`media.rs:437`). There is no "hamstring, two to four weeks" public bulletin distinct from the club's estimate.
- Money links: valuation ×0.8 above 60 days (`market.rs:36`); deal medicals use wear, career injury count and current injury
  (`deals.rs:341`); planning counts long-term injured (`planning.rs:56`); AI retirement +0.4 above 150 days (`mind.rs:248`).

**Scaffolded (D3):** `Managed` treatment, `play_through`, `history_risk`, `Fact::Injury`.

**Absent:** opinions that differ by person; a return-to-play ladder; a rush decision tied to a match; performance cost of
partial fitness; psychological effects; injury → confidence, morale, renewal terms, wage risk; insurance; match-context
injury causes (D1); recurrence risk that reaches matches.

**Conflicts:** none with the brief's direction. D1 must be fixed first or all match-level work is decorative.

**Connections present:** injury → selection (hard exclusion), valuation, medicals, squad planning, retirement, national call-up (≤ 10 days),
club withdrawal from squad, press label `InjuryProne`, grapevine leak, AI training plan (light).
**Missing:** injury → confidence/morale; injury history → renewal/valuation (dead `history_risk`); injury → board/finance (long-term
injured wages); rushed return → manager–medical trust; match context → injury.

### B. Training and coaching

**Strong**
- Load is modelled: acute/chronic EWMA, fatigue debt, condition recovery, day types around fixtures (so a congested week has
  fewer training days automatically).
- Each player has a plan (focus general/group/attribute/position, intensity, extra sessions, recovery sessions). Position
  learning works (`Focus::Position`, familiarity grows).
- Coaches "see" a noisy training rating that drifts; slumps and surges become facts, notes and memories.
- Development uses coach quality *per attribute group* (technical, mental, speed, power, goalkeeping) and facility level;
  youth teams weight `Youngsters`.
- Training incidents exist as contextual kinds (`TrainingConfrontation`, `TacticalDisagreement`, `LateArrival`, `StormedOut`).

**Shallow**
- Plans are chosen by the *player's* AI mind monthly (`mind.rs:52 ai_training`), not set by coaches.
- `team_environment` takes the **best staff member per group** (`max`). One good coach lifts the whole squad; depth, unit
  assignment and coach–player fit do not exist. `consider::compat` is never used for coaching.
- Manager philosophy does not touch training. Archetype only weights selection.
- Loads per day type are constants (`health.rs:23-32`), identical for every club.
- Selection reads the training rating as truth (D15).

**Scaffolded:** `Team.familiarity_weeks` (D4); `stream::TRAINING`.

**Absent:** session types and a weekly microcycle; team-level regime; tactical familiarity and its match effect; coach specialisation by
unit; coach–player fit; staff disagreement on load; positive incidents (breakthrough, outstanding session); training camp for
national sides (only a −3/−8 condition hit on release, `intl.rs:640`).

**Conflicts:** the brief's session-by-session schedule would be a minigame for a role no human inhabits. Keep it at team-week
resolution (§7B).

**Connections present:** training rating → selection form, coach notes, incidents; load → injuries; intensity → development; AI plan reacts to fatigue.
**Missing:** load plan ← fixture density and staff advice; training → tactical familiarity → match execution → manager trust; coach fit → development.

### C. Youth academies, generation, pathways

**Strong**
- A complete pipeline exists in `youth.rs`: grassroots clubs with feeders → statistical age-group football (same model for all,
  S27) → academy scouts (reach-limited) → trials → academy → yearly reviews → scholarship at 16 → pro contract only if the
  club believes → release → local/amateur football → possible re-discovery, plus school and exams.
- Judgement is fallible: `judged_potential` skews with how physically advanced a child looks (`maturity_bias`), so late
  developers are the ones wrongly released. `bio_offset` shifts the whole growth curve; potential is re-rolled once near 19.
- Stagnation: 17–23-year-olds without minutes lose potential (`growth.rs:226`). Mentoring exists.
- Adult amateurs can be trialled by pro clubs (`deals::trials`).
- Families are people in the life model; parental support gates trial invitations.

**Shallow**
- Geography is a club `city` string; nation football culture is one `youth_rating` scalar from the import.
- Recruitment competition: one trial at a time per child; acceptance is `0.5 + 0.5 × support`. There is no comparison of
  offers, no pathway or first-team record, no relocation weighing beyond a cost gate.
- No academy history. Nothing counts who graduated, into which positions, with what first-team minutes.
- Promises to families and agents (pathway, loans, contract timing) are absent; `PromiseKind` covers signed players only.

**Scaffolded (D6):** `AcademyStyle`, `Reach`, local-club `coaching` and `facilities`.

**Conflict (D5):** two generators. Decide which is canonical.

**Connections present:** facilities/reputation → intake PA; board `youth_investment` → academy budget; head of youth judgement →
release/scholarship; school → attitude memories; manager `youth_trust` → selection of young players; board `youth_minutes_target` concern.
**Missing:** academy outcomes → club "pathway" reputation → recruits; staff change → academy character; region → talent; local coaching → growth.

### D. Staff careers and organisations

**Strong**
- Managers have a profile: taste for player types, rotation, veteran loyalty, favouritism toward former players, media style,
  entourage that follows, CV of jobs with how each ended, systems used (`managers.rs`, `careers.rs`).
- New manager → every player is re-assessed through their eyes; favourites come with them; players react to the departure by
  affinity; resignation when the owner relationship breaks; poaching with compensation and the old club's vacancy.
- Tactical schools with lineage: people who played or coached under a school's manager carry it (`evolution.rs`).
- Hiring depends on skill, reputation, the manager's view of the candidate, local ties and coaching licence
  (`staffing.rs`, `affairs::coaching_level`).
- Retired players and anyone else can enter the staff pool (`people::enter_staff_pool`).

**Shallow**
- Reputation is one number per person. "Great youth coach, unproven manager" cannot be expressed.
- Networks are whatever `affinity`/`trust` memories already exist plus a "played here" bonus. Nothing derived from shared careers.
- `board::appoint` promotes only the *current* assistant; ignores the director of football; internal coach promotions and
  external candidates from elsewhere in the club do not compete.
- A new manager replaces only the assistant; other backroom staff stay.

**Scaffolded (D7, D13):** non-manager contracts, retirement and poaching.

**Absent:** careers for assistants, coaches, scouts, physios, analysts, directors; contract expiry and renewal decisions for them;
mentorship edges other than school membership; staff decline with age; recruitment-chief or medical-chief politics.

**Connections present:** manager → selection, statuses, philosophy, board; school → tactics; licence gates → hiring.
**Missing:** backroom turnover → development environment → academy character; sporting director ↔ manager conflict; staff market →
national jobs (federations do hire from the unemployed pool, `intl.rs:157`, so this half exists).

### E. Ownership, boards, club economics

**Strong**
- Owners are persons with wealth, ambition, patience, meddling, frugality, fan sensitivity; ownership kinds (private,
  member-owned, benefactor, investment group, state-backed) set defaults.
- Policy: wage ceiling multiple, youth investment, transfer style, max signing age, debt tolerance, style mandate, youth-minutes
  target, selling stance. These feed wage ceilings, asking prices, projects and appointments.
- Monthly board concerns (finances, youth minutes, style, fan unrest, owner patience) are recorded with weights.
- Yearly: owner injection, facility projects that take years, takeovers (distress and appeal), administration with points
  deduction, recovery.
- Transfer instalments, sell-ons, add-ons and agent fees are payables.
- National economies: inflation, growth, broadcast deals renegotiated from league strength and continental coefficient.

**Shallow**
- The board is `Board { satisfaction, patience, target_position, warnings }` plus one chairman person. The concerns list is
  summed into the single number (`governance.rs:monthly`). Different seats cannot disagree.
- Sacking rule: three low-satisfaction warnings. No alternative-candidate quality, no compensation cost, no owner
  vs sporting-director split.
- Finance has no separation of cash, profit, committed wages, payables or debt service (D12).
- One project at a time per club, chosen as the biggest gap, 50% roll. Stadium, training, youth, academy network and medical
  compete only through that gap. No analytics or scouting facility.
- Fans: `fan_mood` (0–100) and supporter groups exist; ticket demand and attendance have no state (gate = capacity × demand ×
  price computed per match, `matchday.rs:271`).

**Scaffolded / defective:** D8, D10, D11, D12.

**Absent:** rise/decline pressure independent of reputation; decay; supporter-driven demand; sustainability rules; board seats; owner
priorities among facilities; economic shocks beyond those already coded (relegation revenue via tier share, administration).

**Connections present:** owner → wage ceiling, selling stance, style mandate; debt → austerity → listings; takeover → board reset and sometimes manager friction; nation economy → revenue.
**Missing:** squad fame/followers → sponsorship; supporters → demand → revenue; relegation → sponsor clauses; compensation → sacking decision; facilities decay; SD/owner/supporter conflicts.

### F. Competitions, rules, calendar

**Strong**
- Registration rules are data profiles with explainable reasons (`RuleOutcome`): windows, squad size, homegrown, foreigner
  limits, loans, contract length, work permits, cup-tying, minors abroad.
- Rules evolve for stated causes only (injury crisis → subs, red-card epidemic → bans, national decline → homegrown, away-goal
  ties → abolish), with a six-year cooldown per federation and `value_in(season)` reconstruction.
- Schedules avoid international windows and winter breaks; cups with byes; continental groups then knockouts; promotion and
  relegation keep divisions constant and block B-teams.
- Postponements from weather, pitch damage and general hazard; travel delay costs the away squad condition.
- History: archived tables, honours, awards, records engine, chronicle.

**Shallow**
- Postponed fixtures move to "next Wednesday after +7 days" with no check of either team's other fixtures (`incidents.rs:714`).
- Stadium sharing, travel distance and fixture clashes are not modelled.
- Promotion and relegation are fixed counts; no play-offs, no licensing.
- Competition prestige is static (`Competition.reputation` never changes in sim); league strength is a rolling average of top
  clubs' CA (circular).
- Broadcast and prize money are pools by tier; not linked to competition prestige or attendance.

**Defects:** D9. **Unproven:** double booking (§3).

**Connections present:** results → tables → reputation (season end), prize money, honours; failures → rule changes; coefficient → broadcast.
**Missing:** coefficient → qualification places; prestige as state; fixture congestion → selection and training (only implicit).

### G. International football

**Strong**
- National managers are real staff hired from the unemployed pool; they pick squads from an *imperfect* view (`fed_view`: caps
  with that nation, stage, domestic status, own judging), trust memories, position quotas, streaks, philosophy archetype.
- Club pressure can withdraw a knocked player; the national manager remembers who pulled him.
- Dual nationals decide with the same considerations for AI and human (`preferred_nation`, `NationChoice`).
- Caps lock allegiance at the first senior competitive cap; veterans retire from international football.
- Qualifying, continental finals and a world tournament; Elo by match type; managers judged after tournaments.
- Caps → reputation, valuation, work-permit points.

**Shallow**
- Eligibility is `nation` and `nation2` only. Parentage, residency, previous appearances beyond the cap lock are not rules-as-data.
- Federations are `conservatism` and `last_change` (rule changes). No scouting reach, youth structure, coaching or administrative quality.
- National-team tactics are the manager's philosophy directly. No limited camp time; no familiarity.
- Club-versus-country: flat 50% withdrawal if condition < 80 or injured, independent of match importance; release costs
  3 or 8 condition points and extra load if far.
- Youth internationals: friendlies only.

**Absent:** federation attributes; tournament squad-size decisions; camp training; club–federation dispute state; international
success → participation (§14); fame effects beyond reputation drift.

**Connections present:** see above. **Missing:** federation quality → who is seen; call-up importance → club release decision; camp → familiarity → result; call-up → sponsorship/social (not traced).

### H. Optional areas (assessed for importance)

| Area | State | Verdict |
|---|---|---|
| Referees | Persons with accuracy, strictness, controversies, appeals, charges, perceived bias that forms without real bias (tested). | Deep enough. Do not expand. No VAR era: the engine cannot use it. |
| Agents | Persons with negotiating, network, diligence, greed, honesty, capacity, base and reach. | Adequate. Add conflicts of interest only if a slice needs them. |
| Scouting networks | Briefs, per-scout biases, familiarity by nation, capacity, travel penalty, analysts. | Good. Gains come from `Region` (geography) and `StaffCareer` (networks), not new code. |
| Stadium / attendance | Capacity, gate formula, atmosphere from capacity and mood. | One small missing piece: a `Demand` state (§7E). Nothing more. |
| Education / post-career | Courses, badges, pundit/coach/agent/business routes in `affairs.rs`. | Leave alone. |

---

## 5. Cross-system connections

### Present and verified

```
injury → selection | valuation | deal medical | squad plan | retirement | call-up | press label | grapevine leak
training rating → selection form | coach note → memory | incident hazard
manager change → re-assessment of every player | entourage | favourites | school blend | board reset
owner policy → wage ceiling | asking price | projects | appointments
caps → reputation | valuation | permits ; injury on duty → manager LetDown memory
grassroots → trial → academy → scholarship → pro contract → release → amateur → trial
result → table → reputation → revenue (next weeks) ; coefficient → broadcast deal
```

### Missing (the brief's examples, with the exact missing edge)

| Chain in the brief | What is missing |
|---|---|
| injury → selection → performance → media → social → confidence → contract → transfer | injury → confidence; partial fitness → performance; history → contract terms and valuation; rush → recurrence *in matches* (D1) |
| training → tactical familiarity → performance → manager trust → selection → development | familiarity state (D4); its match effect; trust from training as *perceived*, not truth (D15) |
| academy prospect → youth reputation → scouting → contract → social hype → first-team pressure | pathway record; rival academies' competition for kids; hype for under-18s; pressure on promotion |
| new owner → risk appetite → facilities → wages → transfers → supporter expectations → manager pressure | owner priorities among facilities; supporter expectation state; compensation-aware manager decision |
| call-up → fame → travel fatigue → club selection → sponsorship → social following | club's rest decision; camp load; sponsorship response to caps (not traced) |
| staff change → club football direction | turnover for non-managers (D7); academy character from staff |

---

## 6. Cross-cutting: performance, determinism, saves

### 6.1 Performance

Already at 300k players: a day costs about 2.4 s; the largest costs are the social network (530 ms/day), weekly perception (2.15 s),
weekly minds (1.6 s), weekly youth (1.25 s), matches (234 ms). A 300k season is about 15 minutes (`INTEGRATION_REPORT` §11). New
work must not add per-player-per-day cost. Rules of thumb for everything proposed below:

- **Hot state per player: add none.** Return stage is derived from `injury_days / injury_total`. Load uses existing `acute` and `chronic`.
- **Team and club level (≈ 4.6k at 300k):** weekly plans, familiarity, board seats, financial position. Cheap.
- **Staff level (≈ 20 per club, ≈ 90k):** careers, monthly and yearly events. Cheap versus 300k players.
- **Sparse per-case or per-decision records** only: open injury cases, decisions on a real trigger, rulings.
- **Event-driven where the trigger is rare** (return decisions only for players in the last third of a case with a fixture inside the horizon).
- **Derived on demand** (cold, monthly): financial position, pathway rate, academy identity, region talent, durability.
- Growth risks to watch: `Judgement` records (bound per club and compact after two seasons), amateurs never removed (D-note in §3), event log.

### 6.2 Determinism

- **Draws are safe.** Every draw is keyed `(seed, stream, ids, date)`, so a new random draw does not move any other stream.
  New streams should be added to the `stream::ALL` table (`rng.rs:90`, array length 39).
- **Spawns are not safe.** `w.people.push` shifts every later `PersonId`, which are RNG keys. Any system that creates persons
  changes unrelated outcomes after that point. Mitigations: spawn in a fixed phase in sorted order; prefer seats or lazily
  materialised persons over spawning at every club; derive any materialised detail from `(seed, person)` (S20).
  Note that each new person also gets a full `Life`, and `life::init` may create a partner (`pw-sim/src/life.rs:24-37`).
- **Hash-order hazard** (already in `WORLD_SYSTEMS` §5.7): any new code that acts while iterating `FxHashMap` must sort first.
  New sidecars should use sorted `Vec`s or `BTreeMap` for anything that decides an outcome.
- **A cheap guard that fits the existing tests:** an *observe-only* mode per new subsystem (it computes and records but its effects are
  switched off). The world digest must equal the baseline. This is the same technique as `narration_never_changes_the_world`.
  It proves that adding the system did not disturb unrelated results, and it exposes which spawns did.

### 6.3 Save compatibility

Today: header `PWSAVE01`, lz4 of `bincode(World)`; `World` also contains `data: DataPack` (so tuning frozen at creation travels with the
save). bincode is positional and non-self-describing. Adding, removing or reordering a field in any serialized struct breaks
loading. Appending an enum variant is safe for *reading* old data.

Proposed rules (all new persistent state):

1. **Never change the layout of an existing serialized struct.** New per-entity data goes in a sidecar keyed by id.
2. **One new field on `World`:** `ext: Extensions`, excluded from the main block and written as a trailer chunk:
   `[u16 tag][u16 version][u32 len][bytes]` per subsystem. A missing chunk gives `Default`.
3. **Bump the magic to `PWSAVE02` when the trailer lands;** the loader accepts both.
4. **Legacy initialisation is a deterministic function of existing state and never invents history.**
   - `StaffCareer`: one open job since `Staff.joined`; no mentors.
   - Case sidecar: none; open cases get mechanism `Unknown`.
   - Board seats: built from existing owner, chairman, director of football and manager; stances from existing concerns.
   - Region: derived from the club `city` string and nation.
   - Rulings: empty. Anything before the save is reported as "before this save", not explained after the fact.
   - Provenance: every legacy-initialised record carries `Legacy`, following the `backfill` convention.
5. **One golden-save load test** per released schema version. It fits whichever tier your local workflow assigns; I am not proposing to design that workflow.

This also minimises merge conflicts with local work that has changed `World`: the only shared line is the single `ext` field.

---

## 7. Proposed architecture

Conventions used here: **sidecar** = new keyed struct inside `Extensions`; **derived** = computed on demand from existing state, never stored; **cadence** = when it updates.

Shape of every decision (this is the brief's pipeline made concrete):

```
STATE  →  PRESSURES (named, recorded as Facts)
      →  OPINIONS (each stakeholder's estimate + stance)      [new: Ruling]
      →  DECIDER chooses (institutional authority, trust)      [new: Ruling]
      →  CHANCE resolves on TRUE state
      →  EVENT (with cause = the Ruling event)
      →  WHO KNOWS (visibility + info items)
      →  each involved person updates memory from OUTCOME     [new: hindsight]
      →  NEW STATE
```

### 7.0 Shared primitives (used by all systems)

| Primitive | Shape | Precedent already in repo |
|---|---|---|
| `Ruling` | `{ id, kind, decider, subject, options, chosen, opinions: SmallVec<Opinion,4>, pressures: Causes, date, outcome: Pending/…}` emitted as one event `EventKind::Ruling`; downstream events cite `Cause::Event(ruling_event)` | `Decision`/`Causes`/`Fact`; incident `Pressure` |
| `Opinion` | `{ who, recommends, confidence, basis: Fact, bias_source }` | `Report`/`Verdict` in scouting |
| `Estimate` | `{ value, spread, source, date }`; audience views are functions of truth + estimate | `Case.estimate/certainty`, `perceive/sigma` |
| `Seat` | `{ role, holder: PersonId, weight, stance, agenda }` for one institution | Governance owner + chairman |
| `StaffCareer` | jobs, mentors, per-context reputation, derived network | `ManagerProfile.jobs`, `School.adherents` |
| `Region` | small entity per nation subdivision | `Nation.youth_rating`, club `city` |
| `Load` | acute/chronic (exists) plus travel and congestion terms | `PlayerHot.acute/chronic` |
| `Extensions` + trailer | §6.3 | none |

Outcome is separate from decision quality by construction: a `Ruling` stores what was known and advised; the outcome resolves
later on the *true* state; each person's memory updates from the outcome, with a bias toward the outcome
(hindsight), so people can learn the wrong lesson. Quality of decision is recomputable from the record.

**Event nesting** (for compile-conflict reasons, §13): add one new `EventKind` variant per domain that wraps a nested enum
(`Medical(MedicalEvent)`, `Training(...)`), so `pw-view` and `pw-narrate` need one arm per domain, not one per event.

### 7A. Injury, medical, rehab

Keep: `Case`, estimate/certainty, fragility, chronic, wear, treatment choice, `InjuryWorse` leaks.

Change (in order):
1. **D1 fix in the OFM adapter, not in `vendor/`.** Two options; my recommendation is (a):
   a. Keep OFM's foul-injury event as the *contact* channel and accept it with probability `min(1, injury_risk)` scaled to keep
      the mean; add a *non-contact* channel per player from `injury_risk × exposure × condition` keyed
      `(seed, HEALTH, fixture uid, player)`. Non-contact injuries are recorded as a late knock (`minute` drawn in minutes
      played) without changing the engine's substitution flow.
   b. Replace OFM's injuries entirely. Cleaner statistics, but match reports show a player finishing 90 minutes and being injured.
   Recalibrate afterwards (PROGRESS §Step 3: 0.65 vs target 1.2–1.8 per player-season).
2. **`CaseCtx` sidecar (open cases only):** `mechanism`, `fixture` or session, `minute`, `fatigue_at`, `opponent`, `surface`,
   `load_flag`. Folded into a compact `CaseSummary` when closed.
3. **Return ladder, derived:** `Rehab → Individual → PartialTeam → FullTraining → BenchReady → MatchReady` from `injury_days / injury_total`
   and case class. `available()` becomes `Fit | Limited(stage) | Out`. Selection takes `BenchReady` for the bench and `MatchReady` for a
   start; a stage caps condition and sharpness; `can_play` returns `Reason::NotCleared { stage }`.
4. **Opinions** for each open case: physio (optimistic by role), sports scientist (risk-averse by role), player self-view (bias
   from Ambition/Pressure vs Professionalism), manager (need × importance × archetype × trust in the medical room). No new
   staff role in wave 1; add `Doctor` only if playtests show two voices feel thin.
5. **Return decision** (`Ruling`), opened only when a fixture with `importance + stakes ≥ θ` falls inside a horizon and the
   case is past `Individual`. The decider is the manager unless the club's *medical authority* (from owner / director policy) vetoes.
   Chance resolves on the **true** remaining fraction and fragility: setback probability rises with `remaining/total × fragility × (1 − q)`;
   performance factor follows the stage.
6. **Learning:** manager `rush_propensity` (one `i8` per manager sidecar) drifts from outcomes only. Trust memories: physio
   overruled and vindicated / wrong; player "pushed back too soon". Existing `MemoryKind`s may suffice; add two if not.
7. **Body wear:** durability is derived (age, career minutes from season lines, region repeat count from `medical.history`,
   fragility). Feed it into `hazard_mult`. Do not store a new per-player number.
8. **Hesitation** (`psyche` sidecar): only for cases ≥ 60 days, sparse, for a minority (the draw uses Pressure, Composure, age).
   It caps confidence gain and fades over weeks. Not universal.
9. **Knowledge tiers as views:** public bulletin (region and a coarse band), club (estimate, certainty, opinions), player (felt state).
   One truth, one estimate per audience, no duplicated storage.
10. **Money:** wire the dead `history_risk` into valuation and renewal terms (shorter deals, appearance add-ons already exist
    in `deals`); count long-term injured wages in `FinancialPosition` (§7E). No insurance market.

Cadence: daily only for open cases; weekly for opinion convergence (exists); event-driven for the return decision.
RNG: new stream `RETURN` keyed `(player, fixture uid)`. Save: sidecar; legacy open cases get `Unknown` mechanism.

### 7B. Training and coaching (team-week resolution)

- **`Regime`** per team: emphasis over `{tactical, technical, physical, recovery, set piece, individual}` and a rigidity term, derived from
  manager philosophy, staff advice and the club's training-facility level. Recomputed monthly or on manager change.
- **`LoadPlan`** per team per week, chosen Monday from fixture density (0/1/2 matches, travel, international absences).
  It supplies the daily load that `health::team_days` now hard-codes (`health.rs:23-32`), so daily cost does not change.
  Data-driven: `data/engine/training.toml` templates.
- **Tactical familiarity** replaces the dead `familiarity_weeks`: per team, one value per formation and three style values
  (press, tempo, directness), accruing with tactical units and decaying with change. Match effect: an `execution` factor on the
  team sheet. OFM has no such hook, so start with a small attribute-conversion scale (`ofm_conversion`) and document that it is a
  proxy. Newly formed national squads start low, which gives the "limited camp time" effect for free.
- **Coaches by unit, fit by relationship.** Replace `max` per group with best coach *for the player's unit* (goalkeepers, defenders,
  midfield, attack, youth), then ±10% from `compat` and trust. Assignment is derived each Monday, not stored.
- **Staff disagreement** on load is an `Opinion` set (fitness coach or scientist from team ACWR; assistant; manager decides by
  archetype and stubbornness) recorded as a `Ruling{kind: LoadPlan}`. Later injury clusters or complaints resolve its outcome.
- **Perceived training rating** for selection (D15): pass it through the manager's judging attribute like ability.
- **Incidents:** add positive kinds (breakthrough, outstanding session) from the existing `TrainingSurge` fact; feed plan
  pressure into existing incident hazards.
- **Individual optimal load** stays derived from natural fitness, age and injury history; no stored hidden per-player optimum.

Cadence: weekly per team; individual development weekly (exists); regime monthly. RNG: `COACHING`.

### 7C. Youth, generation, pathways

1. **Single source of talent (D5).** Keep the `youth.rs` pipeline as canonical. Rewrite `people::youth_intake` to draw only from
   region pools of children *not yet placed*, or reduce it to a catch-up top-up for nations with thin grassroots. Tune so intake
   volumes match the current annual counts; verify with a metric (§9 Wave 0: share of pros who passed through grassroots vs
   direct intake).
2. **`Region`** (sidecar; 3–12 per nation): `participation`, `talent_bias`, `culture` (technical / physical / tactical education),
   `density`. Grassroots clubs and academies attach to regions. Fallback when the import has none: deterministic grouping of club
   cities. Youth volume and profile come from `Region`, not a nation scalar. Do not hardcode national stereotypes: `culture`
   values are data inputs.
3. **Academy state made mutable and derived:** `Reach` from budget and facilities each year; `AcademyStyle` derived from
   the head of youth's philosophy and the coaching staff's attribute vector (mostly via existing `team_environment` group weights).
   Identity is then a *consequence of who works there* and updates as staff change.
4. **Graduate ledger** per club (ring of 15 seasons): counts by position group and archetype, first-team minutes by 21, mean
   attribute deltas against the intake. **Identity** and **`pathway_rate`** are computed from the ledger, not stored as modifiers.
5. **Pathway offers.** When ≥ 1 academy invites a child inside a window, the family scores clubs by derived pathway rate, head-of-youth
   trust, relocation cost (`household_move_cost`), education demands (`study_required`) and terms. Recorded as a `Ruling`
   so "why did the youngster choose Club B?" is answerable.
6. **Outside the academies:** give local clubs a real growth environment (`coaching`, `facilities` feed `team_environment` for
   players with no team). Amateur and school competitions already exist (`minor.rs`) and should raise visibility to scouts.
7. **Family and background:** keep the existing household and parents-support model; add only relocation willingness and
   financial pressure as derived considerations. No class model.
8. **Youth promises** (pathway, loans, contract timing) are added to the promise ledger only when a slice needs them.
9. **Late bloomers:** the model already has a curve shift (`bio_offset`) and a one-off re-roll. The remaining gap is *perception* (exists) and
   *exposure* for players outside elite academies (fix in 6).

Cadence: yearly for cohorts and ledger; weekly for statistical youth play (exists); monthly for reach and identity.
Save: `Region` from city strings; ledger empty ("history not recorded before this save").

### 7D. Staff careers and organisations

1. **`StaffCareer` for every staff member** (sidecar, created lazily at first change): jobs with role and how each ended, mentors,
   per-context reputation `{manager, coach, scout, youth}`, derived network (union of colleague sets over jobs, capped).
   Migrate the two manager reputation copies (D13) to it.
2. **Lifecycle events:** contract expiry for all staff with a two-sided decision; renewals; poaching of non-managers by
   bigger clubs with compensation for senior roles; retirement; decline with age; growth depending on role exposure and
   mentor rather than +1 for everyone (D7).
3. **Promotion pipeline:** candidates for a vacancy include internal assistants and coaches, unemployed staff, and poach targets.
   The score is taken from the *seats* that decide (§7E), not one formula.
4. **Lineage:** extend `formed_by` into mentor edges (assistant/coach → manager) that blend philosophy on promotion (`evolution::on_appointed` already blends).
5. **Failure and recovery:** a sacked manager's `manager` reputation drops; `appoint`'s rep-proximity filter already lets them fall
   a level. Add explicit alternatives: take an assistant role, a national job (federations already hire from the pool), leave,
   or reinvent (philosophy drift scaled by `adaptability`).
6. **Club direction changes emerge from role changes** (manager → tactics and selection; head of youth → academy; director of
   football → scouting briefs and negotiation). This needs no new modifier, only D7 fixed.

Cadence: contracts monthly; careers on event; growth and decline yearly. RNG: `CAREER`.

### 7E. Ownership, boards, club economics

1. **`BoardRoom`** per club: `Seat`s for owner, chairman, sporting director (existing staff), and, only for member-owned and large clubs,
   one or two more. **Seats reference existing persons; do not spawn new ones** (§6.2). Each seat has a weight and computes
   a `backing(manager)` from its own pressures, recorded as Facts:
   - owner: results vs ambition, style, finance
   - chairman: fans and results
   - sporting director: whether his signings play; recruitment friction
   - supporters' representative (member-owned only): fan mood
   `Board.satisfaction` remains, **as a cached summary** of the weighted backings (S12).
2. **Sack or back is a `Ruling`:** act when weighted backing falls below a threshold **and** the quality gap to the best available
   alternative outweighs compensation × owner frugality. Otherwise record a public "backed for the long term" event. **Pay compensation**
   (contract years left × wage) on sacking (D8).
3. **`FinancialPosition`** (derived monthly): cash, committed wages 12 months, payables and receivables 12 months, debt service,
   runway. Board reviews, negotiation ceilings and squad planning read it instead of `debt / revenue`.
   Replace `debt = -balance` with a `Debt { principal, rate }` sidecar only if the measurements in Wave 0 say it matters.
4. **Facility competition and decay:** yearly, each club scores stadium, training, youth, academy, medical, analytics and scouting
   by gap, by owner style, and by seat agendas (the sporting director asks for scouting and analytics, the chairman for the stadium).
   Levels decay a little when a club under-invests (maintenance shortfall). Analytics and scouting need two new `Facilities`
   fields; that changes a serialized struct, so they go in a sidecar (§6.3).
5. **`Demand`** per club (sidecar, two numbers: `loyalty`, `excitement`) plus attendance. Updates by season. Feeds gate,
   `fan_mood` and the atmosphere already in `officials::pre_match`. It is the only missing piece in "stadium / attendance".
6. **One ledger.** Route club sponsorship, image-rights shares, ambassador wages and prize money through one per-club season ledger by
   category; remove the fame-based `commercial` term from `club_revenue` or from `commerce`, not both (D10).
   Sponsor value follows squad fame and followers with a lag; deals keep their term but carry a relegation clause.
7. **Shocks** (relegation, promotion, continental qualification, sponsor loss, ownership change) already have most triggers;
   what is missing are the *ledger* to make them visible and the *decay* to make them persist (§10 slice 7).

Cadence: board monthly (exists), ledger and demand seasonally, facility decisions yearly.
Legacy: seats from existing owner and chairman; ledger and demand start at current values with `Legacy` provenance.

### 7F. Competitions, rules, calendar

Small, safe fixes; none needs new state:
1. Continental qualification reads `Competition.continental` places and ranks nations by the existing 5-year coefficient (D9).
2. Postponement reschedules to the first date free for both teams (a per-team fixture index exists).
3. A fixture invariant: no team plays twice on a day, checked in the audit; if it fails, fix `schedule_league`'s Wednesday fallback.
4. Competition prestige as a slowly moving value from continental results and league strength; feeds broadcast and reputation stage.
   **Only after Wave 0 measurements.** League strength as defined (top CA average) is circular.
5. Rule constants moved into the profile (D14).
6. Do not add more evolving rules. The four in `evolution.rs` are enough until a system needs another.

### 7G. International football

1. **Eligibility as rules-as-data:** a per-federation table (birth, parentage, residency years, prior caps) evaluated with reasons,
   like `rules.rs`. The cap-lock becomes a data rule. Needs a `Person` field for parentage or a derived function from the life model's household nations.
2. **`Federation` attributes:** scouting reach (extra observation minutes in `fed_view`), youth structure, coaching, administration.
   Derived at first from nation economy and reputation; changes slowly.
3. **Camps:** national sides get a `LoadPlan` with few sessions, therefore low familiarity, therefore simple shapes (via 7B).
4. **Release decision:** replace the flat 50% withdrawal with a `Ruling` between club and federation weighing match importance,
   medical opinions (7A), and the relationship; record a dispute memory that affects later calls.
5. **Dual-national:** add manager contact and federation relationship as considerations (the `GaveChance` memory exists).
6. **International success → participation:** small, lagged, capped effect on `Region.participation`; no automatic buff (§14).
7. **National-manager careers** use `StaffCareer` (peak, rebuild, refuge are then visible in the CV).

### Time scales

| System | Minutes | Days | Weeks | Months | Seasons | Decades |
|---|---|---|---|---|---|---|
| Injury | in-match chance | case progress, rush decision | estimate convergence, setbacks | fragility decay | durability | — |
| Training | — | load | team plan, familiarity, development | regime, coach fit | staff quality | school prestige |
| Youth | — | — | statistical play, scouting | school, reach, identity | cohorts, ledger | region participation |
| Staff | — | — | — | contracts, market | growth/decline, lineage | reputation by context |
| Club | — | — | wages | board review, position | ledger, demand, facilities | rise and decline |
| International | match | camp | window | federation upkeep | tournaments | federation quality |

---

## 8. Which systems share which primitives

| Primitive | A Medical | B Training | C Youth | D Staff | E Club | F Comps | G Intl |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| `Ruling` + `Opinion` | return | load | pathway offer | hire, promote | sack, invest | — | release |
| `Estimate` | diagnosis | load risk | potential | — | finances (public vs internal) | — | federation view |
| `Seat` | medical authority | — | — | hiring authority | board room | federation | federation |
| `StaffCareer` | medical staff | coaches | head of youth | yes | director | — | national manager |
| `Region` | — | — | yes | networks | demand | travel, regional leagues | youth structure |
| `Load` | yes | yes | — | — | — | congestion | travel, camp |
| `Extensions` + trailer | yes | yes | yes | yes | yes | — | yes |

---

## 9. Dependency order

**Wave 0. Truth and foundations** (small; unblocks everything)
- **Measure first (read-only):** injuries per player-season by source; rushes and their outcomes; staff attribute mean and p90 by year; staff
  turnover by role; revenue concentration (top decile share) and rank churn over 20 seasons; facility level distribution;
  academy graduates as a share of first-team minutes; share of professionals who came via grassroots vs direct intake (reveals D5);
  amateur count growth; **no team plays twice a day**; no staff at two clubs.
- Fix D1 (adapter-level injuries) and the trivially dead items D3, D4, D13, D14.
- `Extensions` sidecar, trailer format, `PWSAVE02`, golden-save test.
- `Ruling`/`Opinion`/`Estimate` primitives and the domain-nested event variants.
- Observe-only mode pattern for new systems.

**Wave 1. People** (upstream of medical opinions, coaching, academy identity, hiring)
- D7 staff turnover and decline; `StaffCareer`; promotion pipeline; D13.
- `BoardRoom` seats and sack/back `Ruling`; compensation (D8).

**Wave 2. Bodies and training** (share load and opinions)
- Return ladder, opinions, rush decision, hesitation (A).
- `LoadPlan`, `Regime`, familiarity, coach fit (B).

**Wave 3. Money and structure**
- `FinancialPosition`, one ledger, facility decay and competition, `Demand` (E).
- D9, postponement, invariant (F).

**Wave 4. Pipeline** (needs staff, budgets)
- Single talent source (D5), `Region`, graduate ledger, pathway offers, local-club environment (C).

**Wave 5. International** (needs camps, medical opinions, staff careers)
- Eligibility data, federation attributes, release ruling, participation drift (G).

**Wave 6. Only if measurements ask for it:** debt principal/interest, competition prestige, youth promises, sustainability rules.

Safely parallel: Wave 3 competition fixes with Wave 1; Wave 4 `Region` groundwork with Wave 2.

---

## 10. Vertical slices

Each follows INPUT STATE → DECISION / PROCESS → EVENT → CONSEQUENCES → OTHER SYSTEMS. Each slice ends with the smallest test that would prove it; which test tier runs where is your local workflow's call.

### Slice 0. Bodies matter in matches (wiring, no new feature)
- **Input:** a player with fragility 0.6, workload ratio 1.5, fatigue 60, condition 70 starts a match.
- **Process:** OFM adapter scales contact injuries by `injury_risk` and adds a non-contact channel keyed `(seed, HEALTH, fixture, player)`.
- **Event:** `Injured` with `CaseCtx` (mechanism, minute, fatigue, opponent).
- **Consequences:** same downstream case, estimate and press path.
- **Other systems:** selection, valuation, media, rule pressure (`injuries per club` feeds `evolution::rules`).
- **Proof:** injuries rise monotonically with `injury_risk`; the season mean lands in the 1.2–1.8 band after recalibration; observe-only mode leaves the digest unchanged.

### Slice 1. The rushed return
- **Input:** first-choice striker, hamstring, 30% of the case left, a cup final in four days, board satisfaction 38, a Pragmatist manager; physio says "a week"; scientist says 60% ready; player Ambition 17.
- **Process:** `Ruling{Return}` opens; opinions collected; manager decides; medical authority may veto; chance uses **true** remaining fraction and fragility.
- **Event:** `Ruling` → `RushedBack{cause: ruling}` → later `InjurySetback{cause: ruling}` or a normal appearance.
- **Consequences:** success raises the manager's `rush_propensity`; setback creates *overruled* trust memory (physio), *pushed back* (player), fragility, a press story with criticism.
- **Other systems:** selection (limited player in XI), match (stage-capped condition), media/social (criticism), contract (fragility → renewal terms), transfer (medical), board (cup loss).
- **Proof:** a well-reasoned decision that fails lowers propensity; an unreasoned one that succeeds raises it; the ruling reads back with its opinions.

### Slice 2. The congested fortnight
- **Input:** club plays Sat/Tue/Sat, two players on international duty, team workload ratio 1.4, a Rotator manager.
- **Process:** Monday `LoadPlan` picks a recovery-heavy microcycle; scientist and assistant advise; manager decides (`Ruling{LoadPlan}`).
- **Event:** plan change; familiarity drifts down when injuries force a shape change; incident hazard uses plan pressure.
- **Consequences:** injury cluster or none; complaints (existing `Congestion` pressure); execution penalty when the shape changes mid-match.
- **Other systems:** selection rotation, medical, development, media ("injury crisis"), federation rule pressure.
- **Proof:** weeks with two matches carry fewer tactical units; familiarity crossing a threshold raises `execution`.

### Slice 3. The youngster chooses
- **Input:** a 14-year-old invited by two academies in one window; A has the bigger badge, B has a higher graduate-to-first-team rate and is nearer.
- **Process:** family scores by derived pathway rate, head-of-youth trust, relocation cost, education demands, terms; noise; result recorded as `Ruling`.
- **Event:** `AcademyJoined{cause: ruling}`; the rejected club records the snub in its cooldown.
- **Consequences:** B's intake improves over years because pathway rate is measured from actual graduates and decays; A's weak record becomes visible.
- **Other systems:** scouting (rivals watch), press (wonderkid lists), contract timing, first-team pressure via `youth_trust`, board `youth_minutes_target`.
- **Proof:** with the same invited child and different ledgers, the choice flips; ledgers are empty on a legacy save.

### Slice 4. The board turns
- **Input:** mid-table club, target 6th; owner ambitious with high meddling; sporting director's four signings are not playing; supporters protesting; compensation 14 months; thin candidate pool.
- **Process:** monthly seat review: each seat computes a backing with Facts; weighted backing falls; alternative gap vs compensation × frugality decides.
- **Event:** `Ruling{Sack}` or `Ruling{Back}`; `ManagerSacked{cause}` with compensation paid.
- **Consequences:** the appointment uses the seats' preferences; entourage and staff change; supporters and press react; manager CV records how it ended.
- **Other systems:** finance (compensation), market (new manager's `wants`), academy (if the head of youth changes), culture rivalry (`on_manager_move`), social.
- **Proof:** high compensation and a weak pool keep a poor manager; the same manager is sacked when the pool is strong and the owner is frugal-neutral.

### Slice 5. The coach's career
- **Input:** an assistant (38) with four seasons under a successful manager, contract ending, a club one level up has a vacancy, his current chair is reluctant.
- **Process:** both sides decide; the hiring seats score him; if promoted, philosophy = blend(mentor's school, own).
- **Event:** `StaffMoved{cause}`; `CoachPromoted`; mentor edge recorded.
- **Consequences:** the old club loses its group specialist (development environment drops); the new club's style shifts; lineage propagates.
- **Other systems:** development, academy character, scouting network, national manager pool.
- **Proof:** a club that loses its technical coach loses technical growth in the following seasons (measurable through the coach-group weights).

### Slice 6. A window of club versus country
- **Input:** a dual national (22) and club star; a friendly and a qualifier in one window; hamstring at `Individual` stage; a federation with poor scouting reach; a manager who has picked him before.
- **Process:** call-up from imperfect view including federation reach; release `Ruling` weighs match importance, both medical opinions and the relationship.
- **Event:** `CallUp` or `WithdrewFromSquad{cause}`; possible injury on duty (flag exists).
- **Consequences:** caps and reputation, fatigue, club availability, dispute memory that affects later calls; the national side plays a simple shape after a short camp.
- **Other systems:** endorsements via fame, social following, dual-national allegiance, selection.
- **Proof:** the same case with a friendly vs a qualifier gives different release odds; national familiarity is lower than club familiarity for the same manager.

### Slice 7. Relegation shock
- **Input:** a club relegated; wage bill 70% of the new revenue; frugal owner; facilities 12.
- **Process:** `FinancialPosition` shows a short runway; the board sells high earners (austerity exists), cuts the academy budget, under-invests in maintenance.
- **Event:** `Austerity`, `FacilityDecayed`, `SponsorReduced`, each with causes.
- **Consequences:** squad quality falls; academy identity drifts as staff leave; demand falls; possible administration (exists).
- **Other systems:** transfers, youth, staff market, supporters (protests exist), manager pressure.
- **Proof:** a relegated club with equal facilities and a rich owner behaves differently from a frugal one; facility levels are not monotone in a 20-season run.

---

## 11. Extend, refactor, replace, leave

| Item | Verdict | Why |
|---|---|---|
| `Case`, estimate/certainty, fragility, chronic | **Extend** | Right shape; add sidecar and ladder |
| `available()` binary | **Refactor** to tri-state | Wide caller set (`grep available()`), do carefully |
| OFM injury path | **Extend at adapter** | Vendor stays unmodified |
| Native engine injury path | **Leave** | Already uses `injury_risk` |
| `TrainingPlan` (player) | **Extend** | Keep; add team `LoadPlan` and `Regime` |
| `health::team_days` constants | **Replace** with plan lookup | Same cost |
| `team_environment` (`max`) | **Refactor** to unit-based | Same per-player cost |
| `Team.familiarity_weeks` | **Replace** | Dead field, wrong shape |
| `youth::*` pipeline | **Extend** (canonical) | Richest, has the right knowledge model |
| `people::youth_intake` | **Replace** (draw from region pools) or shrink | D5 |
| `AcademyStyle`, `Reach` | **Refactor** to derived | Currently inert |
| `ManagerProfile` | **Extend** into `StaffCareer` | Keep the fields; add contexts |
| Non-manager staff | **Extend** (turnover) | D7 |
| `Board.satisfaction` | **Keep as cache**; replace as source of truth | S12 |
| `Governance` | **Extend** | Seats read it |
| `club_revenue` | **Refactor** (ledger, remove double count) | D10 |
| `Facilities` struct | **Leave layout**; add sidecar | Save compat |
| `finance::weekly` | **Extend** | Ledger and compensation |
| `Payable` | **Leave** | Works; read by `FinancialPosition` |
| `continental_entrants` | **Refactor** | D9 |
| `evolution::rules` | **Leave** | Sound; only add if a system needs a rule |
| `intl::call_up`, `fed_view` | **Extend** | Add federation reach and release ruling |
| `intl::eligible_nations` | **Replace** with rule-driven | Data rule |
| Referees, agents, awards, records, minor football | **Leave** | Deep enough |
| `save.rs` | **Extend** (trailer) | §6.3 |

---

## 12. What not to build

| Idea from the brief | Why not |
|---|---|
| Session-by-session training schedule per player or per team | Minigame for a role nobody inhabits; team-week plans give the same causal edges |
| Hidden per-player "optimal load" variable | 300k stored hidden numbers; derive from natural fitness, age, injury history |
| Anatomical tissue model, injury grades, per-muscle rehab progress | Adds text, no decisions; body-region wear plus fragility already carry the mechanics |
| Rehab compliance and support-network as separate systems | Professionalism and facility quality already capture it |
| Universal psychological injury effects | Sparse hesitation only (brief itself says "not universal") |
| Insurance market | Wage-risk in `FinancialPosition` is enough |
| Board members as new persons at every club | Each person costs a `Life` and may spawn a partner; shifts ids; use seats over existing persons |
| Stadium calendars and travel simulation for all clubs | Only for real conflicts (shared grounds, congestion); a coarse distance from region is enough |
| Class or wealth simulation for youth | Overdetermines careers; parents' support and move cost exist |
| A second referee or VAR layer | Engine cannot express it |
| Randomised rule changes | Already institutional and cooled down |
| Full financial-regulation engine | One data-driven sustainability profile at most, later |
| Real geography / GIS | Not available in the import; coarse `Region` is enough |
| Per-audience stored copies of each fact (public / club / player) | Store truth plus estimate; derive audience views |

---

## 13. Conflicts with parallel work and API requirements

The local checkout has UI, testing and other simulation changes I cannot see. Likely collision points in the remote, and how to avoid them:

| Where | Risk | Mitigation |
|---|---|---|
| `crates/pw-sim/src/lib.rs` (daily pipeline) | Reordering or new hooks collide | Add hooks in one contiguous block; never reorder existing lines |
| `crates/pw-world/src/world.rs` (`World`) | Any field change collides | Exactly one new field (`ext`) |
| `crates/pw-world/src/event.rs` (1,177 lines; exhaustive matches in `pw-view/src/narrative.rs:40-110` and `pw-narrate/src/events.rs`) | New variants force edits in the UI-adjacent crate | One nested variant per domain; the two match arms are the only `pw-view` edits, done last and tiny |
| `crates/pw-core/src/rng.rs` (`stream::ALL`, length 39) | Both sides add streams | Pick ids far apart (0x30+) and add at the end |
| `data/engine/*.toml`, `pw-data` tuning structs | Both sides tune | New files (`training.toml`), no edits to existing ones |
| `PROGRESS.md` | Both sides append | Do not edit; notes go in `docs/` |
| `crates/pw-cli/tests/world.rs` | Testing workflow rewrites | New tests in new files |
| `crates/pw-match/src/ofm.rs` | Low, but the match adapter may be edited locally | Confirm before Slice 0 |

**API/UI requirements (documented, not designed):** the client will want, when these land: (a) return stage and per-role
opinions on a player's injury page; (b) a public bulletin vs club view of an injury; (c) a `why` view that resolves a `Ruling` event into
its opinions and outcome; (d) board seats and each seat's backing on the club board page; (e) staff career pages; (f) academy identity and
pathway record on the club youth page; (g) region on the nation page; (h) federation attributes; (i) a training-plan page that shows the team's week.
Insight notes in `pw-view/src/pages/insights.rs` already read `medical` state; new sidecars would need adapter functions there.

---

## 14. Critique of the brief

1. **It is a very large list for a repository that has not yet measured its own balance.** `INTEGRATION_REPORT` §13 says economy, wages, reputation
   and fame are unmeasured over 50 seasons; injuries are at 0.65 per player-season against a target of 1.2–1.8; rates "need targets
   before release". Every new state variable needs calibration. Measure first, then add.
2. **Several "new" systems already exist under other names.** The brief reads as if youth, national teams and boards are open ground. They are
   not, and a second version would duplicate (D5 is the warning).
3. **Disagreeing doctors, physios, managers and players will mostly be seen through consequences and text**, because no human can
   inhabit those roles (PROGRESS §3.10). The value is real (AI-world coherence, inbox stories, causal memory), but do not build deliberation
   UI or wide role models. Two medical voices are enough.
4. **The injury list has about 12 dimensions but only 4 change outcomes:** a staged return, a rush decision under disagreement,
   uncertain diagnosis (done), and body-wear history reaching matches (D1). Mechanism, pain, compliance and support network give text more than
   decisions. Take the cheap version.
5. **"Different systems, different time scales" is right, but "who knows" should not become four copies of every fact.**
   Store truth and one estimate; derive the public, club and player views.
6. **"Optimal load differs by player" invites a stored hidden variable per player.** Derive it.
7. **Board members as full people is the most expensive idea in the brief** relative to its value, because of `Life`, partners and id shifts.
   Seats over existing persons give the same disagreement.
8. **"International success → youth participation" is easy to make a hidden buff.** Do it as a slow, capped, lagged drift on regions, or not at all.
9. **The brief does not mention saves or measurement beyond the closing notes, but they are the critical path.** Without the sidecar and trailer, none of it can
   ship on an existing save; without Wave 0 metrics you cannot tell whether a change helped.
10. **Where I disagree with the framing "outcome ≠ quality of decision":** it is the right principle, but it needs a stored record of what was
    known and advised at decision time, or nothing can later show that a good decision failed. That is why `Ruling` is in the foundation
    and not an afterthought.
11. **An idea I would add beyond the brief:** invariants and dashboards (no double-booked days, no person at two clubs, staff quality drift,
    graduate share of minutes). Several defects in §3 would have been caught by them.

---

## 15. Decisions I need from you before any code

1. **D1 fix:** is the OFM adapter safe for me to edit, or has the local checkout already changed `ofm.rs` or the injury path? Preferred option is 7A.1(a). It will shift injury rates and needs a recalibration pass.
2. **D5:** which youth source is canonical? I recommend `youth.rs` with `people::youth_intake` reduced to a region-pool top-up.
3. **Sidecar and trailer:** is `Extensions` acceptable given local changes to `World`? If the local checkout already touched `save.rs` or `World`, I should start from that.
4. **Sacking compensation (D8):** a real economy change. Do you want it on by default?
5. **Board seats over existing persons** (not new persons): agreed?
6. **Measurement first:** may Wave 0 metrics land as a read-only report before any behaviour change? They need no world changes, only a new report function.
7. **Scope check:** the seven slices are ordered by dependency, not size. Slices 0, 1 and 4 give the most visible coherence for the least code; would you want to start there?
