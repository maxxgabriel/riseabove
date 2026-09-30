# Shared status (major decisions only; newest at the bottom)

## 2026-09-29 21:58Z  Agent A
- Save format: schema 5 is the first supported format; `World::ext` is a versioned envelope (ext layout 2). Additive domains use `append_default` steps.
- India tuning (recognition weights/thresholds, scouting, calendar, export markets) lives in `data/worlds/india/pack.toml` and `Scenario` (`w.ext.scenario`), not in code.
- Recognition is organisation-specific (`w.ext.recog.acquaint`); coach vouches have causes (`w.ext.recog.vouch`); export regard is per market x segment.
- Branch of record for Agent A: `local/pathway-integration`.

## 2026-09-30  Agent A
- Pushed `local/pathway-integration` (India recognition/pathway/export engine, UI pages `Development` + pathway panel on player pages, tests `crates/pw-cli/tests/india_ecosystem.rs`). Agent B's data-only branch will be merged in at the final push.

- 2026-09-29T22:06:00Z [B] Agent B works on branch claude/india-real-world-data (from sim/expansion-audit): researched India reference data under data/worlds/india/ (schema: SCHEMA.md), plus validators. Not touching sim architecture.
- 2026-09-29T22:08:57Z [B] Research stopped on user request. Data under data/worlds/india/ is mostly general-knowledge baseline (q C/D) except state_leagues/wb.toml. Validator: tools/validate_india_reference.py. Not modifying Rust.

## 2026-09-30  Agent A
- Merged into `local/pathway-integration`: Agent B data branches and the `ui/pathway-app` UI branch. Clean-checkout suite green (423). Known: the shared working tree at `E:\pers\riseabove` holds another process's uncommitted edits (wage-scale rework in `market.rs`, `pw-import` database module, `pw-view` database methods); with the wage rework in place `tactics::observation_is_one_thing...` fails (it passes on committed head).
- 2026-09-30T02:51:58Z [B] Pushed claude/india-data-a: UT associations, team structures, rule placeholders, +30 universities; validator 0 errors.
