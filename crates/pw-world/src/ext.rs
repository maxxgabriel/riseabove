//! Domain-organised state added by the simulation expansion. One entry point
//! on `World`, one owner per domain. Not a dumping ground: a domain gets a
//! field here only when it has persistent state that no existing struct can hold.
//!
//! # Saves: this is the integration point
//!
//! `World::ext` is written as its own **versioned envelope** (`envelope`): `(EXT_VERSION, bincode bytes of Extensions)`. The rest of
//! the world is positional bincode under the save schema (`pw_sim::save`), so a layout change there needs a schema step; a change
//! *here* needs only an entry in [`steps`], and old saves keep opening. Rules for changing `Extensions`:
//!
//! 1. Bump [`EXT_VERSION`] and add a step `from = old` that produces the new layout from the old bytes.
//! 2. Adding a domain: append the field at the END of `Extensions` and use [`append_default`] as the step (the old bytes are
//!    exactly a prefix of the new). Every field is `Default`, and its default must be "nothing has happened yet".
//! 3. Changing a domain's own layout: keep a frozen copy of its old type in the step, decode with it, encode the new one.
//! 4. The step never invents history. Anything that has to be derived from the rest of the world (a baseline from present
//!    state, say) is done by `pw_sim::legacy::finish`, which runs once after load when [`Extensions::migrated_from`] is set, is
//!    deterministic, and marks what it derived as legacy-derived where provenance exists.

use serde::{Deserialize, Serialize};

use crate::academy::AcademyExt;
use crate::almanac::Almanac;
use crate::ecosystem::Ecosystem;
use crate::medical::MedicalExt;
use crate::pathway::PathwayExt;
use crate::recog::Recog;
use crate::scenario::Scenario;
use crate::ruling::DecisionMemory;
use crate::stafflife::StaffExt;
use crate::training::TrainingExt;

/// Version of the `Extensions` layout written by this build. History: 1 = the layout at the introduction of the envelope; 2 = adds
/// `scenario` (tuning and calendar), `recog` (what organisations know, vouches, market regard) and `pathway` (why players moved, how
/// they were created), all appended and all empty or default in a save from layout 1.
pub const EXT_VERSION: u32 = 2;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Extensions {
    /// Owner: `pw_sim::returns` and `pw_sim::medical`.
    pub medical: MedicalExt,
    /// Owner: `pw_sim::ecosystem`. Regions, associations, participation pools and the routes players take.
    pub ecosystem: Ecosystem,
    /// Owner: `pw_sim::almanac`. Careers as counters, leaderboards, streaks.
    pub almanac: Almanac,
    /// Owner: `pw_sim::pathway`.
    pub academy: AcademyExt,
    /// Owner: `pw_sim::stafflife`.
    pub staff: StaffExt,
    /// Owner: `pw_sim::training`.
    pub training: TrainingExt,
    /// Owner: whichever system makes a material decision; read by any system that asks "why".
    pub decisions: DecisionMemory,
    // ---- layout 2: appended, in this order (the step below depends on it)
    /// Owner: the world builder (from the pack) and `pw_sim::recognition`. Tuning and calendar of the scenario.
    pub scenario: Scenario,
    /// Owner: `pw_sim::recognition` and `pw_sim::foreign`. What each organisation knows, vouches, referral records, market regard.
    pub recog: Recog,
    /// Owner: `pw_sim::pathway`. Why players went where they went, and how each was created.
    pub pathway: PathwayExt,
    /// Set (never saved) when this value was upgraded from an older layout: the version it came from. `pw_sim::legacy::finish`
    /// consumes it after load.
    #[serde(skip)]
    pub migrated_from: Option<u32>,
}

/// One upgrade of the `Extensions` bytes: `from` -> `from + 1`.
pub struct ExtStep {
    pub from: u32,
    pub name: &'static str,
    pub apply: fn(Vec<u8>) -> Result<Vec<u8>, String>,
}

/// The steps that upgrade older `Extensions` bytes to [`EXT_VERSION`], in order.
pub fn steps() -> &'static [ExtStep] {
    &[ExtStep { from: 1, name: "add scenario tuning, organisation knowledge and pathway history", apply: v1_to_v2 }]
}

/// What layout 2 appended to layout 1, in field order.
#[derive(Default, Serialize)]
struct V2Tail {
    scenario: Scenario,
    recog: Recog,
    pathway: PathwayExt,
}

fn v1_to_v2(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    append_default::<V2Tail>(bytes)
}

/// Step helper for adding a domain: the old bytes gain the default of the new trailing field(s).
pub fn append_default<T: Default + Serialize>(mut bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    bytes.extend(bincode::serialize(&T::default()).map_err(|e| e.to_string())?);
    Ok(bytes)
}

/// Run every step from `from` up to `current`. A missing step is an error, never a guess.
pub fn migrate(from: u32, mut bytes: Vec<u8>, steps: &[ExtStep], current: u32) -> Result<Vec<u8>, String> {
    for v in from..current {
        let step = steps.iter().find(|s| s.from == v).ok_or_else(|| format!("no step upgrades the extension state from version {v}"))?;
        bytes = (step.apply)(bytes).map_err(|e| format!("{} (extension state {v} to {}): {e}", step.name, v + 1))?;
    }
    Ok(bytes)
}

/// Decode extension bytes written at `version`, upgrading them if they are older than this build's layout.
pub fn decode_with(version: u32, bytes: Vec<u8>, steps: &[ExtStep], current: u32) -> Result<Extensions, String> {
    if version > current {
        return Err(format!("the extension state is version {version}, newer than this build reads ({current})"));
    }
    if version == 0 {
        return Err("the extension state has no version".into());
    }
    let bytes = migrate(version, bytes, steps, current)?;
    let mut e: Extensions = bincode::deserialize(&bytes).map_err(|e| format!("extension state does not decode: {e}"))?;
    if version < current {
        e.migrated_from = Some(version);
    }
    Ok(e)
}

pub fn decode(version: u32, bytes: Vec<u8>) -> Result<Extensions, String> {
    decode_with(version, bytes, steps(), EXT_VERSION)
}

/// Serde adapter for `World::ext`: `#[serde(with = "crate::ext::envelope")]`.
pub mod envelope {
    use serde::de::Error as _;
    use serde::ser::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{EXT_VERSION, Extensions};

    pub fn serialize<S: Serializer>(e: &Extensions, s: S) -> Result<S::Ok, S::Error> {
        let bytes = bincode::serialize(e).map_err(S::Error::custom)?;
        (EXT_VERSION, bytes).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Extensions, D::Error> {
        let (version, bytes) = <(u32, Vec<u8>)>::deserialize(d)?;
        super::decode(version, bytes).map_err(D::Error::custom)
    }
}
