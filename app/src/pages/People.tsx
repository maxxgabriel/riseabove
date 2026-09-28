import { useState } from "react";
import { EntityFilter, NumberRange, SearchBox, SelectFilter, Chip, useQueryFilters } from "../components/filters";
import { TableView } from "../components/TableView";
import { navigate } from "../router";
import { useStatus } from "../store";
import { POSITIONS, POS_GROUPS, useNationOptions } from "../lookups";
import { Button } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

const SPEC = {
  q: "str", group: "str", pos: "str", nation: "num", club: "num", age_min: "num", age_max: "num", status: "str", injured: "bool", loaned: "bool", kind: "str", min_ca: "num", expiring_days: "num",
} as const;

const TEAM_KINDS = [
  { value: "first", label: "First team" },
  { value: "reserve", label: "Reserves" },
  { value: "youth", label: "Youth teams" },
];

export function People() {
  usePageTitle("People");
  const st = useStatus();
  const observer = st.perspective?.mode !== "inhabit";
  const f = useQueryFilters(SPEC);
  const nations = useNationOptions();
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const posValue = f.get("pos") ? `p:${f.get("pos")}` : f.get("group") ? `g:${f.get("group")}` : "";

  const toolbar = (
    <>
      <SearchBox value={String(f.get("q") ?? "")} onChange={(v) => f.set({ q: v || null })} placeholder="Name" label="Search people by name" />
      <select className="input-sm" aria-label="Position" data-active={posValue !== ""} value={posValue} onChange={(e) => {
        const v = e.target.value;
        if (v.startsWith("g:")) f.set({ group: v.slice(2), pos: null });
        else if (v.startsWith("p:")) f.set({ pos: v.slice(2), group: null });
        else f.set({ pos: null, group: null });
      }}>
        <option value="">Any position</option>
        <optgroup label="Areas">
          {POS_GROUPS.map((g) => <option key={g.value} value={`g:${g.value}`}>{g.label}</option>)}
        </optgroup>
        <optgroup label="Specific positions">
          {POSITIONS.map((p) => <option key={p} value={`p:${p}`}>{p}</option>)}
        </optgroup>
      </select>
      <SelectFilter label="Nationality" all="Any nation" value={f.get("nation") as number | undefined} options={nations} onChange={(v) => f.set({ nation: v })} />
      <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />
      <NumberRange label="Age" min={f.get("age_min") as number | undefined} max={f.get("age_max") as number | undefined} onMin={(v) => f.set({ age_min: v })} onMax={(v) => f.set({ age_max: v })} lo={14} hi={50} />
      <SelectFilter label="Squad" all="All squads" value={f.get("kind") as string | undefined} options={TEAM_KINDS} onChange={(v) => f.set({ kind: v })} />
      <Chip on={f.get("status") === "free"} onClick={() => f.set({ status: f.get("status") === "free" ? null : "free" })}>Free agents</Chip>
      <Chip on={f.get("injured") === true} onClick={() => f.set({ injured: f.get("injured") ? null : true })}>Injured</Chip>
      <Chip on={f.get("loaned") === true} onClick={() => f.set({ loaned: f.get("loaned") ? null : true })}>On loan</Chip>
      {observer && <Chip on={f.get("expiring_days") === 180} onClick={() => f.set({ expiring_days: f.get("expiring_days") ? null : 180 })}>Contract ending soon</Chip>}
      {f.active > 0 && <Button size="sm" variant="ghost" onClick={f.clear}>Clear filters</Button>}
    </>
  );

  const compare = selected.size >= 2 && selected.size <= 6;
  return (
    <div className="page fill">
      <PageHead title="People" sub={observer ? "Everyone in the world, with everything the simulation knows." : "Everyone you know of. Ratings are your best estimates."} />
      <TableView
        id="people"
        table="players"
        label="People"
        filters={f.filters}
        toolbar={toolbar}
        urlSort
        noun={["person", "people"]}
        selectable
        selected={selected}
        onSelected={setSelected}
        actions={
          selected.size > 0 ? (
            <>
              <span className="muted">{selected.size} selected</span>
              <Button size="sm" icon="compare" disabled={!compare} title={compare ? undefined : "Choose between two and six people"} onClick={() => navigate(`/compare?ids=${[...selected].join(",")}`)}>Compare</Button>
              <Button size="sm" variant="ghost" onClick={() => setSelected(new Set())}>Clear</Button>
            </>
          ) : undefined
        }
        empty={<div><strong>No one matches.</strong><div className="muted">{f.active > 0 ? "Try removing a filter." : "This world has no players."}</div></div>}
      />
    </div>
  );
}
