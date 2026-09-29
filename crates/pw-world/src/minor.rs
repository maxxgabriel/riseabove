//! Schools, universities and minor competitions (T in the media brief; 18
//! in the final brief).
//!
//! Football history is not only professional. Children play for their
//! school as well as their grassroots club; some adults play university
//! football on the way to a career in or out of the game; amateur sides
//! play in a local pyramid. Each has competitions with tables, cups,
//! winners, top scorers and a best player, and each keeps its history —
//! so a professional's biography can say where they first scored, and a
//! university can remember the international it produced.
//!
//! These games are not simulated by the match engine (there are hundreds of
//! thousands of them); a cheap result model draws scores from team strength
//! and credits goals to real members weighted by who they are. Everything
//! is keyed on the world seed.

use pw_core::{Date, LocalClubId, NationId, PersonId, PlayerId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum InstKind {
    School,
    University,
}

/// A school or university and its football team.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Institution {
    pub id: u32,
    pub kind: InstKind,
    pub name: String,
    pub nation: NationId,
    pub city: String,
    /// Year founded (generated for the world, or imported).
    pub founded: i32,
    /// Academic and sporting standing, 0–1000.
    pub prestige: u16,
    /// Quality of the team's coaching, 1–20.
    pub coaching: u8,
    /// Current players (school age groups or enrolled students).
    pub members: Vec<PlayerId>,
    /// Former members who became professionals (for "produced" lists).
    pub alumni_pros: SmallVec<[PersonId; 4]>,
}

/// A side in a minor competition.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Entrant {
    Inst(u32),
    Local(LocalClubId),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MinorKind {
    /// Schools in a city or region, a league.
    SchoolLeague,
    /// All schools in a nation, a knockout.
    SchoolCup,
    UniversityLeague,
    /// The all-India inter-university championship: a knockout of every university side.
    UniversityCup,
    /// Adult amateur pyramid, tier 1 at the top.
    AmateurLeague {
        tier: u8,
    },
    /// Grassroots clubs' knockout for the oldest age group.
    GrassrootsCup,
}

impl MinorKind {
    /// A stable code (record scopes, title counts).
    pub const fn code(self) -> u8 {
        match self {
            MinorKind::SchoolLeague => 1,
            MinorKind::SchoolCup => 2,
            MinorKind::UniversityLeague => 3,
            MinorKind::GrassrootsCup => 4,
            MinorKind::UniversityCup => 5,
            MinorKind::AmateurLeague { tier } => 10u8.saturating_add(tier),
        }
    }

    pub fn from_code(c: u8) -> MinorKind {
        match c {
            1 => MinorKind::SchoolLeague,
            2 => MinorKind::SchoolCup,
            3 => MinorKind::UniversityLeague,
            4 => MinorKind::GrassrootsCup,
            5 => MinorKind::UniversityCup,
            t => MinorKind::AmateurLeague { tier: t.saturating_sub(10) },
        }
    }

    pub fn is_cup(self) -> bool {
        matches!(self, MinorKind::SchoolCup | MinorKind::GrassrootsCup | MinorKind::UniversityCup)
    }

    /// The history level this competition belongs to.
    pub fn level(self) -> crate::minor::Level {
        match self {
            MinorKind::SchoolLeague | MinorKind::SchoolCup => Level::School,
            MinorKind::UniversityLeague | MinorKind::UniversityCup => Level::University,
            MinorKind::AmateurLeague { .. } => Level::Amateur,
            MinorKind::GrassrootsCup => Level::Grassroots,
        }
    }
}

/// Levels of the game history is kept for (professional levels live in
/// `honours`; this names the rest, and the record engine uses all of them).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Level {
    Grassroots,
    School,
    University,
    Amateur,
    Youth,
    Professional,
    International,
    /// State and regional representative football.
    State,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Row {
    pub p: u16,
    pub w: u16,
    pub d: u16,
    pub l: u16,
    pub gf: u16,
    pub ga: u16,
    pub pts: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinorComp {
    pub id: u32,
    pub kind: MinorKind,
    pub nation: NationId,
    /// City or region name for local leagues (empty for national ones).
    pub region: String,
    pub season: i32,
    pub entrants: Vec<Entrant>,
    /// League rounds (circle method) or, for cups, the current survivors.
    pub rounds: Vec<SmallVec<[(Entrant, Entrant); 8]>>,
    pub next_round: u16,
    pub table: Vec<(Entrant, Row)>,
    pub alive: Vec<Entrant>,
    pub done: bool,
}

/// A player's season at one level.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MinorLine {
    pub season: i32,
    pub kind: MinorKind,
    pub entrant: Entrant,
    pub apps: u16,
    pub goals: u16,
    /// Sum of match ratings ×10 (for best-player choices).
    pub rating: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Biggest {
    pub date: Date,
    pub winner: Entrant,
    pub loser: Entrant,
    pub score: (u8, u8),
}

/// One competition's finished season, for the history books.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinorSeason {
    pub kind: MinorKind,
    pub nation: NationId,
    pub region: String,
    pub season: i32,
    pub winner: Entrant,
    pub runner_up: Entrant,
    pub top_scorer: PlayerId,
    pub top_goals: u16,
    pub best: PlayerId,
    pub biggest: Option<Biggest>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Minor {
    pub institutions: Vec<Institution>,
    /// Current school or university of a player.
    pub member_of: FxHashMap<PlayerId, u32>,
    /// Year each university student enrolled.
    pub enrolled: FxHashMap<PlayerId, i32>,
    pub comps: Vec<MinorComp>,
    /// This season's lines, (player, comp id) → line.
    pub lines: FxHashMap<(PlayerId, u32), MinorLine>,
    /// Finished seasons' lines per player (bounded).
    pub careers: FxHashMap<PlayerId, SmallVec<[MinorLine; 4]>>,
    pub history: Vec<MinorSeason>,
    /// Biggest win of the season per comp id.
    pub biggest: FxHashMap<u32, Biggest>,
    pub season: i32,
}

impl Minor {
    pub fn join(&mut self, p: PlayerId, inst: u32) {
        self.leave(p);
        self.institutions[inst as usize].members.push(p);
        self.member_of.insert(p, inst);
    }

    pub fn leave(&mut self, p: PlayerId) {
        if let Some(i) = self.member_of.remove(&p) {
            self.institutions[i as usize].members.retain(|&x| x != p);
        }
    }

    /// A player's minor-football career so far (finished seasons, then this one).
    pub fn career(&self, p: PlayerId) -> Vec<MinorLine> {
        let mut v: Vec<MinorLine> = self.careers.get(&p).map_or_else(Vec::new, |x| x.to_vec());
        v.extend(self.lines.iter().filter(|((q, _), _)| *q == p).map(|(_, l)| *l));
        v.sort_by_key(|l| l.season);
        v
    }
}
