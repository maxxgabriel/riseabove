import { EntityLink } from "../components/links";
import { toggleBookmark, useBookmarks } from "../bookmarks";
import { act, notify, useApi } from "../store";
import type { Kind, Named } from "../types";
import { Button, Empty, IconButton, Section } from "../ui/ui";
import { PageHead, usePageTitle } from "./common";

const GROUPS: { k: Kind; title: string }[] = [
  { k: "person", title: "People" },
  { k: "club", title: "Clubs" },
  { k: "comp", title: "Competitions" },
  { k: "nation", title: "Nations" },
];

export function Bookmarks() {
  usePageTitle("Bookmarks");
  const marks = useBookmarks();
  const ov = useApi<{ followed: Named[] }>("overview");
  const followed = ov.data?.followed ?? [];
  const unfollow = async (c: Named) => {
    try {
      await act("club.follow", { club: c.id, follow: false });
      ov.reload();
    } catch (e) {
      notify({ tone: "neg", text: (e as Error).message });
    }
  };
  return (
    <div className="page narrow">
      <PageHead title="Bookmarks" sub="Things you have marked to come back to, and clubs whose matches are recorded in full." />
      {marks.length === 0 && followed.length === 0 && (
        <Empty title="Nothing bookmarked yet" icon="bookmark">Use the bookmark button on any person, club or competition to keep it here.</Empty>
      )}
      {GROUPS.map((g) => {
        const items = marks.filter((m) => m.k === g.k);
        if (items.length === 0) return null;
        return (
          <Section key={g.k} title={g.title}>
            <div className="card list-card">
              <ul className="rows">
                {items.map((m) => (
                  <li key={m.id}>
                    <span><EntityLink r={m}>{m.title}</EntityLink> {m.sub && <span className="hint">{m.sub}</span>}</span>
                    <IconButton icon="x" label={`Remove ${m.title}`} onClick={() => toggleBookmark(m)} />
                  </li>
                ))}
              </ul>
            </div>
          </Section>
        );
      })}
      {followed.length > 0 && (
        <Section title="Followed clubs" aside="Full match details are kept for these clubs">
          <div className="card list-card">
            <ul className="rows">
              {followed.map((c) => (
                <li key={c.id}>
                  <EntityLink r={c}>{c.name}</EntityLink>
                  <Button size="sm" variant="ghost" onClick={() => unfollow(c)}>Unfollow</Button>
                </li>
              ))}
            </ul>
          </div>
        </Section>
      )}
    </div>
  );
}
