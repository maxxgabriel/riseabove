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

use pw_core::ClubId;
use serde::{Deserialize, Serialize};

use crate::FxHashMap;
use crate::academy::AcademyExt;
use crate::almanac::Almanac;
use crate::chat::Chats;
use crate::chronicle::Chronicles;
use crate::ecosystem::Ecosystem;
use crate::ledger::Ledgers;
use crate::lore::Lore;
use crate::medical::MedicalExt;
use crate::pathway::PathwayExt;
use crate::recog::Recog;
use crate::scenario::{CalRule, DataOrigin, MarketDef, NationalRules, RecognitionTuning, Scenario, ScoutingTuning};
use crate::ruling::DecisionMemory;
use crate::stafflife::StaffExt;
use crate::training::TrainingExt;

/// Version of the `Extensions` layout written by this build. History: 1 = the layout at the introduction of the envelope; 2 = adds
/// `scenario` (tuning and calendar), `recog` (what organisations know, vouches, market regard) and `pathway` (why players moved, how
/// they were created), all appended and all empty or default in a save from layout 1; 3 = `Scenario` gains `known_derbies` and
/// `reference` at its end (named derbies and the report of loading reference data), both empty in a save from layout 2; 4 = adds
/// `lore` (names and labels from reference data), appended and empty in a save from layout 3; 5 = adds `chronicle` (the lives humans
/// inhabit, as they were lived), appended and empty in a save from layout 4: a chronicle begins when someone is inhabited; 6 = adds
/// `chats` (group chats and private messages around inhabited people), appended and empty in a save from layout 5; 7 = adds `ledger`
/// (payslips and bonuses of inhabited lives), appended and empty in a save from layout 6.
pub const EXT_VERSION: u32 = 7;

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
    // ---- layout 4: appended
    /// Owner: the world builder. Names, labels and descriptions from the scenario's reference data (`crate::lore`).
    pub lore: Lore,
    // ---- layout 5: appended
    /// Owner: `pw_sim::chronicle`. The career chronicle of each person a human has inhabited.
    pub chronicle: Chronicles,
    // ---- layout 6: appended
    /// Owner: `pw_sim::chat`. Chats around the people humans inhabit.
    pub chats: Chats,
    // ---- layout 7: appended
    /// Owner: `pw_sim::life` and `pw_sim::clauses`. Where the money of each inhabited life went.
    pub ledger: Ledgers,
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
    &[
        ExtStep { from: 1, name: "add scenario tuning, organisation knowledge and pathway history", apply: v1_to_v2 },
        ExtStep { from: 2, name: "add the scenario's known derbies and reference-data report", apply: v2_to_v3 },
        ExtStep { from: 3, name: "add names and labels from reference data", apply: v3_to_v4 },
        ExtStep { from: 4, name: "add career chronicles", apply: v4_to_v5 },
        ExtStep { from: 5, name: "add chats", apply: v5_to_v6 },
        ExtStep { from: 6, name: "add personal ledgers", apply: v6_to_v7 },
    ]
}

/// `Scenario` exactly as layout 2 wrote it. Frozen: never edit it, because old bytes are decoded with it. (Rule 3 at the top.)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioV2 {
    pub recognition: RecognitionTuning,
    pub scouting: ScoutingTuning,
    pub calendar: Vec<CalRule>,
    pub markets: Vec<MarketDef>,
    pub source: String,
    pub club_origin: FxHashMap<ClubId, DataOrigin>,
    pub national: Vec<(String, NationalRules)>,
}

impl From<&Scenario> for ScenarioV2 {
    /// The layout-2 part of a scenario (what a layout-2 build would have written for it).
    fn from(s: &Scenario) -> Self {
        Self {
            recognition: s.recognition.clone(),
            scouting: s.scouting.clone(),
            calendar: s.calendar.clone(),
            markets: s.markets.clone(),
            source: s.source.clone(),
            club_origin: s.club_origin.clone(),
            national: s.national.clone(),
        }
    }
}

impl Default for ScenarioV2 {
    fn default() -> Self {
        Self::from(&Scenario::default())
    }
}

/// What layout 2 appended to layout 1, in field order.
#[derive(Default, Serialize)]
struct V2Tail {
    scenario: ScenarioV2,
    recog: Recog,
    pathway: PathwayExt,
}

fn v1_to_v2(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    append_default::<V2Tail>(bytes)
}

/// `Extensions` as layout 2 wrote it: the same fields in the same order, with the layout-2 scenario.
#[derive(Deserialize)]
struct ExtensionsV2 {
    medical: MedicalExt,
    ecosystem: Ecosystem,
    almanac: Almanac,
    academy: AcademyExt,
    staff: StaffExt,
    training: TrainingExt,
    decisions: DecisionMemory,
    scenario: ScenarioV2,
    recog: Recog,
    pathway: PathwayExt,
}

/// The scenario sits in the middle of `Extensions`, so its two new fields cannot simply be appended to the bytes: decode the whole
/// layout-2 state, give the scenario its new fields empty ("nothing has happened yet": no derbies named, no reference data read),
/// and encode the layout-3 state. Nothing is invented and nothing else changes.
fn v2_to_v3(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let old: ExtensionsV2 = bincode::deserialize(&bytes).map_err(|e| format!("layout 2 does not decode: {e}"))?;
    let s = old.scenario;
    let scenario = Scenario {
        recognition: s.recognition,
        scouting: s.scouting,
        calendar: s.calendar,
        markets: s.markets,
        source: s.source,
        club_origin: s.club_origin,
        national: s.national,
        known_derbies: Vec::new(),
        reference: Default::default(),
    };
    let new = ExtensionsV3 {
        medical: old.medical,
        ecosystem: old.ecosystem,
        almanac: old.almanac,
        academy: old.academy,
        staff: old.staff,
        training: old.training,
        decisions: old.decisions,
        scenario,
        recog: old.recog,
        pathway: old.pathway,
    };
    bincode::serialize(&new).map_err(|e| e.to_string())
}

/// `Extensions` exactly as layout 3 wrote it. A step writes the layout it steps TO, never the current one: encoding the current
/// `Extensions` here would put later layouts' fields into layout-3 bytes, and the later steps would add them a second time.
#[derive(Serialize)]
struct ExtensionsV3 {
    medical: MedicalExt,
    ecosystem: Ecosystem,
    almanac: Almanac,
    academy: AcademyExt,
    staff: StaffExt,
    training: TrainingExt,
    decisions: DecisionMemory,
    scenario: Scenario,
    recog: Recog,
    pathway: PathwayExt,
}

/// Layout 4 appends `lore` at the end: a save from layout 3 took nothing from reference data beyond what `scenario` already holds.
fn v3_to_v4(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    append_default::<Lore>(bytes)
}

/// Layout 5 appends `chronicle` at the end: no life was chronicled before it existed (the next inhabiting begins one).
fn v4_to_v5(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    append_default::<Chronicles>(bytes)
}

/// Layout 6 appends `chats` at the end: nobody had a chat before chats existed.
fn v5_to_v6(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    append_default::<Chats>(bytes)
}

/// Layout 7 appends `ledger` at the end: no payslip was kept before ledgers existed.
fn v6_to_v7(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    append_default::<Ledgers>(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes a layout-1 build wrote for `Extensions`: its seven fields, in order (bincode writes struct fields back to back).
    fn layout_1_bytes(e: &Extensions) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(bincode::serialize(&e.medical).unwrap());
        b.extend(bincode::serialize(&e.ecosystem).unwrap());
        b.extend(bincode::serialize(&e.almanac).unwrap());
        b.extend(bincode::serialize(&e.academy).unwrap());
        b.extend(bincode::serialize(&e.staff).unwrap());
        b.extend(bincode::serialize(&e.training).unwrap());
        b.extend(bincode::serialize(&e.decisions).unwrap());
        b
    }

    /// The bytes a layout-2 build wrote for `Extensions`: layout 1's seven fields, then the layout-2 scenario, recog and pathway.
    fn layout_2_bytes(e: &Extensions) -> Vec<u8> {
        let mut b = layout_1_bytes(e);
        b.extend(bincode::serialize(&ScenarioV2::from(&e.scenario)).unwrap());
        b.extend(bincode::serialize(&e.recog).unwrap());
        b.extend(bincode::serialize(&e.pathway).unwrap());
        b
    }

    #[test]
    fn layout_2_opens_at_layout_3_with_the_new_scenario_fields_empty_and_everything_else_intact() {
        use crate::scenario::{KnownDerby, ReferenceReport};
        let mut old = Extensions::default();
        old.ecosystem.last_year = 2031;
        old.scenario.source = "data/worlds/india/pack.toml".into();
        old.scenario.club_origin.insert(ClubId(7), DataOrigin::ScenarioSeed);
        old.scenario.club_origin.insert(ClubId(9), DataOrigin::Generated);
        old.scenario.national.push(("IND".into(), NationalRules::default()));
        old.scenario.markets.push(MarketDef { key: "gulf".into(), nations: vec!["UAE".into()], start: 12.5 });
        // What a layout-3 build adds must not be in the layout-2 bytes, whatever the current value holds.
        old.scenario.known_derbies.push(KnownDerby { source_id: "rivalry.x".into(), name: "X Derby".into(), a: ClubId(7), b: ClubId(9), derby: true, origin: DataOrigin::Imported });
        old.scenario.reference = ReferenceReport { records: 5, ..Default::default() };
        let e = decode(2, layout_2_bytes(&old)).expect("layout 2 upgrades");
        assert_eq!(e.migrated_from, Some(2), "the load says it came from an older layout");
        assert_eq!(e.ecosystem.last_year, 2031, "what layout 2 held is untouched");
        assert_eq!(e.scenario.source, "data/worlds/india/pack.toml");
        assert_eq!(e.scenario.club_origin.get(&ClubId(7)), Some(&DataOrigin::ScenarioSeed));
        assert_eq!(e.scenario.club_origin.get(&ClubId(9)), Some(&DataOrigin::Generated));
        assert_eq!(e.scenario.national.len(), 1);
        assert_eq!(e.scenario.markets[0].key, "gulf");
        assert_eq!(e.scenario.calendar, Scenario::default().calendar);
        assert!(e.scenario.known_derbies.is_empty(), "no derby is invented for an old save");
        assert_eq!(e.scenario.reference, ReferenceReport::default(), "no reference data is claimed to have been read");
        // And the upgraded state is exactly what layout 3 would write for that value: it decodes again at the current layout.
        let again = decode(EXT_VERSION, bincode::serialize(&e).unwrap()).expect("layout 3 round-trips");
        assert_eq!(again.migrated_from, None);
        assert_eq!(again.scenario, e.scenario);
        // A layout-2 stream with bytes cut off is refused, not half-read.
        let mut short = layout_2_bytes(&old);
        short.truncate(short.len() - 3);
        assert!(decode(2, short).is_err());
    }

    #[test]
    fn layout_3_opens_at_layout_4_with_no_lore_and_everything_else_intact() {
        use crate::lore::{LoreTerm, Source};
        let mut old = Extensions::default();
        old.ecosystem.last_year = 2030;
        old.scenario.club_origin.insert(ClubId(3), DataOrigin::Imported);
        // What layout 4 adds must not be claimed for a save that predates it, whatever the current value holds.
        old.lore.terms.push(LoreTerm { concept: "match.goal".into(), lang: "bn".into(), text: "x".into(), romanised: "gol".into(), register: "neutral".into() });
        old.lore.competitions.insert(pw_core::CompId(1), Source { id: "comp.isl".into(), origin: DataOrigin::ScenarioSeed });
        let v3 = ExtensionsV3 {
            medical: old.medical.clone(),
            ecosystem: old.ecosystem.clone(),
            almanac: old.almanac.clone(),
            academy: old.academy.clone(),
            staff: old.staff.clone(),
            training: old.training.clone(),
            decisions: old.decisions.clone(),
            scenario: old.scenario.clone(),
            recog: old.recog.clone(),
            pathway: old.pathway.clone(),
        };
        let e = decode(3, bincode::serialize(&v3).unwrap()).expect("layout 3 upgrades");
        assert_eq!(e.migrated_from, Some(3));
        assert_eq!(e.ecosystem.last_year, 2030);
        assert_eq!(e.scenario.club_origin.get(&ClubId(3)), Some(&DataOrigin::Imported));
        assert!(e.lore.is_empty() && e.lore.competitions.is_empty(), "no reference name is claimed for an old save");
        // Layout 2 reaches layout 4 through both steps, each writing the layout it steps to.
        let e2 = decode(2, layout_2_bytes(&old)).expect("layout 2 upgrades through 3 to 4");
        assert!(e2.lore.is_empty());
        let again = decode(EXT_VERSION, bincode::serialize(&e).unwrap()).expect("layout 4 round-trips");
        assert_eq!(again.lore, e.lore);
    }

    #[test]
    fn layout_4_opens_at_layout_5_with_no_chronicle_and_everything_else_intact() {
        use crate::chronicle::Life;
        let mut old = Extensions::default();
        old.ecosystem.last_year = 2032;
        old.lore.competitions.insert(pw_core::CompId(2), crate::lore::Source { id: "comp.ifl".into(), origin: DataOrigin::Imported });
        // What layout 5 adds must not be claimed for a save that predates it.
        old.chronicle.lives.insert(pw_core::PersonId(4), Life::default());
        let v3 = ExtensionsV3 {
            medical: old.medical.clone(),
            ecosystem: old.ecosystem.clone(),
            almanac: old.almanac.clone(),
            academy: old.academy.clone(),
            staff: old.staff.clone(),
            training: old.training.clone(),
            decisions: old.decisions.clone(),
            scenario: old.scenario.clone(),
            recog: old.recog.clone(),
            pathway: old.pathway.clone(),
        };
        let mut v4 = bincode::serialize(&v3).unwrap();
        v4.extend(bincode::serialize(&old.lore).unwrap());
        let e = decode(4, v4.clone()).expect("layout 4 upgrades");
        assert_eq!(e.migrated_from, Some(4));
        assert!(e.chats.of.is_empty(), "nobody had chats in an old save");
        // Layout 5 (layout 4 plus a chronicle) opens at layout 6 with no chats.
        let mut v5 = v4;
        v5.extend(bincode::serialize(&old.chronicle).unwrap());
        let e5 = decode(5, v5).expect("layout 5 upgrades");
        assert_eq!(e5.chronicle.lives.len(), 1, "the chronicle layout 5 held is kept");
        assert!(e5.chats.of.is_empty());
        assert!(e5.ledger.of.is_empty(), "no payslip is invented for an old save");
        assert_eq!(e.ecosystem.last_year, 2032);
        assert_eq!(e.lore.competitions.len(), 1, "what layout 4 held is untouched");
        assert!(e.chronicle.lives.is_empty(), "no life is chronicled for an old save");
        let again = decode(EXT_VERSION, bincode::serialize(&e).unwrap()).expect("layout 5 round-trips");
        assert!(again.chronicle.lives.is_empty());
    }

    #[test]
    fn layout_1_reaches_layout_3_through_both_steps() {
        let mut old = Extensions::default();
        old.ecosystem.export = 21.0;
        let e = decode(1, layout_1_bytes(&old)).expect("layout 1 upgrades through 2 to 3");
        assert_eq!(e.migrated_from, Some(1));
        assert!((e.ecosystem.export - 21.0).abs() < 1e-6);
        assert!(e.scenario.known_derbies.is_empty() && e.scenario.reference.records == 0);
    }

    #[test]
    fn layout_1_opens_at_the_current_layout_with_new_domains_empty_and_marked() {
        let mut old = Extensions::default();
        old.ecosystem.last_year = 2031;
        old.ecosystem.export = 33.5;
        let e = decode(1, layout_1_bytes(&old)).expect("layout 1 upgrades");
        assert_eq!(e.migrated_from, Some(1), "the load must say it came from an older layout so the legacy hook runs");
        assert_eq!(e.ecosystem.last_year, 2031, "what layout 1 held is untouched");
        assert!((e.ecosystem.export - 33.5).abs() < 1e-6);
        assert!(e.recog.acquaint.is_empty() && e.recog.vouch.is_empty() && e.recog.export.is_empty() && e.recog.watching.is_empty(), "no organisation knowledge is invented");
        assert!(e.pathway.why.is_empty() && e.pathway.created.is_empty(), "no pathway history is invented");
        assert_eq!(e.scenario, Scenario::default(), "the scenario starts as the built-in defaults, which are the values the simulation always used");
    }

    #[test]
    fn a_current_layout_round_trips_and_is_not_marked_as_migrated() {
        let mut e = Extensions::default();
        e.ecosystem.export = 12.0;
        e.scenario.source = "test".into();
        let bytes = bincode::serialize(&e).unwrap();
        let back = decode(EXT_VERSION, bytes).unwrap();
        assert_eq!(back.migrated_from, None);
        assert_eq!(back.scenario.source, "test");
    }

    #[test]
    fn a_newer_or_missing_layout_is_refused_not_guessed() {
        assert!(decode(EXT_VERSION + 1, Vec::new()).err().unwrap().contains("newer"));
        assert!(decode(0, Vec::new()).is_err());
        assert!(migrate(1, Vec::new(), &[], EXT_VERSION).unwrap_err().contains("no step"));
    }

    #[test]
    fn the_migration_steps_form_an_unbroken_chain_to_the_current_layout() {
        let froms: Vec<u32> = steps().iter().map(|s| s.from).collect();
        let want: Vec<u32> = (1..EXT_VERSION).collect();
        assert_eq!(froms, want, "every layout from 1 up to the current one must have exactly one step");
    }
}
