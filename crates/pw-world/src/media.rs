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
            if f.reasons.len() > 6 {
                if let Some(i) = f.reasons.iter().enumerate().min_by_key(|(_, r)| r.2.unsigned_abs()).map(|(i, _)| i) {
                    f.reasons.remove(i);
                }
            }
        }
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
