import { useSyncExternalStore } from "react";
import { getStatus } from "./store";
import type { Kind } from "./types";

export interface Bookmark {
  k: Kind;
  id: number;
  title: string;
  sub?: string;
}

const listeners = new Set<() => void>();
const cache = new Map<string, Bookmark[]>();
const worldKey = () => `ra.bookmarks.${getStatus().name ?? ""}`;

function read(key: string): Bookmark[] {
  const hit = cache.get(key);
  if (hit) return hit;
  let v: Bookmark[] = [];
  try {
    v = JSON.parse(localStorage.getItem(key) ?? "[]") as Bookmark[];
  } catch {
    /* start empty */
  }
  cache.set(key, v);
  return v;
}

function write(key: string, v: Bookmark[]) {
  cache.set(key, v);
  try {
    localStorage.setItem(key, JSON.stringify(v));
  } catch {
    /* kept for this session only */
  }
  listeners.forEach((l) => l());
}

export function toggleBookmark(b: Bookmark) {
  const key = worldKey();
  const cur = read(key);
  const has = cur.some((x) => x.k === b.k && x.id === b.id);
  write(key, has ? cur.filter((x) => !(x.k === b.k && x.id === b.id)) : [...cur, b]);
}

export function useBookmarks(): Bookmark[] {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => read(worldKey()),
  );
}

export function useIsBookmarked(k: Kind, id: number): boolean {
  return useBookmarks().some((b) => b.k === k && b.id === id);
}
