import type { MouseEvent, ReactNode } from "react";
import { date, money } from "../format";
import { href, refPath } from "../router";
import { useSettings } from "../settings";
import type { Part, Ref } from "../types";

/** A link to a person, club, competition, nation or match. Plain click navigates in place. */
export function EntityLink({ r, children, className = "" }: { r: Ref; children: ReactNode; className?: string }) {
  return (
    <a className={`elink ${className}`} href={href(refPath(r))} onClick={(e: MouseEvent) => e.stopPropagation()}>
      {children}
    </a>
  );
}

export function Money({ v, exact, sign }: { v: number | null | undefined; exact?: boolean; sign?: boolean }) {
  useSettings();
  if (v == null) return null;
  return <span className="num" title={exact ? undefined : money(v, { exact: true })}>{money(v, { exact, sign })}</span>;
}

export function Dt({ d, year }: { d: number | null | undefined; year?: boolean }) {
  useSettings();
  if (d == null) return null;
  return <span className="num">{date(d, { year })}</span>;
}

/** A sentence built from text, links, money and dates. */
export function Parts({ parts }: { parts: Part[] }) {
  useSettings();
  return (
    <>
      {parts.map((p, i) => {
        if (p.r) return <EntityLink key={i} r={p.r}>{p.t}</EntityLink>;
        if (p.m != null) return <span key={i} className="num">{money(p.m)}</span>;
        if (p.d != null) return <span key={i} className="num">{date(p.d)}</span>;
        return <span key={i}>{p.t}</span>;
      })}
    </>
  );
}

export function FormDots({ form }: { form: string }) {
  if (!form) return null;
  return (
    <span className="formdots" aria-label={`Last results ${form.split("").join(" ")}`}>
      {form.split("").map((c, i) => (
        <span key={i} className={`form-dot form-${c.toLowerCase()}`} title={c === "W" ? "Win" : c === "D" ? "Draw" : "Loss"}>
          {c}
        </span>
      ))}
    </span>
  );
}
