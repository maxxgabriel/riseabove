//! Intents: things a person decides to *do* on their own initiative (S5).
//!
//! Requests arrive at a person (offers, summons) and carry a deadline and a
//! default. Intents start *from* a person. AI minds generate intents from
//! utility every week; a human's intents come from the client. Both are queued
//! here and applied by the same system, so anything a human can do an AI
//! person can do too — and does.

use pw_core::{AgentId, Date, PersonId};
use serde::{Deserialize, Serialize};

use crate::interaction::{Tone, Topic};
use crate::life::{Lifestyle, Routine};
use crate::player::TrainingPlan;
use crate::staff::StaffRole;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Intent {
    /// Ask to see someone (manager, coach, agent) about a topic.
    RequestMeeting { with: PersonId, topic: Topic, tone: Tone },
    /// Hand in a formal transfer request.
    TransferRequest,
    WithdrawTransferRequest,
    SetTraining(TrainingPlan),
    SetRoutine(Routine),
    SetLifestyle(Lifestyle),
    /// Engage an agent (they may say no).
    HireAgent(AgentId),
    DropAgent,
    /// Retire from playing. The person continues.
    Retire,
    /// A retired or unattached person looks for a football job.
    SeekStaffJob(StaffRole),
    /// Come back from retirement as a free agent.
    Unretire,
    /// Ask a partner to move in / marry / end it (the partner decides too).
    AskPartner(PartnerAsk),
    /// Look for a relationship (social life becomes open to it).
    OpenToDating(bool),
    /// Sign up to play amateur football locally.
    JoinAmateurFootball,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PartnerAsk {
    MoveIn,
    Marry,
    Separate,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PendingIntent {
    pub person: PersonId,
    pub intent: Intent,
    pub date: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Intents {
    pub queue: Vec<PendingIntent>,
    /// People open to meeting someone (sparse; absent = not looking unless single and young).
    pub dating: crate::FxHashMap<PersonId, bool>,
}

impl Intents {
    pub fn submit(&mut self, person: PersonId, intent: Intent, date: Date) {
        self.queue.push(PendingIntent { person, intent, date });
    }

    pub fn take(&mut self) -> Vec<PendingIntent> {
        std::mem::take(&mut self.queue)
    }
}
