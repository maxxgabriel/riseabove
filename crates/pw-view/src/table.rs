//! The generic table engine. A `Source` says what rows exist, how to sort
//! them and what each cell shows for the current viewer; the engine handles
//! sorting, paging and column selection identically for every table.

use std::cmp::Ordering;

use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Row, Ref, TableReq, TableResp, Tone};

#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    /// Unknown or not applicable: always sorts after known values.
    None,
    Num(f64),
    Text(String),
}

impl Key {
    pub fn text(s: impl AsRef<str>) -> Self {
        Key::Text(s.as_ref().to_lowercase())
    }
}

impl From<f64> for Key {
    fn from(v: f64) -> Self {
        Key::Num(v)
    }
}

pub trait Source {
    type Prep;
    fn cols(&self, c: &Ctx) -> Vec<Col>;
    /// Preset shown when the client does not ask for one.
    fn default_preset(&self) -> &'static str {
        "general"
    }
    fn default_sort(&self) -> (&'static str, bool);
    fn prep(&self, c: &Ctx, f: &Value) -> Self::Prep;
    fn ids(&self, c: &Ctx, p: &Self::Prep, f: &Value) -> Vec<u32>;
    fn key(&self, c: &Ctx, p: &Self::Prep, id: u32, col: &str) -> Key;
    fn cell(&self, c: &Ctx, p: &Self::Prep, id: u32, col: &str) -> Cell;
    fn open(&self, _c: &Ctx, _p: &Self::Prep, _id: u32) -> Option<Ref> {
        None
    }
    fn row_tone(&self, _c: &Ctx, _p: &Self::Prep, _id: u32) -> Option<Tone> {
        None
    }
    /// Sources that come pre-ordered (standings) skip sorting when no sort is requested.
    fn natural_order(&self) -> bool {
        false
    }
    fn note(&self, _c: &Ctx, _p: &Self::Prep, _f: &Value) -> Option<String> {
        None
    }
}

fn cmp_keys(a: &Key, b: &Key, desc: bool) -> Ordering {
    match (a, b) {
        (Key::None, Key::None) => Ordering::Equal,
        (Key::None, _) => Ordering::Greater,
        (_, Key::None) => Ordering::Less,
        (Key::Num(x), Key::Num(y)) => {
            let o = x.partial_cmp(y).unwrap_or(Ordering::Equal);
            if desc { o.reverse() } else { o }
        }
        (Key::Text(x), Key::Text(y)) => {
            let o = x.cmp(y);
            if desc { o.reverse() } else { o }
        }
        (Key::Num(_), Key::Text(_)) => Ordering::Less,
        (Key::Text(_), Key::Num(_)) => Ordering::Greater,
    }
}

pub fn run<S: Source>(src: &S, c: &Ctx, req: &TableReq) -> TableResp {
    let cols = src.cols(c);
    let prep = src.prep(c, &req.filters);
    let mut ids = src.ids(c, &prep, &req.filters);
    let total = ids.len();

    let (sort_key, desc) = match &req.sort {
        Some(s) if cols.iter().any(|x| x.key == s.key && x.sortable) => (Some(s.key.clone()), s.desc),
        _ if src.natural_order() => (None, false),
        _ => {
            let (k, d) = src.default_sort();
            (Some(k.to_string()), d)
        }
    };
    if let Some(key) = &sort_key {
        let mut keyed: Vec<(Key, u32)> = ids.iter().map(|&id| (src.key(c, &prep, id, key), id)).collect();
        keyed.sort_by(|a, b| cmp_keys(&a.0, &b.0, desc).then(a.1.cmp(&b.1)));
        ids = keyed.into_iter().map(|(_, id)| id).collect();
    }

    let preset = req.preset.as_deref().unwrap_or_else(|| src.default_preset());
    let chosen: Vec<&Col> = match &req.columns {
        Some(keys) => {
            let mut v: Vec<&Col> = keys.iter().filter_map(|k| cols.iter().find(|c| c.key == k)).collect();
            if v.is_empty() {
                v = cols.iter().filter(|c| c.presets.contains(&preset)).collect();
            }
            v
        }
        None => cols.iter().filter(|c| c.presets.contains(&preset)).collect(),
    };
    let chosen: Vec<&Col> = if chosen.is_empty() { cols.iter().take(6).collect() } else { chosen };

    let start = req.offset.min(ids.len());
    let end = (start + req.limit.clamp(1, 500)).min(ids.len());
    let rows = ids[start..end]
        .iter()
        .map(|&id| Row {
            id,
            cells: chosen.iter().map(|col| src.cell(c, &prep, id, col.key)).collect(),
            open: src.open(c, &prep, id),
            tone: src.row_tone(c, &prep, id),
        })
        .collect();

    let mut presets: Vec<&'static str> = Vec::new();
    for col in &cols {
        for p in col.presets {
            if !presets.contains(p) {
                presets.push(p);
            }
        }
    }
    TableResp {
        columns: chosen.iter().map(|c| c.key).collect(),
        all_columns: cols,
        presets,
        total,
        offset: start,
        rows,
        note: src.note(c, &prep, &req.filters),
        revision: c.s.revision,
        sort: sort_key.map(|k| (k, desc)),
    }
}

// ---- filter helpers ---------------------------------------------------------------

pub fn f_u32(f: &Value, k: &str) -> Option<u32> {
    f.get(k).and_then(Value::as_u64).map(|v| v as u32)
}

pub fn f_i32(f: &Value, k: &str) -> Option<i32> {
    f.get(k).and_then(Value::as_i64).map(|v| v as i32)
}

pub fn f_str<'a>(f: &'a Value, k: &str) -> Option<&'a str> {
    f.get(k).and_then(Value::as_str).filter(|s| !s.is_empty())
}

pub fn f_bool(f: &Value, k: &str) -> Option<bool> {
    f.get(k).and_then(Value::as_bool)
}
