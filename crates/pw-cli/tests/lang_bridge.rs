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
    // An effect no decision can carry out is never offered: nobody here bids or keeps a shortlist, and there is no reply that only asks for a fuller medical
    // report (see `docs/LANGUAGE.md`), so these would promise what the world does not do.
    for effect in ["transfer.open_bid", "transfer.drop_target", "medical.request_report", "inbox.dismiss"] {
        assert!(lang::effect_choice(effect).is_none(), "{effect} must stay unoffered until something does what it says");
    }
    // The injury decision is worded by the older text, so its options are the simulation's own (surgery, rehabilitation).
    let treatment = with_choices(decision_for(w, pro, DecisionKind::Treatment { surgery_days: 40, rehab_days: 80 }));
    assert!(lang::decision_options(w, &treatment).is_empty());
}

#[test]
fn a_post_that_relays_a_story_is_no_firmer_than_the_story_or_the_post() {
    let w = &shared().world;
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
    // Not vacuous: most relays are the engine's (the rest are of stories it has no event for).
    assert!(engine_posts * 10 >= relays * 8, "{engine_posts} of {relays} relays are the engine's");
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

#[test]
fn supporters_celebrate_in_their_own_language_and_reports_name_the_derby() {
    let s = india_world(58, 500);
    let w = &s.world;
    // The word for a win in the language of the club's state, in its own script.
    let bagan = w.clubs.ids().find(|&c| w.clubs[c].name == "Mohun Bagan Super Giant").expect("Mohun Bagan is in the tiny world");
    assert_eq!(pw_narrate::social::local_word(w, bagan, "match.win").as_deref(), Some("জয়"));
    // Some supporters' celebrations open with such a word; nobody else's posts do.
    let mut local = 0;
    for p in &w.net.posts {
        let text = pw_narrate::social::post(w, p);
        if text.chars().any(|c| ('\u{0980}'..='\u{0D7F}').contains(&c) || ('\u{0900}'..='\u{097F}').contains(&c)) {
            assert_eq!(p.concept, pw_world::socialnet::Concept::Celebrate, "only a celebration uses the local word: {text}");
            local += 1;
        }
    }
    assert!(local > 0, "no supporter celebrated in their own language in {} posts", w.net.posts.len());
    // A report of a derby the reference names says so, at least sometimes; no other report does.
    let derbies = &w.ext.scenario.known_derbies;
    let (mut named, mut derby_reports) = (0, 0);
    for (_, st) in w.media.stories.iter_enumerated() {
        let Some(pw_world::media::StoryLink::Fixture { home, away, .. }) = w.media.links.get(&st.id) else { continue };
        let Some(t) = lang::story(w, st) else { continue };
        let text = format!("{} {}", t.headline, t.body);
        match derbies.iter().find(|d| (d.a == *home && d.b == *away) || (d.a == *away && d.b == *home)) {
            Some(d) => {
                derby_reports += 1;
                if text.contains(d.name.trim_start_matches("The ").trim_start_matches("the ")) {
                    named += 1;
                }
            }
            None => assert!(!derbies.iter().any(|d| text.contains(&d.name)), "a derby name on another match: {text}"),
        }
    }
    assert!(derby_reports > 0 && named > 0, "{derby_reports} derby reports, {named} named the derby");
}

#[test]
#[ignore = "debug"]
fn post_mix() {
    let s = india_world(56, 500);
    let w = &s.world;
    let mut by = std::collections::BTreeMap::<String, usize>::new();
    let mut shown = std::collections::BTreeMap::<String, usize>::new();
    for p in w.net.posts.iter() {
        let f = format!("{:?}", p.frame);
        let f = f.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string();
        let t = lang::post(w, p);
        *by.entry(format!("{:?} / {f} / engine={}", p.concept, t.is_some())).or_default() += 1;
        if let Some(t) = t {
            let n = shown.entry(format!("{:?}/{f}", p.concept)).or_default();
            if *n < 3 {
                *n += 1;
                eprintln!("SAMPLE {:?}/{f}: {t}", p.concept);
            }
        }
    }
    for (k, n) in by {
        eprintln!("MIX {n:5} {k}");
    }
    eprintln!("posts {}", w.net.posts.len());
}

// ---------------------------------------------------------------------------------------------------- social posts: opinion, banter, answers

use pw_world::media::ClaimType;
use pw_world::socialnet::{Concept, Frame, Knew, NO_POST, Post};

/// One India world for every test of the posts (seed 56, 500 days): building it is most of what they cost.
fn shared() -> &'static Sim {
    static W: std::sync::OnceLock<Sim> = std::sync::OnceLock::new();
    W.get_or_init(|| india_world(56, 500))
}

/// A post the world did not make, about something real in it: for the checks that should not depend on what the seed happened to produce.
fn made_up(w: &pw_world::World, id: u32, author: u32, frame: Frame, concept: Concept, about: pw_core::PersonId, club: pw_core::ClubId) -> Post {
    Post {
        id,
        author,
        date: w.date,
        minute: 600,
        frame,
        concept,
        about,
        about2: pw_core::PersonId::NONE,
        club,
        extra: 0,
        intensity: 50,
        claim: ClaimType::Opinion,
        reply_to: NO_POST,
        quote_of: NO_POST,
        refs: Default::default(),
        likes: 0,
        reposts: 0,
        replies: 0,
        depth: 0,
        knew: Knew::Watched,
        prior: 0,
    }
}

/// The concepts whose wording the engine owns (the rest keep the older, personality-driven text: see `the_posts_the_engine_does_not_write_keep_their_text`).
fn engine_concept(c: Concept) -> bool {
    !matches!(c, Concept::Relay | Concept::CompareLegend | Concept::CallOut | Concept::Recall | Concept::Chant | Concept::Meme | Concept::Statement | Concept::Looks | Concept::Folklore)
}

#[test]
fn opinion_banter_and_answers_are_written_by_the_engine_clean_and_the_same_each_time() {
    let w = &shared().world;
    let (mut by_engine, mut opinions, mut agree, mut agree_engine) = (0usize, 0usize, 0usize, 0usize);
    let mut seen: std::collections::BTreeMap<String, usize> = Default::default();
    let mut texts = std::collections::BTreeSet::new();
    for p in &w.net.posts {
        if !engine_concept(p.concept) {
            continue;
        }
        opinions += 1;
        if matches!(p.concept, Concept::Agree | Concept::Disagree) {
            agree += 1;
        }
        let Some(text) = lang::post(w, p) else { continue };
        by_engine += 1;
        if matches!(p.concept, Concept::Agree | Concept::Disagree) {
            agree_engine += 1;
        }
        *seen.entry(format!("{:?}", p.concept)).or_default() += 1;
        let issues = pw_lang::check::check_text(&text);
        assert!(issues.is_empty(), "{:?} on {:?}: {issues:?}: {text}", p.concept, p.frame);
        assert_eq!(text, lang::post(w, p).unwrap(), "the same post reads the same way each time");
        assert_eq!(text, pw_narrate::social::post(w, p), "the page shows the engine's words, not older ones");
        texts.insert(text);
    }
    eprintln!("opinion posts {opinions}, by the engine {by_engine}: {seen:?}");
    // Answers to other posts carry no facts: all of them are the engine's, and so is most of what is said about real events.
    assert_eq!(agree, agree_engine, "every answer is written by the engine");
    assert!(by_engine * 10 >= opinions * 7, "{by_engine} of {opinions} opinion posts are the engine's");
    for c in ["Celebrate", "Lament", "Criticise", "Question", "Mock", "Sarcasm", "Worry"] {
        assert!(seen.get(c).is_some_and(|n| *n > 0), "no {c} post written by the engine in {seen:?}");
    }
    assert!(texts.len() > 100, "{} distinct texts: the words should vary", texts.len());
}

#[test]
fn a_post_about_a_rumour_or_a_claim_is_no_firmer_than_the_story() {
    use pw_world::media::StoryKind;
    let w = &shared().world;
    let hedges = [
        "rumour", "speculation", "sources", "claim", "insist", "word is", "whispers", "talk", "according", "per ", "perhaps", "may be", "one reading", "understood", "reports", "reported", "hear", "reading",
    ];
    let firm = ["confirmed", "officially", "announced", "it is official", "completed", "agreed", "done deal"];
    let mut weak = 0;
    for p in &w.net.posts {
        let Frame::Story { story } = p.frame else { continue };
        let st = &w.media.stories[story];
        let Some(text) = lang::post(w, p) else { continue };
        let lower = text.to_lowercase();
        // Only the stories that rest on someone's word: a leak, a rumour, a guess about a manager's future.
        let rests_on_word = matches!(st.kind, StoryKind::TransferRumour | StoryKind::Leak | StoryKind::ManagerPressure) || st.leaker.is_some() || p.concept == Concept::Relay;
        if !rests_on_word {
            continue;
        }
        weak += 1;
        assert!(firm.iter().all(|f| !lower.contains(f)), "{:?} post about a {:?} story reads firmly: {text}", p.concept, st.kind);
        assert!(hedges.iter().any(|h| lower.contains(h)), "{:?} post about a {:?} story carries no attribution: {text}", p.concept, st.kind);
        if st.kind == StoryKind::TransferRumour && st.claim < 75 {
            assert!(!lower.contains(" bid") && !lower.contains("signs"), "a claim of {} was posted as a deal: {text}", st.claim);
        }
    }
    assert!(weak > 10, "only {weak} posts rest on someone's word");
}

#[test]
fn public_posts_carry_what_the_public_holds_and_nothing_private() {
    let w = &shared().world;
    let authors: Vec<u32> = w.net.accounts.iter().filter(|a| a.kind != pw_world::socialnet::AccountKind::Person && a.club.is_some()).map(|a| a.id).take(40).collect();
    assert!(authors.len() > 10);
    let player = w.players.hot.iter_enumerated().find(|(_, h)| h.club.is_some() && h.status == pw_world::PlayerStatus::Active).map(|x| x.0).unwrap();
    let person = w.players.cold[player].person;
    let club = w.players.hot[player].club;
    let other = w.clubs.ids().find(|&c| c != club).unwrap();
    let concepts = [Concept::Praise, Concept::Criticise, Concept::Lament, Concept::Worry, Concept::Question, Concept::Mock, Concept::Sarcasm, Concept::Celebrate];
    let mut id = 1_000_000;
    let mut written = 0;
    let digits_or_money = |t: &str| -> Option<String> {
        if t.chars().any(|c| c.is_ascii_digit() || c == '₹' || c == '£') {
            return Some("a number or an amount".into());
        }
        ["crore", "lakh", "rs "].iter().find(|m| t.contains(**m)).map(|m| (*m).to_string())
    };
    // Whole words: "what a day" is a cheer, "out for two days" is a diagnosis.
    let words = |t: &str, list: &[&str]| -> Option<String> {
        let ws: Vec<&str> = t.split(|c: char| !c.is_alphanumeric()).collect();
        list.iter().find(|m| ws.contains(m)).map(|m| (*m).to_string())
    };
    // An injury, a signing, a transfer request: said, never how long, what with, for how much, on what wage or why.
    let cases: Vec<(Frame, &str, Vec<&str>)> = vec![
        (Frame::Injury { player }, "injury", vec!["week", "weeks", "month", "months", "days", "ligament", "hamstring", "fracture", "acl", "surgery", "diagnosis"]),
        (Frame::Signing { player, club }, "signing", vec!["wage", "salary", "contract", "years", "clause", "agent"]),
        (Frame::Departure { player, from: club, to: other }, "departure", vec!["wage", "salary", "contract", "clause", "agent"]),
        (Frame::TransferRequest { player }, "request", vec!["wage", "salary", "contract", "clause", "agent", "because"]),
    ];
    for (frame, what, banned) in cases {
        for &a in &authors {
            for c in concepts {
                id += 1;
                let p = made_up(w, id, a, frame, c, person, club);
                let Some(text) = lang::post(w, &p) else { continue };
                written += 1;
                assert!(pw_lang::check::check_text(&text).is_empty(), "{what}: {text}");
                let lower = text.to_lowercase();
                if let Some(bad) = digits_or_money(&lower).or_else(|| words(&lower, &banned)) {
                    panic!("{what}: {bad} in {text:?}");
                }
            }
        }
    }
    assert!(written > 100, "only {written} made-up posts were written");
    // Incidents: the first two people and the club, and only the kinds the press may report; private matters keep to the older text, which has its own rules.
    let personal = ["FamilyEmergency", "RelationshipConflict", "Pregnancy", "MovingProblem", "ExamClash", "ChildcareClash", "UnexpectedBill", "PaperworkProblem", "EconomicDownturn", "TransportDisruption", "SevereWeather", "FederationDispute"];
    let (mut public, mut private) = (0, 0);
    for i in w.incidents.list.iter().take(300) {
        let kind = format!("{:?}", i.kind);
        let about = i.parties.first().copied().unwrap_or(pw_core::PersonId::NONE);
        let p = made_up(w, 2_000_000 + i.id, authors[0], Frame::Incident { incident: i.id }, Concept::Lament, about, i.club);
        match lang::post(w, &p) {
            Some(text) => {
                public += 1;
                assert!(!personal.contains(&kind.as_str()), "a private matter ({kind}) was posted: {text}");
                for &third in i.parties.iter().skip(2) {
                    let name = w.people[third].display_name(&w.names).into_owned();
                    assert!(!text.contains(&name), "{name}, a third party, named in {text}");
                }
            }
            None if personal.contains(&kind.as_str()) => private += 1,
            None => {}
        }
    }
    eprintln!("incidents: {public} public posts written, {private} private ones left to the older text");
}

#[test]
fn the_posts_the_engine_does_not_write_keep_their_text() {
    let w = &shared().world;
    let (mut chants, mut memes) = (0, 0);
    for p in &w.net.posts {
        if matches!(p.concept, Concept::Chant | Concept::Meme | Concept::CallOut | Concept::Statement | Concept::Looks | Concept::Folklore | Concept::Recall | Concept::CompareLegend) {
            assert!(lang::post(w, p).is_none(), "{:?} is not the engine's to write", p.concept);
            let text = pw_narrate::social::post(w, p);
            assert!(!text.trim().is_empty() || p.concept == Concept::Statement, "{:?} lost its text", p.concept);
            match p.concept {
                Concept::Chant => chants += 1,
                Concept::Meme => memes += 1,
                _ => {}
            }
        }
        // A post about a manager's football uses the manager's philosophy, which only the older text has.
        if matches!(p.concept, Concept::Praise | Concept::Criticise) && p.about.is_some() && w.people[p.about].staff.get().is_some_and(|s| w.staff[s].role == pw_world::StaffRole::Manager) {
            assert!(lang::post(w, p).is_none());
        }
    }
    eprintln!("chants {chants}, memes {memes}");
}

#[test]
fn the_account_s_personality_moves_the_words_and_the_same_account_always_sounds_the_same() {
    use pw_world::socialnet::Age;
    let mut w = shared().world.clone();
    let posts: Vec<Post> = w.net.posts.iter().filter(|p| engine_concept(p.concept)).take(600).cloned().collect();
    let say = |w: &pw_world::World| -> Vec<Option<String>> { posts.iter().map(|p| lang::post(w, p)).collect() };
    let set = |w: &mut pw_world::World, joker: bool| {
        for a in w.net.accounts.iter_mut() {
            a.age = if joker { Age::Teen } else { Age::Older };
            a.intensity = if joker { 95 } else { 10 };
            let p = &mut a.persona;
            (p.humour, p.hostility, p.optimism, p.knowledge, p.stats, p.credulity) = if joker { (95, 90, 5, 20, 5, 90) } else { (5, 5, 95, 90, 90, 10) };
        }
    };
    set(&mut w, true);
    let loud = say(&w);
    assert_eq!(loud, say(&w), "the same account sounds the same");
    set(&mut w, false);
    let quiet = say(&w);
    let (mut both, mut differ) = (0, 0);
    for (a, b) in loud.iter().zip(&quiet) {
        if let (Some(a), Some(b)) = (a, b) {
            both += 1;
            differ += usize::from(a != b);
            assert!(pw_lang::check::check_text(a).is_empty() && pw_lang::check::check_text(b).is_empty(), "{a} / {b}");
        }
    }
    eprintln!("{differ} of {both} posts are worded differently by a teenage joker and a formal older account");
    assert!(both > 200 && differ * 3 > both, "personality barely moves the words: {differ} of {both}");
}

#[test]
fn a_reloaded_world_reads_every_post_the_same_way() {
    let w = &shared().world;
    let dir = std::env::temp_dir().join(format!("pw-lang-posts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("w.sav");
    pw_sim::save::save(w, &path).unwrap();
    let back: pw_world::World = pw_sim::save::load(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let mut n = 0;
    for (a, b) in w.net.posts.iter().zip(&back.net.posts) {
        assert_eq!(pw_narrate::social::post(w, a), pw_narrate::social::post(&back, b), "{:?} on {:?} reads differently after a reload", a.concept, a.frame);
        n += 1;
    }
    assert!(n > 500, "{n} posts compared");
}
