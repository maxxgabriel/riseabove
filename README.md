# Rise Above (codename Pathway)

Football Manager, but you manage one player's life instead of a club — with a full autonomous football
world underneath and a life simulation on top.

- The world runs itself at FM scale (target ~300,000 simulated players): clubs, boards, finances, scouting,
  transfers, loans, contracts, youth intakes, managers hired and sacked, leagues, cups and continental
  competitions, promotion and relegation.
- You control one ordinary player. The world never bends for you: every rule that applies to you applies to
  every AI player.
- Depth comes from interacting systems with memory — training, fitness, perception, relationships, promises,
  negotiations, media, money, family — not scripted content. There is no linear arc and no end screen.

**Status:** early development. See [`PROGRESS.md`](PROGRESS.md) for what is built, what is next, and what is
verified. Design documents live in [`plan/`](plan/) and [`foundation/`](foundation/).

## Layout

| Path | Contents |
|---|---|
| `crates/pw-core` | ids, dates, deterministic RNG and math, attributes, positions, roles, tactics |
| `crates/pw-data`, `data/engine/` | engine rules as data (formations, CA weights, age curves, injuries, calendars, tuning) |
| `crates/pw-world` | world state sized for hundreds of thousands of players |
| `crates/pw-match` | match interface; adapter for the vendored engine and a native engine |
| `crates/pw-sim` | the daily world pipeline and all world systems |
| `crates/pw-import` | world import from a user-provided export (`data/IMPORT_FORMAT.md`) |
| `crates/pw-cli` | headless runner (`pathway-sim`) |
| `crates/pw-career` | the player-career layer (in progress) |
| `vendor/ofm-engine` | OpenFootManager's match engine, unmodified (GPL-3) |
| `fm23_dat_tool.py` | read-only inspector/extractor for a locally installed FM23 database |

No game data is included. Worlds are built from data you supply yourself.

## Build

```
cargo run --release -p pw-cli --bin pathway-sim -- synth small --days 365
cargo run --release -p pw-cli --bin pathway-sim -- import <folder> --days 365 --save world.pws
```

## License

Pathway's own code is licensed under Apache-2.0 (see `LICENSE`).

`vendor/ofm-engine` is OpenFootManager's match engine, licensed under GPL-3.0 (see
`vendor/ofm-engine/LICENSE.md`). It is the default match backend, so **binaries built with it are GPL-3.0 as a
whole**. The native engine (`matches.backend = "native"`) does not use it.

Football Manager is a trademark of Sports Interactive. This project is not affiliated with or endorsed by
Sports Interactive or SEGA, and contains none of their data.
