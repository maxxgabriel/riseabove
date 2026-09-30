//! Facts computed from other facts of the same event, so frames can talk about "a draw" or "a two-goal margin" without each channel redoing the sums.

use crate::model::{Certainty, Know, Value};
use std::collections::BTreeMap;

pub fn derived_keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "match.result" => &["total", "margin", "result_kind", "winner", "loser", "winner_goals", "loser_goals"],
        _ => &[],
    }
}

/// Adds derived facts to what a speaker knows. A derived fact is exactly as certain as the weakest input.
pub fn apply(kind: &str, facts: &mut BTreeMap<String, Value>, knows: &mut BTreeMap<String, Know>) {
    if kind != "match.result" {
        return;
    }
    let inputs = ["home", "away", "home_goals", "away_goals"];
    if !inputs.iter().all(|k| facts.contains_key(*k) && knows.contains_key(*k)) {
        return;
    }
    let cert = Certainty::combine(inputs.iter().map(|k| knows[*k].certainty));
    if matches!(cert, Certainty::Unknown | Certainty::Denial | Certainty::Correction) {
        return;
    }
    let src = inputs.iter().filter(|k| knows[**k].certainty == cert).find_map(|k| knows[*k].source.clone());
    let (Value::Num(hg), Value::Num(ag)) = (facts["home_goals"].clone(), facts["away_goals"].clone()) else { return };
    let (home, away) = (facts["home"].clone(), facts["away"].clone());
    let know = Know { certainty: cert, source: src };
    let mut put = |k: &str, v: Value| {
        facts.insert(k.into(), v);
        knows.insert(k.into(), know.clone());
    };
    put("total", Value::Num(hg + ag));
    put("margin", Value::Num((hg - ag).abs()));
    if hg == ag {
        put("result_kind", Value::Text("draw".into()));
    } else {
        let (w, l, wg, lg) = if hg > ag { (home, away, hg, ag) } else { (away, home, ag, hg) };
        put("result_kind", Value::Text("win".into()));
        put("winner", w);
        put("loser", l);
        put("winner_goals", Value::Num(wg));
        put("loser_goals", Value::Num(lg));
    }
}
