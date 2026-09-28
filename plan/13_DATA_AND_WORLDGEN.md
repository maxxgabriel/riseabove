# 13 — Data, World Generation, Editor & Licensing

## 1. Data strategy

- **Default world is fictional and procedurally generated** from real-world *structures* (nations, confederations, league pyramids, calendar types, rule profiles) and statistical profiles (talent density, height distribution, wage levels). No real player names, club names, logos or likenesses ship by default.
- **User data packs** can supply names/clubs; the game provides import formats and validation but no scraped or licensed databases. Users are responsible for what they import.
- FM15/FM23 are used only as **behavioural references** for calibration (aggregate distributions: goals/match, transfer volumes, age curves) — never as shipped data.

## 2. Data pack format

```
data/
  core/
    nations.toml, confederations.toml, regions/*.toml
    competitions/*.toml         # formats, rules refs, calendars
    rules/*.toml                # registration, labour, eligibility, discipline profiles
    attributes.toml             # attribute list, groups, CA weights per position
    age_curves.toml, injuries.toml, weather.toml, economy.toml, tax.toml
    names/<culture>.toml        # first/last name pools with frequency
    templates/news/*.ftl, templates/commentary/*.ftl
  mods/<mod_id>/ (same structure, override/extend)
```

Pack manifest: id, version, dependencies, load order, checksum. Validation: schema check, referential integrity, rule consistency (e.g. every league has a promotion/relegation link consistent with neighbours).

## 3. World generation pipeline

1. **Pick world size** (Small / Medium / Large / Custom) and active nations/leagues.
2. **Nations**: from core data; economy & talent profiles.
3. **Clubs**: per league tier, generate clubs with names (culture-based generators: city + suffix patterns), colours, stadium, history seed (founding year, honours distribution consistent with reputation), finances scaled by tier & nation economy, ownership type.
4. **Reputation & strength seeds**: power-law within leagues (a few dominant clubs), noise for variety.
5. **People**: generate players per club squad (age distribution 17–36, positions by formation needs, CA distribution by club reputation, PA headroom by age), managers & staff (personalities from culture distributions), agents & agencies, referees, journalists.
6. **Youth pool**: grassroots & academy players across nations by talent density.
7. **History bootstrap**: simulate N past seasons headless (default 3) quickly so tables, reputations, records, contracts and relationships are "lived-in" at game start.
8. **Validation**: squad size rules, registration quotas satisfied, finances non-negative, positional coverage, competitions schedulable.
9. **Protagonist insertion**: created per 09 §1 into a club/grassroots slot; world unchanged otherwise.

## 4. Name & identity generation

- Culture pools (first names, surnames, frequency weights), compound surnames, nicknames rules, mononym probability for certain cultures (data), dual-heritage names from family nationality mixes.
- Uniqueness checks to avoid duplicates within the same club/nation generation.
- Face generator uses ethnicity/culture distributions per region in a respectful, non-stereotyped way (diverse, data-driven).

## 5. Editor (in-game, optional)

- Edit clubs, players, competitions before starting; flags save as "edited".
- During play: disabled unless "sandbox" mode.
- Tools: bulk import (CSV/JSON), rule profile editor, league pyramid editor with live validation.

## 6. Localisation of data

Names and templates localised; club names can have local and international variants; competition names per locale.

## 7. Licensing & legal checklist

| Item | Position |
|------|----------|
| open-football code | Apache-2 — keep NOTICE, attribute, track modifications |
| OpenFootManager | GPL-3 — do **not** copy code; reimplementing ideas/algorithms from scratch is fine; document clean-room process |
| Real names/likeness | Not shipped; user packs only |
| Club crests/kits | Procedural only |
| Fonts/icons | Open licences (OFL/Apache/MIT) |
| Third-party crates | License audit in CI (`cargo deny`) |
| FM15/FM23 | Reference behaviour only; no extracted assets or database content redistributed |
