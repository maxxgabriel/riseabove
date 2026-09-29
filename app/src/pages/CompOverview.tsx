import { useState } from "react";
import { EntityLink } from "../components/links";
import { Crest } from "../components/Crest";
import { FmBlock, FmCol, FmPanel, FmRow, ResultsStrip, type TeamBit, type TickerItem } from "../components/Stage";
import { href } from "../router";
import { useApi } from "../store";
import type { Named } from "../types";
import { Icon } from "../ui/Icon";
import { Skeleton } from "../ui/ui";

export interface Head {
  id: number;
  name: string;
  short: string;
  kind: string;
  kind_key: string;
  tier: number;
  teams: number;
  season: string;
  stage: string;
  prev: Named | null;
  next: Named | null;
  meta: { label: string; value: string | TeamBit; sub?: string; ref?: Named }[];
}

interface Section {
  page: number;
  title: string;
  rows: { p?: Named; team: TeamBit; value: string; pill?: boolean }[];
}

interface Overview extends Head {
  ticker: TickerItem[];
  left:
    | { kind: "table"; title: string; total: number; shown: number; rows: { pos: number; team: TeamBit; played: number; gd: number; points: number; zone: "up" | "down" | null }[] }
    | {
        kind: "ties";
        title: string;
        round: string;
        total: number;
        ties: { a: TeamBit; b: TeamBit; goals_a: number | null; goals_b: number | null; played: number; legs: number; hidden: boolean; winner: "a" | "b" | null }[];
      };
  players: Section[];
  teams_stats: Section[];
  held: { results: number; withheld: string[] };
  news: { headline: string; days_ago: number; outlet: string; match: { k: string; id: number }; spoils: boolean; home: TeamBit; away: TeamBit; score: [number, number] | null }[];
}

function ago(days: number): string {
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 14) return `${days}d ago`;
  return `${Math.round(days / 7)}w ago`;
}

export function CompOverview({ id, tabTo }: { id: number; tabTo: (t: string) => string }) {
  const q = useApi<Overview>("comp.overview", { id });
  const [page, setPage] = useState(1);
  const d = q.data;
  if (!d) {
    return (
      <div className="fm-panel" aria-busy="true" style={{ padding: "1.4rem", display: "grid", gap: "0.8rem", gridTemplateColumns: "1fr" }}>
        <Skeleton w="30%" h={26} />
        <Skeleton w="70%" />
        <Skeleton w="60%" />
        <Skeleton w="65%" />
      </div>
    );
  }
  const pages = Math.max(1, ...d.players.map((s) => s.page), ...d.teams_stats.map((s) => s.page));
  const p = Math.min(page, pages);
  return (
    <>
      <ResultsStrip items={d.ticker} />
      <FmPanel className={`fm-panel-${d.left.kind}`}>
        <LeftColumn d={d} tabTo={tabTo} />
        <FmCol title="Player Stats" label="Player statistics">
          {d.held.results > 0 && (
            <p className="fm-note">
              <Icon name="eyeOff" size={15} />
              <span>
                {d.held.results === 1 ? "One result is" : `${d.held.results} results are`} held back until you reveal {d.held.results === 1 ? "it" : "them"}, so goals, assists and ratings leave those matches out
                {d.held.withheld.length > 0 && <> and {d.held.withheld.join(", ").toLowerCase()} are hidden</>}.
              </span>
            </p>
          )}
          {d.players.filter((s) => s.page === p).map((s) => (
            <FmBlock key={s.title} title={s.title}>
              {s.rows.map((r, i) => (
                <FmRow
                  key={i}
                  icon={<Crest name={r.team.full} colors={r.team.colors} id={r.team.id} size={18} plain />}
                  name={r.p?.name}
                  to={r.p}
                  me={r.team.me}
                  value={r.value}
                  pill={r.pill ? Number(r.value) : null}
                />
              ))}
            </FmBlock>
          ))}
          {d.players.filter((s) => s.page === p).length === 0 && <p className="fm-empty">Nobody has played on this page yet.</p>}
        </FmCol>
        <FmCol title="Team Stats" label="Team statistics">
          {d.teams_stats.filter((s) => s.page === p).map((s) => (
            <FmBlock key={s.title} title={s.title}>
              {s.rows.map((r, i) => (
                <FmRow key={i} icon={<Crest name={r.team.full} colors={r.team.colors} id={r.team.id} size={24} plain />} name={r.team.name} to={r.team} me={r.team.me} value={r.value} />
              ))}
            </FmBlock>
          ))}
          {d.teams_stats.filter((s) => s.page === p).length === 0 && <p className="fm-empty">Nothing to rank on this page yet.</p>}
        </FmCol>
        <FmCol title={`${d.short} News`} label="News">
          <News d={d} />
        </FmCol>
        {pages > 1 && (
          <>
            <button className="fm-page prev" onClick={() => setPage(Math.max(1, p - 1))} disabled={p === 1} aria-label="Previous page of statistics">
              <Icon name="left" size={26} />
            </button>
            <button className="fm-page next" onClick={() => setPage(Math.min(pages, p + 1))} disabled={p === pages} aria-label="Next page of statistics">
              <Icon name="right" size={26} />
            </button>
            <div className="fm-dots" role="group" aria-label="Statistics pages">
              {Array.from({ length: pages }, (_, i) => (
                <button key={i} className="fm-dot" aria-current={p === i + 1} aria-label={`Page ${i + 1}`} onClick={() => setPage(i + 1)} />
              ))}
            </div>
          </>
        )}
      </FmPanel>
    </>
  );
}

function LeftColumn({ d, tabTo }: { d: Overview; tabTo: (t: string) => string }) {
  const l = d.left;
  if (l.kind === "ties") {
    return (
      <FmCol title={l.title} label="Current round">
        {l.ties.length === 0 ? (
          <p className="fm-empty">No ties have been drawn yet. {d.stage}.</p>
        ) : (
          <div className="ties">
            {l.ties.map((t, i) => (
              <div key={i} className="tie-row">
                <span className={`tie-a ${t.a.me ? "me" : ""} ${t.winner === "a" ? "win" : ""}`}>
                  <EntityLink r={t.a}>{t.a.name}</EntityLink>
                </span>
                <Crest name={t.a.full} colors={t.a.colors} id={t.a.id} size={20} plain />
                <span className="tie-v" title={t.hidden ? "Result held back" : undefined}>
                  {t.hidden ? "?" : t.winner ? `${t.goals_a}-${t.goals_b}` : t.played > 0 && t.legs > 1 ? "…" : "V"}
                </span>
                <Crest name={t.b.full} colors={t.b.colors} id={t.b.id} size={20} plain />
                <span className={`tie-b ${t.b.me ? "me" : ""} ${t.winner === "b" ? "win" : ""}`}>
                  <EntityLink r={t.b}>{t.b.name}</EntityLink>
                </span>
              </div>
            ))}
          </div>
        )}
        <p className="fm-foot">
          {l.round}, showing {l.ties.length} of {l.total} {l.total === 1 ? "tie" : "ties"}
        </p>
      </FmCol>
    );
  }
  return (
    <FmCol title={l.title} label="League table">
      <div className="mini" role="table" aria-label={l.title}>
        <div className="mini-row mini-head" role="row">
          <span className="pos">#</span>
          <span />
          <span>Team</span>
          <span className="num">P</span>
          <span className="num">GD</span>
          <span className="num">Pts</span>
        </div>
        {l.rows.map((r) => (
          <div key={r.pos} role="row" className={`mini-row ${r.team.me ? "me" : ""} ${r.zone ?? ""}`}>
            <span className="pos num">{r.pos}</span>
            <Crest name={r.team.full} colors={r.team.colors} id={r.team.id} size={18} plain />
            <span className="mini-team">
              <EntityLink r={r.team}>{r.team.name}</EntityLink>
            </span>
            <span className="num">{r.played}</span>
            <span className="num">{r.gd > 0 ? `+${r.gd}` : r.gd}</span>
            <span className="num pts">{r.points}</span>
          </div>
        ))}
      </div>
      <p className="fm-foot">
        {l.shown < l.total ? `Showing ${l.shown} of ${l.total} teams. ` : ""}
        <a href={href(tabTo("table"))}>Full table</a>
      </p>
    </FmCol>
  );
}

function News({ d }: { d: Overview }) {
  const [lead, ...more] = d.news;
  if (!lead) return <p className="fm-empty">Nothing has been written about {d.short} yet.</p>;
  return (
    <>
      <a className="story-lead" href={href(`/match/${lead.match.id}`)}>
        <div className="story-art" style={{ "--a": lead.home.colors?.[0], "--b": lead.away.colors?.[0] } as React.CSSProperties}>
          <Crest name={lead.home.full} colors={lead.home.colors} id={lead.home.id} size={46} />
          <span className="story-score">{lead.score ? `${lead.score[0]} - ${lead.score[1]}` : "v"}</span>
          <Crest name={lead.away.full} colors={lead.away.colors} id={lead.away.id} size={46} />
        </div>
        <div className="story-body">
          <p className="story-head">{lead.headline}</p>
        </div>
        <div className="story-meta">
          <span>
            <Icon name="clock" size={14} /> {ago(lead.days_ago)}
          </span>
          <span>
            <Icon name="pitch" size={14} /> Match
          </span>
          <span>{lead.outlet}</span>
          <span className="go">
            <Icon name="arrowRight" size={16} />
          </span>
        </div>
      </a>
      {more.length > 0 && <h3 className="more-title">More Stories</h3>}
      {more.map((s, i) => (
        <a key={i} className="story-mini" href={href(`/match/${s.match.id}`)}>
          <div className="story-mini-art" style={{ "--a": s.home.colors?.[0], "--b": s.away.colors?.[0] } as React.CSSProperties}>
            <span className="num" style={{ fontFamily: "var(--display)", fontWeight: 700, fontSize: "1.3rem" }}>
              {s.score ? `${s.score[0]}-${s.score[1]}` : "v"}
            </span>
          </div>
          <div>
            <div className="story-mini-head">{s.headline}</div>
            <div className="story-mini-meta">
              <span>{ago(s.days_ago)}</span>
              <span>{s.outlet}</span>
            </div>
          </div>
        </a>
      ))}
    </>
  );
}
