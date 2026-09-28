//! Match simulation behind one interface: `simulate(&MatchInput) -> MatchResult`.
//!
//! Two backends produce the same result type, selected by the data pack
//! (`matches.backend`):
//! - `ofm`: OpenFootManager's calibrated minute engine (vendored, GPL-3),
//!   wrapped by [`ofm`].
//! - `native`: Pathway's zone/possession-chain engine ([`engine`]), which
//!   records richer per-action events but is still being calibrated.
//!
//! Every match runs through the same backend regardless of level of detail
//! (P1); LOD only decides which events are kept.

mod engine;
mod ofm;
mod pitch;
mod types;

pub use pitch::{N_ZONES, ZONES_X, ZONES_Y, zone_xy};
pub use pw_data::MatchBackend;
pub use types::*;

pub fn simulate(inp: &MatchInput) -> MatchResult {
    match inp.tuning.backend {
        MatchBackend::Ofm => ofm::simulate(inp),
        MatchBackend::Native => engine::simulate(inp),
    }
}
