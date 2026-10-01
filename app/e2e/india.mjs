// Plays the India route in a real browser: begins a life, lets time pass, and visits every page the person can reach.
// Fails on console errors, API errors, a page that failed to draw, and text that shows a broken value (NaN, undefined, null, [object).
// usage: CHROMIUM="C:/Program Files/Google/Chrome/Application/chrome.exe" node e2e/india.mjs   (server on BASE with --static app/dist)
import { launch, api, waitIdle, BASE } from "./lib.mjs";

async function until(cond) {
  for (let i = 0; i < 600; i++) {
    const st = await api("world.status");
    if (cond(st)) return st;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error("timed out");
}

await api("world.new", { kind: "india", scale: "tiny", seed: 21 });
await until((s) => s.open && !s.task.running);
const opts = await api("route.options");
const district = opts.states[0].districts[0].id;
let me = null;
for (const start of ["state_league", "school_standout", "university_freshman"]) {
  try {
    me = (await api("route.begin", { start, district })).person;
    break;
  } catch (e) {
    console.log(`start ${start}: ${String(e.message).slice(0, 80)}`);
  }
}
if (me == null) throw new Error("no start could be begun");
await api("advance.start", { mode: "days", n: 60 });
await waitIdle();

const first = async (table, filters = {}) => (await api("table.query", { table, filters, limit: 1 })).rows[0];
const club = (await first("clubs")).id;
const comp = (await first("comps")).id;
const fx = await first("fixtures", { played: true });
const match = fx ? fx.open.id : null;
const inbox = await api("me.inbox");
const thread = inbox.threads[0]?.id;

const routes = [
  "today", "messages", thread != null ? `messages?thread=${thread}` : "messages", "messages?view=chats", "story", "news", "calendar", "football", "contract", "life", "relationships", "press", "social", "journal", "me",
  `person/${me}`, ...["attributes", "stats", "career", "events"].map((t) => `person/${me}/${t}`), "overview", "people", "clubs", `club/${club}`,
  ...["squad", "staff", "fixtures", "finances", "board", "fans", "room", "history"].map((t) => `club/${club}/${t}`),
  "comps", `comp/${comp}`, ...["table", "fixtures", "leaders", "history", "rules"].map((t) => `comp/${comp}/${t}`), "nations", "nation/0", "fixtures", "transfers", "events", "history", "staff",
  match != null ? `match/${match}` : "fixtures", "society", "society/posts", "society/incidents", "society/rivalries", "development", "bookmarks", "settings", "help", "diagnostics", "saves",
];

const { browser, page, errors } = await launch();
page.on("response", async (r) => {
  if (r.status() >= 400 && r.url().includes("/api/")) errors.push(`api ${r.status()} ${r.request().postData()} -> ${(await r.text()).slice(0, 160)}`);
});
const BROKEN = /\bNaN\b|\bundefined\b|\[object|\bInfinity\b|\{\{|\}\}|\bnull\b/;
let bad = 0;
async function visit(route) {
  const before = errors.length;
  await page.goto(`${BASE}/#/${route}`);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(300);
  const text = await page.locator("main").innerText();
  const problems = [];
  if (/failed to draw/i.test(text)) problems.push("page failed to draw");
  if (!(await page.locator("main h1").count())) problems.push("no heading");
  const m = text.match(BROKEN);
  if (m) problems.push(`shows ${m[0]} near: ${text.slice(Math.max(0, m.index - 40), m.index + 40).replace(/\s+/g, " ")}`);
  problems.push(...errors.slice(before));
  console.log(`${problems.length ? "FAIL" : "ok  "} #/${route}${problems.length ? "  " + problems.join(" | ") : ""}`);
  if (problems.length) bad++;
}
for (const r of routes) await visit(r);
// Time passes, and the pages still draw.
for (const r of ["today", "messages", "contract"]) {
  await api("advance.start", { mode: "days", n: 30 });
  await waitIdle();
  await visit(r);
}
await browser.close();
console.log(bad ? `${bad} page(s) with problems` : "all pages drew without errors");
process.exit(bad ? 1 : 0);
