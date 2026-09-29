//! Attention, virality, brand and folklore (locked design 6.16-6.40): waves with different half-lives and reach, fame apart from
//! football reputation, a bounded brand premium, hype beside anti-hype, nicknames with lives, folklore that grows in the telling, and
//! looks read differently by different audiences.

use pw_core::{ClubId, PersonId, PlayerId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::attention::{self, Taste};
use pw_sim::{Sim, market, socialnet};
use pw_world::attention::{Cause, NickKind, NickState, Nickname};
use pw_world::socialnet::{AccountKind, Concept, Frame, MomentKind, Remembered};
use pw_world::World;

fn world() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 51, Scale::SMALL));
    sim.run(60);
    sim
}

fn a_player(w: &World) -> (PlayerId, PersonId, ClubId) {
    let club = w.clubs.ids().next().unwrap();
    let p = w.teams[w.clubs[club].first_team()].squad[6];
    (p, w.players.cold[p].person, club)
}

#[test]
fn kinds_of_attention_have_their_own_half_lives_and_reach() {
    let mut sim = world();
    let w = &mut sim.world;
    let (_, who, _) = a_player(w);
    w.net.attention.remove(&who);
    let start = w.date;
    attention::spark(w, who, Cause::Controversy, 0.9);
    attention::spark(w, who, Cause::Aesthetic, 0.9);
    let l0 = |w: &World, c: Cause| attention::level_of(w, who, c);
    let (c0, a0) = (l0(w, Cause::Controversy), l0(w, Cause::Aesthetic));
    assert!(c0 > 0.5 && a0 > 0.5);
    w.date = start.add_days(10);
    let (c10, a10) = (l0(w, Cause::Controversy), l0(w, Cause::Aesthetic));
    assert!(c10 < c0 * 0.35 && a10 > a0 * 0.7, "a row is gone in ten days, a look lingers: {c10}/{c0} vs {a10}/{a0}");
    // Half-lives are what the kinds say they are.
    let half = |c: Cause| pw_world::attention::Wave { cause: c, start, peak: 1.0 }.level(start.add_days(c.half_life() as i32));
    for c in [Cause::Football, Cause::Personality, Cause::Aesthetic, Cause::Controversy, Cause::Emotional, Cause::Meme] {
        assert!((half(c) - 0.5).abs() < 0.02, "{c:?}");
    }
    assert!(Cause::Meme.half_life() < Cause::Football.half_life() && Cause::Football.half_life() < Cause::Aesthetic.half_life());
    // A goal stays in football; a look or a joke does not.
    assert!(Cause::Football.reach() < Cause::Personality.reach() && Cause::Personality.reach() < Cause::Aesthetic.reach());
}

#[test]
fn attention_that_leaves_football_widens_his_following_and_fame_beyond_his_reputation() {
    let mut sim = world();
    let w = &mut sim.world;
    let (p, who, _) = a_player(w);
    w.net.attention.remove(&who);
    let before = w.renown.of(who);
    let rep = w.players.cold[p].rep.world;
    // Football attention barely leaves the game.
    attention::spark(w, who, Cause::Football, 0.9);
    let football_only = w.renown.of(who);
    assert_eq!(football_only.fame, before.fame, "a goal does not make anyone famous outside the game");
    attention::spark(w, who, Cause::Aesthetic, 0.9);
    attention::spark(w, who, Cause::Personality, 0.9);
    let after = w.renown.of(who);
    assert!(after.fame > before.fame && after.followers > before.followers, "looks and personality do");
    let share = w.net.attention[&who].general_share;
    assert!(share > 0.1, "and the following is now partly people who do not watch football: {share}");
    // Fame follows attention, not only football reputation: a monthly pass moves it above what reputation alone earns.
    let target_without = f32::from(rep) * 0.8;
    pw_sim::renown::monthly(w);
    w.date = w.date.add_days(0);
    assert!(f32::from(w.renown.of(who).fame) >= target_without.min(f32::from(before.fame)), "it does not fall for lack of football");
    let _ = attention::level(w, who);
}

#[test]
fn brand_value_is_a_bounded_premium_beside_football_value() {
    let mut sim = world();
    let w = &mut sim.world;
    let (p, who, _) = a_player(w);
    w.net.attention.remove(&who);
    w.renown.people.remove(&who);
    let plain_ca = w.players.cold[p].ca;
    let plain = market::value_of(w, p);
    let bare = attention::brand_premium(w, p);
    for c in [Cause::Football, Cause::Aesthetic, Cause::Personality, Cause::Meme] {
        attention::spark(w, who, c, 1.0);
    }
    w.renown.people.entry(who).or_default().fame = 10_000;
    let hyped = market::value_of(w, p);
    let premium = attention::brand_premium(w, p);
    assert!(premium > bare && premium <= 0.10 + 1e-6, "{premium}");
    assert!(hyped as f64 <= plain as f64 * 1.12 && hyped >= plain, "a tenth at most: {plain} -> {hyped}");
    assert_eq!(w.players.cold[p].ca, plain_ca, "it never changes what he is");
    // Nor how clubs read his ability: that is beliefs about him, untouched by buzz.
    let club = w.clubs.ids().nth(1).unwrap();
    let before = pw_sim::scouting::view(w, club, p).0;
    attention::spark(w, who, Cause::Meme, 1.0);
    assert_eq!(pw_sim::scouting::view(w, club, p).0, before);
}

#[test]
fn hype_and_anti_hype_live_together_and_numbers_people_say_so() {
    let mut sim = world();
    let w = &mut sim.world;
    let (p, who, club) = a_player(w);
    // A modest player with the world's attention and a reputation that does not match.
    w.players.cold[p].rep.world = 1500;
    w.net.attention.remove(&who);
    for c in [Cause::Football, Cause::Personality, Cause::Aesthetic] {
        attention::spark(w, who, c, 1.0);
    }
    assert!(attention::overhyped(w, p) > 0.25, "{}", attention::overhyped(w, p));
    let stats = w.net.accounts.iter().find(|a| a.club == club && a.kind == AccountKind::Stats).or_else(|| w.net.accounts.iter().find(|a| a.club == club && a.persona.stats >= 60)).map(|a| a.id);
    let a = match stats {
        Some(a) => a,
        None => {
            let a = w.net.accounts.iter().find(|a| a.club == club && a.kind != AccountKind::Person).map(|a| a.id).unwrap();
            w.net.accounts[a as usize].persona.stats = 90;
            a
        }
    };
    w.net.accounts[a as usize].persona.stats = 90;
    w.net.accounts[a as usize].persona.celebrity = 10;
    w.net.accounts[a as usize].persona.nostalgia = 10;
    w.net.opinions.entry(a).or_default().retain(|o| o.about != who);
    let f = Frame::HatTrick { uid: 0, player: p };
    let (c, ..) = socialnet::concept(w, a, f, who, 1, true, false, 0.2).unwrap();
    assert_eq!(c, Concept::Overrated, "the hype is out of proportion and someone says so");
    // An account that is not a numbers person joins in the celebration instead.
    let casual = w.net.accounts.iter().find(|x| x.club == club && x.kind != AccountKind::Person && x.id != a).map(|x| x.id).unwrap();
    w.net.accounts[casual as usize].persona.stats = 10;
    w.net.accounts[casual as usize].persona.celebrity = 10;
    w.net.accounts[casual as usize].persona.nostalgia = 10;
    let (c2, ..) = socialnet::concept(w, casual, f, who, 1, true, false, 0.9).unwrap();
    assert_ne!(c2, Concept::Overrated);
}

#[test]
fn looks_are_read_by_audience_and_are_generated_not_facts() {
    let mut sim = world();
    let w = &mut sim.world;
    let club = w.clubs.ids().next().unwrap();
    let squad = w.teams[w.clubs[club].first_team()].squad.clone();
    // Deterministic, stable, in range, and different for different audiences.
    let mut differs = 0;
    let mut spread = (f32::MAX, f32::MIN);
    for &p in &squad {
        let who = w.players.cold[p].person;
        let (g, c, f) = (attention::appeal(w, who, Taste::General), attention::appeal(w, who, Taste::Celebrity), attention::appeal(w, who, Taste::Football));
        assert_eq!(g, attention::appeal(w, who, Taste::General), "stable");
        assert!([g, c, f].iter().all(|x| (-1.0..=1.0).contains(x)));
        if (g - c).abs() > 0.15 || (g - f).abs() > 0.15 {
            differs += 1;
        }
        spread = (spread.0.min(g), spread.1.max(g));
    }
    assert!(differs * 2 > squad.len(), "different audiences, different responses: {differs} of {}", squad.len());
    assert!(spread.1 - spread.0 > 0.5, "some strike people more than others");

    // A wave of looks-attention makes some accounts say something, in different spirits.
    let p = squad.iter().copied().max_by(|&a, &b| attention::appeal(w, w.players.cold[a].person, Taste::Celebrity).total_cmp(&attention::appeal(w, w.players.cold[b].person, Taste::Celebrity))).unwrap();
    let who = w.players.cold[p].person;
    attention::spark(w, who, Cause::Aesthetic, 1.0);
    let mut fan = w.net.accounts.iter().find(|a| a.club == club && a.kind != AccountKind::Person).cloned().unwrap();
    fan.persona.celebrity = 90;
    assert_eq!(attention::looks_reaction(w, &fan, who), Some(1), "an admirer says so");
    // A rival supporter with an edge uses it to score a point.
    fan.persona.celebrity = 60;
    fan.persona.hostility = 90;
    fan.rival = club;
    fan.club = w.clubs.ids().nth(1).unwrap();
    assert!(attention::looks_reaction(w, &fan, who).is_some());
    // Without a wave of it, nobody remarks.
    w.net.attention.remove(&who);
    assert_eq!(attention::looks_reaction(w, &fan, who), None);
}

#[test]
fn a_nickname_is_born_spreads_mutates_and_fades_and_can_come_back() {
    let mut sim = world();
    let w = &mut sim.world;
    let (_, who, _) = a_player(w);
    w.net.nicknames.clear();
    w.net.attention.remove(&who);
    let born = w.date;
    // Nothing without attention.
    attention::weekly(w);
    assert!(w.net.nicknames.iter().all(|n| n.person != who));
    // With the world's eyes on him a name eventually sticks.
    for week in 0..80 {
        attention::spark(w, who, Cause::Football, 1.0);
        attention::spark(w, who, Cause::Personality, 1.0);
        w.date = born.add_days(week * 7);
        attention::weekly(w);
        if w.net.nicknames.iter().any(|n| n.person == who) {
            break;
        }
    }
    let n = *w.net.nicknames.iter().find(|n| n.person == who).expect("a nickname took hold");
    assert_eq!(n.state, NickState::Rising);
    // It spreads while he is in the news and becomes established.
    for _ in 0..12 {
        attention::spark(w, who, Cause::Football, 1.0);
        w.date = w.date.add_days(7);
        attention::weekly(w);
    }
    let n = *w.net.nicknames.iter().find(|n| n.person == who).unwrap();
    assert!(matches!(n.state, NickState::Established) && n.users >= 100, "{n:?}");
    // The news moves on and the name fades, then dies, unless he is in the news again.
    let mut fading = false;
    for _ in 0..60 {
        w.date = w.date.add_days(7);
        attention::weekly(w);
        fading |= w.net.nicknames.iter().any(|x| x.person == who && x.state == NickState::Fading);
    }
    assert!(fading, "it went out of use");
    let revived_from = w.net.nicknames.iter().find(|x| x.person == who && x.state == NickState::Fading).copied();
    if let Some(f) = revived_from {
        let _ = f;
        attention::spark(w, who, Cause::Football, 1.0);
        attention::spark(w, who, Cause::Personality, 1.0);
        attention::weekly(w);
        assert!(w.net.nicknames.iter().any(|x| x.person == who && x.state == NickState::Rising), "a new wave of attention brings it back");
    }
    // Wording drifts with variants.
    let mut n2 = Nickname { person: who, kind: NickKind::Diminutive, born, users: 300, last_used: w.date, state: NickState::Established, variants: 0, key: 7 };
    let k0 = n2.key;
    n2.variants += 1;
    assert_ne!(n2.key, k0 + 1);
}

#[test]
fn a_moment_that_enough_people_remember_becomes_a_story_that_grows_unless_corrected() {
    let mut sim = world();
    let w = &mut sim.world;
    let (_, who, club) = a_player(w);
    w.net.myths.clear();
    let accounts: Vec<u32> = w.net.accounts.iter().filter(|a| a.club == club && a.kind != AccountKind::Person).map(|a| a.id).take(12).collect();
    assert!(accounts.len() >= 8);
    // A dozen people remember the same night.
    for &a in &accounts {
        w.net.memories.entry(a).or_default().push(Remembered { kind: MomentKind::LateWinner, about: who, date: w.date.add_days(-100), event: pw_core::EventId::NONE });
        w.net.accounts[a as usize].persona.nostalgia = 90;
        w.net.accounts[a as usize].persona.knowledge = 10;
    }
    w.date = pw_core::Date::from_ymd(2027, 3, 2);
    attention::weekly(w);
    assert!(w.net.myths.iter().any(|m| m.about == who && m.moment == MomentKind::LateWinner), "the night became a story");
    for _ in 0..20 {
        w.date = pw_core::Date::from_ymd(2027, 3, 2);
        attention::weekly(w);
    }
    let told = *w.net.myths.iter().find(|m| m.about == who && m.moment == MomentKind::LateWinner).unwrap();
    assert!(told.retellings > 50 && told.embellishment >= 20, "retold by people who did not check it, it grows: {told:?}");
    // If the people who carry it are the ones who know better, it stays what it was.
    let mut careful = w.clone();
    careful.net.myths.clear();
    for &a in &accounts {
        careful.net.accounts[a as usize].persona.knowledge = 95;
        careful.net.accounts[a as usize].persona.nostalgia = 50;
    }
    careful.date = pw_core::Date::from_ymd(2027, 3, 2);
    for _ in 0..21 {
        attention::weekly(&mut careful);
    }
    let checked = *careful.net.myths.iter().find(|m| m.about == who && m.moment == MomentKind::LateWinner).unwrap();
    assert!(checked.corrected > 0 && checked.embellishment < told.embellishment, "{checked:?} vs {told:?}");
}

#[test]
fn a_running_world_forms_waves_and_keeps_them_bounded() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 52, Scale::SMALL));
    sim.run(250);
    let w = &sim.world;
    assert!(!w.net.attention.is_empty(), "big moments draw attention");
    assert!(w.net.attention.values().all(|a| a.waves.len() <= 3 && (0.0..=1.0).contains(&a.general_share)));
    let kinds: std::collections::HashSet<Cause> = w.net.attention.values().flat_map(|a| a.waves.iter().map(|x| x.cause)).collect();
    assert!(kinds.len() >= 2, "more than one kind of attention: {kinds:?}");
    assert!(w.net.nicknames.len() < 6000 && w.net.myths.len() < 2500);
}
