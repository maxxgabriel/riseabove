import { prose } from "./format";
// World state shared across the interface: the session status, the revision that
// tells views when to reload, and the small set of notices the shell shows.
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { ApiError, call } from "./api";
import { getSettings } from "./settings";
import type { Status, StopInfo } from "./types";

const idle: Status = {
  open: false,
  job: { running: false, seq: 0, label: "", from: 0, target: null, days_done: 0, days_total: null, stop: null, stop_requested: false, matches: 0 },
  task: { running: false, seq: 0, label: "", error: null, report: null },
};

let status: Status = idle;
let loaded = false;
let everLoaded = false;
let dataRev = 0;
let lastJobSeq = 0;
let notice: Notice | null = null;
let savedAt: { name: string; revision: number; at: number } | null = null;
let lastAutosave = 0;
const subs = new Set<() => void>();
const emit = () => subs.forEach((l) => l());

export interface Notice {
  id: number;
  tone: "info" | "pos" | "warn" | "neg";
  text: string;
  action?: { label: string; to?: string; run?: () => void };
}
let noticeId = 0;

export function notify(n: Omit<Notice, "id">) {
  notice = { ...n, text: prose(n.text), id: ++noticeId };
  emit();
}
export function dismissNotice() {
  notice = null;
  emit();
}

function subscribe(l: () => void) {
  subs.add(l);
  return () => subs.delete(l);
}

export const useStatus = () => useSyncExternalStore(subscribe, () => status);
export const useStatusLoaded = () => useSyncExternalStore(subscribe, () => loaded);
export const useNotice = () => useSyncExternalStore(subscribe, () => notice);
export const useDataRevision = () => useSyncExternalStore(subscribe, () => dataRev);
export const useDirty = () =>
  useSyncExternalStore(subscribe, () => {
    if (!status.open) return false;
    return !savedAt || savedAt.name !== status.name || savedAt.revision !== status.revision;
  });
export const getStatus = () => status;

let timer: ReturnType<typeof setTimeout> | null = null;
let lastDataBump = 0;

function stopText(s: StopInfo | null): string {
  return s?.text ?? "Stopped.";
}

/** Read the status once and decide whether to keep polling. */
export async function refreshStatus(): Promise<Status> {
  let next: Status;
  try {
    next = await call<Status>("world.status");
  } catch (e) {
    if (!loaded) {
      status = { ...idle, task: { ...idle.task, error: (e as ApiError).message } };
      loaded = true;
      emit();
    }
    schedule(2000);
    return status;
  }
  const prev = status;
  const first = !everLoaded;
  everLoaded = true;
  status = next;
  loaded = true;

  // A job that finished since we last looked leaves a notice explaining why it stopped.
  if (first) lastJobSeq = next.job.seq;
  if (next.job.seq < lastJobSeq) lastJobSeq = next.job.seq;
  if (next.job.seq > lastJobSeq && !next.job.running) {
    lastJobSeq = next.job.seq;
    const st = next.job.stop;
    if (st && st.kind !== "user") {
      const target = st.kind === "decision" ? "/messages" : st.kind === "match" ? "/today" : undefined;
      notify({
        tone: st.kind === "error" ? "neg" : st.kind === "target" ? "info" : "warn",
        text: stopText(st),
        action: target ? { label: st.kind === "decision" ? "Open messages" : "Open today", to: target } : undefined,
      });
    }
    maybeAutosave();
  }

  const now = Date.now();
  if (next.revision !== prev.revision || next.name !== prev.name) {
    if (!next.job.running || now - lastDataBump > 700) {
      dataRev += 1;
      lastDataBump = now;
    }
  } else if (!next.job.running && prev.job.running) {
    dataRev += 1;
  }
  emit();
  if (next.job.running || next.task.running) schedule(next.job.running ? 200 : 300);
  return next;
}

function schedule(ms: number) {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void refreshStatus(), ms);
}

export function markSaved() {
  if (status.open && status.name != null && status.revision != null) {
    savedAt = { name: status.name, revision: status.revision, at: Date.now() };
    emit();
  }
}

function maybeAutosave() {
  if (!getSettings().autosave || !status.open) return;
  if (Date.now() - lastAutosave < 3 * 60_000) return;
  lastAutosave = Date.now();
  void call("world.save", { file: `autosave ${status.name ?? ""}` }).catch(() => undefined);
}

/** Call a method that changes the world, then bring the status up to date. */
export async function act<T = unknown>(method: string, args: Record<string, unknown> = {}): Promise<T> {
  const r = await call<T>(method, args);
  await refreshStatus();
  return r;
}

export async function save(file?: string): Promise<string> {
  const r = await call<{ file: string }>("world.save", file ? { file } : {});
  await refreshStatus();
  markSaved();
  return r.file;
}

/** After loading or creating a world the store forgets what it knew. */
export function worldChanged() {
  dataRev += 1;
  savedAt = null;
  emit();
}

export function startStatus() {
  void refreshStatus();
}

// ---- hooks -----------------------------------------------------------------------------------

/** Re-run when `deps` change or the world moves on. Old data stays visible while new data loads. */
export function useApi<T>(method: string | null, args: Record<string, unknown> = {}, opts: { live?: boolean } = {}) {
  const rev = useDataRevision();
  // A page that is leaving can render once more against the next route; a NaN id then is not worth a request.
  const usable = method && !Object.values(args).some((v) => typeof v === "number" && !Number.isFinite(v));
  const key = usable ? `${method}:${JSON.stringify(args)}` : "";
  const live = opts.live !== false;
  const [state, setState] = useState<{ key: string; rev: number; data?: T; error?: ApiError }>({ key: "", rev: -1 });
  const seq = useRef(0);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    if (!usable || !method) return;
    const mine = ++seq.current;
    call<T>(method, args)
      .then((data) => {
        if (seq.current === mine) setState({ key, rev, data });
      })
      .catch((error: ApiError) => {
        if (seq.current === mine) setState({ key, rev, error });
      });
    return () => {
      seq.current++;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, live ? rev : 0, tick]);

  const same = state.key === key;
  return {
    data: same ? state.data : undefined,
    error: same ? state.error : undefined,
    loading: !same || (state.rev !== rev && live),
    reload: () => setTick((t) => t + 1),
  };
}

/** Several calls of the same method at once; the result is keyed by the JSON of each argument set. */
export function useApiMany<T>(method: string, list: Record<string, unknown>[]) {
  const rev = useDataRevision();
  const key = `${method}:${JSON.stringify(list)}`;
  const [state, setState] = useState<{ key: string; data?: T[]; error?: ApiError }>({ key: "" });
  useEffect(() => {
    let live = true;
    if (list.length === 0) {
      setState({ key, data: [] });
      return;
    }
    Promise.all(list.map((a) => call<T>(method, a)))
      .then((data) => live && setState({ key, data }))
      .catch((error: ApiError) => live && setState({ key, error }));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, rev]);
  const same = state.key === key;
  return { data: same ? state.data : undefined, error: same ? state.error : undefined };
}
