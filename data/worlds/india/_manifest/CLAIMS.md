# Who is researching what (append-only, newest at the bottom)

Two threads are filling this directory in parallel. To keep merges clean each takes different datasets and different files.
If you take something, add a section here and push it. If you need something that is claimed, put the request in the section.

## 2026-09-30 thread "UI and integration" (branch `data/india-research-b`, cut from `claude/india-real-world-data` at 4724be1)

Taking, from Maxx's priority list: 1 media outlets, 2 broadcasters and season coverage, 5 clubs of the four national tiers (tier 3 and 4 first),
7 academies, 9 schools and hostels, 10 stadiums, 11 grassroots programmes, 12 international development partnerships, 14 languages and aliases.

Leaving to the other data thread: 3 state and territorial associations, 4 national competition structure and participants, 6 state leagues,
8 universities, 13 registration and eligibility rules, 15 national, state and youth teams.

Files I write (new files use a `_b` suffix; existing files listed here are edited in place by me only):
- `media/outlets.toml` (in place: verify and upgrade the existing outlets), `media/outlets_*_b.toml` (new outlets)
- `broadcasters/broadcasters.toml` (in place), `broadcasters/rights_b.toml`
- `clubs/tier34_b.toml`, `competitions/membership_b.toml` (memberships for the clubs I add; competition records stay yours)
- `academies/academies.toml` (in place), `academies/academies_b.toml`
- `schools/schools_b.toml`, `stadiums/stadiums_b.toml`, `grassroots/programmes_b.toml`
- `partnerships/partnerships.toml` (in place), `languages/*_b.toml`

Request to the other thread: if you add clubs to state leagues, use ids of the form `club.<slug>` and grep `clubs/` first so we do not both add the same club.

## 2026-09-30 (later) thread "UI and integration": second wave, all new `_b` files, model knowledge marked C/D
Maxx asked for base-knowledge data with no web verification. Nothing from the other data thread had landed on origin yet, so this thread is
also drafting (new `_b` files only, never edits to their files; ids are grepped before use; the other thread's records win on any clash):
`associations/state_assoc_b.toml`, `state_leagues/*_b.toml`, `universities/universities_b.toml`, `national_teams/teams_b.toml`, `rules/rules_b.toml`,
`clubs/women_b.toml`, `media/outlets_specialist_b.toml`, second passes on `stadiums/stadiums_b.toml`, `schools/schools_b.toml`.
