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

mod coach;
mod engine;
mod ofm;
mod pitch;
mod types;

pub use pitch::{N_ZONES, ZONES_X, ZONES_Y, zone_xy};
pub use coach::{Call, Coach, Look, Mind, Nobody, PlayerLook, SideLook, SubCall, Tally};
pub use pw_data::MatchBackend;
pub use types::*;

pub fn simulate(inp: &MatchInput) -> MatchResult {
    match inp.tuning.backend {
        MatchBackend::Ofm => ofm::simulate(inp),
        MatchBackend::Native => engine::simulate(inp),
    }
}

/// The same match with people around it (see [`Coach`]): players walk out in the state the coach describes, and at a few windows each
/// side looks at the game and may change its approach, roles and substitutions. The coached path exists for the OFM backend; the native
/// backend takes the pre-match states and plays on.
pub fn simulate_with(inp: &MatchInput, coach: &mut dyn Coach) -> MatchResult {
    match inp.tuning.backend {
        MatchBackend::Ofm => ofm::simulate_with(inp, coach),
        MatchBackend::Native => {
            let minded = ofm::with_minds(inp, coach);
            let result = engine::simulate(&minded);
            coach.finished(&result);
            result
        }
    }
}
