//! Football language for the Pathway simulation: one semantic event, filtered through what a speaker knows and how sure they are, realised
//! by channel-specific grammars from structured vocabulary. Nothing here writes a fact the caller did not supply.

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

pub mod check;
pub mod cond;
pub mod corpus;
pub mod data;
pub mod derive;
pub mod model;
pub mod render;
pub mod template;
pub mod text;

pub use data::Lang;
pub use model::*;
pub use render::{Engine, Tracker};
