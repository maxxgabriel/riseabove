//! FULL tier: the economy over a decade. A world that runs without crashing can still be inflating: revenue, wages and fees compounding
//! faster than prices, clubs hoarding or bleeding cash, the pool of unemployed staff growing for ever, and every club "breaking its
//! club record" each time it pays a little more than last time. These tests run whole worlds for years and read the money series
//! (`pw_sim::metrics`) against bounds that football reality gives, generous enough that seed noise cannot trip them and tight enough
//! that the drifts found in the long soaks cannot come back (before the fix: revenue x7 and wages x10 in 15 years, 30 or more
//! club-record stories a year in a small world, unemployed staff from 7 to 144).
//!
//! What "healthy" means here:
//! * revenue and wages follow general inflation plus modest real growth: well under 10% a year, nominal;
//! * wages are about half of revenue for the median club, never above it;
//! * the median transfer fee grows with the same prices, not faster;
//! * no club sits on more than two seasons of revenue in cash and few are in administration;
//! * the unemployed in the backroom stay a pool the job market can work through;
//! * a club record signing or sale is an event, not a weekly occurrence.

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_sim::metrics::{self, Level, Snapshot};

fn run(scale: Scale, seed: u64, years: u32) -> Vec<Snapshot> {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), seed, scale));
    metrics::observe(&mut sim, years, |_| {})
}

/// Compound yearly growth in percent between two years of a positive series.
fn growth(series: &[f64], from: usize, to: usize) -> f64 {
    ((series[to] / series[from]).powf(1.0 / (to - from) as f64) - 1.0) * 100.0
}

/// `fee_from`: the year from which fee growth is read. The transfer market warms up more slowly in bigger worlds (clubs must first get
/// to know players, budgets fill, chains form), so the first years' cheap deals are not a base to measure growth from.
fn check_economy(run: &[Snapshot], label: &str, fee_from: usize) {
    let last = run.last().unwrap();
    let years = run.len() - 1;
    let (from, to) = (3, years);
    let revenue: Vec<f64> = run.iter().map(|s| s.revenue_total).collect();
    let wage_ft: Vec<f64> = run.iter().map(|s| s.first_team_wage_median).collect();
    let fees: Vec<f64> = run.iter().map(|s| s.fee_median).collect();

    let g = growth(&revenue, from, to);
    assert!((-6.0..=10.0).contains(&g), "{label}: total revenue grows {g:.1}% a year");
    let g = growth(&wage_ft, from, to);
    assert!((-6.0..=10.0).contains(&g), "{label}: median first-team wage grows {g:.1}% a year");
    // A median of few deals is noise: take the mean of the later years' medians against the earlier ones.
    let early: Vec<f64> = fees[fee_from..fee_from + 3].iter().copied().filter(|&f| f > 0.0).collect();
    let late: Vec<f64> = fees[to - 2..=to].iter().copied().filter(|&f| f > 0.0).collect();
    if !early.is_empty() && !late.is_empty() {
        let (e, l) = (early.iter().sum::<f64>() / early.len() as f64, late.iter().sum::<f64>() / late.len() as f64);
        let g = ((l / e).powf(1.0 / (to - fee_from - 1) as f64) - 1.0) * 100.0;
        assert!(g <= 14.0, "{label}: median transfer fee grows {g:.1}% a year");
    }

    // Whatever the warm-up, a fee is never a large part of what a club earns in a year: the median deal stays under a tenth of the
    // revenue of the average club (at the fixed point of the long runs it is about one twentieth).
    for s in &run[2..] {
        let per_club = s.revenue_total / s.clubs.max(1) as f64;
        assert!(s.fee_median <= per_club * 0.10, "{label}: year {} median fee {:.0} against revenue per club {:.0}", s.year, s.fee_median, per_club);
    }

    for s in &run[1..] {
        assert!((0.25..=0.85).contains(&s.wage_to_revenue_median), "{label}: year {} the median club pays {:.2} of its revenue in wages", s.year, s.wage_to_revenue_median);
        // One club after a windfall is an event; a tenth of the clubs is a hoard.
        assert!(s.clubs_hoarding * 10 <= s.clubs, "{label}: year {} {} of {} clubs hold more than two seasons of revenue in cash", s.year, s.clubs_hoarding, s.clubs);
        assert!(s.clubs_in_debt * 2 <= s.clubs, "{label}: year {} {} of {} clubs are in debt", s.year, s.clubs_in_debt, s.clubs);
        assert!(s.clubs_in_administration * 4 <= s.clubs, "{label}: year {} {} clubs in administration", s.year, s.clubs_in_administration);
        assert_eq!(s.structural_problems, 0, "{label}: year {} structural problems", s.year);
    }
    // Wage and revenue totals stay tied: the bill is never a multiple of what comes in.
    assert!(last.wage_bill_total <= last.revenue_total * 0.9, "{label}: wages {:.0} against revenue {:.0}", last.wage_bill_total, last.revenue_total);

    // The unemployed in the backroom: a pool, not a queue that grows without end.
    // A young world starts with almost nobody out of work and fills up (retired players looking for jobs, sacked coaches) until people
    // give up and jobs are found: measured from the fifth year, the pool is level, and at most a few per club at any time (before the
    // fix it climbed to 550 for 64 clubs and was still rising after fifteen years).
    let base = run[run.len().min(5) - 1].staff_unemployed.max(1);
    assert!(last.staff_unemployed <= base * 2 + 25, "{label}: unemployed staff grew from {base} (year 4) to {}", last.staff_unemployed);
    for s in &run[1..] {
        assert!(s.staff_unemployed <= s.clubs * 4 + 20, "{label}: year {} {} unemployed staff for {} clubs", s.year, s.staff_unemployed, s.clubs);
    }

    // Records: club record signings and sales are rare events (at most about one club in five a year, in any year).
    for s in &run[1..] {
        let stories = (s.record_signings + s.record_sales) as usize;
        assert!(stories * 5 <= s.clubs * 2 + 2, "{label}: year {} {stories} club-record stories for {} clubs", s.year, s.clubs);
    }
    assert!(run[1..].iter().map(|s| s.record_world_fees).sum::<u32>() <= 2 * years as u32, "{label}: world record fees every year");

    // Nothing the trend reader calls a problem in money, debt or structure. (Fees are read above, from the year the market has warmed
    // up: the reader's own warm-up is a third of the run, shorter than the market's in a big world.)
    let findings = metrics::analyse(run);
    for f in findings.iter().filter(|f| f.level == Level::Problem && ["wages", "club balances", "debt", "structure", "staff"].contains(&f.series)) {
        panic!("{label}: {} [{}] {}", "PROBLEM", f.series, f.message);
    }
}

#[test]
fn a_tiny_world_keeps_its_money_in_proportion_for_ten_years() {
    let run = run(Scale::TINY, 1, 10);
    check_economy(&run, "tiny seed 1", 5);
}

#[test]
fn a_small_two_tier_world_keeps_its_money_in_proportion_for_eight_years() {
    let run = run(Scale::SMALL, 3, 8);
    check_economy(&run, "small seed 3", 5);
    // The pyramid keeps its shape in money: the top tier pays more than the tier below, and more than the third.
    let last = run.last().unwrap();
    assert!(last.wage_tier1 > last.wage_tier2 * 1.2, "top-tier median wage {:.0} against second tier {:.0}", last.wage_tier1, last.wage_tier2);
}
