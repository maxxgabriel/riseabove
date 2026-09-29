use pw_core::{Attrs, HiddenAttrs, N_POS, PlayerId, PlayerTraits, Pos, Slot, Tactics, TeamId};
use pw_data::MatchTuning;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Recording level of detail (06 §12). Never changes outcomes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Lod {
    /// Player lines + key events (goals, cards, subs, injuries).
    Standard,
    /// Also commentary-level events and heatmaps.
    Full,
}

#[derive(Clone, Debug)]
pub struct PlayerSheet {
    pub id: PlayerId,
    pub attrs: Attrs,
    pub hidden: HiddenAttrs,
    pub traits: PlayerTraits,
    pub familiarity: [u8; N_POS],
    pub left_foot: u8,
    pub right_foot: u8,
    pub height: u8,
    /// 0–100.
    pub condition: f32,
    pub sharpness: f32,
    pub morale: f32,
    /// Multiplier on injury hazard from the health system (load, wear, proneness).
    pub injury_risk: f32,
}

#[derive(Clone, Debug)]
pub struct TeamSheet {
    pub team: TeamId,
    pub tactics: Tactics,
    pub slots: [Slot; 11],
    /// Starters aligned with `slots`.
    pub xi: [PlayerSheet; 11],
    pub bench: SmallVec<[PlayerSheet; 12]>,
    /// 0–1, how readily the manager changes things in-game.
    pub manager_reactivity: f32,
}

#[derive(Clone, Debug)]
pub struct MatchInput<'a> {
    pub seed: u64,
    pub home: TeamSheet,
    pub away: TeamSheet,
    pub neutral: bool,
    /// Needs a winner today (single-leg knockout or second leg).
    pub decisive: bool,
    /// First-leg goals as (this home team, this away team).
    pub first_leg: Option<(u8, u8)>,
    pub away_goals_rule: bool,
    /// 0–1 (friendly … final).
    pub importance: f32,
    /// ~0.6 lenient … 1.4 strict.
    pub referee_strictness: f32,
    pub max_subs: u8,
    pub lod: Lod,
    pub tuning: &'a MatchTuning,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Ev {
    KickOff,
    HalfTime,
    FullTime,
    ExtraTimeStart,
    Goal,
    OwnGoal,
    PenaltyGoal,
    PenaltyMiss,
    ShotSaved,
    ShotWide,
    ShotBlocked,
    ShotPost,
    Header,
    KeyPass,
    ThroughBall,
    Cross,
    Dribble,
    Tackle,
    Interception,
    Clearance,
    Corner,
    FreeKick,
    Offside,
    Foul,
    Yellow,
    SecondYellow,
    Red,
    Sub,
    Injury,
    Chain,
    ShootoutGoal,
    ShootoutMiss,
    TacticChange,
}

impl Ev {
    /// Kept at every LOD.
    pub const fn is_key(self) -> bool {
        matches!(
            self,
            Ev::KickOff
                | Ev::HalfTime
                | Ev::FullTime
                | Ev::ExtraTimeStart
                | Ev::Goal
                | Ev::OwnGoal
                | Ev::PenaltyGoal
                | Ev::PenaltyMiss
                | Ev::Yellow
                | Ev::SecondYellow
                | Ev::Red
                | Ev::Sub
                | Ev::Injury
                | Ev::ShootoutGoal
                | Ev::ShootoutMiss
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MatchEvent {
    /// Match clock in seconds (extra time continues past 5400).
    pub t: u16,
    pub side: u8,
    pub kind: Ev,
    pub player: PlayerId,
    /// Assister, player fouled, or substitute coming on.
    pub other: PlayerId,
    /// Zone in the acting side's attacking frame.
    pub zone: u8,
    /// xG for shots.
    pub value: f32,
}

impl MatchEvent {
    #[inline]
    pub fn minute(&self) -> u16 {
        self.t / 60 + 1
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct TeamStats {
    pub possession: u8,
    pub shots: u16,
    pub on_target: u16,
    pub xg: f32,
    pub big_chances: u16,
    pub corners: u16,
    pub fouls: u16,
    pub offsides: u16,
    pub passes: u16,
    pub passes_completed: u16,
    pub tackles: u16,
    pub saves: u16,
    pub yellows: u16,
    pub reds: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PlayerLine {
    pub player: PlayerId,
    pub side: u8,
    pub started: bool,
    pub pos: Option<Pos>,
    pub is_keeper: bool,
    pub minutes: u8,
    pub on_at: u8,
    pub off_at: u8,
    pub rating: f32,
    pub goals: u8,
    pub assists: u8,
    pub shots: u8,
    pub on_target: u8,
    pub xg: f32,
    pub xa: f32,
    pub key_passes: u8,
    pub passes: u16,
    pub passes_completed: u16,
    pub progressive_passes: u8,
    pub crosses: u8,
    pub crosses_completed: u8,
    pub dribbles: u8,
    pub dribbles_won: u8,
    pub tackles: u8,
    pub tackles_won: u8,
    pub interceptions: u8,
    pub clearances: u8,
    pub blocks: u8,
    pub aerials_won: u8,
    pub aerials_lost: u8,
    pub fouls: u8,
    pub fouled: u8,
    pub offsides: u8,
    pub yellows: u8,
    pub reds: u8,
    pub saves: u8,
    pub conceded: u8,
    pub injured: bool,
    /// The injury was not from contact (overuse, landing, sprinting); drawn from the body's own risk.
    pub injury_noncontact: bool,
    pub condition_end: u8,
    /// Action counts per zone in the player's attacking frame (Full LOD only).
    pub zones: Option<Box<[u16; 30]>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchResult {
    pub home: TeamId,
    pub away: TeamId,
    pub home_goals: u8,
    pub away_goals: u8,
    pub ht: (u8, u8),
    pub extra_time: bool,
    pub pens: Option<(u8, u8)>,
    pub stats: [TeamStats; 2],
    pub lines: Vec<PlayerLine>,
    pub events: Vec<MatchEvent>,
    pub pom: PlayerId,
}

impl MatchResult {
    pub fn line(&self, p: PlayerId) -> Option<&PlayerLine> {
        self.lines.iter().find(|l| l.player == p)
    }

    /// Winning side after penalties, or `None` for a draw.
    pub fn winner(&self) -> Option<u8> {
        match self.home_goals.cmp(&self.away_goals) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => self.pens.map(|(h, a)| if h > a { 0 } else { 1 }),
        }
    }
}
