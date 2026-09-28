import { useMemo } from "react";
import { useApi } from "./store";
import type { TableResp } from "./types";

export const POSITIONS = ["GK", "DR", "DC", "DL", "WBR", "WBL", "DM", "MR", "MC", "ML", "AMR", "AMC", "AML", "ST"];
export const POS_GROUPS = [
  { value: "gk", label: "Goalkeepers" },
  { value: "def", label: "Defenders" },
  { value: "mid", label: "Midfielders" },
  { value: "att", label: "Attackers" },
];

export function useNationOptions() {
  const q = useApi<TableResp>("table.query", { table: "nations", limit: 300, sort: { key: "name", desc: false } }, { live: false });
  return useMemo(() => (q.data?.rows ?? []).map((r) => ({ value: r.id, label: r.cells[0]?.s ?? String(r.id) })), [q.data]);
}
