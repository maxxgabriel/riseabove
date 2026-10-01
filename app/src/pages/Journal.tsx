import { useState } from "react";
import { Dt, EntityLink } from "../components/links";
import { act, notify, useApi } from "../store";
import type { Named } from "../types";
import { Badge, Button, Empty, Field, Progress, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface JournalResp {
  goals: { i: number; text: string; pinned: number; done: number | null; kind: string; progress?: { now: number; target: number } }[];
  notes: { i: number; date: number; text: string }[];
  history: { who: Named; from: number; to: number | null }[];
}

async function call(method: string, args: Record<string, unknown>) {
  try {
    await act(method, args);
    return true;
  } catch (e) {
    notify({ tone: "neg", text: (e as Error).message });
    return false;
  }
}

export function Journal() {
  usePageTitle("Journal");
  const q = useApi<JournalResp>("me.journal");
  const [kind, setKind] = useState("appearances");
  const [target, setTarget] = useState("");
  const [text, setText] = useState("");
  const [note, setNote] = useState("");
  return (
    <div className="page">
      <PageHead title="Journal" sub="Your own goals and notes. They are kept with the save and the world does not read them." />
      <Async q={q}>
        {(d) => (
          <div className="split">
            <div className="stack">
              <Section title="Goals">
                <div className="card list-card">
                  {d.goals.length === 0 ? (
                    <div className="muted pad">No goals yet. Set yourself one.</div>
                  ) : (
                    <ul className="rows">
                      {d.goals.map((g) => (
                        <li key={g.i}>
                          <div className="grow">
                            <div className={g.done != null ? "struck" : ""}>{g.text}</div>
                            {g.progress && g.done == null && (
                              <div className="promise-progress">
                                <Progress value={Math.min(g.progress.now, g.progress.target)} max={g.progress.target} label={g.text} />
                                <span className="hint">{g.progress.now} of {g.progress.target}</span>
                              </div>
                            )}
                            <div className="hint">Set <Dt d={g.pinned} year={false} />{g.done != null && <> · done <Dt d={g.done} year={false} /></>}</div>
                          </div>
                          {g.done != null ? <Badge tone="pos">Done</Badge> : g.kind === "personal" && <Button size="sm" onClick={() => call("me.goal_done", { i: g.i })}>Mark done</Button>}
                          <Button size="sm" variant="ghost" onClick={() => call("me.goal_done", { i: g.i, remove: true })}>Remove</Button>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
                <div className="card form">
                  <Field label="New goal">
                    <div className="row-inline">
                      <select value={kind} onChange={(e) => setKind(e.target.value)} aria-label="Kind of goal">
                        <option value="appearances">Senior appearances</option>
                        <option value="goals">Senior goals</option>
                        <option value="top_flight">Play in a top league</option>
                        <option value="personal">Something else</option>
                      </select>
                      {kind === "personal" ? (
                        <input type="text" value={text} placeholder="What do you want?" aria-label="Your goal" onChange={(e) => setText(e.target.value)} />
                      ) : kind !== "top_flight" ? (
                        <input type="number" min={1} value={target} placeholder="How many?" aria-label="How many" onChange={(e) => setTarget(e.target.value)} />
                      ) : null}
                      <Button variant="primary" disabled={(kind === "personal" && !text.trim()) || (kind !== "personal" && kind !== "top_flight" && !(Number(target) > 0))} onClick={async () => { if (await call("me.goal", { kind, target: Number(target) || 0, text })) { setText(""); setTarget(""); } }}>Add</Button>
                    </div>
                  </Field>
                </div>
              </Section>
            </div>
            <aside className="stack">
              <Section title="Notes">
                <div className="card form">
                  <textarea rows={3} value={note} placeholder="Write something down" aria-label="New note" onChange={(e) => setNote(e.target.value)} />
                  <div className="formfoot"><Button variant="primary" disabled={!note.trim()} onClick={async () => { if (await call("me.note", { text: note })) setNote(""); }}>Save note</Button></div>
                </div>
                {d.notes.length === 0 ? <Empty title="No notes" icon="info" /> : (
                  <div className="card list-card">
                    <ul className="rows">
                      {d.notes.map((n) => (
                        <li key={n.i}>
                          <div className="grow"><div className="note-text">{n.text}</div><div className="hint"><Dt d={n.date} /></div></div>
                          <Button size="sm" variant="ghost" onClick={() => call("me.note_remove", { i: n.i })}>Delete</Button>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </Section>
              {d.history.length > 1 && (
                <Section title="People you have been">
                  <div className="card list-card">
                    <ul className="rows">{d.history.map((h, i) => <li key={i}><EntityLink r={h.who}>{h.who.name}</EntityLink><span className="hint"><Dt d={h.from} year={false} />{h.to != null && <> to <Dt d={h.to} year={false} /></>}</span></li>)}</ul>
                  </div>
                </Section>
              )}
            </aside>
          </div>
        )}
      </Async>
    </div>
  );
}
