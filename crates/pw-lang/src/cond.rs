//! The tiny condition language of `when`: `key`, `!key`, `key == 'x'`, `n >= 2`, joined with `&&`. Unknown keys make a condition false.

use crate::model::{Cmp, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Term {
    Known(String),
    NotKnown(String),
    Cmp(String, Op, Cmp),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
}

pub fn parse(src: &str) -> Result<Vec<Term>, String> {
    let mut out = Vec::new();
    for part in src.split("&&") {
        let p = part.trim();
        if p.is_empty() {
            return Err("empty condition".into());
        }
        let ops = [("==", Op::Eq), ("!=", Op::Ne), (">=", Op::Ge), ("<=", Op::Le), (">", Op::Gt), ("<", Op::Lt)];
        if let Some((sym, op)) = ops.iter().find(|(s, _)| p.contains(s)) {
            let (k, v) = p.split_once(sym).unwrap();
            let (k, v) = (k.trim(), v.trim());
            let lit = if let Some(s) = v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
                Cmp::S(s.to_string())
            } else if let Ok(n) = v.parse::<f64>() {
                Cmp::N(n)
            } else if v == "true" || v == "false" {
                Cmp::S(v.to_string())
            } else {
                return Err(format!("bad literal {v:?}"));
            };
            out.push(Term::Cmp(k.to_string(), *op, lit));
        } else if let Some(k) = p.strip_prefix('!') {
            out.push(Term::NotKnown(k.trim().to_string()));
        } else {
            out.push(Term::Known(p.to_string()));
        }
    }
    Ok(out)
}

/// `facts` holds only what the speaker knows.
pub fn eval(src: &str, facts: &BTreeMap<String, Value>) -> bool {
    let Ok(terms) = parse(src) else { return false };
    terms.iter().all(|t| match t {
        Term::Known(k) => facts.contains_key(k),
        Term::NotKnown(k) => !facts.contains_key(k),
        Term::Cmp(k, op, lit) => {
            let Some(v) = facts.get(k) else { return false };
            match (v.as_cmp(), lit) {
                (Cmp::N(a), Cmp::N(b)) => match op {
                    Op::Eq => a == *b,
                    Op::Ne => a != *b,
                    Op::Ge => a >= *b,
                    Op::Le => a <= *b,
                    Op::Gt => a > *b,
                    Op::Lt => a < *b,
                },
                (Cmp::S(a), Cmp::S(b)) => match op {
                    Op::Eq => a == *b,
                    Op::Ne => a != *b,
                    _ => false,
                },
                _ => false,
            }
        }
    })
}

/// Fact keys a condition looks at.
pub fn keys(src: &str) -> Vec<String> {
    parse(src)
        .unwrap_or_default()
        .into_iter()
        .map(|t| match t {
            Term::Known(k) | Term::NotKnown(k) | Term::Cmp(k, _, _) => k,
        })
        .collect()
}
