//! Media, fans and audience-specific reputation (11, S13, S16).
//!
//! Outlets and journalists are actors. A story exists only because something
//! real happened or someone real leaked something they knew; what gets printed
//! is shaped by the journalist's source, the outlet's accuracy and its appetite
//! for sensation. Fans are per-club audiences whose feelings about a person
//! are built from what they saw and read, with the reasons kept.

use pw_core::{ClubId, Date, EventId, Money, NationId, OutletId, PersonId, PlayerId, StoryId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::event::Cause;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum OutletKind {
    National,
    Local,
    Tabloid,
    Broadcaster,
    DataSite,
    FanChannel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outlet {
    pub name: String,
    pub nation: NationId,
    pub kind: OutletKind,
    /// Audience size, 1–20.
    pub reach: u8,
    /// How carefully claims are checked, 1–20.
    pub accuracy: u8,
    /// Appetite for drama and exaggeration, 1–20.
    pub sensationalism: u8,
    /// A club the outlet leans towards (local papers, fan channels).
    pub leaning: ClubId,
    /// Earned credibility, 0–100: rises when rumours come true, falls when not.
    pub credibility: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Journalist {
    pub person: PersonId,
    pub outlet: OutletId,
    /// Clubs this journalist covers closely.
    pub beat: SmallVec<[ClubId; 4]>,
    /// People who tell this journalist things.
    pub sources: SmallVec<[PersonId; 8]>,
    /// Track record, 0–100.
    pub credibility: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum StoryKind {
    /// Club X interested in / preparing a bid for a player.
    TransferRumour,
    /// A completed or collapsed deal.
    TransferNews,
    /// Manager under pressure, sacked or appointed.
    ManagerPressure,
    ManagerChange,
    /// A player unhappy (minutes, contract, manager) — from a leak or a quote.
    Unhappy,
    /// A dressing-room bust-up, fine or disciplinary matter.
    Discipline,
    /// Injury news.
    Injury,
    /// Praise: breakthrough, form, milestone.
    Praise,
    /// Criticism: poor form, costly error, wages vs performance.
    Criticism,
    /// A contract renewal or talks stalling.
    Contract,
    /// Personal life (only what is public: a move, a marriage).
    Personal,
    /// Season-end, title, relegation.
    Season,
    /// Someone said something on the record.
    Interview,
    MatchReport,
    /// A longer piece on a player, from how the media reads them.
    Feature,
    /// A ranked list of young talents.
    WonderkidList,
    SeasonReview,
    /// Looking back: anniversaries, careers.
    Retrospective,
    AwardNews,
    /// Data-led analysis (underrated / overrated).
    Analysis,
    International,
    Milestone,
    /// News that began as information someone leaked.
    Leak,
    /// An incident made public (a row, a postponement, a protest).
    IncidentNews,
    /// A denial of an earlier story.
    Denial,
    /// An outlet correcting its own earlier story.
    Correction,
    /// Fans' reaction as news in itself.
    FanReaction,
}

/// What kind of claim a story makes. Kept explicit so nothing downstream
/// mistakes a rumour for a fact.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ClaimType {
    /// A matter of public record.
    Fact,
    /// Reported with confirmation from sources.
    Report,
    /// Unconfirmed, from sources.
    Rumour,
    /// The writer's (or a speaker's) view.
    Opinion,
    /// Conjecture with little behind it.
    Speculation,
    /// Correcting an earlier story.
    Correction,
    /// Someone denying an earlier story.
    Denial,
}

impl ClaimType {
    pub const fn label(self) -> &'static str {
        match self {
            ClaimType::Fact => "fact",
            ClaimType::Report => "report",
            ClaimType::Rumour => "rumour",
            ClaimType::Opinion => "opinion",
            ClaimType::Speculation => "speculation",
            ClaimType::Correction => "correction",
            ClaimType::Denial => "denial",
        }
    }
}

/// Whether what a story says was so (locked design 2.2). Kept apart from how it is framed, why it was written, why someone gave it to
/// the journalist, and how audiences took it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Truth {
    /// True when published and still true.
    Accurate,
    /// True when published; things changed afterwards. An honest report that events overtook.
    AccurateAtTime,
    /// Every fact holds and the picture is wrong: framing or emphasis does the misleading.
    Misleading,
    /// Shaped by a source with an aim, whether or not the content is literally true.
    Manipulated,
    /// Not so when it was published.
    False,
}

impl Truth {
    pub const fn label(self) -> &'static str {
        match self {
            Truth::Accurate => "accurate",
            Truth::AccurateAtTime => "accurate at the time",
            Truth::Misleading => "misleading",
            Truth::Manipulated => "manipulated",
            Truth::False => "false",
        }
    }
}

/// Why the journalist and outlet ran it this way (locked design 2.4): a matter of the writer, not of the facts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Intent {
    Inform,
    /// To draw an audience.
    Engage,
    /// To punish someone who has crossed them.
    Punish,
    /// To do a favour to a friend, a source or the club they lean towards.
    Favour,
}

/// What the person who gave the journalist the story wanted from it (locked design 2.6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum SourceAim {
    /// Nothing but telling.
    Genuine,
    /// An agent lifting a client's price or profile.
    RaiseValue,
    /// A club official shaping how a matter is read.
    ShapeNarrative,
    /// A player forcing a move or a contract.
    ForcePlayer,
    /// Getting back at someone.
    Damage,
    /// Keeping a journalist friendly.
    Friendship,
}

/// How a story is framed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Angle {
    Straight,
    Crisis,
    Hero,
    Villain,
    Numbers,
    HumanInterest,
    Conflict,
    /// The latest chapter of a running story.
    Saga,
    Nostalgia,
    Loyalty,
}

/// What the journalist did to stand the story up.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Verification {
    pub asked: u8,
    pub confirmed: u8,
    pub denied: u8,
    /// 0–100.
    pub confidence: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Scope {
    Local,
    National,
    International,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Style {
    Broadsheet,
    Tabloid,
    Local,
    Statistical,
    Fan,
    Broadcast,
}

/// An outlet's institutional identity (separate from its name and reach).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OutletProfile {
    pub scope: Scope,
    pub style: Style,
    /// Language it publishes in (as the nation whose language it is).
    pub language: NationId,
    /// 0–100 each.
    pub rumour_appetite: u8,
    pub tactical_depth: u8,
    /// How readily it corrects its own mistakes.
    pub corrections: u8,
    /// Stories a week the editors want.
    pub quota: u8,
    pub published_this_week: u8,
    pub audience: u32,
    /// The club it is close to (fan channels, local papers).
    pub affinity: ClubId,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Focus {
    Transfers,
    Tactics,
    Youth,
    HumanInterest,
    Scandal,
    Data,
    Local,
}

/// A journalist's relationship with a source, and what they have learned
/// about how reliable that source is.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SourceTie {
    pub person: PersonId,
    pub since: Date,
    /// 0–100.
    pub strength: u8,
    /// The journalist's estimate, 0–100.
    pub reliability: u8,
    pub hits: u8,
    pub misses: u8,
    pub last_used: Date,
}

/// What kind of story a journalist's record is kept for (credibility is contextual, locked design §2.7).
pub fn topic_of(kind: StoryKind) -> u8 {
    match kind {
        StoryKind::TransferRumour | StoryKind::TransferNews => 0,
        StoryKind::Injury => 1,
        StoryKind::Discipline | StoryKind::Unhappy => 2,
        StoryKind::ManagerPressure | StoryKind::ManagerChange => 3,
        StoryKind::Praise | StoryKind::Criticism => 4,
        _ => 5,
    }
}

/// A journalist's record on one club and one kind of story: what the public saw and what was true when published.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TopicRecord {
    pub club: ClubId,
    pub topic: u8,
    pub public_hits: u8,
    pub public_misses: u8,
    pub hits: u8,
    pub misses: u8,
}

impl TopicRecord {
    fn evidence(&self) -> u16 {
        u16::from(self.public_hits) + u16::from(self.public_misses)
    }
}

const MAX_RECORDS: usize = 24;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JournalistProfile {
    /// 0–100 each.
    pub knowledge: u8,
    pub tactical: u8,
    pub ambition: u8,
    /// Willingness to publish on thin evidence.
    pub risk: u8,
    /// A club they cannot help favouring.
    pub bias: ClubId,
    pub focus: Focus,
    /// Professional standing, 0–10,000.
    pub reputation: u16,
    /// Professional accuracy: claims that were true (`hits`) or false (`misses`) *when they were published*, whatever happened next.
    pub hits: u16,
    pub misses: u16,
    /// What audiences saw: reported things that came to pass, and ones that did not. An honest report can end up here as a miss.
    pub public_hits: u16,
    pub public_misses: u16,
    /// Stories that were true in every fact and misleading in effect, and stories that turned out to have been planted on them.
    pub spin: u16,
    pub fooled: u16,
    /// The same, per club and kind of story: trusted on one club's transfers and not on another's.
    pub ledger: Vec<TopicRecord>,
    /// When they started covering each club on their beat.
    pub beat_since: SmallVec<[(ClubId, Date); 4]>,
    pub ties: SmallVec<[SourceTie; 8]>,
    /// Past and present employers (outlet, from).
    pub employers: SmallVec<[(OutletId, Date); 3]>,
    pub languages: SmallVec<[NationId; 2]>,
}

impl JournalistProfile {
    /// Record how a claim on `club` about `topic` turned out: `honest` (true when published) and `came_true` (what the public saw).
    pub fn record(&mut self, club: ClubId, topic: u8, honest: bool, came_true: bool) {
        let i = match self.ledger.iter().position(|r| r.club == club && r.topic == topic) {
            Some(i) => i,
            None => {
                if self.ledger.len() >= MAX_RECORDS
                    && let Some(weakest) = self.ledger.iter().enumerate().min_by_key(|(_, r)| r.evidence()).map(|(i, _)| i)
                {
                    self.ledger.swap_remove(weakest);
                }
                self.ledger.push(TopicRecord { club, topic, public_hits: 0, public_misses: 0, hits: 0, misses: 0 });
                self.ledger.len() - 1
            }
        };
        let r = &mut self.ledger[i];
        if came_true { r.public_hits = r.public_hits.saturating_add(1) } else { r.public_misses = r.public_misses.saturating_add(1) }
        if honest { r.hits = r.hits.saturating_add(1) } else { r.misses = r.misses.saturating_add(1) }
    }

    /// How far the public has reason to trust this journalist on this club and topic: 0..1 and how much evidence stands behind it.
    /// `None` when nothing is on record.
    pub fn public_trust(&self, club: ClubId, topic: u8) -> Option<(f32, u16)> {
        let r = self.ledger.iter().find(|r| r.club == club && r.topic == topic)?;
        let n = r.evidence();
        Some(((f32::from(r.public_hits) + 1.0) / (n as f32 + 2.0), n))
    }

    pub fn tie(&self, p: PersonId) -> Option<&SourceTie> {
        self.ties.iter().find(|t| t.person == p)
    }

    /// Years on the beat at a club.
    pub fn tenure(&self, c: ClubId, today: Date) -> f32 {
        self.beat_since.iter().find(|x| x.0 == c).map_or(0.0, |x| x.1.days_until(today).max(0) as f32 / 365.0)
    }
}

/// What a running story is about.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ThreadSubject {
    Transfer { player: PlayerId, club: ClubId },
    Injury { player: PlayerId },
    Incident { incident: u32 },
    ManagerPressure { club: ClubId },
    Unrest { club: ClubId },
    Contract { player: PlayerId, club: ClubId },
    Leak { info: u32 },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ThreadState {
    Open,
    /// What was reported came to pass.
    Happened,
    /// It fell through.
    Collapsed,
    /// Denied, and nothing came of it.
    Denied,
    /// Went quiet.
    Faded,
}

/// A story that runs over days or months, with its history.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoryThread {
    pub id: u32,
    pub subject: ThreadSubject,
    pub opened: Date,
    pub last: Date,
    pub stories: Vec<StoryId>,
    pub events: SmallVec<[EventId; 4]>,
    pub state: ThreadState,
    pub closed: Option<Date>,
}

/// What a quoted person said, as a stance (the words come from narration).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Stance {
    Praise,
    Criticise,
    /// Deflect, say nothing of substance.
    Deflect,
    /// Talk up ambitions (hints at wanting a bigger stage).
    Ambition,
    /// Declare commitment to the club.
    Loyalty,
    /// Complain about minutes, role or treatment.
    Complain,
    /// Back a teammate or the manager publicly.
    Support,
    /// Deny a story.
    Deny,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Quote {
    pub speaker: PersonId,
    pub about: PersonId,
    pub stance: Stance,
}

/// Extra structured content a story refers to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StoryLink {
    Quote(Quote),
    Fixture { uid: u64, home: ClubId, away: ClubId, hg: u8, ag: u8, star: PlayerId },
    List(Vec<PlayerId>),
    Reading(crate::perf::Label),
    Honour(u32),
    Tournament(u32),
    Award(crate::event::AwardKind),
    Milestone(crate::event::MilestoneKind, u16),
    Record(crate::event::RecordKind, i64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Story {
    pub id: StoryId,
    pub date: Date,
    pub outlet: OutletId,
    pub journalist: PersonId,
    pub kind: StoryKind,
    /// Main subject.
    pub player: PlayerId,
    pub person: PersonId,
    pub club: ClubId,
    /// Second club (e.g. the interested club in a rumour).
    pub other_club: ClubId,
    /// Claimed fee, if any.
    pub fee: Money,
    /// How strongly the claim is stated, 0–100 ("monitoring" … "agreed").
    pub claim: u8,
    /// Whether the claim matches the truth at the time (hidden from readers).
    pub grounded: bool,
    /// What the story ultimately rests on.
    pub source: Cause,
    /// The person who talked, if it was a leak.
    pub leaker: PersonId,
    /// Tone toward the subject, -100..=100.
    pub tone: i8,
    pub event: EventId,
    pub claim_type: ClaimType,
    pub angle: Angle,
    /// The information item it rests on (`u32::MAX` if none).
    pub info: u32,
    /// The running story it belongs to (`u32::MAX` if none).
    pub thread: u32,
    pub verification: Verification,
    /// Minute of the day it went out (for ordering within a day).
    pub minute: u16,
    /// How newsworthy the editors judged it, 0–100.
    pub news: u8,
    /// Earlier stories it refers back to.
    pub refs: SmallVec<[StoryId; 2]>,
    /// Whether it was so, why it was written, and what its source wanted, three separate things.
    pub truth: Truth,
    pub intent: Intent,
    pub aim: SourceAim,
}

/// One end of a media relationship: a person (a journalist or the subject of coverage), a club, or an outlet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Party {
    Person(PersonId),
    Club(ClubId),
    Outlet(OutletId),
}

/// Why a relationship moved (locked design 2.15): kept so a feud is never a bare number.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum BondCause {
    FairCoverage,
    /// Harsh, and true.
    HarshButTrue,
    FalseStory,
    MisleadingFraming,
    Praised,
    GaveInterview,
    RefusedInterview,
    Exclusive,
    Leaked,
    /// A confidence kept or broken.
    KeptConfidence,
    BrokeConfidence,
    /// Planted a story on them.
    Fooled,
    Corrected,
    Apologised,
}

impl BondCause {
    pub const fn label(self) -> &'static str {
        match self {
            BondCause::FairCoverage => "fair coverage",
            BondCause::HarshButTrue => "harsh coverage that was true",
            BondCause::FalseStory => "a false story",
            BondCause::MisleadingFraming => "a misleading picture",
            BondCause::Praised => "praise",
            BondCause::GaveInterview => "an interview given",
            BondCause::RefusedInterview => "an interview refused",
            BondCause::Exclusive => "an exclusive",
            BondCause::Leaked => "a leak",
            BondCause::KeptConfidence => "a confidence kept",
            BondCause::BrokeConfidence => "a confidence broken",
            BondCause::Fooled => "a story planted on them",
            BondCause::Corrected => "a correction",
            BondCause::Apologised => "an apology",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct BondReason {
    pub cause: BondCause,
    pub date: Date,
    pub story: StoryId,
}

/// How one party sees another in the media's world, in one direction. Respect (professional regard) and warmth (liking) are separate:
/// a journalist can be respected and disliked (locked design 2.20).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MediaBond {
    /// -100..=100.
    pub respect: i8,
    pub warmth: i8,
    /// As a source or as a reporter.
    pub trust: i8,
    /// 0..100: a grievance that outlasts the mood.
    pub grudge: u8,
    pub since: Date,
    pub last: Date,
    pub history: SmallVec<[BondReason; 4]>,
}

impl MediaBond {
    pub fn new(today: Date) -> Self {
        Self { respect: 0, warmth: 0, trust: 0, grudge: 0, since: today, last: today, history: SmallVec::new() }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum FeudCause {
    /// One got the story first.
    Scooped,
    /// One's story contradicted the other's.
    Contradicted,
    /// One was shown wrong, the other was right.
    Discredited,
    /// Went after the other's source.
    PoachedSource,
}

/// Two journalists (and so their outlets) in competition (locked design 2.18, 2.19): it cools with time and flares again when
/// they cover the same ground.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feud {
    pub a: PersonId,
    pub b: PersonId,
    pub heat: u8,
    pub since: Date,
    pub last: Date,
    pub causes: SmallVec<[(FeudCause, Date); 4]>,
}

/// A fanbase's feeling about a person (11 §4), with the reasons it formed.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FanStanding {
    /// -1000..=1000.
    pub score: i16,
    pub reasons: SmallVec<[(FanReason, Date, i16); 4]>,
}

impl FanStanding {
    pub fn label(&self) -> &'static str {
        match self.score {
            600.. => "Idol",
            300..=599 => "Favourite",
            100..=299 => "Liked",
            -99..=99 => "Neutral",
            -299..=-100 => "Criticised",
            -599..=-300 => "Unwanted",
            _ => "Villain",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum FanReason {
    Performances,
    Goals,
    Loyalty,
    LocalLad,
    TransferRequest,
    JoinedRival,
    Wages,
    Interview,
    Trophy,
    Mistakes,
    Effort,
    Leaving,
}

impl FanReason {
    pub const fn label(self) -> &'static str {
        match self {
            FanReason::Performances => "performances",
            FanReason::Goals => "goals",
            FanReason::Loyalty => "loyalty",
            FanReason::LocalLad => "one of their own",
            FanReason::TransferRequest => "transfer request",
            FanReason::JoinedRival => "joined a rival",
            FanReason::Wages => "wages",
            FanReason::Interview => "what they said publicly",
            FanReason::Trophy => "trophies",
            FanReason::Mistakes => "costly mistakes",
            FanReason::Effort => "effort",
            FanReason::Leaving => "the way they left",
        }
    }
}

/// A public reaction (fans on social media, pundits) to a real event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reaction {
    pub date: Date,
    /// Whose fans; `NONE` = neutral public.
    pub club: ClubId,
    pub about: PersonId,
    /// -100..=100.
    pub sentiment: i8,
    pub reason: FanReason,
    pub event: EventId,
    /// Volume, 1–20 (how many people are talking).
    pub volume: u8,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Media {
    pub outlets: pw_core::IdVec<OutletId, Outlet>,
    pub journalists: FxHashMap<PersonId, Journalist>,
    pub stories: pw_core::IdVec<StoryId, Story>,
    pub reactions: Vec<Reaction>,
    /// Fanbase feelings, keyed by (club, person).
    pub fans: FxHashMap<(ClubId, PersonId), FanStanding>,
    /// Public image per person, -1000..=1000 (conduct, charity, controversies).
    pub image: FxHashMap<PersonId, i16>,
    /// Insider (professional) reputation among managers, agents and scouts.
    pub insider: FxHashMap<PersonId, i16>,
    /// Club rivalries (both directions stored), 0–100 intensity.
    pub rivals: FxHashMap<(ClubId, ClubId), u8>,
    /// Structured content behind stories (quotes, lists, match facts).
    pub links: FxHashMap<StoryId, StoryLink>,
    pub outlet_profiles: FxHashMap<OutletId, OutletProfile>,
    pub journalist_profiles: FxHashMap<PersonId, JournalistProfile>,
    pub threads: Vec<StoryThread>,
    pub thread_index: FxHashMap<ThreadSubject, u32>,
    /// Directional relationships between people, clubs and outlets, with the reasons they moved.
    pub bonds: FxHashMap<(Party, Party), MediaBond>,
    pub feuds: Vec<Feud>,
}

impl Media {
    pub fn fan(&self, club: ClubId, p: PersonId) -> Option<&FanStanding> {
        self.fans.get(&(club, p))
    }

    /// Shift a fanbase's feeling about someone, recording why.
    pub fn move_fans(&mut self, club: ClubId, p: PersonId, by: i16, reason: FanReason, date: Date) {
        if club.is_none() || by == 0 {
            return;
        }
        let f = self.fans.entry((club, p)).or_default();
        f.score = (f.score + by).clamp(-1000, 1000);
        if let Some(r) = f.reasons.iter_mut().find(|r| r.0 == reason) {
            r.1 = date;
            r.2 = (r.2 + by).clamp(-1000, 1000);
        } else {
            f.reasons.push((reason, date, by));
            if f.reasons.len() > 6
                && let Some(i) = f.reasons.iter().enumerate().min_by_key(|(_, r)| r.2.unsigned_abs()).map(|(i, _)| i)
            {
                f.reasons.remove(i);
            }
        }
    }

    /// The open (or most recent) thread about a subject.
    pub fn thread(&self, s: ThreadSubject) -> Option<&StoryThread> {
        self.thread_index.get(&s).map(|&i| &self.threads[i as usize])
    }

    pub fn rivalry(&self, a: ClubId, b: ClubId) -> u8 {
        self.rivals.get(&(a, b)).copied().unwrap_or(0)
    }

    pub fn nudge_image(&mut self, p: PersonId, by: i16) {
        let e = self.image.entry(p).or_insert(0);
        *e = (*e + by).clamp(-1000, 1000);
    }

    pub fn nudge_insider(&mut self, p: PersonId, by: i16) {
        let e = self.insider.entry(p).or_insert(0);
        *e = (*e + by).clamp(-1000, 1000);
    }

    /// Keep the recent reactions only; standing already carries the memory.
    pub fn compact(&mut self, before: Date) {
        self.reactions.retain(|r| r.date >= before);
    }
}
