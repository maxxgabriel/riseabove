import { launch, shot, BASE } from "./lib.mjs";
const { browser, page, errors } = await launch();
await page.goto(`${BASE}/#/overview`);
await page.waitForTimeout(600);
await page.keyboard.press("Control+k");
await page.waitForTimeout(300);
await page.keyboard.type("vemar");
await page.waitForTimeout(700);
await shot(page, "k-search");
await page.keyboard.press("ArrowDown");
await page.keyboard.press("Enter");
await page.waitForTimeout(700);
console.log("after search:", page.url());
// Tab order sanity: first few focus stops
await page.goto(`${BASE}/#/people`);
await page.waitForTimeout(600);
const stops = [];
for (let i = 0; i < 14; i++) {
  await page.keyboard.press("Tab");
  stops.push(await page.evaluate(() => { const a = document.activeElement; return (a?.getAttribute("aria-label") || a?.textContent || a?.tagName || "").trim().slice(0, 30) + " <" + a?.tagName + ">"; }));
}
console.log(stops.join("\n"));
await shot(page, "k-focus");
console.log(errors.length ? errors : "no console errors");
await browser.close();
