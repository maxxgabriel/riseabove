//! Small text helpers for server-side messages (the client formats data cells itself).

use pw_core::Date;

pub fn date(d: Date) -> String {
    let (y, m, day) = d.ymd();
    format!("{y}-{m:02}-{day:02}")
}

pub fn ordinal(n: usize) -> String {
    let suffix = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {many}") }
}
