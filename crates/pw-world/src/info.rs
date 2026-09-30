//! Information as a thing that travels (S15, 11 §2).
//!
//! Truth lives in the world. An `InfoItem` is a piece of that truth that
//! people can know: a row in training, a club's interest in a player, a
//! dressing room turning on its manager, an injury worse than announced.
//! Each item records who knows it, how they learned it (saw it, were told by
//! whom, read it), how faithfully their version matches the truth, and every
//! telling along the way with its motive. Journalists are people too: an
//! item reaches the press only when someone with a reason to talk tells
//! someone who writes.
//!
//! This is the substrate for leaks, rumours, inbox messages, press
//! questions, supporter chatter and the "why" of any story.

use pw_core::{ClubId, Date, EventId, Money, PersonId, PlayerId, StoryId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// What a piece of information is about. Semantic, never prose.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum InfoKind {
    /// An incident happened (`incidents` registry id).
    Incident { incident: u32 },
    /// A club is watching or interested in a player.
    Interest { club: ClubId, player: PlayerId },
    /// A bid or talks exist.
    Bid { club: ClubId, player: PlayerId, fee: Money },
    /// A player is unhappy with someone (manager, club, role).
    Unhappy { player: PlayerId, with: PersonId },
    /// A manager's job is in danger.
    JobInDanger { club: ClubId, manager: PersonId },
    /// A disciplinary action (fine, suspension, being dropped for it).
    Discipline { player: PlayerId, club: ClubId },
    /// An injury is worse than has been said.
    InjuryWorse { player: PlayerId, days: u16 },
    /// Contract talks are stalling (or agreed but not announced).
    ContractTalks { player: PlayerId, club: ClubId, stalling: bool },
    /// A private matter in someone's life.
    Private { person: PersonId, what: crate::event::LifeEventKind },
    /// A group in the dressing room has turned on the manager.
    DressingRoom { club: ClubId, leader: PlayerId },
    /// An agent is sounding out clubs for a client.
    Exploring { player: PlayerId, agent: PersonId },
}

impl InfoKind {
    /// The club whose confidentiality this breaks, if any.
    pub fn club(&self) -> ClubId {
        match *self {
            InfoKind::Interest { club, .. }
            | InfoKind::Bid { club, .. }
            | InfoKind::JobInDanger { club, .. }
            | InfoKind::Discipline { club, .. }
            | InfoKind::ContractTalks { club, .. }
            | InfoKind::DressingRoom { club, .. } => club,
            _ => ClubId::NONE,
        }
    }

    /// The main player concerned, if any.
    pub fn player(&self) -> PlayerId {
        match *self {
            InfoKind::Interest { player, .. }
            | InfoKind::Bid { player, .. }
            | InfoKind::Unhappy { player, .. }
            | InfoKind::Discipline { player, .. }
            | InfoKind::InjuryWorse { player, .. }
            | InfoKind::ContractTalks { player, .. }
            | InfoKind::Exploring { player, .. } => player,
            InfoKind::DressingRoom { leader, .. } => leader,
            _ => PlayerId::NONE,
        }
    }
}

/// How faithfully a holder's version matches the truth.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Fidelity {
    Accurate,
    /// The gist is right; details are missing.
    Partial,
    /// True but blown up.
    Exaggerated,
    /// Was true; no longer is.
    Outdated,
    /// Mangled in the retelling.
    Garbled,
    /// Deliberately shaped to mislead.
    Planted,
}

impl Fidelity {
    /// One more retelling.
    pub fn degrade(self, roll: f32, honest: f32) -> Fidelity {
        match self {
            Fidelity::Planted | Fidelity::Outdated => self,
            Fidelity::Garbled => Fidelity::Garbled,
            _ if roll < 0.05 * (1.5 - honest) => Fidelity::Garbled,
            _ if roll < 0.18 * (1.5 - honest) => {
                if self == Fidelity::Accurate {
                    Fidelity::Partial
                } else {
                    Fidelity::Exaggerated
                }
            }
            _ if roll < 0.28 * (1.5 - honest) => Fidelity::Exaggerated,
            _ => self,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Fidelity::Accurate => "accurate",
            Fidelity::Partial => "partly accurate",
            Fidelity::Exaggerated => "exaggerated",
            Fidelity::Outdated => "out of date",
            Fidelity::Garbled => "garbled",
            Fidelity::Planted => "deliberately misleading",
        }
    }
}

/// How someone came to know.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Learned {
    /// Took part in it.
    Involved,
    /// Saw or heard it happen.
    Witnessed,
    /// Told directly by someone.
    Told { by: PersonId },
    /// Read it in a story.
    Read { story: StoryId },
    /// Official channels (club statement, medical report).
    Official,
    /// Worked it out.
    Inferred,
}

/// Why someone passed it on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Motive {
    /// Reporting to someone they answer to.
    Duty,
    /// Confiding in a friend or partner.
    Confiding,
    /// Idle talk.
    Gossip,
    /// Wanting to look important.
    Ego,
    /// Getting back at the subject.
    Revenge,
    /// An agent working the market for a client.
    AgentStrategy,
    /// A club official shaping the narrative.
    ClubStrategy,
    /// A player shaping their own situation.
    PlayerStrategy,
    /// Simply careless.
    Careless,
    /// Keeping a journalist friend sweet.
    PressFriendship,
    /// Worry for someone they care about.
    Concern,
}

impl Motive {
    pub const fn label(self) -> &'static str {
        match self {
            Motive::Duty => "duty",
            Motive::Confiding => "confiding in someone close",
            Motive::Gossip => "gossip",
            Motive::Ego => "wanting to seem in the know",
            Motive::Revenge => "a grudge",
            Motive::AgentStrategy => "an agent's strategy",
            Motive::ClubStrategy => "the club's strategy",
            Motive::PlayerStrategy => "the player's own strategy",
            Motive::Careless => "carelessness",
            Motive::PressFriendship => "friendship with a journalist",
            Motive::Concern => "concern",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Knower {
    pub person: PersonId,
    pub how: Learned,
    pub date: Date,
    pub fidelity: Fidelity,
    /// How sure they are, 0–100.
    pub confidence: u8,
    /// How many people they have told.
    pub told: u8,
}

/// One act of telling (bounded log, for audits and inboxes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tell {
    pub info: u32,
    pub from: PersonId,
    pub to: PersonId,
    pub date: Date,
    pub fidelity: Fidelity,
    pub motive: Motive,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoItem {
    pub id: u32,
    pub kind: InfoKind,
    /// The event it is ultimately about (`NONE` if none).
    pub event: EventId,
    pub date: Date,
    /// How damaging or private, 0–100: raises secrecy for the loyal and
    /// appetite for everyone else.
    pub sensitivity: u8,
    /// Whether it is still true (a bid withdrawn, an injury healed).
    pub true_now: bool,
    pub holders: SmallVec<[Knower; 6]>,
    /// The first story built on it.
    pub published: Option<StoryId>,
    /// The people it concerns have realised it got out.
    pub leak_noticed: bool,
    /// No longer travelling (old, public, or irrelevant).
    pub closed: bool,
}

impl crate::window::Keyed for InfoItem {
    fn key(&self) -> u32 {
        self.id
    }
}

impl InfoItem {
    pub fn knower(&self, p: PersonId) -> Option<&Knower> {
        self.holders.iter().find(|k| k.person == p)
    }

    pub fn knows(&self, p: PersonId) -> bool {
        self.knower(p).is_some()
    }
}

fn forgotten_item() -> &'static InfoItem {
    static ITEM: std::sync::OnceLock<InfoItem> = std::sync::OnceLock::new();
    ITEM.get_or_init(|| InfoItem {
        id: u32::MAX,
        kind: InfoKind::Incident { incident: u32::MAX },
        event: EventId::NONE,
        date: Date(0),
        sensitivity: 0,
        true_now: false,
        holders: SmallVec::new(),
        published: None,
        leak_noticed: true,
        closed: true,
    })
}

/// Holders per item are capped: beyond this, an item is effectively an open
/// secret and the press will have it.
pub const MAX_HOLDERS: usize = 40;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Grapevine {
    /// Addressed by item id. Old items are forgotten (`forget_before`); a forgotten id has no row.
    pub items: crate::window::Window<InfoItem>,
    /// Items still travelling.
    pub active: Vec<u32>,
    /// Recent items per person (bounded).
    pub by_person: FxHashMap<PersonId, SmallVec<[u32; 8]>>,
    /// Recent tellings (bounded ring).
    pub tells: Vec<Tell>,
    /// Last event id absorbed into information items.
    pub absorbed: EventId,
}

impl Grapevine {
    pub fn create(&mut self, kind: InfoKind, event: EventId, date: Date, sensitivity: u8) -> u32 {
        let id = self.items.len() as u32;
        self.items.push(InfoItem { id, kind, event, date, sensitivity, true_now: true, holders: SmallVec::new(), published: None, leak_noticed: false, closed: false });
        self.active.push(id);
        id
    }

    /// Someone learns it. Returns false if they already knew (a second
    /// account may raise their confidence).
    pub fn learn(&mut self, info: u32, k: Knower) -> bool {
        let item = &mut self.items[info as usize];
        if let Some(old) = item.holders.iter_mut().find(|x| x.person == k.person) {
            old.confidence = old.confidence.max(k.confidence).saturating_add(5).min(100);
            return false;
        }
        if item.holders.len() >= MAX_HOLDERS {
            return false;
        }
        item.holders.push(k);
        let list = self.by_person.entry(k.person).or_default();
        list.push(info);
        if list.len() > 16 {
            list.remove(0);
        }
        true
    }

    /// Forget every item created before `date`, and everything that points at one. An id that was forgotten has no row: readers use
    /// `items.get`. Items are created in date order, so this drops a prefix.
    pub fn forget_before(&mut self, date: Date) -> usize {
        let before = self.items.base();
        let base = self.items.forget_front_while(|it| it.date < date);
        if base == before {
            return 0;
        }
        self.active.retain(|&i| i >= base);
        self.by_person.retain(|_, ids| {
            ids.retain(|i| *i >= base);
            !ids.is_empty()
        });
        self.tells.retain(|t| t.info >= base);
        (base - before) as usize
    }

    pub fn record_tell(&mut self, t: Tell) {
        self.tells.push(t);
        if self.tells.len() > 20_000 {
            self.tells.drain(..5_000);
        }
    }

    /// The item with this id. An item the grapevine has forgotten reads as an item nobody knows and nobody is telling.
    pub fn get(&self, id: u32) -> &InfoItem {
        match self.items.get(id as usize) {
            Some(it) => it,
            None if self.items.forgotten(id as usize) => forgotten_item(),
            None => panic!("information item {id} was never created"),
        }
    }

    pub fn known_by(&self, p: PersonId) -> impl Iterator<Item = &InfoItem> + '_ {
        self.by_person.get(&p).into_iter().flatten().map(|&i| &self.items[i as usize])
    }

    /// The chain of tellings that brought `info` to `person` (newest last).
    pub fn chain(&self, info: u32, person: PersonId) -> Vec<Tell> {
        let mut out = Vec::new();
        let mut who = person;
        for _ in 0..12 {
            let Some(t) = self.tells.iter().rev().find(|t| t.info == info && t.to == who) else { break };
            out.push(*t);
            who = t.from;
        }
        out.reverse();
        out
    }
}
