import { useState } from "react";
import { PostDialog } from "../components/Actions";
import { Dt, EntityLink } from "../components/links";
import { useApi } from "../store";
import type { Named } from "../types";
import { Icon } from "../ui/Icon";
import { Badge, Button, Empty, Section } from "../ui/ui";
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

export function PostCard({ p, onReply, onQuote, nested = false }: { p: PostView; onReply?: (p: PostView) => void; onQuote?: (p: PostView) => void; nested?: boolean }) {
  return (
    <article className={`post ${nested ? "nested" : ""}`}>
      <div className="post-head">
        <strong>{p.author.person ? <EntityLink r={p.author.person}>{p.author.display}</EntityLink> : p.author.display}</strong>
        <span className="hint">@{p.author.handle}</span>
        {p.author.you ? <Badge tone="you">You</Badge> : <Badge>{p.author.kind}</Badge>}
        <span className="hint post-date"><Dt d={p.date} year={false} /></span>
      </div>
      {p.parent && <PostCard p={p.parent} nested />}
      <p>{p.text}</p>
      {p.quoted && <PostCard p={p.quoted} nested />}
      {p.about && !nested && <div className="hint">About <EntityLink r={p.about}>{p.about.name}</EntityLink></div>}
      {!nested && (
        <div className="post-foot">
          <Count n={p.replies} label="replies" />
          <Count n={p.reposts} label="reposts" />
          <Count n={p.likes} label="likes" />
          <span className="grow" />
          {onReply && <Button size="sm" variant="ghost" onClick={() => onReply(p)}>Reply</Button>}
          {onQuote && <Button size="sm" variant="ghost" onClick={() => onQuote(p)}>Quote</Button>}
        </div>
      )}
    </article>
  );
}

export function Social() {
  usePageTitle("Social");
  const q = useApi<{ posts: PostView[]; account: { handle: string; followers: number } | null }>("me.feed", { limit: 40 });
  const [compose, setCompose] = useState<{ reply?: PostView; quote?: PostView } | null>(null);
  const target = compose?.reply ?? compose?.quote;
  return (
    <div className="page narrow">
      <PageHead
        title="Social"
        sub="A small part of what people are saying, from the accounts you follow and the ones that mention you."
        actions={<Button variant="primary" icon="plus" onClick={() => setCompose({})}>New post</Button>}
      />
      <Async q={q}>
        {(d) => (
          <div className="stack">
            <Section>
              <div className="card">
                {d.account ? (
                  <div className="iconrow"><Icon name="person" size={15} /> You post as <strong>@{d.account.handle}</strong> · <span className="num">{d.account.followers.toLocaleString()}</span> followers</div>
                ) : (
                  <div className="muted">You have no account yet. Your first post opens one in your name.</div>
                )}
                <p className="hint pt">You choose what you are doing, not the words. Posts go out when the day ends, and people answer in their own way.</p>
              </div>
            </Section>
            {d.posts.length === 0 ? (
              <Empty title="Nothing to read yet" icon="pulse">Posts appear as matches are played and things happen around you.</Empty>
            ) : (
              <div className="card list-card">
                <ul className="feedlist">
                  {d.posts.map((p) => (
                    <li key={p.id}><PostCard p={p} onReply={(x) => setCompose({ reply: x })} onQuote={(x) => setCompose({ quote: x })} /></li>
                  ))}
                </ul>
              </div>
            )}
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
