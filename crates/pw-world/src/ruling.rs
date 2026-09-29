//! Decision memory: what was known, believed and advised when a material
//! decision was made, who supported it, and what eventually happened.
//!
//! Outcome is not the quality of the decision. A ruling stores the stances at
//! the moment of choice, and resolves later against the *true* state, so a
//! sound decision that failed and a reckless one that worked can both be told
//! apart, and people can (wrongly) learn from the outcome alone.
//!
//! Only material decisions are recorded (a rushed return, a manager's fate).
//! Mundane choices are not.

use pw_core::{ClubId, Date, EventId, PersonId, PlayerId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum RulingKind {
    /// Whether an injured player is cleared to play before the body is ready.
    ReturnFromInjury,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum StanceRole {
    Manager,
    Medical,
    Player,
}

/// One person's position on the question at the time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Stance {
    pub who: PersonId,
    pub role: StanceRole,
    /// What they believed about the state of things (for a return: percent of the case still to run). 255 = no view.
    pub believed_pct: u8,
    /// −100 (against) … 100 (for).
    pub backing: i8,
    /// Whether their word could have decided it.
    pub authority: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Outcome {
    Pending,
    /// The risk was accepted and nothing came of it.
    Held,
    /// The risk was accepted and it went wrong.
    Failed,
    /// The person who wanted it was stopped by someone with authority.
    Vetoed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ruling {
    pub id: u32,
    pub kind: RulingKind,
    pub date: Date,
    pub club: ClubId,
    pub subject: PlayerId,
    pub decider: PersonId,
    pub stances: SmallVec<[Stance; 4]>,
    /// The risk actually accepted: percent of the case that was truly left.
    pub true_pct: u8,
    /// How much the decider wanted it (0–100), from the stakes.
    pub want: u8,
    pub outcome: Outcome,
    pub resolved: Option<Date>,
    /// The event that announced it (later events cite this as their cause).
    pub event: EventId,
}

impl Ruling {
    /// Was the decider's belief close to the truth? A sound decision on a bad belief is still a bad decision.
    pub fn belief_error_pct(&self) -> i16 {
        let d = self.stances.iter().find(|s| s.who == self.decider).map_or(255, |s| s.believed_pct);
        if d == 255 { 0 } else { i16::from(d) - i16::from(self.true_pct) }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DecisionMemory {
    pub rulings: Vec<Ruling>,
    next: u32,
}

impl DecisionMemory {
    pub fn add(&mut self, mut r: Ruling) -> u32 {
        r.id = self.next;
        self.next += 1;
        let id = r.id;
        self.rulings.push(r);
        id
    }

    pub fn get(&self, id: u32) -> Option<&Ruling> {
        self.rulings.binary_search_by_key(&id, |r| r.id).ok().map(|i| &self.rulings[i])
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Ruling> {
        match self.rulings.binary_search_by_key(&id, |r| r.id) {
            Ok(i) => Some(&mut self.rulings[i]),
            Err(_) => None,
        }
    }

    pub fn about(&self, subject: PlayerId) -> impl Iterator<Item = &Ruling> {
        self.rulings.iter().filter(move |r| r.subject == subject)
    }

    /// Forget uneventful old rulings; failures and anything unresolved stay.
    pub fn compact(&mut self, before: Date) {
        self.rulings.retain(|r| r.date >= before || matches!(r.outcome, Outcome::Failed | Outcome::Pending));
    }
}
