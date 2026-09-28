use pw_core::{Date, HiddenAttrs, NationId, PlayerId, StaffId};
use serde::{Deserialize, Serialize};

use crate::decision::MindKind;
use crate::names::{NameId, Names};

/// A permanent human in the world. Roles change over a lifetime (player →
/// coach → manager); the person and their id never do.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub first: NameId,
    pub last: NameId,
    pub common: NameId,
    pub dob: Date,
    pub nation: NationId,
    pub nation2: NationId,
    pub hidden: HiddenAttrs,
    pub player: PlayerId,
    pub staff: StaffId,
    pub mind: MindKind,
}

impl Person {
    pub fn display_name<'a>(&self, names: &'a Names) -> std::borrow::Cow<'a, str> {
        if !self.common.is_none() {
            return names.get(self.common).into();
        }
        match (names.get(self.first), names.get(self.last)) {
            ("", l) => l.into(),
            (f, "") => f.into(),
            (f, l) => format!("{f} {l}").into(),
        }
    }

    /// Short form for tables: "E. Haaland", or the common name.
    pub fn short_name(&self, names: &Names) -> String {
        if !self.common.is_none() {
            return names.get(self.common).to_string();
        }
        match (names.get(self.first).chars().next(), names.get(self.last)) {
            (Some(i), l) if !l.is_empty() => format!("{i}. {l}"),
            _ => self.display_name(names).into_owned(),
        }
    }

    #[inline]
    pub fn age(&self, today: Date) -> u32 {
        self.dob.age_on(today)
    }

    #[inline]
    pub fn is_external(&self) -> bool {
        self.mind == MindKind::External
    }
}
