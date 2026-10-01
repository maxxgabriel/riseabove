use serde::{Deserialize, Serialize};

macro_rules! attr_enum {
    ($name:ident, $count:ident, $all:ident { $($variant:ident => $key:literal, $label:literal;)* }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum $name { $($variant),* }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.key())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = <std::borrow::Cow<'de, str>>::deserialize(d)?;
                $name::from_key(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown {} `{s}`", stringify!($name))))
            }
        }

        impl $name {
            pub const $all: [$name; $count] = [$($name::$variant),*];

            #[inline]
            pub const fn idx(self) -> usize {
                self as usize
            }

            pub const fn key(self) -> &'static str {
                match self { $($name::$variant => $key),* }
            }

            pub const fn label(self) -> &'static str {
                match self { $($name::$variant => $label),* }
            }

            pub fn from_key(key: &str) -> Option<Self> {
                match key { $($key => Some($name::$variant),)* _ => None }
            }
        }
    };
}

pub const N_ATTR: usize = 46;
pub const N_HIDDEN: usize = 13;
pub const N_STAFF_ATTR: usize = 18;

attr_enum!(Attr, N_ATTR, ALL {
    Corners => "corners", "Corners";
    Crossing => "crossing", "Crossing";
    Dribbling => "dribbling", "Dribbling";
    Finishing => "finishing", "Finishing";
    FirstTouch => "first_touch", "First Touch";
    FreeKicks => "free_kicks", "Free Kicks";
    Heading => "heading", "Heading";
    LongShots => "long_shots", "Long Shots";
    LongThrows => "long_throws", "Long Throws";
    Marking => "marking", "Marking";
    Passing => "passing", "Passing";
    PenaltyTaking => "penalty_taking", "Penalty Taking";
    Tackling => "tackling", "Tackling";
    Technique => "technique", "Technique";
    Aggression => "aggression", "Aggression";
    Anticipation => "anticipation", "Anticipation";
    Bravery => "bravery", "Bravery";
    Composure => "composure", "Composure";
    Concentration => "concentration", "Concentration";
    Decisions => "decisions", "Decisions";
    Determination => "determination", "Determination";
    Flair => "flair", "Flair";
    Leadership => "leadership", "Leadership";
    OffTheBall => "off_the_ball", "Off the Ball";
    Positioning => "positioning", "Positioning";
    Teamwork => "teamwork", "Teamwork";
    Vision => "vision", "Vision";
    WorkRate => "work_rate", "Work Rate";
    Acceleration => "acceleration", "Acceleration";
    Agility => "agility", "Agility";
    Balance => "balance", "Balance";
    JumpingReach => "jumping_reach", "Jumping Reach";
    NaturalFitness => "natural_fitness", "Natural Fitness";
    Pace => "pace", "Pace";
    Stamina => "stamina", "Stamina";
    Strength => "strength", "Strength";
    AerialReach => "aerial_reach", "Aerial Reach";
    CommandOfArea => "command_of_area", "Command of Area";
    Communication => "communication", "Communication";
    Eccentricity => "eccentricity", "Eccentricity";
    Handling => "handling", "Handling";
    Kicking => "kicking", "Kicking";
    OneOnOnes => "one_on_ones", "One on Ones";
    Reflexes => "reflexes", "Reflexes";
    RushingOut => "rushing_out", "Rushing Out";
    Throwing => "throwing", "Throwing";
});

attr_enum!(Hidden, N_HIDDEN, ALL {
    Consistency => "consistency", "Consistency";
    ImportantMatches => "important_matches", "Important Matches";
    InjuryProneness => "injury_proneness", "Injury Proneness";
    Versatility => "versatility", "Versatility";
    Adaptability => "adaptability", "Adaptability";
    Ambition => "ambition", "Ambition";
    Loyalty => "loyalty", "Loyalty";
    Pressure => "pressure", "Pressure";
    Professionalism => "professionalism", "Professionalism";
    Sportsmanship => "sportsmanship", "Sportsmanship";
    Temperament => "temperament", "Temperament";
    Controversy => "controversy", "Controversy";
    Dirtiness => "dirtiness", "Dirtiness";
});

attr_enum!(StaffAttr, N_STAFF_ATTR, ALL {
    Attacking => "attacking", "Attacking";
    Defending => "defending", "Defending";
    Fitness => "fitness", "Fitness";
    Mental => "mental", "Mental";
    Tactical => "tactical", "Tactical";
    Technical => "technical", "Technical";
    Goalkeeping => "goalkeeping", "Goalkeeping";
    Youngsters => "working_with_youngsters", "Working with Youngsters";
    Motivating => "motivating", "Motivating";
    Discipline => "discipline", "Discipline";
    ManManagement => "man_management", "Man Management";
    JudgingAbility => "judging_ability", "Judging Ability";
    JudgingPotential => "judging_potential", "Judging Potential";
    TacticalKnowledge => "tactical_knowledge", "Tactical Knowledge";
    Physiotherapy => "physiotherapy", "Physiotherapy";
    SportsScience => "sports_science", "Sports Science";
    Negotiating => "negotiating", "Negotiating";
    MediaHandling => "media_handling", "Media Handling";
});

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum AttrGroup {
    Technical,
    Mental,
    Physical,
    Goalkeeping,
}

/// Groups that age differently (04 §2): explosive physical peaks early,
/// mental keeps growing into the thirties.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CurveGroup {
    Speed,
    Power,
    Technical,
    Mental,
    Goalkeeping,
}

pub const N_CURVE_GROUPS: usize = 5;

impl Attr {
    pub const fn group(self) -> AttrGroup {
        let i = self as u8;
        if i <= Attr::Technique as u8 {
            AttrGroup::Technical
        } else if i <= Attr::WorkRate as u8 {
            AttrGroup::Mental
        } else if i <= Attr::Strength as u8 {
            AttrGroup::Physical
        } else {
            AttrGroup::Goalkeeping
        }
    }

    pub const fn curve(self) -> CurveGroup {
        match self {
            Attr::Acceleration | Attr::Agility | Attr::Balance | Attr::Pace => CurveGroup::Speed,
            Attr::JumpingReach | Attr::NaturalFitness | Attr::Stamina | Attr::Strength => CurveGroup::Power,
            _ => match self.group() {
                AttrGroup::Technical => CurveGroup::Technical,
                AttrGroup::Mental => CurveGroup::Mental,
                AttrGroup::Goalkeeping => CurveGroup::Goalkeeping,
                AttrGroup::Physical => CurveGroup::Power,
            },
        }
    }

    pub const fn is_goalkeeping(self) -> bool {
        matches!(self.group(), AttrGroup::Goalkeeping)
    }
}

/// Visible attributes in fixed-point hundredths (100 = 1.00, 2000 = 20.00).
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attrs(#[serde(with = "crate::serde_arr")] pub [u16; N_ATTR]);

impl Attrs {
    pub const MIN: u16 = 100;
    pub const MAX: u16 = 2000;

    pub const fn splat(v: u16) -> Self {
        Self([v; N_ATTR])
    }

    #[inline]
    pub fn get(&self, a: Attr) -> f32 {
        f32::from(self.0[a.idx()]) * 0.01
    }

    #[inline]
    pub fn raw(&self, a: Attr) -> u16 {
        self.0[a.idx()]
    }

    #[inline]
    pub fn set(&mut self, a: Attr, v: f32) {
        self.0[a.idx()] = (v * 100.0).round().clamp(f32::from(Self::MIN), f32::from(Self::MAX)) as u16;
    }

    #[inline]
    pub fn add(&mut self, a: Attr, delta: f32) {
        let v = self.get(a) + delta;
        self.set(a, v);
    }

    /// Whole-number display value, as a player would see it on a report.
    #[inline]
    pub fn display(&self, a: Attr) -> u8 {
        ((self.0[a.idx()] + 50) / 100) as u8
    }

    /// Weighted mean of attributes on the 1–20 scale.
    pub fn weighted(&self, w: &[(Attr, f32)]) -> f32 {
        let (mut s, mut t) = (0.0, 0.0);
        for &(a, wt) in w {
            s += self.get(a) * wt;
            t += wt;
        }
        if t > 0.0 { s / t } else { 0.0 }
    }
}

impl Default for Attrs {
    fn default() -> Self {
        Self::splat(1000)
    }
}

impl std::fmt::Debug for Attrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut m = f.debug_map();
        for a in Attr::ALL {
            m.entry(&a.key(), &self.get(a));
        }
        m.finish()
    }
}

/// Hidden personality/character attributes, 1–20.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HiddenAttrs(pub [u8; N_HIDDEN]);

impl HiddenAttrs {
    #[inline]
    pub fn get(&self, h: Hidden) -> u8 {
        self.0[h.idx()]
    }

    #[inline]
    pub fn f(&self, h: Hidden) -> f32 {
        f32::from(self.0[h.idx()])
    }

    #[inline]
    pub fn set(&mut self, h: Hidden, v: u8) {
        self.0[h.idx()] = v.clamp(1, 20);
    }

    /// Personality label as coaches and agents describe it (derived, never stored).
    pub fn personality_label(&self) -> &'static str {
        let g = |h| self.get(h);
        let (prof, amb, loy, temp, press, sport) = (g(Hidden::Professionalism), g(Hidden::Ambition), g(Hidden::Loyalty), g(Hidden::Temperament), g(Hidden::Pressure), g(Hidden::Sportsmanship));
        match () {
            _ if prof >= 18 && amb >= 15 => "Model Professional",
            _ if prof >= 15 && amb >= 16 => "Driven",
            _ if prof >= 16 => "Professional",
            _ if amb >= 17 && loy <= 7 => "Ambitious",
            _ if loy >= 17 => "Devoted",
            _ if temp <= 5 => "Volatile",
            _ if temp <= 8 && press <= 8 => "Temperamental",
            _ if press >= 16 => "Resolute",
            _ if sport >= 17 => "Honest",
            _ if prof <= 5 => "Casual",
            _ if amb <= 5 => "Low Ambition",
            _ if loy <= 5 => "Mercenary",
            _ => "Balanced",
        }
    }
}

impl Default for HiddenAttrs {
    fn default() -> Self {
        Self([10; N_HIDDEN])
    }
}

/// Staff/coaching attributes, 1–20.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StaffAttrs(pub [u8; N_STAFF_ATTR]);

impl StaffAttrs {
    #[inline]
    pub fn get(&self, a: StaffAttr) -> u8 {
        self.0[a.idx()]
    }

    #[inline]
    pub fn f(&self, a: StaffAttr) -> f32 {
        f32::from(self.0[a.idx()])
    }

    #[inline]
    pub fn set(&mut self, a: StaffAttr, v: u8) {
        self.0[a.idx()] = v.clamp(1, 20);
    }
}

impl Default for StaffAttrs {
    fn default() -> Self {
        Self([8; N_STAFF_ATTR])
    }
}
