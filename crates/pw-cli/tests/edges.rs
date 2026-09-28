//! Edge cases and properties: saves with decisions pending, unusual names,
//! reply and telling graphs without cycles, rule history reconstruction,
//! record sequences, and text checks on arbitrary input.

use pw_core::PersonId;
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_world::{MindKind, World};
use proptest::prelude::*;

fn ran(seed: u64, days: u32) -> Sim {
    let mut s = Sim::new(synthetic::build(DataPack::builtin(), seed, Scale::TINY));
    s.run(days);
    s
}

#[test]
fn a_save_with_a_pending_decision_resumes_it() {
    let mut s = ran(61, 20);
    // A human player: run until a decision is waiting for them.
    let who = {
        let w = &s.world;
        let t = w.teams.iter().find(|t| t.kind == pw_world::TeamKind::First).unwrap();
        w.players.cold[t.squad[0]].person
    };
    s.world.people[who].mind = MindKind::External;
    let mut pending = None;
    for _ in 0..400 {
        s.step();
        if let Some((id, _)) = s.world.decisions.pending_for(who).find(|(_, d)| d.answer.is_none()) {
            pending = Some(id);
            break;
        }
    }
    let Some(id) = pending else { return };
    let dir = std::env::temp_dir().join(format!("pw-edge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("w.sav");
    pw_sim::save::save(&s.world, &path).unwrap();
    let mut b = Sim::new(pw_sim::save::load::<World>(&path).unwrap());
    assert!(b.world.decisions.all.get(id).is_some_and(|d| !d.resolved), "the decision survived the save");
    assert!(b.answer(id, 0), "it can still be answered");
    b.run(10);
    assert!(b.world.decisions.all[id].resolved, "and it resolved");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unicode_and_very_long_names_render_cleanly() {
    let mut s = ran(63, 10);
    let w = &mut s.world;
    let long = "Maximiliano-Bartholomäus Øyvind-Świętosław de la Cruz-Nakamura-Ó Súilleabháin";
    let (a, b) = (w.names.intern("Zoë Ångström-Łukasiewicz"), w.names.intern(long));
    let people: Vec<PersonId> = w.teams.iter().filter(|t| t.kind == pw_world::TeamKind::First).flat_map(|t| t.squad.iter().take(2).map(|&p| w.players.cold[p].person).collect::<Vec<_>>()).collect();
    for (i, &p) in people.iter().enumerate() {
        w.people[p].first = if i % 2 == 0 { a } else { b };
        w.people[p].last = b;
    }
    s.run(200);
    let w = &s.world;
    for e in w.events.since(pw_core::Date(0)) {
        if let Some(line) = pw_narrate::events::line(w, e, PersonId::NONE) {
            let issues: Vec<_> = pw_narrate::quality::check(&line, true, 1000).into_iter().filter(|i| *i != pw_narrate::quality::Issue::Lowercase).collect();
            assert!(issues.is_empty(), "{issues:?}: {line}");
        }
    }
    // Handles stay alphanumeric and non-empty.
    assert!(w.net.accounts.iter().all(|a| !a.handle.is_empty()));
}

#[test]
fn replies_and_tellings_have_no_cycles() {
    let s = ran(67, 400);
    let w = &s.world;
    for p in &w.net.posts {
        for r in [p.reply_to, p.quote_of].into_iter().chain(p.refs.iter().copied()) {
            if r != pw_world::socialnet::NO_POST {
                assert!(r < p.id, "post {} refers forward to {r}", p.id);
            }
        }
    }
    for it in &w.grapevine.items {
        let mut seen = std::collections::HashSet::new();
        for k in &it.holders {
            assert!(seen.insert(k.person), "item {} learned twice by one person", it.id);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn quality_checks_never_panic(s in "\\PC{0,200}") {
        let _ = pw_narrate::quality::check(&s, true, 100);
    }

    #[test]
    fn rule_history_reconstructs_every_season(changes in proptest::collection::vec((1i32..40, 3i32..8), 0..8)) {
        use pw_world::evolution::{Evolution, RuleCause, RuleChange, RuleKey};
        let n = pw_core::NationId(0);
        let mut ev = Evolution::default();
        let mut value = 3;
        let mut timeline: Vec<(i32, i32)> = vec![(i32::MIN, 3)];
        let mut season = 2000;
        for (i, (gap, new)) in changes.into_iter().enumerate() {
            season += gap;
            ev.changes.push(RuleChange { id: i as u32, nation: n, key: RuleKey::Subs, old: value, new, from_season: season, date: pw_core::Date(0), cause: RuleCause::InjuryCrisis { per_club: 0.0 } });
            value = new;
            timeline.push((season, new));
        }
        for probe in 1990..season + 5 {
            let expect = timeline.iter().rev().find(|(from, _)| *from <= probe).map(|x| x.1).unwrap();
            prop_assert_eq!(ev.value_in(n, RuleKey::Subs, probe, value), expect);
        }
    }

    #[test]
    fn records_only_ever_improve(values in proptest::collection::vec(0i64..100, 1..30)) {
        use pw_world::minor::Level;
        use pw_world::records::{Holder, Mark, RecordKey, Scope, Stat};
        let mut s = Sim::new(synthetic::build(DataPack::builtin(), 71, Scale::TINY));
        let w = &mut s.world;
        let key = RecordKey { scope: Scope::World, stat: Stat::GoalsInSeason, level: Level::Amateur };
        let mut best: Option<i64> = None;
        for (i, v) in values.iter().enumerate() {
            let m = Mark { holder: Holder::Person(PersonId(i as u32)), value: *v, date: pw_core::Date(i as i32), against: None };
            pw_sim::records::note(w, key, m, 0, false);
            best = Some(best.map_or(*v, |b| b.max(*v)));
            prop_assert_eq!(w.records.get(&key).map(|r| r.current.value), best);
        }
        let r = w.records.get(&key).unwrap();
        let mut chain: Vec<i64> = r.previous.iter().map(|m| m.value).collect();
        chain.push(r.current.value);
        prop_assert!(chain.windows(2).all(|x| x[0] < x[1]), "holders' marks rise: {:?}", chain);
    }
}
