use pw_core::{ClubId, CompId, Date, Money, PlayerId, TeamId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;
use crate::comp::TableRow;
use crate::stats::StatLine;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Honour {
    pub comp: CompId,
    pub season: i32,
    pub team: TeamId,
    pub club: ClubId,
    pub runner_up: TeamId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwardRecord {
    pub comp: CompId,
    pub season: i32,
    pub kind: crate::event::AwardKind,
    pub player: PlayerId,
    pub club: ClubId,
    pub value: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchivedTable {
    pub comp: CompId,
    pub season: i32,
    pub rows: Vec<TableRow>,
}

/// One spell at a club (clubs × dates), for career timelines.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Spell {
    pub club: ClubId,
    pub from: Date,
    pub to: Option<Date>,
    pub loan: bool,
    pub fee: Money,
}

/// Append-only archive (11 §8). Season statistic lines are indexed per player
/// so career views are O(lines of that player).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct History {
    pub awards: Vec<AwardRecord>,
    pub lines: Vec<StatLine>,
    pub by_player: FxHashMap<PlayerId, Vec<u32>>,
    pub honours: Vec<Honour>,
    pub tables: Vec<ArchivedTable>,
    pub spells: FxHashMap<PlayerId, Vec<Spell>>,
}

impl History {
    pub fn archive_lines(&mut self, lines: Vec<StatLine>) {
        for l in lines {
            let idx = self.lines.len() as u32;
            self.by_player.entry(l.player).or_default().push(idx);
            self.lines.push(l);
        }
    }

    pub fn career(&self, p: PlayerId) -> impl Iterator<Item = &StatLine> {
        self.by_player.get(&p).into_iter().flatten().map(|&i| &self.lines[i as usize])
    }

    pub fn start_spell(&mut self, p: PlayerId, club: ClubId, from: Date, loan: bool, fee: Money) {
        let spells = self.spells.entry(p).or_default();
        if let Some(last) = spells.last_mut().filter(|s| s.to.is_none()) {
            last.to = Some(from);
        }
        spells.push(Spell { club, from, to: None, loan, fee });
    }

    pub fn end_spell(&mut self, p: PlayerId, on: Date) {
        if let Some(last) = self.spells.get_mut(&p).and_then(|s| s.last_mut()).filter(|s| s.to.is_none()) {
            last.to = Some(on);
        }
    }

    pub fn honours_of(&self, club: ClubId) -> impl Iterator<Item = &Honour> {
        self.honours.iter().filter(move |h| h.club == club)
    }
}
