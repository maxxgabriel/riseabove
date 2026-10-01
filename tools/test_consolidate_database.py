"""Small fictional reconciliation fixtures; no downloaded game data in tests."""
import contextlib
import csv
import gzip
import io
import json
from pathlib import Path
import tempfile
import unittest

from consolidate_database import build, digest, rows, write_rows
from validate_consolidated_database import verify


class ConsolidationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.archive, self.fm, self.staging = [self.root / name for name in ("archive", "fm", "inputs")]
        for path in (self.archive, self.fm, self.staging / "reep"):
            path.mkdir(parents=True)
        write_rows(self.archive / "players.csv", ["player_id", "name", "date_of_birth", "last_season", "current_club_id", "market_value_in_eur", "contract_expiration_date"],
                   [dict(player_id="10", name="Alpha Player", date_of_birth="2000-01-01", last_season="2025", current_club_id="2", market_value_in_eur="0", contract_expiration_date="2027-06-30")])
        write_rows(self.archive / "clubs.csv", ["club_id", "name", "domestic_competition_id", "last_season"],
                   [dict(club_id="1", name="Current Club", domestic_competition_id="GB1", last_season="2025"),
                    dict(club_id="2", name="Previous Club", domestic_competition_id="GB1", last_season="2025")])
        write_rows(self.fm / "players.csv", ["id", "name", "birth_date", "nat"],
                   [dict(id="100", name="Alpha Player", birth_date="1999-01-01", nat="ENG"),
                    dict(id="101", name="Second Player", birth_date="1995-01-01", nat="ENG"),
                    dict(id="102", name="Second Player", birth_date="1996-01-01", nat="ENG")])
        for name, fields in [("player_profiles", ["player_id", "date_of_birth", "player_name"]), ("team_details", ["club_id"]), ("player_latest_market_value", ["player_id", "value"])]:
            write_rows(self.staging / f"salimt-{name}.csv", fields, [])
        tables = {
            "players": (["reep_id", "status", "label", "gender", "country"], [dict(reep_id="rp1", status="active", label="Alpha Player"), dict(reep_id="rp2", status="active", label="Second Player")]),
            "teams": (["reep_id", "status", "label"], [dict(reep_id="rt1", status="active", label="Current Club")]),
            "competitions": (["reep_id", "status", "label"], []),
            "coaches": (["reep_id", "status", "label"], []),
            "redirects": (["from_id", "to_id"], []),
            "bridges": (["provider", "namespace", "external_id", "reep_id", "rung", "upstream_status"], [
                dict(provider=p, namespace=n, external_id=k, reep_id=e) for p, n, k, e in [
                    ("transfermarkt", "spieler", "10", "rp1"), ("fm", "player", "100", "rp1"),
                    ("fm", "player", "101", "rp2"), ("fm", "player", "102", "rp2"),
                    ("transfermarkt", "verein", "1", "rt1"), ("opta", "team_numeric", "3", "rt1"),
                    ("opta", "person_numeric", "123", "rp1")]])}
        release = {"stamp": "fictional", "files": {}}
        for name, (fields, values) in tables.items():
            path = self.staging / "reep" / f"{name}.csv.gz"
            with gzip.open(path, "wt", encoding="utf-8", newline="") as stream:
                writer = csv.DictWriter(stream, fields)
                writer.writeheader()
                writer.writerows(values)
            release["files"][f"csv/{name}.csv.gz"] = dict(sha256=digest(path), bytes=path.stat().st_size)
        (self.staging / "reep-release.json").write_text(json.dumps(release), encoding="utf-8")
        (self.staging / "acquisition.json").write_text(json.dumps(dict(observed_on="2026-09-30")), encoding="utf-8")
        self.fpl = dict(teams=[dict(id=1, code=3, name="Current Club")], elements=[dict(code=123, first_name="Alpha", second_name="Player", birth_date="2000-01-01", team=1, element_type=3, removed=False, status="a")])

    def tearDown(self):
        self.temp.cleanup()

    def run_build(self):
        (self.staging / "fpl-bootstrap.json").write_text(json.dumps(self.fpl), encoding="utf-8")
        output = self.root / "output"
        with contextlib.redirect_stdout(io.StringIO()):
            build(self.archive, self.fm, self.staging, output)
        return output

    def test_conflicting_birthdays_quarantine_both_tm_and_catalog_links(self):
        output = self.run_build()
        conflicts = list(rows(output / "identity_conflicts.csv"))
        self.assertEqual({r["source_id"] for r in conflicts}, {"100", "102"})
        catalog = {r["entity_id"]: r for r in rows(output / "catalog_players.csv")}
        self.assertEqual(catalog["rp1"]["dob"], "2000-01-01")
        self.assertEqual(catalog["fm:100"]["dob"], "1999-01-01")
        self.assertEqual(catalog["fm:102"]["dob"], "1996-01-01")
        links = list(rows(output / "identity_links.csv"))
        self.assertEqual({r["external_id"] for r in links if r["validation"] == "quarantined"}, {"100", "102"})

    def test_changed_club_clears_old_contract_and_preserves_recorded_zero(self):
        output = self.run_build()
        player = next(rows(output / "players.csv"))
        self.assertEqual(player["current_club_id"], "1")
        self.assertEqual(player["contract_expiration_date"], "")
        self.assertEqual(player["market_value_in_eur"], "0")
        self.assertEqual(next(rows(output / "tm_player_records_20260706.csv"))["current_club_id"], "2")
        with contextlib.redirect_stdout(io.StringIO()):
            verify(output)

    def test_name_without_birthdate_never_establishes_identity(self):
        self.fpl["elements"][0].update(code=999, birth_date="")
        output = self.run_build()
        squad = next(rows(output / "fpl_current_squad.csv"))
        self.assertEqual(squad["player_id"], "fpl:999")
        self.assertEqual(squad["dob"], "")
        self.assertEqual(squad["basis"], "independent_provider_id")

    def test_checksum_mismatch_prevents_any_pack_creation(self):
        with (self.staging / "reep/players.csv.gz").open("ab") as stream:
            stream.write(b"changed")
        with self.assertRaisesRegex(ValueError, "checksum"):
            self.run_build()
        self.assertFalse((self.root / "output").exists())


if __name__ == "__main__":
    unittest.main()
