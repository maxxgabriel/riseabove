// Local source browser review. Source data and screenshots stay outside tracked data folders.
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { launch, api, BASE, waitIdle } from "./lib.mjs";

const archive = process.env.ARCHIVE;
const fm = process.env.FM;
const output = process.env.SHOT_DIR;
assert(archive && fm && output, "Set ARCHIVE, FM and SHOT_DIR to local folders.");
await mkdir(output, { recursive: true });
if (process.env.IMPORT_WORLD === "1") await api("world.close");
const a = await api("database.attach", { dir: archive });
const f = await api("database.attach", { dir: fm });
const { browser, page, errors } = await launch();
try {
  await page.goto(`${BASE}/#/database?source=${a.id}&table=players.csv`);
  await page.locator(".database-results-meta").waitFor({ timeout: 60000 });
  for (const [width, height] of [[1366, 768], [1440, 900], [1920, 1080], [390, 844]]) {
    await page.setViewportSize({ width, height });
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `horizontal overflow at ${width}`);
    await page.screenshot({ path: `${output}/database-${width}.png` });
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.getByRole("button", { name: "Next", exact: true }).click();
  await page.waitForFunction(() => location.hash.includes("offset=50"));
  await page.locator(".database-pagination").getByText("51–100", { exact: false }).waitFor();
  await page.getByRole("button", { name: "Previous", exact: true }).click();
  await page.locator(".database-pagination").getByText("1–50", { exact: false }).waitFor();
  const id = await page.locator(".database-detail dl > div").filter({ has: page.locator("dt", { hasText: /^player id$/i }) }).locator("dd").evaluate((el) => el.childNodes[0].textContent.trim());
  await page.locator(".database-links button").filter({ hasText: /^appearances$/ }).first().click();
  await page.waitForFunction(() => location.hash.includes("appearances.csv"));
  await page.locator(".database-table-head h2").getByText("appearances", { exact: true }).waitFor();
  await page.locator(".database-results-meta").waitFor({ timeout: 60000 });
  const selectedId = await page.locator(".database-detail dl > div").filter({ has: page.locator("dt", { hasText: /^player id$/i }) }).locator("dd").evaluate((el) => el.childNodes[0].textContent.trim());
  assert.equal(selectedId, id, "related records use the same source ID");
  assert(await page.getByLabel("Name search", { exact: true }).isDisabled());
  await page.screenshot({ path: `${output}/database-history.png` });
  await page.goto(`${BASE}/#/database?source=${f.id}&table=clubs_usable.csv&search=arsenal`);
  await page.locator(".database-results-meta").waitFor({ timeout: 60000 });
  await page.getByText("FM23 · verified exported names; relations unknown", { exact: true }).waitFor();
  assert.equal(await page.getByRole("link", { name: "Open in world →" }).count(), 0);
  await page.evaluate(() => document.querySelector(".database-standalone")?.scrollTo(0, 0));
  await page.screenshot({ path: `${output}/database-fm23.png` });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.evaluate(() => document.documentElement.style.fontSize = "150%");
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "overflow with larger text");
  await page.keyboard.press("Tab");
  assert(await page.evaluate(() => document.activeElement !== document.body), "keyboard focus reaches controls");
  await page.screenshot({ path: `${output}/database-larger-text.png` });
  if (process.env.IMPORT_WORLD === "1") {
    await api("world.new", { kind: "import", dir: archive, seed: 1 });
    await waitIdle();
    let linked;
    for (let offset = 0; offset < 5000 && !linked; offset += 100) {
      const rows = await api("database.query", { source: a.id, table: "players.csv", offset, limit: 100 });
      linked = rows.rows.find((r) => r.refs?.player_id);
    }
    const person = linked?.refs.player_id;
    assert(person, "imported row connects to a world person");
    await page.reload(); // External API setup changed the session; reload its shell status.
    await page.goto(`${BASE}/#/person/${person.id}`);
    await page.getByText("Source records", { exact: true }).waitFor({ timeout: 60000 });
    await page.getByRole("link", { name: "Source records", exact: true }).click();
    await page.locator(".database-detail h3").waitFor();
    assert(locationSafe(page.url()).includes(`value=${linked.fields.player_id}`));
    await page.screenshot({ path: `${output}/database-world-link.png` });
    await api("persp.inhabit", { person: person.id });
    await page.reload();
    await page.getByRole("heading", { name: "Observer view", exact: true }).waitFor();
    await page.getByRole("button", { name: "Switch to observer", exact: true }).click();
    await page.locator(".database-results-meta").waitFor();
    assert.equal((await api("world.status")).perspective.mode, "observer");
  }
  assert.deepEqual(errors, []);
  console.log("Database browser review passed: responsive layouts, pagination, exact source relationships, FM separation, text scaling and keyboard focus.");
} finally { await browser.close(); }
function locationSafe(url) { return decodeURIComponent(url); }
