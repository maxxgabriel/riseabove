# Rise Above — Complete Desktop UI Plan

## 1. The direction

Rise Above should look and behave like a serious football simulation: a substantial desktop game built around people, clubs, competitions, conversations, records, and decisions.

The interface should make a complicated world understandable without turning it into a simplified career story.

The central distinction is:

**The world exists independently. You temporarily provide one person’s decisions.**

The UI must make that distinction visible everywhere. Looking at a club does not mean controlling it. Knowing a person does not mean knowing their thoughts. Being employed does not mean having authority over the whole organisation.

### Decisions established for this plan

| Area | Direction |
|---|---|
| Product | Installed desktop game, not a website or browser-based application |
| Playable scope | All supported kinds of people, delivered in phases |
| Football matches | Watch-only; no direct movement, scripted player moments, or reaction-time choices |
| World clock | Advances when commanded; browsing does not consume days |
| Observer mode | Omniscient, explicitly separated from inhabiting a person |
| Visual direction | Dense, restrained, football-first, closer to a management game than a productivity application |
| Progression | Emergent circumstances, not chapters, unlock trees, or a prescribed career ladder |
| Interaction | Mouse and keyboard; no mobile-first compromises |
| Data | Real simulation state, attributed reports, and persistent history |
| AI | Simulation decision-makers, not an assistant panel or a conversational interface replacing the game |

### What the repository actually contains

The inspected client is an empty entry point, not an existing interface that needs reskinning. The older [UI plan](12_UI_UX.md) proposes React/Tauri, but no corresponding frontend is present.

The simulation already has useful foundations: persistent people, clubs, teams, staff, competitions, contracts, decisions, relationships, negotiations, and match reports.

However:

- The current career wrapper is player-specific and references unfinished modules.
- Several life systems remain design intentions rather than complete world systems.
- Existing perception storage is primarily club/player-oriented, not a complete person-specific knowledge model.
- Stadiums currently include club fields rather than a complete independent venue model.
- Match reports contain events and zones, not continuous player-position tracking.

This is therefore a **complete target UI specification with explicit implementation dependencies**, not a claim that every screen can already be connected to finished systems.

Your supplied North Star and rules override conflicting older documents. The full companion documents were not found in the inspected files; this plan does not invent the missing S-rule definitions.

---

## 2. Desktop technology and application structure

### Recommended implementation

Use **Rust + egui/eframe with the wgpu renderer**, built as a native Windows application first.

The reasons are specific to this project:

- The simulation is already Rust.
- The interface is predominantly tables, reports, timelines, forms, and a 2D match view.
- The game needs close integration with a large local simulation, not web deployment.
- A custom-painted game interface is appropriate; it does not need to resemble Windows Settings.
- It avoids maintaining a separate JavaScript frontend and a webview messaging layer.

Eframe provides the native application framework, while egui supports custom 2D drawing and accessibility integration. These capabilities fit the proposed client; they do not automatically guarantee performance or accessibility. [Official egui documentation](https://github.com/emilk/egui)

Use its native renderer configuration and persisted window settings. Do not build the browser target. [Eframe native options](https://docs.rs/eframe/latest/eframe/struct.NativeOptions.html)

### Important qualification

Do not ship default egui demo styling.

Rise Above needs a dedicated component layer for:

- Football tables.
- Profile headers.
- Attribute reports.
- Fixture lists.
- Contract comparisons.
- Conversation transcripts.
- Calendars.
- Pitch diagrams.
- Match timelines.
- News articles.
- Financial statements.

The framework supplies rendering and interaction primitives. Rise Above supplies the visual identity and domain behaviour.

### Application separation

Use three clear boundaries:

1. **Simulation:** owns world state and resolves actions.
2. **Client-view layer:** produces information appropriate to the current perspective.
3. **Desktop presentation:** draws screens and submits typed commands.

The UI never directly modifies a club balance, relationship, contract, selection, or injury.

The client must not depend on the current player-specific `Career` object as the universal application session. The session identifies the world and, optionally, the inhabited `PersonId`.

### Early technical acceptance gate

Before building many pages, prove these five things in a small native client:

- A large, keyboard-navigable virtualised table.
- A readable profile page at different display scales.
- A resizable three-pane inbox.
- A pitch canvas with a synchronised event timeline.
- Accessible forms and table navigation using Windows screen readers.

If a component needs custom accessibility semantics, implement them in the shared component, not separately on every page.

---

## 3. Visual identity

### Overall character

Think:

- A football annual.
- A club’s working documents.
- A match programme.
- A personal diary.
- A detailed sports database.

Not:

- A startup dashboard.
- A Kanban application.
- A collection of floating statistic cards.
- A cinematic character-selection game.
- A chat window pretending to be the whole product.

### Default palette

| Purpose | Starting colour |
|---|---|
| Main background | `#171B1E` |
| Navigation background | `#121619` |
| Main panels | `#20262B` |
| Raised controls | `#293139` |
| Borders and separators | `#414B53` |
| Primary text | `#E8ECEF` |
| Secondary text | `#B4BEC6` |
| Primary accent | `#89AD91` |
| Links and informational markers | `#9BBFD8` |
| Warnings | `#E5BD75` |
| Errors and adverse events | `#ED9595` |

These are starting tokens, subject to measured contrast checks.

Club colours appear in small identity areas: a narrow header stripe, crest, kit, or pitch marker. They do not recolour every screen.

Provide a light theme with the same hierarchy: off-white document surfaces, dark text, restrained green and blue accents.

### Typography

- Windows default: Segoe UI with appropriate script fallbacks.
- Standard body text: 14 logical pixels.
- Comfortable mode: 16.
- Table numerals: tabular figures.
- Main page title: 22.
- Section heading: 16.
- Secondary metadata: 12–13, never essential content exclusively.
- No oversized marketing headlines inside the game.
- Long reports use short paragraphs and a controlled reading width.

### Density and spacing

- Base spacing unit: 4 logical pixels.
- Common gaps: 8, 12, 16.
- Compact table rows: 26.
- Standard table rows: 30.
- Comfortable rows: 36.
- Buttons: approximately 30–34 high.
- Borders: generally one pixel.
- Corner radius: 2–4, not pill-shaped everywhere.
- Shadows only for genuinely floating menus and dialogs.

### Explicit exclusions

Do not introduce:

- Decorative gradients.
- Glass effects.
- Glowing controls.
- Oversized empty margins.
- A grid of KPI cards as the default page template.
- Sparkles, “AI insights,” or generic assistant suggestions.
- A universal overall score for a person’s life.
- Success confetti for ordinary events.
- Animated number counters.
- Fake activity feeds.
- Unexplained “momentum” or “potential” meters.

Charts must answer a specific question. Otherwise, use a table.

---

## 4. The permanent application shell

### Default desktop layout

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Rise Above  Back Forward  Current perspective   Date    Advance ▾   Search │
├───────────────┬─────────────────────────────────────────────────────────────┤
│ Today         │ Club > First Team > Person                                 │
│ Messages      │ PERSON / ORGANISATION / PAGE TITLE                         │
│ Calendar      │ Relevant tabs                                              │
│               ├──────────────────────────────────────────────┬──────────────┤
│ My Person     │                                              │ Optional     │
│ My Work       │ Main content                                 │ inspector    │
│ People        │                                              │              │
│ Life          │ Tables, documents, pitch, schedule,           │ Related      │
│ Money         │ correspondence, history                      │ information  │
│               │                                              │              │
│ World         │                                              │              │
│ History       │                                              │              │
│ Bookmarks     │                                              │              │
├───────────────┴──────────────────────────────────────────────┴──────────────┤
│ Paused • World date • Current operation / save status                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Shell dimensions

At 1920 × 1080:

- Navigation rail: 196 logical pixels.
- Top bar: approximately 48.
- Breadcrumb and page-header area: content-dependent, generally 68–96.
- Optional inspector: 300–360.
- Status bar: 24.
- Main content occupies the remaining space.

At 1366 × 768:

- Rail reduces to 168.
- Inspector is closed by default and opens on demand.
- Secondary columns become optional.
- Tabs may move into an overflow menu.
- Tables scroll horizontally rather than shrinking text.

Minimum supported window: 1280 × 720. At large text scales, content reflows and the rail can collapse to an accessible navigation menu.

### Navigation behaviour

- Back and Forward preserve filters, scroll position, selected rows, and tabs.
- Clicking a named entity opens its profile.
- A dedicated inspect action opens a preview without leaving the current page.
- Middle-click or Ctrl-click opens another in-game document tab.
- Document tabs are optional, with overflow rather than shrinking indefinitely.
- Bookmarks retain routes and view settings, not unrestricted copies of private data.
- A person changing clubs does not break a saved link.
- A retired or deceased person remains searchable through history.

### Perspective indicator

The top bar must always say either:

- **Inhabiting: [Person]**
- **Observing world — omniscient**

Observer mode uses an additional persistent coloured border or label, not merely a tiny icon.

This is important enough to remain visible in fullscreen match viewing.

### What belongs in the rail

Keep the permanent rail short. Detailed destinations live inside their relevant section.

“My Work” changes with the actual person’s roles:

- Player: Football.
- Manager: Team.
- Scout: Scouting.
- Agent: Representation.
- Journalist: Reporting.
- Director: Club Responsibilities.
- Multiple roles: a role selector within My Work.

Personal pages do not vanish because someone loses their job.

---

## 5. Time, control, authority, and knowledge

### 5.1 World clock

The default state is paused.

The main control is **Advance**, with:

- Next day.
- Until an important decision.
- Until the next relevant match.
- Until a selected date.
- A specified number of days.

A long advance command can run while the user browses or minimises the application. This is an explicit running state, not the default behaviour.

The status bar displays the requested destination and current simulation date.

“Stop” requests a stop at the next safe simulation checkpoint. It does not abandon a half-applied transfer or match.

Closing the application does not simulate elapsed real-world time.

### 5.2 Stops and deadlines

Distinguish:

- An optional user-configured pause.
- A simulation decision deadline.
- A blocking application error.

These are not the same thing.

A decision must never require a human answer for the world to remain valid. It always has a default supplied by the person’s ordinary decision system.

Default stop policy while inhabiting:

- Stop before a pending decision’s resolution boundary.
- Stop before a personally relevant match if watching is enabled.
- Stop for a major change such as dismissal, relocation completion, or a serious personal event.
- Do not stop for every article, result, or routine training report.

The user can disable these stops for a long run. Show the applicable default-handling policy before starting.

Same-day decisions require safe checkpoints within the daily pipeline. Do not build this behaviour by running a full day and discovering afterward that the opportunity has expired.

### 5.3 Taking and relinquishing control

Only one person is inhabited at a time.

Taking control:

1. Open the person.
2. Select **Inhabit** from the perspective controls.
3. Review their circumstances and supported responsibilities.
4. Switch at a safe boundary.
5. Land on Today in that person’s perspective.

Stepping out:

- Preserves submitted instructions and committed actions.
- Returns unanswered decisions to the ordinary AI route.
- Does not reset relationships, cancel contracts, regenerate circumstances, or erase commitments.
- Opens the observer world overview.

No special talent increase, transfer protection, selection advantage, or retirement exemption accompanies control.

### 5.4 Authority

Actions depend on real responsibilities, appointments, contracts, and delegations.

For example:

| Person | Can do | Cannot assume |
|---|---|---|
| Player | Request a loan, discuss selection, accept personal terms | Set club transfer budgets |
| Coach | Submit assessments, manage assigned sessions | Select the first team without delegation |
| Manager | Set authorised squad and training decisions | Overrule ownership or competition rules |
| Scout | Observe, report, recommend | Sign players |
| Agent | Negotiate within a client’s mandate | Accept every offer without consent |
| Director | Act within assigned financial/recruitment authority | Control all board members |
| Journalist | Request interviews, investigate, submit stories | Know confidential facts automatically |
| Family member | Make their own personal decisions | Control the inhabited person’s partner or children as inventory |

An unavailable action either is absent because it is irrelevant, or is disabled with a precise reason when that reason is useful.

### 5.5 Knowledge

Inhabited views distinguish:

- Public fact.
- Direct personal knowledge.
- A report from a named source.
- An estimate.
- A rumour.
- Unknown information.

An assessment shows who made it and when.

Do not expose hidden ability, private negotiation ceilings, another person’s exact trust score, or undisclosed injuries through tooltips, sorting, exports, or charts.

### 5.6 Omniscient observation

Observer mode can inspect actual simulated state, including hidden values and private relationships.

However:

- It is read-only.
- Inspecting an internal decision does not permit overriding it.
- Debug editing is not part of ordinary observer mode.
- Taking control clears and re-queries all perspective-sensitive presentation.
- Open comparisons, inspectors, search results, and exports follow the new perspective.
- The application cannot make the human forget what they saw; acknowledge this once when first entering omniscient mode rather than claiming the mode is competitively fair.

---

## 6. Shared components and interaction rules

These rules apply to every page below.

### 6.1 Entity headers

A person header contains:

- Name and portrait or initials.
- Age and relevant nationalities.
- Current roles and organisations.
- Location when known.
- Contextual status.
- Follow, bookmark, compare, and legitimate contact actions.

A club header contains:

- Crest.
- Name.
- City and nation.
- Competition.
- Current season.
- Team selector.

A small portrait is an identity aid, not the dominant content.

### 6.2 Tables

Every substantial table supports:

- Stable sorting.
- Column resizing.
- Column selection.
- Named view presets.
- Filters with visible active criteria.
- Keyboard row navigation.
- Copy selected values.
- Permission-filtered CSV export where meaningful.
- A pinned identifying column.
- A visible row count.

Large tables render visible rows rather than every database row. Egui’s table API specifically supports this approach. [Table virtualisation documentation](https://docs.rs/egui_extras/latest/egui_extras/struct.TableBody.html)

Important details:

- Unknown values remain unknown and sort separately.
- A missing value is not converted to zero.
- Changing a filter does not silently reset all other filters.
- Live updates do not reorder the table under the pointer; offer an update indicator.
- Row selection and opening an entity are distinct actions.
- Checkbox-based bulk actions appear only when the inhabited person can legitimately perform the action in bulk.

### 6.3 Inspectors

The right-side inspector provides:

- A short entity summary.
- Relevant recent history.
- Related links.
- Source and freshness details.
- Contextual actions.

It should reduce navigation, not hide the only copy of essential information.

### 6.4 Forms and submissions

Use explicit verbs:

- Request meeting.
- Submit team.
- Propose terms.
- Accept offer.
- Decline invitation.
- Publish statement.
- Assign scout.

Avoid a generic “Continue” button for consequential actions.

A form distinguishes:

- Draft.
- Submitted.
- Accepted or applied.
- Rejected.
- Expired.
- Superseded.

The UI waits for simulation acknowledgement before presenting an action as successful.

### 6.5 Confirmations

Use confirmations for:

- Binding contracts.
- Public statements.
- Resignation.
- Retirement announcements.
- Major financial commitments.
- Switching perspectives when a draft would be lost.

Do not confirm every training adjustment or navigation action.

The confirmation summarises the actual commitment and consequences known to the person. It does not reveal future outcomes.

### 6.6 Common states

Every page must handle:

1. Loading.
2. Available data.
3. No relevant records.
4. Unknown or inaccessible information.
5. Historical or incomplete information.
6. Recoverable error.

Examples:

- “No offers have been received.”
- “Your agent has not provided an update.”
- “Detailed positional data was not recorded for this match.”
- “This report was written before the player’s transfer.”
- “You no longer have access to this club’s internal finances.”

“No offers” is a valid game state, not an invitation to generate one.

### 6.7 Keyboard conventions

- Alt–Left / Alt–Right: history.
- Ctrl–F: search within the current page.
- Ctrl–K: global entity search.
- Ctrl–S: save.
- Escape: close the topmost menu or inspector; never silently discard a consequential draft.
- F1: contextual help.
- Advance and match playback shortcuts: configurable and inactive while editing text.

No essential function is hover-only.

---

## 7. Start, setup, and session pages

### 7.1 Main menu

**Purpose:** enter or resume a world without a marketing-style landing page.

Layout:

- Left: Continue, Load World, New World, Settings, Credits, Exit.
- Right: last-world summary.
- Footer: version and data-pack compatibility.

The last-world summary shows:

- World name.
- Simulation date.
- Last played.
- Inhabited person or observer mode.
- Save health.

Do not show fabricated personal news before loading the save.

### 7.2 Load World

Use a save table rather than a grid of giant thumbnails.

Columns:

- World.
- Date.
- Perspective.
- Last saved.
- Version.
- File size.
- Compatibility status.

Selecting a save opens:

- World description.
- Data-pack dependencies.
- Last known inhabited person.
- Available backups.
- Migration warning if relevant.

Actions:

- Load.
- Load backup.
- Duplicate world.
- Locate file.
- Delete with confirmation.

An incompatible save explains the exact issue. Never silently replace an incompatible dataset.

### 7.3 New World — source

Offer:

- Build from an installed dataset.
- Import a supported CSV dataset.
- Generate a synthetic world for testing or fictional play.

Show dataset coverage:

- Nations.
- Competitions.
- Clubs.
- People and roles.
- Venues.
- Media organisations.
- Other supported entities.

Separate “present in source files” from “recognised and supported by the importer.”

A binary file is not automatically a playable database merely because it can be opened.

### 7.4 Dataset inspection and import

Three panes:

- Source files.
- Validation results.
- Selected-file preview.

Validation categories:

- Missing required fields.
- Invalid references.
- Duplicate identities.
- Unrecognised roles.
- Invalid dates or amounts.
- Partial records.
- Unsupported entity types.
- Conflicting sources.

Show counts of accepted, rejected, and unresolved records.

Provide:

- Read-only preview.
- Mapping to supported import fields.
- Export validation report.
- Cancel import.
- Build world when requirements are met.

Never label guessed binary fragments as verified semantic data.

Preserve source IDs and provenance. Imported and generated values must be distinguishable.

### 7.5 World configuration

Use a configuration document with sections:

- World name and starting date.
- Dataset and rules version.
- Nations and competitions.
- Population generation.
- Simulation coverage.
- Historical detail retention.
- Save preferences.

Show a measured benchmark estimate when available, not a fabricated “high performance” badge.

Do not let “detail level” secretly change the outcomes for the inhabited person. Recording and presentation detail are separate from simulation rules.

### 7.6 Enter the world

Choices:

- Observe.
- Inhabit an existing person.
- Add a new ordinary person, once world-generation support exists.

No mandatory career-stage wizard.

For existing people, provide filters by:

- Role.
- Age.
- Nation.
- Organisation.
- Employment status.
- Availability for supported control.

The preview shows actual circumstances, not a difficulty rating implying a promised career.

### 7.7 New ordinary person

Use one editable dossier, not a sequence of dramatic reveal screens.

Sections:

- Identity.
- Background.
- Family connections.
- Location.
- Initial role and circumstances.
- Appearance, where supported.

Rules:

- Initial circumstances must use ordinary world-generation rules.
- No attribute allocation granting special advantages.
- No guaranteed academy acceptance, professional contract, or elite potential.
- Do not display hidden talent through a “recommended archetype” preview.
- Unsupported starting circumstances are unavailable with an explanation.

Creation inserts a person into the same world model as everyone else.

### 7.8 World loading and recovery

Show genuine progress stages:

- Reading save.
- Validating compatibility.
- Rebuilding indexes.
- Preparing views.

Allow cancellation only at safe points.

Recovery page:

- What failed.
- Whether the original file remains intact.
- Last valid backup.
- Available recovery action.
- Diagnostic details on demand.

Never overwrite the only valid save during migration.

---

## 8. Daily life and communication pages

### 8.1 Today

This is a personal briefing, not a dashboard of arbitrary scores.

Main area:

1. Today’s commitments.
2. Decisions awaiting a response.
3. Significant changes since the last viewed date.
4. Relevant upcoming dates.

Side area:

- Current role and employment.
- A compact personal condition summary.
- Next match or work assignment.
- Recently contacted people.

For a footballer, relevant information includes training, selection communication, recovery, and contract matters.

For a journalist, it includes deadlines, scheduled interviews, and assignments.

For an unemployed person, it includes ordinary life and ongoing opportunities—not a blank “no club” screen.

Actions are attached to actual items. The game does not invent a single “best next action.”

### 8.2 Messages

Resizable three-pane layout:

- Folders and filters.
- Message list.
- Selected correspondence.

Folders:

- All.
- Awaiting response.
- Work.
- Personal.
- Contracts and money.
- Invitations.
- Archived.

List columns:

- Sender.
- Subject.
- Received date.
- Response deadline.
- State.

A thread retains the full sequence of communication.

An offer that expires stays visible as an expired offer. Reading a message does not accept or reject it.

### 8.3 Decision detail

This opens inside the message reader or as a full page when the decision is complex.

Show:

- Who is asking.
- What is being requested.
- Relevant documents.
- Available responses.
- Deadline.
- What happens without a response.
- Existing commitments affected.

A contract offer links to its full terms. A medical recommendation links to the actual report.

Do not show numerical forecasts of another person’s emotional response unless the game genuinely models an in-world assessment.

### 8.4 Conversations

A document-like conversation view:

- Participants at the top.
- Context and previous discussion in a narrow side column.
- Transcript in the main area.
- Available topics and responses at the bottom.

Actions can include:

- Ask.
- Clarify.
- Propose.
- Disagree.
- Defer.
- End conversation.

Tone is a modifier to what is said, not a magic success selector.

Show explicit statements, refusals, promises, and unresolved points. A conversation can end without agreement.

No relationship points floating above portraits.

### 8.5 Calendar

Views:

- Agenda.
- Week.
- Month.

The week view is the primary planning surface.

Entries distinguish:

- Required attendance.
- Requested attendance.
- Personal plans.
- Tentative plans.
- Public events being followed.

Each entry shows:

- Time or day block.
- Location.
- Participants.
- Travel requirement.
- Source.
- Confirmation state.

Dragging an item drafts a change; it cannot move a club match or another person’s meeting without their agreement.

For day-level simulation systems, use honest day blocks rather than invented minute-accurate appointments.

### 8.6 Routine planner

A recurring routine editor separate from the calendar.

Show:

- Fixed obligations.
- Available time.
- Rest.
- Work or training.
- Travel.
- Relationships.
- Study.
- Other activities.

Provide named routines such as:

- Normal working week.
- Rehabilitation.
- Off-season.
- Travelling.
- Study-heavy period.

A routine is a standing intention. Actual events can interrupt it.

Display conflicts in plain language: “This leaves insufficient time for the scheduled journey.”

Do not express the person’s entire week as an optimisation puzzle with guaranteed numerical rewards.

### 8.7 Commitments and promises

Two tabs:

- Commitments I made.
- Commitments others made to me.

Columns:

- Parties.
- Subject.
- Date made.
- Due date or review period.
- Current interpretation.
- Supporting evidence.

Detail includes the original conversation and later changes.

Actions:

- Discuss.
- Clarify.
- Renegotiate.
- Withdraw where allowed.
- Mark a personal reminder.

The UI does not declare a disputed promise objectively broken merely because one party believes it is.

### 8.8 Personal notes and bookmarks

Allow notes attached to people, clubs, matches, and dates.

Notes are user-authored and do not change simulation outcomes.

Separate them visibly from:

- A scout’s report.
- An agent’s assessment.
- A recorded promise.
- A simulation-generated event.

No automatically generated “career mission list.”

---

## 9. Person, career, and relationships

### 9.1 Person overview

The canonical page for every person.

Sections:

- Current circumstances.
- Roles and organisations.
- Recent public or known activity.
- Relevant professional summary.
- Relationships known to the viewer.
- Career history excerpt.

Tabs appear according to the person’s actual records:

- Overview.
- Football or profession.
- Statistics.
- Employment.
- Relationships.
- History.

Private personal tabs are available only to the person, authorised viewers, or the omniscient observer.

The same identity continues through playing, coaching, unemployment, family changes, and retirement.

### 9.2 My Person

A private version of the person profile.

Tabs:

- Overview.
- Personal circumstances.
- Professional assessments.
- Priorities.
- Documents.
- History.

Show what the person can know about themselves, not the engine’s hidden values.

“Priorities” means stated intentions such as preferring family proximity or regular playing time. They inform ordinary decision policies; they are not slider-based personality cheats.

### 9.3 Football attributes and assessments

Use familiar grouped attribute columns:

- Technical.
- Mental.
- Physical.
- Goalkeeping when relevant.

Values are:

- Assessed ranges.
- Qualitative descriptions.
- Unknown.

Alongside the values:

- Assessing coach or scout.
- Report date.
- Observation basis.
- Change since a comparable earlier assessment.

Position familiarity uses a compact pitch diagram and a corresponding text list.

No public CA/PA numbers. Omniscient mode can show them in a clearly marked internal-state section.

### 9.4 Performance statistics

Primary content is a season/competition table.

Filters:

- Season.
- Competition.
- Team.
- Starting/substitute appearances.
- Position.
- Minimum minutes.

Columns adapt to role and position.

Examples:

- Appearances, starts, minutes.
- Goals, assists.
- Shots and xG where recorded.
- Chances created.
- Passing.
- Defensive actions.
- Goalkeeping.
- Cards.
- Ratings, with source.

Charts are optional supporting views. Every chart has a table equivalent.

Per-90 statistics show their sample size. Do not rank a ten-minute substitute beside a full-season starter without making the difference obvious.

### 9.5 Comparison

Compare up to four people side by side.

Choose a comparison basis:

- Current assessment.
- Season output.
- Career output.
- Contract terms, if known.

Keep:

- Units consistent.
- Competition context visible.
- Report dates visible.
- Missing information explicit.

Default to aligned rows and bars rather than a large radar chart.

There is no universal “better player” verdict.

### 9.6 Employment and professional history

Chronological table:

- Organisation.
- Role.
- Team or department.
- Start.
- End.
- Status.
- Relevant record.

Loans and overlapping roles are represented explicitly.

A player-coach has two responsibilities, not a renamed player record.

Opening a historical stint filters matches, people, contracts, and events to that period.

### 9.7 Opportunities

A real opportunity register, not a linear pipeline.

Possible records:

- Trial invitation.
- Job opening.
- Formal approach.
- Agent-reported interest.
- Application.
- Interview.
- Negotiation.
- Withdrawn or expired proposal.

Columns:

- Organisation.
- Role.
- Source.
- Location.
- Current state.
- Last contact.
- Deadline.

Several opportunities can coexist, stall, reopen, or disappear.

No “Academy → Breakthrough → Star” progress rail.

### 9.8 Contacts and relationships

Default view: a practical list.

Columns:

- Person.
- Relationship context.
- Organisation or household.
- Last contact.
- Known upcoming commitment.
- Current visible situation.

Filters:

- Family.
- Friends.
- Colleagues.
- Representatives.
- Professional contacts.

Use descriptions such as “recent disagreement” or “regular contact,” not exact private affinity scores.

An optional network view shows only meaningful known connections and has a table alternative.

### 9.9 Relationship detail

Main sections:

- Shared history.
- Recent conversations.
- Known commitments.
- Current circumstances.
- Contact options.

Actions can include inviting, asking, arranging, apologising, or discussing a specific issue.

Another person can decline or propose a different time.

Do not offer buttons whose sole visible purpose is “increase relationship.”

### 9.10 Personal history

A chronological record with filters:

- Work.
- Football.
- Family.
- Health.
- Places.
- Contracts.
- Public events.

Entries link to their actual evidence.

A debut or marriage is recorded because it happened, not because a milestone system needed to fire.

Do not make history the end-of-game screen.

---

## 10. Footballer pages

### 10.1 Football overview

A working summary of:

- Team membership.
- Current training.
- Latest communicated role.
- Next fixture.
- Recent appearances.
- Active football-related discussions.

Selection information is attributed:

- “Started the previous three matches.”
- “Coach described you as an option.”
- “Squad not yet announced.”

Do not expose the manager’s private selection score.

### 10.2 Training — club programme

Week schedule with:

- Session.
- Group.
- Coach.
- Location.
- Expected load.
- Attendance.
- Linked feedback.

A player can inspect, ask questions, and request permitted adjustments.

Only authorised staff can change the club programme.

Cancelled or changed sessions remain traceable in history.

### 10.3 Training — individual plan

Sections:

- Current focus.
- Approved additional work.
- Position or role familiarisation.
- Recovery.
- Coach feedback.
- Review date.

Show the distinction between:

- Desired plan.
- Submitted request.
- Approved plan.
- Sessions actually completed.

Extra work has time and load consequences. It is not a free development button.

### 10.4 Development reports

A dated report library.

Each report contains:

- Author.
- Observation period.
- Areas assessed.
- Evidence.
- Uncertainty.
- Suggested next steps.

Compare reports over time without pretending every change in assessment is a physical change in the person.

Mentoring and coach assignments link to the relevant people and their availability.

### 10.5 Squad place and role

Show:

- Registered teams.
- Current communicated squad status.
- Recent usage.
- Applicable competitions.
- People competing for similar roles, using known information.
- Existing discussions and promises.

Actions:

- Ask about playing time.
- Discuss position.
- Request feedback.
- Discuss a loan.
- Request a move.

A response can be refusal, reassurance, a changed plan, a conditional promise, or no immediate answer.

### 10.6 Contract

Display the actual document in structured sections:

- Parties.
- Dates.
- Salary and payment frequency.
- Bonuses.
- Clauses.
- Squad-status promises.
- Options and triggers.
- Agent involvement.

Use a compact summary above full terms.

Separate guaranteed money from conditional money. Never mix weekly and annual values without labels.

Actions:

- Discuss renewal.
- Ask representative.
- Review an incoming proposal.
- Review termination provisions.

### 10.7 Negotiation

Three aligned columns:

- Current agreement, if any.
- Other party’s proposal.
- Your draft counterproposal.

Below:

- Negotiation log.
- Expiry date.
- Open conditions.
- Representative advice.
- Required approvals.

Actions:

- Accept.
- Counter.
- Reject.
- Ask for time.
- Delegate within a mandate.
- Withdraw.

Private ceilings and exact acceptance probabilities are absent in inhabited mode.

An agreement is not automatically the same as completed employment: medical, registration, club agreement, and other real dependencies remain separate.

### 10.8 Transfers, loans, and trials

Use a list of cases, each with its own dated record.

A case contains relevant elements without forcing a universal order:

- Contact.
- Club-to-club position, if known.
- Personal terms.
- Trial.
- Medical.
- Registration.
- Travel.
- Relocation.

A loan also shows:

- Parent and borrowing clubs.
- End date.
- Recall terms.
- Known playing-time agreement.
- Review history.

A trial does not imply an offer.

### 10.9 Representation

Show:

- Current representative.
- Agreement and fees.
- Mandate.
- Recent communications.
- Active work.
- Potential conflicts.
- Termination conditions.

Instruction controls cover actual preferences:

- Desired opportunities.
- Places to consider.
- Contract priorities.
- Negotiating limits.
- Matters requiring personal approval.

No universal agent “quality score” unless it is clearly an attributed assessment.

### 10.10 International football

Tabs:

- Eligibility.
- Call-ups.
- National teams.
- Fixtures.
- Record.

Eligibility explains the active world rules and recorded facts.

Call-ups are requests or decisions with dates and conditions, not reward notifications.

Nationality changes and selection eligibility must come from the rules engine, not UI assumptions about real-world regulations.

### 10.11 Leadership duties

Shown only when relevant.

Sections:

- Captaincy or other appointment.
- Meetings.
- Representation requests.
- Team concerns shared with the person.
- Media responsibilities.

The page is a responsibility list, not a leadership skill tree.

---

## 11. Match experience

### 11.1 Match centre

The same fixture page supports upcoming, in-progress presentation, and completed states.

Header:

- Teams and crests.
- Competition and stage.
- Venue.
- Date.
- Match status.
- Score only when appropriate to the current presentation time.

Tabs:

- Overview.
- Teams.
- Events.
- Statistics.
- Analysis.
- Related coverage.

World matches and personally relevant matches use the same underlying page.

### 11.2 Pre-match

Main content:

- Published team sheets.
- Known absences.
- Recent results.
- Competition situation.
- Venue and conditions.
- Relevant rivalry or previous meetings.

Private panel for an involved person:

- Received instructions.
- Their selected role.
- Attendance or travel obligations.
- Existing preparation choices supported by the simulation.

No staged “you have been chosen” reveal animation.

If not selected, the page remains usable. The person’s life does not halt until their next appearance.

### 11.3 Watch match

Layout:

- Top: score, clock, competition, playback controls.
- Centre: 2D pitch or event map.
- Left: team lineups and substitutions.
- Right: commentary and selected-player information.
- Bottom: event timeline.

Display modes:

- Full recorded sequence.
- Extended highlights.
- Key events.
- Text only.
- Final report.

Playback controls:

- Play/pause.
- Speed.
- Previous/next event.
- Jump to half.
- Follow a player.
- Leave presentation.

These control presentation, not the already determined football outcome.

#### Watch-only rule

There are no:

- Pass/shoot choices.
- Timing prompts.
- Hero moments.
- Live player-intent sliders.
- Human-only bonuses for watching.

For the initial manager experience, team plans are submitted before the match. Ordinary in-match AI resolves substitutions and tactical reactions. Watching the replay does not let the user revise them after seeing what happened.

### 11.4 Honest 2D presentation

Two supported recording levels:

**Event-map presentation**

- Uses recorded event times and pitch zones.
- Shows where recorded actions occurred.
- Labels the view “Event map.”
- Does not animate invented off-ball positions.

**Tracked 2D replay**

- Requires recorded ball and player positions from the engine.
- Draws actual recorded trajectories.
- Can interpolate only between real samples for presentation.
- Cannot use interpolation to invent unrecorded tactical behaviour.

The current match interface supports the first approach more directly; the second is a recording dependency.

No fabricated heatmap, passing network, or movement trace may be displayed as actual match data.

### 11.5 Spoiler protection

During playback, all match-linked views respect the revealed match time:

- Score.
- Commentary.
- Statistics.
- Substitutions.
- Results elsewhere.
- Personal history.
- News.
- Notifications.

Opening another page must not reveal the final score accidentally.

Leaving playback offers:

- Keep result concealed and resume later.
- Reveal final result.

This is presentation state, not an alternative simulation timeline.

### 11.6 Post-match report

Sections:

- Result and key events.
- Team statistics.
- Player table.
- Tactical or positional analysis supported by recording.
- Medical and disciplinary consequences when known.
- Published reactions.

A personally involved player can see:

- Their performance.
- Coach feedback.
- Relevant conversations.
- Known next obligations.

No automatic “victory reward” screen.

### 11.7 Match analysis

Available visualisations depend on actual recorded detail:

- Shot map.
- Zone activity.
- Player heatmap.
- Event chronology.
- Passing events.
- Team and player comparisons.

Each visualisation states:

- Coverage.
- Units.
- Orientation.
- Missing-data limitations.

Selecting an event jumps to the corresponding recorded moment.

### 11.8 Fixtures and results

Dense list with:

- Date.
- Competition.
- Home.
- Away.
- Venue.
- Status.
- Result.

Filters:

- Team.
- Competition.
- Date range.
- Watched/followed.
- Available report detail.

Postponed, abandoned, awarded, and replayed fixtures have distinct states. Do not flatten everything into an ordinary result.

---

## 12. Club and football-world pages

### 12.1 Club overview

Sections:

- Identity and current competition.
- Recent results and upcoming fixtures.
- Teams.
- Key personnel.
- Venue.
- Published club news.
- Honours excerpt.

The viewer’s relationship to the club appears near the header:

- Employee.
- Former employee.
- Represented client’s club.
- Assigned scout.
- Unconnected observer.

This relationship affects access, not the club’s underlying state.

### 12.2 Squad

Team selector:

- First team.
- Reserves.
- U21.
- U19.
- U18.
- Other teams supported by the dataset.

Useful presets:

- General.
- Selection.
- Performance.
- Contracts.
- Development.
- Availability.

Typical columns:

- Person.
- Position.
- Age.
- Registration.
- Recent minutes.
- Availability.
- Known contract status.
- Assessed suitability.

Private contract or medical columns require access.

### 12.3 Squad depth

A positional layout with an equivalent table.

Show:

- Eligible people.
- Known or assessed role fit.
- Availability.
- Recent use.
- Source of the ordering.

Distinguish the manager’s private planning view from a footballer’s understanding of competition for places.

A drag operation changes a manager’s draft only when authorised. It does not let any viewer reorganise the team.

### 12.4 Staff and responsibilities

Directory grouped by function:

- Management.
- Coaching.
- Recruitment.
- Medical.
- Administration.
- Board.

Show:

- Role.
- Person.
- Department.
- Reporting line.
- Delegated duties.
- Contactability.

Responsibility detail explains who can approve what.

A role can be vacant. Do not generate a named staff member merely to fill the interface.

### 12.5 Dressing room

Primary presentation:

- Known concerns.
- Leadership appointments.
- Recent relevant interactions.
- Shared commitments.
- Groups supported by actual relationship records.

The default is a readable list, not a tangled social graph.

In inhabited mode, distinguish:

- What someone told you.
- What you observed.
- What is rumoured.
- What is unknown.

Exact faction scores and private relationships belong only in observer inspection.

### 12.6 Club facilities

Tabs:

- Training.
- Youth.
- Medical.
- Other supported facilities.

Each facility shows:

- Location.
- Capacity or relevant capability.
- Condition or assessed quality.
- Access.
- Planned works.
- Operational restrictions.

Projects show real budget, approval, and construction states where available.

A player may request improvement; they cannot approve construction.

### 12.7 Club finances

Two access levels:

**Public**

- Published accounts.
- Reported financial condition.
- Public ownership.
- Announced major deals.

**Authorised internal**

- Balance.
- Budgets.
- Wage commitments.
- Transfer instalments.
- Debt.
- Forecast assumptions.
- Ledger.

Budget, cash, profit, and committed future expenditure are separate concepts.

### 12.8 Board and ownership

Show:

- Ownership structure.
- Board members.
- Assigned responsibilities.
- Published strategy.
- Internal mandates where accessible.
- Decisions and meeting records.

A director’s personal view shows their own responsibilities, not automatic authority over the entire board.

Manager evaluation uses documented expectations and communication, not a single omniscient sack-risk percentage.

### 12.9 Club history

Tabs:

- Seasons.
- Honours.
- Managers.
- Transfers.
- Records.
- Venues.
- Notable people.

Historical names and affiliations reflect the relevant date.

A dissolved or renamed club retains an archive.

### 12.10 Competition overview

Header:

- Competition.
- Nation or confederation.
- Season.
- Current stage.

Main sections:

- Standings or stage.
- Recent results.
- Upcoming fixtures.
- Leading statistics.
- Current news.

The competition’s actual structure determines the page. Do not force cups and split-season leagues into one league-table template.

### 12.11 Tables and stages

League table:

- Position.
- Team.
- Played.
- Won/drawn/lost.
- Goals.
- Goal difference.
- Points.
- Form.

Qualification boundaries have textual explanations.

Knockout view:

- Ties.
- Legs.
- Aggregate.
- Winner.
- Relevant tie-breaking rules.

Always provide a list alternative to a wide bracket.

Points deductions, incomplete fixtures, and tie-breakers are explained in context.

### 12.12 Competition statistics

Separate player and team tables.

Filters:

- Season.
- Stage.
- Minimum appearances/minutes.
- Team.
- Position.

Keep ranks tied to the actual eligibility criteria.

Awards are not inferred from whichever table happens to be sorted first.

### 12.13 Competition rules

A readable rules document containing:

- Eligibility.
- Registration.
- Squad limits.
- Substitutions.
- Discipline.
- Tie-breakers.
- Promotion/relegation.
- Qualification.
- Financial or licensing rules if implemented.

Show the active rules version and effective season.

This is the world’s ruleset, not a claim that the game reproduces every current real-world regulation.

### 12.14 Nation and federation

Tabs:

- Overview.
- Domestic football.
- National teams.
- People.
- Calendar.
- Rules.
- History.

Federation responsibilities and private records require appropriate authority or observer mode.

National-team pages reuse team and competition components.

### 12.15 World overview

The observer landing page and public world-browsing entry point.

Main sections:

- Date and active seasons.
- Selected competitions.
- Recent major events.
- Upcoming notable fixtures.
- Transfers.
- Appointments.
- World search.

No world “health score” dominates the page.

Advanced simulation metrics belong in an optional technical inspector, not ordinary football browsing.

### 12.16 World directories

Separate searchable directories for:

- People.
- Clubs.
- Competitions.
- Nations.
- Cities.
- Venues.
- Agencies.
- Media outlets.
- Sponsors and commercial organisations.

Reuse a consistent directory component with entity-specific filters.

---

## 13. Stadiums, places, organisations, and markets

### 13.1 Stadium profile

Header:

- Name.
- City.
- Main users.
- Current operating status.

Tabs:

- Overview.
- Fixtures.
- Facilities.
- Projects.
- History.

Fields where supported:

- Capacity.
- Ownership.
- Surface.
- Dimensions.
- Shared use.
- Attendance.
- Restrictions.

A shared stadium is one venue referenced by multiple clubs, not duplicated independently under each club.

Missing imported details remain missing.

### 13.2 City and region

Show:

- Location.
- Clubs.
- Venues.
- Relevant organisations.
- Languages and travel context supported by world data.
- Available housing and education links when implemented.

Use a simple map only if coordinates are reliable. A directory is preferable to a misleading decorative map.

Travel estimates are labelled estimates.

### 13.3 Public transfer centre

Columns:

- Person.
- From.
- To.
- Type.
- Effective date.
- Published fee or “undisclosed.”
- Confirmation state.

Separate tabs:

- Confirmed.
- Reported interest.
- Rumours.

Rumours retain publication source and date. They do not silently turn into factual transfer records.

### 13.4 Employment market

Job listings and recorded openings, filtered by role.

Show:

- Organisation.
- Role.
- Location.
- Published requirements.
- Application deadline.
- Contact route.

Private approaches appear in the person’s opportunities page, not automatically in the public market.

A vacancy does not guarantee that an application is accepted or even answered.

### 13.5 Agency profile

Show:

- Organisation.
- Agents.
- Publicly known clients.
- Areas of operation.
- Public history.
- Relevant contact routes.

Private mandates, fees, and negotiating positions are restricted.

Do not present agency reputation as an objective percentage when only hearsay is available.

### 13.6 Media outlet profile

Show:

- Outlet identity.
- Coverage area.
- Journalists.
- Published articles.
- Interviews.
- Historical coverage.

Editorial bias or accuracy can be an observed assessment. Raw internal values remain observer-only.

### 13.7 Sponsor and commercial organisation profile

Show:

- Organisation.
- Public partnerships.
- Commercial representatives.
- Relevant campaigns.
- Published announcements.

Private budgets and negotiating priorities are not public.

A sponsor is an organisation; the user inhabits one of its people, not the entire corporate entity.

---

## 14. Work pages for non-player roles

These pages share the same shell, messages, calendar, personal life, money, and history. They are not separate games with separate protagonists.

### 14.1 Manager — work overview

Sections:

- Upcoming fixtures.
- Squad availability.
- Decisions within the manager’s authority.
- Training programme.
- Recruitment requests.
- Staff reports.
- Board communication.

The page links to actual pending work rather than generating a daily checklist.

### 14.2 Manager — selection and tactics

Split layout:

- Squad table on the left.
- Pitch and formation centrally.
- Selected player/role detail on the right.

Tabs:

- Selection.
- Team instructions.
- Individual assignments.
- Set pieces when supported.
- Match plan.

Show eligibility and registration problems before submission.

States:

- Draft.
- Submitted.
- Superseded.
- Locked for the fixture.

No guaranteed outcome estimates.

For watch-only matches, the submitted plan and ordinary in-match decision system govern play.

### 14.3 Manager — squad planning

Views:

- Current needs.
- Contract exposure.
- Age distribution.
- Positional coverage.
- Youth options.
- Loaned players.

A plan expresses recruitment needs and preferences.

The manager can request action from directors or scouts where direct authority is absent.

Do not let this become a second hidden omniscient player database.

### 14.4 Coaches — sessions and assessments

Work overview:

- Assigned groups.
- Sessions.
- Individual plans.
- Report deadlines.
- Available facilities.

Session editor:

- Participants.
- Focus.
- Load.
- Facility.
- Conflicts.
- Review.

Assessment page:

- Observations.
- Evidence period.
- Professional judgement.
- Recipients.

Submitting a report creates a dated report record. It does not magically improve the underlying player.

### 14.5 Assistant manager

Use coach and manager pages according to delegation.

Additional focus:

- Opposition preparation.
- Selection recommendations.
- Staff coordination.
- Delegated meetings.

Recommendations are distinct from final decisions.

### 14.6 Scout — assignments

Table:

- Assignment.
- Region or competition.
- Requested profile.
- Travel schedule.
- Budget.
- Deadline.
- Current findings.

Assignment detail:

- Brief.
- Matches planned.
- People observed.
- Reports submitted.
- Remaining work.

Knowledge requires observation or legitimate sources. Opening a profile does not itself increase scouting knowledge.

### 14.7 Scout — report desk

List of draft and submitted reports.

A report contains:

- Subject.
- Observation dates.
- Assessment.
- Strengths and concerns.
- Suitability to the brief.
- Confidence.
- Recommendation.

Reports can become outdated.

A scout can recommend; signing authority belongs elsewhere.

### 14.8 Analyst

Pages:

- Opposition.
- Match analysis.
- Team patterns.
- Report library.

Every conclusion links to recorded data.

Distinguish measured statistics from interpretation.

Do not fabricate exact expected outcomes for tactical changes.

### 14.9 Physio and medical staff

Work overview:

- Assigned cases.
- Appointments.
- Rehabilitation plans.
- Reviews due.
- Clearance requests.

Case page:

- Symptoms and findings.
- Diagnoses with uncertainty.
- Treatment history.
- Agreed plan.
- Restrictions.
- Review date.
- Relevant consent.

Confidential access is checked in the view layer.

A manager does not automatically see a player’s entire private medical history.

### 14.10 Sports science and fitness staff

Views:

- Workload.
- Recovery.
- Fitness sessions.
- Recommendations.
- Reports.

The person can recommend changes or apply them within their mandate.

Risk is represented as a modelled estimate, not a guarantee of injury or safety.

### 14.11 Head of youth development

Pages:

- Academy overview.
- Intake and trials.
- Development reviews.
- Education coordination.
- Retention decisions.
- Staff assignments.

No list labelled “future stars” based on hidden potential.

Youth assessment carries uncertainty, and release or promotion uses ordinary club decision-making.

### 14.12 Director of football

Pages:

- Recruitment strategy.
- Candidate reports.
- Negotiations.
- Contract planning.
- Staff recruitment.
- Budget authority.

The person can operate only within their delegated remit.

Club interests, player interests, manager preferences, and board constraints remain distinct.

### 14.13 Board member or chair

Work overview:

- Meetings.
- Decisions.
- Financial responsibilities.
- Executive appointments.
- Projects.
- Ownership matters.

Decision detail shows:

- Proposal.
- Supporting reports.
- Participants.
- Required approvals.
- Recorded vote or decision.
- Implementation status.

The user supplies one person’s decision. Other directors are not automatically obedient.

### 14.14 Agent — client book

Table:

- Client.
- Current employment.
- Contract expiry.
- Mandate.
- Active opportunities.
- Contact due.
- Pending approval.

Client detail:

- Instructions.
- Representation agreement.
- Negotiations.
- Communications.
- Commercial matters.

Clients can disagree, reject advice, change priorities, or end representation.

### 14.15 Agent — negotiations and outreach

Separate views for:

- Incoming approaches.
- Authorised outreach.
- Active negotiations.
- Client approvals.
- Completed agreements.

The UI shows who the agent is representing in every negotiation to prevent accidental cross-client actions.

No automatic access to all clubs’ private budgets.

### 14.16 Journalist — assignments

Table:

- Story subject.
- Outlet.
- Deadline.
- Known sources.
- Publication state.
- Outstanding interview requests.

Opening an assignment shows:

- Known facts.
- Source statements.
- Unverified claims.
- Related events.
- Editorial requirements.

The person’s knowledge is not omniscient just because the UI is about reporting.

### 14.17 Journalist — story and interview workspace

Use a structured article workspace:

- Headline.
- Subject.
- Claims.
- Attributed quotations.
- Supporting sources.
- Publication destination.

Actions:

- Request comment.
- Conduct a supported interview.
- Submit.
- Revise.
- Publish when authorised.
- Correct a published story.

Structured choices and stored text are sufficient. The product must not require an LLM to function.

Freeform personal draft text may be stored, but gameplay consequences must be attached to explicit supported claims and actions, not pretend comprehension of arbitrary prose.

### 14.18 Commercial representative

Pages:

- Partnerships.
- Proposed deals.
- Contractual obligations.
- Appearances and campaigns.
- Payments.
- Renewals.

The representative has a mandate and budget, not control of every person associated with the sponsor.

Campaign success is derived from actual world systems, not a decorative marketing score.

### 14.19 Unemployed, retired, and non-football people

Keep:

- Today.
- Messages.
- Calendar.
- People.
- Life.
- Money.
- Opportunities.
- History.

Work pages appear only for supported responsibilities.

Do not manufacture a football job to make the interface look occupied.

---

## 15. Life, household, education, and health

### 15.1 Life overview

Sections:

- Household.
- Current home.
- Upcoming personal commitments.
- Education.
- Health.
- Important recent changes.

Use practical records and current circumstances. Avoid turning family, sleep, and happiness into three competing progress bars.

### 15.2 Household

Table:

- Person.
- Relationship.
- Residence.
- Relevant shared responsibility.
- Known schedule.

Detail includes:

- Shared arrangements.
- Invitations.
- Upcoming commitments.
- Financial responsibilities.
- Relocation discussions.

Household members have their own decisions.

The user cannot unilaterally move an adult partner to another country by selecting a checkbox.

### 15.3 Home and housing

Current-home section:

- Location.
- Occupancy.
- Rent or ownership.
- Recurring costs.
- Commute.
- Agreement dates.

Available-housing list, when supported:

- Location.
- Cost.
- Size/category.
- Availability.
- Relevant travel implications.

No furnished-room minigame is required.

A home is meaningful through location, cost, household arrangements, and daily life.

### 15.4 Relocation

A case page bringing together:

- Proposed destination.
- Employment requirement.
- Travel.
- Housing.
- Household responses.
- School or study changes.
- Language support.
- Club assistance.
- Outstanding decisions.

It is not a mandatory sequence of stages. Some matters can proceed in parallel or remain unresolved.

The move date reflects actual agreements and readiness.

### 15.5 Education

Tabs:

- Current study.
- Timetable.
- Assessments.
- Qualifications.
- Available courses.

Course details:

- Provider.
- Requirements.
- Cost.
- Schedule.
- Workload.
- Qualification awarded.
- Enrolment status.

Education is not an unlock tree guaranteeing a job.

Coaching qualifications reuse this system.

### 15.6 Languages

Show:

- Known languages.
- Assessed proficiency.
- Current study.
- Practical communication difficulties where simulated.

Do not use a language percentage as an exact measure of every conversation.

A language course occupies time and has a provider, schedule, and outcome.

### 15.7 Activities and support

List actual available activities, contacts, or services.

Examples:

- Meet a friend.
- Attend a community event.
- Arrange a hobby activity.
- Request professional support.
- Visit family.

Each action respects availability, location, cost, and consent.

No endlessly repeatable “gain happiness” button.

### 15.8 Health overview

Sections:

- Current concerns.
- Medical appointments.
- Active rehabilitation.
- Restrictions.
- Recovery advice.
- Relevant history.

A body diagram is a navigation aid, with a complete text equivalent.

Separate:

- What the person feels.
- What a professional has diagnosed.
- What remains uncertain.

### 15.9 Injury or illness case

Show:

- Onset.
- Diagnosis.
- Clinician.
- Expected recovery range.
- Treatment.
- Reviews.
- Current restrictions.
- Return-to-work or return-to-play status.

Actions depend on actual options:

- Request consultation.
- Seek another opinion.
- Discuss a plan.
- Report symptoms.

A player cannot instantly clear themselves medically.

### 15.10 Well-being

Use a restrained private page:

- Self-reported concerns.
- Recent circumstances.
- Existing support.
- Appointments.
- Agreed adjustments.

Do not turn bereavement, anxiety, or family illness into optimisation currency.

Content settings can soften presentation while preserving a comprehensible account of what happened.

### 15.11 Retirement and later life

A planning page, not an ending.

Sections:

- Current work.
- Financial circumstances.
- Qualifications.
- Family considerations.
- Possible employment.
- Existing discussions.
- Proposed retirement date, if any.

Actions:

- Discuss.
- Apply for another role.
- Reduce commitments where permitted.
- Announce retirement.
- Continue without change.

No forced epilogue closes the world.

If the inhabited person dies, present the event with restraint, preserve their history, return control to observer mode, and allow another person to be inhabited. Death is not a world reset.

---

## 16. Money and commercial life

### 16.1 Personal finances

Main content:

- Current accessible balances.
- Expected incoming payments.
- Upcoming obligations.
- Recent transactions.
- Monthly statement.

Separate:

- Cash.
- Assets.
- Debt.
- Income.
- Estimated future income.

Do not put an enormous net-worth counter at the centre of every personal page.

### 16.2 Transaction ledger

Columns:

- Date.
- Description.
- Counterparty.
- Category.
- Amount.
- Currency.
- Account.
- Status.

Filters:

- Date range.
- Category.
- Counterparty.
- Pending/settled.

Opening a payment links to its contract or obligation.

Corrections are recorded adjustments, not invisible edits to history.

### 16.3 Budget and recurring commitments

Show:

- Housing.
- Family support.
- Education.
- Representation.
- Travel.
- Personal services.
- Other supported expenses.

Forecast assumptions are visible.

Changing a budget preference does not cancel a legally or contractually binding payment.

### 16.4 Assets and liabilities

Separate tables for:

- Savings.
- Property.
- Investments.
- Businesses.
- Loans.
- Mortgages.

Only implemented asset systems appear.

No decorative trading terminal and no invented real-time market feed.

### 16.5 Sponsorships

Contract list:

- Sponsor.
- Term.
- Guaranteed payment.
- Conditional payment.
- Exclusivity.
- Appearances.
- Outstanding obligations.

Deal page reuses the negotiation and contract components.

Conflicts with club agreements or other deals are explained before submission.

### 16.6 Advisors

Show:

- Current advisor.
- Agreement.
- Authority granted.
- Recommendations.
- Fees.
- Recent activity.

Advice is attributed and may be imperfect. An advisor is not an omniscient optimisation assistant.

---

## 17. Media, public reputation, and history

### 17.1 News

A newspaper-like list and reading pane.

Filters:

- Following.
- Local.
- Club.
- Competition.
- Transfers.
- International.
- Wider world.

Each article has:

- Headline.
- Outlet.
- Author.
- Publication date.
- Subject links.
- Article text.

Do not make every article about the inhabited person.

### 17.2 Article detail

Use comfortable reading width.

Side panel:

- Mentioned people.
- Clubs.
- Match or event.
- Related coverage.
- Corrections or follow-ups.

A news story is a claim made by an outlet, not automatically objective simulation truth.

Omniscient mode may offer a separate “Underlying event” link.

### 17.3 Interviews and press appearances

List:

- Invitations.
- Scheduled appearances.
- Completed interviews.
- Published coverage.

An interview uses the shared conversation screen with:

- Interviewer.
- Outlet.
- Context.
- Record status.
- Question.
- Supported responses.

The resulting quotation and publication retain their provenance.

### 17.4 Public statements and social posts

A compact chronological page, not an infinite engagement feed.

Composer:

- Supported topic.
- Intended audience.
- Wording or structured statement.
- Related event.
- Sponsor disclosure when required.

Before publication, show the exact statement being submitted.

Deleting a post does not erase that it existed or what others already observed.

### 17.5 Public standing

Present different audiences separately:

- Supporters of a club.
- Football professionals.
- Media.
- Wider public.

Use:

- Recent coverage.
- Polls or reports where simulated.
- Known reactions.
- Attributed assessments.
- Historical changes.

Avoid a universal “fame level.”

### 17.6 Awards and records

Tabs:

- Current awards.
- Historical winners.
- Records.
- Nominations.
- Voting details where published.

An award links to its actual competition, season, criteria, and recorded result.

No hidden achievement checklist controls events.

### 17.7 World history

A browsable archive by:

- Date.
- Season.
- Nation.
- Competition.
- Organisation.
- Person.
- Event type.

Preserve important links through renaming, retirement, and dissolution.

If detailed match data was compacted, show surviving summaries and explain the missing detail.

### 17.8 Personal retrospective

Available at any time.

Sections:

- Professional record.
- Important relationships.
- Places lived.
- Significant events.
- Honours.
- Selected matches.
- User-pinned memories.

The user chooses what to revisit. The page does not assign a final life score.

---

## 18. Search, observer tools, settings, and help

### 18.1 Global search

Entity search, not an AI chat box.

Results grouped by:

- People.
- Clubs.
- Competitions.
- Places.
- Organisations.
- Matches.
- Articles.

Keyboard access:

- Type.
- Move through results.
- Inspect.
- Open.

Results obey the active perspective, including snippets and counts.

Searching a hidden fact must not reveal it indirectly.

### 18.2 Advanced search

Entity-specific filters with:

- Clear labels.
- Explicit unknown-value handling.
- Saveable views.
- A results table.
- Export.

A footballer searching for another person does not receive a scout’s hidden professional assessment merely because a filter exists.

### 18.3 Observer world inspector

Separate from ordinary profile pages.

Sections:

- Entity state.
- Decisions.
- Relationships.
- Contracts.
- Events.
- Simulation metrics.

Raw values are clearly labelled internal state.

It is read-only and does not replace the ordinary game UI.

Do not expose this inspector while inhabiting someone through a forgotten shortcut.

### 18.4 Perspective management

Show:

- Current perspective.
- Recent inhabited people.
- Eligible people to inhabit.
- Pending handoff state.

On switching, summarise only necessary consequences:

- Unsubmitted drafts.
- Pending decisions.
- AI resumption.
- Information-scope change.

There is no “main character” slot that permanently owns the world.

### 18.5 Save and world management

Actions:

- Save.
- Save a copy.
- Load.
- Backup history.
- World information.
- Return to menu.

Show current save operation and last successful save.

UI layout preferences are separate from world state.

### 18.6 Settings

Sections:

- Display and scaling.
- Theme and density.
- Accessibility.
- Audio.
- Keyboard.
- Date, number, and currency presentation.
- Advance stop preferences.
- Match playback.
- Recording and archive retention.
- Save behaviour.
- Content presentation.

Settings that alter simulation rules are world configuration, not casual display preferences.

No account, subscription, cloud sync, or telemetry requirement.

### 18.7 Help and glossary

Contextual reference containing:

- Screen explanations.
- Football terms.
- Statistical definitions.
- Contract terminology.
- Information visibility.
- Time and defaults.
- Observer versus inhabited mode.

Help explains how a system works without revealing hidden values for the current case.

### 18.8 Diagnostics and errors

A restrained technical page:

- Application version.
- World compatibility.
- Recent errors.
- Performance timings.
- Data validation issues.
- Export diagnostic report.

Diagnostic exports require explicit user action and should exclude unnecessary personal paths and private data.

No automatic external reporting.

---

## 19. Backend and interface changes required by the UI

### 19.1 Perspective-aware queries

Introduce a shared view context:

- World identity.
- World revision.
- Inhabited person or observer.
- Current permissions.
- Knowledge scope.
- Historical or replay cutoff when applicable.

Every query is evaluated against this context.

Filtering must happen before data reaches the UI—not by sending hidden state and concealing a widget.

### 19.2 Typed navigation

Use stable entity identities for routes:

- Person.
- Club.
- Team.
- Competition and season.
- Fixture.
- Venue.
- Organisation.
- Conversation.
- Contract.
- Decision.

Names are labels, not keys.

Renaming an entity must not break navigation.

### 19.3 Query result conventions

Views return:

- Data.
- Source or provenance where relevant.
- Date/revision.
- Knowledge or access state.
- Recording limitations.
- Available actions.

Do not create a separate access-control implementation for every screen.

### 19.4 Commands

Commands identify:

- Acting person.
- Relevant role or mandate.
- Target.
- Proposed action.
- Expected state revision where needed.
- Unique submission identity.

The simulation validates authority and current state.

Repeated clicks must not create duplicate transfers, payments, messages, or contracts.

### 19.5 Decision timing

Use shared deterministic resolution points.

Human answers can be collected early, but they must not gain timing advantages unavailable to the ordinary AI route.

The UI may pause before a resolution point. It must not require simulation systems to branch on the inhabited person beyond the permitted decision-routing behaviour.

### 19.6 World-owned personal systems

Family, housing, money, routines, media work, and non-player employment must live in the shared world model when implemented.

Do not put the human’s rich life into a special client-owned object while giving everyone else unrelated placeholder outcomes.

Different storage detail is acceptable only where it does not change behaviour.

### 19.7 Background execution

- Simulation runs outside the rendering thread.
- Large searches and report generation run outside the rendering thread.
- The UI consumes bounded snapshots and updates.
- Cancelled searches cannot overwrite newer results.
- Saving occurs at a consistent world boundary.
- Rendering never consumes simulation randomness.

### 19.8 Match presentation

Separate:

- Match simulation.
- Recorded result.
- Playback position.
- Revealed information.

The match outcome must be identical whether watched, skipped, displayed as text, or presented in 2D.

### 19.9 Capability-based page availability

A page is enabled only when its underlying system provides meaningful data and actions.

During development:

- Do not ship fake sponsors, family members, or journalists to populate layouts.
- Do not show inert “coming soon” buttons throughout normal play.
- Keep unfinished-system visibility in a development capability report.
- Show genuine empty-world states differently from unavailable implementation.

---

## 20. Performance and accessibility requirements

### Performance targets

These are acceptance targets to measure, not claims about current performance.

| Operation | Target |
|---|---|
| Cached page navigation | Visible response within 100 ms |
| Basic input feedback | Within one rendered frame |
| Normal interactive rendering | 60 fps on the agreed baseline machine |
| Large search | Immediate acknowledgement; incremental results without freezing |
| Long advance | Progress updates and safe cancellation |
| Static paused page | No unnecessary continuous high-frequency repaint |
| Large tables | Render visible rows, not the entire world |
| Portraits and crests | Bounded, size-aware cache |
| Long saves | UI remains responsive; original valid save preserved |

Use a baseline Windows machine with an integrated GPU and 16 GB RAM for repeatable testing.

Measure the client separately from simulation memory. Do not duplicate a 300,000-person world into page state.

### Accessibility requirements

- Full keyboard operation.
- Visible focus.
- Screen-reader labels for custom controls.
- Keyboard navigation in virtualised tables.
- Scalable text and controls.
- Information never conveyed by colour alone.
- Text alternatives for pitch maps, brackets, charts, and relationship views.
- Reduced motion.
- No flashing event effects.
- Configurable audio with visual equivalents.
- No time-pressure penalty caused by slow reading while paused.
- Working text input for supported scripts and input methods.

Accessibility must be tested on the real native build, not assumed from framework support.

---

## 21. Delivery order

This is an implementation sequence, not a career progression imposed on the player.

### Phase 1 — native foundation

Deliver:

- Window and shell.
- Design tokens.
- Tables.
- Navigation.
- Search.
- Entity headers.
- Inspectors.
- Settings.
- Save/load presentation.
- Perspective indicator.
- Read-only world browsing.

Acceptance: an existing world can be explored comfortably without a frontend framework rewrite later.

### Phase 2 — inhabiting an ordinary footballer

Deliver:

- Perspective switching.
- Today.
- Messages and decisions.
- Calendar.
- Player profiles.
- Training views.
- Contracts.
- Opportunities.
- Agent communication.
- Club and competition pages.

Prerequisite: shared decision and knowledge boundaries are working.

Acceptance: a person can be inhabited and relinquished without special-world behaviour.

### Phase 3 — watch-only match experience

Deliver:

- Match centre.
- Pre-match views.
- Event-map playback.
- Commentary.
- Statistics.
- Spoiler protection.
- Post-match reports.

Tracked 2D motion follows only when authentic telemetry exists.

Acceptance: watching, skipping, and not opening the match produce the same world outcome.

### Phase 4 — world-owned personal life

Deliver:

- Household.
- Routines.
- Housing.
- Relocation.
- Education.
- Health.
- Personal finances.
- Retirement continuity.

Prerequisite: these systems operate for AI people too.

Acceptance: switching away does not suspend anyone’s life.

### Phase 5 — football professions

Deliver:

- Manager.
- Coaches and assistant.
- Scouts.
- Analysts.
- Medical and sports-science staff.
- Head of youth.
- Director of football.
- Board responsibilities.

Each profession becomes inhabitable only when its decision route and authority model exist.

### Phase 6 — broader world professions

Deliver:

- Agents.
- Journalists.
- Commercial representatives.
- Media organisations.
- Sponsorship work.
- Additional ordinary-person roles supported by the simulation.

Acceptance: these people have independent work and decisions, not jobs that exist only to serve the user.

### Phase 7 — long-world polish

Deliver:

- Historical browsing.
- Archive compaction presentation.
- Large-world performance.
- Long-save reliability.
- Accessibility completion.
- Localisation readiness.
- Mature data validation and diagnostics.

No phase should disguise missing systems with scripted personal events.

---

## 22. Verification and acceptance scenarios

### Product-rule review

Every feature records answers to the four supplied questions:

1. Does the underlying system still work without a human?
2. Can an AI-controlled person experience the same circumstances?
3. Is the situation caused by persistent world state?
4. Can circumstances and decisions lead to different plausible outcomes?

For purely presentational features, document that they display state without affecting outcomes.

### Essential gameplay tests

1. **No human:** run an autonomous world without loading the desktop client.
2. **Swap parity:** answer with the same defaults at the same resolution points; compare world hashes with the AI-controlled run.
3. **Step out:** relinquish a person with pending work; the ordinary AI continues it.
4. **No reply:** allow an offer to expire; its default resolves once.
5. **Multiple offers:** competing negotiations coexist without a forced sequence.
6. **Unemployment:** Today, life, money, contacts, and opportunities remain functional.
7. **Role change:** player becomes staff without changing person identity.
8. **Multiple roles:** player-coach permissions remain separate and correct.
9. **Retirement:** professional role ends; the person and world continue.
10. **No observers:** no system depends on a page being opened.

### Information-boundary tests

11. A player cannot obtain a club’s confidential budget through sorting or export.
12. A scout sees assessments, not hidden potential.
13. An agent sees only authorised client information.
14. A journalist’s unverified claim remains attributed.
15. Observer mode reveals internal state but cannot mutate it.
16. Returning from observer mode rechecks every open view.
17. An old bookmark cannot restore revoked access.
18. Unknown values remain unknown in comparisons and statistics.
19. During match playback, other pages do not reveal unrevealed results.

### UI workflow tests

20. Navigate from league table → club → player → contract → back, preserving state.
21. Complete a negotiation using only the keyboard.
22. Resize the window while a table, inbox, or pitch is open.
23. Use comfortable text scaling at the minimum supported window size.
24. Submit twice rapidly; only one action is applied.
25. Receive an updated offer while editing a counterproposal; stale submission is explained.
26. Change search filters rapidly; old results cannot replace the current search.
27. Cancel a long advance safely.
28. Recover from a failed save without losing the previous valid file.
29. Open a historical person whose club no longer exists.
30. Read a match whose detailed recording has been compacted.

### Performance and longevity tests

31. Browse a large world without instantiating one UI object per person.
32. Run multiple seasons while periodically opening and closing profiles.
33. Verify bounded image, query, and report caches.
34. Validate a 20-season autonomy run across several seeds.
35. Confirm rendering and recording changes do not alter simulation outcomes.
36. Test keyboard and screen-reader operation on the packaged Windows build.

---

## 23. Final design standard

The finished interface should make these situations feel natural:

- You spend an evening examining a competition without changing anything.
- You receive no offers for months.
- Someone refuses to speak to you.
- Your manager leaves and your circumstances change.
- Your partner has commitments unrelated to your career.
- A scout’s assessment proves wrong.
- A journalist publishes something inaccurate.
- A promising negotiation collapses.
- You retire and continue living.
- You leave a person, return years later, and find that their life continued.

The UI’s job is to show those circumstances clearly and provide the actions the inhabited person actually has.

**A detailed football world, presented through practical screens and believable information—not a dashboard, not a scripted career, and not a collection of decorative features.**
