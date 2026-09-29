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
