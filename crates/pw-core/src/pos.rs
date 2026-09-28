use serde::{Deserialize, Serialize};

use crate::attr::Attr;

pub const N_POS: usize = 14;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
#[repr(u8)]
pub enum Pos {
    GK,
    DR,
    DC,
    DL,
    WBR,
    WBL,
    DM,
    MR,
    MC,
    ML,
    AMR,
    AMC,
    AML,
    ST,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum PosGroup {
    Gk,
    Def,
    Mid,
    Att,
}

pub const N_POS_GROUPS: usize = 4;

impl PosGroup {
    pub const ALL: [PosGroup; N_POS_GROUPS] = [PosGroup::Gk, PosGroup::Def, PosGroup::Mid, PosGroup::Att];

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            PosGroup::Gk => "Goalkeepers",
            PosGroup::Def => "Defenders",
            PosGroup::Mid => "Midfielders",
            PosGroup::Att => "Forwards",
        }
    }
}

impl Pos {
    pub const ALL: [Pos; N_POS] = [
        Pos::GK,
        Pos::DR,
        Pos::DC,
        Pos::DL,
        Pos::WBR,
        Pos::WBL,
        Pos::DM,
        Pos::MR,
        Pos::MC,
        Pos::ML,
        Pos::AMR,
        Pos::AMC,
        Pos::AML,
        Pos::ST,
    ];

    #[inline]
    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn group(self) -> PosGroup {
        match self {
            Pos::GK => PosGroup::Gk,
            Pos::DR | Pos::DC | Pos::DL | Pos::WBR | Pos::WBL => PosGroup::Def,
            Pos::DM | Pos::MR | Pos::MC | Pos::ML | Pos::AMC => PosGroup::Mid,
            Pos::AMR | Pos::AML | Pos::ST => PosGroup::Att,
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Pos::GK => "GK",
            Pos::DR => "DR",
            Pos::DC => "DC",
            Pos::DL => "DL",
            Pos::WBR => "WBR",
            Pos::WBL => "WBL",
            Pos::DM => "DM",
            Pos::MR => "MR",
            Pos::MC => "MC",
            Pos::ML => "ML",
            Pos::AMR => "AMR",
            Pos::AMC => "AMC",
            Pos::AML => "AML",
            Pos::ST => "ST",
        }
    }

    pub fn from_code(s: &str) -> Option<Pos> {
        Pos::ALL.into_iter().find(|p| p.code() == s)
    }

    /// Nominal location for a team attacking toward x = 1; y = 0 is the left touchline.
    pub const fn base_xy(self) -> (f32, f32) {
        match self {
            Pos::GK => (0.03, 0.5),
            Pos::DR => (0.20, 0.88),
            Pos::DC => (0.17, 0.5),
            Pos::DL => (0.20, 0.12),
            Pos::WBR => (0.32, 0.92),
            Pos::WBL => (0.32, 0.08),
            Pos::DM => (0.33, 0.5),
            Pos::MR => (0.50, 0.88),
            Pos::MC => (0.47, 0.5),
            Pos::ML => (0.50, 0.12),
            Pos::AMR => (0.68, 0.85),
            Pos::AMC => (0.66, 0.5),
            Pos::AML => (0.68, 0.15),
            Pos::ST => (0.80, 0.5),
        }
    }

    pub const fn is_wide(self) -> bool {
        matches!(self, Pos::DR | Pos::DL | Pos::WBR | Pos::WBL | Pos::MR | Pos::ML | Pos::AMR | Pos::AML)
    }

    /// Mirror across the pitch's long axis (used for natural-foot preference).
    pub const fn is_right(self) -> bool {
        matches!(self, Pos::DR | Pos::WBR | Pos::MR | Pos::AMR)
    }

    pub const fn is_left(self) -> bool {
        matches!(self, Pos::DL | Pos::WBL | Pos::ML | Pos::AML)
    }

    /// How close two positions are in required skill set, 0–1. Drives how fast
    /// familiarity transfers and how badly a player plays out of position.
    pub fn similarity(self, other: Pos) -> f32 {
        if self == other {
            return 1.0;
        }
        let (ax, ay) = self.base_xy();
        let (bx, by) = other.base_xy();
        if self == Pos::GK || other == Pos::GK {
            return 0.0;
        }
        let d = ((ax - bx) * (ax - bx) * 2.2 + (ay - by) * (ay - by)).sqrt();
        (1.0 - d * 1.6).clamp(0.0, 0.95)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum Foot {
    Left,
    #[default]
    Right,
    Either,
}

/// Tactical role a player is asked to perform in a slot. Roles shape where a
/// player stands, which actions they prefer, and which attributes matter.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
#[repr(u8)]
pub enum Role {
    Goalkeeper,
    SweeperKeeper,
    CentreBack,
    BallPlayingDefender,
    FullBack,
    WingBack,
    InvertedWingBack,
    Anchor,
    DeepLyingPlaymaker,
    BallWinner,
    CentralMidfielder,
    BoxToBox,
    Mezzala,
    AdvancedPlaymaker,
    WideMidfielder,
    Winger,
    InvertedWinger,
    InsideForward,
    ShadowStriker,
    TargetForward,
    Poacher,
    PressingForward,
    CompleteForward,
    FalseNine,
}

/// How a role behaves in the event-chain engine. Multipliers are around 1.0.
#[derive(Clone, Copy, Debug)]
pub struct RoleProfile {
    /// Extra forward shift in possession (pitch fraction).
    pub push: f32,
    /// Pull toward the touchline in possession (0 keeps nominal y).
    pub width: f32,
    /// Spread of the occupancy distribution.
    pub roam: f32,
    pub risk: f32,
    pub dribble: f32,
    pub shoot: f32,
    pub cross: f32,
    pub hold: f32,
    pub press: f32,
    pub defend: f32,
}

const fn rp(push: f32, width: f32, roam: f32, risk: f32, dribble: f32, shoot: f32, cross: f32, hold: f32, press: f32, defend: f32) -> RoleProfile {
    RoleProfile { push, width, roam, risk, dribble, shoot, cross, hold, press, defend }
}

impl Role {
    pub const ALL: [Role; 24] = [
        Role::Goalkeeper,
        Role::SweeperKeeper,
        Role::CentreBack,
        Role::BallPlayingDefender,
        Role::FullBack,
        Role::WingBack,
        Role::InvertedWingBack,
        Role::Anchor,
        Role::DeepLyingPlaymaker,
        Role::BallWinner,
        Role::CentralMidfielder,
        Role::BoxToBox,
        Role::Mezzala,
        Role::AdvancedPlaymaker,
        Role::WideMidfielder,
        Role::Winger,
        Role::InvertedWinger,
        Role::InsideForward,
        Role::ShadowStriker,
        Role::TargetForward,
        Role::Poacher,
        Role::PressingForward,
        Role::CompleteForward,
        Role::FalseNine,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Role::Goalkeeper => "Goalkeeper",
            Role::SweeperKeeper => "Sweeper Keeper",
            Role::CentreBack => "Centre-Back",
            Role::BallPlayingDefender => "Ball-Playing Defender",
            Role::FullBack => "Full-Back",
            Role::WingBack => "Wing-Back",
            Role::InvertedWingBack => "Inverted Wing-Back",
            Role::Anchor => "Anchor",
            Role::DeepLyingPlaymaker => "Deep-Lying Playmaker",
            Role::BallWinner => "Ball-Winning Midfielder",
            Role::CentralMidfielder => "Central Midfielder",
            Role::BoxToBox => "Box-to-Box Midfielder",
            Role::Mezzala => "Mezzala",
            Role::AdvancedPlaymaker => "Advanced Playmaker",
            Role::WideMidfielder => "Wide Midfielder",
            Role::Winger => "Winger",
            Role::InvertedWinger => "Inverted Winger",
            Role::InsideForward => "Inside Forward",
            Role::ShadowStriker => "Shadow Striker",
            Role::TargetForward => "Target Forward",
            Role::Poacher => "Poacher",
            Role::PressingForward => "Pressing Forward",
            Role::CompleteForward => "Complete Forward",
            Role::FalseNine => "False Nine",
        }
    }

    pub const fn default_for(pos: Pos) -> Role {
        match pos {
            Pos::GK => Role::Goalkeeper,
            Pos::DC => Role::CentreBack,
            Pos::DR | Pos::DL => Role::FullBack,
            Pos::WBR | Pos::WBL => Role::WingBack,
            Pos::DM => Role::Anchor,
            Pos::MC => Role::CentralMidfielder,
            Pos::MR | Pos::ML => Role::WideMidfielder,
            Pos::AMC => Role::AdvancedPlaymaker,
            Pos::AMR | Pos::AML => Role::Winger,
            Pos::ST => Role::CompleteForward,
        }
    }

    pub const fn fits(self, pos: Pos) -> bool {
        use Role::*;
        match pos {
            Pos::GK => matches!(self, Goalkeeper | SweeperKeeper),
            Pos::DC => matches!(self, CentreBack | BallPlayingDefender),
            Pos::DR | Pos::DL => matches!(self, FullBack | WingBack | InvertedWingBack),
            Pos::WBR | Pos::WBL => matches!(self, WingBack | InvertedWingBack | FullBack),
            Pos::DM => matches!(self, Anchor | DeepLyingPlaymaker | BallWinner),
            Pos::MC => matches!(self, CentralMidfielder | BoxToBox | Mezzala | DeepLyingPlaymaker | BallWinner | AdvancedPlaymaker),
            Pos::MR | Pos::ML => matches!(self, WideMidfielder | Winger | InvertedWinger),
            Pos::AMC => matches!(self, AdvancedPlaymaker | ShadowStriker),
            Pos::AMR | Pos::AML => matches!(self, Winger | InvertedWinger | InsideForward),
            Pos::ST => matches!(self, TargetForward | Poacher | PressingForward | CompleteForward | FalseNine),
        }
    }

    pub const fn profile(self) -> RoleProfile {
        use Role::*;
        match self {
            Goalkeeper => rp(0.0, 0.0, 0.03, 0.6, 0.1, 0.0, 0.0, 0.5, 0.0, 1.0),
            SweeperKeeper => rp(0.05, 0.0, 0.05, 0.9, 0.2, 0.0, 0.0, 0.6, 0.2, 1.0),
            CentreBack => rp(0.02, 0.0, 0.06, 0.7, 0.3, 0.2, 0.1, 0.6, 0.8, 1.2),
            BallPlayingDefender => rp(0.04, 0.0, 0.07, 1.2, 0.5, 0.2, 0.1, 0.8, 0.8, 1.1),
            FullBack => rp(0.08, 0.3, 0.09, 0.8, 0.7, 0.3, 0.9, 0.6, 0.9, 1.1),
            WingBack => rp(0.18, 0.6, 0.11, 1.0, 1.0, 0.4, 1.3, 0.6, 1.0, 0.9),
            InvertedWingBack => rp(0.12, -0.4, 0.10, 1.1, 0.8, 0.5, 0.5, 0.9, 1.0, 0.9),
            Anchor => rp(0.0, 0.0, 0.07, 0.6, 0.3, 0.3, 0.1, 0.7, 1.0, 1.3),
            DeepLyingPlaymaker => rp(0.04, 0.0, 0.09, 1.5, 0.5, 0.5, 0.3, 1.0, 0.8, 1.0),
            BallWinner => rp(0.05, 0.0, 0.11, 0.7, 0.5, 0.5, 0.2, 0.6, 1.5, 1.2),
            CentralMidfielder => rp(0.08, 0.0, 0.10, 1.0, 0.7, 0.7, 0.4, 0.8, 1.0, 1.0),
            BoxToBox => rp(0.14, 0.0, 0.13, 1.0, 0.9, 1.1, 0.4, 0.7, 1.2, 1.0),
            Mezzala => rp(0.14, 0.35, 0.12, 1.2, 1.1, 1.1, 0.8, 0.7, 1.0, 0.8),
            AdvancedPlaymaker => rp(0.08, 0.0, 0.12, 1.5, 1.0, 1.0, 0.5, 1.0, 0.8, 0.6),
            WideMidfielder => rp(0.06, 0.5, 0.10, 1.0, 0.9, 0.6, 1.2, 0.7, 1.0, 1.0),
            Winger => rp(0.10, 0.8, 0.10, 1.1, 1.4, 0.8, 1.6, 0.6, 0.8, 0.6),
            InvertedWinger => rp(0.10, -0.2, 0.11, 1.3, 1.3, 1.1, 0.6, 0.7, 0.8, 0.6),
            InsideForward => rp(0.16, -0.4, 0.11, 1.2, 1.3, 1.5, 0.4, 0.6, 0.9, 0.5),
            ShadowStriker => rp(0.18, 0.0, 0.11, 1.1, 1.0, 1.5, 0.2, 0.6, 1.0, 0.5),
            TargetForward => rp(0.04, 0.0, 0.08, 0.8, 0.5, 1.3, 0.2, 1.6, 0.8, 0.4),
            Poacher => rp(0.08, 0.0, 0.06, 0.6, 0.6, 1.7, 0.1, 0.5, 0.6, 0.3),
            PressingForward => rp(0.04, 0.0, 0.10, 0.9, 0.8, 1.2, 0.2, 0.9, 1.7, 0.7),
            CompleteForward => rp(0.04, 0.0, 0.10, 1.1, 1.1, 1.3, 0.3, 1.1, 1.0, 0.5),
            FalseNine => rp(-0.08, 0.0, 0.12, 1.4, 1.2, 1.0, 0.3, 1.2, 1.0, 0.6),
        }
    }

    /// Attributes that define competence in the role (used for role fit).
    pub const fn key_attrs(self) -> &'static [(Attr, f32)] {
        use Attr::*;
        use Role::*;
        match self {
            Goalkeeper => &[(Reflexes, 3.0), (Handling, 2.5), (OneOnOnes, 2.0), (Positioning, 2.0), (AerialReach, 1.5), (CommandOfArea, 1.5), (Concentration, 1.0)],
            SweeperKeeper => &[(Reflexes, 2.5), (RushingOut, 2.5), (Kicking, 1.5), (Passing, 1.5), (OneOnOnes, 2.0), (Anticipation, 1.5), (Handling, 1.5)],
            CentreBack => &[(Marking, 2.5), (Tackling, 2.5), (Positioning, 2.5), (Heading, 2.0), (JumpingReach, 1.5), (Strength, 1.5), (Concentration, 1.0)],
            BallPlayingDefender => &[(Marking, 2.0), (Tackling, 2.0), (Positioning, 2.0), (Passing, 2.0), (Composure, 1.5), (Vision, 1.0), (Heading, 1.5)],
            FullBack => &[(Tackling, 2.0), (Marking, 1.5), (Positioning, 1.5), (Pace, 1.5), (Crossing, 1.5), (Stamina, 1.5), (WorkRate, 1.0)],
            WingBack => &[(Crossing, 2.0), (Pace, 2.0), (Stamina, 2.0), (Dribbling, 1.5), (Tackling, 1.5), (WorkRate, 1.5), (OffTheBall, 1.0)],
            InvertedWingBack => &[(Passing, 2.0), (Tackling, 1.5), (Positioning, 1.5), (Decisions, 1.5), (FirstTouch, 1.5), (Composure, 1.0), (Stamina, 1.0)],
            Anchor => &[(Positioning, 2.5), (Tackling, 2.0), (Marking, 2.0), (Concentration, 1.5), (Anticipation, 1.5), (Decisions, 1.5), (Strength, 1.0)],
            DeepLyingPlaymaker => &[(Passing, 2.5), (Vision, 2.5), (Composure, 2.0), (Decisions, 2.0), (FirstTouch, 1.5), (Technique, 1.5), (Positioning, 1.0)],
            BallWinner => &[(Tackling, 2.5), (Aggression, 2.0), (WorkRate, 2.0), (Stamina, 1.5), (Anticipation, 1.5), (Strength, 1.5), (Bravery, 1.0)],
            CentralMidfielder => &[(Passing, 2.0), (Decisions, 1.5), (FirstTouch, 1.5), (Tackling, 1.5), (Teamwork, 1.5), (Stamina, 1.5), (Vision, 1.0)],
            BoxToBox => &[(Stamina, 2.5), (WorkRate, 2.0), (Passing, 1.5), (Tackling, 1.5), (OffTheBall, 1.5), (Finishing, 1.0), (Teamwork, 1.0)],
            Mezzala => &[(Passing, 1.5), (Dribbling, 2.0), (OffTheBall, 2.0), (Technique, 1.5), (Vision, 1.5), (Acceleration, 1.0), (Decisions, 1.0)],
            AdvancedPlaymaker => &[(Vision, 2.5), (Passing, 2.5), (Technique, 2.0), (FirstTouch, 2.0), (Decisions, 1.5), (Composure, 1.5), (Flair, 1.0)],
            WideMidfielder => &[(Crossing, 2.0), (Passing, 1.5), (Stamina, 1.5), (WorkRate, 1.5), (Tackling, 1.0), (Teamwork, 1.5), (Decisions, 1.0)],
            Winger => &[(Crossing, 2.5), (Dribbling, 2.5), (Pace, 2.0), (Acceleration, 2.0), (Technique, 1.5), (Agility, 1.0), (Flair, 1.0)],
            InvertedWinger => &[(Dribbling, 2.5), (Passing, 1.5), (Technique, 2.0), (Vision, 1.5), (Acceleration, 1.5), (LongShots, 1.0), (Flair, 1.0)],
            InsideForward => &[(Dribbling, 2.5), (Finishing, 2.5), (Acceleration, 2.0), (OffTheBall, 2.0), (Technique, 1.5), (Composure, 1.0), (Pace, 1.0)],
            ShadowStriker => &[(OffTheBall, 2.5), (Finishing, 2.5), (Anticipation, 2.0), (Composure, 1.5), (Dribbling, 1.5), (Acceleration, 1.5), (WorkRate, 1.0)],
            TargetForward => &[(Heading, 2.5), (Strength, 2.5), (JumpingReach, 2.0), (Finishing, 2.0), (Balance, 1.5), (Bravery, 1.0), (FirstTouch, 1.0)],
            Poacher => &[(Finishing, 3.0), (OffTheBall, 2.5), (Anticipation, 2.0), (Composure, 2.0), (Acceleration, 1.5), (FirstTouch, 1.0)],
            PressingForward => &[(WorkRate, 2.5), (Stamina, 2.0), (Aggression, 1.5), (Finishing, 2.0), (Acceleration, 1.5), (Anticipation, 1.5), (Teamwork, 1.0)],
            CompleteForward => &[(Finishing, 2.5), (FirstTouch, 2.0), (Technique, 1.5), (Strength, 1.5), (Heading, 1.5), (Passing, 1.0), (OffTheBall, 2.0), (Composure, 1.0)],
            FalseNine => &[(Passing, 2.0), (Vision, 2.0), (FirstTouch, 2.0), (Technique, 2.0), (Dribbling, 1.5), (Decisions, 1.5), (Finishing, 1.5)],
        }
    }
}
