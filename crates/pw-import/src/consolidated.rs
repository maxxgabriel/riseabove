//! Metadata of an immutable consolidated database. Catalog-only identities are
//! never materialized as simulated people merely because they have an ID.
use std::path::Path;

use serde::Deserialize;

use crate::{ImportError, ImportSet};

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub name: String,
    pub start_date: String,
    pub archive_snapshot: String,
    pub fm_snapshot: String,
    pub registry_stamp: String,
    pub catalog_players: usize,
    pub identity_conflicts: usize,
    pub current_squad_coverage: Vec<String>,
    pub freshness_note: String,
}

pub fn manifest(dir: &Path) -> Result<Option<Manifest>, ImportError> {
    let file = dir.join("riseabove.database.json");
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(ImportError::Io { file: file.display().to_string(), source }),
    };
    let value: Manifest = serde_json::from_str(&text).map_err(|e| ImportError::Config(format!("database manifest: {e}")))?;
    if value.schema_version != 1 || crate::table::parse_date(&value.start_date).is_none() || crate::table::parse_date(&value.archive_snapshot).is_none() {
        return Err(ImportError::Config("unsupported database manifest or invalid snapshot date".into()));
    }
    Ok(Some(value))
}

pub(crate) fn annotate(set: &mut ImportSet, meta: &Manifest) {
    if let Some(source) = set.sources.first_mut() {
        source.snapshot = crate::table::parse_date(&meta.archive_snapshot);
        source.note = format!("Consolidated pack: {}", meta.freshness_note);
    }
    let fpl = set.add_source("fpl", set.start, "Premier League current squad feed; numeric Opta IDs, birth dates and current club. Fantasy prices are not market values.");
    for player in &mut set.players {
        if player.key.starts_with("fpl:") { player.source = fpl; }
    }
    set.issues.add(crate::Severity::Info, "catalog_only", "catalog_players.csv", 0, "",
                   format!("{} catalog identities; only validated projection records enter the world", meta.catalog_players));
    set.issues.add(crate::Severity::Warning, "mixed_snapshot", "riseabove.database.json", 0, "", &meta.freshness_note);
    if meta.identity_conflicts > 0 {
        set.issues.add(crate::Severity::Warning, "quarantined_identity_links", "identity_conflicts.csv", 0, "",
                       format!("{} disputed links remain source records and were not merged", meta.identity_conflicts));
    }
}
