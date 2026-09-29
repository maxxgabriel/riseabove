import { useEffect, useRef, type CSSProperties, type ReactNode } from "react";
import { date } from "../format";
import { Icon } from "../ui/Icon";
import { EntityLink } from "./links";
import { ClubCrest, Crest } from "./Crest";
import { href } from "../router";
import type { Named } from "../types";

/** A team as the overview payload carries it: enough to draw a badge and link to the club. */
export interface TeamBit {
  k: "club";
  id: number;
  name: string;
  full: string;
  colors?: [string, string];
  me: boolean;
}

export const DEFAULT_TINT = "#12483a";

/** The tinted screen behind an entity page: a gradient in the entity's colour under a header and a panel. */
export function Stage({ tint, children, className = "" }: { tint: string; children: ReactNode; className?: string }) {
  // The whole window takes the colour, not only the page: the rail, top bar and status line follow.
  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--tint", tint);
    return () => {
      root.style.removeProperty("--tint");
    };
  }, [tint]);
  return (
    <div className={`stage ${className}`} style={{ "--tint": tint } as CSSProperties}>
      {children}
    </div>
  );
}

export interface MetaBit {
  label: string;
  value: ReactNode;
}

interface HeaderProps {
  crest: ReactNode;
  title: ReactNode;
  sub?: ReactNode;
  subIcon?: "club" | "person" | "trophy" | "globe" | "clock" | "calendar";
  meta?: MetaBit[];
  /** Step to the previous or next entity in a list, the way the arrows beside the title do. */
  step?: { prev: Named | null; next: Named | null; noun: string };
  actions?: ReactNode;
}

export function StageHeader({ crest, title, sub, subIcon, meta, step, actions }: HeaderProps) {
  return (
    <header className="stage-head">
      <div className="stage-crest">{crest}</div>
      {step && (
        <div className="stage-step" aria-label={`Other ${step.noun}`}>
          <StepLink to={step.prev} dir="up" noun={step.noun} />
          <StepLink to={step.next} dir="down" noun={step.noun} />
        </div>
      )}
      <div className="stage-titles">
        <h1 className="stage-title">{title}</h1>
        {sub && (
          <div className="stage-sub">
            {subIcon && <Icon name={subIcon} size={15} />}
            <span>{sub}</span>
          </div>
        )}
      </div>
      <div className="stage-grow" />
      {meta && meta.length > 0 && (
        <dl className="stage-meta">
          {meta.map((m) => (
            <div key={m.label} className="stage-meta-item">
              <dt>{m.label}</dt>
              <dd>{m.value}</dd>
            </div>
          ))}
        </dl>
      )}
      {actions && <div className="stage-actions">{actions}</div>}
    </header>
  );
}

function StepLink({ to, dir, noun }: { to: Named | null; dir: "up" | "down"; noun: string }) {
  const icon = dir === "up" ? "up" : "down";
  if (!to) {
    return (
      <span className="stage-step-btn off" aria-hidden="true">
        <Icon name={icon} size={16} />
      </span>
    );
  }
  return (
    <a className="stage-step-btn" href={href(`/${to.k}/${to.id}`)} title={`${dir === "up" ? "Previous" : "Next"} ${noun}: ${to.name}`} aria-label={`${dir === "up" ? "Previous" : "Next"} ${noun}: ${to.name}`}>
      <Icon name={icon} size={16} />
    </a>
  );
}

export interface TickerItem {
  uid: number;
  date: number;
  status: "ft" | "held" | "next";
  home: TeamBit;
  away: TeamBit;
  hs: number | null;
  as: number | null;
  pens: [number, number] | null;
}

/** A strip of matches, oldest first, scrolled so the latest results sit in view. */
export function ResultsStrip({ items }: { items: TickerItem[] }) {
  const ref = useRef<HTMLDivElement>(null);
  const inner = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current;
    const row = inner.current;
    if (!el || !row) return;
    const place = () => {
      // Whole cards only: as many as fit at a readable width, then the latest results near the middle.
      const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 13;
      const n = Math.max(2, Math.min(10, Math.floor((el.clientWidth - 0.8 * rem) / (10.5 * rem + 0.35 * rem))));
      el.style.setProperty("--card", `${(el.clientWidth - 0.8 * rem - (n - 1) * 0.35 * rem) / n}px`);
      const firstNext = items.findIndex((i) => i.status === "next");
      const at = firstNext < 0 ? items.length - 1 : Math.max(0, firstNext - 1);
      const lead = Math.max(0, Math.min(items.length - n, at - Math.floor(n / 2)));
      const first = row.children[0] as HTMLElement | undefined;
      const card = row.children[lead] as HTMLElement | undefined;
      if (first && card) el.scrollTo({ left: card.offsetLeft - first.offsetLeft, behavior: "instant" });
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(el);
    return () => ro.disconnect();
  }, [items]);
  if (items.length === 0) return null;
  return (
    <div className="strip" ref={ref}>
      <div className="strip-in" ref={inner} role="list" aria-label="Results and coming matches">
      {items.map((m) => (
        <a key={m.uid} role="listitem" className={`strip-card ${m.status}`} href={href(`/match/${m.uid}`)}>
          <div className="strip-top">
            <span className="num">{date(m.date, { year: true })}</span>
            <span>{m.status === "ft" ? "Full time" : m.status === "held" ? "Held back" : "Up next"}</span>
          </div>
          <StripSide t={m.home} goals={m.hs} shown={m.status === "ft"} win={m.status === "ft" && m.hs != null && m.as != null && m.hs > m.as} />
          <StripSide t={m.away} goals={m.as} shown={m.status === "ft"} win={m.status === "ft" && m.hs != null && m.as != null && m.as > m.hs} />
          {m.pens && <div className="strip-pens">Penalties {m.pens[0]}-{m.pens[1]}</div>}
        </a>
      ))}
      </div>
    </div>
  );
}

function StripSide({ t, goals, shown, win }: { t: TeamBit; goals: number | null; shown: boolean; win: boolean }) {
  return (
    <div className={`strip-side ${t.me ? "me" : ""} ${win ? "win" : ""}`}>
      {t.colors ? <Crest name={t.full} colors={t.colors} id={t.id} size={17} plain /> : <ClubCrest id={t.id} name={t.full} size={17} />}
      <span className="strip-name">{t.name}</span>
      <span className="strip-goals num">{shown ? goals : ""}</span>
    </div>
  );
}

/** The big bordered panel with columns separated by hairlines. */
export function FmPanel({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <section className={`fm-panel ${className}`}>{children}</section>;
}

export function FmCol({ title, children, className = "", label }: { title?: ReactNode; children: ReactNode; className?: string; label?: string }) {
  return (
    <div className={`fm-col ${className}`} role="group" aria-label={label}>
      {title && <h2 className="fm-title">{title}</h2>}
      {children}
    </div>
  );
}

/** A small heading over a list of rows, as on the statistics columns. */
export function FmBlock({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="fm-block">
      <h3 className="fm-label">{title}</h3>
      <ul className="fm-rows">{children}</ul>
    </div>
  );
}

export function FmRow({ icon, name, to, me, value, pill }: { icon?: ReactNode; name: ReactNode; to?: Named | { k: string; id: number }; me?: boolean; value?: ReactNode; pill?: number | null }) {
  return (
    <li className={`fm-row ${me ? "me" : ""}`}>
      {icon && <span className="fm-icon">{icon}</span>}
      <span className="fm-name">{to ? <EntityLink r={to as Named}>{name}</EntityLink> : name}</span>
      {pill != null ? <span className={`pill ${pillTone(pill)}`}>{value}</span> : <span className="fm-val num">{value}</span>}
    </li>
  );
}

function pillTone(r: number) {
  if (r >= 7.5) return "hi";
  if (r >= 6.9) return "mid";
  if (r >= 6.2) return "low";
  return "bad";
}

/** Section tabs under a stage header: condensed text with a white underline on the current one. */
export function StageTabs<T extends string>({ tabs, value, onChange, label }: { tabs: { id: T; label: string }[]; value: T; onChange: (t: T) => void; label: string }) {
  return (
    <div className="stage-tabs" role="tablist" aria-label={label}>
      {tabs.map((t) => (
        <button key={t.id} role="tab" className="stage-tab" aria-selected={t.id === value} onClick={() => onChange(t.id)}>
          {t.label}
        </button>
      ))}
    </div>
  );
}
