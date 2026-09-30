"""Build a source-prioritized, one-row-per-player facts view from verified profile layers."""
import argparse
import csv
import hashlib
import json
from collections import Counter
from pathlib import Path


def read_rows(path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream)


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def load_index(path):
    index = {}
    if path.exists():
        for row in read_rows(path):
            index[row["entity_id"]] = row
    return index


def build(root):
    metadata_path = root / "riseabove.database.json"
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    catalog = {row["entity_id"]: row for row in read_rows(root / "catalog_players.csv")}
    reep = load_index(root / "catalog_player_profiles_reep_v0.csv")
    wikidata = load_index(root / "catalog_player_profiles_wikidata.csv")
    fm23 = load_index(root / "catalog_player_profiles_fm23.csv")
    nation_names = {row.get("abbreviation", "").strip().upper(): row.get("name", "").strip()
                    for row in read_rows(root / "fm23_legacy_nations_usable.csv")
                    if row.get("abbreviation") and row.get("name")}

    rows = []
    conflicts = []
    priority = {"catalog DOB": 0, "Reep v0": 1, "Wikidata": 2, "FM23": 3}
    counts = Counter()
    for entity_id, player in sorted(catalog.items()):
        r, w, f = reep.get(entity_id, {}), wikidata.get(entity_id, {}), fm23.get(entity_id, {})
        candidates = {
            "date_of_birth": [(player.get("dob", ""), player.get("dob_source", ""), "catalog DOB"),
                              (w.get("date_of_birth", ""), w.get("date_of_birth_source", ""), "Wikidata")],
            "nationality": [(r.get("nationality", ""), r.get("nationality_source", ""), "Reep v0"),
                            (w.get("nationality", ""), w.get("nationality_source", ""), "Wikidata"),
                            (nation_names.get(f.get("nationality_code", "").strip().upper(),
                                              f.get("nationality_code", "")),
                             f.get("field_source", ""), "FM23")],
            "position": [(r.get("position", ""), r.get("position_source", ""), "Reep v0"),
                         (w.get("position", ""), w.get("position_source", ""), "Wikidata"),
                         (f.get("position", ""), f.get("field_source", ""), "FM23")],
            "height_cm": [(r.get("height_cm", ""), r.get("height_source", ""), "Reep v0"),
                          (w.get("height_cm", ""), w.get("height_source", ""), "Wikidata"),
                          (f.get("height_cm", ""), f.get("field_source", ""), "FM23")],
            "weight_kg": [(f.get("weight_kg", ""), f.get("field_source", ""), "FM23")],
            "preferred_foot": [(f.get("preferred_foot", ""), f.get("field_source", ""), "FM23")],
            "position_detail": [(r.get("position_detail", ""), r.get("position_source", ""), "Reep v0")],
            "fm23_age": [(f.get("fm23_age", ""), f.get("field_source", ""), "FM23")],
            "fm23_apps": [(f.get("fm23_apps", ""), f.get("field_source", ""), "FM23")],
            "fm23_goals": [(f.get("fm23_goals", ""), f.get("field_source", ""), "FM23")],
            "fm23_caps": [(f.get("fm23_caps", ""), f.get("field_source", ""), "FM23")],
            "fm23_youth_apps": [(f.get("fm23_youth_apps", ""), f.get("field_source", ""), "FM23")],
            "best_role": [(f.get("best_role", ""), f.get("field_source", ""), "FM23")],
            "best_duty": [(f.get("best_duty", ""), f.get("field_source", ""), "FM23")],
            "best_position": [(f.get("best_position", ""), f.get("field_source", ""), "FM23")],
            "attributes_json": [(f.get("attributes_json", ""), f.get("field_source", ""), "FM23")],
        }
        result = {"entity_id": entity_id, "name": player.get("name", ""),
                  "eligibility": player.get("eligibility", ""), "gender": player.get("gender", ""),
                  "catalog_country": player.get("country", ""), "sources": ""}
        selected_sources = set()
        for field, options in candidates.items():
            values = {}
            for value, source_label, source in options:
                if value:
                    values.setdefault(value, []).append((source_label, source))
            if not values:
                result[field] = ""
                result[field + "_source"] = "Unknown"
                continue
            ranked = sorted(((min(priority[source] for _, source in labels), value, labels)
                             for value, labels in values.items()), key=lambda row: (row[0], row[1]))
            best_rank, chosen, labels = ranked[0]
            chosen_label, chosen_source = min(((label, source) for label, source in labels),
                                              key=lambda item: priority[item[1]])
            result[field] = chosen
            result[field + "_source"] = chosen_label or chosen_source
            selected_sources.add(chosen_source)
            if len(values) > 1:
                conflicts.append({"entity_id": entity_id, "field": field, "chosen_value": chosen,
                                  "chosen_source": chosen_source,
                                  "candidate_values": json.dumps({value: sorted({label for label, _ in labels})
                                                                   for value, labels in values.items()},
                                                                  ensure_ascii=False, sort_keys=True),
                                  "reason": "sources disagree; retained the highest-priority documented source"})
        result["nationality_code"] = f.get("nationality_code", "")
        result["nationality_code_source"] = f.get("field_source", "Unknown") if f.get("nationality_code") else "Unknown"
        result["sources"] = ";".join(sorted(selected_sources))
        rows.append(result)
        counts["profile_rows"] += 1
        counts["identity_only_rows"] += result["eligibility"] == "identity_only"
        has_sourced_fact = any(result[field] for field in candidates)
        counts["players_with_sourced_facts"] += has_sourced_fact
        counts["identity_only_with_sourced_facts"] += result["eligibility"] == "identity_only" and has_sourced_fact
        for field in candidates:
            if result[field]:
                counts["has_" + field] += 1

    output = root / "catalog_player_facts.csv"
    fields = ["entity_id", "name", "eligibility", "gender", "catalog_country",
              "date_of_birth", "date_of_birth_source", "nationality", "nationality_source",
              "nationality_code", "nationality_code_source",
              "position", "position_source", "position_detail", "position_detail_source",
              "height_cm", "height_cm_source", "weight_kg", "weight_kg_source", "preferred_foot",
              "preferred_foot_source", "fm23_age", "fm23_age_source", "fm23_apps", "fm23_apps_source",
              "fm23_goals", "fm23_goals_source", "fm23_caps", "fm23_caps_source",
              "fm23_youth_apps", "fm23_youth_apps_source", "best_role", "best_role_source",
              "best_duty", "best_duty_source", "best_position", "best_position_source",
              "attributes_json", "attributes_json_source", "sources"]
    with output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    conflict_path = root / "catalog_player_fact_conflicts.csv"
    conflict_fields = ["entity_id", "field", "chosen_value", "chosen_source", "candidate_values", "reason"]
    with conflict_path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=conflict_fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(conflicts)
    for path in (output, conflict_path):
        metadata.setdefault("files", {})[path.name] = {"bytes": path.stat().st_size, "sha256": sha256(path)}
    metadata["catalog_player_facts"] = {**counts, "cross_source_conflict_rows": len(conflicts),
        "source_priority": ["catalog DOB", "Reep v0", "Wikidata", "FM23"],
        "note": "One row per catalog identity; unsourced facts stay blank and identity-only rows remain non-playable."}
    metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(metadata["catalog_player_facts"], indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    build(parser.parse_args().database.resolve())
