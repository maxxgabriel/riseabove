//! Small text helpers for server-side messages (the client formats data cells itself).

use pw_core::Date;

pub fn date(d: Date) -> String {
    let (y, m, day) = d.ymd();
    format!("{y}-{m:02}-{day:02}")
}

/// See `pw_narrate::fmt::singulars`.
pub use pw_narrate::fmt::singulars;

/// "3 August".
pub fn day_month(d: Date) -> String {
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let (_, m, day) = d.ymd();
    format!("{day} {}", MONTHS[(m as usize).saturating_sub(1).min(11)])
}

/// "a day", "5 days", "2 weeks".
pub fn days_words(n: i32) -> String {
    match n {
        ..=1 => "a day".into(),
        2..=13 => format!("{n} days"),
        _ => format!("{} weeks", (n + 3) / 7),
    }
}

/// "Climate and surroundings" from "climate and surroundings".
pub fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}
