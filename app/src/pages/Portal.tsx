import { ClubCrest } from "../components/Crest";
import { Insights } from "../components/Insights";
import { NewsFeed, StoryCard, WorldPulse } from "../components/Newsroom";
import { Dt, EntityLink } from "../components/links";
import { DEFAULT_TINT, Stage, StageHeader } from "../components/Stage";
import { tintOf } from "../color";
import { useClubColors } from "../crest";
import { dateLong, ordinal, relativeDays } from "../format";
import { href } from "../router";
import { act, notify, useApi, useStatus } from "../store";
import { Badge, Button, Meter, Section } from "../ui/ui";
import { Async, usePageTitle } from "./common";
import { OutcomeChip, type TodayResp } from "./Today";

interface InboxPreview {
  threads: { id: number; title: string; preview: string; unread: number; needs_action: boolean; last: number; kind: string }[];
  unread: number;
  awaiting: number;
}

export function Portal() {
  usePageTitle("Today");
  const today = useApi<TodayResp>("me.today");
  const news = useApi<NewsFeed>("news.feed", { filter: "for_you", limit: 7 });
  const inbox = useApi<InboxPreview>("me.inbox", { limit: 10 });
  const revision = useStatus().revision;
  const colors = useClubColors(today.data?.me.club?.id);
  return <Stage tint={colors ? tintOf(colors[0]) : DEFAULT_TINT}>
    <Async q={today}>{(t) => <>
      <StageHeader
        crest={t.me.club ? <ClubCrest id={t.me.club.id} name={t.me.club.name} size={58} /> : undefined}
        title="Today"
        sub={`${dateLong(t.date)} · ${t.me.name}${t.me.club ? ` at ${t.me.club.name}` : " · unattached"}`}
        meta={[
          ...(t.league ? [{ label: t.league.comp.name, value: <span>{ordinal(t.league.position)} <small>{t.league.points} pts</small></span> }] : []),
          ...(t.next_match ? [{ label: "Next match", value: <span>{t.next_match.home ? "v" : "at"} {t.next_match.opponent.name} <small>{relativeDays(t.next_match.date, t.date)}</small></span> }] : []),
        ]}
      />
      <div className="portal-intro world-arrive" key={revision}><span className="portal-live-dot" /> <span>YOUR FOOTBALL WORLD</span><strong>{t.decisions.length ? `${t.decisions.length} waiting for you` : t.day.label}</strong></div>
      <div className="stage-body portal-body">
        <div className="portal-grid">
          <div className="portal-attention stack">
            <Section title="Messages" aside={<a href={href("/messages")}>{inbox.data ? `${inbox.data.unread} unread · ${inbox.data.awaiting} actions` : "Open inbox"}</a>} tone="strong">
              {t.decisions.length > 0 && <div className="portal-decisions">{t.decisions.slice(0, 2).map((d) => <a key={d.id} href={href(`/messages/${d.id}`)}><Badge tone="warn">Decision</Badge><strong>{d.title}</strong><span>{d.summary}</span><small>Reply by <Dt d={d.deadline} year={false} /></small></a>)}</div>}
              {inbox.data?.threads.length ? <div className="portal-inbox-list">{inbox.data.threads.slice(0, 6).map((m) => <a key={m.id} href={href(`/messages/t/${m.id}`)} className={m.unread ? "unread" : ""}><span className="portal-inbox-row"><strong>{m.title}</strong><small><Dt d={m.last} year={false} /></small></span><span className="portal-inbox-preview">{m.preview}</span>{m.needs_action && <Badge tone="warn">Needs an answer</Badge>}</a>)}</div> : <div className="card muted">{inbox.error ? "Inbox unavailable." : "No conversations yet. People will reach you as the world moves."}</div>}
            </Section>
            <Section title="Today's plan">
              <div className="portal-plan">{t.commitments.map((c, i) => <div key={i}><span className="portal-plan-mark" />{c.text}</div>)}{t.queued.map((q, i) => <div key={`q${i}`} className="muted"><span className="portal-plan-mark queued" />{q} <small>Queued</small></div>)}</div>
            </Section>
          </div>
          <div className="portal-editorial stack">
            <Section title="Top story" aside={<a href={href("/news")}>World News</a>} tone="strong">
              {news.data?.stories[0] ? <StoryCard story={news.data.stories[0]} featured /> : <div className="card muted">{news.error ? "News unavailable." : "The next story is still unfolding."}</div>}
              {news.data && news.data.stories.length > 1 && <div className="portal-story-pair">{news.data.stories.slice(1, 3).map((s) => <StoryCard key={s.id} story={s} />)}</div>}
            </Section>
            <WorldPulse limit={7} />
            <Section title="Recent results" aside={t.unrevealed.length ? <Button size="sm" onClick={async () => { await act("match.reveal_all"); notify({ tone: "info", text: "All hidden results are now visible." }); }}>Show hidden results</Button> : <a href={href("/fixtures?show=results")}>All results</a>}>
              {t.recent.length ? <div className="portal-results">{t.recent.map((f) => <a key={f.uid} href={href(`/match/${f.uid}`)}><span>{f.concealed ? <Badge tone="muted">Hidden</Badge> : <OutcomeChip o={f.outcome} />}{f.home ? "v" : "at"} {f.opponent.name}</span><strong>{f.concealed ? "—" : f.score}</strong><small><Dt d={f.date} year={false} /></small></a>)}</div> : <div className="card muted">No matches played yet.</div>}
            </Section>
          </div>
          <aside className="portal-context stack">
            <Section title="Next match" tone="strong">
              {t.next_match ? <a className="portal-next" href={href(`/match/${t.next_match.uid}`)}><span className="hint">{t.next_match.comp.name} · {t.next_match.round}</span><strong>{t.next_match.home ? "v" : "at"} {t.next_match.opponent.name}</strong><span>{relativeDays(t.next_match.date, t.date)} <small>· <Dt d={t.next_match.date} year={false} /></small></span></a> : <div className="card muted">No match is scheduled.</div>}
              {t.next_match && <Insights method="insight.match" args={{ uid: t.next_match.uid }} title="What to watch" limit={2} compact hideEmpty />}
            </Section>
            <Section title="Your status" aside={<a href={href(`/person/${t.me.person}`)}>Profile</a>}>
              <div className="portal-status">{(["condition", "sharpness", "morale", "confidence", "wellbeing", "fatigue"] as const).map((k) => <div className="meter-row" key={k}><span>{k === "fatigue" ? "Legs" : k[0].toUpperCase() + k.slice(1)}</span><Meter value={k === "fatigue" ? 100 - t.condition[k].value : t.condition[k].value} label={t.condition[k].label} /></div>)}</div>
              {(t.availability.injured || t.availability.ban > 0) && <div className="portal-alert">{t.availability.injured ? `Injured: ${t.availability.injury}` : `Suspended for ${t.availability.ban} matches`}</div>}
            </Section>
            <Insights method="insight.person" args={{ id: t.me.person }} title="About your game" limit={2} compact hideEmpty />
            <Section title="Club and career">
              <div className="portal-career">{t.league && <div><span>League</span><strong><EntityLink r={t.league.comp}>{ordinal(t.league.position)} of {t.league.teams}</EntityLink></strong></div>}<div><span>Training</span><strong>{t.plan.intensity}</strong></div>{t.contract && <div><span>Contract ends</span><strong><Dt d={t.contract.end} /></strong></div>}</div>
            </Section>
          </aside>
        </div>
      </div>
    </>}</Async>
  </Stage>;
}
