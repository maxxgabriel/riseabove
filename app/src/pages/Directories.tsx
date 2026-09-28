import type { ReactNode } from "react";
import { EntityFilter, SearchBox, SelectFilter, useQueryFilters } from "../components/filters";
import { TableView } from "../components/TableView";
import { setQuery, useRoute } from "../router";
import { useStatus } from "../store";
import { useNationOptions } from "../lookups";
import { Button, Segmented, Tabs } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

function Directory({ title, sub, children }: { title: string; sub?: ReactNode; children: ReactNode }) {
  usePageTitle(title);
  return (
    <div className="page fill">
      <PageHead title={title} sub={sub} />
      {children}
    </div>
  );
}

const clear = (n: number, f: () => void) => (n > 0 ? <Button size="sm" variant="ghost" onClick={f}>Clear filters</Button> : null);

// ---- clubs ------------------------------------------------------------------------------------

export function Clubs() {
  const f = useQueryFilters({ q: "str", nation: "num", comp: "num" });
  const nations = useNationOptions();
  return (
    <Directory title="Clubs" sub="Every club in the world and where it stands.">
      <TableView
        id="clubs"
        table="clubs"
        label="Clubs"
        filters={f.filters}
        urlSort
        noun={["club", "clubs"]}
        toolbar={
          <>
            <SearchBox value={String(f.get("q") ?? "")} onChange={(v) => f.set({ q: v || null })} placeholder="Club name" label="Search clubs" />
            <SelectFilter label="Nation" all="Any nation" value={f.get("nation") as number | undefined} options={nations} onChange={(v) => f.set({ nation: v })} />
            <EntityFilter kind="comp" id={f.get("comp") as number | undefined} onChange={(id) => f.set({ comp: id })} label="Competition" placeholder="Competition" />
            {clear(f.active, f.clear)}
          </>
        }
      />
    </Directory>
  );
}

// ---- competitions --------------------------------------------------------------------------------

export function Comps() {
  const f = useQueryFilters({ q: "str", nation: "num", kind: "str" });
  const nations = useNationOptions();
  return (
    <Directory title="Competitions" sub="Leagues and cups, and how far each has got.">
      <TableView
        id="comps"
        table="comps"
        label="Competitions"
        filters={f.filters}
        urlSort
        noun={["competition", "competitions"]}
        toolbar={
          <>
            <SearchBox value={String(f.get("q") ?? "")} onChange={(v) => f.set({ q: v || null })} placeholder="Name" label="Search competitions" />
            <SelectFilter label="Nation" all="Any nation" value={f.get("nation") as number | undefined} options={nations} onChange={(v) => f.set({ nation: v })} />
            <SelectFilter label="Type" all="Any type" value={f.get("kind") as string | undefined} options={[{ value: "league", label: "Leagues" }, { value: "cup", label: "Cups" }, { value: "continental", label: "Continental" }]} onChange={(v) => f.set({ kind: v })} />
            {clear(f.active, f.clear)}
          </>
        }
      />
    </Directory>
  );
}

// ---- nations -------------------------------------------------------------------------------------

export function Nations() {
  const f = useQueryFilters({ q: "str" });
  return (
    <Directory title="Nations">
      <TableView
        id="nations"
        table="nations"
        label="Nations"
        filters={f.filters}
        urlSort
        noun={["nation", "nations"]}
        toolbar={<SearchBox value={String(f.get("q") ?? "")} onChange={(v) => f.set({ q: v || null })} placeholder="Nation" label="Search nations" />}
      />
    </Directory>
  );
}

// ---- fixtures -------------------------------------------------------------------------------------

export function Fixtures() {
  const st = useStatus();
  const route = useRoute();
  const inhabiting = st.perspective?.mode === "inhabit";
  const mode = route.query.get("show") ?? "upcoming";
  const f = useQueryFilters({ comp: "num", club: "num", mine: "bool" });
  const filters: Record<string, unknown> = { ...f.filters };
  if (mode === "upcoming") filters.played = false;
  if (mode === "results") filters.played = true;
  // Results run newest first; upcoming runs soonest first.
  return (
    <Directory title="Fixtures and results">
      <TableView
        key={mode}
        id="fixtures"
        table="fixtures"
        label="Fixtures"
        filters={filters}
        urlSort
        defaultSort={{ key: "date", desc: mode === "results" }}
        noun={["match", "matches"]}
        toolbar={
          <>
            <Segmented
              label="Show"
              value={mode}
              onChange={(v) => setQuery({ show: v === "upcoming" ? null : v })}
              options={[{ id: "upcoming", label: "Upcoming" }, { id: "results", label: "Results" }, { id: "all", label: "All" }]}
            />
            <EntityFilter kind="comp" id={f.get("comp") as number | undefined} onChange={(id) => f.set({ comp: id })} label="Competition" placeholder="Competition" />
            <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />
            {inhabiting && (
              <button type="button" className="chip" aria-pressed={f.get("mine") === true} onClick={() => f.set({ mine: f.get("mine") ? null : true })}>My team</button>
            )}
            {clear(f.active, f.clear)}
          </>
        }
        empty={mode === "upcoming" ? "No matches are scheduled with these filters." : "No matches have been played with these filters."}
      />
    </Directory>
  );
}

// ---- transfers -----------------------------------------------------------------------------------

export function Transfers() {
  const f = useQueryFilters({ type: "str", club: "num", comp: "num" });
  return (
    <Directory title="Transfers" sub="Signings, loans, releases and free transfers, newest first.">
      <TableView
        id="transfers"
        table="transfers"
        label="Transfers"
        filters={f.filters}
        urlSort
        noun={["move", "moves"]}
        toolbar={
          <>
            <SelectFilter
              label="Type"
              all="All moves"
              value={f.get("type") as string | undefined}
              options={[{ value: "transfer", label: "Transfers" }, { value: "loan", label: "Loans" }, { value: "free", label: "Free transfers" }, { value: "release", label: "Releases" }, { value: "renewal", label: "Renewals" }]}
              onChange={(v) => f.set({ type: v })}
            />
            <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />
            <EntityFilter kind="comp" id={f.get("comp") as number | undefined} onChange={(id) => f.set({ comp: id })} label="Competition" placeholder="Competition" />
            {clear(f.active, f.clear)}
          </>
        }
      />
    </Directory>
  );
}

// ---- events ---------------------------------------------------------------------------------------

export function Events() {
  const f = useQueryFilters({ group: "str", club: "num", comp: "num" });
  return (
    <Directory title="Events" sub="Everything that has happened that you could know about.">
      <TableView
        id="events"
        table="events"
        label="Events"
        filters={f.filters}
        urlSort
        noun={["event", "events"]}
        toolbar={
          <>
            <SelectFilter
              label="Kind"
              all="All events"
              value={f.get("group") as string | undefined}
              options={[{ value: "transfers", label: "Transfers" }, { value: "career", label: "Careers" }, { value: "health", label: "Injuries" }, { value: "club", label: "Clubs" }, { value: "competition", label: "Competitions" }]}
              onChange={(v) => f.set({ group: v })}
            />
            <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />
            <EntityFilter kind="comp" id={f.get("comp") as number | undefined} onChange={(id) => f.set({ comp: id })} label="Competition" placeholder="Competition" />
            {clear(f.active, f.clear)}
          </>
        }
      />
    </Directory>
  );
}

// ---- history --------------------------------------------------------------------------------------

export function History() {
  const route = useRoute();
  const tab = route.query.get("tab") ?? "honours";
  const f = useQueryFilters({ comp: "num", club: "num" });
  return (
    <Directory title="History" sub="Past winners and individual awards.">
      <Tabs
        label="History sections"
        value={tab}
        onChange={(t) => setQuery({ tab: t === "honours" ? null : t })}
        tabs={[{ id: "honours", label: "Winners" }, { id: "awards", label: "Awards" }]}
      />
      <TableView
        key={tab}
        id={`history-${tab}`}
        table={tab === "awards" ? "awards" : "honours"}
        label={tab === "awards" ? "Awards" : "Winners"}
        filters={f.filters}
        urlSort
        noun={tab === "awards" ? ["award", "awards"] : ["title", "titles"]}
        empty="Nothing has been decided yet. Seasons must finish before honours are recorded."
        toolbar={
          <>
            <EntityFilter kind="comp" id={f.get("comp") as number | undefined} onChange={(id) => f.set({ comp: id })} label="Competition" placeholder="Competition" />
            {tab === "honours" && <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />}
            {clear(f.active, f.clear)}
          </>
        }
      />
    </Directory>
  );
}

// ---- staff ----------------------------------------------------------------------------------------

const ROLES = [
  ["manager", "Managers"], ["assistant", "Assistant managers"], ["coach", "Coaches"], ["gk_coach", "Goalkeeping coaches"], ["fitness", "Fitness coaches"],
  ["scout", "Scouts"], ["physio", "Physios"], ["science", "Sports scientists"], ["youth", "Heads of youth"], ["director", "Directors of football"],
];

export function Staff() {
  const f = useQueryFilters({ q: "str", role: "str", club: "num" });
  return (
    <Directory title="Staff" sub="Managers, coaches and specialists.">
      <TableView
        id="staff"
        table="staff"
        label="Staff"
        filters={f.filters}
        urlSort
        noun={["person", "people"]}
        toolbar={
          <>
            <SearchBox value={String(f.get("q") ?? "")} onChange={(v) => f.set({ q: v || null })} placeholder="Name" label="Search staff" />
            <SelectFilter label="Role" all="Any role" value={f.get("role") as string | undefined} options={ROLES.map(([value, label]) => ({ value, label }))} onChange={(v) => f.set({ role: v })} />
            <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />
            {clear(f.active, f.clear)}
          </>
        }
      />
    </Directory>
  );
}
