//! Where the bytes of a save go: a serde serializer that counts what bincode (fixint, the save format) would write and attributes it
//! to the path of the field it came from (`media.stories[].text`). A diagnostic for save growth: nothing in the simulation uses it.
//!
//! The count equals `bincode::serialized_size` (tested). Paths: `.field` for a struct field, `[]` for a sequence element, `{}` for a
//! map value, `::Variant` for an enum variant. Paths deeper than `max_depth` are counted in their parent but not listed.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use serde::Serialize;
use serde::ser::{self, Serializer};

#[derive(Debug)]
pub struct SizeError(String);

impl std::fmt::Display for SizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SizeError {}

impl ser::Error for SizeError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        SizeError(msg.to_string())
    }
}

/// Bytes by path, aggregated over every instance.
#[derive(Default)]
pub struct Tally {
    pub bytes: HashMap<String, u64>,
    pub count: HashMap<String, u64>,
}

type Sink = Rc<RefCell<Tally>>;

#[derive(Clone)]
struct Sz {
    sink: Sink,
    /// Path of the value being written; `None` once below `max_depth` (counted, not listed).
    path: Option<Rc<str>>,
    depth: usize,
    max: usize,
}

impl Sz {
    fn child(&self, suffix: &str) -> Sz {
        let path = match (&self.path, self.depth < self.max) {
            (Some(p), true) => Some(Rc::from(format!("{p}{suffix}"))),
            _ => None,
        };
        Sz { sink: self.sink.clone(), path, depth: self.depth + 1, max: self.max }
    }

    fn record(&self, bytes: u64) {
        if let Some(p) = &self.path {
            let mut t = self.sink.borrow_mut();
            *t.bytes.entry(p.to_string()).or_default() += bytes;
            *t.count.entry(p.to_string()).or_default() += 1;
        }
    }
}

struct Compound {
    at: Sz,
    total: u64,
    /// Reused child for sequence elements and map values.
    elem: Option<Sz>,
}

impl Compound {
    fn new(at: &Sz, base: u64, elem_suffix: Option<&str>) -> Self {
        Compound { at: at.clone(), total: base, elem: elem_suffix.map(|s| at.child(s)) }
    }

    fn field<T: ?Sized + Serialize>(&mut self, key: &str, v: &T) -> Result<(), SizeError> {
        let c = self.at.child(&format!(".{key}"));
        let n = v.serialize(c.clone())?;
        c.record(n);
        self.total += n;
        Ok(())
    }

    fn element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SizeError> {
        let c = self.elem.clone().unwrap_or_else(|| self.at.child(""));
        let n = v.serialize(c.clone())?;
        c.record(n);
        self.total += n;
        Ok(())
    }

    fn plain<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SizeError> {
        let mut n = 0;
        // Keys and tuple members are counted but not listed.
        let c = Sz { sink: self.at.sink.clone(), path: None, depth: self.at.max + 1, max: self.at.max };
        n += v.serialize(c)?;
        self.total += n;
        Ok(())
    }
}

impl ser::SerializeSeq for Compound {
    type Ok = u64;
    type Error = SizeError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SizeError> {
        self.element(v)
    }
    fn end(self) -> Result<u64, SizeError> {
        Ok(self.total)
    }
}

impl ser::SerializeTuple for Compound {
    type Ok = u64;
    type Error = SizeError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SizeError> {
        self.plain(v)
    }
    fn end(self) -> Result<u64, SizeError> {
        Ok(self.total)
    }
}

impl ser::SerializeTupleStruct for Compound {
    type Ok = u64;
    type Error = SizeError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SizeError> {
        self.plain(v)
    }
    fn end(self) -> Result<u64, SizeError> {
        Ok(self.total)
    }
}

impl ser::SerializeTupleVariant for Compound {
    type Ok = u64;
    type Error = SizeError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SizeError> {
        self.plain(v)
    }
    fn end(self) -> Result<u64, SizeError> {
        self.at.record(self.total);
        Ok(self.total)
    }
}

impl ser::SerializeMap for Compound {
    type Ok = u64;
    type Error = SizeError;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, k: &T) -> Result<(), SizeError> {
        self.plain(k)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SizeError> {
        self.element(v)
    }
    fn end(self) -> Result<u64, SizeError> {
        Ok(self.total)
    }
}

impl ser::SerializeStruct for Compound {
    type Ok = u64;
    type Error = SizeError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, v: &T) -> Result<(), SizeError> {
        self.field(key, v)
    }
    fn end(self) -> Result<u64, SizeError> {
        Ok(self.total)
    }
}

impl ser::SerializeStructVariant for Compound {
    type Ok = u64;
    type Error = SizeError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, v: &T) -> Result<(), SizeError> {
        self.field(key, v)
    }
    fn end(self) -> Result<u64, SizeError> {
        self.at.record(self.total);
        Ok(self.total)
    }
}

impl Serializer for Sz {
    type Ok = u64;
    type Error = SizeError;
    type SerializeSeq = Compound;
    type SerializeTuple = Compound;
    type SerializeTupleStruct = Compound;
    type SerializeTupleVariant = Compound;
    type SerializeMap = Compound;
    type SerializeStruct = Compound;
    type SerializeStructVariant = Compound;

    fn serialize_bool(self, _: bool) -> Result<u64, SizeError> {
        Ok(1)
    }
    fn serialize_i8(self, _: i8) -> Result<u64, SizeError> {
        Ok(1)
    }
    fn serialize_i16(self, _: i16) -> Result<u64, SizeError> {
        Ok(2)
    }
    fn serialize_i32(self, _: i32) -> Result<u64, SizeError> {
        Ok(4)
    }
    fn serialize_i64(self, _: i64) -> Result<u64, SizeError> {
        Ok(8)
    }
    fn serialize_i128(self, _: i128) -> Result<u64, SizeError> {
        Ok(16)
    }
    fn serialize_u8(self, _: u8) -> Result<u64, SizeError> {
        Ok(1)
    }
    fn serialize_u16(self, _: u16) -> Result<u64, SizeError> {
        Ok(2)
    }
    fn serialize_u32(self, _: u32) -> Result<u64, SizeError> {
        Ok(4)
    }
    fn serialize_u64(self, _: u64) -> Result<u64, SizeError> {
        Ok(8)
    }
    fn serialize_u128(self, _: u128) -> Result<u64, SizeError> {
        Ok(16)
    }
    fn serialize_f32(self, _: f32) -> Result<u64, SizeError> {
        Ok(4)
    }
    fn serialize_f64(self, _: f64) -> Result<u64, SizeError> {
        Ok(8)
    }
    fn serialize_char(self, c: char) -> Result<u64, SizeError> {
        Ok(c.len_utf8() as u64)
    }
    fn serialize_str(self, s: &str) -> Result<u64, SizeError> {
        Ok(8 + s.len() as u64)
    }
    fn serialize_bytes(self, b: &[u8]) -> Result<u64, SizeError> {
        Ok(8 + b.len() as u64)
    }
    fn serialize_none(self) -> Result<u64, SizeError> {
        Ok(1)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<u64, SizeError> {
        // The option's payload is this path (an Option adds one byte and no level).
        Ok(1 + v.serialize(self)?)
    }
    fn serialize_unit(self) -> Result<u64, SizeError> {
        Ok(0)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<u64, SizeError> {
        Ok(0)
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<u64, SizeError> {
        self.child(&format!("::{variant}")).record(4);
        Ok(4)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _: &'static str, v: &T) -> Result<u64, SizeError> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _: &'static str, _: u32, variant: &'static str, v: &T) -> Result<u64, SizeError> {
        let c = self.child(&format!("::{variant}"));
        let n = 4 + v.serialize(c.clone())?;
        c.record(n);
        Ok(n)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Compound, SizeError> {
        Ok(Compound::new(&self, 8, Some("[]")))
    }
    fn serialize_tuple(self, _: usize) -> Result<Compound, SizeError> {
        Ok(Compound::new(&self, 0, None))
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Compound, SizeError> {
        Ok(Compound::new(&self, 0, None))
    }
    fn serialize_tuple_variant(self, _: &'static str, _: u32, variant: &'static str, _: usize) -> Result<Compound, SizeError> {
        let c = self.child(&format!("::{variant}"));
        Ok(Compound::new(&c, 4, None))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Compound, SizeError> {
        Ok(Compound::new(&self, 8, Some("{}")))
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Compound, SizeError> {
        Ok(Compound::new(&self, 0, None))
    }
    fn serialize_struct_variant(self, _: &'static str, _: u32, variant: &'static str, _: usize) -> Result<Compound, SizeError> {
        let c = self.child(&format!("::{variant}"));
        Ok(Compound::new(&c, 4, None))
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Bytes by path for `value` (root path `root`), down to `max_depth` path segments. Sorted largest first.
pub fn breakdown<T: Serialize + ?Sized>(root: &str, value: &T, max_depth: usize) -> Vec<(String, u64, u64)> {
    let sink: Sink = Rc::new(RefCell::new(Tally::default()));
    let at = Sz { sink: sink.clone(), path: Some(Rc::from(root)), depth: 0, max: max_depth };
    let n = value.serialize(at.clone()).unwrap_or(0);
    at.record(n);
    let t = sink.borrow();
    let mut v: Vec<_> = t.bytes.iter().map(|(k, &b)| (k.clone(), b, t.count.get(k).copied().unwrap_or(0))).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// Total bytes (equal to `bincode::serialized_size`).
pub fn total<T: Serialize + ?Sized>(value: &T) -> u64 {
    let sink: Sink = Rc::new(RefCell::new(Tally::default()));
    value.serialize(Sz { sink, path: None, depth: 1, max: 0 }).unwrap_or(0)
}

macro_rules! world_fields {
    ($($f:ident)*) => {
        /// Names of every top-level field of the world.
        pub const WORLD_FIELDS: &[&str] = &[$(stringify!($f)),*];

        /// Field paths inside one top-level section of the world (`media`, `ext`, ...), or `None` for an unknown name.
        pub fn section(w: &pw_world::World, name: &str, max_depth: usize) -> Option<Vec<(String, u64, u64)>> {
            match name {
                $(stringify!($f) => Some(breakdown(name, &w.$f, max_depth)),)*
                _ => None,
            }
        }
    };
}

world_fields!(seed date data names nations people players staff clubs teams comps fixtures knowledge events history stats decisions market social talks beliefs lives agents media meetings intents governance economy careers scouting deals youth intl medical ext rooms perf growth honours renown affairs commerce culture grapevine incidents agenda recent_matches pressroom net inbox minor records acclaim officials evolution backfill origins reports days_simulated followed prepared playthrough dossiers boardroom adaptation tactics lifestate);

/// The largest field paths of the whole world, all sections merged, largest first (the section roots themselves are included).
pub fn world_paths(w: &pw_world::World, max_depth: usize) -> Vec<(String, u64, u64)> {
    let mut all: Vec<_> = WORLD_FIELDS.iter().filter_map(|n| section(w, n, max_depth)).flatten().collect();
    all.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    enum Kind {
        A,
        B(u32),
        C { x: u8, y: String },
    }

    #[derive(Serialize)]
    struct Row {
        id: u32,
        name: String,
        tag: Option<u16>,
        kind: Kind,
        pair: (u8, f32),
    }

    #[derive(Serialize)]
    struct Book {
        rows: Vec<Row>,
        by: std::collections::BTreeMap<u32, Vec<u8>>,
    }

    #[test]
    fn the_count_equals_what_bincode_writes() {
        let book = Book {
            rows: vec![
                Row { id: 1, name: "ab".into(), tag: Some(3), kind: Kind::A, pair: (1, 2.0) },
                Row { id: 2, name: String::new(), tag: None, kind: Kind::B(9), pair: (2, 0.5) },
                Row { id: 3, name: "xyz".into(), tag: None, kind: Kind::C { x: 1, y: "hé".into() }, pair: (3, 1.0) },
            ],
            by: [(1, vec![1, 2, 3]), (2, vec![])].into_iter().collect(),
        };
        assert_eq!(total(&book), bincode::serialized_size(&book).unwrap());
        let parts = breakdown("book", &book, 6);
        assert_eq!(parts[0].0, "book");
        assert_eq!(parts[0].1, bincode::serialized_size(&book).unwrap());
        let rows = parts.iter().find(|p| p.0 == "book.rows").unwrap();
        assert_eq!(rows.1, bincode::serialized_size(&book.rows).unwrap());
        assert!(parts.iter().any(|p| p.0 == "book.rows[].kind::C.y" && p.1 == 8 + 3));
    }
}
