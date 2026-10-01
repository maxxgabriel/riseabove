import { useState } from "react";
import { call } from "../api";
import { bytes, fmtInt } from "../format";
import { href, refPath, setQuery, useRoute } from "../router";
import { act, useApi, useStatus } from "../store";
import type { Ref } from "../types";
import { Button, ErrorState, Field, Spinner } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

interface SourceTable { name: string; bytes: number; source: string; status: string }
interface Source { id: number; path: string; tables: SourceTable[] }
interface Sources { sources: Source[]; errors: string[] }
interface SourceRow { row: number; fields: Record<string, string | null>; refs?: Record<string, Ref> }
interface Result {
  rows: SourceRow[]; columns: string[]; indexed_columns: string[];
  total: number; matched: number; offset: number; malformed: number;
  name_search: boolean;
}

const label = (s: string) => s.replace(/\.csv$/, "").replaceAll("_", " ");
const title = (r: SourceRow) => r.fields.name ?? r.fields.label ?? r.fields.alias ?? r.fields.player_name ?? ([r.fields.first_name, r.fields.last_name].filter(Boolean).join(" ")
  || r.fields.player_id || r.fields.game_id || `Record ${r.row + 1}`);

/** Original source facts and history live outside the running world. This is an observer tool. */
export function Database() {
  usePageTitle("Source database");
  const route = useRoute();
  const st = useStatus();
  const sources = useApi<Sources>("database.sources");
  const source = route.query.has("source") ? sources.data?.sources.find((s) => s.id === Number(route.query.get("source")))
    : sources.data?.sources.find((s) => route.query.get("family") ? s.tables.some((t) => t.source === route.query.get("family")) : true);
  const sourceId = source?.id ?? 0;
  const table = route.query.get("table") ?? source?.tables.find((t) => t.name === "catalog_players.csv")?.name ?? source?.tables.find((t) => t.name === "players.csv" || t.name === "clubs_usable.csv")?.name ?? source?.tables[0]?.name ?? "";
  const info = source?.tables.find((t) => t.name === table);
  const offset = Math.max(0, Number(route.query.get("offset")) || 0);
  const column = route.query.get("column") ?? "";
  const value = route.query.get("value") ?? "";
  const search = route.query.get("search") ?? "";
  const observer = !st.open || st.perspective?.mode === "observer";
  const q = useApi<Result>(source && table && observer ? "database.query" : null, { source: sourceId, table, column, value, search, offset, limit: 50 });
  const [dir, setDir] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [chosen, setChosen] = useState<{ key: string; row: number } | null>(null);
  const [draft, setDraft] = useState<{ key: string; search: string; column: string; value: string } | null>(null);
  const key = `${sourceId}|${table}|${column}|${value}|${search}|${offset}`;
  const activeDraft = draft?.key === key ? draft : { key, search, column, value };
  const selected = q.data?.rows.find((r) => chosen?.key === key && chosen.row === r.row) ?? q.data?.rows[0];
  const edit = (patch: Partial<NonNullable<typeof draft>>) => setDraft({ ...activeDraft, ...patch });
  const browse = (nextTable: string, nextColumn = "", nextValue = "") => {
    setQuery({ source: sourceId, table: nextTable, column: nextColumn, value: nextValue, search: null, offset: null }, { replace: false });
  };
  const attach = async () => {
    setBusy(true); setError(null);
    try {
      const result = await call<{ id: number }>("database.attach", { dir });
      sources.reload();
      setQuery({ source: result.id, table: null, column: null, value: null, search: null, offset: null });
      setDir("");
    } catch (e) { setError((e as Error).message); }
    finally { setBusy(false); }
  };
  const observe = async () => {
    setBusy(true); setError(null);
    try { await act("persp.observe", { omniscient: true }); }
    catch (e) { setError((e as Error).message); }
    finally { setBusy(false); }
  };
  const related = (field: string, v: string) => {
    if (info?.source === "FM23") return [];
    const choices = ["player_id", "player_in_id", "player_assist_id"].includes(field) ? [["players.csv", "player_id"], ["transfers.csv", "player_id"], ["player_valuations.csv", "player_id"], ["appearances.csv", "player_id"], ["game_lineups.csv", "player_id"], ["game_events.csv", "player_id"]]
      : field.endsWith("club_id") ? [["clubs.csv", "club_id"], ["club_games.csv", "club_id"], ["players.csv", "current_club_id"]]
      : field === "game_id" ? [["games.csv", "game_id"], ["appearances.csv", "game_id"], ["game_lineups.csv", "game_id"], ["game_events.csv", "game_id"], ["club_games.csv", "game_id"]]
      : field === "competition_id" ? [["competitions.csv", "competition_id"], ["games.csv", "competition_id"]] : [];
    return choices.filter(([t, c]) => source?.tables.some((x) => x.name === t) && !(t === table && c === column && v === value));
  };

  return <div className="page database-page">
    <PageHead title="Source database" sub="Original records, separate from the simulated world." actions={<a className="btn btn-default" href={href(st.open ? "/saves" : "/")}>World and saves</a>} />
    <form className="database-connect" onSubmit={(e) => { e.preventDefault(); void attach(); }}>
      <Field label="Connect a local dataset" hint="Choose the archive, a CSV pack, or the FM23 extracted folder. The files are read in place.">
        <div className="row-inline"><input type="text" aria-label="Dataset folder" value={dir} onChange={(e) => setDir(e.target.value)} placeholder="Folder path" /><Button type="submit" disabled={busy || !dir.trim()}>{busy ? "Connecting…" : "Connect folder"}</Button></div>
      </Field>
    </form>
    {error && <p role="alert" className="tone-neg">{error}</p>}
    {sources.error && <ErrorState error={sources.error} onRetry={sources.reload} />}
    {sources.data?.errors.map((e) => <p key={e} role="alert" className="muted">{e}</p>)}
    {!observer ? <div className="database-empty"><h2>Observer view</h2><p>Source data is separate from your character’s knowledge.</p><Button disabled={busy} onClick={() => void observe()}>Switch to the observer (debug) view</Button></div>
      : source ? <>
        <div className="database-source">
          <label>Dataset <select value={sourceId} onChange={(e) => setQuery({ source: e.target.value, table: null, column: null, value: null, search: null, offset: null })}>
            {sources.data?.sources.map((s) => <option key={s.id} value={s.id}>{s.path}</option>)}
          </select></label>
          <span className="hint">{source.tables.length} tables · {bytes(source.tables.reduce((n, t) => n + t.bytes, 0))}</span>
        </div>
        <div className="database-layout">
          <nav className="database-tables" aria-label="Source tables">
            {source.tables.map((t) => <button key={t.name} aria-current={table === t.name ? "page" : undefined} onClick={() => browse(t.name)}><span>{label(t.name)}</span><small>{bytes(t.bytes)}</small></button>)}
          </nav>
          <main className="database-content">
            <header className="database-table-head"><h2>{label(table)}</h2><p className="muted">{info?.source} · {info?.status}</p>
              <form className="database-filter" onSubmit={(e) => { e.preventDefault(); setQuery({ table, column: activeDraft.column, value: activeDraft.value, search: activeDraft.search, offset: null }); }}>
                <label>Name search<input type="text" value={activeDraft.search} onChange={(e) => edit({ search: e.target.value })} placeholder={q.data?.name_search ? "Name contains…" : "Use a source ID"} disabled={q.data ? !q.data.name_search : false} /></label>
                <label>Source ID field<select value={activeDraft.column} onChange={(e) => edit({ column: e.target.value })}><option value="">All records</option>{q.data?.indexed_columns.map((c) => <option key={c} value={c}>{label(c)}</option>)}{column && !q.data?.indexed_columns.includes(column) && <option value={column}>{label(column)}</option>}</select></label>
                <label>Exact ID<input type="text" value={activeDraft.value} onChange={(e) => edit({ value: e.target.value })} disabled={!activeDraft.column} /></label>
                <Button type="submit">Apply</Button><Button onClick={() => browse(table)}>Clear</Button>
              </form>
            </header>
            {!q.data && !q.error && <div className="database-empty" role="status"><Spinner /><p>Preparing this table. Its first index may take a few seconds.</p></div>}
            {q.error && <ErrorState error={q.error} onRetry={q.reload} />}
            {q.data && <>
              <div className="database-results-meta"><span>{fmtInt(q.data.matched)} matching · {fmtInt(q.data.total)} total records</span><span>Original source records</span></div>
              {q.data.malformed > 0 && <p className="tone-warn">{fmtInt(q.data.malformed)} malformed source rows could not be read.</p>}
              <div className="database-records">
                <div className="database-list" aria-label="Source records">
                  {q.data.rows.map((r) => <button key={r.row} aria-pressed={selected?.row === r.row} onClick={() => setChosen({ key, row: r.row })}>
                    <strong>{title(r)}</strong><small>{r.fields.date ?? r.fields.transfer_date ?? r.fields.date_of_birth ?? r.fields.short_name ?? `Record ${r.row + 1}`}</small>
                  </button>)}
                  {!q.data.rows.length && <p className="muted">No records match this query.</p>}
                </div>
                {selected && <section className="database-detail" aria-label="Selected record">
                  <h3>{title(selected)}</h3><p className="hint">Source record {selected.row + 1} · empty fields are unknown</p>
                  <dl>{q.data.columns.map((col) => {
                    const v = selected.fields[col]; const links = v ? related(col, v) : []; const ref = selected.refs?.[col];
                    return <div key={col}><dt>{label(col)}</dt><dd>{v ?? <span className="muted">Unknown</span>}
                      {(ref || links.length > 0) && <div className="database-links">{ref && <a href={href(refPath(ref))}>Open in world →</a>}{links.map(([t, c]) => <button key={`${t}:${c}`} onClick={() => browse(t, c, v!)}>{label(t)}</button>)}</div>}
                    </dd></div>;
                  })}</dl>
                </section>}
              </div>
              <footer className="database-pagination"><Button disabled={offset === 0} onClick={() => setQuery({ offset: Math.max(0, Math.min(offset, q.data!.matched) - 50) })}>Previous</Button><span>{q.data.rows.length ? `${offset + 1}–${offset + q.data.rows.length} of ${fmtInt(q.data.matched)}` : "0 records on this page"}</span><Button disabled={offset + 50 >= q.data.matched} onClick={() => setQuery({ offset: offset + 50 })}>Next</Button></footer>
            </>}
          </main>
        </div>
      </> : sources.data && <div className="database-empty"><h2>No dataset connected</h2><p>Importing an archive connects its source automatically. You can also connect existing exports above.</p></div>}
  </div>;
}
