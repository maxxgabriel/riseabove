// Wire types. The shared ones (references, cells, columns, tables, requests, errors) are GENERATED from the Rust contract
// (crates/pw-view/src/contract.rs -> contract.generated.ts); the rest still mirror what `pw-view` sends by hand.

import type { PerspectiveView as Perspective, Ref } from "./contract.generated";

export type {
  Cell,
  Col,
  ErrorKind,
  Fmt,
  Kind,
  Knowledge,
  Named,
  Part,
  Ref,
  Row,
  SortSpec,
  TableReq,
  TableResp,
  Tone,
} from "./contract.generated";

export interface StopInfo {
  kind: "target" | "user" | "decision" | "major" | "match" | "error" | string;
  text: string;
}

export interface Job {
  running: boolean;
  seq: number;
  label: string;
  from: number;
  target: number | null;
  days_done: number;
  days_total: number | null;
  stop: StopInfo | null;
  stop_requested: boolean;
  matches: number;
}

export interface TaskState {
  running: boolean;
  seq: number;
  label: string;
  error: string | null;
  report: Record<string, unknown> | null;
}

export type { Perspective };

export interface WorldSettings {
  conceal_mine: boolean;
  stops: { decisions: boolean; matches: boolean; major: boolean };
}

export interface Status {
  open: boolean;
  name?: string;
  date?: number;
  revision?: number;
  perspective?: Perspective;
  job: Job;
  task: TaskState;
  settings?: WorldSettings;
  awaiting?: number;
  unrevealed?: number;
  /** The world's own money: "₹" in a world of Indian regions, "£" elsewhere. */
  currency?: string;
}

export interface SaveInfo {
  file: string;
  size: number;
  modified: number | null;
  info: { name?: string; date?: number; players?: number; clubs?: number; perspective?: string; version?: string } | null;
  has_backup: boolean;
  /** Which save format the file is in and whether this build opens it. */
  format?: { schema: number | null; state: "current" | "upgradable" | "too_new" | "unsupported" | "unreadable"; note: string };
}

export interface SearchItem extends Ref {
  title: string;
  sub: string;
}
export interface SearchResp {
  groups: { label: string; items: SearchItem[] }[];
}

export interface Capability {
  area: string;
  status: "ready" | "partial" | "missing";
  note: string;
}
