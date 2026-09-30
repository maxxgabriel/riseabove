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

/// "1 day", "12 days".
pub fn days(n: u32) -> String {
    if n == 1 { "1 day".into() } else { format!("{n} days") }
}

/// "three weeks", "about two months", "a few days".
pub fn duration_days(d: u16) -> String {
    match d {
        0..=3 => "a few days".into(),
        4..=10 => "a week or so".into(),
        11..=24 => format!("{} weeks", (d + 3) / 7),
        25..=75 => format!("about {} weeks", (d + 3) / 7),
        76..=300 => format!("about {} months", (d + 15) / 30),
        _ => "the rest of the year or longer".into(),
    }
}

/// "1 matches" and "1 points" from counts formatted as digits become "1 match" and "1 point" (a decimal such as "2.1 points" is left alone).
pub fn singulars(mut s: String) -> String {
    const PAIRS: [(&str, &str); 20] = [
        ("matches", "match"),
        ("points", "point"),
        ("goals", "goal"),
        ("weeks", "week"),
        ("months", "month"),
        ("days", "day"),
        ("wins", "win"),
        ("defeats", "defeat"),
        ("assists", "assist"),
        ("appearances", "appearance"),
        ("meetings", "meeting"),
        ("games", "game"),
        ("minutes", "minute"),
        ("caps", "cap"),
        ("years", "year"),
        ("starts", "start"),
        ("spectators", "spectator"),
        ("titles", "title"),
        ("clean sheets", "clean sheet"),
        ("hat-tricks", "hat-trick"),
    ];
    for (many, one) in PAIRS {
        let pat = format!("1 {many}");
        let mut at = 0;
        while let Some(i) = s[at..].find(&pat).map(|i| i + at) {
            let lone = i == 0 || !(s.as_bytes()[i - 1].is_ascii_digit() || s.as_bytes()[i - 1] == b'.');
            let end = i + pat.len();
            let whole = s.as_bytes().get(end).is_none_or(|b| !b.is_ascii_alphanumeric());
            if lone && whole {
                s.replace_range(i..end, &format!("1 {one}"));
            }
            at = i + 1;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::singulars;

    #[test]
    fn a_count_of_one_agrees() {
        assert_eq!(singulars("out for about 1 days".into()), "out for about 1 day");
        assert_eq!(singulars("against 1 appearances in 2030".into()), "against 1 appearance in 2030");
        assert_eq!(singulars("11 days and 2.1 points and 21 matches, 1 matches".into()), "11 days and 2.1 points and 21 matches, 1 match");
        assert_eq!(singulars("1 startling".into()), "1 startling");
    }
}
