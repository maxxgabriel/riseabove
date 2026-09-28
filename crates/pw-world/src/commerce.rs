//! Brands and sponsorship (11 §6). Brands are actors with budgets, sectors
//! and a sensitivity to scandal. They sponsor clubs (shirts, stadiums) and
//! individuals (boots, watches, cars, drinks), with obligations, image-rights
//! splits, exclusivity that can collide with a club's own partners, and
//! morality clauses that end deals when image collapses.

use pw_core::{ClubId, Date, Money, NationId, PersonId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Sector {
    Sportswear,
    Drinks,
    Betting,
    Automotive,
    Finance,
    Fashion,
    Technology,
    Airline,
    Food,
}

impl Sector {
    pub const ALL: [Sector; 9] = [
        Sector::Sportswear,
        Sector::Drinks,
        Sector::Betting,
        Sector::Automotive,
        Sector::Finance,
        Sector::Fashion,
        Sector::Technology,
        Sector::Airline,
        Sector::Food,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Sector::Sportswear => "sportswear",
            Sector::Drinks => "drinks",
            Sector::Betting => "betting",
            Sector::Automotive => "cars",
            Sector::Finance => "finance",
            Sector::Fashion => "fashion",
            Sector::Technology => "technology",
            Sector::Airline => "an airline",
            Sector::Food => "food",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Brand {
    pub name: String,
    pub nation: NationId,
    pub sector: Sector,
    /// 1–20: local shop … global giant.
    pub size: u8,
    /// Yearly marketing budget.
    pub budget: Money,
    /// Committed this year.
    pub committed: Money,
    /// How quickly scandal ends deals, 1–20.
    pub sensitivity: u8,
    /// Wants young faces (true) or established stars.
    pub youthful: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DealEnd {
    Expired,
    Scandal,
    Conflict,
    Retired,
    Faded,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Endorsement {
    pub brand: u32,
    pub person: PersonId,
    pub fee_year: Money,
    pub start: Date,
    pub end: Date,
    /// Appearance days owed per month.
    pub days: u8,
    /// Image-rights share taken by the club, percent.
    pub club_share: u8,
    pub ended: Option<(Date, DealEnd)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ClubSlot {
    Shirt,
    Kit,
    Stadium,
    Sleeve,
}

impl ClubSlot {
    pub const fn label(self) -> &'static str {
        match self {
            ClubSlot::Shirt => "shirt",
            ClubSlot::Kit => "kit",
            ClubSlot::Stadium => "stadium naming",
            ClubSlot::Sleeve => "sleeve",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ClubDeal {
    pub brand: u32,
    pub club: ClubId,
    pub slot: ClubSlot,
    pub fee_year: Money,
    pub start: Date,
    pub end: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Commerce {
    pub brands: Vec<Brand>,
    pub endorsements: Vec<Endorsement>,
    pub club_deals: Vec<ClubDeal>,
    /// Active endorsement indices by person.
    pub by_person: FxHashMap<PersonId, Vec<u32>>,
}

impl Commerce {
    pub fn active_for(&self, p: PersonId) -> impl Iterator<Item = &Endorsement> + '_ {
        self.by_person.get(&p).into_iter().flatten().map(|&i| &self.endorsements[i as usize]).filter(|e| e.ended.is_none())
    }

    pub fn club_partner(&self, club: ClubId, sector: Sector) -> Option<&ClubDeal> {
        self.club_deals.iter().find(|d| d.club == club && self.brands[d.brand as usize].sector == sector)
    }
}
