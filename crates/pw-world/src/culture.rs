//! Football culture (11 §7): what a club stands for, what its supporters
//! expect, which fixtures mean more than three points, and the national
//! trends that shape how the game is played and talked about.
//!
//! Rivalries are generic: between clubs, nations or institutions (schools,
//! universities, local clubs). They are seeded from geography and standing,
//! then *grow from history* — every meeting, title race, elimination,
//! transfer across the divide and manager who switches sides is recorded as
//! a moment and moves the intensity. Match meaning is derived from rivalries
//! and the table on demand; nothing about a fixture is stored twice.

use pw_core::{ClubId, Date, EventId, NationId, PersonId, PlayerId, StaffId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// One side of a rivalry.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Side {
    Club(ClubId),
    Nation(NationId),
    /// A school, university or local club (`institutions` registry id).
    Institution(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum RivalryKind {
    /// Same city.
    Derby,
    /// Same region / neighbouring towns.
    Regional,
    /// Built from decades of meaningful meetings.
    Historic,
    /// Fought over the same title.
    TitleRace,
    /// Fought over promotion.
    Promotion,
    /// Fought against relegation together.
    Relegation,
    /// One side knocked the other out.
    CupRevenge,
    /// Players and managers crossing the divide.
    BadBlood,
    /// Schools and universities.
    Institutional,
    International,
}

impl RivalryKind {
    pub const fn label(self) -> &'static str {
        match self {
            RivalryKind::Derby => "derby",
            RivalryKind::Regional => "regional rivalry",
            RivalryKind::Historic => "historic rivalry",
            RivalryKind::TitleRace => "title rivalry",
            RivalryKind::Promotion => "promotion rivalry",
            RivalryKind::Relegation => "relegation rivalry",
            RivalryKind::CupRevenge => "cup grudge",
            RivalryKind::BadBlood => "bad blood",
            RivalryKind::Institutional => "varsity rivalry",
            RivalryKind::International => "international rivalry",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MomentKind {
    /// A meeting: who won (None = draw) and by how much.
    Meeting { winner: Option<Side>, margin: u8 },
    /// One knocked the other out of a cup.
    Elimination { winner: Side },
    /// The title was decided between them.
    TitleDecided { winner: Side },
    /// A player moved directly between them.
    Transfer { player: PlayerId, to: Side },
    /// A manager moved between them.
    ManagerMove { staff: StaffId, to: Side },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Moment {
    pub date: Date,
    pub kind: MomentKind,
    pub event: EventId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rivalry {
    pub a: Side,
    pub b: Side,
    pub kinds: SmallVec<[RivalryKind; 2]>,
    /// 0–100.
    pub intensity: u8,
    pub since: Date,
    /// Wins for a, draws, wins for b.
    pub h2h: (u16, u16, u16),
    pub last_meeting: Date,
    /// The side still smarting from the last elimination.
    pub revenge_due: Option<Side>,
    /// Most memorable moments, newest last (bounded).
    pub moments: SmallVec<[Moment; 6]>,
}

impl Rivalry {
    pub fn involves(&self, s: Side) -> bool {
        self.a == s || self.b == s
    }

    pub fn other(&self, s: Side) -> Side {
        if self.a == s { self.b } else { self.a }
    }

    pub fn has(&self, k: RivalryKind) -> bool {
        self.kinds.contains(&k)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Rivalries {
    pub list: Vec<Rivalry>,
    index: FxHashMap<(Side, Side), u32>,
}

impl Rivalries {
    fn key(a: Side, b: Side) -> (Side, Side) {
        if a <= b { (a, b) } else { (b, a) }
    }

    pub fn get(&self, a: Side, b: Side) -> Option<&Rivalry> {
        self.index.get(&Self::key(a, b)).map(|&i| &self.list[i as usize])
    }

    pub fn get_mut(&mut self, a: Side, b: Side) -> Option<&mut Rivalry> {
        let i = *self.index.get(&Self::key(a, b))?;
        Some(&mut self.list[i as usize])
    }

    pub fn intensity(&self, a: Side, b: Side) -> u8 {
        self.get(a, b).map_or(0, |r| r.intensity)
    }

    /// Create the rivalry if missing; returns it.
    pub fn ensure(&mut self, a: Side, b: Side, kind: RivalryKind, intensity: u8, today: Date) -> &mut Rivalry {
        let k = Self::key(a, b);
        let i = match self.index.get(&k) {
            Some(&i) => i,
            None => {
                self.list.push(Rivalry {
                    a: k.0,
                    b: k.1,
                    kinds: SmallVec::new(),
                    intensity: 0,
                    since: today,
                    h2h: (0, 0, 0),
                    last_meeting: Date(0),
                    revenge_due: None,
                    moments: SmallVec::new(),
                });
                let i = (self.list.len() - 1) as u32;
                self.index.insert(k, i);
                i
            }
        };
        let r = &mut self.list[i as usize];
        if !r.kinds.contains(&kind) {
            r.kinds.push(kind);
        }
        r.intensity = r.intensity.max(intensity);
        r
    }

    pub fn of(&self, s: Side) -> impl Iterator<Item = &Rivalry> + '_ {
        self.list.iter().filter(move |r| r.involves(s))
    }
}

/// What a club stands for, 0–100 each. Seeded, then shaped by what happens.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Identity {
    /// Faith in its own academy.
    pub youth: u8,
    /// Pride in local players.
    pub local: u8,
    /// Attacking, entertaining football is expected.
    pub flair: u8,
    /// Hard work and grit over glamour.
    pub grit: u8,
    /// Small club against the world.
    pub underdog: u8,
    /// Stars, money, glamour.
    pub glamour: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ClubCulture {
    pub identity: Identity,
    /// Tradition of discipline in the dressing room, 0–100.
    pub discipline: u8,
    /// What supporters and board expect, 0–100 (grows with success).
    pub expectations: u8,
    /// How long supporters give a manager, 0–100.
    pub patience: u8,
    /// Us-against-them intensity of the support, 0–100.
    pub tribalism: u8,
    /// Academy graduates who have made the first team (lifetime).
    pub graduates: u16,
    /// Seasons since the last trophy.
    pub drought: u16,
}

/// National football culture and its slowly drifting trends.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct NationCulture {
    /// How much football dominates life and conversation, 0–100.
    pub fervour: u8,
    /// How loud and aggressive the football media are, 0–100.
    pub media_intensity: u8,
    /// Patience with managers and young players, 0–100.
    pub patience: u8,
    /// Trust in homegrown youth, 0–100.
    pub youth_faith: u8,
    /// The fashionable way to play: pressing, tempo, directness (0–100).
    pub trend_press: u8,
    pub trend_tempo: u8,
    pub trend_direct: u8,
    pub trend_since: Date,
}

/// What a fixture means beyond the points (derived, not stored).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MatchMeaning {
    pub rivalry: u8,
    pub derby: bool,
    pub title_race: bool,
    pub relegation: bool,
    pub promotion: bool,
    /// The side seeking revenge for an elimination.
    pub revenge: Option<Side>,
    /// People returning to a former club (players and managers).
    pub returns: SmallVec<[(PersonId, ClubId); 3]>,
    /// Overall significance, 0–100.
    pub significance: u8,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Culture {
    pub clubs: FxHashMap<ClubId, ClubCulture>,
    pub nations: FxHashMap<NationId, NationCulture>,
    pub rivalries: Rivalries,
}

impl Culture {
    pub fn club(&self, c: ClubId) -> ClubCulture {
        self.clubs.get(&c).cloned().unwrap_or_default()
    }

    pub fn nation(&self, n: NationId) -> NationCulture {
        self.nations.get(&n).copied().unwrap_or_default()
    }
}
