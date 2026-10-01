import { ClubCrest } from "./Crest";
import { Dt, EntityLink, Parts } from "./links";
import { initials } from "../format";
import { href, refPath } from "../router";
import { useApi } from "../store";
import type { NewsFeedView, StorySummary } from "../contract.generated";
import type { Part, Ref } from "../types";
import { Badge, Empty, Section } from "../ui/ui";

export type NewsStory = StorySummary;
export type NewsFeed = NewsFeedView;
export interface PulseItem { id: number; date: number; label: string; parts: Part[]; target: Ref | { k: "news"; id: number } }

const kindText = (s: string) => s.replace(/([a-z])([A-Z])/g, "$1 $2");
export const storyPath = (id: number) => `/news/${id}`;

export function StoryGraphic({ story, compact = false }: { story: NewsStory; compact?: boolean }) {
  const g = story.graphic;
  return (
    <div className={`story-graphic ${compact ? "compact" : ""} graphic-${g.kind}`} aria-hidden="true">
      <div className="graphic-grid" />
      {g.kind === "result" ? (
        <div className="graphic-score">
          <span><ClubCrest id={g.home.id} name={g.home.name} size={compact ? 35 : 56} /><b>{g.home.name}</b></span>
          <strong>{g.score[0]}<i>:</i>{g.score[1]}</strong>
          <span><ClubCrest id={g.away.id} name={g.away.name} size={compact ? 35 : 56} /><b>{g.away.name}</b></span>
        </div>
      ) : g.kind === "club" ? (
        <div className="graphic-identity"><ClubCrest id={g.club.id} name={g.club.name} size={compact ? 48 : 78} /><strong>{g.club.name}</strong></div>
      ) : g.kind === "person" ? (
        <div className="graphic-identity"><span className="graphic-monogram">{initials(g.person.name)}</span><strong>{g.person.name}</strong></div>
      ) : (
        <div className="graphic-identity"><span className="graphic-monogram">RA</span><strong>Rise Above</strong></div>
      )}
      <span className="graphic-kind">{kindText(story.kind)}</span>
    </div>
  );
}

export function StoryCard({ story, featured = false }: { story: NewsStory; featured?: boolean }) {
  return (
    <article className={`editorial-card ${featured ? "featured" : ""}`}>
      <StoryGraphic story={story} compact={!featured} />
      <div className="editorial-copy">
        <div className="editorial-meta"><span>{story.outlet}</span>{story.translated_from && <span className="hint">translated from {story.translated_from}</span>}<span>·</span><Dt d={story.date} year={false} /><Badge tone={story.claim === "rumour" || story.claim === "speculation" ? "warn" : "info"}>{story.claim}</Badge></div>
        <h3><a href={href(storyPath(story.id))}>{story.headline}</a></h3>
        <div className="editorial-foot"><span>{kindText(story.kind)}</span>{story.subject && <EntityLink r={story.subject}>{story.subject.name}</EntityLink>}{story.about_you && <Badge tone="you">About you</Badge>}</div>
      </div>
    </article>
  );
}

export function WorldPulse({ limit = 8, title = "World pulse" }: { limit?: number; title?: string }) {
  const q = useApi<{ items: PulseItem[] }>("world.pulse", { limit });
  return (
    <Section title={title} aside={<a href={href("/news")}>News</a>}>
      {q.data?.items.length ? (
        <ol className="pulse-list">
          {q.data.items.map((item) => {
            const to = item.target.k === "news" ? storyPath(item.target.id) : refPath(item.target);
            return <li key={item.id}><span className="pulse-dot" /><div><div className="pulse-label">{item.label} <span className="hint"><Dt d={item.date} year={false} /></span></div><a className="pulse-link" href={href(to)}><Parts parts={item.parts} /></a></div></li>;
          })}
        </ol>
      ) : q.error ? <div className="card muted">World updates are unavailable.</div> : q.data ? <Empty title="A quiet day" icon="pulse">New events will appear here when the world moves on.</Empty> : <div className="card muted">Loading world updates…</div>}
    </Section>
  );
}
