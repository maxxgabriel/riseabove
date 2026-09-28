import { useEffect, useId, useMemo, useRef, useState } from "react";
import { call } from "../api";
import { currentRoute, setQuery, useRoute } from "../router";
import type { SearchItem, SearchResp } from "../types";
import { Icon } from "../ui/Icon";
import { useApi } from "../store";

type Kind = "str" | "num" | "bool";

/** Filters that live in the address, so a filtered list survives Back and can be bookmarked. */
export function useQueryFilters<K extends string>(spec: Record<K, Kind>) {
  const route = useRoute();
  const filters = useMemo(() => {
    const out: Record<string, string | number | boolean> = {};
    for (const k of Object.keys(spec) as K[]) {
      const v = route.query.get(k);
      if (v == null || v === "") continue;
      if (spec[k] === "num") {
        const n = Number(v);
        if (Number.isFinite(n)) out[k] = n;
      } else if (spec[k] === "bool") out[k] = v === "1";
      else out[k] = v;
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route.key]);
  const set = (patch: Partial<Record<K, string | number | boolean | null>>) => setQuery(patch);
  const clear = () => setQuery(Object.fromEntries((Object.keys(spec) as K[]).map((k) => [k, null])));
  const active = Object.keys(filters).length;
  return { filters, set, clear, active, get: (k: K) => filters[k] };
}

export function SearchBox({ value, onChange, placeholder = "Search", label = "Search", width = "14rem" }: { value: string; onChange: (v: string) => void; placeholder?: string; label?: string; width?: string }) {
  const [text, setText] = useState(value);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => setText(value), [value]);
  const push = (v: string) => {
    setText(v);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => onChange(v), 180);
  };
  return (
    <div className="searchbox" style={{ width }}>
      <Icon name="search" size={14} />
      <input type="text" aria-label={label} placeholder={placeholder} value={text} onChange={(e) => push(e.target.value)} spellCheck={false} />
      {text && (
        <button type="button" aria-label="Clear search" onClick={() => { setText(""); onChange(""); }}>
          <Icon name="x" size={12} />
        </button>
      )}
    </div>
  );
}

export function SelectFilter({ label, value, options, onChange, all }: { label: string; value: string | number | undefined; options: { value: string | number; label: string }[]; onChange: (v: string | null) => void; all: string }) {
  return (
    <select className="input-sm" aria-label={label} value={value == null ? "" : String(value)} onChange={(e) => onChange(e.target.value || null)} data-active={value != null}>
      <option value="">{all}</option>
      {options.map((o) => (
        <option key={o.value} value={o.value}>{o.label}</option>
      ))}
    </select>
  );
}

export function NumberRange({ label, min, max, onMin, onMax, lo = 0, hi = 99 }: { label: string; min?: number; max?: number; onMin: (v: string | null) => void; onMax: (v: string | null) => void; lo?: number; hi?: number }) {
  return (
    <span className="range-filter" role="group" aria-label={label}>
      <span className="range-label">{label}</span>
      <input className="input-sm" type="number" inputMode="numeric" min={lo} max={hi} placeholder="Min" aria-label={`${label} minimum`} value={min ?? ""} onChange={(e) => onMin(e.target.value || null)} />
      <span aria-hidden="true">–</span>
      <input className="input-sm" type="number" inputMode="numeric" min={lo} max={hi} placeholder="Max" aria-label={`${label} maximum`} value={max ?? ""} onChange={(e) => onMax(e.target.value || null)} />
    </span>
  );
}

export function Chip({ on, onClick, children }: { on: boolean; onClick: () => void; children: string }) {
  return (
    <button type="button" className="chip" aria-pressed={on} onClick={onClick}>
      {children}
    </button>
  );
}

/** Pick a club, competition or nation by typing. */
export function EntityFilter({ kind, id, onChange, label, placeholder }: { kind: "club" | "comp" | "nation"; id: number | undefined; onChange: (id: number | null) => void; label: string; placeholder: string }) {
  const [text, setText] = useState("");
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<SearchItem[]>([]);
  const [sel, setSel] = useState(0);
  const listId = useId();
  const known = useApi<{ name: string }>(id != null ? kind : null, id != null ? { id } : {}, { live: false });
  const groupLabel = kind === "club" ? "Clubs" : kind === "comp" ? "Competitions" : "Nations";

  useEffect(() => {
    const t = text.trim();
    if (t.length < 2) {
      setItems([]);
      return;
    }
    let live = true;
    const h = setTimeout(() => {
      call<SearchResp>("search", { q: t, limit: 8 })
        .then((r) => live && setItems(r.groups.find((g) => g.label === groupLabel)?.items ?? []))
        .catch(() => undefined);
    }, 120);
    return () => {
      live = false;
      clearTimeout(h);
    };
  }, [text, groupLabel]);

  if (id != null) {
    return (
      <span className="chip chip-entity" data-on="true">
        <span>{known.data?.name ?? "…"}</span>
        <button type="button" aria-label={`Remove ${label} filter`} onClick={() => onChange(null)}>
          <Icon name="x" size={12} />
        </button>
      </span>
    );
  }
  return (
    <div className="combo">
      <div className="searchbox" style={{ width: "11rem" }}>
        <Icon name="search" size={14} />
        <input
          type="text"
          role="combobox"
          aria-label={label}
          aria-expanded={open && items.length > 0}
          aria-controls={listId}
          placeholder={placeholder}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setOpen(true);
            setSel(0);
          }}
          onFocus={() => setOpen(true)}
          onBlur={() => setTimeout(() => setOpen(false), 120)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSel((s) => Math.min(items.length - 1, s + 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSel((s) => Math.max(0, s - 1));
            } else if (e.key === "Enter" && items[sel]) {
              onChange(items[sel].id);
              setText("");
            } else if (e.key === "Escape") {
              setOpen(false);
            }
          }}
          spellCheck={false}
        />
      </div>
      {open && items.length > 0 && (
        <ul className="combo-list" id={listId} role="listbox">
          {items.map((it, i) => (
            <li key={it.id} role="option" aria-selected={i === sel}>
              <button
                type="button"
                tabIndex={-1}
                className={i === sel ? "sel" : ""}
                onMouseDown={(e) => {
                  e.preventDefault();
                  onChange(it.id);
                  setText("");
                }}
              >
                <span>{it.title}</span>
                <span className="faint">{it.sub}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export { currentRoute };

/** Type a name, pick a person. Used where the choice is added to a list rather than kept as a filter. */
export function PersonPicker({ onPick, placeholder = "Add a player", exclude = [] }: { onPick: (id: number, name: string) => void; placeholder?: string; exclude?: number[] }) {
  const [text, setText] = useState("");
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<SearchItem[]>([]);
  const [sel, setSel] = useState(0);
  const listId = useId();
  useEffect(() => {
    const t = text.trim();
    if (t.length < 2) {
      setItems([]);
      return;
    }
    let live = true;
    const h = setTimeout(() => {
      call<SearchResp>("search", { q: t, limit: 10 })
        .then((r) => live && setItems((r.groups.find((g) => g.label === "People")?.items ?? []).filter((i) => !exclude.includes(i.id))))
        .catch(() => undefined);
    }, 120);
    return () => {
      live = false;
      clearTimeout(h);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [text]);
  const pick = (it: SearchItem) => {
    onPick(it.id, it.title);
    setText("");
    setItems([]);
  };
  return (
    <div className="combo">
      <div className="searchbox" style={{ width: "15rem" }}>
        <Icon name="plus" size={14} />
        <input
          type="text"
          role="combobox"
          aria-label={placeholder}
          aria-expanded={open && items.length > 0}
          aria-controls={listId}
          placeholder={placeholder}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setOpen(true);
            setSel(0);
          }}
          onFocus={() => setOpen(true)}
          onBlur={() => setTimeout(() => setOpen(false), 120)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSel((s) => Math.min(items.length - 1, s + 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSel((s) => Math.max(0, s - 1));
            } else if (e.key === "Enter" && items[sel]) {
              pick(items[sel]);
            } else if (e.key === "Escape") {
              setOpen(false);
            }
          }}
          spellCheck={false}
        />
      </div>
      {open && items.length > 0 && (
        <ul className="combo-list" id={listId} role="listbox">
          {items.map((it, i) => (
            <li key={it.id} role="option" aria-selected={i === sel}>
              <button type="button" tabIndex={-1} className={i === sel ? "sel" : ""} onMouseDown={(e) => { e.preventDefault(); pick(it); }}>
                <span>{it.title}</span>
                <span className="faint">{it.sub}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
