import { useState } from "react";
import { Dt, EntityLink, Money, Parts } from "../components/links";
import { storyPath } from "../components/Newsroom";
import type { ChronicleEntry, ChronicleView, Named } from "../contract.generated";
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

function Timeline({ entries, born }: { entries: ChronicleEntry[]; born: number }) {
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
            <h3 className="chron-year-head">{y}<span className="hint"> · turned {y - ym(born)[0]} that year</span></h3>
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

/** A keepsake as the paper it was: the terms of a contract, a call-up notice, a medal, a cutting from the papers. What the story did
 * not keep is said so, never shown as zero. */
function Doc({ e, who }: { e: ChronicleEntry; who: Named }) {
  const d = e.doc;
  if (!d) return <p><Parts parts={e.parts} /></p>;
  switch (d.kind) {
    case "contract":
      return (
        <div className="doc doc-contract">
          <div className="doc-title">Contract of employment</div>
          <div className="doc-parties">
            Between {d.club ? <EntityLink r={d.club}>{d.club.name}</EntityLink> : "the club"} and <EntityLink r={who}>{who.name}</EntityLink>
          </div>
          <dl className="doc-terms">
            <div><dt>Weekly wage</dt><dd>{d.wage != null ? <Money v={d.wage} exact /> : <span className="muted">Not kept</span>}</dd></div>
            <div><dt>Length</dt><dd>{d.years != null ? `${d.years} ${d.years === 1 ? "year" : "years"}` : <span className="muted">Not known</span>}</dd></div>
            <div><dt>Until</dt><dd>{d.until != null ? <Dt d={d.until} /> : <span className="muted">Not known</span>}</dd></div>
          </dl>
          <div className="doc-sign"><span>Signed</span><span className="num"><Dt d={e.date} /></span></div>
        </div>
      );
    case "call_up":
      return (
        <div className="doc doc-callup">
          <div className="doc-title">Call-up notice</div>
          <div className="doc-parties">To <EntityLink r={who}>{who.name}</EntityLink></div>
          <p className="doc-body">
            You have been selected for the {d.nation ? <EntityLink r={d.nation}>{d.squad ?? d.nation.name}</EntityLink> : (d.squad ?? "squad")}.
          </p>
          <dl className="doc-terms">
            <div><dt>Report</dt><dd>{d.from != null ? <><Dt d={d.from} /> to <Dt d={d.to} /></> : <span className="muted">Dates not on record</span>}</dd></div>
          </dl>
        </div>
      );
    case "medal":
      return (
        <div className="doc doc-medal">
          <div className="medal-disc" aria-hidden="true"><span>{(d.honour ?? "Medal").split(",")[0]}</span></div>
          <div className="medal-text">
            <div className="doc-title">{d.honour ?? "Medal"}</div>
            <div>{d.comp ? <EntityLink r={d.comp}>{d.comp.name}</EntityLink> : (d.comp_name ?? "A competition")}</div>
            <div className="hint">{d.season ?? "Season not known"}{d.club && <> · <EntityLink r={d.club}>{d.club.name}</EntityLink></>}</div>
          </div>
        </div>
      );
    case "clipping":
      return (
        <div className="doc doc-clipping">
          <div className="clip-masthead">{d.outlet ?? "Unknown paper"}</div>
          <div className="clip-date num"><Dt d={e.date} /></div>
          <div className="clip-headline">{d.headline ?? "Headline not kept"}</div>
        </div>
      );
    default:
      return <p><Parts parts={e.parts} /></p>;
  }
}

function Scrapbook({ entries, who }: { entries: ChronicleEntry[]; who: Named }) {
  const kept = entries.filter((e) => e.keepsake).slice().reverse();
  if (kept.length === 0) return <Empty title="Nothing kept yet" icon="bookmark">Contracts, letters, call-ups, medals and clippings are kept here as your career leaves them.</Empty>;
  return (
    <div className="scrapbook">
      {kept.map((e, i) => (
        <article key={i} className={`keepsake k-${e.keepsake}${e.doc ? " is-doc" : ""}`}>
          <header>
            <span className="keepsake-kind">{KEEPSAKE[e.keepsake ?? ""] ?? e.keepsake}</span>
            <span className="hint num"><Dt d={e.date} /></span>
          </header>
          <Doc e={e} who={who} />
          {e.doc && <p className="hint keepsake-line"><Parts parts={e.parts} /></p>}
          {e.story != null && <a className="hint" href={href(storyPath(e.story))}>Read the piece</a>}
          {e.uid != null && <a className="hint" href={href(`/match/${e.uid}`)}>The match</a>}
        </article>
      ))}
    </div>
  );
}

/** What became of the people from your past: public record only, furthest risen first. */
function Became({ d }: { d: ChronicleView }) {
  if (d.became.length === 0)
    return <Empty title="Too soon to tell" icon="people">A year after your paths first crossed, teammates, coaches and the people who noticed you appear here with what became of them.</Empty>;
  return (
    <Section title="What became of them" aside={<span>{d.became.length}</span>}>
      <ol className="became">
        {d.became.map((b) => (
          <li key={b.who.id} className={`became-row${b.retired ? " is-retired" : ""}`}>
            <div className="became-who">
              <EntityLink r={b.who}>{b.who.name}</EntityLink>
              <span className="hint block"><Parts parts={b.how} />, since <Dt d={b.from} /></span>
            </div>
            <div className="became-now">
              <strong>{b.summary}</strong>
              <span className="block">
                {b.role}
                {b.club && <> at <EntityLink r={b.club}>{b.club.name}</EntityLink></>}
                {b.league && <span className="hint"> · <EntityLink r={b.league}>{b.league.name}</EntityLink>{b.level ? `, ${b.level}` : ""}</span>}
              </span>
            </div>
            <div className="became-facts">
              {b.caps != null && b.caps_for ? (
                <span><span className="num">{b.caps}</span> {b.caps === 1 ? "cap" : "caps"} for <EntityLink r={b.caps_for}>{b.caps_for.name}</EntityLink></span>
              ) : (
                <span className="muted">No senior caps</span>
              )}
              {b.managed > 0 && <span className="hint block">{b.managed} {b.managed === 1 ? "manager's job" : "manager's jobs"}</span>}
            </div>
          </li>
        ))}
      </ol>
      <p className="hint">Ordered by how far they have risen: the level of their club, a manager's job, caps for their country. Public record only.</p>
    </Section>
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
  const [tab, setTab] = useState<"timeline" | "scrapbook" | "people" | "became">("timeline");
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
                { id: "became", label: "What became of them", count: d.became.length },
              ]}
            />
            {tab === "timeline" && (
              <div className="split">
                <Timeline entries={d.entries} born={d.born} />
                <aside className="stack"><Reach d={d} /></aside>
              </div>
            )}
            {tab === "scrapbook" && <Scrapbook entries={d.entries} who={d.person} />}
            {tab === "people" && <People d={d} />}
            {tab === "became" && <Became d={d} />}
          </>
        )}
      </Async>
    </div>
  );
}
