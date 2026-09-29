import { useState } from "react";
import { EntityLink, Dt } from "../components/links";
import { TableView } from "../components/TableView";
import { toggleBookmark, useIsBookmarked } from "../bookmarks";
import { fmtInt, plural } from "../format";
import { href, navigate, useRoute } from "../router";
import { useApi } from "../store";
import type { Named } from "../types";
import { Badge, ErrorState, IconButton, KeyVal, Section, Segmented, Skeleton } from "../ui/ui";
import { Insights } from "../components/Insights";
import { usePageTitle } from "./common";
import { brandFor, tintOf } from "../color";
import { Crest } from "../components/Crest";
import { Stage, StageHeader, StageTabs, type MetaBit } from "../components/Stage";
import { CompOverview, type Head } from "./CompOverview";

interface Tie {
  index: number;
  round: number | null;
  a: Named;
  b: Named;
  goals_a: number | null;
  goals_b: number | null;
  legs: number;
  played: number;
  winner: Named | null;
  hidden: boolean;
}
interface CompResp {
  id: number;
  name: string;
  short: string;
  nation: Named | null;
  kind: string;
  kind_key: string;
  tier: number;
  reputation: number;
  format: string;
  is_league: boolean;
  groups: number;
  size: number;
  promote: number;
  relegate: number;
  rules: { yellow_limit: number; bench: number; subs: number; extra_time: boolean; away_goals: boolean; foreigner_limit: number };
  team_kind: string;
  state: { season: string; stage: string; knockout: boolean; start: number; end: number; teams: number; round: number; winner: Named | null; runner_up: Named | null };
  above: Named | null;
  below: Named | null;
  continental_places: { comp: Named; places: number }[];
  ties: Tie[];
  past_editions: number;
  prize_pool: number | null;
}

type Tab = "overview" | "table" | "fixtures" | "leaders" | "history" | "rules";

export function Comp() {
  const route = useRoute();
  const id = Number(route.segs[1]);
  const q = useApi<CompResp>("comp", { id });
  const head = useApi<Head>("comp.overview", { id, light: true });
  usePageTitle(q.data?.name);
  const tab = ((route.segs[2] as Tab) || "overview") as Tab;
  const kind = head.data?.kind_key ?? q.data?.kind_key ?? "league";
  const name = q.data?.name ?? head.data?.name ?? "";
  const tint = tintOf(brandFor(kind, id, name)[0]);
  return (
    <Stage tint={tint}>
      {q.error && !q.data ? (
        <div className="stage-body">
          <ErrorState error={q.error} onRetry={q.reload} />
        </div>
      ) : (
        <CompBody id={id} c={q.data ?? null} head={head.data ?? null} tab={tab} />
      )}
    </Stage>
  );
}

function CompBody({ id, c, head, tab }: { id: number; c: CompResp | null; head: Head | null; tab: Tab }) {
  const bookmarked = useIsBookmarked("comp", id);
  const name = c?.name ?? head?.name ?? "";
  const hasTable = c ? c.is_league || c.groups > 0 : true;
  const tabs: { id: Tab; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "table", label: hasTable ? "Table" : "Rounds" },
    { id: "fixtures", label: "Fixtures" },
    { id: "leaders", label: "Leaders" },
    { id: "history", label: "History" },
    { id: "rules", label: "Rules" },
  ];
  const meta: MetaBit[] = (head?.meta ?? []).map((m) => ({
    label: m.label,
    value:
      typeof m.value === "string" ? (
        m.ref ? <EntityLink r={m.ref}>{m.value}</EntityLink> : m.value
      ) : (
        <>
          <Crest name={m.value.full} colors={m.value.colors} id={m.value.id} size={17} plain />
          <EntityLink r={m.value}>{m.value.name}</EntityLink>
          {m.sub && <small>{m.sub}</small>}
        </>
      ),
  }));
  return (
    <>
      <StageHeader
        crest={<Crest name={name || "…"} kind="comp" id={id} colors={brandFor(head?.kind_key ?? c?.kind_key ?? "league", id, name)} size={58} />}
        title={name || <Skeleton w="14rem" h={36} />}
        sub={head ? `${head.teams} ${head.teams === 1 ? "club" : "clubs"} · ${head.season} · ${head.stage}` : undefined}
        subIcon="club"
        meta={meta}
        step={head ? { prev: head.prev, next: head.next, noun: "competition" } : undefined}
        actions={<IconButton icon="bookmark" label={bookmarked ? "Remove bookmark" : "Bookmark"} aria-pressed={bookmarked} className={bookmarked ? "on" : ""} onClick={() => toggleBookmark({ k: "comp", id, title: name, sub: c?.kind ?? "" })} />}
      />
      <StageTabs tabs={tabs} value={tab} onChange={(t) => navigate(`/comp/${id}${t === "overview" ? "" : `/${t}`}`)} label="Competition sections" />
      {tab === "overview" ? (
        <CompOverview id={id} tabTo={(t) => `/comp/${id}/${t}`} />
      ) : !c ? (
        <div className="stage-body" aria-busy="true">
          <Skeleton w="40%" h={24} />
        </div>
      ) : (
        <div className="stage-body">
          {tab === "table" && (hasTable ? <TablePane c={c} /> : <Bracket c={c} />)}
          {tab === "fixtures" && <Fixtures c={c} />}
          {tab === "leaders" && <Leaders c={c} />}
          {tab === "history" && <TableView id="comp-honours" table="honours" label="Past winners" filters={{ comp: c.id }} height={25} noPresets noColumns empty="No editions have finished yet." />}
          {tab === "rules" && <Rules c={c} />}
        </div>
      )}
    </>
  );
}

function TablePane({ c }: { c: CompResp }) {
  const [group, setGroup] = useState(0);
  return (
    <div className="split">
      <div className="stack">
        {c.groups > 1 && (
          <Segmented
            label="Group"
            value={group}
            onChange={setGroup}
            options={Array.from({ length: c.groups }, (_, i) => ({ id: i, label: `Group ${String.fromCharCode(65 + i)}` }))}
          />
        )}
        <TableView
          id="comp-table"
          table="standings"
          label="League table"
          filters={c.groups > 1 ? { comp: c.id, group } : { comp: c.id }}
          height={Math.max(8, Math.min(24, c.size || 20))}
          noPresets
          noColumns
          urlSort={false}
          noun={["team", "teams"]}
          stickyFirst={false}
        />
        {c.groups <= 1 && <Insights method="insight.comp" args={{ id: c.id }} hideEmpty limit={6} />}
      </div>
      <aside className="stack">
        <StatePanel c={c} />
      </aside>
    </div>
  );
}

function StatePanel({ c }: { c: CompResp }) {
  const s = c.state;
  return (
    <Section title="Season">
      <div className="card">
        <KeyVal
          rows={[
            { k: "Season", v: s.season },
            { k: "Stage", v: s.stage },
            { k: "Runs", v: <span><Dt d={s.start} year={false} /> to <Dt d={s.end} /></span> },
            { k: "Teams", v: <span className="num">{s.teams || c.size}</span> },
            ...(s.winner ? [{ k: "Winner", v: <EntityLink r={s.winner}>{s.winner.name}</EntityLink> }] : []),
            ...(s.runner_up ? [{ k: "Runner-up", v: <EntityLink r={s.runner_up}>{s.runner_up.name}</EntityLink> }] : []),
            ...(c.above ? [{ k: "Above", v: <EntityLink r={c.above}>{c.above.name}</EntityLink> }] : []),
            ...(c.below ? [{ k: "Below", v: <EntityLink r={c.below}>{c.below.name}</EntityLink> }] : []),
          ]}
        />
      </div>
    </Section>
  );
}

function Bracket({ c }: { c: CompResp }) {
  const rounds = new Map<number, Tie[]>();
  for (const t of c.ties) {
    const r = t.round ?? 0;
    rounds.set(r, [...(rounds.get(r) ?? []), t]);
  }
  const keys = [...rounds.keys()].sort((a, b) => a - b);
  return (
    <div className="stack">
      {keys.length === 0 ? (
        <div className="card muted">No ties have been drawn yet. {c.state.stage}.</div>
      ) : (
        <div className="bracket" role="list" aria-label="Rounds">
          {keys.map((k) => (
            <section key={k} className="bracket-round" role="listitem" aria-label={`Round ${k + 1}`}>
              <h3>{k === keys[keys.length - 1] ? (keys.length > 1 ? "Final round" : "Round") : `Round ${k + 1}`}</h3>
              {rounds.get(k)!.map((t) => (
                <div key={t.index} className="tie">
                  <TieSide team={t.a} goals={t.goals_a} win={t.winner?.id === t.a.id} hidden={t.hidden} />
                  <TieSide team={t.b} goals={t.goals_b} win={t.winner?.id === t.b.id} hidden={t.hidden} />
                  {t.hidden && <div className="hint">Result hidden</div>}
                </div>
              ))}
            </section>
          ))}
        </div>
      )}
      <StatePanel c={c} />
    </div>
  );
}

function TieSide({ team, goals, win, hidden }: { team: Named; goals: number | null; win: boolean; hidden: boolean }) {
  return (
    <div className={`tie-side ${win ? "win" : ""}`}>
      <EntityLink r={team}>{team.name}</EntityLink>
      <span className="num">{hidden ? "?" : goals ?? ""}</span>
    </div>
  );
}

function Fixtures({ c }: { c: CompResp }) {
  const [show, setShow] = useState<"upcoming" | "results" | "all">("upcoming");
  const filters: Record<string, unknown> = { comp: c.id };
  if (show === "upcoming") filters.played = false;
  if (show === "results") filters.played = true;
  return (
    <TableView
      key={show}
      id="comp-fixtures"
      table="fixtures"
      label="Fixtures"
      filters={filters}
      height={30}
      noPresets
      noColumns
      defaultSort={{ key: "date", desc: show === "results" }}
      noun={["match", "matches"]}
      toolbar={<Segmented label="Show" value={show} onChange={setShow} options={[{ id: "upcoming", label: "Upcoming" }, { id: "results", label: "Results" }, { id: "all", label: "All" }]} />}
    />
  );
}

function Leaders({ c }: { c: CompResp }) {
  const [by, setBy] = useState("goals");
  return (
    <TableView
      key={by}
      id="comp-leaders"
      table="comp_stats"
      label="Leaders"
      filters={{ comp: c.id }}
      height={25}
      noPresets
      noColumns
      defaultSort={{ key: by, desc: true }}
      noun={["player", "players"]}
      toolbar={
        <Segmented
          label="Rank by"
          value={by}
          onChange={setBy}
          options={[{ id: "goals", label: "Goals" }, { id: "assists", label: "Assists" }, { id: "rating", label: "Rating" }, { id: "cs", label: "Clean sheets" }, { id: "pom", label: "Man of the match" }]}
        />
      }
      empty="No one has played yet."
    />
  );
}

function Rules({ c }: { c: CompResp }) {
  return (
    <div className="grid-2">
      <Section title="Format">
        <div className="card">
          <KeyVal
            rows={[
              { k: "Format", v: c.format },
              { k: "Teams", v: <span className="num">{c.size}</span> },
              ...(c.is_league ? [{ k: "Promoted", v: <span className="num">{c.promote}</span> }, { k: "Relegated", v: <span className="num">{c.relegate}</span> }] : []),
              { k: "Squads", v: c.team_kind },
              { k: "Editions completed", v: <span className="num">{fmtInt(c.past_editions)}</span> },
              ...(c.prize_pool != null ? [{ k: "Prize pool", v: <span className="num">{fmtInt(c.prize_pool)}</span> }] : []),
            ]}
          />
          {c.continental_places.length > 0 && (
            <div className="tags">
              {c.continental_places.map((p) => (
                <Badge key={p.comp.id}>{plural(p.places, "place")} in {p.comp.name}</Badge>
              ))}
            </div>
          )}
        </div>
      </Section>
      <Section title="Match rules">
        <div className="card">
          <KeyVal
            rows={[
              { k: "Substitutes named", v: <span className="num">{c.rules.bench}</span> },
              { k: "Substitutions allowed", v: <span className="num">{c.rules.subs}</span> },
              { k: "Extra time", v: c.rules.extra_time ? "Yes, when tied" : "No" },
              { k: "Away goals rule", v: c.rules.away_goals ? "Yes" : "No" },
              { k: "Yellow cards for a ban", v: <span className="num">{c.rules.yellow_limit || "None"}</span> },
              { k: "Foreign player limit", v: c.rules.foreigner_limit ? <span className="num">{c.rules.foreigner_limit}</span> : "None" },
            ]}
          />
        </div>
      </Section>
    </div>
  );
}

// ---- nations -------------------------------------------------------------------------------------

interface NationResp {
  id: number;
  name: string;
  code: string;
  confed: string;
  reputation: number;
  economy: number | null;
  youth_rating: number | null;
  season: { label: string; start: number; end: number; windows: [number, number][]; winter_break: [number, number] | null };
  leagues: { comp: Named; tier: number; teams: number }[];
  cups: Named[];
  clubs: number;
  players: number;
  window_open: boolean;
}

export function Nation() {
  const route = useRoute();
  const id = Number(route.segs[1]);
  const tab = route.segs[2] ?? "overview";
  const q = useApi<NationResp>("nation", { id });
  usePageTitle(q.data?.name);
  const n = q.data;
  const [c1] = brandFor("nation", id, n?.name ?? "");
  return (
    <Stage tint={tintOf(c1)}>
      {q.error && !q.data ? (
        <div className="stage-body">
          <ErrorState error={q.error} onRetry={q.reload} />
        </div>
      ) : !n ? (
        <div className="stage-body" aria-busy="true">
          <Skeleton w="30%" h={40} />
        </div>
      ) : (
        <>
          <StageHeader
            crest={<Crest name={n.name} kind="nation" id={n.id} size={58} />}
            title={n.name}
            sub={`${n.confed} · ${plural(n.clubs, "club")} · ${fmtInt(n.players)} players`}
            subIcon="globe"
            meta={[
              { label: "Reputation", value: <span className="num">{fmtInt(n.reputation)}</span> },
              { label: "Season", value: n.season.label },
              { label: "Transfer window", value: n.window_open ? "Open" : "Closed" },
            ]}
          />
          <StageTabs
            value={tab}
            onChange={(t) => navigate(`/nation/${n.id}${t === "overview" ? "" : `/${t}`}`)}
            label="Nation sections"
            tabs={[{ id: "overview", label: "Overview" }, { id: "clubs", label: "Clubs" }, { id: "people", label: "People" }]}
          />
          <div className="stage-body">
            {tab === "overview" && (
              <div className="grid-2">
                <Section title="Competitions">
                  <div className="card">
                    <ul className="rows compact">
                      {n.leagues.map((l) => (
                        <li key={l.comp.id}><EntityLink r={l.comp}>{l.comp.name}</EntityLink><span className="hint">Tier {l.tier} · {l.teams} teams</span></li>
                      ))}
                      {n.cups.map((cup) => (
                        <li key={cup.id}><EntityLink r={cup}>{cup.name}</EntityLink><span className="hint">Cup</span></li>
                      ))}
                    </ul>
                  </div>
                </Section>
                <Section title="Season">
                  <div className="card">
                    <KeyVal
                      rows={[
                        { k: "Season", v: n.season.label },
                        { k: "Starts", v: <Dt d={n.season.start} /> },
                        { k: "Ends", v: <Dt d={n.season.end} /> },
                        ...n.season.windows.map((w, i) => ({ k: `Transfer window ${i + 1}`, v: <span><Dt d={w[0]} year={false} /> to <Dt d={w[1]} year={false} /></span> })),
                        ...(n.season.winter_break ? [{ k: "Winter break", v: <span><Dt d={n.season.winter_break[0]} year={false} /> to <Dt d={n.season.winter_break[1]} year={false} /></span> }] : []),
                        { k: "Reputation", v: <span className="num">{fmtInt(n.reputation)}</span> },
                        ...(n.economy != null ? [{ k: "Economy", v: <span className="num">{n.economy.toFixed(2)}</span> }] : []),
                        ...(n.youth_rating != null ? [{ k: "Youth development", v: <span className="num">{n.youth_rating}</span> }] : []),
                      ]}
                    />
                  </div>
                </Section>
              </div>
            )}
            {tab === "clubs" && <TableView id="nation-clubs" table="clubs" label="Clubs" filters={{ nation: n.id }} height={30} noun={["club", "clubs"]} noColumns />}
            {tab === "people" && (
              <p className="muted">
                <a href={href(`/people?nation=${n.id}`)}>Open everyone from {n.name} in the people list</a>, where they can be filtered and sorted.
              </p>
            )}
          </div>
        </>
      )}
    </Stage>
  );
}
