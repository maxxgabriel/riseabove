use pw_core::attr::AttrGroup;
use pw_core::{Attr, Attrs, ClubId, Date, IdVec, Money, N_POS, PersonId, PlayerId, PlayerTraits, Pos, TeamId};
use pw_data::{N_BODY_REGIONS, PositionWeights};
use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Loan, SquadStatus};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum PlayerStatus {
    #[default]
    Active,
    FreeAgent,
    Retired,
    /// Playing outside professional registration: grassroots children, and
    /// adults in amateur and semi-professional football.
    Amateur,
}

/// Fields touched every simulated day. Kept small and `Copy` so daily passes
/// over hundreds of thousands of players stay cache-friendly.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerHot {
    /// Club holding the registration (parent club during a loan).
    pub club: ClubId,
    /// Team the player currently trains and plays with.
    pub team: TeamId,
    pub status: PlayerStatus,
    pub condition: u8,
    pub sharpness: u8,
    pub fitness: u8,
    pub fatigue: u8,
    pub morale: u8,
    pub confidence: u8,
    /// Life well-being (statistical model for AI, life sim for external minds).
    pub wellbeing: u8,
    /// Catalogue index + 1; 0 = fit.
    pub injury: u16,
    pub injury_days: u16,
    pub injury_total: u16,
    /// Matches remaining on suspension.
    pub ban: u8,
    pub yellows: u8,
    /// Acute and chronic training load (EWMA, arbitrary units).
    pub acute: f32,
    pub chronic: f32,
    /// Last five match ratings ×10, newest first; 0 = no data.
    pub form: [u8; 5],
    /// Rolling training rating ×10.
    pub training: u8,
    /// Minutes over roughly the last four weeks (decayed daily).
    pub minutes_4w: u16,
    /// Minutes since the last weekly social pass (promises count these).
    pub minutes_week: u16,
    pub last_match: Date,
}

impl Default for PlayerHot {
    fn default() -> Self {
        Self {
            club: ClubId::NONE,
            team: TeamId::NONE,
            status: PlayerStatus::FreeAgent,
            condition: 100,
            sharpness: 60,
            fitness: 80,
            fatigue: 0,
            morale: 60,
            confidence: 60,
            wellbeing: 65,
            injury: 0,
            injury_days: 0,
            injury_total: 0,
            ban: 0,
            yellows: 0,
            acute: 300.0,
            chronic: 300.0,
            form: [0; 5],
            training: 65,
            minutes_4w: 0,
            minutes_week: 0,
            last_match: Date(i32::MIN / 2),
        }
    }
}

impl PlayerHot {
    #[inline]
    pub fn is_injured(&self) -> bool {
        self.injury != 0
    }

    #[inline]
    pub fn available(&self) -> bool {
        self.injury == 0 && self.ban == 0 && self.status == PlayerStatus::Active
    }

    /// Rolling average of recent ratings, or `None` without matches.
    pub fn form_avg(&self) -> Option<f32> {
        let (s, n) = self.form.iter().filter(|&&r| r > 0).fold((0u32, 0u32), |(s, n), &r| (s + u32::from(r), n + 1));
        (n > 0).then(|| s as f32 / n as f32 / 10.0)
    }

    pub fn push_rating(&mut self, rating: f32) {
        self.form.rotate_right(1);
        self.form[0] = (rating * 10.0).round().clamp(10.0, 100.0) as u8;
    }

    /// Acute:chronic workload ratio (05 §1).
    #[inline]
    pub fn acwr(&self) -> f32 {
        self.acute / self.chronic.max(50.0)
    }
}

/// How hard a player trains (04 §3): development, load and injury risk scale with it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum Intensity {
    Light,
    #[default]
    Normal,
    High,
}

impl Intensity {
    pub const fn load_mult(self) -> f32 {
        match self {
            Intensity::Light => 0.75,
            Intensity::Normal => 1.0,
            Intensity::High => 1.25,
        }
    }

    pub const fn growth_mult(self) -> f32 {
        match self {
            Intensity::Light => 0.85,
            Intensity::Normal => 1.0,
            Intensity::High => 1.12,
        }
    }
}

/// Individual training emphasis (04 §3.2): gains concentrate where the focus is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum Focus {
    #[default]
    General,
    Group(AttrGroup),
    Attribute(Attr),
    /// Learning a new position (familiarity grows).
    Position(Pos),
}

impl Focus {
    /// Development multiplier for an attribute under this focus.
    pub fn weight(&self, a: Attr) -> f32 {
        match *self {
            Focus::General | Focus::Position(_) => 1.0,
            Focus::Group(g) => {
                if a.group() == g {
                    1.35
                } else {
                    0.9
                }
            }
            Focus::Attribute(x) => {
                if a == x {
                    1.8
                } else {
                    0.92
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct TrainingPlan {
    pub focus: Focus,
    pub intensity: Intensity,
    /// Extra individual sessions per week (0–3).
    pub extra: u8,
    /// Dedicated recovery sessions per week (0–3).
    pub recovery: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Reputation {
    pub current: u16,
    pub home: u16,
    pub world: u16,
}

/// Fields read by weekly/monthly systems and views.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerCold {
    pub person: PersonId,
    pub attrs: Attrs,
    /// Potential ability ceiling, 1–200. Never shown; estimated by observers.
    pub pa: u8,
    /// Cached current ability in the best familiar position.
    pub ca: u8,
    pub familiarity: [u8; N_POS],
    pub best_pos: Pos,
    pub left_foot: u8,
    pub right_foot: u8,
    pub height: u8,
    pub weight: u8,
    pub traits: PlayerTraits,
    /// Biological maturity offset in tenths of a year (+ = late developer).
    pub bio_offset: i8,
    pub pa_rerolled: bool,
    pub wear: [u8; N_BODY_REGIONS],
    pub contract: Contract,
    pub loan: Option<Loan>,
    pub value: Money,
    pub rep: Reputation,
    pub status: SquadStatus,
    pub shirt: u8,
    pub caps: u16,
    pub intl_goals: u16,
    pub joined: Date,
    /// Club where the player spent most youth years (homegrown, solidarity).
    pub youth_club: ClubId,
    pub injuries_career: u16,
    /// Competitive first-team appearances (debut detection, milestones).
    pub senior_apps: u16,
    pub senior_goals: u16,
    pub plan: TrainingPlan,
}

impl PlayerCold {
    /// Ability at a position: weighted attributes × familiarity penalty (17 §1).
    pub fn ability_at(&self, pos: Pos, w: &PositionWeights) -> f32 {
        raw_ability(&self.attrs, pos, w) * familiarity_factor(self.familiarity[pos.idx()])
    }

    /// Recompute cached CA and best position from attributes.
    pub fn refresh_ca(&mut self, w: &PositionWeights) {
        let (mut best, mut best_pos) = (0.0f32, self.best_pos);
        for pos in Pos::ALL {
            if self.familiarity[pos.idx()] < 15 {
                continue;
            }
            let a = raw_ability(&self.attrs, pos, w);
            if a > best {
                best = a;
                best_pos = pos;
            }
        }
        if best == 0.0 {
            best = raw_ability(&self.attrs, self.best_pos, w);
        }
        self.ca = best.round().clamp(1.0, 200.0) as u8;
        self.best_pos = best_pos;
    }

    pub fn natural_positions(&self) -> impl Iterator<Item = Pos> + '_ {
        Pos::ALL.into_iter().filter(|p| self.familiarity[p.idx()] >= 18)
    }

    #[inline]
    pub fn attr(&self, a: Attr) -> f32 {
        self.attrs.get(a)
    }

    /// Weaker foot quality 1–20.
    #[inline]
    pub fn weak_foot(&self) -> u8 {
        self.left_foot.min(self.right_foot)
    }
}

/// Position ability on the CA scale (0–200) ignoring familiarity.
#[inline]
pub fn raw_ability(attrs: &Attrs, pos: Pos, w: &PositionWeights) -> f32 {
    let row = w.row(pos);
    let mut s = 0.0;
    for (i, &wt) in row.iter().enumerate() {
        if wt > 0.0 {
            s += wt * (f32::from(attrs.0[i]) - 100.0) / 1900.0;
        }
    }
    s * 200.0
}

/// Multiplier for playing at a familiarity level (03 §5 bands).
#[inline]
pub fn familiarity_factor(f: u8) -> f32 {
    match f {
        18.. => 1.0,
        15..=17 => 0.96,
        12..=14 => 0.9,
        8..=11 => 0.8,
        5..=7 => 0.68,
        _ => 0.55,
    }
}

pub const FAMILIARITY_LABELS: [(u8, &str); 6] = [(18, "Natural"), (15, "Accomplished"), (12, "Competent"), (8, "Unconvincing"), (5, "Awkward"), (0, "Ineffectual")];

pub fn familiarity_label(f: u8) -> &'static str {
    FAMILIARITY_LABELS.iter().find(|(min, _)| f >= *min).map_or("Ineffectual", |(_, l)| l)
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Players {
    pub hot: IdVec<PlayerId, PlayerHot>,
    pub cold: IdVec<PlayerId, PlayerCold>,
}

impl Players {
    pub fn push(&mut self, hot: PlayerHot, cold: PlayerCold) -> PlayerId {
        let id = self.hot.push(hot);
        let id2 = self.cold.push(cold);
        debug_assert_eq!(id, id2);
        id
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.hot.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.hot.is_empty()
    }

    pub fn ids(&self) -> impl DoubleEndedIterator<Item = PlayerId> + ExactSizeIterator + use<> {
        self.hot.ids()
    }
}
