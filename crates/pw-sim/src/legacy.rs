//! Bringing a loaded world's new state up to date (locked design 10.3, 10.4, 10.8).
//!
//! `pw_world::ext` upgrades the *bytes* of the extension state (new domains start empty). What cannot be done on bytes is anything
//! that needs the rest of the world: a present baseline derived from current state, say. That is this module's job, and it runs
//! exactly once, after load, when `Extensions::migrated_from` says the state came from an older layout.
//!
//! Rules: deterministic (no fresh randomness), derived only from existing state, never inventing history (no fake past sightings,
//! results or reasons), and anything derived is marked as legacy-derived wherever the state has provenance to mark it with.

use pw_world::World;

/// Run the initialisation for every layout the world may have come from. Idempotent: clears the marker.
pub fn finish(w: &mut World) {
    let Some(from) = w.ext.migrated_from.take() else { return };
    // Version 1 -> 2 and later steps register their initialisation here, in order, each guarded by `from <= n`.
    let _ = from;
}
