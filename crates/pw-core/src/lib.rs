//! Shared primitives for the Pathway simulation: typed ids, dates, deterministic
//! RNG and math, and the football vocabulary every other crate speaks.

pub mod attr;
pub mod date;
pub mod ids;
pub mod idvec;
pub mod math;
pub mod pos;
pub mod rng;
pub mod serde_arr;
pub mod tactics;
pub mod traits;

pub use attr::{Attr, AttrGroup, Attrs, Hidden, HiddenAttrs, N_ATTR, N_HIDDEN, N_STAFF_ATTR, StaffAttr, StaffAttrs};
pub use date::{Date, Weekday};
pub use ids::*;
pub use idvec::{Id, IdVec};
pub use pos::{Foot, N_POS, Pos, PosGroup, Role};
pub use rng::Rng;
pub use tactics::{Mentality, Slot, Tactics};
pub use traits::PlayerTraits;

/// Whole currency units in the world's base currency.
pub type Money = i64;
