//! QA: fuzzing the pure decision functions. For hundreds of pseudo-random (club, player) pairs, including retired players, amateurs,
//! free agents, players on loan and clubs without a manager, every valuation, package, boardroom and attention function must return
//! (no panic) finite numbers inside their documented ranges.

mod qa_common;

use pw_core::rng::Rng;
use pw_core::{ClubId, PlayerId};
use pw_import::synthetic::Scale;
use pw_sim::{adaptation, attention, bargaining, boardroom, dossier, market, package, scouting};
use pw_world::World;
use qa_common::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn finite(x: f32) -> bool {
    x.is_finite()
}

fn probe(w: &World, club: ClubId, p: PlayerId, out: &mut Vec<String>) {
    let mut bad = |m: String| out.push(format!("club {club:?} player {p:?} ({:?}): {m}", w.players.hot[p].status));
    let (ca, cb, pa, pb) = scouting::view(w, club, p);
    if ![ca, cb, pa, pb].iter().all(|&x| finite(x)) || cb < 0.0 || pb < 0.0 || !(0.0..=201.0).contains(&ca) || !(0.0..=201.0).contains(&pa) {
        bad(format!("scouting::view = {:?}", (ca, cb, pa, pb)));
    }
    if market::fair_value(w, club, p) < 0 || market::value_of(w, p) < 0 || market::wage_demand(w, p, club) < 0 {
        bad("a negative value, fair value or wage demand".into());
    }
    let seller = w.players.hot[p].club;
    if seller.is_some() && market::seller_reservation(w, seller, p) < 0 {
        bad("negative seller reservation".into());
    }
    let want = package::want(w, club, p);
    if !finite(want) || !(-0.5001..=1.0001).contains(&want) {
        bad(format!("package::want = {want}"));
    }
    let pr = package::priorities(w, p);
    if pr.iter().any(|&x| !finite(x) || x < 0.0) || (pr.iter().sum::<f32>() - 1.0).abs() > 1e-3 {
        bad(format!("priorities {pr:?}"));
    }
    let base = {
        let c = market::new_contract(w, p, club, 1.0);
        pw_world::negotiation::Terms { signing_fee: c.wage * 2, ..pw_world::negotiation::Terms::from_contract(&c, 3) }
    };
    let cost = package::club_cost(w, club, p, &base);
    if !cost.is_finite() || cost < 0.0 {
        bad(format!("club_cost {cost}"));
    }
    let u = package::player_utility(w, p, club, &base, &pr);
    if !u.is_finite() {
        bad(format!("player_utility {u}"));
    }
    let (open, _) = package::opening(w, club, p, base, 0.5, 2, want.max(0.0));
    if open.years == 0 || open.wage < 0 || open.signing_fee < 0 {
        bad(format!("an opening package {open:?}"));
    }
    if let Some(d) = boardroom::decide(w, club, p, None, market::fair_value(w, club, p)) {
        if !finite(d.score) || !finite(d.threshold) || d.stances.iter().any(|s| !finite(s.support) || s.support.abs() > 1.0001) {
            bad(format!("boardroom::decide score {} threshold {}", d.score, d.threshold));
        }
        let total: f32 = d.stances.iter().map(|s| s.power).sum();
        if (total - 1.0).abs() > 1e-3 {
            bad(format!("stance powers sum to {total}"));
        }
    }
    if let Some(d) = dossier::build(w, club, p) {
        if !finite(d.current.mid) || !finite(d.ceiling.mid) || d.ceiling.mid + 1e-3 < d.current.mid || d.current.band < 0.0 {
            bad(format!("dossier spans {:?} {:?}", d.current, d.ceiling));
        }
    }
    if seller.is_some() && seller != club {
        let (a, b) = bargaining::initial_limits(w, club, seller, p);
        if a.lo < 0 || a.lo > a.hi || b.lo < 0 || b.lo > b.hi {
            bad(format!("initial limits {a:?} {b:?}"));
        }
    }
    let r = adaptation::readiness(w, club, p);
    if !finite(r.weeks_to_useful) || r.weeks_to_useful < 0.0 || !finite(adaptation::impatience(w, club, p)) {
        bad(format!("adaptation readiness {:?}", r.weeks_to_useful));
    }
    let (lvl, brand, appeal, hype) = (attention::level(w, w.players.cold[p].person), attention::brand_premium(w, p), attention::commercial_appeal(w, p), attention::overhyped(w, p));
    if !finite(lvl) || !finite(brand) || !finite(appeal) || !finite(hype) || brand < 0.0 || lvl < 0.0 {
        bad(format!("attention {:?}", (lvl, brand, appeal, hype)));
    }
}

fn fuzz(sim: &pw_sim::Sim, seed: u64, n: usize) {
    let w = &sim.world;
    let mut rng = Rng::keyed(&[seed, 0xf022]);
    let mut out = Vec::new();
    let mut panics = Vec::new();
    let mut statuses = std::collections::BTreeSet::new();
    for _ in 0..n {
        let club = ClubId(rng.below(w.clubs.len() as u32));
        let p = PlayerId(rng.below(w.players.len() as u32));
        statuses.insert(format!("{:?}", w.players.hot[p].status));
        if catch_unwind(AssertUnwindSafe(|| probe(w, club, p, &mut out))).is_err() {
            panics.push(format!("club {club:?} player {p:?} ({:?})", w.players.hot[p].status));
        }
    }
    assert!(panics.is_empty(), "panics ({} of {n}): {:?}", panics.len(), &panics[..panics.len().min(10)]);
    assert!(out.is_empty(), "{} out-of-range results, first: {:#?}", out.len(), &out[..out.len().min(10)]);
    assert!(statuses.len() >= 2, "the sample covers several player statuses: {statuses:?}");
}

#[test]
fn valuation_package_and_boardroom_functions_survive_arbitrary_pairs() {
    let sim = ran(Scale::SMALL, 151, 200);
    fuzz(&sim, 1, 500);
}

#[test]
fn the_same_functions_survive_a_world_with_clubs_in_trouble() {
    let mut sim = ran(Scale::TINY, 152, 250);
    // Wreck a few clubs: no manager, no cash, huge debt, sacked staff.
    let clubs: Vec<ClubId> = sim.world.clubs.ids().take(4).collect();
    for (i, &c) in clubs.iter().enumerate() {
        let w = &mut sim.world;
        w.clubs[c].manager = pw_core::StaffId::NONE;
        w.clubs[c].finance.balance = if i % 2 == 0 { -500_000_000 } else { 0 };
        w.clubs[c].finance.debt = 900_000_000;
        w.clubs[c].finance.wage_budget = 0;
        if i >= 2 {
            let staff = w.clubs[c].staff.clone();
            for s in staff {
                w.staff[s].club = ClubId::NONE;
            }
            w.clubs[c].staff.clear();
        }
    }
    fuzz(&sim, 2, 400);
}
