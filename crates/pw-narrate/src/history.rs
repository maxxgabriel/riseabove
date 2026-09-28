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

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

use pw_world::records::{Broken, Holder, Mark, RecordKey, Scope, Stat};

pub fn holder(w: &World, h: Holder) -> String {
    match h {
        Holder::Person(p) => crate::fmt::person(w, p),
        Holder::Club(c) => crate::fmt::club(w, c),
        Holder::Entrant(e) => entrant(w, e),
        Holder::Nation(n) => nation(w, n),
    }
}

fn scope(w: &World, s: Scope) -> String {
    match s {
        Scope::World => "the world".into(),
        Scope::Nation(n) => nation(w, n),
        Scope::Comp(c) => w.comps.get(c).map_or_else(|| "?".into(), |x| x.name.clone()),
        Scope::Club(c) => crate::fmt::club(w, c),
        Scope::Institution(i) => institution(w, i),
        Scope::Local(l) => entrant(w, Entrant::Local(l)),
        Scope::Minor(n, code) => comp_name(w, MinorKind::from_code(code), n, ""),
    }
}

fn years_days(d: i64) -> String {
    format!("{} years {} days", d / 365, d % 365)
}

/// A record's value in words.
pub fn value(stat: Stat, v: i64) -> String {
    match stat {
        Stat::YoungestScorer | Stat::YoungestDebut | Stat::OldestScorer => years_days(v),
        Stat::FeePaid | Stat::FeeReceived => crate::fmt::money(v),
        Stat::BiggestWin => format!("a {v}-goal margin"),
        Stat::WinsInRow => format!("{v} wins in a row"),
        Stat::UnbeatenRun => format!("{v} games unbeaten"),
        Stat::PointsInSeason => format!("{v} points"),
        Stat::Titles => format!("{v} titles"),
        Stat::Caps => format!("{v} caps"),
        Stat::Goals | Stat::GoalsInSeason | Stat::IntlGoals => format!("{v} goals"),
        Stat::Apps => format!("{v} appearances"),
    }
}

/// "the Riverton High School all-time scoring record".
pub fn record_name(w: &World, k: RecordKey) -> String {
    let what = match k.stat {
        Stat::Goals => "all-time scoring record",
        Stat::Apps => "appearance record",
        Stat::GoalsInSeason => "record for goals in a season",
        Stat::BiggestWin => "record win",
        Stat::FeePaid => "record signing",
        Stat::FeeReceived => "record sale",
        Stat::Caps => "caps record",
        Stat::IntlGoals => "international scoring record",
        Stat::Titles => "record for most titles",
        Stat::YoungestScorer => "youngest-scorer record",
        Stat::YoungestDebut => "youngest-debutant record",
        Stat::OldestScorer => "oldest-scorer record",
        Stat::WinsInRow => "record winning run",
        Stat::UnbeatenRun => "record unbeaten run",
        Stat::PointsInSeason => "points record",
    };
    format!("the {} {what}", scope(w, k.scope))
}

fn stood(days: i32) -> Option<String> {
    match days {
        d if d >= 730 => Some(format!("{} years", d / 365)),
        d if d >= 60 => Some(format!("{} months", d / 30)),
        _ => None,
    }
}

/// A record falling, with its history — only what the book holds.
pub fn broken(w: &World, b: &Broken) -> String {
    let mut s = format!("{} set {} ({})", holder(w, b.new.holder), record_name(w, b.key), value(b.key.stat, b.new.value));
    if let Some(Mark { holder: h, value: v, .. }) = b.old {
        s.push_str(&format!(", surpassing {} ({})", holder(w, h), value(b.key.stat, v)));
        if let Some(t) = stood(b.stood_days) {
            s.push_str(&format!(", which had stood for {t}"));
        }
    }
    if let Some(a) = b.new.against {
        s.push_str(&format!(" against {}", holder(w, a)));
    }
    s.push('.');
    s
}

/// Context for a professional record announced by `honours`: who held it
/// before and for how long, from the record book's silent mirror.
pub fn pro_context(w: &World, date: pw_core::Date, kind: pw_world::event::RecordKind, club: pw_core::ClubId) -> String {
    use pw_world::event::RecordKind as R;
    let (stat, scope) = match kind {
        R::ClubTopScorer => (Stat::Goals, Some(Scope::Club(club))),
        R::ClubMostApps => (Stat::Apps, Some(Scope::Club(club))),
        R::ClubRecordSigning => (Stat::FeePaid, Some(Scope::Club(club))),
        R::ClubRecordSale => (Stat::FeeReceived, Some(Scope::Club(club))),
        R::ClubBiggestWin => (Stat::BiggestWin, Some(Scope::Club(club))),
        R::WorldRecordFee => (Stat::FeePaid, Some(Scope::World)),
        R::NationMostCaps => (Stat::Caps, None),
        R::NationTopScorer => (Stat::IntlGoals, None),
        R::LeagueGoalsInSeason => return String::new(),
    };
    let Some(b) = w.records.broken.iter().rev().take(500).find(|b| !b.announced && b.new.date == date && b.key.stat == stat && scope.is_none_or(|s| s == b.key.scope)) else { return String::new() };
    let Some(old) = b.old else { return String::new() };
    let mut s = format!(" The previous mark was {} ({})", holder(w, old.holder), value(stat, old.value));
    if let Some(t) = stood(b.stood_days) {
        s.push_str(&format!(", set {t} earlier"));
    }
    s.push('.');
    s
}
