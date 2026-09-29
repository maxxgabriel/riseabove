//! Slice 1: the rushed return, as a scenario built on a small world.

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::ruling::Outcome;
use pw_world::{Archetype, SquadStatus, World};

struct Case {
    w: World,
    club: pw_core::ClubId,
    m: pw_core::StaffId,
    p: pw_core::PlayerId,
    injury: u16,
}

/// A star, willing, six days from full fitness on a case of twenty, at a club whose manager is
/// under pressure and whose owner does not stop him.
fn setup(seed: u64) -> Case {
    let mut s = Sim::new(synthetic::build(DataPack::builtin(), seed, Scale::TINY));
    s.run(2);
    let mut w = s.world;
    let club = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some() && w.teams[w.clubs[c].first_team()].squad.len() > 12).unwrap();
    let m = w.clubs[club].manager.get().unwrap();
    let p = w.teams[w.clubs[club].first_team()].squad[0];
    w.players.cold[p].status = SquadStatus::Star;
    w.medical.willing_to_rush.insert(p, true);
    w.clubs[club].board.satisfaction = 30;
    w.staff[m].philosophy.archetype = Archetype::Pragmatist;
    if let Some(g) = w.governance.get_mut(&club) {
        g.owner.meddling = 95;
    }
    let injury = (0..w.data.injuries.len()).find(|&i| w.data.injuries[i].region.wear_slot().is_some() && w.data.injuries[i].days[2] >= 20).unwrap() as u16 + 1;
    let mut c = Case { w, club, m, p, injury };
    hurt(&mut c);
    c
}

fn hurt(c: &mut Case) {
    let today = c.w.date;
    c.w.players.hot[c.p].injury = c.injury;
    pw_sim::medical::on_injury(&mut c.w, c.p, c.injury, 20);
    let h = &mut c.w.players.hot[c.p];
    h.injury_total = 20;
    h.injury_days = 5;
    let case = c.w.medical.open.get_mut(&c.p).unwrap();
    case.date = today.add_days(-15);
    case.estimate = 20;
    case.certainty = 90;
}

#[test]
fn a_pressed_manager_rushes_a_star_back_and_the_ruling_remembers_why() {
    let mut c = setup(31);
    pw_sim::returns::consider_rush(&mut c.w, c.club, c.m, c.p, 1.0);
    let r = c.w.ext.medical.rushed.get(&c.p).copied().expect("rushed");
    assert_eq!(c.w.players.hot[c.p].injury, 0);
    assert!(c.w.players.hot[c.p].condition < 90, "returns part-fit");
    assert!(pw_sim::returns::hazard_factor(&c.w, c.p) > 1.5);
    let ruling = c.w.ext.decisions.get(r.ruling).unwrap();
    assert_eq!(ruling.outcome, Outcome::Pending);
    assert!(ruling.stances.len() >= 2 && ruling.stances.iter().any(|s| s.authority));
    assert!(ruling.true_pct > 0, "risk accepted is recorded against the truth");
    assert!(pw_sim::invariants::check(&c.w).is_empty());
}

#[test]
fn a_recurrence_in_the_same_region_fails_the_ruling_and_teaches_caution() {
    let mut c = setup(32);
    pw_sim::returns::consider_rush(&mut c.w, c.club, c.m, c.p, 1.0);
    let id = c.w.ext.medical.rushed[&c.p].ruling;
    hurt(&mut c);
    assert_eq!(c.w.ext.decisions.get(id).unwrap().outcome, Outcome::Failed);
    assert!(c.w.ext.medical.rush_bias[&c.m] < 0);
    assert!(c.w.medical.open[&c.p].recurrence);
    assert!(!c.w.ext.medical.rushed.contains_key(&c.p));
}

#[test]
fn an_uneventful_window_vindicates_the_call() {
    let mut c = setup(33);
    pw_sim::returns::consider_rush(&mut c.w, c.club, c.m, c.p, 1.0);
    let id = c.w.ext.medical.rushed[&c.p].ruling;
    c.w.date = c.w.date.add_days(pw_sim::returns::WINDOW_DAYS + 2);
    pw_sim::returns::pre_match(&mut c.w);
    assert_eq!(c.w.ext.decisions.get(id).unwrap().outcome, Outcome::Held);
    assert!(c.w.ext.medical.rush_bias[&c.m] > 0);
}

#[test]
fn a_low_stakes_match_or_a_far_off_case_is_not_rushed() {
    let mut c = setup(34);
    c.w.players.hot[c.p].injury_days = 15; // 75% still to run
    pw_sim::returns::consider_rush(&mut c.w, c.club, c.m, c.p, 1.0);
    assert!(c.w.ext.medical.rushed.is_empty());
    let mut d = setup(35);
    d.w.players.cold[d.p].status = SquadStatus::Backup;
    pw_sim::returns::consider_rush(&mut d.w, d.club, d.m, d.p, 0.2);
    assert!(d.w.ext.medical.rushed.is_empty());
}

/// Injuries per player-season in a running world (all causes, illness excluded), for calibration.
/// Professional football: about 2 time-loss injuries per player-season.
/// `cargo test --release -p pw-cli --test returns season_rate -- --ignored --nocapture`
#[test]
#[ignore = "report"]
fn season_rate() {
    use pw_data::DataPack;
    use pw_import::synthetic::{self, Scale};
    use pw_sim::Sim;
    // TRAINING=0 isolates match injuries; otherwise everything in the tuning file.
    let mut pack = DataPack::builtin();
    if std::env::var("NO_TRAINING_INJURY").is_ok() {
        pack.tuning.health.training_session = 0.0;
    }
    if let Ok(v) = std::env::var("TRAINING_SESSION") {
        pack.tuning.health.training_session = v.parse().unwrap();
    }
    let mut s = Sim::new(synthetic::build(pack, 51, Scale::TINY));
    s.run(365);
    let start = s.world.date;
    s.run(365);
    let w = &s.world;
    let cases: Vec<_> = w.medical.open.values().chain(w.medical.history.values().flatten()).filter(|c| c.region != u8::MAX && start.days_until(c.date) >= 0).collect();
    let squad = w.players.hot.iter_enumerated().filter(|(_, h)| h.club.is_some() && h.status != pw_world::PlayerStatus::Retired).count();
    let per: f64 = cases.len() as f64 / squad.max(1) as f64;
    let mean_days = cases.iter().map(|c| f64::from(c.actual.max(c.estimate))).sum::<f64>() / cases.len().max(1) as f64;
    // Regulars only (900+ minutes in the year): what the professional figures describe.
    let mut minutes: std::collections::HashMap<pw_core::PlayerId, u32> = Default::default();
    for l in w.stats.iter().chain(w.history.lines.iter().filter(|l| l.season >= start.year())) {
        *minutes.entry(l.player).or_default() += l.minutes;
    }
    let regulars: Vec<_> = minutes.iter().filter(|x| *x.1 >= 900).map(|x| *x.0).collect();
    let reg_cases = cases.iter().filter(|c| regulars.contains(&c.player)).count();
    let match_hours: f64 = minutes.values().map(|&m| f64::from(m)).sum::<f64>() / 60.0;
    eprintln!("regulars {}: {:.2} injuries per regular-season; {:.1} injuries per 1000 match-hours overall ({:.0} match-hours)", regulars.len(), reg_cases as f64 / regulars.len().max(1) as f64, cases.len() as f64 / match_hours * 1000.0, match_hours);
    eprintln!("season rate: {} injuries over {squad} club players = {per:.2} per player-season, mean {mean_days:.0} days", cases.len());
}
