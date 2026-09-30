//! The language engine on real world stories: what it writes for an India world must be clean, must not say more than the story does, and
//! must cover the stories it claims to.

use pw_data::DataPack;
use pw_import::india::{self, IndiaScale};
use pw_narrate::lang;
use pw_sim::Sim;
use pw_world::media::StoryKind;

fn india_world(seed: u64, days: u32) -> Sim {
    let mut s = Sim::new(india::build(DataPack::builtin(), seed, IndiaScale::TINY));
    s.run(days);
    s
}

#[test]
fn stories_the_engine_writes_are_clean_and_no_firmer_than_the_story() {
    let s = india_world(51, 500);
    let w = &s.world;
    let (mut written, mut fallback) = (0usize, 0usize);
    let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
    for (_, st) in w.media.stories.iter_enumerated() {
        match lang::story(w, st) {
            Some(t) => {
                written += 1;
                *kinds.entry(format!("{:?}", st.kind)).or_default() += 1;
                for text in [&t.headline, &t.body] {
                    let issues = pw_lang::check::check_text(text);
                    assert!(issues.is_empty(), "{:?} story {}: {issues:?}\n{}\n{}", st.kind, st.id.0, t.headline, t.body);
                }
                assert!(!t.headline.trim().is_empty() && !t.body.trim().is_empty());
                // A rumour that is not close to a deal is never written as one.
                if st.kind == StoryKind::TransferRumour && st.claim < 75 {
                    let lower = format!("{} {}", t.headline, t.body).to_lowercase();
                    assert!(!lower.contains(" bid") && !lower.contains("agreed") && !lower.contains("completed"), "a claim of {} was written as a done deal: {}", st.claim, lower);
                }
            }
            None => fallback += 1,
        }
    }
    eprintln!("engine wrote {written}, older templates {fallback}: {kinds:?}");
    assert!(written > 20, "the engine should carry a real share of the news: {written} of {}", written + fallback);
}

#[test]
fn the_same_story_reads_the_same_way_twice() {
    let s = india_world(52, 400);
    let w = &s.world;
    let mut n = 0;
    for (_, st) in w.media.stories.iter_enumerated() {
        if let (Some(a), Some(b)) = (lang::story(w, st), lang::story(w, st)) {
            assert_eq!((a.headline, a.body), (b.headline, b.body));
            n += 1;
        }
    }
    assert!(n > 0);
}

/// A sample of what it writes: `cargo test -p pw-cli --test lang_bridge sample -- --ignored --nocapture`.
#[test]
#[ignore = "report"]
fn sample() {
    let s = india_world(53, 500);
    let w = &s.world;
    let mut shown = 0;
    for (_, st) in w.media.stories.iter_enumerated() {
        if !matches!(st.kind, StoryKind::Interview) { continue; }
        if let Some(t) = lang::story(w, st) {
            eprintln!("[{:?} / {}]\n{}\n{}\n", st.kind, pw_narrate::press::outlet_name(w, st), t.headline, t.body);
            shown += 1;
            if shown >= 400 || false {
                break;
            }
        }
    }
}

#[test]
#[ignore = "debug"]
fn why_not() {
    let s = india_world(54, 300);
    let w = &s.world;
    let mut by: std::collections::BTreeMap<String, (usize, usize, usize)> = Default::default();
    for (_, st) in w.media.stories.iter_enumerated() {
        let e = by.entry(format!("{:?}", st.kind)).or_default();
        e.0 += 1;
        if let Some(ev) = w.events.get(st.event) {
            e.1 += 1;
            if lang::event(w, &ev.kind, ev.date).is_some() {
                e.2 += 1;
            }
        }
    }
    for (k, v) in by {
        eprintln!("{k}: stories {} with event {} mappable {}", v.0, v.1, v.2);
    }
}

#[test]
#[ignore = "debug"]
fn chain() {
    let s = india_world(54, 300);
    let w = &s.world;
    let mut shown = std::collections::BTreeMap::<String, usize>::new();
    for (_, st) in w.media.stories.iter_enumerated() {
        if !matches!(st.kind, StoryKind::TransferNews | StoryKind::TransferRumour | StoryKind::ManagerChange | StoryKind::Season) {
            continue;
        }
        let n = shown.entry(format!("{:?}", st.kind)).or_default();
        if *n >= 3 {
            continue;
        }
        *n += 1;
        let e = w.events.get(st.event).unwrap();
        eprintln!("{:?} claim {} fee {} club {:?} other {:?} player {:?} :: event {:?} causes {:?}", st.kind, st.claim, st.fee, st.club.0, st.other_club.0, st.player.0, e.kind, e.causes);
        for c in &e.causes {
            if let pw_world::event::Cause::Event(x) = c {
                if let Some(u) = w.events.get(*x) {
                    eprintln!("    -> {:?}", u.kind);
                }
            }
        }
    }
}

#[test]
#[ignore = "debug"]
fn notes() {
    let s = india_world(54, 300);
    let w = &s.world;
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    for (_, st) in w.media.stories.iter_enumerated() {
        if let Err(e) = lang::try_story(w, st) {
            if !e.starts_with("no engine event") {
                let k: String = e.split_whitespace().filter(|x| x.trim_end_matches(':').parse::<u32>().is_err()).collect::<Vec<_>>().join(" ");
                *seen.entry(k).or_default() += 1;
            }
        }
    }
    for (k, n) in seen {
        eprintln!("NOTE {n}x {k}");
    }
}

fn decision_for(w: &pw_world::World, p: pw_core::PlayerId, kind: pw_world::decision::DecisionKind) -> pw_world::decision::Decision {
    pw_world::decision::Decision {
        person: w.players.cold[p].person,
        player: p,
        kind,
        options: Default::default(),
        created: w.date,
        deadline: w.date.add_days(7),
        default: 0,
        answer: None,
        resolved: false,
    }
}

#[test]
fn an_invitation_and_a_bid_reach_the_inbox_in_the_engines_words() {
    let s = india_world(55, 30);
    let w = &s.world;
    let club = *w.youth.academies.keys().min().unwrap();
    let kid = w.players.hot.iter_enumerated().find(|(_, h)| h.status == pw_world::PlayerStatus::Amateur).map(|x| x.0).unwrap();
    let trial = decision_for(w, kid, pw_world::decision::DecisionKind::Trial { club, days: 14 });
    let (subject, body) = lang::decision(w, &trial).expect("a trial invitation is worded by the engine");
    assert!(subject.contains(&w.clubs[club].name) && subject.to_lowercase().contains("trial"), "{subject}");
    assert!(body.contains("you") && !body.is_empty(), "{body}");
    assert!(pw_lang::check::check_text(&body).is_empty() && pw_lang::check::check_text(&subject).is_empty());
    // The same words reach the thread title and the message the client shows.
    assert_eq!(pw_narrate::choices::title(w, &trial), subject);
    let pro = w.players.hot.iter_enumerated().find(|(_, h)| h.club.is_some() && h.status == pw_world::PlayerStatus::Active).map(|x| x.0).unwrap();
    let buyer = w.clubs.ids().find(|&c| c != w.players.hot[pro].club).unwrap();
    let talks = decision_for(w, pro, pw_world::decision::DecisionKind::TransferTalks { club: buyer, fee: 25_000_000 });
    let (subject, body) = lang::decision(w, &talks).expect("talks are worded by the engine");
    assert!(subject.contains(&w.clubs[buyer].name), "{subject}");
    assert!(body.contains("Rs") || body.contains('₹'), "the fee is in the world's money: {body}");
    // A decision the engine has no words for stays with the older text.
    let nation = decision_for(w, pro, pw_world::decision::DecisionKind::Treatment { surgery_days: 40, rehab_days: 80 });
    assert!(lang::decision(w, &nation).is_none());
}

#[test]
fn a_post_that_relays_a_story_is_no_firmer_than_the_story_or_the_post() {
    use pw_world::media::ClaimType;
    use pw_world::socialnet::{Concept, Frame};
    let s = india_world(56, 500);
    let w = &s.world;
    let (mut engine_posts, mut relays) = (0, 0);
    for p in w.net.posts.iter() {
        if p.concept != Concept::Relay {
            continue;
        }
        relays += 1;
        let Some(text) = lang::post(w, p) else { continue };
        engine_posts += 1;
        let issues = pw_lang::check::check_text(&text);
        assert!(issues.is_empty(), "{issues:?}: {text}");
        if matches!(p.claim, ClaimType::Rumour | ClaimType::Speculation) {
            let lower = text.to_lowercase();
            assert!(!lower.contains("completed") && !lower.contains("confirmed"), "a rumour was posted as done: {text}");
        }
        assert!(matches!(p.frame, Frame::Story { .. }));
        assert_eq!(text, lang::post(w, p).unwrap(), "posts read the same way each time");
    }
    eprintln!("relay posts {relays}, written by the engine {engine_posts}");
}

#[test]
#[ignore = "debug"]
fn relay_kinds() {
    use pw_world::socialnet::{Concept, Frame};
    let s = india_world(56, 500);
    let w = &s.world;
    let mut by = std::collections::BTreeMap::<String, usize>::new();
    for p in w.net.posts.iter().filter(|p| p.concept == Concept::Relay) {
        let k = match p.frame {
            Frame::Story { story } => format!("story {:?}", w.media.stories[story].kind),
            other => format!("{:?}", std::mem::discriminant(&other)),
        };
        *by.entry(k).or_default() += 1;
    }
    eprintln!("{by:?}");
}
