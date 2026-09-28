import { useApi } from "../store";
import type { Capability } from "../types";
import { Badge, Kbd, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

const KEYS: [string[], string][] = [
  [["Ctrl", "K"], "Search anything, or jump to a page (also /)"],
  [["Ctrl", "Enter"], "Continue: advance one day, or until something needs you"],
  [["Ctrl", "S"], "Save the world"],
  [["Alt", "←"], "Back (Alt → for forward)"],
  [["↑", "↓"], "Move through a table, Enter to open, Space to select"],
  [["PgUp", "PgDn"], "Move a page at a time in a table"],
  [["?"], "Open this page"],
];

const TERMS: [string, string][] = [
  ["Observing", "You see everything the simulation knows, including hidden ratings. Nothing you do changes the world except advancing time."],
  ["Inhabiting", "You live one player's career. You see what they could know, answer for them, and set their training. Others' contracts, condition and true ratings are hidden."],
  ["Estimates", "When you inhabit a player, other players' attributes appear as ranges based on how much your club has watched them. The true value lies inside the range."],
  ["Hidden results", "Your team's scores can be kept from you so you can watch or reveal them when you choose. Tables leave those results out until you do."],
  ["Following a club", "Full match details, such as lineups and events, are kept only for followed clubs and your own team. Others keep just the result."],
  ["Replies", "Nothing you write is sent as words. A reply becomes something you do, and the world applies it on the next day. The other side answers only if their own mind does."],
  ["Grapevine", "People pass things on, and not always accurately. As a player you hear each item as what you were told and how sure the teller was. Only someone watching the whole world can see whether it is true."],
  ["Condition", "Physical freshness today. It falls with hard work and recovers with rest."],
  ["Match sharpness", "How ready a player is for match pace. It builds with playing time and fades without it."],
  ["Squad status", "The role a club has promised or intends for a player: from key player down to surplus."],
  ["Release clause", "A fee that lets another club buy the player without the current club's consent."],
];

export function Help() {
  usePageTitle("Help");
  const caps = useApi<Capability[]>("capabilities", {}, { live: false });
  return (
    <div className="page narrow">
      <PageHead title="Help" />
      <Section title="Keyboard">
        <div className="card list-card">
          <ul className="rows">
            {KEYS.map(([k, d]) => (
              <li key={d}><span className="muted">{d}</span><span className="keys">{k.map((x) => <Kbd key={x}>{x}</Kbd>)}</span></li>
            ))}
          </ul>
        </div>
      </Section>
      <Section title="Words used here">
        <div className="card">
          <dl className="glossary">
            {TERMS.map(([t, d]) => (
              <div key={t}><dt>{t}</dt><dd>{d}</dd></div>
            ))}
          </dl>
        </div>
      </Section>
      <Section title="What the simulation covers" aside="Only what exists is shown as working">
        <Async q={caps}>
          {(list) => (
            <div className="card list-card">
              <ul className="rows">
                {list.map((c) => (
                  <li key={c.area}>
                    <div>
                      <strong>{c.area}</strong>
                      <div className="hint">{c.note}</div>
                    </div>
                    <Badge tone={c.status === "ready" ? "pos" : c.status === "partial" ? "warn" : "muted"}>{c.status === "ready" ? "Works" : c.status === "partial" ? "Partly" : "Not yet"}</Badge>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </Async>
      </Section>
    </div>
  );
}
