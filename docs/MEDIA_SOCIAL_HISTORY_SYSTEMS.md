# Media, Social and History Systems

Status: written 2026-09-28, **not compiled yet** (the compile, test and validation phase comes after
the final depth pass). Code references are `crate/src/file.rs`.

Every system here follows the one causal chain the project is built on:

```
WORLD STATE → PRESSURE / PROBABILITY → EVENT → KNOWLEDGE → INTERPRETATION → COMMUNICATION → REACTION → CONSEQUENCE
```

Text is rendered from state by `pw-narrate` and never adds a fact that is not in the world. Nothing here
favours the person a human controls. A human's post, reply or answer enters the same functions an AI
mind's choice does.

---

## 1. RNG architecture (`pw-core/src/rng.rs`)

- **World seed.** A new world gets a fresh seed from the operating system (`fresh_seed`: `RandomState`,
  time and pid) unless `--seed` or `world.toml` gives one. The seed goes into the save, so a save replays
  exactly and a different seed gives a different world. `seed_label` / `parse_seed` print and read seeds
  in a form people can share.
- **Named streams.** Every subsystem has its own tag (`stream::*`, 39 in the `ALL` name table). This pass
  added IDENTITY, SUPPORTERS, JOURNALISTS, SOCIAL_ACTIVITY, PRESS, INCIDENTS, SHOCKS, AWARDS, GRAPEVINE,
  CULTURE, MINOR, NEWSROOM and RESPONSE. Tags are never renumbered, because they are part of how saves
  behave.
- **Keyed draws.** `World::rng(stream, &[ids…, period])` and `World::roll(...)` derive a draw from the
  seed, the stream, stable entity ids and a period (`period::day/week/month/year`). One entity's draw in
  one week depends only on those inputs. Iteration order, threads and draws made by other subsystems
  cannot change it.
- **Playthroughs.** Taking control of someone for the first time mixes a fresh salt into the seed.
  Two playthroughs of the same world diverge, and each one still replays exactly.

**How identities vary by save.** Account handles and display names, personas, ages, activity hours,
follower counts, school and university names and founding years, journalist ties, chant word choices
and every incident roll are all keyed on the world seed. Two saves built from the same imported
football world share clubs and players, but their supporters, journalists, schools and stories differ.

## 2. Culture, rivalries and match meaning (`pw-world/src/culture.rs`, `pw-sim/src/culture.rs`)

- **Club identity.** Each club has a style, youth focus, tribalism, expectations and a culture that
  drifts with results and appointments.
- **National trends.** Each nation has trends (pressing, tempo, directness) that move with who is winning.
  Boards consult them when they hire (`fashion`).
- **Rivalries.** Rivalries are typed: derby, title race, promotion, cup, player-crossing and national.
  Each one keeps head-to-head counts, memorable moments and `revenge_due`. New rivalries form when
  clubs meet in title races, promotion races, finals and transfers.
- **Match meaning.** `meaning()` gives a fixture its significance, stakes and derby flag. It feeds
  selection intensity, coverage, supporter reaction, chants and memes.

## 3. Information propagation — the grapevine (`pw-world/src/info.rs`, `pw-sim/src/grapevine.rs`)

- **Information items.** Private facts become `InfoItem`s: incident, interest, bid, unhappy,
  job-in-danger, discipline, worse-than-reported injury, contract talks, private life, dressing room and
  exploring. Each item carries a sensitivity, a `true_now` flag and a list of `Knower`s. A knower records
  who learned it, how, when, the version they hold (`Fidelity`: accurate, partial, exaggerated,
  outdated, garbled, planted) and how confident they are.
- **How it spreads.** People tell their contacts: partner, own agent, clients, teammates, colleagues,
  superiors, board and journalist friends. The chance of telling someone is
  `inclination = base(role, kind) × juiciness × closeness × (1 − discretion)`, and it carries a
  `Motive` such as duty, confiding, gossip, ego, revenge, agent strategy, press friendship or concern.
  Discretion comes from professionalism, low controversy, loyalty to the subject, and whether someone in
  authority asked for it to be kept quiet. Each retelling can degrade the version
  (`Fidelity::degrade`, which depends on the teller's honesty).
- **Reactions.** Knowing changes what people do:
  - A player who hears a club wants them forms a belief.
  - A board queries its manager.
  - An agent starts exploring the market.
  - A manager's trust in the source moves.
  - An incident reaches whoever has to respond to it.
- **Leaks.** A leak is a telling to a journalist. When a story is published, the people who knew and
  were close to the journalist come under suspicion (`LeakSuspected`, board queries). That suspicion can
  be wrong.
- **Humans.** Humans pass things on through `Intent::Tell`, using their own version of the item. So a
  human can leak too, by the same route.

**How rumours spread.**

1. A private state (an interest, a row, a stalled negotiation) becomes an item known to the people
   involved.
2. Some of them tell contacts, for their own reasons.
3. Versions drift with each retelling.
4. Once an item reaches a journalist, the newsroom may print it with the claim strength its verification
   supports.
5. Readers and supporters believe it or not, depending on their trust in that outlet.
6. The thread is followed up, denied or corrected as the truth develops.

## 4. Stochastic incidents and the response AI (`pw-world/src/incident.rs`, `pw-sim/src/incidents.rs`, `pw-sim/src/responses.rs`)

- **Incident kinds.** There are 30 kinds:
  - In the squad: a training confrontation, a tactical disagreement, storming out, arriving late.
  - Around the club: equipment, pitch damage, a travel delay, a postponement, visa, registration and
    paperwork problems, a coach resigning, staff poached.
  - Personal: a family emergency, relationship conflict, pregnancy, moving problems, burglary, an exam
    clash, a childcare clash, an unexpected bill.
  - Club and world: ownership controversy, a sponsor collapsing, an economic downturn, facility damage,
    transport disruption, severe weather, a federation dispute, an investigation, supporter unrest. Each definition (`def`) lists its scope, location, exposure (private, club or public),
  the pressures that make it likely and its base rate.
- **Hazard model.** `hazard = base · exp(Σ wᵢ·pressureᵢ)`. The pressures are all read from state:
  resentment, rivalry for a position, temper, low morale, public criticism, training load, room tension,
  unresolved conflicts, unprofessional habits, nightlife, stress, being new abroad, winter, club
  finances, owner meddling, poor results, fame, pressure on the manager, staff discontent, household
  strain, studying, a weak economy and more. `consider_incident` rolls a keyed stream for each candidate
  each day or week. No incident is scripted.
- **Chains.** `trigger` records the pressures that caused the incident as event causes, with
  `follows` linking it to an earlier incident. Witnesses become knowers. Consequences then follow:
  unavailability, investigations, points of tension, public stories if exposed, and leave.
- **Responses.** Whoever holds authority (manager, captain, board) decides: fine, drop, demand an
  apology, mediate, involve the captain, keep it private, ignore, protect someone, delay, issue a
  statement, grant or refuse leave, or open an inquiry. The decision comes from their `profile`
  (discipline, temperament, trust in the people involved, the club's culture). A human in that role
  gets a decision whose default is the AI's choice. Captains may step in on their own. Deferred
  responses come back later.

**How random incidents become causal chains.** Each part of an incident is recorded as its own event:
the pressure that made it likely, the roll that made it happen, the witnesses who learned of it, the
responder's decision and its consequences (memories such as `Blamed`, `Protected`, `Mediated` and
`Fought`, tension, morale, availability). Any of these can raise the pressure for the next incident.
A dropped player with low temperament and a derby coming is more likely to be in the next row, and a
cover-up that leaks is a story.

## 5. Journalists, outlets and the newsroom (`pw-world/src/media.rs`, `pw-sim/src/newsroom.rs`, `pw-sim/src/media.rs`)

- **Outlets.** Each outlet has a profile: scope (national, regional or club), style (broadsheet,
  tabloid, analytical or fan media), an affinity club, sensationalism and a weekly quota.
- **Journalists.** Journalists are people. Each has a profile (risk appetite, accuracy, a speciality
  focus, tenure) and source ties with strength. They cultivate sources weekly, move between outlets, get
  sacked and retire (`yearly`), and are replaced by new hires.
- **How journalists learn things.**
  - Public facts: match facts, public incidents, published quotes and viral supporter posts.
  - Grapevine items that reached them through a source tie.
  - Their own follow-ups on running threads.

  A journalist cannot write about something they do not know.
- **Pipeline.** Each story goes through the same steps:
  1. **Candidate** (from an info item, a public incident, match facts, a viral post, or an agenda task).
  2. **Newsworthiness** (importance, relevance to the outlet, controversy, freshness).
  3. **Verification** (asking sources; each can confirm or deny).
  4. **Editorial**, which sets the claim type (fact, report, rumour or speculation, or spiked).
  5. **Angle** and minute of publication.
  6. **Publish.**

  Publishing creates the story event with its cause (the event, the heard info, the newsworthiness
  facts, or `Fact::Viral`). It also attaches the story to a thread, refers back to earlier stories and
  schedules follow-ups and denials on the agenda.
- **Threads.** A thread is a running story, such as a transfer saga or an incident. When its subject
  resolves, the thread closes as having happened or not. Each outcome counts toward the journalist's hits
  and misses and the source's reliability, and wrong stories get corrections.
- **Press conferences** (`pw-sim/src/pressroom.rs`). Clubs that are big enough hold one the day before
  a match. Journalists who cover the club ask about what they know: recent stories, the last match,
  pressure after defeats, the rival, public injuries, or something the speaker said before. Answers are
  stances chosen from the speaker's media style, temperament and trust, and from what is true. An
  evasive answer to a hard question can draw a follow-up. Every answer is a quote on the record with
  effects (`press::speak`). A human gets each question as a decision until the day of the match.

## 6. Social population model (`pw-world/src/socialnet.rs`, `pw-sim/src/socialnet.rs`)

The population is built in layers, from cheapest to richest:

1. **Supporter groups** per club: season-ticket holders, online, international, academy, ultras (only
   where the culture is tribal), numbers people and the supporters' trust. Each group has a size, a
   voice, and moods toward the manager, the board and the team. The moods are updated weekly from form
   against expectation, derby losses, academy minutes, owner meddling and financial trouble. The
   club's single `fan_mood` is the size-weighted view of its groups. A group acts only when its state
   justifies it: protests and petitions (board satisfaction and owner image fall), banners against the
   manager (stress, fan standing), or applause.
2. **Persistent accounts.** Each club has 4–16 ordinary accounts, depending on reputation, plus fan
   news, academy watch, stats and rumour accounts at bigger clubs, and an ultra account where the
   culture is tribal. Each nation has a few neutrals and a provocateur. Journalists and famous people
   have accounts too. An account has a persona (optimism, patience, tribalism, humour, hostility,
   loyalty, nostalgia, stats, youth, local, celebrity, stubbornness, knowledge, credulity), an age
   band, activity, a peak hour, followers and credibility.
3. **Opinions and memories.** An opinion of a person moves slowly and stubbornly, and remembers its
   lowest and highest points and the post that last voiced it. Memories record moments: a derby goal, a
   late winner, a mistake, a transfer request, joining a rival.

**Social pipeline (daily).**

1. **Frames** are drawn from real things: results, late winners, hat-tricks, red cards, signings,
   departures, transfer requests, sackings and appointments, awards, milestones, records, long
   injuries, public incidents, published stories and quotes, and people's own posts.
2. The **audience** is the club's accounts, the rival's accounts, and neutrals when the subject is
   famous. Each account sees a frame with a probability based on its activity and intensity, and less
   often in later waves. Seeing moves opinions and memories whether or not the account posts.
3. **Posting** depends on activity, the frame's weight and fatigue (posts already made today). The
   **concept** is chosen from the frame's valence for that account, their opinion and memories, and
   their persona. Options include praise, reluctant praise, conceding they were wrong, doubling down,
   criticise, mock, sarcasm, celebrate, lament, worry, compare to a legend, relay a rumour, question and
   defend.
4. **Threads.** Replies, disagreements and call-outs, and quote-posts from rivals, run to a depth of 3
   with halving probability. Hostile exchanges can end in mutes.
5. **Engagement** is likes and reposts from the unmaterialised audience: reach × resonance with the
   club's mood × the subject's fame.
6. **Trends** are counts per topic and nation. **Viral** posts become a `FanReaction` story sourced to
   the post (`Fact::Viral`).
7. Players who read their mentions feel them (stress, morale and confidence, scaled by their tolerance
   for pressure).
8. **Memes** are born from real moments, such as a derby red card, a derby late winner or a very viral
   post. Rivals reuse them, variants appear, and a meme fades when nobody uses it.
9. **Chants** are born from real moments: late winners and hat-tricks in big games, players who crossed
   to a rival, a board that has lost the terraces. A chant is an original structure (a shape plus a
   seed) filled with names from the world. No existing song's lyrics or tune is used or referenced.

**How posts are generated without lying.** A post is a semantic structure, not text:

- the frame it reacts to
- its concept
- its subject
- a second subject (the legend it compares to)
- references to earlier posts that exist
- how the author knew (watched, read a story, saw a post, heard, own life)
- the author's opinion before posting

The renderer (`pw-narrate/src/social.rs`) takes every noun from the frame or the references. A
call-out ("you called them finished three weeks ago") is only produced when that earlier post exists,
and the time span comes from its date. A relayed rumour is hedged when the author is unsure and the
claim is weak. A legend comparison names a real club legend.

**Contextual trust.** `believes(account, story)` combines:

- the outlet's credibility, weighted by the account's knowledge
- the account's own record with that outlet (`outlet_trust`, updated monthly as the threads they
  relayed resolve)
- their credulity
- desirability (nobody wants to believe their best player is leaving, while everyone wants to believe
  it of a rival)
- corroboration from the thread
- the claim type

Belief is not fact. What an account believes changes what it posts, not what is true.

**Feed.** `feed(person, n)` scores recent posts for one person. It weights accounts they follow, their
club, posts about them, teammates, their rival, real people's accounts, trending topics and
engagement, and applies a recency decay. It is a thin, personal slice of the network.

**Human posting.** `Intent::Post` runs through the same function (`person_post`) an AI person uses. It
leaves a public-praise or public-criticism memory with the subject, and teammates' and managers'
reactions when a player criticises a colleague. It moves image and fan standing, and becomes a frame
others react to.

## 7. Inbox and conversation threads (`pw-world/src/inbox.rs`, `pw-sim/src/inbox.rs`)

- Messages exist only as references to real communications:
  - decisions raised
  - grapevine tellings
  - meetings
  - stories about the person
  - posts replying to or calling them out
  - private events (board warnings and queries)
  - waiting press questions
- Messages are grouped into threads by counterpart and subject.
- Each reply maps onto an intent or a decision answer: thank, ask to meet (with a topic derived from the
  information), pass it on, respond on the record, reply to the post, keep quiet, or ignore. The other
  side's answer comes from their own mind, later, through the same systems. No response is invented.
- Inboxes are kept only for people a human controls. AI minds read the same sources directly.

## 8. School, university and amateur football (`pw-world/src/minor.rs`, `pw-sim/src/minor.rs`)

- **Institutions.** Schools (two per town) and universities (in the towns with the most football) are
  generated from the world's towns and keyed on the seed.
- **Membership.** Children aged 11–18 attend school in their town. Qualified school leavers who are not
  professionals may enrol at university for three years; turning professional means leaving early.
  Players who turn professional become alumni of the institutions that produced them.
- **Competitions** are drawn each September:
  - school leagues (regions of about eight schools)
  - a national schools cup
  - university leagues
  - an amateur pyramid in divisions of 12, ordered by standing
  - a grassroots junior cup for the oldest age groups

  They are played weekly: a double round robin for leagues, and a cup round every third week.
- **Result model.** Team strength is the mean ability of the best 11 (short-handed sides suffer) plus
  coaching. Scores are Poisson draws. Goals are credited to real members, weighted by finishing,
  movement and position. Appearances and ratings are recorded.
- **Season end (June).** The season records the winner, runner-up, top scorer, best player and biggest
  win. Standouts are noticed by academies (`knowledge.observe`). Amateur standings go up or down, and
  lines become career histories (`Minor::career`).

## 9. Records (`pw-world/src/records.rs`, `pw-sim/src/records.rs`)

- **The record book.** One book is keyed by scope, stat and level:
  - Scopes: world, nation, competition, club, institution, local club, minor competition by kind.
  - Stats: goals, appearances, goals in a season, biggest win, fee paid or received, caps,
    international goals, titles, youngest scorer, youngest debut, oldest scorer, winning run, unbeaten
    run, points in a season.
  - Levels: grassroots, school, university, amateur, youth, professional, international.
- **History.** Each record keeps its current mark and its earlier holders. When a record falls
  (`Broken`), the entry holds the old mark, how long it stood and whether it was the holder extending
  their own record. Extending your own record is not news every time.
- **Hooks.** Records are fed by:
  - senior appearances (club tallies mirrored, youngest debut and scorer, oldest scorer)
  - results (runs)
  - the end of a league season (points)
  - minor seasons (competition goals, biggest wins, titles)
  - minor lines (all-time and season records at each school, university or local club)
  - `honours` (club top scorer and appearances, record signing and sale, the world fee record, caps and
    international goals)

  Records from `honours` are mirrored silently, so its existing announcements gain the same context
  (`history::pro_context`).
- **Minimums.** A record needs a minimum value (for example a 5-goal margin, 10 goals in a school
  season, a 6-match winning run) so that trivial marks are not news.

## 10. Awards and voters (`pw-world/src/awards.rs`, `pw-sim/src/awards.rs`)

- **Vote engine.** Named voters rank candidates through their own lens. The lens weights performance,
  trophies, fame, goals, familiarity, longevity, loyalty and international career. Each voter adds
  keyed personal noise, and eligibility rules apply: national managers and captains may not vote for
  their own nation's players, and players may not vote for their teammates. Points are 5-3-1, or one
  per pick for committees. Every ballot is kept with the reason for each pick for ten years, and the
  results are kept for good. Text can then say, from the record, "named on 61 of 180 ballots, mostly for
  their goals".
- **Voted awards.**
  - World Player of the Year and World Young Player: national managers, national captains and
    journalists.
  - Players' Player of the Season: a delegation of each league's players.
  - University and Schools Player of the Year in each nation: that nation's journalists. The winner is
    seen by the nation's professional clubs.

  Statistical awards (Golden Boot, Golden Glove, Team of the Season, Player of the Month, continental
  players, manager of the season) stay in `honours`.

## 11. Halls of fame

- **Scopes.** There are halls for the world, each nation, each club and each school or university.
  Each has a threshold share of the committee vote and a maximum class size.
- **Committees.** A hall's committee is its journalists plus its living members.
- **Candidates.**
  - World: retired for over a year, with a career score of at least 700.
  - Nation: 20 or more caps, or a legend at one of its clubs.
  - Club: its legends, once they have left or retired.
  - School or university: professional alumni with 50 or more senior appearances.
- **Inductions.** Classes are inducted each January. Consequences: fame, love from the club's fans, and
  the world hall also feeds the existing `honours.hall` and its `InductedHallOfFame` event.

## 12. Milestones and the chronicle

- **Milestones.** Milestones (club appearances, career goals, senior appearances, caps) stay in
  `honours`.
- **The chronicle.** The chronicle (`Acclaim::chronicle`) records world-level feats and firsts with
  their count in scope ("the first double in the nation's history", "the fourth"):
  - a club's first league title
  - league-and-cup doubles
  - unbeaten league seasons
  - a nation's first tournament win
  - repeat World Players of the Year
  - the first World Player from a nation
  - the first player in the world to reach 1,000 senior appearances, 400 career goals or 150 caps

## 13. How records and awards work at every level

| Level | Competitions | Records | Awards | Halls |
|---|---|---|---|---|
| Grassroots | junior cup | club goals and appearances, cup goals, biggest win, titles | — | — |
| School | school leagues, schools cup | school all-time goals and appearances, season goals, competition records, titles | Schools Player of the Year (journalists) | school hall (professional alumni) |
| University | university leagues | as school | University Player of the Year (journalists) | university hall |
| Amateur | tiered pyramid | club and division records, titles | — | — |
| Professional | leagues, cups, continental | club and league records, runs, points, youngest and oldest, fees | statistical awards, Players' Player (players' vote) | club halls (committee), nation halls |
| International | tournaments | caps, international goals | World Player and Young Player (managers, captains, journalists), continental | nation and world halls |

## 14. Scale strategy

- **Accounts are bounded.** Accounts per club are capped at 16, plus four special accounts. Supporter
  groups carry the mass: sizes run to the hundreds of thousands, and likes and reposts are aggregates
  of that unmaterialised audience, not individual accounts.
- **Posts are materialised only around real frames**, with fatigue per account per day.
  - Retention is 45 days.
  - Posts that are referenced or viral are kept in `kept`, which is capped at 50,000 with the oldest
    trimmed.
  - Opinions are capped at 12 per account (the mildest view is forgotten), and memories at 8.
- **Grapevine items are compacted** monthly, and the tell log is a bounded ring.
- **Minor football does not use the match engine.** Its result model costs O(squad) per game.
- **Inboxes exist only for humans.** Ballots are summarised after ten years.
- **Everything periodic runs daily, weekly or monthly on keyed streams**, so work can be split or
  deferred without changing outcomes.

## 15. What is shallow

- **Language.**
  - Social and press text uses one English pack with registers and age bands. There is no second locale
    yet, and slang is not region-aware.
  - Chants have a handful of shapes.
  - Meme text is a template.
- **Minor football.**
  - Scores come from a strength model, not play.
  - School teams do not distinguish age groups.
  - University football is one league per nation.
  - There are no school or university staff as people.
- **Supporter groups** do not yet interact with each other or with the stadium (atmosphere is the next
  pass).
- **The inbox** shows communications but has no drafts or free text by design. Replies are limited to
  the defined set.
- **Awards.**
  - There are no fan votes.
  - Committees for institution halls are journalists, because schools have no people yet.
- **The chronicle** covers a fixed set of feats.
- **Historical backfill** before the start date does not exist yet (final pass).

## 16. Risks

- **Compilation.** None of this pass is compiled. Expect borrow-checker issues from iterating World
  fields while mutating, exhaustive matches on new enum variants, and IdVec indexing with NONE ids.
- **Balance.**
  - Incident hazards, grapevine inclinations, social posting rates and engagement constants are
    first guesses.
  - They need measurement in long runs: incident frequency per club-season, the proportion of stories
    that are true, posts per day, and the number of viral stories.
- **Performance.**
  - `socialnet::daily` visits the audience of every frame; big derby days may be expensive.
  - `inbox::daily` scans every person once a day to find humans.
  - `minor::weekly` sorts squads each game.
  - These need profiling at 300k players.
- **Save size.** Posts, grapevine items, votes, the record book and minor careers all grow. The caps
  above need checking against multi-season saves.
- **Coherence.** Several systems move the same quantities (fan mood, stress, morale, image). Their
  combined effect needs bounds tests so that no single system dominates.
