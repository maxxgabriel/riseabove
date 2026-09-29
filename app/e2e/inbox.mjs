// Plays a career until something needs an answer, then answers it and replies to a conversation through the real UI.
// usage: node e2e/inbox.mjs   (server on BASE, default http://127.0.0.1:8787)
import { launch, api, waitIdle, BASE, shot } from "./lib.mjs";

let failed = 0;
const check = (ok, what) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
  if (!ok) failed++;
};

await api("world.new", { kind: "synthetic", scale: "small" });
for (let i = 0; i < 300; i++) {
  await new Promise((r) => setTimeout(r, 200));
  const st = await api("world.status");
  if (st.open && !st.task.running) break;
}
await api("persp.observe");
await api("advance.start", { mode: "days", n: 30 });
await waitIdle();
await api("world.save", { file: "e2e-inbox-base" });

// Someone whose contract is running out gets offers and renewal talks. Try the best of them until one hears something.
const candidates = (await api("table.query", { table: "players", filters: { kind: "first", inhabitable: true, expiring_days: 365 }, sort: { key: "rating", desc: true }, limit: 8 })).rows;
let awaiting = 0;
for (const row of candidates) {
  await api("world.load", { file: "e2e-inbox-base.pws" });
  await waitIdle();
  await api("persp.inhabit", { person: row.id, conceal_mine: false });
  for (let i = 0; i < 14 && awaiting === 0; i++) {
    await api("advance.start", { mode: "days", n: 30 });
    await waitIdle();
    awaiting = (await api("me.messages")).awaiting ?? 0;
  }
  if (awaiting > 0) break;
}
console.log(`awaiting ${awaiting} after playing on`);

const { browser, page, errors } = await launch();
await page.goto(`${BASE}/#/messages`);
await page.waitForSelector(".msglist, .empty", { timeout: 10000 });
await page.waitForTimeout(400);
const threads = await page.locator(".msglist > li").count();
check(threads > 0, `the inbox lists conversations (${threads})`);
// the tab strip belongs right under the heading; a page grid with too few rows once pushed it to the middle of the window
const tabsTop = await page.locator(".inbox-page .tabs").evaluate((el) => el.getBoundingClientRect().top);
const headBottom = await page.locator(".inbox-page h1").evaluate((el) => el.getBoundingClientRect().bottom);
check(tabsTop - headBottom < 90, `the tabs sit under the heading (${Math.round(tabsTop - headBottom)}px apart)`);

// open the one that needs an answer, else the first
const urgent = page.locator(".msglist a.msg.urgent").first();
const target = (await urgent.count()) ? urgent : page.locator(".msglist a.msg").first();
await target.click();
await page.waitForSelector(".thread", { timeout: 10000 });
await shot(page, "inbox-thread");
check(true, "a conversation opens");

// answer a decision, if one is waiting
const choose = page.locator(".answer-buttons button:not([disabled]), .optlist button:not([disabled])").first();
if (await choose.count()) {
  const before = (await api("world.status")).awaiting ?? 0;
  await choose.click();
  await page.waitForSelector("dialog[open]");
  await shot(page, "inbox-confirm");
  await page.locator("dialog[open] footer button.primary, dialog[open] .dialog-foot button.primary, dialog[open] button.btn-primary").last().click().catch(async () => {
    await page.locator("dialog[open] button").last().click();
  });
  await page.waitForTimeout(600);
  const after = (await api("world.status")).awaiting ?? 0;
  check(after < before || (await page.locator(".answer .note").count()) > 0, `answering a decision is recorded (awaiting ${before} -> ${after})`);
  await shot(page, "inbox-answered");
} else {
  console.log("skip  no decision was waiting in the opened conversation");
}

// reply to a message, if the world offered one
await page.goto(`${BASE}/#/messages`);
await page.waitForTimeout(500);
const all = await api("me.inbox");
const withReply = (all.threads ?? []).find((t) => t.needs_action === false && t.count > 0);
let replied = false;
for (const t of (all.threads ?? []).slice(0, 12)) {
  await page.goto(`${BASE}/#/messages/t/${t.id}`);
  await page.waitForSelector(".thread");
  const btn = page.locator(".replybar button:not([disabled])").first();
  if (await btn.count()) {
    const label = (await btn.innerText()).trim();
    await btn.click();
    await page.waitForTimeout(600);
    const noted = await page.locator(".thread .note").filter({ hasText: /You replied/ }).count();
    check(noted > 0, `replying "${label}" is recorded in the conversation`);
    await shot(page, "inbox-replied");
    replied = true;
    break;
  }
}
if (!replied) console.log(`skip  none of the ${(all.threads ?? []).length} conversations offered a reply`);
void withReply;

// a reply is an intent: it shows as queued on Today until the next day
await page.goto(`${BASE}/#/today`);
await page.waitForLoadState("networkidle");
await page.waitForTimeout(400);
const today = await page.locator("main").innerText();
check(!/failed to draw/i.test(today), "Today draws after replying");

check(errors.length === 0, `no console errors${errors.length ? ": " + errors.join(" | ") : ""}`);
await browser.close();
console.log(failed ? `${failed} check(s) failed` : "inbox flow passed");
process.exit(failed ? 1 : 0);
