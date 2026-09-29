// Shared helpers for driving the app in a real browser.
import { chromium } from "playwright-core";
import { fileURLToPath } from "node:url";

export const BASE = process.env.BASE ?? "http://127.0.0.1:8787";
export const SHOTS = fileURLToPath(new URL("./shots/", import.meta.url));

export async function launch(opts = {}) {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium-1194/chrome-linux/chrome", args: ["--no-sandbox"] });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: opts.scheme ?? "dark", deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(`console: ${m.text()}`);
  });
  return { browser, ctx, page, errors };
}

export async function api(method, args = {}) {
  const r = await fetch(`${BASE}/api/${method}`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(args) });
  const t = await r.text();
  if (!r.ok) throw new Error(`${method}: ${t}`);
  return t ? JSON.parse(t) : null;
}

export async function ensureWorld(scale = "small") {
  let st = await api("world.status");
  if (!st.open) {
    await api("world.new", { kind: "synthetic", scale });
    for (let i = 0; i < 200; i++) {
      await new Promise((r) => setTimeout(r, 200));
      st = await api("world.status");
      if (st.open && !st.task.running) break;
    }
  }
  return st;
}

export async function waitIdle() {
  for (let i = 0; i < 600; i++) {
    const st = await api("world.status");
    if (!st.job.running && !st.task.running) return st;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error("still busy");
}

export async function shot(page, name) {
  await page.screenshot({ path: `${SHOTS}${name}.png` });
}
