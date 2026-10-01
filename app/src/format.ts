import { getSettings } from "./settings";
import { getStatus } from "./store";
import type { Fmt } from "./types";

const DAY = 86_400_000;
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_LONG = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAYS_LONG = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

export const toDate = (d: number) => new Date(d * DAY);
export const weekday = (d: number, long = false) => (long ? WEEKDAYS_LONG : WEEKDAYS)[toDate(d).getUTCDay()];
export const monthName = (m: number, long = false) => (long ? MONTHS_LONG : MONTHS)[m];

/** A date in an explicit style. */
export function dateIn(style: "short" | "iso" | "us", d: number, withYear = true): string {
  const t = toDate(d);
  const y = t.getUTCFullYear();
  const m = t.getUTCMonth();
  const day = t.getUTCDate();
  const mm = String(m + 1).padStart(2, "0");
  const dd = String(day).padStart(2, "0");
  if (style === "iso") return withYear ? `${y}-${mm}-${dd}` : `${mm}-${dd}`;
  if (style === "us") return withYear ? `${mm}/${dd}/${y}` : `${mm}/${dd}`;
  return withYear ? `${day} ${MONTHS[m]} ${y}` : `${day} ${MONTHS[m]}`;
}

/** Days since 1970-01-01 to a calendar date, as the person chose to read it. */
export function date(d: number | null | undefined, opts: { year?: boolean } = {}): string {
  if (d == null || !Number.isFinite(d)) return "";
  return dateIn(getSettings().dateStyle, d, opts.year !== false);
}

/** Sentences written by the simulation carry dates as 2026-08-24; show them the way the reader prefers. */
export function prose(text: string): string {
  return text.replace(/\b(\d{4})-(\d{2})-(\d{2})\b/g, (m, y, mo, d) => {
    const days = Date.UTC(Number(y), Number(mo) - 1, Number(d)) / DAY;
    return Number.isFinite(days) ? date(days) : m;
  });
}

export function dateLong(d: number): string {
  const t = toDate(d);
  return `${WEEKDAYS_LONG[t.getUTCDay()]} ${t.getUTCDate()} ${MONTHS_LONG[t.getUTCMonth()]} ${t.getUTCFullYear()}`;
}

export function year(d: number): number {
  return toDate(d).getUTCFullYear();
}

export function ordinal(n: number): string {
  const v = n % 100;
  if (v >= 11 && v <= 13) return `${n}th`;
  switch (n % 10) {
    case 1:
      return `${n}st`;
    case 2:
      return `${n}nd`;
    case 3:
      return `${n}rd`;
    default:
      return `${n}th`;
  }
}

const int = new Intl.NumberFormat("en-GB", { maximumFractionDigits: 0 });
const inr = new Intl.NumberFormat("en-IN", { maximumFractionDigits: 0 });
export const fmtInt = (n: number) => int.format(n);

/** Money, compact by default. `exact` shows every digit. */
export function money(v: number | null | undefined, opts: { exact?: boolean; sign?: boolean } = {}): string {
  if (v == null || !Number.isFinite(v)) return "";
  const sym = getSettings().currency || getStatus().currency || "£";
  const a = Math.abs(v);
  const sign = v < 0 ? "−" : opts.sign && v > 0 ? "+" : "";
  if (sym === "₹") {
    // Indian grouping, and lakh and crore for large sums.
    if (opts.exact || a < 100_000) return `${sign}₹${inr.format(a)}`;
    if (a < 10_000_000) return `${sign}₹${(a / 100_000).toFixed(a < 1_000_000 ? 2 : 1).replace(/\.?0+$/, "")} lakh`;
    return `${sign}₹${(a / 10_000_000).toFixed(a < 100_000_000 ? 2 : 1).replace(/\.?0+$/, "")} crore`;
  }
  if (opts.exact || a < 10_000) return `${sign}${sym}${int.format(a)}`;
  const c = (x: number, unit: string, digits: number) => {
    const s = x.toFixed(digits).replace(/\.0+$/, "").replace(/(\.\d*[1-9])0+$/, "$1");
    return `${sign}${sym}${s}${unit}`;
  };
  if (a < 1_000_000) return c(a / 1_000, "k", a < 100_000 ? 1 : 0);
  if (a < 1_000_000_000) return c(a / 1_000_000, "m", a < 10_000_000 ? 2 : a < 100_000_000 ? 1 : 0);
  return c(a / 1_000_000_000, "bn", 2);
}

export function byFmt(fmt: Fmt, n: number): string {
  if (!Number.isFinite(n)) return "";
  switch (fmt) {
    case "int":
      return int.format(n);
    case "dec1":
      return n.toFixed(1);
    case "dec2":
      return n.toFixed(2);
    case "money":
      return money(n);
    case "date":
      return date(n);
    case "pct":
      return `${Math.round(n * 100)}%`;
    case "ordinal":
      return ordinal(n);
    default:
      return String(n);
  }
}

export function plural(n: number, one: string, many = `${one}s`): string {
  return `${fmtInt(n)} ${n === 1 ? one : many}`;
}

/** Age in whole years given a date of birth and today, both in days. */
export function ageAt(dob: number, today: number): number {
  const a = toDate(dob);
  const b = toDate(today);
  let age = b.getUTCFullYear() - a.getUTCFullYear();
  if (b.getUTCMonth() < a.getUTCMonth() || (b.getUTCMonth() === a.getUTCMonth() && b.getUTCDate() < a.getUTCDate())) age -= 1;
  return age;
}

export function relativeDays(d: number, today: number): string {
  const n = d - today;
  if (n === 0) return "today";
  if (n === 1) return "tomorrow";
  if (n === -1) return "yesterday";
  if (n > 0) return `in ${n} days`;
  return `${-n} days ago`;
}

export function duration(days: number): string {
  if (days < 0) return "ended";
  if (days < 14) return plural(days, "day");
  if (days < 90) return plural(Math.round(days / 7), "week");
  if (days < 730) return plural(Math.round(days / 30.4), "month");
  return `${(days / 365.25).toFixed(1)} years`;
}

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

export function timeAgo(secs: number): string {
  const s = Math.max(0, Date.now() / 1000 - secs);
  if (s < 60) return "just now";
  if (s < 3600) return plural(Math.floor(s / 60), "minute") + " ago";
  if (s < 86400) return plural(Math.floor(s / 3600), "hour") + " ago";
  return plural(Math.floor(s / 86400), "day") + " ago";
}

/** Upper-case the first letter; an empty string stays empty (indexing `s[0]` of one throws). */
export function cap(s: string | null | undefined): string {
  if (!s) return "";
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Up to `n` initials of a name; a name with no letters gives "?". */
export function initials(name: string | null | undefined, n = 2): string {
  const out = (name ?? "")
    .split(/\s+/)
    .map((w) => w.charAt(0))
    .filter(Boolean)
    .join("")
    .slice(0, n)
    .toUpperCase();
  return out || "?";
}

/** Where a word band sits, as a gauge fill (0-100): the word's place among the words, never an engine number. */
export function bandFill(b: { step: number; steps: number }): number {
  return b.steps > 0 ? (b.step / b.steps) * 100 : 0;
}

/** How a feeling pulls on someone, in words. */
export function pullWord(pull: "pos" | "neg" | "flat" | string): string {
  return pull === "pos" ? "Lifts you" : pull === "neg" ? "Weighs on you" : "Neither way";
}
