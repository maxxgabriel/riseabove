# 12 — UI / UX

## 1. UX principles

1. **Player's eyes.** Every screen is framed as what the player knows, sees, and hears. Uncertainty is shown, not hidden (ranges, confidence badges, "according to your agent").
2. **One primary action per moment.** The home screen always answers "what should I do now?".
3. **Readable depth.** Summary first, detail on demand (drill-down panels, not modal walls).
4. **Calm by default.** Notifications are prioritised; the game never spams.
5. **Keyboard-first and accessible.** Full keyboard navigation, screen-reader labels, colour-blind safe palettes, scalable text (80–200%), reduced-motion mode.
6. **Fast.** Screen transitions < 100 ms from cache; heavy queries streamed with skeletons.

## 2. Information architecture

```
Top bar: Date · Advance controls (Next phase / Next day / Until event / Until date) · Pending decisions badge · Search
Left nav:
  Home
  Calendar
  Inbox
  Training
  Match Day
  Performance
  Club (squad, dressing room, staff, club info)
  Career (contract, agent, transfers, goals, history)
  International
  Life (schedule, family & relationships, home, education, health)
  Money
  Media (press, social, reputation)
  World (tables, fixtures, news, transfers, awards, records, database browser)
  Settings
```

## 3. Key screens

### 3.1 Home
- Today card: schedule blocks, next match countdown, pending decisions with deadlines.
- Status strip: condition, sharpness, morale, confidence, stress (icons + words; exact values only where the club shares data).
- Selection outlook for next match (band + main factor).
- Latest from: manager/coach, agent, family, media (one line each).
- Week planner quick-edit (time budget).

### 3.2 Calendar
- Month/week views: matches (competition-coloured), training blocks, international windows, transfer windows, contract dates, education exams, family events, sponsor duties.
- Drag to plan free time; conflicts highlighted (e.g. sponsor event vs recovery day).

### 3.3 Inbox
- Threads by sender (Manager, Coach, Agent, Club, Family, Partner, Friends, Media, Federation, Sponsors).
- Decision messages show: options, consequences hints (perceived), deadline, default action.
- Filters: needs action, football, life, money.

### 3.4 Training
- Club weekly schedule (read-only unless manager allows input).
- Personal plan: focus, extra sessions, recovery, trait/position learning.
- Coach reports: last session ratings, 4-week trend, feedback quotes.
- Load gauge (if sports-science shares data).

### 3.5 Match Day
- Pre-match: squad sheet reveal animation, opponent brief (perceived), personal prep choice, manager instructions for your role.
- Live: text commentary feed, 2D radar, score/time, protagonist stat tally, momentum bar, key events timeline, speed controls (×1/×4/×16/highlights only/instant), Player Moments prompts.
- Post-match: rating with breakdown, stats vs season average, heatmap, pass map (from recorded actions), coach comment, media quotes, fan sentiment, injury report.

### 3.6 Performance
- Season/career stats tables with competition filters.
- Charts: rating trend, minutes per month, xG vs goals, per-90 radar vs league position peers (percentiles).
- Perceived attributes: bars with uncertainty ranges and history.

### 3.7 Club
- Squad list (perceived; position depth chart with you highlighted).
- Dressing room: relationship web (friends/rivals/leaders), hierarchy, social groups.
- Staff directory with who you get on with.
- Club info: finances summary (public), board mood (media-reported), facilities, history.

### 3.8 Career
- Contract: terms, bonuses progress, clauses, expiry countdown, renewal status.
- Agent: instructions, monthly report, interest board (as presented), change agent.
- Transfers: rumours (with reliability), official approaches, negotiation state (via agent).
- Goals & milestones; career history timeline.

### 3.9 International
- Eligibility panel (all nations + status + rule explanations).
- Call-up history, caps/goals, current squad, upcoming windows/tournaments.

### 3.10 Life
- Weekly schedule (time budget editor with presets).
- Family & relationships cards (relationship meters, recent interactions, upcoming events).
- Home (housing, location, commute), relocation planner.
- Education (courses, grades, exams).
- Health & mind (well-being gauges, support options).

### 3.11 Money
- Monthly statement, income breakdown, taxes, expenses, net worth chart, assets, sponsorships, financial advisor.

### 3.12 Media
- Press conference view, interview history, social media composer (templated posts), reputation layers, fan affinity per club, press coverage feed.

### 3.13 World
- Competition hubs (tables, fixtures, stats, awards), news, transfer centre (public deals), records, history archive, database browser (only perceived data about others: public stats, reputation, ages, clubs).

## 4. Conversations UI

Dialog panel with speaker portrait (generated face system, §8), context summary, options with tone chips, and "what I know" sidebar (relationship state, previous promises). Outcomes appear as immediate reactions + later consequences in inbox.

## 5. Notifications & advance flow

- Priority levels: Blocking (must decide now), Important (deadline soon), Info.
- "Until event" stops on Blocking and Important by default (configurable).
- Deadlines show default outcome ("If you don't respond, your agent will decline.").

## 6. Onboarding

- Creation wizard (09 §1) with explanations of fairness ("your talent is hidden, like everyone's").
- Guided first week with contextual tips; tips can be disabled; glossary tooltips on every metric.

## 7. Visual design system

- Design tokens (color, spacing, type scale), light/dark themes, competition colour coding, status iconography.
- Components: data tables (virtualised, sortable, column presets), stat cards, range bars, sparkline, radar, heatmap, timeline, relationship graph, calendar grid, conversation panel.
- Typography: legible numerals (tabular), 14px base.

## 8. Generated assets

- Faces: 2D layered portrait generator (seeded) with age progression.
- Club crests & kits: procedural generator (shapes, colours, patterns) for fictional world; user data packs can supply images.
- No real logos shipped.

## 9. Localisation

- All text via ICU message format; templates for commentary/news use grammatical-gender-aware slots.
- Initial: English; architecture supports others.
- Date/number/currency formatting per locale.

## 10. Performance & tech

- React with virtualised lists; queries via Tauri commands; memoised view models; web workers for chart prep; live match streamed events.
- Target: 60 fps UI, first paint < 1.5 s after load.

## 11. Accessibility checklist

Keyboard traps none; focus visible; ARIA roles; colour contrast ≥ 4.5:1; text alternatives for charts (data tables); adjustable match speed; no flashing effects.
