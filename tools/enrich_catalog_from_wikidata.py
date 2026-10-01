"""Fill otherwise-empty identity-only profiles using exact current Reep QID links."""
import argparse
import csv
import gzip
import hashlib
import json
import time
import urllib.error
import urllib.parse
import urllib.request
from email.utils import parsedate_to_datetime
from collections import defaultdict
from datetime import date, datetime, timezone
from pathlib import Path

API = "https://www.wikidata.org/w/api.php"
PROPERTIES = ("P569", "P27", "P413", "P2048")
FIELDS = ("date_of_birth", "nationality", "position", "height_cm")
AGENT = "RiseAboveDatabase/1.0 (https://github.com/maxxgabriel/riseabove; local football simulation) Python-urllib"


def read_csv(path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream)


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def request_json(params, attempts=6):
    params.setdefault("maxlag", 5)
    url = API + "?" + urllib.parse.urlencode(params)
    request = urllib.request.Request(url, headers={"User-Agent": AGENT, "Accept": "application/json"})
    for attempt in range(attempts):
        try:
            with urllib.request.urlopen(request, timeout=90) as response:
                data = json.loads(response.read())
                if "error" in data:
                    if data["error"].get("code") == "maxlag" and attempt + 1 < attempts:
                        lag = data["error"].get("lag")
                        try:
                            delay = max(5, int(lag) + 2)
                        except (TypeError, ValueError):
                            delay = max(5, 2 ** (attempt + 3))
                        print(json.dumps({"api_status": "maxlag", "retry_after_seconds": delay}), flush=True)
                        time.sleep(delay)
                        continue
                    raise RuntimeError("Wikidata API error: " + json.dumps(data["error"], ensure_ascii=False))
                return data
        except urllib.error.HTTPError as error:
            if error.code not in (429, 503) or attempt + 1 == attempts:
                raise
            retry_after = error.headers.get("Retry-After")
            try:
                delay = max(5, int(retry_after)) if retry_after else max(5, 2 ** (attempt + 3))
            except ValueError:
                try:
                    until = parsedate_to_datetime(retry_after).timestamp()
                    delay = max(5, int(until - time.time()))
                except (TypeError, ValueError, OverflowError):
                    delay = max(5, 2 ** (attempt + 3))
            print(json.dumps({"api_status": error.code, "retry_after_seconds": delay}), flush=True)
            time.sleep(delay)
        except Exception:
            if attempt + 1 == attempts:
                raise
            time.sleep(min(60, 2 ** (attempt + 3)))


def item_ids(entity, prop):
    values = set()
    for claim in entity.get("claims", {}).get(prop, []):
        if claim.get("rank") == "deprecated":
            continue
        value = claim.get("mainsnak", {}).get("datavalue", {}).get("value", {})
        if value.get("id", "").startswith("Q"):
            values.add(value["id"])
    return values


def quantity_values(entity, prop):
    values = set()
    for claim in entity.get("claims", {}).get(prop, []):
        if claim.get("rank") == "deprecated":
            continue
        value = claim.get("mainsnak", {}).get("datavalue", {}).get("value", {})
        try:
            amount = float(value.get("amount", ""))
        except (TypeError, ValueError):
            continue
        unit = value.get("unit", "")
        if unit.endswith("/Q11573"):  # metre
            amount *= 100
        elif unit.endswith("/Q174728"):  # centimetre
            pass
        else:
            continue
        if 100 <= amount <= 230:
            values.add(str(int(round(amount))))
    return values


def simple_values(entity, prop):
    values = set()
    for claim in entity.get("claims", {}).get(prop, []):
        if claim.get("rank") == "deprecated":
            continue
        value = claim.get("mainsnak", {}).get("datavalue", {}).get("value")
        if prop == "P569" and isinstance(value, dict) and claim.get("mainsnak", {}).get("datavalue", {}).get("type") == "time":
            raw = value.get("time", "")
            if value.get("precision") == 11 and len(raw) >= 11 and raw[1:5].isdigit() and raw[6:8].isdigit() and raw[9:11].isdigit():
                try:
                    values.add(date(int(raw[1:5]), int(raw[6:8]), int(raw[9:11])).isoformat())
                except ValueError:
                    pass
    return values


def enrich(root):
    metadata_path = root / "riseabove.database.json"
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    catalog = {row["entity_id"]: row for row in read_csv(root / "catalog_players.csv")}
    existing_profiles = {row["entity_id"] for row in read_csv(root / "catalog_player_profiles_reep_v0.csv")}

    # A QID is accepted only from the current Reep overlay after its explicit
    # DOB agreement check. No name, club, or fuzzy match is used here.
    qid_targets = defaultdict(set)
    overlay = root / "provenance" / "reep-v1-overlay-links.csv.gz"
    overlay_acquisition = json.loads((root / "provenance" / "reep-v1-overlay-links.acquisition.json").read_text(encoding="utf-8"))
    if (overlay_acquisition.get("stamp") != metadata.get("registry_stamp")
            or sha256(overlay) != overlay_acquisition.get("sha256")):
        raise ValueError("Reep v1 overlay release/checksum does not match the consolidated pack")
    with gzip.open(overlay, "rt", encoding="utf-8-sig", newline="") as stream:
        for row in csv.DictReader(stream):
            entity = row.get("reep_id", "")
            if (row.get("entity_type") == "player" and row.get("dob_check") == "dob-agrees"
                    and row.get("qid") and entity in catalog
                    and catalog[entity].get("eligibility") == "identity_only"):
                qid_targets[row["qid"]].add(entity)

    # Freeze the accepted QID set for reproducible reruns even if later provider
    # crosswalks add more biography rows for these same people.
    ambiguous_qids = {qid: sorted(entities) for qid, entities in qid_targets.items() if len(entities) != 1}
    qid_list_path = root / "provenance" / "wikidata-identity-qids.txt"
    if qid_list_path.exists():
        requested_qids = sorted({line.strip() for line in qid_list_path.read_text(encoding="utf-8").splitlines() if line.strip()})
    else:
        requested_qids = sorted(qid for qid, entities in qid_targets.items()
                                 if len(entities) == 1 and any(entity not in existing_profiles for entity in entities))
        qid_list_path.write_text("\n".join(requested_qids) + "\n", encoding="utf-8")
    targets = {qid: sorted(qid_targets[qid]) for qid in requested_qids
               if qid in qid_targets and len(qid_targets[qid]) == 1}
    acquisition = root / "provenance" / "wikidata-entity-cache"
    acquisition.mkdir(parents=True, exist_ok=True)
    rows = []
    conflicts = [{"entity_id": ";".join(entities), "wikidata_qid": qid, "field": "identity",
                  "candidate_values": ";".join(entities),
                  "reason": "current Reep overlay maps this QID to multiple catalog players"}
                 for qid, entities in sorted(ambiguous_qids.items())]
    all_label_ids = set()
    entity_facts = {}
    qids = sorted(targets)
    target_signature = hashlib.sha256("\n".join(qids).encode("utf-8")).hexdigest()[:12]
    for offset in range(0, len(qids), 50):
        batch = qids[offset:offset + 50]
        cache = acquisition / ("entities-" + target_signature + "-" + str(offset // 50).zfill(5) + ".json")
        if cache.exists():
            data = json.loads(cache.read_text(encoding="utf-8"))
            if not data.get("entities") and not data.get("missing"):
                cache.unlink()
                data = None
        else:
            data = None
        if data is None:
            raw_data = request_json({"action": "wbgetentities", "ids": "|".join(batch),
                                     "props": "claims|labels", "languages": "en", "format": "json"})
            data = {"entities": {}, "missing": []}
            for qid, entity in raw_data.get("entities", {}).items():
                if "missing" in entity:
                    data["missing"].append(qid)
                    continue
                nationalities = item_ids(entity, "P27")
                positions = item_ids(entity, "P413")
                all_label_ids.update(nationalities | positions)
                data["entities"][qid] = {
                    "name": entity.get("labels", {}).get("en", {}).get("value", ""),
                    "date_of_birth": sorted(simple_values(entity, "P569")),
                    "nationality_ids": sorted(nationalities),
                    "position_ids": sorted(positions),
                    "height_cm": sorted(quantity_values(entity, "P2048")),
                }
            if set(data["entities"]) | set(data["missing"]) != set(batch):
                raise RuntimeError("Wikidata response omitted one or more requested QIDs")
            if not data["entities"] and not data["missing"]:
                raise RuntimeError("Wikidata returned no entities for a non-empty QID batch")
            cache.write_text(json.dumps(data, separators=(",", ":"), ensure_ascii=False), encoding="utf-8")
            time.sleep(1.0)
        for qid, entity in data.get("entities", {}).items():
            all_label_ids.update(entity.get("nationality_ids", []))
            all_label_ids.update(entity.get("position_ids", []))
            entity_facts[qid] = entity
        print(json.dumps({"batch": offset // 50 + 1, "batches": (len(qids) + 49) // 50,
                          "profiles_loaded": len(entity_facts)}), flush=True)

    labels = {}
    item_ids_sorted = sorted(all_label_ids)
    label_signature = hashlib.sha256("\n".join(item_ids_sorted).encode("utf-8")).hexdigest()[:12]
    for offset in range(0, len(item_ids_sorted), 50):
        batch = item_ids_sorted[offset:offset + 50]
        cache = acquisition / ("labels-" + label_signature + "-" + str(offset // 50).zfill(5) + ".json")
        if cache.exists():
            data = json.loads(cache.read_text(encoding="utf-8"))
            if not data.get("entities"):
                cache.unlink()
                data = None
        else:
            data = None
        if data is None:
            data = request_json({"action": "wbgetentities", "ids": "|".join(batch),
                                 "props": "labels", "languages": "en", "format": "json"})
            if not data.get("entities"):
                raise RuntimeError("Wikidata returned no labels for a non-empty item batch")
            cache.write_text(json.dumps(data, separators=(",", ":"), ensure_ascii=False), encoding="utf-8")
            time.sleep(1.0)
        for qid, entity in data.get("entities", {}).items():
            label = entity.get("labels", {}).get("en", {}).get("value", "")
            if label:
                labels[qid] = label

    snapshot = datetime.now(timezone.utc).date().isoformat()
    for qid, entities in sorted(targets.items()):
        facts = entity_facts.get(qid)
        if not facts:
            continue
        for entity in entities:
            record = {"entity_id": entity, "wikidata_qid": qid, "source_name": facts["name"],
                      "date_of_birth": "", "date_of_birth_source": "Unknown",
                      "nationality": "", "nationality_source": "Unknown",
                      "position": "", "position_source": "Unknown",
                      "height_cm": "", "height_source": "Unknown",
                      "snapshot": snapshot, "link_basis": "reep-v1-overlay:dob-agrees",
                      "validation": "exact Wikidata QID from current Reep DOB-agreement overlay"}
            values_by_field = {
                "date_of_birth": facts["date_of_birth"],
                "nationality": {labels[x] for x in facts["nationality_ids"] if x in labels},
                "position": {labels[x] for x in facts["position_ids"] if x in labels},
                "height_cm": facts["height_cm"],
            }
            source_columns = {"date_of_birth": "date_of_birth_source", "nationality": "nationality_source",
                              "position": "position_source", "height_cm": "height_source"}
            for field, values in values_by_field.items():
                values = {value for value in values if value}
                if len(values) == 1:
                    record[field] = next(iter(values))
                    record[source_columns[field]] = "Wikidata-CC0-" + snapshot
                elif len(values) > 1:
                    conflicts.append({"entity_id": entity, "wikidata_qid": qid, "field": field,
                                      "candidate_values": ";".join(sorted(values)),
                                      "reason": "Wikidata has multiple non-deprecated values; left unknown"})
            if any(record[field] for field in FIELDS):
                rows.append(record)

    output = root / "catalog_player_profiles_wikidata.csv"
    fields = ["entity_id", "wikidata_qid", "source_name", "date_of_birth", "date_of_birth_source",
              "nationality", "nationality_source", "position", "position_source", "height_cm",
              "height_source", "snapshot", "link_basis", "validation"]
    with output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    conflict_path = root / "catalog_player_profile_conflicts_wikidata.csv"
    conflict_fields = ["entity_id", "wikidata_qid", "field", "candidate_values", "reason"]
    with conflict_path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=conflict_fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(conflicts)
    for path in (output, conflict_path):
        metadata.setdefault("files", {})[path.name] = {"bytes": path.stat().st_size, "sha256": sha256(path)}
    metadata["wikidata_identity_only_enrichment"] = {
        "source": "Wikidata structured entity data, CC0",
        "snapshot": snapshot,
        "identity_policy": "exact current Reep player QID with dob_check=dob-agrees",
        "population": "identity_only players on the frozen exact-QID acquisition list",
        "qid_targets": len(targets), "profile_rows": len(rows), "conflict_rows": len(conflicts),
        "date_of_birth_policy": "Wikidata P569 included only when a single valid date is recorded",
        "cache_directory": "provenance/wikidata-entity-cache",
        "ambiguous_qid_links": len(ambiguous_qids),
        "missing_wikidata_entities": len(qids) - len(entity_facts),
    }
    metadata["sources"] = list(dict.fromkeys(metadata.get("sources", []) + [
        "Wikidata structured entity data (CC0), exact-QID supplemental biographies"
    ]))
    metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    acquisition_path = root / "provenance" / "wikidata-enrichment.acquisition.json"
    acquisition_path.write_text(json.dumps({
        "url": API,
        "snapshot": snapshot,
        "license": "CC0-1.0",
        "properties": {"P569": "date of birth", "P27": "country of citizenship",
                       "P413": "position played", "P2048": "height"},
        "identity_source": "current Reep overlay, entity_type=player and dob_check=dob-agrees",
        "population": "identity_only catalog players on the frozen exact-QID acquisition list",
        "qids_requested": len(qids), "qids_sha256": target_signature,
        "qids_file": "provenance/wikidata-identity-qids.txt",
        "qids_file_sha256": sha256(qid_list_path),
        "output_sha256": sha256(output), "conflicts_sha256": sha256(conflict_path),
        "missing_entities": len(qids) - len(entity_facts),
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"qid_targets": len(targets), "profile_rows": len(rows),
                      "conflict_rows": len(conflicts), "identity_only_facts": {
                          field: sum(bool(row[field]) for row in rows) for field in FIELDS}}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    enrich(parser.parse_args().database.resolve())
