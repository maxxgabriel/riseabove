import { useMemo } from "react";
import { EntityFilter, SearchBox, SelectFilter, useQueryFilters } from "../components/filters";
import { TableView } from "../components/TableView";
import { fmtInt } from "../format";
import { href, setQuery, useRoute } from "../router";
import { useStatus, useApiMany } from "../store";
import type { TableResp } from "../types";
import { Button, Section, Segmented } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

type Facet = "q" | "club" | "person" | "kind" | "open";

interface Sheet {
  id: string;
  title: string;
  about: string;
  noun: [string, string];
  facets?: Facet[];
  /** What the list is empty of, said the way the world would say it. */
  empty: string;
  /** Only shown when watching the whole world: the list would give away what people hide. */
  observer?: boolean;
}

interface Shelf {
  title: string;
  sheets: Sheet[];
}

const INSTITUTION_KINDS = [
  { value: "university", label: "Universities" },
  { value: "academy", label: "Academies" },
  { value: "school", label: "Schools" },
  { value: "league", label: "Leagues" },
  { value: "federation", label: "Federations" },
];

const SHELVES: Shelf[] = [
  {
    title: "Supporters",
    sheets: [
      { id: "posts", title: "Posts", about: "What people are saying online, and about whom.", noun: ["post", "posts"], facets: ["q", "club", "person"], empty: "Nobody has posted anything that matches." },
      { id: "chants", title: "Chants", about: "The songs each ground sings, and how well known they are.", noun: ["chant", "chants"], facets: ["q", "club"], empty: "No chants have taken hold yet." },
      { id: "memes", title: "In-jokes", about: "Phrases supporters have made their own.", noun: ["in-joke", "in-jokes"], facets: ["q", "club"], empty: "No in-jokes have caught on yet." },
      { id: "groups", title: "Supporter groups", about: "The organised crowds around each club, and where they stand with it.", noun: ["group", "groups"], facets: ["q", "club"], empty: "No supporter groups match." },
      { id: "rivalries", title: "Rivalries", about: "Which clubs cannot stand each other, how deep it runs and how the meetings have gone.", noun: ["rivalry", "rivalries"], facets: ["q", "club"], empty: "No rivalries match." },
    ],
  },
  {
    title: "Press",
    sheets: [
      { id: "conferences", title: "Press conferences", about: "Who faced the press, and how many questions they took.", noun: ["conference", "conferences"], facets: ["club", "person"], empty: "No press conferences match." },
      { id: "quotes", title: "Quotes", about: "What people said about each other, in the papers' words.", noun: ["quote", "quotes"], facets: ["person"], empty: "Nobody has been quoted yet." },
      { id: "outlets", title: "Outlets", about: "Papers, sites and broadcasters, and how far they reach.", noun: ["outlet", "outlets"], facets: ["q"], empty: "No outlets match." },
      { id: "journalists", title: "Journalists", about: "Who writes for whom, and which clubs they follow.", noun: ["journalist", "journalists"], facets: ["q", "club"], empty: "No journalists match." },
      { id: "grapevine", title: "Grapevine", about: "What is going round, who knows it, and whether it is still alive.", noun: ["item", "items"], facets: ["q"], empty: "Nothing is going round.", observer: true },
    ],
  },
  {
    title: "Trouble and officials",
    sheets: [
      { id: "incidents", title: "Incidents", about: "Trouble off the pitch, as far as you would have heard of it.", noun: ["incident", "incidents"], facets: ["club", "person", "open"], empty: "No incidents match." },
      { id: "referees", title: "Referees", about: "Who takes the whistle, and how they have been getting on.", noun: ["referee", "referees"], facets: ["q"], empty: "No referees match." },
      { id: "controversies", title: "Big decisions", about: "The calls that started arguments, and what came of the appeals.", noun: ["decision", "decisions"], facets: ["q", "club"], empty: "No big decisions match." },
      { id: "charges", title: "Charges and fines", about: "What the authorities have punished, and what it cost.", noun: ["charge", "charges"], facets: ["q", "club"], empty: "Nobody has been charged." },
    ],
  },
  {
    title: "Records and honours",
    sheets: [
      { id: "record_book", title: "Record book", about: "Every record that stands and who holds it.", noun: ["record", "records"], facets: ["q"], empty: "No records match." },
      { id: "records_broken", title: "Records broken", about: "When a record fell and how long it had stood.", noun: ["record", "records"], facets: ["q"], empty: "No record has fallen yet." },
      { id: "votes", title: "Award votes", about: "The ballots, who won and how the voters split.", noun: ["ballot", "ballots"], facets: ["q"], empty: "No award has been voted on yet." },
      { id: "hall_members", title: "Halls of fame", about: "Who has been inducted.", noun: ["member", "members"], facets: ["q"], empty: "Nobody has been inducted yet." },
      { id: "chronicle", title: "Chronicle", about: "The big moments the game remembers.", noun: ["entry", "entries"], facets: ["q"], empty: "Nothing has been written into the chronicle yet." },
    ],
  },
  {
    title: "The game itself",
    sheets: [
      { id: "schools", title: "Tactical schools", about: "Ways of playing that have followers, and who founded them.", noun: ["school", "schools"], facets: ["q"], empty: "No tactical school has formed yet." },
      { id: "rule_changes", title: "Rule changes", about: "When the laws moved and where.", noun: ["change", "changes"], facets: ["q"], empty: "The rules have not changed." },
      { id: "institutions", title: "Schools, academies and bodies", about: "The places that shape players and run the game away from the top clubs.", noun: ["institution", "institutions"], facets: ["q", "kind"], empty: "No institutions match." },
      { id: "minor_seasons", title: "Lower football", about: "How the leagues below the professional game finished.", noun: ["season", "seasons"], facets: ["q"], empty: "No lower-league season has finished yet." },
    ],
  },
];

const ALL = SHELVES.flatMap((s) => s.sheets);

function useVisible(): Shelf[] {
  const st = useStatus();
  const observing = st.perspective?.mode !== "inhabit";
  return useMemo(() => SHELVES.map((s) => ({ ...s, sheets: s.sheets.filter((x) => observing || !x.observer) })), [observing]);
}

export function Society() {
  const route = useRoute();
  const id = route.segs[1];
  const sheet = id ? ALL.find((s) => s.id === id) : undefined;
  if (sheet) return <SheetPage sheet={sheet} />;
  return <Index missing={id} />;
}

function Index({ missing }: { missing?: string }) {
  usePageTitle("Society");
  const shelves = useVisible();
  const list = useMemo(() => shelves.flatMap((s) => s.sheets), [shelves]);
  const counts = useApiMany<TableResp>("table.query", list.map((s) => ({ table: s.id, limit: 1 })));
  const total = (i: number) => counts.data?.[i]?.total;
  let at = 0;
  return (
    <div className="page">
      <PageHead title="Society" sub="Everything that grew up around the game: what supporters say, what the press writes, who keeps the peace, and what the record books hold." />
      {missing && <p className="muted">There is no list called “{missing}”.</p>}
      <div className="stack">
        {shelves.map((shelf) => (
          <Section key={shelf.title} title={shelf.title}>
            <div className="hub-grid">
              {shelf.sheets.map((s) => {
                const n = total(at++);
                return (
                  <a key={s.id} className="hub-card" href={href(`/society/${s.id}`)}>
                    <span className="hub-title">{s.title}</span>
                    <span className="hub-about">{s.about}</span>
                    <span className="hub-count">{n == null ? "" : n === 0 ? "None yet" : `${fmtInt(n)} ${n === 1 ? s.noun[0] : s.noun[1]}`}</span>
                  </a>
                );
              })}
            </div>
          </Section>
        ))}
      </div>
    </div>
  );
}

function SheetPage({ sheet }: { sheet: Sheet }) {
  usePageTitle(sheet.title);
  const st = useStatus();
  const f = useQueryFilters({ q: "str", club: "num", person: "num", kind: "str", open: "bool" });
  const facets = sheet.facets ?? [];
  const has = (x: Facet) => facets.includes(x);
  const observing = st.perspective?.mode !== "inhabit";
  const blocked = sheet.observer && !observing;
  return (
    <div className="page fill">
      <PageHead
        title={sheet.title}
        sub={sheet.about}
        crumbs={[{ label: "Society", to: "/society" }, { label: sheet.title }]}
      />
      {blocked ? (
        <p className="muted">This is only visible to someone watching the whole world.</p>
      ) : (
        <TableView
          key={sheet.id}
          id={`society-${sheet.id}`}
          table={sheet.id}
          label={sheet.title}
          filters={f.filters}
          urlSort
          noun={sheet.noun}
          empty={f.active > 0 ? "Nothing matches these filters." : sheet.empty}
          toolbar={
            <>
              {has("q") && <SearchBox value={String(f.get("q") ?? "")} onChange={(v) => f.set({ q: v || null })} placeholder="Search" label={`Search ${sheet.title.toLowerCase()}`} />}
              {has("club") && <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />}
              {has("person") && <EntityFilter kind="person" id={f.get("person") as number | undefined} onChange={(id) => f.set({ person: id })} label="Person" placeholder="Person" />}
              {has("kind") && <SelectFilter label="Kind" all="Any kind" value={f.get("kind") as string | undefined} options={INSTITUTION_KINDS} onChange={(v) => f.set({ kind: v })} />}
              {has("open") && (
                <Segmented
                  label="Status"
                  value={f.get("open") === true ? "open" : f.get("open") === false ? "settled" : "all"}
                  onChange={(v) => setQuery({ open: v === "open" ? "1" : v === "settled" ? "0" : null })}
                  options={[{ id: "all", label: "All" }, { id: "open", label: "Open" }, { id: "settled", label: "Settled" }]}
                />
              )}
              {f.active > 0 && <Button size="sm" variant="ghost" onClick={f.clear}>Clear filters</Button>}
            </>
          }
        />
      )}
    </div>
  );
}
