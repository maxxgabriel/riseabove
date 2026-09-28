# Pathway world import format

A world is a folder of UTF-8 CSV files with a header row. Column order does
not matter; unknown columns are ignored; optional columns may be absent or
empty. Load with `pw-cli new --import <folder>`.

Dates: `YYYY-MM-DD` or `DD/MM/YYYY`. Money: whole units of one base currency
(use the currency of your export consistently). Attributes: FM's 1–20 scale;
if a file's attribute values exceed 20 the whole file is read as 1–100 and
divided by 5.

## `world.toml` (optional)

```toml
start_date = "2022-07-01"   # game start; default 2022-07-01
seed = 42                    # world seed; default derived from folder name
```

## `nations.csv` (required)

| column | req | meaning |
|---|---|---|
| `code` | ✓ | unique short code, e.g. `ENG` |
| `name` | ✓ | display name |
| `confederation` | ✓ | `UEFA`, `CONMEBOL`, `CONCACAF`, `CAF`, `AFC`, `OFC` |
| `reputation` | | 0–10000 football strength (default 3000) |
| `calendar` | | `autumn_spring` (default), `autumn_spring_break`, `calendar_year`, `calendar_year_nordic` |
| `economy` | | wage/revenue level, 1.0 = richest (default 0.5) |
| `youth_rating` | | 1–20 youth development (default 10) |

## `competitions.csv` (required)

| column | req | meaning |
|---|---|---|
| `id` | ✓ | unique id (any string) |
| `name` | ✓ | |
| `short_name` | | |
| `nation` | ✓* | nation code (*empty for continental) |
| `confederation` | | for continental competitions |
| `kind` | ✓ | `league`, `cup`, `continental` |
| `tier` | | league tier (1 = top). For cups: how many league tiers enter (default 4) |
| `team_kind` | | `first` (default), `reserve`, `u21`, `u19`, `u18` |
| `teams` | | intended size (continental: e.g. 32) |
| `promote` / `relegate` | | automatic places to/from adjacent tiers |
| `reputation` | | 0–10000 |
| `format` | | `league` (default for leagues), `knockout` (cups), `groups` (continental) |
| `legs` | | league passes (2) / knockout legs (1 or 2) |
| `groups`, `group_size`, `advance` | | group stage shape (default 8, 4, 2) |
| `prize_pool` | | total prize money |

League membership comes from `clubs.csv:league`. Youth leagues and explicit
entrants can be listed in `competition_entrants.csv` (`competition`, `club`).

## `clubs.csv` (required)

| column | req | meaning |
|---|---|---|
| `id` | ✓ | unique id |
| `name` | ✓ | |
| `short_name` | | |
| `nation` | ✓ | nation code |
| `city` | | |
| `league` | ✓* | competition id of the first team's league (*empty = no league) |
| `reputation` | | 0–10000 (default from league) |
| `balance`, `transfer_budget`, `wage_budget` | | money (wage budget weekly) |
| `stadium`, `capacity` | | |
| `training_facilities`, `youth_facilities`, `youth_recruitment` | | 1–20 |
| `colour1`, `colour2` | | hex `#RRGGBB` |
| `founded` | | year |
| `teams` | | extra sides, `;`-separated: `reserve;u21;u18` |

## `players.csv` (required)

| column | req | meaning |
|---|---|---|
| `id` | ✓ | unique id |
| `first_name`, `last_name`, `common_name` | ✓ (one of) | |
| `dob` | ✓ | date of birth |
| `nationality` | ✓ | nation code (unknown codes are created as minor nations) |
| `second_nationality` | | |
| `club` | | club id; empty = free agent |
| `team` | | `first` (default), `reserve`, `u21`, `u19`, `u18` |
| `positions` | ✓ | our codes (`ST,AMC`) or FM notation (`AM (RLC), ST (C)`); first = natural |
| `foot` | | `left`, `right`, `either` — or `left_foot`/`right_foot` 1–20 |
| `height`, `weight` | | cm, kg |
| 46 attribute columns | ✓ | keys below |
| 13 hidden columns | | keys below (random if absent) |
| `ca`, `pa` | | current/potential ability 1–200; FM negative PA ranges accepted |
| `wage`, `contract_end` | | weekly wage, date |
| `value` | | |
| `reputation_current`, `reputation_home`, `reputation_world` | | 0–10000 |
| `squad_status` | | `star`, `important`, `regular`, `squad`, `impact_sub`, `fringe`, `backup`, `youngster`, `not_needed` |
| `loan_from`, `loan_end` | | parent club id and loan end date when the player is on loan at `club` |

Attribute keys: `corners crossing dribbling finishing first_touch free_kicks
heading long_shots long_throws marking passing penalty_taking tackling
technique aggression anticipation bravery composure concentration decisions
determination flair leadership off_the_ball positioning teamwork vision
work_rate acceleration agility balance jumping_reach natural_fitness pace
stamina strength aerial_reach command_of_area communication eccentricity
handling kicking one_on_ones reflexes rushing_out throwing`

Hidden keys: `consistency important_matches injury_proneness versatility
adaptability ambition loyalty pressure professionalism sportsmanship
temperament controversy dirtiness`

## `history.csv` (optional — generated if absent)

Past seasons before the start date. Seasons given here are marked **imported**; any top-flight
season not given is generated from the world seed and marked **generated**. Only include history you
have the right to use (see `docs/DATA_PACK_BOUNDARY.md`).

| column | req | meaning |
|---|---|---|
| `competition` | ✓ | competition `id` from `competitions.csv` |
| `season` | ✓ | year the season started |
| `champion` | ✓ | club `id` |
| `runner_up` | | club `id` |
| `top_scorer` | | name as text (not linked to a player) |
| `top_goals` | | goals |

## `staff.csv` (optional — generated per club if absent)

| column | req | meaning |
|---|---|---|
| `id`, `first_name`, `last_name`, `dob`, `nationality` | ✓ | |
| `club` | | empty = unemployed |
| `role` | ✓ | `manager`, `assistant`, `coach`, `gk_coach`, `fitness_coach`, `scout`, `physio`, `sports_scientist`, `head_of_youth`, `director_of_football` |
| staff attribute columns | | `attacking defending fitness mental tactical technical goalkeeping working_with_youngsters motivating discipline man_management judging_ability judging_potential tactical_knowledge physiotherapy sports_science negotiating media_handling` |
| `reputation` | | 0–10000 |
| `formation` | | preferred formation key (`4-4-2`, `4-3-3`, `4-2-3-1`, …) |
