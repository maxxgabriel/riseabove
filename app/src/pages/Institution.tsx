import { Crest } from "../components/Crest";
import { EntityLink } from "../components/links";
import { Stage, StageHeader } from "../components/Stage";
import { brandFor, tintOf } from "../color";
import type { InstitutionPerson, InstitutionView } from "../contract.generated";
import { useRoute } from "../router";
import { useApi } from "../store";
import { Badge, Empty, ErrorState, KeyVal, Section, Skeleton } from "../ui/ui";
import { usePageTitle } from "./common";

function People({ list, empty }: { list: InstitutionPerson[]; empty: string }) {
  if (list.length === 0) return <p className="muted">{empty}</p>;
  // Where they are now is said of former players; a list where nobody has it leaves the column out.
  const now = list.some((p) => p.now != null);
  return (
    <table className="minitable">
      <thead>
        <tr><th>Name</th><th className="num">Age</th>{now && <th>Now</th>}</tr>
      </thead>
      <tbody>
        {list.map((p) => (
          <tr key={p.who.id}>
            <td><EntityLink r={p.who}>{p.who.name}</EntityLink></td>
            <td className="num">{p.age}</td>
            {now && <td className="wrap muted">{p.now ?? "Not known"}</td>}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** A school, sports school or university: where it is, its standing, its teams, its players and what it has won. */
export function Institution() {
  const route = useRoute();
  const id = Number(route.segs[1]);
  const q = useApi<InstitutionView>("institution", { id });
  usePageTitle(q.data?.name);
  const d = q.data;
  const [c1] = brandFor("club", id + 100000, d?.name ?? "");
  return (
    <Stage tint={tintOf(c1)}>
      {q.error && !d ? (
        <div className="stage-body"><ErrorState error={q.error} onRetry={q.reload} /></div>
      ) : !d ? (
        <div className="stage-body" aria-busy="true"><Skeleton w="30%" h={40} /></div>
      ) : (
        <>
          <StageHeader
            crest={<Crest name={d.name} kind="club" id={id + 100000} size={58} />}
            title={d.name}
            sub={<>{d.kind}{d.city ? ` · ${d.city}` : ""} · <EntityLink r={d.nation}>{d.nation.name}</EntityLink></>}
            subIcon="globe"
            meta={[
              { label: "Standing", value: d.standing },
              { label: "Founded", value: d.founded != null ? <span className="num">{d.founded}</span> : <span className="muted">Not known</span> },
              { label: "Players", value: <span className="num">{d.players.length}</span> },
            ]}
          />
          <div className="stage-body">
            {d.yours && <p className="inst-yours"><Badge tone="you">You</Badge> {d.yours}</p>}
            <div className="grid-2">
              <div className="stack">
                <Section title="The place" aside={<Badge tone={d.origin === "Imported" ? "pos" : "muted"}>{d.origin}</Badge>}>
                  <div className="card">
                    <KeyVal
                      rows={[
                        { k: "Kind", v: d.kind },
                        { k: "Where", v: [d.city, d.region, d.state].filter(Boolean).join(", ") || "Not known" },
                        { k: "Standing", v: d.standing },
                        { k: "Football", v: d.football ?? "Not known" },
                        { k: "Facilities", v: d.facilities ?? "Not known" },
                        { k: "Scholarships", v: d.scholarships != null ? `${d.scholarships} a year` : "Not known" },
                      ]}
                    />
                  </div>
                </Section>
                <Section title="Teams this season" aside={<span>{d.teams.length}</span>}>
                  {d.teams.length === 0 ? (
                    <p className="muted">Not playing in a competition this season.</p>
                  ) : (
                    <table className="minitable">
                      <thead>
                        <tr><th>Competition</th><th>Season</th><th>Standing</th><th className="num">P</th><th className="num">Pts</th></tr>
                      </thead>
                      <tbody>
                        {d.teams.map((t, i) => (
                          <tr key={i}>
                            <td className="wrap">{t.comp}</td>
                            <td className="num">{t.season}</td>
                            <td>{t.standing ?? <span className="muted">Not started</span>}</td>
                            <td className="num">{t.played ?? "–"}</td>
                            <td className="num">{t.points ?? "–"}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </Section>
                <Section title="Honours" aside={<span>{d.titles.length}</span>}>
                  {d.titles.length === 0 ? (
                    <p className="muted">Nothing won or reached a final of, on record.</p>
                  ) : (
                    <ul className="rows compact">
                      {d.titles.map((t, i) => (
                        <li key={i}>
                          <span className="grow">{t.comp}</span>
                          <span className="num muted">{t.season}</span>
                          <Badge tone={t.finish === "won" ? "warn" : "muted"}>{t.finish === "won" ? "Winners" : "Runners-up"}</Badge>
                        </li>
                      ))}
                    </ul>
                  )}
                </Section>
              </div>
              <div className="stack">
                <Section title="Players now" aside={<span>{d.players.length}</span>}>
                  <People list={d.players} empty="No players on its books." />
                </Section>
                <Section title="Went on to play professionally" aside={<span>{d.alumni.length}</span>}>
                  {d.alumni.length === 0 ? <Empty title="None yet" icon="people">Former players who turn professional are listed here.</Empty> : <People list={d.alumni} empty="" />}
                </Section>
              </div>
            </div>
          </div>
        </>
      )}
    </Stage>
  );
}
