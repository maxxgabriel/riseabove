# Consolidated local database — 30 September 2026

The installed pack is `E:/pers/riseabove/database/current`. **Start → Import a dataset** selects it automatically.
The **Database** page defaults to its reconciled player catalog. The native importer, development server and desktop app
use the same pack. `RISEABOVE_DATABASE` can point another installation at it. No downloaded data is tracked in Git.

## Actual coverage

| Layer | Records / scope |
| --- | --- |
| Reconciled identity catalog | 568,260 identities; includes historical and incomplete records |
| Joined player facts view | 568,260 rows, one per catalog identity; 331,527 have sourced facts, including 95,938 `identity_only` players |
| Reep exact-ID biographies | 140,374 profiles; 76,306 `identity_only` players gained sourced nationality, position or height facts |
| FM23 exact UID profiles | 25,526 `identity_only` records with historical position, physical profile and attributes |
| Wikidata exact QID profiles | 5,295 identity-only profile rows; 5,249 full DOBs, 2,108 nationalities, 1,742 positions, 1,309 heights |
| Fact conflicts / remaining identity-only gap | 11,236 cross-source disagreements logged with both values and sources; 235,749 `identity_only` rows still have no sourced profile facts and keep those fields blank |
| FM23 exported player facts | 189,252 unique FM UIDs, plus two smaller dated supplemental exports |
| FM ↔ Transfermarkt verified join candidates | 39,618, with explicit provider-ID evidence and birthday checks where available |
| Rejected identity links | 1,279; retained separately in `identity_conflicts.csv` |
| September squad update | Premier League: 667 source rows, 550 eligible current squad candidates |
| Validated playable import | 21,365 players, 1,435 clubs, 54 competitions, 201 nations, 3,955 staff |
| Verified clubs outside modeled leagues | 887 clubs and 2,563 current-snapshot players; exact Transfermarkt ID → Reep team bridge and supported country |
| OpenFootball World Cup supplement | 1,248 registered players (3 July 2026); 1,104 unique exact name-and-birth-date catalog links |
| Imported history | 164 completed seasons and 58,099 career spells |

These are different counts. Catalog-only people do not enter the simulation or become playable just because they have an ID.
Generated youth/background population is also separate from imported records.
In the seven-day review save, 16,608 records pass the app's inhabitable-player filter; that count changes with the world's state.

The player projection now also admits 2,563 latest-season roster rows at clubs absent from the modeled league set. Their club IDs
have a unique validated Transfermarkt `verein` → active Reep men's team bridge, and their country exactly matches a supported
country. They are playable/searchable under the verified club record; **their competition remains unknown**, so the game does not
invent fixtures or league standings for them. Another 253 candidate club IDs remain out of this supplement because the team link,
country, or men's-team evidence was not sufficient. The generated crosswalk is `verified_unmodeled_clubs.csv`; rerun
`python tools/augment_current_clubs.py database/current` after rebuilding the pack.

**Freshness is mixed.** The identity register is dated 26 September 2026 and PL squads were fetched on 30 September.
Other playable squads retain the archive's July 2026 baseline. Supplemental profiles are from October 2025.
FM23 attributes remain June 2022 historical evidence; they do not replace current inferred ratings, wages or clubs.
The supplemental Reep v0 biography snapshot is dated 21 June 2026 and is joined by exact provider IDs or a current-release
Wikidata ID with `dob-agrees`; v0 DOB values are not exported. Its CC0 data and licence are retained with acquisition hashes.
The separate Wikidata supplement uses current CC0 structured facts only through exact current-release Reep QIDs whose DOB
check agrees. FM23 profile values are joined by exact FM UID and remain labelled historical. The combined
`catalog_player_facts.csv` view preserves field-level source labels and sends disagreements to
`catalog_player_fact_conflicts.csv`.
The World Cup roster is an additional July 3 snapshot, not a 2026/27 global squad refresh.
The source tables preserve missing fields. Simulation estimates keep their existing provenance labels.

The World Cup supplement is browseable as `worldcup_2026_squads.csv` (from [OpenFootball](https://github.com/openfootball/football.json)). Unique catalog links require a normalized full name
and exact date of birth; 23 ambiguous and 121 unmatched rows remain unlinked. Among the 1,104 linked people, 824 are in
the current playable import, 171 point to clubs absent from the club file, 51 point to clubs with no latest-season record,
and 42 have an older activity row. It adds evidence and identity links, not new playable players. The retained source and
CC0 license are in `provenance/openfootball-worldcup-2026/`.

## Identity and conflict rules

* Reep bridges are scoped by provider and namespace. Redirects are resolved; retired bridges and ambiguous mappings are excluded.
* An explicit FM/Transfermarkt bridge contradicted by birthdays is quarantined. Catalog birthday contradictions also stay separate.
* FPL uses stable numeric Opta codes, not season-local element IDs. A fallback match needs a unique normalized full name **and**
  exact birthday. A name without a birthday never establishes identity.
* Conflicting or unmatched FPL identities are kept under a separate `fpl:` namespace. Invalid birthday links are not made playable.
* Current club updates clear an old club's contract. Fantasy prices are never used as football market values.
* Older same-ID profiles can fill absent immutable birthdays. They cannot overwrite a newer roster or contract.
* Source CSVs, reconciliation ledgers and original records remain in the pack. Raw queries require observer mode when a world is open.

## Storage and cleanup

One canonical directory holds the playable projection, reconciled catalog, historical tables and provenance.
Unchanged tables share disk blocks through hardlinks; queries index only the selected table and cap pages at 100 rows.
The full catalog, raw history and source indexes are not embedded in world saves.

Original archive and extracted FM files are under `current/provenance/`. The former `archive/` and `fm23_extracted/` paths
are small compatibility junctions, preserving existing tools and tests. The discarded FM2017 candidate, unused PlayersDB
download, Git LFS pointer and prototype packs were removed. Rejection and download manifests retain their acquisition context.
Undecoded binary files are retained as unique source evidence; their existence does not establish usable relationships.

## Rebuild a new version

```powershell
python tools/fetch_database_sources.py --staging E:/pers/riseabove-datasets/staging-next
python tools/consolidate_database.py --archive archive --fm database/current/provenance/fm23-furkan/catalog --staging E:/pers/riseabove-datasets/staging-next --legacy fm23_extracted/2300_fm/usable --extra-fm "database/current/provenance/fm23-siddhraj/source/merged_players (1).csv" "database/current/provenance/fm23-platinum/source/FM 2023.csv" --output database/build-next
python tools/augment_current_clubs.py database/build-next
python tools/enrich_catalog_from_reep_v0.py database/build-next
python tools/enrich_catalog_from_wikidata.py database/build-next
python tools/enrich_catalog_from_fm23.py database/build-next
python tools/build_catalog_player_facts.py database/build-next
python tools/validate_consolidated_database.py database/build-next
```

Inspect and load the new version before replacing `current`. Refreshing a source never overwrites an established career.
The builder refuses an existing output folder; input checksums and an explicit manifest keep versions reviewable.

## Verification

* All 89 manifest-listed CSV tables passed byte/hash and structural checks. Joined profile tables have unique entity IDs and no links outside the catalog.
* Four fictional Python reconciliation tests pass: birthday conflicts, name-only exclusion, zero preservation/contract clearing, checksum rejection.
* Two Rust import regressions pass, including filtering future evidence against the configured start **before** parsing the archive.
* Smoke: 29 tests passed. Full: 429 passed, 21 existing skips; all-target check and three-seed short simulations completed.
* Actual pack imported successfully: 21,365 players and 1,435 clubs (including the 887 verified no-league clubs); advanced seven days and saved/reloaded at 7 October 2026.
  The review save has 44,977 total simulated player records, including generated background/youth, and is 28,817,860 bytes.
* Browser checks passed: installed pack selection, responsive start, catalog reconciliation, profile/source links and observer-only access.
  Screenshots/logs are in `%TEMP%/riseabove-consolidated-review/`. Frontend production build passes.
* After consolidation and cleanup, all three real-archive tests passed through the compatibility junction.
* Native desktop build passes with `--features custom-protocol --locked`, embedding the production frontend.
  Launch `app/src-tauri/target/debug/pathway-app.exe` from this checkout.

The short simulations still flag existing club-balance and save-growth problems. This database consolidation does not resolve those
balance issues or establish a complete, current roster for every competition worldwide.
