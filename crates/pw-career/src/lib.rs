//! The human side of the port (S3–S5).
//!
//! This crate holds no world facts. A running game is the world simulation
//! plus a small `Session`: which person the human currently inhabits, the
//! answers they gave (for replays), their private notes and pinned goals,
//! and what they have already read. Family, money, contracts, relationships,
//! history — all of it lives in the world, for everyone.
//!
//! Inhabiting someone is `take_control(person)`: their decisions start coming
//! from the human instead of their AI mind. Nothing else changes. Stepping out
//! hands them back to their own mind, which carries on from their personality
//! and memories. There is no ending; retirement is a change of job.

pub mod create;
pub mod feed;
pub mod views;

use pw_core::{Date, DecisionId, EventId, PersonId, PlayerId};
use pw_sim::Sim;
use pw_world::{Intent, World};
use serde::{Deserialize, Serialize};

pub use create::{NewPerson, create_person};

/// A goal the human pins for themselves. It never changes the world's odds;
/// it is tracked against real statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Goal {
    pub text: String,
    pub pinned: Date,
    pub kind: GoalKind,
    pub done: Option<Date>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum GoalKind {
    /// Reach this many senior appearances.
    Appearances(u16),
    /// Score this many senior goals.
    Goals(u16),
    /// Play for a club in a top-tier league.
    TopFlight,
    /// Free text: the human decides when it's done.
    Personal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoggedAnswer {
    pub date: Date,
    pub decision: DecisionId,
    pub choice: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Session {
    /// The person currently inhabited, if any.
    pub controlled: Option<PersonId>,
    /// Everyone inhabited so far in this playthrough, with the date they were taken.
    pub history: Vec<(PersonId, Date, Option<Date>)>,
    /// Answers given (decision log, for replay and debugging — P6).
    pub log: Vec<LoggedAnswer>,
    /// Intents submitted, in order.
    pub acts: Vec<(Date, PersonId, Intent)>,
    pub notes: Vec<(Date, String)>,
    pub goals: Vec<Goal>,
    /// Last event already shown in the feed.
    pub seen: EventId,
}

/// A running game: the world plus the human's session in it.
pub struct Game {
    pub sim: Sim,
    pub session: Session,
}

#[derive(Serialize, Deserialize)]
struct SaveFile {
    world: World,
    session: Session,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Advance {
    Day,
    Week,
    Days(u32),
    /// Until something needs the person: a decision, an important item in
    /// their feed, or a match for their team tomorrow.
    UntilEvent,
    Until(Date),
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct AdvanceReport {
    pub days: u32,
    pub stopped_for: Option<String>,
}

impl Game {
    pub fn new(world: World) -> Self {
        Self { sim: Sim::new(world), session: Session { seen: EventId::NONE, ..Session::default() } }
    }

    pub fn world(&self) -> &World {
        &self.sim.world
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), pw_sim::save::SaveError> {
        #[derive(Serialize)]
        struct View<'a> {
            world: &'a World,
            session: &'a Session,
        }
        pw_sim::save::save(&View { world: &self.sim.world, session: &self.session }, path)
    }

    pub fn load(path: &std::path::Path) -> Result<Self, pw_sim::save::SaveError> {
        let f: SaveFile = pw_sim::save::load(path)?;
        Ok(Self { sim: Sim::new(f.world), session: f.session })
    }

    /// Inhabit `person`. The first time in a playthrough, the world's future
    /// randomness is re-keyed with `salt`, so a new playthrough of the same
    /// world diverges while this one stays reproducible (S22).
    pub fn take_control(&mut self, person: PersonId, salt: u64) -> bool {
        if self.sim.world.people.get(person).is_none() {
            return false;
        }
        if let Some(prev) = self.session.controlled {
            self.release_inner(prev);
        }
        if self.session.history.is_empty() {
            self.sim.world.begin_playthrough(salt);
        }
        let ok = self.sim.world.take_control(person);
        if ok {
            let today = self.sim.world.date;
            self.session.controlled = Some(person);
            self.session.history.push((person, today, None));
            self.session.seen = self.sim.world.events.last_id();
        }
        ok
    }

    /// Step out; the person's own mind takes over again.
    pub fn release(&mut self) {
        if let Some(p) = self.session.controlled.take() {
            self.release_inner(p);
        }
    }

    fn release_inner(&mut self, p: PersonId) {
        self.sim.world.release_control(p);
        let today = self.sim.world.date;
        if let Some(h) = self.session.history.iter_mut().rev().find(|h| h.0 == p && h.2.is_none()) {
            h.2 = Some(today);
        }
    }

    pub fn me(&self) -> Option<PersonId> {
        self.session.controlled
    }

    pub fn my_player(&self) -> Option<PlayerId> {
        let p = self.session.controlled?;
        let pl = self.sim.world.people[p].player;
        pl.is_some().then_some(pl)
    }

    /// Do something on your own initiative. It is applied by the world's
    /// intent system on the next simulated day, exactly as an AI's would be.
    pub fn act(&mut self, intent: Intent) -> bool {
        let Some(me) = self.session.controlled else { return false };
        let today = self.sim.world.date;
        self.sim.world.intents.submit(me, intent, today);
        self.session.acts.push((today, me, intent));
        true
    }

    /// Reply to an inbox message with one of its options (by index). The
    /// reply becomes an intent or an answer, like any other choice.
    pub fn reply(&mut self, msg: u32, option: usize) -> bool {
        let Some(me) = self.session.controlled else { return false };
        let opts = pw_sim::inbox::options(&self.sim.world, msg);
        let Some(&r) = opts.get(option) else { return false };
        pw_sim::inbox::reply(&mut self.sim.world, me, msg, r)
    }

    /// Mark a thread read.
    pub fn read_thread(&mut self, thread: u32) {
        if let Some(me) = self.session.controlled {
            pw_sim::inbox::read(&mut self.sim.world, me, thread);
        }
    }

    pub fn pending(&self) -> Vec<DecisionId> {
        let Some(me) = self.session.controlled else { return Vec::new() };
        self.sim.world.decisions.pending_for(me).filter(|(_, d)| d.answer.is_none()).map(|(id, _)| id).collect()
    }

    pub fn answer(&mut self, id: DecisionId, choice: u8) -> bool {
        let ok = self.sim.world.decisions.answer(id, choice);
        if ok {
            let date = self.sim.world.date;
            self.session.log.push(LoggedAnswer { date, decision: id, choice });
        }
        ok
    }

    pub fn step(&mut self) -> pw_sim::DayStats {
        let stats = self.sim.step();
        self.check_goals();
        stats
    }

    pub fn advance(&mut self, mode: Advance) -> AdvanceReport {
        let mut rep = AdvanceReport::default();
        let today = self.sim.world.date;
        let limit = match mode {
            Advance::Day => 1,
            Advance::Week => 7,
            Advance::Days(n) => n,
            Advance::UntilEvent => 60,
            Advance::Until(d) => today.days_until(d).clamp(0, 4000) as u32,
        };
        let before = self.sim.world.events.last_id();
        for _ in 0..limit {
            self.step();
            rep.days += 1;
            if mode != Advance::UntilEvent {
                continue;
            }
            if !self.pending().is_empty() {
                rep.stopped_for = Some("A decision needs your answer.".into());
                break;
            }
            if let Some(me) = self.session.controlled
                && feed::important_since(&self.sim.world, me, before)
            {
                rep.stopped_for = Some("Something important happened.".into());
                break;
            }
            if self.match_tomorrow() {
                rep.stopped_for = Some("Match tomorrow.".into());
                break;
            }
        }
        rep
    }

    fn match_tomorrow(&self) -> bool {
        let Some(p) = self.my_player() else { return false };
        let w = &self.sim.world;
        let team = w.players.hot[p].team;
        team.is_some() && w.fixtures.on(w.date.add_days(1)).iter().any(|&f| w.fixtures.get(f).involves(team))
    }

    fn check_goals(&mut self) {
        let Some(p) = self.my_player() else { return };
        let w = &self.sim.world;
        let c = &w.players.cold[p];
        let top = {
            let club = w.playing_club(p);
            club.is_some() && w.clubs[club].league.is_some() && w.comps[w.clubs[club].league].tier == 1
        };
        let today = w.date;
        for g in self.session.goals.iter_mut().filter(|g| g.done.is_none()) {
            let done = match g.kind {
                GoalKind::Appearances(n) => c.senior_apps >= n,
                GoalKind::Goals(n) => c.senior_goals >= n,
                GoalKind::TopFlight => top,
                GoalKind::Personal => false,
            };
            if done {
                g.done = Some(today);
            }
        }
    }
}
