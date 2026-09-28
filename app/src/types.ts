// Wire types. These mirror what `pw-view` sends; nothing here is computed on the client.

export type Kind = "person" | "club" | "comp" | "nation" | "match" | "team";
export interface Ref {
  k: Kind;
  id: number;
}
export interface Named extends Ref {
  name: string;
}
export type Tone = "pos" | "neg" | "warn" | "muted" | "info";

/** A run of a sentence: plain text, a link, a money amount or a date. */
export interface Part {
  t: string;
  r?: Ref;
  m?: number;
  d?: number;
}

export interface Cell {
  n?: number;
  s?: string;
  r?: Ref;
  tone?: Tone;
  u?: boolean;
  bar?: number;
  sub?: string;
  range?: [number, number];
  parts?: Part[];
}

export type Fmt = "text" | "int" | "dec1" | "dec2" | "money" | "date" | "pct" | "ordinal";

export interface Col {
  key: string;
  label: string;
  short: string;
  fmt: Fmt;
  align: "left" | "right" | "center";
  w: number;
  presets: string[];
  sortable: boolean;
  help: string;
}

export interface Row {
  id: number;
  cells: Cell[];
  open?: Ref;
  tone?: Tone;
}

export interface TableResp {
  all_columns: Col[];
  columns: string[];
  presets: string[];
  total: number;
  offset: number;
  rows: Row[];
  note?: string;
  revision: number;
  sort: [string, boolean] | null;
}

export interface SortSpec {
  key: string;
  desc: boolean;
}

export interface TableReq {
  table: string;
  filters?: Record<string, unknown>;
  sort?: SortSpec | null;
  offset?: number;
  limit?: number;
  columns?: string[] | null;
  preset?: string | null;
}

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

export type Perspective =
  | { mode: "observer" }
  | { mode: "inhabit"; person: number; name: string; club: string | null };

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
}

export interface SaveInfo {
  file: string;
  size: number;
  modified: number | null;
  info: { name?: string; date?: number; players?: number; clubs?: number; perspective?: string; version?: string } | null;
  has_backup: boolean;
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
