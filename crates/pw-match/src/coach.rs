//! Hooks for the people around a match: what their staff can see while it is played, and what they may change (locked design 7.1-7.22).
//!
//! The engine never hands a manager its parameters. At a few moments (a first-half check, half-time, the hour, the last quarter) it hands
//! each side a [`Look`]: the football evidence so far (shots, chances, who lost duels, who is booked, how tired they look) and nothing
//! about press intensity or hidden attributes. The side answers with a [`Call`]: a change of approach, a change of role, substitutions.
//! What a call *does* is decided by the engine, not by the caller.
//!
//! Before kick-off the coach may also describe the state each player walks out in ([`Mind`]): behaviour, not a bonus.

use pw_core::{PlayerId, Pos, Role, Tactics};
use smallvec::SmallVec;

use crate::types::MatchResult;

/// How a player arrives on the pitch, seen from what it does to his play, each -1..1 around 0 (an ordinary day).
/// Different states move different behaviours: unfocused players err, angry ones foul, cautious ones stop taking chances.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Mind {
    /// Attention on the game: concentration and decision quality.
    pub focus: f32,
    /// Emotional control: composure under pressure, and how far he lets himself be provoked.
    pub calm: f32,
    /// Appetite for the difficult ball or the dribble against the safe one.
    pub risk: f32,
    /// Energy he brings to the work off the ball.
    pub drive: f32,
    /// Belief in himself; feeds the form he plays to.
    pub confidence: f32,
}

impl Mind {
    pub const NEUTRAL: Mind = Mind { focus: 0.0, calm: 0.0, risk: 0.0, drive: 0.0, confidence: 0.0 };

    pub fn is_neutral(&self) -> bool {
        *self == Mind::NEUTRAL
    }

    /// The same state, kept in range.
    pub fn clamped(self) -> Mind {
        let c = |x: f32| x.clamp(-1.0, 1.0);
        Mind { focus: c(self.focus), calm: c(self.calm), risk: c(self.risk), drive: c(self.drive), confidence: c(self.confidence) }
    }
}

/// What one player has been seen to do so far.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tally {
    pub duels_won: u8,
    pub duels_lost: u8,
    pub passes_ok: u8,
    pub passes_lost: u8,
    pub shots: u8,
    pub fouls: u8,
    pub fouled: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerLook {
    pub player: PlayerId,
    pub pos: Pos,
    pub role: Role,
    /// How fresh he looks, 0-100.
    pub condition: u8,
    pub booked: bool,
    pub tally: Tally,
}

/// What one side has produced and what it can change.
#[derive(Clone, Debug)]
pub struct SideLook {
    pub tactics: Tactics,
    /// Share of the ball so far, 0-100.
    pub possession: u8,
    pub shots: u16,
    pub on_target: u16,
    pub big_chances: u16,
    pub corners: u16,
    pub fouls: u16,
    pub yellows: u16,
    pub reds: u16,
    /// Actions in the attacking third or box: territory taken.
    pub territory: u16,
    /// Passes lost in the side's own third: build-up broken.
    pub lost_own_third: u16,
    pub subs_made: u8,
    pub subs_max: u8,
    pub players: SmallVec<[PlayerLook; 11]>,
    pub bench: SmallVec<[(PlayerId, Pos); 12]>,
}

#[derive(Clone, Debug)]
pub struct Look {
    pub minute: u8,
    pub half_time: bool,
    pub goals: [u8; 2],
    pub sides: [SideLook; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct SubCall {
    pub out: PlayerId,
    pub inn: PlayerId,
}

/// What a side decides to change at a window. Empty means: carry on as planned.
#[derive(Clone, Debug, Default)]
pub struct Call {
    /// New instructions (the engine reads what it can from them).
    pub tactics: Option<Tactics>,
    /// Role changes for men already on the pitch.
    pub roles: SmallVec<[(PlayerId, Role); 2]>,
    pub subs: SmallVec<[SubCall; 3]>,
}

impl Call {
    pub fn is_empty(&self) -> bool {
        self.tactics.is_none() && self.roles.is_empty() && self.subs.is_empty()
    }
}

/// The people around a match. Everything has a default so a coach implements only what it has something to say about.
pub trait Coach {
    /// Does this coach make the decisions the engine's own managers would (style changes, tactical substitutions)? When it does, they
    /// keep only the substitutions for players who are plainly spent.
    fn takes_over(&self) -> bool {
        true
    }

    /// The state a player walks out in (`side` 0 home, 1 away).
    fn mind(&self, _side: u8, _player: PlayerId) -> Mind {
        Mind::NEUTRAL
    }

    /// Minutes at which the sides are given a look, besides half-time.
    fn windows(&self) -> &'static [u8] {
        &[30, 60, 75]
    }

    /// Both sides look at the game and each answers.
    fn call(&mut self, _look: &Look) -> [Call; 2] {
        [Call::default(), Call::default()]
    }

    /// Which substitutions of a call were actually made, in order.
    fn applied(&mut self, _side: u8, _call: &Call, _subs_made: &[bool]) {}

    /// The last look, at full time: the whole match as it could be seen.
    fn full_time(&mut self, _look: &Look) {}

    /// The match is over.
    fn finished(&mut self, _result: &MatchResult) {}
}

/// A coach with nothing to say: the engine's own managers run the match.
pub struct Nobody;

impl Coach for Nobody {
    fn takes_over(&self) -> bool {
        false
    }
}
