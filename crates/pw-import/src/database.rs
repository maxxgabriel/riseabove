//! Local source database, independent of the simulated world's save.
//!
//! CSV rows stay in the user's folder. Compact, disposable indexes hold byte offsets and foreign keys;
//! a query seeks only the requested rows. No FM name or candidate ID becomes a player or a club relation.

use std::collections::{BTreeMap, VecDeque};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use bincode::Options;

use crate::{ImportError, hash64};
use crate::table::decode_cp1252;

const CACHE_VERSION: u32 = 4;
const MEMORY_BUDGET: usize = 96 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub bytes: u64,
    pub source: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Record {
    /// Stable within this unchanged source file, not a simulated entity ID.
    pub row: u32,
    pub fields: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct Page {
    pub table: String,
    pub columns: Vec<String>,
    pub indexed_columns: Vec<String>,
    pub rows: Vec<Record>,
    pub total: usize,
    pub matched: usize,
    pub malformed: u32,
    pub offset: usize,
    pub index_bytes: usize,
    pub cached: bool,
    pub name_search: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Stamp {
    version: u32,
    bytes: u64,
    modified_ns: u128,
}

#[derive(Serialize, Deserialize)]
struct Index {
    stamp: Stamp,
    headers: Vec<String>,
    offsets: Vec<u64>,
    // Hash collisions are checked against the actual row before a record is returned.
    keys: BTreeMap<String, Vec<(u32, u32)>>,
    // A dense key number halves each posting's size; the exact source string still proves identity.
    values: BTreeMap<String, BTreeMap<u64, (String, u32)>>,
    collisions: BTreeMap<String, std::collections::BTreeSet<u64>>,
    names: Vec<(String, u32)>,
    malformed: u32,
}

impl Index {
    fn compact(&mut self) {
        self.offsets.shrink_to_fit();
        for keys in self.keys.values_mut() { keys.shrink_to_fit(); }
        self.names.shrink_to_fit();
    }

    fn valid(&self, stamp: &Stamp) -> bool {
        self.stamp == *stamp && !self.headers.is_empty()
            && self.offsets.iter().all(|offset| *offset < stamp.bytes)
            && self.offsets.windows(2).all(|pair| pair[0] < pair[1])
            && self.keys.iter().all(|(column, keys)| {
                self.headers.contains(column) && self.values.contains_key(column)
                    && keys.iter().all(|(key, row)| (*row as usize) < self.offsets.len() && (*key as usize) < self.values[column].len())
                    && keys.windows(2).all(|pair| pair[0] <= pair[1])
            })
            && (self.names.is_empty() || (self.names.len() == self.offsets.len()
                && self.names.iter().enumerate().all(|(i, (_, row))| *row as usize == i)))
    }

    fn bytes(&self) -> usize {
        self.offsets.capacity() * 8 + self.keys.values().map(|v| v.capacity() * std::mem::size_of::<(u32, u32)>()).sum::<usize>()
            + self.names.iter().map(|(s, _)| s.capacity() + std::mem::size_of::<(String, u32)>()).sum::<usize>()
            + self.values.values().flat_map(|v| v.values()).map(|(s, _)| s.capacity() + 72).sum::<usize>()
    }
}

pub struct Catalog {
    root: PathBuf,
    cache: PathBuf,
    tables: Vec<TableInfo>,
    indexes: VecDeque<(String, Index)>,
}

fn io(file: &str, source: std::io::Error) -> ImportError {
    ImportError::Io { file: file.into(), source }
}

fn stamp(path: &Path) -> Result<Stamp, ImportError> {
    let md = path.metadata().map_err(|e| io(&path.display().to_string(), e))?;
    Ok(Stamp { version: CACHE_VERSION, bytes: md.len(), modified_ns: md.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos()) })
}

fn reader(path: &Path, name: &str, capacity: usize) -> Result<csv::Reader<BufReader<File>>, ImportError> {
    let f = File::open(path).map_err(|e| io(name, e))?;
    Ok(csv::ReaderBuilder::new().delimiter(if name.eq_ignore_ascii_case("Staff list.csv") { b';' } else { b',' }).flexible(true)
        .from_reader(BufReader::with_capacity(capacity, f)))
}

fn text(bytes: &[u8], cp1252: bool) -> String {
    if cp1252 { decode_cp1252(bytes).trim().to_string() } else { String::from_utf8_lossy(bytes).trim().to_string() }
}

fn index_key(col: &str) -> bool {
    // UUID primary keys in multi-million-row fact tables add no useful relationship index.
    (col == "id" || col == "entity_id" || col.ends_with("_id"))
        && !matches!(col, "appearance_id" | "game_lineups_id" | "game_event_id" | "game_events_id" | "id_status" | "person_id_candidate")
}

fn build(path: &Path, name: &str, initial: Stamp) -> Result<Index, ImportError> {
    let mut rdr = reader(path, name, 1 << 20)?;
    let cp1252 = name.eq_ignore_ascii_case("Staff list.csv");
    let mut headers: Vec<String> = rdr.byte_headers().map_err(|source| ImportError::Csv { file: name.into(), source })?.iter()
        .map(|b| text(b, cp1252).trim_start_matches('\u{feff}').to_lowercase()).collect();
    let mut used = std::collections::BTreeSet::new();
    for (i, header) in headers.iter_mut().enumerate() {
        if header.is_empty() { *header = format!("field_{}", i + 1); }
        let original = header.clone();
        let mut suffix = 2;
        while !used.insert(header.clone()) { *header = format!("{original}_{suffix}"); suffix += 1; }
    }
    let key_cols: Vec<usize> = headers.iter().enumerate().filter_map(|(i, h)| index_key(h).then_some(i)).collect();
    let name_cols: Vec<usize> = headers.iter().enumerate().filter_map(|(i, h)| matches!(h.as_str(), "name" | "first_name" | "last_name" | "short_name" | "coach_name").then_some(i)).collect();
    let mut idx = Index { stamp: initial, headers, offsets: Vec::new(), keys: BTreeMap::new(), values: BTreeMap::new(), collisions: BTreeMap::new(), names: Vec::new(), malformed: 0 };
    for &col in &key_cols {
        idx.keys.insert(idx.headers[col].clone(), Vec::new());
        idx.values.insert(idx.headers[col].clone(), BTreeMap::new());
    }
    let mut rec = csv::ByteRecord::new();
    loop {
        match rdr.read_byte_record(&mut rec) {
            Ok(false) => break,
            Err(csv_err) => {
                if csv_err.is_io_error() { return Err(ImportError::Csv { file: name.into(), source: csv_err }); }
                idx.malformed += 1;
                continue;
            }
            Ok(true) => {}
        }
        let row = u32::try_from(idx.offsets.len()).map_err(|_| ImportError::Config(format!("{name}: more than 2^32 records")))?;
        while idx.headers.len() < rec.len() { idx.headers.push(format!("extra_field_{}", idx.headers.len() + 1)); }
        idx.offsets.push(rec.position().expect("CSV position").byte());
        for &col in &key_cols {
            let value = rec.get(col).map(|b| text(b, cp1252)).unwrap_or_default();
            if !value.is_empty() {
                let hash = hash64(&value);
                let header = &idx.headers[col];
                let values = idx.values.get_mut(header).expect("column");
                let next = u32::try_from(values.len()).map_err(|_| ImportError::Config(format!("{name}: too many distinct IDs")))?;
                let dense = match values.entry(hash) {
                    std::collections::btree_map::Entry::Vacant(e) => { e.insert((value, next)); next }
                    std::collections::btree_map::Entry::Occupied(e) => {
                        if e.get().0 != value { idx.collisions.entry(header.clone()).or_default().insert(hash); }
                        e.get().1
                    }
                };
                idx.keys.get_mut(header).expect("column").push((dense, row));
            }
        }
        if !name_cols.is_empty() {
            let name = name_cols.iter().filter_map(|&i| rec.get(i)).map(|b| text(b, cp1252)).collect::<Vec<_>>().join(" ").to_lowercase();
            idx.names.push((name, row));
        }
    }
    if stamp(path)? != idx.stamp { return Err(ImportError::Config(format!("{name} changed while it was being indexed; retry"))); }
    for keys in idx.keys.values_mut() { keys.sort_unstable(); }
    idx.compact();
    Ok(idx)
}

impl Catalog {
    /// A source may be the archive, a pack, the usable FM folder, or the parent of the extracted FM folder.
    pub fn open(dir: &Path, cache: &Path) -> Result<Self, ImportError> {
        let root = [dir.to_path_buf(), dir.join("usable"), dir.join("2300_fm/usable")].into_iter()
            .find(|p| p.is_dir() && std::fs::read_dir(p).is_ok_and(|rd| rd.flatten().any(|e| e.path().extension().is_some_and(|x| x == "csv"))))
            .ok_or_else(|| ImportError::Empty("No exported CSV tables found in this folder.".into()))?
            .canonicalize().map_err(|e| io("database folder", e))?;
        let fm = root.join("fm23_export_catalog.csv").exists();
        let consolidated = root.join("riseabove.database.json").exists();
        let family = if fm { "FM23" } else if crate::transfermarkt::detect(&root) { "Transfermarkt" } else { "CSV pack" };
        let mut tables = Vec::new();
        for entry in std::fs::read_dir(&root).map_err(|e| io("database folder", e))? {
            let entry = entry.map_err(|e| io("database folder", e))?;
            if !entry.file_type().map_err(|e| io("database entry", e))?.is_file() || entry.path().extension().is_none_or(|x| x != "csv") { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            let source = if consolidated && name.starts_with("fm23_") { "FM23" }
                else if consolidated && name.starts_with("worldcup_") { "OpenFootball" }
                else if consolidated && name.starts_with("registry_") { "Reep" }
                else { family };
            let status = if consolidated && name == "catalog_players.csv" { "reconciled identity catalog; not all records are playable" }
            else if consolidated && name == "worldcup_2026_squads.csv" { "registered squads and dated evidence: 3 July 2026; unlinked rows are not assumed to be new people" }
            else if consolidated && name == "fm23_player_records.csv" { "historical player facts: June 2022" }
            else if consolidated && name.starts_with("fm23_legacy_") { "legacy export; relationships unverified" }
            else if name.contains("unresolved") || name.contains("manifest") || name.contains("catalog") || name.contains("blocks") || name.contains("sections") || name == "fm23_tables.csv" {
                "source ledger"
            } else if fm && name == "players.csv" { "exported player records; relationships unverified" }
            else if fm && (name.contains("people") || name.contains("manual")) { "names only; relations unknown" }
            else if fm { "verified exported names; relations unknown" } else { "source records" };
            tables.push(TableInfo { name, bytes: entry.metadata().map_err(|e| io("database entry", e))?.len(), source: source.into(), status: status.into() });
        }
        tables.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { root, cache: cache.to_path_buf(), tables, indexes: VecDeque::new() })
    }

    pub fn root(&self) -> &Path { &self.root }
    pub fn tables(&self) -> &[TableInfo] { &self.tables }

    fn index(&mut self, name: &str) -> Result<bool, ImportError> {
        if !self.tables.iter().any(|t| t.name == name) { return Err(ImportError::Config("Choose a table from this database.".into())); }
        let path = self.root.join(name);
        let current = stamp(&path)?;
        if let Some(pos) = self.indexes.iter().position(|(n, i)| n == name && i.stamp == current) {
            let entry = self.indexes.remove(pos).expect("found");
            self.indexes.push_front(entry);
            return Ok(true);
        }
        self.indexes.retain(|(n, _)| n != name);
        let cache_name = format!("{:016x}.pwi", hash64(&format!("{}|{name}", self.root.display())));
        let cache_path = self.cache.join(cache_name);
        // Stamp is the first field. Reject stale caches before allocating their large arrays.
        let cache_current = File::open(&cache_path).ok().and_then(|f| bincode::deserialize_from::<_, Stamp>(BufReader::new(f)).ok()).is_some_and(|s| s == current);
        let cached = cache_current.then(|| File::open(&cache_path).ok().and_then(|f| {
            let len = f.metadata().ok()?.len();
            bincode::DefaultOptions::new().with_fixint_encoding().with_limit(len)
                .deserialize_from::<_, Index>(BufReader::new(f)).ok().filter(|idx| idx.valid(&current))
                .map(|mut idx| { idx.compact(); idx })
        })).flatten();
        let from_cache = cached.is_some();
        let idx = match cached { Some(i) => i, None => build(&path, name, current)? };
        if !from_cache && std::fs::create_dir_all(&self.cache).is_ok() {
            // Cache failure never makes source data unavailable. Commit a complete cache atomically.
            let tmp = cache_path.with_extension(format!("{}.tmp", std::process::id()));
            if let Ok(f) = File::create(&tmp) {
                let mut writer = BufWriter::new(f);
                let ok = bincode::serialize_into(&mut writer, &idx).is_ok() && writer.flush().is_ok();
                drop(writer);
                if ok {
                    let _ = std::fs::remove_file(&cache_path);
                    let _ = std::fs::rename(&tmp, &cache_path);
                } else { let _ = std::fs::remove_file(&tmp); }
            }
        }
        // Always retain the current table, even when it alone exceeds the budget. Evict older tables.
        while !self.indexes.is_empty() && self.indexes.iter().map(|(_, i)| i.bytes()).sum::<usize>() + idx.bytes() > MEMORY_BUDGET { self.indexes.pop_back(); }
        self.indexes.push_front((name.into(), idx));
        Ok(from_cache)
    }

    /// Exact foreign-key filtering uses a sorted posting list. Name search applies only to name columns.
    /// Full history tables can be browsed or selected by player/game/club ID without scanning their contents.
    pub fn query(&mut self, name: &str, column: Option<&str>, value: &str, search: &str, offset: usize, limit: usize) -> Result<Page, ImportError> {
        let cached = self.index(name)?;
        let idx = &self.indexes.front().expect("indexed").1;
        let mut rdr = reader(&self.root.join(name), name, 8 << 10)?;
        let cp1252 = name.eq_ignore_ascii_case("Staff list.csv");
        // Prime the reader before seeking so headers are never read as a data row.
        rdr.byte_headers().map_err(|source| ImportError::Csv { file: name.into(), source })?;
        let mut rec = csv::ByteRecord::new();
        let mut read = |row: u32| -> Result<Record, ImportError> {
            let mut position = csv::Position::new();
            position.set_byte(idx.offsets[row as usize]);
            rdr.seek(position).map_err(|source| ImportError::Csv { file: name.into(), source })?;
            if !rdr.read_byte_record(&mut rec).map_err(|source| ImportError::Csv { file: name.into(), source })? {
                return Err(ImportError::Config(format!("{name} changed; reload the database")));
            }
            Ok(Record { row, fields: idx.headers.iter().enumerate().map(|(i, h)| {
                let s = rec.get(i).map(|b| text(b, cp1252)).unwrap_or_default();
                (h.clone(), (!s.is_empty()).then_some(s))
            }).collect() })
        };
        let search = search.trim().to_lowercase();
        if !search.is_empty() && idx.names.is_empty() {
            return Err(ImportError::Config("This table has no name fields. Filter by a source ID instead.".into()));
        }
        let candidates: Vec<u32> = if let Some(column) = column {
            let postings = idx.keys.get(column).ok_or_else(|| ImportError::Config(format!("{column} is not an indexed ID column")))?;
            let hash = hash64(value);
            let identity = idx.values[column].get(&hash);
            let key = identity.map_or(u32::MAX, |(_, key)| *key);
            let begin = postings.partition_point(|(k, _)| *k < key);
            let end = postings.partition_point(|(k, _)| *k <= key);
            // Retained source strings decide identity. Usually this reads no rows before pagination;
            // an actual hash collision falls back to verifying each candidate against the source.
            if idx.collisions.get(column).is_some_and(|c| c.contains(&hash)) {
                let mut ids = Vec::with_capacity(end - begin);
                for &(_, row) in &postings[begin..end] {
                    if read(row)?.fields.get(column).and_then(Option::as_deref) == Some(value) { ids.push(row); }
                }
                ids
            } else if identity.is_some_and(|(s, _)| s == value) { postings[begin..end].iter().map(|(_, row)| *row).collect() }
            else { Vec::new() }
        } else if search.is_empty() {
            Vec::new() // Unfiltered pagination does not allocate a list of all row IDs.
        } else if idx.names.is_empty() {
            return Err(ImportError::Config("This table has no name fields. Filter by a source ID instead.".into()));
        } else { idx.names.iter().filter(|(n, _)| n.contains(&search)).map(|(_, row)| *row).collect() };
        let filtered: Vec<u32> = if column.is_some() && !search.is_empty() {
            candidates.into_iter().filter(|row| idx.names.get(*row as usize).is_some_and(|(s, _)| s.contains(&search))).collect()
        } else { candidates };
        let all = column.is_none() && search.is_empty();
        let matched = if all { idx.offsets.len() } else { filtered.len() };
        let end = offset.saturating_add(limit.clamp(1, 100)).min(matched);
        let mut rows = Vec::new();
        for i in offset.min(end)..end { rows.push(read(if all { i as u32 } else { filtered[i] })?); }
        if stamp(&self.root.join(name))? != idx.stamp { return Err(ImportError::Config(format!("{name} changed during the query; retry"))); }
        Ok(Page { table: name.into(), columns: idx.headers.clone(), indexed_columns: idx.keys.keys().cloned().collect(), rows, total: idx.offsets.len(), matched,
            malformed: idx.malformed, offset, index_bytes: idx.bytes(), cached, name_search: !idx.names.is_empty() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);

    #[test]
    fn indexed_rows_preserve_unknowns_quotes_keys_and_survive_cache_reload() {
        let dir = std::env::temp_dir().join(format!("pw-database-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("players.csv"), "player_id,name,value\n01,\"Alex, One\",\n1,Alex Two,0\n2,\"Long\nName\",42\n").unwrap();
        std::fs::write(dir.join("appearances.csv"), "player_id,game_id,goals\n01,9,\n1,9,0\n01,10,2\n").unwrap();
        let cache = dir.join("cache");
        let mut db = Catalog::open(&dir, &cache).unwrap();
        let p = db.query("players.csv", None, "", "alex", 0, 50).unwrap();
        assert_eq!(p.matched, 2);
        assert_eq!(p.rows[0].fields["value"], None);
        assert_eq!(p.rows[1].fields["value"].as_deref(), Some("0"));
        assert_eq!(db.query("players.csv", None, "", "long", 0, 1).unwrap().rows[0].fields["name"].as_deref(), Some("Long\nName"));
        let p = db.query("appearances.csv", Some("player_id"), "01", "", 1, 10).unwrap();
        assert_eq!(p.matched, 2);
        assert_eq!(p.rows[0].fields["game_id"].as_deref(), Some("10"));
        drop(db);
        let mut db = Catalog::open(&dir, &cache).unwrap();
        assert!(db.query("appearances.csv", Some("player_id"), "01", "", 0, 10).unwrap().cached);
        assert!(db.query("appearances.csv", Some("player_id"), "01", "alex", 0, 10).is_err());
        drop(db);
        // A damaged disposable cache must rebuild from the source, not lose records or panic.
        for entry in std::fs::read_dir(&cache).unwrap() {
            let entry = entry.unwrap();
            let mut bytes = std::fs::read(entry.path()).unwrap();
            bytes.truncate(40);
            std::fs::write(entry.path(), bytes).unwrap();
        }
        let mut db = Catalog::open(&dir, &cache).unwrap();
        assert_eq!(db.query("appearances.csv", Some("player_id"), "01", "", 0, 10).unwrap().matched, 2);
        assert!(db.query("../players.csv", None, "", "", 0, 10).is_err());
        std::fs::write(dir.join("appearances.csv"), "player_id,game_id,goals\n2,9,3\n").unwrap();
        assert_eq!(db.query("appearances.csv", Some("player_id"), "01", "", 0, 10).unwrap().matched, 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fm_exports_remain_names_and_unresolved_candidate_ids_are_not_join_keys() {
        let dir = std::env::temp_dir().join(format!("pw-database-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let usable = dir.join("2300_fm/usable");
        std::fs::create_dir_all(&usable).unwrap();
        std::fs::write(usable.join("fm23_export_catalog.csv"), "file,status\npeople_names_usable.csv,name_only\n").unwrap();
        std::fs::write(usable.join("people_names_usable.csv"), "name,person_id_candidate,id_status\nAlex,123,not_decoded\n").unwrap();
        std::fs::write(usable.join("players.csv"), "id,name,passing,club\n123,Alex,12,Example Club\n").unwrap();
        let mut db = Catalog::open(&dir, &dir.join("cache")).unwrap();
        assert!(db.tables().iter().find(|t| t.name == "people_names_usable.csv").unwrap().status.contains("unknown"));
        let p = db.query("people_names_usable.csv", None, "", "alex", 0, 50).unwrap();
        assert!(!p.indexed_columns.contains(&"person_id_candidate".into()));
        assert_eq!(p.rows[0].fields["id_status"].as_deref(), Some("not_decoded"));
        assert!(db.tables().iter().find(|t| t.name == "players.csv").unwrap().status.contains("player records"));
        let players = db.query("players.csv", Some("id"), "123", "", 0, 10).unwrap();
        assert_eq!(players.rows[0].fields["passing"].as_deref(), Some("12"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
