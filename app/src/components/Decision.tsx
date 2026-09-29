import { useState } from "react";
import { prose, relativeDays } from "../format";
import { act, notify, useApi, useStatus } from "../store";
import type { Named, Part } from "../types";
import { Icon } from "../ui/Icon";
import { Badge, Button, Dialog } from "../ui/ui";
import { Async } from "../pages/common";
import { Dt, EntityLink, Money } from "./links";

// ---- wire types -------------------------------------------------------------------------------

export interface TermRow {
  label: string;
  money?: number | null;
  date?: number;
  text?: string | null;
}
export interface DecisionOption {
  i: number;
  label: string;
  kind: string;
  positive: boolean;
  default: boolean;
  effect?: string;
  tone?: string;
  counter?: { wage: number; years: number; status: string | null; release_clause: number };
}
interface TalkLog {
  date: number;
  side: "club" | "you" | "agent";
  text: string;
}
interface MeetingLine {
  side: "you" | "them";
  name: string;
  ref: { k: "person"; id: number };
  text: string;
  tone: string;
}
export interface DecisionDetail {
  id: string;
  kind: "decision" | "event";
  dkind?: string;
  title: string;
  from?: Named | string | null;
  created?: number;
  deadline?: number;
  state?: "awaiting" | "answered" | "settled" | "expired" | "info";
  paragraphs?: string[];
  options?: DecisionOption[];
  answer?: number | null;
  default?: { i: number; label: string };
  without_response?: string | null;
  consequences?: string[];
  terms?: TermRow[] | null;
  current_terms?: TermRow[] | null;
  outcome?: string | null;
  talk?: {
    kind: string;
    club: Named;
    state: "your_turn" | "club_turn" | "agreed" | "collapsed";
    round: number;
    max_rounds: number;
    agent: Named | null;
    fee: number | null;
    seller: Named | null;
    log: TalkLog[];
  } | null;
  meeting?: {
    topic: string;
    date: number;
    state: "pending" | "held" | "lapsed";
    lines: MeetingLine[];
    outcomes: string[];
    with: Named;
    why: string[];
  } | null;
  incident?: {
    what: string;
    place: string;
    date: number;
    parties: Named[];
    witnesses: number;
    club: Named | null;
    responses: string[];
    resolved: boolean;
  } | null;
  press?: {
    club: Named;
    date: number;
    journalist: Named;
    outlet: string | null;
    follow_up: boolean;
    number: number;
    of: number;
    earlier: { question: string; stance: string }[];
  } | null;
  // Events
  date?: number;
  parts?: Part[];
  primary?: { k: string; id: number } | null;
  story?: { outlet: string; headline: string; body: string; date: number } | null;
  why?: string[];
}

// ---- pieces -----------------------------------------------------------------------------------

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

export function TermsTable({ terms, current }: { terms: TermRow[]; current?: TermRow[] | null }) {
  return (
    <div className="card list-card">
      <table className="minitable terms">
        <thead>
          <tr>
            <th>Terms</th>
            {current && <th>Now</th>}
            <th>{current ? "Offered" : ""}</th>
          </tr>
        </thead>
        <tbody>
          {terms.map((r, i) => {
            const cur = current?.[i];
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
  );
}

const SIDE_NAME: Record<TalkLog["side"], string> = { club: "Club", you: "You", agent: "Your agent" };

export function TalkBlock({ t }: { t: NonNullable<DecisionDetail["talk"]> }) {
  const state = { your_turn: "Your move", club_turn: "Waiting for the club", agreed: "Agreed", collapsed: "Talks ended" }[t.state];
  return (
    <div className="card">
      <div className="talk-head">
        <div>
          <strong>{t.kind} talks with <EntityLink r={t.club}>{t.club.name}</EntityLink></strong>
          <div className="hint">
            Round {Math.min(t.round, t.max_rounds)} of {t.max_rounds}
            {t.seller && <> · buying from <EntityLink r={t.seller}>{t.seller.name}</EntityLink></>}
            {t.fee != null && <> · fee <Money v={t.fee} /></>}
          </div>
        </div>
        <Badge tone={t.state === "agreed" ? "pos" : t.state === "collapsed" ? "neg" : t.state === "your_turn" ? "warn" : "info"}>{state}</Badge>
      </div>
      {t.agent && <div className="hint">Your agent {t.agent.name} is handling the details.</div>}
      {t.log.length > 0 && (
        <ol className="talklog" aria-label="What has been said">
          {t.log.map((l, i) => (
            <li key={i} className={`side-${l.side}`}>
              <span className="talklog-who">{SIDE_NAME[l.side]}</span>
              <span>{prose(l.text)}</span>
              <span className="hint"><Dt d={l.date} year={false} /></span>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

export function MeetingBlock({ m }: { m: NonNullable<DecisionDetail["meeting"]> }) {
  return (
    <div className="card">
      <div className="talk-head">
        <div>
          <strong>About {m.topic}</strong>
          <div className="hint">With <EntityLink r={m.with}>{m.with.name}</EntityLink> · <Dt d={m.date} /></div>
        </div>
        {m.state === "lapsed" && <Badge tone="muted">Never took place</Badge>}
        {m.state === "pending" && <Badge tone="warn">Waiting</Badge>}
      </div>
      <ol className="transcript">
        {m.lines.map((l, i) => (
          <li key={i} className={l.side}>
            <div className="bubble">
              <div className="bubble-who"><EntityLink r={l.ref}>{l.name}</EntityLink> <span className="hint">({l.tone})</span></div>
              <div>“{prose(l.text)}”</div>
            </div>
          </li>
        ))}
      </ol>
      {m.outcomes.length > 0 && (
        <ul className="outcomes">
          {m.outcomes.map((o, i) => (
            <li key={i}><Icon name="check" size={13} /> {prose(o)}</li>
          ))}
        </ul>
      )}
      {m.why.length > 0 && <div className="hint">Why it came up: {m.why.map(prose).join("; ")}</div>}
    </div>
  );
}

export function StoryCard({ s }: { s: NonNullable<DecisionDetail["story"]> }) {
  return (
    <div className="card story">
      <div className="hint">{s.outlet} · <Dt d={s.date} /></div>
      <h3 className="story-head">{prose(s.headline)}</h3>
      <p>{prose(s.body)}</p>
    </div>
  );
}

function IncidentBlock({ x }: { x: NonNullable<DecisionDetail["incident"]> }) {
  return (
    <div className="card">
      <div className="talk-head">
        <div>
          <strong>What happened</strong>
          <div className="hint">{x.place} · <Dt d={x.date} />{x.club && <> · <EntityLink r={x.club}>{x.club.name}</EntityLink></>}</div>
        </div>
        {x.resolved ? <Badge tone="pos">Settled</Badge> : <Badge tone="warn">Open</Badge>}
      </div>
      <p>{prose(x.what)}</p>
      <div className="hint">
        {x.parties.length > 0 && <>Involved: {x.parties.map((p, i) => <span key={p.id}>{i > 0 && ", "}<EntityLink r={p}>{p.name}</EntityLink></span>)}. </>}
        {x.witnesses > 0 && <>{x.witnesses} {x.witnesses === 1 ? "person" : "people"} saw it.</>}
      </div>
      {x.responses.length > 0 && (
        <ul className="outcomes">
          {x.responses.map((r, i) => <li key={i}><Icon name="check" size={13} /> {prose(r)}</li>)}
        </ul>
      )}
    </div>
  );
}

function PressBlock({ p }: { p: NonNullable<DecisionDetail["press"]> }) {
  return (
    <div className="card">
      <div className="talk-head">
        <div>
          <strong>Press conference, <EntityLink r={p.club}>{p.club.name}</EntityLink></strong>
          <div className="hint">Question {p.number} of {p.of} · <Dt d={p.date} /> · <EntityLink r={p.journalist}>{p.journalist.name}</EntityLink>{p.outlet && <>, {p.outlet}</>}</div>
        </div>
        {p.follow_up && <Badge tone="info">Follow-up</Badge>}
      </div>
      {p.earlier.length > 0 && (
        <ul className="outcomes">
          {p.earlier.map((e, i) => <li key={i}><Icon name="check" size={13} /> {prose(e.question)} <span className="faint">You said: {e.stance.toLowerCase()}.</span></li>)}
        </ul>
      )}
    </div>
  );
}

function optionSummary(o: DecisionOption): string | null {
  if (!o.counter) return null;
  const c = o.counter;
  const bits = [`${c.years} year${c.years === 1 ? "" : "s"}`];
  if (c.status) bits.push(c.status);
  if (c.release_clause > 0) bits.push("with a release clause");
  return bits.join(", ");
}

// ---- the card ---------------------------------------------------------------------------------

/** One decision, complete: what it is about, what it would mean, and how to answer it. */
export function DecisionCard({ id, showTitle = true }: { id: string; showTitle?: boolean }) {
  const st = useStatus();
  const q = useApi<DecisionDetail>("me.message", { id });
  const [choice, setChoice] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const today = st.date ?? 0;

  const confirm = async (d: DecisionDetail) => {
    if (choice == null) return;
    setBusy(true);
    try {
      await act("me.answer", { id: d.id, choice });
      setChoice(null);
      q.reload();
      notify({ tone: "pos", text: "Your answer is recorded. It takes effect when the day ends, and you can change it until then." });
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Async q={q}>
      {(d) => {
        const open = d.state === "awaiting" || d.state === "answered";
        const opt = d.options?.find((o) => o.i === choice);
        const chosen = d.answer != null ? d.options?.find((o) => o.i === d.answer) : undefined;
        const detailed = d.options?.some((o) => o.effect || o.counter);
        return (
          <article className="msg-article decision">
            {showTitle && (
              <header>
                <div className="hint">
                  {d.from && typeof d.from === "object" && <>From <EntityLink r={d.from}>{d.from.name}</EntityLink> · </>}
                  {typeof d.from === "string" && <>From {d.from} · </>}
                  {d.created != null && <Dt d={d.created} />}
                </div>
                <h2>{d.title}</h2>
              </header>
            )}
            <div className="msg-body">{d.paragraphs?.map((p, i) => <p key={i}>{prose(p)}</p>)}</div>
            {d.press && <PressBlock p={d.press} />}
            {d.incident && <IncidentBlock x={d.incident} />}
            {d.meeting && <MeetingBlock m={d.meeting} />}
            {d.talk && <TalkBlock t={d.talk} />}
            {d.terms && d.terms.length > 0 && <TermsTable terms={d.terms} current={d.current_terms} />}
            {d.consequences && d.consequences.length > 0 && (
              <ul className="consequences">
                {d.consequences.map((c, i) => <li key={i}>{prose(c)}</li>)}
              </ul>
            )}
            {open && d.options && d.options.length > 0 && (
              <div className="answer">
                {d.deadline != null && <div className="hint">Answer by <Dt d={d.deadline} /> ({relativeDays(d.deadline, today)}).</div>}
                {chosen && <div className="note"><Icon name="check" size={15} /><span>You chose: <strong>{chosen.label}</strong>. You can change this until the day ends.</span></div>}
                {detailed ? (
                  <ul className="optlist">
                    {d.options.map((o) => (
                      <li key={o.i} className={o.i === d.answer ? "chosen" : ""}>
                        <div>
                          <div className="optlist-label">{o.label}{o.default && <span className="hint"> · your default</span>}</div>
                          {optionSummary(o) && <div className="muted">{optionSummary(o)}</div>}
                          {o.counter && <div className="muted">Asks for <Money v={o.counter.wage} exact /> a week.</div>}
                          {o.effect && <div className="muted">{o.effect}</div>}
                        </div>
                        <Button size="sm" variant={o.i === d.answer ? "default" : "primary"} onClick={() => setChoice(o.i)} disabled={o.i === d.answer}>{o.i === d.answer ? "Chosen" : "Choose"}</Button>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <div className="answer-buttons">
                    {d.options.map((o, i) => (
                      <Button key={o.i} variant={o.i === d.answer ? "default" : i === 0 && d.answer == null ? "primary" : "default"} onClick={() => setChoice(o.i)} disabled={o.i === d.answer}>
                        {o.label}
                      </Button>
                    ))}
                  </div>
                )}
                {d.without_response && d.state === "awaiting" && <div className="hint">{prose(d.without_response)}</div>}
              </div>
            )}
            {!open && d.outcome && <div className="note"><Icon name="check" size={15} /><span>{prose(d.outcome)}</span></div>}
            <Dialog
              open={choice != null}
              onClose={() => setChoice(null)}
              title={opt ? `${opt.label}?` : "Confirm"}
              width={440}
              footer={
                <>
                  <Button variant="ghost" onClick={() => setChoice(null)}>Not yet</Button>
                  <Button variant="primary" disabled={busy} onClick={() => confirm(d)}>{opt?.label ?? "Confirm"}</Button>
                </>
              }
            >
              {opt?.effect && <p>{opt.effect}</p>}
              <p className="muted">Nothing happens until the day ends. Until then you can come back and answer differently.</p>
            </Dialog>
          </article>
        );
      }}
    </Async>
  );
}

/** A dated, linkless label for a decision kind, used in lists. */
export function kindLabel(k: string | undefined): string {
  switch (k) {
    case "transfer_talks": return "Transfer talks";
    case "renewal": return "New contract";
    case "contract": return "Contract offer";
    case "loan": return "Loan";
    case "free_agent": return "Contract offer";
    case "negotiation": return "Contract talks";
    case "meeting": return "Conversation";
    case "partner": return "Home life";
    case "trial": return "Trial";
    case "nation": return "Allegiance";
    case "treatment": return "Treatment";
    case "endorsement": return "Endorsement";
    case "incident": return "Incident";
    case "incident_leave": return "Time away";
    case "incident_apology": return "Apology";
    case "press_question": return "Press question";
    case "appeal": return "Appeal";
    default: return "Decision";
  }
}

