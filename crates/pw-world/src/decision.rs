//! The DecisionPort (01 §5, S5–S6). A decision is something the world needs a
//! person to answer: an offer, a summons, a counter in contract talks. The
//! option set is generated from the current state; the person's own AI mind
//! evaluates the same options and its choice becomes the default applied at
//! the deadline. AI minds answer immediately; `External` minds (a human)
//! answer through the client. The simulation never asks who is behind a mind.

use pw_core::{ClubId, Date, DecisionId, IdVec, MeetingId, Money, NationId, PersonId, PlayerId, TalkId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::contract::{Contract, Loan, SquadStatus};
use crate::interaction::Tone;

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
    /// Personal terms on the table outside structured talks (legacy/simple offers).
    ContractOffer { club: ClubId, contract: Contract, renewal: bool },
    /// Parent club proposes a loan.
    LoanOffer { loan: Loan },
    /// A club offers a contract to an unattached player.
    FreeAgentOffer { club: ClubId, contract: Contract },
    /// Your turn in contract talks.
    Negotiation { talk: TalkId },
    /// Someone has called a meeting with you, or answered yours.
    Meeting { meeting: MeetingId },
    /// A partner has proposed something that needs an answer.
    Partner { partner: PersonId, ask: crate::intent::PartnerAsk },
    /// A club invites an unattached player for a trial.
    Trial { club: ClubId, days: u8 },
    /// A dual national has been called up; accepting commits them to `nation`,
    /// rejecting commits them to `other`.
    NationChoice { nation: NationId, other: NationId },
    /// A serious injury: accept = surgery, reject = rehabilitation.
    Treatment { surgery_days: u16, rehab_days: u16 },
    /// A brand offers an endorsement.
    Endorsement { brand: u32, fee_year: Money, years: u8, days: u8 },
    /// Something happened that is yours to deal with (options are responses).
    Incident { incident: u32 },
    /// You are asked (or may ask) something about an incident.
    IncidentAsk { incident: u32, ask: crate::incident::Ask },
    /// A question at a press conference.
    PressQuestion { conference: u32, question: u8 },
}

/// One available answer. Choices are semantic; the client renders them.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Choice {
    Accept,
    Reject,
    /// Counter-offer in talks.
    Counter { wage: Money, years: u8, status: Option<SquadStatus>, release_clause: Money },
    /// Answer a conversation in a tone.
    Respond(Tone),
    /// Refuse to take part.
    Decline,
    /// Deal with an incident in this way.
    Handle(crate::incident::Response),
    /// Answer a question (or post) with this stance.
    Say(crate::media::Stance),
}

impl Choice {
    pub fn is_positive(&self) -> bool {
        matches!(self, Choice::Accept | Choice::Counter { .. } | Choice::Respond(_))
    }
}

impl DecisionKind {
    pub fn title(&self) -> &'static str {
        match self {
            DecisionKind::TransferTalks { .. } => "Transfer approach",
            DecisionKind::ContractOffer { renewal: true, .. } => "Contract renewal offer",
            DecisionKind::ContractOffer { .. } => "Contract offer",
            DecisionKind::LoanOffer { .. } => "Loan proposal",
            DecisionKind::FreeAgentOffer { .. } => "Contract offer",
            DecisionKind::Negotiation { .. } => "Contract talks",
            DecisionKind::Meeting { .. } => "Meeting",
            DecisionKind::Partner { .. } => "Your partner",
            DecisionKind::Trial { .. } => "Trial invitation",
            DecisionKind::NationChoice { .. } => "International allegiance",
            DecisionKind::Treatment { .. } => "Treatment",
            DecisionKind::Endorsement { .. } => "Endorsement offer",
            DecisionKind::Incident { .. } => "Something to deal with",
            DecisionKind::PressQuestion { .. } => "Press conference",
            DecisionKind::IncidentAsk { ask: crate::incident::Ask::RequestLeave, .. } => "Ask for time away?",
            DecisionKind::IncidentAsk { ask: crate::incident::Ask::Apologise, .. } => "Apologise?",
        }
    }

    /// The fixed two-way option set for simple offers.
    pub fn simple_options(&self) -> SmallVec<[Choice; 5]> {
        let mut v = SmallVec::new();
        v.push(Choice::Accept);
        v.push(Choice::Reject);
        v
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub person: PersonId,
    pub player: PlayerId,
    pub kind: DecisionKind,
    /// Options generated from state when the decision was raised.
    pub options: SmallVec<[Choice; 5]>,
    pub created: Date,
    pub deadline: Date,
    /// Index of what this person's own AI mind would choose; applied at the deadline.
    pub default: u8,
    pub answer: Option<u8>,
    pub resolved: bool,
}

impl Decision {
    pub fn chosen(&self) -> Choice {
        let i = usize::from(self.answer.unwrap_or(self.default));
        self.options.get(i).copied().unwrap_or(Choice::Reject)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Decisions {
    pub all: IdVec<DecisionId, Decision>,
    /// First unresolved decision (everything before is resolved) — keeps scans short.
    pub cursor: u32,
}

impl Decisions {
    pub fn push(&mut self, d: Decision) -> DecisionId {
        self.all.push(d)
    }

    fn open(&self) -> impl Iterator<Item = (DecisionId, &Decision)> {
        self.all.iter_enumerated().skip(self.cursor as usize).filter(|(_, d)| !d.resolved)
    }

    pub fn pending_for(&self, person: PersonId) -> impl Iterator<Item = (DecisionId, &Decision)> {
        self.open().filter(move |(_, d)| d.person == person)
    }

    pub fn answer(&mut self, id: DecisionId, choice: u8) -> bool {
        match self.all.get_mut(id) {
            Some(d) if !d.resolved && usize::from(choice) < d.options.len() => {
                d.answer = Some(choice);
                true
            }
            _ => false,
        }
    }

    /// Decisions ready to apply today: answered, or past their deadline.
    pub fn due(&self, today: Date) -> Vec<DecisionId> {
        self.open().filter(|(_, d)| d.answer.is_some() || today >= d.deadline).map(|(id, _)| id).collect()
    }

    pub fn resolve(&mut self, id: DecisionId) {
        self.all[id].resolved = true;
        while (self.cursor as usize) < self.all.len() && self.all[DecisionId(self.cursor)].resolved {
            self.cursor += 1;
        }
    }

    pub fn has_open(&self, player: PlayerId, pred: impl Fn(&DecisionKind) -> bool) -> bool {
        self.open().any(|(_, d)| d.player == player && pred(&d.kind))
    }
}
