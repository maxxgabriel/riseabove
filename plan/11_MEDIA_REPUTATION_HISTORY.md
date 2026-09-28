# 11 — Media, Fans, Reputation, Awards, History & Legacy

## 1. Media ecosystem

- **Outlets** (Persons/organisations): national papers, local papers, tabloids, TV/radio, podcasts, football data sites, fan channels. Attributes: reach (by region), accuracy, sensationalism, club bias, tone.
- **Journalists**: persons with specialities (club beats, transfer insider), relationships with players/agents (sources), credibility.
- **Stories** are generated from world events via templates with parameters; stored as template id + params (localisable, compact).

### Story types

Match reports, player ratings columns, transfer rumours (true/false with reliability tied to journalist source quality), interviews, press conference quotes, controversies, milestone features, season reviews, "wonderkid lists", injury news, manager pressure stories, fan protests, award shortlists, retirement tributes, "where are they now" pieces (history-driven).

## 2. Press conferences & interviews

- Pre/post-match (when selected by club), mixed zone, national team duty, sponsor events.
- Questions drawn from context (form, rumours, rivalry, manager comments).
- Answer options with tones; consequences: manager relationship, fan sentiment, rival reaction, media narrative, own confidence/pressure. Same resolution model for AI players' quoted answers.
- "No comment" always available.

## 3. Social media

- Followers per platform (abstract platforms), growth from reputation/performances/posts.
- Posts: training content, celebrations, charity, sponsor posts (contract obligations), replies to criticism (risk).
- Fan sentiment feed (templated), including rival fans.
- Controversy risk from impulsive posts (Temperament, Controversy) — bounded, never unavoidable.

## 4. Fans

- Club fanbase: size, loyalty, expectations, patience, rivalry map.
- Player–fanbase affinity per club: builds with performances, loyalty, derby goals, local origin, statements; drops with poor form, transfer requests, joining rivals.
- States: Idol, Favourite, Liked, Neutral, Criticised, Unwanted, Villain (e.g. after moving to a rival). Affects home morale modifiers (small), media tone, legacy.

## 5. Reputation layers

| Layer | Scope | Inputs |
|-------|-------|--------|
| Sporting reputation | football world (regional map) | performances weighted by competition prestige, honours, caps |
| Public image | general public/media | conduct, charity, media behaviour, controversies |
| Club-specific legend status | per club | appearances, key moments, trophies, loyalty |
| Professional reputation (insider) | managers/agents/scouts | training attitude, professionalism reports, reliability |

Reputation decays slowly without exposure; big tournaments amplify.

## 6. Awards system

- Matchday: player of the match.
- Monthly: league player/young player/goal of the month (voting by panel model).
- Season: league player of the season, young player, team of the season, top scorer, most assists, golden glove, fans' awards, club awards (player of season, young player).
- Continental & global: annual best player award with a voting model (journalists/coaches/captains blocs with biases: reputation, trophies, big-match performances, international tournaments), young player award (U21), goalkeeper award, team of the year.
- Tournament awards: golden ball/boot/glove, best young player.
- Hall of fame inductions (clubs, nations) after retirement.

Voting model: each voter bloc ranks candidates with weights + regional bias + noise; results published with points.

## 7. Records & statistics

- Records tracked per competition, club, nation, world: most appearances, goals, fastest goals, youngest/oldest scorer/debutant, consecutive appearances, clean sheets, unbeaten runs, transfer fees, attendance, biggest wins.
- Record-break detection triggers media stories and legacy entries.
- Statistics aggregation: per season per competition per player; all-time aggregates; head-to-head.

## 8. History & world timeline

- Season archives: tables, winners, top scorers, awards, promotions/relegations, manager changes, record transfers.
- Club histories: honours timeline, managers list, legends, attendance history, finances summary.
- Person histories: career tables, awards, caps, transfers, injuries, key matches.
- "World news archive" browsable by date.
- Compaction (01 §8) preserves aggregates and notable events forever.

## 9. Legacy screen (end of career & anytime)

- Career timeline (clubs, loans, internationals), stats, honours, awards, records.
- Reputation curve over time; club legend statuses; fan quotes.
- Relationships: best mentors, closest teammates, rivals, family milestones.
- "Greatest moments" (auto-selected from events with high narrative score).
- Comparison to the world's all-time lists (percentile).
- Post-career epilogue text (generated from state).
