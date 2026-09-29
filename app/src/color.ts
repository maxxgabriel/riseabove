// Colour helpers for crests and page tints. Presentation only; nothing here is game state.

export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.replace(/./g, (c) => c + c) : h.padEnd(6, "0").slice(0, 6), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function rgbToHex(r: number, g: number, b: number): string {
  const c = (x: number) => Math.round(Math.max(0, Math.min(255, x))).toString(16).padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}

export function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return [0, 0, l];
  const s = d / (1 - Math.abs(2 * l - 1));
  let h = 0;
  if (max === rn) h = ((gn - bn) / d) % 6;
  else if (max === gn) h = (bn - rn) / d + 2;
  else h = (rn - gn) / d + 4;
  return [(h * 60 + 360) % 360, s, l];
}

export function hslToHex(h: number, s: number, l: number): string {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  const [r, g, b] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return rgbToHex((r + m) * 255, (g + m) * 255, (b + m) * 255);
}

/** A dark, muted version of a colour, good as the tint of a whole page. */
export function tintOf(hex: string): string {
  const [h, s, l] = rgbToHsl(...hexToRgb(hex));
  // Near-greys stay grey; saturated colours are pulled in so a red kit does not paint the page red.
  return hslToHex(h, Math.min(s, 0.55) * (l < 0.08 ? 0.5 : 1), 0.2);
}

/** Black or near-white, whichever reads better on this background. */
export function readableOn(hex: string): string {
  const [r, g, b] = hexToRgb(hex).map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.42 ? "#101412" : "#ffffff";
}

export function hashOf(s: string | number): number {
  const str = String(s);
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/** Colours for things that have none of their own: competitions, nations, people without a club. */
export function brandFor(kind: string, id: number, name: string): [string, string] {
  const palettes: Record<string, [string, string][]> = {
    league: [["#0f5a3f", "#d8e8dc"], ["#0d4d7a", "#dbe8f3"], ["#5b2a86", "#eadcf5"], ["#7a1f2b", "#f3dcdf"], ["#8a5a0a", "#f5e6c8"]],
    cup: [["#8f1d2c", "#f6e3e6"], ["#a0561a", "#f7e6d6"], ["#1b5d73", "#dcecf1"], ["#6b2d86", "#efe1f6"]],
    continental: [["#152a5e", "#dfe6f6"], ["#243b7a", "#e0e6f7"], ["#0e4b5a", "#d9edf1"]],
    supercup: [["#7a5a0a", "#f3ead0"], ["#5a1f5f", "#f0dcf2"]],
    nation: [["#1f4e79", "#e3ecf5"], ["#7a2530", "#f3e2e5"], ["#28623a", "#e1efe5"], ["#5c4a17", "#f2ead0"]],
  };
  const list = palettes[kind] ?? palettes.league;
  return list[hashOf(`${id}:${name}`) % list.length];
}

export function initials(name: string): string {
  const words = name
    .replace(/[^\p{L}\p{N}\s'-]/gu, "")
    .split(/\s+/)
    .filter((w) => w && !/^(fc|afc|cf|sc|the|of|de|del|la|el|ac|as|cd|cup|league)$/i.test(w));
  if (words.length === 0) return name.slice(0, 2).toUpperCase();
  if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
  return (words[0][0] + words[1][0]).toUpperCase();
}
