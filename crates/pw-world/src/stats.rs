use pw_core::{ClubId, CompId, PlayerId};
use pw_match::PlayerLine;
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

/// A player's totals in one competition-season for one club.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct StatLine {
    pub player: PlayerId,
    pub club: ClubId,
    pub comp: CompId,
    pub season: i32,
    pub apps: u16,
    pub starts: u16,
    pub minutes: u32,
    pub goals: u16,
    pub assists: u16,
    pub yellows: u8,
    pub reds: u8,
    pub clean_sheets: u16,
    pub conceded: u16,
    pub pom: u16,
    /// Sum of ratings ×10 for rated appearances.
    pub rating_sum: u32,
    pub xg: f32,
    pub shots: u16,
    pub key_passes: u16,
    pub tackles: u16,
}

impl StatLine {
    pub fn avg_rating(&self) -> f32 {
        if self.apps == 0 { 0.0 } else { self.rating_sum as f32 / 10.0 / f32::from(self.apps) }
    }

    pub fn add(&mut self, l: &PlayerLine, pom: bool) {
        if l.minutes == 0 {
            return;
        }
        self.apps += 1;
        self.starts += u16::from(l.started);
        self.minutes += u32::from(l.minutes);
        self.goals += u16::from(l.goals);
        self.assists += u16::from(l.assists);
        self.yellows += l.yellows;
        self.reds += l.reds;
        self.pom += u16::from(pom);
        self.rating_sum += (l.rating * 10.0).round() as u32;
        self.xg += l.xg;
        self.shots += u16::from(l.shots);
        self.key_passes += u16::from(l.key_passes);
        self.tackles += u16::from(l.tackles_won);
        if l.is_keeper {
            self.conceded += u16::from(l.conceded);
            if l.conceded == 0 && l.minutes >= 60 {
                self.clean_sheets += 1;
            }
        }
    }
}

/// Current-season statistics, keyed by (player, competition, club).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SeasonStats {
    lines: FxHashMap<(PlayerId, CompId, ClubId), StatLine>,
}

impl SeasonStats {
    pub fn record(&mut self, comp: CompId, club: ClubId, season: i32, line: &PlayerLine, pom: bool) {
        let e = self.lines.entry((line.player, comp, club)).or_insert_with(|| StatLine {
            player: line.player,
            club,
            comp,
            season,
            ..Default::default()
        });
        e.add(line, pom);
    }

    pub fn iter(&self) -> impl Iterator<Item = &StatLine> {
        self.lines.values()
    }

    pub fn for_player(&self, p: PlayerId) -> impl Iterator<Item = &StatLine> {
        self.lines.values().filter(move |l| l.player == p)
    }

    pub fn for_comp(&self, c: CompId) -> impl Iterator<Item = &StatLine> {
        self.lines.values().filter(move |l| l.comp == c)
    }

    /// Remove and return a competition's lines when its season closes.
    pub fn take_comp(&mut self, c: CompId) -> Vec<StatLine> {
        let mut out: Vec<StatLine> = self.lines.values().filter(|l| l.comp == c).copied().collect();
        self.lines.retain(|_, l| l.comp != c);
        out.sort_by_key(|l| (l.player, l.club));
        out
    }

    pub fn get(&self, p: PlayerId, c: CompId, club: ClubId) -> Option<&StatLine> {
        self.lines.get(&(p, c, club))
    }
}
