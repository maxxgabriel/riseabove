// Screens for the insights: observer pages, then an inhabited player's Today. usage: node e2e/insights.mjs [person=8]
import { launch, shot, api, waitIdle } from "./lib.mjs";
const who = Number(process.argv[2] ?? 8);
const { browser, page, errors } = await launch({ scheme: process.env.SCHEME ?? "dark" });
await api("persp.inhabit", { person: who, conceal_mine: false });
await waitIdle();
for (const [name, route] of [["ins-today", "#/"], ["ins-me", `#/person/${who}`]]) {
  await page.goto(`http://127.0.0.1:8787/${route}`);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(600);
  await shot(page, name);
}
console.log(errors.join("\n") || "no console errors");
await browser.close();
