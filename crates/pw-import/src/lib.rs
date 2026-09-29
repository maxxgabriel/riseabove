//! Building worlds from data, as a pipeline:
//!
//! ```text
//! source files → adapter (parse) → ImportSet (validated) → resolve (ids, keys, identity) → assemble → World + origins
//! ```
//!
//! Adapters: the pack CSV format (`data/IMPORT_FORMAT.md`) and the Transfermarkt-style archive with its staff list
//! (`docs/DB_INTEGRATION_AUDIT.md`). The simulation never sees a file layout. `synthetic` builds fixtures for tests
//! and benchmarks only.

mod assemble;
pub mod builder;
mod csv_pack;
pub mod fixture;
pub mod geo;
pub mod infer;
pub mod model;
pub mod india;
mod positions;
mod resolve;
mod staff_list;
pub mod synthetic;
mod table;
pub mod transfermarkt;

use std::path::Path;

use pw_data::DataPack;
use pw_world::World;

pub use model::{ImportSet, Issue, Severity};
pub use positions::parse_positions;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{file}: {source}")]
    Csv { file: String, source: csv::Error },
    #[error("{file}: {source}")]
    Io { file: String, source: std::io::Error },
    #[error("missing required file {0}")]
    Missing(String),
    #[error("world.toml: {0}")]
    Config(String),
    #[error("{0}")]
    Empty(String),
}

#[derive(Debug, Default)]
pub struct ImportReport {
    pub nations: usize,
    pub competitions: usize,
    pub clubs: usize,
    pub players: usize,
    pub staff: usize,
    pub spells: usize,
    pub seasons: usize,
    /// The first findings that were more than informational.
    pub warnings: Vec<String>,
    /// Every finding code with its exact count, largest first.
    pub findings: Vec<(String, u32)>,
    /// Source rows that could not be placed safely (see `World::origins`).
    pub unresolved: usize,
    pub possible_duplicates: usize,
    /// Per fact group: people whose value was [imported, inferred, generated, unknown].
    pub facets: Vec<(&'static str, [usize; 4])>,
}

impl ImportReport {
    pub fn summary(&self) -> String {
        let mut s = format!(
            "{} nations, {} competitions, {} clubs, {} players, {} staff; {} past seasons, {} career spells; {} unresolved rows, {} possible duplicates",
            self.nations, self.competitions, self.clubs, self.players, self.staff, self.seasons, self.spells, self.unresolved, self.possible_duplicates
        );
        if !self.facets.is_empty() {
            s.push_str("\n  origin of each fact group (imported / estimated / generated / unknown):");
            for (name, c) in &self.facets {
                s.push_str(&format!("\n    {name:<14} {} / {} / {} / {}", c[0], c[1], c[2], c[3]));
            }
        }
        if !self.findings.is_empty() {
            s.push_str("\n  findings:");
            for (code, n) in self.findings.iter().take(24) {
                s.push_str(&format!("\n    {code:<24} {n}"));
            }
        }
        s
    }
}

/// A stable hash of a source id, used to key randomness by record rather than by position in a file.
pub fn hash64(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[derive(Clone, Copy, Debug)]
pub struct LoadOptions {
    /// Read the large per-match files of the archive (career counters, top scorers, shirt numbers).
    pub match_files: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self { match_files: true }
    }
}

#[derive(serde::Deserialize, Default)]
struct WorldToml {
    start_date: Option<String>,
    seed: Option<u64>,
}

/// Parse a folder of any supported layout into the import representation, without building a world.
pub fn parse_dir(dir: &Path, opt: LoadOptions) -> Result<ImportSet, ImportError> {
    let mut set = if transfermarkt::detect(dir) {
        let mut set = transfermarkt::parse(dir, &transfermarkt::TmOptions { start: None, with_match_files: opt.match_files })?;
        if let Ok(text) = std::fs::read_to_string(dir.join("world.toml")) {
            let cfg: WorldToml = toml::from_str(&text).map_err(|e| ImportError::Config(e.to_string()))?;
            if let Some(d) = cfg.start_date.as_deref().and_then(|s| table::parse_date(s)) {
                set.start = Some(d);
            }
            set.seed = cfg.seed.or(set.seed);
        }
        set
    } else {
        csv_pack::parse(dir)?
    };
    staff_list::parse_into(&mut set, dir)?;
    if set.clubs.is_empty() || set.players.is_empty() {
        return Err(ImportError::Empty("the folder holds no clubs or no players that could be read".into()));
    }
    resolve::resolve(&mut set);
    Ok(set)
}

/// Load a world from an import folder with a fresh random seed unless `world.toml` fixes one.
pub fn load_dir(dir: &Path, pack: DataPack) -> Result<(World, ImportReport), ImportError> {
    load_dir_seeded(dir, pack, None)
}

/// Load a world with an explicit seed (overrides `world.toml`). The real data is the same for every seed; everything
/// generated around it (hidden attributes not supplied, regens, staff, press, supporters) differs from seed to seed.
pub fn load_dir_seeded(dir: &Path, pack: DataPack, seed: Option<u64>) -> Result<(World, ImportReport), ImportError> {
    load_dir_with(dir, pack, seed, LoadOptions::default())
}

pub fn load_dir_with(dir: &Path, pack: DataPack, seed: Option<u64>, opt: LoadOptions) -> Result<(World, ImportReport), ImportError> {
    let set = parse_dir(dir, opt)?;
    Ok(assemble::assemble(&set, pack, seed))
}

/// Build a world from an already parsed set (used by tests and tools that adjust a set first).
pub fn build_world(set: &ImportSet, pack: DataPack, seed: Option<u64>) -> (World, ImportReport) {
    assemble::assemble(set, pack, seed)
}
