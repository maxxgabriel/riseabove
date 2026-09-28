import { launch, shot, api, ensureWorld } from "./lib.mjs";
const { browser, page, errors } = await launch();
await ensureWorld("small");
await page.goto("http://127.0.0.1:8787/#/people");
await page.waitForSelector(".dt-row", { timeout: 8000 });
await page.waitForTimeout(400);
await shot(page, "people");
console.log(errors.join("\n") || "no errors");
await browser.close();
