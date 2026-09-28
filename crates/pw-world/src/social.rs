//! Relationships, memories and promises between people (07 §11, 09 §3–4).
//!
//! Relationships are directed and sparse: only pairs that have actually
//! interacted are stored. Every person in the world uses the same store, so a
//! manager's trust in an AI midfielder and in the protagonist evolve by the
//! same rules (P1).

use pw_core::{ClubId, Date, PersonId, Pos};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::contract::SquadStatus;

/// How `from` feels about `to`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rel {
    /// Liking, -100..=100.
    pub affinity: i8,
    /// Reliability in their eyes, 0..=100 (50 neutral).
    pub trust: u8,
    /// Professional regard, 0..=100 (50 neutral).
    pub respect: u8,
    pub since: Date,
    pub last: Date,
}

impl Rel {
    pub fn neutral(today: Date, affinity: i8) -> Self {
        Self { affinity, trust: 50, respect: 50, since: today, last: today }
    }

    pub fn label(&self) -> &'static str {
        match (self.affinity, self.trust) {
            (a, _) if a >= 60 => "Close",
            (a, t) if a >= 25 && t >= 55 => "Good",
            (a, _) if a >= 10 => "Friendly",
            (a, _) if a <= -60 => "Hostile",
            (a, _) if a <= -25 => "Strained",
            (_, t) if t <= 25 => "Distrustful",
            _ => "Neutral",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MemoryKind {
    PromiseKept,
    PromiseBroken,
    Argument,
    PublicPraise,
    PublicCriticism,
    Celebrated,
    Mentored,
    Dropped,
    Backed,
    Fined,
    TransferRequest,
    RefusedLoan,
    ExtraWork,
    Apologised,
    Rivalry,
    Settled,
    Insulted,
    Supported,
}

impl MemoryKind {
    pub fn text(self) -> &'static str {
        match self {
            MemoryKind::PromiseKept => "kept a promise",
            MemoryKind::PromiseBroken => "broke a promise",
            MemoryKind::Argument => "had an argument",
            MemoryKind::PublicPraise => "praised publicly",
            MemoryKind::PublicCriticism => "criticised publicly",
            MemoryKind::Celebrated => "celebrated together",
            MemoryKind::Mentored => "mentored",
            MemoryKind::Dropped => "was dropped",
            MemoryKind::Backed => "backed",
            MemoryKind::Fined => "was fined",
            MemoryKind::TransferRequest => "handed in a transfer request",
            MemoryKind::RefusedLoan => "refused a loan",
            MemoryKind::ExtraWork => "put in extra work",
            MemoryKind::Apologised => "apologised",
            MemoryKind::Rivalry => "competed for a place",
            MemoryKind::Settled => "helped settle in",
            MemoryKind::Insulted => "was insulted",
            MemoryKind::Supported => "offered support",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Memory {
    pub from: PersonId,
    pub about: PersonId,
    pub kind: MemoryKind,
    pub date: Date,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum PromiseKind {
    /// At least this share of available league minutes over the promise window.
    Minutes { share: f32 },
    Status(SquadStatus),
    NewContract,
    /// Will not stand in the way of a move (optionally to a bigger club).
    LetLeave,
    Position(Pos),
    Loan,
    Captaincy,
}

impl PromiseKind {
    pub fn text(&self) -> String {
        match self {
            PromiseKind::Minutes { share } => format!("regular football ({:.0}% of minutes)", share * 100.0),
            PromiseKind::Status(s) => format!("status as {}", s.label()),
            PromiseKind::NewContract => "an improved contract".into(),
            PromiseKind::LetLeave => "permission to leave if a suitable offer arrives".into(),
            PromiseKind::Position(p) => format!("a chance to play as {}", p.code()),
            PromiseKind::Loan => "a loan move for game time".into(),
            PromiseKind::Captaincy => "the captaincy".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PromiseState {
    Open,
    Kept,
    Broken,
    Void,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Promise {
    pub from: PersonId,
    pub to: PersonId,
    pub club: ClubId,
    pub kind: PromiseKind,
    pub made: Date,
    pub due: Date,
    pub state: PromiseState,
    /// Minutes available to the player's team since `made` (for minutes promises).
    pub team_minutes: u32,
    pub player_minutes: u32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Social {
    rel: FxHashMap<(PersonId, PersonId), Rel>,
    pub memories: Vec<Memory>,
    pub promises: Vec<Promise>,
}

impl Social {
    #[inline]
    pub fn get(&self, from: PersonId, to: PersonId) -> Option<Rel> {
        self.rel.get(&(from, to)).copied()
    }

    /// Existing relationship or a first impression from `compat` (-30..=30).
    pub fn get_or(&self, from: PersonId, to: PersonId, today: Date, compat: i8) -> Rel {
        self.get(from, to).unwrap_or_else(|| Rel::neutral(today, compat))
    }

    /// Shift how `from` sees `to`.
    pub fn adjust(&mut self, from: PersonId, to: PersonId, today: Date, compat: i8, affinity: i32, trust: i32, respect: i32) {
        let r = self.rel.entry((from, to)).or_insert_with(|| Rel::neutral(today, compat));
        r.affinity = (i32::from(r.affinity) + affinity).clamp(-100, 100) as i8;
        r.trust = (i32::from(r.trust) + trust).clamp(0, 100) as u8;
        r.respect = (i32::from(r.respect) + respect).clamp(0, 100) as u8;
        r.last = today;
    }

    pub fn remember(&mut self, from: PersonId, about: PersonId, kind: MemoryKind, date: Date) {
        self.memories.push(Memory { from, about, kind, date });
    }

    pub fn memories_between(&self, a: PersonId, b: PersonId) -> impl Iterator<Item = &Memory> {
        self.memories.iter().filter(move |m| (m.from == a && m.about == b) || (m.from == b && m.about == a))
    }

    pub fn relations_of(&self, from: PersonId) -> impl Iterator<Item = (PersonId, Rel)> + '_ {
        self.rel.iter().filter(move |((f, _), _)| *f == from).map(|((_, t), r)| (*t, *r))
    }

    pub fn open_promises_to(&self, to: PersonId) -> impl Iterator<Item = &Promise> {
        self.promises.iter().filter(move |p| p.to == to && p.state == PromiseState::Open)
    }

    /// Forget relationships nobody has touched in years and old memories.
    pub fn prune(&mut self, before: Date) {
        self.rel.retain(|_, r| r.last >= before || r.affinity.unsigned_abs() >= 40);
        self.memories.retain(|m| m.date >= before);
        self.promises.retain(|p| p.state == PromiseState::Open || p.due >= before);
    }

    pub fn len(&self) -> usize {
        self.rel.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rel.is_empty()
    }
}

/// First-impression compatibility between two people (-30..=30): shared
/// nation/language, age gap, clashing temperaments.
pub fn compatibility(a: &crate::Person, b: &crate::Person, today: Date) -> i8 {
    use pw_core::Hidden;
    let mut c: i32 = 0;
    if a.nation == b.nation {
        c += 12;
    } else if a.nation2.is_some() && (a.nation2 == b.nation || a.nation2 == b.nation2) {
        c += 6;
    }
    let gap = (a.dob.days_until(b.dob).abs() / 365).min(20);
    c += 6 - gap / 2;
    let ta = i32::from(a.hidden.get(Hidden::Temperament));
    let tb = i32::from(b.hidden.get(Hidden::Temperament));
    if ta <= 6 && tb <= 6 {
        c -= 10;
    }
    let pa = i32::from(a.hidden.get(Hidden::Professionalism));
    let pb = i32::from(b.hidden.get(Hidden::Professionalism));
    c -= (pa - pb).abs() / 3;
    let _ = today;
    c.clamp(-30, 30) as i8
}

pub type Interactions = SmallVec<[(PersonId, PersonId); 8]>;
