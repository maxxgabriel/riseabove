# Database integration audit

Audit date 2026-09-29, run against the user's local files **before** any import code was written.
All numbers below were measured on the files as found; none of the source files were modified.

Everything here is local user data. None of it is committed (`/archive/`, `/fm23_extracted/` and
`/fm23_test_extract_*/` are git-ignored) and none of it ships with the game; the importer reads a folder at
runtime, as `docs/DATA_PACK_BOUNDARY.md` requires.

## 1. Sources found

| Location | What it is | Usable for a world? |
| --- | --- | --- |
| `archive/` (13 CSV files, 875 MB) | Transfermarkt-style relational dataset (snapshot mid-2026) plus a staff list | **Yes: the primary source** |
| `fm23_extracted/2300_fm/usable/*.csv` | FM23 language-table exports: names of nations, clubs, competitions, cities, stadiums, media, awards | Names only, **no relations**. Not a world |
| `fm23_extracted/2300_fm/usable/*.raw`, `*/block_*.bin` | Decompressed FM23 binary tables (`people_db`, `server_db`, `client_db`, history tables) | **No.** Person/club/contract schema is not decoded (see `fm23_unresolved_entity_types.csv`) |
| `fm23_test_extract_20260928*/` | Earlier extraction experiments (one empty, one 390 raw blocks) | No |

The FM23 folders contain no player, staff, contract or club-to-league data that can be read without guessing the
binary schema. Guessing a proprietary binary layout would fabricate facts, so nothing is taken from them. They stay
preserved. The one thing they could supply, nation names and abbreviations, is already in the archive.

## 2. `archive/` file by file

Encoding is UTF-8 with `,` separators for every file except `Staff list.csv`, which is **Windows-1252**, `;`-separated
and quoted. Dates are `YYYY-MM-DD` (players.csv birth dates carry a ` 00:00:00` suffix).

| File | Rows | Key | Schema highlights | Populates |
| --- | ---: | --- | --- | --- |
| `countries.csv` | 124 | `country_id` | name, TM code (`ARM1`), confederation (`europa/asien/amerika/afrika`), club/player counts | Nations |
| `competitions.csv` | 65 | `competition_id` | name **is a slug** (`bundesliga`), type/sub_type, country, confederation, `total_clubs` | Competitions |
| `clubs.csv` | 796 | `club_id` | name, `domestic_competition_id`, stadium, seats, `coach_name`, `last_season`, squad stats | Clubs, stadiums, manager link |
| `players.csv` | 50,149 | `player_id` | name parts, DOB, citizenship, position/sub_position, foot, height, contract end, agent, market value, `current_club_id`, `last_season`, caps/goals | People, players, contracts, agents |
| `transfers.csv` | 175,165 | none (player+date) | player, date, from/to club id+name, fee, market value at transfer | Player club history, fee history |
| `player_valuations.csv` | 656,301 | player+date | market value over time | Value history |
| `games.csv` | 88,958 | `game_id` | competition, season, date, clubs, goals, managers, referee, formations, attendance | Past seasons, tables, managers, referees |
| `club_games.csv` | 177,916 | game+club | per-club result, managers, positions | (redundant with games) |
| `appearances.csv` | 1,894,350 | `appearance_id` | player, game, goals, assists, cards, minutes (2012 onward) | Career apps/goals, cards |
| `game_lineups.csv` | 3,179,016 | id | starters/subs, position, shirt number, captain (2013 onward) | Position usage, shirt numbers, captains |
| `game_events.csv` | 1,274,469 | id | goals, cards, substitutions with minute | (not needed for world start) |
| `national_teams.csv` | 124 | `national_team_id` | name, country, squad size, FIFA rank, coach | National sides |
| `Staff list.csv` | 8,974 | none (name+team) | name, French nation(s), abbreviated team, job, age, wage, 6 coaching ratings (`57% (3.0)`) | Staff |

### Scope actually importable

* 31 domestic first divisions, 10 domestic cups, 9 super cups, UEFA competitions, 6 national-team tournaments. **Only the
  top tier of each country is present; there is no pyramid below it.**
* Season keys use the start year: `2025` is 2025/26, complete; the newest game is dated 2026-07-06, the newest valuation
  2026-06-12. The snapshot is therefore taken as mid-July 2026 and the world starts on **2026-07-15**.
* 548 clubs have `last_season = 2025`; 19,092 players have `last_season = 2025` at one of them (squads 19-59, median 35).
  A further 16,000 rows point at a current club that is active but carry an older `last_season`: those are stale
  (retired, or moved out of the tracked leagues) and are **not** imported as players.
* 20 of the 548 clubs (Colombia, `COL1`) reference a competition that is absent from `competitions.csv`.

## 3. Mapping to Pathway entities

| Pathway entity | Source | Transformation |
| --- | --- | --- |
| `Nation` | `countries.csv` + every citizenship string in `players.csv` | Code is derived: ISO-style 3-letter table for known names, otherwise an explicit `X` + hash code (marked). Confederation from `countries.csv` (`europa`→UEFA...). Reputation/economy/youth rating are **not in the source**: derived from league strength, flagged INFERRED. Nations that only appear as a citizenship become minor nations flagged INFERRED |
| `Competition` (league) | `competitions.csv` `domestic_league` | Tier 1, `teams = total_clubs`. Display name = `"{country} {name}"` because slugs collide (`bundesliga`: Austria and Germany; `premier-liga`: Russia and Ukraine; `superliga`: Denmark and Romania) |
| `Competition` (cup / super cup) | `domestic_cup`, `domestic_super_cup` | Knockout format; entrants = clubs of that country |
| `Competition` (UEFA) | `international_cup`, UEFA `other` | Continental; qualifying rounds are not modelled: only the main competitions are created |
| Play-offs, national-team tournaments | `play_off`, `national_team_competition` | Not created as club competitions. National tournaments feed the national-side layer only |
| `Club` | `clubs.csv` (`last_season = 2025`) | Name/stadium/seats imported. Reputation, balance, budgets, facilities are **not in the source**: derived from league strength and total squad value (INFERRED). City, colours, founding year unknown (left empty / default, flagged UNKNOWN) |
| `Person` + `Player` | `players.csv` | Name parts, DOB, citizenship, foot, height, position imported. See "Attributes" below |
| `Contract` | `contract_expiration_date`, `agent_name`, market value | Contract end imported when present and not already past the start date. Wage is **not in the source** for players: derived from value and club level (INFERRED) |
| `Player` history | `transfers.csv` | Spells and fees before the start date, joined by `player_id` and club id (club names are not trusted for identity) |
| Player career counters | `appearances.csv` | Senior apps/goals since 2012 (documented coverage limit); older career totals are UNKNOWN, never zero |
| Past seasons | `games.csv` league games | Champion / runner-up from the computed table of complete seasons, provenance IMPORTED (derived from imported games). No top scorer lists are imported |
| Staff | `Staff list.csv` + `clubs.csv:coach_name` | See section 6 |
| Referees | `games.csv:referee` | Names only, IMPORTED; assigned by the existing referee system |

### Attributes (the largest gap)

The archive has **no technical, mental or physical attributes, no CA/PA and no hidden traits**. Market value is the only
ability signal (median 350,000 EUR, 99th percentile 30 M, max 200 M). Policy:

* Ability is **INFERRED** from market value, age and league level with a documented monotone rule. When the value is missing
  (18%) ability is **GENERATED** from the club's level, and flagged as such.
* Potential is INFERRED from age and ability (young high-value players get headroom). Attributes are generated to fit
  position and ability (existing generator) and flagged GENERATED.
* Hidden attributes (professionalism, ambition, ...) are GENERATED from the world seed.
* Every one of these is recorded per person in the world's origin book so the interface can say "estimated".

## 4. Identifiers and foreign keys

| Relation | Status |
| --- | --- |
| `players.current_club_id` → `clubs.club_id` | 2,986 of 50,149 rows reference an untracked club (lower divisions, other countries): not imported, reported |
| `clubs.domestic_competition_id` → `competitions.competition_id` | 20 clubs point at `COL1`, missing. Policy: create the league as INFERRED from its clubs and mark the name as not supplied |
| `games.competition_id` → competitions | `POCP`, `CGB`, `COL1`, `KLUB`, `UKRS` missing (1,214 games with empty type). Skipped, counted |
| `games.home/away_club_id` → clubs | 12,818 / 11,242 rows unknown (opponents outside the tracked clubs). Skipped for tables of unknown clubs |
| `transfers.from/to_club_id` → clubs | 109,676 / 92,746 unknown. History keeps the club **name as text** with the id as source id, no world club |
| `transfers.player_id` → players | all resolvable |
| `player_valuations.player_id` → players | all resolvable |
| Staff `Team` → club | **No key.** Free text with FM abbreviations (`Man Utd`, `Paris SG`, `A. Bilbao`); only 32 of 1,039 team strings equal a normalised TM club name. See section 6 |

Source ids are kept: every imported nation, competition, club and person carries `SourceRef { source, id }` in the world.

## 5. Duplicate and collision risks

* `player_id` and `club_id` are unique. Exactly **one** name+DOB duplicate pair exists; 938 names are shared by 2,263 different
  players. **Identity is by source id only.** No people are merged on name equality. A name+DOB duplicate with different ids is
  reported, not merged.
* No duplicate club names, but competition display names collide (fixed by country prefix).
* Staff names: 93 duplicated names; staff have no id. Identity = name + nation + age + team string; ambiguity is reported.
* A staff member who is also a former player is not linked to a `player_id` (no evidence beyond a name).

## 6. Staff list specifics

* 8,974 rows: 2,113 managers, 3,627 with no team (`-`), 1,094 managers with a team. Jobs include Owner/Chairman/Director/President.
* Nation is French text, `Espagne / Pays Basque` style (1,023 dual entries). A fixed French→English table covers the 166 distinct tokens; an unknown
  token stays UNKNOWN (never guessed).
* Ratings `57% (3.0)`: 8 coaching values on a 0-100% scale plus a 1-5 star value. They map to Pathway's staff attributes only partially
  (attacking, defending, tactical, technical, fitness, goalkeeping). The other staff attributes are GENERATED and flagged.
* Wage is a locale-formatted number with U+00A0 thousands separators and an unstated period and currency. Treated as UNKNOWN (not imported).
* **Club resolution rule:** a staff row joins a club only with evidence: (a) a manager whose normalised name equals `clubs.csv:coach_name` of exactly
  one club resolves that club and teaches the alias `Team string → club`; (b) other staff with the same team string and a compatible nation then join
  it, flagged INFERRED. Everything else stays unattached and appears in the unresolved report.

## 7. Unsupported and ignored fields

`image_url`, `url`, `filename`, `player_code`, `team_image_url`, city of birth (kept as text only), `net_transfer_record`, `total_market_value`
(recomputed), attendance and referee per match beyond the referee name pool, all `game_events` (not needed at world start), `club_games` (duplicate of `games`),
Owner/Chairman/Director rows (no matching entity type: reported), FM23 binary tables.

## 8. Ambiguous mappings (decisions taken)

1. **Which players are current:** `last_season = 2025` at an active tracked club. Older rows are history only.
2. **Expired contracts:** 758 current players have a contract end before the start date. The contract end is treated as UNKNOWN and a new contract end is GENERATED, flagged.
3. **Missing contract (21%):** GENERATED, flagged.
4. **Future-dated transfers:** 513 rows dated after the snapshot (up to 2030) are pre-agreed or scheduled moves. They are not history and are not applied.
5. **Positions:** `sub_position` maps to a natural position; `Missing` (199 current players) falls back to `position` group, flagged INFERRED.
6. **Height outliers** (17 cm etc.): outside 150-210 cm is treated as UNKNOWN and generated.
7. **Foot missing (12%):** GENERATED (flagged).

## 9. Missing-value policy

Nothing missing becomes zero. The import representation stores `Option` for every optional source field
(`None` = UNKNOWN). The world build then substitutes a value and records how: `Imported` (from the source),
`Inferred` (a documented rule over imported values), `Generated` (seeded randomness). Origin is kept per fact group
(identity, position, attributes, hidden traits, potential, contract, value, reputation, physique, career counters).
Counts of every substitution appear in the import report.

## 10. Provenance and licensing

The archive is a scrape of Transfermarkt pages (each row keeps its `url`). The dataset wrapper's licence does not extend to the underlying site content and
the staff list has no stated origin. It is for the user's personal local use; it must not be redistributed or committed, which the repository enforces by ignore rules
and by `docs/DATA_PACK_BOUNDARY.md`. Any future distribution requires replacing it with a licensed source or a fully generated world. Market values are third-party
estimates, not facts.

## 11. Imported versus generated in the running world

* `World.origins` (persisted with the save) records, per imported person/club/competition/nation, the source and id.
* Per player, an origin label per fact group. Generated regens and young intakes have **no entry**, which means "generated by the game".
* The import report and the person page show the difference; `world.origins` survives save/load.
* Past seasons keep the existing `Provenance::Imported / Generated` marks.

## 12. Pipeline

```
SOURCE FILES
   -> parser (per adapter: pack CSV, Transfermarkt archive, staff list) -> ImportSet + Issues
   -> validation (dates, ranges, encodings, enums) -> drops or marks bad fields
   -> resolution (foreign keys, identity evidence, duplicates, staff to club)
   -> assemble (world building through the ordinary builder functions)
   -> World + OriginBook -> versioned save
```

The simulation never reads a CSV layout. All adapters emit the same `ImportSet`.
