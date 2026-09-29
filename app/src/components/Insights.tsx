import { useState } from "react";
import { useApi } from "../store";
import type { Named, Tone } from "../types";
import { Icon, type IconName } from "../ui/Icon";
import { Section, Skeleton } from "../ui/ui";
import { EntityLink } from "./links";

export interface InsightItem {
  kind: string;
  tone: Tone;
  title: string;
  text: string;
  basis: string;
  link: Named | null;
}
interface InsightsResp {
  items: InsightItem[];
  total: number;
  held: number;
}

// One icon per kind of note, so a list reads at a glance.
const ICONS: Record<string, IconName> = {
  form: "pulse",
  output: "pitch",
  record: "trophy",
  standing: "list",
  role: "person",
  discipline: "warn",
  body: "training",
  injury: "warn",
  growth: "up",
  contract: "contract",
  opinion: "chat",
  profile: "star",
  milestone: "star",
  table: "list",
  goals: "pitch",
  squad: "people",
  manager: "person",
  board: "club",
  finance: "contract",
  calendar: "calendar",
  stakes: "trophy",
  history: "clock",
  players: "person",
  match: "pitch",
};

/** Split a sentence around the first mention of `name`; `null` when it is not mentioned. */
export function splitOnName(text: string, name: string): [string, string] | null {
  const at = name ? text.indexOf(name) : -1;
  return at < 0 ? null : [text.slice(0, at), text.slice(at + name.length)];
}

/** The sentence, with the first mention of the linked name turned into a link (or the link added after it). */
function Sentence({ text, link }: { text: string; link: Named | null }) {
  if (!link) return <>{text}</>;
  const parts = splitOnName(text, link.name);
  if (!parts) {
    return (
      <>
        {text} <EntityLink r={link}>{link.name}</EntityLink>
      </>
    );
  }
  return (
    <>
      {parts[0]}
      <EntityLink r={link}>{link.name}</EntityLink>
      {parts[1]}
    </>
  );
}

/**
 * Notes an assistant coach would point out, each computed from recorded state and showing
 * the numbers it rests on. `limit` shows the first few with a way to see the rest.
 */
export function Insights({ method, args, title = "Insights", limit = 6, aside, compact = false, hideEmpty = false }: { method: string; args: Record<string, unknown>; title?: string; limit?: number; aside?: React.ReactNode; compact?: boolean; hideEmpty?: boolean }) {
  const q = useApi<InsightsResp>(method, args);
  const [all, setAll] = useState(false);
  if (q.error && !q.data) return null;
  if (!q.data) {
    return (
      <Section title={title}>
        <div className="card insights-skel" aria-busy="true">
          <Skeleton w="40%" />
          <Skeleton w="85%" />
          <Skeleton w="30%" />
          <Skeleton w="70%" />
        </div>
      </Section>
    );
  }
  const { items, total, held } = q.data;
  if (hideEmpty && items.length === 0 && held === 0) return null;
  const shown = all ? items : items.slice(0, limit);
  return (
    <Section title={title} aside={aside ?? (total > 0 ? <span className="faint num">{total}</span> : undefined)}>
      <div className={`card insights${compact ? " compact" : ""}`}>
        {items.length === 0 ? (
          <p className="muted insights-none">Nothing stands out right now.</p>
        ) : (
          <ul className="insight-list">
            {shown.map((it, i) => (
              <li key={`${it.kind}-${i}`} className={`insight tone-${it.tone}`}>
                <span className="insight-mark" aria-hidden="true">
                  <Icon name={ICONS[it.kind] ?? "info"} size={13} />
                </span>
                <div className="insight-body">
                  <div className="insight-title">{it.title}</div>
                  <div className="insight-text">
                    <Sentence text={it.text} link={it.link} />
                  </div>
                  {!compact && it.basis && <div className="insight-basis">{it.basis}</div>}
                </div>
              </li>
            ))}
          </ul>
        )}
        {items.length > limit && (
          <button type="button" className="insight-more" onClick={() => setAll((v) => !v)}>
            {all ? "Show fewer" : `Show ${items.length - limit} more`}
          </button>
        )}
        {held > 0 && (
          <p className="insight-held">
            <Icon name="eyeOff" size={13} /> Notes that would give away {held === 1 ? "an unrevealed result" : `${held} unrevealed results`} are left out until you reveal {held === 1 ? "it" : "them"}.
          </p>
        )}
      </div>
    </Section>
  );
}
