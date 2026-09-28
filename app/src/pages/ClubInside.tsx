import { Dt, EntityLink, Money } from "../components/links";
import { fmtInt, plural } from "../format";
import { href } from "../router";
import type { Named } from "../types";
import { Badge, KeyVal, Meter, Section } from "../ui/ui";

/** What `club.systems` returns. Anything marked internal is null unless you may look inside this club. */
export interface Systems {
  internal: boolean;
  board: null | {
    owner: Named | null;
    kind: string;
    since: number;
    chairman: Named | null;
    administration: number | null;
    projects: { kind: string; target: number; started: number; completes: number; cost: number | null }[];
    owner_traits: null | { wealth: number; ambition: number; patience: number; meddling: number; frugality: number; fan_sensitivity: number };
    policy: null | {
      wage_cap_mult: number;
      youth_investment: number;
      transfer_style: string;
      max_signing_age: number;
      sell_to_rivals: boolean;
      debt_tolerance: number;
      style_mandate: number;
      youth_minutes_target: number;
      selling_stance: number;
    };
    concerns: null | { label: string; value: number }[];
    red_months: number | null;
  };
  plan: null | {
    built: number;
    homegrown_gap: number;
    groups: { group: string; depth: number; target_depth: number; quality: number; target_quality: number; avg_age: number; expiring: number; injured: number; prospects: number; ageing_starters: number }[];
    needs: { group: string; role: string; min_ability: number; max_age: number; homegrown: boolean; wage_band: number; fee_band: number; urgency: number }[];
    sell: Named[];
    promote: Named[];
  };
  scouting: null | { reports: number; scouts: { who: Named; based: string | null; capacity: number | null; briefs: string[] }[] };
  room: null | {
    harmony: number;
    backing: number;
    groups: { bond: string; cohesion: number; stance: number; size: number; leader: Named | null }[];
    influential: { who: Named; influence: number; standing: string | null }[];
  };
  sponsors: { brand: string; slot: string; until: number; fee: number | null }[];
  rivalries: { with: Named | string; intensity: number; why: string[]; record: [number, number, number]; since: number; last_met: number }[];
  supporters: { kind: string; size: number; manager: number; board: number; team: number; voice: number; last_acted: number }[];
  culture: null | {
    identity: { youth: number; local: number; flair: number; grit: number; underdog: number; glamour: number };
    discipline: number;
    expectations: number;
    patience: number;
    tribalism: number;
    graduates: number;
    drought: number;
  };
}

const who = (n: Named | null) => (n ? <EntityLink r={n}>{n.name}</EntityLink> : <span className="muted">Nobody</span>);

/** Moods run from -100 to 100; say them in words and keep the number for those who want it. */
function Mood({ v }: { v: number }) {
  const [word, tone] = v <= -40 ? ["Hostile", "neg"] : v <= -15 ? ["Unhappy", "warn"] : v < 15 ? ["Neutral", "muted"] : v < 40 ? ["Pleased", "pos"] : ["Delighted", "pos"];
  return <span className={`tone-${tone}`} title={`${v > 0 ? "+" : ""}${v}`}>{word}</span>;
}

function Internal() {
  return <Badge tone="info">Observer only</Badge>;
}

// ---- board -------------------------------------------------------------------------------------

export function BoardTab({ s }: { s: Systems }) {
  const b = s.board;
  return (
    <div className="split">
      <div className="stack">
        <Section title="Ownership">
          <div className="card">
            {b ? (
              <KeyVal
                rows={[
                  { k: "Owner", v: who(b.owner) },
                  { k: "Kind", v: b.kind },
                  { k: "Owned since", v: <Dt d={b.since} year /> },
                  ...(b.chairman && b.chairman.id !== b.owner?.id ? [{ k: "Chairman", v: who(b.chairman) }] : []),
                  ...(b.administration != null ? [{ k: "In administration since", v: <Dt d={b.administration} year /> }] : []),
                  ...(b.red_months ? [{ k: "Months deep in the red", v: <span className="num tone-warn">{b.red_months}</span> }] : []),
                ]}
              />
            ) : (
              <span className="muted">No board is recorded for this club.</span>
            )}
          </div>
        </Section>
        {b && b.projects.length > 0 && (
          <Section title="Building work">
            <div className="card">
              <table className="minitable">
                <thead>
                  <tr><th>Project</th><th className="r">Started</th><th className="r">Finishes</th>{s.internal && <th className="r">Cost</th>}</tr>
                </thead>
                <tbody>
                  {b.projects.map((p, i) => (
                    <tr key={i}>
                      <td>{p.kind[0].toUpperCase() + p.kind.slice(1)}</td>
                      <td className="r"><Dt d={p.started} year /></td>
                      <td className="r"><Dt d={p.completes} year /></td>
                      {s.internal && <td className="r"><Money v={p.cost} /></td>}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Section>
        )}
        <Section title="Sponsors">
          <div className="card">
            {s.sponsors.length === 0 ? (
              <span className="muted">No sponsorship deals are running.</span>
            ) : (
              <table className="minitable">
                <thead>
                  <tr><th>Brand</th><th>On the</th><th className="r">Until</th>{s.internal && <th className="r">Per year</th>}</tr>
                </thead>
                <tbody>
                  {s.sponsors.map((p, i) => (
                    <tr key={i}>
                      <td>{p.brand}</td>
                      <td>{p.slot}</td>
                      <td className="r"><Dt d={p.until} year /></td>
                      {s.internal && <td className="r"><Money v={p.fee} /></td>}
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </Section>
        {s.plan && <PlanSection plan={s.plan} />}
        {s.scouting && <ScoutingSection sc={s.scouting} />}
      </div>
      <aside className="stack">
        {b?.owner_traits && (
          <Section title="The owner" aside={<Internal />}>
            <div className="card meters">
              <div className="meter-row"><span>Wealth</span><Money v={b.owner_traits.wealth} /></div>
              {(
                [
                  ["Ambition", b.owner_traits.ambition],
                  ["Patience", b.owner_traits.patience],
                  ["Meddling", b.owner_traits.meddling],
                  ["Frugality", b.owner_traits.frugality],
                  ["Follows the fans", b.owner_traits.fan_sensitivity],
                ] as const
              ).map(([k, v]) => (
                <div key={k} className="meter-row"><span>{k}</span><Meter value={v} tone="flat" label={String(v)} /></div>
              ))}
            </div>
          </Section>
        )}
        {b?.policy && (
          <Section title="What the board wants" aside={<Internal />}>
            <div className="card">
              <KeyVal
                rows={[
                  { k: "Transfers", v: b.policy.transfer_style[0].toUpperCase() + b.policy.transfer_style.slice(1) },
                  { k: "Signing age limit", v: b.policy.max_signing_age ? <span className="num">{b.policy.max_signing_age}</span> : "None" },
                  { k: "Top wage", v: <span className="num">{b.policy.wage_cap_mult.toFixed(1)}× the median</span> },
                  { k: "Debt allowed", v: <span className="num">{b.policy.debt_tolerance.toFixed(2)}× a year's revenue</span> },
                  { k: "Youth spending", v: <span className="num">{b.policy.youth_investment}%</span> },
                  { k: "Academy minutes", v: <span className="num">{b.policy.youth_minutes_target}% of first-team minutes</span> },
                  { k: "Style", v: styleWord(b.policy.style_mandate) },
                  { k: "Sells to rivals", v: b.policy.sell_to_rivals ? "Yes" : "No" },
                  { k: "Selling", v: b.policy.selling_stance < 0.9 ? "Eager to sell" : b.policy.selling_stance > 1.3 ? "Reluctant to sell" : "Open to offers" },
                ]}
              />
            </div>
          </Section>
        )}
        {b?.concerns && (
          <Section title="On the board's mind" aside={<Internal />}>
            <div className="card">
              {b.concerns.length === 0 ? (
                <span className="muted">Nothing is troubling them.</span>
              ) : (
                <ul className="rows compact">
                  {b.concerns.map((c) => (
                    <li key={c.label}>
                      <span>{c.label[0].toUpperCase() + c.label.slice(1)}</span>
                      <span className={`num ${c.value < 0 ? "tone-warn" : "tone-pos"}`}>{c.value < 0 ? "Worried" : "Pleased"}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </Section>
        )}
      </aside>
    </div>
  );
}

const styleWord = (n: number) => (n <= -2 ? "Very defensive" : n === -1 ? "Defensive" : n === 0 ? "No preference" : n === 1 ? "Attacking" : "Very attacking");

function PlanSection({ plan }: { plan: NonNullable<Systems["plan"]> }) {
  return (
    <Section title="Squad plan" aside={<Internal />}>
      <div className="card stack tight">
        <table className="minitable">
          <thead>
            <tr><th>Group</th><th className="r">Depth</th><th className="r">Quality</th><th className="r">Avg age</th><th className="r">Expiring</th><th className="r">Injured</th><th className="r">Prospects</th></tr>
          </thead>
          <tbody>
            {plan.groups.map((g) => (
              <tr key={g.group}>
                <td>{g.group}</td>
                <td className={`r num ${g.depth < g.target_depth ? "tone-warn" : ""}`}>{g.depth} <span className="faint">/ {g.target_depth}</span></td>
                <td className={`r num ${g.quality < g.target_quality ? "tone-warn" : ""}`}>{Math.round(g.quality)} <span className="faint">/ {Math.round(g.target_quality)}</span></td>
                <td className="r num">{g.avg_age.toFixed(1)}</td>
                <td className="r num">{g.expiring || ""}</td>
                <td className="r num">{g.injured || ""}</td>
                <td className="r num">{g.prospects || ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="hint">Depth and quality are shown against what the club is aiming for. Built <Dt d={plan.built} year />.</p>
        {plan.homegrown_gap > 0 && <p>{plural(plan.homegrown_gap, "more homegrown player")} needed to keep the squad legal.</p>}
        {plan.needs.length > 0 && (
          <div>
            <h3 className="subhead">Looking to sign</h3>
            <ul className="rows compact">
              {plan.needs.map((n, i) => (
                <li key={i}>
                  <span><strong>{n.group}</strong> <span className="muted">{n.role}, {n.max_age} or younger, ability {n.min_ability}+{n.homegrown ? ", must be homegrown" : ""}</span></span>
                  <span className="muted">up to <Money v={n.fee_band} /></span>
                </li>
              ))}
            </ul>
          </div>
        )}
        {plan.sell.length > 0 && (
          <div>
            <h3 className="subhead">Would sell</h3>
            <p>{plan.sell.map((p, i) => <span key={p.id}>{i > 0 && ", "}<EntityLink r={p}>{p.name}</EntityLink></span>)}</p>
          </div>
        )}
        {plan.promote.length > 0 && (
          <div>
            <h3 className="subhead">Expected to step up</h3>
            <p>{plan.promote.map((p, i) => <span key={p.id}>{i > 0 && ", "}<EntityLink r={p}>{p.name}</EntityLink></span>)}</p>
          </div>
        )}
      </div>
    </Section>
  );
}

function ScoutingSection({ sc }: { sc: NonNullable<Systems["scouting"]> }) {
  return (
    <Section title="Scouting" aside={<Internal />}>
      <div className="card">
        {sc.scouts.length === 0 ? (
          <span className="muted">The club has no scouts.</span>
        ) : (
          <ul className="rows compact">
            {sc.scouts.map((s) => (
              <li key={s.who.id}>
                <span><EntityLink r={s.who}>{s.who.name}</EntityLink>{s.based && <span className="faint"> · based in {s.based}</span>}</span>
                <span className="muted rows-right">{s.briefs.length ? s.briefs.join(", ") : "No brief"}</span>
              </li>
            ))}
          </ul>
        )}
        <p className="hint">{plural(sc.reports, "report")} filed so far.</p>
      </div>
    </Section>
  );
}

// ---- fans ----------------------------------------------------------------------------------------

export function FansTab({ club, s }: { club: number; s: Systems }) {
  const total = s.supporters.reduce((a, g) => a + g.size, 0);
  return (
    <div className="split">
      <div className="stack">
        <Section title="Supporter groups" aside={total > 0 ? <span className="num">{fmtInt(total)} in all</span> : undefined}>
          <div className="card">
            {s.supporters.length === 0 ? (
              <span className="muted">No organised supporters yet.</span>
            ) : (
              <table className="minitable">
                <thead>
                  <tr><th>Group</th><th className="r">Size</th><th className="r">Voice</th><th>Of the manager</th><th>Of the board</th><th>Of the team</th></tr>
                </thead>
                <tbody>
                  {[...s.supporters].sort((a, b) => b.size - a.size).map((g) => (
                    <tr key={g.kind}>
                      <td>{g.kind}</td>
                      <td className="r num">{fmtInt(g.size)}</td>
                      <td className="r"><Meter value={g.voice} tone="flat" /></td>
                      <td><Mood v={g.manager} /></td>
                      <td><Mood v={g.board} /></td>
                      <td><Mood v={g.team} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </Section>
        <Section title="Rivalries">
          {s.rivalries.length === 0 ? (
            <div className="card"><span className="muted">No rivalries have formed.</span></div>
          ) : (
            <div className="card">
              <ul className="rows">
                {[...s.rivalries].sort((a, b) => b.intensity - a.intensity).map((r, i) => (
                  <li key={i}>
                    <span className="grow">
                      <strong>{typeof r.with === "string" ? r.with : <EntityLink r={r.with}>{r.with.name}</EntityLink>}</strong>
                      <span className="muted"> · {r.why.length ? r.why.join(", ") : "no particular reason"}</span>
                      <span className="hint block">Since <Dt d={r.since} year />; last met <Dt d={r.last_met} year />; record {r.record[0]}-{r.record[1]}-{r.record[2]}</span>
                    </span>
                    <Meter value={r.intensity} tone={r.intensity > 66 ? "neg" : "warn"} label={r.intensity > 66 ? "Bitter" : r.intensity > 33 ? "Heated" : "Mild"} />
                  </li>
                ))}
              </ul>
            </div>
          )}
        </Section>
        <Section title="In their words">
          <div className="card">
            <ul className="rows compact">
              <li><a href={href(`/society/chants?club=${club}`)}>Chants</a><span className="muted">what the ground sings</span></li>
              <li><a href={href(`/society/memes?club=${club}`)}>In-jokes</a><span className="muted">phrases the fans have made their own</span></li>
              <li><a href={href(`/society/posts?club=${club}`)}>Posts</a><span className="muted">what people are saying online</span></li>
            </ul>
          </div>
        </Section>
      </div>
      <aside className="stack">
        {s.culture && <CultureSection c={s.culture} />}
      </aside>
    </div>
  );
}

function CultureSection({ c }: { c: NonNullable<Systems["culture"]> }) {
  return (
    <Section title="What the club stands for" aside={<Internal />}>
      <div className="card meters">
        {(
          [
            ["Trust in the academy", c.identity.youth],
            ["Pride in local players", c.identity.local],
            ["Expects flair", c.identity.flair],
            ["Grit over glamour", c.identity.grit],
            ["Small club against the world", c.identity.underdog],
            ["Stars and money", c.identity.glamour],
          ] as const
        ).map(([k, v]) => (
          <div key={k} className="meter-row"><span>{k}</span><Meter value={v} tone="flat" label={String(v)} /></div>
        ))}
        <hr className="soft" />
        {(
          [
            ["Expectations", c.expectations],
            ["Patience with managers", c.patience],
            ["Us against them", c.tribalism],
            ["Discipline", c.discipline],
          ] as const
        ).map(([k, v]) => (
          <div key={k} className="meter-row"><span>{k}</span><Meter value={v} tone="flat" label={String(v)} /></div>
        ))}
        <div className="meter-row"><span>Academy graduates in the first team</span><span className="num">{c.graduates}</span></div>
        <div className="meter-row"><span>Seasons since a trophy</span><span className="num">{c.drought}</span></div>
      </div>
    </Section>
  );
}

// ---- dressing room -----------------------------------------------------------------------------

export function RoomTab({ room }: { room: NonNullable<Systems["room"]> }) {
  return (
    <div className="split">
      <div className="stack">
        <Section title="Cliques" aside={<Internal />}>
          <div className="card">
            {room.groups.length === 0 ? (
              <span className="muted">Nobody has formed a clique.</span>
            ) : (
              <table className="minitable">
                <thead>
                  <tr><th>Bond</th><th>Led by</th><th className="r">Players</th><th>Tight</th><th>Trust in the manager</th></tr>
                </thead>
                <tbody>
                  {room.groups.map((g, i) => (
                    <tr key={i}>
                      <td>{g.bond}</td>
                      <td>{who(g.leader)}</td>
                      <td className="r num">{g.size}</td>
                      <td><Meter value={g.cohesion} tone="flat" /></td>
                      <td><Meter value={g.stance} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </Section>
        <Section title="Who carries weight" aside={<Internal />}>
          <div className="card">
            <ul className="rows compact">
              {room.influential.map((p) => (
                <li key={p.who.id}>
                  <span><EntityLink r={p.who}>{p.who.name}</EntityLink>{p.standing && <span className="faint"> · {p.standing}</span>}</span>
                  <Meter value={p.influence} tone="flat" label={String(p.influence)} />
                </li>
              ))}
            </ul>
          </div>
        </Section>
      </div>
      <aside className="stack">
        <Section title="The room" aside={<Internal />}>
          <div className="card meters">
            <div className="meter-row"><span>Harmony</span><Meter value={room.harmony} label={String(room.harmony)} /></div>
            <div className="meter-row"><span>Backing the manager</span><Meter value={room.backing} label={String(room.backing)} /></div>
          </div>
        </Section>
      </aside>
    </div>
  );
}
