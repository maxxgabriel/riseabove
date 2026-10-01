// usage: node e2e/shots.mjs name=#/route [name=#/route ...]   (env SCHEME=light, W=1440, H=900, DENSITY=compact|comfortable|spacious, SCALE=100)
import { launch, shot, ensureWorld } from "./lib.mjs";
const { browser, page, errors } = await launch({ scheme: process.env.SCHEME ?? "dark" });
if (process.env.DENSITY || process.env.SCALE) {
  const v = JSON.stringify({ density: process.env.DENSITY ?? "comfortable", textScale: Number(process.env.SCALE ?? 100) });
  await page.addInitScript((val) => localStorage.setItem("ra.settings", val), v);
}
if (process.env.W) await page.setViewportSize({ width: Number(process.env.W), height: Number(process.env.H ?? 900) });
await ensureWorld("small");
for (const arg of process.argv.slice(2)) {
  const eq = arg.indexOf("=");
  const [name, route] = [arg.slice(0, eq), arg.slice(eq + 1)];
  await page.goto(`http://127.0.0.1:8787/${route}`);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(500);
  await shot(page, name);
}
console.log(errors.join("\n") || "no console errors");
await browser.close();
