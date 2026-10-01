//! Estimating what a source does not state (locked design §11). Every result is labelled `Inferred` (or `Generated` when there
//! is no player-specific evidence at all) in the world's origin book.
//!
//! Market value is **one signal among several and never the ability oracle**. It prices expected ability, mixes in age,
//! league economics and contract leverage, and would turn into a self-confirming loop if it alone decided ability. Ability is
//! therefore estimated from independent evidence combined by reliability:
//!
//! * the club's standing (a prior: what a player at this club is typically like);
//! * where the player sits in his own club's value order (a *relative* measure, so league price levels cancel out);
//! * minutes played over the last twelve months as a share of his club's most-used player (independent of price);
//! * senior international caps;
//! * the absolute value, age-adjusted, with the lowest weight because it carries the most distortion.
//!
//! The result is a distribution, not a number. The world's truth is then *sampled* from it with the world's seed, so two players
//! with the same evidence differ, outliers stay possible, and value never determines ability exactly. Personality traits are never
//! inferred from any of this (§11.10).

use pw_core::math::interp;
use pw_data::Market;
use pw_sim::generate::ca_share_at;

/// How much of the growth still to come is already priced into a young player's value.
const VALUE_ANTICIPATION: f32 = 0.35;

/// Ability at prime age that a market value pays for: the exact inverse of the world's own price curve, so that reading a price into
/// ability and pricing that ability again lands where it started (up to the age, contract and reputation factors).
pub fn peak_from_value(value: f64, m: &Market) -> f32 {
    (100.0 + ((value.max(1.0) as f32) / m.value_base).ln() / m.value_exp).clamp(35.0, 195.0)
}

/// Current ability and potential a value implies on its own at an age: the single-signal reading, used as one input only.
pub fn value_signal(value: f64, age: f32, m: &Market) -> (f32, f32) {
    let peak = peak_from_value(value, m);
    let share = ca_share_at(age);
    let young = share + (1.0 - share) * VALUE_ANTICIPATION;
    let old = interp(&[(29.0, 1.0), (31.0, 1.02), (33.0, 1.05), (36.0, 1.10), (40.0, 1.16)], age);
    let ca = if age < 27.0 { peak * young } else { peak * old };
    (ca.clamp(20.0, 195.0), peak.max(ca).clamp(20.0, 200.0))
}

/// Ability implied by a league's reputation when nothing else is known.
pub fn league_level(reputation: u16) -> f32 {
    60.0 + f32::from(reputation) / 10_000.0 * 70.0
}

/// Everything the importer may know about one player's level. Absent evidence stays `None`; nothing defaults to zero.
#[derive(Clone, Copy, Debug)]
pub struct Evidence {
    pub age: f32,
    pub value: Option<f64>,
    /// Rank among his club's valued players, 0 (lowest) to 1 (highest).
    pub value_rank_in_club: Option<f32>,
    /// Minutes in the last twelve months and that as a share (0..1) of his club's most-used player.
    pub minutes: Option<(u32, f32)>,
    pub caps: u16,
    /// The standard a typical player at his club has, on the ability scale.
    pub club_level: f32,
}

/// A posterior over a player's ability and potential.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    pub ca_mean: f32,
    pub ca_sd: f32,
    pub pa_mean: f32,
    pub pa_sd: f32,
    /// Whether any player-specific evidence went in (otherwise this is the club prior alone).
    pub informed: bool,
}

const PRIOR_WEIGHT: f32 = 0.30;
const PRIOR_SD: f32 = 16.0;

pub fn estimate(e: &Evidence, m: &Market) -> Estimate {
    let mu0 = e.club_level;
    let mut sum = PRIOR_WEIGHT * mu0;
    let mut weight = PRIOR_WEIGHT;
    let mut add = |mu: f32, w: f32| {
        sum += mu * w;
        weight += w;
    };
    let mut informed = false;
    if let Some(r) = e.value_rank_in_club {
        add(mu0 + 28.0 * (r.clamp(0.0, 1.0) - 0.5), 0.25);
        informed = true;
    }
    if let Some((minutes, share)) = e.minutes {
        // A few minutes say little; a season of them says a lot.
        let reliability = (minutes as f32 / 1200.0).min(1.0);
        add(mu0 + 30.0 * (share.clamp(0.0, 1.0) - 0.45), 0.35 * reliability);
        informed |= reliability > 0.0;
    }
    if e.caps >= 5 {
        add(mu0 + if e.caps >= 30 { 14.0 } else if e.caps >= 10 { 9.0 } else { 5.0 }, 0.10);
        informed = true;
    }
    let mut value_pa = None;
    if let Some(v) = e.value {
        let (ca, _) = value_signal(v, e.age, m);
        add(ca, 0.25);
        value_pa = Some(peak_from_value(v, m));
        informed = true;
    }
    let ca_mean = (sum / weight).clamp(20.0, 195.0);
    let ca_sd = PRIOR_SD / (1.0 + 4.0 * (weight - PRIOR_WEIGHT)).sqrt();

    // Potential: what a player of this age and level typically grows to, and — for the young, whose price includes the market's
    // view of them — the market's expectation with a modest weight.
    let typical = if e.age < 27.0 { ca_mean / ca_share_at(e.age) } else { ca_mean };
    let market_weight = value_pa.map_or(0.0, |_| 0.4 * (1.0 - (e.age - 17.0) / 8.0).clamp(0.0, 1.0));
    let pa_mean = ((1.0 - market_weight) * typical + market_weight * value_pa.unwrap_or(typical)).clamp(ca_mean, 200.0);
    let pa_sd = if e.age < 24.0 { 13.0 } else if e.age < 28.0 { 8.0 } else { 4.0 };
    Estimate { ca_mean, ca_sd, pa_mean, pa_sd, informed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Evidence {
        Evidence { age: 25.0, value: None, value_rank_in_club: None, minutes: None, caps: 0, club_level: 110.0 }
    }

    #[test]
    fn value_alone_is_monotone_and_bounded() {
        let mut last = 0.0;
        for v in [10_000.0, 100_000.0, 350_000.0, 1e6, 5e6, 3e7, 2e8, 9e8] {
            let p = peak_from_value(v, &Market::default());
            assert!(p >= last, "{v}: {p} < {last}");
            assert!((35.0..=195.0).contains(&p));
            last = p;
        }
    }

    #[test]
    fn reading_a_price_into_ability_inverts_the_worlds_own_price_curve() {
        // The importer reads prices into ability with a table; the world prices ability with a formula. If they drift apart, every
        // imported value jumps at the first monthly revaluation. Prime age, mid-contract, neutral reputation.
        let m = Market::default();
        for ca in [50.0f32, 70.0, 90.0, 110.0, 130.0, 150.0, 170.0, 185.0] {
            let price = f64::from(m.value_base * (m.value_exp * (ca - 100.0)).exp());
            let back = peak_from_value(price, &m);
            assert!((back - ca).abs() < 0.5, "ability {ca} is priced {price:.0} but reads back as {back:.0}");
        }
    }

    #[test]
    fn a_single_signal_reads_young_players_below_their_peak_and_old_ones_above() {
        let v = 20_000_000.0;
        let (c18, p18) = value_signal(v, 18.0, &Market::default());
        let (c25, _) = value_signal(v, 25.0, &Market::default());
        let (c28, p28) = value_signal(v, 28.0, &Market::default());
        let (c35, _) = value_signal(v, 35.0, &Market::default());
        assert!(c18 < c25 && c25 < c28, "{c18} {c25} {c28}");
        assert!(p18 >= c18 && (p18 - p28).abs() < 1e-3);
        assert!(c35 > c28);
    }

    #[test]
    fn with_no_player_specific_evidence_the_estimate_is_the_club_prior_and_says_so() {
        let e = estimate(&base(), &Market::default());
        assert!(!e.informed);
        assert!((e.ca_mean - 110.0).abs() < 1e-3);
        assert!((e.ca_sd - PRIOR_SD).abs() < 1e-3, "no evidence: the widest spread");
    }

    #[test]
    fn evidence_narrows_the_estimate_and_moves_it_the_right_way() {
        let blind = estimate(&base(), &Market::default());
        let star = estimate(&Evidence { value: Some(60e6), value_rank_in_club: Some(1.0), minutes: Some((2800, 1.0)), caps: 40, ..base() }, &Market::default());
        let squad = estimate(&Evidence { value: Some(400_000.0), value_rank_in_club: Some(0.1), minutes: Some((300, 0.1)), ..base() }, &Market::default());
        assert!(star.informed && squad.informed);
        assert!(star.ca_mean > blind.ca_mean && blind.ca_mean > squad.ca_mean, "{} {} {}", star.ca_mean, blind.ca_mean, squad.ca_mean);
        assert!(star.ca_sd < blind.ca_sd, "more evidence, less uncertainty");
    }

    #[test]
    fn price_alone_cannot_decide_ability_the_club_and_the_minutes_pull_against_it() {
        // Two players with the same value: one is a regular at a strong club, the other never plays at a weak one.
        let regular = estimate(&Evidence { value: Some(5e6), value_rank_in_club: Some(0.7), minutes: Some((2600, 0.95)), club_level: 125.0, ..base() }, &Market::default());
        let unused = estimate(&Evidence { value: Some(5e6), value_rank_in_club: Some(0.7), minutes: Some((150, 0.05)), club_level: 95.0, ..base() }, &Market::default());
        assert!(regular.ca_mean > unused.ca_mean + 15.0, "{} vs {}", regular.ca_mean, unused.ca_mean);
        // Whatever the price, the estimate stays within reach of the club's standard rather than tracking the price.
        let cheap = estimate(&Evidence { value: Some(100_000.0), club_level: 125.0, ..base() }, &Market::default());
        let dear = estimate(&Evidence { value: Some(80e6), club_level: 125.0, ..base() }, &Market::default());
        assert!((dear.ca_mean - cheap.ca_mean) < (peak_from_value(80e6, &Market::default()) - peak_from_value(100_000.0, &Market::default())) * 0.5, "value moves the estimate, but not one for one");
    }

    #[test]
    fn young_players_carry_headroom_and_the_market_view_of_them_raises_it() {
        let young = estimate(&Evidence { age: 18.0, value: Some(15e6), ..base() }, &Market::default());
        let old = estimate(&Evidence { age: 31.0, value: Some(15e6), ..base() }, &Market::default());
        assert!(young.pa_mean > young.ca_mean + 10.0, "{} {}", young.pa_mean, young.ca_mean);
        assert!((old.pa_mean - old.ca_mean).abs() < 1e-3, "a veteran has no headroom");
        assert!(young.pa_sd > old.pa_sd, "potential is the more uncertain of the two");
    }

    #[test]
    fn a_few_minutes_carry_little_weight() {
        let brief = estimate(&Evidence { minutes: Some((60, 1.0)), ..base() }, &Market::default());
        let season = estimate(&Evidence { minutes: Some((2700, 1.0)), ..base() }, &Market::default());
        assert!(season.ca_mean - base().club_level > 4.0 * (brief.ca_mean - base().club_level).max(0.1), "{} vs {}", season.ca_mean, brief.ca_mean);
    }
}
