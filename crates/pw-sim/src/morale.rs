//! Weekly morale (03 §8) as a sum of named reasons: minutes against what the
//! player's status promised, how things stand with the manager, promises in
//! play, wages against teammates, results, the fans, teammates, the contract,
//! interest from elsewhere, and life at home (from the life model). The list
//! is kept on the person so "why am I unhappy?" always has an answer.
//!
//! Well-being is not computed here any more: the life model (`life.rs`) does
//! it monthly for every person with one set of rules.

use pw_core::Hidden;
use pw_world::life::{Mood, MoodFactor};
use pw_world::{PlayerStatus, PromiseState, World};

use crate::consider;

pub fn weekly(w: &mut World) {
    let ids: Vec<pw_core::PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect();
    let updates: Vec<(pw_core::PlayerId, f32, Mood)> = ids.iter().map(|&p| (p, compose(w, p))).map(|(p, (t, m))| (p, t, m)).collect();
    for (p, target, mood) in updates {
        let who = w.players.cold[p].person;
        let h = &mut w.players.hot[p];
        h.morale = pw_core::math::ewma(f32::from(h.morale), target, 0.25).clamp(5.0, 100.0) as u8;
        h.confidence = pw_core::math::ewma(f32::from(h.confidence), 60.0, 0.1).clamp(5.0, 100.0) as u8;
        w.lives[who].morale_why = mood;
    }
}

fn compose(w: &World, p: pw_core::PlayerId) -> (f32, Mood) {
    let who = w.players.cold[p].person;
    let person = &w.people[who];
    let ambition = person.hidden.f(Hidden::Ambition) / 20.0;
    let loyalty = person.hidden.f(Hidden::Loyalty) / 20.0;
    let mut mood = Mood::new();
    let mut push = |f: MoodFactor, v: f32| {
        let v = v.round().clamp(-30.0, 30.0) as i8;
        if v != 0 {
            mood.push((f, v));
        }
    };

    let (share, expected) = consider::minutes_share(w, p);
    let team_mins = consider::team_minutes_4w(w, w.players.hot[p].team);
    if team_mins >= 180 {
        push(MoodFactor::PlayingTime, ((share - expected) * 30.0).clamp(-20.0, 8.0) * (0.7 + ambition * 0.6));
    }
    if let Some(m) = w.manager_of_player(p) {
        let trust = consider::trust(w, who, m);
        let aff = consider::affinity(w, who, m);
        push(MoodFactor::Manager, (trust - 0.5) * 16.0 + aff * 8.0 - consider::grievance(w, who, m) * 4.0);
    }
    let open = w.social.open_promises_to(who).count() as f32;
    let broken = w.social.promises_to(who).filter(|pr| pr.state == PromiseState::Broken && pr.due.days_until(w.date) < 180).count() as f32;
    push(MoodFactor::Promises, open * 2.0 - broken * 7.0);
    let wage = f32::from(consider::wage_vs_peers(w, p));
    if wage < 80.0 {
        push(MoodFactor::Wage, -(80.0 - wage) / 6.0 * (0.5 + ambition));
    }
    let club = w.playing_club(p);
    if club.is_some() {
        push(MoodFactor::TeamResults, (f32::from(w.clubs[club].fan_mood) - 50.0) / 8.0);
        if let Some(f) = w.media.fan(club, who) {
            push(MoodFactor::Fans, f32::from(f.score) / 80.0 * (1.2 - person.hidden.f(Hidden::Pressure) / 40.0));
        }
    }
    let team = w.players.hot[p].team;
    if team.is_some() {
        let mates: Vec<f32> = w.teams[team].squad.iter().filter(|&&x| x != p).map(|&x| consider::affinity(w, who, w.players.cold[x].person)).filter(|a| a.abs() > 0.05).collect();
        if !mates.is_empty() {
            let avg = mates.iter().sum::<f32>() / mates.len() as f32;
            push(MoodFactor::Teammates, avg * 15.0);
        }
    }
    // The dressing room: settling in, and the mood of one's group.
    let (settling, group) = crate::dressing::mood_inputs(w, p);
    if let Some(s) = settling {
        push(MoodFactor::Settling, s);
    }
    if let Some(g) = group {
        push(MoodFactor::Manager, g);
    }
    let left = consider::contract_days_left(w, p);
    if (0..240).contains(&left) && !crate::negotiation::in_talks(w, p) {
        push(MoodFactor::Contract, -4.0 * (1.0 - loyalty * 0.5));
    }
    let interest = f32::from(consider::heard_interest(w, who).min(3));
    if interest > 0.0 {
        // Flattering for the ambitious; unsettling for the loyal.
        push(MoodFactor::Interest, interest * (ambition * 3.0 - loyalty * 1.5));
    }
    if w.market.has_requested(p) {
        push(MoodFactor::Role, -4.0);
    }
    let wb = f32::from(w.players.hot[p].wellbeing);
    push(MoodFactor::Family, (wb - 60.0) * 0.15);

    let total: f32 = mood.iter().map(|&(_, v)| f32::from(v)).sum();
    ((58.0 + total).clamp(5.0, 100.0), mood)
}
