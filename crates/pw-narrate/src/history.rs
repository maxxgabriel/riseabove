//! Names and lines for history below the professional game (and, with the
//! record and award engines, for history at every level).

use pw_world::minor::{Entrant, InstKind, Level, MinorKind, MinorSeason};
use pw_world::World;

use crate::fmt::{nation, player};

pub fn entrant(w: &World, e: Entrant) -> String {
    match e {
        Entrant::Inst(i) => w.minor.institutions.get(i as usize).map_or_else(|| "?".into(), |x| x.name.clone()),
        Entrant::Local(l) => w.youth.local.get(l).map_or_else(|| "?".into(), |x| x.name.clone()),
    }
}

pub fn institution(w: &World, i: u32) -> String {
    w.minor.institutions.get(i as usize).map_or_else(|| "?".into(), |x| x.name.clone())
}

pub fn level(l: Level) -> &'static str {
    match l {
        Level::Grassroots => "grassroots",
        Level::School => "school",
        Level::University => "university",
        Level::Amateur => "amateur",
        Level::Youth => "youth",
        Level::Professional => "professional",
        Level::International => "international",
    }
}

/// "the Riverton and district Schools League", "the Norland Amateur League (tier 2)".
pub fn comp_name(w: &World, kind: MinorKind, n: pw_core::NationId, region: &str) -> String {
    let where_ = if region.is_empty() { nation(w, n) } else { region.to_string() };
    match kind {
        MinorKind::SchoolLeague => format!("{where_} Schools League"),
        MinorKind::SchoolCup => format!("{where_} Schools Cup"),
        MinorKind::UniversityLeague => format!("{where_} Universities League"),
        MinorKind::AmateurLeague { tier: 1 } => format!("{where_} Amateur League"),
        MinorKind::AmateurLeague { tier } => format!("{where_} Amateur League, division {tier}"),
        MinorKind::GrassrootsCup => format!("{where_} Junior Cup"),
    }
}

pub fn season_line(w: &World, s: &MinorSeason) -> String {
    let name = comp_name(w, s.kind, s.nation, &s.region);
    let mut out = format!("{} won the {name} {}/{:02}", entrant(w, s.winner), s.season, (s.season + 1) % 100);
    if s.top_scorer.is_some() {
        out.push_str(&format!("; top scorer {} ({})", player(w, s.top_scorer), s.top_goals));
    }
    if s.best.is_some() && s.best != s.top_scorer {
        out.push_str(&format!("; best player {}", player(w, s.best)));
    }
    out.push('.');
    out
}

pub fn inst_kind(k: InstKind) -> &'static str {
    match k {
        InstKind::School => "school",
        InstKind::University => "university",
    }
}
