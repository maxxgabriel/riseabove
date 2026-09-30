//! Small pure text helpers: Indian money formatting, dates resolved against the simulation clock, lists, ordinals, a/an.

use pw_core::{Date, Weekday};

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

pub fn month_name(m: u32) -> &'static str {
    MONTHS[(m as usize + 11) % 12]
}

pub fn weekday_name(w: Weekday) -> &'static str {
    DAYS[w.index() as usize % 7]
}

/// Indian digit grouping: 12,34,567.
pub fn group_indian(n: i64) -> String {
    let neg = n < 0;
    let s = n.unsigned_abs().to_string();
    let out = if s.len() <= 3 {
        s
    } else {
        let (head, tail) = s.split_at(s.len() - 3);
        let mut parts = Vec::new();
        let mut h = head;
        while h.len() > 2 {
            let (a, b) = h.split_at(h.len() - 2);
            parts.push(b.to_string());
            h = a;
        }
        if !h.is_empty() {
            parts.push(h.to_string());
        }
        parts.reverse();
        format!("{},{}", parts.join(","), tail)
    };
    if neg { format!("-{out}") } else { out }
}

fn trim_decimal(x: f64) -> String {
    let s = format!("{x:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// Rupee amounts the way Indian football reporting writes them: ₹2.5 crore, ₹75 lakh, ₹85,000. `words` spells the symbol as "Rs".
pub fn money(rupees: i64, words: bool) -> String {
    let sym = if words { "Rs " } else { "₹" };
    let a = rupees.unsigned_abs() as f64;
    let body = if a >= 1e7 {
        format!("{} crore", trim_decimal(a / 1e7))
    } else if a >= 1e5 {
        format!("{} lakh", trim_decimal(a / 1e5))
    } else {
        group_indian(rupees.abs())
    };
    if rupees < 0 { format!("-{sym}{body}") } else { format!("{sym}{body}") }
}

/// Days from `now` to `then`: negative = in the past.
pub fn delta(then: Date, now: Date) -> i32 {
    then.0 - now.0
}

/// How an event date is written relative to the day the text is published. Resolved only from the two dates, never guessed.
pub fn when(then: Date, now: Date) -> String {
    let d = delta(then, now);
    let (y, m, day) = then.ymd();
    let (ny, _, _) = now.ymd();
    match d {
        0 => "today".into(),
        -1 => "yesterday".into(),
        1 => "tomorrow".into(),
        -6..=-2 | 2..=6 => format!("on {}", weekday_name(then.weekday())),
        -13..=-7 => format!("last {}", weekday_name(then.weekday())),
        7..=13 => format!("next {}", weekday_name(then.weekday())),
        _ if d.abs() <= 60 && y == ny => format!("on {day} {}", month_name(m)),
        _ if y == ny => format!("in {}", month_name(m)),
        _ => format!("in {} {y}", month_name(m)),
    }
}

/// "12 March 2027".
pub fn absolute(d: Date) -> String {
    let (y, m, day) = d.ymd();
    format!("{day} {} {y}", month_name(m))
}

pub fn parse_date(s: &str) -> Option<Date> {
    let mut it = s.split('-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(Date::from_ymd(y, m, d))
}

/// "1 week"/"3 weeks": the noun agreeing with `n` (regular plurals only; irregular nouns get an explicit plural in the lexicon).
pub fn plural(n: i64, noun: &str) -> String {
    if n == 1 {
        noun.to_string()
    } else if noun.ends_with(['s', 'x']) || noun.ends_with("ch") || noun.ends_with("sh") {
        format!("{noun}es")
    } else if noun == "goal" || noun.ends_with(|c: char| c.is_alphabetic()) {
        format!("{noun}s")
    } else {
        noun.to_string()
    }
}

pub fn ordinal(n: i64) -> String {
    let suf = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

/// "first".."tenth" in words, then digits: "11th".
pub fn ordinal_word(n: i64) -> String {
    const W: [&str; 10] = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth"];
    if (1..=10).contains(&n) { W[n as usize - 1].to_string() } else { ordinal(n) }
}

pub fn number_words(n: i64) -> String {
    const W: [&str; 11] = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];
    if (0..=10).contains(&n) { W[n as usize].to_string() } else { n.to_string() }
}

/// "a", "a and b", "a, b and c".
pub fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        _ => format!("{} and {}", items[..items.len() - 1].join(", "), items[items.len() - 1]),
    }
}

/// True when `word` starts with a vowel sound (so takes "an"). Handles the common English exceptions.
pub fn vowel_sound(word: &str) -> bool {
    let w: String = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    if w.is_empty() {
        return false;
    }
    const AN_EXC: [&str; 6] = ["hour", "honest", "honour", "heir", "hono", "herb"];
    const A_EXC: [&str; 9] = ["uni", "use", "usu", "eu", "one", "once", "ubiq", "uti", "ura"];
    if AN_EXC.iter().any(|p| w.starts_with(p)) {
        return true;
    }
    if A_EXC.iter().any(|p| w.starts_with(p)) {
        return false;
    }
    // Numerals are read aloud: "an 8th", "an 11-year-old", "an 80,000 crowd", "a 19-year-old".
    if w.starts_with(|c: char| c.is_ascii_digit()) {
        let digits: String = w.chars().filter(|c| c.is_ascii_digit() || *c == ',').filter(|c| *c != ',').take_while(|c| c.is_ascii_digit()).collect();
        return digits.parse::<u64>().is_ok_and(number_starts_with_vowel);
    }
    if word.len() >= 2 && word.chars().take(2).all(|c| c.is_ascii_uppercase()) {
        // Initialisms: read letter by letter; F, H, L, M, N, R, S, X begin with a vowel sound.
        return matches!(word.chars().next(), Some('A' | 'E' | 'F' | 'H' | 'I' | 'L' | 'M' | 'N' | 'O' | 'R' | 'S' | 'X'));
    }
    matches!(w.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'))
}

fn number_starts_with_vowel(n: u64) -> bool {
    match n {
        1000..=999_999 => number_starts_with_vowel(n / 1000),
        100..=999 => n / 100 == 8,
        _ => n == 8 || n == 11 || n == 18 || (80..=89).contains(&n),
    }
}

/// Rewrites every "a X"/"an X" article to agree with the next word. Only touches whole-word `a`/`an`.
pub fn fix_articles(s: &str) -> String {
    let words: Vec<&str> = s.split(' ').collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        let lower = w.to_lowercase();
        if (lower == "a" || lower == "an") && i + 1 < words.len() && !words[i + 1].is_empty() {
            let an = vowel_sound(words[i + 1]);
            let base = if an { "an" } else { "a" };
            let cap = w.chars().next().is_some_and(|c| c.is_uppercase());
            out.push(if cap { format!("{}{}", base[..1].to_uppercase(), &base[1..]) } else { base.to_string() });
        } else {
            out.push((*w).to_string());
        }
    }
    out.join(" ")
}

pub fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indian_money() {
        assert_eq!(money(25_000_000, false), "₹2.5 crore");
        assert_eq!(money(7_500_000, false), "₹75 lakh");
        assert_eq!(money(85_000, false), "₹85,000");
        assert_eq!(money(120_000_000, true), "Rs 12 crore");
        assert_eq!(group_indian(1_234_567), "12,34,567");
    }

    #[test]
    fn dates_resolve_against_now() {
        let now = Date::from_ymd(2026, 8, 20);
        assert_eq!(when(Date::from_ymd(2026, 8, 19), now), "yesterday");
        assert_eq!(when(now, now), "today");
        assert_eq!(when(Date::from_ymd(2026, 8, 21), now), "tomorrow");
        assert_eq!(when(Date::from_ymd(2026, 8, 17), now), "on Monday");
        assert_eq!(when(Date::from_ymd(2026, 8, 10), now), "last Monday");
        assert_eq!(when(Date::from_ymd(2026, 7, 1), now), "on 1 July");
        assert_eq!(when(Date::from_ymd(2025, 12, 3), now), "in December 2025");
    }

    #[test]
    fn articles() {
        assert_eq!(fix_articles("a injury and an bid, a hour, an unit, a MRI"), "an injury and a bid, an hour, a unit, an MRI");
        assert_eq!(fix_articles("a 19-year-old and an 11-year-old"), "a 19-year-old and an 11-year-old");
    }
}
