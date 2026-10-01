//! Long-run balance metrics. A world that runs without crashing can still be drifting: money inflating, fame saturating, saves growing
//! without bound. `observe` runs a world year by year and takes a [`Snapshot`] of the quantities that reveal that; `analyse` reads the
//! series for trends rather than single bad values.

use pw_core::Date;
use pw_world::player::PlayerSource;
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
    /// Players created in the period, by `PlayerSource::ALL` order: every new person in the world has a stated origin.
    pub created: [u32; 7],
    /// Everyone ever created, retired or not. `created` summed over the run explains this; nothing appears without a source.
    pub players_total: usize,
    /// Everyone not retired: professionals, free agents, amateurs and grassroots children. Its change over a period is explained by
    /// `created` and `retirements` and nothing else (the population table prints the remainder).
    pub alive: usize,
    // clubs and money
    pub clubs: usize,
    pub balance_p10: f64,
    pub balance_median: f64,
    pub balance_p90: f64,
    pub clubs_in_debt: usize,
    pub revenue_median: f64,
    pub wage_to_revenue_median: f32,
    pub wage_to_revenue_p90: f32,
    /// Median weekly wage in first teams, separate from reserves and youth squads.
    pub first_team_wage_median: f64,
    /// Median weekly wage across all active contracted players.
    pub player_wage_median: f64,
    pub player_wage_p99: f64,
    /// Mean weekly first-team wage, and the median first-team wage of clubs in league tier 1, 2 and 3 or lower.
    pub first_team_wage_mean: f64,
    pub wage_tier1: f64,
    pub wage_tier2: f64,
    pub wage_tier3: f64,
    /// Sum of season revenue and of the wage bills (52 weeks) over all clubs: the two must not drift apart.
    pub revenue_total: f64,
    pub wage_bill_total: f64,
    /// Clubs that hold more than two seasons of revenue in cash (hoarding), and clubs in administration now.
    pub clubs_hoarding: usize,
    pub clubs_in_administration: usize,
    /// Administrations begun, in the period.
    pub insolvencies: u32,
    /// Club-record signings and sales, world-record fees, and all other records broken, in the period.
    pub record_signings: u32,
    pub record_sales: u32,
    pub record_world_fees: u32,
    pub records_other: u32,
    /// Mean best-14 ability of the clubs in tier-1 leagues (league quality).
    pub top_tier_quality: f32,
    /// The world's general price index, the mean national wage index, the summed top-flight broadcast pools, and the 90th-percentile club reputation.
    pub price_index: f32,
    pub wage_index: f32,
    pub broadcast_pools: f64,
    // market
    pub transfers: u32,
    pub loans: u32,
    pub fees_total: f64,
    /// Sellers that held a deal until they had a replacement, how many got it, how many collapsed for lack of it (per period).
    pub chain_waits: u32,
    pub chain_done: u32,
    pub chain_failed: u32,
    /// Bargaining: signals used, and deals in which something said was found to be false.
    pub signals: u32,
    pub bluffs_caught: u32,
    /// Important signings the club talked itself out of, and signings judged in the period.
    pub cases_declined: u32,
    pub cases_judged: u32,
    /// Players settling in now, players whose settling took far too long this period, and plans that failed this period.
    pub settling: usize,
    pub struggled: u32,
    pub plan_failures: u32,
    /// Clubs' appetite for risk: lowest, mean, highest.
    pub appetite_min: f32,
    pub appetite_mean: f32,
    pub appetite_max: f32,
    pub fee_median: f64,
    /// Median of fee over the market's estimate of the player now: what prices do, apart from who is being bought.
    pub fee_to_value: f64,
    /// Median years left on the seller's contract at the moment of a fee deal (set by `observe`): short when initial contracts run out,
    /// longer once renewals have matured. A price read from contract length moves with it.
    pub deal_contract_years: f64,
    /// Median of the seller-situation factor on the asking price at the moment of a fee deal (stance, expiry, listing, board): a fee
    /// over value that rises with it is the mix of who is sold, not a change of price.
    pub deal_asking_factor: f64,
    pub fee_p90: f64,
    pub fee_p95: f64,
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
    /// Six largest serialized world sections, included in balance reports to diagnose save growth.
    pub top_sections: String,
    /// Structural problems found by `validate::problems` (dangling ids, double registrations, insane finances).
    pub structural_problems: usize,
}

/// Uncompressed serialized size of each world section, largest first. Used by the save-growth diagnostics, not by the simulation.
pub fn section_sizes(w: &World) -> Vec<(&'static str, u64)> {
    sections(w, false)
}

/// The same sections after lz4 (what a save file holds for them, to within block framing).
pub fn section_sizes_lz4(w: &World) -> Vec<(&'static str, u64)> {
    sections(w, true)
}

fn measure<T: serde::Serialize>(lz4: bool, v: &T) -> u64 {
    if lz4 { lz4_flex::compress(&bincode::serialize(v).unwrap_or_default()).len() as u64 } else { bincode::serialized_size(v).unwrap_or(0) }
}

fn sections(w: &World, lz4: bool) -> Vec<(&'static str, u64)> {
    let mut sizes = vec![
        ("seed", measure(lz4, &w.seed)),
        ("date", measure(lz4, &w.date)),
        ("data", measure(lz4, &w.data)),
        ("names", measure(lz4, &w.names)),
        ("nations", measure(lz4, &w.nations)),
        ("people", measure(lz4, &w.people)),
        ("players", measure(lz4, &w.players)),
        ("staff", measure(lz4, &w.staff)),
        ("clubs", measure(lz4, &w.clubs)),
        ("teams", measure(lz4, &w.teams)),
        ("comps", measure(lz4, &w.comps)),
        ("fixtures", measure(lz4, &w.fixtures)),
        ("knowledge", measure(lz4, &w.knowledge)),
        ("events", measure(lz4, &w.events)),
        ("history", measure(lz4, &w.history)),
        ("stats", measure(lz4, &w.stats)),
        ("decisions", measure(lz4, &w.decisions)),
        ("market", measure(lz4, &w.market)),
        ("social", measure(lz4, &w.social)),
        ("talks", measure(lz4, &w.talks)),
        ("beliefs", measure(lz4, &w.beliefs)),
        ("lives", measure(lz4, &w.lives)),
        ("agents", measure(lz4, &w.agents)),
        ("media", measure(lz4, &w.media)),
        ("meetings", measure(lz4, &w.meetings)),
        ("intents", measure(lz4, &w.intents)),
        ("governance", measure(lz4, &w.governance)),
        ("economy", measure(lz4, &w.economy)),
        ("careers", measure(lz4, &w.careers)),
        ("scouting", measure(lz4, &w.scouting)),
        ("deals", measure(lz4, &w.deals)),
        ("youth", measure(lz4, &w.youth)),
        ("intl", measure(lz4, &w.intl)),
        ("medical", measure(lz4, &w.medical)),
        ("ext", measure(lz4, &w.ext)),
        ("rooms", measure(lz4, &w.rooms)),
        ("perf", measure(lz4, &w.perf)),
        ("growth", measure(lz4, &w.growth)),
        ("honours", measure(lz4, &w.honours)),
        ("renown", measure(lz4, &w.renown)),
        ("affairs", measure(lz4, &w.affairs)),
        ("commerce", measure(lz4, &w.commerce)),
        ("culture", measure(lz4, &w.culture)),
        ("grapevine", measure(lz4, &w.grapevine)),
        ("incidents", measure(lz4, &w.incidents)),
        ("agenda", measure(lz4, &w.agenda)),
        ("recent_matches", measure(lz4, &w.recent_matches)),
        ("pressroom", measure(lz4, &w.pressroom)),
        ("net", measure(lz4, &w.net)),
        ("inbox", measure(lz4, &w.inbox)),
        ("minor", measure(lz4, &w.minor)),
        ("records", measure(lz4, &w.records)),
        ("acclaim", measure(lz4, &w.acclaim)),
        ("officials", measure(lz4, &w.officials)),
        ("evolution", measure(lz4, &w.evolution)),
        ("backfill", measure(lz4, &w.backfill)),
        ("origins", measure(lz4, &w.origins)),
        ("reports", measure(lz4, &w.reports)),
        ("days_simulated", measure(lz4, &w.days_simulated)),
        ("followed", measure(lz4, &w.followed)),
        ("prepared", measure(lz4, &w.prepared)),
        ("playthrough", measure(lz4, &w.playthrough)),
        ("dossiers", measure(lz4, &w.dossiers)),
        ("boardroom", measure(lz4, &w.boardroom)),
        ("adaptation", measure(lz4, &w.adaptation)),
        ("tactics", measure(lz4, &w.tactics)),
        ("lifestate", measure(lz4, &w.lifestate)),
    ];
    sizes.sort_by(|a, b| b.1.cmp(&a.1));
    sizes
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
    // A period is (previous snapshot, now]: the day of the previous snapshot was counted in that one.
    let from = if year == 0 { since } else { Date(since.0 + 1) };
    s.created = w.players.created_by_source(from, Date(w.date.0 + 1));
    s.players_total = w.players.len();
    s.alive = w.players.ids().filter(|&p| w.players.hot[p].status != PlayerStatus::Retired).count();
    // Players.
    let (mut ages, mut cas, mut wages, mut fame): (Vec<f32>, Vec<f32>, Vec<f64>, Vec<f32>) = (vec![], vec![], vec![], vec![]);
    let mut first_team_wages: Vec<f64> = Vec::new();
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
                    if h.team.is_some() && w.teams[h.team].kind == pw_world::TeamKind::First {
                        first_team_wages.push(wage as f64);
                    }
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
    s.first_team_wage_mean = first_team_wages.iter().sum::<f64>() / first_team_wages.len().max(1) as f64;
    s.first_team_wage_median = pct(&mut first_team_wages, 0.5).unwrap_or(0.0);
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
    s.revenue_total = revenues.iter().sum();
    s.wage_bill_total = w.clubs.ids().map(|c| w.clubs[c].finance.wage_bill as f64 * 52.0).sum();
    for c in w.clubs.ids() {
        let revenue = crate::finance::season_revenue(w, c).max(1);
        if w.clubs[c].finance.balance > revenue * 2 {
            s.clubs_hoarding += 1;
        }
        if w.governance.get(&c).is_some_and(|g| g.administration.is_some()) {
            s.clubs_in_administration += 1;
        }
    }
    let (mut t1, mut t2, mut t3, mut q1): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f32>) = (vec![], vec![], vec![], vec![]);
    for c in w.clubs.ids() {
        let league = w.clubs[c].league;
        if league.is_none() {
            continue;
        }
        let tier = w.comps[league].tier;
        let team = w.clubs[c].first_team();
        let mut cas: Vec<f32> = Vec::new();
        for &p in &w.teams[team].squad {
            let pl = &w.players.cold[p];
            cas.push(f32::from(pl.ca));
            let wage = pl.contract.current_wage(w.date);
            if wage > 0 {
                match tier {
                    1 => t1.push(wage as f64),
                    2 => t2.push(wage as f64),
                    _ => t3.push(wage as f64),
                }
            }
        }
        if tier == 1 && !cas.is_empty() {
            cas.sort_by(|a, b| b.total_cmp(a));
            q1.push(cas.iter().take(14).sum::<f32>() / cas.len().min(14) as f32);
        }
    }
    s.wage_tier1 = pct(&mut t1, 0.5).unwrap_or(0.0);
    s.wage_tier2 = pct(&mut t2, 0.5).unwrap_or(0.0);
    s.wage_tier3 = pct(&mut t3, 0.5).unwrap_or(0.0);
    s.top_tier_quality = mean(&q1);
    s.price_index = w.economy.global();
    s.wage_index = if w.economy.nations.is_empty() { 1.0 } else { w.economy.nations.values().map(|e| e.wage_index).sum::<f32>() / w.economy.nations.len() as f32 };
    s.broadcast_pools = w.economy.nations.values().map(|e| e.broadcast_pool as f64).sum();
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
    let (mut fees, mut ratios): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    for e in w.events.since(since) {
        match e.kind {
            EventKind::Transfer { fee, player, .. } => {
                s.transfers += 1;
                if fee > 0 {
                    fees.push(fee as f64);
                    let value = crate::market::value_of(w, player);
                    if value > 0 {
                        ratios.push(fee as f64 / value as f64);
                    }
                }
            }
            EventKind::LoanMove { .. } => s.loans += 1,
            EventKind::Retired { .. } => s.retirements += 1,
            EventKind::YouthIntake { count, .. } => s.youth_intakes += u32::from(count),
            EventKind::ManagerSacked { .. } => s.sackings += 1,
            EventKind::Administration { .. } => s.insolvencies += 1,
            EventKind::RecordBroken { kind, .. } => match kind {
                pw_world::event::RecordKind::ClubRecordSigning => s.record_signings += 1,
                pw_world::event::RecordKind::ClubRecordSale => s.record_sales += 1,
                pw_world::event::RecordKind::WorldRecordFee => s.record_world_fees += 1,
                _ => s.records_other += 1,
            },
            _ => {}
        }
    }
    // Bargaining and the boardroom.
    for d in w.deals.deals.iter().filter(|d| d.opened >= since || d.log.last().is_some_and(|l| l.0 >= since)) {
        for (date, line) in &d.log {
            if *date < since {
                continue;
            }
            match line {
                pw_world::deals::DealLine::AwaitingReplacement(_) => s.chain_waits += 1,
                pw_world::deals::DealLine::ReplacementSigned => s.chain_done += 1,
                pw_world::deals::DealLine::Ended(pw_world::deals::DealEnd::ReplacementFailed) => s.chain_failed += 1,
                pw_world::deals::DealLine::Signalled(_) => s.signals += 1,
                _ => {}
            }
        }
        s.bluffs_caught += u32::from(d.signals.caught);
    }
    s.settling = w.adaptation.current.len();
    s.struggled = w.adaptation.done.values().filter(|d| d.struggled && d.date >= since).count() as u32;
    s.plan_failures = w.events.since(since).iter().filter(|e| matches!(e.kind, EventKind::PlanFailed { .. })).count() as u32;
    s.cases_declined = w.boardroom.cases.iter().filter(|c| c.state == pw_world::boardroom::CaseState::Declined && c.date >= since).count() as u32;
    s.cases_judged = w.boardroom.cases.iter().filter(|c| c.outcome.is_some_and(|o| o.date >= since)).count() as u32;
    if !w.boardroom.appetite.is_empty() {
        let v: Vec<f32> = w.boardroom.appetite.values().map(|a| a.level).collect();
        s.appetite_min = v.iter().copied().fold(1.0, f32::min);
        s.appetite_max = v.iter().copied().fold(0.0, f32::max);
        s.appetite_mean = v.iter().sum::<f32>() / v.len() as f32;
    }
    s.fees_total = fees.iter().sum();
    s.fee_median = pct(&mut fees.clone(), 0.5).unwrap_or(0.0);
    s.fee_to_value = pct(&mut ratios, 0.5).unwrap_or(0.0);
    s.fee_p90 = pct(&mut fees.clone(), 0.9).unwrap_or(0.0);
    s.fee_p95 = pct(&mut fees.clone(), 0.95).unwrap_or(0.0);
    s.fee_max = pct(&mut fees, 1.0).unwrap_or(0.0);
    // Memory and growth.
    s.records = w.records.records.len();
    s.stories = w.media.stories.len();
    s.posts = w.net.posts.len();
    s.accounts = w.net.accounts.len();
    s.events = w.events.len();
    s.save_bytes = bincode::serialized_size(w).unwrap_or(0);
    s.top_sections = section_sizes(w)
        .into_iter()
        .take(6)
        .map(|(name, bytes)| format!("{name} {:.1}", bytes as f64 / 1e6))
        .collect::<Vec<_>>()
        .join(", ");
    s.structural_problems = crate::validate::problems(w).len();
    s
}

thread_local! {
    /// Fees and the public value of the player at the moment of the deal, collected only while `observe` runs on this thread.
    /// Diagnostic state: never saved, never read by the simulation.
    static DEALS: std::cell::RefCell<Option<Vec<(f64, f64, f64, f64)>>> = const { std::cell::RefCell::new(None) };
}

/// A transfer fee was agreed for a player the market valued at `value` (before the move changes his contract): remembered for the
/// fee-over-value series. Nothing happens unless a measurement run is collecting.
pub fn note_deal(value: pw_core::Money, fee: pw_core::Money, years_left: f32, asking_factor: f32) {
    if fee > 0 && value > 0 {
        DEALS.with(|d| {
            if let Some(v) = d.borrow_mut().as_mut() {
                v.push((fee as f64, value as f64, f64::from(years_left), f64::from(asking_factor)));
            }
        });
    }
}

/// Median fee over the player's value on the day of the deal. The snapshot's own `fee_to_value` divides by the value after the
/// move, which already carries the new contract and so rises as contracts lengthen; this one does not.
fn take_deal_ratio() -> Option<(f64, f64, f64)> {
    DEALS.with(|d| {
        let mut b = d.borrow_mut();
        let v = b.as_mut()?;
        let mut ratios: Vec<f64> = v.iter().map(|&(fee, value, _, _)| fee / value).collect();
        let mut years: Vec<f64> = v.iter().map(|&(_, _, y, _)| y).collect();
        let mut asks: Vec<f64> = v.iter().map(|&(_, _, _, a)| a).collect();
        v.clear();
        Some((pct(&mut ratios, 0.5)?, pct(&mut years, 0.5)?, pct(&mut asks, 0.5)?))
    })
}

/// Run `years` seasons of 365 days from the world's current date, calling `each` after every year with that year's snapshot.
pub fn observe(sim: &mut Sim, years: u32, mut each: impl FnMut(&Snapshot)) -> Vec<Snapshot> {
    let mut out = Vec::new();
    let mut since = sim.world.date;
    DEALS.with(|d| *d.borrow_mut() = Some(Vec::new()));
    out.push(snapshot(&sim.world, since, 0));
    each(&out[0]);
    for y in 1..=years {
        sim.run(365);
        let mut s = snapshot(&sim.world, since, y);
        if let Some((ratio, years, ask)) = take_deal_ratio() {
            s.fee_to_value = ratio;
            s.deal_contract_years = years;
            s.deal_asking_factor = ask;
        }
        since = sim.world.date;
        each(&s);
        out.push(s);
    }
    DEALS.with(|d| *d.borrow_mut() = None);
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

    if let Some(bad) = run.iter().find(|s| s.structural_problems > 0) {
        flag(Level::Problem, "structure", format!("{} structural problem(s) in year {}: run `pathway-sim check` on a save to list them", bad.structural_problems, bad.year));
    }
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
    // The mean wage follows the wage bill and so the revenue; the median also moves with how spread out ability is inside squads
    // (a league whose stars fade pays its middle players more of a fixed bill), so it is read with a higher bar.
    if let Some(g) = yearly_growth(&series(&|s| if s.first_team_wage_mean > 0.0 { s.first_team_wage_mean } else { s.first_team_wage_median })) {
        if g > 12.0 {
            flag(Level::Problem, "wages", format!("mean first-team wage inflates {g:.0}% a year"));
        } else if g > 6.0 {
            flag(Level::Warn, "wages", format!("mean first-team wage inflates {g:.0}% a year"));
        }
    }
    if let Some(g) = yearly_growth(&series(&|s| s.first_team_wage_median)) {
        if g > 20.0 {
            flag(Level::Problem, "wages", format!("median first-team wage inflates {g:.0}% a year"));
        } else if g > 12.0 {
            flag(Level::Warn, "wages", format!("median first-team wage inflates {g:.0}% a year"));
        }
    }
    // Fees: a median of a handful of deals says nothing, so only years with a real market count.
    let market_years: Vec<&Snapshot> = run[warm..].iter().filter(|s| s.transfers >= 15 && s.fee_median > 0.0).collect();
    let fee_growth = match (market_years.first(), market_years.last()) {
        (Some(a), Some(b)) if b.year > a.year => Some(((b.fee_median / a.fee_median).powf(1.0 / f64::from(b.year - a.year)) - 1.0) * 100.0),
        _ => None,
    };
    // A median fee rises when prices rise, and also when dearer players are the ones moving. Prices are what fee over value measures;
    // only that is a problem. The median alone, with prices steady, is a market of different deals, worth a look and no more.
    let price_growth = match (market_years.first(), market_years.last()) {
        (Some(a), Some(b)) if b.year > a.year && a.fee_to_value > 0.0 && b.fee_to_value > 0.0 => Some(((b.fee_to_value / a.fee_to_value).powf(1.0 / f64::from(b.year - a.year)) - 1.0) * 100.0),
        _ => None,
    };
    match (fee_growth, price_growth) {
        (_, Some(p)) if p > 15.0 => flag(Level::Problem, "fees", format!("clubs pay {p:.0}% more a year over what players are worth")),
        (_, Some(p)) if p > 8.0 => flag(Level::Warn, "fees", format!("clubs pay {p:.0}% more a year over what players are worth")),
        (Some(g), Some(_)) if g > 15.0 => flag(Level::Warn, "fees", format!("median transfer fee rises {g:.0}% a year at steady prices: dearer players are moving")),
        (Some(g), None) if g > 15.0 => flag(Level::Problem, "fees", format!("median transfer fee inflates {g:.0}% a year")),
        (Some(g), _) if g > 8.0 => flag(Level::Warn, "fees", format!("median transfer fee inflates {g:.0}% a year")),
        _ => {}
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
    // Everyone not retired, amateurs and children included: the active count above hides a pool that only ever fills.
    if let Some(g) = yearly_growth(&series(&|s| s.alive as f64)) {
        if g > 8.0 {
            flag(Level::Problem, "population", format!("everyone not retired (amateurs and children included) grows {g:.0}% a year after warm-up: creations outrun retirements, see the population table"));
        } else if g > 4.0 {
            flag(Level::Warn, "population", format!("everyone not retired (amateurs and children included) grows {g:.0}% a year after warm-up: creations outrun retirements, see the population table"));
        }
    }
    // Every change in the population is a creation (with a source) or a retirement; anything else is a leak.
    for (prev, now) in run.iter().zip(run.iter().skip(1)).skip(1) {
        let created: u32 = now.created.iter().sum();
        let unexplained = now.alive as i64 - prev.alive as i64 - i64::from(created) + i64::from(now.retirements);
        if unexplained != 0 {
            flag(Level::Problem, "population", format!("year {}: {unexplained:+} players appeared or vanished without a creation record or a retirement", now.year));
            break;
        }
    }
    let turnover = last.retirements as f64 / last.alive.max(last.active_players).max(1) as f64;
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
    // Posts (kept for weeks) and the event log (kept for about a year, headlines for good) are windows: what they hold is a level, and
    // a level that keeps climbing is the problem. (Read as a cumulative log, a window's yearly change is noise around zero once it is
    // full, and any later year looked like many times "year 2".)
    for (name, f) in [("posts", (&|s: &Snapshot| s.posts as f64) as &dyn Fn(&Snapshot) -> f64), ("events", &|s| s.events as f64)] {
        let v: Vec<f64> = run.iter().map(f).collect();
        if v.len() >= 4 && v[2] > 0.0 {
            let rate = v[v.len() - 1] / v[2];
            if rate > 2.5 {
                flag(Level::Warn, name, format!("{name} held are {rate:.1}x what they were in year 2"));
            }
        }
    }
    for (name, f) in [("stories", (&|s: &Snapshot| s.stories as f64) as &dyn Fn(&Snapshot) -> f64)] {
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
    s.push_str("year  active  first  resv  youth  adult<1st  free  amat  sqd  sqdMax  age  meanCA  balMed   revMed   inDebt  wage/rev  wageFT  wageAll fee50  fee90   feeMax  settle(n/s) planfail chain(w/d/f) sig  apt(min/mean/max)  xfers  loans  retire  intake  fame99  famSat  mgrs(u)  saveMB\n");
    for r in run {
        s.push_str(&format!(
            "{:>4} {:>7} {:>6} {:>5} {:>6} {:>9} {:>5} {:>5} {:>5.1} {:>6} {:>4.1} {:>7.1} {:>7} {:>8} {:>7} {:>8.2} {:>7} {:>7} {:>6} {:>6} {:>8} {:>6}/{:<4} {:>8} {:>3}/{}/{} {:>4} {:>4.2}/{:.2}/{:.2} {:>6} {:>6} {:>7} {:>7} {:>7.0} {:>6.1}% {:>4}({:<3}) {:>7.1}\n",
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
            money(r.first_team_wage_median),
            money(r.player_wage_median),
            money(r.fee_median),
            money(r.fee_p90),
            money(r.fee_max),
            r.settling,
            r.struggled,
            r.plan_failures,
            r.chain_waits,
            r.chain_done,
            r.chain_failed,
            r.signals,
            r.appetite_min,
            r.appetite_mean,
            r.appetite_max,
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
    for r in run {
        s.push_str(&format!("  save y{}: {}\n", r.year, r.top_sections));
    }
    s
}

/// Where the people came from: new players by source per year against retirements, and the change in everyone not retired.
/// `unexplained` is what is left of that change after creations and retirements: it must be zero (nothing enters or leaves the
/// population without a stated source or a retirement).
pub fn render_population(run: &[Snapshot]) -> String {
    let mut s = String::new();
    s.push_str("year   alive  delta  created");
    for src in PlayerSource::ALL {
        s.push_str(&format!(" {:>9}", src.label().split(' ').next_back().unwrap_or("?")));
    }
    s.push_str("  retired  unexplained  everCreated\n");
    for (i, r) in run.iter().enumerate() {
        let delta = if i == 0 { 0 } else { r.alive as i64 - run[i - 1].alive as i64 };
        let created: u32 = r.created.iter().sum();
        let unexplained = if i == 0 { 0 } else { delta - i64::from(created) + i64::from(r.retirements) };
        s.push_str(&format!("{:>4} {:>7} {:>6} {:>8}", r.year, r.alive, delta, created));
        for c in r.created {
            s.push_str(&format!(" {c:>9}"));
        }
        s.push_str(&format!("  {:>7}  {:>11}  {:>11}\n", r.retirements, unexplained, r.players_total));
    }
    s
}

/// The money series of a run, one row per year: wages by tier, fees, cash, revenue against wages, unemployment, records and insolvency.
pub fn render_economy(run: &[Snapshot]) -> String {
    let mut s = String::new();
    s.push_str("year  wageT1  wageT2  wageT3  wageMean wagePl50 wagePl99  fee50  fee/val cyrs  ask  fee90  fee95   feeMax   balP10  balMed  balP90  hoard  inDebt admin(n/new)  revTot  wageTot  wage/rev w/r90  free  staffU  recSign recSale recWorld recOther quality\n");
    for r in run {
        s.push_str(&format!(
            "{:>4} {:>7} {:>7} {:>7} {:>8} {:>8} {:>8} {:>6} {:>8.2} {:>4.1} {:>5.2} {:>6} {:>6} {:>8} {:>8} {:>7} {:>7} {:>6} {:>6} {:>5}/{:<5} {:>7} {:>8} {:>8.2} {:>6.2} {:>5} {:>7} {:>7} {:>7} {:>8} {:>8} {:>6.1} {:>7.3} {:>6.3} {:>7} {:>6.0}\n",
            r.year,
            money(r.wage_tier1),
            money(r.wage_tier2),
            money(r.wage_tier3),
            money(r.first_team_wage_mean),
            money(r.player_wage_median),
            money(r.player_wage_p99),
            money(r.fee_median),
            r.fee_to_value,
            r.deal_contract_years,
            r.deal_asking_factor,
            money(r.fee_p90),
            money(r.fee_p95),
            money(r.fee_max),
            money(r.balance_p10),
            money(r.balance_median),
            money(r.balance_p90),
            r.clubs_hoarding,
            r.clubs_in_debt,
            r.clubs_in_administration,
            r.insolvencies,
            money(r.revenue_total),
            money(r.wage_bill_total),
            if r.revenue_total > 0.0 { r.wage_bill_total / r.revenue_total } else { 0.0 },
            r.wage_to_revenue_p90,
            r.free_agents,
            r.staff_unemployed,
            r.record_signings,
            r.record_sales,
            r.record_world_fees,
            r.records_other,
            r.top_tier_quality,
            r.price_index,
            r.wage_index,
            money(r.broadcast_pools),
            r.club_rep_p90,
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
                alive: 1000,
                created: [80, 0, 0, 0, 0, 0, 0],
                clubs: 50,
                balance_median: 5e6,
                first_team_wage_median: 5000.0,
                player_wage_median: 5000.0,
                fee_median: 1e6,
                transfers: 40,
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
            s.first_team_wage_median *= 1.25f64.powi(i as i32);
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
    fn a_window_that_stays_full_is_fine_but_one_that_keeps_filling_is_not() {
        // A full window whose level wobbles: the yearly change goes from 3 to 600, and that is noise, not growth.
        let mut steady = flat(8);
        for (i, s) in steady.iter_mut().enumerate() {
            s.posts = if i == 0 { 0 } else { 40_000 + [0, 3, 600, 200, 900, 100, 700][i - 1] };
        }
        assert!(analyse(&steady).iter().all(|f| f.series != "posts"), "{:?}", analyse(&steady));
        let mut filling = flat(8);
        for (i, s) in filling.iter_mut().enumerate() {
            s.posts = 10_000 * (i + 1) * (i + 1);
        }
        assert!(analyse(&filling).iter().any(|f| f.series == "posts"));
    }

    #[test]
    fn growth_is_measured_from_the_first_full_year() {
        assert!((yearly_growth(&[1.0, 100.0, 110.0, 121.0]).unwrap() - 10.0).abs() < 1e-6);
        assert!(yearly_growth(&[1.0, 0.0, 5.0, 6.0]).is_none());
    }

    #[test]
    fn a_handful_of_deals_cannot_show_fee_inflation_but_a_real_market_can() {
        let mut thin = flat(8);
        for (i, s) in thin.iter_mut().enumerate() {
            s.transfers = 4;
            s.fee_median *= 3.0f64.powi(i as i32);
        }
        assert!(analyse(&thin).iter().all(|f| f.series != "fees"), "medians of four deals are noise");
        let mut real = flat(8);
        for (i, s) in real.iter_mut().enumerate() {
            s.fee_median *= 1.3f64.powi(i as i32);
        }
        assert!(analyse(&real).iter().any(|f| f.series == "fees" && f.level == Level::Problem));
    }

    #[test]
    fn dearer_players_moving_is_not_price_inflation_but_paying_more_for_the_same_player_is() {
        let mut mix = flat(8);
        for (i, s) in mix.iter_mut().enumerate() {
            s.fee_median *= 1.3f64.powi(i as i32);
            s.fee_to_value = 1.1;
        }
        let f = analyse(&mix);
        assert!(f.iter().any(|x| x.series == "fees" && x.level == Level::Warn) && f.iter().all(|x| x.series != "fees" || x.level != Level::Problem), "{f:?}");
        let mut prices = flat(8);
        for (i, s) in prices.iter_mut().enumerate() {
            s.fee_to_value = 1.1 * 1.2f64.powi(i as i32);
        }
        assert!(analyse(&prices).iter().any(|x| x.series == "fees" && x.level == Level::Problem));
    }

    #[test]
    fn structural_damage_in_any_year_is_a_problem() {
        let mut run = flat(6);
        run[3].structural_problems = 2;
        assert!(analyse(&run).iter().any(|f| f.series == "structure" && f.level == Level::Problem));
    }
}
