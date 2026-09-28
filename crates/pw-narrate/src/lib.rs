//! Narration (S17–S18): text as a pure function of world state.
//!
//! Nothing in this crate writes to the world. Every line it produces renders
//! an event, a story, a meeting, a belief or a piece of state that exists; if
//! there is no source, there is no text. Variety comes from composition —
//! templates keyed by what happened, context facts drawn from history, and
//! the voice of whoever is speaking — with the template picked by a stable
//! hash of the source, so the same event always reads the same way.

pub mod choices;
pub mod events;
pub mod fmt;
pub mod grapevine;
pub mod history;
pub mod inbox;
pub mod incidents;
pub mod lexicon;
pub mod press;
pub mod social;
pub mod talk;

pub use fmt::{club, money, person, player};

/// Pick one of several phrasings, stably, from a source key.
pub fn pick<'a>(key: u64, options: &[&'a str]) -> &'a str {
    if options.is_empty() {
        return "";
    }
    let h = pw_core::rng::hash_key(&[pw_core::rng::stream::NARRATION, key]);
    options[(h % options.len() as u64) as usize]
}
