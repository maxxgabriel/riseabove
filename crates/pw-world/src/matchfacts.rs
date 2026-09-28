//! The facts of recent senior matches, kept for a few weeks: who scored and
//! when, late winners, hat-tricks, comebacks, red cards, the best player,
//! what the match meant. The press, supporters, awards and records read
//! these; nothing here is prose, and nothing is kept beyond what happened.

use std::collections::VecDeque;

use pw_core::{ClubId, CompId, Date, FixtureId, PlayerId, TeamId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Goal {
    pub player: PlayerId,
    pub assist: PlayerId,
    /// Match minute.
    pub minute: u8,
    /// 0 home, 1 away.
    pub side: u8,
    pub penalty: bool,
    pub own_goal: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchFacts {
    pub uid: u64,
    pub fixture: FixtureId,
    pub date: Date,
    pub comp: CompId,
    pub home_team: TeamId,
    pub away_team: TeamId,
    pub home: ClubId,
    pub away: ClubId,
    pub hg: u8,
    pub ag: u8,
    pub pens: Option<(u8, u8)>,
    pub goals: SmallVec<[Goal; 8]>,
    pub reds: SmallVec<[(PlayerId, u8); 2]>,
    /// The goal that decided it in the last minutes.
    pub late_winner: Option<Goal>,
    pub hat_tricks: SmallVec<[PlayerId; 1]>,
    /// The winner was behind at some point.
    pub comeback: bool,
    pub pom: PlayerId,
    /// Best rating ×10.
    pub pom_rating: u8,
    /// Players who scored on their senior debut.
    pub debut_goals: SmallVec<[PlayerId; 2]>,
    /// What the fixture meant (0–100) and whether it was a derby.
    pub significance: u8,
    pub derby: bool,
}

impl MatchFacts {
    /// +1 home win, 0 draw, -1 away win (penalties decide).
    pub fn result(&self) -> i8 {
        match self.hg.cmp(&self.ag) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => self.pens.map_or(0, |(h, a)| if h > a { 1 } else { -1 }),
        }
    }

    pub fn winner(&self) -> Option<ClubId> {
        match self.result() {
            1 => Some(self.home),
            -1 => Some(self.away),
            _ => None,
        }
    }

    pub fn involves(&self, c: ClubId) -> bool {
        self.home == c || self.away == c
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RecentMatches {
    pub list: VecDeque<MatchFacts>,
}

impl RecentMatches {
    /// Keep four weeks.
    pub fn push(&mut self, m: MatchFacts) {
        let today = m.date;
        while self.list.front().is_some_and(|f| f.date.days_until(today) > 28) {
            self.list.pop_front();
        }
        self.list.push_back(m);
    }

    pub fn by_uid(&self, uid: u64) -> Option<&MatchFacts> {
        self.list.iter().rev().find(|m| m.uid == uid)
    }

    pub fn on(&self, d: Date) -> impl Iterator<Item = &MatchFacts> + '_ {
        self.list.iter().filter(move |m| m.date == d)
    }

    pub fn of_club(&self, c: ClubId) -> impl DoubleEndedIterator<Item = &MatchFacts> + '_ {
        self.list.iter().filter(move |m| m.involves(c))
    }
}
