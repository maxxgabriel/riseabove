use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    /// Preferred moves (03 §4). Each biases action selection in the match engine.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct PlayerTraits: u32 {
        const CUTS_INSIDE          = 1 << 0;
        const TRIES_TRICKS         = 1 << 1;
        const SHOOTS_FROM_DISTANCE = 1 << 2;
        const STAYS_BACK           = 1 << 3;
        const RUNS_WITH_BALL       = 1 << 4;
        const PLAYS_ONE_TWOS       = 1 << 5;
        const DIVES_INTO_TACKLES   = 1 << 6;
        const CURLS_BALL           = 1 << 7;
        const PLACES_SHOTS         = 1 << 8;
        const BEATS_OFFSIDE_TRAP   = 1 << 9;
        const LOBS_KEEPER          = 1 << 10;
        const KNOCKS_BALL_PAST     = 1 << 11;
        const COMES_DEEP           = 1 << 12;
        const GETS_INTO_BOX        = 1 << 13;
        const HUGS_LINE            = 1 << 14;
        const DWELLS_ON_BALL       = 1 << 15;
        const ARRIVES_LATE         = 1 << 16;
        const MOVES_INTO_CHANNELS  = 1 << 17;
        const PENALTY_BOX_PLAYER   = 1 << 18;
        const BACK_TO_GOAL         = 1 << 19;
        const KILLER_BALLS         = 1 << 20;
        const MARKS_TIGHTLY        = 1 << 21;
        const OUTSIDE_OF_FOOT      = 1 << 22;
        const LONG_THROW           = 1 << 23;
        const POWER_SHOT           = 1 << 24;
        const SIMPLE_PASSES        = 1 << 25;
    }
}

impl PlayerTraits {
    pub const LABELS: [(PlayerTraits, &'static str); 26] = [
        (Self::CUTS_INSIDE, "Cuts inside from wing"),
        (Self::TRIES_TRICKS, "Tries tricks"),
        (Self::SHOOTS_FROM_DISTANCE, "Shoots from distance"),
        (Self::STAYS_BACK, "Stays back at all times"),
        (Self::RUNS_WITH_BALL, "Runs with ball often"),
        (Self::PLAYS_ONE_TWOS, "Plays one-twos"),
        (Self::DIVES_INTO_TACKLES, "Dives into tackles"),
        (Self::CURLS_BALL, "Curls ball"),
        (Self::PLACES_SHOTS, "Places shots"),
        (Self::BEATS_OFFSIDE_TRAP, "Tries to beat offside trap"),
        (Self::LOBS_KEEPER, "Likes to lob keeper"),
        (Self::KNOCKS_BALL_PAST, "Knocks ball past opponent"),
        (Self::COMES_DEEP, "Comes deep to get ball"),
        (Self::GETS_INTO_BOX, "Gets into opposition area"),
        (Self::HUGS_LINE, "Hugs line"),
        (Self::DWELLS_ON_BALL, "Dwells on ball"),
        (Self::ARRIVES_LATE, "Arrives late in box"),
        (Self::MOVES_INTO_CHANNELS, "Moves into channels"),
        (Self::PENALTY_BOX_PLAYER, "Penalty box player"),
        (Self::BACK_TO_GOAL, "Plays with back to goal"),
        (Self::KILLER_BALLS, "Tries killer balls often"),
        (Self::MARKS_TIGHTLY, "Marks opponent tightly"),
        (Self::OUTSIDE_OF_FOOT, "Uses outside of foot"),
        (Self::LONG_THROW, "Long throw specialist"),
        (Self::POWER_SHOT, "Shoots with power"),
        (Self::SIMPLE_PASSES, "Plays short simple passes"),
    ];

    pub fn labels(self) -> impl Iterator<Item = &'static str> {
        Self::LABELS.into_iter().filter(move |(t, _)| self.contains(*t)).map(|(_, l)| l)
    }
}
