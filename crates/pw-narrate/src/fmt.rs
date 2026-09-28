//! Names, money, dates and small labels.

use pw_core::{ClubId, Money, PersonId, PlayerId};
use pw_world::World;

pub fn person(w: &World, p: PersonId) -> String {
    if p.is_none() {
        return "someone".into();
    }
    w.people.get(p).map_or_else(|| "someone".into(), |x| x.display_name(&w.names).into_owned())
}

pub fn player(w: &World, p: PlayerId) -> String {
    if p.is_none() {
        return "a player".into();
    }
    w.player_name(p)
}

pub fn club(w: &World, c: ClubId) -> String {
    if c.is_none() {
        return "no club".into();
    }
    w.clubs[c].name.clone()
}

pub fn club_short(w: &World, c: ClubId) -> String {
    if c.is_none() {
        return "-".into();
    }
    w.clubs[c].short_name.clone()
}

pub fn money(m: Money) -> String {
    let a = m.unsigned_abs() as f64;
    let sign = if m < 0 { "-" } else { "" };
    if a >= 1e6 {
        format!("{sign}{:.1}m", a / 1e6)
    } else if a >= 1e3 {
        format!("{sign}{:.0}k", a / 1e3)
    } else {
        format!("{sign}{a:.0}")
    }
}

pub fn wage(m: Money) -> String {
    format!("{} a week", money(m))
}

/// Qualitative word for a 0–100 value.
pub fn level(v: u8) -> &'static str {
    match v {
        0..=19 => "very poor",
        20..=39 => "poor",
        40..=54 => "okay",
        55..=69 => "good",
        70..=84 => "very good",
        _ => "excellent",
    }
}

/// How a person sounds when describing a mood value (−30..30).
pub fn feeling(v: i8) -> &'static str {
    match v {
        i8::MIN..=-12 => "deeply unhappy about",
        -11..=-5 => "unhappy about",
        -4..=-1 => "a little frustrated by",
        0..=3 => "fine with",
        4..=10 => "pleased with",
        _ => "delighted with",
    }
}

pub fn nation(w: &World, n: pw_core::NationId) -> String {
    if n.is_none() {
        return "abroad".into();
    }
    w.nations[n].name.clone()
}
