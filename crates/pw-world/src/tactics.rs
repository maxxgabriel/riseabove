//! Tactical intelligence as history (locked design 7.1-7.34): what staff saw and concluded, what managers changed and why, what worked,
//! who was right, what a club remembers of an opponent, and how well drilled a squad is in each way of playing.
//!
//! Nothing here is an engine parameter. A sign is football evidence; a diagnosis is a belief about it and can be wrong; a response is
//! what a manager chose, and the outcome of a response is judged twice: by what happened to the problem (process) and by what
//! happened on the scoreboard (result). The two disagree often enough that managers learn wrong lessons.

use std::collections::VecDeque;

use pw_core::{ClubId, Date, PlayerId, StaffId, Tactics};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;

/// What can be seen from the touchline.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum SignKind {
    /// They are getting the better of the chances.
    ChancesAgainst,
    /// We are not making any.
    NoChances,
    /// The middle of the pitch is theirs.
    ControlLost,
    /// Our build-up breaks down in our own third.
    BuildUpBroken,
    /// One man keeps being beaten.
    BeatenMan,
    /// Legs are going.
    Fading,
    /// A booked player is at risk of a second card.
    Booked,
    /// Nothing wrong with the football: the scoreline (with the aggregate, the clock and what is at stake) calls for something.
    Scoreline,
}

impl SignKind {
    pub const ALL: [SignKind; 8] =
        [SignKind::ChancesAgainst, SignKind::NoChances, SignKind::ControlLost, SignKind::BuildUpBroken, SignKind::BeatenMan, SignKind::Fading, SignKind::Booked, SignKind::Scoreline];

    pub const fn label(self) -> &'static str {
        match self {
            SignKind::ChancesAgainst => "they keep getting the better chances",
            SignKind::NoChances => "we are not creating anything",
            SignKind::ControlLost => "they control the middle of the pitch",
            SignKind::BuildUpBroken => "our build-up keeps breaking down",
            SignKind::BeatenMan => "one player keeps being beaten",
            SignKind::Fading => "the legs are going",
            SignKind::Booked => "a booked player is a risk",
            SignKind::Scoreline => "the scoreline",
        }
    }
}

/// A belief about why a sign is there. Several can be held at once, with different confidence.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Diagnosis {
    /// The way we are set up leaves too much space.
    TooOpen,
    /// We are too cautious to hurt them.
    TooCautious,
    /// Our press is not working.
    PressNotWorking,
    /// They are simply better today.
    TheyAreBetter,
    /// One man is having a bad day.
    IndividualForm,
    /// Tiredness.
    Fatigue,
    /// It is not going for us; it will turn.
    Unlucky,
    /// A booking that could become a dismissal.
    CardRisk,
    /// The match is slipping away and a goal is needed.
    NeedAGoal,
    /// What the side has is worth guarding.
    ProtectingALead,
}

impl Diagnosis {
    pub const fn label(self) -> &'static str {
        match self {
            Diagnosis::TooOpen => "the team is too open",
            Diagnosis::TooCautious => "the team is too cautious",
            Diagnosis::PressNotWorking => "the press is not working",
            Diagnosis::TheyAreBetter => "they are the better side today",
            Diagnosis::IndividualForm => "a player is having a bad day",
            Diagnosis::Fatigue => "tiredness",
            Diagnosis::Unlucky => "bad luck",
            Diagnosis::CardRisk => "a booking that could become a red",
            Diagnosis::NeedAGoal => "a goal is needed",
            Diagnosis::ProtectingALead => "the lead is worth protecting",
        }
    }
}

pub const N_RESPONSE: usize = 10;

/// What a manager can do about it. Which of these suits which problem is for the manager to judge, and philosophies differ.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Response {
    /// Keep the plan.
    Hold,
    /// Get tighter and more careful.
    Compact,
    /// Push forward and take the risk.
    GoForIt,
    /// Press higher and harder.
    PressHigher,
    /// Sit deeper and wait.
    SitDeep,
    /// Play through the pressure: short, patient.
    PlayThrough,
    /// Go long and fight for the second ball.
    GoDirect,
    /// Take the man off.
    ReplacePlayer,
    /// Bring fresh legs on.
    Refresh,
    /// Move the booked man to a safer job.
    CoverBooked,
}

impl Response {
    pub const ALL: [Response; N_RESPONSE] =
        [Response::Hold, Response::Compact, Response::GoForIt, Response::PressHigher, Response::SitDeep, Response::PlayThrough, Response::GoDirect, Response::ReplacePlayer, Response::Refresh, Response::CoverBooked];

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            Response::Hold => "kept the plan",
            Response::Compact => "tightened up",
            Response::GoForIt => "went for it",
            Response::PressHigher => "pressed higher",
            Response::SitDeep => "dropped deeper",
            Response::PlayThrough => "played through the pressure",
            Response::GoDirect => "went more direct",
            Response::ReplacePlayer => "took a player off",
            Response::Refresh => "brought on fresh legs",
            Response::CoverBooked => "protected a booked player",
        }
    }

    /// Does this change how the team plays as a whole?
    pub const fn is_structural(self) -> bool {
        !matches!(self, Response::Hold | Response::ReplacePlayer | Response::Refresh | Response::CoverBooked)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Verdict {
    Helped,
    NoEffect,
    Worsened,
}

impl Verdict {
    pub const fn label(self) -> &'static str {
        match self {
            Verdict::Helped => "helped",
            Verdict::NoEffect => "changed little",
            Verdict::Worsened => "made it worse",
        }
    }
}

/// One important tactical decision, with enough to explain it afterwards (locked design 7.34).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trace {
    pub uid: u64,
    pub date: Date,
    pub club: ClubId,
    pub manager: StaffId,
    pub minute: u8,
    pub half_time: bool,
    /// Goals for and against when he decided.
    pub score: (u8, u8),
    /// What he saw.
    pub sign: SignKind,
    /// What he thought it meant, how sure he was, and the explanation he rejected.
    pub believed: Diagnosis,
    pub confidence: u8,
    pub rejected: Option<Diagnosis>,
    pub response: Response,
    /// The player at the heart of it, if any.
    pub about: PlayerId,
    /// Minutes of evidence he wanted before acting, and how many he had.
    pub wanted_minutes: u8,
    pub had_minutes: u8,
    /// He saw the danger in the change and went ahead anyway (7.33).
    pub saw_risk: bool,
    pub took_risk: bool,
    /// He was answering something the other side had just changed.
    pub reacting: bool,
    /// How much of the intended change the team could carry out, 0..100.
    pub executed: u8,
    pub backed_by: SmallVec<[StaffId; 2]>,
    pub against: SmallVec<[StaffId; 2]>,
    /// What the change did to the problem, and what the score did after it.
    pub by_process: Option<Verdict>,
    pub by_result: Option<Verdict>,
}

impl Trace {
    /// The lesson he drew and the lesson the evidence supports part company.
    pub fn misread(&self) -> bool {
        matches!((self.by_process, self.by_result), (Some(p), Some(r)) if p != r)
    }
}

/// A manager's tactical record: what he has learned to do and what history says of him (7.27-7.29).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManagerTactics {
    /// What he believes works, per response: -100..100. Moved by results, not always by causes.
    pub lessons: [i8; N_RESPONSE],
    pub changes: u16,
    pub half_time_changes: u16,
    /// Behind at some point after a change and did not lose.
    pub comebacks: u16,
    /// Ahead when he changed or stood pat, and lost.
    pub collapses: u16,
    /// Behind at the hour and did nothing.
    pub stood_pat_behind: u16,
    /// Changes that helped the problem, and changes that only looked as if they did.
    pub good_reads: u16,
    pub misreads: u16,
    pub credited_luck: u16,
}

impl Default for ManagerTactics {
    fn default() -> Self {
        Self { lessons: [0; N_RESPONSE], changes: 0, half_time_changes: 0, comebacks: 0, collapses: 0, stood_pat_behind: 0, good_reads: 0, misreads: 0, credited_luck: 0 }
    }
}

/// How much a manager has found a member of his staff right or wrong when they disagreed (7.13).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Credit {
    pub right: u16,
    pub wrong: u16,
    /// Right and not listened to.
    pub ignored_right: u16,
    /// Wrong and listened to.
    pub heeded_wrong: u16,
}

impl Credit {
    /// 0..1 from the record, 0.5 with nothing to go on.
    pub fn standing(&self) -> f32 {
        let (r, w) = (f32::from(self.right), f32::from(self.wrong));
        (r + 2.0) / (r + w + 4.0)
    }
}

/// What a club's staff remember of one opponent from earlier meetings (7.23-7.24).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OppMemory {
    pub last: Date,
    pub meetings: u8,
    pub press: u8,
    pub tempo: u8,
    pub direct: u8,
    pub formation: u8,
    /// What hurt us last time.
    pub hurt_by: Option<SignKind>,
    pub worked: Option<Response>,
    pub failed: Option<Response>,
    /// Who compiled it. When that person has left, it is trusted less.
    pub author: StaffId,
}

/// The way an opponent was seen to play in one match.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StyleRec {
    pub date: Date,
    pub formation: u8,
    pub press: u8,
    pub tempo: u8,
    pub direct: u8,
    pub mentality: i8,
}

pub const N_STYLES: usize = 6;

/// A way of playing, coarse enough to be trained: the squad is drilled in each.
pub const STYLE_NAMES: [&str; N_STYLES] = ["high press", "attacking", "defensive", "possession", "counter-attacking", "balanced"];

/// The style a set of instructions amounts to.
pub fn style_of(t: &Tactics) -> usize {
    use pw_core::Mentality;
    if t.press >= 70 {
        0
    } else if t.mentality >= Mentality::Positive {
        1
    } else if t.mentality <= Mentality::Defensive {
        2
    } else if t.directness <= 30 && t.tempo <= 50 {
        3
    } else if t.directness >= 70 {
        4
    } else {
        5
    }
}

/// How well a squad knows each way of playing, 0..100 (7.10-7.11). The menu is what has been trained: a change into something the
/// squad has never rehearsed is improvisation, and it shows.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Drill {
    pub manager: StaffId,
    pub fam: [u8; N_STYLES],
    pub since: Date,
}

/// The tactical book of the world.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Tactical {
    /// Recent important decisions, oldest first.
    pub traces: VecDeque<Trace>,
    pub managers: FxHashMap<StaffId, ManagerTactics>,
    /// (manager, staff member) -> record of being right or wrong when they disagreed.
    pub credit: FxHashMap<(StaffId, StaffId), Credit>,
    /// (club, opponent) -> what the club remembers.
    pub memory: FxHashMap<(ClubId, ClubId), OppMemory>,
    /// How each club was seen to play in its last matches, oldest first.
    pub styles: FxHashMap<ClubId, VecDeque<StyleRec>>,
    pub drill: FxHashMap<ClubId, Drill>,
}

impl Tactical {
    pub const TRACE_CAP: usize = 600;

    pub fn keep(&mut self, t: Trace) {
        if self.traces.len() >= Self::TRACE_CAP {
            self.traces.pop_front();
        }
        self.traces.push_back(t);
    }

    pub fn record_of(&self, m: StaffId) -> Option<&ManagerTactics> {
        self.managers.get(&m)
    }

    /// The decisions a club's manager took in one fixture.
    pub fn of_fixture(&self, uid: u64, club: ClubId) -> impl Iterator<Item = &Trace> + '_ {
        self.traces.iter().filter(move |t| t.uid == uid && t.club == club)
    }
}
