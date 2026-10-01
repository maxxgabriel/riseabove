import { useState } from "react";
import { InhabitDialog } from "../components/InhabitDialog";
import { EntityFilter, NumberRange, SearchBox, SelectFilter, useQueryFilters } from "../components/filters";
import { TableView } from "../components/TableView";
import { POSITIONS, useNationOptions } from "../lookups";
import { useStatus } from "../store";
import { Button } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

export function Inhabit() {
  usePageTitle("Choose someone to live as");
  const st = useStatus();
  const f = useQueryFilters({ q: "str", pos: "str", nation: "num", club: "num", age_min: "num", age_max: "num", kind: "str" });
  const nations = useNationOptions();
  const [pick, setPick] = useState<{ id: number; name: string } | null>(null);
  const you = st.perspective?.mode === "inhabit" ? st.perspective : null;
  return (
    <div className="page fill">
      <PageHead
        crumbs={[{ label: "People", to: "/people" }]}
        title="Choose someone to live as"
        sub={you ? `You are ${you.name}. Choosing someone else changes your point of view only.` : "Pick a player under contract. You will see the world through their eyes and answer for them."}
      />
      <TableView
        id="inhabit"
        table="players"
        label="Players you can inhabit"
        filters={{ ...f.filters, inhabitable: true }}
        preset="inhabit"
        urlSort
        noun={["player", "players"]}
        noColumns
        onOpen={(row) => setPick({ id: row.id, name: row.cells[0]?.s ?? "this player" })}
        toolbar={
          <>
            <SearchBox value={String(f.get("q") ?? "")} onChange={(v) => f.set({ q: v || null })} placeholder="Name" label="Search players" />
            <SelectFilter label="Position" all="Any position" value={f.get("pos") as string | undefined} options={POSITIONS.map((p) => ({ value: p, label: p }))} onChange={(v) => f.set({ pos: v })} />
            <SelectFilter label="Nationality" all="Any nation" value={f.get("nation") as number | undefined} options={nations} onChange={(v) => f.set({ nation: v })} />
            <EntityFilter kind="club" id={f.get("club") as number | undefined} onChange={(id) => f.set({ club: id })} label="Club" placeholder="Club" />
            <NumberRange label="Age" min={f.get("age_min") as number | undefined} max={f.get("age_max") as number | undefined} onMin={(v) => f.set({ age_min: v })} onMax={(v) => f.set({ age_max: v })} lo={14} hi={45} />
            <SelectFilter label="Squad" all="All squads" value={f.get("kind") as string | undefined} options={[{ value: "first", label: "First team" }, { value: "youth", label: "Youth teams" }]} onChange={(v) => f.set({ kind: v })} />
            {f.active > 0 && <Button size="sm" variant="ghost" onClick={f.clear}>Clear filters</Button>}
          </>
        }
      />
      {pick && <InhabitDialog open onClose={() => setPick(null)} person={{ id: pick.id, name: pick.name, short: pick.name }} />}
    </div>
  );
}
