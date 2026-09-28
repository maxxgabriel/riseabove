//! Football grammar: the game's own vocabulary, chosen for a voice.
//!
//! Terms are concepts (`Term`), not words. A concept is rendered through
//! the voice's dialect (lexical differences only), era (words enter the
//! language when the game has them — "xG" in the analytics era, "gegenpress"
//! once pressing is in fashion), register and age band. Style phrases
//! describe a real philosophy's numbers; nothing is said about a team that
//! its tactics do not show.
//!
//! Voices never come from nationality beyond vocabulary: no pack may
//! caricature where someone is from, how they speak, or what they believe.

use crate::lexicon::{AgeBand, Dialect, Era, Register, Voice};
use crate::pick;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Term {
    Pitch,
    Match,
    /// Zero in a score.
    Nil,
    Boots,
    Kit,
    /// The team's manager, informally.
    Boss,
    Supporters,
    Ground,
    Striker,
    Defence,
    Press,
    LowBlock,
    HighLine,
    Counter,
    Possession,
    LongBall,
    Overload,
    Transition,
    /// Expected-goals language.
    Chances,
    Playmaker,
}

/// A word or phrase for a football concept in this voice.
pub fn term(v: &Voice, t: Term, key: u64) -> &'static str {
    use Dialect::*;
    use Term::*;
    let young = v.age == AgeBand::Young;
    let analytic = v.register == Register::Analytical;
    let later = v.era >= Era::Analytics;
    let options: &[&str] = match (t, v.dialect) {
        (Pitch, American) => &["field"],
        (Pitch, _) => &["pitch", "park"],
        (Match, American) => &["game"],
        (Match, _) => &["match", "game"],
        (Nil, British | Irish) => &["nil"],
        (Nil, _) => &["zero", "nil"],
        (Boots, American) => &["cleats"],
        (Boots, _) => &["boots"],
        (Kit, American) => &["uniform", "jersey"],
        (Kit, _) => &["kit", "shirt"],
        (Boss, British | Irish) if v.register == Register::Casual || v.register == Register::Terrace => &["the gaffer", "the boss", "the manager"],
        (Boss, American) => &["the coach", "the manager"],
        (Boss, _) => &["the manager", "the boss", "the head coach"],
        (Supporters, American) => &["fans", "supporters"],
        (Supporters, _) => &["supporters", "fans", "the faithful"],
        (Ground, American) => &["stadium"],
        (Ground, _) => &["ground", "stadium"],
        (Striker, _) if v.era == Era::Classic => &["centre-forward", "striker"],
        (Striker, American) => &["forward", "striker"],
        (Striker, _) => &["striker", "number nine", "forward"],
        (Defence, American) => &["defense", "back line"],
        (Defence, _) => &["defence", "back line", "back four"],
        (Press, _) if analytic && later => &["high press (PPDA down)", "pressing intensity"],
        (Press, _) if v.era >= Era::Modern => &["press", "counter-press", "pressing game"],
        (Press, _) => &["closing down", "harrying"],
        (LowBlock, _) if young => &["parking the bus", "sitting in"],
        (LowBlock, _) if v.era >= Era::Modern => &["low block", "deep block", "sitting deep"],
        (LowBlock, _) => &["defending deep", "putting men behind the ball"],
        (HighLine, _) => &["high line", "high defensive line"],
        (Counter, _) => &["counter-attack", "break", "transition"],
        (Possession, _) if v.era >= Era::Modern => &["possession", "control", "positional play"],
        (Possession, _) => &["keeping the ball", "passing game"],
        (LongBall, _) if young || v.register == Register::Terrace => &["hoofball", "lumping it forward"],
        (LongBall, _) => &["long balls", "direct play", "going long"],
        (Overload, _) if v.era >= Era::Analytics => &["overloads", "half-space overloads", "numbers in wide areas"],
        (Overload, _) => &["getting numbers forward", "overloads"],
        (Transition, _) if v.era >= Era::Modern => &["transitions", "turnovers"],
        (Transition, _) => &["breaks", "turnovers"],
        (Chances, _) if later && analytic => &["xG", "expected goals", "shot quality"],
        (Chances, _) if later => &["xG", "chances", "big chances"],
        (Chances, _) => &["chances", "openings"],
        (Playmaker, _) if v.era == Era::Classic => &["schemer", "playmaker"],
        (Playmaker, _) => &["playmaker", "creator", "number ten"],
    };
    pick(key ^ (t as u64).wrapping_mul(0x51ed), options)
}

/// A score in this voice ("2-0", "two-nil").
pub fn score(v: &Voice, a: u8, b: u8, key: u64) -> String {
    let words = ["nil", "one", "two", "three", "four", "five"];
    let spoken = matches!(v.register, Register::Casual | Register::Terrace) && a <= 5 && b <= 5 && key.is_multiple_of(3);
    if spoken {
        let n = |x: u8| if x == 0 { term(v, Term::Nil, key).to_string() } else { words[usize::from(x)].to_string() };
        format!("{}-{}", n(a), n(b))
    } else {
        format!("{a}-{b}")
    }
}

/// How a team plays, from its philosophy's numbers (press, tempo,
/// directness, 0–100) — used for managers, schools and teams.
pub fn style(v: &Voice, press: u8, tempo: u8, direct: u8, key: u64) -> String {
    let mut parts: Vec<String> = Vec::new();
    match press {
        0..=34 => parts.push(term(v, Term::LowBlock, key).to_string()),
        66..=100 => parts.push(format!("a relentless {}", term(v, Term::Press, key))),
        _ => {}
    }
    match direct {
        0..=34 => parts.push(term(v, Term::Possession, key ^ 1).to_string()),
        66..=100 => parts.push(term(v, Term::LongBall, key ^ 2).to_string()),
        _ => {}
    }
    match tempo {
        66..=100 => parts.push(format!("quick {}", term(v, Term::Transition, key ^ 3))),
        0..=30 => parts.push("a slow build-up".to_string()),
        _ => {}
    }
    if parts.is_empty() {
        return "a balanced approach".to_string();
    }
    match parts.len() {
        1 => parts.remove(0),
        _ => {
            let last = parts.pop().unwrap_or_default();
            format!("{} and {last}", parts.join(", "))
        }
    }
}

/// A supporter's complaint about a manager's style — only what the style
/// actually is ("too much hoofball" only for direct teams).
pub fn style_complaint(v: &Voice, press: u8, tempo: u8, direct: u8, key: u64) -> Option<String> {
    if direct >= 66 {
        return Some(format!("{} {}", pick(key, &["enough of the", "sick of the", "too much"]), term(v, Term::LongBall, key)));
    }
    if press <= 34 {
        return Some(format!("{} again", term(v, Term::LowBlock, key)));
    }
    if tempo <= 30 && direct <= 34 {
        return Some(pick(key, &["sideways passing, no end product", "all possession, no threat", "passing it to death"]).to_string());
    }
    if press >= 70 && tempo >= 70 {
        return Some(pick(key, &["legs have gone from all that pressing", "can't press like this every week"]).to_string());
    }
    None
}

/// Banter: light and good-humoured for jokers, sharper for the hostile —
/// never about who someone is, only what happened.
pub fn banter(v: &Voice, humour: u8, hostility: u8, key: u64) -> &'static str {
    let young = v.age == AgeBand::Young;
    match (humour >= 60, hostility >= 60, young) {
        (true, false, true) => pick(key, &["lol ok", "not the flex you think it is", "we move"]),
        (true, false, false) => pick(key, &["enjoy that", "same time next year?", "we'll take it"]),
        (_, true, true) => pick(key, &["actually embarrassing", "sit down", "can't make it up"]),
        (_, true, false) => pick(key, &["shocking", "embarrassing from them", "no excuses"]),
        _ => pick(key, &["that's football", "it is what it is", "fair result"]),
    }
}
