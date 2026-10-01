//! Group chats and private messages around the people a human inhabits: the squad's group, the university side's group, the family
//! group, the partner, the agent and teammates one to one.
//!
//! A message exists because something happened and someone had a reason to say something about it: a teammate after a win, the
//! captain after you were left out, your mother after a call-up. It stores who spoke and what they reacted to, never prose; the view
//! words it. Nobody's decisions read these: they are what the person hears, not a channel the simulation runs on.

use pw_core::{ClubId, Date, EventId, NationId, PersonId, RegionId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

/// Messages kept per room (older ones scroll away).
pub const ROOM_KEEP: usize = 240;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Room {
    /// The squad's group chat at a club.
    Squad { club: ClubId },
    /// A school or university side's group.
    Team { inst: u32 },
    /// Parents and siblings.
    Family,
    /// One to one.
    Direct { with: PersonId },
}

/// Who spoke. Parents and siblings are part of a household, not people of their own in the world.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Sender {
    Person(PersonId),
    Mother,
    Father,
    Sibling,
    /// The owner of the chats.
    You,
}

/// What the message is a reaction to.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum About {
    CallUp { nation: NationId },
    Capped { nation: NationId },
    StateSide { state: RegionId },
    Debut,
    FirstGoal,
    Contract { club: ClubId },
    Moved { club: ClubId },
    Scholarship { inst: u32 },
    Award,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Said {
    /// The squad after a match: result from the club's side (1 win, 0 draw, -1 loss), the scorer they single out, a derby.
    AfterMatch { uid: u64, result: i8, gf: u8, ga: u8, scored: PersonId, derby: bool },
    /// To you after you made the difference.
    WellDone { uid: u64 },
    Congrats { about: About },
    Proud { about: About },
    GetWell { days: u16 },
    /// After being released or left out for a while.
    HeadUp,
    /// The captain or a friend checking on you quietly.
    CheckIn,
    Welcome { who: PersonId },
    Farewell { who: PersonId },
    Birthday { who: PersonId },
    /// Family missing you when you play far from home.
    Missing,
    /// Your agent passing on interest, or saying there is none yet.
    AgentNews { club: ClubId },
    /// News about the club (appended).
    ClubNews { news: crate::chronicle::ClubNews },
    /// A teammate married or became a parent (appended).
    LifeNews { who: PersonId, child: bool },
    // ---- appended after layout 8
    /// The manager, privately: could you play, before the medical room has cleared you?
    AskedIfReady,
    /// A teammate after your first match back from an injury.
    GoodToHaveYouBack { uid: u64 },
    /// The old squad's chat when one of its former members does something big (a first cap, a manager's job).
    OldTeamNews { who: PersonId, then: crate::chronicle::Then },
    // ---- appended after layout 9
    /// Someone who heard you were recognised in public: an autograph asked for, a photo at the airport.
    Spotted { spot: crate::chronicle::Spot },
    /// Your partner deciding about a move abroad: coming with you, or staying behind.
    PartnerMove { coming: bool },
    /// The family after a move far from home (`abroad`, or to another state): someone coming for the first weeks to help you
    /// settle, or nobody able to.
    FamilyMove { coming: bool, abroad: bool },
    /// The family after your match, watched from home `ahead` hours ahead of where you play (negative: behind); `result` from your
    /// side.
    WatchedFromHome { uid: u64, result: i8, ahead: i8 },
    /// The family missing you across a time difference of `ahead` hours (home ahead of where you live; negative: behind).
    MissingFar { ahead: i8 },
}

/// Where the family is: the parents' nation, or the person's own when that is not known.
pub fn family_nation(w: &crate::World, who: PersonId) -> NationId {
    let own = w.people.get(who).map_or(NationId::NONE, |x| x.nation);
    w.lives.get(who).map(|l| l.household.parents.nation).filter(|n| n.is_some()).unwrap_or(own)
}

/// How many hours home (where the family is, `family_nation`) is ahead of where the person lives now; negative when behind.
/// `None` when either place is unknown or both keep the same hours. The second value says whether both clocks are real data
/// (`Environment::known`) rather than inferred.
pub fn home_ahead(w: &crate::World, who: PersonId) -> Option<(i8, bool)> {
    let home = family_nation(w, who);
    let here = w.lives.get(who).map(|l| l.home).filter(|n| n.is_some())?;
    if home.is_none() || here == home {
        return None;
    }
    let (a, b) = (&w.nations.get(home)?.env, &w.nations.get(here)?.env);
    let ahead = a.tz - b.tz;
    (ahead != 0).then_some((ahead, a.known && b.known))
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ChatMsg {
    pub date: Date,
    pub from: Sender,
    pub said: Said,
    /// The event it reacts to, if one (`EventId::NONE` otherwise).
    pub event: EventId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chat {
    pub room: Room,
    pub opened: Date,
    pub msgs: Vec<ChatMsg>,
    /// Messages read up to this count (the owner's view of the room).
    pub read: u32,
    /// Total ever posted, so `read` survives older messages scrolling away.
    pub posted: u32,
}

impl Chat {
    pub fn unread(&self) -> u32 {
        self.posted.saturating_sub(self.read)
    }
}

/// One person's chats and what has been read from the world for them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Inbox {
    pub chats: Vec<Chat>,
    pub cursor: EventId,
    /// Last day the captain or a friend checked in, so a quiet word stays rare.
    pub checked_in: Date,
    /// Times left out of the squad so far this year, as last seen.
    pub omitted_seen: u16,
}

impl Inbox {
    pub fn room(&mut self, room: Room, today: Date) -> usize {
        if let Some(i) = self.chats.iter().position(|c| c.room == room) {
            return i;
        }
        self.chats.push(Chat { room, opened: today, msgs: Vec::new(), read: 0, posted: 0 });
        self.chats.len() - 1
    }

    pub fn post(&mut self, room: Room, today: Date, from: Sender, said: Said, event: EventId) {
        let i = self.room(room, today);
        let c = &mut self.chats[i];
        if c.msgs.iter().rev().take(12).any(|m| m.from == from && m.said == said) {
            return;
        }
        c.msgs.push(ChatMsg { date: today, from, said, event });
        c.posted += 1;
        if c.msgs.len() > ROOM_KEEP {
            c.msgs.remove(0);
        }
    }
}

/// Owned by `pw_sim::chat`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Chats {
    pub of: FxHashMap<PersonId, Inbox>,
}
