// Visits every page against a real server and fails on console errors or a page that failed to draw.
// usage: node e2e/smoke.mjs   (server on BASE, default http://127.0.0.1:8787)
import { launch, api, waitIdle, BASE } from "./lib.mjs";

await api("world.new", { kind: "synthetic", scale: "small" });
for (let i = 0; i < 300; i++) {
  await new Promise((r) => setTimeout(r, 200));
  const st = await api("world.status");
  if (st.open && !st.task.running) break;
}
await api("persp.observe");
await api("advance.start", { mode: "days", n: 45 });
await waitIdle();

const first = async (table, filters = {}) => (await api("table.query", { table, filters, limit: 1 })).rows[0];
const player = (await first("players", { kind: "first", inhabitable: true })).id;
const club = (await first("clubs")).id;
const match = (await first("fixtures", { played: true })).open.id;

const SOCIETY = ["posts", "chants", "memes", "groups", "rivalries", "conferences", "quotes", "outlets", "journalists", "grapevine", "incidents", "referees", "controversies", "charges", "record_book", "records_broken", "votes", "hall_members", "chronicle", "schools", "rule_changes", "institutions", "minor_seasons"];
const observer = [
  "overview", "people", `person/${player}`, ...["attributes", "stats", "career", "events"].map((t) => `person/${player}/${t}`),
  "clubs", `club/${club}`, ...["squad", "staff", "fixtures", "finances", "board", "fans", "room", "history"].map((t) => `club/${club}/${t}`),
  "comps", "comp/0", ...["fixtures", "leaders", "history", "rules"].map((t) => `comp/0/${t}`), "nations", "nation/0",
  "fixtures", "transfers", "events", "history", "history?tab=awards", "staff", `match/${match}`,
  "society", ...SOCIETY.map((t) => `society/${t}`), "society/nope",
  `compare?ids=${player}`, "bookmarks", "settings", "help", "diagnostics", "saves", "inhabit", "nope/nothing",
];
const inhabited = ["today", "messages", "calendar", "football", "contract", "life", "relationships", "press", "social", "journal", "society", "society/posts", "society/incidents", "me", `person/${player}`];

const { browser, page, errors } = await launch();
page.on("response", async (r) => {
  if (r.status() >= 400 && r.url().includes("/api/")) errors.push(`api ${r.status()} ${r.request().postData()} -> ${(await r.text()).slice(0, 160)}`);
});
let bad = 0;
async function visit(route) {
  const before = errors.length;
  await page.goto(`${BASE}/#/${route}`);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(350);
  const text = await page.locator("main").innerText();
  const problems = [];
  if (/failed to draw/i.test(text)) problems.push("page failed to draw");
  if (!(await page.locator("main h1").count()) && !route.startsWith("nope")) problems.push("no heading");
  problems.push(...errors.slice(before));
  console.log(`${problems.length ? "FAIL" : "ok  "} #/${route}${problems.length ? "  " + problems.join(" | ") : ""}`);
  if (problems.length) bad++;
}
for (const r of observer) await visit(r);
await api("persp.inhabit", { person: player, conceal_mine: true });
for (const r of inhabited) await visit(r);
await api("advance.start", { mode: "until_match" });
await waitIdle();
await visit("today");
await visit("messages");
await browser.close();
console.log(bad ? `${bad} page(s) with problems` : "all pages drew without errors");
process.exit(bad ? 1 : 0);
