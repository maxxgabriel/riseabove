import { EntityLink, Money } from "../components/links";
import { PersonPicker } from "../components/filters";
import { AttrValue } from "../components/visuals";
import { navigate, setQuery, useRoute } from "../router";
import { useApiMany } from "../store";
import type { Named } from "../types";
import { Avatar, Empty, IconButton, Section } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

interface PersonLite {
  id: number;
  name: string;
  initials: string;
  age: number;
  nations: Named[];
  roles: { label: string; org: Named | null }[];
  player: null | { best_pos: string; value: number | null; height: number; foot: string; contract: null | { wage: number; end: number }; internal: null | { ca: number; pa: number } };
}
interface AttrLite {
  available: boolean;
  known: boolean;
  groups: { name: string; attrs: { key: string; label: string; kind: "exact" | "range" | "unknown"; v?: number; lo?: number; hi?: number }[] }[];
}

export function Compare() {
  usePageTitle("Compare players");
  const route = useRoute();
  const ids = (route.query.get("ids") ?? "").split(",").map(Number).filter((n) => Number.isFinite(n) && n >= 0 && route.query.get("ids") !== "").slice(0, 6);
  const people = useApiMany<PersonLite>("person", ids.map((id) => ({ id })));
  const attrs = useApiMany<AttrLite>("person.attributes", ids.map((id) => ({ id })));
  const set = (next: number[]) => setQuery({ ids: next.join(",") || null });
  const ps = people.data;
  const as = attrs.data;
  const players = ps?.filter((p) => p.player) ?? [];

  return (
    <div className="page">
      <PageHead
        crumbs={[{ label: "People", to: "/people" }]}
        title="Compare players"
        sub="Up to six players side by side. The best value in each row is marked."
        actions={ids.length < 6 ? <PersonPicker exclude={ids} onPick={(id) => set([...ids, id])} /> : undefined}
      />
      {ids.length === 0 ? (
        <Empty title="Choose players to compare" icon="compare">
          Search for a player above, or select several in <a href="#/people">People</a> and press Compare.
        </Empty>
      ) : !ps || !as ? (
        <p className="muted">Loading…</p>
      ) : (
        <>
          <div className="compare" style={{ ["--cols" as string]: ps.length }}>
            <div className="cmp-row cmp-head">
              <div />
              {ps.map((p) => (
                <div key={p.id} className="cmp-cell cmp-person">
                  <Avatar initials={p.initials} size={34} />
                  <div>
                    <EntityLink r={{ k: "person", id: p.id }}>{p.name}</EntityLink>
                    <div className="hint">{p.roles[0]?.org?.name ?? p.roles[0]?.label}</div>
                  </div>
                  <IconButton icon="x" label={`Remove ${p.name}`} onClick={() => set(ids.filter((i) => i !== p.id))} />
                </div>
              ))}
            </div>
            <Row label="Age" vals={ps.map((p) => p.age)} lower />
            <Row label="Position" vals={ps.map((p) => p.player?.best_pos ?? "")} />
            <Row label="Height" vals={ps.map((p) => p.player?.height ?? null)} render={(v) => `${v} cm`} />
            <Row label="Foot" vals={ps.map((p) => p.player?.foot ?? "")} />
            <Row label="Market value" vals={ps.map((p) => p.player?.value ?? null)} render={(v) => <Money v={v as number} />} />
            <Row label="Wage per week" vals={ps.map((p) => p.player?.contract?.wage ?? null)} render={(v) => <Money v={v as number} exact />} lower />
            {players.some((p) => p.player?.internal) && (
              <>
                <Row label="Current ability" vals={ps.map((p) => p.player?.internal?.ca ?? null)} note="Observer only" />
                <Row label="Potential" vals={ps.map((p) => p.player?.internal?.pa ?? null)} note="Observer only" />
              </>
            )}
          </div>
          {as.find((x) => x.available)?.groups?.map((g, gi) => (
            <Section key={g.name} title={g.name}>
              <div className="compare" style={{ ["--cols" as string]: ps.length }}>
                {g.attrs.map((a, ai) => {
                  const vals = as.map((x) => x.groups?.[gi]?.attrs[ai]);
                  const nums = vals.map((v) => (v && v.kind !== "unknown" ? v.v ?? null : null));
                  const best = Math.max(...nums.map((n) => n ?? -1));
                  return (
                    <div key={a.key} className="cmp-row">
                      <div className="cmp-label">{a.label}</div>
                      {vals.map((v, i) => (
                        <div key={i} className={`cmp-cell ${nums[i] === best && best >= 0 && nums.filter((n) => n === best).length < nums.length ? "best" : ""}`}>
                          {v ? <AttrValue kind={v.kind} v={v.v} lo={v.lo} hi={v.hi} /> : null}
                        </div>
                      ))}
                    </div>
                  );
                })}
              </div>
            </Section>
          ))}
          <button className="linkbtn" style={{ justifySelf: "start" }} onClick={() => navigate("/people")}>Back to people</button>
        </>
      )}
    </div>
  );
}

function Row({ label, vals, render, lower, note }: { label: string; vals: (string | number | null)[]; render?: (v: string | number) => React.ReactNode; lower?: boolean; note?: string }) {
  const nums = vals.map((v) => (typeof v === "number" ? v : null));
  const known = nums.filter((n): n is number => n != null);
  const best = known.length > 1 ? (lower ? Math.min(...known) : Math.max(...known)) : null;
  const differ = new Set(known).size > 1;
  return (
    <div className="cmp-row">
      <div className="cmp-label">{label}{note && <span className="hint"> · {note}</span>}</div>
      {vals.map((v, i) => (
        <div key={i} className={`cmp-cell num ${differ && best != null && nums[i] === best ? "best" : ""}`}>
          {v == null || v === "" ? <span className="faint">–</span> : render ? render(v) : v}
        </div>
      ))}
    </div>
  );
}
