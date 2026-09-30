"""Derive exact-ID, no-league club rows for current Transfermarkt squads."""
import argparse
import csv
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path


def rows(path: Path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream)


def main(root: Path):
    meta_path = root / "riseabove.database.json"
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    clubs = list(rows(root / "clubs.csv"))
    players = list(rows(root / "players.csv"))
    countries = {r["country_name"] for r in rows(root / "countries.csv")}
    teams = {r["reep_id"]: r for r in rows(root / "registry_teams.csv")
             if r["status"] == "active" and r.get("gender") == "men"}
    bridges = defaultdict(set)
    for link in rows(root / "identity_links.csv"):
        if (link["provider"], link["namespace"], link["validation"]) == ("transfermarkt", "verein", "provider_claim"):
            bridges[link["external_id"]].add(link["reep_id"])

    newest = max(int(c["last_season"] or 0) for c in clubs)
    fresh_clubs = {c["club_id"] for c in clubs if int(c["last_season"] or 0) == newest}
    current_rows = [p for p in players if int(p["last_season"] or 0) == newest and p["current_club_id"]]
    current_by_club = Counter(p["current_club_id"] for p in current_rows)

    supplement = []
    for club_id, count in sorted(current_by_club.items(), key=lambda item: int(item[0])):
        if club_id in fresh_clubs or len(bridges[club_id]) != 1:
            continue
        reep_id = next(iter(bridges[club_id]))
        team = teams.get(reep_id)
        if not team or team["country"] not in countries:
            continue
        supplement.append({
            "club_id": club_id,
            "club_name": team["label"],
            "country": team["country"],
            "reep_id": reep_id,
            "registry_status": team["status"],
            "gender": team["gender"],
            "crosswalk_validation": "unique_provider_claim",
            "evidence_players": count,
            "tm_snapshot": meta["archive_snapshot"],
            "reep_snapshot": meta["registry_stamp"],
            "snapshot": "2026-09-26",
        })

    output = root / "verified_unmodeled_clubs.csv"
    fields = ["club_id", "club_name", "country", "reep_id", "registry_status", "gender",
              "crosswalk_validation", "evidence_players", "tm_snapshot", "reep_snapshot", "snapshot"]
    with output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(supplement)

    meta["sources"] = list(dict.fromkeys(meta.get("sources", []) + [
        "Reep active men's team registry, exact Transfermarkt club-ID crosswalk (2026-09-26)"]))
    meta["verified_unmodeled_clubs"] = {
        "table": output.name,
        "clubs": len(supplement),
        "current_players": sum(int(r["evidence_players"]) for r in supplement),
        "competition_policy": "unknown; no competition membership is inferred",
        "country_policy": "exact match to a supported country name via active Reep team ID",
    }
    data = output.read_bytes()
    meta.setdefault("files", {})[output.name] = {
        "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()
    }
    meta_path.write_text(json.dumps(meta, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(meta["verified_unmodeled_clubs"], indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    main(parser.parse_args().database.resolve())
