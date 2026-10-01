import { useState } from "react";
import { Dt, EntityLink, Parts } from "../components/links";
import { storyPath } from "../components/Newsroom";
import type { ChronicleEntry, ChronicleView } from "../contract.generated";
import { monthName } from "../format";
import { href } from "../router";
import { useApi } from "../store";
import { Empty, Section, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

const CATS: { id: string; label: string }[] = [
  { id: "all", label: "Everything" },
  { id: "football", label: "Football" },
  { id: "moves", label: "Moves" },
  { id: "honours", label: "Honours" },
  { id: "international", label: "Selection" },
  { id: "recognition", label: "Recognition" },
  { id: "people", label: "People" },
  { id: "club", label: "Club" },
  { id: "injury", label: "Injuries" },
  { id: "life", label: "Life" },
];

const KEEPSAKE: Record<string, string> = {
  contract: "Contract",
  scholarship: "Scholarship letter",
  trial: "Trial invitation",
  call_up: "Call-up",
  cap: "First cap",
  medal: "Medal",
  clipping: "Press clipping",
  team_sheet: "Team sheet",
  certificate: "Certificate",
  transfer: "Transfer",
};

const REACH: Record<string, string> = { local: "Local", national: "National", abroad: "Abroad" };

/** Calendar month of a day number (days since 1970). */
function ym(d: number): [number, number] {
  const t = new Date(d * 86400000);
  return [t.getUTCFullYear(), t.getUTCMonth()];
}

function Line({ e }: { e: ChronicleEntry }) {
  return (
    <li className={`chron-line c-${e.cat}`}>
      <span className="chron-day num"><Dt d={e.date} year={false} /></span>
      <span className="chron-text">
        <Parts parts={e.parts} />
        {e.uid != null && <> · <a className="elink" href={href(`/match/${e.uid}`)}>match</a></>}
        {e.story != null && <> · <a className="elink" href={href(storyPath(e.story))}>read</a></>}
        {e.learned != null && <span className="hint block">You learned this on <Dt d={e.learned} /></span>}
      </span>
    </li>
  );
}

function Timeline({ entries }: { entries: ChronicleEntry[] }) {
  const [cat, setCat] = useState("all");
  const shown = entries.filter((e) => cat === "all" || e.cat === cat);
  // Newest year first; within a year, the months in order, as a life is remembered.
  const years = new Map<number, Map<number, ChronicleEntry[]>>();
  for (const e of shown) {
    const [y, m] = ym(e.date);
    const months = years.get(y) ?? new Map<number, ChronicleEntry[]>();
    months.set(m, [...(months.get(m) ?? []), e]);
    years.set(y, months);
  }
  const counts = new Map<string, number>();
  for (const e of entries) counts.set(e.cat, (counts.get(e.cat) ?? 0) + 1);
  return (
    <Section title="Timeline" aside={<span>{shown.length} of {entries.length}</span>}>
      <div className="chips" role="group" aria-label="Show">
        {CATS.filter((c) => c.id === "all" || counts.has(c.id)).map((c) => (
          <button key={c.id} type="button" className="chip" aria-pressed={cat === c.id} onClick={() => setCat(c.id)}>
            {c.label}
            {c.id !== "all" && <span className="faint">{counts.get(c.id)}</span>}
          </button>
        ))}
      </div>
      {shown.length === 0 ? (
        <p className="muted">Nothing of this kind yet.</p>
      ) : (
        [...years.entries()].sort((a, b) => b[0] - a[0]).map(([y, months]) => (
          <div key={y} className="chron-year">
            <h3 className="chron-year-head">{y}</h3>
            {[...months.entries()].sort((a, b) => a[0] - b[0]).map(([m, list]) => (
              <div key={m} className="chron-month">
                <div className="chron-month-head">{monthName(m, true)}</div>
                <ul className="chron-lines">{list.map((e, i) => <Line key={i} e={e} />)}</ul>
              </div>
            ))}
          </div>
        ))
      )}
    </Section>
  );
}

function Scrapbook({ entries }: { entries: ChronicleEntry[] }) {
  const kept = entries.filter((e) => e.keepsake).slice().reverse();
  if (kept.length === 0) return <Empty title="Nothing kept yet" icon="bookmark">Contracts, letters, call-ups, medals and clippings are kept here as your career leaves them.</Empty>;
  return (
    <div className="scrapbook">
      {kept.map((e, i) => (
        <article key={i} className={`keepsake k-${e.keepsake}`}>
          <header>
            <span className="keepsake-kind">{KEEPSAKE[e.keepsake ?? ""] ?? e.keepsake}</span>
            <span className="hint num"><Dt d={e.date} /></span>
          </header>
          <p><Parts parts={e.parts} /></p>
          {e.story != null && <a className="hint" href={href(storyPath(e.story))}>Read the piece</a>}
          {e.uid != null && <a className="hint" href={href(`/match/${e.uid}`)}>The match</a>}
        </article>
      ))}
    </div>
  );
}

function People({ d }: { d: ChronicleView }) {
  if (d.people.length === 0) return <Empty title="Nobody yet" icon="people">Teammates, coaches and the people who first noticed you appear here once your paths have crossed for a while.</Empty>;
  return (
    <Section title="People from your past" aside={<span>{d.people.length}</span>}>
      <table className="minitable">
        <thead>
          <tr><th>Who</th><th>How you know them</th><th>Together</th><th>Now</th></tr>
        </thead>
        <tbody>
          {d.people.map((p) => (
            <tr key={p.who.id}>
              <td><EntityLink r={p.who}>{p.who.name}</EntityLink></td>
              <td className="wrap"><Parts parts={p.how} /></td>
              <td className="num">{p.from === p.to ? <Dt d={p.from} /> : <><Dt d={p.from} year={false} /> to <Dt d={p.to} /></>}</td>
              <td className="wrap muted">{p.now ?? "Not known"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Section>
  );
}

function Reach({ d }: { d: ChronicleView }) {
  return (
    <Section title="How far your name has travelled">
      <ul className="rows compact">
        {d.reach.map((r) => (
          <li key={r.layer}>
            <div className="grow">
              <div>{REACH[r.layer] ?? r.layer}</div>
              <div className="hint">{r.first != null ? <>First: {r.outlet}, <Dt d={r.first} /></> : "Nobody has written about you yet"}</div>
            </div>
            <span className="num">{r.stories}</span>
          </li>
        ))}
      </ul>
      <p className="hint">Pieces about you that the press still has on file, by how far the outlet reaches from home.</p>
    </Section>
  );
}

export function Chronicle() {
  usePageTitle("Your story");
  const q = useApi<ChronicleView>("me.chronicle");
  const [tab, setTab] = useState<"timeline" | "scrapbook" | "people">("timeline");
  return (
    <div className="page">
      <Async q={q}>
        {(d) => (
          <>
            <PageHead title="Your story" sub={<>The life of <EntityLink r={d.person}>{d.person.name}</EntityLink> as it happened, kept since <Dt d={d.since} />. Every name opens what it is about.</>} />
            <Tabs
              label="Your story"
              value={tab}
              onChange={setTab}
              tabs={[
                { id: "timeline", label: "Timeline", count: d.entries.length },
                { id: "scrapbook", label: "Scrapbook", count: d.entries.filter((e) => e.keepsake).length },
                { id: "people", label: "People from your past", count: d.people.length },
              ]}
            />
            {tab === "timeline" && (
              <div className="split">
                <Timeline entries={d.entries} />
                <aside className="stack"><Reach d={d} /></aside>
              </div>
            )}
            {tab === "scrapbook" && <Scrapbook entries={d.entries} />}
            {tab === "people" && <People d={d} />}
          </>
        )}
      </Async>
    </div>
  );
}
