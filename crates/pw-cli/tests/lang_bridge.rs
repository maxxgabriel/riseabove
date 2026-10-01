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
    // Two worlds: a rare bad phrasing shows up in one seed and not another.
    for seed in [51, 61] {
        clean_and_no_firmer(seed);
    }
}

fn clean_and_no_firmer(seed: u64) {
    let s = india_world(seed, 500);
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
        if !matches!(st.kind, StoryKind::Unhappy | StoryKind::Praise | StoryKind::AwardNews | StoryKind::Milestone) { continue; }
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
fn the_engines_options_are_the_decisions_own_choices_with_what_each_does() {
    use pw_world::decision::{Choice, DecisionKind};
    let s = india_world(55, 30);
    let w = &s.world;
    let with_choices = |mut d: pw_world::decision::Decision| {
        d.options = d.kind.simple_options();
        d
    };
    let club = *w.youth.academies.keys().min().unwrap();
    let kid = w.players.hot.iter_enumerated().find(|(_, h)| h.status == pw_world::PlayerStatus::Amateur).map(|x| x.0).unwrap();
    let uni = w.minor.institutions.iter().find(|i| i.kind == pw_world::minor::InstKind::University).map(|i| i.id).unwrap();
    let pro = w.players.hot.iter_enumerated().find(|(_, h)| h.club.is_some() && h.status == pw_world::PlayerStatus::Active).map(|x| x.0).unwrap();
    let buyer = w.clubs.ids().find(|&c| c != w.players.hot[pro].club).unwrap();
    for d in [
        with_choices(decision_for(w, kid, DecisionKind::Trial { club, days: 14 })),
        with_choices(decision_for(w, kid, DecisionKind::Scholarship { institution: uni, tier: 2 })),
        with_choices(decision_for(w, pro, DecisionKind::TransferTalks { club: buyer, fee: 25_000_000 })),
    ] {
        let opts = lang::decision_options(w, &d);
        assert_eq!(opts.len(), 2, "{:?}: {opts:?}", d.kind);
        // Each engine option is one of the decision's choices, accept and reject both covered, and says what it does in clean words.
        let chosen: Vec<Choice> = opts.iter().map(|o| d.options[o.index]).collect();
        assert!(chosen.contains(&Choice::Accept) && chosen.contains(&Choice::Reject), "{:?}", d.kind);
        for o in &opts {
            assert!(!o.label.is_empty() && !o.consequence.is_empty());
            assert!(pw_lang::check::check_text(&o.consequence).is_empty(), "{}", o.consequence);
        }
        assert!(lang::decision(w, &d).is_some(), "{:?} is worded", d.kind);
    }
    // An effect no decision can carry out is never offered.
    assert!(lang::effect_choice("transfer.open_bid").is_none() && lang::effect_choice("inbox.dismiss").is_none());
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

#[test]
#[ignore = "debug"]
fn uncovered() {
    let s = india_world(54, 500);
    let w = &s.world;
    let mut by = std::collections::BTreeMap::<String, usize>::new();
    for (_, st) in w.media.stories.iter_enumerated() {
        if lang::story(w, st).is_none() {
            let link = match w.media.links.get(&st.id) {
                Some(pw_world::media::StoryLink::Milestone(k, _)) => format!("milestone {k:?}"),
                Some(pw_world::media::StoryLink::Record(k, _)) => format!("record {k:?}"),
                Some(l) => format!("{}", format!("{l:?}").split(|c: char| !c.is_alphanumeric()).next().unwrap_or("")),
                None => "no link".into(),
            };
            *by.entry(format!("{:?} / {link}", st.kind)).or_default() += 1;
        }
    }
    let mut v: Vec<_> = by.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, n) in v.into_iter().take(25) {
        eprintln!("UNCOVERED {n:5} {k}");
    }
}

#[test]
fn an_old_story_reads_the_same_as_the_world_ages() {
    let mut s = india_world(57, 300);
    let before: Vec<(u32, String, String)> = s
        .world
        .media
        .stories
        .iter_enumerated()
        .filter_map(|(id, st)| lang::story(&s.world, st).map(|t| (id.0, t.headline, t.body)))
        .collect();
    assert!(before.len() > 100);
    s.run(400);
    let mut changed = Vec::new();
    for (id, h, b) in &before {
        let st = s.world.media.stories.get(pw_core::StoryId(*id)).expect("stories are kept");
        let now = match lang::try_story(&s.world, st) {
            Ok(t) => t,
            Err(why) => {
                changed.push(format!("{id} ({:?}, published {:?}): no longer writable: {why}", st.kind, st.date));
                continue;
            }
        };
        if (&now.headline, &now.body) != (h, b) {
            changed.push(format!("{id}: {h} / {b}\n   now: {} / {}", now.headline, now.body));
        }
    }
    assert!(changed.is_empty(), "{} of {} stories read differently a year later, e.g.\n{}", changed.len(), before.len(), changed.iter().take(3).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
#[ignore = "timing"]
fn cost() {
    let s = india_world(58, 300);
    let w = &s.world;
    let stories: Vec<_> = w.media.stories.iter().collect();
    let t0 = std::time::Instant::now();
    let mut n = 0;
    for st in &stories {
        if lang::story(w, st).is_some() {
            n += 1;
        }
    }
    let per = t0.elapsed().as_secs_f64() * 1e6 / stories.len() as f64;
    eprintln!("COST {} stories, {n} written, {per:.0} us per story", stories.len());
}

#[test]
fn a_published_injury_story_never_carries_the_diagnosis_or_the_days() {
    let s = india_world(59, 400);
    let w = &s.world;
    let mut checked = 0;
    for (_, st) in w.media.stories.iter_enumerated() {
        if st.kind != StoryKind::Injury {
            continue;
        }
        let Some(t) = lang::story(w, st) else { continue };
        let text = format!("{} {}", t.headline, t.body).to_lowercase();
        for inj in &w.data.injuries {
            assert!(!text.contains(&inj.name.to_lowercase()), "the diagnosis `{}` is in a public story: {text}", inj.name);
        }
        assert!(!text.contains("week") && !text.contains("recovery") && !text.contains("ruled out"), "the time out is in a public story: {text}");
        checked += 1;
    }
    assert!(checked > 20, "{checked}");
}

/// How varied the writing is: distinct frames used per kind of story, and how many stories share the most used headline frame.
#[test]
#[ignore = "report"]
fn variety() {
    let s = india_world(60, 500);
    let w = &s.world;
    let mut by: std::collections::BTreeMap<String, std::collections::BTreeMap<String, usize>> = Default::default();
    for (_, st) in w.media.stories.iter_enumerated() {
        if let Some(t) = lang::story(w, st) {
            if let Some(h) = t.frames.first() {
                *by.entry(format!("{:?}", st.kind)).or_default().entry(h.clone()).or_default() += 1;
            }
        }
    }
    for (k, m) in by {
        let total: usize = m.values().sum();
        let top = m.values().max().copied().unwrap_or(0);
        eprintln!("VARIETY {k}: {total} stories, {} headline frames used, most common {:.0}%", m.len(), 100.0 * top as f32 / total as f32);
    }
}

#[test]
fn features_analysis_incidents_and_fan_reactions_are_written_by_the_engine() {
    let s = india_world(54, 500);
    let w = &s.world;
    let mut written = std::collections::BTreeMap::<&str, usize>::new();
    for (_, st) in w.media.stories.iter_enumerated() {
        if let Some(t) = lang::story(w, st) {
            for f in &t.frames {
                for kind in ["player.reading", "match.analysis", "incident.reported", "fans.reaction", "player.criticism", "manager.pressure", "player.discipline"] {
                    if f.starts_with(kind) {
                        *written.entry(kind).or_default() += 1;
                    }
                }
            }
        }
    }
    // The common ones must appear in a year and a half of an India world; the rarer ones are covered by the cleanliness test when they do.
    for kind in ["player.reading", "match.analysis", "incident.reported", "fans.reaction"] {
        assert!(written.get(kind).copied().unwrap_or(0) > 0, "{kind} never written: {written:?}");
    }
}

#[test]
fn money_takes_the_worlds_form_rupees_in_india_and_a_symbol_elsewhere() {
    let mut s = Sim::new(pw_import::synthetic::build(DataPack::builtin(), 61, pw_import::synthetic::Scale::TINY));
    s.run(400);
    let w = &s.world;
    let mut with_money = 0;
    for (_, st) in w.media.stories.iter_enumerated() {
        if let Some(t) = lang::story(w, st) {
            let text = format!("{} {}", t.headline, t.body);
            assert!(!text.contains('₹') && !text.contains("crore") && !text.contains("lakh") && !text.contains("Rs "), "rupees outside India: {text}");
            if text.contains('£') {
                with_money += 1;
            }
        }
    }
    assert!(with_money > 0, "no engine story in a synthetic world mentions money");
    let india = india_world(62, 300);
    let w = &india.world;
    let rupees = w.media.stories.iter_enumerated().filter_map(|(_, st)| lang::story(w, st)).filter(|t| t.body.contains('₹') || t.body.contains("Rs ") || t.headline.contains('₹')).count();
    let pounds = w.media.stories.iter_enumerated().filter_map(|(_, st)| lang::story(w, st)).filter(|t| t.body.contains('£') || t.headline.contains('£')).count();
    assert!(rupees > 0 && pounds == 0, "India: {rupees} stories in rupees, {pounds} in pounds");
}
