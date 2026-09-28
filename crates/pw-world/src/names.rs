use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::FxHashMap;

/// Interned string handle. Hundreds of thousands of people share a few tens of
/// thousands of distinct names, so people store 4-byte handles.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NameId(pub u32);

impl NameId {
    pub const NONE: NameId = NameId(u32::MAX);

    pub const fn is_none(self) -> bool {
        self.0 == u32::MAX
    }
}

impl Default for NameId {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Default, Clone)]
pub struct Names {
    strings: Vec<Box<str>>,
    index: FxHashMap<Box<str>, NameId>,
}

impl Names {
    pub fn intern(&mut self, s: &str) -> NameId {
        let s = s.trim();
        if s.is_empty() {
            return NameId::NONE;
        }
        if let Some(&id) = self.index.get(s) {
            return id;
        }
        let id = NameId(self.strings.len() as u32);
        self.strings.push(s.into());
        self.index.insert(s.into(), id);
        id
    }

    #[inline]
    pub fn get(&self, id: NameId) -> &str {
        if id.is_none() { "" } else { &self.strings[id.0 as usize] }
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Serialize for Names {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.strings.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Names {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let strings: Vec<Box<str>> = Vec::deserialize(d)?;
        let index = strings.iter().enumerate().map(|(i, s)| (s.clone(), NameId(i as u32))).collect();
        Ok(Self { strings, index })
    }
}
