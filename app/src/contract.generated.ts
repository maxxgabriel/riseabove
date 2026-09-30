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

/** How the viewer knows a cell's value when it is not simply known; the tags of `Knowledge`. */
export type CellKnow =
  | { kind: "unknown" }
  | { kind: "hidden" }
  | { kind: "range"; lo: number; hi: number }
  | { kind: "reported"; source: string };

export interface Cell {
  n?: number;
  s?: string;
  r?: Ref;
  tone?: Tone;
  k?: CellKnow;
  bar?: number;
  sub?: string;
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
export type InsightVisual =
  | { kind: "sparkline"; label: string; unit: string; values: number[] }
  | { kind: "sequence"; label: string; unit: string; values: string[] }
  | { kind: "comparison"; label: string; unit: string; names: string[]; values: number[] };

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

export interface AspectRow {
  label: string;
  level: string;
  note: string;
}

export interface DistrictView {
  available: boolean;
  reason: string | null;
  name: string;
  state: string;
  association: string | null;
  population_k: number;
  aspects: AspectRow[];
  academies: Named[];
  universities: string[];
  schools: number;
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

export interface ReferenceStatusRow {
  label: string;
  records: number;
}

export interface DerbyRow {
  name: string;
  a: string;
  b: string;
  kind: string;
  origin: string;
}

export interface ScenarioView {
  available: boolean;
  source: string;
  calendar: CalendarRow[];
  clubs_imported: number;
  clubs_seeded: number;
  clubs_generated: number;
  clubs_unknown: number;
  reference_loaded: boolean;
  reference_files: number;
  reference_records: number;
  reference_by_status: ReferenceStatusRow[];
  reference_findings: number;
  finding_samples: string[];
  clubs_matched: number;
  clubs_from_reference: number;
  derbies: DerbyRow[];
  note: string;
}

export interface ObserveReq {
  omniscient?: boolean | null;
  public?: boolean | null;
}

export interface InhabitReq {
  person: number;
}

export interface StopsReq {
  decisions?: boolean | null;
  matches?: boolean | null;
  major?: boolean | null;
}

export interface SettingsReq {
  conceal_mine?: boolean | null;
  stops?: StopsReq | null;
}

export interface SaveReq {
  file?: string | null;
}

export interface LoadReq {
  file: string;
  backup?: boolean | null;
}

export interface FileReq {
  file: string;
}

export interface AnswerReq {
  id: string;
  choice: number;
}

export interface ReplyReq {
  message: number;
  key: string;
}

export interface IdReq {
  id: number;
}

export interface GoalReq {
  kind?: string | null;
  target?: number | null;
  text?: string | null;
}

export interface GoalDoneReq {
  i: number;
  remove?: boolean | null;
}

export interface NoteReq {
  text?: string | null;
}

export interface IndexReq {
  i: number;
}

export interface FocusReq {
  kind?: string | null;
  value?: string | null;
}

export interface PlanReq {
  intensity?: string | null;
  extra?: number | null;
  recovery?: number | null;
  focus?: FocusReq | null;
}

export interface FollowReq {
  club: number;
  follow?: boolean | null;
}

export interface RevealReq {
  uid: number;
}

export interface RouteReq {
  start: string;
  district?: number | null;
  first?: string | null;
  last?: string | null;
}

export interface CreatePersonReq {
  first?: string | null;
  last?: string | null;
  age?: number | null;
  pos?: string | null;
  club?: number | null;
  nation?: number | null;
}

export interface RoutineHours {
  rest?: number | null;
  recovery?: number | null;
  family?: number | null;
  partner?: number | null;
  social?: number | null;
  study?: number | null;
  hobbies?: number | null;
  media?: number | null;
  nightlife?: number | null;
  language?: number | null;
}

export type ActReq =
  | { action: "meet"; with: number; topic: string; tone?: string | null }
  | { action: "transfer_request" }
  | { action: "withdraw_request" }
  | { action: "routine"; hours?: RoutineHours | null }
  | { action: "lifestyle"; value: string }
  | { action: "hire_agent"; agent: number }
  | { action: "drop_agent" }
  | { action: "retire" }
  | { action: "unretire" }
  | { action: "seek_job"; role: string }
  | { action: "dating"; open?: boolean | null }
  | { action: "partner"; ask: string }
  | { action: "amateur" }
  | { action: "nation"; nation: number }
  | { action: "retire_international" }
  | { action: "pain"; on?: boolean | null }
  | { action: "mentor"; person: number }
  | { action: "press"; about: number; stance: string }
  | { action: "enrol"; course: string }
  | { action: "move_home"; buy?: boolean | null; quality?: number | null }
  | { action: "helper"; helper: string; quality?: number | null }
  | { action: "dismiss_helper"; helper: string }
  | { action: "giving"; pct?: number | null; community?: number | null }
  | { action: "foundation" }
  | { action: "invest"; amount?: number | null; risk?: number | null }
  | { action: "career"; path: string }
  | { action: "leave_career" }
  | { action: "post"; concept: string; about?: number | null; reply_to?: number | null; quote_of?: number | null }
;

export interface Band {
  label: string;
  step: number;
  steps: number;
}

export interface Done {
  ok: boolean;
}

export interface Started {
  started: boolean;
}

export interface Saved {
  file: string;
}

export interface Closed {
  closed: boolean;
}

export interface Deleted {
  deleted: boolean;
}

export interface StopRequested {
  requested: boolean;
}

export interface Followed {
  followed: boolean;
}

export interface Revealed {
  revealed: number;
}

export interface RevealedAll {
  revealed: string;
}

export interface Created {
  person: number;
}

export interface FocusView {
  kind: string;
  value: string | null;
}

export interface PlanView {
  focus: FocusView;
  intensity: string;
  extra: number;
  recovery: number;
}

export interface PlanSet {
  plan: PlanView;
  applies: string;
}

export interface PromiseProgress {
  actual: number;
  promised: number;
}

export interface PromiseRow {
  id: number;
  mine: boolean;
  with: Named;
  text: string;
  made: number;
  due: number;
  progress: PromiseProgress | null;
  state: string;
  days_left: number;
}

export interface PromisesView {
  promises: PromiseRow[];
}

export interface GoalProgress {
  now: number;
  target: number;
}

export interface GoalRow {
  i: number;
  text: string;
  pinned: number;
  done: number | null;
  kind: string;
  progress: GoalProgress | null;
}

export interface NoteRow {
  i: number;
  date: number;
  text: string;
}

export interface InhabitedRow {
  who: Named;
  from: number;
  to: number | null;
}

export interface JournalView {
  goals: GoalRow[];
  notes: NoteRow[];
  history: InhabitedRow[];
}

export interface AgentRow {
  id: number;
  who: Named;
  fee_pct: number;
  since: number;
  until: number;
  satisfaction: Band;
  reputation: number;
  clients: number;
  base: string;
}

export interface AgentView {
  agent: AgentRow | null;
  player: boolean;
}

export interface OwnStoryView {
  id: number;
  date: number;
  outlet: string;
  headline: string;
  body: string;
}

export interface InsightItem {
  kind: string;
  tone: Tone;
  title: string;
  text: string;
  basis: string;
  link: Named | null;
  visual: InsightVisual | null;
}

export interface InsightsView {
  items: InsightItem[];
  total: number;
  held: number;
}

export interface ActDone {
  ok: boolean;
  text: string;
  applies: string;
}

/** Every method: whether it is a query or a command, and its request and response types where they are declared. */
export interface ApiMethods {
  "app.info": { kind: "query"; req: Record<string, unknown>; res: AppInfo };
  "world.status": { kind: "query"; req: Record<string, unknown>; res: StatusView };
  "world.new": { kind: "command"; req: Record<string, unknown>; res: Started };
  "world.inspect_import": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "world.datasets": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "world.saves": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "world.save": { kind: "command"; req: SaveReq; res: Saved };
  "world.load": { kind: "command"; req: LoadReq; res: Started };
  "world.close": { kind: "command"; req: Record<string, unknown>; res: Closed };
  "world.delete_save": { kind: "command"; req: FileReq; res: Deleted };
  "settings.set": { kind: "command"; req: SettingsReq; res: Done };
  "advance.start": { kind: "command"; req: Record<string, unknown>; res: StatusView };
  "advance.stop": { kind: "command"; req: Record<string, unknown>; res: StopRequested };
  "persp.observe": { kind: "command"; req: ObserveReq; res: Done };
  "persp.inhabit": { kind: "command"; req: InhabitReq; res: Done };
  "person.create": { kind: "command"; req: CreatePersonReq; res: Created };
  "route.options": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "route.begin": { kind: "command"; req: RouteReq; res: Created };
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
  "insight.club": { kind: "query"; req: Record<string, unknown>; res: InsightsView };
  "insight.comp": { kind: "query"; req: Record<string, unknown>; res: InsightsView };
  "insight.match": { kind: "query"; req: Record<string, unknown>; res: InsightsView };
  "insight.person": { kind: "query"; req: Record<string, unknown>; res: InsightsView };
  "club": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "club.systems": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "club.follow": { kind: "command"; req: FollowReq; res: Followed };
  "comp": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "nation": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "match": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "match.watch": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "match.reveal": { kind: "command"; req: RevealReq; res: Revealed };
  "match.reveal_all": { kind: "command"; req: Record<string, unknown>; res: RevealedAll };
  "me.today": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.viewed": { kind: "command"; req: Record<string, unknown>; res: Done };
  "me.messages": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.inbox": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.thread": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.thread_read": { kind: "command"; req: IdReq; res: Done };
  "me.reply": { kind: "command"; req: ReplyReq; res: ActDone };
  "me.message": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.answer": { kind: "command"; req: AnswerReq; res: Done };
  "me.act": { kind: "command"; req: ActReq; res: ActDone };
  "me.options": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.self": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.life": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "person.life": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.people": { kind: "query"; req: Record<string, unknown>; res: PeopleView };
  "me.promises": { kind: "query"; req: Record<string, unknown>; res: PromisesView };
  "me.rumours": { kind: "query"; req: Record<string, unknown>; res: RumoursView };
  "me.press": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.feed": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "social.thread": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.story": { kind: "query"; req: IdReq; res: OwnStoryView };
  "me.agent": { kind: "query"; req: Record<string, unknown>; res: AgentView };
  "me.journal": { kind: "query"; req: Record<string, unknown>; res: JournalView };
  "me.goal": { kind: "command"; req: GoalReq; res: Done };
  "me.goal_done": { kind: "command"; req: GoalDoneReq; res: Done };
  "me.note": { kind: "command"; req: NoteReq; res: Done };
  "me.note_remove": { kind: "command"; req: IndexReq; res: Done };
  "me.calendar": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.football": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "me.plan": { kind: "command"; req: PlanReq; res: PlanSet };
  "me.contract": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "pathway.player": { kind: "query"; req: PersonReq; res: PathwayView };
  "ecosystem.regions": { kind: "query"; req: Record<string, unknown>; res: RegionOutputView };
  "ecosystem.export": { kind: "query"; req: Record<string, unknown>; res: ExportView };
  "ecosystem.scenario": { kind: "query"; req: Record<string, unknown>; res: ScenarioView };
  "ecosystem.district": { kind: "query"; req: Record<string, unknown>; res: DistrictView };
  "database.sources": { kind: "query"; req: Record<string, unknown>; res: unknown };
  "database.attach": { kind: "command"; req: Record<string, unknown>; res: unknown };
  "database.query": { kind: "query"; req: Record<string, unknown>; res: unknown };
}

/** The methods whose response type is declared here. */
export type TypedMethod =
  | "app.info"
  | "world.status"
  | "world.new"
  | "world.save"
  | "world.load"
  | "world.close"
  | "world.delete_save"
  | "settings.set"
  | "advance.start"
  | "advance.stop"
  | "persp.observe"
  | "persp.inhabit"
  | "person.create"
  | "route.begin"
  | "table.query"
  | "person.attributes"
  | "insight.club"
  | "insight.comp"
  | "insight.match"
  | "insight.person"
  | "club.follow"
  | "match.reveal"
  | "match.reveal_all"
  | "me.viewed"
  | "me.thread_read"
  | "me.reply"
  | "me.answer"
  | "me.act"
  | "me.people"
  | "me.promises"
  | "me.rumours"
  | "me.story"
  | "me.agent"
  | "me.journal"
  | "me.goal"
  | "me.goal_done"
  | "me.note"
  | "me.note_remove"
  | "me.plan"
  | "pathway.player"
  | "ecosystem.regions"
  | "ecosystem.export"
  | "ecosystem.scenario"
  | "ecosystem.district"
;
