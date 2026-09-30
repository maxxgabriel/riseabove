//! The API contract (locked design 9): one authoritative source for what crosses the boundary between the simulation and the client.
//!
//! * **View types, not world types.** The payloads declared here are built by the pages from perspective-resolved state
//!   (`Ctx`), never from `World` structures, so nothing hidden can ride along in a field a developer forgot to omit.
//! * **One source of truth.** A payload type is declared once with `contract!`; the same declaration is a Rust struct (serialised
//!   with serde) and a TypeScript interface. `typescript()` renders every declaration; `crates/pw-view/tests/contract.rs` fails when
//!   `app/src/contract.generated.ts` differs from it. Types not yet migrated are `untyped` in the manifest, and the manifest says so.
//! * **Structured errors.** `ErrorKind` is the category a client can act on ("you do not know this" is not "the program failed").
//! * **Semantic values.** `Knowledge<T>` keeps exact, estimated, reported, unknown and hidden apart; none of them is a `0` or a `null`.
//! * **Queries and commands are separate.** Every method is one or the other in the manifest, and a test proves queries change nothing.

use serde::Serialize;
use serde_json::Value;

use crate::model::{Named, Tone};

// ------------------------------------------------------------------ type mapping

/// How a Rust payload type is written in TypeScript.
pub trait Ts {
    fn ts() -> String;
}

macro_rules! ts_prim {
    ($($t:ty => $s:literal),* $(,)?) => {$( impl Ts for $t { fn ts() -> String { $s.into() } } )*};
}

ts_prim!(bool => "boolean", u8 => "number", u16 => "number", u32 => "number", u64 => "number", usize => "number", i8 => "number", i16 => "number", i32 => "number", i64 => "number",
    f32 => "number", f64 => "number", String => "string", &'static str => "string", Value => "unknown");

impl<T: Ts> Ts for Option<T> {
    fn ts() -> String {
        format!("{} | null", T::ts())
    }
}

impl<T: Ts> Ts for Vec<T> {
    fn ts() -> String {
        let t = T::ts();
        if t.contains(' ') { format!("({t})[]") } else { format!("{t}[]") }
    }
}

/// Declare a payload type once: a serialisable Rust struct and its TypeScript interface.
macro_rules! contract {
    ($( $(#[$m:meta])* pub struct $name:ident { $( $(#[$fm:meta])* pub $f:ident : $t:ty ),* $(,)? } )*) => {$(
        $(#[$m])*
        #[derive(Clone, Debug, Serialize)]
        pub struct $name { $( $(#[$fm])* pub $f: $t ),* }

        impl Ts for $name {
            fn ts() -> String { stringify!($name).into() }
        }

        impl $name {
            pub fn declaration() -> String {
                let mut s = format!("export interface {} {{\n", stringify!($name));
                $( s.push_str(&format!("  {}: {};\n", stringify!($f), <$t as Ts>::ts())); )*
                s.push_str("}\n");
                s
            }
        }
    )*};
}

/// Declare a request once: a Rust struct that refuses fields it does not name (so a client cannot slip authoritative world state, a
/// wage or a score, into a command) and its TypeScript interface. An `Option` field may be left out.
macro_rules! request {
    ($( $(#[$m:meta])* pub struct $name:ident { $( $(#[$fm:meta])* pub $f:ident : $t:ty ),* $(,)? } )*) => {$(
        $(#[$m])*
        #[derive(Clone, Debug, Default, serde::Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $( $(#[$fm])* pub $f: $t ),* }

        impl Ts for $name {
            fn ts() -> String { stringify!($name).into() }
        }

        impl $name {
            pub fn declaration() -> String {
                let mut s = format!("export interface {} {{\n", stringify!($name));
                $( s.push_str(&format!("  {}\n", ts_field(stringify!($f), &<$t as Ts>::ts()))); )*
                s.push_str("}\n");
                s
            }
        }
    )*};
}

/// A request with one shape per kind, told apart by a tag field (`me.act`'s `action`). Unknown kinds and unknown fields are refused.
macro_rules! request_enum {
    ($(#[$m:meta])* pub enum $name:ident tag $tag:literal { $( $(#[$vm:meta])* $var:ident $key:literal { $( $f:ident : $t:ty ),* $(,)? } ),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, serde::Deserialize, Serialize)]
        #[serde(tag = $tag, deny_unknown_fields)]
        pub enum $name { $( $(#[$vm])* #[serde(rename = $key)] $var { $( $f: $t ),* } ),* }

        impl Ts for $name {
            fn ts() -> String { stringify!($name).into() }
        }

        impl $name {
            /// Every kind, in declaration order.
            pub const TAGS: &'static [&'static str] = &[$($key),*];

            pub fn tag(&self) -> &'static str {
                match self { $( $name::$var { .. } => $key ),* }
            }

            pub fn declaration() -> String {
                let mut s = format!("export type {} =\n", stringify!($name));
                $(
                    s.push_str(&format!("  | {{ {}: \"{}\"", $tag, $key));
                    $( s.push_str(&format!("; {}", ts_field(stringify!($f), &<$t as Ts>::ts()).trim_end_matches(';'))); )*
                    s.push_str(" }\n");
                )*
                s.push_str(";\n");
                s
            }
        }
    };
}

/// `name: type;`, or `name?: type;` for a field that may be left out.
fn ts_field(name: &str, ty: &str) -> String {
    if ty.ends_with("| null") { format!("{name}?: {ty};") } else { format!("{name}: {ty};") }
}

/// Read a call's arguments as its declared request type. A malformed request (a missing or mistyped field, a field the request does
/// not have) is an `InvalidRequest` that names the problem, never a half-read command.
pub fn request<T: serde::de::DeserializeOwned>(args: Value) -> crate::model::ApiResult<T> {
    let args = if args.is_null() { Value::Object(Default::default()) } else { args };
    serde_json::from_value(args).map_err(|e| crate::model::ApiError::Bad(format!("Invalid request: {e}.")))
}

// ------------------------------------------------------------------ errors

/// What went wrong, in terms the client can act on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The thing asked for does not exist.
    NotFound,
    /// The viewer's perspective does not allow this (observing when a person is needed, someone else's private life).
    UnauthorizedPerspective,
    /// The request itself is malformed or asks for something meaningless.
    InvalidRequest,
    /// The request is fine but the world's state forbids it now (already answered, nothing to withdraw).
    StateConflict,
    /// The viewer does not and cannot know this; it is not a failure.
    UnavailableInformation,
    /// A save this build cannot open (too new, no longer upgradable, damaged, not a save).
    SaveIncompatible,
    /// The simulation is advancing or another operation is running; try again.
    SimulationBusy,
    /// The program failed.
    InternalError,
}

impl ErrorKind {
    pub const ALL: [ErrorKind; 8] = [
        ErrorKind::NotFound,
        ErrorKind::UnauthorizedPerspective,
        ErrorKind::InvalidRequest,
        ErrorKind::StateConflict,
        ErrorKind::UnavailableInformation,
        ErrorKind::SaveIncompatible,
        ErrorKind::SimulationBusy,
        ErrorKind::InternalError,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            ErrorKind::NotFound => "not_found",
            ErrorKind::UnauthorizedPerspective => "unauthorized_perspective",
            ErrorKind::InvalidRequest => "invalid_request",
            ErrorKind::StateConflict => "state_conflict",
            ErrorKind::UnavailableInformation => "unavailable_information",
            ErrorKind::SaveIncompatible => "save_incompatible",
            ErrorKind::SimulationBusy => "simulation_busy",
            ErrorKind::InternalError => "internal_error",
        }
    }

    /// The older three-way code (`bad_request`, `not_found`, `state`) that clients written before the categories still read.
    pub const fn legacy_code(self) -> &'static str {
        match self {
            ErrorKind::NotFound => "not_found",
            ErrorKind::InvalidRequest => "bad_request",
            _ => "state",
        }
    }

    pub const fn http_status(self) -> u16 {
        match self {
            ErrorKind::NotFound => 404,
            ErrorKind::InvalidRequest => 400,
            ErrorKind::UnauthorizedPerspective => 403,
            ErrorKind::InternalError => 500,
            ErrorKind::SimulationBusy => 423,
            _ => 409,
        }
    }

    /// Would the same call succeed a moment later?
    pub const fn retryable(self) -> bool {
        matches!(self, ErrorKind::SimulationBusy)
    }
}

impl Ts for ErrorKind {
    fn ts() -> String {
        "ErrorKind".into()
    }
}

contract! {
    /// What the client receives when a call fails.
    pub struct ErrorBody {
        pub kind: ErrorKind,
        /// The older three-way code, kept so existing clients keep working.
        pub code: String,
        pub message: String,
        pub retryable: bool,
    }
}

impl ErrorBody {
    pub fn of(e: &crate::model::ApiError) -> Self {
        let kind = e.kind();
        Self { kind, code: kind.legacy_code().into(), message: e.to_string(), retryable: kind.retryable() }
    }
}

// ------------------------------------------------------------------ semantic values

/// How a viewer knows one value (locked design 9.6): never a bare `0`, `false` or `null` standing for any of these.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Knowledge<T> {
    /// Known exactly: a public fact, or the viewer's own.
    #[serde(rename = "exact")]
    Known { v: T },
    /// Assessed: a best estimate inside a range.
    #[serde(rename = "range")]
    Estimated { lo: T, hi: T, v: T },
    /// Someone said so.
    Reported { v: T, source: String },
    /// Nobody the viewer has access to has looked.
    Unknown,
    /// Exists, and the viewer may not see it.
    Hidden,
}

pub const KNOWLEDGE_TS: &str = "export type Knowledge<T> =\n  | { kind: \"exact\"; v: T }\n  | { kind: \"range\"; lo: T; hi: T; v: T }\n  | { kind: \"reported\"; v: T; source: string }\n  | { kind: \"unknown\" }\n  | { kind: \"hidden\" };\n";

/// An attribute with its name and how the viewer knows it.
#[derive(Clone, Debug, Serialize)]
pub struct AttrRow {
    pub key: &'static str,
    pub label: &'static str,
    #[serde(flatten)]
    pub know: Knowledge<u8>,
}

impl Ts for AttrRow {
    fn ts() -> String {
        "AttrRow".into()
    }
}

pub const ATTR_ROW_TS: &str = "export type AttrRow = { key: string; label: string } & Knowledge<number>;\n";

// ------------------------------------------------------------------ hand-declared wire types (the table protocol and references)

impl Ts for crate::model::Ref {
    fn ts() -> String {
        "Ref".into()
    }
}
impl Ts for crate::model::Named {
    fn ts() -> String {
        "Named".into()
    }
}
impl Ts for crate::model::Tone {
    fn ts() -> String {
        "Tone".into()
    }
}

/// Wire types whose Rust definition carries serde attributes the generator does not read. They are written here once, and
/// `tests/contract.rs` checks real responses against these declarations field by field.
pub const CORE_TS: &str = r#"export type Kind = "person" | "club" | "comp" | "nation" | "match" | "team";
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
"#;

// ------------------------------------------------------------------ typed payloads

contract! {
    pub struct AppInfo {
        pub name: String,
        pub version: String,
        pub data_dir: String,
    }

    pub struct StopInfo {
        pub kind: String,
        pub text: String,
    }

    pub struct Job {
        pub running: bool,
        pub seq: u64,
        pub label: String,
        pub from: i32,
        pub target: Option<i32>,
        pub days_done: u32,
        pub days_total: Option<u32>,
        pub stop: Option<StopInfo>,
        pub stop_requested: bool,
        pub matches: u64,
    }

    pub struct Task {
        pub running: bool,
        pub seq: u64,
        pub label: String,
        pub error: Option<String>,
        pub report: Option<Value>,
    }

    pub struct StopsView {
        pub decisions: bool,
        pub matches: bool,
        pub major: bool,
    }

    pub struct SettingsView {
        pub conceal_mine: bool,
        pub stops: StopsView,
    }

    pub struct StatusView {
        pub open: bool,
        pub name: Option<String>,
        pub date: Option<i32>,
        pub revision: Option<u64>,
        pub perspective: Option<PerspectiveView>,
        pub job: Job,
        pub task: Task,
        pub settings: Option<SettingsView>,
        pub awaiting: Option<usize>,
        pub unrevealed: Option<usize>,
    }

    pub struct PositionView {
        pub code: String,
        pub fam: u8,
        pub level: String,
    }

    pub struct AttrGroupView {
        pub name: String,
        pub attrs: Vec<AttrRow>,
    }

    pub struct HiddenAttr {
        pub label: String,
        pub v: u8,
    }

    pub struct InternalAbility {
        pub ca: u8,
        pub pa: u8,
    }

    pub struct AttributesView {
        pub available: bool,
        pub reason: Option<String>,
        pub source: String,
        pub known: bool,
        pub groups: Vec<AttrGroupView>,
        pub positions: Vec<PositionView>,
        /// Present in the omniscient view only.
        pub hidden: Option<Vec<HiddenAttr>>,
        pub internal: Option<InternalAbility>,
        pub personality: Option<String>,
    }

    pub struct Evidence {
        pub text: String,
        pub date: i32,
    }

    /// Someone in the viewer's life: how it feels, in words, and what it rests on. Never the internal numbers (locked design 8.7).
    pub struct RelationshipRow {
        pub who: Named,
        pub role: String,
        pub label: String,
        pub tone: Tone,
        pub trust: String,
        pub respect: String,
        pub since: i32,
        pub last: i32,
        pub why: Option<Evidence>,
        pub evidence: Vec<Evidence>,
    }

    pub struct PeopleView {
        pub people: Vec<RelationshipRow>,
    }

    pub struct RumourRow {
        pub kind: String,
        pub date: i32,
        pub via: String,
        /// In words, never a percentage (locked design 8.5).
        pub sureness: String,
        pub text: String,
        pub club: Option<Named>,
        pub fee: Option<i64>,
    }

    pub struct RumoursView {
        pub rumours: Vec<RumourRow>,
    }

    pub struct PersonReq {
        pub id: u32,
    }
}

contract! {
    /// One step of a route, with why it happened where that was recorded.
    pub struct StepRow {
        pub date: i32,
        pub kind: String,
        pub target: Option<String>,
        pub why: Option<String>,
        /// False for a step that happened before reasons were kept: the reason is unknown, not absent.
        pub recorded: bool,
    }

    /// How a player came to exist in the world.
    pub struct CreationView {
        /// Unknown for a record derived from an older save.
        pub date: Option<i32>,
        pub region: String,
        pub provider: String,
        pub institution: Option<String>,
        pub age: Option<u8>,
        pub first_env: String,
        pub first_finder: Option<Named>,
        pub why: String,
        pub provenance: String,
    }

    pub struct EvidenceRow {
        pub rule: String,
        pub holds: bool,
        pub detail: Option<i32>,
    }

    /// One judgement: a rule set applied to a player, with the evidence and the reason.
    pub struct EligibilityRow {
        pub body_kind: String,
        pub body: String,
        pub eligible: bool,
        pub reason: String,
        pub evidence: Vec<EvidenceRow>,
    }

    pub struct TierRow {
        pub level: String,
        pub games: u32,
        pub proof: String,
    }

    /// What one organisation holds of one player.
    pub struct KnownBy {
        pub org: String,
        pub looks: u8,
        pub years: u8,
        pub how: String,
        pub first: i32,
        pub last: i32,
        pub first_by: Option<String>,
    }

    /// A coach's recommendation, with its cause.
    pub struct VouchView {
        pub from: String,
        pub basis: String,
        pub strength: String,
        pub credibility: String,
        pub date: i32,
    }

    pub struct WatchRow {
        pub org: String,
        pub by: String,
        pub games_left: u8,
        pub since: i32,
    }

    /// Who knows the player and on what basis. Omniscient view only: this is what organisations hold, not what the player knows.
    pub struct RecognitionView {
        pub standing: String,
        pub evidence: Vec<TierRow>,
        pub known_by: Vec<KnownBy>,
        pub vouch: Option<VouchView>,
        pub watching: Vec<WatchRow>,
        pub buzz: bool,
    }

    pub struct PathwayView {
        pub available: bool,
        pub reason: Option<String>,
        pub player: Named,
        pub steps: Vec<StepRow>,
        pub creation: Option<CreationView>,
        pub eligibility: Vec<EligibilityRow>,
        pub recognition: Option<RecognitionView>,
    }

    /// One quality of a place, in words, with what it means for a young player there.
    pub struct AspectRow {
        pub label: String,
        pub level: String,
        pub note: String,
    }

    /// Somewhere a child grows up: how visible and well coached it is, and who is near enough to notice.
    pub struct DistrictView {
        pub available: bool,
        pub reason: Option<String>,
        pub name: String,
        pub state: String,
        pub association: Option<String>,
        pub population_k: u32,
        pub aspects: Vec<AspectRow>,
        pub academies: Vec<Named>,
        pub universities: Vec<String>,
        pub schools: u32,
    }

    pub struct RegionOutputRow {
        pub region: String,
        pub kind: String,
        pub professionals: u32,
        pub top_tier: u32,
        pub internationals: u32,
        pub senior_apps: u32,
        pub value: i64,
    }

    pub struct RegionOutputView {
        pub available: bool,
        pub rows: Vec<RegionOutputRow>,
        pub note: String,
    }

    pub struct SegmentRegard {
        pub segment: String,
        pub level: u32,
        pub exports: u16,
        pub successes: u16,
        pub visits: u16,
        pub provenance: String,
    }

    pub struct MarketRow {
        pub name: String,
        pub nations: Vec<String>,
        pub segments: Vec<SegmentRegard>,
    }

    pub struct ExportView {
        pub available: bool,
        pub markets: Vec<MarketRow>,
        pub note: String,
    }

    pub struct CalendarRow {
        pub event: String,
        pub when: String,
    }

    /// How many reference records carry one provenance status.
    pub struct ReferenceStatusRow {
        pub label: String,
        pub records: u32,
    }

    /// A named derby or rivalry between two clubs of the world: a label, with no strength and no history.
    pub struct DerbyRow {
        pub name: String,
        pub a: String,
        pub b: String,
        pub kind: String,
        pub origin: String,
    }

    pub struct ScenarioView {
        pub available: bool,
        pub source: String,
        pub calendar: Vec<CalendarRow>,
        pub clubs_imported: u32,
        pub clubs_seeded: u32,
        pub clubs_generated: u32,
        pub clubs_unknown: u32,
        /// Whether reference data was read when this world was built (a world from an older save, or another builder, says no).
        pub reference_loaded: bool,
        pub reference_files: u32,
        pub reference_records: u32,
        pub reference_by_status: Vec<ReferenceStatusRow>,
        /// Problems found reading the reference data: malformed, unknown or contradictory records, and pack clubs it does not know.
        pub reference_findings: u32,
        pub finding_samples: Vec<String>,
        /// Clubs matched to a reference club by stable id or exact name and state.
        pub clubs_matched: u32,
        /// Clubs that took the place of a made-up one: a real name, and where the record allows it a ground and a founding year.
        pub clubs_from_reference: u32,
        pub derbies: Vec<DerbyRow>,
        pub note: String,
    }
}

// ------------------------------------------------------------------ requests (commands)

request! {
    /// Stop inhabiting. The public view unless the omniscient debug view is asked for by name (`public: false` is the older way).
    pub struct ObserveReq {
        pub omniscient: Option<bool>,
        pub public: Option<bool>,
    }

    pub struct InhabitReq {
        pub person: u32,
    }

    pub struct StopsReq {
        pub decisions: Option<bool>,
        pub matches: Option<bool>,
        pub major: Option<bool>,
    }

    pub struct SettingsReq {
        pub conceal_mine: Option<bool>,
        pub stops: Option<StopsReq>,
    }

    pub struct SaveReq {
        pub file: Option<String>,
    }

    pub struct LoadReq {
        pub file: String,
        pub backup: Option<bool>,
    }

    pub struct FileReq {
        pub file: String,
    }

    /// An answer to one of the inhabited person's decisions (`id` as the inbox gives it, `d` and a number).
    pub struct AnswerReq {
        pub id: String,
        pub choice: u8,
    }

    pub struct ReplyReq {
        pub message: u32,
        pub key: String,
    }

    pub struct IdReq {
        pub id: u32,
    }

    pub struct GoalReq {
        pub kind: Option<String>,
        pub target: Option<u64>,
        pub text: Option<String>,
    }

    pub struct GoalDoneReq {
        pub i: u32,
        pub remove: Option<bool>,
    }

    pub struct NoteReq {
        pub text: Option<String>,
    }

    pub struct IndexReq {
        pub i: u32,
    }

    pub struct FocusReq {
        pub kind: Option<String>,
        pub value: Option<String>,
    }

    pub struct PlanReq {
        pub intensity: Option<String>,
        pub extra: Option<u64>,
        pub recovery: Option<u64>,
        pub focus: Option<FocusReq>,
    }

    pub struct FollowReq {
        pub club: u32,
        pub follow: Option<bool>,
    }

    pub struct RevealReq {
        pub uid: u64,
    }

    pub struct RouteReq {
        pub start: String,
        pub district: Option<u32>,
        pub first: Option<String>,
        pub last: Option<String>,
    }

    pub struct CreatePersonReq {
        pub first: Option<String>,
        pub last: Option<String>,
        pub age: Option<u64>,
        pub pos: Option<String>,
        pub club: Option<u32>,
        pub nation: Option<u32>,
    }

    /// Hours a week for each part of life; one left out keeps its current share.
    pub struct RoutineHours {
        pub rest: Option<u64>,
        pub recovery: Option<u64>,
        pub family: Option<u64>,
        pub partner: Option<u64>,
        pub social: Option<u64>,
        pub study: Option<u64>,
        pub hobbies: Option<u64>,
        pub media: Option<u64>,
        pub nightlife: Option<u64>,
        pub language: Option<u64>,
    }
}

request_enum! {
    /// Something the inhabited person decides to do (`me.act`). Each action names exactly the choices it takes; none carries a value
    /// the world owns (a wage, a fee, an ability).
    pub enum ActReq tag "action" {
        Meet "meet" { with: u32, topic: String, tone: Option<String> },
        TransferRequest "transfer_request" {},
        WithdrawRequest "withdraw_request" {},
        Routine "routine" { hours: Option<RoutineHours> },
        Lifestyle "lifestyle" { value: String },
        HireAgent "hire_agent" { agent: u32 },
        DropAgent "drop_agent" {},
        Retire "retire" {},
        Unretire "unretire" {},
        SeekJob "seek_job" { role: String },
        Dating "dating" { open: Option<bool> },
        Partner "partner" { ask: String },
        Amateur "amateur" {},
        Nation "nation" { nation: u32 },
        RetireInternational "retire_international" {},
        Pain "pain" { on: Option<bool> },
        Mentor "mentor" { person: u32 },
        Press "press" { about: u32, stance: String },
        Enrol "enrol" { course: String },
        MoveHome "move_home" { buy: Option<bool>, quality: Option<u64> },
        Helper "helper" { helper: String, quality: Option<u64> },
        DismissHelper "dismiss_helper" { helper: String },
        Giving "giving" { pct: Option<u64>, community: Option<u64> },
        Foundation "foundation" {},
        Invest "invest" { amount: Option<i64>, risk: Option<u64> },
        Career "career" { path: String },
        LeaveCareer "leave_career" {},
        Post "post" { concept: String, about: Option<u32>, reply_to: Option<u32>, quote_of: Option<u32> },
    }
}

contract! {
    /// A state a person would describe in words (condition, stress, a bond): the word, and its place among the words there are
    /// (`step` of `steps`, the best being `steps`) for drawing a gauge. Never the engine's number behind it (locked design 8.5).
    pub struct Band {
        pub label: String,
        pub step: u8,
        pub steps: u8,
    }
}

contract! {
    /// A command that has nothing to report but that it was done.
    pub struct Done {
        pub ok: bool,
    }

    /// A long operation (building, loading) has begun; `world.status` follows it.
    pub struct Started {
        pub started: bool,
    }

    pub struct Saved {
        pub file: String,
    }

    pub struct Closed {
        pub closed: bool,
    }

    pub struct Deleted {
        pub deleted: bool,
    }

    pub struct StopRequested {
        pub requested: bool,
    }

    pub struct Followed {
        pub followed: bool,
    }

    pub struct Revealed {
        pub revealed: u64,
    }

    /// Every hidden result revealed at once (`revealed` is `"all"`).
    pub struct RevealedAll {
        pub revealed: String,
    }

    /// A person made (or begun on a route) and now inhabited.
    pub struct Created {
        pub person: u32,
    }

    /// What training concentrates on: `general` (no value), an attribute `group`, one `attribute` or a `position`.
    pub struct FocusView {
        pub kind: String,
        pub value: Option<String>,
    }

    pub struct PlanView {
        pub focus: FocusView,
        pub intensity: String,
        pub extra: u8,
        pub recovery: u8,
    }

    pub struct PlanSet {
        pub plan: PlanView,
        pub applies: String,
    }
}

contract! {
    /// How a minutes promise is going: the share of the team's minutes played, against the share promised.
    pub struct PromiseProgress {
        pub actual: f32,
        pub promised: f32,
    }

    pub struct PromiseRow {
        pub id: u32,
        /// The viewer made it (otherwise it was made to the viewer).
        pub mine: bool,
        pub with: Named,
        pub text: String,
        pub made: i32,
        pub due: i32,
        pub progress: Option<PromiseProgress>,
        /// open, kept, broken or void.
        pub state: String,
        pub days_left: i32,
    }

    pub struct PromisesView {
        pub promises: Vec<PromiseRow>,
    }

    pub struct GoalProgress {
        pub now: u16,
        pub target: u16,
    }

    pub struct GoalRow {
        pub i: u32,
        pub text: String,
        pub pinned: i32,
        pub done: Option<i32>,
        /// appearances, goals, top_flight or personal.
        pub kind: String,
        pub progress: Option<GoalProgress>,
    }

    pub struct NoteRow {
        pub i: u32,
        pub date: i32,
        pub text: String,
    }

    /// Someone the human has inhabited, and when.
    pub struct InhabitedRow {
        pub who: Named,
        pub from: i32,
        pub to: Option<i32>,
    }

    pub struct JournalView {
        pub goals: Vec<GoalRow>,
        pub notes: Vec<NoteRow>,
        pub history: Vec<InhabitedRow>,
    }

    pub struct AgentRow {
        pub id: u32,
        pub who: Named,
        pub fee_pct: u8,
        pub since: i32,
        pub until: i32,
        pub satisfaction: Band,
        pub reputation: u16,
        pub clients: u32,
        pub base: String,
    }

    pub struct AgentView {
        pub agent: Option<AgentRow>,
        /// False for someone who does not play: they have no agent to show.
        pub player: bool,
    }

    /// One story about the viewer, in full.
    pub struct OwnStoryView {
        pub id: u32,
        pub date: i32,
        pub outlet: String,
        pub headline: String,
        pub body: String,
    }
}

/// A small picture beside a note: a run of numbers, a run of results, or two things side by side.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InsightVisual {
    Sparkline { label: String, unit: String, values: Vec<f32> },
    Sequence { label: String, unit: String, values: Vec<String> },
    Comparison { label: String, unit: String, names: Vec<String>, values: Vec<i32> },
}

impl Ts for InsightVisual {
    fn ts() -> String {
        "InsightVisual".into()
    }
}

pub const INSIGHT_VISUAL_TS: &str = "export type InsightVisual =\n  | { kind: \"sparkline\"; label: string; unit: string; values: number[] }\n  | { kind: \"sequence\"; label: string; unit: string; values: string[] }\n  | { kind: \"comparison\"; label: string; unit: string; names: string[]; values: number[] };\n";

contract! {
    /// One note about a person, club, competition or match: what it says, and what it rests on (`basis`).
    pub struct InsightItem {
        pub kind: String,
        pub tone: Tone,
        pub title: String,
        pub text: String,
        pub basis: String,
        pub link: Option<Named>,
        pub visual: Option<InsightVisual>,
    }

    pub struct InsightsView {
        pub items: Vec<InsightItem>,
        /// Notes there were before `limit` cut the list.
        pub total: u32,
        /// Results the viewer has not revealed, which the notes leave out.
        pub held: u32,
    }
}

/// The picture at the head of a story: the result it reports, the club or person it is about, or just the paper's type.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoryGraphic {
    Result {
        #[serde(rename = "match")]
        fixture: crate::model::Ref,
        home: Named,
        away: Named,
        score: Vec<u8>,
    },
    Club {
        club: Named,
    },
    Person {
        person: Named,
    },
    Type,
}

impl Ts for StoryGraphic {
    fn ts() -> String {
        "StoryGraphic".into()
    }
}

pub const STORY_GRAPHIC_TS: &str = "export type StoryGraphic =\n  | { kind: \"result\"; match: Ref; home: Named; away: Named; score: number[] }\n  | { kind: \"club\"; club: Named }\n  | { kind: \"person\"; person: Named }\n  | { kind: \"type\" };\n";

contract! {
    /// A story as a list shows it. A story about a result the viewer has not revealed never enters a list; opened directly, its
    /// headline says it is held back.
    pub struct StorySummary {
        pub id: u32,
        pub date: i32,
        pub outlet: String,
        pub headline: String,
        pub kind: String,
        /// What sort of claim it makes (fact, rumour, opinion ...), in words.
        pub claim: String,
        pub about_you: bool,
        pub following: bool,
        pub graphic: StoryGraphic,
        pub subject: Option<Named>,
    }

    /// A story in full: the summary and its text.
    pub struct StoryFull {
        pub id: u32,
        pub date: i32,
        pub outlet: String,
        pub headline: String,
        pub kind: String,
        pub claim: String,
        pub about_you: bool,
        pub following: bool,
        pub graphic: StoryGraphic,
        pub subject: Option<Named>,
        pub body: String,
    }

    pub struct NewsFeedView {
        pub stories: Vec<StorySummary>,
        /// for_you, following or world.
        pub filter: String,
    }
}

/// A declared payload as it goes on the wire.
pub fn wire<T: Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

contract! {
    /// What `me.act` answers: the action in words, and when the world applies it.
    pub struct ActDone {
        pub ok: bool,
        pub text: String,
        pub applies: String,
    }
}

/// Which view is looking. The omniscient view is the debug view and says so.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PerspectiveView {
    #[serde(rename = "observer")]
    Observer { omniscient: bool },
    Public { omniscient: bool },
    Inhabit { person: u32, name: String, club: Option<String> },
}

impl Ts for PerspectiveView {
    fn ts() -> String {
        "PerspectiveView".into()
    }
}

pub const PERSPECTIVE_TS: &str = "export type PerspectiveView =\n  | { mode: \"observer\"; omniscient: boolean }\n  | { mode: \"public\"; omniscient: boolean }\n  | { mode: \"inhabit\"; person: number; name: string; club: string | null };\n";

// ------------------------------------------------------------------ the manifest

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MethodKind {
    /// Asks for state the viewer may see. Changes nothing; safe to repeat.
    Query,
    /// Expresses an intent (or manages the session). Never carries authoritative world state.
    Command,
}

#[derive(Clone, Copy, Debug)]
pub struct MethodSpec {
    pub name: &'static str,
    pub kind: MethodKind,
    /// The TypeScript request and response types, or `None` while the payload is not yet a declared contract type.
    pub request: Option<&'static str>,
    pub response: Option<&'static str>,
}

const fn q(name: &'static str) -> MethodSpec {
    MethodSpec { name, kind: MethodKind::Query, request: None, response: None }
}
const fn c(name: &'static str) -> MethodSpec {
    MethodSpec { name, kind: MethodKind::Command, request: None, response: None }
}
const fn typed(mut m: MethodSpec, req: Option<&'static str>, res: &'static str) -> MethodSpec {
    m.request = req;
    m.response = Some(res);
    m
}

/// Every method of `Api::call`, as a query or a command. A source-level test keeps this list and the dispatcher in step.
pub fn manifest() -> Vec<MethodSpec> {
    vec![
        typed(q("app.info"), None, "AppInfo"),
        typed(q("world.status"), None, "StatusView"),
        typed(c("world.new"), None, "Started"),
        q("world.inspect_import"),
        q("world.datasets"),
        q("world.saves"),
        typed(c("world.save"), Some("SaveReq"), "Saved"),
        typed(c("world.load"), Some("LoadReq"), "Started"),
        typed(c("world.close"), None, "Closed"),
        typed(c("world.delete_save"), Some("FileReq"), "Deleted"),
        typed(c("settings.set"), Some("SettingsReq"), "Done"),
        typed(c("advance.start"), None, "StatusView"),
        typed(c("advance.stop"), None, "StopRequested"),
        typed(c("persp.observe"), Some("ObserveReq"), "Done"),
        typed(c("persp.inhabit"), Some("InhabitReq"), "Done"),
        typed(c("person.create"), Some("CreatePersonReq"), "Created"),
        q("route.options"),
        typed(c("route.begin"), Some("RouteReq"), "Created"),
        typed(q("table.query"), Some("TableReq"), "TableResp"),
        q("search"),
        q("overview"),
        q("world.pulse"),
        typed(q("news.feed"), None, "NewsFeedView"),
        typed(q("news.story"), Some("IdReq"), "StoryFull"),
        q("diagnostics"),
        q("capabilities"),
        q("person"),
        typed(q("person.attributes"), Some("PersonReq"), "AttributesView"),
        q("crest.colors"),
        q("comp.overview"),
        typed(q("insight.club"), None, "InsightsView"),
        typed(q("insight.comp"), None, "InsightsView"),
        typed(q("insight.match"), None, "InsightsView"),
        typed(q("insight.person"), None, "InsightsView"),
        q("club"),
        q("club.systems"),
        typed(c("club.follow"), Some("FollowReq"), "Followed"),
        q("comp"),
        q("nation"),
        q("match"),
        q("match.watch"),
        typed(c("match.reveal"), Some("RevealReq"), "Revealed"),
        typed(c("match.reveal_all"), None, "RevealedAll"),
        q("me.today"),
        typed(c("me.viewed"), None, "Done"),
        q("me.messages"),
        q("me.inbox"),
        q("me.thread"),
        typed(c("me.thread_read"), Some("IdReq"), "Done"),
        typed(c("me.reply"), Some("ReplyReq"), "ActDone"),
        q("me.message"),
        typed(c("me.answer"), Some("AnswerReq"), "Done"),
        typed(c("me.act"), Some("ActReq"), "ActDone"),
        q("me.options"),
        q("me.self"),
        q("me.life"),
        q("person.life"),
        typed(q("me.people"), None, "PeopleView"),
        typed(q("me.promises"), None, "PromisesView"),
        typed(q("me.rumours"), None, "RumoursView"),
        q("me.press"),
        q("me.feed"),
        q("social.thread"),
        typed(q("me.story"), Some("IdReq"), "OwnStoryView"),
        typed(q("me.agent"), None, "AgentView"),
        typed(q("me.journal"), None, "JournalView"),
        typed(c("me.goal"), Some("GoalReq"), "Done"),
        typed(c("me.goal_done"), Some("GoalDoneReq"), "Done"),
        typed(c("me.note"), Some("NoteReq"), "Done"),
        typed(c("me.note_remove"), Some("IndexReq"), "Done"),
        q("me.calendar"),
        q("me.football"),
        typed(c("me.plan"), Some("PlanReq"), "PlanSet"),
        q("me.contract"),
        typed(q("pathway.player"), Some("PersonReq"), "PathwayView"),
        typed(q("ecosystem.regions"), None, "RegionOutputView"),
        typed(q("ecosystem.export"), None, "ExportView"),
        typed(q("ecosystem.scenario"), None, "ScenarioView"),
        typed(q("ecosystem.district"), None, "DistrictView"),
        q("database.sources"),
        c("database.attach"),
        q("database.query"),
    ]
}

pub fn kind_of(method: &str) -> Option<MethodKind> {
    manifest().into_iter().find(|m| m.name == method).map(|m| m.kind)
}

// ------------------------------------------------------------------ TypeScript

/// Every declaration of the contract, as one TypeScript module. `app/src/contract.generated.ts` must equal this.
pub fn typescript() -> String {
    let mut out = String::new();
    out.push_str("// GENERATED from crates/pw-view/src/contract.rs. Do not edit by hand.\n");
    out.push_str("// Regenerate with: UPDATE_CONTRACT=1 cargo test -p pw-view --test contract\n\n");
    out.push_str("export type ErrorKind =\n");
    for k in ErrorKind::ALL {
        out.push_str(&format!("  | \"{}\"\n", k.name()));
    }
    out.push_str(";\n\n");
    out.push_str(CORE_TS);
    out.push('\n');
    out.push_str(KNOWLEDGE_TS);
    out.push_str(ATTR_ROW_TS);
    out.push_str(PERSPECTIVE_TS);
    out.push_str(INSIGHT_VISUAL_TS);
    out.push_str(STORY_GRAPHIC_TS);
    out.push('\n');
    for d in declarations() {
        out.push_str(&d);
        out.push('\n');
    }
    out.push_str("/** Every method: whether it is a query or a command, and its request and response types where they are declared. */\n");
    out.push_str("export interface ApiMethods {\n");
    for m in manifest() {
        let kind = if m.kind == MethodKind::Query { "query" } else { "command" };
        out.push_str(&format!("  \"{}\": {{ kind: \"{kind}\"; req: {}; res: {} }};\n", m.name, m.request.unwrap_or("Record<string, unknown>"), m.response.unwrap_or("unknown")));
    }
    out.push_str("}\n\n");
    out.push_str("/** The methods whose response type is declared here. */\n");
    out.push_str("export type TypedMethod =\n");
    for m in manifest().iter().filter(|m| m.response.is_some()) {
        out.push_str(&format!("  | \"{}\"\n", m.name));
    }
    out.push_str(";\n");
    out
}

/// The interfaces declared with `contract!`, in dependency order.
pub fn declarations() -> Vec<String> {
    vec![
        ErrorBody::declaration(),
        AppInfo::declaration(),
        StopInfo::declaration(),
        Job::declaration(),
        Task::declaration(),
        StopsView::declaration(),
        SettingsView::declaration(),
        StatusView::declaration(),
        PositionView::declaration(),
        AttrGroupView::declaration(),
        HiddenAttr::declaration(),
        InternalAbility::declaration(),
        AttributesView::declaration(),
        Evidence::declaration(),
        RelationshipRow::declaration(),
        PeopleView::declaration(),
        RumourRow::declaration(),
        RumoursView::declaration(),
        PersonReq::declaration(),
        StepRow::declaration(),
        CreationView::declaration(),
        EvidenceRow::declaration(),
        EligibilityRow::declaration(),
        TierRow::declaration(),
        KnownBy::declaration(),
        VouchView::declaration(),
        WatchRow::declaration(),
        RecognitionView::declaration(),
        PathwayView::declaration(),
        AspectRow::declaration(),
        DistrictView::declaration(),
        RegionOutputRow::declaration(),
        RegionOutputView::declaration(),
        SegmentRegard::declaration(),
        MarketRow::declaration(),
        ExportView::declaration(),
        CalendarRow::declaration(),
        ReferenceStatusRow::declaration(),
        DerbyRow::declaration(),
        ScenarioView::declaration(),
        ObserveReq::declaration(),
        InhabitReq::declaration(),
        StopsReq::declaration(),
        SettingsReq::declaration(),
        SaveReq::declaration(),
        LoadReq::declaration(),
        FileReq::declaration(),
        AnswerReq::declaration(),
        ReplyReq::declaration(),
        IdReq::declaration(),
        GoalReq::declaration(),
        GoalDoneReq::declaration(),
        NoteReq::declaration(),
        IndexReq::declaration(),
        FocusReq::declaration(),
        PlanReq::declaration(),
        FollowReq::declaration(),
        RevealReq::declaration(),
        RouteReq::declaration(),
        CreatePersonReq::declaration(),
        RoutineHours::declaration(),
        ActReq::declaration(),
        Band::declaration(),
        Done::declaration(),
        Started::declaration(),
        Saved::declaration(),
        Closed::declaration(),
        Deleted::declaration(),
        StopRequested::declaration(),
        Followed::declaration(),
        Revealed::declaration(),
        RevealedAll::declaration(),
        Created::declaration(),
        FocusView::declaration(),
        PlanView::declaration(),
        PlanSet::declaration(),
        PromiseProgress::declaration(),
        PromiseRow::declaration(),
        PromisesView::declaration(),
        GoalProgress::declaration(),
        GoalRow::declaration(),
        NoteRow::declaration(),
        InhabitedRow::declaration(),
        JournalView::declaration(),
        AgentRow::declaration(),
        AgentView::declaration(),
        OwnStoryView::declaration(),
        InsightItem::declaration(),
        InsightsView::declaration(),
        StorySummary::declaration(),
        StoryFull::declaration(),
        NewsFeedView::declaration(),
        ActDone::declaration(),
    ]
}

/// The field names of an interface declared in TypeScript text: (required, optional).
pub fn interface_fields(ts: &str, name: &str) -> Option<(Vec<String>, Vec<String>)> {
    let start = ts.find(&format!("export interface {name} "))?;
    let body = &ts[start..];
    let body = &body[body.find('{')? + 1..];
    let body = &body[..body.find("\n}")?];
    let (mut req, mut opt) = (Vec::new(), Vec::new());
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with("/**") || line.starts_with("//") || line.starts_with('*') || !line.contains(':') {
            continue;
        }
        let (field, ty) = line.split_once(':').unwrap_or(("", ""));
        let field = field.trim();
        // A field that may be `null` or is marked `?` may be absent from a payload built without it; the rest must be there.
        if let Some(f) = field.strip_suffix('?') {
            opt.push(f.to_string());
        } else if ty.contains("| null") {
            opt.push(field.to_string());
        } else {
            req.push(field.to_string());
        }
    }
    Some((req, opt))
}
