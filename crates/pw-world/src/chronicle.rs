//! A life as it was lived, for the people a human inhabits: the career chronicle.
//!
//! The event log forgets (`pw_sim::retention`), matches are not events, and a stats table says nothing of why. The chronicle keeps,
//! for each chronicled person, the lines of their own story as they happened, each pointing at what produced it (an event, a
//! match, a story, a person), and the people they crossed paths with, so that what those people do later can come back as part of
//! the story. Lines hold ids, never prose: names and wording are the view's, so a renamed club reads right everywhere.
//!
//! Only what the person could know is written: a club's private reading of them appears when the club tells them (a trial, a
//! signing), dated when it happened and marked with when it became known.

use pw_core::{ClubId, CompId, Date, EventId, NationId, PersonId, RegionId, StoryId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;
use crate::ecosystem::StageKind;
use crate::event::{AwardKind, LifeEventKind, MilestoneKind, RecordKind};
use crate::pathway::Why;
use crate::recog::{Learned, Org};

/// How far a piece of coverage reached, from the person's own home.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Layer {
    /// A local paper or fan channel.
    Local,
    /// The national press and broadcasters of the person's own country.
    National,
    /// Outlets in another country.
    Abroad,
}

impl Layer {
    pub const ALL: [Layer; 3] = [Layer::Local, Layer::National, Layer::Abroad];
}

/// How the person came to a club.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Join {
    Transfer,
    Loan,
    LoanReturn,
    Academy,
    Signed,
}

/// What made a match worth remembering.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Big {
    Final,
    Decisive,
    Derby,
    BigOccasion,
    Brace,
    HatTrick,
    BestOnPitch,
    Winner,
    /// The goal that won a match by one (appended: older chronicles never hold it).
    Decider,
}

/// What a person from the past did next.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Then {
    BecameManager { club: ClubId },
    JoinedStaff { club: ClubId, role: crate::staff::StaffRole },
    Moved { club: ClubId },
    Capped { nation: NationId },
    Honoured { award: AwardKind },
    Retired,
    /// Joined the club the chronicled person plays for.
    JoinedYourClub { club: ClubId },
    /// Now manages the chronicled person.
    ManagesYou { club: ClubId },
}

/// How two people's paths crossed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TieKind {
    Teammate { club: ClubId },
    Classmate { inst: u32 },
    Coach { club: ClubId },
    /// The person who first took the chronicled person seriously at this organisation.
    Scout { org: Org },
    /// The manager of a club when it let the chronicled person go.
    LetGo { club: ClubId },
    Mentor,
    /// The first person outside the family who took them seriously (`ecosystem::PlayerStory::found_by`).
    Finder,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tie {
    pub person: PersonId,
    pub kind: TieKind,
    pub from: Date,
    /// Last day they were together (equal to `from` for a one-off crossing such as a scouting look).
    pub to: Date,
}

impl Tie {
    pub fn days(&self) -> i32 {
        self.from.days_until(self.to)
    }
}

/// One line of the story. Every variant carries the ids the view needs to name and link it.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Line {
    /// Where the story starts: the district grown up in, the first institution, the first person who took them seriously.
    Began { region: RegionId, institution: Option<u32>, finder: PersonId },
    /// A step on the pathway with the reason kept at the time.
    Step { stage: StageKind, why: Why },
    Joined { club: ClubId, how: Join },
    Released { club: ClubId },
    Contract { club: ClubId, first: bool, renewal: bool, until: Date },
    Trial { club: ClubId },
    TrialOutcome { club: ClubId, offered: bool },
    Scholarship { inst: u32, tier: u8, contested: bool },
    Enrolled { inst: u32 },
    Graduated { inst: u32 },
    /// A season of school, university or amateur football (`minor::MinorSeason` at `history`): the person's line in it and how
    /// their side and they finished.
    MinorSeason { history: u32, apps: u16, goals: u16, won: bool, runner_up: bool, top_scorer: bool, best: bool },
    /// School exams.
    Exams { passed: bool },
    /// A calendar year of senior football at one club (`perf::SeasonLine`), written once the year is over.
    Season { year: i32, club: ClubId, apps: u16, starts: u16, goals: u16, assists: u16, rating10: u16, benched: u16, omitted: u16 },
    /// An organisation's first serious look, learned when it said so.
    Noticed { org: Org, by: PersonId, how: Learned },
    Debut { club: ClubId, comp: CompId, uid: u64 },
    FirstGoal { club: ClubId, comp: CompId, uid: u64 },
    Match { uid: u64, club: ClubId, opp: ClubId, comp: CompId, goals: u8, assists: u8, result: i8, big: Big },
    StateSide { state: RegionId },
    NationalSquad { nation: NationId },
    Capped { nation: NationId },
    WithdrewFromSquad { nation: NationId },
    Injury { injury: u16, days: u16 },
    Setback { days: u16 },
    Recovered,
    Honour { award: AwardKind, comp: CompId, season: i32 },
    Title { comp: CompId, club: ClubId, season: i32 },
    Promoted { comp: CompId, club: ClubId },
    Relegated { comp: CompId, club: ClubId },
    Captain { club: ClubId },
    Mentor { mentor: PersonId },
    Milestone { kind: MilestoneKind, count: u16, club: ClubId },
    Record { kind: RecordKind, club: ClubId, value: i64 },
    Breakout,
    /// Coverage: the first at each reach, and the pieces that were about the person in depth.
    Press { story: StoryId, layer: Layer, first: bool },
    Endorsed { brand: u32 },
    Life { kind: LifeEventKind },
    MovedHome { bought: bool },
    Retired,
    /// Someone from the past, and what they did next (`tie` indexes `Life::ties`).
    Meanwhile { who: PersonId, tie: u16, then: Then },
    /// A match against someone from the past.
    Faced { who: PersonId, tie: u16, uid: u64, club: ClubId },
    /// Something that changed the club you were at (appended: older chronicles never hold it).
    AtClub { club: ClubId, news: ClubNews },
    // ---- appended after layout 8: older chronicles never hold these
    /// The first goal for a club after scoring elsewhere first.
    FirstGoalFor { club: ClubId, uid: u64 },
    /// Back training with the group, part of the way back from an injury.
    BackWithGroup,
    /// The manager asked whether you could play before the medical room had cleared you.
    AskedIfReady { by: PersonId },
    /// The first match after an injury of `days`.
    Comeback { uid: u64, club: ClubId, opp: ClubId, days: u16 },
    /// A club the press linked you with that never came for you (`story` is the first link).
    NothingCameOfIt { club: ClubId, story: StoryId },
    /// A story about you that was later corrected by its outlet (`corrected`) or denied (`answer` is that piece).
    Answered { story: StoryId, answer: StoryId, corrected: bool },
    /// The first time people of a wider circle talked about you online (`club` and `nation` are the first speaker's).
    Talked { reach: FanReach, club: ClubId, nation: NationId, region: RegionId },
    /// The terms of a contract written in the same event as its `Contract` line (weekly wage): kept for the scrapbook, since the
    /// contract on the player's record is replaced by the next one. Not a line of its own in the timeline.
    Terms { club: ClubId, until: Date, wage: i64 },
    /// Playing abroad, the language of `nation` reached a new level: 1 getting by, 2 comfortable, 3 fluent.
    Language { nation: NationId, level: u8 },
    /// At a trial's verdict, the club told you its people had not seen you the same way: `keen` rated you higher than `doubtful`.
    TrialViews { club: ClubId, keen: PersonId, doubtful: PersonId },
}

/// How far from home the people talking about you are.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FanReach {
    /// Supporters of your own club.
    OwnClub,
    /// Supporters of another club in your country.
    OtherClub,
    /// People in another state or region of your country.
    OtherState,
    /// People in another country.
    Abroad,
}

/// What changed at a club while you were there.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ClubNews {
    NewManager { who: PersonId },
    ManagerSacked { who: PersonId },
    Takeover { owner: PersonId },
    Administration,
    PointsDeducted { points: u8 },
    Investment,
    Facility { kind: crate::governance::ProjectKind },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub date: Date,
    pub line: Line,
    /// The event it came from, if it came from one (`EventId::NONE` otherwise).
    pub event: EventId,
    /// When the person learned of it, if later than `date`.
    pub learned: Option<Date>,
}

/// One chronicled life.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Life {
    pub since: Date,
    /// The last event read from the log.
    pub cursor: EventId,
    pub entries: Vec<Entry>,
    pub ties: Vec<Tie>,
    /// Reaches already covered once (for "first" press lines).
    pub reached: Vec<Layer>,
    /// Organisations whose first look has been told.
    pub told: Vec<Org>,
    /// Season lines already written, by (year, club).
    pub seasons: Vec<(i32, ClubId)>,
}

impl Life {
    pub fn push(&mut self, date: Date, line: Line, event: EventId) {
        self.entries.push(Entry { date, line, event, learned: None });
    }

    /// Index of the tie of this kind with this person, if any.
    pub fn tie_of(&self, who: PersonId, kind: TieKind) -> Option<usize> {
        self.ties.iter().position(|t| t.person == who && t.kind == kind)
    }

    /// The tie that best describes someone: the longest shared spell, a scout or a coach before a teammate.
    pub fn best_tie(&self, who: PersonId) -> Option<usize> {
        let rank = |k: TieKind| match k {
            TieKind::Scout { .. } | TieKind::LetGo { .. } | TieKind::Mentor | TieKind::Finder => 3,
            TieKind::Coach { .. } => 2,
            TieKind::Classmate { .. } | TieKind::Teammate { .. } => 1,
        };
        self.ties.iter().enumerate().filter(|(_, t)| t.person == who).max_by_key(|(_, t)| (rank(t.kind), t.days())).map(|(i, _)| i)
    }

    /// Whether a line of this exact content already exists (backfill and the daily pass can meet).
    pub fn has(&self, line: &Line) -> bool {
        self.entries.iter().any(|e| &e.line == line)
    }
}

/// The miles of one calendar year at one club: away trips to matches, in km between the home regions on the map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Travel {
    pub year: i32,
    pub club: ClubId,
    pub km: u32,
    pub trips: u16,
}

/// Owned by `pw_sim::chronicle`: the travel of each chronicled person, by year and club (layout 9).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Journeys {
    pub of: FxHashMap<PersonId, Vec<Travel>>,
}

impl Journeys {
    pub fn of(&self, who: PersonId, year: i32, club: ClubId) -> Option<Travel> {
        self.of.get(&who)?.iter().find(|t| t.year == year && t.club == club).copied()
    }
}

/// Owned by `pw_sim::chronicle`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Chronicles {
    pub lives: FxHashMap<PersonId, Life>,
}

impl Chronicles {
    pub fn of(&self, who: PersonId) -> Option<&Life> {
        self.lives.get(&who)
    }
}
