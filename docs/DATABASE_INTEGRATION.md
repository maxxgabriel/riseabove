# Local database integration

The consolidated pack is now installed at `database/current` and is the app's default import choice.
See [CONSOLIDATED_DATABASE.md](CONSOLIDATED_DATABASE.md) for current coverage, matching rules, cleanup and verification.
The older acquisition paths below have moved into that pack's `provenance/` folder.

The game reads local datasets at runtime. `archive/`, `fm23_extracted/`, extracted binaries and their contents remain ignored by Git.
Source browsing is separate from simulation state; CSV history and its indexes are never embedded in a world save.

## Coverage

| Source | Available in the game | Limits |
| --- | --- | --- |
| Transfermarkt archive | Playable import of current tracked clubs/players; source browser exposes all 13 CSV tables, including appearances, events, lineups, valuations and transfers | Stale and unresolved records remain source records. The archive does not contain a complete league pyramid or player attributes. |
| FM23 usable exports | All 48 CSV exports can be browsed and searched by their actual exported names or verified source IDs | These are names and extraction ledgers. Player/staff/contract binary schemas and club-to-league relationships are still undecoded. Candidate IDs do not establish identity. |
| Downloaded public FM23 player export | 189,252 unique FM UIDs, birth dates, positions, attributes, wage strings, club and division labels, available in the source browser | A June 2022 snapshot inferred from ages/DOBs. No accompanying club/competition UID tables, contract end dates or PA. Connecting it does not add simulated players. |
| CSV packs | The existing playable pack importer plus the same source browser | Supplied facts and missing fields keep their existing provenance. |

The local files contain 7,506,407 archive rows and 6,221,995 FM export rows. The latter include duplicate exports and language tables;
these counts describe source rows, not unique playable entities. No malformed rows were found in the complete browser index scan.

Importing an archive automatically connects its source folder and a sibling `fm23_extracted/` folder when present. Existing saves can
connect source folders through **World → Database**. The source browser also works before opening a world. Raw source querying is
restricted to the omniscient observer when a world is open; public and inhabited perspectives cannot query it.

## Missing facts

* Blank source fields are `Unknown`. A recorded zero remains zero.
* Missing current market value: carry forward the latest valid valuation for the **same player ID**, dated no later than world start
  and at most 365 days old. Conflicting values on the latest date stay unresolved. The carried value is labelled **Inferred**.
* Without a usable valuation, world building uses the existing public player valuation model and labels the value **Inferred**.
  This is a simulation starting estimate, not a recovered price from the source.
* Missing position: at least three recognized, dated lineups in the same role at the stated current club within two years, with
  that role a strict majority of the recognized lineups. The role is **Inferred**. Future lineups cannot supply roles or shirts.
* Career appearances/goals: only observed non-future appearances contribute. An absent goal field does not become zero.
  No appearances in a partial dataset does not establish zero minutes. Unsupported past career totals display as unavailable.
  When an appearance omits its date, the date can be recovered from its exact game ID; conflicting game dates stay unresolved.
* Known zero wages are preserved during assembly. Missing wages retain the existing labelled estimation rule.
* Missing birth dates and unverified identity/club links are not guessed. Unresolved source rows remain accessible in the browser.
* FM names are never merged into Transfermarkt people or clubs by name or coincidentally equal numeric IDs.

Source records link to world profiles only through their **source namespace and source ID**. Related history links use exact source
IDs. The underlying source values remain visible, including fields that were unsupported by the playable importer.
Starting estimates and recovered positions are applied to new imports. Connecting sources to an established save provides access
to its source records; it does not overwrite the save's played history, attributes or original import provenance.

## Indexing and memory

Tables are indexed lazily. A streaming CSV scan builds row byte offsets, exact ID dictionaries, compact sorted ID postings and a
name index where actual name columns exist. Queries seek only requested rows; pages contain at most 100 records. Multi-million-row
UUID primary keys that have no useful relationships are not indexed. CSV quotes, multiline fields, CP1252 staff exports, blank
fields and additional columns are preserved.

Indexes live in the app data directory's `database-indexes/`; connected paths live in `database-sources.json`. Indexes are disposable:
source size/mtime or cache-version changes invalidate them, and damaged caches rebuild from the original CSV. A source changing
during a scan or query produces a retry error. Cache write failures do not make source records unavailable.

The catalog evicts older indexes toward a 96 MiB budget. The active index remains available if it exceeds that budget; this is not
a hard process memory cap. Temporary allocations during construction and the world's own memory are additional. Connecting a
folder does not eagerly index every table. The database mutex is separate from the world mutex, so a source scan does not block
Continue. World references are resolved only for the current page.

## Command line and review

Release indexing on the local files, during concurrent compilation:

| Source | Cold scan of every table | Restart using disk indexes | Largest active index |
| --- | ---: | ---: | ---: |
| Archive, 13 tables | 32.385 s | 3.357 s | Lineups: 117.3 MB after construction, 117.7 MB after cache reload |
| FM23, 48 tables | 7.863 s | 1.409 s | Localized strings: 54.4 MB after construction |

These totals deliberately open **every** table. Normal browsing opens only the selected one. Subsequent one-row archive reads
measured 0.2–34 ms in this run. Timings depend on disk/cache and system load; they are not an upper latency bound.
The older posting representation used 174.1 MB for appearances and 192.0 MB for lineups; dense key postings reduce them to
99.2 MB and 117.3 MB. Cached vectors are compacted after decoding to avoid spare-capacity growth on restart.

```powershell
target/release/pathway-sim.exe database archive --all
target/release/pathway-sim.exe database fm23_extracted --all
target/release/pathway-sim.exe database archive --table appearances.csv --column player_id --value SOURCE_ID
```

`--cache DIR` chooses an external cache folder. `--all` verifies row counts and reads both ends of every table, reporting index sizes,
first-query and subsequent-query timings. It does not build a world or run simulation.

Regression coverage includes missing/zero distinctions, dated evidence, conflicting values, source identity, quotes/multiline fields,
pagination, source changes, damaged cache rebuilding, candidate-ID exclusion, remembered paths, world links, observer permissions
and repeatable read-only API responses. `app/e2e/database.mjs` reviews actual browser layouts and source/world navigation using the
existing Playwright dependency. Set `BASE`, `CHROMIUM`, `ARCHIVE`, `FM`, `SHOT_DIR`; optionally `IMPORT_WORLD=1` on a disposable session.

Full FM23 playable integration remains dependent on a validated desktop binary schema. Making the exported names available does
not resolve those missing relationships or establish a real playable FM23 world.

## Online acquisition, 30 September 2026

Downloaded the `fm2023.csv` file from [Furkan Uluta's public Kaggle dataset](https://www.kaggle.com/datasets/furkanuluta/football-manager-22-complete-player-dataset)
without requiring the user to export anything. It contains 189,345 rows, all with UIDs and parseable birth dates. Its 93 repeated
UIDs are identical across all fields. The normalized catalog keeps 189,252 records and every original field, trims outer whitespace,
uses the actual UID as `id`, and adds an ISO birth date and original source-row number. It does not merge people by name.

Local files live outside this repository at `E:/pers/riseabove-datasets/online/fm23-furkan/`. The original download and its hash,
publisher metadata, normalization script, manifest and source catalog are retained there. No downloaded game records or converter
are committed. The publisher reports an unknown license; this acquisition does not establish permission to redistribute the game data.

The catalog is connected to the disposable review server on port 8789. Its exact-ID lookup for Haaland (`29179241`) returns one
record; the full table reports 189,252 rows and zero malformed records. Player exports now have a distinct source-browser label
instead of the old names-only treatment. The focused catalog regression and server build pass.

The larger export is not the full approximately 450,000-player FM23 database. Its 2022-06-27 snapshot is inferred from the common
intersection of supplied ages and DOBs, rather than a stated publisher snapshot date. Wage currency and period, transfer-value ranges
and `Not for Sale` remain source strings. Club/division labels are not treated as verified entity IDs, and none of these records have
been added to an existing played world. The earlier local binary-schema limitation still applies; this separate CSV provides actual
player fields for a future adapter.

Other acquired candidates were smaller or obsolete: the Siddhraj export has 91,672 rows / 87,163 UIDs; Platinum has 8,452 UIDs;
the advertised 150,000+ Ajinkya dataset is explicitly FM2017. They are kept separate and are not silently merged into this snapshot.
