import { EntityLink, Parts, Dt } from "../components/links";
import { useBookmarks } from "../bookmarks";
import { fmtInt, plural } from "../format";
import { href } from "../router";
import { useApi } from "../store";
import type { Named, Part } from "../types";
import { Empty, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface OverviewResp {
  name: string;
  date: number;
  counts: { players: number; clubs: number; competitions: number; nations: number };
  leagues: { comp: Named; stage: string; leader: null | { team: Named; points: number; played: number } }[];
  upcoming: { uid: number; date: number; comp: string; home: Named; away: Named }[];
  recent: { date: number; kind: string; parts: Part[] }[];
  followed: Named[];
}

export function Overview() {
  usePageTitle("Overview");
  const q = useApi<OverviewResp>("overview");
  const marks = useBookmarks();
  return (
    <div className="page">
      <Async q={q}>
        {(o) => (
          <>
            <PageHead
              title={o.name}
              sub={`${plural(o.counts.clubs, "club")} in ${plural(o.counts.nations, "nation")}, ${fmtInt(o.counts.players)} active players, ${plural(o.counts.competitions, "competition")}.`}
            />
            <div className="split">
              <div className="stack">
                <Section title="Leagues" aside="Top division of each nation">
                  <div className="card list-card">
                    {o.leagues.length === 0 && <Empty title="No leagues yet" />}
                    <ul className="rows">
                      {o.leagues.map((l) => (
                        <li key={l.comp.id}>
                          <div>
                            <EntityLink r={l.comp}>{l.comp.name}</EntityLink>
                            <div className="hint">{l.stage}</div>
                          </div>
                          <div className="rows-right">
                            {l.leader && l.leader.played > 0 ? (
                              <>
                                <EntityLink r={l.leader.team}>{l.leader.team.name}</EntityLink>
                                <div className="hint num">{l.leader.points} points from {l.leader.played}</div>
                              </>
                            ) : (
                              <span className="faint">No matches played yet</span>
                            )}
                          </div>
                        </li>
                      ))}
                    </ul>
                  </div>
                </Section>
                <Section title="This week" aside={<a href={href("/fixtures")}>All fixtures</a>}>
                  <div className="card list-card">
                    {o.upcoming.length === 0 ? (
                      <div className="muted pad">No matches in the next six days.</div>
                    ) : (
                      <ul className="rows">
                        {o.upcoming.map((f) => (
                          <li key={f.uid}>
                            <div className="fx">
                              <span className="fx-date"><Dt d={f.date} year={false} /></span>
                              <EntityLink r={f.home} className="fx-team">{f.home.name}</EntityLink>
                              <a className="fx-v" href={href(`/match/${f.uid}`)}>v</a>
                              <EntityLink r={f.away} className="fx-team">{f.away.name}</EntityLink>
                            </div>
                            <span className="hint">{f.comp}</span>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </Section>
              </div>
              <aside className="stack">
                <Section title="Recent" aside={<a href={href("/events")}>All events</a>}>
                  <div className="card list-card">
                    {o.recent.length === 0 ? (
                      <div className="muted pad">Nothing notable has happened yet. Advance time to see the world move.</div>
                    ) : (
                      <ul className="rows feed">
                        {o.recent.map((e, i) => (
                          <li key={i}>
                            <div>
                              <span className="feed-kind">{e.kind}</span>
                              <div><Parts parts={e.parts} /></div>
                            </div>
                            <span className="hint"><Dt d={e.date} year={false} /></span>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </Section>
                {(o.followed.length > 0 || marks.length > 0) && (
                  <Section title="Yours" aside={<a href={href("/bookmarks")}>All</a>}>
                    <div className="card list-card">
                      <ul className="rows">
                        {o.followed.map((c) => (
                          <li key={`f${c.id}`}><EntityLink r={c}>{c.name}</EntityLink><span className="hint">Followed</span></li>
                        ))}
                        {marks.slice(0, 6).map((b) => (
                          <li key={`${b.k}${b.id}`}><EntityLink r={b}>{b.title}</EntityLink><span className="hint">{b.sub}</span></li>
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
