use pw_core::{ClubId, Date, Money, PersonId, StaffAttr, StaffAttrs};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum StaffRole {
    Manager,
    Assistant,
    Coach,
    GkCoach,
    FitnessCoach,
    Scout,
    Physio,
    SportsScientist,
    HeadOfYouth,
    DirectorOfFootball,
}

impl StaffRole {
    pub const fn label(self) -> &'static str {
        match self {
            StaffRole::Manager => "Manager",
            StaffRole::Assistant => "Assistant Manager",
            StaffRole::Coach => "Coach",
            StaffRole::GkCoach => "Goalkeeping Coach",
            StaffRole::FitnessCoach => "Fitness Coach",
            StaffRole::Scout => "Scout",
            StaffRole::Physio => "Physio",
            StaffRole::SportsScientist => "Sports Scientist",
            StaffRole::HeadOfYouth => "Head of Youth Development",
            StaffRole::DirectorOfFootball => "Director of Football",
        }
    }

    /// Attributes that make someone good in this job.
    pub const fn key_attrs(self) -> &'static [StaffAttr] {
        use StaffAttr::*;
        match self {
            StaffRole::Manager => &[TacticalKnowledge, ManManagement, Motivating, JudgingAbility, Discipline],
            StaffRole::Assistant => &[TacticalKnowledge, ManManagement, JudgingAbility, Tactical],
            StaffRole::Coach => &[Attacking, Defending, Technical, Tactical, Mental],
            StaffRole::GkCoach => &[Goalkeeping, Technical],
            StaffRole::FitnessCoach => &[Fitness, Motivating],
            StaffRole::Scout => &[JudgingAbility, JudgingPotential],
            StaffRole::Physio => &[Physiotherapy],
            StaffRole::SportsScientist => &[SportsScience, Fitness],
            StaffRole::HeadOfYouth => &[Youngsters, JudgingPotential, Technical],
            StaffRole::DirectorOfFootball => &[JudgingAbility, JudgingPotential, Negotiating],
        }
    }
}

/// Manager archetype (17 §4) — selects default selection weights.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum Archetype {
    #[default]
    Pragmatist,
    Developer,
    Rotator,
    Loyalist,
}

/// A coach's football beliefs; drives tactics, selection and training defaults.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Philosophy {
    /// Preferred formations (indices into the data pack), best first.
    pub formations: [u8; 2],
    pub mentality: i8,
    pub press: u8,
    pub tempo: u8,
    pub directness: u8,
    pub youth_trust: u8,
    pub archetype: Archetype,
}

impl Default for Philosophy {
    fn default() -> Self {
        Self { formations: [0, 2], mentality: 0, press: 50, tempo: 50, directness: 50, youth_trust: 50, archetype: Archetype::Pragmatist }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct ManagerRecord {
    pub games: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub trophies: u16,
    pub sackings: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Staff {
    pub person: PersonId,
    pub role: StaffRole,
    pub club: ClubId,
    pub attrs: StaffAttrs,
    pub wage: Money,
    pub contract_end: Date,
    pub reputation: u16,
    pub philosophy: Philosophy,
    pub joined: Date,
    pub record: ManagerRecord,
    pub retired: bool,
}

impl Staff {
    pub fn role_rating(&self, role: StaffRole) -> f32 {
        let keys = role.key_attrs();
        keys.iter().map(|&a| self.attrs.f(a)).sum::<f32>() / keys.len() as f32
    }

    #[inline]
    pub fn employed(&self) -> bool {
        self.club.is_some() && !self.retired
    }
}
