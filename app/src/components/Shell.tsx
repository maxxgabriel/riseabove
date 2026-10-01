import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { back, forward, href, navigate, useRoute } from "../router";
import { setSettings, useSettings } from "../settings";
import { dismissNotice, notify, save, useApi, useDirty, useNotice, useStatus, act } from "../store";
import { dateLong, relativeDays } from "../format";
import type { MeTodayView } from "../contract.generated";
import { Icon, type IconName } from "../ui/Icon";
import { Button, IconButton, Kbd, Menu } from "../ui/ui";
import { AdvanceControl, useAdvance } from "./Advance";
import { ClubCrest } from "./Crest";
import { SearchPalette } from "./Search";

interface NavItem {
  label: string;
  to: string;
  icon: IconName;
  match?: string[];
  badge?: number;
}

function useNav(): { top: NavItem[]; groups: { label: string; items: NavItem[] }[]; bottom: NavItem[] } {
  const st = useStatus();
  const inhabiting = st.perspective?.mode === "inhabit";
  const world: NavItem[] = [
    { label: "Overview", to: "/overview", icon: "overview" },
    { label: "People", to: "/people", icon: "people", match: ["/person", "/staff", "/compare", "/inhabit"] },
    { label: "Clubs", to: "/clubs", icon: "club", match: ["/club"] },
    { label: "Competitions", to: "/comps", icon: "trophy", match: ["/comp"] },
    { label: "Nations", to: "/nations", icon: "globe", match: ["/nation"] },
    { label: "Fixtures", to: "/fixtures", icon: "list", match: ["/match"] },
    { label: "Transfers", to: "/transfers", icon: "transfer" },
    { label: "Society", to: "/society", icon: "chat" },
    { label: "Events", to: "/events", icon: "pulse" },
    { label: "History", to: "/history", icon: "clock" },
    { label: "Development", to: "/development", icon: "globe" },
    { label: "Begin a life", to: "/begin", icon: "person" },
    { label: "Districts", to: "/district", icon: "globe" },
    { label: "Source database", to: "/database", icon: "list" },
  ];
  const groups = [];
  if (inhabiting) {
    groups.push({
      label: "My career",
      items: [
        { label: "Today", to: "/today", icon: "calendar" as IconName },
        { label: "Messages", to: "/messages", icon: "mail" as IconName, badge: st.awaiting ?? 0 },
        { label: "News", to: "/news", icon: "star" as IconName },
        { label: "Calendar", to: "/calendar", icon: "clock" as IconName },
        { label: "Me", to: "/me", icon: "person" as IconName, match: [`/person/${st.perspective && st.perspective.mode === "inhabit" ? st.perspective.person : -1}`] },
        { label: "Football", to: "/football", icon: "training" as IconName },
        { label: "Contract", to: "/contract", icon: "contract" as IconName },
      ],
    });
    groups.push({
      label: "My life",
      items: [
        { label: "Life", to: "/life", icon: "home" as IconName },
        { label: "People", to: "/relationships", icon: "people" as IconName },
        { label: "Press and fans", to: "/press", icon: "star" as IconName },
        { label: "Social", to: "/social", icon: "pulse" as IconName },
        { label: "Journal", to: "/journal", icon: "bookmark" as IconName },
      ],
    });
  }
  groups.push({ label: "World", items: world });
  if (!inhabiting) groups.unshift({ label: "Around the world", items: [{ label: "News", to: "/news", icon: "star" as IconName }] });
  groups.push({ label: "Yours", items: [{ label: "Bookmarks", to: "/bookmarks", icon: "bookmark" as IconName }] });
  return {
    top: [],
    groups,
    bottom: [
      { label: "World and saves", to: "/saves", icon: "save" },
      { label: "Settings", to: "/settings", icon: "sliders" },
      { label: "Help", to: "/help", icon: "help" },
    ],
  };
}

function isCurrent(item: NavItem, path: string, inhabitingId?: number): boolean {
  if (item.to === "/me") return inhabitingId != null && path === `/person/${inhabitingId}`;
  if (inhabitingId != null && item.to === "/people" && path === `/person/${inhabitingId}`) return false;
  if (path === item.to || path.startsWith(`${item.to}/`)) return true;
  return !!item.match?.some((m) => path === m || path.startsWith(`${m}/`));
}

function Rail() {
  const nav = useNav();
  const st = useStatus();
  const route = useRoute();
  const inhabitingId = st.perspective?.mode === "inhabit" ? st.perspective.person : undefined;
  const renderItem = (it: NavItem) => (
    <a key={it.to} className="rail-item" href={href(it.to)} aria-current={isCurrent(it, route.path, inhabitingId) ? "page" : undefined} title={it.label}>
      <Icon name={it.icon} />
      <span className="rail-label">{it.label}</span>
      {it.badge ? <span className="rail-badge" aria-label={`${it.badge} waiting`}>{it.badge}</span> : null}
    </a>
  );
  const s = useSettings();
  return (
    <nav className="rail" aria-label="Main">
      <div className="rail-brand">
        <span className="rail-mark"><Icon name="up" size={16} strokeWidth={2.4} /></span>
        <span className="rail-brand-name">Rise Above</span>
      </div>
      {nav.groups.map((g, i) => (
        <div key={g.label} role="group" aria-label={g.label}>
          {i > 0 && <div className="rail-sep" />}
          {g.items.map(renderItem)}
        </div>
      ))}
      <div className="rail-spacer" />
      {inhabitingId != null && <RailNext key={st.job.running ? "running" : st.revision} />}
      <div className="rail-sep" />
      {nav.bottom.map(renderItem)}
      <button className="rail-item rail-collapse" onClick={() => setSettings({ railCollapsed: !s.railCollapsed })} title={s.railCollapsed ? "Expand sidebar" : "Collapse sidebar"} aria-label={s.railCollapsed ? "Expand sidebar" : "Collapse sidebar"}>
        <Icon name={s.railCollapsed ? "right" : "left"} />
        <span className="rail-label">Collapse</span>
      </button>
    </nav>
  );
}

/** The next match, read once per change in the world (not while time runs). */
function RailNext() {
  const t = useApi<MeTodayView>("me.today", {}, { live: false }).data;
  const nm = t?.next_match;
  if (!t || !nm) return null;
  return (
    <a className="rail-next" href={href(`/match/${nm.uid}`)}>
      <span className="rail-next-label">Next match</span>
      <span className="rail-next-body">
        <ClubCrest id={nm.opponent.id} name={nm.opponent.name} size={22} />
        <span>
          <span className="rail-next-opp">{nm.home ? "v" : "at"} {nm.opponent.name}</span>
          <span className="rail-next-when">{nm.comp.name} · {relativeDays(nm.date, t.date)}</span>
        </span>
      </span>
    </a>
  );
}

function PerspectiveChip() {
  const st = useStatus();
  const p = st.perspective;
  const busy = st.job.running;
  const you = p?.mode === "inhabit";
  const debug = p?.mode === "observer";
  // Watching from outside opens in the public view; the omniscient view is a debug tool asked for by name.
  const observe = async (omniscient: boolean) => {
    try {
      await act("persp.observe", { omniscient });
      if (you) navigate("/overview");
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    }
  };
  const label = you ? p.name : debug ? "Observer (debug)" : "Public view";
  return (
    <Menu
      align="end"
      label="Perspective"
      items={
        you
          ? [
              { kind: "label", label: "You are inhabiting" },
              { label: p.name, hint: p.club ?? undefined, icon: "person", onSelect: () => navigate("/me") },
              { kind: "sep" },
              { label: "Choose someone else…", icon: "swap", onSelect: () => navigate("/inhabit"), disabled: busy },
              { label: "Go back to watching from outside", icon: "eye", onSelect: () => void observe(false), disabled: busy },
            ]
          : [
              { kind: "label", label: debug ? "Observer (debug view)" : "Public view" },
              { label: debug ? "You see everything the simulation knows, hidden ability and private feelings included." : "You see what the public can: results, news and what clubs make known.", icon: "eye", onSelect: () => undefined, disabled: true },
              { kind: "sep" },
              debug
                ? { label: "Switch to the public view", icon: "eye", onSelect: () => void observe(false), disabled: busy }
                : { label: "Switch to the observer (debug) view", icon: "eye", onSelect: () => void observe(true), disabled: busy },
              { label: "Inhabit a player…", icon: "person", onSelect: () => navigate("/inhabit"), disabled: busy },
            ]
      }
    >
      {({ setRef, toggle, open }) => (
        <button
          ref={setRef}
          type="button"
          className={`persp-chip ${you ? "you" : ""} ${debug ? "debug" : ""}`}
          onClick={toggle}
          aria-haspopup="menu"
          aria-expanded={open}
          title={you ? "You are living this person's career. Others' private details are hidden." : debug ? "Observer (debug): you can see everything" : "Public view: what anyone following the game could know"}
        >
          <span className="dot" />
          <span className="name">{label}</span>
          <Icon name="down" size={12} />
        </button>
      )}
    </Menu>
  );
}

function TopBar({ onSearch }: { onSearch: () => void }) {
  const st = useStatus();
  return (
    <header className="top">
      <div className="top-nav">
        <IconButton icon="arrowLeft" label="Back (Alt+Left)" onClick={back} />
        <IconButton icon="arrowRight" label="Forward (Alt+Right)" onClick={forward} />
      </div>
      <button type="button" className="top-search" onClick={onSearch} aria-label="Search" aria-keyshortcuts="Control+K">
        <Icon name="search" size={15} />
        <span>Search</span>
        <Kbd>Ctrl K</Kbd>
      </button>
      <div className="top-spacer" />
      {st.date != null && (
        <div className="top-date" title="The date in the world">
          <strong>{dateLong(st.date)}</strong>
        </div>
      )}
      <AdvanceControl />
      <PerspectiveChip />
    </header>
  );
}

function NoticeBar() {
  const n = useNotice();
  const route = useRoute();
  // A notice that points somewhere has done its job once you are there.
  useEffect(() => {
    const to = n?.action?.to;
    if (to && route.path === to.split("?")[0]) dismissNotice();
  }, [n, route.path]);
  useEffect(() => {
    if (!n || n.tone === "warn" || n.tone === "neg") return;
    const t = setTimeout(dismissNotice, 6000);
    return () => clearTimeout(t);
  }, [n]);
  if (!n) return null;
  return (
    <div className={`notice ${n.tone}`} role="status">
      <Icon name={n.tone === "neg" ? "warn" : n.tone === "warn" ? "info" : "check"} size={15} />
      <span className="grow">{n.text}</span>
      {n.action && (
        <Button size="sm" onClick={() => (n.action?.to ? navigate(n.action.to) : n.action?.run?.())}>
          {n.action.label}
        </Button>
      )}
      <IconButton icon="x" label="Dismiss" onClick={dismissNotice} />
    </div>
  );
}

function StatusBar() {
  const st = useStatus();
  const dirty = useDirty();
  const doSave = async () => {
    try {
      const f = await save();
      notify({ tone: "pos", text: `Saved as ${f.replace(/\.pws$/, "")}.` });
    } catch (e) {
      notify({ tone: "neg", text: `Could not save: ${(e as Error).message}` });
    }
  };
  return (
    <footer className="status">
      <span>{st.name}</span>
      {dirty ? <span className="unsaved">Unsaved changes</span> : <span>Saved</span>}
      <span className="grow" />
      {st.job.running && <span>{st.job.matches.toLocaleString()} matches played</span>}
      <button onClick={doSave} disabled={st.job.running} title="Save (Ctrl+S)"><Icon name="save" size={13} /> Save</button>
    </footer>
  );
}

export function Shell({ children }: { children: ReactNode }) {
  const st = useStatus();
  const s = useSettings();
  const route = useRoute();
  const [searching, setSearching] = useState(false);
  const adv = useAdvance();
  const main = useRef<HTMLElement>(null);
  const scrolls = useRef(new Map<string, number>());

  // Return to where you were when you come back to a page.
  useLayoutEffect(() => {
    const el = main.current;
    if (el) el.scrollTop = scrolls.current.get(route.key) ?? 0;
  }, [route.key]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      const typing = t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable;
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setSearching(true);
      } else if (!typing && !mod && e.key === "/") {
        e.preventDefault();
        setSearching(true);
      } else if (!typing && !mod && e.key === "?") {
        navigate("/help");
      } else if (mod && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void save()
          .then((f) => notify({ tone: "pos", text: `Saved as ${f.replace(/\.pws$/, "")}.` }))
          .catch((err: Error) => notify({ tone: "neg", text: err.message }));
      } else if (mod && e.key === "Enter") {
        e.preventDefault();
        if (!adv.running) void adv.primary();
      } else if (e.altKey && e.key === "ArrowLeft") {
        back();
      } else if (e.altKey && e.key === "ArrowRight") {
        forward();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [adv]);

  const you = st.perspective?.mode === "inhabit";
  return (
    <div className={`app ${s.railCollapsed ? "rail-collapsed" : ""}`}>
      {you && <div className="stripe" aria-hidden="true" />}
      <a
        className="skip"
        href="#main"
        onClick={(e) => {
          // The hash is the router's, so move focus by hand.
          e.preventDefault();
          main.current?.focus();
        }}
      >
        Skip to content
      </a>
      <Rail />
      <TopBar onSearch={() => setSearching(true)} />
      <main
        className="main"
        ref={main}
        id="main"
        tabIndex={-1}
        onScroll={(e) => scrolls.current.set(route.key, e.currentTarget.scrollTop)}
      >
        <NoticeBar />
        {children}
      </main>
      <StatusBar />
      <SearchPalette open={searching} onClose={() => setSearching(false)} />
    </div>
  );
}


