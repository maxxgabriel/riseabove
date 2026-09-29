import { useEffect, useState } from "react";
import { DecisionCard, MeetingBlock, StoryCard, kindLabel, type DecisionDetail } from "../components/Decision";
import { Dt, EntityLink, Parts } from "../components/links";
import { prose } from "../format";
import { call } from "../api";
import { href, navigate, useRoute } from "../router";
import { act, notify, useApi } from "../store";
import type { Named, Part } from "../types";
import { Icon, type IconName } from "../ui/Icon";
import { Badge, Button, Empty, Tabs } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

// ---- conversations ----------------------------------------------------------------------------

interface ThreadRow {
  id: number;
  title: string;
  with: Named | null;
  kind: "person" | "press" | "post" | "club" | "decision";
  last: number;
  count: number;
  unread: number;
  needs_action: boolean;
  deadline: number | null;
  preview: string;
  last_kind: string;
}
interface Reply {
  key: string;
  label: string;
  effect: string | null;
  quiet: boolean;
}
interface PostView {
  id: number;
  date: number;
  author: { handle: string; display: string; kind: string; you: boolean; person: Named | null };
  text: string;
  likes: number;
  reposts: number;
  replies: number;
}
interface ThreadMessage {
  id: number;
  kind: "decision" | "tell" | "meeting" | "story" | "mention" | "private" | "question";
  date: number;
  from: Named | null;
  read: boolean;
  text: string;
  replied: { label: string; date: number } | null;
  replies: Reply[];
  decision?: { id: string; state: string; title: string };
  press?: { club: Named; date: number; number: number; of: number };
  answered_with?: string;
  confidence?: number;
  meeting?: DecisionDetail["meeting"];
  parts?: Part[];
  label?: string;
  story?: NonNullable<DecisionDetail["story"]>;
  post?: PostView;
}
interface ThreadResp {
  id: number;
  title: string;
  with: Named | null;
  kind: ThreadRow["kind"];
  opened: number;
  last: number;
  messages: ThreadMessage[];
}

const THREAD_ICON: Record<ThreadRow["kind"], IconName> = { person: "person", press: "mail", post: "pulse", club: "club", decision: "contract" };

const FILTERS = [
  { id: "all", label: "All" },
  { id: "action", label: "Needs an answer" },
  { id: "unread", label: "Unread" },
] as const;

export function Messages() {
  usePageTitle("Messages");
  const route = useRoute();
  const seg = route.segs[1];
  const activity = route.query.get("view") === "activity" || (seg != null && /^[de]\d+$/.test(seg));
  return (
    <div className="page fill inbox-page">
      <PageHead title="Messages" sub="Conversations with the people who reach you, and everything that has been decided or reported about you." />
      <Tabs
        label="Messages"
        value={activity ? "activity" : "conversations"}
        onChange={(v) => navigate(v === "activity" ? "/messages?view=activity" : "/messages")}
        tabs={[
          { id: "conversations", label: "Conversations" },
          { id: "activity", label: "All activity" },
        ]}
      />
      {activity ? <Activity /> : <Conversations />}
    </div>
  );
}

function Conversations() {
  const route = useRoute();
  const filter = route.query.get("filter") ?? "all";
  const selected = route.segs[1] === "t" ? Number(route.segs[2]) : null;
  const q = useApi<{ threads: ThreadRow[]; unread: number; awaiting: number }>("me.inbox");
  return (
    <Async q={q}>
      {(d) => {
        const list = d.threads.filter((t) => (filter === "action" ? t.needs_action : filter === "unread" ? t.unread > 0 : true))
          .sort((a, b) => Number(b.needs_action) - Number(a.needs_action) || Number(b.unread > 0) - Number(a.unread > 0) || b.last - a.last);
        const current = selected ?? list[0]?.id ?? null;
        return (
          <div className="inbox">
            <div className="inbox-list">
              <div className="inbox-summary"><span>INBOX</span><strong>{d.awaiting} need action</strong><small>{d.unread} unread</small></div>
              <div className="chips" role="group" aria-label="Show">
                {FILTERS.map((f) => {
                  const n = f.id === "action" ? d.awaiting : f.id === "unread" ? d.unread : 0;
                  return (
                    <button key={f.id} className="chip" aria-pressed={filter === f.id} onClick={() => navigate(`/messages${selected != null ? `/t/${selected}` : ""}${f.id === "all" ? "" : `?filter=${f.id}`}`, { replace: true })}>
                      {f.label}{n > 0 ? ` (${n})` : ""}
                    </button>
                  );
                })}
              </div>
              {list.length === 0 ? (
                <Empty title={filter === "all" ? "No conversations yet" : "Nothing here"} icon="mail">
                  {filter === "all" ? "People will get in touch as time passes: teammates, your manager, your agent, journalists, supporters. Offers and decisions arrive here too." : "Try another filter."}
                </Empty>
              ) : (
                <ul className="msglist" aria-label="Conversations">
                  {list.map((t) => (
                    <li key={t.id}>
                      <a href={href(`/messages/t/${t.id}${filter === "all" ? "" : `?filter=${filter}`}`)} aria-current={t.id === current ? "true" : undefined} className={`msg ${t.id === current ? "sel" : ""} ${t.needs_action ? "urgent" : ""}`}>
                        <div className="msg-top">
                          <strong className={`msg-subject ${t.unread > 0 ? "unread" : ""}`}>
                            <Icon name={THREAD_ICON[t.kind]} size={13} /> {t.title}
                          </strong>
                          <span className="hint"><Dt d={t.last} year={false} /></span>
                        </div>
                        <div className="msg-sub">
                          {t.needs_action && <Badge tone="warn">Needs an answer</Badge>}
                          {t.needs_action && t.deadline != null && <span className="hint">Due <Dt d={t.deadline} year={false} /></span>}
                          {t.unread > 0 && <Badge tone="you">{t.unread} new</Badge>}
                          {t.count > 1 && <span className="hint">{t.count} messages</span>}
                        </div>
                        <div className="msg-preview">{prose(t.preview)}</div>
                      </a>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div className="inbox-detail">{current != null ? <ThreadView id={current} key={current} onRead={q.reload} /> : <Empty title="No conversation selected" icon="mail" />}</div>
          </div>
        );
      }}
    </Async>
  );
}

function ThreadView({ id, onRead }: { id: number; onRead: () => void }) {
  const q = useApi<ThreadResp>("me.thread", { id });
  const unread = q.data?.messages.some((m) => !m.read) ?? false;
  useEffect(() => {
    if (!unread) return;
    const t = setTimeout(() => {
      void call("me.thread_read", { id }).then(onRead, () => undefined);
    }, 800);
    return () => clearTimeout(t);
  }, [unread, id, onRead]);
  return (
    <Async q={q}>
      {(t) => (
        <div className="thread-workspace">
          <div className="thread">
          <header className="thread-head">
            <h2>{t.with ? <EntityLink r={t.with}>{t.title}</EntityLink> : t.title}</h2>
            <div className="hint">Since <Dt d={t.opened} /> · {t.messages.length} {t.messages.length === 1 ? "message" : "messages"}</div>
          </header>
          <ol className="thread-msgs">
            {t.messages.map((m) => (
              <li key={m.id} className={m.read ? "" : "fresh"}>
                <MessageCard m={m} threadTitle={t.title} onDone={q.reload} />
              </li>
            ))}
          </ol>
          </div>
          <aside className="thread-context" aria-label="Conversation context">
            <span>CONVERSATION</span>
            <strong>{t.title}</strong>
            <small>{t.kind === "decision" ? "A decision awaits your response" : <>{t.messages.length} messages since <Dt d={t.opened} year={false} /></>}</small>
            {t.with && <EntityLink r={t.with}>Open {t.with.name}</EntityLink>}
          </aside>
        </div>
      )}
    </Async>
  );
}

const SOURCE_LABEL: Record<ThreadMessage["kind"], string> = {
  decision: "Needs your decision",
  question: "Press question",
  tell: "Told you",
  meeting: "Conversation",
  story: "In the press",
  mention: "Online",
  private: "Private",
};

function MessageCard({ m, onDone }: { m: ThreadMessage; threadTitle: string; onDone: () => void }) {
  const head = (
    <div className="mc-head">
      <span className="mc-kind">{SOURCE_LABEL[m.kind]}</span>
      {m.from && <span className="hint">from <EntityLink r={m.from}>{m.from.name}</EntityLink></span>}
      <span className="hint mc-date"><Dt d={m.date} /></span>
    </div>
  );
  if ((m.kind === "decision" || m.kind === "question") && m.decision) {
    return (
      <div className="mc">
        {head}
        <DecisionCard id={m.decision.id} showTitle={m.kind === "decision"} />
      </div>
    );
  }
  return (
    <div className="mc">
      {head}
      {m.kind === "story" && m.story ? (
        <StoryCard s={m.story} />
      ) : m.kind === "mention" && m.post ? (
        <div className="card post">
          <div className="post-head"><strong>{m.post.author.display}</strong> <span className="hint">@{m.post.author.handle} · {m.post.author.kind}</span></div>
          <p>{m.post.text}</p>
          <div className="hint">{m.post.replies} replies · {m.post.reposts} reposts · {m.post.likes} likes</div>
        </div>
      ) : m.kind === "meeting" && m.meeting ? (
        <>
          {m.parts && <p><Parts parts={m.parts} /></p>}
          <MeetingBlock m={m.meeting} />
        </>
      ) : m.parts ? (
        <p><Parts parts={m.parts} /></p>
      ) : (
        <p>{prose(m.text)}</p>
      )}
      {m.kind === "tell" && m.confidence != null && <div className="hint">You are about {m.confidence}% sure this is right. People pass things on imperfectly.</div>}
      {m.kind === "question" && m.answered_with && <div className="note"><Icon name="check" size={15} /><span>You answered: {m.answered_with.toLowerCase()}.</span></div>}
      {m.replied ? (
        <div className="note"><Icon name="check" size={15} /><span>You replied: {m.replied.label}. <span className="hint"><Dt d={m.replied.date} year={false} /></span></span></div>
      ) : (
        m.replies.length > 0 && <ReplyBar message={m} onDone={onDone} />
      )}
    </div>
  );
}

function ReplyBar({ message, onDone }: { message: ThreadMessage; onDone: () => void }) {
  const [busy, setBusy] = useState<string | null>(null);
  const send = async (r: Reply) => {
    setBusy(r.key);
    try {
      const res = await act<{ text: string; applies: string }>("me.reply", { message: message.id, key: r.key });
      notify({ tone: "pos", text: res.applies === "now" ? `${res.text}.` : `Queued: ${res.text}. The world acts on it when the day ends.` });
      onDone();
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    } finally {
      setBusy(null);
    }
  };
  return (
    <div className="replybar" role="group" aria-label="Reply">
      {message.replies.map((r) => (
        <Button key={r.key} size="sm" variant={r.quiet ? "ghost" : "default"} title={r.effect ?? undefined} disabled={busy != null} onClick={() => send(r)}>
          {r.label}
        </Button>
      ))}
    </div>
  );
}

// ---- all activity ------------------------------------------------------------------------------

interface Msg {
  id: string;
  kind: "decision" | "event";
  dkind?: string;
  date: number;
  subject: string;
  preview: string | null;
  parts?: Part[];
  from: Named | null;
  state: "awaiting" | "answered" | "settled" | "expired" | "info";
  folder: "awaiting" | "contracts" | "work" | "invitations" | "conversations" | "life" | "press";
  needs_action: boolean;
  deadline: number | null;
  unread?: boolean;
}

const FOLDERS = [
  { id: "all", label: "All" },
  { id: "awaiting", label: "Needs an answer" },
  { id: "contracts", label: "Contracts and moves" },
  { id: "conversations", label: "Conversations" },
  { id: "press", label: "Press" },
  { id: "life", label: "Life" },
  { id: "work", label: "Work" },
] as const;

const STATE_LABEL: Record<Msg["state"], { text: string; tone: "warn" | "pos" | "muted" | "info" | "neg" }> = {
  awaiting: { text: "Needs an answer", tone: "warn" },
  answered: { text: "Answered", tone: "info" },
  settled: { text: "Settled", tone: "pos" },
  expired: { text: "Expired", tone: "muted" },
  info: { text: "", tone: "muted" },
};

function Activity() {
  const route = useRoute();
  const sel = route.segs[1] && /^[de]\d+$/.test(route.segs[1]) ? route.segs[1] : undefined;
  const folder = route.query.get("folder") ?? "all";
  const q = useApi<{ messages: Msg[]; awaiting: number }>("me.messages");
  return (
    <Async q={q}>
      {(d) => {
        const list = d.messages.filter((m) => folder === "all" || m.folder === folder);
        const selected = sel ?? list[0]?.id;
        const qs = (f: string) => `?view=activity${f === "all" ? "" : `&folder=${f}`}`;
        return (
          <div className="inbox">
            <div className="inbox-list">
              <div className="chips" role="group" aria-label="Folders">
                {FOLDERS.map((f) => {
                  const n = f.id === "awaiting" ? d.awaiting : 0;
                  return (
                    <button key={f.id} className="chip" aria-pressed={folder === f.id} onClick={() => navigate(`/messages${sel ? `/${sel}` : ""}${qs(f.id)}`, { replace: true })}>
                      {f.label}{n > 0 ? ` (${n})` : ""}
                    </button>
                  );
                })}
              </div>
              {list.length === 0 ? (
                <Empty title="Nothing here" icon="mail">Offers and news about you will arrive as time passes.</Empty>
              ) : (
                <ul className="msglist" aria-label="Activity">
                  {list.map((m) => (
                    <li key={m.id}>
                      <a href={href(`/messages/${m.id}${qs(folder)}`)} aria-current={m.id === selected ? "true" : undefined} className={`msg ${m.id === selected ? "sel" : ""} ${m.needs_action ? "urgent" : ""}`}>
                        <div className="msg-top">
                          <strong className="msg-subject">{m.kind === "decision" ? kindLabel(m.dkind) : m.subject}</strong>
                          <span className="hint"><Dt d={m.date} year={false} /></span>
                        </div>
                        <div className="msg-sub">
                          {m.from ? m.from.name : ""}
                          {m.state !== "info" && <Badge tone={STATE_LABEL[m.state].tone}>{STATE_LABEL[m.state].text}</Badge>}
                        </div>
                        {m.preview && <div className="msg-preview">{prose(m.preview)}</div>}
                        {m.parts && <div className="msg-preview"><Parts parts={m.parts} /></div>}
                      </a>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div className="inbox-detail">{selected ? <ActivityDetail id={selected} key={selected} /> : <Empty title="No message selected" icon="mail" />}</div>
          </div>
        );
      }}
    </Async>
  );
}

function ActivityDetail({ id }: { id: string }) {
  if (id.startsWith("d")) return <DecisionCard id={id} />;
  return <EventDetail id={id} />;
}

function EventDetail({ id }: { id: string }) {
  const q = useApi<DecisionDetail>("me.message", { id });
  return (
    <Async q={q}>
      {(d) => (
        <article className="msg-article">
          <header>
            <h2>{d.title}</h2>
            {d.date != null && <div className="hint"><Dt d={d.date} /></div>}
          </header>
          <p className="msg-body">{d.parts && <Parts parts={d.parts} />}</p>
          {d.story && <StoryCard s={d.story} />}
          {d.meeting && <MeetingBlock m={d.meeting} />}
          {d.why && d.why.length > 0 && (
            <div className="hint">Why: {d.why.map(prose).join("; ")}</div>
          )}
        </article>
      )}
    </Async>
  );
}
