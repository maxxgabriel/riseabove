# Data-pack boundary

What ships in this repository, and what a world's data pack may bring in.

## Never in the repository

- **Lyrics of football songs**, including terrace adaptations of popular songs, and any melody. Chants
  are original structures built by `pw-narrate/src/social.rs::chant`, filled with names from the world.
  No chant references or imitates an existing song.
- **Proprietary game data.** No Football Manager (or any other game's) database, attributes, ratings,
  person or club records, and no files derived from them. Pathway loads a *user-made export* at runtime
  from the user's own directory (`data/IMPORT_FORMAT.md`). The converter lives outside this repository,
  and so do the exported files.
- **Club artwork**: crests, kits, logos, sponsor marks, stadium images, photos of people.
- **Unlicensed historical datasets**: past league tables, scorer lists, attendances or match records
  copied from sources the user has no right to redistribute.

## What ships

- **Engine rules and tuning** (`data/engine/*.toml`): formations, position weights, age curves,
  injuries, calendars, rule profiles and tuning. These are simulation rules written for this project.
- **A synthetic world** for tests and benchmarks, generated in code.
- **Word lists for narration**: common football vocabulary, which no one owns, and original phrasing.

## What a pack may bring (the user's responsibility)

- The world (`nations.csv`, `competitions.csv`, `clubs.csv`, `players.csv`, optional `staff.csv`),
  converted by the user from data they have the right to use locally.
- Optional `history.csv` of past seasons. Every season loaded from it is marked **imported**. Any
  season not supplied is generated and marked **generated**, and generated history names only
  generated figures, never real people (`pw-sim/src/backfill.rs`, checked by
  `audit::Violation::PastFigureNamedLikeReal`).

## Checks

- A scan of `crates/` and `data/` for well-known club names, song titles and game-data identifiers
  found nothing in code or shipped data (2026-09-28).
- New narration packs must keep to original phrasing. A pack may never caricature a nationality,
  region or group. Dialects change vocabulary only (`lexicon::Dialect`).
