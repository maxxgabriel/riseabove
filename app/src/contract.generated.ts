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
export type StoryGraphic =
  | { kind: "result"; match: Ref; home: Named; away: Named; score: number[] }
  | { kind: "club"; club: Named }
  | { kind: "person"; person: Named }
  | { kind: "type" };

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
  currency: string | null;
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
  shared: Part[];
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

export interface LabelRow {
  name: string;
  detail: string;
  note: string;
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
  associations: LabelRow[];
  press: LabelRow[];
  broadcasters: LabelRow[];
  institutions_real: number;
  programmes: LabelRow[];
  partnerships: LabelRow[];
  coaching_ladder: LabelRow[];
  referee_ladder: LabelRow[];
  representative_sides: LabelRow[];
  rules: LabelRow[];
  languages: LabelRow[];
  note: string;
}

export interface PersonReq {
  id: number;
}

export interface PersonOptReq {
  id?: number | null;
}

export interface DistrictReq {
  id?: number | null;
}

export interface CompOverviewReq {
  id: number;
  light?: boolean | null;
}

export interface InsightReq {
  id: number;
  limit?: number | null;
}

export interface InsightMatchReq {
  uid: number;
  limit?: number | null;
}

export interface MatchReq {
  uid: number;
}

export interface MessageReq {
  id: string;
}

export interface LimitReq {
  limit?: number | null;
}

export interface NewsFeedReq {
  filter?: string | null;
  limit?: number | null;
}

export interface SearchReq {
  q?: string | null;
  limit?: number | null;
}

export interface CalendarReq {
  from?: number | null;
  to?: number | null;
}

export interface DirReq {
  dir: string;
}

export interface DatabaseQueryReq {
  source?: number | null;
  table: string;
  column?: string | null;
  value?: string | null;
  search?: string | null;
  offset?: number | null;
  limit?: number | null;
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

export interface ChronicleEntry {
  date: number;
  cat: string;
  parts: Part[];
  uid: number | null;
  story: number | null;
  learned: number | null;
  keepsake: string | null;
}

export interface ChronicleTie {
  who: Named;
  how: Part[];
  from: number;
  to: number;
  now: string | null;
}

export interface ChronicleReach {
  layer: string;
  first: number | null;
  outlet: string | null;
  stories: number;
}

export interface ChronicleView {
  person: Named;
  since: number;
  entries: ChronicleEntry[];
  people: ChronicleTie[];
  reach: ChronicleReach[];
  born: number;
}

export interface TrainingWeekRow {
  date: number;
  group: string;
  week: string;
  coach: Named | null;
  traces: Part[][];
}

export interface TrainingLogView {
  weeks: TrainingWeekRow[];
}

export interface PayslipRow {
  date: number;
  wage: number;
  other: number;
  tax: number;
  net: number;
  living: number;
  family: number;
  left: number;
  bonuses: number;
}

export interface BonusRow {
  date: number;
  what: string;
  gross: number;
  kept: number;
}

export interface MoneyView {
  per_week: number;
  months: PayslipRow[];
  bonuses: BonusRow[];
  meaning: string[];
}

export interface MatchdayStep {
  when: string;
  kind: string;
  parts: Part[];
}

export interface MatchdayView {
  uid: number;
  played: boolean;
  steps: MatchdayStep[];
}

export interface ChatRoomRow {
  id: number;
  kind: string;
  title: string;
  with: Named | null;
  last: number | null;
  preview: string;
  unread: number;
}

export interface ChatsView {
  rooms: ChatRoomRow[];
}

export interface ChatLine {
  date: number;
  from: string;
  who: Named | null;
  mine: boolean;
  text: Part[];
}

export interface ChatView {
  id: number;
  kind: string;
  title: string;
  lines: ChatLine[];
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

export interface StorySummary {
  id: number;
  date: number;
  outlet: string;
  headline: string;
  kind: string;
  claim: string;
  about_you: boolean;
  following: boolean;
  graphic: StoryGraphic;
  subject: Named | null;
  translated_from: string | null;
}

export interface StoryFull {
  id: number;
  date: number;
  outlet: string;
  headline: string;
  kind: string;
  claim: string;
  about_you: boolean;
  following: boolean;
  graphic: StoryGraphic;
  subject: Named | null;
  translated_from: string | null;
  body: string;
}

export interface NewsFeedView {
  stories: StorySummary[];
  filter: string;
}

export interface ActDone {
  ok: boolean;
  text: string;
  applies: string;
}

export interface DecisionOptionView {
  i: number;
  label: string;
  kind: string;
  positive: boolean;
  default: boolean;
  consequence?: string;
  effect?: string;
  tone?: string;
  counter?: unknown;
}

export interface ThreadDecision {
  id: string;
  state: string;
  title: string;
  deadline: number;
  options: DecisionOptionView[];
  kind: string;
}

export interface DecisionDefault {
  i: number;
  label: string;
}

export interface DecisionDetailView {
  id: string;
  kind: string;
  dkind: string;
  title: string;
  from: unknown;
  created: number;
  deadline: number;
  state: string;
  paragraphs: string[];
  options: DecisionOptionView[];
  answer: number | null;
  default: DecisionDefault;
  without_response: string | null;
  consequences: string[];
  terms: unknown;
  current_terms: unknown;
  talk: unknown;
  meeting: unknown;
  incident: unknown;
  press: unknown;
  outcome: string | null;
}

/** `me.message`: an event, or a decision in full. */
export type MeMessage = MeMessageView | DecisionDetailView;

export interface PulseTarget {
  k: string;
  id: number;
}

export interface DatasetsView {
  datasets: DatasetRow[];
}

export interface DatasetRow {
  path: string;
  database: unknown;
}

export interface InspectImportView {
  ok: boolean;
  files: ImportFile[];
  counts?: ImportCounts;
  warnings?: string[];
  findings?: ImportFinding[];
  start?: number | null;
  database?: unknown;
  error?: string;
}

export interface ImportFile {
  name: string;
  size: number;
}

export interface ImportCounts {
  nations: number;
  competitions: number;
  clubs: number;
  players: number;
  staff: number;
  unresolved: number;
}

export interface ImportFinding {
  code: string;
  count: number;
}

export interface DatabaseSourcesView {
  sources: DatabaseSource[];
  errors: string[];
}

export interface DatabaseSource {
  id: number;
  path: string;
  tables: DatabaseTable[];
}

export interface DatabaseTable {
  name: string;
  bytes: number;
  source: string;
  status: string;
}

export interface DatabaseAttached {
  id: number;
  sources: DatabaseSource[];
}

export interface DatabaseQueryView {
  table: string;
  columns: string[];
  indexed_columns: string[];
  rows: DatabaseRecord[];
  total: number;
  matched: number;
  malformed: number;
  offset: number;
  name_search: boolean;
  source: number;
}

export interface DatabaseRecord {
  row: number;
  fields: Record<string, string | null>;
  refs?: Record<string, Ref>;
}

export interface WorldSavesViewSaveFormat {
  note: string;
  schema: number;
  state: string;
}

export interface WorldSavesViewSaveInfo {
  clubs: number;
  date: number;
  name: string;
  perspective: string;
  players: number;
  version: string;
}

export interface WorldSavesViewSave {
  file: string;
  format: WorldSavesViewSaveFormat | null;
  has_backup: boolean;
  info: WorldSavesViewSaveInfo;
  modified: number;
  size: number;
}

export interface WorldSavesView {
  dir: string;
  saves: WorldSavesViewSave[];
}

export interface RouteOptionsViewStart {
  age: number;
  blurb: string;
  key: string;
  label: string;
}

export interface RouteOptionsViewStateDistrict {
  id: number;
  name: string;
  population_k: number;
}

export interface RouteOptionsViewState {
  districts: RouteOptionsViewStateDistrict[];
  id: number;
  name: string;
}

export interface RouteOptionsView {
  available: boolean;
  starts: RouteOptionsViewStart[];
  states: RouteOptionsViewState[];
}

export interface SearchViewGroupItem {
  id: number;
  k: string;
  sub: string;
  title: string;
}

export interface SearchViewGroup {
  items: SearchViewGroupItem[];
  label: string;
}

export interface SearchView {
  groups: SearchViewGroup[];
}

export interface OverviewViewCounts {
  clubs: number;
  competitions: number;
  nations: number;
  players: number;
}

export interface OverviewViewLeagueLeader {
  played: number;
  points: number;
  team: Named;
}

export interface OverviewViewLeague {
  comp: Named;
  leader: OverviewViewLeagueLeader | null;
  stage: string;
}

export interface OverviewViewRecent {
  date: number;
  kind: string;
  parts: Part[];
}

export interface OverviewViewUpcoming {
  away: Named;
  comp: string;
  date: number;
  home: Named;
  uid: number;
}

export interface OverviewView {
  counts: OverviewViewCounts | null;
  date: number;
  followed: Named[];
  leagues: OverviewViewLeague[];
  name: string;
  recent: OverviewViewRecent[];
  upcoming: OverviewViewUpcoming[];
}

export interface WorldPulseViewItem {
  date: number;
  id: number;
  label: string;
  parts: Part[];
  target: PulseTarget;
}

export interface WorldPulseView {
  items: WorldPulseViewItem[];
}

export interface Capability {
  area: string;
  note: string;
  status: string;
}

export interface DiagnosticsViewTimings {
  avg_ms: number;
  recent: number[][];
  samples: number;
  worst_ms: number;
}

export interface DiagnosticsViewWorld {
  clubs: number;
  competitions: number;
  date: number;
  days_simulated: number;
  decisions: number;
  events: number;
  fixtures: number;
  followed: number;
  name: string;
  people: number;
  players: number;
  reports: number;
  seed: number;
  staff: number;
  teams: number;
}

export interface DiagnosticsView {
  capabilities: Capability[];
  perspective: string;
  revision: number;
  timings: DiagnosticsViewTimings;
  version: string;
  world: DiagnosticsViewWorld;
}

export interface PersonViewPlayerAvailability {
  detail: string;
  label: string;
  tone: string;
}

export interface PersonViewPlayerCondition {
  condition: number;
  confidence: number;
  fatigue: number;
  fitness: number;
  morale: number;
  sharpness: number;
  wellbeing: number;
}

export interface PersonViewPlayerContract {
  appearance_bonus: number;
  assist_bonus: number;
  cap_bonus: number;
  clean_sheet_bonus: number;
  club: Named;
  continental_bonus: number;
  days_left: number;
  end: number;
  goal_bonus: number;
  kind: string;
  loyalty_bonus: number;
  options: unknown[];
  promised_status: unknown;
  promotion_bonus: number;
  release_clause: number;
  relegation_cut: number;
  relegation_release: number;
  start: number;
  title_bonus: number;
  wage: number;
  yearly_rise: number;
}

export interface PersonViewPlayerInternalPlan {
  focus: string;
  intensity: string;
}

export interface PersonViewPlayerInternalReputation {
  current: number;
  home: number;
  world: number;
}

export interface PersonViewPlayerInternal {
  bio_offset: number;
  ca: number;
  pa: number;
  personality: string;
  plan: PersonViewPlayerInternalPlan | null;
  reputation: PersonViewPlayerInternalReputation;
}

export interface PersonViewPlayerLoan {
  club: Named;
  end: number;
  parent: Named;
}

export interface PersonViewPlayerPosition {
  code: string;
  fam: number;
  level: string;
}

export interface PersonViewPlayer {
  availability: PersonViewPlayerAvailability;
  best_pos: string;
  caps: number;
  career_coverage: string;
  condition: PersonViewPlayerCondition | null;
  contract: PersonViewPlayerContract | null;
  foot: string;
  form: number[];
  hidden: string[];
  height: number;
  internal: PersonViewPlayerInternal | null;
  intl_goals: number;
  joined: number | null;
  loan: PersonViewPlayerLoan | null;
  player_id: number;
  positions: PersonViewPlayerPosition[];
  senior_apps: number | null;
  senior_goals: number | null;
  shirt: number;
  squad_status: string | null;
  team: string | null;
  traits: string[];
  unknown: string[];
  value: number | null;
  weight: number;
  youth_club: Named | null;
}

export interface PersonViewRole {
  label: string;
  org: Named | null;
}

export interface PersonViewStaffAttr {
  label: string;
  v: number;
}

export interface PersonViewStaffContract {
  end: number;
  wage: number;
}

export interface PersonViewStaffRecord {
  draws: number;
  games: number;
  losses: number;
  sackings: number;
  trophies: number;
  wins: number;
}

export interface PersonViewStaffStyle {
  archetype: string;
  directness: number;
  formations: string[];
  mentality: number;
  press: number;
  tempo: number;
}

export interface PersonViewStaff {
  attrs: PersonViewStaffAttr[] | null;
  club: Named | null;
  contract: PersonViewStaffContract | null;
  joined: number;
  record: PersonViewStaffRecord;
  reputation: number;
  role: string;
  role_rating: number | null;
  staff_id: number;
  style: PersonViewStaffStyle | null;
}

export interface PersonView {
  age: number;
  can_inhabit: boolean;
  dob: number;
  id: number;
  initials: string;
  is_me: boolean;
  name: string;
  nations: Named[];
  perspective: string;
  player: PersonViewPlayer | null;
  provenance: unknown;
  roles: PersonViewRole[];
  short: string;
  staff: PersonViewStaff | null;
  status: string;
}

export interface CrestColorsView {
  colors: Record<string, string[]>;
}

export interface CompOverviewViewHeld {
  results: number;
  withheld: unknown[];
}

export interface CompOverviewViewLeftRowTeam {
  colors: string[];
  full: string;
  id: number;
  k: string;
  me: boolean;
  name: string;
}

export interface CompOverviewViewLeftRow {
  gd: number;
  played: number;
  points: number;
  pos: number;
  team: CompOverviewViewLeftRowTeam | null;
  zone: string | null;
}

export interface CompOverviewViewLeftTy {
  a: CompOverviewViewLeftRowTeam;
  b: CompOverviewViewLeftRowTeam;
  goals_a: number | null;
  goals_b: number | null;
  hidden: boolean;
  legs: number;
  played: number;
  winner: string | null;
}

export interface CompOverviewViewLeft {
  date?: number | null;
  kind: string;
  round?: string;
  rows?: CompOverviewViewLeftRow[];
  shown?: number;
  ties?: CompOverviewViewLeftTy[];
  title: string;
  total: number;
}

export interface CompOverviewViewMeta {
  label: string;
  ref?: Named;
  sub?: string;
  value: unknown;
}

export interface CompOverviewViewNew {
  away: CompOverviewViewLeftRowTeam | null;
  date: number;
  days_ago: number;
  headline: string;
  home: CompOverviewViewLeftRowTeam;
  match: Ref;
  outlet: string;
  score: number[];
  spoils: boolean;
}

export interface CompOverviewViewPlayerRow {
  p: Named;
  pill: boolean;
  team: CompOverviewViewLeftRowTeam | null;
  value: string;
}

export interface CompOverviewViewPlayer {
  page: number;
  rows: CompOverviewViewPlayerRow[];
  title: string;
}

export interface CompOverviewViewTeamsStatRow {
  team: CompOverviewViewLeftRowTeam | null;
  value: string;
}

export interface CompOverviewViewTeamsStat {
  page: number;
  rows: CompOverviewViewTeamsStatRow[];
  title: string;
}

export interface CompOverviewViewTicker {
  as: number | null;
  away: CompOverviewViewLeftRowTeam | null;
  date: number;
  home: CompOverviewViewLeftRowTeam;
  hs: number | null;
  pens: number[] | null;
  round: number;
  status: string;
  uid: number;
}

export interface CompOverviewView {
  held: CompOverviewViewHeld;
  id: number;
  kind: string;
  kind_key: string;
  left: CompOverviewViewLeft;
  meta: CompOverviewViewMeta[];
  name: string;
  news: CompOverviewViewNew[];
  next: Named | null;
  players: CompOverviewViewPlayer[];
  prev: Named | null;
  season: string;
  short: string;
  stage: string;
  teams: number;
  teams_stats: CompOverviewViewTeamsStat[];
  ticker: CompOverviewViewTicker[];
  tier: number;
}

export interface ClubViewAcademy {
  age_groups: string[];
  kind: string;
  name: string;
  origin: string;
  residential: boolean | null;
}

export interface ClubViewAlsoKnown {
  kind: string;
  origin: string;
  text: string;
}

export interface ClubViewBoard {
  patience: number;
  satisfaction: number;
  target_position: number;
  warnings: number;
}

export interface ClubViewChannel {
  name: string;
  real: boolean;
}

export interface ClubViewFacilities {
  academy: number;
  medical: number;
  training: number;
  youth: number;
}

export interface ClubViewFinance {
  balance: number;
  debt: number;
  season_income: number;
  season_spend: number;
  transfer_budget: number;
  wage_bill: number;
  wage_budget: number;
}

export interface ClubViewLeague {
  comp: Named;
  played: number | null;
  points: number | null;
  position: number | null;
  teams: number;
}

export interface ClubViewManagerRecord {
  draws: number;
  games: number;
  losses: number;
  wins: number;
}

export interface ClubViewManager {
  person: Named;
  record: ClubViewManagerRecord;
  since: number;
}

export interface ClubViewNeed {
  max_age: number;
  min_ability: number;
  pos: string;
  urgency: number;
}

export interface ClubViewPartner {
  active: boolean;
  origin: string;
  purpose: string;
  what: string;
  with: string;
}

export interface ClubViewStaffCount {
  count: number;
  role: string;
}

export interface ClubViewTeam {
  captain: Named | null;
  comp: Named | null;
  kind: string;
  kind_key: string;
  squad: number;
  team: number;
}

export interface ClubViewPlace {
  region: string;
  state: string | null;
  climate: string;
  language: string | null;
  population: string;
  football: string;
  nearby: Named[];
  universities: string[];
  from_home: string | null;
  training: string;
  ground: string | null;
}

export interface ClubView {
  academy: ClubViewAcademy | null;
  place: ClubViewPlace | null;
  also_known: ClubViewAlsoKnown[];
  board: ClubViewBoard | null;
  capacity: number;
  channels: ClubViewChannel[];
  city: string;
  colors: string[];
  facilities: ClubViewFacilities | null;
  fan_mood: number;
  finance: ClubViewFinance | null;
  followed: boolean;
  founded: number;
  id: number;
  league: ClubViewLeague | null;
  manager: ClubViewManager | null;
  name: string;
  nation: Named;
  needs: ClubViewNeed[] | null;
  ownership: string;
  partners: ClubViewPartner[];
  relation: string;
  reputation: number;
  short: string;
  stadium: string;
  staff_counts: ClubViewStaffCount[];
  teams: ClubViewTeam[];
}

export interface ClubSystemsViewBoardConcern {
  label: string;
  value: number;
}

export interface ClubSystemsViewBoardOwnerTraits {
  ambition: number;
  fan_sensitivity: number;
  frugality: number;
  meddling: number;
  patience: number;
  wealth: number;
}

export interface ClubSystemsViewBoardPolicy {
  debt_tolerance: number;
  max_signing_age: number;
  sell_to_rivals: boolean;
  selling_stance: number;
  style_mandate: number;
  transfer_style: string;
  wage_cap_mult: number;
  youth_investment: number;
  youth_minutes_target: number;
}

export interface ClubSystemsViewBoardProject {
  completes: number;
  cost: number | null;
  kind: string;
  started: number;
  target: number;
}

export interface ClubSystemsViewBoard {
  administration: number | null;
  chairman: Named;
  concerns: ClubSystemsViewBoardConcern[] | null;
  kind: string;
  owner: Named;
  owner_traits: ClubSystemsViewBoardOwnerTraits | null;
  policy: ClubSystemsViewBoardPolicy | null;
  projects: ClubSystemsViewBoardProject[];
  red_months: number | null;
  revenue_history: number[] | null;
  since: number;
}

export interface ClubSystemsViewCultureIdentity {
  flair: number;
  glamour: number;
  grit: number;
  local: number;
  underdog: number;
  youth: number;
}

export interface ClubSystemsViewCulture {
  discipline: number;
  drought: number;
  expectations: number;
  graduates: number;
  identity: ClubSystemsViewCultureIdentity;
  patience: number;
  tribalism: number;
}

export interface ClubSystemsViewPlanGroup {
  ageing_starters: number;
  avg_age: number;
  depth: number;
  expiring: number;
  group: string;
  injured: number;
  prospects: number;
  quality: number;
  target_depth: number;
  target_quality: number;
}

export interface ClubSystemsViewPlanNeed {
  fee_band: number;
  group: string;
  homegrown: boolean;
  max_age: number;
  min_ability: number;
  role: string;
  urgency: number;
  wage_band: number;
}

export interface ClubSystemsViewPlan {
  built: number;
  groups: ClubSystemsViewPlanGroup[];
  homegrown_gap: number;
  needs: ClubSystemsViewPlanNeed[];
  promote: Named[];
  sell: Named[];
}

export interface ClubSystemsViewRivalry {
  intensity: number;
  last_met: number | null;
  record: number[];
  since: number;
  why: string[];
  with: Named;
}

export interface ClubSystemsViewRoomGroup {
  bond: string;
  cohesion: number;
  leader: Named;
  size: number;
  stance: number;
}

export interface ClubSystemsViewRoomInfluential {
  influence: number;
  standing: string;
  who: Named;
}

export interface ClubSystemsViewRoom {
  backing: number;
  groups: ClubSystemsViewRoomGroup[];
  harmony: number;
  influential: ClubSystemsViewRoomInfluential[];
  updated: number;
}

export interface ClubSystemsViewScoutingScout {
  based: string;
  briefs: string[];
  capacity: number;
  who: Named;
}

export interface ClubSystemsViewScouting {
  reports: number;
  scouts: ClubSystemsViewScoutingScout[];
}

export interface ClubSystemsViewSponsor {
  brand: string;
  fee: number | null;
  slot: string;
  until: number;
}

export interface ClubSystemsViewSupporter {
  board: number;
  kind: string;
  last_acted: number;
  manager: number;
  size: number;
  team: number;
  voice: number;
}

export interface ClubSystemsView {
  board: ClubSystemsViewBoard | null;
  culture: ClubSystemsViewCulture | null;
  internal: boolean;
  plan: ClubSystemsViewPlan | null;
  rivalries: ClubSystemsViewRivalry[];
  room: ClubSystemsViewRoom | null;
  scouting: ClubSystemsViewScouting | null;
  sponsors: ClubSystemsViewSponsor[];
  supporters: ClubSystemsViewSupporter[];
}

export interface CompViewRules {
  away_goals: boolean;
  bench: number;
  extra_time: boolean;
  foreigner_limit: number;
  subs: number;
  yellow_limit: number;
}

export interface CompViewState {
  end: number | null;
  knockout: boolean;
  round: number;
  runner_up: Named | null;
  season: string;
  season_year: number;
  stage: string;
  start: number | null;
  teams: number;
  winner: Named | null;
}

export interface CompViewTy {
  a: Named;
  b: Named;
  goals_a: number;
  goals_b: number;
  hidden: boolean;
  index: number;
  legs: number;
  played: number;
  round: number;
  winner: Named | null;
}

export interface CompView {
  above: Named | null;
  below: Named | null;
  continental_places: unknown[];
  format: string;
  groups: number;
  id: number;
  is_league: boolean;
  kind: string;
  kind_key: string;
  name: string;
  nation: Named;
  past_editions: number;
  prize_pool: number | null;
  promote: number;
  relegate: number;
  reputation: number;
  rules: CompViewRules;
  short: string;
  size: number;
  state: CompViewState | null;
  team_kind: string;
  tier: number;
  ties: CompViewTy[];
}

export interface NationViewLeague {
  comp: Named;
  teams: number;
  tier: number;
}

export interface NationViewSeason {
  end: number | null;
  label: string;
  start: number | null;
  windows: number[][];
  winter_break: unknown;
}

export interface NationViewSideRecord {
  drawn: number;
  lost: number;
  won: number;
}

export interface NationViewSide {
  captain: Named | null;
  level: string;
  manager: Named;
  record: NationViewSideRecord;
  selected: number;
  since: number;
  squad: number;
}

export interface NationViewWorldEconomy {
  broadcast_pool: number | null;
  coefficient: number;
  deal_until: number | null;
  growth: number | null;
  league_strength: number;
  rank: number;
  wage_index: number | null;
}

export interface NationView {
  clubs: number;
  code: string;
  confed: string;
  cups: Named[];
  economy: number | null;
  id: number;
  leagues: NationViewLeague[];
  name: string;
  players: number;
  reputation: number;
  season: NationViewSeason;
  sides: NationViewSide[];
  window_open: boolean;
  world_economy: NationViewWorldEconomy | null;
  youth_rating: number | null;
}

export interface MatchViewAway {
  club: number;
  colors: string[];
  mine: boolean;
  short: string;
  team: Named;
}

export interface MatchViewDetailEvent {
  kind: string;
  label: string;
  minute: number;
  other: Named | null;
  player: Named | null;
  side: number;
  t: number;
  tier: string;
  x: number;
  xg: number;
  y: number;
}

export interface MatchViewDetailLine {
  assists: number;
  clearances: number;
  conceded: number;
  dribbles: number;
  dribbles_won: number;
  fouls: number;
  goals: number;
  injured: boolean;
  interceptions: number;
  keeper: boolean;
  key_passes: number;
  minutes: number;
  off_at: number;
  on_at: number;
  on_target: number;
  passes: number;
  passes_completed: number;
  player: Named;
  pos: string | null;
  rating: number;
  reds: number;
  saves: number;
  shots: number;
  side: number;
  started: boolean;
  tackles: number;
  tackles_won: number;
  x: number;
  xa: number;
  xg: number;
  y: number;
  yellows: number;
}

export interface MatchViewDetailShapeAway {
  formation: string;
}

export interface MatchViewDetailStat {
  big_chances: number;
  corners: number;
  fouls: number;
  offsides: number;
  on_target: number;
  passes: number;
  passes_completed: number;
  possession: number;
  reds: number;
  saves: number;
  shots: number;
  tackles: number;
  xg: number;
  yellows: number;
}

export interface MatchViewDetail {
  events: MatchViewDetailEvent[];
  lines: MatchViewDetailLine[];
  man_of_the_match: Named;
  shape_away: MatchViewDetailShapeAway | null;
  shape_home: MatchViewDetailShapeAway | null;
  stats: MatchViewDetailStat[];
}

export interface MatchViewPreAwayAbsence {
  player: Named;
  why: string;
}

export interface MatchViewPreAway {
  absences: MatchViewPreAwayAbsence[];
  position: number | null;
}

export interface MatchViewPre {
  away: MatchViewPreAway | null;
  home: MatchViewPreAway;
}

export interface MatchViewPreviou {
  away: string;
  comp: string;
  date: number;
  home: string;
  score: string;
  uid: number;
}

export interface MatchViewRules {
  bench: number;
  extra_time: boolean;
  subs: number;
}

export interface MatchViewScore {
  away: number;
  extra_time: boolean;
  home: number;
  ht_away: number;
  ht_home: number;
  pens: number[] | null;
  text: string;
}

export interface MatchView {
  away: MatchViewAway | null;
  can_follow: boolean;
  capacity: number | null;
  comp: Named;
  concealed: boolean;
  date: number;
  decisive: boolean;
  detail: MatchViewDetail | null;
  detail_kept: boolean;
  home: MatchViewAway;
  neutral: boolean;
  pre: MatchViewPre | null;
  previous: MatchViewPreviou[];
  round: string;
  rules: MatchViewRules;
  score: MatchViewScore | null;
  status: string;
  uid: number;
  venue: string;
  watching: boolean;
}

export interface MeTodayViewAvailability {
  ban: number;
  days: number;
  injured: boolean;
  injury: string | null;
}

export interface MeTodayViewChange {
  date: number;
  id: string;
  important: boolean;
  kind: string;
  parts: Part[];
}

export interface MeTodayViewCommitment {
  kind: string;
  ref?: Ref;
  text: string;
}

export interface MeTodayViewConditionCondition {
  label: string;
  step: number;
  steps: number;
}

export interface MeTodayViewCondition {
  condition: MeTodayViewConditionCondition | null;
  confidence: MeTodayViewConditionCondition;
  fatigue: MeTodayViewConditionCondition;
  morale: MeTodayViewConditionCondition;
  sharpness: MeTodayViewConditionCondition;
  wellbeing: MeTodayViewConditionCondition;
}

export interface MeTodayViewContract {
  club: Named;
  days_left: number;
  end: number;
  status: string;
  wage: number;
}

export interface MeTodayViewDay {
  kind: string;
  label: string;
}

export interface MeTodayViewLeague {
  comp: Named;
  points: number;
  position: number;
  teams: number;
}

export interface MeTodayViewMe {
  age: number;
  club: Named | null;
  name: string;
  person: number;
  position: string;
  shirt: number;
  squad_status: string | null;
  status: string;
  team: string | null;
}

export interface MeTodayViewMind {
  pull: string;
  text: string;
}

export interface MeTodayViewNextMatch {
  comp: Named;
  date: number;
  days: number;
  home: boolean;
  opponent: Named;
  round: string;
  uid: number;
  venue: unknown;
}

export interface MeTodayViewPlanFocus {
  kind: string;
  value: string | null;
}

export interface MeTodayViewPlan {
  extra: number;
  focus: MeTodayViewPlanFocus | null;
  intensity: string;
  recovery: number;
}

export interface MeTodayViewPromises {
  next_due: number | null;
  open: number;
}

export interface MeTodayViewRecent {
  comp: Named;
  concealed: boolean;
  date: number;
  days: number;
  home: boolean;
  opponent: Named;
  outcome?: string;
  round: string;
  score?: string;
  uid: number;
  venue: unknown;
}

export interface MeTodayViewWaitingOn {
  date?: number;
  kind: string;
  ref?: Ref;
  since: number;
  text: string;
}

export interface MeTodayViewKnownFace {
  who: Named;
  how: Part[];
  role: string;
}

export interface MeTodayViewOnThisDay {
  years_ago: number;
  date: number;
  parts: Part[];
}

export interface MeTodayViewAround {
  kind: string;
  date: number;
  parts: Part[];
}

export interface MeTodayViewBuildup {
  name: string | null;
  significance: number;
  lines: Part[][];
}

export interface MeTodayViewSettlingPart {
  label: string;
  words: string;
}

export interface MeTodayViewSettling {
  club: Named;
  since: number;
  plan: string;
  parts: MeTodayViewSettlingPart[];
}

export interface MeTodayViewAtmosphere {
  mood: string;
  lines: string[];
}

export interface MeTodayViewMissed {
  uid: number;
  date: number;
  opponent: Named;
  score: string;
  outcome: string;
}

export interface MeTodayViewRecovery {
  injury: string;
  since: number;
  treatment: string;
  estimate: number;
  sureness: string;
  stages: string[];
  stage: number;
  setbacks: number;
  recurrence: boolean;
  rushed: boolean;
  physio: Named | null;
  missed: MeTodayViewMissed[];
}

export interface MeTodayView {
  availability: MeTodayViewAvailability;
  settling: MeTodayViewSettling | null;
  atmosphere: MeTodayViewAtmosphere | null;
  buildup: MeTodayViewBuildup | null;
  known_faces: MeTodayViewKnownFace[];
  recovery: MeTodayViewRecovery | null;
  changes: MeTodayViewChange[];
  commitments: MeTodayViewCommitment[];
  conceal_mine: boolean;
  condition: MeTodayViewCondition | null;
  contract: MeTodayViewContract | null;
  date: number;
  day: MeTodayViewDay;
  decisions: unknown[];
  form: number[];
  last_viewed: number;
  league: MeTodayViewLeague | null;
  lifestyle: string;
  me: MeTodayViewMe | null;
  mind: MeTodayViewMind[];
  minutes_4w: number;
  next_match: MeTodayViewNextMatch | null;
  plan: MeTodayViewPlan | null;
  plan_pending: MeTodayViewPlan | null;
  promises: MeTodayViewPromises;
  queued: string[];
  recent: MeTodayViewRecent[];
  routine_hours: number;
  unrevealed: unknown[];
  waiting_on: MeTodayViewWaitingOn[];
  on_this_day: MeTodayViewOnThisDay[];
  around: MeTodayViewAround[];
  weekday: number;
}

export interface MeMessagesViewMessage {
  date: number;
  deadline: number | null;
  folder: string;
  from: unknown;
  id: string;
  important: boolean;
  kind: string;
  needs_action: boolean;
  dkind?: string;
  preview?: string;
  parts?: Part[];
  state: string;
  subject: string;
  unread?: boolean;
}

export interface MeMessagesView {
  awaiting: number;
  messages: MeMessagesViewMessage[];
  unread: number;
}

export interface MeInboxViewThread {
  count: number;
  deadline: unknown;
  id: number;
  kind: string;
  last: number;
  last_kind: string;
  needs_action: boolean;
  opened: number;
  preview: string;
  title: string;
  unread: number;
  with: Named | null;
}

export interface MeInboxView {
  awaiting: number;
  threads: MeInboxViewThread[];
  unread: number;
}

export interface MeThreadViewMessageMeetingLine {
  name: string;
  ref: Ref;
  side: string;
  text: string;
  tone: string;
}

export interface MeThreadViewMessageMeeting {
  date: number;
  lines: MeThreadViewMessageMeetingLine[];
  outcomes: string[];
  state: string;
  topic: string;
  why: string[];
  with: Named;
}

export interface MeThreadViewMessagePostAuthor {
  display: string;
  followers: number;
  handle: string;
  kind: string;
  person: unknown;
  you: boolean;
}

export interface MeThreadViewMessagePostParent {
  about: Named | null;
  author: MeThreadViewMessagePostAuthor;
  date: number;
  id: number;
  likes: number;
  parent: unknown;
  quote_of: unknown;
  quoted: unknown;
  replies: number;
  reply_to: unknown;
  reposts: number;
  text: string;
}

export interface MeThreadViewMessagePost {
  about: Named | null;
  author: MeThreadViewMessagePostAuthor;
  date: number;
  id: number;
  likes: number;
  parent: MeThreadViewMessagePostParent | null;
  quote_of: number | null;
  quoted: MeThreadViewMessagePostParent | null;
  replies: number;
  reply_to: number | null;
  reposts: number;
  text: string;
}

export interface MeThreadViewMessageReplied {
  date: number;
  label: string;
}

export interface MeThreadViewMessageReply {
  effect: string;
  key: string;
  label: string;
  quiet: boolean;
}

export interface MeThreadViewMessageStory {
  body: string;
  date: number;
  headline: string;
  id: number;
  outlet: string;
}

export interface MeThreadViewMessage {
  date: number;
  from: Named | null;
  id: number;
  kind: string;
  label?: string;
  meeting?: MeThreadViewMessageMeeting | null;
  parts?: Part[];
  post?: MeThreadViewMessagePost | null;
  read: boolean;
  replied: MeThreadViewMessageReplied | null;
  replies: MeThreadViewMessageReply[];
  story?: MeThreadViewMessageStory | null;
  text: string;
  decision?: ThreadDecision;
  sureness?: string;
}

export interface MeThreadView {
  id: number;
  kind: string;
  last: number;
  messages: MeThreadViewMessage[];
  opened: number;
  title: string;
  with: Named | null;
}

export interface MeMessageViewStory {
  body: string;
  date: number;
  headline: string;
  outlet: string;
}

export interface MeMessageView {
  date: number;
  id: string;
  kind: string;
  meeting: MeThreadViewMessageMeeting | null;
  parts: Part[];
  primary: Ref | null;
  story: MeMessageViewStory | null;
  title: string;
  why: string[];
}

export interface MeOptionsViewAgent {
  base: string;
  clients: number;
  id: number;
  person: Named;
  reputation: number;
}

export interface MeOptionsViewCareer {
  key: string;
  label: string;
}

export interface MeOptionsViewCours {
  cost: number;
  done: boolean;
  effort: number;
  key: string;
  label: string;
  requires: string | null;
  studying: boolean;
}

export interface MeOptionsViewHelperHired {
  cost: number;
  quality: number;
}

export interface MeOptionsViewHelper {
  base_cost: number;
  hired: MeOptionsViewHelperHired | null;
  key: string;
  label: string;
}

export interface MeOptionsViewMeetWith {
  role: string;
  who: Named;
}

export interface MeOptionsViewNation {
  id: number;
  name: string;
}

export interface MeOptionsView {
  agents: MeOptionsViewAgent[];
  careers: MeOptionsViewCareer[];
  courses: MeOptionsViewCours[];
  helpers: MeOptionsViewHelper[];
  lifestyles: MeOptionsViewCareer[];
  meet_with: MeOptionsViewMeetWith[];
  nations: MeOptionsViewNation[];
  posts: MeOptionsViewCareer[];
  roles: MeOptionsViewCareer[];
  routine_budget: number;
  stances: MeOptionsViewCareer[];
  tones: MeOptionsViewCareer[];
  topics: MeOptionsViewCareer[];
}

export interface MeSelfViewCareer {
  apps: number;
  caps: number;
  clubs: number;
  goals: number;
  position: string;
  status: string;
}

export interface MeSelfViewMood {
  factor: string;
  pull: string;
  text: string;
}

export interface MeSelfViewTold {
  date: number;
  sureness: string;
  text: string;
}

export interface MeSelfView {
  age: number;
  career: MeSelfViewCareer;
  fulfilment: MeTodayViewConditionCondition;
  hint: string;
  mood: MeSelfViewMood[];
  name: string;
  nation: Named;
  personality: string;
  sleep: MeTodayViewConditionCondition;
  stress: MeTodayViewConditionCondition;
  told: MeSelfViewTold[];
  wellbeing: MeSelfViewMood[];
}

export interface MeLifeViewGiving {
  community: number;
  foundation: boolean;
  pct: number;
}

export interface MeLifeViewHelper {
  cost: number;
  label: string;
  quality: number;
}

export interface MeLifeViewHome {
  kind: string | null;
  nation: Named;
  quality: number | null;
  since: number;
}

export interface MeLifeViewLanguage {
  level: MeTodayViewConditionCondition;
  nation: Named;
}

export interface MeLifeViewMoney {
  debt: number;
  family_support: number;
  income: number;
  invested: number;
  lifestyle: string;
  savings: number;
  spending: number;
}

export interface MeLifeViewParents {
  alive: number;
  closeness: string;
  health: string;
  nation: string;
}

export interface MeLifeViewPartner {
  bond: MeTodayViewConditionCondition;
  lives: string | null;
  occupation: string;
  since: number;
  status: string;
  who: Named;
}

export interface MeLifeViewRoutineRow {
  hours: number;
  key: string;
  label: string;
}

export interface MeLifeViewRoutine {
  budget: number;
  rows: MeLifeViewRoutineRow[];
  total: number;
}

export interface MeLifeViewStudying {
  course: string;
  done: number;
  effort: number;
}

export interface MeLifeView {
  children: number;
  education: number;
  giving: MeLifeViewGiving | null;
  helpers: MeLifeViewHelper[];
  home: MeLifeViewHome;
  languages: MeLifeViewLanguage[];
  money: MeLifeViewMoney | null;
  morale: MeSelfViewMood[];
  occupation: string;
  open_to_dating: boolean | null;
  parents: MeLifeViewParents;
  partner: MeLifeViewPartner | null;
  qualifications: MeThreadViewMessageReplied[];
  routine: MeLifeViewRoutine;
  siblings: number;
  studying: MeLifeViewStudying | null;
  wellbeing: MeSelfViewMood[];
  work: unknown;
}

export interface MePressViewFan {
  club: Named;
  label: string;
  reasons: string[];
  score: number;
}

export interface MePressViewQuote {
  about: unknown;
  at_conference: boolean;
  date: number;
  id: number;
  stance: string;
}

export interface MePressViewReaction {
  date: number;
  sentiment: number;
  text: string;
}

export interface MePressViewStory {
  about_you: boolean;
  date: number;
  headline: string;
  id: number;
  outlet: string;
}

export interface MePressView {
  fans: MePressViewFan[];
  image: string;
  quotes: MePressViewQuote[];
  reactions: MePressViewReaction[];
  stories: MePressViewStory[];
}

export interface MeFeedViewAccount {
  followers: number;
  handle: string;
}

export interface MeFeedViewPostAuthor {
  display: string;
  followers: number;
  handle: string;
  kind: string;
  person: Named | null;
  you: boolean;
}

export interface MeFeedViewPostParent {
  about: Named | null;
  author: MeThreadViewMessagePostAuthor;
  date: number;
  id: number;
  likes: number;
  parent: unknown;
  quote_of: unknown;
  quoted: unknown;
  replies: number;
  reply_to: number | null;
  reposts: number;
  text: string;
}

export interface MeFeedViewPost {
  about: Named | null;
  author: MeFeedViewPostAuthor;
  date: number;
  id: number;
  likes: number;
  parent: MeFeedViewPostParent | null;
  quote_of: number | null;
  quoted: MeThreadViewMessagePostParent | null;
  replies: number;
  reply_to: number | null;
  reposts: number;
  text: string;
}

export interface MeFeedView {
  account: MeFeedViewAccount | null;
  posts: MeFeedViewPost[];
}

export interface SocialThreadViewPost {
  about: Named | null;
  author: MeFeedViewPostAuthor;
  date: number;
  id: number;
  likes: number;
  parent: MeFeedViewPostParent | null;
  quote_of: unknown;
  quoted: unknown;
  replies: number;
  reply_to: number | null;
  reposts: number;
  text: string;
}

export interface SocialThreadViewReply {
  about: Named | null;
  author: MeThreadViewMessagePostAuthor;
  date: number;
  id: number;
  likes: number;
  parent: unknown;
  quote_of: unknown;
  quoted: unknown;
  replies: number;
  reply_to: number;
  reposts: number;
  text: string;
}

export interface SocialThreadView {
  post: SocialThreadViewPost | null;
  replies: SocialThreadViewReply[];
}

export interface MeCalendarViewDayEntry {
  kind: string;
  label: string;
  ref?: Ref;
  required?: boolean;
  result?: string;
  source: string;
  state: string;
  sub?: string;
}

export interface MeCalendarViewDay {
  date: number;
  entries: MeCalendarViewDayEntry[];
}

export interface MeCalendarView {
  days: MeCalendarViewDay[];
  from: number;
  to: number;
  today: number;
}

export interface MeFootballViewOptionsAttribute {
  group: string;
  key: string;
  label: string;
}

export interface MeFootballViewOptionsPosition {
  code: string;
}

export interface MeFootballViewOptions {
  attributes: MeFootballViewOptionsAttribute[];
  positions: MeFootballViewOptionsPosition[];
}

export interface MeFootballViewRival {
  age: number;
  available: boolean;
  minutes_4w: number;
  player: Named;
  pos: string;
}

export interface MeFootballViewSeason {
  apps: number;
  minutes: number;
  starts: number;
}

export interface MeFootballViewUsagePlayed {
  assists: number;
  goals: number;
  minutes: number;
  rating: number;
  started: boolean;
}

export interface MeFootballViewUsage {
  comp: Named;
  concealed: boolean;
  date: number;
  days: number;
  home: boolean;
  opponent: Named;
  outcome?: string;
  played?: MeFootballViewUsagePlayed | null;
  round: string;
  score?: string;
  uid: number;
  venue: unknown;
}

export interface MeFootballView {
  minutes_4w: number;
  options: MeFootballViewOptions | null;
  plan: MeTodayViewPlan | null;
  plan_pending: MeTodayViewPlan | null;
  promised: unknown;
  rivals: MeFootballViewRival[];
  season: MeFootballViewSeason;
  squad_status: string;
  team: string | null;
  usage: MeFootballViewUsage[];
}

export interface MeContractViewAgent {
  fee_pct: number;
  satisfaction: string;
  until: number;
  who: Named;
}

export interface MeContractViewSummary {
  days_left: number;
  end: number;
  kind: string;
  status: string;
  wage: number;
}

export interface MeContractViewTerm {
  date?: number;
  label: string;
  money?: number | null;
  text?: string;
}

export interface MeContractView {
  agent?: MeContractViewAgent | null;
  club?: Named;
  guaranteed_note?: string;
  has_contract: boolean;
  history?: unknown[];
  listed?: boolean;
  loan?: unknown;
  offers?: unknown[];
  status?: string;
  summary?: MeContractViewSummary | null;
  talks?: unknown;
  terms?: MeContractViewTerm[];
  transfer_request?: number | null;
}

/** Every method: whether it is a query or a command, and its request and response types where they are declared. */
export interface ApiMethods {
  "app.info": { kind: "query"; req: Record<string, unknown>; res: AppInfo };
  "world.status": { kind: "query"; req: Record<string, unknown>; res: StatusView };
  "world.new": { kind: "command"; req: Record<string, unknown>; res: Started };
  "world.inspect_import": { kind: "query"; req: DirReq; res: InspectImportView };
  "world.datasets": { kind: "query"; req: Record<string, unknown>; res: DatasetsView };
  "world.saves": { kind: "query"; req: Record<string, unknown>; res: WorldSavesView };
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
  "route.options": { kind: "query"; req: Record<string, unknown>; res: RouteOptionsView };
  "route.begin": { kind: "command"; req: RouteReq; res: Created };
  "table.query": { kind: "query"; req: TableReq; res: TableResp };
  "search": { kind: "query"; req: SearchReq; res: SearchView };
  "overview": { kind: "query"; req: Record<string, unknown>; res: OverviewView };
  "world.pulse": { kind: "query"; req: LimitReq; res: WorldPulseView };
  "news.feed": { kind: "query"; req: NewsFeedReq; res: NewsFeedView };
  "news.story": { kind: "query"; req: IdReq; res: StoryFull };
  "diagnostics": { kind: "query"; req: Record<string, unknown>; res: DiagnosticsView };
  "capabilities": { kind: "query"; req: Record<string, unknown>; res: Capability[] };
  "person": { kind: "query"; req: PersonReq; res: PersonView };
  "person.attributes": { kind: "query"; req: PersonReq; res: AttributesView };
  "crest.colors": { kind: "query"; req: Record<string, unknown>; res: CrestColorsView };
  "comp.overview": { kind: "query"; req: CompOverviewReq; res: CompOverviewView };
  "insight.club": { kind: "query"; req: InsightReq; res: InsightsView };
  "insight.comp": { kind: "query"; req: InsightReq; res: InsightsView };
  "insight.match": { kind: "query"; req: InsightMatchReq; res: InsightsView };
  "insight.person": { kind: "query"; req: InsightReq; res: InsightsView };
  "club": { kind: "query"; req: IdReq; res: ClubView };
  "club.systems": { kind: "query"; req: IdReq; res: ClubSystemsView };
  "club.follow": { kind: "command"; req: FollowReq; res: Followed };
  "comp": { kind: "query"; req: IdReq; res: CompView };
  "nation": { kind: "query"; req: IdReq; res: NationView };
  "match": { kind: "query"; req: MatchReq; res: MatchView };
  "match.watch": { kind: "query"; req: MatchReq; res: MatchView };
  "match.reveal": { kind: "command"; req: RevealReq; res: Revealed };
  "match.reveal_all": { kind: "command"; req: Record<string, unknown>; res: RevealedAll };
  "me.today": { kind: "query"; req: Record<string, unknown>; res: MeTodayView };
  "me.viewed": { kind: "command"; req: Record<string, unknown>; res: Done };
  "me.messages": { kind: "query"; req: LimitReq; res: MeMessagesView };
  "me.inbox": { kind: "query"; req: LimitReq; res: MeInboxView };
  "me.thread": { kind: "query"; req: IdReq; res: MeThreadView };
  "me.thread_read": { kind: "command"; req: IdReq; res: Done };
  "me.reply": { kind: "command"; req: ReplyReq; res: ActDone };
  "me.message": { kind: "query"; req: MessageReq; res: MeMessage };
  "me.answer": { kind: "command"; req: AnswerReq; res: Done };
  "me.act": { kind: "command"; req: ActReq; res: ActDone };
  "me.options": { kind: "query"; req: Record<string, unknown>; res: MeOptionsView };
  "me.self": { kind: "query"; req: Record<string, unknown>; res: MeSelfView };
  "me.life": { kind: "query"; req: Record<string, unknown>; res: MeLifeView };
  "person.life": { kind: "query"; req: PersonOptReq; res: MeLifeView };
  "me.people": { kind: "query"; req: Record<string, unknown>; res: PeopleView };
  "me.promises": { kind: "query"; req: Record<string, unknown>; res: PromisesView };
  "me.rumours": { kind: "query"; req: Record<string, unknown>; res: RumoursView };
  "me.press": { kind: "query"; req: Record<string, unknown>; res: MePressView };
  "me.feed": { kind: "query"; req: LimitReq; res: MeFeedView };
  "social.thread": { kind: "query"; req: IdReq; res: SocialThreadView };
  "me.story": { kind: "query"; req: IdReq; res: OwnStoryView };
  "me.agent": { kind: "query"; req: Record<string, unknown>; res: AgentView };
  "me.journal": { kind: "query"; req: Record<string, unknown>; res: JournalView };
  "me.chronicle": { kind: "query"; req: Record<string, unknown>; res: ChronicleView };
  "me.chats": { kind: "query"; req: Record<string, unknown>; res: ChatsView };
  "me.money": { kind: "query"; req: Record<string, unknown>; res: MoneyView };
  "me.training": { kind: "query"; req: Record<string, unknown>; res: TrainingLogView };
  "me.matchday": { kind: "query"; req: MatchReq; res: MatchdayView };
  "me.chat": { kind: "query"; req: IdReq; res: ChatView };
  "me.chat_read": { kind: "command"; req: IdReq; res: Done };
  "me.goal": { kind: "command"; req: GoalReq; res: Done };
  "me.goal_done": { kind: "command"; req: GoalDoneReq; res: Done };
  "me.note": { kind: "command"; req: NoteReq; res: Done };
  "me.note_remove": { kind: "command"; req: IndexReq; res: Done };
  "me.calendar": { kind: "query"; req: CalendarReq; res: MeCalendarView };
  "me.football": { kind: "query"; req: Record<string, unknown>; res: MeFootballView };
  "me.plan": { kind: "command"; req: PlanReq; res: PlanSet };
  "me.contract": { kind: "query"; req: Record<string, unknown>; res: MeContractView };
  "pathway.player": { kind: "query"; req: PersonReq; res: PathwayView };
  "ecosystem.regions": { kind: "query"; req: Record<string, unknown>; res: RegionOutputView };
  "ecosystem.export": { kind: "query"; req: Record<string, unknown>; res: ExportView };
  "ecosystem.scenario": { kind: "query"; req: Record<string, unknown>; res: ScenarioView };
  "ecosystem.district": { kind: "query"; req: DistrictReq; res: DistrictView };
  "database.sources": { kind: "query"; req: Record<string, unknown>; res: DatabaseSourcesView };
  "database.attach": { kind: "command"; req: DirReq; res: DatabaseAttached };
  "database.query": { kind: "query"; req: DatabaseQueryReq; res: DatabaseQueryView };
}

/** The methods whose response type is declared here. */
export type TypedMethod =
  | "app.info"
  | "world.status"
  | "world.new"
  | "world.inspect_import"
  | "world.datasets"
  | "world.saves"
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
  | "route.options"
  | "route.begin"
  | "table.query"
  | "search"
  | "overview"
  | "world.pulse"
  | "news.feed"
  | "news.story"
  | "diagnostics"
  | "capabilities"
  | "person"
  | "person.attributes"
  | "crest.colors"
  | "comp.overview"
  | "insight.club"
  | "insight.comp"
  | "insight.match"
  | "insight.person"
  | "club"
  | "club.systems"
  | "club.follow"
  | "comp"
  | "nation"
  | "match"
  | "match.watch"
  | "match.reveal"
  | "match.reveal_all"
  | "me.today"
  | "me.viewed"
  | "me.messages"
  | "me.inbox"
  | "me.thread"
  | "me.thread_read"
  | "me.reply"
  | "me.message"
  | "me.answer"
  | "me.act"
  | "me.options"
  | "me.self"
  | "me.life"
  | "person.life"
  | "me.people"
  | "me.promises"
  | "me.rumours"
  | "me.press"
  | "me.feed"
  | "social.thread"
  | "me.story"
  | "me.agent"
  | "me.journal"
  | "me.chronicle"
  | "me.chats"
  | "me.money"
  | "me.training"
  | "me.matchday"
  | "me.chat"
  | "me.chat_read"
  | "me.goal"
  | "me.goal_done"
  | "me.note"
  | "me.note_remove"
  | "me.calendar"
  | "me.football"
  | "me.plan"
  | "me.contract"
  | "pathway.player"
  | "ecosystem.regions"
  | "ecosystem.export"
  | "ecosystem.scenario"
  | "ecosystem.district"
  | "database.sources"
  | "database.attach"
  | "database.query"
;
