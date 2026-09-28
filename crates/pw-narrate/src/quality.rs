//! Text quality checks (20 in the final brief).
//!
//! Every rendered line can be checked for the failures generated text is
//! prone to: an empty or placeholder name, an unfilled template, a doubled
//! word or space, a missing capital, gendered pronouns where the world
//! does not record gender, and runaway length. Tests render large samples
//! of events, posts, headlines and messages through these checks; a debug
//! client can show violations next to the text.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Issue {
    Empty,
    /// "?" or "NONE" standing in for a missing name.
    Placeholder,
    /// `{`, `}` or `{}` left in the text.
    Unfilled,
    DoubledSpace,
    /// The same word twice in a row ("the the").
    DoubledWord,
    /// A sentence that should start with a capital does not.
    Lowercase,
    /// he/she/him/her/his/hers/himself/herself.
    Gendered,
    TooLong,
    /// Punctuation doubled (",," "..") outside an ellipsis.
    Punctuation,
}

const GENDERED: [&str; 8] = ["he", "she", "him", "her", "his", "hers", "himself", "herself"];

/// Check a line of text. `sentence` asks for a capital first letter.
pub fn check(text: &str, sentence: bool, max_len: usize) -> Vec<Issue> {
    let mut v = Vec::new();
    let t = text.trim();
    if t.is_empty() {
        v.push(Issue::Empty);
        return v;
    }
    if t.contains(" ? ") || t.starts_with("? ") || t.ends_with(" ?") || t.contains("NONE") || t == "?" {
        v.push(Issue::Placeholder);
    }
    if t.contains('{') || t.contains('}') {
        v.push(Issue::Unfilled);
    }
    if t.contains("  ") {
        v.push(Issue::DoubledSpace);
    }
    let words: Vec<String> = t.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()).collect();
    if words.windows(2).any(|p| !p[0].is_empty() && p[0] == p[1] && !matches!(p[0].as_str(), "ha" | "no" | "so" | "very")) {
        v.push(Issue::DoubledWord);
    }
    if sentence && t.chars().next().is_some_and(|c| c.is_lowercase()) {
        v.push(Issue::Lowercase);
    }
    if words.iter().any(|w| GENDERED.contains(&w.as_str())) {
        v.push(Issue::Gendered);
    }
    if t.chars().count() > max_len {
        v.push(Issue::TooLong);
    }
    if t.contains(",,") || t.contains(" ,") || (t.contains("..") && !t.contains("...")) {
        v.push(Issue::Punctuation);
    }
    v
}

/// Convenience: whether a line passes.
pub fn ok(text: &str, sentence: bool, max_len: usize) -> bool {
    check(text, sentence, max_len).is_empty()
}
