import { NewsFeed, StoryCard, StoryGraphic, storyPath, type NewsStory } from "../components/Newsroom";
import { Dt, EntityLink } from "../components/links";
import { href, navigate, useRoute } from "../router";
import { useApi } from "../store";
import { Badge, Empty, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

type NewsFilter = "for_you" | "following" | "world";
const filters: { id: NewsFilter; label: string }[] = [
  { id: "for_you", label: "For you" },
  { id: "following", label: "Following" },
  { id: "world", label: "World" },
];

export function News() {
  usePageTitle("World News");
  const route = useRoute();
  const filter = filters.find((f) => f.id === route.query.get("filter"))?.id ?? "for_you";
  const selected = route.segs[1] != null ? Number(route.segs[1]) : null;
  const q = useApi<NewsFeed>("news.feed", { filter, limit: 36 });
  return (
    <div className="page news-page">
      <PageHead title="World News" sub="The football world, as recorded and reported." />
      <Tabs label="News view" value={filter} tabs={filters} onChange={(id) => navigate(`/news${id === "for_you" ? "" : `?filter=${id}`}`)} />
      <Async q={q}>
        {(feed) => selected != null && Number.isInteger(selected) ? <NewsDetail id={selected} stories={feed.stories} /> : <NewsFront stories={feed.stories} filter={filter} />}
      </Async>
    </div>
  );
}

function NewsFront({ stories, filter }: { stories: NewsStory[]; filter: NewsFilter }) {
  if (!stories.length) return <Empty title={filter === "following" ? "No stories from followed clubs yet" : "No stories yet"} icon="mail">Football stories arrive as events unfold.</Empty>;
  const [lead, ...rest] = stories;
  return (
    <div className="news-front">
      <div className="news-main">
        <StoryCard story={lead} featured />
        <div className="news-card-grid">{rest.slice(0, 8).map((s) => <StoryCard key={s.id} story={s} />)}</div>
      </div>
      <aside className="news-briefs">
        <h2>Latest dispatches</h2>
        {rest.slice(8).map((s) => <a key={s.id} href={href(storyPath(s.id))}><span className="hint">{s.outlet} · <Dt d={s.date} year={false} /></span><strong>{s.headline}</strong></a>)}
        {rest.length <= 8 && <p className="muted">More stories will appear as the world advances.</p>}
      </aside>
    </div>
  );
}

function NewsDetail({ id, stories }: { id: number; stories: NewsStory[] }) {
  const q = useApi<NewsStory & { body: string }>("news.story", { id });
  return <Async q={q}>{(s) => (
    <div className="news-detail">
      <article>
        <a className="hint" href={href("/news")}>← All news</a>
        <StoryGraphic story={s} />
        <div className="editorial-meta"><span>{s.outlet}</span>{s.translated_from && <span className="hint">translated from {s.translated_from}</span>}<Dt d={s.date} /><Badge tone={s.claim === "rumour" || s.claim === "speculation" ? "warn" : "info"}>{s.claim}</Badge></div>
        <h1>{s.headline}</h1>
        <p>{s.body}</p>
        {s.subject && <p className="hint">About <EntityLink r={s.subject}>{s.subject.name}</EntityLink></p>}
      </article>
      <aside className="news-briefs"><h2>More news</h2>{stories.filter((other) => other.id !== id).slice(0, 8).map((other) => <a key={other.id} href={href(storyPath(other.id))}><span className="hint">{other.outlet}</span><strong>{other.headline}</strong></a>)}</aside>
    </div>
  )}</Async>;
}
