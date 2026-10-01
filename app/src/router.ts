import { useSyncExternalStore } from "react";
import type { Ref } from "./types";

export interface Route {
  path: string;
  segs: string[];
  query: URLSearchParams;
  /** The raw hash, used as a stable identity for scroll restoration and caching. */
  key: string;
}

const cache = new Map<string, Route>();

function parse(raw: string): Route {
  const hit = cache.get(raw);
  if (hit) return hit;
  const h = raw.replace(/^#/, "") || "/";
  const [p, q = ""] = h.split("?");
  const path = p.startsWith("/") ? p : `/${p}`;
  const route: Route = {
    path,
    segs: path.split("/").filter(Boolean).map(decodeURIComponent),
    query: new URLSearchParams(q),
    key: raw || "#/",
  };
  if (cache.size > 200) cache.clear();
  cache.set(raw, route);
  return route;
}

export function currentRoute(): Route {
  return parse(location.hash);
}

export function useRoute(): Route {
  return useSyncExternalStore(
    (l) => {
      window.addEventListener("hashchange", l);
      return () => window.removeEventListener("hashchange", l);
    },
    () => parse(location.hash),
  );
}

export function href(to: string): string {
  return `#${to.startsWith("/") ? to : `/${to}`}`;
}

export function navigate(to: string, opts: { replace?: boolean } = {}) {
  const target = href(to);
  if (target === location.hash) return;
  if (opts.replace) history.replaceState(null, "", target);
  else history.pushState(null, "", target);
  window.dispatchEvent(new HashChangeEvent("hashchange"));
}

/** Change query parameters on the current route without adding history entries. */
export function setQuery(patch: Record<string, string | number | boolean | null | undefined>, opts: { replace?: boolean } = { replace: true }) {
  const r = currentRoute();
  const q = new URLSearchParams(r.query);
  for (const [k, v] of Object.entries(patch)) {
    if (v == null || v === "" || v === false) q.delete(k);
    else q.set(k, v === true ? "1" : String(v));
  }
  const s = q.toString();
  navigate(s ? `${r.path}?${s}` : r.path, opts);
}

export function refPath(r: Ref): string {
  switch (r.k) {
    case "person":
      return `/person/${r.id}`;
    case "club":
      return `/club/${r.id}`;
    case "comp":
      return `/comp/${r.id}`;
    case "nation":
      return `/nation/${r.id}`;
    case "match":
      return `/match/${r.id}`;
    case "inst":
      return `/institution/${r.id}`;
    default:
      return `/club/${r.id}`;
  }
}

export const back = () => history.back();
export const forward = () => history.forward();
