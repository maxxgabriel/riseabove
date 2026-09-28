//! PlayerCareerCore: the protagonist's side of the world (09, 10, 11).
//!
//! The protagonist is an ordinary player whose person has an `External`
//! mind. Everything here reads the world through what he could know (P3) and
//! acts on it only through the same decision and conversation paths any
//! player has (P1).

pub mod create;
pub mod goals;
pub mod inbox;
pub mod life;
pub mod milestones;
pub mod selfview;
pub mod talk;
pub mod views;

use pw_core::{Date, DecisionId, PersonId, PlayerId};
use pw_sim::Sim;
use pw_world::World;
use serde::{Deserialize, Serialize};

pub use create::{CreateOptions, StartStage, TalentTier};
pub use inbox::{Message, Priority, Sender};

#[derive(Clone, Serialize, Deserialize)]
pub struct Career {
    pub person: PersonId,
    pub player: PlayerId,
    pub created: Date,
    pub inbox: inbox::Inbox,
    pub life: life::Life,
    pub milestones: Vec<milestones::Milestone>,
    pub goals: Vec<goals::CareerGoal>,
}

/// A running game: the world plus the protagonist's life in it.
pub struct Game {
    pub sim: Sim,
    pub career: Career,
}

#[derive(Serialize, Deserialize)]
struct SaveFile {
    world: World,
    career: Career,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Advance {
    Day,
    /// Until something needs the player: a decision, an important message or a match.
    UntilEvent,
    Until(Date),
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct AdvanceReport {
    pub days: u32,
    pub stopped_for: Option<String>,
}

impl Game {
    pub fn new(world: World, opts: CreateOptions) -> Result<Self, create::CreateError> {
        let mut sim = Sim::new(world);
        let career = create::create(&mut sim.world, opts)?;
        let mut g = Self { sim, career };
        inbox::welcome(&mut g);
        Ok(g)
    }

    pub fn world(&self) -> &World {
        &self.sim.world
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), pw_sim::save::SaveError> {
        // Serialize by reference through a borrowed view to avoid cloning the world.
        #[derive(Serialize)]
        struct View<'a> {
            world: &'a World,
            career: &'a Career,
        }
        pw_sim::save::save(&View { world: &self.sim.world, career: &self.career }, path)
    }

    pub fn load(path: &std::path::Path) -> Result<Self, pw_sim::save::SaveError> {
        let f: SaveFile = pw_sim::save::load(path)?;
        Ok(Self { sim: Sim::new(f.world), career: f.career })
    }

    /// Simulate one day and let the career react to it.
    pub fn step(&mut self) {
        let day = self.sim.world.date;
        life::daily(self);
        self.sim.step();
        inbox::ingest(self, day);
        milestones::detect(self, day);
        goals::update(self);
        if day.weekday() == pw_core::Weekday::Mon {
            life::weekly(self);
            inbox::coach_report(self);
        }
        if self.sim.world.date.day() == 1 {
            life::monthly(self);
            inbox::agent_report(self);
        }
    }

    pub fn advance(&mut self, mode: Advance) -> AdvanceReport {
        let mut rep = AdvanceReport::default();
        let limit = match mode {
            Advance::Day => 1,
            Advance::UntilEvent => 60,
            Advance::Until(d) => self.sim.world.date.days_until(d).clamp(0, 400) as u32,
        };
        let unread_before = self.career.inbox.important_unread();
        for _ in 0..limit {
            self.step();
            rep.days += 1;
            if mode != Advance::UntilEvent {
                continue;
            }
            if self.pending_decision().is_some() {
                rep.stopped_for = Some("A decision needs your answer".into());
                break;
            }
            if self.career.inbox.important_unread() > unread_before {
                rep.stopped_for = Some("New important message".into());
                break;
            }
            if self.match_tomorrow() {
                rep.stopped_for = Some("Match tomorrow".into());
                break;
            }
        }
        rep
    }

    pub fn pending_decision(&self) -> Option<DecisionId> {
        self.sim.world.decisions.pending_for(self.career.person).find(|(_, d)| d.answer.is_none()).map(|(id, _)| id)
    }

    pub fn answer(&mut self, id: DecisionId, choice: u8) -> bool {
        let ok = self.sim.world.decisions.answer(id, choice);
        if ok {
            self.career.inbox.mark_decided(id);
        }
        ok
    }

    fn match_tomorrow(&self) -> bool {
        let w = &self.sim.world;
        let team = w.players.hot[self.career.player].team;
        team.is_some() && w.fixtures.on(w.date.add_days(1)).iter().any(|&f| w.fixtures.get(f).involves(team))
    }

}
