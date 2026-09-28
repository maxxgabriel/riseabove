//! Who owns and runs clubs, and the world economy they live in (07 §9–10,
//! 02 §8). Owners have wealth, ambition, patience and a habit (or not) of
//! meddling; boards set policy — wage structure, transfer style, youth
//! investment, debt tolerance, selling stance — and commission facility
//! projects that take years. Clubs can grow rich, fall into debt, enter
//! administration and be bought.

use pw_core::{Date, Money, NationId, PersonId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::club::Ownership;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Owner {
    pub person: PersonId,
    pub kind: Ownership,
    /// Personal/group wealth available to the club over time.
    pub wealth: Money,
    /// 0–100: how much success they demand.
    pub ambition: u8,
    /// 0–100: how long they wait for it.
    pub patience: u8,
    /// 0–100: how much they interfere in football decisions.
    pub meddling: u8,
    /// 0–100: how tight they are with money.
    pub frugality: u8,
    /// 0–100: how much fan opinion sways them.
    pub fan_sensitivity: u8,
    pub since: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum TransferStyle {
    /// Buy young, develop, sell at a profit.
    Develop,
    /// Pay for proven players now.
    WinNow,
    Balanced,
    /// Undervalued players found through data and scouting.
    Value,
    /// Academy first; few signings.
    Homegrown,
}

impl TransferStyle {
    pub const fn label(self) -> &'static str {
        match self {
            TransferStyle::Develop => "buy young and develop",
            TransferStyle::WinNow => "win now",
            TransferStyle::Balanced => "balanced",
            TransferStyle::Value => "find value",
            TransferStyle::Homegrown => "academy first",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Policy {
    /// Highest wage as a multiple of the squad's median wage.
    pub wage_cap_mult: f32,
    /// 0–100 share of spending directed to the academy.
    pub youth_investment: u8,
    pub transfer_style: TransferStyle,
    /// Won't pay fees for players older than this (0 = no limit).
    pub max_signing_age: u8,
    /// Will they sell to a rival?
    pub sell_to_rivals: bool,
    /// Debt as a multiple of annual revenue before austerity.
    pub debt_tolerance: f32,
    /// Style the board wants: −2 defensive … +2 attacking.
    pub style_mandate: i8,
    /// Share of first-team minutes the board wants for academy graduates (percent).
    pub youth_minutes_target: u8,
    /// How readily they sell when a big offer comes (0.6 eager … 1.6 reluctant).
    pub selling_stance: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ProjectKind {
    Training,
    Youth,
    AcademyNetwork,
    Medical,
    Stadium,
}

impl ProjectKind {
    pub const fn label(self) -> &'static str {
        match self {
            ProjectKind::Training => "training ground",
            ProjectKind::Youth => "youth facilities",
            ProjectKind::AcademyNetwork => "academy recruitment network",
            ProjectKind::Medical => "medical centre",
            ProjectKind::Stadium => "stadium expansion",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub kind: ProjectKind,
    /// New facility level, or seats added for a stadium.
    pub target: u32,
    pub cost: Money,
    pub started: Date,
    pub completes: Date,
}

/// What worries or pleases the board right now, with weights (−30..30).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum BoardConcern {
    Results,
    Finances,
    YouthMinutes,
    Style,
    FanUnrest,
    OwnerPatience,
}

impl BoardConcern {
    pub const fn label(self) -> &'static str {
        match self {
            BoardConcern::Results => "results",
            BoardConcern::Finances => "finances",
            BoardConcern::YouthMinutes => "chances for academy players",
            BoardConcern::Style => "style of play",
            BoardConcern::FanUnrest => "fan unrest",
            BoardConcern::OwnerPatience => "the owner's patience",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Governance {
    pub owner: Owner,
    /// The person chairing the board (often the owner; elected at member-owned clubs).
    pub chairman: PersonId,
    pub policy: Policy,
    pub projects: Vec<Project>,
    /// In administration since this date.
    pub administration: Option<Date>,
    /// Consecutive months deep in the red.
    pub red_months: u8,
    pub concerns: SmallVec<[(BoardConcern, i8); 6]>,
    /// Annual revenue for the last few seasons, newest last.
    pub revenue_history: SmallVec<[Money; 5]>,
    /// Money set aside each season for the academy.
    pub academy_budget: Money,
    /// Money injected by the owner over time (for the history books).
    pub injected: Money,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NationEconomy {
    /// Wage level relative to the start of the world (1.0).
    pub wage_index: f32,
    /// Top-flight broadcast pool per season.
    pub broadcast_pool: Money,
    /// Season the current broadcast deal runs to.
    pub deal_until: i32,
    /// Real growth trend, per year.
    pub growth: f32,
    /// Top-league strength 0..1, rolling.
    pub league_strength: f32,
    /// Continental results over recent seasons (coefficient), newest last.
    pub coefficient: SmallVec<[f32; 5]>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Economy {
    /// Football-wide price level (fees, wages) relative to the start of the world.
    pub global_index: f32,
    pub nations: FxHashMap<NationId, NationEconomy>,
    pub last_year: i32,
}

impl Economy {
    pub fn wage_index(&self, n: NationId) -> f32 {
        let g = if self.global_index > 0.0 { self.global_index } else { 1.0 };
        g * self.nations.get(&n).map_or(1.0, |e| e.wage_index)
    }

    pub fn global(&self) -> f32 {
        if self.global_index > 0.0 { self.global_index } else { 1.0 }
    }

    pub fn coefficient(&self, n: NationId) -> f32 {
        self.nations.get(&n).map_or(0.0, |e| e.coefficient.iter().sum())
    }
}
