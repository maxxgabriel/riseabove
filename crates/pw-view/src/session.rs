//! A running game: the simulation plus everything the client needs to present
//! it from one perspective. Presentation state lives here, never in the world.

use std::collections::BTreeSet;
use std::path::Path;

use pw_core::{Date, DecisionId, PersonId};
use pw_sim::Sim;
use pw_world::{MindKind, PlayerStatus, World};
use serde::{Deserialize, Serialize};

use crate::model::{ApiError, ApiResult};

/// Whose eyes the client looks through.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Persp {
    /// Omniscient, read-only.
    Observer,
    /// Playing as one person; information is limited to what they could know.
    Inhabit { person: u32 },
}

impl Persp {
    pub fn person(self) -> Option<PersonId> {
        match self {
            Persp::Observer => None,
            Persp::Inhabit { person } => Some(PersonId(person)),
        }
    }
}

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

/// Everything about the session that is saved beside the world.
#[derive(Clone, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    pub persp: Persp,
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
    pub sim: Sim,
    pub meta: Meta,
    /// Bumped on every change to the world so clients can refresh.
    pub revision: u64,
    /// Rolling per-day timings for diagnostics.
    pub timings: Vec<(i32, u32)>,
}

#[derive(Serialize, Deserialize)]
struct SaveFile {
    world: World,
    meta: Meta,
}

impl Session {
    pub fn new(world: World, name: String) -> Self {
        let last_viewed = world.date.0;
        let sim = Sim::new(world);
        Self {
            sim,
            meta: Meta {
                name,
                persp: Persp::Observer,
                concealed: BTreeSet::new(),
                conceal_mine: true,
                stops: StopPolicy::default(),
                last_viewed,
                created_days: 0,
                warned: BTreeSet::new(),
            },
            revision: 1,
            timings: Vec::new(),
        }
    }

    pub fn w(&self) -> &World {
        &self.sim.world
    }

    pub fn save(&self, path: &Path) -> ApiResult<()> {
        // The world is cloned so the worker can keep simulating; saves are rare.
        let file = SaveFile { world: self.sim.world.clone(), meta: self.meta.clone() };
        pw_sim::save::save(&file, path).map_err(|e| ApiError::State(e.to_string()))
    }

    pub fn load(path: &Path) -> ApiResult<Self> {
        let f: SaveFile = pw_sim::save::load(path).map_err(|e| ApiError::State(e.to_string()))?;
        Ok(Self { sim: Sim { world: f.world }, meta: f.meta, revision: 1, timings: Vec::new() })
    }

    /// The inhabited person's player, if any and still active.
    pub fn my_player(&self) -> Option<pw_core::PlayerId> {
        let p = self.meta.persp.person()?;
        let person = self.w().people.get(p)?;
        person.player.get()
    }

    pub fn my_person(&self) -> Option<PersonId> {
        self.meta.persp.person()
    }

    pub fn observe(&mut self) {
        self.release_inhabited();
        self.meta.persp = Persp::Observer;
        self.revision += 1;
    }

    fn release_inhabited(&mut self) {
        if let Some(p) = self.meta.persp.person() {
            if let Some(person) = self.sim.world.people.get_mut(p) {
                person.mind = MindKind::Ai;
            }
        }
    }

    pub fn inhabit(&mut self, person: PersonId) -> ApiResult<()> {
        let w = &self.sim.world;
        let Some(pp) = w.people.get(person) else { return Err(ApiError::NotFound(format!("person {}", person.0))) };
        let Some(pl) = pp.player.get() else { return Err(ApiError::State("Only players can be inhabited for now.".into())) };
        if w.players.hot[pl].status == PlayerStatus::Retired {
            return Err(ApiError::State("This person has retired.".into()));
        }
        self.release_inhabited();
        self.sim.world.people[person].mind = MindKind::External;
        self.meta.persp = Persp::Inhabit { person: person.0 };
        self.meta.last_viewed = self.sim.world.date.0;
        self.revision += 1;
        Ok(())
    }

    /// Answer a decision that belongs to the inhabited person.
    pub fn answer(&mut self, id: DecisionId, choice: u8) -> ApiResult<()> {
        let me = self.my_person().ok_or_else(|| ApiError::State("You are observing; there is nobody to answer for.".into()))?;
        let d = self.sim.world.decisions.all.get(id).ok_or_else(|| ApiError::NotFound(format!("decision {}", id.0)))?;
        if d.person != me {
            return Err(ApiError::State("That decision belongs to someone else.".into()));
        }
        if d.resolved {
            return Err(ApiError::State("That decision has already been settled.".into()));
        }
        if !self.sim.answer(id, choice) {
            return Err(ApiError::Bad("That is not one of the available responses.".into()));
        }
        self.revision += 1;
        Ok(())
    }

    pub fn today(&self) -> Date {
        self.sim.world.date
    }
}
