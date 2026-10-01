//! Template syntax used by every frame, wrapper and label.
//!
//! * `{key}` `{key:mode}`  a fact of the event (modes: `full` `short` `desc` `role` `has` `is` `was` for entities; `words` `ord` for numbers and money; `abs` for dates)
//! * `{lex:concept.form}`  a lexicon word chosen for the speaker's voice and the channel
//! * `{col:head.slot}` `{col:head.slot.form}`  a collocate of `head` that is valid for this event
//! * `{cert:source}`       the speaker's source for the sentence, as declared in the `source` table
//! * `{clause}`            the wrapped clause (wrappers only)
//! * `{~a|b|c}`            variation by composition: one alternative, chosen by seed
//! * `[?key: text]`        text that appears only when the speaker knows `key` at least as firmly as the sentence

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Text(String),
    Field { key: String, mode: String },
    Lex { concept: String, form: String },
    Col { head: String, slot: String, form: String },
    Cert(String),
    Clause,
    Alt(Vec<Vec<Node>>),
    Opt { key: String, body: Vec<Node> },
}

pub fn parse(src: &str) -> Result<Vec<Node>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let out = parse_seq(&chars, &mut i, None)?;
    if i != chars.len() {
        return Err(format!("unexpected `{}` in template {src:?}", chars[i]));
    }
    Ok(out)
}

/// Parses until end of input or an unmatched `stop` character (not consumed).
fn parse_seq(c: &[char], i: &mut usize, stop: Option<char>) -> Result<Vec<Node>, String> {
    let mut out = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, out: &mut Vec<Node>| {
        if !text.is_empty() {
            out.push(Node::Text(std::mem::take(text)));
        }
    };
    while *i < c.len() {
        let ch = c[*i];
        if Some(ch) == stop || (stop == Some('|') && ch == '}') {
            break;
        }
        match ch {
            '{' => {
                flush(&mut text, &mut out);
                *i += 1;
                out.push(parse_brace(c, i)?);
            }
            '[' if c.get(*i + 1) == Some(&'?') => {
                flush(&mut text, &mut out);
                *i += 2;
                let start = *i;
                while *i < c.len() && c[*i] != ':' {
                    *i += 1;
                }
                if *i >= c.len() {
                    return Err("unterminated [? key".into());
                }
                let key: String = c[start..*i].iter().collect();
                *i += 1;
                let body = parse_seq(c, i, Some(']'))?;
                if c.get(*i) != Some(&']') {
                    return Err("unterminated [? ... ]".into());
                }
                *i += 1;
                out.push(Node::Opt { key: key.trim().to_string(), body });
            }
            '}' | ']' => return Err(format!("stray `{ch}`")),
            _ => {
                text.push(ch);
                *i += 1;
            }
        }
    }
    flush(&mut text, &mut out);
    Ok(out)
}

fn parse_brace(c: &[char], i: &mut usize) -> Result<Node, String> {
    if c.get(*i) == Some(&'~') {
        *i += 1;
        let mut alts = Vec::new();
        loop {
            alts.push(parse_seq(c, i, Some('|'))?);
            match c.get(*i) {
                Some('|') => *i += 1,
                Some('}') => {
                    *i += 1;
                    break;
                }
                _ => return Err("unterminated {~...}".into()),
            }
        }
        return Ok(Node::Alt(alts));
    }
    let start = *i;
    while *i < c.len() && c[*i] != '}' {
        *i += 1;
    }
    if *i >= c.len() {
        return Err("unterminated {".into());
    }
    let inner: String = c[start..*i].iter().collect();
    *i += 1;
    let inner = inner.trim();
    if inner == "clause" {
        return Ok(Node::Clause);
    }
    if let Some(rest) = inner.strip_prefix("lex:") {
        let (concept, form) = rest.split_once('.').ok_or_else(|| format!("{{lex:{rest}}} needs concept.form"))?;
        return Ok(Node::Lex { concept: concept.into(), form: form.into() });
    }
    if let Some(rest) = inner.strip_prefix("col:") {
        let mut it = rest.splitn(3, '.');
        let head = it.next().unwrap_or("");
        let slot = it.next().ok_or_else(|| format!("{{col:{rest}}} needs head.slot"))?;
        let form = it.next().unwrap_or("base");
        return Ok(Node::Col { head: head.into(), slot: slot.into(), form: form.into() });
    }
    if let Some(rest) = inner.strip_prefix("cert:") {
        return Ok(Node::Cert(rest.into()));
    }
    if inner.is_empty() {
        return Err("empty {}".into());
    }
    let (key, mode) = inner.split_once(':').unwrap_or((inner, ""));
    Ok(Node::Field { key: key.trim().into(), mode: mode.trim().into() })
}

/// Fact keys a template requires unconditionally (outside optional segments), for lint.
pub fn required_keys(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
        match n {
            Node::Field { key, .. } => out.push(key.clone()),
            Node::Alt(alts) => {
                for a in alts {
                    required_keys(a, out);
                }
            }
            _ => {}
        }
    }
}

/// Every fact key mentioned anywhere, optional or not.
pub fn all_keys(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
        match n {
            Node::Field { key, .. } => out.push(key.clone()),
            Node::Opt { key, body } => {
                out.push(key.clone());
                all_keys(body, out);
            }
            Node::Alt(alts) => {
                for a in alts {
                    all_keys(a, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_form() {
        let t = parse("{buyer} {lex:reject.pp} {~a|b {x}} [?fee: for {fee:words}] {clause}{cert:source}{col:bid.adj}").unwrap();
        assert!(matches!(t[0], Node::Field { .. }));
        assert!(t.iter().any(|n| matches!(n, Node::Alt(a) if a.len() == 2)));
        assert!(t.iter().any(|n| matches!(n, Node::Opt { key, .. } if key == "fee")));
        assert!(parse("{oops").is_err());
        assert!(parse("stray }").is_err());
    }
}
