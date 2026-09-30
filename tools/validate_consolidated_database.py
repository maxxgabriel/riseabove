"""Verify an immutable consolidated pack before installing it or cleaning inputs."""
import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path


def records(path):
    cp = path.name == "Staff list.csv"
    with path.open(encoding="cp1252" if cp else "utf-8-sig", newline="") as stream:
        yield from csv.DictReader(stream, delimiter=";" if cp else ",")


def verify(root):
    meta = json.loads((root / "riseabove.database.json").read_text(encoding="utf-8"))
    counts = {}
    for name, expected in meta["files"].items():
        path = root / name
        if path.parent.resolve() != root.resolve():
            raise ValueError("manifest path escapes database")
        with path.open("rb") as stream:
            actual = hashlib.file_digest(stream, "sha256").hexdigest()
        if actual != expected["sha256"] or path.stat().st_size != expected["bytes"]:
            raise ValueError(f"File changed: {name}")
        count = 0
        for row in records(path):
            if None in row:
                raise ValueError(f"Malformed row in {name}")
            count += 1
        counts[name] = count
    for name, field in [("players.csv", "player_id"), ("clubs.csv", "club_id"), ("catalog_players.csv", "entity_id")]:
        seen = set()
        for row in records(root / name):
            if not row[field] or row[field] in seen:
                raise ValueError(f"Duplicate/empty {field} in {name}")
            seen.add(row[field])
    if counts["catalog_players.csv"] != meta["catalog_players"]:
        raise ValueError("catalog count mismatch")
    clubs = {r["club_id"] for r in records(root / "clubs.csv")}
    players = {r["player_id"] for r in records(root / "players.csv")}
    for row in records(root / "fpl_current_squad.csv"):
        if row["club_id"] not in clubs:
            raise ValueError("current squad has unknown club")
        if row["removed"] == "False" and row["status"] != "u" and row["dob"] and row["player_id"] not in players:
            # A rejected identity link is deliberately not made playable.
            rejected = {r["source_id"] for r in records(root / "identity_conflicts.csv") if r["source"] == "FPL"}
            if row["opta_id"] not in rejected:
                raise ValueError("current squad has unknown player")
    match_ids = collections.Counter(r["fm_id"] for r in records(root / "fm_tm_matches.csv"))
    if any(n != 1 for n in match_ids.values()):
        raise ValueError("FM identity was mapped more than once")
    report = {"ok": True, "files_checked": len(meta["files"]), "rows": counts,
              "catalog_players": meta["catalog_players"], "identity_conflicts": meta["identity_conflicts"]}
    (root / "validation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    verify(parser.parse_args().database.resolve())
