"""Attach exact-FM-UID FM23 facts to identity-only catalog entries."""
import argparse
import csv
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path


def read_csv(path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def number(raw, unit):
    match = re.search(r"\d+(?:\.\d+)?", raw or "")
    if not match:
        return ""
    try:
        value = float(match.group())
    except ValueError:
        return ""
    if unit == "cm" and not 100 <= value <= 230:
        return ""
    if unit == "kg" and not 30 <= value <= 200:
        return ""
    return str(int(value)) if value.is_integer() else str(value)


def enrich(root):
    metadata_path = root / "riseabove.database.json"
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    catalog = list(read_csv(root / "catalog_players.csv"))
    fm_targets = defaultdict(list)
    for player in catalog:
        if player.get("eligibility") == "identity_only" and player.get("fm_id"):
            fm_targets[player["fm_id"]].append(player["entity_id"])

    record_paths = [root / "fm23_player_records.csv", root / "fm23_supplement_1_records.csv",
                    root / "fm23_supplement_2_records.csv"]
    records = {}
    source_files = {}
    attribute_fields = None
    for path in record_paths:
        for row in read_csv(path):
            uid = row.get("id", "").strip()
            if not uid or uid in records:
                continue
            records[uid] = row
            source_files[uid] = path.name
            if attribute_fields is None:
                excluded = {"id", "inf", "name", "dob_raw", "nat", "division", "club", "based",
                            "preferred_foot", "right_foot", "left_foot", "position", "height", "weight",
                            "age", "transfer_value", "wage", "at_apps", "at_gls", "team", "caps",
                            "yth_apps", "style", "rc_injury", "best_role", "best_duty", "best_pos",
                            "birth_date", "source_row"}
                attribute_fields = [field for field in row if field not in excluded]

    rows = []
    conflicts = []
    matched_ids = 0
    for fm_id, entities in sorted(fm_targets.items()):
        if len(entities) != 1:
            conflicts.append({"entity_id": ";".join(sorted(entities)), "fm_id": fm_id,
                              "reason": "one FM23 UID maps to multiple identity-only catalog entities"})
            continue
        source = records.get(fm_id)
        if not source:
            continue
        matched_ids += 1
        attributes = {field: source.get(field, "") for field in attribute_fields if source.get(field, "")}
        rows.append({
            "entity_id": entities[0], "fm_id": fm_id,
            "source_name": source.get("name", ""),
            "nationality_code": source.get("nat", ""),
            "position": source.get("position", ""),
            "height_cm": number(source.get("height", ""), "cm"),
            "weight_kg": number(source.get("weight", ""), "kg"),
            "preferred_foot": source.get("preferred_foot", ""),
            "fm23_age": source.get("age", ""),
            "fm23_apps": source.get("at_apps", ""),
            "fm23_goals": source.get("at_gls", ""),
            "fm23_caps": source.get("caps", ""),
            "fm23_youth_apps": source.get("yth_apps", ""),
            "best_role": source.get("best_role", ""),
            "best_duty": source.get("best_duty", ""),
            "best_position": source.get("best_pos", ""),
            "attributes_json": json.dumps(attributes, ensure_ascii=False, sort_keys=True, separators=(",", ":")),
            "source_file": source_files[fm_id],
            "source_snapshot": "FM23 historical export; original source date not present on every row",
            "field_source": "FM23 exact player UID",
        })

    output = root / "catalog_player_profiles_fm23.csv"
    fields = ["entity_id", "fm_id", "source_name", "nationality_code", "position", "height_cm", "weight_kg",
              "preferred_foot", "fm23_age", "fm23_apps", "fm23_goals", "fm23_caps", "fm23_youth_apps",
              "best_role", "best_duty", "best_position", "attributes_json", "source_file", "source_snapshot", "field_source"]
    with output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    conflict_path = root / "catalog_player_profile_conflicts_fm23.csv"
    with conflict_path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=["entity_id", "fm_id", "reason"], lineterminator="\n")
        writer.writeheader()
        writer.writerows(conflicts)
    for path in (output, conflict_path):
        metadata.setdefault("files", {})[path.name] = {"bytes": path.stat().st_size, "sha256": digest(path)}
    metadata["fm23_identity_only_enrichment"] = {
        "source": "local FM23 player exports; exact FM player UID only",
        "population": "identity_only catalog entries with one unique FM23 UID",
        "matched_players": len(rows), "conflict_rows": len(conflicts),
        "snapshot_policy": "historical FM23 facts kept separate from current ratings and contracts",
        "unmatched_catalog_fm_uids": len(fm_targets) - matched_ids - sum(len(set(r["fm_id"].split(";"))) for r in conflicts),
    }
    metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(metadata["fm23_identity_only_enrichment"], indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    enrich(parser.parse_args().database.resolve())
