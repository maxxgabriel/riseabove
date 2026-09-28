# 09 — PlayerCareerCore: The Protagonist's Football Career

## 1. Creation

### 1.1 Start options

| Start | Age | Situation |
|-------|-----|-----------|
| Grassroots | 8–11 | School + local club; scouted into an academy or not |
| Academy | 12–15 | Already at an academy (chosen tier or random) |
| Scholar | 16–17 | Scholarship year, pro contract decision looming |
| Late starter | 17–20 | Semi-pro/amateur, trying to break in |
| Import existing player | any | Replace an existing generated youth (world-gen option) — becomes ordinary protagonist |

### 1.2 Creation choices (bounded by fairness budget)

- Identity: name, birthplace, nationalities via family (parents/grandparents from other nations = eligibility options), languages.
- Body: height range (sampled within chosen band), preferred foot, body type.
- Position preference (initial familiarity).
- Background: family football history, household income band, siblings → life sim seeds.
- Talent: **not chosen directly**. Player picks a *talent tier* whose PA is sampled from the same distribution the world uses for that tier of youth in that nation (e.g. "local talent", "academy prospect", "elite prospect" = percentiles), or "Unknown" (sampled from the full distribution). The PA is hidden like everyone's.
- Personality starting point: chosen archetype adjusts hidden attributes within the nation's normal distribution (no maxed-out traits).

Ironman option: no reloads, single autosave.

## 2. Career stages & core loops

### 2.1 Weekly loop (any stage)

```
Monday: recovery / review (coach feedback, stats)
Tue–Thu: training (choices: focus, extra sessions, recovery), meetings (manager, coach, agent), life activities
Fri: tactical prep, selection forecast, pre-match media
Sat/Sun: match day (squad announcement → match → post-match)
Evenings: life sim (family, friends, education, rest, hobbies)
Inbox & decisions throughout
```

### 2.2 Stage-specific systems

| Stage | Unique systems |
|-------|----------------|
| Grassroots | Local matches (simplified league), school, parents' support, scout visits, trial invitations |
| Academy | Age-group squad selection, education obligations, annual retention review, growth spurts, youth tournaments, youth internationals |
| Scholar | Scholarship contract, first-team training call-ups, pro-contract negotiation (often family + agent involvement), U18/U21 leagues |
| Breakthrough | Bench appearances, debut, loans, B-team, first pro wage, homegrown status |
| Established | Status battles, renewals, transfers, European competition, internationals, leadership |
| Veteran | Role change, mentoring, captaincy, wage cuts, move to lower level / abroad, testimonial |
| Retirement | Decision flow, farewell match, post-career path |

## 3. Playing time & status battles

- Squad depth chart visible as perceived ("you're currently behind X and Y").
- Levers the protagonist actually has (same as AI players via their minds):
  - Train well (training ratings → coach perception).
  - Perform when given minutes.
  - Talk to manager: ask for more minutes, ask what to improve, ask for position change, ask to be loaned, ask to leave. Outcome depends on manager Man Management, relationship, protagonist reputation, and the tone chosen.
  - Accept a new role/position.
  - Request transfer (public or private), hand in transfer request.
- Promises system: manager may promise minutes/status; broken promises damage trust both ways; the protagonist can remind or escalate.

## 4. Relationships inside football

| Relationship | Drivers | Effects |
|--------------|---------|---------|
| Manager | performance, attitude, conversations, promises, public comments | selection trust, status, contract support |
| Coaches | training attitude, improvement | perception accuracy, focus approvals, mentoring |
| Teammates | shared time, nationality/language, personality, rivalry for position, celebrations, arguments | chemistry (partnership bonus in match engine), dressing-room support, future network (future managers/agents) |
| Captain / leaders | respect, conduct | influence on morale, dressing room events |
| Board / chairman | reputation, public stance | contract support, sale decisions |
| Fans | performances, loyalty signals, comments | chants, reputation, legend status |
| Agent | results, communication | quality of representation |
| National coach | form, exposure, fit | call-ups |

Interactions are conversations with options + tone (calm, assertive, aggressive, humble, joking), resolved by personalities and relationship state. Every major conversation is logged in personal history.

## 5. Match-day experience

1. Squad announcement (starting/bench/not in squad) with reason if manager shares.
2. Pre-match: team talk (manager's), personal preparation choice (focus, calm, fired-up — small bounded morale/confidence effects equal to AI players' pre-match states).
3. Live match: text + 2D radar, protagonist focus filter, optional Player Moments (06 §14).
4. Post-match: rating, stats, heatmap, coach comment, media questions, fan reaction, social media, injury news.

## 6. Milestones & achievements

Auto-detected: debut (club, league, continental, international), first goal/assist, first start, 50/100/250/500 appearances, goals milestones, hat-tricks, clean-sheet runs (GK), trophies, awards, captaincy, youngest/oldest records, promotions, transfer records, testimonial. Each has presentation (inbox + media + legacy entry).

## 7. Career goals (player-set)

The player may pin goals (e.g. "Play in top division by 21", "100 caps", "Win league with boyhood club"). The game tracks progress and generates reflection moments. Goals never change world odds.

## 8. Discipline & professionalism incidents

Late to training, fines, disputes, suspensions — generated from personality & choices (e.g. skipping recovery, arguing publicly). Clubs fine per internal rules; repeated issues affect status and transfer interest. Kept football-focused; no glamorising harmful behaviour (10 §12).

## 9. International career

- Youth call-ups by age category; senior call-ups.
- Dual eligibility decisions: federations may approach; switching per rules engine; decision has long-term consequences (cannot switch back after competitive senior caps above threshold).
- International windows affect fatigue, club relations (club may complain), reputation boost.
- Tournaments: squad selection drama, pre-tournament camp, tournament matches, media intensity, awards.

## 10. Leadership path

Vice-captain → captain; leadership duties (team meetings, mediating conflicts, speaking to media); leadership can grow with experience (hidden Leadership development), language proficiency matters.

## 11. Retirement

- Triggered only by the player (or by permanent medical ineligibility, handled sensitively).
- Retirement advisor: shows physical outlook (perceived), market interest, financial readiness, family preference.
- Options: retire at season end, announce farewell tour, testimonial match, drop down leagues, go abroad for a final chapter, become player-coach.

## 12. Post-career (the Person continues)

| Path | Systems |
|------|---------|
| Coaching | Coaching badges (course schedules, costs, exams), youth coach → assistant → manager (becomes a manager Person; could hand off to a manager-mode later, v2) |
| Punditry / media | Media offers based on reputation & Media Handling; weekly segments |
| Scouting | Scout role at a club |
| Agent | Start an agency (v2) |
| Club ambassador | Legends at former clubs |
| Business & life | Investments, family, charity foundation (10) |
| Full retirement | Legacy screen, epilogue |

v1 scope: post-career as a narrative/life-sim phase with light systems; manager mode continuation is v2.

## 13. Autopilot / delegation

For busy stretches the player can delegate: training focus policy, renewal acceptance thresholds, loan acceptance rules, media tone default. Delegation uses the protagonist's own AiMind + policies.

## 14. Fairness notes specific to the career layer

- Selection forecast, agent reports, interest notices all derive from perceptions available in-world.
- Conversation outcomes use the same functions as AI-AI conversations (e.g. an AI player asking for more minutes).
- The protagonist never receives "hints" that AI players couldn't act on.
