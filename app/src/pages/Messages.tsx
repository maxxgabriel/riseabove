import { useState } from "react";
import { EntityLink, Money, Parts, Dt } from "../components/links";
import { prose, relativeDays } from "../format";
import { href, navigate, useRoute } from "../router";
import { act, notify, useApi, useStatus } from "../store";
import type { Named, Part } from "../types";
import { Icon } from "../ui/Icon";
import { Badge, Button, Dialog, Empty } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface Msg {
  id: string;
  kind: "decision" | "event";
  date: number;
  subject: string;
  preview: string | null;
  parts?: Part[];
  from: Named | null;
  state: "awaiting" | "answered" | "settled" | "expired" | "info";
  folder: "awaiting" | "contracts" | "work" | "invitations";
  needs_action: boolean;
  deadline: number | null;
}
interface TermRow {
  label: string;
  money?: number | null;
  date?: number;
  text?: string | null;
}
interface Detail {
  id: string;
  kind: "decision" | "event";
  title: string;
  from?: Named;
  created?: number;
  deadline?: number;
  state?: Msg["state"];
  paragraphs?: string[];
  options?: { i: number; label: string }[];
  answer?: number | null;
  default?: { i: number; label: string };
  without_response?: string;
  consequences?: string[];
  terms?: TermRow[] | null;
  current_terms?: TermRow[] | null;
  outcome?: string | null;
  date?: number;
  parts?: Part[];
  primary?: { k: string; id: number } | null;
}

const FOLDERS = [
  { id: "all", label: "All" },
  { id: "awaiting", label: "Needs an answer" },
  { id: "contracts", label: "Contracts and moves" },
  { id: "work", label: "Work" },
  { id: "invitations", label: "Invitations" },
] as const;

const STATE_LABEL: Record<Msg["state"], { text: string; tone: "warn" | "pos" | "muted" | "info" | "neg" }> = {
  awaiting: { text: "Needs an answer", tone: "warn" },
  answered: { text: "Answered", tone: "info" },
  settled: { text: "Settled", tone: "pos" },
  expired: { text: "Expired", tone: "muted" },
  info: { text: "", tone: "muted" },
};

export function Messages() {
  usePageTitle("Messages");
  const route = useRoute();
  const sel = route.segs[1];
  const folder = route.query.get("folder") ?? "all";
  const q = useApi<{ messages: Msg[]; awaiting: number }>("me.messages");
  return (
    <div className="page fill inbox-page">
      <PageHead title="Messages" sub="Offers, decisions and news about you." />
      <Async q={q}>
        {(d) => {
          const list = d.messages.filter((m) => folder === "all" || m.folder === folder);
          const selected = sel ?? list[0]?.id;
          return (
            <div className="inbox">
              <div className="inbox-list">
                <div className="chips" role="tablist" aria-label="Folders">
                  {FOLDERS.map((f) => {
                    const n = f.id === "awaiting" ? d.awaiting : 0;
                    return (
                      <button key={f.id} role="tab" aria-selected={folder === f.id} className="chip" aria-pressed={folder === f.id} onClick={() => navigate(`/messages${sel ? `/${sel}` : ""}${f.id === "all" ? "" : `?folder=${f.id}`}`, { replace: true })}>
                        {f.label}{n > 0 ? ` (${n})` : ""}
                      </button>
                    );
                  })}
                </div>
                {list.length === 0 ? (
                  <Empty title="Nothing here" icon="mail">Offers and news about you will arrive as time passes.</Empty>
                ) : (
                  <ul className="msglist" role="listbox" aria-label="Messages">
                    {list.map((m) => (
                      <li key={m.id}>
                        <a
                          href={href(`/messages/${m.id}${folder === "all" ? "" : `?folder=${folder}`}`)}
                          role="option"
                          aria-selected={m.id === selected}
                          className={`msg ${m.id === selected ? "sel" : ""} ${m.needs_action ? "urgent" : ""}`}
                        >
                          <div className="msg-top">
                            <strong className="msg-subject">{m.subject}</strong>
                            <span className="hint"><Dt d={m.date} year={false} /></span>
                          </div>
                          <div className="msg-sub">
                            {m.from ? m.from.name : ""}
                            {m.state !== "info" && <Badge tone={STATE_LABEL[m.state].tone}>{STATE_LABEL[m.state].text}</Badge>}
                          </div>
                          {m.preview && <div className="msg-preview">{prose(m.preview)}</div>}
                          {m.parts && <div className="msg-preview"><Parts parts={m.parts} /></div>}
                        </a>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div className="inbox-detail">{selected ? <MessageDetail id={selected} key={selected} /> : <Empty title="No message selected" icon="mail" />}</div>
            </div>
          );
        }}
      </Async>
    </div>
  );
}

/** Whether an offered term is better or worse for the player than what they have now, where that is unambiguous. */
function direction(label: string, cur: TermRow | undefined, next: TermRow): "pos" | "neg" | null {
  if (!cur) return null;
  const pct = (r: TermRow) => parseFloat(r.text ?? "");
  if (next.money != null && cur.money != null && next.money !== cur.money && /wage|bonus/i.test(label)) return next.money > cur.money ? "pos" : "neg";
  if (/relegat/i.test(label) && !Number.isNaN(pct(next)) && !Number.isNaN(pct(cur)) && pct(next) !== pct(cur)) return pct(next) > pct(cur) ? "neg" : "pos";
  if (/rise/i.test(label) && !Number.isNaN(pct(next)) && !Number.isNaN(pct(cur)) && pct(next) !== pct(cur)) return pct(next) > pct(cur) ? "pos" : "neg";
  return null;
}

function TermValue({ r }: { r: TermRow }) {
  if (r.text != null && r.text !== "") return <>{r.text}</>;
  if (r.date != null) return <Dt d={r.date} />;
  if (r.money != null) return <Money v={r.money} exact />;
  return <span className="faint">None</span>;
}

function MessageDetail({ id }: { id: string }) {
  const st = useStatus();
  const q = useApi<Detail>("me.message", { id });
  const [choice, setChoice] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const today = st.date ?? 0;

  const confirm = async (d: Detail) => {
    if (choice == null) return;
    setBusy(true);
    try {
      await act("me.answer", { id: d.id, choice });
      setChoice(null);
      q.reload();
      notify({ tone: "pos", text: "Your answer has been given. It takes effect when the day ends." });
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Async q={q}>
      {(d) => {
        if (d.kind === "event") {
          return (
            <article className="msg-article">
              <header>
                <h2>{d.title}</h2>
                {d.date != null && <div className="hint"><Dt d={d.date} /></div>}
              </header>
              <p className="msg-body">{d.parts && <Parts parts={d.parts} />}</p>
            </article>
          );
        }
        const open = d.state === "awaiting";
        const opt = d.options?.find((o) => o.i === choice);
        return (
          <article className="msg-article">
            <header>
              <div className="hint">{d.from && <>From <EntityLink r={d.from}>{d.from.name}</EntityLink> · </>}{d.created != null && <Dt d={d.created} />}</div>
              <h2>{d.title}</h2>
            </header>
            <div className="msg-body">
              {d.paragraphs?.map((p, i) => <p key={i}>{prose(p)}</p>)}
            </div>
            {d.terms && d.terms.length > 0 && (
              <div className="card list-card">
                <table className="minitable terms">
                  <thead>
                    <tr>
                      <th>Terms</th>
                      {d.current_terms && <th>Now</th>}
                      <th>{d.current_terms ? "Offered" : ""}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {d.terms.map((r, i) => {
                      const cur = d.current_terms?.[i];
                      const changed = cur && JSON.stringify([cur.money, cur.date, cur.text]) !== JSON.stringify([r.money, r.date, r.text]);
                      const dir = direction(r.label, cur, r);
                      return (
                        <tr key={i}>
                          <td className="muted">{r.label}</td>
                          {cur && <td className="num faint"><TermValue r={cur} /></td>}
                          <td className={`num ${changed ? "changed" : ""} ${dir ?? ""}`}><TermValue r={r} /></td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
            {d.consequences && d.consequences.length > 0 && (
              <ul className="consequences">
                {d.consequences.map((c, i) => <li key={i}>{prose(c)}</li>)}
              </ul>
            )}
            {open && d.options && (
              <div className="answer">
                <div className="hint">Reply by <Dt d={d.deadline} /> ({relativeDays(d.deadline ?? today, today)}).</div>
                <div className="answer-buttons">
                  {d.options.map((o, i) => (
                    <Button key={o.i} variant={i === 0 ? "primary" : "default"} onClick={() => setChoice(o.i)}>{o.label}</Button>
                  ))}
                </div>
                {d.without_response && <div className="hint">{prose(d.without_response)}</div>}
              </div>
            )}
            {!open && d.outcome && (
              <div className="note"><Icon name="check" size={15} /><span>{prose(d.outcome)}</span></div>
            )}
            {d.state === "answered" && (
              <div className="note"><Icon name="info" size={15} /><span>Your answer is recorded. It takes effect when the day ends.</span></div>
            )}
            <Dialog
              open={choice != null}
              onClose={() => setChoice(null)}
              title={opt ? `${opt.label}?` : "Confirm"}
              width={430}
              footer={
                <>
                  <Button variant="ghost" onClick={() => setChoice(null)}>Not yet</Button>
                  <Button variant="primary" disabled={busy} onClick={() => confirm(d)}>{opt?.label ?? "Confirm"}</Button>
                </>
              }
            >
              <p>This answer cannot be changed once it is given.</p>
              {d.consequences && d.consequences[d.options?.findIndex((o) => o.i === choice) ?? 0] && (
                <p className="muted">{d.consequences[d.options?.findIndex((o) => o.i === choice) ?? 0]}</p>
              )}
            </Dialog>
          </article>
        );
      }}
    </Async>
  );
}
