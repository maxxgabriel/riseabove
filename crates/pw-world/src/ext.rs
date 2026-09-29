//! Domain-organised state added by the simulation expansion. One entry point
//! on `World`, one owner per domain. Not a dumping ground: a domain gets a
//! field here only when it has persistent state that no existing struct can hold.
//!
//! Migration: every field is `Default`, and legacy initialisation must be a
//! deterministic function of existing state that never invents history.
//! Saves are positional, so adding a domain here breaks older saves until the
//! save architecture gains versioning.

use serde::{Deserialize, Serialize};

use crate::medical::MedicalExt;
use crate::ruling::DecisionMemory;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Extensions {
    /// Owner: `pw_sim::returns` and `pw_sim::medical`.
    pub medical: MedicalExt,
    /// Owner: whichever system makes a material decision; read by any system that asks "why".
    pub decisions: DecisionMemory,
}
