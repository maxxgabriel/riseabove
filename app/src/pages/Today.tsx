import { EntityLink, Money, Parts, Dt } from "../components/links";
import { RatingChips } from "../components/visuals";
import { dateLong, duration, ordinal, prose, relativeDays } from "../format";
import { href } from "../router";
import { act, notify, useApi, useStatus } from "../store";
import type { Named, Part } from "../types";
import { Icon, type IconName } from "../ui/Icon";
import { Badge, Button, KeyVal, Meter, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface FixtureBrief {
  uid: number;
  date: number;
  comp: Named;
  round: string;
  opponent: Named;
  home: boolean;
  days: number;
  venue: string | null;
  concealed?: boolean;
  score?: string;
  outcome?: "win" | "draw" | "loss";
}
type Word = { label: string; value: number };
export interface TodayResp {
  date: number;
  weekday: number;
  me: { person: number; name: string; age: number; position: string; club: Named | null; team: string | null; status: string; squad_status: string | null; shirt: number };
  day: { label: string; kind: string };
  commitments: { kind: string; text: string; ref?: { k: string; id: number } }[];
  decisions: { id: string; title: string; summary: string; deadline: number; from: Named }[];
  changes: { date: number; kind: string; parts: Part[] }[];
  next_match: FixtureBrief | null;
  recent: FixtureBrief[];
  unrevealed: FixtureBrief[];
  condition: Record<"condition" | "sharpness" | "morale" | "confidence" | "wellbeing" | "fatigue", Word>;
  availability: { injured: boolean; injury: string | null; days: number; ban: number };
  contract: null | { club: Named; end: number; days_left: number; status: string; wage: number };
  league: null | { comp: Named; position: number; teams: number; points: number };
  form: number[];
  minutes_4w: number;
  plan: { focus: { kind: string; value: string | null }; intensity: string; extra: number; recovery: number };
  conceal_mine: boolean;
}

const KIND_ICON: Record<string, IconName> = { match: "pitch", training: "training", recovery: "refresh", rest: "clock", medical: "warn", discipline: "warn" };

export function Today() {
  usePageTitle("Today");
  const q = useApi<TodayResp>("me.today");
  const st = useStatus();
  return (
    <div className="page">
      <Async q={q}>
        {(t) => (
          <>
            <PageHead
              title="Today"
              sub={`${dateLong(t.date)}. You are ${t.me.name}${t.me.club ? `, ${t.me.position} at ${t.me.club.name}${t.me.team && t.me.team !== "First team" ? ` (${t.me.team})` : ""}` : `, without a club`}.`}
            />
            <div className="split">
              <div className="stack">
                {t.decisions.length > 0 && (
                  <Section title="Waiting for your answer">
                    <div className="stack tight">
                      {t.decisions.map((d) => (
                        <div key={d.id} className="card attention">
                          <div>
                            <div className="attn-title">{d.title} <span className="muted">from</span> <EntityLink r={d.from}>{d.from.name}</EntityLink></div>
                            <div className="muted">{prose(d.summary)}</div>
                            <div className="hint">Reply by <Dt d={d.deadline} /> ({relativeDays(d.deadline, t.date)})</div>
                          </div>
                          <a className="btn btn-primary btn-md" href={href(`/messages/${d.id}`)}>Open</a>
                        </div>
                      ))}
                    </div>
                  </Section>
                )}
                <Section title="Today's plan">
                  <div className="card list-card">
                    <ul className="rows">
                      {t.commitments.map((c, i) => (
                        <li key={i}>
                          <span className="iconrow"><Icon name={KIND_ICON[c.kind] ?? "info"} size={15} />{c.ref ? <a href={href(`/match/${c.ref.id}`)}>{c.text}</a> : c.text}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                </Section>
                {t.next_match && (
                  <Section title="Next match" aside={<a href={href("/calendar")}>Calendar</a>}>
                    <a className="card nextmatch" href={href(`/match/${t.next_match.uid}`)}>
                      <div className="nm-main">
                        <div className="nm-opp">{t.next_match.home ? "v" : "at"} {t.next_match.opponent.name}</div>
                        <div className="muted">{t.next_match.comp.name} · {t.next_match.round}</div>
                      </div>
                      <div className="nm-when">
                        <div className="num"><strong>{relativeDays(t.next_match.date, t.date)}</strong></div>
                        <div className="hint"><Dt d={t.next_match.date} year={false} />{t.next_match.venue ? ` · ${t.next_match.venue}` : ""}</div>
                      </div>
                    </a>
                  </Section>
                )}
                <Section
                  title="Recent results"
                  aside={
                    t.unrevealed.length > 0 ? (
                      <Button size="sm" onClick={async () => { await act("match.reveal_all"); notify({ tone: "info", text: "All hidden results are now visible." }); }}>Show all hidden results</Button>
                    ) : undefined
                  }
                >
                  <div className="card list-card">
                    {t.recent.length === 0 ? (
                      <div className="muted pad">No matches played yet.</div>
                    ) : (
                      <ul className="rows">
                        {t.recent.map((f) => (
                          <li key={f.uid}>
                            <span className="result-row">
                              {f.concealed ? <Badge tone="muted"><Icon name="lock" size={11} />Hidden</Badge> : <OutcomeChip o={f.outcome} />}
                              <a href={href(`/match/${f.uid}`)}>{f.home ? "v" : "at"} {f.opponent.name}</a>
                              {!f.concealed && <span className="num"><strong>{f.score}</strong></span>}
                            </span>
                            <span className="hint">{f.comp.name} · <Dt d={f.date} year={false} /></span>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                  {t.conceal_mine && <p className="hint">Your team's results stay hidden until you show them. This can be changed in Settings.</p>}
                </Section>
                {t.changes.length > 0 && (
                  <Section title="Since you last looked">
                    <div className="card list-card">
                      <ul className="rows feed">
                        {t.changes.map((c, i) => (
                          <li key={i}><div><span className="feed-kind">{c.kind}</span><div><Parts parts={c.parts} /></div></div><span className="hint"><Dt d={c.date} year={false} /></span></li>
                        ))}
                      </ul>
                    </div>
                  </Section>
                )}
              </div>
              <aside className="stack">
                <Section title="You" aside={<a href={href(`/person/${t.me.person}`)}>Profile</a>}>
                  <div className="card">
                    <div className="condition">
                      {(["condition", "sharpness", "morale", "confidence", "wellbeing", "fatigue"] as const).map((k) => {
                        const w = t.condition[k];
                        const names = { condition: "Condition", sharpness: "Sharpness", morale: "Morale", confidence: "Confidence", wellbeing: "Wellbeing", fatigue: "Legs" };
                        return (
                          <div key={k} className="meter-row">
                            <span>{names[k]}</span>
                            <Meter value={k === "fatigue" ? 100 - w.value : w.value} label={w.label} />
                          </div>
                        );
                      })}
                    </div>
                    {(t.availability.injured || t.availability.ban > 0) && (
                      <div className="tags">
                        {t.availability.injured && <Badge tone="neg">Injured: {t.availability.injury?.toLowerCase()}, about {t.availability.days} days</Badge>}
                        {t.availability.ban > 0 && <Badge tone="warn">Suspended for {t.availability.ban} {t.availability.ban === 1 ? "match" : "matches"}</Badge>}
                      </div>
                    )}
                  </div>
                </Section>
                <Section title="Form">
                  <div className="card">
                    <KeyVal
                      rows={[
                        { k: "Recent ratings", v: <RatingChips values={t.form} /> },
                        { k: "Minutes, last 4 weeks", v: <span className="num">{t.minutes_4w}</span> },
                        ...(t.league ? [{ k: "League", v: <span><EntityLink r={t.league.comp}>{t.league.comp.name}</EntityLink>: {ordinal(t.league.position)} of {t.league.teams}, <span className="num">{t.league.points} pts</span></span> }] : []),
                      ]}
                    />
                  </div>
                </Section>
                {t.contract && (
                  <Section title="Contract" aside={<a href={href("/contract")}>Details</a>}>
                    <div className="card">
                      <KeyVal
                        rows={[
                          { k: "Club", v: <EntityLink r={t.contract.club}>{t.contract.club.name}</EntityLink> },
                          { k: "Role", v: t.contract.status },
                          { k: "Wage per week", v: <Money v={t.contract.wage} exact /> },
                          { k: "Runs until", v: <span><Dt d={t.contract.end} /> <span className="faint">({duration(t.contract.days_left)})</span></span> },
                        ]}
                      />
                    </div>
                  </Section>
                )}
                <Section title="Training" aside={<a href={href("/football")}>Change</a>}>
                  <div className="card">
                    <KeyVal
                      rows={[
                        { k: "Intensity", v: t.plan.intensity[0].toUpperCase() + t.plan.intensity.slice(1) },
                        { k: "Focus", v: focusText(t.plan.focus) },
                      ]}
                    />
                  </div>
                </Section>
              </aside>
            </div>
          </>
        )}
      </Async>
      <span hidden>{st.revision}</span>
    </div>
  );
}

export function focusText(f: { kind: string; value: string | null }): string {
  if (f.kind === "general" || !f.value) return "General";
  if (f.kind === "group") return `${f.value[0].toUpperCase()}${f.value.slice(1)} attributes`;
  if (f.kind === "position") return `Playing as ${f.value}`;
  return f.value.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());
}

export function OutcomeChip({ o }: { o?: "win" | "draw" | "loss" }) {
  if (!o) return null;
  return <span className={`form-dot form-${o === "win" ? "w" : o === "draw" ? "d" : "l"}`} title={o[0].toUpperCase() + o.slice(1)}>{o === "win" ? "W" : o === "draw" ? "D" : "L"}</span>;
}
