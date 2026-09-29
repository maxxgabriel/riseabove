# Documentation map

Read in this order of authority:

| Layer | Where | Answers |
| --- | --- | --- |
| **Locked design** | `design/LOCKED_DESIGN_DECISIONS.md` | what the game is meant to mean |
| **Code and tests** | `crates/`, `app/` | what is actually implemented |
| **Implementation status** | `IMPLEMENTATION_STATUS.md` | the current gap between design and code |
| **ADRs** | `adr/` | why important architectural choices exist |
| **Historical reports** | `INTEGRATION_REPORT.md`, `FINAL_DEPTH_PASS.md`, `WORLD_SYSTEMS.md`, `DB_INTEGRATION_AUDIT.md` | what was true at an earlier commit |

Rules: a stale report does not make working code wrong; never build a "missing" feature just because an old report lists it as
missing; check the branch first. Documentation never overrules a failing test.

Data and imports: `DB_INTEGRATION_AUDIT.md` (sources and mappings), `../data/IMPORT_FORMAT.md` (pack format),
`DATA_PACK_BOUNDARY.md` (what may be committed: nothing proprietary). Branch bookkeeping: `LOCAL_INTEGRATION_STATUS.md`.
