//! Dressing rooms (07 §6). A squad is not a list: it has a hierarchy of
//! influence, groups that form around language, age, time together and
//! liking, newcomers who have or have not settled, and a collective mood
//! toward the manager that leaders carry and spread.

use pw_core::{ClubId, Date, PlayerId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Standing {
    /// The voice of the dressing room.
    Leader,
    Influential,
    Established,
    Peripheral,
    /// Not yet part of it.
    Newcomer,
}

impl Standing {
    pub const fn label(self) -> &'static str {
        match self {
            Standing::Leader => "dressing-room leader",
            Standing::Influential => "influential",
            Standing::Established => "established",
            Standing::Peripheral => "on the fringes",
            Standing::Newcomer => "still settling",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub members: SmallVec<[PlayerId; 8]>,
    pub leader: PlayerId,
    /// What binds them, for text (e.g. a shared language or generation).
    pub bond: Bond,
    /// How tight the group is, 0–100.
    pub cohesion: u8,
    /// Average trust in the manager, 0–100.
    pub stance: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Bond {
    Nationality(pw_core::NationId),
    Generation,
    Veterans,
    Academy,
    Friendship,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Room {
    pub club: ClubId,
    pub groups: Vec<Group>,
    pub standing: FxHashMap<PlayerId, Standing>,
    /// Influence score 0–100 per player.
    pub influence: FxHashMap<PlayerId, u8>,
    /// Settling-in progress for recent arrivals, 0–100.
    pub integration: FxHashMap<PlayerId, u8>,
    /// Overall harmony, 0–100.
    pub harmony: u8,
    /// Collective trust in the manager, 0–100.
    pub backing: u8,
    pub updated: Date,
}

impl Room {
    pub fn group_of(&self, p: PlayerId) -> Option<&Group> {
        self.groups.iter().find(|g| g.members.contains(&p))
    }

    pub fn leaders(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.standing.iter().filter(|(_, s)| **s == Standing::Leader).map(|(p, _)| *p)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Rooms {
    pub clubs: FxHashMap<ClubId, Room>,
}
