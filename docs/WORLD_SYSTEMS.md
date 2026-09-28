# World Systems — what exists, how it connects, what was assumed

Companion to `PRODUCT_NORTH_STAR.md`, `SYSTEMIC_SIMULATION_RULES.md` (S1–S27) and
`ANTI_LINEAR_DESIGN_CHECKLIST.md`. This file lists the systems of the breadth pass
(commits `2d4fc60` … `2df1756`), the consequence chains that link them, the major
assumptions made, and the known risks. **None of it has been compiled yet.**

Rule of thumb used throughout: world state → decisions/events → knowledge → text.
A system may *read* another system's state; it changes the world only by writing its
own state, pushing events (with causes) or submitting intents/decisions that the owning
system applies. `Person.mind` is read only for decision routing (AI choice now vs a
`Decision` for a human) and level of detail.

---

## 1. Systems (world state → sim module)

| Area | World state | Simulation | Runs |
|---|---|---|---|
| Rules & registration | `data/engine/rules.toml`, `pw-data/rules.rs`, `pw-world/rules.rs` | checks in `can_sign`, `can_loan`, selection pools, negotiation completion | on demand |
| Economy | `governance::Economy` | `economy.rs` | yearly (Jul 1), club revenue on demand |
| Owners, boards, projects | `governance::Governance` | `governance.rs`, `board.rs` | monthly, yearly, weekly |
| Manager careers | `careers::Careers` | `managers.rs` | monthly, yearly, on hire/fire |
| Scouting network | `scouting::Scouting` | `scouting.rs` (via `perception`) | monthly assign, weekly reports |
| Squad planning | `deals::SquadPlan` | `planning.rs` | monthly (after statuses) |
| Club-to-club deals | `deals::Deals` | `deals.rs` | daily talks, monthly payables, Monday recalls/pre-contracts/trials |
| Youth pipeline | `youth::Youth` | `youth.rs` | weekly play/scouting, monthly school, June reviews, Sept cohorts |
| National teams | `intl::Intl` | `intl.rs` | daily windows/tournaments, monthly federation upkeep |
| Medical history | `medical::Medical` | `medical.rs` (from `health`) | on injury, weekly, monthly |
| Development depth | `growth::Growth` | `growth.rs` | monthly |
| Dressing rooms | `dressing::Rooms` | `dressing.rs` | weekly integration, monthly structure |
| Performance records | `perf::Perf` | `interpret.rs` (from `matchday`) | per match, monthly readings |
| Honours & records | `honours::Honours`, `history` | `honours.rs` (+ `season`) | per match/transfer/cap, monthly, Dec 20 votes |
| Renown | `renown::Renowns` | `renown.rs` | monthly |
| Press expansion | `media::{StoryLink, Quote, Stance}` | `press.rs` (from `media::weekly`) | weekly |
| Personal affairs | `affairs::AffairsBook` | `affairs.rs` | monthly + intents |
| Brands & sponsorship | `commerce::Commerce` | `commerce.rs` | Jul 1 club deals, monthly offers/reviews |

Earlier systems still in place and extended, not rewritten: seasons, matchday,
selection, health, development, perception, market, contracts, negotiation, talks,
social, morale, life, minds/intents, agents, media, staffing, finance, reputation.

---

## 2. Cross-system consequence chains

These are the routes by which one system's state ends up changing another's. Each is
implemented as a read or a hook, not a script.

**Football pyramid**
- Grassroots ability → academy scouts' `judged_potential` (with maturity bias) → trials
  → academy → scholarship/release (June reviews) → pro contract (`contracts::weekly` via
  `worth_pro_contract`) → first-team squad planning → loans (planning's development
  targets) → stagnation if minutes stay low (`growth`: potential erodes).
- Released academy players → local clubs / amateur football → adult amateur trials at
  pro clubs (`deals::trials`) → second chances.

**Information**
- Club scouts' assignments (briefs from squad needs, nations, youth, competitions) →
  reports with biases → `scouting::view` → market searches and shortlists → bids.
- International matches → scouts with nation/youth/player briefs observe; finals are
  watched by big clubs → knowledge → transfer interest.
- Wonderkid lists and "underrated" analysis pieces → big clubs observe the players → the
  market reacts to media.
- Agents pitch clients → clubs gain evidence; pundits (ex-players) become journalists
  whose sources are people who like them → real leaks → rumours → beliefs.

**Money**
- Nation economy (growth, inflation, broadcast deals) → club revenue, wage index, values
  → budgets → wage ceilings in negotiation → which moves happen.
- Owners (patient, ambitious, asset-strippers) → selling stance, wage ceilings, project
  investment, sacking temper; administration → points deduction → relegation → revenue.
- Brands (budgets follow the economy) → club shirt/kit deals → club finance; personal
  endorsements → player income (club image-rights share → club finance) → lifestyle,
  savings, investments.
- Record fees → record-signing stress; add-ons and sell-ons pay out as events happen.

**People and feelings**
- Training ratings, minutes vs promised status, promises, conversations, press quotes,
  injuries on duty, withdrawals, dressing-room leaders → memories → trust/affinity →
  selection trust, morale, meeting outcomes, transfer requests, board's sense of the
  dressing room (backing < 30 lowers board satisfaction) → sackings → manager careers
  (entourages follow, favourites get signed) → the new manager's preferences.
- Dressing room: group leaders pull members' trust in the manager; departing leaders
  lower friends' morale; newcomers' integration → `Settling` morale factor.
- Press: managers praise/back/criticise players by how they read them and their own
  man-management and temper; players complain, talk up ambition or declare loyalty from
  their situation → memories on the people named, teammates close ranks, fans and image
  move.

**Bodies**
- Workload, fatigue, wear, age, well-being, medical fragility, chronic conditions and a
  personal trainer/chef → injury hazard → injuries (club or country) → diagnosis
  estimate (quality of physios/facilities) → treatment choice (player's mind/decision) →
  setbacks / rushed returns (club need, board pressure, manager archetype, player's
  willingness) → fragility → next injury; chronic conditions → selection rests the
  player more.
- Injured on international duty → club manager remembers national manager (LetDown).

**Careers and reputation**
- Caps → world reputation (weekly drift target), market value premium, work-permit points
  (nation strength via Elo), dressing-room influence, awards candidacy.
- Per-appearance records → lenses (manager by archetype, fans, media, analyst, scout) →
  labels → press features → fans / fame / clubs' knowledge.
- Awards → reputation, fame, confidence; legends → fan love and ambassador careers; hall
  of fame → fame.
- Renown (fame) → endorsements, stress, pundit/ambassador routes; image → endorsements
  end on scandal (brand sensitivity).
- Coaching badges → staff attributes, licence gates for manager/assistant/coach hiring
  → who can become a manager → manager careers.

**Life**
- Relocation (transfer/loan/job) → partner decides, languages, home sold/lost, local
  helpers dropped, cost of living changes → spending floor, housing cost → savings/debt →
  stress → well-being → training → coaches' view.

---

## 3. Control-any-person check

Every new choice has one function that applies it, reachable from an AI mind and from a
human's intent or decision:

| Choice | AI path | Human path |
|---|---|---|
| Dual-nationality allegiance | `intl::preferred_nation` → `answer_call` | `DecisionKind::NationChoice`, `Intent::DeclareForNation` |
| Retire from international football | `intl::veterans_consider` submits the intent | `Intent::RetireFromInternational` |
| Surgery vs rehab | `medical::ai_prefers_surgery` | `DecisionKind::Treatment` |
| Playing through pain | personality in `medical::weekly` | `Intent::PlayThroughPain` |
| Mentoring | `growth::mentoring` | `Intent::Mentor` (the mentee decides) |
| Speaking to the press | `press::press_conferences`, `player_interviews` | `Intent::SpeakToPress` |
| Study, home, helpers, giving, investing | `affairs::ai_choices` | `Intent::{Enrol, MoveHome, HireHelper, DismissHelper, SetGiving, StartFoundation, Invest}` |
| Post-playing career | `affairs::ai_next_step` via `mind` → intent | `Intent::PursueCareer`, `Intent::LeaveCareer` |
| Endorsements | `commerce::ai_accepts` | `DecisionKind::Endorsement` |
| Join amateur football | `youth::drift_to_amateur` | `Intent::JoinAmateurFootball` |
| Trials | youth/deals trial logic | `DecisionKind::Trial` |

Known exceptions (documented, to fix later): national managers' squad selection,
federations' hiring and club boards/owners have no human path yet (nobody can inhabit a
federation or a board as a *role*; the person can be inhabited but the role logic is AI).

---

## 4. Major assumptions

Football structure
- One confederation per nation from nation data; continental finals in years ≡ 0 (mod 4),
  world tournament in years ≡ 2 (mod 4); qualifying in the Sep/Oct/Nov windows of the
  previous year (groups of 3–4, double round robin); finals of 4/8/16/32 with groups of
  four, top two to knockouts. Youth national sides play friendlies only.
- A competitive senior cap locks a player to a nation; declaring is also permanent.
- Registration: rules profiles are generic (`default`, `uefa_bloc`, `points_permit`,
  `foreigner_cap`) and assigned per confederation/nation in `rules.toml`; they are
  plausible, not real-world exact.
- Grassroots/academy football below U18 is statistical (`youth::play_group`), not engine
  matches.
- Club shirt and kit deals only (no stadium naming/sleeve deals generated yet, though the
  types exist).

Numbers (all tunable, none calibrated)
- Scales: reputation 0–10,000, fame 0–10,000, image −1,000..1,000, fan standing
  −1,000..1,000, most "quality" values 1–20.
- Coaching licence required by club reputation: ≥6000 Pro, ≥3000 A, ≥1200 B, else C.
  Staff with ≥100 games managed and people who never played professionally (imported
  staff) count as fully licensed.
- Injury: recurrence within a year ×1.3 duration; fragility decays 7% a month;
  chronic when region wear ≥70 or ≥3 injuries in a region; surgery +15% time.
- Stagnation: 17–23-year-olds with <120 minutes per 4 weeks for 26+ weeks lose 1 PA a
  month, up to 12.
- Endorsement value ∝ (0.6·fame + 0.4·world rep)² × brand size; big clubs (rep ≥ 7000)
  take 20% image rights.
- Elo for nations on the 0–10,000 scale with K = 40/80/120/160 by match type.

Scale choices
- Per-player appearance history keeps the last 10 apps plus season lines; season lines
  grow for the life of the save (≈ 1 per player-season-club).
- Growth records are kept only for players ≤ 27.
- Renown is only tracked for people who have become known (rep ≥ 1500 or already known).
- Story links and stories are never compacted (same as before).

---

## 5. Architectural risks noticed

1. **Uncompiled volume.** ~11k lines since the last compile. Expect borrow-checker work
   in the new modules (closures capturing `w` alongside iteration over `w.*` fields),
   exhaustive-match fallout from new `EventKind`/`DecisionKind`/`Intent`/`Fact` variants
   in any `match` I did not find, and small type mismatches (integer widths in
   `saturating_*` chains, `&&T` vs `&T` in closures).
2. **Event log growth.** International squads, milestones, awards, press and commerce add
   many public events per year. `EventLog` needs compaction/archiving by age and salience
   before 20-season runs.
3. **O(n) scans in monthly systems.** `intl::pools` and `affairs`/`renown` iterate all
   players/people monthly; `honours::legends_and_hall` scans tallies; `press` scans
   perf maps; `commerce::offers` iterates renown. Fine at 50k players, needs indexing at
   300k (e.g. a per-nation eligibility index, a fame index).
4. **Double-counting risk in morale.** `dressing::mood_inputs` pushes a second `Manager`
   factor; `Settling` now comes from both life (`settledness`) and the dressing room.
   Needs a balance pass so unhappy newcomers are not punished twice.
5. **Money flows.** Endorsement income passes through `life::finances` taxed; club
   image-rights shares, sponsorship and ambassador wages move club balances directly
   (outside `finance::weekly`). Budgets are recomputed from season revenue, so these are
   partly invisible to planning until the next season. Needs one ledger.
6. **National manager identity.** National managers are ordinary staff with `club = NONE`
   tracked in `intl.managers`; every system that treats "unemployed manager" must filter
   them (done for board, staffing, finances). Missed filters would let clubs hire a
   sitting national manager.
7. **Determinism.** New systems use keyed RNG throughout, but several iterate
   `FxHashMap`s and act in iteration order (e.g. `commerce::offers`, `renown`,
   `interpret::monthly`). Where the action touches shared budgets or events, results
   depend on hash order — deterministic for a given build, but fragile. Sort keys before
   acting where order matters.
8. **Text still assumes some genders** in older narration; press text was made neutral in
   this pass; the rest should follow.

---

## 6. Deliberately shallow (documented, not hidden)

- Youth international tournaments; club-versus-country release negotiations beyond
  "club pressure withdraws a player carrying a knock".
- Personal staff are effects, not people (no chef/trainer persons with relationships).
- Businesses are an investment random walk with a skill bonus, not companies.
- Housing is kind + quality + money; no neighbourhoods, commutes or property market.
- Charity has no specific causes or beneficiaries.
- Brand deals: obligations are a stress cost only; no appearance scheduling or
  campaigns; no brand-vs-brand rivalry beyond sector exclusivity.
- Manager-of-the-season and world awards are single-pass votes; no shortlists or
  ceremonies.
- Retrospectives cover title anniversaries, legends and the hall of fame only.
- Stadium naming/sleeve sponsorship types exist but are not generated.
- Pundits publish through the existing rumour/news machinery; they do not yet produce
  opinion pieces of their own.
