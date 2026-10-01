//! Advancing the world clock on a worker thread. The session lock is released
//! between days so the client can keep browsing while time passes, and a stop
//! request lands at the next day boundary (a safe checkpoint).

use std::sync::atomic::Ordering;
use std::sync::{Arc, MutexGuard};
use std::time::{Duration, Instant};

use pw_core::{Date, PlayerId};
use pw_world::EventKind as E;
use pw_world::event::Visibility;
use serde::{Deserialize, Serialize};

use crate::Shared;
use crate::model::{ApiError, ApiResult};
use crate::session::Session;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AdvanceReq {
    Days {
        n: u32,
    },
    UntilDate {
        date: i32,
    },
    /// Until something needs the inhabited person, or `max_days` pass.
    UntilEvent {
        max_days: Option<u32>,
    },
    /// Until the next match of the inhabited person's team has been played.
    UntilMatch,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct StopInfo {
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Job {
    pub running: bool,
    pub seq: u64,
    pub label: String,
    pub from: i32,
    pub target: Option<i32>,
    pub days_done: u32,
    pub days_total: Option<u32>,
    pub stop: Option<StopInfo>,
    pub stop_requested: bool,
    pub matches: u64,
}

fn lock_session(sh: &Shared) -> MutexGuard<'_, Option<Session>> {
    sh.session.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn start(sh: &Arc<Shared>, req: AdvanceReq) -> ApiResult<()> {
    let (from, label, target, total, max_days, needs_persp) = {
        let g = lock_session(sh);
        let s = g.as_ref().ok_or_else(|| ApiError::State("No world is open.".into()))?;
        let today = s.today();
        let inhabiting = s.my_person().is_some();
        match req {
            AdvanceReq::Days { n } => {
                if n == 0 || n > 3660 {
                    return Err(ApiError::Bad("Choose between 1 and 3660 days.".into()));
                }
                let label = if n == 1 { "Advancing one day".to_string() } else { format!("Advancing {n} days") };
                (today.0, label, Some(today.0 + n as i32), Some(n), n, false)
            }
            AdvanceReq::UntilDate { date } => {
                if date <= today.0 {
                    return Err(ApiError::Bad("That date has already passed.".into()));
                }
                let n = (date - today.0) as u32;
                if n > 3660 {
                    return Err(ApiError::Bad("That is more than ten years ahead.".into()));
                }
                (today.0, "Advancing to the chosen date".into(), Some(date), Some(n), n, false)
            }
            AdvanceReq::UntilEvent { max_days } => (today.0, "Advancing until something needs you".into(), None, None, max_days.unwrap_or(400).min(3660), true),
            AdvanceReq::UntilMatch => (today.0, "Advancing to your next match".into(), None, None, 400, true),
        }
        .pipe(|t| if t.5 && !inhabiting { Err(ApiError::State("Only an inhabited person has events to wait for. Advance by days or to a date instead.".into())) } else { Ok(t) })?
    };
    let _ = needs_persp;

    {
        let mut job = sh.job.lock().unwrap_or_else(|e| e.into_inner());
        if job.running {
            return Err(ApiError::Busy("The world is already advancing.".into()));
        }
        let seq = job.seq + 1;
        *job = Job { running: true, seq, label, from, target, days_total: total, ..Default::default() };
    }
    sh.stop.store(false, Ordering::SeqCst);

    let sh2 = Arc::clone(sh);
    std::thread::Builder::new().name("pw-advance".into()).spawn(move || run(sh2, req, max_days)).map_err(|e| ApiError::State(e.to_string()))?;
    Ok(())
}

trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

pub fn request_stop(sh: &Shared) {
    sh.stop.store(true, Ordering::SeqCst);
    sh.job.lock().unwrap_or_else(|e| e.into_inner()).stop_requested = true;
}

fn finish(sh: &Shared, stop: StopInfo) {
    let mut job = sh.job.lock().unwrap_or_else(|e| e.into_inner());
    job.running = false;
    job.stop = Some(stop);
}

fn run(sh: Arc<Shared>, req: AdvanceReq, max_days: u32) {
    let mut done = 0u32;
    let mut matches = 0u64;
    // Catch a panic from the simulation so the client sees an error, not a hang.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        loop {
            if sh.stop.load(Ordering::SeqCst) {
                break StopInfo { kind: "user".into(), text: "Stopped at the end of the day.".into() };
            }
            let mut g = lock_session(&sh);
            let Some(s) = g.as_mut() else {
                break StopInfo { kind: "error".into(), text: "The world was closed.".into() };
            };
            let before = s.today();
            let t0 = Instant::now();
            let stats = s.game.step();
            matches += stats.matches as u64;
            s.timings.push((before.0, t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32));
            if s.timings.len() > 400 {
                s.timings.drain(..200);
            }
            s.revision += 1;
            done += 1;
            let now = s.today();
            let stop = after_step(s, before, &req, done, max_days);
            drop(g);

            {
                let mut job = sh.job.lock().unwrap_or_else(|e| e.into_inner());
                job.days_done = done;
                job.matches = matches;
            }
            if let Some(stop) = stop {
                break stop;
            }
            let _ = now;
            std::thread::sleep(Duration::from_micros(150));
        }
    }));
    match result {
        Ok(stop) => finish(&sh, stop),
        Err(_) => finish(&sh, StopInfo { kind: "error".into(), text: "The simulation stopped unexpectedly. The world was left at the last completed day.".into() }),
    }
}

/// Decide whether to stop after a day was simulated, and keep presentation state in step.
fn after_step(s: &mut Session, played: Date, req: &AdvanceReq, done: u32, max_days: u32) -> Option<StopInfo> {
    let now = s.today();
    let mut stop: Option<StopInfo> = None;
    let me = s.my_person();
    let my_player = s.my_player();
    let my_team = my_player.map(|p| s.w().players.hot[p].team).filter(|t| t.is_some());

    let mut played_mine: Option<String> = None;
    if let Some(team) = my_team {
        let mine: Vec<u64> = s.w().fixtures.on(played).iter().map(|&f| s.w().fixtures.get(f)).filter(|f| f.involves(team) && f.score.is_some()).map(|f| f.uid).collect();
        for uid in mine {
            if s.meta.conceal_mine {
                s.meta.concealed.insert(uid);
            }
            played_mine = Some(format!("Your team played on {}", crate::fmt::date(played)));
        }
    }

    if let (Some(me), true) = (me, s.meta.stops.decisions) {
        let mut hit: Option<(u32, String, bool)> = None;
        for (id, d) in s.w().decisions.pending_for(me) {
            if d.answer.is_some() {
                continue;
            }
            let expiring = d.deadline.0 <= now.0 + 1;
            if !s.meta.warned.contains(&id.0) {
                hit = Some((id.0, format!("{} needs your response", d.kind.title()), false));
                break;
            }
            if expiring && !s.meta.warned.contains(&(id.0 | 0x8000_0000)) {
                hit = Some((id.0 | 0x8000_0000, format!("{} expires soon; it will be settled by default", d.kind.title()), true));
                break;
            }
        }
        if let Some((key, text, _)) = hit {
            s.meta.warned.insert(key);
            if key & 0x8000_0000 == 0 {
                // First sighting also marks nothing about expiry.
            }
            stop = Some(StopInfo { kind: "decision".into(), text });
        }
    }

    if stop.is_none()
        && s.meta.stops.major
        && let Some(text) = major_event(s, played, my_player)
    {
        stop = Some(StopInfo { kind: "major".into(), text });
    }

    if stop.is_none()
        && s.meta.stops.matches
        && let (Some(msg), Some(_)) = (&played_mine, my_team)
    {
        stop = Some(StopInfo { kind: "match".into(), text: format!("{msg}. The result is waiting for you.") });
    }

    // Explicit targets.
    if stop.is_none() {
        match req {
            AdvanceReq::Days { n } if done >= *n => {
                stop = Some(StopInfo { kind: "target".into(), text: "Done.".into() });
            }
            AdvanceReq::UntilDate { date } if now.0 >= *date => {
                stop = Some(StopInfo { kind: "target".into(), text: "Reached the chosen date.".into() });
            }
            AdvanceReq::UntilMatch if played_mine.is_some() => {
                stop = Some(StopInfo { kind: "match".into(), text: "Your match has been played.".into() });
            }
            AdvanceReq::UntilEvent { .. } | AdvanceReq::UntilMatch if done >= max_days => {
                stop = Some(StopInfo { kind: "target".into(), text: "Nothing needed you in that time.".into() });
            }
            _ => {}
        }
    }
    // A target that the stop policy pre-empted is still fine; an explicit day count wins over policy stops only when reached.
    stop
}

/// An event since the previous day that materially changes the inhabited person's circumstances.
fn major_event(s: &Session, since: Date, player: Option<PlayerId>) -> Option<String> {
    let p = player?;
    let w = s.w();
    let club = w.players.hot[p].club;
    for e in w.events.since(since) {
        let mine_vis = match e.vis {
            Visibility::Public => true,
            Visibility::Club(c) => c == club,
            Visibility::Person(pp) => Some(pp) == s.my_person(),
            Visibility::Between(a, b) => Some(a) == s.my_person() || Some(b) == s.my_person(),
        };
        if !mine_vis {
            continue;
        }
        match &e.kind {
            E::Injured { player, .. } if *player == p => return Some("You have picked up an injury.".into()),
            E::Suspended { player, .. } if *player == p => return Some("You have been suspended.".into()),
            E::Transfer { player, .. } | E::LoanMove { player, .. } | E::Released { player, .. } if *player == p => {
                return Some("Your situation at the club has changed.".into());
            }
            E::ManagerSacked { club: c, .. } if *c == club => return Some("Your manager has left the club.".into()),
            E::ManagerAppointed { club: c, .. } if *c == club => return Some("A new manager has been appointed.".into()),
            E::Retired { person } if Some(*person) == s.my_person() => return Some("You have retired.".into()),
            _ => {}
        }
    }
    None
}
