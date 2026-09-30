import { bandFill, cap, pullWord } from "../format";
import type { Band } from "../contract.generated";
import { useEffect, useMemo, useState } from "react";
import { ConfirmAction, queueAction, useOptions, type Options } from "../components/Actions";
import { Dt, EntityLink, Money } from "../components/links";
import { navigate, useRoute } from "../router";
import { useApi } from "../store";
import type { Named } from "../types";
import { Badge, Button, Field, KeyVal, Meter, Section, Segmented, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

type Word = Band;
interface Factor {
  text: string;
  factor: string;
  pull: "pos" | "neg" | "flat";
}
interface SelfResp {
  name: string;
  age: number;
  nation: Named;
  personality: string;
  hint: string | null;
  mood: Factor[];
  wellbeing: Factor[];
  told: { text: string; date: number; sureness: string }[];
  career: { apps: number; goals: number; caps: number; position: string; status: string; clubs: number } | null;
  stress: Word;
  sleep: Word;
  fulfilment: Word;
}
interface LifeResp {
  home: { nation: Named; since: number; kind: string | null; quality: number | null };
  languages: { nation: Named; level: Band }[];
  partner: { who: Named; status: string; since: number; bond: Word; lives: string | null; occupation: string } | null;
  children: number;
  siblings: number;
  parents: { alive: number; nation: string | null; health: string; closeness: string };
  money: { savings: number; debt: number; income: number; spending: number; family_support: number; lifestyle: string; invested: number };
  routine: { rows: { key: string; label: string; hours: number }[]; total: number; budget: number };
  occupation: string;
  education: number;
  studying: { course: string; done: number; effort: number } | null;
  qualifications: { label: string; date: number }[];
  helpers: { label: string; quality: number; cost: number }[];
  giving: { pct: number; community: number; foundation: boolean } | null;
  work: { path: string; income: number; standing: string; since: number } | null;
  open_to_dating: boolean | null;
}

type TabId = "overview" | "week" | "money" | "home" | "work";

export function Life() {
  usePageTitle("Life");
  const route = useRoute();
  const tab = (route.query.get("tab") as TabId) || "overview";
  const me = useApi<SelfResp>("me.self");
  const life = useApi<LifeResp>("me.life");
  return (
    <div className="page">
      <PageHead title="Life" sub="How you are, how you spend your week, and everything outside the pitch." />
      <Tabs
        label="Life"
        value={tab}
        onChange={(t) => navigate(t === "overview" ? "/life" : `/life?tab=${t}`, { replace: true })}
        tabs={[
          { id: "overview", label: "How you are" },
          { id: "week", label: "Your week" },
          { id: "money", label: "Money" },
          { id: "home", label: "Home and family" },
          { id: "work", label: "Study and work" },
        ]}
      />
      <Async q={me}>
        {(s) => (
          <Async q={life}>
            {(l) => (
              <div className="tabbody">
                {tab === "overview" && <Overview s={s} l={l} />}
                {tab === "week" && <Week l={l} />}
                {tab === "money" && <MoneyTab l={l} />}
                {tab === "home" && <Home l={l} />}
                {tab === "work" && <Work l={l} s={s} />}
              </div>
            )}
          </Async>
        )}
      </Async>
    </div>
  );
}

// ---- how you are ------------------------------------------------------------------------------

function FactorList({ items }: { items: Factor[] }) {
  if (items.length === 0) return <p className="muted">Nothing stands out.</p>;
  return (
    <ul className="rows compact">
      {items.map((f, i) => (
        <li key={i}>
          <span>{f.text}</span>
          <span className={f.pull === "pos" ? "tone-pos" : f.pull === "neg" ? "tone-neg" : "muted"}>{pullWord(f.pull)}</span>
        </li>
      ))}
    </ul>
  );
}

function Overview({ s, l }: { s: SelfResp; l: LifeResp }) {
  return (
    <div className="split">
      <div className="stack">
        <Section title="Your state">
          <div className="card">
            <div className="meters">
              <div className="meter-row"><span>Fulfilment</span><Meter value={bandFill(s.fulfilment)} label={s.fulfilment.label} /></div>
              <div className="meter-row"><span>Sleep</span><Meter value={bandFill(s.sleep)} label={s.sleep.label} /></div>
              <div className="meter-row"><span>Calm (stress)</span><Meter value={bandFill(s.stress)} label={s.stress.label} /></div>
            </div>
            {s.hint && <p className="muted pt">{s.hint}</p>}
          </div>
        </Section>
        <Section title="What is affecting your mood" aside="On the pitch and around the club">
          <div className="card"><FactorList items={s.mood} /></div>
        </Section>
        <Section title="What is affecting how you live" aside="Away from football">
          <div className="card"><FactorList items={s.wellbeing} /></div>
        </Section>
      </div>
      <aside className="stack">
        <Section title="You">
          <div className="card">
            <KeyVal
              rows={[
                { k: "Age", v: s.age },
                { k: "Nation", v: <EntityLink r={s.nation}>{s.nation.name}</EntityLink> },
                { k: "Personality", v: s.personality },
                { k: "Lives in", v: <EntityLink r={l.home.nation}>{l.home.nation.name}</EntityLink> },
                ...(s.career ? [{ k: "Career", v: `${s.career.apps} appearances, ${s.career.goals} goals${s.career.caps ? `, ${s.career.caps} caps` : ""}` }] : []),
              ]}
            />
          </div>
        </Section>
        <Section title="What you have been told" aside="Not always right">
          <div className="card list-card">
            {s.told.length === 0 ? (
              <div className="muted pad">Nobody has told you what they think of you lately.</div>
            ) : (
              <ul className="rows">
                {s.told.slice(0, 6).map((t, i) => (
                  <li key={i}>
                    <span>{t.text}</span>
                    <span className="hint"><Dt d={t.date} year={false} /></span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </Section>
      </aside>
    </div>
  );
}

// ---- your week ------------------------------------------------------------------------------------

function Week({ l }: { l: LifeResp }) {
  const opts = useOptions();
  const [hours, setHours] = useState<Record<string, number>>({});
  useEffect(() => {
    setHours(Object.fromEntries(l.routine.rows.map((r) => [r.key, r.hours])));
  }, [l.routine]);
  const total = Object.values(hours).reduce((a, b) => a + b, 0);
  const over = total > l.routine.budget;
  const changed = l.routine.rows.some((r) => hours[r.key] !== r.hours);
  const set = (k: string, v: number) => setHours((h) => ({ ...h, [k]: Math.max(0, Math.min(60, v)) }));
  return (
    <div className="split">
      <div className="stack">
        <Section title="How you spend your week" aside={<span className={over ? "tone-neg" : ""}>{total} of {l.routine.budget} hours</span>}>
          <div className="card">
            <p className="muted pb">Football takes what it takes. This is the time you have left. The world reads it every day: rest, family and friends change how you feel, nights out and study change what you can do.</p>
            <ul className="sliders">
              {l.routine.rows.map((r) => (
                <li key={r.key}>
                  <label htmlFor={`h-${r.key}`}>{r.label}</label>
                  <input id={`h-${r.key}`} type="range" min={0} max={30} value={hours[r.key] ?? r.hours} onChange={(e) => set(r.key, Number(e.target.value))} />
                  <span className="num sliders-val">{hours[r.key] ?? r.hours}</span>
                </li>
              ))}
            </ul>
            <div className="formfoot">
              {over && <span className="tone-neg">That is more hours than a week has. The world will cut the excess.</span>}
              <Button variant="primary" disabled={!changed} onClick={() => queueAction("routine", { hours })}>Change my week</Button>
            </div>
          </div>
        </Section>
      </div>
      <aside className="stack">
        <Section title="Lifestyle">
          <div className="card">
            <p className="muted pb">How you live when you spend money. It affects your savings, your mood and what people say.</p>
            {opts.data && (
              <Segmented
                label="Lifestyle"
                value={l.money.lifestyle}
                options={opts.data.lifestyles.map((x) => ({ id: x.key, label: cap(x.label) }))}
                onChange={(v) => void queueAction("lifestyle", { value: v })}
              />
            )}
          </div>
        </Section>
      </aside>
    </div>
  );
}

// ---- money ---------------------------------------------------------------------------------------------

function MoneyTab({ l }: { l: LifeResp }) {
  const [pct, setPct] = useState(l.giving?.pct ?? 0);
  const [community, setCommunity] = useState(l.giving?.community ?? 0);
  const [amount, setAmount] = useState("");
  const [risk, setRisk] = useState(8);
  const m = l.money;
  const givingChanged = pct !== (l.giving?.pct ?? 0) || community !== (l.giving?.community ?? 0);
  return (
    <div className="split">
      <div className="stack">
        <Section title="Where you stand">
          <div className="card">
            <KeyVal
              rows={[
                { k: "Savings", v: <Money v={m.savings} exact /> },
                { k: "Invested", v: <Money v={m.invested} exact /> },
                { k: "Debt", v: m.debt > 0 ? <span className="tone-neg"><Money v={m.debt} exact /></span> : <span className="faint">None</span> },
                { k: "Income, per year", v: <Money v={m.income} exact /> },
                { k: "Spending, per year", v: <Money v={m.spending} exact /> },
                { k: "Sent to family, per year", v: <Money v={m.family_support} exact /> },
                { k: "Lifestyle", v: m.lifestyle },
              ]}
            />
          </div>
        </Section>
        <Section title="Invest some savings">
          <div className="card form">
            <p className="muted">Money put to work grows or shrinks at the pace of the risk you take. It can be lost.</p>
            <div className="row-inline">
              <input type="number" min={0} step={1000} value={amount} placeholder="Amount" aria-label="Amount to invest" onChange={(e) => setAmount(e.target.value)} />
              <select value={risk} onChange={(e) => setRisk(Number(e.target.value))} aria-label="Risk">
                <option value={3}>Cautious</option>
                <option value={8}>Balanced</option>
                <option value={14}>Bold</option>
                <option value={20}>Reckless</option>
              </select>
              <Button variant="primary" disabled={!(Number(amount) > 0)} onClick={async () => { if (await queueAction("invest", { amount: Math.round(Number(amount)), risk })) setAmount(""); }}>Invest</Button>
            </div>
          </div>
        </Section>
      </div>
      <aside className="stack">
        <Section title="Giving back">
          <div className="card form">
            <Field label={`Share of income: ${pct}%`}>
              <input type="range" min={0} max={60} value={pct} onChange={(e) => setPct(Number(e.target.value))} />
            </Field>
            <Field label={`Community work: ${community} hours a month`}>
              <input type="range" min={0} max={40} value={community} onChange={(e) => setCommunity(Number(e.target.value))} />
            </Field>
            <div className="formfoot">
              <Button variant="primary" disabled={!givingChanged} onClick={() => queueAction("giving", { pct, community })}>Save</Button>
              {l.giving && !l.giving.foundation && (
                <ConfirmAction label="Start a foundation" title="Start a foundation?" action="foundation">
                  <p>A foundation gives your giving a name and a base. It changes how people see you over time; it does not change what you earn.</p>
                </ConfirmAction>
              )}
              {l.giving?.foundation && <Badge tone="pos">You run a foundation</Badge>}
            </div>
          </div>
        </Section>
      </aside>
    </div>
  );
}

// ---- home and family ------------------------------------------------------------------------------

const QUALITY = ["", "Basic", "Modest", "Comfortable", "Fine", "Luxurious"];

function Home({ l }: { l: LifeResp }) {
  const [buy, setBuy] = useState(false);
  const [quality, setQuality] = useState(3);
  const p = l.partner;
  return (
    <div className="split">
      <div className="stack">
        <Section title="Home">
          <div className="card form">
            <KeyVal
              rows={[
                { k: "Where", v: <EntityLink r={l.home.nation}>{l.home.nation.name}</EntityLink> },
                { k: "You live in", v: l.home.kind ? `${l.home.kind} home${l.home.quality ? `, ${QUALITY[l.home.quality]?.toLowerCase()}` : ""}` : "Not known" },
                { k: "Since", v: <Dt d={l.home.since} /> },
              ]}
            />
            <div className="row-inline">
              <Segmented label="Rent or buy" value={buy ? "buy" : "rent"} options={[{ id: "rent", label: "Rent" }, { id: "buy", label: "Buy" }]} onChange={(v) => setBuy(v === "buy")} />
              <select value={quality} onChange={(e) => setQuality(Number(e.target.value))} aria-label="Quality">
                {[1, 2, 3, 4, 5].map((q) => <option key={q} value={q}>{QUALITY[q]}</option>)}
              </select>
              <Button onClick={() => queueAction("move_home", { buy, quality })}>Move</Button>
            </div>
          </div>
        </Section>
        <Section title="Languages">
          <div className="card list-card">
            <ul className="rows">
              {l.languages.map((x) => (
                <li key={x.nation.id}><EntityLink r={x.nation}>{x.nation.name}</EntityLink><Meter value={bandFill(x.level)} label={x.level.label} /></li>
              ))}
            </ul>
          </div>
        </Section>
      </div>
      <aside className="stack">
        <Section title="Partner">
          <div className="card form">
            {p ? (
              <>
                <KeyVal
                  rows={[
                    { k: "Partner", v: <EntityLink r={p.who}>{p.who.name}</EntityLink> },
                    { k: "You are", v: p.status },
                    { k: "Together since", v: <Dt d={p.since} /> },
                    { k: "Bond", v: <Meter value={bandFill(p.bond)} label={p.bond.label} /> },
                    { k: "Works as", v: p.occupation },
                    ...(p.lives ? [{ k: "Lives in", v: p.lives }] : []),
                  ]}
                />
                <div className="formfoot">
                  {p.status !== "living together" && p.status !== "married" && (
                    <ConfirmAction label="Ask to move in" title={`Ask ${p.who.name} to move in?`} action="partner" args={{ ask: "movein" }}>
                      <p>They will answer you. A yes changes your home life and your week.</p>
                    </ConfirmAction>
                  )}
                  {p.status !== "married" && (
                    <ConfirmAction label="Ask to marry" title={`Ask ${p.who.name} to marry you?`} action="partner" args={{ ask: "marry" }}>
                      <p>They will answer you. This is a big question.</p>
                    </ConfirmAction>
                  )}
                  <ConfirmAction label="Say you should separate" title={`Tell ${p.who.name} you should separate?`} action="partner" args={{ ask: "separate" }} danger>
                    <p>They will hear it and answer. It affects both of your lives.</p>
                  </ConfirmAction>
                </div>
              </>
            ) : (
              <>
                <p className="muted">You are not seeing anyone.</p>
                <Segmented
                  label="Looking"
                  value={l.open_to_dating === true ? "open" : "closed"}
                  options={[{ id: "open", label: "Open to meeting someone" }, { id: "closed", label: "Not looking" }]}
                  onChange={(v) => void queueAction("dating", { open: v === "open" })}
                />
              </>
            )}
          </div>
        </Section>
        <Section title="Family">
          <div className="card">
            <KeyVal
              rows={[
                { k: "Children", v: l.children },
                { k: "Siblings", v: l.siblings },
                { k: "Parents living", v: `${l.parents.alive} of 2` },
                { k: "Closeness to them", v: l.parents.closeness },
                { k: "Their health", v: l.parents.health },
              ]}
            />
          </div>
        </Section>
      </aside>
    </div>
  );
}

// ---- study and work ------------------------------------------------------------------------------------------

function Work({ l, s }: { l: LifeResp; s: SelfResp }) {
  const opts = useOptions();
  const playing = s.career?.status === "Active";
  const done = useMemo(() => new Set(l.qualifications.map((q) => q.label)), [l.qualifications]);
  return (
    <div className="split">
      <div className="stack">
        <Section title="Study">
          <div className="card list-card">
            {l.studying && (
              <div className="pad">
                <strong>Studying: {l.studying.course}</strong>
                <div className="hint">{l.studying.done} of {l.studying.effort} units done</div>
              </div>
            )}
            <StudyList opts={opts.data} done={done} busy={!!l.studying} />
          </div>
        </Section>
        {l.qualifications.length > 0 && (
          <Section title="Qualifications">
            <div className="card list-card">
              <ul className="rows">{l.qualifications.map((q) => <li key={q.label}><span>{q.label}</span><span className="hint"><Dt d={q.date} /></span></li>)}</ul>
            </div>
          </Section>
        )}
      </div>
      <aside className="stack">
        <Section title="Work">
          <div className="card form">
            <KeyVal rows={[{ k: "Occupation", v: l.occupation }, ...(l.work ? [{ k: "Working in", v: l.work.path }, { k: "Standing", v: l.work.standing }, { k: "Income, per year", v: <Money v={l.work.income} exact /> }] : [])]} />
            {l.work ? (
              <ConfirmAction label="Leave this work" title="Leave your work?" action="leave_career" danger><p>You stop working in it. Your standing there is not kept.</p></ConfirmAction>
            ) : (
              <>
                {playing && <p className="hint">A second career starts once you no longer play for a club.</p>}
                <div className="chips">
                  {opts.data?.careers.map((c) => (
                    <button key={c.key} className="chip" disabled={playing} onClick={() => void queueAction("career", { path: c.key })}>Start in {c.label}</button>
                  ))}
                </div>
              </>
            )}
          </div>
        </Section>
        <Section title="Help around you">
          <HelpersList l={l} opts={opts.data} />
        </Section>
      </aside>
    </div>
  );
}

function StudyList({ opts, done, busy }: { opts: Options | undefined; done: Set<string>; busy: boolean }) {
  if (!opts) return null;
  return (
    <ul className="rows">
      {opts.courses.map((c) => {
        const locked = !!c.requires && !done.has(c.requires);
        return (
          <li key={c.key}>
            <div>
              <div>{c.label}</div>
              <div className="hint">{c.effort} units · <Money v={c.cost} exact />{c.requires ? ` · needs the ${c.requires}` : ""}</div>
            </div>
            {c.done ? <Badge tone="pos">Done</Badge> : c.studying ? <Badge tone="info">Studying</Badge> : <Button size="sm" disabled={busy || locked} title={locked ? `Needs the ${c.requires}` : undefined} onClick={() => void queueAction("enrol", { course: c.key })}>Enrol</Button>}
          </li>
        );
      })}
    </ul>
  );
}

function HelpersList({ l, opts }: { l: LifeResp; opts: Options | undefined }) {
  if (!opts) return null;
  return (
    <div className="card list-card">
      <ul className="rows">
        {opts.helpers.map((h) => {
          const hired = l.helpers.find((x) => x.label === h.label);
          return (
            <li key={h.key}>
              <div>
                <div>{cap(h.label)}</div>
                <div className="hint">{hired ? <>Costs <Money v={hired.cost} exact /> a year</> : <>From <Money v={h.base_cost} exact /> a year</>}</div>
              </div>
              {hired ? (
                <Button size="sm" variant="ghost" onClick={() => void queueAction("dismiss_helper", { helper: h.key })}>Let go</Button>
              ) : (
                <Button size="sm" onClick={() => void queueAction("helper", { helper: h.key, quality: 10 })}>Hire</Button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

