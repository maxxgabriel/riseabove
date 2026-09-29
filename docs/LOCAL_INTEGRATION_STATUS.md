# Local integration status

Local-only branch `local/pathway-integration`. **Never pushed.** No remote tracking is configured.

## Starting SHAs (recorded 2026-09-29)

| Ref | SHA | Role |
| --- | --- | --- |
| `origin/claude/kind-planck-t66j8f` | `401b7aef40112bda1ac95cd046bfdae9616680b6` | Backend/world branch (integration + depth work, integration report). Branch base for local work. |
| `origin/ui/desktop-client` | `ad70ad25b3751da8a0696ffe57b676358d721ef3` | Frontend branch (Tauri desktop client, `pw-view`, `pw-serve`, `pw-narrate`). Already contains the backend tip above. |
| `origin/main` / local `main` | `d19d5f3` | Initial workspace; untouched. |

Local-only inputs present in the working tree (git-ignored, never committed): `fm23_extracted/`, `fm23_test_extract_20260928b/`,
`fm23_dat_tool.py` output. Untracked `plan/UI_PLAN.md` was carried over unchanged.

## Frontend sync log

| Date | Frontend SHA merged | Merge commit | Notes |
| --- | --- | --- | --- |
