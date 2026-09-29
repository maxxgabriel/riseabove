# Pathway — Progress, Next Steps, and What's Needed From You

Last updated: 2026-09-28. This is the living log for the project: what the game is, what is built,
what state each piece is in, what comes next (in order), and what is blocked on you.

---

## 1. What we are building (agreed)

**Football Manager, but you manage one player's life instead of a club — with FM's width and depth, plus a life sim.**

- The world is a full, autonomous football ecosystem at FM scale (target ~300,000 simulated players):
  clubs, boards, finances, scouting, transfers, loans, contracts, youth intakes, managers hired and
  sacked, leagues, cups, continental competitions, promotion/relegation — all running every day
  whether or not you are involved.
- You control one ordinary player (the protagonist). The world never bends for you (fairness, P1):
  you are a normal `Player` whose person has an `External` mind; every rule that applies to you applies
  to every AI player.
- Depth comes from interacting systems with memory, not scripted content:
  training choices, fitness and body wear, how coaches perceive you, relationships with the manager,
  coaches, teammates, agent, family, partner and media; promises made and broken; contract negotiations;
  transfer sagas; money, housing, education, health and public image.
- **Not linear.** There is no "play through and retire" arc and no end screen. Stories emerge from systems
  colliding. Retirement will be a transition (the person continues), design deferred.

Your design choices (2026-09-28):

| Question | Choice |
|---|---|
| Day loop | **FM-style planning**: set weekly plans/routines across deep screens, react to events and conversations, then advance. Every day still resolves in full detail underneath. |
| Matches | **Watch only.** You influence matches through preparation, fitness, relationships and pre-match choices. No in-match control. |
| After retirement | **Decide later.** Build the playing career and life sim first. Retirement is a hook, not an ending. |
| World data | **Your FM23 export**, converted by you into our import format (§6). No fictional world. |
| Match engine | Use something that already works: OpenFootManager's engine is vendored and wrapped (§3.4). |

---

## 2. Decisions log

| Date | Decision | Why |
|------|----------|-----|
| 2026-09-28 | Clean Rust workspace in this repo instead of forking open-football | 300k-player scale and P1/P3/P6 (fairness, perception, determinism) need ID-indexed hot/cold stores, keyed RNG and ordered parallel merges from day one. open-football (Apache-2) stays usable as an algorithm reference with attribution. |
| 2026-09-28 | World data comes from your FM23 export, not a generated fictional world | Your decision. The engine loads a documented import format (`data/IMPORT_FORMAT.md`). Regens (new youth players) take names from each nation's imported players. A small synthetic world exists **only** for tests and performance benchmarks. |
| 2026-09-28 | Engine rules/tuning data (formations, position weights, age curves, injuries, calendars, tuning constants) ship in `data/engine/*.toml` | These are simulation rules, not world data (P7 data-driven, moddable). |
| 2026-09-28 | Default match backend = OpenFootManager's engine, vendored in `vendor/ofm-engine` (unmodified source, GPL-3) behind `pw_match::simulate` | Your direction: use something already working. Measured through our adapter: **2.5 ms/match**, deterministic when seeded, calibrated to goals 2.9 / shots 23.5 / on target 9.2 / fouls 20.6 / yellows 3.5 / reds 0.12 / corners 9.2 / subs 5.4 / home win 44% / draw 25% / average rating 6.76 (2,000-match run). |
| 2026-09-28 | Rejected open-football's engine as the match backend | Measured at **3.9 s/match** single-threaded → ~10 minutes per simulated weekend at our scale (budget: 6 s). Its own bench notes residual non-determinism. Using it only for your matches would break fairness (every match must use the same outcome model). |
| 2026-09-28 | Our own zone/possession-chain engine kept in-tree as `backend = "native"` | Richer per-action events; not yet calibrated (was producing too many shots). Can replace OFM later without touching anything else. |
| 2026-09-28 | Saves = bincode + lz4 (pure Rust), crash-safe temp-file + rename | No C toolchain needed. History lives in the save; an SQLite history DB can be added behind the same API later. |
| 2026-09-28 | Relationships, promises, conversations and contract negotiations live in the **world** (for everyone), not only the protagonist layer | Fairness (P1) and depth: a manager's trust in any AI midfielder evolves by the same rules as his trust in you. |

**License note (important):** the vendored OFM engine is GPL-3. For personal/local use this is fine. If Pathway
is ever distributed, GPL-3 would apply to the whole program — or the native engine must replace it first.

---

## 3. What exists — crate by crate

Legend: ✅ written and verified (compiled/tested) · 🟡 written, compiled earlier, changed since without re-verifying ·
⚠️ written, never compiled · ⬜ not started.

Verification rule from you: **I write code only; building, testing and verification happen when you ask.**
So everything touched after the last verification is marked 🟡/⚠️ and must be checked before it's trusted.

### 3.1 `crates/pw-core` — shared primitives — ✅ (+ one small unverified edit)
- Typed ids (`PlayerId`, `ClubId`, `TeamId`, …, new `TalkId` ⚠️) and `IdVec` (typed dense vectors that deref to slices for rayon).
- `Date` (days since 1970, leap-safe ages: 29 Feb birthdays age on 1 March), weekday/month helpers.
- Keyed counter RNG: every system derives its stream from `(world seed, system, entity, date…)` — no global RNG,
  results never depend on thread scheduling (P6 determinism).
- Deterministic `exp`/`ln`/`sigmoid` (no platform libm in outcome paths).
- Football vocabulary: 46 visible attributes (1–20, fixed-point), 13 hidden personality attributes, 18 staff attributes,
  14 positions, 24 tactical roles with behaviour profiles and key attributes, tactics, 26 player traits.
- Tests: date round-trip, RNG determinism, math accuracy — all passed.

### 3.2 `crates/pw-data` + `data/engine/*.toml` — engine rules — ✅ (+ tuning fields added 🟡)
- Formations (9), CA weights per position (FM-style, each row sums to 1), age curves per attribute group
  (speed peaks early, mental grows into the 30s), injury catalogue (29 injuries: minor knocks to ACL/Achilles ruptures,
  illnesses, with days-out ranges, body-region wear and possible permanent attribute loss), calendars
  (autumn–spring, with winter break, calendar-year, Nordic) and international windows, tuning constants.
- Match tuning now includes `backend`, `ofm_conversion`, `ofm_fatigue`. Mods can override any file.

### 3.3 `crates/pw-world` — the world state — 🟡 (compiled earlier; social/negotiation/plan added since ⚠️)
Everything the simulation reads and writes, sized for 300k players:
- People (permanent identities; roles change over a lifetime), players split into **hot** (daily: condition, sharpness,
  fatigue, morale, confidence, well-being, injury, bans, workload EWMAs, form) and **cold** (attributes, CA/PA, positions,
  feet, body wear, contract, loan, value, reputation, squad status, career counters, training plan) stores.
- Staff with roles and philosophies (formations, mentality, pressing, tempo, youth trust, archetype), clubs (finance,
  facilities, board, needs, fan mood), teams (first/reserve/U21/U19/U18), nations with live season dates and windows,
  competitions (league, cup, continental, groups, knockout ties, round dates), fixtures (compactable), interned names.
- Perception (P3): clubs store only *evidence* (minutes observed, when); estimates are `truth + σ·stable bias`, so they
  converge with evidence and never flicker. No per-attribute storage for 300k×clubs pairs.
- Events with visibility (public / club-internal / private to a person), history (season stat lines per player,
  honours, awards, archived tables, club spells), current-season stats, decision queue, market book, match reports.
- Explainable rule checks (why you can't play/sign: window closed, squad full, suspended, injured, minors abroad…).
- **New, unverified ⚠️:** `social.rs` — directed relationships (affinity/trust/respect), memories ("broke a promise",
  "argued", "backed", "mentored"…), promise ledger (minutes, status, new contract, let-leave, position, loan, captaincy),
  compatibility for first impressions. `negotiation.rs` — full contract terms (wage, years, signing fee, appearance/goal/
  clean-sheet bonuses, release clause, status promise, yearly rise, relegation cut, sell-on to player) and a multi-round
  talks state machine. `TrainingPlan` on every player (focus: general/group/attribute/position; intensity; extra and
  recovery sessions). World now holds `social` and `talks`.

### 3.4 `crates/pw-match` + `vendor/ofm-engine` — match simulation — ✅
- One interface: `simulate(&MatchInput) -> MatchResult` with full per-player lines (minutes, position, rating, goals,
  assists, shots, xG, passes, tackles, interceptions, dribbles, crosses, clearances, blocks, cards, saves, conceded,
  injuries, condition at the end, sub timings) and an event stream (key events always; commentary-level events for
  matches involving watched teams).
- OFM adapter: maps our 46 attributes onto OFM's 19, derives OFM's archetype traits from attributes, drives OFM's
  live-match state minute by minute with its own in-game AI managers (throttled), adds planned substitutions,
  handles extra time and shootouts (including aggregate ties), converts OFM's report, and computes Pathway ratings.
- Native engine (alternative backend): zone grid, possession chains, ball-aware team shapes, three-part pass model,
  take-ons, xG, set pieces, cards, fatigue, subs, injuries, ET, shootouts. Not calibrated yet.
- Tests: determinism, completeness (≥22 players rated, decisive matches produce a winner), stronger side wins more,
  calibration report (ignored test, run on demand).

### 3.5 `crates/pw-sim` — the living world — 🟡 (compiled with zero warnings; small edits since)
Daily pipeline in the plan's order: seasons & draws → contracts/loans → people (intake, retirement) → monthly
(valuation, squad statuses, squad planning, knowledge forgetting, AI life model, vacancies) → board (Mondays) →
training & health → market → decisions → matches → weekly aftermath (development, scouting, morale, reputation,
finances) → yearly compaction.
- **Seasons:** per-nation calendars, round-robin fixtures spread over Saturdays avoiding international windows and winter
  breaks, domestic cups with byes and midweek rounds, continental group stages + knockouts (entrants from last season's
  tables by nation strength), knockout progression, titles, prize money, awards (top scorer, player/young player of the
  season), promotion/relegation that keeps division sizes constant and blocks B-teams from their first team's division.
- **Matchday:** parallel selection + simulation, applied in fixture order (deterministic). Updates tables, ties
  (aggregate, away goals, pens), condition, sharpness, form, workload, morale, confidence, yellow accumulation, bans
  (second yellow 1, straight red 3), match injuries from the catalogue, stats, perception for both clubs, debuts and first
  goals, manager records, fan mood, gate receipts, and stores full reports for watched teams.
- **Selection:** manager-perspective scoring (perceived ability per slot, form incl. training, fitness, role fit, trust,
  youth policy, rotation, seeded noise by consistency) solved exactly with a Hungarian assignment; formation chosen from
  the manager's two preferences; bench guarantees a keeper; captain by leadership. Selection **forecast** for any player.
- **Health:** team day types (match, after, before, training, rest, off-season), acute:chronic workload, fatigue debt,
  condition recovery (natural fitness, age, well-being), sharpness decay, training ratings (what coaches see), training
  injuries and illness, rehab speed from medical facilities, body wear and permanent losses.
- **Development (weekly):** potential room × age curve (with biological maturity offset) × (coaching + facilities +
  match minutes) × well-being × professionalism, position emphasis, decline with age/natural fitness/wear, weekly caps,
  CA never exceeds PA, one-time potential re-roll around 19 (late bloomers).
- **Perception:** coaches observe their squads weekly, scouts sample players across nations by strength, stale thin
  evidence forgotten; `club_view` gives a club's CA/PA estimate with bands.
- **Market:** valuation, wage demands, squad planning (depth + quality needs), perception-driven searches, asking
  prices by status/contract, bids and rejections, AI and external player decisions, transfers, development loans,
  loan returns, free-agent signings.
- **Contracts:** expiry at contract end, releases, weekly renewal approaches staggered per player.
- **People:** youth intakes each year (count and quality from facilities, nation youth rating, club reputation; rare
  gems), retirements (age, ability, free agency, keepers later, long injuries), some retirees become coaches/managers.
- **Board:** targets from reputation rank, weekly satisfaction, warnings, sackings, appointments (unemployed managers by
  fit, assistant promotion, newly qualified coach).
- **Finance:** season revenue by reputation/economy, wage and transfer budgets, weekly wages (loan splits), fees.
- **Reputation / morale / AI life model:** weekly reputation drift, morale from playing time vs status, monthly
  well-being for AI minds (the same channel the protagonist's life sim writes to — fairness).
- **Save/load:** generic (`save`/`load` any serializable game state).
- First end-to-end run (synthetic 4,200 players, 400 days): **no panics, 2.7 ms per simulated day**, 1,502 matches,
  2.35 goals/match, titles, 1,416 renewals, 313 loans, 75 retirements, 64 youth intakes.
  It exposed: **zero transfers** (squad-size rule counted youth sides → fixed, unverified), **35 sackings in one season**
  (boards retuned, unverified), injuries ~0.65 per player-season (below the 1.2–1.8 target → tune later).

### 3.6 `crates/pw-import` — world building — 🟡
- `load_dir(folder)`: reads the CSVs in `data/IMPORT_FORMAT.md` (nations, competitions, clubs, players, optional staff,
  optional explicit entrants, optional world.toml). Tolerant: FM position notation (`AM (RLC), ST (C)`), FM negative
  PA ranges, 1–100 attribute files auto-scaled, missing CA/PA/hidden/contracts/reputation/staff derived deterministically,
  FM's CA→PA headroom preserved on our CA scale, unknown nationalities created as minor nations, loans supported,
  regional same-tier leagues hang off the promotion chain.
- Synthetic fixture (tests/benchmarks only): `tiny`, `small`, `huge` (~300k players).

### 3.7 `crates/pw-cli` — headless runner — 🟡
`pathway-sim synth [tiny|small|huge] [--days N] [--save F]`, `import DIR [--days N] [--save F]`, `run F --days N`,
`report F`. Prints timing per 30 days, match/goal/home/draw stats, event counts (transfers, loans, renewals,
retirements, injuries, sackings, bids), top-division tables and scorers.

### 3.8 The systemic life layer — ⚠️ written 2026-09-28, never compiled
Built to `docs/PRODUCT_NORTH_STAR.md` and `docs/SYSTEMIC_SIMULATION_RULES.md`. Everything below runs for **every
person in the world**, whether or not anyone is inhabited; the only reads of `Person.mind` are decision routing
(who answers) and recording level of detail.

- **Causal events** (`pw-world/event.rs`): every event has an id, visibility and a `causes` list (earlier events or
  typed facts such as "training below par for 3 weeks", "share of minutes 20% vs 60% expected", "club X has been
  watching"). `why <event>` in the client renders them.
- **Memories** (`pw-world/social.rs`): typed, dated, sourced episodes with salience that fades by kind and by the
  person's grudge-holding; formative ones never fully fade. Relationship numbers are a summary moved by memories.
- **Beliefs** (`pw-world/beliefs.rs`): what each person has been told, by whom, through which channel, how sure.
- **Life** (`pw-world/life.rs`, `pw-sim/life.rs`): one model for everyone — home and languages, partner as a real
  person (dating → living together → marriage, or separation — both sides decide), children, ageing parents,
  money (tax, lifestyle, family support, debt), routine hours, stress, sleep, fulfilment, well-being with reasons.
  Relocation: the partner decides for themselves whether to come.
- **Considerations** (`pw-sim/consider.rs`): the shared factor library decisions draw on.
- **Social dynamics** (`pw-sim/social.rs`): coaches notice training streaks (coach notes + memories), teammates bond
  or become rivals, promises come due and are kept or broken, influential unhappy players spread unrest, managers
  pick captains.
- **Conversations** (`pw-sim/talk.rs`): one resolver for everyone. Managers summon players for real reasons; players
  ask for minutes, feedback, contracts, loans, to leave, to follow up promises. Tones land differently on
  different people; outcomes (promises, deferrals, refusals, fines, listings, fall-outs) come from both people's
  state and history.
- **Contract talks** (`pw-sim/negotiation.rs`): multi-round for every renewal, transfer, free-agent and first-pro
  deal; club ceilings, agent skill, walk-aways, deadlines. Replaces the old accept/reject.
- **AI minds** (`pw-sim/mind.rs`) and **intents** (`pw-sim/intents.rs`): AI people act on their own initiative
  through the same intents a human uses (meetings, transfer requests, agents, training plans, routines,
  retirement, staff jobs). Retirement is a choice for everyone; nobody's world ends.
- **Agents** (`pw-sim/agents.rs`): agents are people with networks, honesty and greed; they pitch clients (clubs gain
  real evidence), hear about interest through their ties, and pass on what they choose to.
- **Press and fans** (`pw-sim/media.rs`): outlets and journalists with sources; rumours only from real tracking
  leaked by a real person; news from public events and leaks; outlet credibility tracks whether rumours came true;
  per-club fan standing with reasons; public image.
- **Staff market** (`pw-sim/staffing.rs`): clubs hire backroom staff from the pool retired players (anyone) join.
- **Narration** (`crates/pw-narrate`): text as a pure function of state; every line points to its source.
- **Career layer** (`crates/pw-career`): no world facts — a session (who is inhabited, decision log, notes, goals),
  `take_control`/`release`, intents, perceived views, feed, and creating a new person via the world's generator.
- **Text client** (`pathway` binary): `pathway new synth small --warmup 365`, then `find`, `become <id>`, `next event`,
  `decisions`, `answer`, `meet manager minutes calm`, `train`, `routine`, `why <event>` and more (`help`).

### 3.9 Breadth pass — ⚠️ written 2026-09-28, never compiled
Ten segmented commits (`2d4fc60` … `2df1756`, ≈ 11k lines). Full catalogue, consequence chains, assumptions and
risks: **`docs/WORLD_SYSTEMS.md`**.
- **Rules** (`rules.toml`, profiles per confederation/nation): work permits (points), homegrown and foreigner quotas,
  loan limits, contract length caps, cup-tying, match eligibility; selection and signing honour them.
- **Economy & governance**: nation economies (growth, inflation, broadcast deals, league strength), owners with
  temperaments and transfer styles, board concerns, austerity, administration and points deductions, takeovers,
  stadium/training/academy projects.
- **Manager careers**: archetypes and media styles, job histories, entourages that follow, favourite players,
  tactical changes, resignations, poaching, retirement.
- **Scouting network**: assignments by brief (nation, competition, youth, player, need), scout biases and
  familiarity, capacity, reports and verdicts, analysts and recommendations; clubs see the world through them.
- **Squad planning & deals**: multi-season plans and needs, shortlists, enquiries, gazumping, medicals, add-ons,
  sell-ons, buy-backs, payables, pre-contracts, trials, loan terms (options, obligations, recalls).
- **Youth pipeline**: local grassroots clubs, academies with styles and reach, age-group sides, trials,
  scholarships, releases, school and exams, amateur football for those who don't make it.
- **National teams**: federations and managers, squads from imperfect views, windows, friendlies, qualifying,
  continental finals and a world tournament, caps and allegiance, club-vs-country friction.
- **Medical, growth, dressing rooms, performance**: injury cases with diagnosis uncertainty, treatment choices,
  setbacks, rushed returns, fragility and chronic conditions; mentoring, character drift, stagnation, learned
  traits; dressing-room hierarchy, groups, integration and influence; per-appearance records read differently by
  managers, fans, media, analysts and scouts.
- **Renown, press, honours**: local/continental/fame/followers; interviews and press conferences with real
  consequences, match reports, features, wonderkid lists, season reviews, retrospectives; playmaker/golden
  glove/team and manager of the season, player of the month, world and continental awards by votes, records,
  milestones, club legends, hall of fame.
- **Affairs & commerce**: coaching badges and courses (licence gates for jobs), homes and cost of living, personal
  staff, giving and foundations, investments; post-playing careers embodied in real systems (pundits and
  journalists at real outlets with real sources, agents as agencies, coaches/scouts/analysts/directors in the
  staff pool, ambassadors, business); brands, club sponsorship, endorsements with image rights, clashes and
  morality clauses.

### 3.9b Media, social and history pass — ⚠️ written 2026-09-28, never compiled

See `docs/MEDIA_SOCIAL_HISTORY_SYSTEMS.md`. Eleven commits (3d013fa…adf8e58):
- Seeds and streams: a fresh seed for every new world, named RNG streams, keyed draws.
- Culture: club identities, national trends, typed rivalries with memory, match meaning.
- The grapevine: information items, versions, motives, leaks.
- Contextual incidents: 30 kinds from 29 pressures, and an incident-response AI.
- The newsroom: journalists as people, outlet profiles, verification, threads, corrections, the agenda.
- Press conferences built on what journalists know.
- Social media: persistent accounts, opinions, replies and call-outs, supporter groups, chants, memes,
  contextual trust, the feed, human posting.
- An inbox built from real communications, where replies become intents.
- School, university, amateur and grassroots competitions with history.
- A generic record engine with holder histories.
- Voted awards with ballots, halls of fame at every scope, and a chronicle of firsts.

### 3.9c Integration — ✅ compiled, formatted, lint-clean, tested (2026-09-29)

The whole workspace (the Rust crates; the desktop client is §3.10) compiles with zero warnings and is clippy-clean.
Formatting is enforced by a width-200 `rustfmt.toml`.

- **Tests:** 39 pass. There are 7 ignored runs: the long runs of 5, 20 and 50 seasons and a causal-chain
  report (all run and passing), and 3 match-engine calibration and diagnostic reports.
- **Audit:** the semantic truth audit is clean in every season of the long runs.
- **Performance:** at 302k players a day takes about 2.4 s.

See `docs/INTEGRATION_REPORT.md`.

### 3.10 `app/`, `crates/pw-view`, `crates/pw-serve` — desktop client — 🟡 built and tested in a browser, on synthetic worlds only

Tauri 2 shell and a React/TypeScript interface over one JSON endpoint (`pw_view::Api::call`); `pw-serve` exposes the same
endpoint over local HTTP for development and browser tests. See `app/README.md`.

- **Observer**: overview, people, clubs (squad, staff, fixtures, finances, board, fans, dressing room, history), competitions,
  nations, fixtures and results, match pages, transfers, events, history and awards, and 23 lists for the wider world
  (posts, chants, rivalries, incidents, press conferences, referees, records, halls of fame, tactical schools, lower football).
- **Inhabiting a player**: today, messages (world inbox with replies that become intents, every decision kind), calendar,
  football, contract, life, people and promises, press and fans, social feed, journal, agent, and the actions the world accepts.
- **Not there**: anything the simulation has no screen-level route for. Managers, chairmen and heads of youth cannot be
  inhabited, so incident handling, press answers and appeals are decided by AI; Help → "What the simulation covers" says so.
- **Checked**: `cargo test --workspace --exclude ofm-engine`, `npm test`, and two browser scripts (`app/e2e/smoke.mjs` visits
  every route on a fresh world; `app/e2e/inbox.mjs` answers a decision and replies to a conversation). Imported (non-synthetic)
  worlds have only been tried on tiny hand-made data.
- **Unrevealed results**: while one of your matches is unrevealed, its scoreline is kept out of tables, match pages, Today,
  match-report stories, the events feed and posts about it (`crates/pw-view/tests/api.rs`, `a_concealed_result_is_not_given_away_…`).
  It covers what the API renders; anything a future system prints about a match must go through `Ctx::headline` / `Ctx::post_text`.
- **Insights** (`crates/pw-view/src/pages/insights.rs`, `app/src/components/Insights.tsx`): `insight.person`, `insight.club`,
  `insight.comp` and `insight.match` return short notes computed on request from what the world already records, each with the
  numbers it rests on (`basis`). Players: form against the year's average, goals and assists against expected, big-match against
  weak-opposition rating, standing among team-mates in the same line, selection, scoring runs and droughts, cards, workload spikes
  and tiredness (acute/chronic load, the same ratio the injury hazard uses), the medical room (open case with the medical team's own
  certainty, injury history, fragile regions, chronic conditions), development against players of the same age, contract and minutes
  against squad status, how the press, supporters and the manager see them, followers, milestones. Clubs: table position against
  the board's target, runs, form, home against away, best and worst attack and defence, dependence on one scorer, squad age against
  the league, fit players by line, treatment list, the manager's record, board patience and the wage bill. Competitions: title,
  promotion, continental and relegation races with points and matches left, form side, leaders. Matches: what is at stake, form going in,
  home and away records, earlier meetings, key absentees, the one to watch, and after the match what it did to each side's run.
  Private state (body condition, contracts, the medical room, engine numbers such as ability and the board's state) is only used
  for the person themselves, their club or an observer, and results you have not revealed are taken out of every count, not just hidden
  in the text (`insights_*` tests in `crates/pw-view/tests/api.rs`). Not covered: the season statistics and leaders tables still
  include unrevealed results; tactical analysis, scouting and squad-planning advice do not exist. Writing the notes showed that the
  synthetic match engine is generous (a striker scoring 54 league goals in 24 games, season average ratings above 9), which is a
  balance matter for the simulation, not the interface.
- **Look** (`app/src/styles/stage.css`, `app/src/components/Stage.tsx`, `Crest.tsx`, `app/src/pages/CompOverview.tsx`): the client is skinned
  after the Football Manager overview screens the owner pointed at: the whole window takes a dark tint (`--tint`, set by the page:
  a competition's colour, a club's kit colour), headings are Barlow Condensed with an underline, and entity pages open with a
  header (badge, title, meta blocks) over a strip of matches and one bordered panel of columns. Competitions open on an Overview
  (`comp.overview`): the last ten results and next four fixtures, the table (or the current round's ties), player and team
  statistic leaders, and match-report stories about the competition. Club, person, match, nation and Today use the same header.
  Badges are generated (a shield in the club's two colours with a pattern picked from its id), not real logos; `crest.colors`
  sends every club's colours once so any list can draw one. Team statistics are worked out from recorded results (goals, goals
  conceded, clean sheets, biggest win, and expected goals, shots and cards from player lines); possession and xG against are not
  recorded, so they are not shown. Results you have not revealed are left out of the strip and table, goals, assists and average
  rating are recomputed without them, and the sections that cannot be taken back out (man of the match, clean sheets, cards,
  expected goals) are held back with a note (`the_competition_overview_*` tests). The light theme keeps a light shell around dark
  stages. Not done: real portraits, kits or logos, and the reference's second page of statistics is a guess at what FM shows there.
- **Speed** (release, small synthetic world, four simulated years, measured after merging the simulation branch of 2026-09-28):
  about 23 s in all; an ordinary day costs 17 ms in year 1 and 34 ms in year 4, a Monday about 220 ms and the worst day about
  340 ms, from the weekly systems (morale, media, agents, youth, manager summons). `cargo run --release -p pw-view --example
  profile -- small 4 [--hash]` prints this and a fingerprint of the world, which is how speedups here were shown to change nothing.

---

## 4. What's next (in order)

### Step 1 — World-level social depth — ⚠️ written (§3.8); needs compiling and balancing
1. Weekly relationship dynamics for every squad: manager↔player trust from training ratings, match ratings,
   professionalism and incidents; teammate affinity from shared time and compatibility; rivalries for the same
   position; cliques; dressing-room influence (reputation, leadership, tenure) that spreads unrest when an influential
   player is unhappy.
2. Promise ledger evaluation: minutes/status/contract/let-leave/position/loan/captaincy promises tracked over their
   window; kept or broken → memories, trust and morale shifts both ways.
3. Conversations: one resolver for everyone (topic × tone × personalities × relationship × standing).
   Topics: playing time, role/position, new contract, loan, transfer request, feedback, promise follow-up, teammate
   complaint, apology. Tones: calm, assertive, aggressive, humble, joking. Outcomes: promises, refusals, deferrals
   ("prove it in training"), status changes, transfer listing, fines, dropped, media leaks.
   AI players with grievances start conversations too.
4. Contract negotiations for everyone: opening offer, player ask (from personality and market), counters, club limits,
   agent quality, walk-away, deadlines; renewals, transfers (personal terms after a fee), free agents, first pro deals.
   Replace today's accept/reject contract decision with this.
5. Hook into existing systems: selection uses real manager trust (not just squad status); development and workload use
   each player's training plan; AI coaches set plans monthly (weakest key attributes, intensity by professionalism).

### Step 2 — Living as one person, at FM depth — partly written (§3.8: control of anyone, views, feed, conversations, talks, life, agents, press, staff careers); the rest below
1. Creation: identity, family background and eligibility, body, position, **talent tier sampled from the world's own
   youth distribution for that nation** (PA stays hidden like everyone's), personality archetype within normal ranges,
   start stage (academy / scholar / late starter / take over an existing youth player), club choice.
2. Weekly planning (FM-style): training plan (focus, intensity, extras, recovery), life routine (rest, family, partner,
   friends, study, hobbies, media/sponsor, language lessons) with presets, lifestyle (diet, nightlife, spending).
   Plans resolve daily into load, training performance (what coaches see), well-being, relationships and money through
   the same bounded channels AI players use.
3. People in your life as real persons: parents, siblings, partner (dating, moving together, separation), friends,
   agent — each with personality, needs, location and their own events (illness, weddings, career moves) that create
   requests and trade-offs.
4. Agent: quality, honesty, network, fee, instructions (priorities, red lines, autonomy); monthly reports built from
   what clubs actually know about you; negotiates for you.
5. Transfer sagas: rumours (with reliability), approaches, talks, medical, personal terms, relocation (housing, family
   decisions, language, settling in).
6. Media & public image: journalists and outlets, stories generated from events, press conferences and interviews with
   toned answers (manager/teammate/fan consequences), social media, fan affinity per club, controversies.
7. Money: monthly ledger (wages, bonuses, signing fees, image rights, sponsorships), per-nation tax, agent fees,
   expenses, savings, investments with risk profiles, property rent/buy, lifestyle creep.
8. Health & mind: stress, burnout, confidence crises, psychologist/time off, body map, rehab choices (rush back vs
   caution, second opinions).
9. Education & growth: school (young starts), courses, languages, coaching badges (for later).
10. Match day (watch only): squad announcement with the manager's stated reason, role instructions, pre-match
    preparation choice, live replay (text + 2D), post-match rating breakdown, heatmap, coach feedback, media.
11. Self-perception: your attributes as ranges from your coaches' view, tightening with evidence; development trend;
    coach reports composed from real perception data (not canned text).
12. Milestones, records and career goals you pin — tracked from real stats; no scripted chapters.

### Step 3 — World gaps
- National teams, registration quotas/work permits, takeovers/administration and facility projects are now written
  (§3.9, unverified). Still missing: youth international tournaments, discipline carry-over across competitions,
  sustainability (spending) rules.
- Injury rate calibration (target 1.2–1.8 time-loss injuries per player-season), goals 2.35 → 2.6–2.9 on real data.

### Step 4 — Client (Tauri 2 + React/TypeScript) — professional, dense, keyboard-first
Home (today, status strip, next match + selection outlook, pending decisions), Calendar, Inbox (threads by sender,
decisions with deadlines/defaults), Training (club schedule, your plan, coach reports, load gauge), Match Day
(pre-match, live text + 2D, post-match), Performance (season/career stats, charts, perceived attributes with ranges),
Club (squad depth with you highlighted, dressing room web, staff), Career (contract, agent, negotiations, transfer
talks, history), Life (routine, people, home, education, health), Money, Media, World (tables, fixtures, news,
transfers, records, database browser with perceived-only data), Settings. Hand-built design system (tokens, light/dark,
tabular numerals, virtualised tables) — no generic AI styling.

### Step 5 — Performance & polish
300k-player benchmark (`pathway-sim synth huge`), parallelise remaining weekly loops, memory budget (< 3 GB),
save size/time, 20-season autonomy benchmark, soak tests.

---

## 5. Verification status (for when you ask me to build/test)

Last verified points:
- `pw-core`, `pw-data`: tests passed.
- `pw-match`: tests passed; calibration run as recorded above.
- `pw-world`, `pw-sim`, `pw-import`, `pw-cli`: compiled with zero warnings; one 400-day synthetic run (before §3.8).

**Everything in §3.8 and §3.9 was written without compiling** (your instruction: write code now, build later). Expect a
round of compile fixes. Suggested order when you ask:
```
cargo check -p pw-world
cargo check -p pw-sim
cargo check -p pw-narrate -p pw-career
cargo check -p pw-cli
cargo test -p pw-core -p pw-data -p pw-match -p pw-sim
cargo run --release -p pw-cli --bin pathway-sim -- synth small --days 400   # world health with the new systems
cargo run --release -p pw-cli --bin pathway -- new synth small --warmup 180  # play
```
Then: balance passes on meeting frequency, promise outcomes, rumour volume, relationship formation, agent
coverage and life events using headless runs (checklist §3 in `docs/ANTI_LINEAR_DESIGN_CHECKLIST.md`).

Known gaps, next in line (see `docs/WORLD_SYSTEMS.md` §5–§6 for risks and shallow areas):
- Client commands for the new intents and decisions (courses, homes, helpers, giving, careers, press, allegiance,
  treatment, endorsements) — the world supports them; `pathway` doesn't expose them yet.
- Humans cannot yet inhabit a *role* such as national manager, board or owner (the person can be inhabited; the
  role's choices are AI).
- Event-log compaction, per-nation eligibility and fame indices for 300k-player scale.
- Automated checks from the checklist (mind-read allowlist, swap test, takeover-changes-nothing test).

---

## 6. What I need from you

### 6.1 Your FM23 world export (the one real blocker for playing on real data)
Produce a folder in the format described in **`data/IMPORT_FORMAT.md`**. Minimum:
- `nations.csv` — code, name, confederation (+ reputation, calendar type, economy, youth rating if you have them).
- `competitions.csv` — every league you want active (id, name, nation, kind=league, tier, promote/relegate counts),
  domestic cups, continental competitions (and youth leagues if you want U18/U21 football).
- `clubs.csv` — id, name, nation, league id (+ reputation, finances, stadium, facilities, colours, youth sides).
- `players.csv` — id, names, date of birth, nationality, club, team (first/u21/u18…), positions (FM notation is fine),
  **all 46 attributes** (FM 1–20), and ideally CA, PA, contract end, wage, value, squad status, reputation.
- Optional: `staff.csv` (managers/coaches/scouts with attributes; generated automatically if absent),
  `competition_entrants.csv`, `world.toml` (start date, seed).

Tips:
- Hidden attributes (consistency, professionalism, etc.) are optional — random within normal ranges if absent,
  but real values make personalities meaningful.
- If your export tool gives attributes on 1–100, that's detected and handled.
- Players without a club become free agents; youth players need a `team` column (`u18`, `u21`) to land in youth sides.
- Start small if you like (one nation, two tiers) to test, then grow.

### 6.2 Decisions still open
- **Post-career**: world-side paths now exist (coach, pundit, journalist, agent, analyst, scout, director,
  ambassador, business). Open: whether a human in a manager's job gets FM-style club-management screens.
- **Which nations/leagues are active** at launch (affects performance; everything scales, but 300k players with youth
  sides is the upper target).
- **Distribution intent:** personal only (GPL engine is fine) or public someday (native engine must be finished first).

### 6.3 Things only you can do (later)
- Tauri needs an app icon set before a release build: `npx tauri icon path/to/logo.png` in `app/` (any square PNG).
- `npm install` in `app/` the first time (needs network).
- Tell me when you want a build/test/verification pass — I won't run them otherwise.

---

## 7. Repository map

```
Cargo.toml                 workspace
PROGRESS.md                this file
data/engine/*.toml         engine rules (formations, weights, curves, injuries, calendar, tuning)
data/IMPORT_FORMAT.md      what your FM export must look like
crates/pw-core             ids, dates, RNG, math, attributes, positions, roles, tactics, traits
crates/pw-data             engine data pack loader
crates/pw-world            world state (people, players, clubs, comps, perception, social, negotiations, …)
crates/pw-match            match interface + OFM adapter + native engine
crates/pw-sim              daily pipeline and all world systems
crates/pw-import           FM-export CSV import + synthetic test fixture
crates/pw-cli              headless runner (`pathway-sim`)
crates/pw-career           protagonist layer (being rebuilt at full depth)
vendor/ofm-engine          OpenFootManager match engine (GPL-3, unmodified source)
crates/pw-narrate          every sentence shown to a person, rendered from state
crates/pw-view             the API the client calls: pages, table queries, actions, inbox
crates/pw-serve            that API over local HTTP (development and browser tests)
app/                       Tauri 2 + React client (see app/README.md)
plan/, foundation/         original design documents
```
