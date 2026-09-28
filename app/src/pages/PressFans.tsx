import { useState } from "react";
import { SpeakDialog } from "../components/Actions";
import { Dt, EntityLink } from "../components/links";
import { StoryCard } from "../components/Decision";
import { prose } from "../format";
import { useApi } from "../store";
import type { Named } from "../types";
import { Badge, Button, Dialog, Empty, Section } from "../ui/ui";
import { Async, PageHead, usePageTitle } from "./common";

interface PressResp {
  stories: { id: number; date: number; outlet: string; headline: string; about_you: boolean }[];
  reactions: { date: number; text: string; sentiment: number }[];
  fans: { club: Named; label: string; score: number; reasons: string[] }[];
  quotes: { id: number; date: number; stance: string; about: Named | null; at_conference: boolean }[];
  image: "positive" | "neutral" | "negative";
}

export function PressFans() {
  usePageTitle("Press and fans");
  const q = useApi<PressResp>("me.press");
  const me = useApi<{ me: Named & { person: number } }>("me.today");
  const [speak, setSpeak] = useState(false);
  const [story, setStory] = useState<number | null>(null);
  return (
    <div className="page">
      <PageHead
        title="Press and fans"
        sub="What is written about you, what supporters think, and what you have said on the record."
        actions={me.data && <Button variant="primary" onClick={() => setSpeak(true)}>Speak to the press</Button>}
      />
      <Async q={q}>
        {(d) => (
          <div className="split">
            <div className="stack">
              <Section title="In the papers">
                <div className="card list-card">
                  {d.stories.length === 0 ? (
                    <div className="muted pad">Nothing has been written about you or your club lately.</div>
                  ) : (
                    <ul className="rows">
                      {d.stories.map((s) => (
                        <li key={s.id}>
                          <div className="grow">
                            <button className="linkbtn story-link" onClick={() => setStory(s.id)}>{prose(s.headline)}</button>
                            <div className="hint">{s.outlet} · <Dt d={s.date} year={false} /></div>
                          </div>
                          {s.about_you && <Badge tone="you">About you</Badge>}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </Section>
              <Section title="Around your club" aside="What supporters and others are saying">
                <div className="card list-card">
                  {d.reactions.length === 0 ? (
                    <div className="muted pad">Quiet.</div>
                  ) : (
                    <ul className="rows">
                      {d.reactions.map((r, i) => (
                        <li key={i}>
                          <span>{prose(r.text)}</span>
                          <span className="hint"><Dt d={r.date} year={false} /></span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </Section>
            </div>
            <aside className="stack">
              <Section title="Supporters">
                <div className="card">
                  {d.fans.length === 0 ? (
                    <div className="muted">No supporters have formed an opinion of you yet.</div>
                  ) : (
                    <ul className="rows compact">
                      {d.fans.map((f) => (
                        <li key={f.club.id}>
                          <div>
                            <EntityLink r={f.club}>{f.club.name}</EntityLink>
                            {f.reasons.length > 0 && <div className="hint">Because of {f.reasons.join(", ")}</div>}
                          </div>
                          <Badge tone={f.score > 150 ? "pos" : f.score < -150 ? "neg" : "info"}>{f.label}</Badge>
                        </li>
                      ))}
                    </ul>
                  )}
                  <p className="hint pt">In the press you come across as {d.image === "positive" ? "well regarded" : d.image === "negative" ? "poorly regarded" : "neither loved nor hated"}.</p>
                </div>
              </Section>
              <Section title="What you have said">
                <div className="card list-card">
                  {d.quotes.length === 0 ? (
                    <div className="muted pad">You have not spoken on the record.</div>
                  ) : (
                    <ul className="rows">
                      {d.quotes.map((q) => (
                        <li key={q.id}>
                          <span>
                            {q.stance}
                            {q.about ? <> about <EntityLink r={q.about}>{q.about.name}</EntityLink></> : ""}
                            {q.at_conference && <span className="hint"> · at a press conference</span>}
                          </span>
                          <span className="hint"><Dt d={q.date} year={false} /></span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </Section>
            </aside>
          </div>
        )}
      </Async>
      {me.data && <SpeakDialog open={speak} onClose={() => setSpeak(false)} about={{ k: "person", id: me.data.me.person, name: "yourself" }} />}
      <StoryDialog id={story} onClose={() => setStory(null)} />
    </div>
  );
}

function StoryDialog({ id, onClose }: { id: number | null; onClose: () => void }) {
  const q = useApi<{ id: number; date: number; outlet: string; headline: string; body: string }>(id != null ? "me.story" : null, { id: id ?? 0 });
  return (
    <Dialog open={id != null} onClose={onClose} title="Story" width={560}>
      {q.data ? <StoryCard s={q.data} /> : <Empty title="Loading" />}
    </Dialog>
  );
}
