"""Join public Reep v0 biographies to the current player catalog via exact IDs."""
import argparse
import csv
import gzip
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

SNAPSHOT = "2026-06-21"
FIELDS = {
    "key_transfermarkt": {"transfermarkt": {"spieler"}},
    "key_opta": {"opta": {"person"}},
    "key_opta_numeric": {"opta": {"person_numeric"}},
    "key_wyscout": {"wyscout": {"player"}},
    "key_skillcorner": {"skillcorner": {"player"}},
    "key_api_football": {"api_football": {"player"}},
    "key_capology": {"capology": {"player"}},
    "key_fbref": {"fbref": {"person"}, "fbref_dsg": {"person"}},
    "key_fbref_verified": {"fbref": {"person"}, "fbref_dsg": {"person"}},
    "key_worldfootball": {"worldfootball": {"person_numeric"}},
    "key_soccerdonna": {"soccerdonna": {"spieler"}},
    "key_uefa": {"uefa": {"player"}},
    "key_national_football_teams": {"national_football_teams": {"player"}},
    "key_sofifa": {"eafc": {"player"}},
    "key_fotmob": {"fotmob": {"person"}},
    "key_understat": {"understat": {"player"}},
    "key_besoccer": {"besoccer": {"player"}},
    "key_sportmonks": {"sportmonks": {"player"}},
}
FACTS = ("nationality", "position", "position_detail", "height_cm")
NAMESPACES = defaultdict(set)
for provider_map in FIELDS.values():
    for provider, namespaces in provider_map.items():
        NAMESPACES[provider].update(namespaces)


def read_csv(path: Path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream)


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def enrich(root: Path):
    meta_path = root / "riseabove.database.json"
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    people_path = root / "provenance" / "reep-v0-people.csv"
    people_acq = json.loads(people_path.with_suffix(".acquisition.json").read_text(encoding="utf-8"))
    if people_acq.get("license") != "CC0-1.0" or sha256(people_path) != people_acq.get("sha256"):
        raise ValueError("Reep v0 source license/checksum metadata is missing or does not match")
    license_path = root / "provenance" / "reep-v0-LICENSE.txt"
    if not license_path.exists() or sha256(license_path) != people_acq.get("license_sha256"):
        raise ValueError("Reep v0 CC0 license text is missing or does not match acquisition metadata")

    players = {r["reep_id"] for r in read_csv(root / "registry_players.csv")}
    redirects = {r["from_id"]: r["to_id"] for r in read_csv(root / "registry_redirects.csv") if r.get("to_id")}

    def survivor(entity):
        seen = set()
        while entity in redirects:
            if entity in seen:
                raise ValueError("Reep redirect cycle")
            seen.add(entity)
            entity = redirects[entity]
        return entity

    catalog = {r["entity_id"]: r for r in read_csv(root / "catalog_players.csv")}
    keys_by_provider = defaultdict(set)
    for row in read_csv(people_path):
        if row.get("type") != "player":
            continue
        for field, providers in FIELDS.items():
            value = row.get(field, "")
            for provider in providers:
                if value:
                    keys_by_provider[provider].add(value)

    bridges = defaultdict(set)
    bridge_path = root / "provenance" / "reep-bridges.csv.gz"
    with gzip.open(bridge_path, "rt", encoding="utf-8-sig", newline="") as stream:
        for link in csv.DictReader(stream):
            provider, namespace, external_id = link["provider"], link["namespace"], link["external_id"]
            if (external_id not in keys_by_provider.get(provider, ())
                    or namespace not in NAMESPACES.get(provider, ())
                    or link.get("upstream_status") == "retired"):
                continue
            entity = survivor(link["reep_id"])
            if entity in players:
                bridges[(provider, namespace, external_id)].add(entity)

    qids = defaultdict(set)
    overlay_path = root / "provenance" / "reep-v1-overlay-links.csv.gz"
    overlay_acq = json.loads((root / "provenance" / "reep-v1-overlay-links.acquisition.json").read_text(encoding="utf-8"))
    if overlay_acq.get("stamp") != meta.get("registry_stamp") or sha256(overlay_path) != overlay_acq.get("sha256"):
        raise ValueError("Reep v1 overlay release/checksum does not match the consolidated pack")
    with gzip.open(overlay_path, "rt", encoding="utf-8-sig", newline="") as stream:
        for link in csv.DictReader(stream):
            if (link.get("entity_type") == "player" and link.get("dob_check") == "dob-agrees"
                    and link.get("qid") and link["reep_id"] in players):
                qids[link["qid"]].add(link["reep_id"])

    # First pass resolves each source row. No names are join keys. QIDs are used
    # only when the current release says its separate overlay DOB check agrees.
    candidates = defaultdict(list)
    conflicts = []
    counts = Counter()
    for row in read_csv(people_path):
        if row.get("type") != "player":
            continue
        targets = set()
        bases = set()
        for field, providers in FIELDS.items():
            external_id = row.get(field, "")
            if not external_id:
                continue
            for provider, namespaces in providers.items():
                for namespace in namespaces:
                    found = bridges.get((provider, namespace, external_id), set())
                    targets.update(found)
                    if found:
                        bases.add(f"{provider}:{external_id}")
        qid = row.get("key_wikidata", "")
        if qid:
            found = qids.get(qid, set())
            targets.update(found)
            if found:
                bases.add(f"wikidata:{qid}:dob-agrees")
        if len(targets) > 1:
            counts["ambiguous_source_rows"] += 1
            conflicts.append({"entity_id": "", "source_reep_id": row["reep_id"], "field": "identity",
                              "candidate_values": ";".join(sorted(targets)), "source_ids": ";".join(sorted(bases)),
                              "reason": "verified provider IDs resolve to multiple current player entities"})
            continue
        if not targets:
            counts["unmatched_source_rows"] += 1
            continue
        target = next(iter(targets))
        candidates[target].append((row, bases))

    output_rows = []
    for entity, source_rows in sorted(candidates.items()):
        current = catalog.get(entity)
        if current is None:
            counts["not_in_catalog"] += 1
            continue
        result = {"entity_id": entity, "source_reep_ids": ";".join(sorted({r["reep_id"] for r, _ in source_rows})),
                  "source_name": " | ".join(sorted({r["name"] for r, _ in source_rows if r.get("name")})),
                  "full_name": " | ".join(sorted({r["full_name"] for r, _ in source_rows if r.get("full_name")})),
                  "nationality": "", "nationality_source": "Unknown", "position": "", "position_detail": "",
                  "position_source": "Unknown", "height_cm": "", "height_source": "Unknown",
                  "link_basis": "", "snapshot": SNAPSHOT, "validation": "exact provider ID or QID with current-release DOB agreement"}
        field_sources = {name: set() for name in FACTS}
        for field in FACTS:
            values = {r.get(field, "").strip() for r, _ in source_rows if r.get(field, "").strip()}
            if field == "height_cm":
                normalized = set()
                for value in values:
                    try:
                        height = float(value)
                    except ValueError:
                        continue
                    if height.is_integer() and 100 <= height <= 230:
                        normalized.add(str(int(height)))
                values = normalized
            if len(values) > 1:
                conflicts.append({"entity_id": entity, "source_reep_id": result["source_reep_ids"], "field": field,
                                  "candidate_values": ";".join(sorted(values)), "source_ids": ";".join(sorted({x for _, ids in source_rows for x in ids})),
                                  "reason": "source rows disagree; value left unknown"})
                continue
            if values:
                result[field] = next(iter(values))
                field_sources[field].add("reep-v0-wikidata-2026-06-21")
        if result["nationality"]:
            result["nationality_source"] = "reep-v0-wikidata-2026-06-21"
        if result["position"] or result["position_detail"]:
            result["position_source"] = "reep-v0-wikidata-2026-06-21"
        if result["height_cm"]:
            result["height_source"] = "reep-v0-wikidata-2026-06-21"
        result["link_basis"] = " | ".join(sorted({x for _, ids in source_rows for x in ids}))
        if not any(result[f] for f in FACTS):
            counts["identity_only_matches"] += 1
            continue
        if current.get("eligibility") == "identity_only":
            counts["identity_only_enriched"] += 1
        counts["profile_rows"] += 1
        for field in FACTS:
            if result[field]:
                counts[f"has_{field}"] += 1
        output_rows.append(result)

    output = root / "catalog_player_profiles_reep_v0.csv"
    fields = ["entity_id", "source_reep_ids", "source_name", "full_name", "nationality", "nationality_source",
              "position", "position_detail", "position_source", "height_cm", "height_source", "link_basis", "snapshot", "validation"]
    with output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(output_rows)
    conflict_path = root / "catalog_player_profile_conflicts_reep_v0.csv"
    conflict_fields = ["entity_id", "source_reep_id", "field", "candidate_values", "source_ids", "reason"]
    with conflict_path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=conflict_fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(conflicts)

    for path in (output, conflict_path):
        data = path.read_bytes()
        meta.setdefault("files", {})[path.name] = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    meta["sources"] = list(dict.fromkeys(meta.get("sources", []) + ["Reep v0 Wikidata-derived biography snapshot (CC0, 2026-06-21); exact provider-ID joins only"]))
    meta["reep_v0_biography_enrichment"] = {"snapshot": SNAPSHOT, "profile_rows": len(output_rows),
        "identity_only_enriched": counts["identity_only_enriched"], "dob_policy": "not exported; current Reep v1 withholds DOB",
        "identity_policy": "exact provider IDs, or exact Wikidata QID only when current Reep overlay dob_check agrees",
        "conflict_rows": len(conflicts), "unmatched_source_rows": counts["unmatched_source_rows"],
        "ambiguous_source_rows": counts["ambiguous_source_rows"]}
    meta_path.write_text(json.dumps(meta, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"enrichment": meta["reep_v0_biography_enrichment"], "counts": counts}, ensure_ascii=True, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    enrich(parser.parse_args().database.resolve())
