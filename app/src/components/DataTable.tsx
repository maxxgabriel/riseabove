import { useVirtualizer } from "@tanstack/react-virtual";
import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent as RKeyboardEvent, type ReactNode } from "react";
import { call } from "../api";
import { byFmt } from "../format";
import { navigate, refPath } from "../router";
import { useDataRevision } from "../store";
import { useSettings } from "../settings";
import type { Cell, Col, Row, SortSpec, TableReq, TableResp } from "../types";
import { Icon } from "../ui/Icon";
import { ClubCrest } from "./Crest";
import { EntityLink, FormDots, Parts } from "./links";

const PAGE = 80;
const ROW_H = { compact: 24, comfortable: 30, spacious: 38 } as const;
const HEAD_H = { compact: 28, comfortable: 32, spacious: 38 } as const;
const CHECK_W = 34;

export interface TableState {
  sort?: SortSpec | null;
  preset?: string | null;
  columns?: string[] | null;
}

interface Meta {
  all_columns: Col[];
  columns: string[];
  presets: string[];
  total: number;
  note?: string;
  sort: [string, boolean] | null;
}

// ---- data ------------------------------------------------------------------------------------

/**
 * Pages of rows on demand. When the world moves on, rows already on screen stay until
 * their replacements arrive, so a table never flashes empty while time passes.
 */
function useTableRows(req: Omit<TableReq, "offset" | "limit">, want: [number, number]) {
  const rev = useDataRevision();
  const key = JSON.stringify(req);
  const [, bump] = useState(0);
  const st = useRef({ key: "", rev: -1, gen: 0, fresh: new Map<number, Row[]>(), stale: new Map<number, Row[]>(), pending: new Set<number>(), meta: null as Meta | null, error: null as string | null });

  const s = st.current;
  if (s.key !== key) {
    // A different question: forget everything and start over.
    s.key = key;
    s.gen += 1;
    s.fresh = new Map();
    s.stale = new Map();
    s.pending = new Set();
    s.meta = null;
    s.error = null;
    s.rev = rev;
  } else if (s.rev !== rev) {
    // Same question, newer world: keep what is shown until it is replaced.
    for (const [p, rows] of s.fresh) s.stale.set(p, rows);
    s.fresh = new Map();
    s.pending = new Set();
    s.rev = rev;
  }

  const fetchPage = useCallback(
    (p: number) => {
      const cur = st.current;
      if (cur.fresh.has(p) || cur.pending.has(p)) return;
      cur.pending.add(p);
      const gen = cur.gen;
      const mine = cur.rev;
      call<TableResp>("table.query", { ...req, offset: p * PAGE, limit: PAGE })
        .then((r) => {
          const c = st.current;
          if (c.gen !== gen || c.rev !== mine) return;
          c.pending.delete(p);
          c.fresh.set(p, r.rows);
          c.meta = { all_columns: r.all_columns, columns: r.columns, presets: r.presets, total: r.total, note: r.note, sort: r.sort };
          c.error = null;
          bump((n) => n + 1);
        })
        .catch((e: Error) => {
          const c = st.current;
          if (c.gen !== gen) return;
          c.pending.delete(p);
          c.error = e.message;
          bump((n) => n + 1);
        });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key],
  );

  const lo = Math.floor(Math.max(0, want[0]) / PAGE);
  const hi = Math.floor(Math.max(0, want[1]) / PAGE);
  useEffect(() => {
    fetchPage(0);
    for (let p = lo; p <= hi + (want[1] % PAGE > PAGE * 0.6 ? 1 : 0); p++) fetchPage(p);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, rev, lo, hi, fetchPage]);

  const rowAt = (i: number): Row | undefined => {
    const p = Math.floor(i / PAGE);
    return st.current.fresh.get(p)?.[i % PAGE] ?? st.current.stale.get(p)?.[i % PAGE];
  };
  return { meta: s.meta, rowAt, error: s.error, reload: () => { st.current.gen++; st.current.fresh = new Map(); st.current.pending = new Set(); bump((n) => n + 1); } };
}

// ---- cells -----------------------------------------------------------------------------------

function CellView({ col, cell }: { col: Col; cell: Cell }) {
  if (cell.k?.kind === "unknown") return <span className="unknown" title="Not known to you">?</span>;
  if (cell.k?.kind === "hidden") return <span className="unknown" title="Not for you to see">—</span>;
  let body: ReactNode;
  if (cell.parts) {
    body = <Parts parts={cell.parts} />;
  } else if (col.key === "form" && cell.s != null) {
    body = <FormDots form={cell.s} />;
  } else if (cell.k?.kind === "range") {
    body = (
      <span className="num range" title="Your best estimate; the true value lies within this range">
        {Math.round(cell.k.lo)}–{Math.round(cell.k.hi)}
      </span>
    );
  } else {
    const text = cell.s ?? (cell.n != null ? byFmt(col.fmt, cell.n) : "");
    body =
      cell.r && text ? (
        cell.r.k === "club" ? (
          <span className="withcrest">
            <ClubCrest id={cell.r.id} name={text} size={15} />
            <EntityLink r={cell.r}>{text}</EntityLink>
          </span>
        ) : (
          <EntityLink r={cell.r}>{text}</EntityLink>
        )
      ) : (
        text
      );
  }
  if (cell.bar != null) {
    body = (
      <>
        <span className="inbar" aria-hidden="true"><span style={{ width: `${cell.bar * 100}%` }} /></span>
        {body}
      </>
    );
  }
  return <span className={cell.tone ? `tone-${cell.tone}` : undefined} title={cell.sub}>{body}</span>;
}

// ---- table -----------------------------------------------------------------------------------

export interface DataTableProps {
  id: string;
  req: Omit<TableReq, "offset" | "limit">;
  onMeta?: (m: Meta) => void;
  onSort: (key: string, col: Col) => void;
  /** "fill" takes the space it is given; a number is the most rows to show before scrolling. */
  height?: "fill" | number;
  selectable?: boolean;
  selected?: Set<number>;
  onSelected?: (s: Set<number>) => void;
  onOpen?: (row: Row) => void;
  empty?: ReactNode;
  stickyFirst?: boolean;
  label: string;
}

export interface MetaOut extends Meta {}

function widthsKey(id: string) {
  return `ra.colw.${id}`;
}

function DataTableInner(props: DataTableProps) {
  const { id, req, onSort, height = "fill", selectable, selected, onSelected, onOpen, empty, stickyFirst = true, label } = props;
  const settings = useSettings();
  const rowH = Math.round(ROW_H[settings.density] * (settings.textScale / 100));
  const headH = Math.round(HEAD_H[settings.density] * (settings.textScale / 100));
  const scale = settings.textScale / 100;

  const scrollRef = useRef<HTMLDivElement>(null);
  const [range, setRange] = useState<[number, number]>([0, 40]);
  const { meta, rowAt, error, reload } = useTableRows(req, range);
  const total = meta?.total ?? 0;

  useEffect(() => {
    if (meta) props.onMeta?.(meta);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [meta?.total, meta?.columns.join(","), meta?.note, meta?.sort?.join(",")]);

  // Column widths the person has dragged.
  const [custom, setCustom] = useState<Record<string, number>>(() => {
    try {
      return JSON.parse(localStorage.getItem(widthsKey(id)) ?? "{}") as Record<string, number>;
    } catch {
      return {};
    }
  });
  const cols: Col[] = useMemo(() => {
    if (!meta) return [];
    return meta.columns.map((k) => meta.all_columns.find((c) => c.key === k)).filter((c): c is Col => !!c);
  }, [meta]);
  const widths = cols.map((c) => Math.round((custom[c.key] ?? c.w) * scale));
  const lead = selectable ? CHECK_W : 0;
  const totalW = lead + widths.reduce((a, b) => a + b, 0);
  const template = `${selectable ? `${CHECK_W}px ` : ""}${widths.map((w, i) => (i === widths.length - 1 ? `minmax(${w}px, 1fr)` : `${w}px`)).join(" ")}`;

  const shown = height === "fill" ? undefined : meta && total === 0 ? undefined : Math.min(total || 3, height);
  const virt = useVirtualizer({
    count: total,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowH,
    overscan: 12,
    paddingStart: headH,
  });
  const items = virt.getVirtualItems();
  const first = items[0]?.index ?? 0;
  const last = items[items.length - 1]?.index ?? 40;
  useEffect(() => {
    setRange((r) => (r[0] === first && r[1] === last ? r : [first, Math.max(last, first + 30)]));
  }, [first, last]);

  // Rows keep their size when density changes.
  useLayoutEffect(() => {
    virt.measure();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rowH, headH]);

  const [active, setActive] = useState<number>(-1);
  const open = useCallback(
    (row: Row) => {
      if (onOpen) onOpen(row);
      else if (row.open) navigate(refPath(row.open));
    },
    [onOpen],
  );

  const onKey = (e: RKeyboardEvent) => {
    if (total === 0) return;
    const page = Math.max(1, Math.floor(((scrollRef.current?.clientHeight ?? 400) - headH) / rowH) - 1);
    let next = active;
    switch (e.key) {
      case "ArrowDown":
        next = Math.min(total - 1, active + 1);
        break;
      case "ArrowUp":
        next = Math.max(0, active < 0 ? 0 : active - 1);
        break;
      case "PageDown":
        next = Math.min(total - 1, active + page);
        break;
      case "PageUp":
        next = Math.max(0, active - page);
        break;
      case "Home":
        next = 0;
        break;
      case "End":
        next = total - 1;
        break;
      case "Enter": {
        const row = rowAt(active);
        if (row) open(row);
        e.preventDefault();
        return;
      }
      case " ": {
        const row = rowAt(active);
        if (selectable && row && onSelected) {
          const s = new Set(selected);
          if (s.has(row.id)) s.delete(row.id);
          else s.add(row.id);
          onSelected(s);
        }
        e.preventDefault();
        return;
      }
      default:
        return;
    }
    e.preventDefault();
    setActive(next);
    virt.scrollToIndex(next, { align: "auto" });
  };

  const [drag, setDrag] = useState<{ key: string; x: number; w: number } | null>(null);
  useEffect(() => {
    if (!drag) return;
    const move = (e: MouseEvent) => setCustom((c) => ({ ...c, [drag.key]: Math.max(40, Math.round((drag.w + (e.clientX - drag.x)) / scale)) }));
    const up = () => {
      setDrag(null);
      setCustom((c) => {
        try {
          localStorage.setItem(widthsKey(id), JSON.stringify(c));
        } catch {
          /* ignore */
        }
        return c;
      });
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
    return () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
  }, [drag, id, scale]);

  const effSort = meta?.sort;
  const toggleAll = (on: boolean) => {
    if (!onSelected) return;
    if (!on) return onSelected(new Set());
    const s = new Set<number>();
    for (let i = 0; i < Math.min(total, PAGE * 3); i++) {
      const r = rowAt(i);
      if (r) s.add(r.id);
    }
    onSelected(s);
  };

  const style = height === "fill" ? undefined : shown != null ? { height: shown * rowH + headH + 2 } : meta && total === 0 ? { height: "auto" } : { height: 3 * rowH + headH + 2 };
  const isEmpty = !!meta && total === 0;

  return (
    <div className={`dt ${height === "fill" ? "dt-fill" : ""}`} style={style}>
      {error && !meta && (
        <div className="dt-error" role="alert">
          {error} <button className="linkbtn" onClick={reload}>Try again</button>
        </div>
      )}
      <div ref={scrollRef} className="dt-scroll" role="grid" aria-label={label} aria-rowcount={total} tabIndex={0} onKeyDown={onKey} aria-activedescendant={active >= 0 ? `${id}-r${active}` : undefined}>
        {isEmpty ? (
          <div className={`dt-empty ${height === "fill" ? "" : "compact"}`}>{empty ?? "Nothing matches."}</div>
        ) : (
          <div className="dt-inner" style={{ minWidth: Math.max(totalW, 1), height: virt.getTotalSize() }}>
            <div className="dt-head" role="row" style={{ gridTemplateColumns: template, height: headH, minWidth: totalW }}>
              {selectable && (
                <div className="dt-h dt-check" role="columnheader">
                  <input type="checkbox" aria-label="Select all shown" checked={!!selected && selected.size > 0} ref={(el) => { if (el) el.indeterminate = !!selected && selected.size > 0 && selected.size < total; }} onChange={(e) => toggleAll(e.target.checked)} />
                </div>
              )}
              {cols.map((c, i) => {
                const sorted = effSort && effSort[0] === c.key;
                return (
                  <div
                    key={c.key}
                    role="columnheader"
                    aria-sort={sorted ? (effSort[1] ? "descending" : "ascending") : "none"}
                    className={`dt-h align-${c.align} ${i === 0 && stickyFirst ? "sticky" : ""} ${sorted ? "sorted" : ""}`}
                    style={i === 0 && stickyFirst ? { left: lead } : undefined}
                    title={c.help ? `${c.label}: ${c.help}` : c.label}
                  >
                    {c.sortable ? (
                      <button type="button" className="dt-sort" onClick={() => onSort(c.key, c)}>
                        <span className="dt-label">{c.short}</span>
                        {sorted && <Icon name={effSort[1] ? "sortDown" : "sortUp"} size={12} />}
                      </button>
                    ) : (
                      <span className="dt-label">{c.short}</span>
                    )}
                    <span
                      className="dt-resize"
                      onMouseDown={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setDrag({ key: c.key, x: e.clientX, w: widths[i] });
                      }}
                      onDoubleClick={() => {
                        setCustom((cc) => {
                          const n = { ...cc };
                          delete n[c.key];
                          try {
                            localStorage.setItem(widthsKey(id), JSON.stringify(n));
                          } catch {
                            /* ignore */
                          }
                          return n;
                        });
                      }}
                    />
                  </div>
                );
              })}
            </div>
            {items.map((v) => {
              const row = rowAt(v.index);
              return (
                <RowView
                  key={v.index}
                  rid={`${id}-r${v.index}`}
                  index={v.index}
                  top={v.start}
                  h={rowH}
                  row={row}
                  cols={cols}
                  template={template}
                  width={totalW}
                  lead={lead}
                  stickyFirst={stickyFirst}
                  selectable={!!selectable}
                  selected={row ? !!selected?.has(row.id) : false}
                  active={active === v.index}
                  onOpen={open}
                  onActive={setActive}
                  onToggle={(r) => {
                    if (!onSelected) return;
                    const s = new Set(selected);
                    if (s.has(r.id)) s.delete(r.id);
                    else s.add(r.id);
                    onSelected(s);
                  }}
                />
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

interface RowProps {
  rid: string;
  index: number;
  top: number;
  h: number;
  row?: Row;
  cols: Col[];
  template: string;
  width: number;
  lead: number;
  stickyFirst: boolean;
  selectable: boolean;
  selected: boolean;
  active: boolean;
  onOpen: (r: Row) => void;
  onActive: (i: number) => void;
  onToggle: (r: Row) => void;
}

const RowView = memo(function RowView({ rid, index, top, h, row, cols, template, width, lead, stickyFirst, selectable, selected, active, onOpen, onActive, onToggle }: RowProps) {
  const cls = ["dt-row", index % 2 ? "odd" : "", selected ? "selected" : "", active ? "active" : "", row?.tone ? `rowtone-${row.tone}` : "", row?.open ? "openable" : ""].join(" ");
  return (
    <div
      id={rid}
      role="row"
      aria-rowindex={index + 2}
      aria-selected={selectable ? selected : undefined}
      className={cls}
      style={{ transform: `translateY(${top}px)`, height: h, gridTemplateColumns: template, minWidth: width, width: "100%" }}
      onClick={() => {
        onActive(index);
        if (row) onOpen(row);
      }}
    >
      {selectable && (
        <div className="dt-c dt-check" role="gridcell" onClick={(e) => e.stopPropagation()}>
          {row && <input type="checkbox" aria-label="Select row" checked={selected} onChange={() => onToggle(row)} />}
        </div>
      )}
      {cols.map((c, i) => (
        <div
          key={c.key}
          role="gridcell"
          className={`dt-c align-${c.align} ${i === 0 && stickyFirst ? "sticky" : ""} ${c.fmt !== "text" ? "num" : ""}`}
          style={i === 0 && stickyFirst ? { left: lead } : undefined}
        >
          {row ? row.cells[i] && <CellView col={c} cell={row.cells[i]} /> : <span className="skeleton" style={{ width: c.fmt === "text" ? "70%" : "50%", height: 10 }} />}
        </div>
      ))}
    </div>
  );
});

export const DataTable = DataTableInner;
