// Run against a disposable server session; never against a user's open career.
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { api, BASE, launch, waitIdle } from "./lib.mjs";

const output = process.env.SHOT_DIR;
assert(output, "Set SHOT_DIR to a temporary output folder.");
await mkdir(output, { recursive: true });
await api("world.close");
const installed = (await api("world.datasets")).datasets[0];
assert(installed, "Installed database is discoverable");
const { browser, page, errors } = await launch();
try {
  await page.goto(BASE);
  await page.getByRole("button", { name: "Use installed database", exact: true }).waitFor();
  await page.getByRole("button", { name: "Use installed database", exact: true }).click();
  assert.equal(await page.getByLabel("Folder with the dataset", { exact: true }).inputValue(), installed.path);
  for (const [width, height] of [[1440, 900], [390, 844]]) {
    await page.setViewportSize({ width, height });
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "start layout fits viewport");
    await page.screenshot({ path: `${output}/installed-database-${width}.png` });
  }
  const connected = await api("database.attach", { dir: installed.path });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`${BASE}/#/database?source=${connected.id}&table=catalog_players.csv&search=haaland`);
  await page.locator(".database-results-meta").waitFor({ timeout: 60000 });
  await page.screenshot({ path: `${output}/consolidated-catalog.png` });
  const catalog = await api("database.query", { source: connected.id, table: "catalog_players.csv", search: "haaland", limit: 100 });
  assert.equal(catalog.total, installed.database.catalog_players);
  assert(catalog.rows.some((r) => r.fields.fm_id?.split(";").includes("29179241")), "FM identity is reconciled into catalog");
  await api("world.load", { file: "consolidated-db-review.pws" });
  await waitIdle();
  const status = await api("world.status");
  assert.equal(status.date, 20733, "seven-day imported career reloads at 7 October 2026");
  const actual = await api("database.query", { source: connected.id, table: "players.csv", column: "player_id", value: "418560", limit: 1 });
  assert.equal(actual.matched, 1);
  const person = actual.rows[0].refs?.player_id;
  assert(person, "reconciled ID opens a simulated player");
  // The world was loaded through the external harness, so refresh the shell's
  // session status before navigating to an in-world route.
  await page.reload();
  await page.goto(`${BASE}/#/person/${person.id}`);
  await page.getByText("Source records", { exact: true }).waitFor({ timeout: 60000 });
  await page.screenshot({ path: `${output}/consolidated-player.png` });
  const legacy = await api("database.query", { source: connected.id, table: "fm23_player_records.csv", column: "id", value: "29179241", limit: 1 });
  assert.equal(legacy.matched, 1);
  assert(!legacy.rows[0].refs, "historical FM UID is not misread as a Transfermarkt player ID");
  await api("persp.inhabit", { person: person.id });
  const denied = await fetch(`${BASE}/api/database.query`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ source: connected.id, table: "catalog_players.csv" }) });
  assert.equal(denied.status, 403, "raw catalog is still observer-only");
  await api("persp.observe");
  assert.deepEqual(errors, []);
  console.log("Installed pack, catalog reconciliation, player links, save/reload, responsive start and visibility checks passed.");
} finally { await browser.close(); }
