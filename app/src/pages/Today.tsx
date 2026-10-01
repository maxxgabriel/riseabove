import { EntityLink, Money, Parts, Dt } from "../components/links";
import { RatingChips } from "../components/visuals";
import { bandFill, cap, initials, dateLong, duration, ordinal, prose, relativeDays } from "../format";
import type { Band } from "../contract.generated";
import { href } from "../router";
import { act, notify, useApi, useStatus } from "../store";
import type { Named, Part } from "../types";
import { Icon, type IconName } from "../ui/Icon";
import { Avatar, Badge, Button, KeyVal, Meter, Section } from "../ui/ui";
import { Insights } from "../components/Insights";
import { Async, usePageTitle } from "./common";
import { tintOf } from "../color";
import { ClubCrest } from "../components/Crest";
import { useClubColors } from "../crest";
import { DEFAULT_TINT, ResultsStrip, Stage, StageHeader, type TeamBit, type TickerItem } from "../components/Stage";

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
type Word = Band;
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
  queued: string[];
  mind: { text: string; pull: "pos" | "neg" | "flat" }[];
  promises: { open: number; next_due: number | null };
  plan_pending: unknown | null;
  routine_hours: number;
  waiting_on: { kind: string; text: string; since: number; date?: number; ref?: { k: string; id: number } }[];
  known_faces: { who: Named; how: Part[]; role: string }[];
  buildup: null | { name: string | null; significance: number; lines: Part[][] };
  atmosphere: null | { mood: string; lines: string[] };
  recovery: null | {
    injury: string; since: number; treatment: string; estimate: number; sureness: string; stages: string[]; stage: number; setbacks: number;
    recurrence: boolean; rushed: boolean; physio: Named | null; missed: { uid: number; date: number; opponent: Named; score: string; outcome: string }[];
  };
}

const KIND_ICON: Record<string, IconName> = { match: "pitch", training: "training", recovery: "refresh", rest: "clock", medical: "warn", discipline: "warn" };

export function Today() {
  usePageTitle("Today");
  const q = useApi<TodayResp>("me.today");
  const st = useStatus();
  const club = q.data?.me.club ?? null;
  const colors = useClubColors(club?.id);
  return (
    <Stage tint={colors ? tintOf(colors[0]) : DEFAULT_TINT}>
      <Async q={q}>
        {(t) => (
          <>
            <StageHeader
              crest={club ? <ClubCrest id={club.id} name={club.name} size={58} plain={false} /> : <Avatar initials={initials(t.me.name)} size={58} you />}
              title="Today"
              sub={`${dateLong(t.date)}. You are ${t.me.name}${t.me.club ? `, ${t.me.position} at ${t.me.club.name}${t.me.team && t.me.team !== "First team" ? ` (${t.me.team})` : ""}` : `, without a club`}.`}
              meta={[
                ...(t.league ? [{ label: t.league.comp.name, value: <span>{ordinal(t.league.position)} <small>{t.league.points} pts</small></span> }] : []),
                ...(t.next_match ? [{ label: "Next match", value: <span>{t.next_match.home ? "v" : "at"} {t.next_match.opponent.name} <small>{relativeDays(t.next_match.date, t.date)}</small></span> }] : []),
                ...(t.contract ? [{ label: "Contract", value: <span>until <Dt d={t.contract.end} /></span> }] : []),
              ]}
            />
            <ResultsStrip items={strip(t)} />
            <div className="stage-body today-body">
            <div className="split today-layout">
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
                {t.queued.length > 0 && (
                  <Section title="Set in motion" aside="Happens when the day ends">
                    <div className="card list-card">
                      <ul className="rows">
                        {t.queued.map((q, i) => <li key={i}><span className="iconrow"><Icon name="clock" size={15} />{q}</span></li>)}
                      </ul>
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
                {t.next_match && <Insights method="insight.match" args={{ uid: t.next_match.uid }} title="Ahead of the match" limit={3} compact hideEmpty />}
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
                            <Meter value={bandFill(w)} label={w.label} />
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
                {t.mind.length > 0 && (
                  <Section title="On your mind" aside={<a href={href("/life")}>More</a>}>
                    <div className="card">
                      <ul className="rows compact">
                        {t.mind.slice(0, 4).map((m, i) => (
                          <li key={i}><span>{m.text}</span><Badge tone={m.pull === "pos" ? "pos" : m.pull === "neg" ? "warn" : "muted"}>{mindState(m.pull)}</Badge></li>
                        ))}
                      </ul>
                    </div>
                  </Section>
                )}
                {t.promises.open > 0 && (
                  <Section title="Promises" aside={<a href={href("/relationships?tab=promises")}>See all</a>}>
                    <div className="card">
                      <div>{t.promises.open} open {t.promises.open === 1 ? "promise" : "promises"}{t.promises.next_due != null && <>, the next due <Dt d={t.promises.next_due} year={false} /></>}.</div>
                    </div>
                  </Section>
                )}
                <Insights method="insight.person" args={{ id: t.me.person }} limit={4} compact hideEmpty aside={<a href={href(`/person/${t.me.person}`)}>All</a>} />
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
                        { k: "Intensity", v: cap(t.plan.intensity) },
                        { k: "Focus", v: focusText(t.plan.focus) },
                      ]}
                    />
                  </div>
                </Section>
              </aside>
            </div>
            </div>
          </>
        )}
      </Async>
      <span hidden>{st.revision}</span>
    </Stage>
  );
}

/** My team's recent results and the next match, as the strip under the header. */
function strip(t: TodayResp): TickerItem[] {
  const me = t.me.club;
  if (!me) return [];
  const mk = (f: FixtureBrief, status: TickerItem["status"]): TickerItem => {
    const mine: TeamBit = { k: "club", id: me.id, name: me.name, full: me.name, me: true };
    const opp: TeamBit = { k: "club", id: f.opponent.id, name: f.opponent.name, full: f.opponent.name, me: false };
    const [h, a] = status === "ft" && f.score ? f.score.split(/[–-]/).map((x) => Number(x.trim())) : [null, null];
    return { uid: f.uid, date: f.date, status, home: f.home ? mine : opp, away: f.home ? opp : mine, hs: h, as: a, pens: null };
  };
  const done = [...t.recent].reverse().map((f) => mk(f, f.concealed ? "held" : "ft"));
  return t.next_match ? [...done, mk(t.next_match, "next")] : done;
}

export function focusText(f: { kind: string; value: string | null }): string {
  if (f.kind === "general" || !f.value) return "General";
  if (f.kind === "group") return `${cap(f.value)} attributes`;
  if (f.kind === "position") return `Playing as ${f.value}`;
  return f.value.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());
}

/** Player-facing direction in place of the underlying score (its strength is in the sentence itself). */
function mindState(pull: "pos" | "neg" | "flat"): string {
  return pull === "pos" ? "Positive" : pull === "neg" ? "Under strain" : "Stable";
}

export function OutcomeChip({ o }: { o?: "win" | "draw" | "loss" }) {
  if (!o) return null;
  return <span className={`form-dot form-${o === "win" ? "w" : o === "draw" ? "d" : "l"}`} title={cap(o)}>{o === "win" ? "W" : o === "draw" ? "D" : "L"}</span>;
}
