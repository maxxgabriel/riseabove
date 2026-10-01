import type { RouteOptionsView } from "../contract.generated";
import { useMemo, useState } from "react";
import { navigate } from "../router";
import { act, notify, useApi, useStatus } from "../store";
import { Badge, Button, Empty, Field, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

type Options = RouteOptionsView;

/** What each start means for the first few years, in plain words, so the choice is an informed one. */
const WHAT_NEXT: Record<string, string> = {
  school_standout: "Nobody outside your school knows you. Games, district trials and whoever happens to watch decide whether anyone ever looks at you.",
  released_academy: "You were good enough to be taken in and not good enough to be kept. Universities, state sides and trials are still open; the door back is narrow.",
  university_freshman: "A scholarship buys you time and a team, not a career. Scouts watch university football only in some places, and only some of the time.",
  university_star: "Three years of games are on the record. What you do this year decides whether anyone takes the step of signing you.",
  state_league: "Real football, in front of real selectors. The state side and the national pyramid are within reach, with no guarantee of either.",
  semi_pro: "One step from the leagues people watch. Wages are small, contracts short, and the next club is the only ladder there is.",
};

/** Begin a life somewhere on the route in an India world. Talent is drawn like everyone's and is never shown or chosen. */
export function Begin() {
  usePageTitle("Begin a life");
  const st = useStatus();
  const q = useApi<Options>("route.options");
  return (
    <div className="page narrow">
      <PageHead title="Begin a life" sub="Pick where the story starts. Your talent is drawn like everyone else's, and you will not be told what it is." />
      {st.perspective?.mode === "inhabit" && <p className="muted">You are already {st.perspective.name}. Beginning a new life adds a new person to the world and switches your point of view to them; the world does not restart.</p>}
      <Async q={q}>{(o) => (o.available ? <Chooser o={o} /> : <Empty title="No route in this world">Only an India world has a route to begin on. Create one from the world page.</Empty>)}</Async>
    </div>
  );
}

function Chooser({ o }: { o: Options }) {
  const [start, setStart] = useState(o.starts[0]?.key ?? "");
  const [stateId, setStateId] = useState<number | null>(o.states[0]?.id ?? null);
  const [district, setDistrict] = useState<number | null>(null);
  const [first, setFirst] = useState("");
  const [last, setLast] = useState("");
  const [working, setWorking] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const state = useMemo(() => o.states.find((s) => s.id === stateId) ?? null, [o.states, stateId]);
  const chosen = o.starts.find((s) => s.key === start);
  const districtId = district ?? state?.districts[0]?.id ?? null;

  const go = async () => {
    setErr(null);
    setWorking(true);
    try {
      await act("route.begin", { start, district: districtId, first: first.trim() || undefined, last: last.trim() || undefined });
      navigate("/today");
    } catch (e) {
      setErr((e as Error).message);
      notify({ tone: "neg", text: (e as Error).message });
    } finally {
      setWorking(false);
    }
  };

  return (
    <div className="stack">
      <Section title="Where do you begin?">
        <div className="scale-list" role="radiogroup" aria-label="Where you begin">
          {o.starts.map((s) => (
            <button key={s.key} role="radio" aria-checked={start === s.key} className="scale" onClick={() => setStart(s.key)}>
              <strong>
                {s.label} <Badge tone="muted">age {s.age}</Badge>
              </strong>
              <span>{s.blurb}</span>
            </button>
          ))}
        </div>
        {chosen && WHAT_NEXT[chosen.key] && <p className="muted">{WHAT_NEXT[chosen.key]}</p>}
      </Section>

      <Section title="Where are you from?" aside="The district decides which schools, clubs and scouts are near you">
        <div className="card form">
          <Field label="State">
            <select
              value={stateId ?? ""}
              onChange={(e) => {
                setStateId(Number(e.target.value));
                setDistrict(null);
              }}
            >
              {o.states.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="District" hint={state && districtId != null ? `${(state.districts.find((d) => d.id === districtId)?.population_k ?? 0).toLocaleString()} thousand people. More people usually means more competition to be seen among, not fewer chances.` : undefined}>
            <select value={districtId ?? ""} onChange={(e) => setDistrict(Number(e.target.value))}>
              {(state?.districts ?? []).map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </select>
          </Field>
        </div>
      </Section>

      <Section title="Your name" aside="Optional. Left empty, the world gives you one from your district's language">
        <div className="card form">
          <Field label="First name">
            <input value={first} onChange={(e) => setFirst(e.target.value)} maxLength={40} autoComplete="off" />
          </Field>
          <Field label="Last name">
            <input value={last} onChange={(e) => setLast(e.target.value)} maxLength={40} autoComplete="off" />
          </Field>
        </div>
      </Section>

      {err && (
        <p role="alert" className="neg">
          {err}
        </p>
      )}
      <div className="start-actions">
        <Button variant="primary" disabled={working || !start || districtId == null} onClick={go}>
          Begin
        </Button>
      </div>
    </div>
  );
}
