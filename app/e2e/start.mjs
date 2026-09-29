import { launch, api, waitIdle, shot, BASE } from "./lib.mjs";
await api("world.close");
const { browser, page, errors } = await launch();
await page.goto(`${BASE}/`);
await page.waitForTimeout(800);
await shot(page, "start-1");
console.log(await page.locator("button, a").allInnerTexts());
await browser.close();
console.log(errors.length ? errors : "no console errors");
