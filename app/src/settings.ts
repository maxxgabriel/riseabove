import { useSyncExternalStore } from "react";

export type Theme = "system" | "light" | "dark";
export type Density = "compact" | "comfortable" | "spacious";
export type DateStyle = "short" | "iso" | "us";

export interface Settings {
  theme: Theme;
  density: Density;
  textScale: number; // percent
  dateStyle: DateStyle;
  currency: string; // a symbol, or "" for the world's own money; the simulation has one unit of account
  reduceMotion: boolean;
  railCollapsed: boolean;
  autosave: boolean;
}

const KEY = "ra.settings";
const defaults: Settings = {
  theme: "system",
  density: "comfortable",
  textScale: 100,
  dateStyle: "short",
  currency: "",
  reduceMotion: false,
  railCollapsed: false,
  autosave: true,
};

function read(): Settings {
  try {
    return { ...defaults, ...(JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Settings>) };
  } catch {
    return { ...defaults };
  }
}

let current = read();
const listeners = new Set<() => void>();

export function applySettings(s: Settings) {
  const root = document.documentElement;
  const dark = s.theme === "dark" || (s.theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  root.dataset.theme = dark ? "dark" : "light";
  root.dataset.density = s.density;
  root.dataset.motion = s.reduceMotion ? "reduce" : "full";
  root.style.setProperty("--text-scale", String(s.textScale / 100));
}

export function getSettings(): Settings {
  return current;
}

export function setSettings(patch: Partial<Settings>) {
  current = { ...current, ...patch };
  try {
    localStorage.setItem(KEY, JSON.stringify(current));
  } catch {
    /* storage may be unavailable; the setting still applies for this session */
  }
  applySettings(current);
  listeners.forEach((l) => l());
}

export function useSettings(): Settings {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => current,
  );
}

if (typeof matchMedia !== "undefined") {
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (current.theme === "system") applySettings(current);
  });
}
