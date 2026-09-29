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
        c("world.new"),
        q("world.inspect_import"),
        q("world.saves"),
        c("world.save"),
        c("world.load"),
        c("world.close"),
        c("world.delete_save"),
        c("settings.set"),
        c("advance.start"),
        c("advance.stop"),
        c("persp.observe"),
        c("persp.inhabit"),
        c("person.create"),
        q("route.options"),
        c("route.begin"),
        typed(q("table.query"), Some("TableReq"), "TableResp"),
        q("search"),
        q("overview"),
        q("world.pulse"),
        q("news.feed"),
        q("news.story"),
        q("diagnostics"),
        q("capabilities"),
        q("person"),
        typed(q("person.attributes"), Some("PersonReq"), "AttributesView"),
        q("crest.colors"),
        q("comp.overview"),
        q("insight.club"),
        q("insight.comp"),
        q("insight.match"),
        q("insight.person"),
        q("club"),
        q("club.systems"),
        c("club.follow"),
        q("comp"),
        q("nation"),
        q("match"),
        q("match.watch"),
        c("match.reveal"),
        c("match.reveal_all"),
        q("me.today"),
        c("me.viewed"),
        q("me.messages"),
        q("me.inbox"),
        q("me.thread"),
        c("me.thread_read"),
        c("me.reply"),
        q("me.message"),
        c("me.answer"),
        c("me.act"),
        q("me.options"),
        q("me.self"),
        q("me.life"),
        q("person.life"),
        typed(q("me.people"), None, "PeopleView"),
        q("me.promises"),
        typed(q("me.rumours"), None, "RumoursView"),
        q("me.press"),
        q("me.feed"),
        q("social.thread"),
        q("me.story"),
        q("me.agent"),
        q("me.journal"),
        c("me.goal"),
        c("me.goal_done"),
        c("me.note"),
        c("me.note_remove"),
        q("me.calendar"),
        q("me.football"),
        c("me.plan"),
        q("me.contract"),
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
