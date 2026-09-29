//! A running game: the simulation plus everything the client needs to present
//! it from one perspective. Presentation state lives here, never in the world.

use std::collections::BTreeSet;
use std::path::Path;

use pw_career::Game;
use pw_core::{Date, DecisionId, PersonId};
use pw_world::{Intent, PlayerStatus, World};
use serde::{Deserialize, Serialize};

use crate::model::{ApiError, ApiResult};

/// When a long advance stops on its own.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StopPolicy {
    pub decisions: bool,
    pub matches: bool,
    pub major: bool,
}

impl Default for StopPolicy {
    fn default() -> Self {
        Self { decisions: true, matches: true, major: true }
    }
}

/// Presentation state saved beside the world and the career session.
#[derive(Clone, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    /// Fixtures whose result the viewer has not revealed yet.
    pub concealed: BTreeSet<u64>,
    pub conceal_mine: bool,
    pub stops: StopPolicy,
    /// Date the viewer last looked at Today, for "since you last looked".
    pub last_viewed: i32,
    pub created_days: u64,
    /// Decisions the viewer has been warned about while advancing.
    pub warned: BTreeSet<u32>,
}

pub struct Session {
    /// The world plus the human's career session (who is inhabited, answers, notes, goals).
    pub game: Game,
    pub meta: Meta,
    /// Bumped on every change to the world so clients can refresh.
    pub revision: u64,
    /// Rolling per-day timings for diagnostics.
    pub timings: Vec<(i32, u32)>,
}

/// The save layout: the same first two fields as the text client's, so either program can open
/// the other's file (the trailing presentation state is ignored by the text client).
#[derive(Serialize, Deserialize)]
struct SaveFile {
    world: World,
    session: pw_career::Session,
    meta: Meta,
}

#[derive(Deserialize)]
struct TextClientSave {
    world: World,
    session: pw_career::Session,
}

impl Session {
    pub fn new(world: World, name: String) -> Self {
        let meta = Meta::fresh(name, world.date);
        Self { game: Game::new(world), meta, revision: 1, timings: Vec::new() }
    }

    pub fn w(&self) -> &World {
        &self.game.sim.world
    }

    pub fn save(&self, path: &Path) -> ApiResult<()> {
        // The world is cloned so the worker can keep simulating; saves are rare.
        let file = SaveFile { world: self.game.sim.world.clone(), session: self.game.session.clone(), meta: self.meta.clone() };
        pw_sim::save::save(&file, path).map_err(|e| ApiError::State(e.to_string()))
    }

    pub fn load(path: &Path) -> ApiResult<Self> {
        match pw_sim::save::load::<SaveFile>(path) {
            Ok(f) => Ok(Self::assemble(f.world, f.session, f.meta)),
            // Only a shape mismatch may mean "text client save"; version, damage and io errors are shown as they are.
            Err(first) if !matches!(first, pw_sim::save::SaveError::Encode(_)) => Err(ApiError::State(first.to_string())),
            Err(first) => {
                // A save from the text client has no presentation state.
                let f: TextClientSave = pw_sim::save::load(path).map_err(|_| ApiError::State(first.to_string()))?;
                let meta = Meta::fresh(path.file_stem().map_or("Saved world".into(), |s| s.to_string_lossy().to_string()), f.world.date);
                Ok(Self::assemble(f.world, f.session, meta))
            }
        }
    }

    fn assemble(world: World, session: pw_career::Session, meta: Meta) -> Self {
        let mut game = Game::new(world);
        game.session = session;
        Self { game, meta, revision: 1, timings: Vec::new() }
    }

    /// The inhabited person, if any.
    pub fn my_person(&self) -> Option<PersonId> {
        self.game.session.controlled
    }

    /// The inhabited person's player record, if they have one.
    pub fn my_player(&self) -> Option<pw_core::PlayerId> {
        let person = self.w().people.get(self.my_person()?)?;
        person.player.get()
    }

    pub fn observe(&mut self) {
        self.game.release();
        self.revision += 1;
    }

    pub fn inhabit(&mut self, person: PersonId, salt: u64) -> ApiResult<()> {
        let w = self.w();
        let Some(pp) = w.people.get(person) else { return Err(ApiError::NotFound(format!("person {}", person.0))) };
        let Some(pl) = pp.player.get() else { return Err(ApiError::State("Only people who play can be inhabited for now.".into())) };
        let _ = (pl, PlayerStatus::Retired); // a retired player may still be inhabited: retirement is a change of job
        if !self.game.take_control(person, salt) {
            return Err(ApiError::State("That person cannot be inhabited.".into()));
        }
        self.meta.last_viewed = self.game.sim.world.date.0;
        self.revision += 1;
        Ok(())
    }

    /// Answer a decision that belongs to the inhabited person.
    pub fn answer(&mut self, id: DecisionId, choice: u8) -> ApiResult<()> {
        let me = self.my_person().ok_or_else(|| ApiError::State("You are observing; there is nobody to answer for.".into()))?;
        let d = self.w().decisions.all.get(id).ok_or_else(|| ApiError::NotFound(format!("decision {}", id.0)))?;
        if d.person != me {
            return Err(ApiError::State("That decision belongs to someone else.".into()));
        }
        if d.resolved {
            return Err(ApiError::State("That decision has already been settled.".into()));
        }
        if !self.game.answer(id, choice) {
            return Err(ApiError::Bad("That is not one of the available responses.".into()));
        }
        self.revision += 1;
        Ok(())
    }

    /// Something the inhabited person decides to do; the world applies it on the next simulated day.
    pub fn act(&mut self, intent: Intent) -> ApiResult<()> {
        if !self.game.act(intent) {
            return Err(ApiError::State("You are observing; there is nobody to act for.".into()));
        }
        self.revision += 1;
        Ok(())
    }

    pub fn today(&self) -> Date {
        self.game.sim.world.date
    }
}

impl Meta {
    fn fresh(name: String, date: Date) -> Self {
        Self { name, concealed: BTreeSet::new(), conceal_mine: true, stops: StopPolicy::default(), last_viewed: date.0, created_days: 0, warned: BTreeSet::new() }
    }
}
