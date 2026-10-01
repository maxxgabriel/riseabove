//! Who knows a player, who vouches for him, and how the world abroad sees a country's players (the India brief, items 3, 4, 17, 18).
//!
//! Public standing (`ecosystem::Repute`) is what the football world in general can say about a young player. It is not what any one
//! organisation knows. Each club, university, state selection panel or market abroad knows players through its own people, its own
//! coverage and its own recommendations, so the same boy can be well known to one and unheard-of to another.

use pw_core::{ClubId, Date, LocalClubId, PersonId, PlayerId, RegionId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

/// An organisation that knows players through its own scouts and staff.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Org {
    /// A professional club or academy.
    Club(ClubId),
    /// A university or school (`minor::Institution::id`).
    Institution(u32),
    /// A state's selectors.
    State(RegionId),
    /// The national federation's camps and youth sides.
    Federation(pw_core::NationId),
    /// Clubs in a market abroad (index into `Scenario::markets`).
    Market(u8),
}

/// How an organisation first came to know a player.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Learned {
    /// One of its people watched him play.
    Watched,
    /// Someone it trusts recommended him.
    Recommended,
    /// He came to it: a trial, an open day, a selection.
    Came,
    /// It heard the talk (a clip, a result), which only made it look.
    Buzz,
}

/// What one organisation has of one player.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Acquaintance {
    /// Distinct looks by its people.
    pub sightings: u8,
    /// Distinct calendar years someone was watching.
    pub years: u8,
    pub first: Date,
    pub last: Date,
    /// The person who first took him seriously there.
    pub first_by: PersonId,
    pub how: Learned,
}

/// Where a recommendation comes from: the place that knows the child, never a bare yes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Source {
    /// His local club's coaches.
    Club(LocalClubId),
    /// His school's or university's coaches.
    Institution(u32),
    /// His district's selectors.
    District(RegionId),
    /// A named person in the game (a scout or coach who knows him).
    Person(PersonId),
}

/// What the recommender's own knowledge of him rests on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum VouchBasis {
    /// Coached him for this many months.
    Trained { months: u16 },
    /// Watched him in this many of his games.
    Watched { games: u16 },
    /// From a save that predates recommendations: it is known that someone vouched, not who or why.
    Legacy,
}

/// A coach's (or teacher's, selector's) recommendation, with its cause. Made only from a relationship and an observation, and only
/// when the person rates him clearly above the others they coach.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Vouch {
    pub from: Source,
    pub basis: VouchBasis,
    /// How strongly the recommender rates him against those they know, 0-1.
    pub strength: f32,
    /// How far their word counts (their coaching, their record), 0-1.
    pub credibility: f32,
    pub date: Date,
}

/// How past recommendations from a source have turned out for one organisation.
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct Referrals {
    pub hits: u16,
    pub misses: u16,
}

impl Referrals {
    /// -1..1; little evidence pulls towards 0.
    pub fn record(self) -> f32 {
        (f32::from(self.hits) - f32::from(self.misses)) / (f32::from(self.hits) + f32::from(self.misses) + 3.0)
    }
}

/// What a foreign market sees of a country's players, by the kind of football they play.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Segment {
    /// Under-21 and academy football.
    Youth,
    /// Senior national-team football.
    Senior,
    /// Domestic professional league.
    League,
    /// University football (usually invisible abroad).
    University,
}

impl Segment {
    pub const ALL: [Segment; 4] = [Segment::Youth, Segment::Senior, Segment::League, Segment::University];

    pub const fn label(self) -> &'static str {
        match self {
            Segment::Youth => "youth football",
            Segment::Senior => "the senior national side",
            Segment::League => "the domestic league",
            Segment::University => "university football",
        }
    }
}

/// How one market regards one kind of the country's football, 0-100, and how it has been earned.
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct Regard {
    pub level: f32,
    /// Players from here who have joined clubs in this market (drives successes and failures).
    pub exports: u16,
    /// Of those, ones who then played regularly.
    pub successes: u16,
    /// Times the market's clubs have sent someone to watch.
    pub visits: u16,
    /// True when the level was derived from a single legacy number rather than earned in this save.
    pub legacy: bool,
}

/// An organisation has sent someone to look at a player because it heard about him (buzz), and is watching him over several games
/// before any judgement forms.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Watch {
    pub org: Org,
    pub player: PlayerId,
    /// The person sent.
    pub by: PersonId,
    /// Games still to watch.
    pub left: u8,
    pub since: Date,
}

/// Owned by `pw_sim::recognition` and `pw_sim::foreign`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Recog {
    /// Looks that talk has set going and that are not finished. Buzz starts one; only watching makes evidence.
    pub watching: Vec<Watch>,
    /// What each organisation has of each player. Sparse: most pairs have nothing.
    pub acquaint: FxHashMap<(Org, PlayerId), Acquaintance>,
    /// Who vouches for a player (one recommendation, the strongest, per player).
    pub vouch: FxHashMap<PlayerId, Vouch>,
    /// How each organisation's dealings with a source have gone.
    pub referrals: FxHashMap<(Org, Source), Referrals>,
    /// The regard each market has for each kind of the country's football.
    pub export: FxHashMap<(u8, Segment), Regard>,
}

impl Recog {
    pub fn known(&self, org: Org, p: PlayerId) -> Option<&Acquaintance> {
        self.acquaint.get(&(org, p))
    }

    pub fn regard(&self, market: u8, seg: Segment) -> f32 {
        self.export.get(&(market, seg)).map_or(0.0, |r| r.level)
    }
}
