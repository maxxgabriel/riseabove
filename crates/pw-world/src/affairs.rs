//! Personal affairs (10 §3–§6): study and qualifications, where and how a
//! person lives, the people they employ, what they give back, what they
//! invest in, and what they do once playing is over. Kept sparse: only people
//! who have done something beyond the default have an entry.

use pw_core::{AgentId, ClubId, Date, Money, OutletId, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Course {
    CoachingC,
    CoachingB,
    CoachingA,
    CoachingPro,
    SportsScience,
    Journalism,
    Business,
    DataAnalysis,
    Scouting,
    MediaTraining,
}

impl Course {
    pub const fn label(self) -> &'static str {
        match self {
            Course::CoachingC => "C coaching licence",
            Course::CoachingB => "B coaching licence",
            Course::CoachingA => "A coaching licence",
            Course::CoachingPro => "Pro coaching licence",
            Course::SportsScience => "sports science diploma",
            Course::Journalism => "journalism course",
            Course::Business => "business degree",
            Course::DataAnalysis => "data analysis course",
            Course::Scouting => "scouting certificate",
            Course::MediaTraining => "media training",
        }
    }

    /// Study hours-months to complete (hours per week × months).
    pub const fn effort(self) -> u16 {
        match self {
            Course::CoachingC => 40,
            Course::CoachingB => 70,
            Course::CoachingA => 110,
            Course::CoachingPro => 160,
            Course::SportsScience => 150,
            Course::Journalism => 90,
            Course::Business => 180,
            Course::DataAnalysis => 90,
            Course::Scouting => 40,
            Course::MediaTraining => 15,
        }
    }

    /// Course fees.
    pub const fn cost(self) -> Money {
        match self {
            Course::CoachingC => 1_000,
            Course::CoachingB => 3_000,
            Course::CoachingA => 8_000,
            Course::CoachingPro => 20_000,
            Course::SportsScience => 12_000,
            Course::Journalism => 6_000,
            Course::Business => 25_000,
            Course::DataAnalysis => 5_000,
            Course::Scouting => 1_500,
            Course::MediaTraining => 2_000,
        }
    }

    pub const fn requires(self) -> Option<Course> {
        match self {
            Course::CoachingB => Some(Course::CoachingC),
            Course::CoachingA => Some(Course::CoachingB),
            Course::CoachingPro => Some(Course::CoachingA),
            _ => None,
        }
    }

    /// Coaching level this qualification represents (0 = not coaching).
    pub const fn coaching_level(self) -> u8 {
        match self {
            Course::CoachingC => 1,
            Course::CoachingB => 2,
            Course::CoachingA => 3,
            Course::CoachingPro => 4,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Study {
    pub course: Course,
    pub started: Date,
    /// Hours-months done.
    pub done: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum HomeKind {
    /// With parents or host family.
    #[default]
    Family,
    /// Club digs / shared housing.
    Digs,
    Rented,
    Owned,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Home {
    pub kind: HomeKind,
    /// 1–5.
    pub quality: u8,
    /// Property value if owned.
    pub value: Money,
    pub mortgage: Money,
    pub since: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Helper {
    /// Private chef / nutritionist: recovery and body.
    Chef,
    /// Personal trainer / physio: extra fitness work, lower injury hazard.
    Trainer,
    /// Financial adviser: better returns, fewer disasters.
    Adviser,
    /// PR / image management.
    Publicist,
    /// Security: privacy for the famous.
    Security,
    /// Language tutor.
    Tutor,
}

impl Helper {
    pub const ALL: [Helper; 6] = [Helper::Chef, Helper::Trainer, Helper::Adviser, Helper::Publicist, Helper::Security, Helper::Tutor];

    pub const fn label(self) -> &'static str {
        match self {
            Helper::Chef => "private chef",
            Helper::Trainer => "personal trainer",
            Helper::Adviser => "financial adviser",
            Helper::Publicist => "publicist",
            Helper::Security => "security",
            Helper::Tutor => "language tutor",
        }
    }

    /// Monthly cost at quality 10, before cost of living.
    pub const fn base_cost(self) -> Money {
        match self {
            Helper::Chef => 4_000,
            Helper::Trainer => 5_000,
            Helper::Adviser => 3_000,
            Helper::Publicist => 6_000,
            Helper::Security => 8_000,
            Helper::Tutor => 1_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Hired {
    pub helper: Helper,
    /// 1–20.
    pub quality: u8,
    pub cost: Money,
    pub since: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CareerPath {
    Coach,
    Pundit,
    Journalist,
    Agent,
    Analyst,
    Scout,
    Director,
    Ambassador,
    Business,
}

impl CareerPath {
    pub const fn label(self) -> &'static str {
        match self {
            CareerPath::Coach => "coaching",
            CareerPath::Pundit => "punditry",
            CareerPath::Journalist => "journalism",
            CareerPath::Agent => "representing players",
            CareerPath::Analyst => "performance analysis",
            CareerPath::Scout => "scouting",
            CareerPath::Director => "running a football department",
            CareerPath::Ambassador => "a club ambassador role",
            CareerPath::Business => "business",
        }
    }
}

/// Where a post-playing career is being pursued.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Employer {
    None,
    Club(ClubId),
    Outlet(OutletId),
    Agency(AgentId),
    Own,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Work {
    pub path: CareerPath,
    pub employer: Employer,
    pub since: Date,
    /// Monthly income from it.
    pub income: Money,
    /// How well it is going, 0–100.
    pub standing: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Affairs {
    pub quals: SmallVec<[(Course, Date); 3]>,
    pub studying: Option<Study>,
    pub home: Home,
    pub staff: SmallVec<[Hired; 2]>,
    /// Share of income given to causes, percent.
    pub giving_pct: u8,
    /// Hours a month of community work.
    pub community: u8,
    /// Runs a foundation (fixed running cost, larger image and local effects).
    pub foundation: bool,
    /// Invested money and its risk appetite (1–20).
    pub invested: Money,
    pub risk: u8,
    pub work: Option<Work>,
    /// Past post-playing jobs.
    pub past_work: SmallVec<[(CareerPath, Date, Date); 2]>,
}

impl Affairs {
    pub fn has(&self, c: Course) -> bool {
        self.quals.iter().any(|q| q.0 == c)
    }

    pub fn coaching_level(&self) -> u8 {
        self.quals.iter().map(|q| q.0.coaching_level()).max().unwrap_or(0)
    }

    pub fn helper(&self, h: Helper) -> Option<&Hired> {
        self.staff.iter().find(|x| x.helper == h)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AffairsBook {
    pub people: FxHashMap<PersonId, Affairs>,
}

impl AffairsBook {
    pub fn of(&self, p: PersonId) -> Option<&Affairs> {
        self.people.get(&p)
    }

    pub fn entry(&mut self, p: PersonId) -> &mut Affairs {
        self.people.entry(p).or_default()
    }
}
