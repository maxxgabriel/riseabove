import { useEffect, useRef, useState } from "react";
import { EntityLink, Dt } from "../components/links";
import { OutcomeChip, focusText } from "./Today";
import { cap, fmtInt } from "../format";
import { href } from "../router";
import { act, notify, useApi } from "../store";
import type { Named } from "../types";
import { Badge, KeyVal, Section, Segmented } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface Plan {
  focus: { kind: string; value: string | null };
  intensity: "light" | "normal" | "high";
  extra: number;
  recovery: number;
}
interface FootballResp {
  team: string | null;
  squad_status: string;
  promised: string | null;
  minutes_4w: number;
  season: { apps: number; starts: number; minutes: number };
  plan: Plan;
  usage: { uid: number; date: number; comp: Named; opponent: Named; home: boolean; concealed?: boolean; score?: string; outcome?: "win" | "draw" | "loss"; played?: { started: boolean; minutes: number; rating: number; goals: number; assists: number } }[];
  rivals: { player: Named; pos: string; age: number; available: boolean; minutes_4w: number }[];
  options: { attributes: { group: string; key: string; label: string }[]; positions: { code: string }[] };
}

const INTENSITY = [
  { id: "light", label: "Light", text: "Less workload. Slower development, more freshness." },
  { id: "normal", label: "Normal", text: "The club's standard load." },
  { id: "high", label: "High", text: "More workload. Faster development, more tiredness." },
] as const;

export function Football() {
  usePageTitle("Football");
  const q = useApi<FootballResp>("me.football");
  return (
    <div className="page">
      <Async q={q}>
        {(f) => (
          <>
            <PageHead title="Football" sub={f.team ? `Your place in the ${f.team.toLowerCase()} squad and how you train.` : "You are not attached to a squad."} />
            <div className="split">
              <div className="stack">
                <TrainingPlan f={f} reload={q.reload} />
                <Section title="Recent matches">
                  <div className="card list-card">
                    {f.usage.length === 0 ? (
                      <div className="muted pad">You have not played for this team yet.</div>
                    ) : (
                      <ul className="rows">
                        {f.usage.map((u) => (
                          <li key={u.uid}>
                            <span className="result-row">
                              {u.concealed ? <Badge tone="muted">Hidden</Badge> : <OutcomeChip o={u.outcome} />}
                              <a href={href(`/match/${u.uid}`)}>{u.home ? "v" : "at"} {u.opponent.name}</a>
                              {!u.concealed && <span className="num"><strong>{u.score}</strong></span>}
                            </span>
                            <span className="hint num">
                              {u.concealed ? "" : u.played ? `${u.played.started ? "Started" : "Substitute"}, ${u.played.minutes} min${u.played.rating ? `, rated ${u.played.rating.toFixed(1)}` : ""}${u.played.goals ? `, ${u.played.goals} ${u.played.goals === 1 ? "goal" : "goals"}` : ""}${u.played.assists ? `, ${u.played.assists} ${u.played.assists === 1 ? "assist" : "assists"}` : ""}` : "Did not play"}
                              {" · "}<Dt d={u.date} year={false} />
                            </span>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </Section>
              </div>
              <aside className="stack">
                <Section title="Your place">
                  <div className="card">
                    <KeyVal
                      rows={[
                        { k: "Squad status", v: f.squad_status },
                        { k: "Promised", v: f.promised ?? "Nothing promised" },
                        { k: "Minutes, last 4 weeks", v: <span className="num">{fmtInt(f.minutes_4w)}</span> },
                        { k: "This season", v: <span className="num">{f.season.apps} apps, {f.season.starts} starts, {fmtInt(f.season.minutes)} min</span> },
                      ]}
                    />
                  </div>
                </Section>
                {f.rivals.length > 0 && (
                  <Section title="Competing for your role" aside="Same kind of position">
                    <div className="card list-card">
                      <ul className="rows">
                        {f.rivals.map((r) => (
                          <li key={r.player.id}>
                            <span><EntityLink r={r.player}>{r.player.name}</EntityLink> <span className="faint">{r.pos}, {r.age}</span></span>
                            <span className="hint num">{r.available ? `${r.minutes_4w} min` : "Unavailable"}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  </Section>
                )}
              </aside>
            </div>
          </>
        )}
      </Async>
    </div>
  );
}

function TrainingPlan({ f, reload }: { f: FootballResp; reload: () => void }) {
  const [plan, setPlan] = useState<Plan>(f.plan);
  const [saved, setSaved] = useState(true);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => setPlan(f.plan), [f.plan.intensity, f.plan.extra, f.plan.recovery, f.plan.focus.kind, f.plan.focus.value]);

  const change = (patch: Partial<Plan>) => {
    const next = { ...plan, ...patch };
    setPlan(next);
    setSaved(false);
    clearTimeout(timer.current);
    timer.current = setTimeout(async () => {
      try {
        await act("me.plan", { intensity: next.intensity, extra: next.extra, recovery: next.recovery, focus: { kind: next.focus.kind, value: next.focus.value ?? "" } });
        setSaved(true);
        reload();
      } catch (e) {
        notify({ tone: "neg", text: (e as Error).message });
      }
    }, 350);
  };

  const groups = ["technical", "mental", "physical", "goalkeeping"];
  const attrsByGroup = f.options.attributes;
  const kind = plan.focus.kind;

  return (
    <Section title="Training plan" aside={saved ? <span className="hint">Saved</span> : <span className="hint">Saving…</span>}>
      <div className="card plan">
        <div className="plan-row">
          <div>
            <div className="plan-label">Intensity</div>
            <div className="hint">{INTENSITY.find((i) => i.id === plan.intensity)?.text}</div>
          </div>
          <Segmented label="Training intensity" value={plan.intensity} onChange={(v) => change({ intensity: v })} options={INTENSITY.map((i) => ({ id: i.id, label: i.label }))} />
        </div>
        <div className="plan-row">
          <div>
            <div className="plan-label">Extra sessions</div>
            <div className="hint">Personal sessions on top of the club schedule. They add load and development.</div>
          </div>
          <Segmented label="Extra sessions" value={plan.extra} onChange={(v) => change({ extra: v })} options={[0, 1, 2, 3].map((n) => ({ id: n, label: String(n) }))} />
        </div>
        <div className="plan-row">
          <div>
            <div className="plan-label">Recovery work</div>
            <div className="hint">Time spent recovering lowers built-up tiredness.</div>
          </div>
          <Segmented label="Recovery" value={plan.recovery} onChange={(v) => change({ recovery: v })} options={[{ id: 0, label: "None" }, { id: 1, label: "Some" }, { id: 2, label: "A lot" }]} />
        </div>
        <div className="plan-row">
          <div>
            <div className="plan-label">Focus</div>
            <div className="hint">Where your development effort goes. Currently: {focusText(plan.focus)}.</div>
          </div>
          <div className="plan-focus">
            <select aria-label="Focus type" value={kind} onChange={(e) => {
              const k = e.target.value;
              const first = k === "group" ? "technical" : k === "attribute" ? attrsByGroup[0]?.key : k === "position" ? f.options.positions[0]?.code : null;
              change({ focus: { kind: k, value: first ?? null } });
            }}>
              <option value="general">General</option>
              <option value="group">A kind of attribute</option>
              <option value="attribute">One attribute</option>
              <option value="position">Learning a position</option>
            </select>
            {kind === "group" && (
              <select aria-label="Attribute group" value={plan.focus.value ?? "technical"} onChange={(e) => change({ focus: { kind, value: e.target.value } })}>
                {groups.filter((g) => g !== "goalkeeping" || attrsByGroup.some((a) => a.group.toLowerCase() === "goalkeeping")).map((g) => <option key={g} value={g}>{cap(g)}</option>)}
              </select>
            )}
            {kind === "attribute" && (
              <select aria-label="Attribute" value={plan.focus.value ?? ""} onChange={(e) => change({ focus: { kind, value: e.target.value } })}>
                {[...new Set(attrsByGroup.map((a) => a.group))].map((g) => (
                  <optgroup key={g} label={g}>
                    {attrsByGroup.filter((a) => a.group === g).map((a) => <option key={a.key} value={a.key}>{a.label}</option>)}
                  </optgroup>
                ))}
              </select>
            )}
            {kind === "position" && (
              <select aria-label="Position" value={plan.focus.value ?? ""} onChange={(e) => change({ focus: { kind, value: e.target.value } })}>
                {f.options.positions.map((p) => <option key={p.code} value={p.code}>{p.code}</option>)}
              </select>
            )}
          </div>
        </div>
        <p className="hint">The simulation applies this plan to your workload, tiredness and development. Club-wide programmes and coach approval are not simulated yet.</p>
      </div>
    </Section>
  );
}
