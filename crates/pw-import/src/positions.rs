use pw_core::Pos;
use smallvec::SmallVec;

/// Parse positions in our codes (`ST, AMC`) or FM notation
/// (`D/WB (R), DM`, `AM (RLC), ST (C)`). Order is preserved; the first is the
/// player's natural position.
pub fn parse_positions(s: &str) -> SmallVec<[Pos; 6]> {
    let mut out: SmallVec<[Pos; 6]> = SmallVec::new();
    let mut push = |p: Pos| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    for part in split_groups(s) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(p) = Pos::from_code(&part.to_ascii_uppercase().replace(' ', "")) {
            push(p);
            continue;
        }
        let (roles, sides) = match part.find('(') {
            Some(i) => (&part[..i], part[i + 1..].trim_end_matches(')')),
            None => (part, ""),
        };
        let sides: SmallVec<[char; 3]> = {
            let v: SmallVec<[char; 3]> = sides.chars().filter(|c| matches!(c, 'R' | 'L' | 'C' | 'r' | 'l' | 'c')).map(|c| c.to_ascii_uppercase()).collect();
            if v.is_empty() { smallvec::smallvec!['C'] } else { v }
        };
        for role in roles.split('/') {
            let role = role.trim().to_ascii_uppercase();
            for &side in &sides {
                let p = match (role.as_str(), side) {
                    ("GK", _) => Some(Pos::GK),
                    ("D", 'R') => Some(Pos::DR),
                    ("D", 'C') => Some(Pos::DC),
                    ("D", 'L') => Some(Pos::DL),
                    ("WB", 'R') => Some(Pos::WBR),
                    ("WB", 'L') => Some(Pos::WBL),
                    ("WB", _) => None,
                    ("DM", _) => Some(Pos::DM),
                    ("M", 'R') => Some(Pos::MR),
                    ("M", 'C') => Some(Pos::MC),
                    ("M", 'L') => Some(Pos::ML),
                    ("AM", 'R') => Some(Pos::AMR),
                    ("AM", 'C') => Some(Pos::AMC),
                    ("AM", 'L') => Some(Pos::AML),
                    ("ST" | "S" | "F", _) => Some(Pos::ST),
                    _ => None,
                };
                if let Some(p) = p {
                    push(p);
                }
            }
        }
    }
    out
}

/// Split on commas that are not inside parentheses.
fn split_groups(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' | ';' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fm_notation() {
        assert_eq!(parse_positions("AM (RLC), ST (C)").as_slice(), &[Pos::AMR, Pos::AML, Pos::AMC, Pos::ST]);
        assert_eq!(parse_positions("D/WB (R)").as_slice(), &[Pos::DR, Pos::WBR]);
        assert_eq!(parse_positions("GK").as_slice(), &[Pos::GK]);
        assert_eq!(parse_positions("DM, M (C)").as_slice(), &[Pos::DM, Pos::MC]);
        assert_eq!(parse_positions("ST,AMC").as_slice(), &[Pos::ST, Pos::AMC]);
    }
}
