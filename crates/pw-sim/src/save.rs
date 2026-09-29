//! Saves: versioned container, bincode → lz4, written to a temp file then renamed (crash-safe, 01 §8).
//!
//! ```text
//! "PWSAVE03"  8 bytes  container tag (v1 was "PWSAVE01": no schema version, no checksum; "PWSAVE02" had no metadata block)
//! schema      u32 LE   version of the world model the payload was written with
//! checksum    u64 LE   FNV-1a of everything after it (metadata length, metadata, lz4 block), so truncation or bit rot is reported
//! meta_len    u32 LE   length of the metadata block
//! meta        ...      JSON [`Meta`]: created schema, build, world seed, migration history (readable without decoding the world)
//! lz4 block   ...      size-prepended lz4 of the bincode payload
//! ```
//!
//! The payload type changes whenever the world model does. Bump [`SCHEMA_VERSION`] with every change
//! to a serialised type and register a [`Step`] that turns the previous payload into the new one. A step
//! that needs the old shape keeps a frozen copy of the old type next to itself (`legacy_vN`), decodes with
//! it and encodes the new one. Older saves are then upgraded on load: the original is copied to a backup first
//! and the upgraded file replaces it only after the whole chain and a decode succeeded.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

const MAGIC: &[u8; 8] = b"PWSAVE03";
/// The first versioned container: the same as the current one without the metadata block.
const NO_META_MAGIC: &[u8; 8] = b"PWSAVE02";
const LEGACY_MAGIC: &[u8; 8] = b"PWSAVE01";
const HEADER: usize = 8 + 4 + 8;

/// Version of the serialised world model written by this build.
///
/// * 1: the unversioned development format (`PWSAVE01`).
/// * 2: first versioned format (development only).
/// * 3: provenance book, position-level squad plans, professional/public journalist records. Schemas 1 and 2 were development
///   formats whose world shape changed without steps; they cannot be upgraded, and say so.
pub const SCHEMA_VERSION: u32 = 3;
/// The oldest schema this build can still upgrade from.
pub const OLDEST_SUPPORTED: u32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("encode: {0}")]
    Encode(#[from] bincode::Error),
    #[error("not a Pathway save file")]
    Format,
    #[error("decompress: {0}")]
    Decompress(#[from] lz4_flex::block::DecompressError),
    #[error("this save is damaged (checksum mismatch); restore it from a backup")]
    Damaged,
    #[error("this save was made by a newer version of the game (save format {found}, this build reads up to {supported}); update the game to open it")]
    TooNew { found: u32, supported: u32 },
    #[error(
        "this save uses format {found}, which this build can no longer upgrade (oldest readable format is {oldest}). Nothing was changed; open it with the game version that wrote it, or start a new world"
    )]
    Unsupported { found: u32, oldest: u32 },
    #[error("upgrading the save from format {from} failed: {reason}. The original is untouched")]
    Migration { from: u32, reason: String },
    #[error("this save does not pass validation after loading: {0}. The file was not changed")]
    Invalid(String),
}

/// What is recorded about a save beside the world itself (§10.15). Readable with [`inspect`] without decoding the world.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Meta {
    /// The schema this save was first written with. `None` for a container that predates metadata: unknown, not zero.
    pub created_schema: Option<u32>,
    pub schema: u32,
    /// Version of the build that last wrote the file.
    pub build: String,
    pub world_seed: Option<u64>,
    /// Version of the import provenance model, when the world came from an import.
    pub import_provenance: Option<u32>,
    /// Every upgrade this file went through, in order.
    pub migrations: Vec<Migration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Migration {
    pub from: u32,
    pub to: u32,
    pub name: String,
}

/// What a save's writer knows about it beyond the bytes.
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub world_seed: Option<u64>,
    pub import_provenance: Option<u32>,
}

impl Info {
    pub fn of_world(w: &pw_world::World) -> Self {
        Self { world_seed: Some(w.seed), import_provenance: (!w.origins.sources.is_empty()).then_some(pw_world::origin::PROVENANCE_VERSION) }
    }
}

/// A seed for something a migration must invent, stable across runs: the same save and the same migration always give the same value
/// (§10.8). Never use fresh randomness in a migration.
pub fn stable_seed(world_seed: u64, migration_id: &str, entity_id: u64) -> u64 {
    let name = migration_id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3));
    pw_core::rng::hash_key(&[world_seed, name, entity_id])
}

/// One upgrade step: `from` → `from + 1`, working on the decompressed payload.
pub struct Step {
    pub from: u32,
    pub name: &'static str,
    pub apply: fn(Vec<u8>) -> Result<Vec<u8>, String>,
}

/// The steps that upgrade older payloads to [`SCHEMA_VERSION`], in order.
pub fn default_steps() -> &'static [Step] {
    &[]
}

/// What a save file is, read from its header alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveInfo {
    pub schema: u32,
    pub bytes: u64,
    pub compat: Compat,
    /// `None` for containers that predate metadata (or when it cannot be read).
    pub meta: Option<Meta>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Compat {
    Current,
    /// Opens after `steps` upgrade steps (a backup is written first).
    Upgradable { steps: usize },
    TooNew,
    Unsupported,
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn atomic_write(path: &Path, parts: &[&[u8]]) -> Result<(), SaveError> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        for p in parts {
            f.write_all(p)?;
        }
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn write_payload(payload: &[u8], meta: &Meta, path: &Path) -> Result<(), SaveError> {
    let packed = lz4_flex::compress_prepend_size(payload);
    let meta_bytes = serde_json::to_vec(meta).map_err(|e| SaveError::Migration { from: meta.schema, reason: format!("metadata: {e}") })?;
    let mut body = Vec::with_capacity(4 + meta_bytes.len() + packed.len());
    body.extend_from_slice(&(meta_bytes.len() as u32).to_le_bytes());
    body.extend_from_slice(&meta_bytes);
    body.extend_from_slice(&packed);
    atomic_write(path, &[MAGIC, &meta.schema.to_le_bytes(), &fnv1a(&body).to_le_bytes(), &body])
}

fn fresh_meta(schema: u32, info: &Info) -> Meta {
    Meta { created_schema: Some(schema), schema, build: env!("CARGO_PKG_VERSION").to_string(), world_seed: info.world_seed, import_provenance: info.import_provenance, migrations: Vec::new() }
}

pub fn save<T: Serialize>(value: &T, path: &Path) -> Result<(), SaveError> {
    save_with(value, path, &Info::default())
}

/// Save with what the writer knows about the value (world seed, import provenance) recorded in the metadata.
pub fn save_with<T: Serialize>(value: &T, path: &Path, info: &Info) -> Result<(), SaveError> {
    write_payload(&bincode::serialize(value)?, &fresh_meta(SCHEMA_VERSION, info), path)
}

struct Raw {
    schema: u32,
    /// `None` for a container that predates metadata.
    meta: Option<Meta>,
    packed: Vec<u8>,
}

fn parse(bytes: &[u8]) -> Result<Raw, SaveError> {
    if bytes.len() >= 8 && &bytes[..8] == LEGACY_MAGIC {
        return Err(SaveError::Unsupported { found: 1, oldest: OLDEST_SUPPORTED });
    }
    let with_meta = bytes.len() >= 8 && &bytes[..8] == MAGIC;
    if bytes.len() < HEADER || !(with_meta || &bytes[..8] == NO_META_MAGIC) {
        return Err(SaveError::Format);
    }
    let schema = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes"));
    let sum = u64::from_le_bytes(bytes[12..20].try_into().expect("8 bytes"));
    let body = &bytes[HEADER..];
    if fnv1a(body) != sum {
        return Err(SaveError::Damaged);
    }
    if !with_meta {
        return Ok(Raw { schema, meta: None, packed: body.to_vec() });
    }
    let len = body.get(..4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")) as usize).ok_or(SaveError::Format)?;
    let meta_bytes = body.get(4..4 + len).ok_or(SaveError::Format)?;
    let meta: Meta = serde_json::from_slice(meta_bytes).map_err(|_| SaveError::Format)?;
    Ok(Raw { schema, meta: Some(meta), packed: body[4 + len..].to_vec() })
}

fn classify(schema: u32, steps: &[Step], current: u32, oldest: u32) -> Compat {
    match schema {
        s if s == current => Compat::Current,
        s if s > current => Compat::TooNew,
        s if s < oldest => Compat::Unsupported,
        s => {
            let n = (s..current).filter(|v| steps.iter().any(|st| st.from == *v)).count();
            if n == (current - s) as usize { Compat::Upgradable { steps: n } } else { Compat::Unsupported }
        }
    }
}

/// Read a save's header and metadata without decoding the world.
pub fn inspect(path: &Path) -> Result<SaveInfo, SaveError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let bytes = f.metadata()?.len();
    let mut head = [0u8; HEADER + 4];
    let n = f.read(&mut head)?;
    if n >= 8 && &head[..8] == LEGACY_MAGIC {
        return Ok(SaveInfo { schema: 1, bytes, compat: Compat::Unsupported, meta: None });
    }
    let with_meta = n >= 8 && &head[..8] == MAGIC;
    if n < 12 || !(with_meta || &head[..8] == NO_META_MAGIC) {
        return Err(SaveError::Format);
    }
    let schema = u32::from_le_bytes(head[8..12].try_into().expect("4 bytes"));
    let meta = if with_meta && n >= HEADER + 4 {
        let len = u32::from_le_bytes(head[HEADER..HEADER + 4].try_into().expect("4 bytes")) as usize;
        let mut buf = vec![0u8; len.min(1 << 20)];
        f.read_exact(&mut buf).ok().and_then(|()| serde_json::from_slice::<Meta>(&buf).ok())
    } else {
        None
    };
    Ok(SaveInfo { schema, bytes, compat: classify(schema, default_steps(), SCHEMA_VERSION, OLDEST_SUPPORTED), meta })
}

/// A backup path beside `path` that does not exist yet: `name.schemaN.bak`, `name.schemaN.1.bak`, ...
fn backup_path(path: &Path, schema: u32) -> PathBuf {
    let stem = path.file_name().map_or_else(|| "save".into(), |s| s.to_string_lossy().to_string());
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut n = 0;
    loop {
        let name = if n == 0 { format!("{stem}.schema{schema}.bak") } else { format!("{stem}.schema{schema}.{n}.bak") };
        let p = dir.join(name);
        if !p.exists() {
            return p;
        }
        n += 1;
    }
}

pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, SaveError> {
    load_impl(path, default_steps(), SCHEMA_VERSION, OLDEST_SUPPORTED, None)
}

/// Load, and after any upgrade run `validate` on the result before the upgraded file replaces the original (sections 10.9, 10.10). A
/// failed validation leaves the original untouched and no backup behind.
pub fn load_checked<T: DeserializeOwned>(path: &Path, validate: &dyn Fn(&T) -> Result<(), String>) -> Result<T, SaveError> {
    load_impl(path, default_steps(), SCHEMA_VERSION, OLDEST_SUPPORTED, Some(validate))
}

/// A world, validated after any upgrade.
pub fn load_world(path: &Path) -> Result<pw_world::World, SaveError> {
    load_checked(path, &|w: &pw_world::World| crate::validate::check(w))
}

/// Load with an explicit chain (the default chain and versions are used by [`load`]; tests use their own).
/// An older save is upgraded in place after its original was copied to a backup.
pub fn load_with<T: DeserializeOwned>(path: &Path, steps: &[Step], current: u32, oldest: u32) -> Result<T, SaveError> {
    load_impl(path, steps, current, oldest, None)
}

/// [`load_with`] plus a validator run on the upgraded value.
pub fn load_with_checked<T: DeserializeOwned>(path: &Path, steps: &[Step], current: u32, oldest: u32, validate: &dyn Fn(&T) -> Result<(), String>) -> Result<T, SaveError> {
    load_impl(path, steps, current, oldest, Some(validate))
}

fn load_impl<T: DeserializeOwned>(path: &Path, steps: &[Step], current: u32, oldest: u32, validate: Option<&dyn Fn(&T) -> Result<(), String>>) -> Result<T, SaveError> {
    let bytes = std::fs::read(path)?;
    let raw = parse(&bytes)?;
    match classify(raw.schema, steps, current, oldest) {
        Compat::Current => Ok(bincode::deserialize(&lz4_flex::decompress_size_prepended(&raw.packed)?)?),
        Compat::TooNew => Err(SaveError::TooNew { found: raw.schema, supported: current }),
        Compat::Unsupported => Err(SaveError::Unsupported { found: raw.schema, oldest }),
        Compat::Upgradable { .. } => {
            let mut payload = lz4_flex::decompress_size_prepended(&raw.packed)?;
            let mut meta = raw.meta.clone().unwrap_or_else(|| Meta { created_schema: None, schema: raw.schema, build: String::new(), world_seed: None, import_provenance: None, migrations: Vec::new() });
            for v in raw.schema..current {
                let step = steps.iter().find(|s| s.from == v).expect("classified as complete");
                payload = (step.apply)(payload).map_err(|reason| SaveError::Migration { from: v, reason: format!("{}: {reason}", step.name) })?;
                meta.migrations.push(Migration { from: v, to: v + 1, name: step.name.to_string() });
            }
            let value: T = bincode::deserialize(&payload).map_err(|e| SaveError::Migration { from: raw.schema, reason: format!("upgraded data does not decode: {e}") })?;
            if let Some(check) = validate {
                check(&value).map_err(SaveError::Invalid)?;
            }
            meta.schema = current;
            meta.build = env!("CARGO_PKG_VERSION").to_string();
            std::fs::copy(path, backup_path(path, raw.schema))?;
            write_payload(&payload, &meta, path)?;
            Ok(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct V1 {
        name: String,
        goals: u32,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct V3 {
        name: String,
        goals: u32,
        assists: u32,
        team: String,
    }

    fn v1_to_v2(p: Vec<u8>) -> Result<Vec<u8>, String> {
        // v2 added `assists`.
        #[derive(Serialize)]
        struct V2 {
            name: String,
            goals: u32,
            assists: u32,
        }
        let old: V1 = bincode::deserialize(&p).map_err(|e| e.to_string())?;
        bincode::serialize(&V2 { name: old.name, goals: old.goals, assists: 0 }).map_err(|e| e.to_string())
    }

    fn v2_to_v3(p: Vec<u8>) -> Result<Vec<u8>, String> {
        #[derive(Deserialize)]
        struct V2 {
            name: String,
            goals: u32,
            assists: u32,
        }
        let old: V2 = bincode::deserialize(&p).map_err(|e| e.to_string())?;
        bincode::serialize(&V3 { name: old.name, goals: old.goals, assists: old.assists, team: "unknown".into() }).map_err(|e| e.to_string())
    }

    const STEPS: &[Step] = &[Step { from: 1, name: "add assists", apply: v1_to_v2 }, Step { from: 2, name: "add team", apply: v2_to_v3 }];

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pw-save-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("world.pws")
    }

    fn write_at(path: &Path, schema: u32, v: &impl Serialize) {
        let meta = Meta { created_schema: Some(schema), schema, build: "test".into(), world_seed: None, import_provenance: None, migrations: Vec::new() };
        write_payload(&bincode::serialize(v).unwrap(), &meta, path).unwrap();
    }

    #[test]
    fn current_round_trip() {
        let p = temp("round");
        let v = V1 { name: "Ana".into(), goals: 3 };
        save(&v, &p).unwrap();
        assert_eq!(inspect(&p).unwrap().schema, SCHEMA_VERSION);
        assert_eq!(inspect(&p).unwrap().compat, Compat::Current);
        assert_eq!(load::<V1>(&p).unwrap(), v);
    }

    #[test]
    fn old_save_is_backed_up_then_upgraded_through_every_step() {
        let p = temp("chain");
        write_at(&p, 1, &V1 { name: "Ana".into(), goals: 3 });
        let original = std::fs::read(&p).unwrap();
        let got: V3 = load_with(&p, STEPS, 3, 1).unwrap();
        assert_eq!(got, V3 { name: "Ana".into(), goals: 3, assists: 0, team: "unknown".into() });
        // The original bytes survive in the backup and the file itself is now current.
        let bak = p.with_file_name("world.pws.schema1.bak");
        assert_eq!(std::fs::read(&bak).unwrap(), original);
        assert_eq!(inspect(&p).unwrap().schema, 3);
        // A second upgrade of another old file never overwrites an earlier backup.
        assert_eq!(backup_path(&p, 1), p.with_file_name("world.pws.schema1.1.bak"));
    }

    #[test]
    fn upgraded_file_loads_again_without_a_second_backup() {
        let p = temp("again");
        write_at(&p, 1, &V1 { name: "Bo".into(), goals: 1 });
        let first: V3 = load_with(&p, STEPS, 3, 1).unwrap();
        let second: V3 = load_with(&p, STEPS, 3, 1).unwrap();
        assert_eq!(first, second);
        // The save itself plus exactly one backup.
        assert_eq!(std::fs::read_dir(p.parent().unwrap()).unwrap().count(), 2);
    }

    #[test]
    fn newer_save_is_refused_with_a_clear_message_and_left_alone() {
        let p = temp("new");
        write_at(&p, 9, &V1 { name: "x".into(), goals: 1 });
        let before = std::fs::read(&p).unwrap();
        let err = load_with::<V1>(&p, STEPS, 3, 1).unwrap_err();
        assert!(matches!(err, SaveError::TooNew { found: 9, supported: 3 }));
        assert!(err.to_string().contains("newer version"));
        assert_eq!(std::fs::read(&p).unwrap(), before);
        assert!(!p.with_file_name("world.pws.schema9.bak").exists());
    }

    #[test]
    fn too_old_or_missing_step_is_unsupported_and_untouched() {
        let p = temp("old");
        write_at(&p, 1, &V1 { name: "x".into(), goals: 1 });
        // No step registered from 1.
        let err = load_with::<V1>(&p, &[], 3, 1).unwrap_err();
        assert!(matches!(err, SaveError::Unsupported { found: 1, .. }));
        // Below the oldest supported version.
        let err = load_with::<V1>(&p, STEPS, 3, 2).unwrap_err();
        assert!(err.to_string().contains("no longer upgrade"));
        assert!(std::fs::read_dir(p.parent().unwrap()).unwrap().count() == 1, "no backup for a refused save");
    }

    #[test]
    fn failed_step_keeps_the_original_and_writes_no_backup() {
        fn boom(_: Vec<u8>) -> Result<Vec<u8>, String> {
            Err("cannot convert".into())
        }
        let p = temp("fail");
        write_at(&p, 1, &V1 { name: "x".into(), goals: 1 });
        let before = std::fs::read(&p).unwrap();
        let err = load_with::<V1>(&p, &[Step { from: 1, name: "boom", apply: boom }], 2, 1).unwrap_err();
        assert!(matches!(err, SaveError::Migration { from: 1, .. }));
        assert_eq!(std::fs::read(&p).unwrap(), before);
        assert_eq!(std::fs::read_dir(p.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn upgraded_data_that_does_not_decode_is_a_migration_error() {
        let p = temp("wrongshape");
        write_at(&p, 1, &V1 { name: "x".into(), goals: 1 });
        // A step that returns bytes of the wrong shape must not overwrite the save.
        fn wrong(_: Vec<u8>) -> Result<Vec<u8>, String> {
            Ok(vec![1, 2])
        }
        let err = load_with::<V3>(&p, &[Step { from: 1, name: "wrong", apply: wrong }], 2, 1).unwrap_err();
        assert!(matches!(err, SaveError::Migration { .. }));
        assert_eq!(inspect(&p).unwrap().schema, 1);
    }

    #[test]
    fn the_development_formats_are_refused_with_a_plain_message_and_left_alone() {
        let p = temp("dev2");
        write_at(&p, 2, &V1 { name: "old".into(), goals: 1 });
        let before = std::fs::read(&p).unwrap();
        let err = load::<V1>(&p).unwrap_err();
        assert!(matches!(err, SaveError::Unsupported { found: 2, oldest: OLDEST_SUPPORTED }), "{err}");
        assert!(err.to_string().contains("no longer upgrade") && err.to_string().contains("Nothing was changed"));
        assert_eq!(inspect(&p).unwrap().compat, Compat::Unsupported);
        assert_eq!(std::fs::read(&p).unwrap(), before);
    }

    #[test]
    fn legacy_unversioned_saves_are_recognised() {
        let p = temp("legacy");
        let mut bytes = LEGACY_MAGIC.to_vec();
        bytes.extend_from_slice(&lz4_flex::compress_prepend_size(&bincode::serialize(&V1 { name: "x".into(), goals: 0 }).unwrap()));
        std::fs::write(&p, &bytes).unwrap();
        let info = inspect(&p).unwrap();
        assert_eq!((info.schema, info.compat), (1, Compat::Unsupported));
        let err = load::<V1>(&p).unwrap_err();
        assert!(matches!(err, SaveError::Unsupported { found: 1, .. }), "{err}");
        assert_eq!(std::fs::read(&p).unwrap(), bytes);
    }

    #[test]
    fn corruption_and_foreign_files_are_reported() {
        let p = temp("bad");
        save(&V1 { name: "x".into(), goals: 1 }, &p).unwrap();
        let mut bytes = std::fs::read(&p).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        std::fs::write(&p, &bytes).unwrap();
        assert!(matches!(load::<V1>(&p).unwrap_err(), SaveError::Damaged));
        std::fs::write(&p, b"hello").unwrap();
        assert!(matches!(load::<V1>(&p).unwrap_err(), SaveError::Format));
        assert!(matches!(inspect(&p).unwrap_err(), SaveError::Format));
    }

    #[test]
    fn metadata_records_how_the_save_began_and_every_upgrade_it_went_through() {
        let p = temp("meta");
        save_with(&V1 { name: "x".into(), goals: 1 }, &p, &Info { world_seed: Some(42), import_provenance: Some(1) }).unwrap();
        let m = inspect(&p).unwrap().meta.expect("metadata");
        assert_eq!((m.created_schema, m.schema, m.world_seed, m.import_provenance), (Some(SCHEMA_VERSION), SCHEMA_VERSION, Some(42), Some(1)));
        assert!(m.migrations.is_empty() && !m.build.is_empty());

        let q = temp("meta-up");
        write_at(&q, 1, &V1 { name: "Ana".into(), goals: 3 });
        let _: V3 = load_with(&q, STEPS, 3, 1).unwrap();
        let m = inspect(&q).unwrap().meta.expect("metadata");
        assert_eq!(m.created_schema, Some(1), "the schema it began with is kept");
        assert_eq!(m.schema, 3);
        assert_eq!(m.migrations.iter().map(|x| (x.from, x.to, x.name.as_str())).collect::<Vec<_>>(), vec![(1, 2, "add assists"), (2, 3, "add team")]);
    }

    #[test]
    fn a_container_from_before_metadata_still_loads_and_its_origin_is_unknown_not_zero() {
        let p = temp("nometa");
        let payload = bincode::serialize(&V1 { name: "old".into(), goals: 2 }).unwrap();
        let packed = lz4_flex::compress_prepend_size(&payload);
        atomic_write(&p, &[NO_META_MAGIC, &SCHEMA_VERSION.to_le_bytes(), &fnv1a(&packed).to_le_bytes(), &packed]).unwrap();
        let info = inspect(&p).unwrap();
        assert_eq!((info.schema, info.compat, info.meta.clone()), (SCHEMA_VERSION, Compat::Current, None));
        assert_eq!(load::<V1>(&p).unwrap(), V1 { name: "old".into(), goals: 2 });
        // Upgraded, the metadata starts from "unknown" rather than pretending a creation schema.
        let q = temp("nometa-up");
        let packed = lz4_flex::compress_prepend_size(&bincode::serialize(&V1 { name: "old".into(), goals: 2 }).unwrap());
        atomic_write(&q, &[NO_META_MAGIC, &1u32.to_le_bytes(), &fnv1a(&packed).to_le_bytes(), &packed]).unwrap();
        let _: V3 = load_with(&q, STEPS, 3, 1).unwrap();
        assert_eq!(inspect(&q).unwrap().meta.unwrap().created_schema, None);
    }

    #[test]
    fn a_failed_validation_after_upgrade_keeps_the_original_and_writes_no_backup() {
        let p = temp("invalid");
        write_at(&p, 1, &V1 { name: "Ana".into(), goals: 3 });
        let before = std::fs::read(&p).unwrap();
        let err = load_with_checked::<V3>(&p, STEPS, 3, 1, &|v| if v.team == "unknown" { Err("no team".into()) } else { Ok(()) }).unwrap_err();
        assert!(matches!(err, SaveError::Invalid(ref m) if m == "no team"), "{err}");
        assert_eq!(std::fs::read(&p).unwrap(), before, "the original is untouched");
        assert_eq!(std::fs::read_dir(p.parent().unwrap()).unwrap().count(), 1, "no backup, no temp file");
        // A validator that accepts lets the upgrade through.
        let ok: V3 = load_with_checked(&p, STEPS, 3, 1, &|_| Ok(())).unwrap();
        assert_eq!(ok.goals, 3);
    }

    #[test]
    fn migration_seeds_are_stable_and_differ_by_migration_and_entity() {
        assert_eq!(stable_seed(7, "add-form", 12), stable_seed(7, "add-form", 12));
        assert_ne!(stable_seed(7, "add-form", 12), stable_seed(7, "add-form", 13));
        assert_ne!(stable_seed(7, "add-form", 12), stable_seed(7, "add-mood", 12));
        assert_ne!(stable_seed(7, "add-form", 12), stable_seed(8, "add-form", 12));
    }
}
