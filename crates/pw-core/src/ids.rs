use serde::{Deserialize, Serialize};

use crate::idvec::Id;

macro_rules! define_ids {
    ($($name:ident),* $(,)?) => {$(
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $name(pub u32);

        impl $name {
            pub const NONE: Self = Self(u32::MAX);

            #[inline]
            pub const fn is_none(self) -> bool {
                self.0 == u32::MAX
            }

            #[inline]
            pub const fn is_some(self) -> bool {
                self.0 != u32::MAX
            }

            #[inline]
            pub fn get(self) -> Option<Self> {
                self.is_some().then_some(self)
            }
        }

        impl Id for $name {
            #[inline]
            fn from_index(i: usize) -> Self {
                debug_assert!(i < u32::MAX as usize);
                Self(i as u32)
            }

            #[inline]
            fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::NONE
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if self.is_none() {
                    f.write_str(concat!(stringify!($name), "(-)"))
                } else {
                    write!(f, concat!(stringify!($name), "({})"), self.0)
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    )*};
}

define_ids!(
    PersonId,
    PlayerId,
    StaffId,
    ClubId,
    TeamId,
    NationId,
    CompId,
    FixtureId,
    DecisionId,
    CultureId,
    TalkId,
    EventId,
    AgentId,
    OutletId,
    StoryId,
    MeetingId,
);
