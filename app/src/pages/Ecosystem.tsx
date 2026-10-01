import { useState } from "react";
import type { EligibilityRow, ExportView, LabelRow, PathwayView, RegionOutputView, ScenarioView } from "../contract.generated";
import { date as fmtDate, fmtInt, money } from "../format";
import { EntityLink } from "../components/links";
import { useApi } from "../store";
import { Badge, Empty, KeyVal, Section, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

type Tab = "regions" | "abroad" | "scenario";

/** Where the country's football comes from, how the world abroad sees it, and what the scenario is made of. */
export function Ecosystem() {
  usePageTitle("Development");
  const [tab, setTab] = useState<Tab>("regions");
  return (
    <div className="page">
      <PageHead title="Development" sub="Where players come from, who notices them, and how the world abroad sees the country's football." />
      <Tabs
        label="Development sections"
        value={tab}
        onChange={setTab}
        tabs={[
          { id: "regions", label: "Regions" },
          { id: "abroad", label: "Abroad" },
          { id: "scenario", label: "Scenario" },
        ]}
      />
      {tab === "regions" && <Regions />}
      {tab === "abroad" && <Abroad />}
      {tab === "scenario" && <Scenario />}
    </div>
  );
}

function Regions() {
  const q = useApi<RegionOutputView>("ecosystem.regions");
  return (
    <Async q={q}>
      {(d) =>
        !d.available || d.rows.length === 0 ? (
          <Empty title="Nobody has come through yet">{d.available ? "Regions appear here once players from them have made ten senior appearances." : d.note}</Empty>
        ) : (
          <Section title="What each region has produced" aside={d.note}>
            <div className="card">
              <table className="minitable">
                <thead>
                  <tr>
                    <th>Region</th>
                    <th>Kind</th>
                    <th className="r">Professionals</th>
                    <th className="r" title="Now at the top of the national pyramid">Top tier</th>
                    <th className="r">Internationals</th>
                    <th className="r">Senior apps</th>
                    <th className="r">Market value</th>
                  </tr>
                </thead>
                <tbody>
                  {d.rows.map((r) => (
                    <tr key={`${r.kind}-${r.region}`}>
                      <td>{r.region}</td>
                      <td className="muted">{r.kind}</td>
                      <td className="r num">{fmtInt(r.professionals)}</td>
                      <td className="r num">{fmtInt(r.top_tier)}</td>
                      <td className="r num">{fmtInt(r.internationals)}</td>
                      <td className="r num">{fmtInt(r.senior_apps)}</td>
                      <td className="r num">{money(r.value)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Section>
        )
      }
    </Async>
  );
}

function Abroad() {
  const q = useApi<ExportView>("ecosystem.export");
  return (
    <Async q={q}>
      {(d) =>
        !d.available ? (
          <Empty title="No markets">{d.note}</Empty>
        ) : (
          <>
            <p className="muted">{d.note}</p>
            {d.markets.map((m) => (
              <Section key={m.name} title={m.name} aside={m.nations.join(", ")}>
                <div className="card">
                  <table className="minitable">
                    <thead>
                      <tr>
                        <th>Kind of football</th>
                        <th className="r">Regard</th>
                        <th className="r">Signed from here</th>
                        <th className="r">Playing regularly</th>
                        <th className="r">Scout visits</th>
                        <th>Basis</th>
                      </tr>
                    </thead>
                    <tbody>
                      {m.segments.map((s) => (
                        <tr key={s.segment}>
                          <td>{s.segment}</td>
                          <td className="r num">{s.level}</td>
                          <td className="r num">{s.exports}</td>
                          <td className="r num">{s.successes}</td>
                          <td className="r num">{s.visits}</td>
                          <td>
                            <Badge tone={s.provenance === "Earned" ? "pos" : "muted"}>{s.provenance}</Badge>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </Section>
            ))}
          </>
        )
      }
    </Async>
  );
}

function Scenario() {
  const q = useApi<ScenarioView>("ecosystem.scenario");
  return (
    <Async q={q}>
      {(d) => (
        <>
          <p className="muted">{d.note}</p>
          <Section title="Where the data came from">
            <div className="card">
              <KeyVal
                rows={[
                  { k: "Tuning and calendar", v: d.source },
                  { k: "Clubs imported from a verified, sourced record", v: <span className="num">{fmtInt(d.clubs_imported)}</span> },
                  { k: "Clubs named by the scenario (starting values are seeds)", v: <span className="num">{fmtInt(d.clubs_seeded)}</span> },
                  { k: "Clubs generated to fill places", v: <span className="num">{fmtInt(d.clubs_generated)}</span> },
                  ...(d.clubs_unknown > 0 ? [{ k: "Clubs of unknown origin (older save)", v: <span className="num">{fmtInt(d.clubs_unknown)}</span> }] : []),
                ]}
              />
            </div>
          </Section>
          <Section title="Reference data" aside="Researched real-world records, read with the standing each one claims. Only a sourced, verified record makes a club Imported; everything else is a seed.">
            {!d.reference_loaded ? (
              <Empty title="No reference data was read">This world was built without the researched India reference data (an older save, or another builder).</Empty>
            ) : (
              <div className="card">
                <KeyVal
                  rows={[
                    { k: "Files and records read", v: <span className="num">{fmtInt(d.reference_files)} files, {fmtInt(d.reference_records)} records</span> },
                    ...d.reference_by_status.map((s) => ({ k: `Records: ${s.label.toLowerCase()}`, v: <span className="num">{fmtInt(s.records)}</span> })),
                    { k: "Clubs matched to a real club (by id or exact name and state)", v: <span className="num">{fmtInt(d.clubs_matched)}</span> },
                    { k: "Clubs given a real club's place instead of a made-up one", v: <span className="num">{fmtInt(d.clubs_from_reference)}</span> },
                    {
                      k: "Findings while reading",
                      v: <Badge tone={d.reference_findings === 0 ? "pos" : "warn"}>{fmtInt(d.reference_findings)}</Badge>,
                    },
                  ]}
                />
                {d.finding_samples.length > 0 && (
                  <ul className="muted">
                    {d.finding_samples.map((f, i) => (
                      <li key={i}>{f}</li>
                    ))}
                  </ul>
                )}
              </div>
            )}
          </Section>
          <Section title="Known derbies" aside="Names only. A rivalry's strength comes from what happens in this world, never from the real one.">
            {d.derbies.length === 0 ? (
              <Empty title="No derby is named">{d.reference_loaded ? "None of the derbies in the reference data has both its clubs in this world." : "No reference data was read."}</Empty>
            ) : (
              <div className="card">
                <table className="minitable">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Clubs</th>
                      <th>Kind</th>
                      <th>Data origin</th>
                    </tr>
                  </thead>
                  <tbody>
                    {d.derbies.map((r, i) => (
                      <tr key={i}>
                        <td>{r.name}</td>
                        <td>
                          {r.a} v {r.b}
                        </td>
                        <td>{r.kind}</td>
                        <td>
                          <Badge tone={r.origin === "Imported" ? "pos" : "muted"}>{r.origin}</Badge>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Section>
          {d.reference_loaded && (
            <>
              <LabelTable title="Football associations" aside="The body each state's football answers to." rows={d.associations} cols={["Association", "State", "Based in"]} />
              <LabelTable
                title="Press"
                aside={`Real outlets this world's newsrooms are. What they write comes from what happens here. ${fmtInt(d.institutions_real)} schools, hostels and universities in this world are real places too.`}
                rows={d.press}
                cols={["Outlet", "Kind and base", "Languages"]}
              />
              <LabelTable title="Broadcasters" aside="Who holds the rights in the starting season." rows={d.broadcasters} cols={["Broadcaster", "Competitions", "Languages"]} />
              <LabelTable title="Development programmes" aside="Described, not simulated: names and what they are for." rows={d.programmes} cols={["Programme", "Run by, kind and where", "What it is"]} />
              <LabelTable title="Partnerships" rows={d.partnerships} cols={["Parties", "What it covers", "Purpose"]} />
              <LabelTable title="Coaching ladder" rows={d.coaching_ladder} cols={["Licence", "Step and body", "Requirement"]} />
              <LabelTable title="Referees' ladder" rows={d.referee_ladder} cols={["Grade", "Step and body", "Scope"]} />
              <LabelTable title="Representative sides" rows={d.representative_sides} cols={["Side", "Kind and age", "Who may play"]} />
              <LabelTable title="Rules in words" aside="Rules as the reference words them. Numbers in a rule are not applied from here." rows={d.rules} cols={["Topic", "Applies to", "Rule"]} />
              <LabelTable title="Languages" rows={d.languages} cols={["Language", "Football words known", "Example"]} />
            </>
          )}
          <Section title="Calendar">
            <div className="card">
              <KeyVal rows={d.calendar.map((c) => ({ k: c.event.replace(/([A-Z])/g, " $1").trim(), v: c.when }))} />
            </div>
          </Section>
        </>
      )}
    </Async>
  );
}

/** A named list from the reference data: three columns and the record's standing. Nothing is shown when the reference has none. */
function LabelTable({ title, aside, rows, cols }: { title: string; aside?: string; rows: LabelRow[]; cols: [string, string, string] }) {
  if (rows.length === 0) return null;
  // A column the reference leaves empty for every row is not shown.
  const showDetail = rows.some((r) => r.detail.trim() !== "");
  const showNote = rows.some((r) => r.note.trim() !== "");
  return (
    <Section title={title} aside={aside}>
      <div className="card">
        <table className="minitable">
          <thead>
            <tr>
              <th>{cols[0]}</th>
              {showDetail && <th>{cols[1]}</th>}
              {showNote && <th>{cols[2]}</th>}
              <th>Data origin</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={i}>
                <td className="wrap">{r.name}</td>
                {showDetail && <td className="wrap">{r.detail}</td>}
                {showNote && <td className="wrap muted">{r.note}</td>}
                <td>
                  <Badge tone={r.origin === "Imported" ? "pos" : "muted"}>{r.origin}</Badge>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Section>
  );
}

function Eligibility({ rows }: { rows: EligibilityRow[] }) {
  if (rows.length === 0) return <p className="muted">No side has a claim on him under the rules in force.</p>;
  return (
    <div className="stack">
      {rows.map((r, i) => (
        <div key={i} className="card">
          <div className="row between">
            <strong>{r.body}</strong>
            <Badge tone={r.eligible ? "pos" : "muted"}>{r.eligible ? "May play" : "May not play"}</Badge>
          </div>
          <div className="muted">{r.reason}</div>
          <ul className="plain">
            {r.evidence.map((e, j) => (
              <li key={j} className={e.holds ? "" : "faint"}>
                {e.holds ? "✓" : "✗"} {e.rule}
                {e.detail != null ? ` (${e.detail})` : ""}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

/** A player's route with why each step happened, how he came to exist, whom he may play for, and (omniscient view) who knows him. */
export function PathwayPanel({ id, quiet = false }: { id: number; quiet?: boolean }) {
  const q = useApi<PathwayView>("pathway.player", { id });
  return (
    <Async q={q}>
      {(d) =>
        !d.available ? (
          quiet ? null : <Empty title="No pathway to show">{d.reason}</Empty>
        ) : (
          <>
            <Section title="How he got here">
              <div className="card">
                <ol className="plain">
                  {d.steps.map((s, i) => (
                    <li key={i}>
                      <span className="num muted">{fmtDate(s.date)}</span> {s.kind}
                      {s.target ? `, ${s.target}` : ""}
                      {s.why ? <span className="muted"> · {s.why}</span> : <span className="faint"> · reason not recorded</span>}
                    </li>
                  ))}
                </ol>
              </div>
            </Section>
            {d.creation && (
              <Section title="Where he began" aside={<Badge tone="muted">{d.creation.provenance}</Badge>}>
                <div className="card">
                  <KeyVal
                    rows={[
                      { k: "Grew up in", v: d.creation.region },
                      { k: "First played through", v: d.creation.provider },
                      { k: "First institution", v: d.creation.institution ?? "Not recorded" },
                      { k: "Age when he began", v: d.creation.age != null ? String(d.creation.age) : "Not recorded" },
                      { k: "First competitive setting", v: d.creation.first_env },
                      { k: "First taken seriously by", v: d.creation.first_finder ? <EntityLink r={d.creation.first_finder}>{d.creation.first_finder.name}</EntityLink> : "Nobody yet" },
                    ]}
                  />
                </div>
              </Section>
            )}
            <Section title="Who he may play for">
              <Eligibility rows={d.eligibility} />
            </Section>
            {d.recognition && (
              <Section title="Who knows him" aside={<Badge tone="info">Observer only</Badge>}>
                <div className="card">
                  <p>
                    His name carries: <strong>{d.recognition.standing}</strong>.{d.recognition.buzz ? " There is talk about him, which sends people to look but proves nothing." : ""}
                  </p>
                  <KeyVal rows={d.recognition.evidence.map((e) => ({ k: `${e.level} football`, v: `${e.games} games, ${e.proof} evidence` }))} />
                  {d.recognition.vouch && (
                    <p>
                      Recommended by {d.recognition.vouch.from}, who {d.recognition.vouch.basis}. They rate him {d.recognition.vouch.strength} among those they know; their word carries {d.recognition.vouch.credibility} weight.
                    </p>
                  )}
                  {d.recognition.known_by.length === 0 ? (
                    <p className="muted">No organisation holds anything on him.</p>
                  ) : (
                    <ul className="plain">
                      {d.recognition.known_by.map((k, i) => (
                        <li key={i}>
                          {k.org}: {k.looks} {k.looks === 1 ? "look" : "looks"} over {k.years} {k.years === 1 ? "year" : "years"}; {k.how}
                          {k.first_by ? `, first by ${k.first_by}` : ""}
                        </li>
                      ))}
                    </ul>
                  )}
                  {d.recognition.watching.map((w, i) => (
                    <p key={i} className="muted">
                      {w.by} of {w.org} is watching him, {w.games_left} {w.games_left === 1 ? "game" : "games"} to go.
                    </p>
                  ))}
                </div>
              </Section>
            )}
          </>
        )
      }
    </Async>
  );
}
