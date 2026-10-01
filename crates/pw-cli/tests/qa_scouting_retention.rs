//! Regression cases for the bounded recruitment working set.
mod qa_common;

use pw_core::{Date, StaffId};
use pw_import::synthetic::Scale;
use pw_world::PlayerStatus;
use pw_world::scouting::{Assignment, Brief, Report, Verdict};
use qa_common::*;

#[test]
fn a_report_book_is_bounded_deterministic_and_keeps_explicit_targets() {
    let mut s = sim(Scale::SMALL, 44);
    let w = &mut s.world;
    let club = w.clubs.ids().next().unwrap();
    let own = w.teams[w.clubs[club].first_team()].squad[0];
    let others: Vec<_> = w.players.ids().filter(|&p| w.players.hot[p].club != club && w.players.hot[p].status == PlayerStatus::Active).take(pw_sim::scouting::GENERAL_REPORT_LIMIT + 100).collect();
    assert_eq!(others.len(), pw_sim::scouting::GENERAL_REPORT_LIMIT + 100);
    let target = *others.last().unwrap();
    let loaned = others[others.len() - 2];
    let shortlisted = others[others.len() - 3];
    let retired = others[others.len() - 4];
    let trial = others[others.len() - 5];
    let contracted = others[others.len() - 6];
    let academy_trial = w.players.ids().filter(|&p| w.players.hot[p].status == PlayerStatus::Amateur && w.age(p) < 16).last().unwrap();
    let borrower = w.players.hot[loaned].club;
    w.players.cold[loaned].contract.club = club;
    w.players.cold[loaned].loan = Some(pw_world::contract::Loan { parent: club, club: borrower, start: w.date, end: w.date.add_days(120), wage_share: 50, fee: 0, buy_option: 0, recall: true });
    w.deals.shortlists.insert((club, pw_core::PosGroup::Mid), pw_world::deals::Shortlist { updated: w.date, targets: [(shortlisted, 1.0)].into_iter().collect(), failures: 0 });
    pw_sim::people::retire(w, retired);
    w.deals.trials.push(pw_world::deals::Trial { player: trial, club, from: w.date, until: w.date.add_days(21) });
    let terms = pw_world::negotiation::Terms::from_contract(&w.players.cold[contracted].contract, 3);
    w.deals.pre_contracts.push(pw_world::deals::PreContract { player: contracted, club, terms, signed: w.date });
    w.youth.trials.push(pw_world::youth::AcademyTrial { player: academy_trial, club, from: w.date, until: w.date.add_days(21) });
    let report = Report { scout: StaffId(0), date: w.date.add_days(-30), minutes: 180, ca: 100, pa: 125, band: 15, grade: 3, verdict: Verdict::Monitor, notes: Default::default(), context: 5000 };
    w.scouting.reports.clear();
    for &p in &others {
        w.scouting.file(club, p, report.clone());
        w.knowledge.observe(club, p, 180, w.date);
    }
    w.scouting.file(club, own, Report { date: Date(w.date.0 - 700), ..report.clone() });
    w.scouting.file(club, academy_trial, report.clone());
    w.scouting.assignments.push(Assignment { scout: StaffId(0), club, brief: Brief::Player(target), since: w.date, until: w.date.add_days(30) });
    let mut reversed = w.clone();
    reversed.scouting.reports.clear();
    for &p in others.iter().rev() {
        reversed.scouting.file(club, p, report.clone());
    }
    reversed.scouting.file(club, own, Report { date: Date(w.date.0 - 700), ..report });
    reversed.scouting.file(club, academy_trial, w.scouting.of(club, academy_trial)[0].clone());
    pw_sim::scouting::compact_reports(w);
    pw_sim::scouting::compact_reports(&mut reversed);
    assert_eq!(w.scouting.reports.len(), pw_sim::scouting::GENERAL_REPORT_LIMIT + 7);
    assert!(!w.scouting.of(club, target).is_empty());
    assert!(!w.scouting.of(club, own).is_empty());
    assert!(!w.scouting.of(club, loaned).is_empty());
    assert!(!w.scouting.of(club, shortlisted).is_empty());
    assert!(w.scouting.of(club, retired).is_empty());
    for p in [trial, contracted, academy_trial] {
        assert!(!w.scouting.of(club, p).is_empty(), "a current commitment lost its report: {p:?}");
    }
    let keys = |world: &pw_world::World| { let mut keys: Vec<_> = world.scouting.reports.keys().copied().collect(); keys.sort(); keys };
    assert_eq!(keys(w), keys(&reversed), "insertion order cannot choose the reports that survive");
    let forgotten = others[pw_sim::scouting::GENERAL_REPORT_LIMIT + 1];
    assert!(w.scouting.of(club, forgotten).is_empty());
    assert!(w.knowledge.seen(club, forgotten).is_some(), "compact exposure evidence survives full report retirement");
    pw_sim::scouting::compact_reports(w);
    assert_eq!(keys(w), keys(&reversed));

    let path = temp_path("bounded-scouting");
    pw_sim::save::save_with(w, &path, &pw_sim::save::Info::of_world(w)).unwrap();
    let mut loaded = pw_sim::save::load_world(&path).unwrap();
    cleanup(&path);
    pw_sim::scouting::compact_reports(&mut loaded);
    assert_eq!(keys(w), keys(&loaded), "reloading cannot choose a different working report book");
    assert_eq!(w.knowledge.seen(club, forgotten), loaded.knowledge.seen(club, forgotten));
}
