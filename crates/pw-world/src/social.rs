//! Relationships, memories and promises between people (07 §11, 09 §3–4, S12).
//!
//! The source of truth is the *memory*: a typed, dated, sourced episode ("broke
//! a promise to me", "argued in front of the squad", "put in extra work all
//! winter"). The `Rel` numbers are a cached summary that memories move; any
//! behaviour that needs to know *why* reads the memories themselves.
//!
//! Relationships are directed and sparse: only pairs that have actually
//! interacted are stored. Every person in the world uses the same store and
//! the same rules, so a manager's trust in an AI midfielder and in the person
//! a human inhabits evolve identically (P1).

use pw_core::math::exp;
use pw_core::{ClubId, Date, EventId, PersonId, Pos};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::contract::SquadStatus;

/// How `from` feels about `to` — a summary of their shared memories.
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
    /// Trained poorly over a stretch the observer noticed.
    PoorAttitude,
    /// Stood up for them in a meeting or to the press.
    DefendedMe,
    /// Talked calmly and honestly in a difficult conversation.
    HonestTalk,
    /// Asked for something and was refused.
    Refused,
    /// Was given a chance (a debut, a start, a new role).
    GaveChance,
    /// Negotiated hard or badly on their behalf / against them.
    HardBargain,
    LetDown,
    /// Joined a club this person (or fanbase) resents.
    Betrayal,
    /// Played together through a long stretch.
    SharedPitch,
    /// Believed to have leaked something to the press (rightly or not).
    Leaked,
    /// A confrontation: a fight, a shouting match, a shove.
    Fought,
    /// Helped settle a conflict.
    Mediated,
    /// Was shielded by someone in authority.
    Protected,
    /// Was blamed (fairly or not) for something.
    Blamed,
    /// Was trusted with something private.
    Confided,
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
            MemoryKind::PoorAttitude => "showed a poor attitude in training",
            MemoryKind::DefendedMe => "stood up for them",
            MemoryKind::HonestTalk => "had an honest conversation",
            MemoryKind::Refused => "turned down a request",
            MemoryKind::GaveChance => "gave them a chance",
            MemoryKind::HardBargain => "drove a hard bargain",
            MemoryKind::LetDown => "let them down",
            MemoryKind::Betrayal => "crossed a line they won't forget",
            MemoryKind::SharedPitch => "shared the pitch",
            MemoryKind::Leaked => "leaked to the press",
            MemoryKind::Fought => "clashed with them",
            MemoryKind::Mediated => "helped settle things",
            MemoryKind::Protected => "protected them",
            MemoryKind::Blamed => "blamed them",
            MemoryKind::Confided => "confided in them",
        }
    }

    /// Immediate effect on the rememberer's view: (affinity, trust, respect).
    pub const fn effect(self) -> (i8, i8, i8) {
        match self {
            MemoryKind::PromiseKept => (6, 14, 6),
            MemoryKind::PromiseBroken => (-12, -24, -8),
            MemoryKind::Argument => (-14, -6, -4),
            MemoryKind::PublicPraise => (10, 4, 6),
            MemoryKind::PublicCriticism => (-16, -8, -6),
            MemoryKind::Celebrated => (5, 1, 1),
            MemoryKind::Mentored => (12, 8, 10),
            MemoryKind::Dropped => (-5, -3, 0),
            MemoryKind::Backed => (8, 8, 4),
            MemoryKind::Fined => (-8, -4, 0),
            MemoryKind::TransferRequest => (-10, -14, -4),
            MemoryKind::RefusedLoan => (-4, -6, 0),
            MemoryKind::ExtraWork => (3, 6, 8),
            MemoryKind::Apologised => (6, 4, 2),
            MemoryKind::Rivalry => (-3, 0, 2),
            MemoryKind::Settled => (10, 5, 2),
            MemoryKind::Insulted => (-18, -6, -6),
            MemoryKind::Supported => (10, 6, 3),
            MemoryKind::PoorAttitude => (-4, -8, -8),
            MemoryKind::DefendedMe => (14, 10, 6),
            MemoryKind::HonestTalk => (4, 6, 4),
            MemoryKind::Refused => (-5, -3, 0),
            MemoryKind::GaveChance => (10, 6, 4),
            MemoryKind::HardBargain => (-4, -2, 3),
            MemoryKind::LetDown => (-8, -12, -4),
            MemoryKind::Betrayal => (-30, -25, -10),
            MemoryKind::SharedPitch => (2, 1, 1),
            MemoryKind::Leaked => (-10, -20, -6),
            MemoryKind::Fought => (-20, -10, -6),
            MemoryKind::Mediated => (8, 6, 8),
            MemoryKind::Protected => (12, 8, 2),
            MemoryKind::Blamed => (-14, -16, -4),
            MemoryKind::Confided => (4, 5, 1),
        }
    }

    /// Days for a memory to lose half its weight (before personality).
    pub const fn half_life_days(self) -> u16 {
        match self {
            MemoryKind::Celebrated | MemoryKind::SharedPitch | MemoryKind::Dropped | MemoryKind::Confided => 60,
            MemoryKind::Refused | MemoryKind::HardBargain | MemoryKind::Rivalry | MemoryKind::HonestTalk => 120,
            MemoryKind::Argument | MemoryKind::Fined | MemoryKind::PoorAttitude | MemoryKind::ExtraWork | MemoryKind::Apologised => 180,
            MemoryKind::PublicPraise | MemoryKind::PublicCriticism | MemoryKind::RefusedLoan | MemoryKind::Backed | MemoryKind::Supported | MemoryKind::Fought | MemoryKind::Mediated => 270,
            MemoryKind::PromiseKept
            | MemoryKind::TransferRequest
            | MemoryKind::LetDown
            | MemoryKind::Insulted
            | MemoryKind::GaveChance
            | MemoryKind::Leaked
            | MemoryKind::Protected
            | MemoryKind::Blamed => 365,
            MemoryKind::PromiseBroken | MemoryKind::Settled | MemoryKind::DefendedMe => 540,
            MemoryKind::Mentored | MemoryKind::Betrayal => 1460,
        }
    }

    /// Formative memories never fade below a floor.
    pub const fn formative(self) -> bool {
        matches!(self, MemoryKind::Mentored | MemoryKind::Betrayal | MemoryKind::DefendedMe | MemoryKind::PromiseBroken)
    }

    pub const fn negative(self) -> bool {
        let (a, t, _) = self.effect();
        a + t < 0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Memory {
    pub from: PersonId,
    pub about: PersonId,
    pub kind: MemoryKind,
    pub date: Date,
    /// Initial strength, 0–100.
    pub salience: u8,
    /// Happened in front of others (squad, press, fans).
    pub public: bool,
    /// The event that created it.
    pub cause: EventId,
}

impl Memory {
    /// Current weight, 0–100: salience decaying by kind; `grudge` (0.5–2.0)
    /// is the rememberer's disposition to hold on to things.
    pub fn weight(&self, today: Date, grudge: f32) -> f32 {
        let age = self.date.days_until(today).max(0) as f32;
        let hl = f32::from(self.kind.half_life_days()) * grudge.clamp(0.4, 2.5);
        let w = f32::from(self.salience) * exp(-0.693 * age / hl.max(1.0));
        if self.kind.formative() { w.max(f32::from(self.salience) * 0.25) } else { w }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum PromiseKind {
    /// At least this share of available league minutes over the promise window.
    Minutes {
        share: f32,
    },
    Status(SquadStatus),
    NewContract,
    /// Will not stand in the way of a move (optionally to a bigger club).
    LetLeave,
    Position(Pos),
    Loan,
    Captaincy,
    /// Improve something specific in training before being reconsidered.
    ImproveTraining,
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
            PromiseKind::ImproveTraining => "a clear improvement in training".into(),
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
    pub id: u32,
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
    /// The conversation or event in which it was made.
    pub cause: EventId,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Social {
    rel: FxHashMap<(PersonId, PersonId), Rel>,
    /// Memories held by each person (sparse).
    held: FxHashMap<PersonId, SmallVec<[Memory; 4]>>,
    pub promises: Vec<Promise>,
    next_promise: u32,
    /// Positions in `promises` by promisee and by promiser (rebuilt on prune).
    by_to: FxHashMap<PersonId, SmallVec<[u32; 2]>>,
    by_from: FxHashMap<PersonId, SmallVec<[u32; 4]>>,
}

impl Social {
    #[inline]
    pub fn get(&self, from: PersonId, to: PersonId) -> Option<Rel> {
        self.rel.get(&(from, to)).copied()
    }

    /// Every stored relationship's endpoints (for validation).
    pub fn endpoints(&self) -> impl Iterator<Item = (PersonId, PersonId)> + '_ {
        self.rel.keys().copied()
    }

    /// Everyone who has a relationship toward `to` (a full scan: use rarely,
    /// e.g. when someone starts a new career and needs contacts).
    pub fn toward(&self, to: PersonId) -> impl Iterator<Item = (PersonId, Rel)> + '_ {
        self.rel.iter().filter(move |((_, b), _)| *b == to).map(|((a, _), r)| (*a, *r))
    }

    /// Existing relationship or a first impression from `compat` (-30..=30).
    pub fn get_or(&self, from: PersonId, to: PersonId, today: Date, compat: i8) -> Rel {
        self.get(from, to).unwrap_or_else(|| Rel::neutral(today, compat))
    }

    /// Shift how `from` sees `to` without a memory (slow, ambient drift such as
    /// training together every day).
    pub fn adjust(&mut self, from: PersonId, to: PersonId, today: Date, compat: i8, affinity: i32, trust: i32, respect: i32) {
        let r = self.rel.entry((from, to)).or_insert_with(|| Rel::neutral(today, compat));
        r.affinity = (i32::from(r.affinity) + affinity).clamp(-100, 100) as i8;
        r.trust = (i32::from(r.trust) + trust).clamp(0, 100) as u8;
        r.respect = (i32::from(r.respect) + respect).clamp(0, 100) as u8;
        r.last = today;
    }

    /// Record an episode: `from` remembers `kind` about `about`. `intensity`
    /// (0.2–2.0) scales both the immediate effect and the salience — being
    /// criticised by a hero stings more than by a stranger.
    #[allow(clippy::too_many_arguments)]
    pub fn remember(&mut self, from: PersonId, about: PersonId, kind: MemoryKind, date: Date, cause: EventId, public: bool, intensity: f32, compat: i8) {
        if from == about || from.is_none() || about.is_none() {
            return;
        }
        let k = intensity.clamp(0.2, 2.0) * if public { 1.25 } else { 1.0 };
        let (a, t, r) = kind.effect();
        let scale = |v: i8| (f32::from(v) * k).round() as i32;
        self.adjust(from, about, date, compat, scale(a), scale(t), scale(r));
        let salience = (60.0 * k).clamp(10.0, 100.0) as u8;
        let list = self.held.entry(from).or_default();
        list.push(Memory { from, about, kind, date, salience, public, cause });
        // Bound memory per person: forget the weakest non-formative episode.
        if list.len() > 48
            && let Some(i) = list.iter().enumerate().filter(|(_, m)| !m.kind.formative()).min_by_key(|(_, m)| (m.salience, m.date)).map(|(i, _)| i)
        {
            list.remove(i);
        }
    }

    /// Everything `from` remembers about `about`, oldest first.
    pub fn recall(&self, from: PersonId, about: PersonId) -> impl Iterator<Item = &Memory> {
        self.held.get(&from).into_iter().flatten().filter(move |m| m.about == about)
    }

    /// Everything a person remembers, about anyone.
    pub fn memories_of(&self, from: PersonId) -> impl Iterator<Item = &Memory> {
        self.held.get(&from).into_iter().flatten()
    }

    /// Memories either side holds about the other.
    pub fn memories_between(&self, a: PersonId, b: PersonId) -> impl Iterator<Item = &Memory> {
        self.recall(a, b).chain(self.recall(b, a))
    }

    /// Weighted strength of one kind of memory `from` holds about `about`.
    pub fn weight_of(&self, from: PersonId, about: PersonId, kind: MemoryKind, today: Date, grudge: f32) -> f32 {
        self.recall(from, about).filter(|m| m.kind == kind).map(|m| m.weight(today, grudge)).sum()
    }

    /// Net weight of negative memories minus positive ones (a grievance score).
    pub fn grievance(&self, from: PersonId, about: PersonId, today: Date, grudge: f32) -> f32 {
        self.recall(from, about).map(|m| if m.kind.negative() { m.weight(today, grudge) } else { -0.6 * m.weight(today, grudge) }).sum()
    }

    /// The strongest memory `from` holds about `about` right now.
    pub fn defining_memory(&self, from: PersonId, about: PersonId, today: Date, grudge: f32) -> Option<&Memory> {
        self.recall(from, about).max_by(|a, b| a.weight(today, grudge).total_cmp(&b.weight(today, grudge)))
    }

    pub fn relations_of(&self, from: PersonId) -> impl Iterator<Item = (PersonId, Rel)> + '_ {
        self.rel.iter().filter(move |((f, _), _)| *f == from).map(|((_, t), r)| (*t, *r))
    }

    pub fn make_promise(&mut self, from: PersonId, to: PersonId, club: ClubId, kind: PromiseKind, made: Date, due: Date, cause: EventId) -> u32 {
        let id = self.next_promise;
        self.next_promise += 1;
        let at = self.promises.len() as u32;
        self.promises.push(Promise { id, from, to, club, kind, made, due, state: PromiseState::Open, team_minutes: 0, player_minutes: 0, cause });
        self.by_to.entry(to).or_default().push(at);
        self.by_from.entry(from).or_default().push(at);
        id
    }

    pub fn promise(&self, id: u32) -> Option<&Promise> {
        // Ids rise in the order promises are made, and pruning keeps the order.
        self.promises.binary_search_by_key(&id, |p| p.id).ok().map(|i| &self.promises[i])
    }

    pub fn promise_mut(&mut self, id: u32) -> Option<&mut Promise> {
        self.promises.binary_search_by_key(&id, |p| p.id).ok().map(move |i| &mut self.promises[i])
    }

    /// Promises made to `to` (any state).
    pub fn promises_to(&self, to: PersonId) -> impl Iterator<Item = &Promise> {
        self.by_to.get(&to).into_iter().flatten().map(|&i| &self.promises[i as usize])
    }

    /// Promises made by `from` (any state).
    pub fn promises_by(&self, from: PersonId) -> impl Iterator<Item = &Promise> {
        self.by_from.get(&from).into_iter().flatten().map(|&i| &self.promises[i as usize])
    }

    pub fn open_promises_to(&self, to: PersonId) -> impl Iterator<Item = &Promise> {
        self.promises_to(to).filter(|p| p.state == PromiseState::Open)
    }

    pub fn open_promises_between(&self, from: PersonId, to: PersonId) -> impl Iterator<Item = &Promise> {
        self.promises_to(to).filter(move |p| p.from == from && p.state == PromiseState::Open)
    }

    /// How many promises `from` has broken to anyone within `days` — a reputation
    /// for keeping one's word that others can hear about.
    pub fn broken_by(&self, from: PersonId, today: Date, days: i32) -> usize {
        self.promises_by(from).filter(|p| p.state == PromiseState::Broken && p.due.days_until(today) <= days).count()
    }

    /// Forget stale relationships and faded memories; settled promises go after a while.
    pub fn prune(&mut self, today: Date, before: Date) {
        self.rel.retain(|_, r| r.last >= before || r.affinity.unsigned_abs() >= 40 || r.trust <= 20 || r.trust >= 80);
        for list in self.held.values_mut() {
            list.retain(|m| m.kind.formative() || m.weight(today, 1.0) >= 3.0);
        }
        self.held.retain(|_, l| !l.is_empty());
        self.promises.retain(|p| p.state == PromiseState::Open || p.due >= before);
        self.by_to.clear();
        self.by_from.clear();
        for (i, p) in self.promises.iter().enumerate() {
            self.by_to.entry(p.to).or_default().push(i as u32);
            self.by_from.entry(p.from).or_default().push(i as u32);
        }
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

/// How long a person holds on to things: low temperament and high loyalty
/// remember longer; easy-going people let go.
pub fn grudge_factor(p: &crate::Person) -> f32 {
    use pw_core::Hidden;
    let temp = p.hidden.f(Hidden::Temperament);
    let loyal = p.hidden.f(Hidden::Loyalty);
    (1.0 + (10.0 - temp) * 0.05 + (loyal - 10.0) * 0.03).clamp(0.5, 2.0)
}

pub type Interactions = SmallVec<[(PersonId, PersonId); 8]>;
