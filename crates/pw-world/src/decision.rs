use pw_core::{ClubId, Date, DecisionId, IdVec, Money, PersonId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Loan};

/// Who makes a person's choices. The simulation never asks *who* is behind an
/// `External` mind; it only routes the request out and applies the default at
/// the deadline if nobody answers (P1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum MindKind {
    #[default]
    Ai,
    External,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DecisionKind {
    /// A club wants to talk about a transfer; accept means "open to talks".
    TransferTalks { club: ClubId, fee: Money },
    /// Personal terms on the table (new club or renewal at the current club).
    ContractOffer { club: ClubId, contract: Contract, renewal: bool },
    /// Parent club proposes a loan.
    LoanOffer { loan: Loan },
    /// A club offers a trial or a contract to an unattached player.
    FreeAgentOffer { club: ClubId, contract: Contract },
}

impl DecisionKind {
    pub fn options(&self) -> &'static [&'static str] {
        match self {
            DecisionKind::TransferTalks { .. } => &["Open to talks", "Not interested"],
            DecisionKind::ContractOffer { .. } | DecisionKind::FreeAgentOffer { .. } => &["Accept", "Reject"],
            DecisionKind::LoanOffer { .. } => &["Accept loan", "Refuse"],
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            DecisionKind::TransferTalks { .. } => "Transfer approach",
            DecisionKind::ContractOffer { renewal: true, .. } => "Contract renewal offer",
            DecisionKind::ContractOffer { .. } => "Contract offer",
            DecisionKind::LoanOffer { .. } => "Loan proposal",
            DecisionKind::FreeAgentOffer { .. } => "Contract offer",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub person: PersonId,
    pub player: PlayerId,
    pub kind: DecisionKind,
    pub created: Date,
    pub deadline: Date,
    /// What this person's own AI mind would choose; applied at the deadline.
    pub default: u8,
    pub answer: Option<u8>,
    pub resolved: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Decisions {
    pub all: IdVec<DecisionId, Decision>,
}

impl Decisions {
    pub fn push(&mut self, d: Decision) -> DecisionId {
        self.all.push(d)
    }

    pub fn pending_for(&self, person: PersonId) -> impl Iterator<Item = (DecisionId, &Decision)> {
        self.all.iter_enumerated().filter(move |(_, d)| d.person == person && !d.resolved)
    }

    pub fn answer(&mut self, id: DecisionId, choice: u8) -> bool {
        match self.all.get_mut(id) {
            Some(d) if !d.resolved && usize::from(choice) < d.kind.options().len() => {
                d.answer = Some(choice);
                true
            }
            _ => false,
        }
    }

    /// Decisions ready to apply today: answered, or past their deadline.
    pub fn due(&self, today: Date) -> Vec<DecisionId> {
        self.all
            .iter_enumerated()
            .filter(|(_, d)| !d.resolved && (d.answer.is_some() || today >= d.deadline))
            .map(|(id, _)| id)
            .collect()
    }

    pub fn has_open(&self, player: PlayerId, pred: impl Fn(&DecisionKind) -> bool) -> bool {
        self.all.iter().any(|d| !d.resolved && d.player == player && pred(&d.kind))
    }
}
