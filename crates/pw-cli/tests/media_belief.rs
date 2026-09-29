//! Media credibility and belief are contextual (locked design §2.7, §2.9-2.10): trust in a journalist depends on their record on
//! that club and kind of story, and passing a story on is a different choice from believing it.

use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::Sim;
use pw_sim::socialnet::{believes, pass_on};
use pw_world::media::{ClaimType, topic_of};

/// A world with stories and accounts, and one story to experiment with.
fn world() -> (pw_world::World, pw_core::StoryId) {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 41, Scale::SMALL));
    sim.run(200);
    let w = sim.world;
    let story = w
        .media
        .stories
        .iter()
        .find(|s| matches!(s.claim_type, ClaimType::Report | ClaimType::Rumour) && s.club.is_some() && w.media.journalist_profiles.contains_key(&s.journalist))
        .map(|s| s.id)
        .expect("a transfer or contract story exists after 200 days");
    assert!(!w.net.accounts.is_empty());
    (w, story)
}

#[test]
fn trust_in_a_journalist_depends_on_their_record_on_that_club_and_topic() {
    let (mut w, story) = world();
    let (journalist, club, topic) = {
        let s = &w.media.stories[story];
        (s.journalist, s.club, topic_of(s.kind))
    };
    let a = 0u32;
    let unknown = believes(&w, a, story);
    let record = |w: &mut pw_world::World, hits: u8, misses: u8, club, topic| {
        let p = w.media.journalist_profiles.get_mut(&journalist).unwrap();
        p.ledger.retain(|r| !(r.club == club && r.topic == topic));
        for _ in 0..hits {
            p.record(club, topic, true, true);
        }
        for _ in 0..misses {
            p.record(club, topic, true, false);
        }
    };
    record(&mut w, 20, 0, club, topic);
    let reliable_here = believes(&w, a, story);
    record(&mut w, 0, 20, club, topic);
    let unreliable_here = believes(&w, a, story);
    assert!(reliable_here > unreliable_here, "a journalist who has been right on this club and topic is believed more ({reliable_here:.2} vs {unreliable_here:.2})");
    assert!(reliable_here >= unknown && unknown >= unreliable_here || (reliable_here - unreliable_here) > 0.05, "{unreliable_here:.2} {unknown:.2} {reliable_here:.2}");
    // A great record somewhere else changes nothing about this story: credibility is not one global number.
    record(&mut w, 0, 20, club, topic);
    let before = believes(&w, a, story);
    let other_club = w.clubs.ids().find(|&c| c != club).unwrap();
    record(&mut w, 30, 0, other_club, topic);
    record(&mut w, 30, 0, club, (topic + 1) % 6);
    assert_eq!(believes(&w, a, story), before, "a record on another club or another kind of story does not carry over");
}

#[test]
fn passing_a_story_on_is_not_the_same_as_believing_it() {
    let (mut w, story) = world();
    let club = w.media.stories[story].club;
    w.media.stories[story].tone = -50;
    w.media.stories[story].news = 90;
    let (joker, sober) = (1u32.min(w.net.accounts.len() as u32 - 1), 0u32);
    assert_ne!(joker, sober);
    // Two supporters of the club's rival who are equally sceptical, one a hostile joker and one sober.
    for (id, hostility, humour) in [(joker, 100, 100), (sober, 0, 0)] {
        let acc = &mut w.net.accounts[id as usize];
        acc.rival = club;
        acc.club = pw_core::ClubId::NONE;
        acc.persona.hostility = hostility;
        acc.persona.humour = humour;
        acc.persona.credulity = 0;
        acc.persona.knowledge = 100;
    }
    let (joker_belief, sober_belief) = (believes(&w, joker, story), believes(&w, sober, story));
    assert!((joker_belief - sober_belief).abs() < 0.05, "same scepticism, same wish, same belief: {joker_belief:.2} vs {sober_belief:.2}");
    let (joker_share, sober_share) = (pass_on(&w, joker, story), pass_on(&w, sober, story));
    assert!(joker_share > sober_share + 0.3, "equal belief, very different sharing ({joker_share:.2} vs {sober_share:.2}): temperament and spite decide it");
    // Doubt does not stop the joker: a low belief and a high chance of passing it on.
    w.net.accounts[joker as usize].persona.credulity = 0;
    assert!(joker_belief < 0.9 && joker_share > 0.4, "he passes on what he does not fully believe: belief {joker_belief:.2}, share {joker_share:.2}");
    // And belief alone does not compel sharing: a fully credulous account with no spite or humour stays modest.
    let acc = &mut w.net.accounts[sober as usize];
    acc.persona.credulity = 100;
    acc.rival = pw_core::ClubId::NONE;
    let believer = pass_on(&w, sober, story);
    assert!(believer < 0.5, "a believer with nothing to gain is not compelled to share: {believer:.2}");
}
