//! Football agents (08 §7). An agent is a person with a business: clients,
//! a network of clubs they can reach, a reputation, and their own interests.
//! Honest agents pass on everything; greedy ones push the deals that pay them.
//! They work for AI players and for the person a human inhabits in exactly the
//! same way.

use pw_core::{AgentId, ClubId, Date, NationId, PersonId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    pub person: PersonId,
    /// Nation the agency is based in; the network radiates from here.
    pub base: NationId,
    /// Additional nations the agent has contacts in.
    pub reach: Vec<NationId>,
    /// 1–20.
    pub negotiating: u8,
    /// Size and quality of club contacts, 1–20.
    pub network: u8,
    /// How much work they put in per client, 1–20.
    pub diligence: u8,
    /// Fee appetite, 1–20 (drives which deals they favour).
    pub greed: u8,
    /// How faithfully they report to clients, 1–20.
    pub honesty: u8,
    /// 0–10,000.
    pub reputation: u16,
    pub clients: Vec<PlayerId>,
    /// Soft cap before service quality drops.
    pub capacity: u8,
    pub active: bool,
}

impl Agent {
    /// Service quality per client falls off beyond capacity.
    pub fn attention(&self) -> f32 {
        let load = self.clients.len() as f32 / f32::from(self.capacity.max(1));
        let base = f32::from(self.diligence) / 20.0;
        if load <= 1.0 { base } else { base / load }
    }

    pub fn covers(&self, nation: NationId) -> bool {
        self.base == nation || self.reach.contains(&nation)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Representation {
    pub agent: AgentId,
    pub since: Date,
    pub until: Date,
    /// Percent of contract value the agent takes.
    pub fee_pct: u8,
    /// Clubs the agent has pitched this client to recently (last one).
    pub last_pitch: Date,
    /// How satisfied the client is with the agent, 0–100.
    pub satisfaction: u8,
    /// When the agent last began quietly sounding out clubs for the client.
    pub explored: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Agents {
    pub list: pw_core::IdVec<AgentId, Agent>,
    pub of_player: FxHashMap<PlayerId, Representation>,
    /// Agent ↔ club relationship strength (grows with completed deals), 0–100.
    pub club_ties: FxHashMap<(AgentId, ClubId), u8>,
}

impl Agents {
    pub fn agent_of(&self, p: PlayerId) -> Option<AgentId> {
        self.of_player.get(&p).map(|r| r.agent)
    }

    pub fn tie(&self, a: AgentId, c: ClubId) -> u8 {
        self.club_ties.get(&(a, c)).copied().unwrap_or(0)
    }

    pub fn strengthen(&mut self, a: AgentId, c: ClubId, by: u8) {
        let e = self.club_ties.entry((a, c)).or_insert(0);
        *e = e.saturating_add(by).min(100);
    }
}
