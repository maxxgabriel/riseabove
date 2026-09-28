import { useState } from "react";
import { EntityLink, Money, Dt } from "../components/links";
import { SelectFilter } from "../components/filters";
import { TableView } from "../components/TableView";
import { toggleBookmark, useIsBookmarked } from "../bookmarks";
import { fmtInt, ordinal, plural } from "../format";
import { href, navigate, useRoute } from "../router";
import { act, notify, useApi, useStatus } from "../store";
import type { Named } from "../types";
import { Badge, Button, IconButton, KeyVal, Meter, Section, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";
import { BoardTab, FansTab, RoomTab, type Systems } from "./ClubInside";

interface ClubResp {
  id: number;
  name: string;
  short: string;
  city: string;
  nation: Named;
  colors: [string, string];
  stadium: string;
  capacity: number;
  founded: number;
  reputation: number;
  ownership: string;
  fan_mood: number;
  relation: string;
  followed: boolean;
  league: null | { comp: Named; position: number | null; teams: number; points: number | null; played: number | null };
  manager: null | { person: Named; since: number; record: { games: number; wins: number; draws: number; losses: number } };
  teams: { team: number; kind: string; kind_key: string; squad: number; comp: Named | null; captain: Named | null }[];
  staff_counts: { role: string; count: number }[];
  finance: null | { balance: number; transfer_budget: number; wage_budget: number; wage_bill: number; season_income: number; season_spend: number; debt: number };
  facilities: null | { training: number; youth: number; academy: number; medical: number };
  board: null | { satisfaction: number; patience: number; target_position: number; warnings: number };
  needs: null | { pos: string; min_ability: number; max_age: number; urgency: number }[];
}

type Tab = "overview" | "squad" | "staff" | "fixtures" | "finances" | "board" | "fans" | "room" | "history";

export function Club() {
  const route = useRoute();
  const id = Number(route.segs[1]);
  const tab = (route.segs[2] as Tab) || "overview";
  const q = useApi<ClubResp>("club", { id });
  usePageTitle(q.data?.name);
  return (
    <div className="page">
      <Async q={q}>{(c) => <ClubBody c={c} tab={tab} reload={q.reload} />}</Async>
    </div>
  );
}

function Swatch({ colors }: { colors: [string, string] }) {
  return (
    <span className="swatch" aria-hidden="true" style={{ background: `linear-gradient(135deg, ${colors[0]} 50%, ${colors[1]} 50%)` }} />
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
  return (
    <>
      <PageHead
        crumbs={[{ label: "Clubs", to: "/clubs" }]}
        title={<span className="person-title"><Swatch colors={c.colors} /><span>{c.name}{c.relation === "Your club" && <Badge tone="you">Your club</Badge>}</span></span>}
        sub={
          <span className="person-sub">
            <span>{c.city}</span>
            <EntityLink r={c.nation}>{c.nation.name}</EntityLink>
            {c.league && <EntityLink r={c.league.comp}>{c.league.comp.name}</EntityLink>}
          </span>
        }
        actions={
          <>
            <IconButton icon="bookmark" label={bookmarked ? "Remove bookmark" : "Bookmark"} aria-pressed={bookmarked} className={bookmarked ? "on" : ""} onClick={() => toggleBookmark({ k: "club", id: c.id, title: c.name, sub: c.league?.comp.name })} />
            <Button icon={c.followed ? "check" : "plus"} onClick={toggleFollow} title="Keep full match details (lineups and events) for this club's games">
              {c.followed ? "Following" : "Follow"}
            </Button>
          </>
        }
      />
      <Tabs tabs={tabs} value={tab} onChange={(t) => navigate(`/club/${c.id}${t === "overview" ? "" : `/${t}`}`)} label="Club sections" />
      {tab === "overview" && <Overview c={c} />}
      {tab === "squad" && <Squad c={c} />}
      {tab === "staff" && <TableView id="club-staff" table="staff" label="Staff" filters={{ club: c.id }} height={30} noun={["person", "people"]} noColumns />}
      {tab === "fixtures" && <ClubFixtures c={c} />}
      {tab === "finances" && c.finance && <Finances c={c} f={c.finance} />}
      {tab === "board" && <Async q={sys}>{(s) => <BoardTab s={s} />}</Async>}
      {tab === "fans" && <Async q={sys}>{(s) => <FansTab club={c.id} s={s} />}</Async>}
      {tab === "room" && <Async q={sys}>{(s) => (s.room ? <RoomTab room={s.room} /> : <p className="muted">The dressing room is not open to you.</p>)}</Async>}
      {tab === "history" && <History c={c} />}
    </>
  );
}

function Overview({ c }: { c: ClubResp }) {
  const lg = c.league;
  const today = useStatus().date ?? 0;
  const first = c.teams.find((t) => t.kind_key === "first")?.team;
  return (
    <div className="split">
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
        <Section title="Club">
          <div className="card">
            <KeyVal
              rows={[
                { k: "Stadium", v: <span>{c.stadium || "Not named"}{c.capacity ? <span className="faint num"> ({fmtInt(c.capacity)})</span> : null}</span> },
                { k: "Founded", v: c.founded || "Unknown" },
                { k: "Ownership", v: c.ownership },
                { k: "Reputation", v: <span className="num">{fmtInt(c.reputation)}</span> },
                { k: "Fans", v: <Meter value={c.fan_mood} label={c.fan_mood >= 66 ? "Happy" : c.fan_mood >= 40 ? "Mixed" : "Unhappy"} /> },
              ]}
            />
          </div>
        </Section>
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
                <div key={k} className="meter-row"><span>{k[0].toUpperCase() + k.slice(1)}</span><Meter value={c.facilities![k] * 5} label={`${c.facilities![k]} / 20`} /></div>
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
