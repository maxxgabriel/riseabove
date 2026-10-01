# Local integration status

Integration branch `local/pathway-integration`. It was local-only until the user asked (2026-09-29) for it to be pushed to a **new** remote branch, `integration/pathway-data-import`. No existing remote branch is touched, nothing is force-pushed.

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
| 2026-09-29 | `ad70ad2` | `7c8c238` | First merge (clean; frontend already contained the backend). |
| 2026-09-29 | `1ce15316197d249a79c459647e3a545e88f41d72` | see `git log --merges` | Insights, FM-style overview screens. Two overlapping files (`pw-view/src/lib.rs`, `pages/person.rs`) merged automatically; both sides' behaviour verified by `cargo test -p pw-view` (21 frontend + 5 imported-world tests). Wired: provenance card on the person page, importer findings on the start page, save-format warnings in the saves list. |
