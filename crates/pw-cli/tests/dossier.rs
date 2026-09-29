//! Belief dossiers (locked design 1.3-1.10, 1.14): evaluators disagree, exposure sets confidence, the manager weighs staff by trust,
//! the same evidence reads differently under a different philosophy, and a change of manager or new evidence leaves a reason behind.

use pw_core::{ClubId, PlayerId, StaffId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, dossier};
use pw_world::dossier::{Confidence, Pending, Reason};
use pw_world::{StaffRole, World};

fn world() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 77, Scale::SMALL));
    sim.run(40);
    sim
}

fn club_with_staff(w: &World) -> ClubId {
    w.clubs.ids().find(|&c| w.dossiers.map.keys().filter(|k| k.0 == c).count() > 20 && w.clubs[c].staff.len() >= 3).expect("a club with staff and dossiers")
}

#[test]
fn clubs_hold_dossiers_on_their_own_players_built_from_several_evaluators() {
    let sim = world();
    let w = &sim.world;
    let club = club_with_staff(w);
    let squad = &w.teams[w.clubs[club].first_team()].squad;
    let with: Vec<_> = squad.iter().filter_map(|&p| w.dossiers.get(club, p)).collect();
    assert!(with.len() * 10 >= squad.len() * 9, "the club has a dossier on nearly every one of its own players: {} of {}", with.len(), squad.len());
    let many = with.iter().filter(|d| d.opinions.len() >= 3).count();
    assert!(many * 2 >= with.len(), "most dossiers rest on three or more people's readings");
    for d in &with {
        let total: f32 = d.opinions.iter().map(|o| o.weight).sum();
        assert!((total - 1.0).abs() < 1e-3, "the manager's weights add to one: {total}");
        assert!(d.evidence.training && d.evidence.medical, "his own player is watched in training and his medical record is known");
        assert!(d.ceiling.mid >= d.current.mid - 1e-3, "a ceiling is never below the current level");
    }
}

#[test]
fn evaluators_disagree_and_the_club_is_never_simply_right() {
    let sim = world();
    let w = &sim.world;
    let club = club_with_staff(w);
    let squad = &w.teams[w.clubs[club].first_team()].squad;
    let (mut disagreeing, mut exact, mut n) = (0, 0, 0);
    let (mut sum_err, mut sum_truth_var) = (0.0f32, 0.0f32);
    for &p in squad {
        let Some(d) = w.dossiers.get(club, p) else { continue };
        n += 1;
        let cas: Vec<f32> = d.opinions.iter().map(|o| o.ca.mid).collect();
        let (lo, hi) = (cas.iter().cloned().fold(f32::MAX, f32::min), cas.iter().cloned().fold(f32::MIN, f32::max));
        if hi - lo >= 2.0 {
            disagreeing += 1;
        }
        let truth = f32::from(w.players.cold[p].ca);
        if (d.current.mid - truth).abs() < 0.05 {
            exact += 1;
        }
        sum_err += (d.current.mid - truth).abs();
        sum_truth_var += (truth - 100.0).abs();
    }
    assert!(disagreeing * 2 >= n, "staff readings differ for most players: {disagreeing} of {n}");
    assert_eq!(exact, 0, "the club's belief is never exactly the hidden ability");
    assert!(sum_err / (n as f32) < sum_truth_var / (n as f32), "but it is far closer to the truth than a blind guess");
}

#[test]
fn exposure_sets_confidence_and_unseen_players_are_less_certain() {
    let sim = world();
    let w = &sim.world;
    let club = club_with_staff(w);
    let own: Vec<PlayerId> = w.teams[w.clubs[club].first_team()].squad.clone();
    let mine = own.iter().filter_map(|&p| w.dossiers.get(club, p)).map(|d| d.current.band).sum::<f32>() / own.len() as f32;
    // Someone at another club, never seen, is judged from the same staff with almost no exposure.
    let other = w.clubs.ids().find(|&c| c != club && w.clubs[c].nation == w.clubs[club].nation).unwrap();
    let stranger = w.teams[w.clubs[other].first_team()].squad.iter().copied().find(|&p| w.knowledge.seen(club, p).is_none() && w.scouting.of(club, p).is_empty()).expect("an unseen player");
    let unseen = dossier::build(w, club, stranger).expect("a dossier");
    assert!(unseen.current.band > mine * 1.5, "unseen band {} against own squad {}", unseen.current.band, mine);
    assert!(!unseen.evidence.training && unseen.evidence.minutes_seen == 0);
    assert!(unseen.current_confidence <= Confidence::Medium, "{:?}", unseen.current_confidence);
    // What could not be judged is marked unknown, not assumed fine.
    assert!(unseen.risks.iter().any(|r| !r.known), "with no exposure some risks are flagged as unknown");
}

#[test]
fn the_manager_weighs_inherited_and_proven_staff_differently() {
    let mut sim = world();
    let club = club_with_staff(&sim.world);
    let w = &mut sim.world;
    let manager = w.clubs[club].manager;
    let staff: Vec<StaffId> = w.clubs[club].staff.iter().copied().filter(|&s| s != manager && w.staff[s].role != StaffRole::Scout).collect();
    let s = staff[0];
    // Same person, worked together for years and hired by the manager...
    w.staff[manager].joined = w.date.add_days(-4 * 365);
    w.staff[s].joined = w.date.add_days(-3 * 365);
    let hired = dossier::trust(w, manager, s);
    // ...against inherited from the previous regime.
    w.staff[s].joined = w.date.add_days(-9 * 365);
    let inherited = dossier::trust(w, manager, s);
    assert!(hired > inherited, "hired {hired} vs inherited {inherited}");
    // A record of being right raises trust; a record of being wrong lowers it.
    w.dossiers.track.insert(s, pw_world::dossier::Track { right: 8, wrong: 0 });
    let proven = dossier::trust(w, manager, s);
    w.dossiers.track.insert(s, pw_world::dossier::Track { right: 0, wrong: 8 });
    let wrong = dossier::trust(w, manager, s);
    assert!(proven > inherited && inherited > wrong, "{proven} {inherited} {wrong}");
    assert!((0.1..=1.0).contains(&wrong) && (0.1..=1.0).contains(&proven));
}

#[test]
fn a_more_trusted_evaluator_moves_the_clubs_reading_more() {
    let mut sim = world();
    // A player somewhere whose most optimistic reader is not the manager himself.
    let (club, p, loudest) = {
        let w = &sim.world;
        w.dossiers
            .map
            .iter()
            .filter(|(_, d)| d.opinions.len() >= 3 && d.spread > 3.0)
            .filter_map(|(&(c, p), d)| {
                let top = d.opinions.iter().max_by(|a, b| a.ca.mid.total_cmp(&b.ca.mid))?;
                (top.by != w.clubs[c].manager && top.ca.mid > d.current.mid + 1.0).then_some((c, p, top.by))
            })
            .min()
            .expect("a disputed player whose loudest reader is not the manager")
    };
    let w = &mut sim.world;
    w.dossiers.track.insert(loudest, pw_world::dossier::Track { right: 12, wrong: 0 });
    let up = dossier::build(w, club, p).unwrap().current.mid;
    w.dossiers.track.insert(loudest, pw_world::dossier::Track { right: 0, wrong: 12 });
    let down = dossier::build(w, club, p).unwrap().current.mid;
    assert!(up > down, "trusting the most optimistic evaluator raises the reading: {up} vs {down}");
}

#[test]
fn the_same_dossier_is_worth_different_amounts_to_different_philosophies() {
    let mut sim = world();
    let (club, young) = {
        let w = &sim.world;
        w.dossiers
            .map
            .iter()
            .filter(|(_, d)| w.age(d.player) <= 20 && d.ceiling.mid > d.current.mid + 8.0 && d.viewer.is_some())
            .map(|(&(c, p), _)| (c, p))
            .min()
            .expect("a promising youngster on some club's books")
    };
    let w = &mut sim.world;
    let manager = w.dossiers.get(club, young).unwrap().viewer;
    w.staff[manager].philosophy.youth_trust = 95;
    let developer = dossier::worth(w, club, young, 1.0).unwrap();
    w.staff[manager].philosophy.youth_trust = 5;
    let win_now = dossier::worth(w, club, young, 1.0).unwrap();
    assert!(developer > win_now, "a youth-trusting manager sees more in him: {developer} vs {win_now}");
    // A manager who cannot afford the wait discounts the growth even if he trusts youth.
    w.staff[manager].philosophy.youth_trust = 95;
    let no_time = dossier::worth(w, club, young, 0.1).unwrap();
    assert!(no_time < developer, "{no_time} vs {developer}");
}

#[test]
fn a_new_manager_reads_the_same_evidence_differently_and_the_change_is_explained() {
    let mut sim = world();
    let club = club_with_staff(&sim.world);
    let w = &mut sim.world;
    let old = w.clubs[club].manager;
    let assistant = w.clubs[club].staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::Assistant).expect("an assistant");
    let p = w.teams[w.clubs[club].first_team()].squad[3];
    // Promote the assistant, as a sacking would.
    w.clubs[club].manager = assistant;
    w.staff[assistant].role = StaffRole::Manager;
    let before = w.dossiers.get(club, p).unwrap().clone();
    assert_eq!(before.viewer, old);
    w.date = w.date.add_days(31);
    // The monthly pass rebuilds it under the new manager and records why anything moved.
    dossier::monthly(w);
    let after = w.dossiers.get(club, p).unwrap();
    assert_eq!(after.viewer, assistant);
    assert_ne!(before.opinions.iter().map(|o| o.weight.to_bits()).collect::<Vec<_>>(), after.opinions.iter().map(|o| o.weight.to_bits()).collect::<Vec<_>>(), "the weighing changed");
    let moved = (before.current.mid - after.current.mid).abs() >= 4.0 || (before.ceiling.mid - after.ceiling.mid).abs() >= 6.0;
    if moved {
        assert!(after.history.iter().any(|r| r.reason == Reason::NewManager), "{:?}", after.history);
    }
}

#[test]
fn readings_that_turn_out_far_off_cost_an_evaluator_standing_and_a_vindication_becomes_history() {
    let mut sim = world();
    let club = club_with_staff(&sim.world);
    let w = &mut sim.world;
    let scout = w.clubs[club].staff.iter().copied().find(|&s| w.staff[s].role != StaffRole::Manager).unwrap();
    // A star of the world, whom this evaluator once thought an ordinary prospect.
    let star = w.players.ids().max_by_key(|&p| w.players.cold[p].ca).unwrap();
    w.players.cold[star].rep.world = 9000;
    let today = w.date;
    let public = pw_sim::market::public_view(w, star).0;
    w.dossiers.pending.push(Pending { by: scout, club, player: star, date: today.add_days(-366), ca: public - 60.0, pa: public - 40.0 });
    let events = w.events.len();
    dossier::settle(w);
    assert!(w.dossiers.track.get(&scout).is_some_and(|t| t.wrong == 1), "a reading 60 points out is a miss");
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, pw_world::EventKind::AssessmentVindicated { was_right: false, .. })), "and it is remembered publicly");
    assert!(w.dossiers.pending.is_empty(), "checked readings are not kept");
}
