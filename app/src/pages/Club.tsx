import { useState } from "react";
import { EntityLink, Money, Dt } from "../components/links";
import { SelectFilter } from "../components/filters";
import { TableView } from "../components/TableView";
import { toggleBookmark, useIsBookmarked } from "../bookmarks";
import { cap, fmtInt, ordinal, plural } from "../format";
import { href, navigate, useRoute } from "../router";
import { act, notify, useApi, useStatus } from "../store";
import type { ClubView } from "../contract.generated";
import { Badge, Button, ErrorState, IconButton, KeyVal, Meter, Metric, Section, SideCard, Skeleton, StatStrip } from "../ui/ui";
import { Insights } from "../components/Insights";
import { Async, usePageTitle } from "./common";
import { tintOf } from "../color";
import { Crest } from "../components/Crest";
import { DEFAULT_TINT, Stage, StageHeader, StageTabs, type MetaBit } from "../components/Stage";
import { BoardTab, FansTab, RoomTab, type Systems } from "./ClubInside";

type ClubResp = ClubView;

type Tab = "overview" | "squad" | "staff" | "fixtures" | "finances" | "board" | "fans" | "room" | "history";

export function Club() {
  const route = useRoute();
  const id = Number(route.segs[1]);
  const tab = (route.segs[2] as Tab) || "overview";
  const q = useApi<ClubResp>("club", { id });
  usePageTitle(q.data?.name);
  return (
    <Stage tint={q.data ? tintOf(q.data.colors[0]) : DEFAULT_TINT}>
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
        <ClubBody c={q.data} tab={tab} reload={q.reload} />
      )}
    </Stage>
  );
}

function ClubBody({ c, tab, reload }: { c: ClubResp; tab: Tab; reload: () => void }) {
  const bookmarked = useIsBookmarked("club", c.id);
  const sys = useApi<Systems>("club.systems", { id: c.id });
  const tabs: { id: Tab; label: string; hidden?: boolean }[] = [
    { id: "overview", label: "Overview" },
    { id: "squad", label: "Squad" },
    { id: "staff", label: "Staff" },
    { id: "fixtures", label: "Fixtures" },
    { id: "finances", label: "Finances", hidden: !c.finance },
    { id: "board", label: "Board" },
    { id: "fans", label: "Fans" },
    { id: "room", label: "Dressing room", hidden: !sys.data?.room },
    { id: "history", label: "History" },
  ];
  const toggleFollow = async () => {
    try {
      await act("club.follow", { club: c.id, follow: !c.followed });
      reload();
      notify({ tone: "info", text: c.followed ? `Stopped following ${c.name}.` : `Following ${c.name}. Full match details are now kept for their games.` });
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    }
  };
  const meta: MetaBit[] = [];
  if (c.league) meta.push({ label: "League", value: <><EntityLink r={c.league.comp}>{c.league.comp.name}</EntityLink>{c.league.position && <small>{ordinal(c.league.position)}</small>}</> });
  if (c.manager) meta.push({ label: "Manager", value: <EntityLink r={c.manager.person}>{c.manager.person.name}</EntityLink> });
  meta.push({ label: "Stadium", value: <span className="num">{fmtInt(c.capacity)}</span> });
  meta.push({ label: "Founded", value: <span className="num">{c.founded}</span> });
  return (
    <>
      <StageHeader
        crest={<Crest name={c.name} colors={c.colors} id={c.id} size={58} />}
        title={
          <>
            {c.name}
            {c.relation === "Your club" && <Badge tone="you">Your club</Badge>}
          </>
        }
        sub={[c.city, c.nation.name].filter(Boolean).join(" · ")}
        subIcon="globe"
        meta={meta}
        actions={
          <>
            <IconButton icon="bookmark" label={bookmarked ? "Remove bookmark" : "Bookmark"} aria-pressed={bookmarked} className={bookmarked ? "on" : ""} onClick={() => toggleBookmark({ k: "club", id: c.id, title: c.name, sub: c.league?.comp.name })} />
            <Button icon={c.followed ? "check" : "plus"} onClick={toggleFollow} title="Keep full match details (lineups and events) for this club's games">
              {c.followed ? "Following" : "Follow"}
            </Button>
          </>
        }
      />
      <StageTabs tabs={tabs.filter((t) => !t.hidden)} value={tab} onChange={(t) => navigate(`/club/${c.id}${t === "overview" ? "" : `/${t}`}`)} label="Club sections" />
      <div className="stage-body">
        {tab === "overview" && <Overview c={c} />}
        {tab === "squad" && <Squad c={c} />}
        {tab === "staff" && <TableView id="club-staff" table="staff" label="Staff" filters={{ club: c.id }} height={30} noun={["person", "people"]} noColumns />}
        {tab === "fixtures" && <ClubFixtures c={c} />}
        {tab === "finances" && c.finance && <Finances c={c} f={c.finance} />}
        {tab === "board" && <Async q={sys}>{(s) => <BoardTab s={s} />}</Async>}
        {tab === "fans" && <Async q={sys}>{(s) => <FansTab club={c.id} s={s} />}</Async>}
        {tab === "room" && <Async q={sys}>{(s) => (s.room ? <RoomTab room={s.room} /> : <p className="muted">The dressing room is not open to you.</p>)}</Async>}
        {tab === "history" && <History c={c} />}
      </div>
    </>
  );
}

function Overview({ c }: { c: ClubResp }) {
  const lg = c.league;
  const today = useStatus().date ?? 0;
  const first = c.teams.find((t) => t.kind_key === "first")?.team;
  return (
    <>
      <StatStrip className="club-stat-strip">
        <Metric label="League position" value={lg?.position ? ordinal(lg.position) : "—"} detail={lg?.comp.name ?? "No league"} tone="accent" />
        <Metric label="Points" value={lg?.points ?? "—"} detail={lg?.played ? `${lg.played} matches` : "Season not started"} />
        <Metric label="Reputation" value={fmtInt(c.reputation)} detail="Club standing" />
        <Metric label="Supporters" value={c.fan_mood >= 66 ? "Positive" : c.fan_mood >= 40 ? "Mixed" : "Low"} detail="Current mood" tone={c.fan_mood >= 66 ? "pos" : c.fan_mood >= 40 ? "warn" : "neg"} />
      </StatStrip>
      <div className="split club-layout">
      <div className="stack">
        {lg && (
          <Section title="League" aside={<EntityLink r={lg.comp}>{lg.comp.name}</EntityLink>}>
            <div className="card standing">
              {lg.position ? (
                <>
                  <div className="standing-pos num">{ordinal(lg.position)}</div>
                  <div>
                    <div>{lg.played ? `${lg.points} points from ${plural(lg.played, "match", "matches")}` : "Season not started"}</div>
                    <div className="hint">of {lg.teams} clubs</div>
                  </div>
                </>
              ) : (
                <span className="muted">Not in a league this season.</span>
              )}
            </div>
          </Section>
        )}
        <Insights method="insight.club" args={{ id: c.id }} />
        <Section title="Fixtures and results" aside={<a href={href(`/club/${c.id}/fixtures`)}>All</a>}>
          <TableView id="club-fixtures-brief" table="fixtures" label="Recent and upcoming fixtures" filters={{ ...(first != null ? { team: first } : { club: c.id }), from: today - 12, to: today + 45 }} height={7} noPresets noColumns empty="No fixtures." />
        </Section>
        {c.manager && (
          <Section title="Manager">
            <div className="card">
              <KeyVal
                rows={[
                  { k: "Manager", v: <EntityLink r={c.manager.person}>{c.manager.person.name}</EntityLink> },
                  { k: "In charge since", v: <Dt d={c.manager.since} /> },
                  { k: "Record", v: <span className="num">{c.manager.record.wins} won, {c.manager.record.draws} drawn, {c.manager.record.losses} lost</span> },
                ]}
              />
            </div>
          </Section>
        )}
        {c.needs && c.needs.length > 0 && (
          <Section title="Looking for" aside={<Badge tone="info">Observer only</Badge>}>
            <div className="card">
              <ul className="rows compact">
                {c.needs.map((n, i) => (
                  <li key={i}>
                    <span><strong>{n.pos}</strong> <span className="muted">up to age {n.max_age}, ability {n.min_ability}+</span></span>
                    <Meter value={n.urgency} tone={n.urgency > 66 ? "warn" : "pos"} />
                  </li>
                ))}
              </ul>
            </div>
          </Section>
        )}
      </div>
      <aside className="stack">
        <SideCard title="Club">
            <KeyVal
              rows={[
                { k: "Stadium", v: <span>{c.stadium || "Not named"}{c.capacity ? <span className="faint num"> ({fmtInt(c.capacity)})</span> : null}</span> },
                { k: "Founded", v: c.founded || "Unknown" },
                { k: "Ownership", v: c.ownership },
                { k: "Reputation", v: <span className="num">{fmtInt(c.reputation)}</span> },
                { k: "Fans", v: <Meter value={c.fan_mood} label={c.fan_mood >= 66 ? "Happy" : c.fan_mood >= 40 ? "Mixed" : "Unhappy"} /> },
              ]}
            />
        </SideCard>
        <Place c={c} />
        <Identity c={c} />
        <Section title="Squads">
          <div className="card">
            <ul className="rows compact">
              {c.teams.map((t) => (
                <li key={t.team}>
                  <span>{t.kind}{t.comp && <span className="faint"> · <EntityLink r={t.comp}>{t.comp.name}</EntityLink></span>}</span>
                  <span className="num muted">{t.squad} players</span>
                </li>
              ))}
            </ul>
          </div>
        </Section>
        {c.facilities && (
          <Section title="Facilities">
            <div className="card meters">
              {(["training", "youth", "academy", "medical"] as const).map((k) => (
                <div key={k} className="meter-row"><span>{cap(k)}</span><Meter value={c.facilities![k] * 5} label={`${c.facilities![k]} / 20`} /></div>
              ))}
            </div>
          </Section>
        )}
        {c.board && (
          <Section title="Board" aside={<Badge tone="info">Observer only</Badge>}>
            <div className="card meters">
              <div className="meter-row"><span>Satisfaction</span><Meter value={c.board.satisfaction} label={String(c.board.satisfaction)} /></div>
              <div className="meter-row"><span>Patience</span><Meter value={c.board.patience} label={String(c.board.patience)} /></div>
              <div className="meter-row"><span>Target finish</span><span className="num">{ordinal(c.board.target_position)}</span></div>
              {c.board.warnings > 0 && <div className="meter-row"><span>Warnings given</span><span className="num tone-warn">{c.board.warnings}</span></div>}
            </div>
          </Section>
        )}
        <Section title="Staff">
          <div className="card">
            <ul className="rows compact">
              {c.staff_counts.filter((s) => s.count > 0).map((s) => (
                <li key={s.role}><span>{s.role}</span><span className="num muted">{s.count}</span></li>
              ))}
            </ul>
            <a className="cardlink" href={href(`/club/${c.id}/staff`)}>See staff</a>
          </div>
        </Section>
      </aside>
      </div>
    </>
  );
}

/** What the club is called and known by, its academy, partners and media, as the reference data has them. Nothing for a club the
 * reference does not describe. */
function Identity({ c }: { c: ClubResp }) {
  const a = c.academy;
  if (c.also_known.length === 0 && !a && c.partners.length === 0 && c.channels.length === 0) return null;
  const origin = (o: string) => <Badge tone={o === "Imported" ? "pos" : "muted"}>{o}</Badge>;
  return (
    <Section title="Known as">
      <div className="card">
        {c.also_known.length > 0 && (
          <ul className="rows compact">
            {c.also_known.map((n, i) => (
              <li key={i}>
                <span>
                  {n.text} <span className="faint">· {n.kind}</span>
                </span>
                {origin(n.origin)}
              </li>
            ))}
          </ul>
        )}
        <KeyVal
          rows={[
            ...(a
              ? [
                  {
                    k: "Academy",
                    v: (
                      <span>
                        {a.name} <span className="faint">· {[a.kind, a.residential ? "residential" : null, a.age_groups.length > 0 ? a.age_groups.join(", ") : null].filter(Boolean).join(" · ")}</span>
                      </span>
                    ),
                  },
                ]
              : []),
            ...c.partners.map((p) => ({
              k: p.active ? "Partner" : "Former partner",
              v: (
                <span title={p.purpose}>
                  {p.with}
                  {p.what && <span className="faint"> · {p.what}</span>}
                </span>
              ),
            })),
            ...(c.channels.length > 0 ? [{ k: "Club media", v: c.channels.map((m) => m.name).join(", ") }] : []),
          ]}
        />
      </div>
    </Section>
  );
}

function ClubFixtures({ c }: { c: ClubResp }) {
  const [team, setTeam] = useState<number | null>(c.teams.find((t) => t.kind_key === "first")?.team ?? null);
  return (
    <TableView
      id="club-fixtures"
      table="fixtures"
      label="Fixtures"
      filters={team != null ? { team } : { club: c.id }}
      height={40}
      noPresets
      noColumns
      noun={["match", "matches"]}
      toolbar={
        <SelectFilter label="Squad" all="All squads" value={team ?? undefined} options={c.teams.map((t) => ({ value: t.team, label: t.kind }))} onChange={(v) => setTeam(v == null ? null : Number(v))} />
      }
    />
  );
}

function Squad({ c }: { c: ClubResp }) {
  const [kind, setKind] = useState("first");
  return (
    <TableView
      id="club-squad"
      table="players"
      label="Squad"
      filters={{ club: c.id, kind }}
      height={32}
      noun={["player", "players"]}
      toolbar={
        <SelectFilter
          label="Squad"
          all="All squads"
          value={kind === "all" ? undefined : kind}
          options={c.teams.map((t) => ({ value: t.kind_key, label: `${t.kind} (${t.squad})` }))}
          onChange={(v) => setKind(v ?? "all")}
        />
      }
    />
  );
}

function Finances({ c, f }: { c: ClubResp; f: NonNullable<ClubResp["finance"]> }) {
  const wagePct = f.wage_budget > 0 ? Math.round((f.wage_bill / f.wage_budget) * 100) : 0;
  return (
    <div className="grid-2">
      <Section title="Position">
        <div className="card">
          <KeyVal
            rows={[
              { k: "Bank balance", v: <span className={f.balance < 0 ? "tone-neg" : ""}><Money v={f.balance} /></span> },
              { k: "Debt", v: <Money v={f.debt} /> },
              { k: "Transfer budget", v: <Money v={f.transfer_budget} /> },
            ]}
          />
        </div>
      </Section>
      <Section title="Wages">
        <div className="card">
          <KeyVal
            rows={[
              { k: "Wage bill per week", v: <Money v={f.wage_bill} /> },
              { k: "Wage budget per week", v: <Money v={f.wage_budget} /> },
              { k: "Budget used", v: <Meter value={Math.min(100, wagePct)} tone={wagePct > 100 ? "neg" : wagePct > 90 ? "warn" : "pos"} label={`${wagePct}%`} /> },
            ]}
          />
        </div>
      </Section>
      <Section title="This season">
        <div className="card">
          <KeyVal
            rows={[
              { k: "Income", v: <Money v={f.season_income} /> },
              { k: "Spending", v: <Money v={f.season_spend} /> },
              { k: "Net", v: <span className={f.season_income - f.season_spend < 0 ? "tone-neg" : "tone-pos"}><Money v={f.season_income - f.season_spend} sign /></span> },
            ]}
          />
        </div>
      </Section>
      <div className="note"><span>Amounts are shown to the nearest useful unit. Hover a figure to see it in full.</span></div>
      <span hidden>{c.id}</span>
    </div>
  );
}

function History({ c }: { c: ClubResp }) {
  return (
    <div className="stack">
      <Section title="Titles">
        <TableView id="club-honours" table="honours" label="Titles" filters={{ club: c.id }} height={10} noPresets noColumns empty="No titles yet." />
      </Section>
      <Section title="Transfers">
        <TableView id="club-transfers" table="transfers" label="Transfers" filters={{ club: c.id }} height={14} noPresets noColumns empty="No transfers yet." />
      </Section>
      <Section title="Events">
        <TableView id="club-events" table="events" label="Club events" filters={{ club: c.id }} height={12} noPresets noColumns empty="Nothing has happened yet." />
      </Section>
    </div>
  );
}

/** Where the club is, as a place to live and play: climate, language, people, football, neighbours, how far from home. */
function Place({ c }: { c: ClubResp }) {
  const p = c.place;
  if (!p) return null;
  return (
    <Section title="The place">
      <KeyVal
        rows={[
          { k: "Where", v: <>{p.region}{p.state ? `, ${p.state}` : ""}</> },
          ...(p.from_home ? [{ k: "From home", v: p.from_home }] : []),
          { k: "Climate", v: p.climate },
          ...(p.language ? [{ k: "Language", v: p.language }] : []),
          { k: "People", v: p.population },
          { k: "Football", v: p.football },
          ...(p.nearby.length ? [{ k: "Neighbours", v: <>{p.nearby.map((n, i) => <span key={n.id}>{i > 0 && ", "}<EntityLink r={n}>{n.name}</EntityLink></span>)}</> }] : []),
          ...(p.universities.length ? [{ k: "Universities", v: p.universities.join(", ") }] : []),
        ]}
      />
    </Section>
  );
}
