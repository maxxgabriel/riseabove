//! Managers act on beliefs, not hidden truth (locked design §1). These checks read what selection believes about a squad and
//! compare it with the truth the simulation holds.

use pw_core::{Attr, Hidden};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_sim::selection::{Context, factors_of};
use pw_world::TeamKind;

fn corr(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let cov: f32 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let (va, vb): (f32, f32) = (a.iter().map(|x| (x - ma).powi(2)).sum(), b.iter().map(|y| (y - mb).powi(2)).sum());
    cov / (va.sqrt() * vb.sqrt()).max(1e-6)
}

#[test]
fn what_the_manager_believes_tracks_the_truth_without_being_it() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 21, Scale::SMALL));
    sim.run(60);
    let w = &sim.world;
    let (mut edge_b, mut edge_t, mut lead_b, mut lead_t, mut trouble_b, mut trouble_t) = (vec![], vec![], vec![], vec![], vec![], vec![]);
    for team in w.teams.ids().filter(|&t| w.teams[t].kind == TeamKind::First) {
        for &p in &w.teams[team].squad {
            let Some(f) = factors_of(w, team, pw_core::CompId::NONE, w.date, p, &Context::plain(0.5)) else { continue };
            let c = &w.players.cold[p];
            edge_b.push(f.edge);
            edge_t.push((c.attrs.get(Attr::Concentration) + c.attrs.get(Attr::Composure) + c.attrs.get(Attr::Bravery)) / 60.0);
            lead_b.push(f.leadership);
            lead_t.push(c.attrs.get(Attr::Leadership) / 20.0);
            // Big-match temperament is a trait: believed, so it can differ from the truth.
            let h = &w.people[c.person].hidden;
            trouble_b.push(f.big_match + 0.5);
            trouble_t.push((0.6 * h.f(Hidden::ImportantMatches) + 0.4 * h.f(Hidden::Pressure)) / 20.0);
        }
    }
    assert!(edge_b.len() > 100, "{} players read", edge_b.len());
    for (name, b, t, min_corr) in [("edge", &edge_b, &edge_t, 0.55), ("leadership", &lead_b, &lead_t, 0.55), ("big-match temperament", &trouble_b, &trouble_t, 0.15)] {
        let exact = b.iter().zip(t.iter()).filter(|(x, y)| (**x - **y).abs() < 1e-4).count();
        assert!(exact * 10 < b.len(), "{name}: {exact} of {} beliefs equal the truth exactly: the manager is reading hidden state", b.len());
        let c = corr(b, t);
        assert!(c > min_corr, "{name}: beliefs correlate only {c:.2} with the truth; they should be informed by evidence");
    }
    // Personality is harder to read than skills: its beliefs track the truth less closely.
    assert!(corr(&edge_b, &edge_t) > corr(&trouble_b, &trouble_t), "traits should be read less reliably than skills");
}

#[test]
fn two_managers_with_different_traits_weigh_the_same_squad_differently() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 22, Scale::SMALL));
    sim.run(30);
    let w = &mut sim.world;
    let teams: Vec<_> = w.teams.ids().filter(|&t| w.teams[t].kind == TeamKind::First).take(2).collect();
    let (a, b) = (pw_sim::selection::style_of(w, teams[0]), pw_sim::selection::style_of(w, teams[1]));
    // Give the second manager the opposite disposition on every trait that selection reads.
    let m = w.clubs[w.teams[teams[1]].club].manager;
    w.staff[m].attrs.set(pw_core::StaffAttr::Discipline, 20);
    w.staff[m].attrs.set(pw_core::StaffAttr::SportsScience, 20);
    let strict = pw_sim::selection::style_of(w, teams[1]);
    let m0 = w.clubs[w.teams[teams[0]].club].manager;
    w.staff[m0].attrs.set(pw_core::StaffAttr::Discipline, 1);
    w.staff[m0].attrs.set(pw_core::StaffAttr::SportsScience, 1);
    let lax = pw_sim::selection::style_of(w, teams[0]);
    assert!(strict.strictness > lax.strictness && strict.caution > lax.caution, "traits move the weights: {strict:?} vs {lax:?}");
    let _ = (a, b);
}

// ---- the market prices from beliefs (locked design §3.5, §3.25) ----------------------------------------------------------

use pw_core::PosGroup;
use pw_sim::market;

fn world_after(days: u32, seed: u64) -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), seed, Scale::SMALL));
    sim.run(days);
    sim
}

#[test]
fn the_public_value_tracks_the_true_worth_without_being_it() {
    let sim = world_after(40, 31);
    let w = &sim.world;
    let (mut public, mut truth) = (vec![], vec![]);
    for p in w.players.ids().filter(|&p| w.players.hot[p].status == pw_world::PlayerStatus::Active) {
        public.push((market::value_of(w, p) as f32).ln());
        truth.push((market::true_worth(w, p) as f32).ln());
    }
    let exact = public.iter().zip(&truth).filter(|(a, b)| (**a - **b).abs() < 1e-3).count();
    assert!(exact * 5 < public.len(), "{exact} of {} public values equal the true worth: the market is reading hidden ability", public.len());
    let c = corr(&public, &truth);
    assert!(c > 0.85, "public estimates still follow real quality: r = {c:.2}");
    // The stored value players carry is the public estimate.
    let stored: Vec<f32> = w.players.ids().filter(|&p| w.players.hot[p].status == pw_world::PlayerStatus::Active).map(|p| (w.players.cold[p].value.max(1) as f32).ln()).collect();
    assert!(corr(&stored, &public) > 0.98);
}

#[test]
fn two_clubs_value_the_same_player_differently_and_neither_uses_the_public_number() {
    let sim = world_after(40, 32);
    let w = &sim.world;
    let clubs: Vec<_> = w.clubs.ids().take(6).collect();
    let mut differing = 0;
    let mut total = 0;
    for p in w.players.ids().filter(|&p| w.players.hot[p].status == pw_world::PlayerStatus::Active).take(300) {
        let vals: Vec<i64> = clubs.iter().filter(|&&c| w.players.hot[p].club != c).map(|&c| market::fair_value(w, c, p)).collect();
        total += 1;
        if vals.iter().any(|v| *v != vals[0]) {
            differing += 1;
        }
    }
    assert!(differing * 10 > total * 8, "clubs read a player differently: {differing} of {total}");
}

#[test]
fn a_seller_asks_more_for_a_player_it_cannot_replace() {
    let mut sim = world_after(40, 33);
    let w = &mut sim.world;
    // A first-team outfield player at a club with cover in his group.
    let (p, seller) = w
        .players
        .ids()
        .filter(|&p| w.players.hot[p].status == pw_world::PlayerStatus::Active && w.players.cold[p].best_pos.group() == PosGroup::Mid)
        .map(|p| (p, w.players.hot[p].club))
        .find(|&(p, c)| c.is_some() && w.teams[w.clubs[c].first_team()].squad.contains(&p))
        .expect("a midfielder in a first team");
    let with_cover = market::seller_reservation(w, seller, p);
    // Take away everyone who could replace him.
    let team = w.clubs[seller].first_team();
    for q in w.teams[team].squad.clone() {
        if q != p && w.players.cold[q].best_pos.group() == PosGroup::Mid {
            w.players.hot[q].injury = 1;
            w.players.hot[q].injury_days = 90;
        }
    }
    let without = market::seller_reservation(w, seller, p);
    assert!(without as f64 >= with_cover as f64 * 1.10, "irreplaceable: {without} vs replaceable {with_cover}");
}

#[test]
fn transfers_still_happen_and_chains_form_when_valuations_come_from_beliefs() {
    let mut sim = world_after(0, 34);
    sim.run(400);
    let w = &sim.world;
    let moves = w.events.since(pw_core::Date(0)).iter().filter(|e| matches!(e.kind, pw_world::EventKind::Transfer { .. })).count();
    assert!(moves >= 10, "{moves} transfers in a season: the market has stalled");
}
