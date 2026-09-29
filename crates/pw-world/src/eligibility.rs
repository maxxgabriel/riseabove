//! Who may play for a representative side, as a judgement rather than a yes or no: the body asked, the rule that decided it, the
//! reason, and the evidence each rule looked at. The rules themselves are data (`Ecosystem::eligibility` for state sides, the
//! nation's `RuleProfile` for national sides), and the exact rule set of a country belongs in that country's pack.

use pw_core::{NationId, RegionId};
use smallvec::SmallVec;

/// The side being asked.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Body {
    State(RegionId),
    National(NationId),
}

/// A named rule. The evidence and the reason refer to these.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Rule {
    /// The age window of the side.
    Age,
    /// Players of the top division are with their clubs.
    TopDivision,
    /// One state a year.
    OneStatePerYear,
    /// Grew up in the place.
    Birth,
    /// Registered with a club based there.
    Club,
    /// At school or university there.
    Institution,
    /// Has lived there long enough.
    Residence,
    /// Holds the nationality.
    Nationality,
    /// A parent has it.
    Parent,
    /// Tied to another nation by competitive senior caps.
    CapTied,
    /// Has declared for another nation.
    Declared,
}

impl Rule {
    pub const fn label(self) -> &'static str {
        match self {
            Rule::Age => "age",
            Rule::TopDivision => "top division",
            Rule::OneStatePerYear => "one state a year",
            Rule::Birth => "born or raised there",
            Rule::Club => "registered with a club there",
            Rule::Institution => "at school or university there",
            Rule::Residence => "long residence",
            Rule::Nationality => "nationality",
            Rule::Parent => "a parent's nationality",
            Rule::CapTied => "competitive caps",
            Rule::Declared => "a declaration",
        }
    }
}

/// What one rule found.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Evidence {
    pub rule: Rule,
    pub holds: bool,
    /// The number behind it where there is one (age, years of residence, caps); 0 otherwise.
    pub detail: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// Eligible on this ground (the first that applied, in the order the rules list them).
    Qualifies(Rule),
    TooYoung,
    TooOld,
    TopDivision,
    /// Already played for another state this year.
    RepresentedAnother,
    /// No ground applies.
    NoGround,
    /// Tied to another nation.
    CapTied,
    /// Declared for another nation.
    Declared,
}

impl Reason {
    pub fn text(self) -> String {
        match self {
            Reason::Qualifies(r) => format!("qualifies through {}", r.label()),
            Reason::TooYoung => "too young for the side".into(),
            Reason::TooOld => "too old for the side".into(),
            Reason::TopDivision => "with a top-division club, not available".into(),
            Reason::RepresentedAnother => "has already played for another state this year".into(),
            Reason::NoGround => "no rule gives a claim to play for the side".into(),
            Reason::CapTied => "tied to another nation by competitive caps".into(),
            Reason::Declared => "has declared for another nation".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Judgement {
    pub body: Body,
    pub eligible: bool,
    pub reason: Reason,
    pub evidence: SmallVec<[Evidence; 6]>,
}

impl Judgement {
    pub fn deny(body: Body, reason: Reason, evidence: SmallVec<[Evidence; 6]>) -> Self {
        Self { body, eligible: false, reason, evidence }
    }

    pub fn allow(body: Body, rule: Rule, evidence: SmallVec<[Evidence; 6]>) -> Self {
        Self { body, eligible: true, reason: Reason::Qualifies(rule), evidence }
    }
}
