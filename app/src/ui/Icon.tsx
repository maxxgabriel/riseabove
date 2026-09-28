import type { SVGProps } from "react";

// A small hand-drawn set on a 16px grid. Strokes only, so colour follows the text.
const paths = {
  overview: "M2.5 2.5h4.5v4.5H2.5zM9 2.5h4.5v4.5H9zM2.5 9H7v4.5H2.5zM9 9h4.5v4.5H9z",
  calendar: "M2.75 3.5h10.5v10H2.75zM2.75 6.5h10.5M5.5 2v3M10.5 2v3",
  people: "M6 7a2.25 2.25 0 1 0 0-4.5A2.25 2.25 0 0 0 6 7zM1.75 13.5c0-2.4 1.9-4 4.25-4s4.25 1.6 4.25 4M10.5 3a2.25 2.25 0 0 1 0 4.4M12 9.7c1.4.5 2.25 1.7 2.25 3.8",
  club: "M8 1.75 13 3.5v4c0 3-2 5-5 6.75C5 12.5 3 10.5 3 7.5v-4z",
  trophy: "M5 2.5h6v4a3 3 0 0 1-6 0zM5 4H2.75c0 2 .9 3 2.4 3.25M11 4h2.25c0 2-.9 3-2.4 3.25M8 9.5V12M5.5 13.5h5",
  globe: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM2 8h12M8 2c1.8 1.7 2.6 3.7 2.6 6S9.8 12.3 8 14C6.2 12.3 5.4 10.3 5.4 8S6.2 3.7 8 2z",
  list: "M5.5 4h8M5.5 8h8M5.5 12h8M2.5 4h.01M2.5 8h.01M2.5 12h.01",
  transfer: "M2.5 5.5h10M10 3l2.75 2.5L10 8M13.5 10.5h-10M6 8l-2.75 2.5L6 13",
  pulse: "M1.5 8h3l1.75-4.5L9 12.5 10.75 8h3.75",
  clock: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM8 4.5V8l2.25 1.5",
  star: "M8 2l1.85 4 4.4.5-3.3 3 .95 4.3L8 11.55 4.1 13.8l.95-4.3-3.3-3L6.15 6z",
  bookmark: "M4 2.5h8v11L8 10.75 4 13.5z",
  sliders: "M2.5 4.5h6M11.5 4.5h2M2.5 11.5h2M7.5 11.5h6M10 3v3M5.5 10v3",
  help: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM6.25 6.25a1.75 1.75 0 1 1 2.5 1.6c-.5.3-.75.7-.75 1.15M8 11.25h.01",
  search: "M7 12A5 5 0 1 0 7 2a5 5 0 0 0 0 10zM10.75 10.75 14 14",
  down: "M4 6l4 4 4-4",
  up: "M4 10l4-4 4 4",
  right: "M6 4l4 4-4 4",
  left: "M10 4 6 8l4 4",
  arrowLeft: "M13 8H3M7 4 3 8l4 4",
  arrowRight: "M3 8h10M9 4l4 4-4 4",
  play: "M5 3.5v9l7.5-4.5z",
  fast: "M2.5 3.5v9l5-4.5zM8.5 3.5v9l5-4.5z",
  stop: "M4.5 4.5h7v7h-7z",
  x: "M4 4l8 8M12 4l-8 8",
  check: "M3.5 8.5l3 3 6-7",
  info: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM8 7.25v4M8 5h.01",
  warn: "M8 2.5 14 13H2zM8 6.5v3M8 11.25h.01",
  mail: "M2 3.5h12v9H2zM2.5 4 8 8.5 13.5 4",
  training: "M2 6v4M4 4.5v7M12 4.5v7M14 6v4M4 8h8",
  contract: "M4 2h6l3 3v9H4zM10 2v3h3M6 8h4M6 10.5h4",
  person: "M8 8a2.75 2.75 0 1 0 0-5.5A2.75 2.75 0 0 0 8 8zM2.5 14c0-2.7 2.4-4.5 5.5-4.5s5.5 1.8 5.5 4.5",
  save: "M3 3h8l2 2v8H3zM5.5 3v3.5h4V3M5.5 13V9.5h5V13",
  folder: "M2 4.5h4l1.25 1.5H14v7H2z",
  plus: "M8 3v10M3 8h10",
  minus: "M3 8h10",
  trash: "M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.5 9h6l.5-9",
  filter: "M2 3h12l-4.5 5.5V13l-3-1.5V8.5z",
  columns: "M2.5 3h11v10h-11zM6.25 3v10M9.75 3v10",
  sortDown: "M8 3v10M4.5 9.5 8 13l3.5-3.5",
  sortUp: "M8 13V3M4.5 6.5 8 3l3.5 3.5",
  eye: "M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8s-2.5 4.5-6.5 4.5S1.5 8 1.5 8zM8 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
  eyeOff: "M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8s-2.5 4.5-6.5 4.5S1.5 8 1.5 8zM8 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM3 3l10 10",
  lock: "M4 7.5h8v6H4zM5.5 7.5v-2a2.5 2.5 0 0 1 5 0v2",
  external: "M9 2.5h4.5V7M13.5 2.5l-6 6M11.5 9v4.5h-9v-9H7",
  compare: "M4 3v10M12 3v10M1.5 6 4 3l2.5 3M9.5 10 12 13l2.5-3",
  pitch: "M2 3.5h12v9H2zM8 3.5v9M8 9.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z",
  home: "M2.5 7.5 8 2.5l5.5 5M4 6.5v7h8v-7",
  refresh: "M13 8a5 5 0 1 1-1.5-3.5M13 2.5v2.5h-2.5",
  copy: "M5.5 5.5h8v8h-8zM10.5 5.5v-3h-8v8h3",
  swap: "M2.5 5h11M10.5 2l3 3-3 3M13.5 11h-11M5.5 8l-3 3 3 3",
} as const;

export type IconName = keyof typeof paths | "more";

interface Props extends Omit<SVGProps<SVGSVGElement>, "name"> {
  name: IconName;
  size?: number;
}

export function Icon({ name, size = 16, ...rest }: Props) {
  if (name === "more") {
    return (
      <svg viewBox="0 0 16 16" width={size} height={size} aria-hidden="true" fill="currentColor" {...rest}>
        <circle cx="3.5" cy="8" r="1.15" />
        <circle cx="8" cy="8" r="1.15" />
        <circle cx="12.5" cy="8" r="1.15" />
      </svg>
    );
  }
  return (
    <svg
      viewBox="0 0 16 16"
      width={size}
      height={size}
      aria-hidden="true"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.4}
      strokeLinecap="round"
      strokeLinejoin="round"
      {...rest}
    >
      <path d={paths[name]} />
    </svg>
  );
}
