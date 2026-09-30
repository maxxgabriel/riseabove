import { cap } from "../format";
import { TalkButton } from "../components/Actions";
import { Dt, EntityLink } from "../components/links";
import { navigate, useRoute } from "../router";
import { useApi } from "../store";
import type { Named } from "../types";
import { Badge, Empty, Progress, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface Person {
  who: Named;
  role: string;
  label: string;
  tone: "pos" | "warn" | "neg";
  evidence: { text: string; date: number }[];
  trust: string;
  respect: string;
  since: number;
  last: number;
  why: { text: string; date: number } | null;
}
interface Promise_ {
  id: number;
  mine: boolean;
  with: Named;
  text: string;
  made: number;
  due: number;
  days_left: number;
  state: "open" | "kept" | "broken" | "void";
  progress: { actual: number; promised: number } | null;
}
interface Rumour {
  kind: string;
  date: number;
  via: string;
  sureness: string;
  text: string;
  club?: Named;
}

type TabId = "people" | "promises" | "heard";

export function Relationships() {
  usePageTitle("People around you");
  const route = useRoute();
  const tab = (route.query.get("tab") as TabId) || "people";
  return (
    <div className="page">
      <PageHead title="People around you" sub="Who you know, what they think of you and why, what has been promised, and what you have heard." />
      <Tabs
        label="People"
        value={tab}
        onChange={(t) => navigate(t === "people" ? "/relationships" : `/relationships?tab=${t}`, { replace: true })}
        tabs={[
          { id: "people", label: "People" },
          { id: "promises", label: "Promises" },
          { id: "heard", label: "What you have heard" },
        ]}
      />
      <div className="tabbody">
        {tab === "people" && <People />}
        {tab === "promises" && <Promises />}
        {tab === "heard" && <Heard />}
      </div>
    </div>
  );
}

function People() {
  const q = useApi<{ people: Person[] }>("me.people");
  return (
    <Async q={q}>
      {(d) =>
        d.people.length === 0 ? (
          <Empty title="You do not know anyone yet" icon="people">People come into your life as you play, train and live.</Empty>
        ) : (
          <div className="card list-card">
            <ul className="rows people-rows">
              {d.people.map((p) => (
                <li key={p.who.id}>
                  <div className="grow">
                    <div><strong><EntityLink r={p.who}>{p.who.name}</EntityLink></strong> {p.role && <span className="muted">· {p.role}</span>}</div>
                    <div className="hint">
                      {p.why ? <>{p.why.text} (<Dt d={p.why.date} year={false} />). </> : null}
                      Trust: {p.trust}. Respect: {p.respect}. Last spoke <Dt d={p.last} year={false} />.
                    </div>
                  </div>
                  <Badge tone={p.tone}>{p.label}</Badge>
                  <TalkButton who={p.who} />
                </li>
              ))}
            </ul>
          </div>
        )
      }
    </Async>
  );
}

function PromiseRow({ p }: { p: Promise_ }) {
  return (
    <li>
      <div className="grow">
        <div>{p.text}</div>
        <div className="hint">
          {p.mine ? "You promised" : "Promised to you by"} <EntityLink r={p.with}>{p.with.name}</EntityLink> on <Dt d={p.made} year={false} />
          {p.state === "open" && <> · due <Dt d={p.due} year={false} /> ({p.days_left >= 0 ? `${p.days_left} days left` : "overdue"})</>}
        </div>
        {p.progress && p.state === "open" && (
          <div className="promise-progress">
            <Progress value={Math.round(p.progress.actual * 100)} max={Math.max(1, Math.round(p.progress.promised * 100))} label="Progress" />
            <span className="hint">{Math.round(p.progress.actual * 100)}% of minutes so far, {Math.round(p.progress.promised * 100)}% promised</span>
          </div>
        )}
      </div>
      <Badge tone={p.state === "open" ? "info" : p.state === "kept" ? "pos" : p.state === "broken" ? "neg" : "muted"}>{p.state === "void" ? "No longer applies" : cap(p.state)}</Badge>
    </li>
  );
}

function Promises() {
  const q = useApi<{ promises: Promise_[] }>("me.promises");
  return (
    <Async q={q}>
      {(d) => {
        const to = d.promises.filter((p) => !p.mine);
        const by = d.promises.filter((p) => p.mine);
        if (d.promises.length === 0) return <Empty title="Nothing has been promised" icon="contract">Promises are made in conversations, by you or to you. They are kept or broken, and both are remembered.</Empty>;
        return (
          <div className="split">
            <div className="stack">
              <h2>Promised to you</h2>
              <div className="card list-card">{to.length ? <ul className="rows">{to.map((p) => <PromiseRow key={p.id} p={p} />)}</ul> : <div className="muted pad">Nobody has promised you anything.</div>}</div>
            </div>
            <div className="stack">
              <h2>Promised by you</h2>
              <div className="card list-card">{by.length ? <ul className="rows">{by.map((p) => <PromiseRow key={p.id} p={p} />)}</ul> : <div className="muted pad">You have not promised anyone anything.</div>}</div>
            </div>
          </div>
        );
      }}
    </Async>
  );
}

function Heard() {
  const q = useApi<{ rumours: Rumour[] }>("me.rumours");
  return (
    <Async q={q}>
      {(d) =>
        d.rumours.length === 0 ? (
          <Empty title="Nothing has reached you" icon="mail">Word about you and your future gets to you through your agent, the press and other people. It is not always right.</Empty>
        ) : (
          <div className="card list-card">
            <ul className="rows">
              {d.rumours.map((r, i) => (
                <li key={i}>
                  <div className="grow">
                    <div>{r.club ? <><EntityLink r={r.club}>{r.club.name}</EntityLink>{r.text.slice(r.club.name.length)}</> : r.text}</div>
                    <div className="hint">From {r.via} · <Dt d={r.date} year={false} /> · you are {r.sureness}</div>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        )
      }
    </Async>
  );
}
