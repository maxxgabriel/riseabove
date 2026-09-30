//! Wire types shared by every page: entity references, table cells and the
//! generic table protocol. Everything here is plain data the client renders.

use pw_core::{ClubId, CompId, NationId, PersonId};
use serde::{Deserialize, Serialize};

/// A stable pointer to a thing the client can navigate to.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ref {
    pub k: &'static str,
    pub id: u32,
}

impl Ref {
    pub fn person(id: PersonId) -> Self {
        Self { k: "person", id: id.0 }
    }
    pub fn club(id: ClubId) -> Self {
        Self { k: "club", id: id.0 }
    }
    pub fn comp(id: CompId) -> Self {
        Self { k: "comp", id: id.0 }
    }
    pub fn nation(id: NationId) -> Self {
        Self { k: "nation", id: id.0 }
    }
    pub fn fixture(uid: u64) -> Self {
        Self { k: "match", id: uid as u32 }
    }
}

/// A reference with its display label.
#[derive(Clone, Debug, Serialize)]
pub struct Named {
    pub k: &'static str,
    pub id: u32,
    pub name: String,
}

impl Named {
    pub fn new(r: Ref, name: impl Into<String>) -> Self {
        Self { k: r.k, id: r.id, name: name.into() }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Pos,
    Neg,
    Warn,
    Muted,
    Info,
}

/// One table cell. `n` is the raw value the client formats by column kind,
/// `s` is text, `u` marks a value the viewer cannot know.
#[derive(Clone, Debug, Serialize, Default)]
pub struct Cell {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<Ref>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub u: bool,
    /// Fraction 0..=1 drawn as an inline bar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bar: Option<f32>,
    /// Secondary line or tooltip text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    /// Range `[lo, hi]` for assessed values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<[f32; 2]>,
    /// A sentence made of text and links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<Part>>,
}

/// A run of text, optionally a link.
#[derive(Clone, Debug, Serialize)]
pub struct Part {
    pub t: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<Ref>,
    /// A money amount, formatted by the client with the chosen currency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m: Option<f64>,
    /// A date (days since 1970), formatted by the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<i32>,
}

impl Part {
    pub fn t(t: impl Into<String>) -> Self {
        Self { t: t.into(), r: None, m: None, d: None }
    }
    /// A run of text that links to something; a reference to nothing is plain text.
    pub fn l(r: Ref, t: impl Into<String>) -> Self {
        Self { t: t.into(), r: (r.id != u32::MAX).then_some(r), m: None, d: None }
    }
    pub fn money(m: i64) -> Self {
        Self { t: String::new(), r: None, m: Some(m as f64), d: None }
    }
    pub fn date(d: pw_core::Date) -> Self {
        Self { t: String::new(), r: None, m: None, d: Some(d.0) }
    }
}

impl Cell {
    pub fn num(n: impl Into<f64>) -> Self {
        Self { n: Some(n.into()), ..Default::default() }
    }
    pub fn text(s: impl Into<String>) -> Self {
        Self { s: Some(s.into()), ..Default::default() }
    }
    /// A link to something. A reference to nothing (an id that means "none") is plain text: it would open a page that does not exist.
    pub fn link(r: Ref, s: impl Into<String>) -> Self {
        let r = (r.id != u32::MAX).then_some(r);
        Self { s: Some(s.into()), r, ..Default::default() }
    }
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn tone(mut self, t: Tone) -> Self {
        self.tone = Some(t);
        self
    }
    pub fn with_sub(mut self, s: impl Into<String>) -> Self {
        self.sub = Some(s.into());
        self
    }
    pub fn with_bar(mut self, f: f32) -> Self {
        self.bar = Some(f.clamp(0.0, 1.0));
        self
    }
    pub fn with_ref(mut self, r: Ref) -> Self {
        self.r = Some(r);
        self
    }
    pub fn with_parts(mut self, parts: Vec<Part>) -> Self {
        self.parts = Some(parts);
        self
    }
    pub fn with_num(mut self, n: f64) -> Self {
        self.n = Some(n);
        self
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Fmt {
    Text,
    Int,
    Dec1,
    Dec2,
    Money,
    Date,
    Pct,
    Ordinal,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Left,
    Right,
    Center,
}

/// Column metadata. `presets` lists the named views the column belongs to.
#[derive(Clone, Debug, Serialize)]
pub struct Col {
    pub key: &'static str,
    pub label: &'static str,
    pub short: &'static str,
    pub fmt: Fmt,
    pub align: Align,
    pub w: u16,
    pub presets: &'static [&'static str],
    pub sortable: bool,
    /// Longer explanation shown in a header tooltip.
    pub help: &'static str,
}

impl Col {
    pub const fn new(key: &'static str, label: &'static str, fmt: Fmt, w: u16, presets: &'static [&'static str]) -> Self {
        let align = match fmt {
            Fmt::Text => Align::Left,
            _ => Align::Right,
        };
        Self { key, label, short: label, fmt, align, w, presets, sortable: true, help: "" }
    }
    pub const fn help(mut self, s: &'static str) -> Self {
        self.help = s;
        self
    }
    pub const fn left(mut self) -> Self {
        self.align = Align::Left;
        self
    }
    pub const fn center(mut self) -> Self {
        self.align = Align::Center;
        self
    }
    pub const fn nosort(mut self) -> Self {
        self.sortable = false;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct SortSpec {
    pub key: String,
    #[serde(default)]
    pub desc: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TableReq {
    pub table: String,
    #[serde(default)]
    pub filters: serde_json::Value,
    #[serde(default)]
    pub sort: Option<SortSpec>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Restrict output to these columns; default is the source's own default set.
    #[serde(default)]
    pub columns: Option<Vec<String>>,
    #[serde(default)]
    pub preset: Option<String>,
}

fn default_limit() -> usize {
    60
}

#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub id: u32,
    pub cells: Vec<Cell>,
    /// Navigate here on activation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<Ref>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TableResp {
    /// Every column the source offers.
    pub all_columns: Vec<Col>,
    /// Keys of the columns in `rows`, in order.
    pub columns: Vec<&'static str>,
    pub presets: Vec<&'static str>,
    pub total: usize,
    pub offset: usize,
    pub rows: Vec<Row>,
    /// Text describing what is missing or withheld from this view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub revision: u64,
    pub sort: Option<(String, bool)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Bad(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    State(String),
    /// The viewer's perspective does not allow it.
    #[error("{0}")]
    Unauthorized(String),
    /// The viewer cannot know this.
    #[error("{0}")]
    Unavailable(String),
    /// A save this build cannot open.
    #[error("{0}")]
    SaveIncompatible(String),
    /// The simulation is busy.
    #[error("{0}")]
    Busy(String),
    #[error("{0}")]
    Internal(String),
}

impl ApiError {
    /// The category a client acts on (locked design 9.5).
    pub fn kind(&self) -> crate::contract::ErrorKind {
        use crate::contract::ErrorKind as K;
        match self {
            ApiError::Bad(_) => K::InvalidRequest,
            ApiError::NotFound(_) => K::NotFound,
            ApiError::State(_) => K::StateConflict,
            ApiError::Unauthorized(_) => K::UnauthorizedPerspective,
            ApiError::Unavailable(_) => K::UnavailableInformation,
            ApiError::SaveIncompatible(_) => K::SaveIncompatible,
            ApiError::Busy(_) => K::SimulationBusy,
            ApiError::Internal(_) => K::InternalError,
        }
    }

    /// The older three-way code (`bad_request`, `not_found`, `state`): kept so existing clients and tests read the same.
    pub fn code(&self) -> &'static str {
        self.kind().legacy_code()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

/// How sure someone is of what they have heard, in words: uncertain knowledge is never turned into a false percentage (locked design 8.5).
pub fn sureness(confidence: u8) -> &'static str {
    match confidence {
        90.. => "almost certain",
        70..=89 => "fairly sure",
        50..=69 => "only half sure",
        30..=49 => "doubtful",
        _ => "hardly sure at all",
    }
}
