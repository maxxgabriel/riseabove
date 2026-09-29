# Rise Above desktop UI redesign: execution plan

Status: shared system and Living Football World implementation complete. TypeScript/Vite build, targeted view tests, and browser screenshot validation passed locally on 2026-09-29.

Verified base: `origin/integration/pathway-data-import` at `ae6249d` (2026-09-29). This branch fast-forwarded from `origin/ui/desktop-client` at `1ce1531`, which is an ancestor. Work branch: `codex/ui-ux-redesign` in `E:\pers\riseabove-ui-desktop`.

## Outcome and boundaries

Make the client feel like a dense football operations desktop product. The corrected references establish a blue-black workspace, blue utility bar, violet structural accents, stacked information panels, and compact side context. Keep Rise Above's own typography, crest treatment, and player perspective. Do not copy reference assets, wording, or coordinates.

The initial redesign was a presentation change. The Living Football World phase adds read-only News, pulse, and insight evidence views over recorded data. Preserve existing actions, saved data, concealed results, and simulation behavior. Never infer hidden traits, relationships, or unrecorded match statistics from visible data.

## Living Football World implementation

- Today is a three-zone portal linking inbox, decisions, ranked world stories, visible events, recent results, next match, and personal condition. It collapses into one reading order on narrow screens.
- World News provides For You, Following, and World views, plus story detail. Ranking uses recorded newsworthiness, recency, club relevance, and deterministic ties; selection limits repeated subjects and story kinds. Story graphics use crests, typography, and recorded scores.
- Messages prioritizes action and unread threads, shows deadlines when present, and gives conversations a dedicated reading area with context. Existing reply actions remain intact.
- Social is a continuous feed with author identity and thread pages. Existing post, reply, and quote actions remain intact.
- Insights are compact evidence rows. Optional visuals use visible match ratings, recent club results, and current standings. Hidden match data is filtered before the views return it.
- Frontend build passed. Targeted `pw-view` checks passed for newsroom methods, concealed press/crowd output, and hidden-result insights. Browser captures cover Today, News, Messages, Social, Player, Club, Competition, and Match at 1366×768, 1440×900, and 1920×1080; a narrow Today view and 20px text scaling were also checked.

## Current architecture and audit

| Area | Current implementation | Redesign implication |
| --- | --- | --- |
| Runtime | React 19 and Vite frontend in `app/`; Tauri 2 host in `app/src-tauri/`; Rust simulation/API behind `app/src/api.ts` and `app/src/store.ts` | Keep backend and command contracts unchanged. |
| Routing | Hash route parser in `app/src/router.ts`, page switch in `app/src/App.tsx` | Keep URLs and page actions stable. |
| Shell | `components/Shell.tsx` with grouped rail, top bar, status line, Continue, and `SearchPalette` | Refine hierarchy and density in place. Ctrl+K already searches entities and pages; extend actions only where existing APIs support them. |
| Entity frame | `components/Stage.tsx` has `Stage`, `StageHeader`, `StageTabs`, fixture strip, and four-column competition primitives | Keep the useful shared frame. Limit contextual color to the hero and small accents; global surfaces stay neutral. |
| Styling | `styles/tokens.css`, `base.css`, `ui.css`, `table.css`, `shell.css`, `pages.css`, `stage.css` | Tokens exist for colors and a few sizes; add semantic surface, spacing, type, radius, row, motion, and layer tokens. Consolidate later dark-theme overrides in `stage.css` instead of stacking another theme. |
| Components | `ui/ui.tsx` provides `Section`, `Badge`, `Meter`, `KeyVal`, states, controls; `components/DataTable.tsx` is virtualized and already supports sorting/selection; `components/Insights.tsx` uses recorded-state notes | Extend these before adding new primitives. Separate flat information bands, side widgets, and interactive cards so `.card` is not the default for every section. |
| Type and icons | Inter for UI, Barlow Condensed for display; custom SVG icon family in `ui/Icon.tsx`; programmatic crests in `components/Crest.tsx` | Retain and normalize scale, baseline, and crest bounds. No new component or icon package needed. |
| Visual capture | `app/e2e/shots.mjs` and `lib.mjs` use `playwright-core`, a local API at `127.0.0.1:8787`, and a configured Chromium binary | Capture the five pages before the first visual change when that environment is available. The helper defaults to a Linux Chromium path, so set `CHROMIUM` on Windows and fix its output path if needed. |

### Concrete problems visible in the current code

- The former root tint and `stage.css` dark overrides spread club or cup color through the canvas and shell. The redesign confines that tint to the entity frame.
- `.card` is used repeatedly around lists and `KeyVal` blocks on Today, Person, and Club; many panels share the same outlined treatment.
- `.page` has a 1500px cap while `Stage` screens use different spacing; wide-monitor behavior is inconsistent.
- `CompOverview` always uses a four-column `FmPanel`, so a cup bracket and league table get similar weight even though their priorities differ.
- A played match with no `detail` currently renders the score, a note, and talking points, then leaves the rest of the page empty.
- Today renders `mind` as signed simulation values. The player-facing view needs semantic wording based only on visible text/evidence.
- Existing `DataTable` already has valuable keyboard, sorting, and virtualization behavior. Restyle it rather than replace it.

## Design system decisions

1. **Surfaces:** midnight-blue application canvas, near-black navigation, a blue utility bar, and violet structural accents. Use three subtle content elevations; contextual club/competition color belongs in the first hero region, crests, and small state marks. Borders separate dense data modules.
2. **Typography:** Barlow Condensed for entity names, scores, and section identity; Inter for controls, facts, and tables. Standardize page/section/label sizes and tabular numerals. Avoid reducing body readability to achieve density.
3. **Density:** one compact rhythm for shell controls, rows, badges, and tables. The default information row should target roughly 28–34px. Preserve text scaling from Settings.
4. **Shape:** 2–4px control corners, 4–6px panels, round only semantic pills and avatars. Use violet for navigation and selection, mint for positive state and performance, and orange for the controlled player/current context.
5. **State:** success, warning, danger, info, and neutral each have both text/icon and color. Conspicuous orange stays reserved for “you” where already established.
6. **Components:** refine `Section`, `Badge`, `Meter`, `DataTable`, `StageHeader`, and `ResultsStrip`; introduce `SideCard`, `Metric`, `InsightTile`, `StatStrip`, and `MatchSummaryGrid` only when at least two screens need the same structure.

## Implemented in this worktree

- Replaced the page-wide contextual wash with neutral dark surfaces, explicit elevation, spacing, compact control/row tokens, and a restrained entity-hero tint.
- Reoriented the visual system to the corrected references: blue-black workspace, blue utility bar, violet selection/panel structure, and mint performance state.
- Refined the rail, top chrome, entity header, tabs, section headers, data tables, insight tiles, and right-rail information treatment.
- Added reusable `SideCard`, `Metric`, and `StatStrip` components to `ui/ui.tsx`.
- Redesigned the player overview with a scan-first stat strip and denser, calmer page hierarchy while retaining existing information and observer-only panels.
- Redesigned Today’s page rhythm and replaced exposed signed “On your mind” values with player-facing direction labels.
- Redesigned the club overview with a league/supporter stat strip and a compact club context card.
- Made competition overview weighting depend on whether it is a league table or a knockout-tie view.
- Added a genuine result-only match summary that shows only retained facts, available detail limits, previous meetings, and recorded talking points; detailed matches retain their existing score, events, players, and statistics view.
- Added reduced-motion-safe styling through the existing global preference support. No simulation or API behavior changed.
- Fixed the existing Windows screenshot-helper URL-to-path conversion so local browser captures write to the intended `app/e2e/shots/` directory.

## Implementation sequence

Each checkpoint should leave the frontend buildable and allow a screenshot comparison. Keep changes in `app/src`; touch Rust only if a real, already permitted field is missing and separately approved as a backend task.

### 0. Baseline and visual inventory

- Capture Today, controlled Person, Club, a league, a cup, and a result-only Match at 1366×768, 1920×1080, and 2560×1440 with the existing `shots.mjs` setup or equivalent browser capture.
- Record current route, world/save, viewport, color scheme, density setting, and whether match data is concealed. Use the same fixture for later comparisons.
- Inventory CSS selectors used by the five screens and flag rules overridden by later files. Capture interaction paths for navigation, Continue, save, follow, reveal, tabs, and table rows.

### 1. Tokens and shared surface language

- Expand `styles/tokens.css` with semantic app/shell/hero/panel surfaces, border strength, spacing, type sizes, radii, control and row heights, motion, and z-index.
- Move global skin rules out of the tail of `stage.css` into the appropriate token/component files. Keep dark and light themes viable.
- Update `ui/ui.tsx` and its styles with two or three intentional `Section` treatments, compact `SideCard`, `Metric`, `InsightTile`, and consistent badge/meter anatomy.
- Restyle the existing table in `styles/table.css` without changing data, sorting, keyboard access, selection, or virtualization.

### 2. Shell, page frame, and responsive grid

- Refine `components/Shell.tsx`, `components/Stage.tsx`, `styles/shell.css`, and `styles/stage.css`: compact rail, layered top chrome, clear active state, slimmer fixture strip, and restrained hero tint.
- Replace blanket page width behavior with reusable content plus context-rail layouts. At 1366px keep main and rail readable; at 1920px and above add useful columns where real content exists.
- Preserve search, back/forward, Continue, notifications, collapse, status, and all keyboard shortcuts. Treat broader command actions as a later enhancement because the current palette already searches entities/pages.

### 3. The five reference screens

| Screen | Primary layout change | Existing data and guardrail |
| --- | --- | --- |
| Person (`pages/Person.tsx`) | Focused player hero, a concise metric strip, visible state groups, insights as short fact tiles, compact contract/position rail. | Use `PersonResp` and `player_stats`; do not expose hidden traits or duplicate hero values in the rail. |
| Today (`pages/Today.tsx`) | Prioritize decisions and next action, then fixture strip/next match, recent results and world changes; compact personal context rail. Collapse empty modules. | Use `TodayResp`; turn `mind` into qualitative labels only when its text supports them. Keep concealed scores concealed. |
| Club (`pages/Club.tsx`) | Clear club hero and league status, fixture/form band, useful squad/facility/board rail, information sheets rather than repeated cards. | Use current club response and tables; avoid repeating the same fact in hero, main, and rail. |
| Competition (`pages/CompOverview.tsx`, `pages/Comp.tsx`) | League: standings dominate. Cup: active ties/bracket dominate. Leaders and news support the main module. | Branch on the existing `left.kind`; keep held-result handling and competition tabs. |
| Match (`pages/Match.tsx`) | Give result-only matches a complete summary layout with recorded score, context, talking points, available navigation and related data; organize detailed matches into a fuller center. | Render only `MatchResp` fields and recorded insights. Concealment/watch/reveal behavior stays authoritative. No invented shots, xG, or reactions. |

### 4. Secondary screens and finishing pass

- Apply the same row/card/rail language to Messages, news, Social, Life/Relationships, and other surfaces touched by shared components. Preserve each screen's function and tone.
- Create compact loading, empty, and error variants shaped to their modules. Avoid fixed-height empty panels.
- Audit hover/focus, reduced motion, tab order, selected state, text scaling, contrast, overflow, and icon alignment. Remove replaced CSS selectors and duplicate overrides.
- Compare the five key screenshots side by side before calling the redesign done.

## Acceptance gates

- The five screens look like one product at 1366×768, 1920×1080, and 2560×1440. No accidental horizontal overflow, clipped rails, unreadable text, or giant empty result page.
- Existing routes, tabs, links, table controls, Continue, save, follow, concealed-result reveal, and available actions still work.
- Played matches without detailed records have a useful honest summary. Concealed results reveal nothing until the existing action allows it.
- No new backend behavior, hidden data exposure, copied reference assets, or large UI dependency.
- Shared styles control recurring rows, headings, panels, badges, and tables. Obsolete overrides are removed as each region migrates.

## Verification workflow for implementation

From `app/`, use `npm run build` for the TypeScript and Vite gate. Use the existing `npm test` only for affected UI behavior. Use the local API/Chromium setup with `node e2e/shots.mjs name=#/route ...` for screenshot capture, setting `W`, `H`, `CHROMIUM`, and `BASE` as required. Inspect screenshots manually at each target viewport and check the five screens together at the end. Run simulation validation only if simulation code changes.

## Validation completed

- `npm run build` completed successfully (`tsc --noEmit` and Vite production build).
- Local Chromium validation captured Today, Person, Club, Competition, concealed Match, and revealed Match at 1440×900 without browser console errors.
- The production bundle reports the existing Vite advisory that the main JavaScript output is 510.36 kB minified. No dependencies or bundle splitting behavior changed in this redesign.

## Immediate next work package

Capture the baseline screens, then implement checkpoints 1 and 2. Revisit the five page layouts only after the shared tokens, surface hierarchy, and shell are stable.
