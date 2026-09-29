# Real-world reference data: schema

This folder holds the **researched Day-0 baseline** for India. It is organised by domain; each
domain is one or more TOML files. The same layout is meant to be reused for other nations under
`data/worlds/<nation>/`.

Reality boundary: everything here describes the real world **as of the start of the 2026-27 season**
(Day 0 = 2026-07-01) or earlier. Nothing here is simulated history. From Day 0 onward the simulation
owns every change (tiers, results, partnerships, sponsors, reputations).

`pack.toml` is separate: it holds **scenario seeds** (initial strengths, coarse travel grid) that the
world builder reads, labelled as such. It references the reference data by id.

## Files

```
data/worlds/india/
  SCHEMA.md                  this file
  MANIFEST.toml              research log: one [[dataset]] per file (what, sources, coverage, gaps, last checked)
  GAP_AUDIT.md               what was placeholder/generated before, and its classification
  pack.toml                  scenario seeds read by the world builder
  geography/states.toml      [[state]]
  geography/districts.toml   [[district]]
  associations/*.toml        [[association]]
  competitions/*.toml        [[competition]]
  competitions/membership.toml [[membership]]   club-in-competition by season
  clubs/*.toml               [[club]]  (+ clubs/ownership.toml [[ownership]])
  stadiums/stadiums.toml     [[stadium]]
  state_leagues/*.toml       [[competition]] + [[club]] + [[membership]] for one state's leagues
  academies/academies.toml   [[academy]]
  universities/universities.toml [[university]]
  schools/schools.toml       [[school]]
  media/outlets.toml         [[outlet]]
  broadcasters/*.toml        [[broadcaster]] + [[rights]]
  grassroots/programmes.toml [[programme]]
  partnerships/partnerships.toml [[partnership]]
  rules/*.toml               [[rule]]
  national_teams/*.toml      [[team]] (national and state representative teams)
  development/coach_education.toml [[licence]]
  development/referees.toml  [[grade]]
  culture/rivalries.toml     [[rivalry]]
  commerce/*.toml            [[sponsorship]]
  languages/languages.toml   [[language]]
  languages/terminology.toml [[term]]
```

## Every file

Every file starts with a `[meta]` table:

```toml
[meta]
dataset = "clubs.isl"             # unique dataset name
quality = "A"                     # dataset-level grade, see below
effective = "2026-07-01"          # the date the snapshot describes
season = "2026-27"                # when the data is season-specific
last_checked = "2026-09-29"
coverage = "All 13 ISL 2026-27 clubs."
gaps = ["Founded year of X unverified."]
```

## Every record

Every record has a unique `id` and a `prov` (provenance) inline table:

```toml
prov = { status = "verified", q = "A", src = ["https://www.the-aiff.com/..."], note = "optional" }
```

- `status`: `imported` (copied from a structured source), `verified` (checked against an official or
  two independent strong sources), `inferred` (reasoned from sources, not stated by them),
  `scenario_seed` (a deliberate scenario choice, not a fact), `generated` (produced by code),
  `unknown` (entity is real but key facts could not be confirmed).
- `q`: `A` officially verified, `B` strong secondary verification, `C` partial/inferred,
  `D` scenario seed, `E` generated.
- `src`: URLs actually consulted (official first). Never a URL that was not opened or returned by search.
- Optional `conflicts = [{ field = "founded", values = ["1889", "1891"], src = ["url", "url"], resolution = "unresolved" }]`.

**Unknown is valid.** Omit a field you could not verify. Never fill a plausible guess.

## Ids

Lowercase, `kind.slug`, globally unique within the world:

| prefix | entity |
|---|---|
| `state.` | state/UT, slug = the lowercase key used in `pack.toml` (`state.kl`, `state.wb`) |
| `district.` | `district.<statekey>.<slug>` (`district.kl.malappuram`) |
| `assoc.` | federation / association (`assoc.aiff`, `assoc.kfa`, `assoc.ifa`) |
| `comp.` | competition (`comp.isl`, `comp.santosh-trophy`) |
| `club.` | club (`club.mohun-bagan-sg`) |
| `stadium.` | ground |
| `academy.` | academy / development centre |
| `uni.` | university |
| `school.` | school, sports school, sports hostel |
| `media.` | media outlet |
| `bcast.` | broadcaster / streaming platform |
| `prog.` | grassroots or development programme |
| `partner.` | partnership |
| `rule.` | rule |
| `team.` | national or representative team |
| `licence.` | coaching licence |
| `refgrade.` | referee grade |
| `rivalry.` | rivalry / derby |
| `lang.` | language (`lang.ml`, ISO 639 code) |

A reference to an entity outside India (a foreign partner club) is written inline as
`{ name = "...", nation = "ESP", kind = "club" }`, not as an id.

## Record shapes

Fields marked `?` are optional. Enumerations are lowercase snake_case.

### `[[state]]`
`id, key, name, kind (state|union_territory), capital?, languages (lang ids: official/principal), population_2011?, prov`

### `[[district]]`
`id, state, name, prov`

### `[[association]]`
`id, name, abbr?, kind (national|state|institutional|district), state? (state id), parent? (assoc id),
hq_city?, website?, aiff_member? (bool), status (active|suspended|provisional|unknown),
competitions? (comp ids), prov`

### `[[competition]]`
`id, name, short?, aliases?, organiser (assoc id), operator? (text), level (national|state|district|zonal|continental),
tier? (int, national pyramid position; state leagues use their in-state tier), state? (state id),
age (senior|u23|u21|u20|u19|u18|u17|u16|u15|u14|u13|u12), gender (men|women|mixed),
kind (league|cup|tournament|championship), season, effective_from?, effective_to?, teams?,
format? (short text), stages? (list of short texts), promotion? (text), relegation? (text),
qualifies_to? (comp ids), fed_by? (comp ids), calendar? (text: months), foreign_rule? (rule id),
tiebreak? (list), prize? (text), status (active|suspended|defunct|unknown), prov`

### `[[membership]]`
`club, competition, season, stage? (text), note?, prov` (no id needed)

### `[[club]]`
`id, name (common display name), official_name?, short?, aliases?, city, state, district?,
founded? (int year), kind (professional|institutional|departmental|academy|amateur|university),
home_ground? (stadium id), status (active|inactive|defunct|suspended), parent? (text: corporate/institutional parent),
academy? (academy id), website?, prov`

### `[[ownership]]`
`club, owner (text), kind (corporate|consortium|individual|institutional|member_club|government|unknown),
effective_from?, effective_to?, prov`

### `[[stadium]]`
`id, name, aliases?, city, state, capacity? (int), capacity_as_of? (year or date), surface? (natural|artificial|hybrid),
home_clubs? (club ids), owner? (text), prov`

### `[[academy]]`
`id, name, city, state, parent? (club id), parent_name? (text, for non-club parents),
kind (club|independent|state|sai|corporate|school), residential? (bool), age_groups? (list),
accreditation? (list of { body, rating, season }), competitions? (comp ids), programmes? (list of text),
website?, status (active|inactive|unknown), prov`

### `[[university]]`
`id, name, short?, city, state, kind (central|state|private|deemed|institute_of_national_importance),
football? (list of comp ids it takes part in), zone? (aiu zone: north|south|east|west),
achievements? (list of { competition, season, result, src }) — verified only,
residential? (bool), sports_note? (short text, sourced), website?, prov`

### `[[school]]`
`id, name, city, state, kind (school|sports_school|sports_hostel|sai_centre|military|academy_school),
residential? (bool), competitions? (comp ids), programme? (text), operator? (text), prov`

### `[[outlet]]`
`id, name, kind (national_sports|football_specialist|general_news|tv_sports|digital_sports|regional|
local_newspaper|official_club|official_federation|official_league|campus|radio|news_agency),
medium (list: print|digital|tv|radio|social|video), languages (lang ids), reach (national|multi_state|state|local),
home_state? (state id), home_city?, focus? (list: professional|national_team|youth|state|grassroots|women|
transfers|tactics|stats|international|regional_leagues), football_emphasis (specialist|strong|general),
style? (list: breaking|long_form|analysis|match_reports|features|video),
owner? (text), founded? (int), website?, audience? (large|medium|small|niche) — only when defensible,
active (bool), prov`

No credibility or quality scores. The simulation grows its own.

### `[[broadcaster]]`
`id, name, owner?, medium (list), languages (lang ids), region, website?, prov`

### `[[rights]]`
`broadcaster, competition (comp id or text for foreign comps), season, effective_from?, effective_to?,
platforms? (list), languages? (lang ids), note?, prov`

### `[[programme]]`
`id, name, operator (text or assoc/club id), kind (grassroots|talent_id|school|residential|coach_dev|women|
festival|infrastructure), ages? (text, "6-12"), region (national or state ids), active_from?, active_to?,
description (one line), prov`

### `[[partnership]]`
`id, indian (list of ids), foreign (list of { name, nation, kind }), start?, end?,
status (active|expired|unknown), purpose (one line),
components (list: player_development|coach_development|exchange|technical|commercial|scouting|academy|women),
prov`

### `[[rule]]`
`id, competition? (comp id), topic (foreign_players|registration|homegrown|u21|u22|u23|eligibility|
state_eligibility|national_eligibility|squad_size|transfer_window|loan|salary_cap|licensing|
promotion|substitutions), statement (one sentence), params? (inline table of numbers/bools),
season?, effective_from?, effective_to?, prov`

### `[[team]]`
`id, name, kind (national|state_representative|institutional), gender, age (senior|u23|u20|u17|...),
association (assoc id), competitions? (list of text/comp ids), eligibility? (text), prov`

### `[[licence]]`
`id, name, body, order (int, 1 = entry), prerequisite? (licence id), min_age?, requirement? (text), grants? (text), prov`

### `[[grade]]`
`id, name, body, order, scope (district|state|national|international), requirement?, prov`

### `[[rivalry]]`
`id, name?, a, b (club or state ids), kind (derby|rivalry), basis (one line), prov`

### `[[sponsorship]]`
`entity (id), sponsor (text), kind (title|principal|kit|shirt|sleeve|broadcast|naming|official_partner),
season?, effective_from?, effective_to?, prov`

### `[[language]]`
`id, code (ISO 639-1 or -3), name, script?, prov`

### `[[term]]`
`id, concept, canonical, synonyms? (list), register (formal|neutral|casual|broadcast|headline),
region? (state ids or "national"), lang (lang id), constraints? (one line), prov`
