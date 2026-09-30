import { useEffect, useState } from "react";
import { ApiError, call, inTauri } from "../api";
import { bytes, date, timeAgo } from "../format";
import { navigate } from "../router";
import { act, markSaved, refreshStatus, useApi, useStatus, worldChanged } from "../store";
import type { SaveInfo } from "../types";
import { Icon } from "../ui/Icon";
import { Button, Dialog, Field, Segmented, Spinner } from "../ui/ui";
import { usePageTitle } from "./common";

interface SavesResp {
  saves: SaveInfo[];
  dir: string;
}
interface Inspect {
  ok: boolean;
  files: { name: string; size: number }[];
  counts?: { nations: number; competitions: number; clubs: number; players: number; staff: number; unresolved?: number };
  warnings?: string[];
  findings?: { code: string; count: number }[];
  error?: string;
  database?: DatasetMetadata | null;
}

const INDIA_SCALES = [
  { id: "tiny", label: "Six states", note: "The pyramid and the district pools of six states. Quick to build." },
  { id: "regional", label: "Twelve states", note: "Twelve states with eight-club state leagues." },
  { id: "full", label: "All of India", note: "Every state and union territory in the pack. Slower to build and to run." },
];
interface DatasetMetadata {
  name: string;
  start_date: string;
  catalog_players: number;
  freshness_note: string;
}
interface DatasetsResp { datasets: { path: string; database: DatasetMetadata }[] }

const SCALES = [
  { id: "tiny", label: "Tiny", note: "8 clubs in one nation. Fast to try things." },
  { id: "small", label: "Small", note: "64 clubs in two nations, about 4,200 people." },
  { id: "huge", label: "Huge", note: "About 300,000 people in 40 nations. Slow to build and to run." },
];

/** A freshly created India world opens on the page where the player chooses where their life begins. */
let beginNext = false;

/** Start a world (or wait for one) and go to the right first page. */
export async function afterOpen() {
  const st = await refreshStatus();
  worldChanged();
  markSaved();
  const toBegin = beginNext && st.perspective?.mode !== "inhabit";
  beginNext = false;
  navigate(st.perspective?.mode === "inhabit" ? "/today" : toBegin ? "/begin" : "/overview", { replace: true });
}

const stem = (file: string) => file.replace(/\.pws$/, "");
const sameName = (name: string, file: string) => name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") === file.toLowerCase();

export function Start({ inApp = false }: { inApp?: boolean }) {
  usePageTitle(inApp ? "World and saves" : "Start");
  const st = useStatus();
  const saves = useApi<SavesResp>("world.saves");
  const datasets = useApi<DatasetsResp>("world.datasets");
  const [scale, setScale] = useState("small");
  const [indiaScale, setIndiaScale] = useState("tiny");
  const [kind, setKind] = useState<"synthetic" | "india" | "import">("synthetic");
  const [dir, setDir] = useState("");
  const [inspect, setInspect] = useState<Inspect | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<SaveInfo | null>(null);
  const task = st.task;
  const installed = datasets.data?.datasets[0];
  useEffect(() => {
    if (installed) { setDir((previous) => previous || installed.path); setKind("import"); }
  }, [installed?.path]);

  // When a build or load finishes, open it.
  const [awaiting, setAwaiting] = useState(false);
  useEffect(() => {
    if (awaiting && !task.running) {
      setAwaiting(false);
      setBusy(false);
      if (task.error) setErr(task.error);
      else if (st.open) void afterOpen();
    }
  }, [awaiting, task.running, task.error, st.open]);

  const begin = async (method: string, args: Record<string, unknown>) => {
    setErr(null);
    setBusy(true);
    beginNext = method === "world.new" && args.kind === "india";
    try {
      await call(method, args);
      setAwaiting(true);
      await refreshStatus();
    } catch (e) {
      setBusy(false);
      setErr((e as ApiError).message);
    }
  };

  const browse = async () => {
    // The desktop shell has a native folder picker; a browser has only the typed path.
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({ directory: true, title: "Choose the dataset folder" });
    if (typeof picked === "string") {
      setDir(picked);
      setInspect(null);
    }
  };
  const check = async () => {
    setErr(null);
    setInspect(null);
    try {
      setInspect(await call<Inspect>("world.inspect_import", { dir }));
    } catch (e) {
      setErr((e as ApiError).message);
    }
  };

  const list = saves.data?.saves ?? [];
  const latest = list[0];
  const working = busy || task.running;

  return (
    <div className={`start ${inApp ? "in-app" : ""}`}>
      <div className="start-col">
        {!inApp && (
          <header className="start-head">
            <span className="rail-mark big"><Icon name="up" size={22} strokeWidth={2.4} /></span>
            <div>
              <h1>Rise Above</h1>
              <div className="page-sub">A football world you can watch, or live in.</div>
            </div>
          </header>
        )}
        {inApp && (
          <header className="start-head">
            <div>
              <h1>World and saves</h1>
              <div className="page-sub">{st.open ? `${st.name} is open.` : "No world is open."}</div>
            </div>
            {st.open && (
              <div className="page-actions">
                <Button
                  onClick={async () => {
                    try {
                      await act("world.close");
                      worldChanged();
                      navigate("/", { replace: true });
                    } catch (e) {
                      setErr((e as Error).message);
                    }
                  }}
                  disabled={st.job.running}
                >
                  Close world
                </Button>
              </div>
            )}
          </header>
        )}

        {working && (
          <div className="card start-busy" role="status">
            <Spinner />
            <div>
              <div className="empty-title">{task.label || "Working"}</div>
              <div className="hint">This can take a moment for larger worlds.</div>
            </div>
          </div>
        )}
        {err && !working && (
          <div className="card start-error" role="alert">
            <Icon name="warn" />
            <div>{err}</div>
          </div>
        )}

        {!inApp && latest && !working && (
          <section className="card start-continue">
            <div>
              <div className="hint">Continue</div>
              <div className="start-save-name">{latest.info?.name ?? latest.file.replace(/\.pws$/, "")}</div>
              <div className="page-sub num">
                {latest.info?.date != null && `${date(latest.info.date)} · `}
                {latest.info?.perspective ? `${latest.info.perspective} · ` : ""}
                {latest.modified ? `saved ${timeAgo(latest.modified)}` : ""}
              </div>
            </div>
            <Button variant="primary" onClick={() => begin("world.load", { file: latest.file })}>Continue</Button>
          </section>
        )}

        <section className="card start-new">
          <h2>New world</h2>
          <Segmented
            label="World source"
            value={kind}
            onChange={setKind}
            options={[
              { id: "synthetic", label: "Test world" },
              { id: "india", label: "India pathway" },
              { id: "import", label: "Import a dataset" },
            ]}
          />
          <p><a href="#/database">Browse source database</a></p>
          {kind === "india" ? (
            <>
              <p className="muted">Football in India from the ground up: children in districts, schools and universities, state leagues, the national pyramid, and the scouts and coaches who notice them. Start anywhere on the route.</p>
              <div className="scale-list" role="radiogroup" aria-label="Size">
                {INDIA_SCALES.map((s) => (
                  <button key={s.id} role="radio" aria-checked={indiaScale === s.id} className="scale" onClick={() => setIndiaScale(s.id)}>
                    <strong>{s.label}</strong>
                    <span>{s.note}</span>
                  </button>
                ))}
              </div>
              <div className="start-actions">
                <Button variant="primary" disabled={working} onClick={() => begin("world.new", { kind: "india", scale: indiaScale })}>Create world</Button>
              </div>
            </>
          ) : kind === "synthetic" ? (
            <>
              <p className="muted">A generated league system with invented people, useful for trying everything out. Real data can be imported from a folder of CSV files.</p>
              <div className="scale-list" role="radiogroup" aria-label="Size">
                {SCALES.map((s) => (
                  <button key={s.id} role="radio" aria-checked={scale === s.id} className="scale" onClick={() => setScale(s.id)}>
                    <strong>{s.label}</strong>
                    <span>{s.note}</span>
                  </button>
                ))}
              </div>
              <div className="start-actions">
                <Button variant="primary" disabled={working} onClick={() => begin("world.new", { kind: "synthetic", scale })}>Create world</Button>
              </div>
            </>
          ) : (
            <>
                {installed && (
                  <div className="import-installed">
                    <strong>{installed.database.name}</strong>
                    <p className="muted">{installed.database.freshness_note}</p>
                    <Button onClick={() => { setDir(installed.path); setInspect(null); }}>Use installed database</Button>
                  </div>
                )}
              <Field label="Folder with the dataset" hint="The folder is read in place and is not changed. Files are checked before a world is built.">
                <div className="row-inline">
                  <input aria-label="Folder with the dataset" type="text" value={dir} onChange={(e) => { setDir(e.target.value); setInspect(null); }} placeholder="/path/to/dataset" spellCheck={false} />
                  {inTauri() && <Button onClick={browse}>Browse…</Button>}
                </div>
              </Field>
              {inspect && (
                <div className={`inspect ${inspect.ok ? "" : "bad"}`}>
                  {inspect.ok && inspect.counts ? (
                    <>
                      <div className="empty-title">Ready to import</div>
                      {inspect.database && <p className="muted">{inspect.database.freshness_note} The catalog contains {inspect.database.catalog_players.toLocaleString()} identities; the counts below are playable import records.</p>}
                      <div className="num muted">
                        {inspect.counts.nations} nations · {inspect.counts.competitions} competitions · {inspect.counts.clubs} clubs · {inspect.counts.players.toLocaleString()} players · {inspect.counts.staff.toLocaleString()} staff
                      </div>
                      {!!inspect.counts.unresolved && <div className="muted">{inspect.counts.unresolved.toLocaleString()} source rows cannot be placed safely and are left out (they are listed, never guessed).</div>}
                      {!!inspect.findings?.length && (
                        <details>
                          <summary>What the importer found</summary>
                          <ul>{inspect.findings.map((f) => <li key={f.code}><span className="num">{f.count.toLocaleString()}</span> {f.code.replace(/_/g, " ")}</li>)}</ul>
                        </details>
                      )}
                      {!!inspect.warnings?.length && (
                        <details>
                          <summary>{inspect.warnings.length} warnings</summary>
                          <ul>{inspect.warnings.slice(0, 20).map((w, i) => <li key={i}>{w}</li>)}</ul>
                        </details>
                      )}
                    </>
                  ) : (
                    <div>{inspect.error}</div>
                  )}
                  <details>
                    <summary>{inspect.files.length} files found</summary>
                    <ul className="num">{inspect.files.map((f) => <li key={f.name}>{f.name} <span className="faint">{bytes(f.size)}</span></li>)}</ul>
                  </details>
                </div>
              )}
              <div className="start-actions">
                <Button onClick={check} disabled={!dir.trim() || working}>Check folder</Button>
                <Button variant="primary" onClick={() => begin("world.new", { kind: "import", dir })} disabled={!dir.trim() || working || (inspect != null && !inspect.ok)}>Import</Button>
              </div>
            </>
          )}
        </section>

        <section className="card">
          <div className="section-head">
            <h2>Saved worlds</h2>
            <span className="hint">{saves.data?.dir}</span>
          </div>
          {list.length === 0 ? (
            <p className="muted" style={{ padding: "0.6rem 0" }}>Nothing saved yet. Worlds are saved from the status bar or with Ctrl+S.</p>
          ) : (
            <ul className="savelist">
              {list.map((s) => (
                <li key={s.file}>
                  <div className="save-main">
                    <div className="save-name">{s.info?.name ?? stem(s.file)}{s.info?.name && !sameName(s.info.name, stem(s.file)) && <span className="faint"> · {stem(s.file)}</span>}</div>
                    <div className="hint num">
                      {s.info?.date != null && `${date(s.info.date)} · `}
                      {s.info?.perspective ?? ""}{s.info?.players ? ` · ${s.info.players.toLocaleString()} people` : ""} · {bytes(s.size)}
                      {s.modified ? ` · ${timeAgo(s.modified)}` : ""}
                    </div>
                    {s.format && s.format.state !== "current" && <div className="hint">{s.format.note}</div>}
                  </div>
                  <Button size="sm" disabled={working || s.format?.state === "too_new" || s.format?.state === "unsupported" || s.format?.state === "unreadable"} title={s.format?.note || undefined} onClick={() => begin("world.load", { file: s.file })}>Load</Button>
                  {s.has_backup && <Button size="sm" variant="ghost" disabled={working} title="Load the previous save" onClick={() => begin("world.load", { file: s.file, backup: true })}>Previous save</Button>}
                  <Button size="sm" variant="ghost" onClick={() => setConfirmDelete(s)}>Delete</Button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
      <Dialog
        open={!!confirmDelete}
        onClose={() => setConfirmDelete(null)}
        title="Delete this save?"
        width={420}
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmDelete(null)}>Keep it</Button>
            <Button
              variant="danger"
              onClick={async () => {
                const f = confirmDelete?.file;
                setConfirmDelete(null);
                if (f) {
                  await call("world.delete_save", { file: f }).catch(() => undefined);
                  saves.reload();
                }
              }}
            >
              Delete
            </Button>
          </>
        }
      >
        <p>“{confirmDelete?.info?.name ?? confirmDelete?.file}” and its backup will be removed from disk. This cannot be undone.</p>
      </Dialog>
    </div>
  );
}
