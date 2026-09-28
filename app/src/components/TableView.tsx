import { useCallback, useMemo, useState, type ReactNode } from "react";
import { fmtInt } from "../format";
import { currentRoute, setQuery } from "../router";
import type { Col, SortSpec } from "../types";
import { Icon } from "../ui/Icon";
import { Menu, Segmented } from "../ui/ui";
import { DataTable, type MetaOut, type TableState } from "./DataTable";
import type { Row } from "../types";

const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

function loadPersisted(id: string): { preset?: string | null; columns?: string[] | null } {
  try {
    return JSON.parse(localStorage.getItem(`ra.table.${id}`) ?? "{}");
  } catch {
    return {};
  }
}

/**
 * Sort lives in the address (so Back returns to the same view); the chosen preset and
 * columns are remembered per table.
 */
export function useTableState(id: string, opts: { urlSort?: boolean } = {}) {
  const [saved, setSaved] = useState(() => loadPersisted(id));
  const [localSort, setLocalSort] = useState<SortSpec | null>(null);
  const q = currentRoute().query;
  const urlSort: SortSpec | null = opts.urlSort && q.get("sort") ? { key: q.get("sort")!, desc: q.get("desc") === "1" } : null;

  const state: TableState = { sort: opts.urlSort ? urlSort : localSort, preset: saved.preset ?? null, columns: saved.columns ?? null };

  const persist = useCallback(
    (patch: { preset?: string | null; columns?: string[] | null }) => {
      setSaved((s) => {
        const n = { ...s, ...patch };
        try {
          localStorage.setItem(`ra.table.${id}`, JSON.stringify(n));
        } catch {
          /* ignore */
        }
        return n;
      });
    },
    [id],
  );

  const setSort = useCallback(
    (s: SortSpec | null) => {
      if (opts.urlSort) setQuery({ sort: s?.key ?? null, desc: s?.desc ? "1" : null });
      else setLocalSort(s);
    },
    [opts.urlSort],
  );
  return { state, setSort, setPreset: (preset: string | null) => persist({ preset, columns: null }), setColumns: (columns: string[] | null) => persist({ columns }) };
}

export interface TableViewProps {
  id: string;
  table: string;
  label: string;
  filters?: Record<string, unknown>;
  toolbar?: ReactNode;
  height?: "fill" | number;
  urlSort?: boolean;
  /** Forced preset (no picker). */
  preset?: string;
  noPresets?: boolean;
  noColumns?: boolean;
  /** Fixed sort when the person has not chosen one. */
  defaultSort?: SortSpec;
  noun?: [string, string];
  selectable?: boolean;
  selected?: Set<number>;
  onSelected?: (s: Set<number>) => void;
  actions?: ReactNode;
  empty?: ReactNode;
  onOpen?: (row: Row) => void;
  stickyFirst?: boolean;
  /** Show the row count. Defaults on for full-page tables. */
  showCount?: boolean;
  /** Called when the table reports how many rows match. */
  onTotal?: (n: number) => void;
}

export function TableView(p: TableViewProps) {
  const { state, setSort, setPreset, setColumns } = useTableState(p.id, { urlSort: p.urlSort });
  const [meta, setMeta] = useState<MetaOut | null>(null);

  const preset = p.preset ?? state.preset ?? null;
  const sort = state.sort ?? p.defaultSort ?? null;
  const req = useMemo(
    () => ({ table: p.table, filters: p.filters ?? {}, sort, columns: p.preset ? null : state.columns, preset }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [p.table, JSON.stringify(p.filters ?? {}), sort?.key, sort?.desc, preset, JSON.stringify(state.columns)],
  );

  const onSort = (key: string, col: Col) => {
    const eff = meta?.sort;
    const cur = state.sort;
    if (cur?.key === key) {
      if (!cur.desc === (col.fmt === "text")) setSort({ key, desc: !cur.desc });
      else setSort(null);
    } else if (!cur && eff && eff[0] === key) {
      setSort({ key, desc: !eff[1] });
    } else {
      setSort({ key, desc: col.fmt !== "text" });
    }
  };

  const noun = p.noun ?? ["row", "rows"];
  const total = meta?.total;
  const presetKey = preset ?? meta?.presets[0];
  const presets = meta?.presets ?? [];
  const cols = meta?.all_columns ?? [];
  const visible = new Set(meta?.columns ?? []);

  const toggleColumn = (c: Col) => {
    if (!meta) return;
    const set = new Set(meta.columns);
    if (set.has(c.key)) set.delete(c.key);
    else set.add(c.key);
    const list = meta.all_columns.filter((x) => set.has(x.key) || x === meta.all_columns[0]).map((x) => x.key);
    setColumns(list);
  };

  return (
    <div className={`tv ${p.height === "fill" || p.height == null ? "tv-fill" : ""}`}>
      {(p.toolbar || p.actions || !p.noPresets || !p.noColumns || p.showCount || p.height == null || p.height === "fill") && <div className="tv-bar">
        <div className="tv-filters">{p.toolbar}</div>
        <div className="tv-tools">
          {p.actions}
          {total != null && (p.showCount ?? (p.height == null || p.height === "fill")) && (
            <span className="tv-count num" aria-live="polite">
              {fmtInt(total)} {total === 1 ? noun[0] : noun[1]}
            </span>
          )}
          {!p.noPresets && !p.preset && presets.length > 1 && (
            <Segmented
              size="sm"
              label="View"
              value={presetKey ?? ""}
              onChange={(v) => setPreset(v)}
              options={presets.map((x) => ({ id: x, label: cap(x) }))}
            />
          )}
          {!p.noColumns && cols.length > 4 && (
            <Menu
              align="end"
              label="Columns"
              items={[
                { kind: "label", label: "Columns" },
                ...cols.slice(1).map((c) => ({ label: c.label, checked: visible.has(c.key), onSelect: () => toggleColumn(c), hint: c.help ? undefined : undefined })),
                { kind: "sep" as const },
                { label: "Reset to view", onSelect: () => setColumns(null), disabled: !state.columns },
              ]}
            >
              {({ setRef, toggle, open }) => (
                <button ref={setRef} type="button" className="btn btn-default btn-sm" onClick={toggle} aria-expanded={open} aria-haspopup="menu">
                  <Icon name="columns" size={14} />
                  Columns
                </button>
              )}
            </Menu>
          )}
        </div>
      </div>}
      {meta?.note && (
        <div className="tv-note">
          <Icon name="info" size={14} />
          {meta.note}
        </div>
      )}
      <DataTable
        id={p.id}
        label={p.label}
        req={req}
        onMeta={(m) => {
          setMeta(m);
          p.onTotal?.(m.total);
        }}
        onSort={onSort}
        height={p.height ?? "fill"}
        selectable={p.selectable}
        selected={p.selected}
        onSelected={p.onSelected}
        onOpen={p.onOpen}
        empty={p.empty}
        stickyFirst={p.stickyFirst}
      />
    </div>
  );
}
