//! The football economy over decades (02 §8, 07 §9). Prices and wages
//! inflate; each nation's economy drifts with its own trend; top-flight
//! broadcast deals are renegotiated every few seasons on the back of league
//! strength and continental results, which is how leagues grow rich or fall
//! behind. Clubs' revenue comes from these pools, their gates, their fame and
//! their prize money.

use pw_core::math::interp;
use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Money, NationId};
use pw_world::event::{EventKind, Visibility};
use pw_world::governance::NationEconomy;
use pw_world::{CompKind, TeamKind, World};
use smallvec::SmallVec;

pub fn ensure(w: &mut World) {
    if w.economy.global_index <= 0.0 {
        w.economy.global_index = 1.0;
        w.economy.last_year = w.date.year();
    }
    for n in w.nations.ids() {
        if w.economy.nations.contains_key(&n) || w.nations[n].leagues.is_empty() {
            continue;
        }
        let rep = f32::from(w.nations[n].reputation) / 10_000.0;
        let econ = w.nations[n].economy;
        let pool = (f64::from(w.data.tuning.finance.revenue_top) * 8.0 * f64::from(rep).powf(2.0) * f64::from(econ)).max(2_000_000.0) as Money;
        let mut rng = Rng::keyed(&[w.seed, stream::WORLDGEN, 0xec0, u64::from(n.0)]);
        w.economy.nations.insert(
            n,
            NationEconomy {
                wage_index: 1.0,
                broadcast_pool: pool,
                deal_until: w.date.year() + rng.range_i32(0, 2),
                growth: rng.normal_ms(0.015, 0.01),
                league_strength: rep,
                coefficient: SmallVec::new(),
            },
        );
    }
}

/// Once a year (at the turn of the football year): inflation, national
/// growth, league strength, and broadcast renewals.
pub fn yearly(w: &mut World) {
    let year = w.date.year();
    if w.economy.last_year >= year {
        return;
    }
    w.economy.last_year = year;
    let mut rng = Rng::keyed(&[w.seed, stream::WORLDGEN, 0xec1, year as u64]);
    let inflation = rng.normal_ms(0.03, 0.012).clamp(-0.01, 0.08);
    w.economy.global_index *= 1.0 + inflation;
    let nations: Vec<NationId> = w.economy.nations.keys().copied().collect();
    let mut nations = nations;
    nations.sort();
    for n in nations {
        let strength = league_strength(w, n);
        let today = w.date;
        let e = w.economy.nations.get_mut(&n).unwrap();
        // Growth trends revert slowly to the mean.
        e.growth = e.growth * 0.8 + 0.2 * 0.015 + rng.normal() * 0.008;
        e.wage_index *= 1.0 + e.growth;
        e.league_strength = e.league_strength * 0.7 + strength * 0.3;
        if year >= e.deal_until {
            // No continental results (a nation with no European competition, or a young world) is neutral, not a bad record.
            let coef: f32 = if e.coefficient.is_empty() { 1.3 } else { e.coefficient.iter().sum::<f32>() / e.coefficient.len() as f32 };
            // The pool is in real terms (inflation and national growth are applied to revenue as a whole, see `club_revenue`), and a renewal moves it
            // around where it was: a strong league and good continental results earn a rise, a weak one a cut, an ordinary one none. A
            // positive average here would compound every renewal on top of inflation and national growth, and revenue, wages and
            // fees with it, without end.
            let boom = rng.normal() * 0.06 + (e.league_strength - 0.8) * 0.3 + (coef - 1.3) * 0.05;
            e.broadcast_pool = ((e.broadcast_pool as f64) * (1.0 + f64::from(boom.clamp(-0.2, 0.3)))) as Money;

            e.deal_until = year + 3;
            let pool = e.broadcast_pool;
            w.events.push(today, Visibility::Public, EventKind::BroadcastDeal { nation: n, pool });
        }
    }
}

/// Strength of a nation's top league: the quality of its clubs' best players.
fn league_strength(w: &World, n: NationId) -> f32 {
    let Some(&top) = w.nations[n].leagues.first() else { return 0.0 };
    let teams = &w.comps[top].state.entrants;
    if teams.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    for &t in teams {
        let mut cas: Vec<u8> = w.teams[t].squad.iter().map(|&p| w.players.cold[p].ca).collect();
        cas.sort_by(|a, b| b.cmp(a));
        let best: f32 = cas.iter().take(14).map(|&c| f32::from(c)).sum::<f32>() / 14.0;
        total += best;
    }
    (total / teams.len() as f32 / 180.0).clamp(0.0, 1.0)
}

/// Record continental results into each nation's coefficient at season end.
pub fn record_continental(w: &mut World, comp: pw_core::CompId) {
    if w.comps[comp].kind != CompKind::Continental {
        return;
    }
    let mut per_nation: pw_world::FxHashMap<NationId, (f32, u32)> = Default::default();
    for r in &w.comps[comp].state.table {
        let n = w.clubs[w.teams[r.team].club].nation;
        let e = per_nation.entry(n).or_default();
        e.0 += f32::from(r.points.max(0) as u16) / f32::from(r.played.max(1));
        e.1 += 1;
    }
    for (n, (pts, k)) in per_nation {
        if let Some(e) = w.economy.nations.get_mut(&n) {
            e.coefficient.push(pts / k as f32);
            if e.coefficient.len() > 5 {
                e.coefficient.remove(0);
            }
        }
    }
}

/// A club's season revenue: its share of the national broadcast pool (equal
/// part plus merit by last finish), commercial income from reputation and
/// the fame of its squad, all scaled by the economy it lives in.
pub fn club_revenue(w: &World, club: ClubId) -> Money {
    let c = &w.clubs[club];
    let rep = f64::from(c.reputation) / 10_000.0;
    let econ = f64::from(w.nations[c.nation].economy);
    let idx = f64::from(w.economy.wage_index(c.nation));
    let base = (f64::from(w.data.tuning.finance.revenue_top) * rep.powf(2.2) * econ).max(150_000.0);
    let broadcast = if c.league.is_some() && w.comps[c.league].team_kind == TeamKind::First {
        let league = &w.comps[c.league];
        let pool = w.economy.nations.get(&c.nation).map_or(0.0, |e| e.broadcast_pool as f64);
        let tier_share = interp(&[(1.0, 1.0), (2.0, 0.18), (3.0, 0.05), (5.0, 0.01)], f32::from(league.tier)) as f64;
        let n = league.state.entrants.len().max(1) as f64;
        let pos = league.position_of(c.first_team()).unwrap_or(league.state.entrants.len()) as f64;
        let merit = 1.0 - (pos - 1.0) / n;
        pool * tier_share * (0.5 / n + 0.5 * merit * 2.0 / n)
    } else {
        0.0
    };
    let fame: f64 = c.teams.first().map_or(0.0, |&t| w.teams[t].squad.iter().map(|&p| f64::from(w.players.cold[p].rep.world)).sum::<f64>()) / 10_000.0;
    let commercial = fame * 150_000.0 * econ;
    // `idx` is the world's general price level times the nation's own real growth.
    ((base + broadcast + commercial) * idx) as Money
}
