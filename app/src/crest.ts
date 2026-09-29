import { useEffect, useSyncExternalStore } from "react";
import { call } from "./api";
import { useStatus } from "./store";

// Every club's badge colours, fetched once per world so any list can draw a crest.
let colors: Record<string, [string, string]> | null = null;
let inflight: Promise<void> | null = null;
let world = "";
const listeners = new Set<() => void>();

function load() {
  if (colors || inflight) return;
  inflight = call<{ colors: Record<string, [string, string]> }>("crest.colors", {})
    .then((r) => {
      colors = r.colors;
    })
    .catch(() => {
      colors = {};
    })
    .finally(() => {
      inflight = null;
      listeners.forEach((l) => l());
    });
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

/** The badge colours of a club, or undefined until they have loaded. */
export function useClubColors(id: number | null | undefined): [string, string] | undefined {
  const st = useStatus();
  const key = st.open ? `${st.name ?? ""}:${st.task?.running ? "busy" : "idle"}` : "";
  useEffect(() => {
    if (key !== world) {
      world = key;
      colors = null;
      inflight = null;
    }
    if (st.open && !st.task?.running) load();
  }, [key, st.open, st.task?.running]);
  const all = useSyncExternalStore(subscribe, () => colors);
  return id == null ? undefined : all?.[String(id)];
}
