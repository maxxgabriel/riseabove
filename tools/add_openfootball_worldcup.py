"""Add a dated World Cup squad source and only unambiguous identity links.

OpenFootball's 2026 World Cup file is supplemental evidence, not a current
global roster. A player links to the catalog only on unique normalized full
name + exact date of birth. The source is also retained verbatim for review.
"""
import argparse
import csv
import datetime as dt
import hashlib
import json
from pathlib import Path
import shutil
import unicodedata


SOURCE_URL = "https://raw.githubusercontent.com/openfootball/worldcup.json/master/2026/worldcup.squads.json"
SNAPSHOT = "2026-07-03"
TABLE = "worldcup_2026_squads.csv"


def rows(path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def key(value):
    normalized = unicodedata.normalize("NFKD", (value or "").casefold())
    return "".join(c for c in normalized if c.isalnum() and not unicodedata.combining(c))


def add(database: Path, source: Path, license_file: Path):
    target = database / TABLE
    if target.exists():
        raise ValueError(f"{target} already exists; refusing to overwrite an installed source")
    meta_path = database / "riseabove.database.json"
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    if "files" not in meta or "catalog_players.csv" not in meta["files"]:
        raise ValueError("Not a consolidated Rise Above database")

    catalog = {}
    for record in rows(database / "catalog_players.csv"):
        dob = record.get("dob", "")
        name = key(record.get("name"))
        if name and dob:
            catalog.setdefault((name, dob[:10]), []).append(record)

    club_rows = list(rows(database / "clubs.csv"))
    current_season = max(int(r.get("last_season") or 0) for r in club_rows)
    active_clubs = {r["club_id"] for r in club_rows if int(r.get("last_season") or 0) == current_season}
    all_clubs = {r["club_id"] for r in club_rows}
    tm_players = {r["player_id"]: r for r in rows(database / "players.csv")}
    imported = {
        pid for pid, r in tm_players.items()
        if int(r.get("last_season") or 0) >= current_season
        and r.get("current_club_id") in active_clubs
    }

    source_data = json.loads(source.read_text(encoding="utf-8-sig"))
    output = []
    counts = {"unique_name_birth_date": 0, "ambiguous": 0, "unmatched": 0}
    for country in source_data:
        for player in country.get("players", []):
            matches = catalog.get((key(player.get("name")), player.get("date_of_birth", "")[:10]), [])
            entity = matches[0].get("entity_id", "") if len(matches) == 1 else ""
            tm_id = matches[0].get("player_id", "") if len(matches) == 1 else ""
            if len(matches) == 1:
                validation = "unique_name_birth_date"
                counts[validation] += 1
            elif matches:
                validation = "ambiguous"
                counts[validation] += 1
            else:
                validation = "unmatched"
                counts[validation] += 1

            tm = tm_players.get(tm_id, {}) if tm_id else {}
            club = tm.get("current_club_id", "")
            if tm_id in imported:
                availability = "in_current_import"
            elif not tm_id:
                availability = "no_unique_transfermarkt_player_link"
            elif not tm or int(tm.get("last_season") or 0) < current_season:
                availability = "older_player_activity_record"
            elif club not in all_clubs:
                availability = "current_club_missing_from_club_file"
            elif club not in active_clubs:
                availability = "current_club_has_no_latest_season_record"
            else:
                availability = "not_in_current_import"

            club_info = player.get("club") or {}
            output.append({
                "source_id": f"{country.get('fifa_code', '')}-{player.get('number', '')}",
                "snapshot": SNAPSHOT,
                "team": country.get("name", ""),
                "fifa_code": country.get("fifa_code", ""),
                "shirt_number": player.get("number", ""),
                "position": player.get("pos", ""),
                "player_name": player.get("name", ""),
                "date_of_birth": player.get("date_of_birth", ""),
                "club_name": club_info.get("name", ""),
                "club_country": club_info.get("country", ""),
                "entity_id": entity,
                "transfermarkt_id": tm_id,
                "catalog_link_validation": validation,
                "current_import_status": availability,
            })

    fields = list(output[0])
    with target.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        writer.writerows(output)

    provenance = database / "provenance" / "openfootball-worldcup-2026"
    provenance.mkdir(parents=True, exist_ok=True)
    retained_source = provenance / "worldcup.squads.json"
    retained_license = provenance / "LICENSE.md"
    shutil.copy2(source, retained_source)
    shutil.copy2(license_file, retained_license)
    acquisition = {
        "source_url": SOURCE_URL,
        "source_snapshot": SNAPSHOT,
        "fetched_on": dt.date.today().isoformat(),
        "license": "CC0-1.0",
        "license_file": "LICENSE.md",
        "source_sha256": digest(retained_source),
        "license_sha256": digest(retained_license),
        "rows": len(output),
        "identity_link_method": "unique normalized full name plus exact date of birth; no name-only links",
        "identity_link_counts": counts,
    }
    (provenance / "acquisition.json").write_text(json.dumps(acquisition, indent=2) + "\n", encoding="utf-8")

    meta["sources"].append("OpenFootball 2026 World Cup registered squads (CC0, 2026-07-03 snapshot)")
    meta["worldcup_2026_squad_rows"] = len(output)
    meta["worldcup_2026_unique_identity_links"] = counts["unique_name_birth_date"]
    meta["files"][TABLE] = {"bytes": target.stat().st_size, "sha256": digest(target)}
    meta_path.write_text(json.dumps(meta, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return {"rows": len(output), "identity_link_counts": counts, "current_import_status": dict(__import__("collections").Counter(r["current_import_status"] for r in output))}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    parser.add_argument("source_json", type=Path)
    parser.add_argument("license_file", type=Path)
    args = parser.parse_args()
    print(json.dumps(add(args.database.resolve(), args.source_json.resolve(), args.license_file.resolve()), indent=2))
