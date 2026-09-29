import type { Part } from "../types";
import { Parts } from "./links";

/** A 1-20 value. Tint deepens with the value; the number is always shown. */
export function AttrValue({ v, lo, hi, kind }: { v?: number; lo?: number; hi?: number; kind: "exact" | "range" | "unknown" }) {
  if (kind === "unknown") return <span className="attr attr-unknown" title="Not assessed">–</span>;
  const val = v ?? 0;
  const pct = Math.round(6 + Math.max(0, val - 1) * 4.2);
  const style = { background: `color-mix(in srgb, var(--accent) ${pct}%, transparent)` };
  if (kind === "range" && lo != null && hi != null) {
    return (
      <span className="attr attr-range num" style={style} title={`Your estimate: between ${lo} and ${hi}`}>
        {lo}–{hi}
      </span>
    );
  }
  return (
    <span className={`attr num ${val >= 15 ? "hi" : ""}`} style={style}>
      {val}
    </span>
  );
}

const PITCH: Record<string, [number, number]> = {
  GK: [0.5, 0.91], DL: [0.14, 0.73], DC: [0.5, 0.73], DR: [0.86, 0.73], WBL: [0.07, 0.58], WBR: [0.93, 0.58], DM: [0.5, 0.6],
  ML: [0.12, 0.44], MC: [0.5, 0.44], MR: [0.88, 0.44], AML: [0.16, 0.27], AMC: [0.5, 0.27], AMR: [0.84, 0.27], ST: [0.5, 0.1],
};

/** Where a player is comfortable, drawn on a pitch with attack at the top. */
export function PitchMap({ positions }: { positions: { code: string; fam: number; level: string }[] }) {
  const byCode = new Map(positions.map((p) => [p.code, p]));
  return (
    <figure className="pitchmap">
      <svg viewBox="0 0 100 130" role="img" aria-label={`Positions: ${positions.filter((p) => p.fam >= 12).map((p) => `${p.code} ${p.level}`).join(", ") || "none"}`}>
        <rect x="1" y="1" width="98" height="128" rx="3" className="pm-line" />
        <line x1="1" y1="65" x2="99" y2="65" className="pm-line" />
        <circle cx="50" cy="65" r="10" className="pm-line" />
        <rect x="26" y="1" width="48" height="18" className="pm-line" />
        <rect x="26" y="111" width="48" height="18" className="pm-line" />
        {Object.entries(PITCH).map(([code, [x, y]]) => {
          const p = byCode.get(code);
          const fam = p?.fam ?? 0;
          const cls = fam >= 18 ? "n3" : fam >= 14 ? "n2" : fam >= 9 ? "n1" : "n0";
          return (
            <g key={code} transform={`translate(${x * 100} ${y * 130})`}>
              <title>{`${code}: ${p?.level ?? "Not suited"}`}</title>
              <circle r={fam >= 9 ? 6.6 : 3.6} className={`pm-dot ${cls}`} />
              {fam >= 9 && <text y="1.9" textAnchor="middle" className="pm-text">{code.length > 2 ? code.slice(0, 3) : code}</text>}
            </g>
          );
        })}
      </svg>
      <figcaption className="hint">Position comfort. Filled circles are positions this player can play.</figcaption>
    </figure>
  );
}

export function RatingChips({ values }: { values: number[] }) {
  if (!values.length) return <span className="faint">No matches yet</span>;
  return (
    <span className="ratings">
      {values.map((v, i) => (
        <span key={i} className={`rating ${v >= 7.5 ? "r-hi" : v < 6 ? "r-lo" : ""} num`}>{v.toFixed(1)}</span>
      ))}
    </span>
  );
}

export function Sentence({ parts }: { parts: Part[] }) {
  return <span className="sentence"><Parts parts={parts} /></span>;
}
