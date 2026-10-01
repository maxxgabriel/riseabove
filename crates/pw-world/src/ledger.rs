//! Where the money of an inhabited life went, month by month: the payslip (wage and other income, tax, the cost of living, what was
//! sent home, what was left) and each bonus as it was paid. Recorded by the systems that move the money (`pw_sim::life` monthly,
//! `pw_sim::clauses` for bonuses), only for people with a chronicle; the view reads it to make a wage a life.

use pw_core::{Date, Money, PersonId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

/// Lines kept per person (about ten years of payslips and bonuses).
pub const KEEP: usize = 600;

/// What a bonus was for.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Paid {
    /// A senior match: an appearance, with goals, assists and a clean sheet if any.
    Match { goals: u8, assists: u8, clean: bool },
    Cap,
    Continental,
    Title,
    Promotion,
    Loyalty,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Entry {
    /// A month: gross wage, other income (endorsements, work), tax on both, take-home, living costs, money sent to family.
    Payslip { wage: Money, other: Money, tax: Money, net: Money, living: Money, family: Money },
    /// A bonus: what the club paid and what reached the player after tax and the agent's share.
    Bonus { paid: Paid, gross: Money, kept: Money },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Line {
    pub date: Date,
    pub entry: Entry,
}

/// Owned by `pw_sim::life` (payslips) and `pw_sim::clauses` (bonuses).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Ledgers {
    pub of: FxHashMap<PersonId, Vec<Line>>,
}

impl Ledgers {
    pub fn record(&mut self, who: PersonId, date: Date, entry: Entry) {
        let v = self.of.entry(who).or_default();
        v.push(Line { date, entry });
        if v.len() > KEEP {
            v.remove(0);
        }
    }
}
