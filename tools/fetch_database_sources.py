"""Refresh public identity/current-PL inputs; never changes a played world.

python tools/fetch_database_sources.py --staging E:/pers/riseabove-datasets/staging-NEXT
Then rebuild a NEW immutable pack with consolidate_database.py and validate it.
The local July archive and dated FM exports are deliberately not overwritten.
"""
import argparse
import concurrent.futures
import datetime as dt
import hashlib
import json
from pathlib import Path
import shutil
import urllib.request


def fetch(url, destination):
    request = urllib.request.Request(url, headers={"User-Agent": "RiseAbove/1.0 public-database-import"})
    temporary = destination.with_suffix(destination.suffix + ".part")
    with urllib.request.urlopen(request, timeout=180) as source, temporary.open("wb") as target:
        shutil.copyfileobj(source, target)
    temporary.replace(destination)


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def download(root):
    root.mkdir(parents=True, exist_ok=True)
    (root / "reep").mkdir(exist_ok=True)
    fetch("https://data.reep.football/releases/latest.json", root / "reep-latest.json")
    latest = json.loads((root / "reep-latest.json").read_text(encoding="utf-8"))
    fetch(latest["manifest_url"], root / "reep-release.json")
    release = json.loads((root / "reep-release.json").read_text(encoding="utf-8"))
    for name in ("LICENSE.txt", "schema.json"):
        info = release["files"][name]
        fetch(info["url"], root / name)
        if sha(root / name) != info["sha256"]:
            raise ValueError(f"Checksum mismatch: {name}")

    def table(name):
        info = release["files"][f"csv/{name}.csv.gz"]
        path = root / "reep" / f"{name}.csv.gz"
        if not path.exists() or sha(path) != info["sha256"]:
            fetch(info["url"], path)
        if sha(path) != info["sha256"] or path.stat().st_size != info["bytes"]:
            raise ValueError(f"Checksum mismatch: {name}")
        print(f"Verified Reep {name}", flush=True)

    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        list(pool.map(table, ("players", "teams", "competitions", "coaches", "redirects", "bridges", "overlay_links")))
    # Reep v0's CC0 biography export is frozen. Keep its acquisition separate from
    # v1: v0 IDs are not compatible with v1 IDs, so enrichment joins via provider IDs.
    v0_commit = "b5a19b42975460d271e214743c88b539082559ef"
    v0_url = f"https://raw.githubusercontent.com/withqwerty/reep/{v0_commit}/data/people.csv"
    v0_path = root / "reep-v0-people.csv"
    if not v0_path.exists() or sha(v0_path) != "9c3c1eb59ab149dfd45ba43eceafd8f71820aad0451adee8be9d2d9e279adf7e":
        fetch(v0_url, v0_path)
    if sha(v0_path) != "9c3c1eb59ab149dfd45ba43eceafd8f71820aad0451adee8be9d2d9e279adf7e":
        raise ValueError("Reep v0 people.csv checksum mismatch")
    v0_license_url = f"https://raw.githubusercontent.com/withqwerty/reep/{v0_commit}/LICENSE"
    v0_license_path = root / "reep-v0-LICENSE.txt"
    if not v0_license_path.exists():
        fetch(v0_license_url, v0_license_path)
    if sha(v0_license_path) != "2fb5e423d614313817097977094bac11ac8996498e210175c9470f96b272d55a":
        raise ValueError("Pinned Reep v0 CC0 licence checksum mismatch")
    v0_acquisition = {"url": v0_url, "commit": v0_commit, "snapshot": "2026-06-21", "license": "CC0-1.0",
                      "bytes": v0_path.stat().st_size, "sha256": sha(v0_path),
                      "license_url": v0_license_url, "license_sha256": sha(v0_license_path)}
    (root / "reep-v0-people.acquisition.json").write_text(json.dumps(v0_acquisition, indent=2) + "\n", encoding="utf-8")
    fpl_url = "https://fantasy.premierleague.com/api/bootstrap-static/"
    fetch(fpl_url, root / "fpl-bootstrap.json")
    observed = dt.datetime.now(dt.timezone(dt.timedelta(minutes=330))).date().isoformat()
    metadata = {"observed_on": observed, "sources": [{"url": fpl_url, "downloaded_on": observed,
                "sha256": sha(root / "fpl-bootstrap.json")}, {"url": latest["manifest_url"], "role": "provider-ID crosswalk"}]}
    metadata["sources"].append({"url": v0_url, "commit": v0_commit, "observed_on": "2026-06-21",
                                "role": "frozen CC0 biographical profile source", "sha256": sha(v0_path)})
    # Pin supplementary profiles; their commit date is NOT a 2026 squad date.
    revision = "62ca05786a63f65ebf1220eaa8de6a4ef29abf69"
    for name in ("player_profiles", "team_details", "player_latest_market_value"):
        url = f"https://raw.githubusercontent.com/salimt/football-datasets/{revision}/datalake/transfermarkt/{name}/{name}.csv"
        path = root / f"salimt-{name}.csv"
        fetch(url, path)
        metadata["sources"].append({"url": url, "commit": revision, "observed_on": "2025-10-18", "sha256": sha(path)})
    overlay = release["files"]["csv/overlay_links.csv.gz"]
    overlay_acquisition = {"url": overlay["url"], "stamp": release["stamp"], "license": "CC0-1.0",
                          "bytes": overlay["bytes"], "sha256": overlay["sha256"]}
    (root / "reep-v1-overlay-links.acquisition.json").write_text(json.dumps(overlay_acquisition, indent=2) + "\n", encoding="utf-8")
    metadata["sources"].append({"url": overlay["url"], "stamp": release["stamp"],
                                "role": "Wikidata overlay IDs; DOB-check required for identity use", "sha256": overlay["sha256"]})
    (root / "acquisition.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    print(f"Downloaded inputs as of {observed}; rebuild and validate before installing.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--staging", type=Path, required=True)
    download(parser.parse_args().staging.resolve())
