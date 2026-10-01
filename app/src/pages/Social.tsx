import { useState } from "react";
import { PostDialog } from "../components/Actions";
import { Dt, EntityLink } from "../components/links";
import { WorldPulse } from "../components/Newsroom";
import { initials } from "../format";
import { href, useRoute } from "../router";
import { useApi } from "../store";
import type { Named } from "../types";
import { Badge, Button, Empty } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

export interface PostView {
  id: number;
  date: number;
  author: { handle: string; display: string; kind: string; followers: number; you: boolean; person: Named | null };
  text: string;
  about: Named | null;
  likes: number;
  reposts: number;
  replies: number;
  parent: PostView | null;
  quoted: PostView | null;
}

function Count({ n, label }: { n: number; label: string }) {
  return <span className="num" title={`${n} ${label}`}>{n.toLocaleString()} <span className="faint">{label}</span></span>;
}

function Initials({ name }: { name: string }) {
  return <span className="social-avatar" aria-hidden="true">{initials(name)}</span>;
}

export function PostCard({ p, onReply, onQuote, nested = false }: { p: PostView; onReply?: (p: PostView) => void; onQuote?: (p: PostView) => void; nested?: boolean }) {
  return (
    <article className={`post social-post ${nested ? "nested" : ""}`}>
      <Initials name={p.author.display} />
      <div className="social-post-body"><div className="post-head">
        <strong>{p.author.person ? <EntityLink r={p.author.person}>{p.author.display}</EntityLink> : p.author.display}</strong>
        <span className="hint">@{p.author.handle}</span>
        {p.author.you ? <Badge tone="you">You</Badge> : <Badge>{p.author.kind}</Badge>}
        <span className="hint post-date"><Dt d={p.date} year={false} /></span>
      </div>
      {p.parent && <div className="social-reference"><span>Replying to</span><PostCard p={p.parent} nested /></div>}
      <p>{p.text}</p>
      {p.quoted && <div className="social-reference"><span>Quoting</span><PostCard p={p.quoted} nested /></div>}
      {p.about && !nested && <div className="hint">About <EntityLink r={p.about}>{p.about.name}</EntityLink></div>}
      {!nested && (
        <div className="post-foot">
          <Count n={p.replies} label="replies" />
          <Count n={p.reposts} label="reposts" />
          <Count n={p.likes} label="likes" />
          <span className="grow" />
          <a href={href(`/social/post/${p.id}`)}>Open thread</a>
          {onReply && <Button size="sm" variant="ghost" onClick={() => onReply(p)}>Reply</Button>}
          {onQuote && <Button size="sm" variant="ghost" onClick={() => onQuote(p)}>Quote</Button>}
        </div>
      )}
      </div>
    </article>
  );
}

export function Social() {
  usePageTitle("Social");
  const route = useRoute();
  const selected = route.segs[1] === "post" ? Number(route.segs[2]) : null;
  const q = useApi<{ posts: PostView[]; account: { handle: string; followers: number } | null }>("me.feed", { limit: 40 });
  const [compose, setCompose] = useState<{ reply?: PostView; quote?: PostView } | null>(null);
  const target = compose?.reply ?? compose?.quote;
  return (
    <div className="page social-page">
      <PageHead
        title="Social"
        sub="The football conversation around you."
        actions={<Button variant="primary" icon="plus" onClick={() => setCompose({})}>New post</Button>}
      />
      <Async q={q}>
        {(d) => (
          <div className="social-layout">
            <div className="social-stream">
            {selected != null ? <SocialThread id={selected} onReply={(p) => setCompose({ reply: p })} onQuote={(p) => setCompose({ quote: p })} /> : <>
            <div className="social-stream-head"><strong>Latest posts</strong><span>{d.posts.length} in your feed</span></div>
            {d.posts.length === 0 ? (
              <Empty title="Nothing to read yet" icon="pulse">Posts appear as matches are played and things happen around you.</Empty>
            ) : (
              <div className="social-feed">
                <ul>
                  {d.posts.map((p) => (
                    <li key={p.id}><PostCard p={p} onReply={(x) => setCompose({ reply: x })} onQuote={(x) => setCompose({ quote: x })} /></li>
                  ))}
                </ul>
              </div>
            )}
            </>}
            </div>
            <aside className="social-aside">
              <div className="social-account"><span>YOUR ACCOUNT</span>{d.account ? <><strong>@{d.account.handle}</strong><small>{d.account.followers.toLocaleString()} followers</small></> : <p>Your first post opens an account in your name.</p>}</div>
              <WorldPulse limit={5} />
            </aside>
          </div>
        )}
      </Async>
      <PostDialog
        open={compose != null}
        onClose={() => setCompose(null)}
        replyTo={compose?.reply?.id}
        quoteOf={compose?.quote?.id}
        about={target?.about ?? null}
        context={target && <PostCard p={target} nested />}
      />
    </div>
  );
}

function SocialThread({ id, onReply, onQuote }: { id: number; onReply: (p: PostView) => void; onQuote: (p: PostView) => void }) {
  const q = useApi<{ post: PostView; replies: PostView[] }>("social.thread", { id });
  return <Async q={q}>{(d) => <>
    <div className="social-stream-head"><a href={href("/social")}>← Feed</a><strong>Conversation</strong><span>{d.replies.length} replies</span></div>
    <PostCard p={d.post} onReply={onReply} onQuote={onQuote} />
    <div className="social-reply-head">Replies</div>
    {d.replies.length ? d.replies.map((p) => <PostCard key={p.id} p={p} onReply={onReply} onQuote={onQuote} />) : <p className="muted">No replies yet.</p>}
  </>}</Async>;
}
