> **Historical report.** Written for branch `claude/kind-planck-t66j8f` (commit `401b7ae`). It may not describe the current code; see `IMPLEMENTATION_STATUS.md` and `docs/README.md`.

# Integration report

Date: 2026-09-28. Scope: the whole workspace after the media/social/history pass and the final depth
pass (`docs/MEDIA_SOCIAL_HISTORY_SYSTEMS.md`, `docs/FINAL_DEPTH_PASS.md`), taken from "never compiled"
to compiling, formatted, lint-clean, tested and validated over long runs.

This report claims **zero known reproducible correctness failures in the tested scope**. It does not
claim that no bugs exist. The scope and its gaps are stated below.

How to reproduce:

```
cargo check --workspace --all-targets --exclude pathway-app
cargo clippy --workspace --all-targets --exclude pathway-app --exclude ofm-engine
cargo test --release --workspace --exclude pathway-app --exclude ofm-engine
cargo test --release -p pw-cli --test world -- --ignored --nocapture   # long runs and reports
PW_PROFILE=1 target/release/pathway-sim synth 52 --days 120             # ~300k players, per-system timing
```

`pathway-app` (the desktop client) has not been started and is excluded. `ofm-engine` is vendored
upstream source, excluded from our lints and formatting.

---

## 1. Compilation problems found and fixed

The crates were checked bottom-up (`pw-core` → `pw-data` → `pw-world` → `pw-match` → `pw-sim` →
`pw-import` → `pw-narrate` → `pw-career` → `pw-cli`), then the whole workspace with all targets.

| Crate | Problems | Fixes |
|---|---|---|
| pw-core, pw-data, pw-world, pw-match | none | — |
| pw-sim | 11 errors: `retain` patterns on mutable references (3); an ambiguous integer type; borrows of `World` held across mutation (competition data in the chronicle, a closure over teams in culture, a doubly borrowed RNG in governance); moving a non-`Copy` `EventKind` out of a reference (2); a non-exhaustive `Choice` match; an explicit dereference inside an implicit-borrow pattern. 6 warnings (unused imports, a dead duplicate `freshness`, unused fields, an unreachable arm) | fixed in place. Nothing was disabled; the duplicate function was removed because `timely` already did its job |
| pw-narrate | 4 errors: `use EventKind::*` shadowed the `CoachNote` type with a variant | full paths |
| pw-career | 1 error: a local `club` shadowed the `club()` formatter | renamed the local |
| ofm-engine (tests) | missing `serde_json` for upstream's own test | dev-dependency in the manifest only; source untouched |
| doctest | a formula in a module comment was compiled as Rust | marked as `text` |

**Formatting:** a `rustfmt.toml` (width 200, "Max" heuristics) matches the codebase's wide style
(about 500 diffs instead of about 2,200 with the defaults). The vendored engine opts out.

**Clippy:** 139 warnings went to 0.

- Mechanical fixes were applied automatically (let-chains, `is_multiple_of`, useless conversions).
- The rest were fixed by hand:
  - type aliases for complex tuples
  - a `NewPost` builder replacing a 14-argument function
  - boxed match selections
  - `clamp`, `?`, merged identical branches
- Justified exceptions:
  - `needless_range_loop` in two engine loops that index several arrays together
  - a documented `too-many-arguments-threshold = 11` in `clippy.toml`

## 2. Tests: counts and failures fixed

| Layer | Where | Count |
|---|---|---|
| Unit | pw-core, pw-data, pw-sim, pw-import, pw-match | 15 |
| Property (proptest) | `pw-cli/tests/edges.rs`: text checks on arbitrary input, rule-history reconstruction, records only improving | 3 |
| Edge cases | `edges.rs`: save with a pending decision, Unicode and very long names, no cycles in replies or tellings | 3 |
| Whole-world invariants and scenarios | `pw-cli/tests/world.rs` | 14 |
| Language and media matrix | `pw-cli/tests/language.rs` | 4 |
| Long runs and reports (`--ignored`) | `world.rs`: 5, 20 and 50 seasons, the causal-chain report; pw-match calibration and diagnostics | 7 |

**Result: 39 passing, 0 failing; the 7 ignored tests were run manually** (results in §10 and §3).

**Failures found by the tests, and their fixes** (each one is a real bug):

1. **Crash:** a press answer about nobody (the last match) looked up a missing person.
2. **Save/load drift:** loading a save re-ran world preparation (a monthly market pass, a round of
   knowledge, a round of minds), so a loaded world diverged from the continuous one. Preparation now
   runs once, recorded in `World::prepared`.
3. **Crash:** a club-less fine reached the grapevine. The underlying bug was an incident response
   fining someone with no club; only a club can fine.
4. **Crash:** people created during a day (cohorts, referees, journalists) had no life until the next
   day. Lives are now synced after each phase that can create people.
5. **Missing history:** backfill, referee pools and rule pressures read league tables before the tables
   are drawn, so the backfill produced zero seasons. They now use league membership.
6. **Truth:** an agent sounding out the market produced a headline naming "no club".
7. **Truth:** a quote with no subject gave the headline "X criticises " followed by nothing.
8. **Truth:** follow-up journalists wrote private information they had never heard, as if it came from
   their own source. They can now use it only once it is published, and they cite the earlier story.
9. **Truth:** grapevine compaction dropped the holders that old stories cite, so a leak's source path
   became uncheckable; this was found by a 50-season run. Cited holders are now kept.
10. **Consistency:** retired players stayed members of local clubs and could play amateur football.
11. **Checker:** the doubled-word check misread rhetoric ("goals, goals, goals").
12. **Guard semantics:** the past-name guard compared generated figures against people born years later.
    It now compares against the people who existed when the past was written.
13. **Perceived bias never formed:** supporters' grievances against referees decayed once per match on
    the first of each month, which wiped them. They now fade once a month. The test asserts that belief
    in a biased referee forms while calls stay equally correct for home and away sides: 2,166 calls
    against home sides were 91.3% correct, 2,409 against away sides 90.5%, within a four-standard-error
    binomial tolerance. A first, smaller version of this test (four weeks of calls) failed on noise; it
    was rebuilt with enough data rather than loosened.
14. **Agents exploring constantly** (about 6,300 times a season on the small world): the dedup looked in
    a short recent-items list, and routine contract talks counted as unrest. Now each representation
    records its last exploration (at most every half season), and only unhappiness, discipline, a split
    dressing room or stalling talks trigger it. The result is about 1,650 a season.

## 3. Causal chains demonstrated

Tested (`incidents_become_causal_chains_that_differ_by_seed`, 3 seeds). Each chain runs:

- a contextual incident, whose recorded causes are the pressures that made it likely
- then information items that at least two people know
- then a response from someone with authority

Every seed produced thousands of such chains, and their mix differs between seeds. The incident mix
was recalibrated during testing (§11).

Other links are asserted by tests or by the audit on every season of the long runs:

- information → journalist → story; every story from a source is checked for a real source path
- story → supporters' belief and relay → trust in the outlet, updated as threads resolve
- viral post → fan-reaction story, sourced to the post
- referee call → grievance → perceived bias → supporters' opinions → appeals and charges
- records at every level → record events carrying their history
- votes → awards, with ballots and reasons

`causal_chain_report` (ignored; run with `--nocapture`) counts chain links over two autonomous
seasons on the small world, three seeds (301, 302, 303):

| Link | Per world |
|---|---|
| leaks suspected by colleagues | 383–499 |
| boards asking managers to explain what they heard | 1,100–1,148 |
| agents quietly exploring the market | 3,223–3,401 |
| captains mediating | 605–672 |
| incident responses | 10.5k–11.1k |
| stories resting on a source | 642–907 |
| stories about viral supporter posts | 143–395 |
| denials | 39–50 |
| running stories resolved (happened, denied, collapsed, faded) | 610–799 |
| red-card appeals decided | 13–19 |
| misconduct charges | 66–83 |

Together these show the cascade in brief item P: a private event is witnessed, told on, suspected as a
leak, queried by the board, and acted on by an agent. Rarer links: supporter-group actions (1–10),
records (12–15), chronicle entries (1–5), a tactical school (0–1).

Supporters changing their minds ("fair enough, I was wrong" or doubling down) and call-outs are rare
(0–8). Opinions move slowly, so the thresholds for these are seldom crossed in two seasons (§12).

## 4. RNG reproducibility

- **Same seed, same world.** Two runs with the same seed produce identical digests over 200 days: event
  kinds, tables, posts, stories, incidents and records.
- **Save and load.** Save at day 120, load, and run 60 more days: the result equals the continuous run.
- **Narration draws no randomness from the world.** A world whose every line is rendered daily evolves
  identically to one nobody reads, so adding wording variants cannot shift match, injury or transfer
  streams (`narration_never_changes_the_world`).
- **Identities are stable** through time and through save and load (`identities_are_stable_within_a_save`).

## 5. Multi-seed variation

Different seeds produce different world digests and different supporter handles; the test asserts
both. Incident chains differ in shape across seeds.

Across three seeds the language matrix saw 19,332 posts, 49% of them distinct. The rest are expected
repeats of short reactions to the same shared events.

## 6. Social and news truthfulness

The semantic audit (`pw-sim/src/audit.rs`) runs after every tested season and must be clean. It checks:

- no story stated as fact without being grounded or public
- no transfer reported as done without the transfer
- no story from a source the journalist never heard it from
- no post referring to a missing post, or citing a story published after it
- no call-out without a real earlier post
- no relayed rumour without a story
- no record "broken" by a mark that is not better
- no inbox message without a source
- no vote that doesn't add up, or that names nobody
- no generated past figure named like a real person
- no broken school or university membership
- no rule value out of range

Result: 0 violations in every season of the 5-, 20- and 50-season runs, after the fixes in §2.
Before those fixes, the audit caught the sourceless follow-ups and the compaction loss.

## 7. Grammar and text generation

`pw-narrate/src/quality.rs` checks every rendered event line, post and headline in the tests for:

- empty text, placeholders and unfilled templates
- doubled words and spaces
- malformed punctuation
- missing capitals (on sentences)
- gendered pronouns
- length

Results:

- Every line in the tested worlds passes.
- Formal and neutral outlets never use casual slang (checked across seeds).
- Vocabulary follows dialect (lexical only), era (no xG before the analytics era) and register.
- Young people's slang moves with the era.
- Stats and casual accounts sound different.
- Unicode and very long names render cleanly.

## 8. Save/load

- A save continues exactly (§4).
- History survives save and load: records, votes and minor-football history.
- A decision pending at save time can be answered after loading, and it resolves.
- Timing: a world of about 300k players saved to about 480 MB, and loading plus reporting took
  8.5 seconds.

## 9. Autonomy

`a_world_without_a_protagonist_is_alive` runs two seasons with no human. It asserts that all of the
following occur:

- incidents, news and rumours
- transfers and appointments
- retirements and post-playing careers
- awards and records
- minor football and refereeing controversies

In one run: 28,663 incidents (before recalibration), 12,018 published stories, 590 rumours,
91 transfers, 105 retirements, 15 new careers, 185 awards, 143 records, 40 minor-football seasons and
309 disputed calls. No system anywhere checks for a protagonist.

## 10. Long runs

| Run (final code; the three ran concurrently) | Result | Notes |
|---|---|---|
| 5 seasons, small (64 clubs, about 5,400 active players) | pass; audit clean every season | 76 s; posts in the 45-day window stay at 3–5k |
| 20 seasons, small | pass; audit clean every season | 405 s; 3 rule changes, 10 tactical schools, 399 votes, 1,106 records, 184k stories |
| 50 seasons, tiny (8 clubs) | pass; audit clean every season | 178 s; 6 schools, 320 votes, 74.5k stories |

What the long runs checked for (brief §33):

- **Social account and post explosion:** found and fixed (§11); posts are now flat.
- **Event-log and history growth:** stories accumulate at about 9k a season on the small world (kept
  as history), and kept posts grow by about 2k a season (referenced or viral, capped at 50k).
- **Record corruption:** none (audit).
- **Squads:** no empty squads in the runs.
- **Population:** the active population is stable (about 500–680 in squads on tiny). Retired people
  and amateurs accumulate, because people persist.
- **Rule evolution:** rare and caused (0 changes in 50 tiny seasons, where one nation has little
  measurable pressure; 3 in 20 small seasons).
- **Tactical monoculture:** schools appear slowly (1–10 over the runs).
- **Not yet measured:** economic balance, inflation and reputation saturation over 50 seasons. These
  need dedicated metrics (§13).

## 11. Performance

Profiling uses the per-system timer (`PW_PROFILE`) on the daily pipeline.

**Scaling bugs found and fixed.** Each was a world-sized scan inside a per-person loop:

| Where | Problem | Fix |
|---|---|---|
| meetings | "days since they last met" and "pending?" scanned all meeting history | indexes by pair and by open meeting |
| morale | scanned all promises for every player each week | promise indexes by promiser and promisee |
| grapevine | recomputed contacts for every holder of every item daily | contacts once per person per day, a hash set of knowers, and a person passes something on only in the week after hearing it |
| manager summons | scanned a week of world events for every player | one set of recent bans |
| transfers | scanned a year of events for a transfer request | the market's own record |
| minutes and fixtures | scanned every fixture in the world per player | a per-team fixture index |

**Growth bugs found and fixed:**

| Problem | Fix |
|---|---|
| Social post explosion: every viral post became a meme, rivals reused all memes daily, and famous people's accounts reacted to everything like supporters | memes are born only from viral jibes (one per club a month), used on match days with fatigue, and fade; real people post about their own moments, and their accounts are created lazily |
| Incident rates: about 14k incidents a season on the small world, because household incidents fired without circumstances | rebased so that pressures carry the probability (about 5.7k a season, mostly squad friction) |

**Measured** after the fixes (synthetic worlds; release build; 4 cores; 120 days from 1 July, which
covers the transfer window and the season start):

| Players | Clubs | ms/day | Peak memory | Save after 120 days |
|---|---|---|---|---|
| 5.8k | 88 | 17 | 68 MB | 10 MB |
| 23k | 352 | 77 | 254 MB | 42 MB |
| 52k | 792 | 231 | 526 MB | 93 MB |
| 99k | 1,496 | 584 | 1.0 GB | 174 MB |
| 302k | 4,576 | 2,411 (6,910 before the last two fixes; 181 s for 30 days before the first) | 2.46 GB | 480 MB (load and report 8.5 s) |

The world builds in 2.8 s at 302k players.

Largest remaining costs at 300k:

| System | ms/day |
|---|---|
| social network daily | 530 |
| perception (weekly) | 2,150 per call |
| matches | 234 |
| minds (weekly) | 1,600 per call |
| grapevine | 194 |
| youth (weekly) | 1,250 per call |

All scale roughly linearly. A 300k season is about 15 minutes of simulation. The PROGRESS budget (a
simulated weekend in 6 seconds) is met up to about 50k players, but not at 300k.

## 12. Remaining shallow systems

- Minor football uses a strength result model, not play; school age groups are not separated.
- Referees affect only card strictness and whether big calls are correct; there is no VAR era yet.
- Supporter groups do not yet interact with each other.
- Chants and memes have few shapes; English only (one pack, with dialect, era and register variants).
- Inbox replies are a fixed set per message type.
- Rule evolution covers four rules.
- The chronicle covers a fixed set of feats.
- Institution hall committees are journalists (schools have no people).
- Backfill covers top flights only (no past transfers or managers).
- Several pre-existing systems (economy, market) are not validated here beyond staying alive.
- Supporters' opinions move slowly. "I was wrong" and doubling down are rare in two seasons, and so
  are call-outs of earlier posts. The mechanism works (the renderer and the audit check it) but it is
  seldom triggered.
- Fame-based person accounts appear only when a person posts, and people post only about their own
  big moments.

## 13. Remaining known risks

- **Balance over decades:** economy, wages, reputation and fame are not yet measured over 50 seasons.
  Fame inflation is suspected: the number of people above a fame threshold grew fourfold in three
  seasons on the small world, before accounts became lazy.
- **Calibration:** the rates (incidents about 5.7k a season on the small world, agent explorations
  about 1,650, stories from sources about 320–450) are plausible but not tuned against real football. They need targets
  before release.
- **History growth:** stories (about 9k a season on the small world), kept posts and the event log grow
  with history. Save size at 300k is 480 MB after 120 days.
- **Save format:** saves are unversioned `bincode`. Any change to `World` breaks old saves; there is no
  migration.
- **Determinism:** it depends on `FxHashMap` iteration never deciding outcomes. The replay and
  save/load tests pass, but a new unsorted iteration could break this silently. The digest tests are
  the guard.
- **Humans:** a human gets more time on some decisions (meetings three days ahead, press questions due
  on match day). These are allowances to respond, not extra opportunities, but they make an
  AI-versus-human world diverge.
- **Not covered by tests:**
  - hidden-state exposure through UI sorting, tooltips or export (the views use perception, but this is
    untested)
  - registration rules on transfer completion
  - manager changes during negotiations
  - death (not modelled)

## 14. Remaining known bugs

**None known and reproducible in the tested scope** after the fixes above. Areas where problems are
likely but unproven are listed in §13.

## 15. Systems that need deeper algorithms next

- **Performance of the relationship model** at 300k: incremental or sampled drift instead of touching
  every player-manager pair weekly.
- **Newsroom:** editorial libel risk, the subject's power, and outlet-specific voter lenses.
- **Supporter posts:** multi-frame narratives ("the third derby loss this season") from memory queries
  rather than single frames.
- **Incident responses** that consult precedent (how similar cases were handled).
- **Tactical meta with real mechanical counters:** today the meta is fashion and lineage; the match
  engine does not reward counters.
- **Language packs** beyond English, and data-driven chant and meme shape libraries.
- **Save migration and versioning.**
- **Economic and long-horizon balance dashboards** built on the profiler and audit infrastructure.
