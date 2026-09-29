import { useEffect, useMemo, useRef, useState } from "react";
import { EntityLink, Dt } from "../components/links";
import { dateLong, fmtInt, ordinal } from "../format";
import { href, useRoute } from "../router";
import { act, notify, useApi } from "../store";
import type { Named } from "../types";
import { Icon } from "../ui/Icon";
import { Badge, Button, Section, Segmented, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface Side {
  team: Named;
  short: string;
  club: number;
  colors: [string, string];
  mine: boolean;
}
interface Ev {
  t: number;
  minute: number;
  side: 0 | 1;
  kind: string;
  label: string;
  tier: "key" | "shot" | "minor";
  player: Named | null;
  other: Named | null;
  x: number;
  y: number;
  xg: number;
}
interface Line {
  player: Named | null;
  side: 0 | 1;
  started: boolean;
  pos: string | null;
  keeper: boolean;
  minutes: number;
  rating: number;
  goals: number;
  assists: number;
  shots: number;
  on_target: number;
  passes: number;
  passes_completed: number;
  tackles: number;
  tackles_won: number;
  saves: number;
  yellows: number;
  reds: number;
  injured: boolean;
  xg: number;
}
interface TeamStats {
  possession: number;
  shots: number;
  on_target: number;
  xg: number;
  big_chances: number;
  corners: number;
  fouls: number;
  offsides: number;
  passes: number;
  passes_completed: number;
  tackles: number;
  saves: number;
  yellows: number;
  reds: number;
}
interface Detail {
  events: Ev[];
  lines: Line[];
  stats: [TeamStats, TeamStats];
  man_of_the_match: Named | null;
  shape_home: { formation: string | null };
  shape_away: { formation: string | null };
}
interface MatchResp {
  uid: number;
  comp: Named;
  round: string;
  date: number;
  home: Side;
  away: Side;
  venue: string;
  capacity: number | null;
  neutral: boolean;
  decisive: boolean;
  status: "played" | "scheduled" | "not played";
  concealed: boolean;
  watching: boolean;
  score: null | { home: number; away: number; ht_home: number; ht_away: number; extra_time: boolean; pens: [number, number] | null; text: string };
  detail: Detail | null;
  detail_kept: boolean;
  can_follow: boolean;
  pre: { home: { position: number | null; absences: { player: Named; why: string }[] }; away: { position: number | null; absences: { player: Named; why: string }[] } };
  previous: { uid: number; date: number; comp: string; home: string; away: string; score: string | null }[];
}

type Tab = "summary" | "lineups" | "map";

export function Match() {
  const route = useRoute();
  const uid = Number(route.segs[1]);
  const [watching, setWatching] = useState(false);
  const q = useApi<MatchResp>(watching ? "match.watch" : "match", { uid });
  usePageTitle(q.data ? `${q.data.home.short} v ${q.data.away.short}` : undefined);
  useEffect(() => setWatching(false), [uid]);
  return (
    <div className="page">
      <Async q={q}>{(m) => <MatchBody m={m} watching={watching} setWatching={setWatching} reload={q.reload} />}</Async>
    </div>
  );
}

function MatchBody({ m, watching, setWatching, reload }: { m: MatchResp; watching: boolean; setWatching: (w: boolean) => void; reload: () => void }) {
  const [tab, setTab] = useState<Tab>("summary");
  const played = m.status === "played";
  const hidden = m.concealed && !watching;
  const detail = m.detail;
  const [minute, setMinute] = useState<number>(95);
  const [revealed, setRevealed] = useState(false);

  const reveal = async () => {
    try {
      await act("match.reveal", { uid: m.uid });
      setWatching(false);
      reload();
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    }
  };
  const follow = async (club: number, name: string) => {
    await act("club.follow", { club, follow: true });
    notify({ tone: "info", text: `Following ${name}. Full details will be kept for their future matches.` });
    reload();
  };

  const showScore = played && !hidden && m.score;
  // In watch mode the score builds up as the scrubber moves.
  const scoreAt = useMemo(() => (detail ? runningScore(detail.events, m.score) : null), [detail, m.score]);
  const live = watching && detail && scoreAt ? scoreAt(minute) : null;

  return (
    <>
      <PageHead
        crumbs={[{ label: "Fixtures", to: "/fixtures" }, { label: m.comp.name, r: m.comp }]}
        title={`${m.home.short} v ${m.away.short}`}
        sub={
          <span className="person-sub">
            <span>{m.round}</span>
            <span>{dateLong(m.date)}</span>
            <span>{venueText(m)}</span>
          </span>
        }
      />

      <div className="card scoreboard" aria-live="polite">
        <TeamHead side={m.home} pos={m.pre.home.position} />
        <div className="scoreline">
          {hidden ? (
            <>
              <div className="score num hidden-score" aria-label="Result hidden"><Icon name="lock" size={22} /></div>
              <div className="hint">Result hidden</div>
            </>
          ) : live ? (
            <>
              <div className="score num">{live.home} – {live.away}</div>
              <div className="hint num">{minute >= endMinute(detail!.events) ? "Full-time" : minute < 1 ? "Kick-off" : `${Math.floor(minute)}'`}</div>
            </>
          ) : showScore && m.score ? (
            <>
              <div className="score num">{m.score.home} – {m.score.away}</div>
              <div className="hint num">
                Half-time {m.score.ht_home}–{m.score.ht_away}
                {m.score.extra_time && " · after extra time"}
                {m.score.pens && ` · ${m.score.pens[0]}–${m.score.pens[1]} on penalties`}
              </div>
            </>
          ) : (
            <>
              <div className="score num faint">v</div>
              <div className="hint">{m.status === "scheduled" ? <Dt d={m.date} /> : "Not played"}</div>
            </>
          )}
        </div>
        <TeamHead side={m.away} pos={m.pre.away.position} right />
      </div>

      {m.concealed && !watching && (
        <div className="note">
          <Icon name="lock" size={15} />
          <span>You chose to keep this result hidden. Standings and tables leave it out too.</span>
          <span className="grow" />
          <Button size="sm" onClick={() => setWatching(true)} icon="play">Watch it play out</Button>
          <Button size="sm" variant="primary" onClick={reveal}>Show the result</Button>
        </div>
      )}
      {watching && (
        <div className="note">
          <Icon name="eye" size={15} />
          <span>Watching. The result stays hidden everywhere else until you choose to show it.</span>
          <span className="grow" />
          <Button size="sm" onClick={() => setWatching(false)}>Stop watching</Button>
          <Button size="sm" variant="primary" onClick={reveal}>Show the result</Button>
        </div>
      )}
      {played && !hidden && !detail && (
        <div className="note">
          <Icon name="info" size={15} />
          <span>Only the result was kept for this match. Detailed records are kept for followed clubs and for your own team.</span>
          <span className="grow" />
          {m.can_follow && (
            <>
              <Button size="sm" onClick={() => follow(m.home.club, m.home.short)}>Follow {m.home.short}</Button>
              <Button size="sm" onClick={() => follow(m.away.club, m.away.short)}>Follow {m.away.short}</Button>
            </>
          )}
        </div>
      )}

      {detail && (watching || (played && !hidden)) && (
        <>
          {watching && <Scrubber events={detail.events} minute={minute} setMinute={setMinute} onEnd={() => setRevealed(true)} />}
          <Tabs
            value={tab}
            onChange={setTab}
            label="Match sections"
            tabs={[{ id: "summary", label: "Summary" }, { id: "lineups", label: "Players", hidden: watching && !revealed }, { id: "map", label: "Event map" }]}
          />
          {tab === "summary" && <Summary m={m} d={detail} until={watching ? minute : 200} done={!watching || minute >= endMinute(detail.events)} />}
          {tab === "lineups" && <Lineups m={m} d={detail} />}
          {tab === "map" && <EventMap m={m} d={detail} until={watching ? minute : 200} />}
        </>
      )}

      {(m.status !== "played" || hidden) && <Preview m={m} />}
    </>
  );
}

const endMinute = (events: Ev[]) => Math.max(90, ...events.map((e) => e.minute));

const SHOT_KINDS = new Set(["Goal", "OwnGoal", "PenaltyGoal", "PenaltyMiss", "ShotSaved", "ShotWide", "ShotBlocked", "ShotPost"]);

/** What can honestly be said about the match so far, counted from the events that have happened. */
function statsSoFar(events: Ev[], until: number): Partial<TeamStats>[] {
  return [0, 1].map((side) => {
    const mine = events.filter((e) => e.minute <= until && e.side === side);
    const theirs = events.filter((e) => e.minute <= until && e.side !== side);
    const count = (f: (e: Ev) => boolean) => mine.filter(f).length;
    const shots = mine.filter((e) => SHOT_KINDS.has(e.kind));
    return {
      shots: shots.length,
      on_target: count((e) => e.kind === "Goal" || e.kind === "PenaltyGoal" || e.kind === "ShotSaved"),
      xg: shots.reduce((a, e) => a + (e.xg ?? 0), 0),
      corners: count((e) => e.kind === "Corner"),
      fouls: count((e) => e.kind === "Foul"),
      offsides: count((e) => e.kind === "Offside"),
      yellows: count((e) => e.kind === "Yellow" || e.kind === "SecondYellow"),
      reds: count((e) => e.kind === "Red"),
      saves: theirs.filter((e) => e.kind === "ShotSaved").length,
    };
  });
}

function runningScore(events: Ev[], final: MatchResp["score"]) {
  // Own goals are recorded against the side that made them; flip if the totals only agree that way.
  const tally = (flip: boolean) => (upTo: number) => {
    let h = 0;
    let a = 0;
    for (const e of events) {
      if (e.minute > upTo) continue;
      const scored = e.kind === "Goal" || e.kind === "PenaltyGoal";
      const own = e.kind === "OwnGoal";
      if (!scored && !own) continue;
      const side = own ? (flip ? e.side : 1 - e.side) : e.side;
      if (side === 0) h++;
      else a++;
    }
    return { home: h, away: a };
  };
  const primary = tally(false);
  if (final) {
    const end = primary(200);
    if ((end.home !== final.home || end.away !== final.away)) {
      const alt = tally(true);
      const e2 = alt(200);
      if (e2.home === final.home && e2.away === final.away) return alt;
    }
  }
  return primary;
}

function venueText(m: MatchResp): string {
  if (m.neutral) return "Neutral venue";
  const cap = m.capacity ? fmtInt(m.capacity) : null;
  if (m.venue) return cap ? `${m.venue} (${cap})` : m.venue;
  return cap ? `Capacity ${cap}` : "";
}

function TeamHead({ side, pos, right }: { side: Side; pos: number | null; right?: boolean }) {
  return (
    <div className={`teamhead ${right ? "right" : ""}`}>
      <span className="swatch" style={{ background: `linear-gradient(135deg, ${side.colors[0]} 50%, ${side.colors[1]} 50%)`, width: "2.4rem", height: "2.4rem" }} aria-hidden="true" />
      <div>
        <EntityLink r={{ k: "club", id: side.club }} className="teamhead-name">{side.team.name}</EntityLink>
        <div className="hint teamhead-sub">{pos ? <span>{ordinal(pos)} in the table</span> : null}{side.mine ? <Badge tone="you">Your team</Badge> : null}</div>
      </div>
    </div>
  );
}

// ---- watch controls --------------------------------------------------------------------------------

function Scrubber({ events, minute, setMinute, onEnd }: { events: Ev[]; minute: number; setMinute: (n: number) => void; onEnd: () => void }) {
  const end = endMinute(events);
  const [playing, setPlaying] = useState(true);
  const [speed, setSpeed] = useState(1);
  const ref = useRef(minute);
  ref.current = minute;
  useEffect(() => {
    if (!playing) return;
    const id = setInterval(() => {
      const next = Math.min(end, ref.current + 0.5 * speed);
      setMinute(next);
      if (next >= end) {
        setPlaying(false);
        onEnd();
      }
    }, 100);
    return () => clearInterval(id);
  }, [playing, speed, end, setMinute, onEnd]);
  useEffect(() => {
    setMinute(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const marks = events.filter((e) => e.minute <= minute && (e.kind === "Goal" || e.kind === "PenaltyGoal" || e.kind === "OwnGoal" || e.kind === "Red" || e.kind === "SecondYellow"));
  return (
    <div className="card scrubber">
      <Button icon={playing ? "stop" : "play"} onClick={() => { if (minute >= end) setMinute(0); setPlaying((p) => !p); }}>
        {playing ? "Pause" : minute >= end ? "Replay" : "Play"}
      </Button>
      <div className="scrub-track">
        <input type="range" min={0} max={end} step={0.5} value={minute} onChange={(e) => { setPlaying(false); setMinute(Number(e.target.value)); if (Number(e.target.value) >= end) onEnd(); }} aria-label="Match time" />
        <div className="scrub-marks" aria-hidden="true">
          {marks.map((e, i) => (
            <span key={i} className={`scrub-mark side${e.side} ${e.kind === "Red" || e.kind === "SecondYellow" ? "card-red" : ""}`} style={{ left: `${(e.minute / end) * 100}%` }} title={`${e.minute}' ${e.label}`} />
          ))}
        </div>
      </div>
      <span className="num scrub-time">{minute >= end ? "Full-time" : `${Math.floor(minute)}'`}</span>
      <Segmented size="sm" label="Speed" value={speed} onChange={setSpeed} options={[{ id: 1, label: "1×" }, { id: 3, label: "3×" }, { id: 8, label: "8×" }]} />
    </div>
  );
}

// ---- summary ---------------------------------------------------------------------------------------

const TIMELINE = new Set(["Goal", "OwnGoal", "PenaltyGoal", "PenaltyMiss", "Yellow", "SecondYellow", "Red", "Sub", "Injury", "ShootoutGoal", "ShootoutMiss"]);

function kindIcon(k: string): string {
  switch (k) {
    case "Goal":
    case "PenaltyGoal":
    case "ShootoutGoal":
      return "●";
    case "OwnGoal":
      return "○";
    case "Yellow":
    case "SecondYellow":
      return "▮";
    case "Red":
      return "▮";
    case "Sub":
      return "⇄";
    case "Injury":
      return "+";
    default:
      return "×";
  }
}

const STAT_ROWS: { label: string; key: keyof TeamStats; dec?: boolean; suffix?: string; final?: boolean }[] = [
  { label: "Possession", key: "possession", suffix: "%", final: true },
  { label: "Shots", key: "shots" },
  { label: "On target", key: "on_target" },
  { label: "Expected goals", key: "xg", dec: true },
  { label: "Big chances", key: "big_chances", final: true },
  { label: "Corners", key: "corners" },
  { label: "Passes", key: "passes", final: true },
  { label: "Tackles", key: "tackles", final: true },
  { label: "Saves", key: "saves" },
  { label: "Fouls", key: "fouls" },
  { label: "Offsides", key: "offsides" },
  { label: "Yellow cards", key: "yellows" },
  { label: "Red cards", key: "reds" },
];

function Summary({ m, d, until, done }: { m: MatchResp; d: Detail; until: number; done: boolean }) {
  const evs = d.events.filter((e) => TIMELINE.has(e.kind) && e.minute <= until).sort((a, b) => a.minute - b.minute);
  const cur: Partial<TeamStats>[] = done ? d.stats : statsSoFar(d.events, until);
  return (
    <div className="split">
      <div className="stack">
        <Section title="Key moments">
          <div className="card timeline">
            {evs.length === 0 ? (
              <div className="muted">Nothing yet.</div>
            ) : (
              evs.map((e, i) => {
                const body = (
                  <span className="tl-text">
                    <span className={`tl-ico ${e.kind === "Red" || e.kind === "SecondYellow" ? "red" : e.kind === "Yellow" ? "yellow" : e.kind.includes("Goal") ? "goal" : ""}`} aria-hidden="true">{kindIcon(e.kind)}</span>
                    <span>
                      <strong>{e.label}</strong>
                      {e.player && <> <EntityLink r={e.player}>{e.player.name}</EntityLink></>}
                      {e.other && <span className="muted">{e.kind === "Sub" ? " for " : " (assist "}{e.other.name}{e.kind === "Sub" ? "" : ")"}</span>}
                    </span>
                  </span>
                );
                return (
                  <div key={i} className="tl">
                    <div className="tl-home">{e.side === 0 ? body : null}</div>
                    <span className="tl-min num">{e.minute}'</span>
                    <div className="tl-away">{e.side === 1 ? body : null}</div>
                  </div>
                );
              })
            )}
          </div>
        </Section>
        {d.man_of_the_match && done && (
          <p className="muted">Man of the match: <EntityLink r={d.man_of_the_match}>{d.man_of_the_match.name}</EntityLink></p>
        )}
      </div>
      <aside className="stack">
        <Section title="Match statistics" aside={<span className="num">{d.shape_home.formation} v {d.shape_away.formation}</span>}>
          <div className="card statbars">
            {STAT_ROWS.filter((r) => done || !r.final).map((r) => (
              <StatRow key={r.key} label={r.label} a={cur[0][r.key] ?? 0} b={cur[1][r.key] ?? 0} dec={r.dec} suffix={r.suffix} />
            ))}
          </div>
          {!done && <p className="hint">Counted as it happens. Possession, passing and tackles appear at full time.</p>}
        </Section>
        <span hidden>{m.uid}</span>
      </aside>
    </div>
  );
}

function StatRow({ label, a, b, dec, suffix = "" }: { label: string; a: number; b: number; dec?: boolean; suffix?: string }) {
  const total = a + b || 1;
  const f = (n: number) => (dec ? n.toFixed(2) : String(n)) + suffix;
  return (
    <div className="statrow">
      <span className={`num ${a > b ? "lead" : ""}`}>{f(a)}</span>
      <div className="statmid">
        <span className="statlabel">{label}</span>
        <div className="statbar" aria-hidden="true">
          <span className="a" style={{ width: `${(a / total) * 100}%` }} />
          <span className="b" style={{ width: `${(b / total) * 100}%` }} />
        </div>
      </div>
      <span className={`num ${b > a ? "lead" : ""}`}>{f(b)}</span>
    </div>
  );
}

// ---- players -------------------------------------------------------------------------------------------

const POS_ORDER = ["GK", "DR", "DC", "DL", "WBR", "WBL", "DM", "MR", "MC", "ML", "AMR", "AMC", "AML", "ST"];

function Lineups({ m, d }: { m: MatchResp; d: Detail }) {
  const motm = d.man_of_the_match?.id;
  const sides = [0, 1] as const;
  return (
    <div className="grid-2">
      {sides.map((s) => {
        const lines = d.lines
          .filter((l) => l.side === s && l.minutes > 0)
          .sort((a, b) => Number(b.started) - Number(a.started) || POS_ORDER.indexOf(a.pos ?? "") - POS_ORDER.indexOf(b.pos ?? ""));
        const side = s === 0 ? m.home : m.away;
        return (
          <Section key={s} title={side.team.name} aside={<span className="num">{s === 0 ? d.shape_home.formation : d.shape_away.formation}</span>}>
            <div className="card list-card">
              <table className="minitable">
                <thead>
                  <tr><th>Pos</th><th>Player</th><th className="r">Min</th><th className="r">Rating</th><th className="r">G</th><th className="r">A</th><th className="r">Sh</th><th className="r">Pass</th><th className="r">Tkl</th></tr>
                </thead>
                <tbody>
                  {lines.map((l, i) => (
                    <tr key={i} className={l.started ? "" : "sub"}>
                      <td className="faint">{l.pos ?? ""}</td>
                      <td>
                        {l.player ? <EntityLink r={l.player}>{l.player.name}</EntityLink> : "Unknown"}
                        {l.player?.id === motm && <span title="Man of the match" className="motm"> ★</span>}
                        {l.yellows > 0 && <span className="cardchip yellow" title="Yellow card" />}
                        {l.reds > 0 && <span className="cardchip red" title="Red card" />}
                        {l.injured && <span className="tone-neg" title="Injured"> +</span>}
                      </td>
                      <td className="r num">{l.minutes}</td>
                      <td className={`r num ${l.rating >= 7.5 ? "tone-pos" : l.rating < 6 ? "tone-neg" : ""}`}>{l.rating ? l.rating.toFixed(1) : ""}</td>
                      <td className="r num">{l.goals || ""}</td>
                      <td className="r num">{l.assists || ""}</td>
                      <td className="r num">{l.shots || ""}</td>
                      <td className="r num">{l.passes ? `${l.passes_completed}/${l.passes}` : ""}</td>
                      <td className="r num">{l.tackles ? `${l.tackles_won}/${l.tackles}` : l.saves ? `${l.saves} sv` : ""}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Section>
        );
      })}
    </div>
  );
}

// ---- event map ------------------------------------------------------------------------------------------

function EventMap({ m, d, until }: { m: MatchResp; d: Detail; until: number }) {
  const [show, setShow] = useState<"key" | "shots" | "all">("shots");
  const evs = d.events.filter((e) => e.minute <= until && (show === "all" ? e.kind !== "KickOff" && e.kind !== "FullTime" && e.kind !== "HalfTime" : show === "shots" ? e.tier !== "minor" : e.tier === "key" && e.kind !== "KickOff" && e.kind !== "FullTime" && e.kind !== "HalfTime"));
  const W = 105;
  const H = 68;
  return (
    <div className="stack">
      <div className="map-tools">
        <Segmented label="Show" value={show} onChange={setShow} options={[{ id: "key", label: "Goals and cards" }, { id: "shots", label: "Shots" }, { id: "all", label: "Every recorded action" }]} />
        <span className="hint">Positions are approximate: the match is recorded by pitch zone, not by player tracking.</span>
      </div>
      <figure className="card map">
        <svg viewBox={`-2 -2 ${W + 4} ${H + 4}`} role="img" aria-label={`Event map for ${m.home.short} against ${m.away.short}. ${evs.length} events shown.`}>
          <rect x="0" y="0" width={W} height={H} rx="1.5" className="pm-line" />
          <line x1={W / 2} y1="0" x2={W / 2} y2={H} className="pm-line" />
          <circle cx={W / 2} cy={H / 2} r="9" className="pm-line" />
          <rect x="0" y={H / 2 - 20} width="16.5" height="40" className="pm-line" />
          <rect x={W - 16.5} y={H / 2 - 20} width="16.5" height="40" className="pm-line" />
          <rect x="0" y={H / 2 - 9} width="5.5" height="18" className="pm-line" />
          <rect x={W - 5.5} y={H / 2 - 9} width="5.5" height="18" className="pm-line" />
          {evs.map((e, i) => {
            const jx = ((i * 37) % 11) / 11 - 0.5;
            const jy = ((i * 53) % 13) / 13 - 0.5;
            const cx = e.x * W + jx * 5;
            const cy = e.y * H + jy * 8;
            const goal = e.kind === "Goal" || e.kind === "PenaltyGoal" || e.kind === "OwnGoal";
            const r = goal ? 2.1 : e.tier === "shot" ? 1.4 : e.tier === "key" ? 1.5 : 0.9;
            const cls = `mk ${e.side === 0 ? "home" : "away"} ${goal ? "goal" : ""}`;
            return (
              <circle key={i} cx={cx} cy={cy} r={r} className={cls}>
                <title>{`${e.minute}' ${e.label}${e.player ? ` — ${e.player.name}` : ""}${e.xg > 0.05 && e.tier === "shot" ? ` (xG ${e.xg.toFixed(2)})` : ""}`}</title>
              </circle>
            );
          })}
        </svg>
        <figcaption className="map-legend">
          <span><span className="mk-sample home" /> {m.home.short} attacks right</span>
          <span><span className="mk-sample away" /> {m.away.short} attacks left</span>
          <span className="faint">Larger marks are goals</span>
        </figcaption>
      </figure>
    </div>
  );
}

// ---- before the match --------------------------------------------------------------------------------------

function Preview({ m }: { m: MatchResp }) {
  const absent = (s: "home" | "away") => m.pre[s].absences;
  return (
    <div className="grid-2">
      {m.status !== "played" && <Section title="Before the match">
        <div className="card">
          {(["home", "away"] as const).map((s) => (
            <div key={s} className="pre-side">
              <div className="pre-title">{m[s].team.name}</div>
              {absent(s).length === 0 ? (
                <div className="muted">No one missing.</div>
              ) : (
                <ul className="rows compact">
                  {absent(s).map((a, i) => (
                    <li key={i}><EntityLink r={a.player}>{a.player.name}</EntityLink><span className="hint">{a.why}</span></li>
                  ))}
                </ul>
              )}
            </div>
          ))}
        </div>
      </Section>}
      <Section title="Previous meetings">
        <div className="card list-card">
          {m.previous.length === 0 ? (
            <div className="muted pad">These sides have not met in the records.</div>
          ) : (
            <ul className="rows">
              {m.previous.map((p) => (
                <li key={p.uid}>
                  <a href={href(`/match/${p.uid}`)}>{p.home} {p.score ?? "v"} {p.away}</a>
                  <span className="hint"><Dt d={p.date} /> · {p.comp}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </Section>
    </div>
  );
}

