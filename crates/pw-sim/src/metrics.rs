//! Long-run balance metrics. A world that runs without crashing can still be drifting: money inflating, fame saturating, saves growing
//! without bound. `observe` runs a world year by year and takes a [`Snapshot`] of the quantities that reveal that; `analyse` reads the
//! series for trends rather than single bad values.

use pw_core::Date;
use pw_world::{EventKind, PlayerStatus, StaffRole, World};

use crate::Sim;

/// One year-end reading. Money is in the world's currency; "period" means since the previous snapshot.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub year: u32,
    pub date: i32,
    // population
    pub active_players: usize,
    pub free_agents: usize,
    pub amateurs: usize,
    /// Players registered with a club's first team, its reserves, and its youth sides (U16-U21).
    pub in_first_teams: usize,
    pub in_reserves: usize,
    pub in_youth_sides: usize,
    /// Players aged 22+ who are not in a first team (still in reserve or youth sides).
    pub adults_below_first_team: usize,
    /// Mean and largest first-team squad.
    pub squad_mean: f32,
    pub squad_max: usize,
    pub mean_age: f32,
    pub mean_ca: f32,
    pub p99_ca: f32,
    pub retirements: u32,
    pub youth_intakes: u32,
    // clubs and money
    pub clubs: usize,
    pub balance_p10: f64,
    pub balance_median: f64,
    pub balance_p90: f64,
    pub clubs_in_debt: usize,
    pub revenue_median: f64,
    pub wage_to_revenue_median: f32,
    pub wage_to_revenue_p90: f32,
    pub player_wage_median: f64,
    pub player_wage_p99: f64,
    // market
    pub transfers: u32,
    pub loans: u32,
    pub fees_total: f64,
    pub fee_median: f64,
    pub fee_p90: f64,
    pub fee_max: f64,
    // fame and reputation
    pub fame_mean: f32,
    pub fame_p99: f32,
    pub fame_saturated: f32,
    pub club_rep_p90: f32,
    pub club_rep_saturated: f32,
    // people
    pub managers_employed: usize,
    pub managers_unemployed: usize,
    /// Clubs with nobody in the manager's chair at the snapshot.
    pub clubs_without_manager: usize,
    pub sackings: u32,
    pub staff_total: usize,
    pub staff_unemployed: usize,
    // memory and growth
    pub records: usize,
    pub stories: usize,
    pub posts: usize,
    pub accounts: usize,
    pub events: usize,
    pub save_bytes: u64,
}

fn pct<T: Copy + PartialOrd>(v: &mut [T], q: f32) -> Option<T> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(v[((v.len() - 1) as f32 * q) as usize])
}

/// Read the world now. `since` is the previous snapshot's date (events in between are counted as the period's activity).
pub fn snapshot(w: &World, since: Date, year: u32) -> Snapshot {
    let mut s = Snapshot { year, date: w.date.0, ..Default::default() };
    // Players.
    let (mut ages, mut cas, mut wages, mut fame): (Vec<f32>, Vec<f32>, Vec<f64>, Vec<f32>) = (vec![], vec![], vec![], vec![]);
    for p in w.players.ids() {
        let h = &w.players.hot[p];
        match h.status {
            PlayerStatus::Active => {
                let c = &w.players.cold[p];
                ages.push(w.age_years(p));
                cas.push(f32::from(c.ca));
                fame.push(f32::from(c.rep.world));
                let wage = c.contract.current_wage(w.date);
                if wage > 0 {
                    wages.push(wage as f64);
                }
                s.active_players += 1;
                if h.team.is_some() {
                    match w.teams[h.team].kind {
                        pw_world::TeamKind::First => s.in_first_teams += 1,
                        pw_world::TeamKind::Reserve => s.in_reserves += 1,
                        _ => s.in_youth_sides += 1,
                    }
                    if w.age(p) >= 22 && w.teams[h.team].kind != pw_world::TeamKind::First {
                        s.adults_below_first_team += 1;
                    }
                }
            }
            PlayerStatus::FreeAgent => s.free_agents += 1,
            PlayerStatus::Amateur => s.amateurs += 1,
            _ => {}
        }
    }
    let sizes: Vec<usize> = w.teams.iter().filter(|t| t.kind == pw_world::TeamKind::First).map(|t| t.squad.len()).collect();
    s.squad_mean = sizes.iter().sum::<usize>() as f32 / sizes.len().max(1) as f32;
    s.squad_max = sizes.iter().copied().max().unwrap_or(0);
    let mean = |v: &[f32]| if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 };
    s.mean_age = mean(&ages);
    s.mean_ca = mean(&cas);
    s.p99_ca = pct(&mut cas.clone(), 0.99).unwrap_or(0.0);
    s.player_wage_median = pct(&mut wages.clone(), 0.5).unwrap_or(0.0);
    s.player_wage_p99 = pct(&mut wages, 0.99).unwrap_or(0.0);
    s.fame_mean = mean(&fame);
    s.fame_p99 = pct(&mut fame.clone(), 0.99).unwrap_or(0.0);
    s.fame_saturated = fame.iter().filter(|&&f| f >= 9_000.0).count() as f32 / fame.len().max(1) as f32;
    // Clubs.
    let (mut balances, mut ratios, mut reps, mut revenues): (Vec<f64>, Vec<f32>, Vec<f32>, Vec<f64>) = (vec![], vec![], vec![], vec![]);
    for c in w.clubs.ids() {
        let f = &w.clubs[c].finance;
        balances.push(f.balance as f64);
        if f.debt > 0 || f.balance < 0 {
            s.clubs_in_debt += 1;
        }
        let revenue = crate::finance::season_revenue(w, c).max(1) as f32;
        revenues.push(revenue as f64);
        ratios.push(f.wage_bill as f32 * 52.0 / revenue);
        reps.push(f32::from(w.clubs[c].reputation));
    }
    s.clubs = w.clubs.len();
    s.balance_p10 = pct(&mut balances.clone(), 0.1).unwrap_or(0.0);
    s.balance_median = pct(&mut balances.clone(), 0.5).unwrap_or(0.0);
    s.balance_p90 = pct(&mut balances, 0.9).unwrap_or(0.0);
    s.revenue_median = pct(&mut revenues, 0.5).unwrap_or(0.0);
    s.wage_to_revenue_median = pct(&mut ratios.clone(), 0.5).unwrap_or(0.0);
    s.wage_to_revenue_p90 = pct(&mut ratios, 0.9).unwrap_or(0.0);
    s.club_rep_p90 = pct(&mut reps.clone(), 0.9).unwrap_or(0.0);
    s.club_rep_saturated = reps.iter().filter(|&&r| r >= 9_500.0).count() as f32 / reps.len().max(1) as f32;
    s.clubs_without_manager = w.clubs.iter().filter(|c| c.manager.is_none()).count();
    // Staff.
    for st in w.staff.iter() {
        if st.retired {
            continue;
        }
        s.staff_total += 1;
        if !st.employed() {
            s.staff_unemployed += 1;
        }
        if st.role == StaffRole::Manager {
            if st.employed() {
                s.managers_employed += 1;
            } else {
                s.managers_unemployed += 1;
            }
        }
    }
    // Activity in the period.
    let mut fees: Vec<f64> = Vec::new();
    for e in w.events.since(since) {
        match e.kind {
            EventKind::Transfer { fee, .. } => {
                s.transfers += 1;
                if fee > 0 {
                    fees.push(fee as f64);
                }
            }
            EventKind::LoanMove { .. } => s.loans += 1,
            EventKind::Retired { .. } => s.retirements += 1,
            EventKind::YouthIntake { .. } => s.youth_intakes += 1,
            EventKind::ManagerSacked { .. } => s.sackings += 1,
            _ => {}
        }
    }
    s.fees_total = fees.iter().sum();
    s.fee_median = pct(&mut fees.clone(), 0.5).unwrap_or(0.0);
    s.fee_p90 = pct(&mut fees.clone(), 0.9).unwrap_or(0.0);
    s.fee_max = pct(&mut fees, 1.0).unwrap_or(0.0);
    // Memory and growth.
    s.records = w.records.records.len();
    s.stories = w.media.stories.len();
    s.posts = w.net.posts.len();
    s.accounts = w.net.accounts.len();
    s.events = w.events.len();
    s.save_bytes = bincode::serialized_size(w).unwrap_or(0);
    s
}

/// Run `years` seasons of 365 days from the world's current date, calling `each` after every year with that year's snapshot.
pub fn observe(sim: &mut Sim, years: u32, mut each: impl FnMut(&Snapshot)) -> Vec<Snapshot> {
    let mut out = Vec::new();
    let mut since = sim.world.date;
    out.push(snapshot(&sim.world, since, 0));
    each(&out[0]);
    for y in 1..=years {
        sim.run(365);
        let s = snapshot(&sim.world, since, y);
        since = sim.world.date;
        each(&s);
        out.push(s);
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Warn,
    Problem,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub level: Level,
    pub series: &'static str,
    pub message: String,
}

/// Compound yearly growth of a positive series, from the first full year (year 1) to the last, in percent per year.
fn yearly_growth(series: &[f64]) -> Option<f64> {
    let (first, last) = (*series.get(1)?, *series.last()?);
    let years = (series.len() - 2) as f64;
    (first > 0.0 && last > 0.0 && years >= 1.0).then(|| ((last / first).powf(1.0 / years) - 1.0) * 100.0)
}

/// Read a run's snapshots for trends. Needs at least four snapshots (year 0 plus three years) to say anything about direction.
pub fn analyse(run: &[Snapshot]) -> Vec<Finding> {
    let mut out = Vec::new();
    if run.len() < 4 {
        return out;
    }
    // A young world fills up first (fame accrues, children grow into the amateur game), so trends are read after a warm-up of a third of
    // the run; `yearly_growth` skips the first element, so each series starts one year early.
    let warm = ((run.len() - 1) / 3).max(1);
    let series = |f: &dyn Fn(&Snapshot) -> f64| run[warm - 1..].iter().map(f).collect::<Vec<f64>>();
    let mut flag = |level: Level, name: &'static str, msg: String| out.push(Finding { level, series: name, message: msg });
    let last = run.last().expect("non-empty");

    // Money: drift, debt, wages against revenue, fee inflation.
    if let Some(g) = yearly_growth(&series(&|s| s.balance_median)) {
        if g > 30.0 {
            flag(Level::Problem, "club balances", format!("median club balance grows {g:.0}% a year: money is being created faster than it is spent"));
        } else if g < -30.0 {
            flag(Level::Problem, "club balances", format!("median club balance falls {:.0}% a year: clubs are being bled", -g));
        }
    }
    if last.clubs_in_debt * 2 > last.clubs {
        flag(Level::Problem, "debt", format!("{} of {} clubs are in debt at the end", last.clubs_in_debt, last.clubs));
    } else if run[1].clubs > 0 && last.clubs_in_debt > run[1].clubs_in_debt * 2 + 3 {
        flag(Level::Warn, "debt", format!("clubs in debt rose from {} to {}", run[1].clubs_in_debt, last.clubs_in_debt));
    }
    if last.wage_to_revenue_median > 1.0 {
        flag(Level::Problem, "wages", format!("the median club pays {:.0}% of its revenue in wages", last.wage_to_revenue_median * 100.0));
    } else if last.wage_to_revenue_median > 0.85 {
        flag(Level::Warn, "wages", format!("the median club pays {:.0}% of its revenue in wages", last.wage_to_revenue_median * 100.0));
    }
    if let Some(g) = yearly_growth(&series(&|s| s.player_wage_median)) {
        if g > 12.0 {
            flag(Level::Problem, "wages", format!("median player wage inflates {g:.0}% a year"));
        } else if g > 6.0 {
            flag(Level::Warn, "wages", format!("median player wage inflates {g:.0}% a year"));
        }
    }
    if let Some(g) = yearly_growth(&series(&|s| s.fee_median)) {
        if g > 15.0 {
            flag(Level::Problem, "fees", format!("median transfer fee inflates {g:.0}% a year"));
        } else if g > 8.0 {
            flag(Level::Warn, "fees", format!("median transfer fee inflates {g:.0}% a year"));
        }
    }
    // Fame and reputation saturation.
    if last.fame_saturated > 0.05 {
        flag(Level::Problem, "fame", format!("{:.0}% of players have world fame of 9,000 or more: fame is saturating", last.fame_saturated * 100.0));
    } else if last.fame_saturated > run[1].fame_saturated * 2.0 + 0.01 {
        flag(Level::Warn, "fame", format!("famous players rose from {:.1}% to {:.1}%", run[1].fame_saturated * 100.0, last.fame_saturated * 100.0));
    }
    if last.club_rep_saturated > 0.10 {
        flag(Level::Warn, "club reputation", format!("{:.0}% of clubs sit at the reputation ceiling", last.club_rep_saturated * 100.0));
    }
    // Ability drift.
    let drift = last.mean_ca - run[warm].mean_ca;
    if drift.abs() > 8.0 {
        flag(Level::Problem, "ability", format!("mean ability moved {drift:+.1} over the run: the population is inflating or deflating"));
    } else if drift.abs() > 4.0 {
        flag(Level::Warn, "ability", format!("mean ability moved {drift:+.1} over the run"));
    }
    // Population and turnover.
    let pop = (last.active_players as f64 / run[1].active_players.max(1) as f64 - 1.0) * 100.0 / (run.len() - 2) as f64;
    if pop.abs() > 10.0 {
        flag(Level::Problem, "population", format!("the active player count changes {pop:+.0}% a year"));
    } else if pop.abs() > 4.0 {
        flag(Level::Warn, "population", format!("the active player count changes {pop:+.0}% a year"));
    }
    let turnover = last.retirements as f64 / last.active_players.max(1) as f64;
    if !(0.02..=0.16).contains(&turnover) {
        flag(Level::Warn, "retirement", format!("{:.1}% of players retired this year", turnover * 100.0));
    }
    if last.managers_employed > 0 && last.managers_unemployed as f64 > last.managers_employed as f64 * 3.0 {
        flag(Level::Warn, "managers", format!("{} unemployed managers for {} jobs: the pool is filling up", last.managers_unemployed, last.managers_employed));
    }
    if last.clubs_without_manager * 10 > last.clubs {
        flag(Level::Problem, "managers", format!("{} of {} clubs have no manager", last.clubs_without_manager, last.clubs));
    }
    if run[1].managers_unemployed > 0 && last.managers_unemployed == 0 && last.sackings > 0 {
        flag(Level::Problem, "managers", "the pool of unemployed managers ran dry".to_string());
    }
    let s0 = run[1].staff_unemployed.max(1) as f64;
    if last.staff_unemployed as f64 > s0 * 3.0 + 50.0 {
        flag(Level::Warn, "staff", format!("unemployed staff grew from {} to {}", run[1].staff_unemployed, last.staff_unemployed));
    }
    // Growth of what is kept.
    if let Some(g) = yearly_growth(&series(&|s| s.save_bytes as f64)) {
        if g > 40.0 {
            flag(Level::Problem, "save size", format!("the world's size grows {g:.0}% a year"));
        } else if g > 20.0 {
            flag(Level::Warn, "save size", format!("the world's size grows {g:.0}% a year"));
        }
    }
    for (name, f) in [("stories", (&|s: &Snapshot| s.stories as f64) as &dyn Fn(&Snapshot) -> f64), ("posts", &|s| s.posts as f64), ("events", &|s| s.events as f64)] {
        // Cumulative logs grow linearly; only a rate that itself keeps climbing is a problem.
        let v: Vec<f64> = run.iter().map(f).collect();
        let per_year: Vec<f64> = v.windows(2).map(|p| p[1] - p[0]).collect();
        if per_year.len() >= 3 && per_year[0] > 0.0 {
            let rate = per_year[per_year.len() - 1] / per_year[1].max(1.0);
            if rate > 2.5 {
                flag(Level::Warn, name, format!("yearly additions to {name} are {rate:.1}x what they were in year 2"));
            }
        }
    }
    out
}

fn money(v: f64) -> String {
    let a = v.abs();
    let s = if a >= 1e9 { format!("{:.1}b", a / 1e9) } else if a >= 1e6 { format!("{:.1}m", a / 1e6) } else if a >= 1e3 { format!("{:.0}k", a / 1e3) } else { format!("{a:.0}") };
    if v < 0.0 { format!("-{s}") } else { s }
}

/// A plain-text table of the run, one row per year.
pub fn render(run: &[Snapshot]) -> String {
    let mut s = String::new();
    s.push_str("year  active  first  resv  youth  adult<1st  free  amat  sqd  sqdMax  age  meanCA  balMed   revMed   inDebt  wage/rev  wageMed  fee50  fee90   feeMax  xfers  loans  retire  intake  fame99  famSat  mgrs(u)  saveMB\n");
    for r in run {
        s.push_str(&format!(
            "{:>4} {:>7} {:>6} {:>5} {:>6} {:>9} {:>5} {:>5} {:>5.1} {:>6} {:>4.1} {:>7.1} {:>7} {:>8} {:>7} {:>8.2} {:>8} {:>6} {:>6} {:>8} {:>6} {:>6} {:>7} {:>7} {:>7.0} {:>6.1}% {:>4}({:<3}) {:>7.1}\n",
            r.year,
            r.active_players,
            r.in_first_teams,
            r.in_reserves,
            r.in_youth_sides,
            r.adults_below_first_team,
            r.free_agents,
            r.amateurs,
            r.squad_mean,
            r.squad_max,
            r.mean_age,
            r.mean_ca,
            money(r.balance_median),
            money(r.revenue_median),
            r.clubs_in_debt,
            r.wage_to_revenue_median,
            money(r.player_wage_median),
            money(r.fee_median),
            money(r.fee_p90),
            money(r.fee_max),
            r.transfers,
            r.loans,
            r.retirements,
            r.youth_intakes,
            r.fame_p99,
            r.fame_saturated * 100.0,
            r.managers_employed,
            r.managers_unemployed,
            r.save_bytes as f64 / 1e6,
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(years: usize) -> Vec<Snapshot> {
        (0..years)
            .map(|y| Snapshot {
                year: y as u32,
                active_players: 1000,
                clubs: 50,
                balance_median: 5e6,
                player_wage_median: 5000.0,
                fee_median: 1e6,
                mean_ca: 100.0,
                wage_to_revenue_median: 0.6,
                retirements: 80,
                managers_employed: 50,
                managers_unemployed: 10,
                staff_unemployed: 20,
                save_bytes: 50_000_000,
                stories: 1000 * (y + 1),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn a_steady_world_raises_nothing() {
        assert!(analyse(&flat(8)).is_empty(), "{:?}", analyse(&flat(8)));
    }

    #[test]
    fn too_short_a_run_says_nothing_about_direction() {
        assert!(analyse(&flat(3)).is_empty());
    }

    #[test]
    fn money_inflating_wages_fees_and_size_are_caught_as_trends_not_single_values() {
        let mut run = flat(8);
        for (i, s) in run.iter_mut().enumerate() {
            let k = 1.5f64.powi(i as i32);
            s.balance_median *= k;
            s.player_wage_median *= 1.25f64.powi(i as i32);
            s.fee_median *= 1.3f64.powi(i as i32);
            s.save_bytes = (50_000_000.0 * 1.6f64.powi(i as i32)) as u64;
        }
        let f = analyse(&run);
        for series in ["club balances", "wages", "fees", "save size"] {
            assert!(f.iter().any(|x| x.series == series && x.level == Level::Problem), "{series} not flagged in {f:?}");
        }
    }

    #[test]
    fn saturation_debt_and_a_dry_manager_pool_are_caught() {
        let mut run = flat(6);
        let last = run.last_mut().unwrap();
        last.fame_saturated = 0.2;
        last.clubs_in_debt = 40;
        last.wage_to_revenue_median = 1.2;
        last.managers_unemployed = 0;
        last.sackings = 5;
        let f = analyse(&run);
        for series in ["fame", "debt", "wages", "managers"] {
            assert!(f.iter().any(|x| x.series == series && x.level == Level::Problem), "{series} not flagged in {f:?}");
        }
    }

    #[test]
    fn a_log_that_grows_steadily_is_fine_but_one_that_accelerates_is_not() {
        assert!(analyse(&flat(8)).iter().all(|f| f.series != "stories"));
        let mut run = flat(8);
        for (i, s) in run.iter_mut().enumerate() {
            s.stories = (1000.0 * 1.9f64.powi(i as i32)) as usize;
        }
        assert!(analyse(&run).iter().any(|f| f.series == "stories"));
    }

    #[test]
    fn growth_is_measured_from_the_first_full_year() {
        assert!((yearly_growth(&[1.0, 100.0, 110.0, 121.0]).unwrap() - 10.0).abs() < 1e-6);
        assert!(yearly_growth(&[1.0, 0.0, 5.0, 6.0]).is_none());
    }
}
