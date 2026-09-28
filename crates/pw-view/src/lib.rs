//! The client-facing API of the simulation.
//!
//! Every page the desktop app shows is answered here, from one perspective
//! (an omniscient observer, or one inhabited person), before any data reaches
//! the interface. The transport is a single call, `call(method, args)`, used
//! identically by the desktop shell and the development server.

mod advance;
mod ctx;
mod fmt;
mod model;
mod narrative;
mod pages;
mod session;
mod table;
mod tables;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use model::{ApiError, ApiResult};
use advance::{AdvanceReq, Job};
use ctx::Ctx;
use session::Session;

pub struct Shared {
    session: Mutex<Option<Session>>,
    job: Mutex<Job>,
    task: Mutex<Task>,
    stop: AtomicBool,
    dir: PathBuf,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Task {
    pub running: bool,
    pub seq: u64,
    pub label: String,
    pub error: Option<String>,
    pub report: Option<Value>,
}

#[derive(Clone)]
pub struct Api {
    sh: Arc<Shared>,
}

#[derive(Deserialize)]
struct NewWorld {
    kind: String,
    #[serde(default)]
    scale: Option<String>,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

fn slug(s: &str) -> String {
    let mut out: String = s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "world".into() } else { out }
}

impl Api {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let dir = data_dir.into();
        let _ = std::fs::create_dir_all(dir.join("saves"));
        Self {
            sh: Arc::new(Shared {
                session: Mutex::new(None),
                job: Mutex::new(Job::default()),
                task: Mutex::new(Task::default()),
                stop: AtomicBool::new(false),
                dir,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<Session>> {
        self.sh.session.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn with<R>(&self, f: impl FnOnce(&Ctx) -> ApiResult<R>) -> ApiResult<R> {
        let g = self.lock();
        let s = g.as_ref().ok_or_else(|| ApiError::State("No world is open.".into()))?;
        f(&Ctx::new(s))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Session) -> ApiResult<R>) -> ApiResult<R> {
        let mut g = self.lock();
        let s = g.as_mut().ok_or_else(|| ApiError::State("No world is open.".into()))?;
        f(s)
    }

    fn job_running(&self) -> bool {
        self.sh.job.lock().unwrap_or_else(|e| e.into_inner()).running
    }

    fn not_while_advancing(&self) -> ApiResult<()> {
        if self.job_running() {
            Err(ApiError::State("The world is advancing. Stop it first.".into()))
        } else {
            Ok(())
        }
    }

    /// Dispatch one call. Errors carry a code and a message fit to show to the person.
    pub fn call(&self, method: &str, args: Value) -> ApiResult<Value> {
        match method {
            "app.info" => Ok(json!({"name": "Rise Above", "version": env!("CARGO_PKG_VERSION"), "data_dir": self.sh.dir.display().to_string()})),
            "world.status" => Ok(self.status()),
            "world.new" => self.world_new(args),
            "world.inspect_import" => self.inspect_import(args),
            "world.saves" => Ok(self.saves()),
            "world.save" => self.save(args),
            "world.load" => self.load(args),
            "world.close" => {
                self.not_while_advancing()?;
                *self.lock() = None;
                Ok(json!({"closed": true}))
            }
            "world.delete_save" => self.delete_save(args),
            "settings.set" => self.with_mut(|s| {
                if let Some(v) = args.get("conceal_mine").and_then(Value::as_bool) {
                    s.meta.conceal_mine = v;
                }
                if let Some(st) = args.get("stops") {
                    if let Some(v) = st.get("decisions").and_then(Value::as_bool) {
                        s.meta.stops.decisions = v;
                    }
                    if let Some(v) = st.get("matches").and_then(Value::as_bool) {
                        s.meta.stops.matches = v;
                    }
                    if let Some(v) = st.get("major").and_then(Value::as_bool) {
                        s.meta.stops.major = v;
                    }
                }
                Ok(json!({"ok": true}))
            }),

            "advance.start" => {
                let req: AdvanceReq = serde_json::from_value(args).map_err(|e| ApiError::Bad(e.to_string()))?;
                advance::start(&self.sh, req)?;
                Ok(self.status())
            }
            "advance.stop" => {
                advance::request_stop(&self.sh);
                Ok(json!({"requested": true}))
            }

            "persp.observe" => {
                self.not_while_advancing()?;
                self.with_mut(|s| {
                    s.observe();
                    Ok(json!({"ok": true}))
                })
            }
            "persp.inhabit" => {
                self.not_while_advancing()?;
                let id = args.get("person").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing person".into()))?;
                self.with_mut(|s| {
                    let salt = s.w().seed ^ (u64::from(s.today().0 as u32) << 20) ^ id;
                    s.inhabit(pw_core::PersonId(id as u32), salt)?;
                    Ok(json!({"ok": true}))
                })
            }
            "person.create" => {
                self.not_while_advancing()?;
                self.with_mut(|s| pages::person::create(s, &args))
            }

            "table.query" => {
                let req: model::TableReq = serde_json::from_value(args).map_err(|e| ApiError::Bad(e.to_string()))?;
                self.with(|c| tables::query(c, &req))
            }
            "search" => self.with(|c| pages::world::search(c, &args)),
            "overview" => self.with(pages::world::overview),
            "diagnostics" => self.with(pages::world::diagnostics),
            "capabilities" => Ok(pages::world::capabilities()),

            "person" => self.with(|c| pages::person::get(c, &args)),
            "person.attributes" => self.with(|c| pages::person::attributes(c, &args)),
            "club" => self.with(|c| pages::club::get(c, &args)),
            "club.systems" => self.with(|c| pages::club::systems(c, &args)),
            "club.follow" => self.with_mut(|s| pages::club::follow(s, &args)),
            "comp" => self.with(|c| pages::club::comp(c, &args)),
            "nation" => self.with(|c| pages::club::nation(c, &args)),
            "match" => self.with(|c| pages::matchp::get(c, &args, false)),
            "match.watch" => self.with(|c| pages::matchp::get(c, &args, true)),
            "match.reveal" => self.with_mut(|s| pages::matchp::reveal(s, &args)),
            "match.reveal_all" => self.with_mut(pages::matchp::reveal_all),

            "me.today" => self.with(pages::me::today),
            "me.viewed" => self.with_mut(pages::me::mark_viewed),
            "me.messages" => self.with(|c| pages::inbox::inbox(c, &args)),
            "me.message" => self.with(|c| pages::inbox::message(c, &args)),
            "me.answer" => self.with_mut(|s| pages::inbox::answer(s, &args)),
            "me.act" => self.with_mut(|s| pages::act::act(s, &args)),
            "me.options" => self.with(pages::act::options),
            "me.self" => self.with(pages::life::self_view),
            "me.life" => self.with(pages::life::life),
            "person.life" => self.with(|c| pages::life::life_of(c, &args)),
            "me.people" => self.with(pages::life::people),
            "me.promises" => self.with(pages::life::promises),
            "me.rumours" => self.with(pages::life::rumours),
            "me.press" => self.with(pages::life::press),
            "me.story" => self.with(|c| pages::life::story(c, &args)),
            "me.agent" => self.with(pages::life::agent),
            "me.journal" => self.with(pages::life::journal),
            "me.goal" => self.with_mut(|s| pages::life::add_goal(s, &args)),
            "me.goal_done" => self.with_mut(|s| pages::life::goal_done(s, &args)),
            "me.note" => self.with_mut(|s| pages::life::add_note(s, &args)),
            "me.note_remove" => self.with_mut(|s| pages::life::remove_note(s, &args)),
            "me.calendar" => self.with(|c| pages::me::calendar(c, &args)),
            "me.football" => self.with(pages::me::football),
            "me.plan" => self.with_mut(|s| pages::me::set_plan(s, &args)),
            "me.contract" => self.with(pages::me::contract),
            other => Err(ApiError::NotFound(format!("method {other}"))),
        }
    }

    // ---- world lifecycle -----------------------------------------------------------------

    pub fn status(&self) -> Value {
        let job = self.sh.job.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let task = self.sh.task.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let g = self.lock();
        match g.as_ref() {
            None => json!({"open": false, "task": task, "job": job}),
            Some(s) => {
                let c = Ctx::new(s);
                let me = s.my_person();
                let awaiting = me.map_or(0, |m| s.w().decisions.pending_for(m).filter(|(_, d)| d.answer.is_none()).count());
                json!({
                    "open": true, "name": s.meta.name, "date": s.today().0, "revision": s.revision,
                    "perspective": match me { None => json!({"mode": "observer"}), Some(person) => json!({
                        "mode": "inhabit", "person": person.0, "name": c.person_name(person),
                        "club": c.my_club().get().map(|cl| c.club_name(cl)),
                    }) },
                    "job": job, "task": task,
                    "settings": {"conceal_mine": s.meta.conceal_mine, "stops": {"decisions": s.meta.stops.decisions, "matches": s.meta.stops.matches, "major": s.meta.stops.major}},
                    "awaiting": awaiting, "unrevealed": s.meta.concealed.len(),
                })
            }
        }
    }

    fn start_task(&self, label: &str) -> ApiResult<u64> {
        let mut t = self.sh.task.lock().unwrap_or_else(|e| e.into_inner());
        if t.running {
            return Err(ApiError::State("Another operation is still running.".into()));
        }
        let seq = t.seq + 1;
        *t = Task { running: true, seq, label: label.into(), error: None, report: None };
        Ok(seq)
    }

    fn finish_task(sh: &Shared, error: Option<String>, report: Option<Value>) {
        let mut t = sh.task.lock().unwrap_or_else(|e| e.into_inner());
        t.running = false;
        t.error = error;
        t.report = report;
    }

    fn world_new(&self, args: Value) -> ApiResult<Value> {
        self.not_while_advancing()?;
        let req: NewWorld = serde_json::from_value(args).map_err(|e| ApiError::Bad(e.to_string()))?;
        let label = match req.kind.as_str() {
            "synthetic" => "Building a test world",
            "import" => "Importing the dataset",
            _ => return Err(ApiError::Bad("Unknown world source.".into())),
        };
        self.start_task(label)?;
        let sh = Arc::clone(&self.sh);
        std::thread::spawn(move || {
            let built = std::panic::catch_unwind(|| build_world(&req));
            match built {
                Ok(Ok((world, name, report))) => {
                    *sh.session.lock().unwrap_or_else(|e| e.into_inner()) = Some(Session::new(world, name));
                    *sh.job.lock().unwrap_or_else(|e| e.into_inner()) = Job::default();
                    Api::finish_task(&sh, None, Some(report));
                }
                Ok(Err(e)) => Api::finish_task(&sh, Some(e), None),
                Err(_) => Api::finish_task(&sh, Some("The world could not be built.".into()), None),
            }
        });
        Ok(json!({"started": true}))
    }

    fn inspect_import(&self, args: Value) -> ApiResult<Value> {
        let dir = args.get("dir").and_then(Value::as_str).ok_or_else(|| ApiError::Bad("missing folder".into()))?;
        let files: Vec<Value> = match std::fs::read_dir(dir) {
            Ok(rd) => {
                let mut v: Vec<Value> = rd
                    .flatten()
                    .filter(|e| e.path().is_file())
                    .map(|e| json!({"name": e.file_name().to_string_lossy(), "size": e.metadata().map(|m| m.len()).unwrap_or(0)}))
                    .collect();
                v.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
                v
            }
            Err(e) => return Err(ApiError::State(format!("Cannot read that folder: {e}"))),
        };
        match pw_import::load_dir(Path::new(dir), DataPack::builtin()) {
            Ok((w, rep)) => Ok(json!({
                "ok": true, "files": files,
                "counts": {"nations": rep.nations, "competitions": rep.competitions, "clubs": rep.clubs, "players": rep.players, "staff": rep.staff},
                "warnings": rep.warnings, "start": w.date.0,
            })),
            Err(e) => Ok(json!({"ok": false, "files": files, "error": e.to_string()})),
        }
    }

    fn saves_dir(&self) -> PathBuf {
        self.sh.dir.join("saves")
    }

    pub fn saves(&self) -> Value {
        let mut out: Vec<Value> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(self.saves_dir()) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("pws") {
                    continue;
                }
                let meta_path = p.with_extension("json");
                let side: Value = std::fs::read_to_string(&meta_path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
                let md = e.metadata().ok();
                out.push(json!({
                    "file": p.file_name().map(|s| s.to_string_lossy().to_string()),
                    "size": md.as_ref().map(|m| m.len()).unwrap_or(0),
                    "modified": md.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()),
                    "info": side,
                    "has_backup": p.with_extension("bak").exists(),
                }));
            }
        }
        out.sort_by(|a, b| b["modified"].as_u64().cmp(&a["modified"].as_u64()));
        json!({"saves": out, "dir": self.saves_dir().display().to_string()})
    }

    fn save(&self, args: Value) -> ApiResult<Value> {
        self.not_while_advancing()?;
        let g = self.lock();
        let s = g.as_ref().ok_or_else(|| ApiError::State("No world is open.".into()))?;
        let file = args.get("file").and_then(Value::as_str).map(slug).unwrap_or_else(|| slug(&s.meta.name));
        let path = self.saves_dir().join(format!("{file}.pws"));
        if path.exists() {
            let _ = std::fs::copy(&path, path.with_extension("bak"));
        }
        s.save(&path)?;
        let c = Ctx::new(s);
        let info = json!({
            "name": s.meta.name, "date": s.today().0, "players": s.w().players.len(), "clubs": s.w().clubs.len(),
            "perspective": s.my_person().map_or("Observer".to_string(), |p| c.person_name(p)),
            "version": env!("CARGO_PKG_VERSION"),
        });
        let _ = std::fs::write(path.with_extension("json"), info.to_string());
        Ok(json!({"file": format!("{file}.pws")}))
    }

    fn load(&self, args: Value) -> ApiResult<Value> {
        self.not_while_advancing()?;
        let file = args.get("file").and_then(Value::as_str).ok_or_else(|| ApiError::Bad("missing file".into()))?;
        let backup = args.get("backup").and_then(Value::as_bool).unwrap_or(false);
        let mut path = self.saves_dir().join(Path::new(file).file_name().ok_or_else(|| ApiError::Bad("bad file name".into()))?);
        if backup {
            path = path.with_extension("bak");
        }
        self.start_task("Loading the world")?;
        let sh = Arc::clone(&self.sh);
        std::thread::spawn(move || match Session::load(&path) {
            Ok(s) => {
                *sh.session.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
                *sh.job.lock().unwrap_or_else(|e| e.into_inner()) = Job::default();
                Api::finish_task(&sh, None, None);
            }
            Err(e) => Api::finish_task(&sh, Some(format!("This save could not be loaded: {e}. The file has not been changed.")), None),
        });
        Ok(json!({"started": true}))
    }

    fn delete_save(&self, args: Value) -> ApiResult<Value> {
        let file = args.get("file").and_then(Value::as_str).ok_or_else(|| ApiError::Bad("missing file".into()))?;
        let name = Path::new(file).file_name().ok_or_else(|| ApiError::Bad("bad file name".into()))?;
        let p = self.saves_dir().join(name);
        for ext in ["pws", "json", "bak"] {
            let _ = std::fs::remove_file(p.with_extension(ext));
        }
        Ok(json!({"deleted": true}))
    }
}

fn build_world(req: &NewWorld) -> Result<(pw_world::World, String, Value), String> {
    match req.kind.as_str() {
        "synthetic" => {
            let scale = match req.scale.as_deref() {
                Some("tiny") => Scale::TINY,
                Some("huge") => Scale::HUGE,
                _ => Scale::SMALL,
            };
            let seed = req.seed.unwrap_or(42);
            let w = synthetic::build(DataPack::builtin(), seed, scale);
            let name = req.name.clone().unwrap_or_else(|| format!("Test world {}", req.scale.as_deref().unwrap_or("small")));
            let report = json!({"players": w.players.len(), "clubs": w.clubs.len()});
            Ok((w, name, report))
        }
        _ => {
            let dir = req.dir.as_deref().ok_or("Choose a folder to import.")?;
            let (w, rep) = pw_import::load_dir(Path::new(dir), DataPack::builtin()).map_err(|e| e.to_string())?;
            let name = req.name.clone().unwrap_or_else(|| Path::new(dir).file_name().map_or("Imported world".into(), |s| s.to_string_lossy().to_string()));
            let report = json!({"nations": rep.nations, "competitions": rep.competitions, "clubs": rep.clubs, "players": rep.players, "staff": rep.staff, "warnings": rep.warnings});
            Ok((w, name, report))
        }
    }
}
