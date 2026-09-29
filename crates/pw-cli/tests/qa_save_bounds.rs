//! QA: historical detail shrinks after it stops affecting the world, while rows and references remain stable.
mod qa_common;

use pw_import::synthetic::Scale;
use pw_world::{PlayerStatus, TeamKind};
use qa_common::*;

#[test]
fn the_size_of_a_world_grows_sub_linearly_over_six_years() {
    let mut s = sim(Scale::TINY, 3);
    let mut sizes = Vec::new();
    for _ in 0..6 {
        s.run(365);
        let sections = pw_sim::metrics::section_sizes(&s.world);
        sizes.push(sections.iter().map(|(_, bytes)| bytes).sum::<u64>() as f64);
        eprintln!(
            "year {}: {:.1} MB; {}",
            sizes.len(),
            sizes.last().unwrap() / 1e6,
            sections.iter().take(8).map(|(name, bytes)| format!("{name} {:.1}", *bytes as f64 / 1e6)).collect::<Vec<_>>().join(", ")
        );
    }
    let (year2, year3, year5, year6) = (sizes[1], sizes[2], sizes[4], sizes[5]);
    assert!(year6 < 1.6 * year3, "year 6 is {:.1} MB against {:.1} MB in year 3: {:?}", year6 / 1e6, year3 / 1e6, sizes.iter().map(|x| (x / 1e5).round() / 10.0).collect::<Vec<_>>());
    let (early_addition, late_addition) = (sizes[1] - sizes[0], sizes[5] - sizes[4]);
    assert!(late_addition < 0.75 * early_addition, "year 2 added {:.2} MB, year 6 added {:.2} MB", early_addition / 1e6, late_addition / 1e6);
    assert!(year5 > 0.0 && year2 > 0.0);
}

#[test]
fn dossiers_are_only_about_players_a_club_has_a_current_reason_to_assess() {
    let s = ran(Scale::TINY, 4, 2 * 365);
    let w = &s.world;
    let wanted: std::collections::HashSet<_> = pw_sim::dossier::wanted(w).into_iter().collect();
    let mut stale = 0;
    for (&(club, player), dossier) in &w.dossiers.map {
        let status = w.players.hot[player].status;
        // Retirement can happen after the monthly dossier pass; that last reading disappears at the next pass.
        if status == PlayerStatus::Retired {
            assert!(dossier.date.days_until(w.date) < 130, "retired player {player:?} has a stale dossier from {:?}", dossier.date);
        }
        if !wanted.contains(&(club, player)) {
            stale += 1;
            assert!(dossier.date.days_until(w.date) < 130, "dossier on {player:?} for {club:?} outlived its reason: {:?}", dossier.date);
        }
        if status == PlayerStatus::Amateur {
            assert!(!wanted.contains(&(club, player)));
        }
    }
    for &(club, player) in &wanted {
        let h = &w.players.hot[player];
        assert_ne!(h.status, PlayerStatus::Amateur);
        if h.team.is_some() && w.teams[h.team].club == club {
            assert!(matches!(w.teams[h.team].kind, TeamKind::First | TeamKind::Reserve), "youth player {player:?} is in the club's own dossier book");
        }
    }
    let mut per_club = std::collections::HashMap::new();
    for &(club, _) in &wanted {
        *per_club.entry(club).or_insert(0usize) += 1;
    }
    let max = per_club.values().copied().max().unwrap_or(0);
    assert!(max <= 400, "one club holds {max} dossiers");
    assert!(stale <= w.dossiers.map.len() / 2 + 1);

    let amateur = w.players.hot.iter_enumerated().find(|(_, h)| h.status == PlayerStatus::Amateur).map(|(p, _)| p).expect("world has an amateur");
    let club = w.clubs.ids().next().expect("world has a club");
    let (ca, band, _, _) = pw_sim::scouting::view(w, club, amateur);
    assert!(ca.is_finite() && band > 0.0, "a missing dossier falls back to the club's general scouting view");
}

#[test]
fn closed_talk_detail_is_compacted_without_dropping_rows_or_private_limits() {
    let mut s = sim(Scale::TINY, 5);
    s.run(3 * 365 + 30);
    let w = &s.world;
    let today = w.date;
    let old_talks: Vec<_> = w.talks.iter().filter(|talk| !talk.is_open() && talk.opened.days_until(today) > 760).collect();
    assert!(old_talks.len() > 20, "expected old talks to inspect; found {}", old_talks.len());
    for talk in old_talks {
        assert!(talk.log.len() <= 1 && talk.moves.is_empty() && talk.ask.is_none(), "old closed talk from {:?} retained round detail", talk.opened);
        assert!(talk.end.is_some());
    }

    // A grapevine index may only point at a person who still holds that item after compaction.
    for (&person, ids) in &w.grapevine.by_person {
        for &id in ids {
            assert!(w.grapevine.items[id as usize].knows(person), "stale knowledge index: {person:?} -> item {id}");
        }
    }
    assert!(pw_sim::validate::problems(w).is_empty());
    assert!(pw_sim::audit::audit(w).is_empty());
}
