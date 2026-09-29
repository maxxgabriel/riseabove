// GENERATED from crates/pw-view/src/contract.rs. Do not edit by hand.
// Regenerate with: UPDATE_CONTRACT=1 cargo test -p pw-view --test contract

export type ErrorKind =
  | "not_found"
  | "unauthorized_perspective"
  | "invalid_request"
  | "state_conflict"
  | "unavailable_information"
  | "save_incompatible"
  | "simulation_busy"
  | "internal_error"
;

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

export type Knowledge<T> =
  | { kind: "exact"; v: T }
  | { kind: "range"; lo: T; hi: T; v: T }
  | { kind: "reported"; v: T; source: string }
  | { kind: "unknown" }
  | { kind: "hidden" };
export type AttrRow = { key: string; label: string } & Knowledge<number>;
export type PerspectiveView =
  | { mode: "observer"; omniscient: boolean }
  | { mode: "public"; omniscient: boolean }
  | { mode: "inhabit"; person: number; name: string; club: string | null };

export interface ErrorBody {
  kind: ErrorKind;
  code: string;
  message: string;
  retryable: boolean;
}

export interface AppInfo {
  name: string;
  version: string;
  data_dir: string;
}

export interface StopInfo {
  kind: string;
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

export interface Task {
  running: boolean;
  seq: number;
  label: string;
  error: string | null;
  report: unknown | null;
}

export interface StopsView {
  decisions: boolean;
  matches: boolean;
  major: boolean;
}

export interface SettingsView {
  conceal_mine: boolean;
  stops: StopsView;
}

export interface StatusView {
  open: boolean;
  name: string | null;
  date: number | null;
  revision: number | null;
  perspective: PerspectiveView | null;
  job: Job;
  task: Task;
  settings: SettingsView | null;
  awaiting: number | null;
  unrevealed: number | null;
}

export interface PositionView {
  code: string;
  fam: number;
  level: string;
}

export interface AttrGroupView {
  name: string;
  attrs: AttrRow[];
}

export interface HiddenAttr {
  label: string;
  v: number;
}

export interface InternalAbility {
  ca: number;
  pa: number;
}

export interface AttributesView {
  available: boolean;
  reason: string | null;
  source: string;
  known: boolean;
  groups: AttrGroupView[];
  positions: PositionView[];
  hidden: HiddenAttr[] | null;
  internal: InternalAbility | null;
  personality: string | null;
}

export interface Evidence {
  text: string;
  date: number;
}

export interface RelationshipRow {
  who: Named;
  role: string;
  label: string;
  tone: Tone;
  trust: string;
  respect: string;
  since: number;
  last: number;
  why: Evidence | null;
  evidence: Evidence[];
}

export interface PeopleView {
  people: RelationshipRow[];
}

export interface RumourRow {
  kind: string;
  date: number;
  via: string;
  sureness: string;
  text: string;
  club: Named | null;
  fee: number | null;
}

export interface RumoursView {
  rumours: RumourRow[];
}

export interface PersonReq {
  id: number;
}

export interface StepRow {
  date: number;
  kind: string;
  target: string | null;
  why: string | null;
  recorded: boolean;
}

export interface CreationView {
  date: number | null;
  region: string;
  provider: string;
  institution: string | null;
  age: number | null;
  first_env: string;
  first_finder: Named | null;
  why: string;
  provenance: string;
}

export interface EvidenceRow {
  rule: string;
  holds: boolean;
  detail: number | null;
}

export interface EligibilityRow {
  body_kind: string;
  body: string;
  eligible: boolean;
  reason: string;
  evidence: EvidenceRow[];
}

export interface TierRow {
  level: string;
  games: number;
  proof: string;
}

export interface KnownBy {
  org: string;
  looks: number;
  years: number;
  how: string;
  first: number;
  last: number;
  first_by: string | null;
}

export interface VouchView {
  from: string;
  basis: string;
  strength: string;
  credibility: string;
  date: number;
}

export interface WatchRow {
  org: string;
  by: string;
  games_left: number;
  since: number;
}

export interface RecognitionView {
  standing: string;
  evidence: TierRow[];
  known_by: KnownBy[];
  vouch: VouchView | null;
  watching: WatchRow[];
  buzz: boolean;
}

export interface PathwayView {
  available: boolean;
  reason: string | null;
  player: Named;
  steps: StepRow[];
  creation: CreationView | null;
  eligibility: EligibilityRow[];
  recognition: RecognitionView | null;
}

export interface RegionOutputRow {
  region: string;
  kind: string;
  professionals: number;
  top_tier: number;
  internationals: number;
  senior_apps: number;
  value: number;
}

export interface RegionOutputView {
  available: boolean;
  rows: RegionOutputRow[];
  note: string;
}

export interface SegmentRegard {
  segment: string;
  level: number;
  exports: number;
  successes: number;
  visits: number;
  provenance: string;
}

export interface MarketRow {
  name: string;
  nations: string[];
  segments: SegmentRegard[];
}

export interface ExportView {
  available: boolean;
  markets: MarketRow[];
  note: string;
}

export interface CalendarRow {
  event: string;
  when: string;
}

export interface ScenarioView {
  available: boolean;
  source: string;
  calendar: CalendarRow[];
  clubs_imported: number;
  clubs_seeded: number;
  clubs_generated: number;
  clubs_unknown: number;
  note: string;
}

/** Every method: whether it is a query or a command, and its request and response types where they are declared. */
export interface ApiMethods {
  "app.info": { kind: "query"; req: Record<string, unknown>; res: AppInfo };
  "world.status": { kind: "query"; req: Record<string, unknown>; res: StatusView };
  "world.new": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "world.inspect_import": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "world.saves": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "world.save": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "world.load": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "world.close": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "world.delete_save": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "settings.set": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "advance.start": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "advance.stop": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "persp.observe": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "persp.inhabit": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "person.create": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "route.options": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "route.begin": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "table.query": { kind: "query"; req: TableReq; res: TableResp };
  "search": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "overview": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "world.pulse": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "news.feed": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "news.story": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "diagnostics": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "capabilities": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "person": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "person.attributes": { kind: "query"; req: PersonReq; res: AttributesView };
  "crest.colors": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "comp.overview": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "insight.club": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "insight.comp": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "insight.match": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "insight.person": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "club": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "club.systems": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "club.follow": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "comp": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "nation": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "match": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "match.watch": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "match.reveal": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "match.reveal_all": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.today": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.viewed": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.messages": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.inbox": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.thread": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.thread_read": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.reply": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.message": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.answer": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.act": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.options": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.self": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.life": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "person.life": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.people": { kind: "query"; req: Record<string, unknown>; res: PeopleView };
  "me.promises": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.rumours": { kind: "query"; req: Record<string, unknown>; res: RumoursView };
  "me.press": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.feed": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "social.thread": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.story": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.agent": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.journal": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.goal": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.goal_done": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.note": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.note_remove": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.calendar": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.football": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.plan": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "me.contract": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "pathway.player": { kind: "query"; req: PersonReq; res: PathwayView };
  "ecosystem.regions": { kind: "query"; req: Record<string, unknown>; res: RegionOutputView };
  "ecosystem.export": { kind: "query"; req: Record<string, unknown>; res: ExportView };
  "ecosystem.scenario": { kind: "query"; req: Record<string, unknown>; res: ScenarioView };
}

/** The methods whose response type is declared here. */
export type TypedMethod =
  | "app.info"
  | "world.status"
  | "table.query"
  | "person.attributes"
  | "me.people"
  | "me.rumours"
  | "pathway.player"
  | "ecosystem.regions"
  | "ecosystem.export"
  | "ecosystem.scenario"
;
