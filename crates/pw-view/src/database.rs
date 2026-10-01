//! Source browsing is read-only and never touches a simulated day's working data or the save layout.

use std::path::{Path, PathBuf};
use pw_import::database::Catalog;
use serde_json::{Value, json};
use crate::{Api, ApiError, ApiResult};

pub(super) struct Databases {
    catalogs: Vec<Catalog>,
    dir: PathBuf,
    errors: Vec<String>,
}

impl Databases {
    pub fn new(dir: &Path) -> Self {
        let mut out = Self { catalogs: Vec::new(), dir: dir.into(), errors: Vec::new() };
        if let Ok(s) = std::fs::read_to_string(dir.join("database-sources.json")) {
            match serde_json::from_str::<Vec<PathBuf>>(&s) {
                Ok(paths) => for path in paths { if let Err(e) = out.attach(&path) { out.errors.push(format!("{}: {e}", path.display())); } },
                Err(e) => out.errors.push(format!("Cannot read database-sources.json: {e}")),
            }
        }
        out
    }

    fn attach(&mut self, dir: &Path) -> Result<usize, pw_import::ImportError> {
        let catalog = Catalog::open(dir, &self.dir.join("database-indexes"))?;
        if let Some(i) = self.catalogs.iter().position(|c| c.root() == catalog.root()) {
            self.catalogs[i] = catalog; // Reattach explicitly refreshes the table inventory.
            return Ok(i);
        }
        self.catalogs.push(catalog);
        Ok(self.catalogs.len() - 1)
    }

    fn value(&self) -> Value {
        json!({"sources": self.catalogs.iter().enumerate().map(|(i, c)| json!({"id": i, "path": c.root().display().to_string().trim_start_matches("\\\\?\\"), "tables": c.tables()})).collect::<Vec<_>>(), "errors": self.errors})
    }

    pub(super) fn attach_import(&mut self, root: &Path) {
        let candidates = [root.to_path_buf(), root.parent().unwrap_or(root).join("fm23_extracted")];
        for path in candidates {
            if path.is_dir() && let Err(e) = self.attach(&path) { self.errors.push(format!("{}: {e}", path.display())); }
        }
        let paths: Vec<_> = self.catalogs.iter().map(|c| c.root().to_path_buf()).collect();
        if let Ok(encoded) = serde_json::to_vec(&paths) && let Err(e) = std::fs::write(self.dir.join("database-sources.json"), encoded) {
            self.errors.push(format!("Could not remember source folders: {e}"));
        }
    }
}

impl Api {
    pub(super) fn datasets(&self) -> Value {
        // An explicit install path takes precedence. The checkout fallback makes
        // the local pack usable from both the server and desktop development app.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let candidates = [std::env::var_os("RISEABOVE_DATABASE").map(PathBuf::from), Some(root.join("database/current"))];
        let mut found = Vec::new();
        for dir in candidates.into_iter().flatten() {
            if let Ok(Some(meta)) = pw_import::consolidated::manifest(&dir) {
                let dir = dir.canonicalize().unwrap_or(dir);
                let path = dir.display().to_string().trim_start_matches("\\\\?\\").to_string();
                if !found.iter().any(|v: &Value| v["path"] == path) { found.push(json!({"path": path, "database": meta})); }
            }
        }
        json!({"datasets": found})
    }

    pub(super) fn database_sources(&self) -> ApiResult<Value> {
        let mut db = self.sh.database.lock().unwrap_or_else(|e| e.into_inner());
        if db.catalogs.is_empty() {
            if let Some(path) = self.datasets()["datasets"][0]["path"].as_str() { db.attach_import(Path::new(path)); }
        }
        Ok(db.value())
    }

    pub(super) fn database_attach(&self, args: Value) -> ApiResult<Value> {
        let dir = crate::contract::request::<crate::contract::DirReq>(args)?.dir;
        let dir = dir.as_str();
        let mut db = self.sh.database.lock().unwrap_or_else(|e| e.into_inner());
        let id = db.attach(Path::new(dir)).map_err(|e| ApiError::Bad(e.to_string()))?;
        let paths: Vec<_> = db.catalogs.iter().map(|c| c.root().to_path_buf()).collect();
        let encoded = serde_json::to_vec(&paths).map_err(|e| ApiError::State(e.to_string()))?;
        std::fs::write(db.dir.join("database-sources.json"), encoded).map_err(|e| ApiError::State(format!("Folder connected, but its path could not be remembered: {e}")))?;
        Ok(json!({"id": id, "sources": db.value()["sources"]}))
    }

    pub(super) fn database_query(&self, args: Value) -> ApiResult<Value> {
        {
            let session = self.lock();
            if session.as_ref().is_some_and(|s| !crate::ctx::Ctx::new(s).observer()) {
                return Err(ApiError::Unauthorized("Source records are available in the observer view. They are separate from what your character knows.".into()));
            }
        }
        let req: crate::contract::DatabaseQueryReq = crate::contract::request(args)?;
        let source = req.source.unwrap_or(0) as usize;
        let table = req.table.as_str();
        let column = req.column.as_deref().filter(|s| !s.is_empty());
        let value = req.value.as_deref().unwrap_or("");
        let search = req.search.as_deref().unwrap_or("");
        if search.len() > 500 || value.len() > 500 { return Err(ApiError::Bad("The query is too long.".into())); }
        let offset = req.offset.unwrap_or(0) as usize;
        let limit = req.limit.unwrap_or(50).min(100) as usize;
        let mut db = self.sh.database.lock().unwrap_or_else(|e| e.into_inner());
        let cat = db.catalogs.get_mut(source).ok_or_else(|| ApiError::Bad("Connect this database folder first.".into()))?;
        let tm = cat.tables().iter().any(|t| t.name == table && t.source == "Transfermarkt");
        let result = cat.query(table, column, value, search, offset, limit).map_err(|e| ApiError::Bad(e.to_string()))?;
        // Release the database lock before taking the world lock. An advancing world never waits for CSV indexing.
        drop(db);
        let mut page = serde_json::to_value(result).map_err(|e| ApiError::State(e.to_string()))?;
        let session = self.lock();
        if session.as_ref().is_some_and(|s| !crate::ctx::Ctx::new(s).observer()) {
            return Err(ApiError::Unauthorized("Switch to observer view to browse source records.".into()));
        }
        if tm {
            if let Some(s) = session.as_ref() {
                let origins = &s.w().origins;
                let tm_source = |source: u8| origins.sources.get(source as usize).is_some_and(|s| s.name == "transfermarkt" || (tm && s.name == "fpl"));
                let source_kind = |column: &str| {
                    if matches!(column, "player_id" | "player_in_id" | "player_assist_id") { Some("player") }
                    else if column.ends_with("club_id") { Some("club") }
                    else if column == "competition_id" { Some("competition") } else { None }
                };
                // Resolve only this page's IDs. Building JSON for the whole population on every page is unnecessary.
                let wanted: std::collections::HashSet<String> = page["rows"].as_array().into_iter().flatten()
                    .filter_map(|row| row["fields"].as_object()).flat_map(|fields| fields.iter())
                    .filter_map(|(column, value)| Some(format!("{}:{}", source_kind(column)?, value.as_str()?))).collect();
                let mut refs = std::collections::HashMap::new();
                for (id, p) in &origins.people { if tm_source(p.src.source) && wanted.contains(&p.src.id) { refs.insert(p.src.id.as_str(), json!({"k": "person", "id": id.0})); } }
                for (id, r) in &origins.clubs { if tm_source(r.source) && wanted.contains(&r.id) { refs.insert(r.id.as_str(), json!({"k": "club", "id": id.0})); } }
                for (id, r) in &origins.comps { if tm_source(r.source) && wanted.contains(&r.id) { refs.insert(r.id.as_str(), json!({"k": "comp", "id": id.0})); } }
                if let Some(rows) = page["rows"].as_array_mut() {
                    for row in rows {
                        let mut links = serde_json::Map::new();
                        if let Some(fields) = row["fields"].as_object() {
                            for (col, val) in fields {
                                let Some(kind) = source_kind(col) else { continue };
                                if let Some(v) = val.as_str().and_then(|v| refs.get(format!("{kind}:{v}").as_str())) { links.insert(col.clone(), v.clone()); }
                            }
                        }
                        row["refs"] = Value::Object(links);
                    }
                }
            }
        }
        if let Some(object) = page.as_object_mut() {
            object.remove("cached");
            object.remove("index_bytes");
        }
        page["source"] = json!(source);
        Ok(page)
    }
}
