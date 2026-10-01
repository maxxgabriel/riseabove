"""Build a local, versioned Rise Above database with Python's standard library.

No name-only merges. Provider IDs are namespace scoped; contradictory birthdays
quarantine a crosswalk. Source rows remain available, including unresolved ones.
The native importer reads the playable archive projection, not the entire census.
Usage: python tools/consolidate_database.py --archive archive --fm C:/.../catalog
       --staging E:/.../staging-20260930 --output database/build-20260930
"""
from __future__ import annotations

import argparse
import collections
import csv
import datetime as dt
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import unicodedata

VERSION = 1
BASE_DATE = "2026-07-06"
FM_DATE = "2022-06-27"


def rows(path: Path):
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt", encoding="utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream)


def write_rows(path: Path, fields, records):
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(records)


def digest(path: Path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def iso(value):
    try:
        return dt.date.fromisoformat(str(value)[:10]).isoformat()
    except (TypeError, ValueError):
        return ""


def name_key(value):
    # Normalization is used ONLY with an exact birthday and a unique result.
    value = re.sub(r"\s*\(\d+\)$", "", value or "")
    return "".join(c for c in unicodedata.normalize("NFKD", value.casefold())
                   if c.isalnum() and not unicodedata.combining(c))


def unique_index(records, field):
    out = {}
    for record in records:
        key = record[field].strip()
        if not key:
            raise ValueError(f"missing {field}")
        if key in out and record != out[key]:
            raise ValueError(f"conflicting duplicate {field}={key}")
        out[key] = record
    return out


def link_or_copy(source: Path, target: Path):
    # Unchanged files share disk blocks; neither path is modified after linking.
    try:
        os.link(source, target)
    except OSError:
        shutil.copy2(source, target)


def build(archive: Path, fm_root: Path, staging: Path, output: Path, legacy: Path | None = None, extra_fm=()):
    if output.exists():
        raise ValueError(f"Output already exists: {output}; build a new version")
    release = json.loads((staging / "reep-release.json").read_text(encoding="utf-8-sig"))
    fpl = json.loads((staging / "fpl-bootstrap.json").read_text(encoding="utf-8-sig"))
    observed = json.loads((staging / "acquisition.json").read_text(encoding="utf-8-sig"))["observed_on"]
    start = dt.date.fromisoformat(observed)
    if start < dt.date.fromisoformat(BASE_DATE):
        raise ValueError("Snapshot predates archive")
    checked = {}
    for name in ("players", "teams", "competitions", "coaches", "redirects", "bridges", "overlay_links"):
        path = staging / "reep" / f"{name}.csv.gz"
        entry = release["files"][f"csv/{name}.csv.gz"]
        actual = digest(path)
        if actual != entry["sha256"] or path.stat().st_size != entry["bytes"]:
            raise ValueError(f"Reep checksum/size mismatch: {name}")
        checked[name] = actual
    v0_path = staging / "reep-v0-people.csv"
    v0_acquisition = json.loads((staging / "reep-v0-people.acquisition.json").read_text(encoding="utf-8-sig"))
    if v0_acquisition.get("license") != "CC0-1.0" or digest(v0_path) != v0_acquisition.get("sha256"):
        raise ValueError("Reep v0 biography source checksum/licence mismatch")
    v0_license = staging / "reep-v0-LICENSE.txt"
    if (not v0_license.exists() or digest(v0_license) != v0_acquisition.get("license_sha256")):
        raise ValueError("Reep v0 CC0 licence text is missing or does not match acquisition metadata")

    output.mkdir(parents=True)
    source_dir = output / "provenance"
    source_dir.mkdir()
    shutil.copy2(staging / "reep-release.json", source_dir / "reep-release.json")
    shutil.copy2(staging / "acquisition.json", source_dir / "acquisition.json")
    shutil.copy2(staging / "fpl-bootstrap.json", source_dir / "fpl-bootstrap.json")
    for name in ("LICENSE.txt", "schema.json"):
        if (staging / name).exists():
            shutil.copy2(staging / name, source_dir / f"reep-{name}")
    # Keep the frozen biographical Reep v0 source, its licence and the current
    # v1 Wikidata overlay together. The enrichment step joins IDs across versions.
    for name in ("reep-v0-people.csv", "reep-v0-people.acquisition.json", "reep-v0-LICENSE.txt",
                 "reep-v1-overlay-links.acquisition.json"):
        if (staging / name).exists():
            shutil.copy2(staging / name, source_dir / name)
    overlay_path = staging / "reep" / "overlay_links.csv.gz"
    if overlay_path.exists():
        shutil.copy2(overlay_path, source_dir / "reep-v1-overlay-links.csv.gz")
    for path in staging.glob("salimt-*.metadata.json"):
        shutil.copy2(path, source_dir / path.name)
    for name in ("manifest.json", "kaggle-metadata.json"):
        path = fm_root.parent / name
        if path.exists():
            shutil.copy2(path, source_dir / f"fm23-{name}")

    tm = unique_index(rows(archive / "players.csv"), "player_id")
    fm = unique_index(rows(fm_root / "players.csv"), "id")
    profiles = unique_index(rows(staging / "salimt-player_profiles.csv"), "player_id")
    clubs = unique_index(rows(archive / "clubs.csv"), "club_id")
    fm_keys = set(fm)
    # Compact crosswalk subsets, indexed by actual provider IDs. Match bridges
    # remain in a compressed source artifact; no 8-million-row RAM dictionary.
    redirects = {r["from_id"]: r["to_id"] for r in rows(staging / "reep/redirects.csv.gz")}

    def survivor(key):
        seen = set()
        while key in redirects:
            if key in seen:
                raise ValueError("Reep redirect cycle")
            seen.add(key)
            key = redirects[key]
        return key

    desired = {("transfermarkt", "spieler"), ("transfermarkt", "verein"),
               ("transfermarkt", "wettbewerb"), ("fm", "player"),
               ("opta", "person_numeric"), ("opta", "team_numeric")}
    bridge = collections.defaultdict(dict)
    ambiguous = set()
    links = []
    for row in rows(staging / "reep/bridges.csv.gz"):
        namespace = (row["provider"], row["namespace"])
        if namespace not in desired or row.get("upstream_status") == "retired":
            continue
        key, entity = row["external_id"], survivor(row["reep_id"])
        if not entity:
            continue
        old = bridge[namespace].get(key)
        if old and old != entity:
            ambiguous.add((*namespace, key))
        bridge[namespace][key] = entity
        links.append({**row, "reep_id": entity})
    for provider, namespace, key in ambiguous:
        del bridge[(provider, namespace)][key]
    links = [r for r in links if (r["provider"], r["namespace"], r["external_id"]) not in ambiguous]
    tm_bridge = bridge[("transfermarkt", "spieler")]
    fm_bridge = bridge[("fm", "player")]
    tm_by_entity = collections.defaultdict(list)
    for key, entity in tm_bridge.items():
        if key in tm or key in profiles:
            tm_by_entity[entity].append(key)
    conflicts = []
    matches = []
    for fm_id in sorted(fm_keys):
        entity = fm_bridge.get(fm_id)
        candidates = tm_by_entity.get(entity, [])
        if len(candidates) != 1:
            continue
        tm_id = candidates[0]
        tm_record = tm.get(tm_id) or profiles[tm_id]
        tm_dob = iso(tm_record.get("date_of_birth"))
        fm_dob = iso(fm[fm_id]["birth_date"])
        if tm_dob and fm_dob and tm_dob != fm_dob:
            conflicts.append(dict(kind="birthday_conflict", source="FM23/Reep/Transfermarkt",
                                  source_id=fm_id, target_id=tm_id,
                                  detail=f"{fm_dob} != {tm_dob}"))
            continue
        matches.append(dict(fm_id=fm_id, player_id=tm_id, entity_id=entity,
                            basis="provider_crosswalk" + ("+exact_birth_date" if tm_dob else ""),
                            attribute_snapshot=FM_DATE))
    print(f"FM ID reconciliation: {len(matches)} matches; {len(conflicts)} birthday conflicts", flush=True)

    # Older profiles fill immutable missing birthdays only on the SAME TM ID;
    # club/contract/value fields from 2025 cannot overwrite July/September 2026.
    fills = 0
    for key, row in tm.items():
        profile = profiles.get(key, {})
        old_dob, supplement = iso(row.get("date_of_birth")), iso(profile.get("date_of_birth"))
        if old_dob and supplement and old_dob != supplement:
            conflicts.append(dict(kind="birthday_conflict", source="Transfermarkt profiles",
                                  source_id=key, target_id=key, detail=f"{old_dob} != {supplement}"))
        elif not old_dob and supplement:
            row["date_of_birth"] = supplement
            row["identity_source"] = "transfermarkt-profiles-2025"
            fills += 1

    # FPL person code is Opta's numeric identity, not its season-local element ID.
    names = collections.defaultdict(set)
    for key, row in {**profiles, **tm}.items():
        birth = iso(row.get("date_of_birth"))
        if birth:
            for label in (row.get("name"), row.get("player_name"),
                          f"{row.get('first_name', '')} {row.get('last_name', '')}"):
                if label and name_key(label):
                    names[(name_key(label), birth)].add(key)
    club_by_entity = collections.defaultdict(list)
    for key, entity in bridge[("transfermarkt", "verein")].items():
        club_by_entity[entity].append(key)
    team_map = {}
    for team in fpl["teams"]:
        entity = bridge[("opta", "team_numeric")].get(str(team["code"]))
        candidates = club_by_entity.get(entity, [])
        if len(candidates) != 1:
            raise ValueError(f"Current PL team has no unique ID mapping: {team['name']}")
        team_map[team["id"]] = candidates[0]
        key = candidates[0]
        row = clubs.setdefault(key, {k: "" for k in next(iter(clubs.values()))})
        row.update(club_id=key, name=team["name"], domestic_competition_id="GB1",
                   last_season="2025", club_source="fpl", club_observed_on=observed)

    latest_season = max(int(c["last_season"]) for c in clubs.values() if c["last_season"])
    current_teams = set(team_map.values())
    # Demoted teams remain in the catalog. Their new league isn't guessed.
    for key, row in clubs.items():
        if row.get("domestic_competition_id") == "GB1" and key not in current_teams:
            row["last_season"] = str(latest_season - 1)
    current_player_ids = set()
    fpl_rows = []
    fpl_mapped = fpl_new = 0
    identity_conflicts = set()
    for element in fpl["elements"]:
        birth = iso(element.get("birth_date"))
        code = str(element["code"])
        entity = bridge[("opta", "person_numeric")].get(code)
        candidates = tm_by_entity.get(entity, [])
        basis = "provider_crosswalk+exact_birth_date"
        if len(candidates) != 1:
            candidates = sorted(names.get((name_key(f"{element['first_name']} {element['second_name']}"), birth), set())) if birth else []
            basis = "unique_name+exact_birth_date"
        key = candidates[0] if len(candidates) == 1 else ""
        original = tm.get(key) or profiles.get(key) or {}
        old_birth = iso(original.get("date_of_birth"))
        if key and (not birth or not old_birth or birth != old_birth):
            identity_conflicts.add(code)
            conflicts.append(dict(kind="fpl_identity_conflict", source="FPL", source_id=code,
                                  target_id=key, detail=f"birthdays {birth!r} / {old_birth!r}"))
            key = ""
        if not key:
            key = f"fpl:{code}"
            entity = f"fpl:{code}"
            basis = "independent_provider_id"
            fpl_new += 1
        else:
            fpl_mapped += 1
        fpl_rows.append(dict(player_id=key, opta_id=code, entity_id=entity or f"tm:{key}",
                             name=f"{element['first_name']} {element['second_name']}",
                             dob=birth, club_id=team_map[element["team"]], basis=basis,
                             removed=element.get("removed", False), status=element.get("status"),
                             observed_on=observed, minutes=element.get("minutes"),
                             goals=element.get("goals_scored"), assists=element.get("assists")))
        if element.get("removed") or element.get("status") == "u" or not birth or code in identity_conflicts:
            continue
        if key in current_player_ids:
            raise ValueError(f"Two current FPL rows resolve to the same person: {key}")
        current_player_ids.add(key)
        row = tm.setdefault(key, {k: "" for k in next(iter(tm.values()))})
        old_club = row.get("current_club_id")
        changed_club = bool(old_club and old_club != team_map[element["team"]])
        row.update(player_id=key, first_name=element["first_name"], last_name=element["second_name"],
                   name=f"{element['first_name']} {element['second_name']}", date_of_birth=birth,
                   last_season=str(latest_season), current_club_id=team_map[element["team"]],
                   current_club_domestic_competition_id="GB1", identity_source="fpl",
                   club_source="fpl", club_observed_on=observed)
        # Contracts and loan data refer to their club. Do not transfer them to a new one.
        if changed_club:
            row["contract_expiration_date"] = ""
            row["agent_name"] = ""
        if not row.get("position"):
            row["position"] = {1: "Goalkeeper", 2: "Defender", 3: "Midfield", 4: "Attack"}.get(element["element_type"], "")
    # FPL is partial for youth, but the current playable PL projection uses its
    # verified adult/registered pool. Absent players retain dated catalog records.
    for key, row in tm.items():
        if row.get("current_club_id") in current_teams and key not in current_player_ids:
            row["last_season"] = str(latest_season - 1)
    print(f"FPL: {fpl_mapped} mapped, {fpl_new} separate IDs, {len(current_player_ids)} current eligible rows", flush=True)

    # Native archive projection and all original history; store source provenance
    # beside it. Modified identity tables are new files, never hardlinked.
    for path in archive.glob("*.csv"):
        if path.name not in ("players.csv", "clubs.csv"):
            link_or_copy(path, output / path.name)
    with (archive / "players.csv").open(encoding="utf-8-sig", newline="") as stream:
        player_fields = next(csv.reader(stream))
    player_fields += ["identity_source", "club_source", "club_observed_on"]
    write_rows(output / "players.csv", player_fields, (tm[k] for k in sorted(tm)))
    with (archive / "clubs.csv").open(encoding="utf-8-sig", newline="") as stream:
        club_fields = next(csv.reader(stream))
    club_fields += ["club_source", "club_observed_on"]
    write_rows(output / "clubs.csv", club_fields, (clubs[k] for k in sorted(clubs)))
    (output / "world.toml").write_text(f'start_date = "{observed}"\n', encoding="utf-8")
    link_or_copy(fm_root / "players.csv", output / "fm23_player_records.csv")
    if legacy:
        for path in sorted(legacy.glob("*.csv")):
            link_or_copy(path, output / f"fm23_legacy_{path.name}")
    for index, path in enumerate(extra_fm):
        # Older supplements retain their original headers/duplicate rows; the
        # source browser handles duplicate header names without losing columns.
        link_or_copy(path, output / f"fm23_supplement_{index + 1}_records.csv")
    link_or_copy(staging / "salimt-player_profiles.csv", output / "tm_profile_records_2025.csv")
    link_or_copy(staging / "salimt-team_details.csv", output / "tm_team_records_2025.csv")
    link_or_copy(staging / "salimt-player_latest_market_value.csv", output / "tm_valuation_records_2025.csv")
    # Preserve the unmodified original identity tables for every superseded fact.
    link_or_copy(archive / "players.csv", output / "tm_player_records_20260706.csv")
    link_or_copy(archive / "clubs.csv", output / "tm_club_records_20260706.csv")
    write_rows(output / "fm_tm_matches.csv", ["fm_id", "player_id", "entity_id", "basis", "attribute_snapshot"], matches)
    write_rows(output / "fpl_current_squad.csv", ["player_id", "opta_id", "entity_id", "name", "dob", "club_id", "basis", "removed", "status", "observed_on", "minutes", "goals", "assists"], fpl_rows)
    for name in ("players", "teams", "competitions", "coaches", "redirects"):
        with gzip.open(staging / "reep" / f"{name}.csv.gz", "rb") as src, (output / f"registry_{name}.csv").open("wb") as dst:
            shutil.copyfileobj(src, dst)
    # Full provider-ID crosswalk stays compressed: useful for future adapters,
    # never embedded in the world or eagerly expanded on opening the app.
    link_or_copy(staging / "reep/bridges.csv.gz", source_dir / "reep-bridges.csv.gz")

    rejected_fm = {r["source_id"] for r in conflicts if r["source"] == "FM23/Reep/Transfermarkt"}
    catalog = {}
    for record in rows(staging / "reep/players.csv.gz"):
        key = survivor(record["reep_id"])
        if not key or key != record["reep_id"]:
            continue
        catalog[key] = dict(entity_id=key, name=record["label"], gender=record.get("gender", ""),
                            country=record.get("country", ""), status=record["status"],
                            dob="", dob_source="Unknown", tm_id="", fm_id="", club_id="",
                            club_source="Unknown", club_observed_on="", eligibility="identity_only")
    for key, row in sorted({**profiles, **tm}.items()):
        entity = tm_bridge.get(key, f"tm:{key}")
        entry = catalog.setdefault(entity, dict(entity_id=entity, gender="", country="", status="", fm_id=""))
        # Multiple provider IDs are preserved as a list; no arbitrary first ID.
        entry["tm_id"] = ";".join(sorted(set(filter(None, [*entry.get("tm_id", "").split(";"), key]))))
        birth = iso(row.get("date_of_birth"))
        existing = entry.get("dob")
        if existing and birth and existing != birth:
            raise ValueError(f"Canonical birthday conflict for {entity}: {existing} / {birth}")
        entry.update(name=row.get("name") or re.sub(r"\s*\(\d+\)$", "", row.get("player_name", "")),
                     dob=birth or existing or "", dob_source=row.get("identity_source") or ("transfermarkt-2026" if key in tm else "transfermarkt-2025"),
                     club_id=row.get("current_club_id", ""), club_source=row.get("club_source") or ("transfermarkt" if key in tm else "transfermarkt-2025"),
                     club_observed_on=row.get("club_observed_on") or (BASE_DATE if key in tm else "2025-10-18"),
                     eligibility="projection_candidate" if key in tm else "historical_profile")
    for fm_id, row in sorted(fm.items()):
        entity = fm_bridge.get(fm_id) if fm_id not in rejected_fm else None
        known = catalog.get(entity, {})
        if known.get("dob") and iso(row["birth_date"]) != known["dob"]:
            conflicts.append(dict(kind="catalog_birthday_conflict", source="FM23/Reep/catalog",
                                  source_id=fm_id, target_id=entity, detail=f"{row['birth_date']} != {known['dob']}"))
            rejected_fm.add(fm_id)
            entity = None
        # A crosswalk contradicted by DOB stays separate, not lost.
        entity = entity or f"fm:{fm_id}"
        entry = catalog.setdefault(entity, dict(entity_id=entity, name=row["name"], gender="", country=row["nat"], status="",
                                               tm_id="", dob="", dob_source="Unknown", club_id="", club_source="Unknown", club_observed_on="",
                                               eligibility="historical_profile"))
        entry["fm_id"] = ";".join(sorted(set(filter(None, [*entry.get("fm_id", "").split(";"), fm_id]))))
        if not entry.get("dob"):
            entry["dob"], entry["dob_source"] = iso(row["birth_date"]), "fm23-2022"
    for link in links:
        link["validation"] = "quarantined" if link["provider"] == "fm" and link["namespace"] == "player" and link["external_id"] in rejected_fm else "provider_claim"
    write_rows(output / "identity_links.csv", ["provider", "namespace", "external_id", "reep_id", "rung", "upstream_status", "validation"], links)
    write_rows(output / "identity_conflicts.csv", ["kind", "source", "source_id", "target_id", "detail"], conflicts)
    for entry in catalog.values():
        keys = entry.get("tm_id", "").split(";")
        entry["player_id"] = keys[0] if len(keys) == 1 and keys[0] in tm else ""
    catalog_fields = ["entity_id", "player_id", "name", "gender", "country", "status", "dob", "dob_source", "tm_id", "fm_id", "club_id", "club_source", "club_observed_on", "eligibility"]
    write_rows(output / "catalog_players.csv", catalog_fields, (catalog[k] for k in sorted(catalog)))
    file_manifest = {p.name: {"bytes": p.stat().st_size, "sha256": digest(p)} for p in sorted(output.glob("*.csv"))}
    manifest = dict(schema_version=VERSION, name="Rise Above consolidated database", built_on=observed,
                    start_date=observed, archive_snapshot=BASE_DATE, fm_snapshot=FM_DATE,
                    registry_stamp=release["stamp"], current_squad_coverage=["Premier League"],
                    freshness_note="PL squads observed on build date; other squads remain the July archive snapshot. FM23 attributes are historical only.",
                    catalog_players=len(catalog), fm_records=len(fm), fm_tm_matches=len(matches),
                    identity_conflicts=len(conflicts), birthday_fills=fills,
                    fpl_records=len(fpl_rows), fpl_playable_candidates=len(current_player_ids),
                    sources=["Transfermarkt local archive", "FM23 Kaggle export", "Reep stamped register", "Transfermarkt profiles 2025", "Premier League FPL"],
                    files=file_manifest, release_checksums=checked)
    (output / "riseabove.database.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (output / "README.md").write_text(
        "# Rise Above database\n\nImport this folder in the app. The identity catalog is larger than the playable projection.\n\n"
        + manifest["freshness_note"] + "\n\nUnknown fields stay blank. Recorded zero stays zero.\n"
        + "identity_conflicts.csv lists rejected links. fm_tm_matches.csv links dated FM attributes; those attributes do not overwrite 2026 ratings.\n"
        + "Unchanged source/history files share disk blocks. Do not edit this immutable version; rebuild a new pack instead.\n",
        encoding="utf-8")
    print(json.dumps({k: v for k, v in manifest.items() if k not in ("files", "release_checksums")}, indent=2), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("archive", "fm", "staging", "output"):
        parser.add_argument(f"--{key}", type=Path, required=True)
    parser.add_argument("--legacy", type=Path, help="Optional existing FM23 usable CSV exports")
    parser.add_argument("--extra-fm", type=Path, nargs="*", default=[], help="Dated FM23 supplemental exports, kept as source records")
    args = parser.parse_args()
    build(args.archive.resolve(), args.fm.resolve(), args.staging.resolve(), args.output.resolve(), args.legacy.resolve() if args.legacy else None, [p.resolve() for p in args.extra_fm])
