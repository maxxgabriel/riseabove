import { useEffect, useMemo, useRef, useState } from "react";
import { call } from "../api";
import { href, navigate, refPath } from "../router";
import { useStatus } from "../store";
import type { SearchItem, SearchResp } from "../types";
import { Icon, type IconName } from "../ui/Icon";
import { Kbd } from "../ui/ui";

interface Entry {
  key: string;
  icon: IconName;
  title: string;
  sub?: string;
  to: string;
  group: string;
}

const PAGES: { title: string; to: string; icon: IconName; words: string }[] = [
  { title: "World overview", to: "/overview", icon: "overview", words: "home overview world" },
  { title: "People", to: "/people", icon: "people", words: "players footballers people directory" },
  { title: "Staff", to: "/staff", icon: "people", words: "managers coaches staff" },
  { title: "Clubs", to: "/clubs", icon: "club", words: "clubs teams" },
  { title: "Competitions", to: "/comps", icon: "trophy", words: "competitions leagues cups" },
  { title: "Nations", to: "/nations", icon: "globe", words: "nations countries" },
  { title: "Fixtures and results", to: "/fixtures", icon: "list", words: "fixtures results matches schedule" },
  { title: "Transfers", to: "/transfers", icon: "transfer", words: "transfers signings loans" },
  { title: "Events", to: "/events", icon: "pulse", words: "events news feed" },
  { title: "History and honours", to: "/history", icon: "clock", words: "history honours awards records" },
  { title: "Compare people", to: "/compare", icon: "compare", words: "compare" },
  { title: "Bookmarks", to: "/bookmarks", icon: "bookmark", words: "bookmarks following favourites" },
  { title: "Saves and world", to: "/saves", icon: "save", words: "save load world saves" },
  { title: "Settings", to: "/settings", icon: "sliders", words: "settings preferences theme" },
  { title: "Help", to: "/help", icon: "help", words: "help glossary keys shortcuts" },
  { title: "Diagnostics", to: "/diagnostics", icon: "info", words: "diagnostics capabilities" },
];
const ME_PAGES: typeof PAGES = [
  { title: "Today", to: "/today", icon: "calendar", words: "today" },
  { title: "Messages", to: "/messages", icon: "mail", words: "messages inbox decisions offers" },
  { title: "Calendar", to: "/calendar", icon: "calendar", words: "calendar schedule" },
  { title: "Football", to: "/football", icon: "training", words: "football training squad place" },
  { title: "Contract", to: "/contract", icon: "contract", words: "contract wage" },
  { title: "My profile", to: "/me", icon: "person", words: "me profile my player" },
];
const ICON: Record<string, IconName> = { person: "person", club: "club", comp: "trophy", nation: "globe", match: "pitch" };

export function SearchPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const st = useStatus();
  const [q, setQ] = useState("");
  const [items, setItems] = useState<{ label: string; items: SearchItem[] }[]>([]);
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const inhabiting = st.perspective?.mode === "inhabit";

  useEffect(() => {
    if (open) {
      setQ("");
      setItems([]);
      setSel(0);
      setTimeout(() => input.current?.focus(), 0);
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const t = q.trim();
    if (t.length < 2) {
      setItems([]);
      return;
    }
    let live = true;
    const h = setTimeout(() => {
      call<SearchResp>("search", { q: t, limit: 6 })
        .then((r) => live && setItems(r.groups.map((g) => ({ label: g.label, items: g.items }))))
        .catch(() => live && setItems([]));
    }, 90);
    return () => {
      live = false;
      clearTimeout(h);
    };
  }, [q, open]);

  const entries: Entry[] = useMemo(() => {
    const t = q.trim().toLowerCase();
    const pages = [...(inhabiting ? ME_PAGES : []), ...PAGES].filter((p) => !t || p.title.toLowerCase().includes(t) || p.words.includes(t));
    const out: Entry[] = [];
    for (const g of items) for (const it of g.items) out.push({ key: `${it.k}${it.id}`, icon: ICON[it.k] ?? "info", title: it.title, sub: it.sub, to: refPath(it), group: g.label });
    for (const p of pages.slice(0, t ? 5 : 20)) out.push({ key: p.to, icon: p.icon, title: p.title, to: p.to, group: "Go to" });
    return out;
  }, [items, q, inhabiting]);

  useEffect(() => setSel(0), [entries.length, q]);
  if (!open) return null;

  const go = (e: Entry) => {
    onClose();
    navigate(e.to);
  };
  let lastGroup = "";
  return (
    <div className="palette-back" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="palette" role="dialog" aria-label="Search">
        <div className="palette-input">
          <Icon name="search" />
          <input
            ref={input}
            type="text"
            value={q}
            placeholder="Search people, clubs, competitions, or jump to a page"
            aria-label="Search"
            aria-controls="palette-list"
            aria-activedescendant={entries[sel] ? `pal-${sel}` : undefined}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setSel((s) => Math.min(entries.length - 1, s + 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setSel((s) => Math.max(0, s - 1));
              } else if (e.key === "Enter" && entries[sel]) {
                go(entries[sel]);
              } else if (e.key === "Escape") {
                onClose();
              }
            }}
          />
          <Kbd>Esc</Kbd>
        </div>
        <ul className="palette-list" id="palette-list" role="listbox">
          {entries.length === 0 && <li className="palette-empty">{q.trim().length >= 2 ? "No matches." : "Type at least two letters."}</li>}
          {entries.map((e, i) => {
            const head = e.group !== lastGroup ? e.group : null;
            lastGroup = e.group;
            return (
              <li key={e.key + i} role="presentation">
                {head && <div className="palette-group">{head}</div>}
                <a
                  id={`pal-${i}`}
                  role="option"
                  aria-selected={i === sel}
                  className={`palette-item ${i === sel ? "sel" : ""}`}
                  href={href(e.to)}
                  onMouseMove={() => setSel(i)}
                  onClick={(ev) => {
                    ev.preventDefault();
                    go(e);
                  }}
                >
                  <Icon name={e.icon} />
                  <span className="palette-title">{e.title}</span>
                  {e.sub && <span className="palette-sub">{e.sub}</span>}
                </a>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
