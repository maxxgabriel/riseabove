import { useState } from "react";
import { EntityLink, Money, Dt } from "../components/links";
import { TableView } from "../components/TableView";
import { AttrValue, PitchMap, RatingChips } from "../components/visuals";
import { toggleBookmark, useIsBookmarked } from "../bookmarks";
import { ageAt, date, duration, fmtInt } from "../format";
import { href, navigate, useRoute } from "../router";
import { useApi, useStatus } from "../store";
import type { Named, Ref } from "../types";
import { Avatar, Badge, Button, ErrorState, IconButton, KeyVal, Meter, Metric, Section, Skeleton, StatStrip } from "../ui/ui";
import { PersonActions } from "../components/Actions";
import { InhabitDialog } from "../components/InhabitDialog";
import { Insights } from "../components/Insights";
import { Async, usePageTitle } from "./common";
import { tintOf } from "../color";
import { ClubCrest } from "../components/Crest";
import { useClubColors } from "../crest";
import { DEFAULT_TINT, Stage, StageHeader, StageTabs, type MetaBit } from "../components/Stage";
import { Icon } from "../ui/Icon";

interface PersonResp {
  id: number;
  name: string;
  short: string;
  initials: string;
  age: number;
  dob: number;
  status: string;
  is_me: boolean;
  can_inhabit: boolean;
  perspective: string;
  nations: Named[];
  roles: { label: string; org: Named | null }[];
  player: PlayerInfo | null;
  staff: StaffInfo | null;
  /** Observer only: where an imported record came from and which of its facts were filled in. */
  provenance?: { source: string | null; id: string; snapshot: number | null; facts: { group: string; origin: string }[] } | null;
}
interface PlayerInfo {
  player_id: number;
  best_pos: string;
  positions: { code: string; fam: number; level: string }[];
  foot: string;
  height: number;
  weight: number;
  shirt: number;
  traits: string[];
  team: string | null;
  loan: { parent: Named; club: Named; end: number } | null;
  availability: { label: string; tone: "pos" | "neg" | "warn" | "muted"; detail: string };
  contract: null | {
    club: Named;
    wage: number;
    start: number;
    end: number;
    days_left: number;
    release_clause: number;
    promised_status: string | null;
    appearance_bonus: number;
    goal_bonus: number;
    yearly_rise: number;
    relegation_cut: number;
    kind: string;
  };
  squad_status: string | null;
  value: number | null;
  condition: null | Record<"condition" | "sharpness" | "fitness" | "fatigue" | "morale" | "confidence" | "wellbeing", number>;
  form: number[];
  caps: number;
  intl_goals: number;
  senior_apps: number;
  senior_goals: number;
  joined: number | null;
  youth_club: Named | null;
  internal: null | { ca: number; pa: number; reputation: { current: number; home: number; world: number }; personality: string; bio_offset: number; plan: { focus: string; intensity: string } };
}
interface StaffInfo {
  role: string;
  role_rating: number;
  reputation: number;
  club: Named | null;
  contract: { end: number; wage: number } | null;
  joined: number | null;
  record: { games: number; wins: number; draws: number; losses: number; sackings: number; trophies: number };
  attrs: { label: string; v: number }[];
  style: string | null;
}

type Tab = "overview" | "attributes" | "stats" | "career" | "events";

export function Person() {
  const route = useRoute();
  const id = Number(route.segs[1]);
  const tab = (route.segs[2] as Tab) || "overview";
  const q = useApi<PersonResp>("person", { id });
  usePageTitle(q.data?.name);
  const club = q.data ? clubOf(q.data) : null;
  const colors = useClubColors(club?.id);
  const tint = colors ? tintOf(colors[0]) : DEFAULT_TINT;
  return (
    <Stage tint={tint}>
      {q.error && !q.data ? (
        <div className="stage-body">
          <ErrorState error={q.error} onRetry={q.reload} />
        </div>
      ) : !q.data ? (
        <div className="stage-body" aria-busy="true">
          <Skeleton w="30%" h={40} />
          <Skeleton w="60%" />
        </div>
      ) : (
        <PersonBody p={q.data} tab={tab} club={club} />
      )}
    </Stage>
  );
}

/** The club a person belongs to right now, for the page colours: their contract, their staff post or a role. */
function clubOf(p: PersonResp): Named | null {
  if (p.player?.contract) return p.player.contract.club;
  if (p.staff?.club) return p.staff.club;
  return p.roles.find((r) => r.org?.k === "club")?.org ?? null;
}

function PersonBody({ p, tab, club }: { p: PersonResp; tab: Tab; club: Named | null }) {
  const st = useStatus();
  const bookmarked = useIsBookmarked("person", p.id);
  const [inhabit, setInhabit] = useState(false);
  const isPlayer = !!p.player;
  const tabs: { id: Tab; label: string; hidden?: boolean }[] = [
    { id: "overview", label: "Overview" },
    { id: "attributes", label: "Attributes", hidden: !isPlayer },
    { id: "stats", label: "Statistics", hidden: !isPlayer },
    { id: "career", label: "Career", hidden: !isPlayer },
    { id: "events", label: "History" },
  ];
  const avail = p.player?.availability;
  const busy = st.job.running;
  const meta: MetaBit[] = [];
  if (p.player) {
    meta.push({ label: "Position", value: p.player.best_pos });
    if (p.nations[0]) meta.push({ label: "Nation", value: <EntityLink r={p.nations[0]}>{p.nations[0].name}</EntityLink> });
    if (p.player.contract) meta.push({ label: "Contract", value: <><span>until</span> <Dt d={p.player.contract.end} /></> });
    if (p.player.value != null) meta.push({ label: "Value", value: <Money v={p.player.value} /> });
  } else {
    if (p.staff) meta.push({ label: "Role", value: p.staff.role });
    if (p.nations[0]) meta.push({ label: "Nation", value: <EntityLink r={p.nations[0]}>{p.nations[0].name}</EntityLink> });
  }
  return (
    <>
      <StageHeader
        crest={
          <span className="person-crest">
            <Avatar initials={p.initials} size={64} you={p.is_me} />
            {club && <ClubCrest id={club.id} name={club.name} size={26} />}
          </span>
        }
        title={
          <>
            {p.name}
            {p.is_me && <Badge tone="you">You</Badge>}
          </>
        }
        sub={
          <span className="person-sub">
            <span className="num">{p.age} years old</span>
            {p.roles.map((r, i) => (
              <span key={i}>
                {r.label}
                {r.org && (
                  <>
                    {" · "}
                    <EntityLink r={r.org}>{r.org.name}</EntityLink>
                  </>
                )}
              </span>
            ))}
            {p.player?.loan && (
              <span>
                On loan from <EntityLink r={p.player.loan.parent}>{p.player.loan.parent.name}</EntityLink>
              </span>
            )}
          </span>
        }
        meta={meta}
        actions={
          <>
            {avail && avail.label !== "Available" && <Badge tone={avail.tone === "muted" ? "muted" : avail.tone} title={avail.detail}>{avail.label}</Badge>}
            <IconButton
              icon="bookmark"
              label={bookmarked ? "Remove bookmark" : "Bookmark"}
              aria-pressed={bookmarked}
              className={bookmarked ? "on" : ""}
              onClick={() => toggleBookmark({ k: "person", id: p.id, title: p.name, sub: p.roles[0]?.label })}
            />
            {isPlayer && (
              <Button icon="compare" onClick={() => navigate(`/compare?ids=${p.id}`)}>Compare</Button>
            )}
            {st.perspective?.mode === "inhabit" && !p.is_me && <PersonActions who={{ k: "person", id: p.id, name: p.name }} canMentor={isPlayer} />}
            {p.can_inhabit && !p.is_me && (
              <Button variant="primary" icon="person" onClick={() => setInhabit(true)} disabled={busy} title={busy ? "Wait for the world to stop advancing" : undefined}>
                Inhabit
              </Button>
            )}
          </>
        }
      />
      <StageTabs tabs={tabs.filter((t) => !t.hidden)} value={tab} onChange={(t) => navigate(`/person/${p.id}${t === "overview" ? "" : `/${t}`}`)} label="Person sections" />
      <div className="stage-body">
        {tab === "overview" && (p.player ? <PlayerOverview p={p} pl={p.player} /> : p.staff ? <StaffOverview p={p} s={p.staff} /> : <p className="muted">No further details.</p>)}
        {tab === "attributes" && <AttributesTab id={p.id} />}
        {tab === "stats" && <StatsTab id={p.id} />}
        {tab === "career" && <CareerTab id={p.id} />}
        {tab === "events" && <EventsTab id={p.id} />}
      </div>
      <InhabitDialog open={inhabit} onClose={() => setInhabit(false)} person={{ id: p.id, name: p.name, short: p.short }} />
    </>
  );
}

// ---- overview (player) --------------------------------------------------------------------------

const CONDITION: { key: keyof NonNullable<PlayerInfo["condition"]>; label: string; invert?: boolean }[] = [
  { key: "condition", label: "Condition" },
  { key: "sharpness", label: "Match sharpness" },
  { key: "morale", label: "Morale" },
  { key: "confidence", label: "Confidence" },
  { key: "wellbeing", label: "Wellbeing" },
  { key: "fatigue", label: "Fatigue", invert: true },
];

function PlayerOverview({ p, pl }: { p: PersonResp; pl: PlayerInfo }) {
  const st = useStatus();
  const today = st.date ?? 0;
  return (
    <>
      <StatStrip className="person-stat-strip">
        <Metric label="Position" value={pl.best_pos} detail={pl.squad_status ?? pl.team ?? "First team"} tone="accent" />
        <Metric label="Recent rating" value={pl.form.length ? pl.form[pl.form.length - 1].toFixed(1) : "—"} detail="Latest appearance" />
        <Metric label="Senior career" value={fmtInt(pl.senior_apps)} detail={`${fmtInt(pl.senior_goals)} goals`} />
        {pl.value != null && <Metric label="Market value" value={<Money v={pl.value} />} detail="Estimated value" tone="accent" />}
      </StatStrip>
      <div className="split player-layout">
      <div className="stack">
        <Insights method="insight.person" args={{ id: p.id }} />
        {pl.condition && (
          <Section title="Right now" aside={pl.availability.detail || undefined}>
            <div className="card meters">
              {CONDITION.map((c) => {
                const v = pl.condition![c.key];
                return (
                  <div key={c.key} className="meter-row">
                    <span>{c.label}</span>
                    <Meter value={c.invert ? 100 - v : v} label={String(v)} />
                  </div>
                );
              })}
            </div>
          </Section>
        )}
        <Section title="Recent form" aside="Match ratings, most recent last">
          <div className="card"><RatingChips values={pl.form} /></div>
        </Section>
        <Section
          title="This season"
          aside={<a href={href(`/person/${p.id}/stats`)}>All seasons</a>}
        >
          <TableView id="person-season" table="player_stats" label="This season" filters={{ person: p.id }} height={4} noPresets noColumns />
        </Section>
        <Section title="Recent history" aside={<a href={href(`/person/${p.id}/events`)}>See all</a>}>
          <TableView id="person-events-brief" table="events" label="Recent history" filters={{ person: p.id }} height={6} noPresets noColumns empty="Nothing has happened yet." />
        </Section>
      </div>
      <aside className="stack">
        <Section title="Contract">
          {pl.contract ? (
            <div className="card">
              <KeyVal
                rows={[
                  { k: "Club", v: <EntityLink r={pl.contract.club}>{pl.contract.club.name}</EntityLink> },
                  { k: "Wage per week", v: <Money v={pl.contract.wage} exact /> },
                  { k: "Runs until", v: <><Dt d={pl.contract.end} /> <span className="faint">({duration(pl.contract.days_left)})</span></> },
                  ...(pl.squad_status ? [{ k: "Squad status", v: pl.squad_status }] : []),
                  { k: "Release clause", v: pl.contract.release_clause > 0 ? <Money v={pl.contract.release_clause} /> : "None" },
                  ...(pl.contract.appearance_bonus || pl.contract.goal_bonus ? [{ k: "Bonuses", v: <span><Money v={pl.contract.appearance_bonus} /> per appearance, <Money v={pl.contract.goal_bonus} /> per goal</span> }] : []),
                ]}
              />
              {p.is_me && <a href={href("/contract")} className="cardlink">Contract details and offers</a>}
            </div>
          ) : (
            <div className="card muted">
              {pl.team ? "Contract terms are private to the player and club." : "Not signed to a club."}
            </div>
          )}
        </Section>
        <Section title="Positions">
          <div className="card"><PitchMap positions={pl.positions} /></div>
        </Section>
        <Section title="Profile">
          <div className="card">
            <KeyVal
              rows={[
                { k: "Born", v: <span className="num">{date(p.dob)} <span className="faint">({ageAt(p.dob, today)})</span></span> },
                { k: "Nationality", v: p.nations.map((n, i) => <span key={n.id}>{i > 0 && ", "}<EntityLink r={n}>{n.name}</EntityLink></span>) },
                { k: "Height, weight", v: <span className="num">{pl.height} cm, {pl.weight} kg</span> },
                { k: "Preferred foot", v: pl.foot },
                { k: "Best position", v: pl.best_pos },
                ...(pl.team ? [{ k: "Squad", v: pl.team }] : []),
                ...(pl.shirt ? [{ k: "Shirt number", v: pl.shirt }] : []),
                ...(pl.youth_club ? [{ k: "Youth club", v: <EntityLink r={pl.youth_club}>{pl.youth_club.name}</EntityLink> }] : []),
                ...(pl.joined != null ? [{ k: "Joined", v: <Dt d={pl.joined} /> }] : []),
                { k: "Senior career", v: <span className="num">{fmtInt(pl.senior_apps)} apps, {fmtInt(pl.senior_goals)} goals</span> },
                ...(pl.caps ? [{ k: "International", v: <span className="num">{pl.caps} caps, {pl.intl_goals} goals</span> }] : []),
                ...(pl.value != null ? [{ k: "Market value", v: <Money v={pl.value} /> }] : []),
              ]}
            />
            {pl.traits.length > 0 && <div className="tags">{pl.traits.map((t) => <Badge key={t}>{t}</Badge>)}</div>}
          </div>
        </Section>
        {pl.internal && (
          <Section title="Under the hood" aside={<Badge tone="info">Observer only</Badge>}>
            <div className="card">
              <KeyVal
                rows={[
                  { k: "Current ability", v: <span className="num">{pl.internal.ca}</span>, hint: "The simulation's true rating of the player right now" },
                  { k: "Potential ability", v: <span className="num">{pl.internal.pa}</span>, hint: "The highest the player can reach" },
                  { k: "Personality", v: pl.internal.personality },
                  { k: "Reputation", v: <span className="num">{fmtInt(pl.internal.reputation.current)} <span className="faint">(home {fmtInt(pl.internal.reputation.home)}, world {fmtInt(pl.internal.reputation.world)})</span></span> },
                  { k: "Training", v: `${pl.internal.plan.intensity}, focus ${pl.internal.plan.focus.toLowerCase()}` },
                ]}
              />
            </div>
          </Section>
        )}
        {p.provenance && <DataSource prov={p.provenance} />}
      </aside>
      </div>
    </>
  );
}

/** Imported data stays distinguishable: each group of facts says whether it was read, estimated, generated or is unknown. */
function DataSource({ prov }: { prov: NonNullable<PersonResp["provenance"]> }) {
  const tone = (o: string) => (o === "imported" ? "pos" : o.startsWith("estimated") ? "warn" : "muted");
  return (
    <Section title="Data source" aside={<Badge tone="info">Observer only</Badge>}>
      <div className="card">
        <KeyVal
          rows={[
            { k: "Record", v: <span className="num">{prov.source ?? "unknown"} · {prov.id}</span> },
            ...(prov.snapshot != null ? [{ k: "Data as of", v: <Dt d={prov.snapshot} /> }] : []),
            ...prov.facts.map((f) => ({ k: f.group, v: <Badge tone={tone(f.origin)}>{f.origin}</Badge> })),
          ]}
        />
      </div>
    </Section>
  );
}

function StaffOverview({ p, s }: { p: PersonResp; s: StaffInfo }) {
  return (
    <div className="split">
      <div className="stack">
        <Insights method="insight.person" args={{ id: p.id }} hideEmpty />
        <Section title="Abilities">
          <div className="card">
            <ul className="attrgrid-tight">
              {s.attrs.map((a) => (
                <li key={a.label}><span>{a.label}</span><AttrValue kind="exact" v={a.v} /></li>
              ))}
            </ul>
          </div>
        </Section>
        {s.record.games > 0 && (
          <Section title="Record as manager">
            <div className="card">
              <KeyVal rows={[
                { k: "Games", v: <span className="num">{fmtInt(s.record.games)}</span> },
                { k: "Won, drawn, lost", v: <span className="num">{s.record.wins}, {s.record.draws}, {s.record.losses}</span> },
                { k: "Trophies", v: <span className="num">{s.record.trophies}</span> },
                { k: "Sackings", v: <span className="num">{s.record.sackings}</span> },
              ]} />
            </div>
          </Section>
        )}
      </div>
      <aside className="stack">
        <Section title="Position">
          <div className="card">
            <KeyVal rows={[
              { k: "Role", v: s.role },
              { k: "Club", v: s.club ? <EntityLink r={s.club}>{s.club.name}</EntityLink> : "None" },
              ...(s.contract ? [{ k: "Wage per week", v: <Money v={s.contract.wage} exact /> }, { k: "Contract ends", v: <Dt d={s.contract.end} /> }] : []),
              ...(s.joined != null ? [{ k: "Joined", v: <Dt d={s.joined} /> }] : []),
              { k: "Nationality", v: p.nations.map((n, i) => <span key={n.id}>{i > 0 && ", "}<EntityLink r={n}>{n.name}</EntityLink></span>) },
              { k: "Reputation", v: <span className="num">{fmtInt(s.reputation)}</span> },
              { k: "Rating in role", v: <span className="num">{s.role_rating.toFixed(1)}</span> },
              ...(s.style ? [{ k: "Style", v: s.style }] : []),
            ]} />
          </div>
        </Section>
      </aside>
    </div>
  );
}

// ---- tabs --------------------------------------------------------------------------------------

interface AttrsResp {
  available: boolean;
  reason?: string;
  source: string;
  known: boolean;
  groups: { name: string; attrs: { key: string; label: string; kind: "exact" | "range" | "unknown"; v?: number; lo?: number; hi?: number }[] }[];
  hidden: { label: string; v: number }[] | null;
  internal: { ca: number; pa: number } | null;
  positions: { code: string; fam: number; level: string }[];
}

function AttributesTab({ id }: { id: number }) {
  const q = useApi<AttrsResp>("person.attributes", { id });
  return (
    <Async q={q}>
      {(a) =>
        !a.available ? (
          <p className="muted">{a.reason}</p>
        ) : (
          <div className="stack">
            <div className={`note ${a.known ? "" : "warn"}`}>
              <Icon name={a.known ? "info" : "eyeOff"} size={15} />
              <span>{a.source}</span>
            </div>
            <div className="attrgrid">
              {a.groups.map((g) => (
                <Section key={g.name} title={g.name}>
                  <div className="card">
                    <ul className="attrlist">
                      {g.attrs.map((x) => (
                        <li key={x.key}>
                          <span>{x.label}</span>
                          <AttrValue kind={x.kind} v={x.v} lo={x.lo} hi={x.hi} />
                        </li>
                      ))}
                    </ul>
                  </div>
                </Section>
              ))}
            </div>
            {a.hidden && (
              <Section title="Hidden traits" aside={<Badge tone="info">Observer only</Badge>}>
                <div className="card">
                  <ul className="attrgrid-tight">
                    {a.hidden.map((h) => (
                      <li key={h.label}><span>{h.label}</span><AttrValue kind="exact" v={h.v} /></li>
                    ))}
                  </ul>
                </div>
              </Section>
            )}
          </div>
        )
      }
    </Async>
  );
}

function StatsTab({ id }: { id: number }) {
  return <TableView id="person-stats" table="player_stats" label="Season statistics" filters={{ person: id }} height={30} noPresets noColumns empty="No appearances yet." />;
}

function CareerTab({ id }: { id: number }) {
  return (
    <div className="stack">
      <Section title="Clubs">
        <TableView id="person-spells" table="spells" label="Clubs" filters={{ person: id }} height={12} noPresets noColumns />
      </Section>
      <Section title="Awards">
        <TableView id="person-awards" table="awards" label="Awards" filters={{ person: id }} height={8} noPresets noColumns empty="No individual awards yet." />
      </Section>
    </div>
  );
}

function EventsTab({ id }: { id: number }) {
  return <TableView id="person-events" table="events" label="History" filters={{ person: id }} height={30} noPresets noColumns empty="Nothing has happened yet." />;
}

export type { Ref };
