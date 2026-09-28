//! Life off the pitch (10, S19). Every person has a `Life`: where they live,
//! the languages they speak, their household (partner, children, parents),
//! how they spend their week, their money, stress, and the reasons behind their
//! mood. One model runs for everyone; the only difference a human makes is who
//! chooses the routine and the lifestyle.
//!
//! Partners are real `Person`s created when a relationship forms. Parents are
//! held compactly (a person's parents only become full records if someone
//! interacts with them), but their state — health, closeness, where they live —
//! evolves for everyone by the same rules (S20).

use pw_core::{Date, Money, NationId, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum Occupation {
    #[default]
    None,
    Student,
    Service,
    Trade,
    Professional,
    Creative,
    Athlete,
    Carer,
}

impl Occupation {
    pub const ALL: [Occupation; 8] = [
        Occupation::None,
        Occupation::Student,
        Occupation::Service,
        Occupation::Trade,
        Occupation::Professional,
        Occupation::Creative,
        Occupation::Athlete,
        Occupation::Carer,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Occupation::None => "not working",
            Occupation::Student => "student",
            Occupation::Service => "works in services",
            Occupation::Trade => "works in a trade",
            Occupation::Professional => "professional career",
            Occupation::Creative => "creative career",
            Occupation::Athlete => "athlete",
            Occupation::Carer => "carer",
        }
    }

    /// How much this career ties someone to where they are (0–20).
    pub const fn rootedness(self) -> u8 {
        match self {
            Occupation::None | Occupation::Carer => 3,
            Occupation::Student => 7,
            Occupation::Service => 5,
            Occupation::Trade => 9,
            Occupation::Creative => 8,
            Occupation::Athlete => 12,
            Occupation::Professional => 15,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PartnerStatus {
    Dating,
    Living,
    Married,
}

impl PartnerStatus {
    pub const fn label(self) -> &'static str {
        match self {
            PartnerStatus::Dating => "dating",
            PartnerStatus::Living => "living together",
            PartnerStatus::Married => "married",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Partner {
    pub person: PersonId,
    pub since: Date,
    pub status: PartnerStatus,
    /// Relationship quality, 0–100.
    pub bond: u8,
    /// Where the partner currently lives (they may stay behind after a move).
    pub lives: NationId,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Parents {
    pub nation: NationId,
    /// 0, 1 or 2 living parents.
    pub alive: u8,
    /// Health of the frailer parent, 0–100.
    pub health: u8,
    /// Emotional closeness, 0–100.
    pub closeness: u8,
    /// How much they push and support the football career, 0–20.
    pub support: u8,
    /// Household income band 1–5 (shapes early career and money worries).
    pub means: u8,
}

impl Default for Parents {
    fn default() -> Self {
        Self { nation: NationId::NONE, alive: 2, health: 85, closeness: 60, support: 10, means: 3 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Household {
    pub partner: Option<Partner>,
    pub children: u8,
    pub youngest_born: Date,
    pub parents: Parents,
    pub siblings: u8,
    /// Former partners (people persist; exes can reappear in stories).
    pub past_partners: SmallVec<[PersonId; 2]>,
}

/// How a person spends a week's free hours (10 §1). Club obligations come
/// first; this is the rest. AI minds set it from personality; a human sets it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Routine {
    pub rest: u8,
    pub recovery: u8,
    pub family: u8,
    pub partner: u8,
    pub social: u8,
    pub study: u8,
    pub hobbies: u8,
    pub media: u8,
    pub nightlife: u8,
    pub language: u8,
}

impl Default for Routine {
    fn default() -> Self {
        Self { rest: 14, recovery: 3, family: 6, partner: 6, social: 6, study: 0, hobbies: 6, media: 1, nightlife: 2, language: 0 }
    }
}

impl Routine {
    /// Free waking hours after club duties, sleep and daily life.
    pub const BUDGET: u32 = 60;

    pub fn total(&self) -> u32 {
        [self.rest, self.recovery, self.family, self.partner, self.social, self.study, self.hobbies, self.media, self.nightlife, self.language]
            .iter()
            .map(|&h| u32::from(h))
            .sum()
    }

    /// Scale down proportionally if it exceeds the budget.
    pub fn normalised(mut self) -> Self {
        let t = self.total();
        if t > Self::BUDGET {
            let k = Self::BUDGET as f32 / t as f32;
            for h in [
                &mut self.rest,
                &mut self.recovery,
                &mut self.family,
                &mut self.partner,
                &mut self.social,
                &mut self.study,
                &mut self.hobbies,
                &mut self.media,
                &mut self.nightlife,
                &mut self.language,
            ] {
                *h = (f32::from(*h) * k).floor() as u8;
            }
        }
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub enum Lifestyle {
    Frugal,
    #[default]
    Modest,
    Comfortable,
    Lavish,
}

impl Lifestyle {
    pub const ALL: [Lifestyle; 4] = [Lifestyle::Frugal, Lifestyle::Modest, Lifestyle::Comfortable, Lifestyle::Lavish];

    /// Share of net income spent at this lifestyle, and a floor in money per month.
    pub const fn spend(self) -> (f32, Money) {
        match self {
            Lifestyle::Frugal => (0.35, 600),
            Lifestyle::Modest => (0.55, 1_000),
            Lifestyle::Comfortable => (0.75, 2_000),
            Lifestyle::Lavish => (1.05, 5_000),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Lifestyle::Frugal => "frugal",
            Lifestyle::Modest => "modest",
            Lifestyle::Comfortable => "comfortable",
            Lifestyle::Lavish => "lavish",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Finances {
    pub savings: Money,
    pub debt: Money,
    /// Last month, after tax.
    pub income: Money,
    pub spending: Money,
    pub lifestyle: Lifestyle,
    /// Money sent to family each month.
    pub family_support: Money,
}

impl Finances {
    pub fn net_worth(&self) -> Money {
        self.savings - self.debt
    }
}

/// Why someone feels the way they do (03 §8). Weekly morale and monthly
/// well-being are sums of these; the list is kept so the "why" survives.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MoodFactor {
    PlayingTime,
    Role,
    Manager,
    Promises,
    TeamResults,
    Wage,
    Settling,
    Partner,
    Family,
    Money,
    Stress,
    Injury,
    Fans,
    Teammates,
    Contract,
    Routine,
    Interest,
}

impl MoodFactor {
    pub const fn label(self) -> &'static str {
        match self {
            MoodFactor::PlayingTime => "playing time",
            MoodFactor::Role => "role in the squad",
            MoodFactor::Manager => "relationship with the manager",
            MoodFactor::Promises => "promises",
            MoodFactor::TeamResults => "team results",
            MoodFactor::Wage => "wages compared with teammates",
            MoodFactor::Settling => "settling in",
            MoodFactor::Partner => "relationship at home",
            MoodFactor::Family => "family",
            MoodFactor::Money => "money",
            MoodFactor::Stress => "stress",
            MoodFactor::Injury => "injury",
            MoodFactor::Fans => "the fans",
            MoodFactor::Teammates => "teammates",
            MoodFactor::Contract => "contract situation",
            MoodFactor::Routine => "balance of the week",
            MoodFactor::Interest => "interest from other clubs",
        }
    }
}

pub type Mood = SmallVec<[(MoodFactor, i8); 8]>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Life {
    /// Nation the person lives in now.
    pub home: NationId,
    pub home_since: Date,
    /// Fluency by nation's language, 0–100.
    pub languages: SmallVec<[(NationId, u8); 3]>,
    pub household: Household,
    pub occupation: Occupation,
    pub routine: Routine,
    pub finances: Finances,
    /// 0–100.
    pub stress: u8,
    /// Sleep quality 0–100.
    pub sleep: u8,
    /// Sense of purpose/fulfilment, 0–100.
    pub fulfilment: u8,
    /// Education level reached (0 none … 5 degree).
    pub education: u8,
    /// Components of current well-being, with reasons.
    pub wellbeing_why: Mood,
    /// Components of current morale, with reasons.
    pub morale_why: Mood,
    /// Consecutive weeks of training well below / above the person's norm.
    pub train_low_weeks: u8,
    pub train_high_weeks: u8,
    /// Consecutive weeks of match form well below the person's norm.
    pub form_low_weeks: u8,
    /// Set once the life has been initialised by the world generator.
    pub ready: bool,
}

impl Default for Life {
    fn default() -> Self {
        Self {
            home: NationId::NONE,
            home_since: Date::default(),
            languages: SmallVec::new(),
            household: Household::default(),
            occupation: Occupation::None,
            routine: Routine::default(),
            finances: Finances::default(),
            stress: 25,
            sleep: 70,
            fulfilment: 60,
            education: 2,
            wellbeing_why: Mood::new(),
            morale_why: Mood::new(),
            train_low_weeks: 0,
            train_high_weeks: 0,
            form_low_weeks: 0,
            ready: false,
        }
    }
}

impl Life {
    pub fn fluency(&self, nation: NationId) -> u8 {
        self.languages.iter().find(|(n, _)| *n == nation).map_or(0, |&(_, f)| f)
    }

    pub fn learn_language(&mut self, nation: NationId, gain: f32) {
        if nation.is_none() {
            return;
        }
        match self.languages.iter_mut().find(|(n, _)| *n == nation) {
            Some((_, f)) => *f = (f32::from(*f) + gain).clamp(0.0, 100.0) as u8,
            None => self.languages.push((nation, gain.clamp(0.0, 100.0) as u8)),
        }
    }

    pub fn partner(&self) -> Option<&Partner> {
        self.household.partner.as_ref()
    }

    pub fn years_here(&self, today: Date) -> f32 {
        self.home_since.days_until(today).max(0) as f32 / 365.0
    }

    /// The single strongest reason behind a mood list.
    pub fn top_reason(mood: &Mood) -> Option<(MoodFactor, i8)> {
        mood.iter().copied().max_by_key(|(_, v)| v.unsigned_abs())
    }
}
