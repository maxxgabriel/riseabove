import { launch, shot, api, ensureWorld, waitIdle } from "./lib.mjs";
const { browser, page, errors } = await launch();
await ensureWorld("small");
const st0 = await api("world.status");
if (st0.perspective.mode === "inhabit") await api("persp.observe");
await page.goto("http://127.0.0.1:8787/#/inhabit?kind=first&age_min=21&age_max=28");
await page.waitForSelector(".dt-row");
await page.waitForTimeout(400);
await shot(page, "inhabit-list");
await page.locator(".dt-row").nth(3).click();
await page.waitForSelector("dialog[open]");
await shot(page, "inhabit-dialog");
await page.getByRole("button", { name: "Inhabit", exact: true }).click();
await page.waitForURL(/#\/today/);
await page.waitForTimeout(600);
await shot(page, "today-1");
// advance until something needs us
await page.keyboard.press("Control+Enter");
await page.waitForTimeout(300);
await shot(page, "advancing");
await waitIdle();
await page.waitForTimeout(600);
await shot(page, "today-2");
for (const [n, r] of [["messages", "#/messages"], ["calendar", "#/calendar"], ["football", "#/football"], ["contract", "#/contract"]]) {
  await page.goto(`http://127.0.0.1:8787/${r}`);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(500);
  await shot(page, n);
}
console.log(errors.join("\n") || "no console errors");
await browser.close();
