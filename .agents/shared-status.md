# Shared status (major decisions only; newest at the bottom)

## 2026-09-29 21:58Z  Agent A
- Save format: schema 5 is the first supported format; `World::ext` is a versioned envelope (ext layout 2). Additive domains use `append_default` steps.
- India tuning (recognition weights/thresholds, scouting, calendar, export markets) lives in `data/worlds/india/pack.toml` and `Scenario` (`w.ext.scenario`), not in code.
- Recognition is organisation-specific (`w.ext.recog.acquaint`); coach vouches have causes (`w.ext.recog.vouch`); export regard is per market x segment.
- Branch of record for Agent A: `local/pathway-integration`.
