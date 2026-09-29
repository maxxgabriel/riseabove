//! Estimating what a source does not state. Every function here is a documented rule over imported values; the
//! assembler labels its results `Inferred` (or `Generated` when randomness is involved) in the world's origin book.
//!
//! Market value is the only ability signal some sources carry. It prices *expected* ability, so it runs ahead of
//! current ability for the young and behind it for the old:
//!
//! * `peak(value)`: the ability that value buys at prime age, by log-interpolating a fixed table;
//! * young players (< 27): current ability is a share of that peak, the share growing with age (`pw_sim::generate::ca_share_at`
//!   blended 35% toward 1, because value already prices part of the growth), and potential is the peak;
//! * players 30+: value falls faster than ability, so ability sits above `peak` by a few percent per year.

use pw_core::math::interp;
use pw_sim::generate::ca_share_at;

/// (market value in EUR, ability at prime age on the 1–200 scale).
const VALUE_TO_PEAK: [(f32, f32); 14] = [
    (25_000.0, 45.0),
    (50_000.0, 55.0),
    (100_000.0, 66.0),
    (250_000.0, 80.0),
    (500_000.0, 92.0),
    (1_000_000.0, 103.0),
    (2_500_000.0, 116.0),
    (5_000_000.0, 126.0),
    (10_000_000.0, 136.0),
    (20_000_000.0, 148.0),
    (40_000_000.0, 161.0),
    (80_000_000.0, 173.0),
    (150_000_000.0, 184.0),
    (250_000_000.0, 192.0),
];

const VALUE_ANTICIPATION: f32 = 0.35;

/// Ability at prime age that a market value pays for.
pub fn peak_from_value(value: f64) -> f32 {
    let pts: Vec<(f32, f32)> = VALUE_TO_PEAK.iter().map(|&(v, c)| (v.ln(), c)).collect();
    interp(&pts, (value.max(1.0) as f32).ln())
}

/// (current ability, potential) implied by a value at an age.
pub fn ability_from_value(value: f64, age: f32) -> (f32, f32) {
    let peak = peak_from_value(value);
    let share = ca_share_at(age);
    let young = share + (1.0 - share) * VALUE_ANTICIPATION;
    let old = interp(&[(29.0, 1.0), (31.0, 1.02), (33.0, 1.05), (36.0, 1.10), (40.0, 1.16)], age);
    let ca = if age < 27.0 { peak * young } else { peak * old };
    (ca.clamp(20.0, 195.0), peak.max(ca).clamp(20.0, 200.0))
}

/// Ability of a typical player at a club whose imported players have these values (their median), used only
/// for players whose own value is missing.
pub fn club_level(values: &mut [f64]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    Some(peak_from_value(values[values.len() / 2]))
}

/// Ability implied by a league's reputation when nothing else is known.
pub fn league_level(reputation: u16) -> f32 {
    60.0 + f32::from(reputation) / 10_000.0 * 70.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_to_ability_is_monotone_and_bounded() {
        let mut last = 0.0;
        for v in [10_000.0, 100_000.0, 350_000.0, 1e6, 5e6, 3e7, 2e8, 9e8] {
            let p = peak_from_value(v);
            assert!(p >= last, "{v}: {p} < {last}");
            assert!((45.0..=192.0).contains(&p));
            last = p;
        }
    }

    #[test]
    fn young_players_are_below_their_peak_and_old_players_above() {
        let v = 20_000_000.0;
        let (c18, p18) = ability_from_value(v, 18.0);
        let (c25, _) = ability_from_value(v, 25.0);
        let (c28, p28) = ability_from_value(v, 28.0);
        let (c35, _) = ability_from_value(v, 35.0);
        assert!(c18 < c25 && c25 < c28, "{c18} {c25} {c28}");
        assert!(p18 >= c18 && (p18 - p28).abs() < 1e-3, "potential is the value's peak whatever the age");
        assert!(c35 > c28, "an old player of the same value is better than a prime one");
    }

    #[test]
    fn median_level_needs_evidence() {
        assert_eq!(club_level(&mut []), None);
        let l = club_level(&mut [1e6, 3e5, 2e6]).unwrap();
        assert_eq!(l, peak_from_value(1e6));
    }
}
