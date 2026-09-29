// Screens in the Football Manager style. usage: node e2e/fm.mjs [route ...]   (routes like "#/comp/0")
import { launch, shot, api } from "./lib.mjs";
const routes = process.argv.slice(2);
const wide = process.env.WIDE === "1";
const { browser, ctx, page, errors } = await launch({ scheme: process.env.SCHEME ?? "dark" });
if (wide) await page.setViewportSize({ width: 1900, height: 940 });
if (process.env.W) await page.setViewportSize({ width: Number(process.env.W), height: Number(process.env.H ?? 800) });
for (const r of routes.length ? routes : ["#/comp/0"]) {
  await page.goto(`http://127.0.0.1:8787/${r}`);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(700);
  const name = "fm-" + r.replace(/[#/]+/g, "-").replace(/^-|-$/g, "") + (wide ? "-wide" : "") + (process.env.W ? "-w" + process.env.W : "") + (process.env.SCHEME === "light" ? "-light" : "");
  await shot(page, name);
  console.log("shot", name);
}
console.log(errors.join("\n") || "no console errors");
await browser.close();
