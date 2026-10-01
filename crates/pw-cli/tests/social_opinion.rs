//! Social opinion (locked design 6.1-6.13): several dimensions instead of one number, events that move different ones, expectation,
//! audiences who see the same person differently, memories that return, and opinions that change what people believe and say.

use pw_core::{ClubId, PersonId, PlayerId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::socialnet::{self, Audience};
use pw_sim::Sim;
use pw_world::socialnet::{AccountId, AccountKind, Concept, Dim, Frame, MomentKind, N_DIMS, Remembered};
use pw_world::World;

fn world() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 41, Scale::SMALL));
    sim.run(200);
    sim
}

/// A supporter account of `club` of the given kind, else any.
fn supporter(w: &World, club: ClubId, kind: AccountKind) -> AccountId {
    w.net.accounts.iter().find(|a| a.club == club && a.kind == kind && a.active).or_else(|| w.net.accounts.iter().find(|a| a.club == club && a.active && a.kind != AccountKind::Person)).map(|a| a.id).expect("a supporter")
}

fn a_player_of(w: &World, club: ClubId) -> (PlayerId, PersonId) {
    let p = w.teams[w.clubs[club].first_team()].squad[7];
    (p, w.players.cold[p].person)
}

fn frame_hat_trick(w: &World, p: PlayerId) -> Frame {
    let _ = w;
    Frame::HatTrick { uid: 0, player: p }
}

#[test]
fn different_events_move_different_dimensions() {
    let sim = world();
    let w = &sim.world;
    let club = w.clubs.ids().next().unwrap();
    let (p, _) = a_player_of(w, club);

    let hat = socialnet::impact_for(w, frame_hat_trick(w, p), true, 1, 0.8, false);
    assert!(hat[Dim::Form.idx()] > hat[Dim::Trust.idx()].abs() + 15, "a hat-trick is about form: {hat:?}");
    assert!(hat[Dim::Football.idx()] > 10 && hat[Dim::Trust.idx()] == 0 && hat[Dim::Affection.idx()] < hat[Dim::Form.idx()], "and barely about trust or affection");

    let req = socialnet::impact_for(w, Frame::TransferRequest { player: p }, true, -1, 0.8, false);
    assert!(req[Dim::Trust.idx()] < -20 && req[Dim::Identification.idx()] < -20 && req[Dim::Resentment.idx()] > 10, "a transfer request is about trust and belonging: {req:?}");
    assert_eq!(req[Dim::Football.idx()], 0, "it says nothing about how good he is");

    // The same red card, read through allegiance: our man's is a lapse in form and discipline, their rival's is "dirty as always".
    let ours = socialnet::impact_for(w, Frame::RedCard { uid: 0, player: p }, true, -1, 0.5, false);
    let theirs = socialnet::impact_for(w, Frame::RedCard { uid: 0, player: p }, false, 1, 0.5, false);
    assert!(ours[Dim::Form.idx()] < 0 && theirs[Dim::Form.idx()] == 0);
    assert!(theirs[Dim::Resentment.idx()] > 0 && theirs[Dim::Trust.idx()] < 0);

    // An opposing player's late winner: respect for the player and a grievance against him.
    let against = socialnet::impact_for(w, Frame::LateWinner { uid: 0, player: p }, true, -1, 0.8, true);
    assert!(against[Dim::Football.idx()] > 0 && against[Dim::Resentment.idx()] > 20 && against[Dim::Affection.idx()] < 0, "{against:?}");
    // A derby makes the grievance bigger.
    let plain = socialnet::impact_for(w, Frame::LateWinner { uid: 0, player: p }, true, -1, 0.8, false);
    assert!(against[Dim::Resentment.idx()] > plain[Dim::Resentment.idx()]);
}

#[test]
fn a_person_can_be_excellent_disliked_and_terrible_value_at_once_and_the_summary_depends_on_who_is_asking() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().next().unwrap();
    let (_, person) = a_player_of(w, club);
    let stats = supporter(w, club, AccountKind::Stats);
    let ultra = supporter(w, club, AccountKind::Ultra);
    let casual = supporter(w, club, AccountKind::Casual);
    for a in [stats, ultra, casual] {
        w.net.opinions.entry(a).or_default().retain(|o| o.about != person);
    }
    // Brilliant, not one of ours, overpaid: pushed in by hand through the same door events use.
    let mut d = [0i16; N_DIMS];
    d[Dim::Football.idx()] = 800;
    d[Dim::Affection.idx()] = -300;
    d[Dim::Identification.idx()] = -600;
    d[Dim::Value.idx()] = -700;
    for a in [stats, ultra, casual] {
        w.net.accounts[a as usize].persona.stubbornness = 0;
        socialnet::apply(w, a, person, d);
    }
    let read = |w: &World, a: AccountId| w.net.opinion(a, person).unwrap().clone();
    let (s, u, c) = (read(w, stats), read(w, ultra), read(w, casual));
    // The dimensions are the same shape for everyone (expectation aside); what they add up to is not.
    assert!(s.dims[Dim::Football.idx()] > 400 && s.dims[Dim::Affection.idx()] < 0 && s.dims[Dim::Value.idx()] < -300, "contradictory readings live together: {:?}", s.dims);
    let scores = [s.score, u.score, c.score];
    assert!(scores.iter().max().unwrap() - scores.iter().min().unwrap() >= 40, "each audience sums it differently: {scores:?}");
    // Summaries follow what each kind cares about: the stats account is warmer than the ultra to a great player who is not "one of us".
    let (mut stats_acc, mut ultra_acc) = (w.net.accounts[stats as usize].clone(), w.net.accounts[ultra as usize].clone());
    stats_acc.persona.stats = 95;
    ultra_acc.persona.tribalism = 95;
    ultra_acc.persona.local = 90;
    assert!(socialnet::summarise(&stats_acc, &d) > socialnet::summarise(&ultra_acc, &d), "numbers people rate the player, ultras do not take to an outsider");
}

#[test]
fn what_was_expected_decides_whether_a_good_game_impresses() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().next().unwrap();
    let squad = w.teams[w.clubs[club].first_team()].squad.clone();
    let (star, rookie) = (squad[2], squad[9]);
    // A record signing on the best wage with a promise of a place, and a raw academy youngster.
    let team_max = squad.iter().map(|&q| w.players.cold[q].contract.current_wage(w.date)).max().unwrap();
    w.players.cold[star].contract.wage = team_max * 3;
    w.players.cold[star].contract.promised_status = Some(pw_world::SquadStatus::Star);
    w.players.cold[star].rep.world = 8000;
    let who = w.players.cold[rookie].person;
    w.people[who].dob = w.date.add_days(-19 * 365);
    w.players.cold[rookie].youth_club = club;
    w.players.cold[rookie].contract.wage = 1;
    w.players.cold[rookie].rep.world = 200;
    w.media.image.clear();
    assert!(socialnet::expectation(w, star) > socialnet::expectation(w, rookie) + 0.3, "{} vs {}", socialnet::expectation(w, star), socialnet::expectation(w, rookie));

    let a = supporter(w, club, AccountKind::Supporter);
    w.net.accounts[a as usize].persona.stubbornness = 0;
    let good_game = |w: &mut World, who: PersonId| {
        w.net.opinions.entry(a).or_default().retain(|o| o.about != who);
        let mut d = [0i16; N_DIMS];
        d[Dim::Football.idx()] = 100;
        d[Dim::Form.idx()] = 100;
        socialnet::apply(w, a, who, d);
        w.net.opinion(a, who).unwrap().dims
    };
    let star_person = w.players.cold[star].person;
    let (s, r) = (good_game(w, star_person), good_game(w, who));
    assert!(r[Dim::Form.idx()] > s[Dim::Form.idx()] + 30, "the same game excites more from the raw youngster than from the star: {} vs {}", r[Dim::Form.idx()], s[Dim::Form.idx()]);
    // And a poor one disappoints more from the star.
    let bad = |w: &mut World, who: PersonId| {
        w.net.opinions.entry(a).or_default().retain(|o| o.about != who);
        let mut d = [0i16; N_DIMS];
        d[Dim::Form.idx()] = -100;
        socialnet::apply(w, a, who, d);
        w.net.opinion(a, who).unwrap().dims[Dim::Form.idx()]
    };
    assert!(bad(w, star_person) < bad(w, who) - 30);
}

#[test]
fn audiences_see_the_same_person_differently_and_rivals_hold_opinions_too() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().next().unwrap();
    let rival = w.net.accounts.iter().find(|a| a.club == club && a.rival.is_some()).map(|a| a.rival).unwrap_or(w.clubs.ids().nth(1).unwrap());
    let (_, person) = a_player_of(w, club);
    for a in w.net.accounts.iter().filter(|a| a.club == club || a.rival == club).map(|a| a.id).collect::<Vec<_>>() {
        w.net.opinions.entry(a).or_default().retain(|o| o.about != person);
        w.net.accounts[a as usize].persona.stubbornness = 0;
    }
    let own: Vec<AccountId> = w.net.accounts.iter().filter(|a| a.club == club && a.active && a.kind != AccountKind::Person).map(|a| a.id).collect();
    let others: Vec<AccountId> = w.net.accounts.iter().filter(|a| a.rival == club && a.active && a.kind != AccountKind::Person).map(|a| a.id).collect();
    assert!(!own.is_empty());
    // Our man sent off: ours see a lapse; the club's rivals see a dirty player.
    let p = w.people[person].player;
    for &a in &own {
        socialnet::apply(w, a, person, socialnet::impact_for(w, Frame::RedCard { uid: 0, player: p }, true, -1, 0.6, false));
    }
    for &a in &others {
        socialnet::apply(w, a, person, socialnet::impact_for(w, Frame::RedCard { uid: 0, player: p }, false, 1, 0.6, false));
    }
    let (mine, _, n_own) = socialnet::audience_view(w, person, Audience::OwnSupporters(club)).expect("our own hold a view");
    assert!(n_own > 0);
    if let Some((theirs, _, n_rival)) = socialnet::audience_view(w, person, Audience::RivalSupporters(club)) {
        assert!(n_rival > 0);
        assert!(theirs[Dim::Resentment.idx()] > 0 && mine[Dim::Form.idx()] < 0 && theirs[Dim::Form.idx()] == 0, "own: {mine:?} rival: {theirs:?}");
    }
    assert!(socialnet::audience_view(w, person, Audience::Neutral).is_none(), "nobody neutral has weighed in");
    let _ = rival;
}

#[test]
fn opinion_changes_what_people_believe_and_say() {
    let mut sim = world();
    let w = &mut sim.world;
    // A story that says something bad about a person.
    let id = w.media.stories.iter().find(|s| s.person.is_some() && s.outlet.is_some() && s.club.is_some()).map(|s| s.id).expect("a story");
    w.media.stories[id].tone = -50;
    let (about, club) = (w.media.stories[id].person, w.media.stories[id].club);
    let a = supporter(w, club, AccountKind::Supporter);
    let set = |w: &mut World, trust: i16| {
        w.net.opinions.entry(a).or_default().retain(|o| o.about != about);
        w.net.accounts[a as usize].persona.stubbornness = 0;
        let mut d = [0i16; N_DIMS];
        d[Dim::Trust.idx()] = trust;
        socialnet::apply(w, a, about, d);
    };
    set(w, -700);
    let distrustful = socialnet::believes(w, a, id);
    set(w, 700);
    let trusting = socialnet::believes(w, a, id);
    assert!(distrustful > trusting + 0.1, "someone who does not trust him believes the worst more readily: {distrustful} vs {trusting}");
    // A good story is the other way round.
    w.media.stories[id].tone = 50;
    set(w, -700);
    let sour = socialnet::believes(w, a, id);
    set(w, 700);
    let fond = socialnet::believes(w, a, id);
    assert!(fond > sour);

    // Those who trust him or see him as one of their own defend him when something goes wrong.
    let p = w.people[about].player;
    let f = Frame::RedCard { uid: 0, player: p };
    set(w, 800);
    let mut d = [0i16; N_DIMS];
    d[Dim::Identification.idx()] = 400;
    socialnet::apply(w, a, about, d);
    let (c, ..) = socialnet::concept(w, a, f, about, -1, true, false, 0.9).expect("they say something");
    assert_eq!(c, Concept::Defend);
}

#[test]
fn an_old_episode_comes_back_when_a_new_one_echoes_it() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().next().unwrap();
    let (p, person) = a_player_of(w, club);
    let a = supporter(w, club, AccountKind::Supporter);
    w.net.opinions.entry(a).or_default().retain(|o| o.about != person);
    let f = Frame::TransferRequest { player: p };
    // Nothing remembered: they simply worry or complain.
    w.net.memories.entry(a).or_default().clear();
    let (plain, ..) = socialnet::concept(w, a, f, person, -1, true, false, 0.3).unwrap();
    assert_ne!(plain, Concept::Recall);
    // The same thing happened a year ago: "here we go again".
    let old = w.date.add_days(-300);
    w.net.memories.entry(a).or_default().push(Remembered { kind: MomentKind::TransferRequest, about: person, date: old, event: pw_core::EventId::NONE });
    let (recalled, ..) = socialnet::concept(w, a, f, person, -1, true, false, 0.3).unwrap();
    assert_eq!(recalled, Concept::Recall);
    // A memory from last week is not history yet.
    w.net.memories.get_mut(&a).unwrap().clear();
    w.net.memories.entry(a).or_default().push(Remembered { kind: MomentKind::TransferRequest, about: person, date: w.date.add_days(-5), event: pw_core::EventId::NONE });
    let (fresh, ..) = socialnet::concept(w, a, f, person, -1, true, false, 0.3).unwrap();
    assert_ne!(fresh, Concept::Recall);
}

#[test]
fn a_running_world_forms_multidimensional_views_and_keeps_them_in_range() {
    let sim = world();
    let w = &sim.world;
    let mut held = 0;
    let mut mixed = 0;
    for (a, ops) in &w.net.opinions {
        let acc = &w.net.accounts[*a as usize];
        for o in ops {
            held += 1;
            assert!(o.dims.iter().all(|d| (-1000..=1000).contains(d)));
            assert_eq!(o.score, socialnet::summarise(acc, &o.dims), "the score is only ever the summary of the dimensions");
            if o.dims.iter().any(|&d| d > 60) && o.dims.iter().any(|&d| d < -60) {
                mixed += 1;
            }
            assert!(o.low <= o.score && o.score <= o.high);
        }
    }
    assert!(held > 100, "supporters have formed views: {held}");
    assert!(mixed > 0, "some people think well of him in one way and badly in another");
}
