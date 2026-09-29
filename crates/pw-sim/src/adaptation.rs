//! Settling in after a move (locked design 4.12-4.21).
//!
//! Each channel (surroundings, body clock, the football, the manager's system, life outside the club, confidence) has its own
//! distance, computed from real differences between where a player has been and where he is going, never from nationalities; his own
//! experience shortens it. Each has its own clock, set by his traits and by what the club does for him. The manager chooses how fast
//! to bring him in; playing heavily while unsettled strains him, playing well early speeds everything up. What a club expects of a
//! signing (`readiness`) is its belief, made from public facts and blurred by its staff's own judgement, never the true state.

use pw_core::{ClubId, Hidden, PersonId, PlayerId};
use pw_world::adaptation::{Adapting, Channel, Integration, Levels, N_CHANNELS, Readiness, Settled, support};
use pw_world::dossier::{Confidence, Domain};
use pw_world::knowledge::{Observer, perceive};
use pw_world::nation::Environment;
use pw_world::{PlayerStatus, World};

use crate::{boardroom, consider, dossier, planning, scouting};

fn env(w: &World, club: ClubId) -> Environment {
    w.nations[w.clubs[club].nation].env
}

/// Places a person has lived or played, as environments: where he was born, where he lives, and every club he has been at.
fn known_places(w: &World, p: PlayerId) -> Vec<pw_core::NationId> {
    let who = w.players.cold[p].person;
    let mut out = vec![w.people[who].nation];
    let home = w.lives[who].home;
    if home.is_some() {
        out.push(home);
    }
    if let Some(spells) = w.history.spells.get(&p) {
        out.extend(spells.iter().filter(|s| s.club.is_some()).map(|s| w.clubs[s.club].nation));
    }
    out.retain(|n| n.is_some());
    out.sort();
    out.dedup();
    out
}

fn climate_gap(a: &Environment, b: &Environment) -> f32 {
    (f32::from((a.climate - b.climate).abs()) / 6.0 * 0.6 + f32::from(a.humidity.abs_diff(b.humidity)) / 100.0 * 0.15 + f32::from(a.altitude.abs_diff(b.altitude)) / 2500.0 * 0.4).clamp(0.0, 1.0)
}

fn style_gap(a: &Environment, b: &Environment) -> f32 {
    ((f32::from(a.pace.abs_diff(b.pace)) + f32::from(a.physical.abs_diff(b.physical)) + f32::from(a.tempo.abs_diff(b.tempo))) / 90.0).clamp(0.0, 1.0)
}

fn tz_gap(a: &Environment, b: &Environment) -> f32 {
    (f32::from((a.tz - b.tz).abs()) / 8.0).clamp(0.0, 1.0)
}

/// How well he already speaks the language of `env`'s country, 0..1: fluency in that country's tongue or any that shares its family.
fn fluency(w: &World, who: PersonId, to: pw_core::NationId) -> f32 {
    let life = &w.lives[who];
    let target = w.nations[to].env.language;
    let best = life.languages.iter().map(|&(n, f)| if n == to || w.nations[n].env.language == target { f } else { 0 }).max().unwrap_or(0);
    f32::from(best) / 100.0
}

/// How far a move takes a player from what he knows on each channel. `from` is where he was (a club, or none for a free agent, in
/// which case where he lives); experience of similar places takes off part of the distance.
pub fn distances(w: &World, p: PlayerId, from: ClubId, to: ClubId) -> Levels {
    let who = w.players.cold[p].person;
    let e_to = env(w, to);
    let to_nation = w.clubs[to].nation;
    let e_from = if from.is_some() {
        env(w, from)
    } else {
        let home = w.lives[who].home;
        w.nations[if home.is_some() { home } else { w.people[who].nation }].env
    };
    let places = known_places(w, p);
    let has = |f: &dyn Fn(&Environment) -> bool| places.iter().any(|&n| f(&w.nations[n].env));

    let mut d = [0.0f32; N_CHANNELS];
    let similar_climate = has(&|e| climate_gap(e, &e_to) < 0.25);
    d[Channel::Environment.idx()] = climate_gap(&e_from, &e_to) * if similar_climate { 0.4 } else { 1.0 };
    let similar_clock = has(&|e| (e.tz - e_to.tz).abs() <= 2);
    d[Channel::Routine.idx()] = tz_gap(&e_from, &e_to) * if similar_clock { 0.4 } else { 1.0 };
    let similar_style = has(&|e| style_gap(e, &e_to) < 0.2);
    let level_jump = if from.is_some() { (f32::from(w.nations[w.clubs[from].nation].reputation) - f32::from(w.nations[to_nation].reputation)).abs() / 10_000.0 * 0.5 } else { 0.2 };
    d[Channel::Football.idx()] = ((style_gap(&e_from, &e_to) * 0.7 + level_jump) * if similar_style { 0.5 } else { 1.0 }).clamp(0.0, 1.0);

    // The manager's system: how different his ideas are from those the player has been working under.
    let new_mgr = w.clubs[to].manager.get();
    let old_mgr = if from.is_some() { w.clubs[from].manager.get() } else { None };
    d[Channel::Tactical.idx()] = match (new_mgr, old_mgr) {
        (Some(n), Some(o)) if n == o => 0.05,
        (Some(n), Some(o)) => {
            let (a, b) = (&w.staff[n].philosophy, &w.staff[o].philosophy);
            let gap = (f32::from(a.press.abs_diff(b.press)) + f32::from(a.tempo.abs_diff(b.tempo)) + f32::from(a.directness.abs_diff(b.directness))) / 300.0;
            (gap + if a.formations[0] != b.formations[0] { 0.25 } else { 0.0 }).clamp(0.05, 1.0)
        }
        (Some(_), None) => 0.6,
        _ => 0.3,
    };
    // Someone he has played under before, in the new dressing room, shortens it.
    let staff_overlap = w.clubs[to].staff.iter().any(|&s| w.social.get(w.staff[s].person, who).is_some_and(|r| r.trust > 55));
    if staff_overlap {
        d[Channel::Tactical.idx()] *= 0.75;
    }

    // Life outside the club: language above all, culture, the household, and having people around.
    let lang = 1.0 - fluency(w, who, to_nation);
    let culture = if e_from.culture == e_to.culture { 0.0 } else { 0.6 };
    let family = consider::household_move_cost(w, who, to_nation).min(1.0);
    let squad = &w.teams[w.clubs[to].first_team()].squad;
    let compatriots = squad.iter().filter(|&&q| q != p && w.people[w.players.cold[q].person].nation == w.people[who].nation).count().min(2) as f32;
    let friends = squad.iter().filter(|&&q| q != p && w.social.get(who, w.players.cold[q].person).is_some_and(|r| r.affinity > 20)).count().min(2) as f32;
    d[Channel::Social.idx()] = (0.5 * lang + 0.25 * culture + 0.25 * family - 0.12 * compatriots - 0.08 * friends).clamp(0.0, 1.0);

    let hid = &w.people[who].hidden;
    let fame = f32::from(w.players.cold[p].rep.world) / 10_000.0;
    let pressure = fame * (1.0 - hid.f(Hidden::Pressure) / 20.0);
    d[Channel::Mental.idx()] = (0.2 + 0.4 * family + 0.3 * pressure + 0.15 * d[Channel::Environment.idx()].max(d[Channel::Social.idx()])).clamp(0.0, 1.0);
    d
}

/// What a club does to ease a newcomer in (section 4.17): as much as its means, its staff and the newcomer's situation allow.
pub fn club_support(w: &World, club: ClubId, p: PlayerId) -> u8 {
    let c = &w.clubs[club];
    let rich = c.reputation >= 3500;
    let mut s = 0u8;
    if rich {
        s |= support::HOUSING | support::FAMILY;
    }
    let abroad = w.people[w.players.cold[p].person].nation != c.nation;
    if abroad && c.reputation >= 4500 {
        s |= support::LANGUAGE;
    }
    if c.facilities.medical >= 10 {
        s |= support::NUTRITION;
    }
    let staff = |r: pw_world::StaffRole| c.staff.iter().any(|&x| w.staff[x].role == r && !w.staff[x].retired);
    if staff(pw_world::StaffRole::DirectorOfFootball) || staff(pw_world::StaffRole::Assistant) {
        s |= support::LIAISON;
    }
    if staff(pw_world::StaffRole::SportsScientist) {
        s |= support::PSYCHOLOGY;
    }
    if staff(pw_world::StaffRole::FitnessCoach) {
        s |= support::CONDITIONING;
    }
    let team = c.first_team();
    let pos = w.players.cold[p].best_pos.group();
    if w.teams[team].squad.iter().any(|&q| q != p && w.age(q) >= 28 && (w.players.cold[q].best_pos.group() == pos || w.people[w.players.cold[q].person].nation == w.people[w.players.cold[p].person].nation)) {
        s |= support::MENTOR;
    }
    s
}

/// Weeks each channel needs for this player at this club.
fn weeks_for(w: &World, p: PlayerId, distance: &Levels, sup: u8) -> Levels {
    let who = w.players.cold[p].person;
    let hid = &w.people[who].hidden;
    let adapt = hid.f(Hidden::Adaptability) / 20.0;
    let prof = hid.f(Hidden::Professionalism) / 20.0;
    let resilience = (hid.f(Hidden::Pressure) + hid.f(Hidden::Professionalism)) / 40.0;
    let age = w.age_years(p);
    let has = |f: u8| sup & f != 0;
    let mut out = [0.0f32; N_CHANNELS];
    for ch in Channel::ALL {
        let i = ch.idx();
        let trait_factor = match ch {
            Channel::Environment | Channel::Routine => 1.15 - 0.3 * adapt,
            Channel::Football => 1.2 - 0.4 * adapt,
            Channel::Tactical => 1.25 - 0.45 * prof,
            Channel::Social => (1.2 - 0.4 * adapt) * if age > 28.0 { 1.1 } else { 1.0 },
            Channel::Mental => 1.3 - 0.6 * resilience,
        };
        let help = match ch {
            Channel::Environment | Channel::Routine => (if has(support::NUTRITION) { 0.9 } else { 1.0 }) * (if has(support::CONDITIONING) { 0.85 } else { 1.0 }),
            Channel::Football | Channel::Tactical => if has(support::MENTOR) { 0.9 } else { 1.0 },
            Channel::Social => (if has(support::LANGUAGE) { 0.8 } else { 1.0 }) * (if has(support::HOUSING) || has(support::FAMILY) { 0.9 } else { 1.0 }) * (if has(support::LIAISON) { 0.9 } else { 1.0 }) * (if has(support::MENTOR) { 0.92 } else { 1.0 }),
            Channel::Mental => (if has(support::PSYCHOLOGY) { 0.85 } else { 1.0 }) * (if has(support::LIAISON) { 0.9 } else { 1.0 }) * (if has(support::HOUSING) || has(support::FAMILY) { 0.92 } else { 1.0 }),
        };
        out[i] = (ch.base_weeks() * (0.4 + 1.6 * distance[i]) * trait_factor * help).max(1.0);
    }
    out
}

/// How the manager brings a signing in (section 4.20): urgency, how much better he is than what is there, how ready he is in body
/// and system, the manager's own appetite for risk, and what else he has.
pub fn choose_integration(w: &World, club: ClubId, p: PlayerId, distance: &Levels) -> Integration {
    let group = w.players.cold[p].best_pos.group();
    let urgency = planning::need_detail(w, club, group).map_or(0.4, |n| f32::from(n.urgency) / 3.0);
    let ca = scouting::view(w, club, p).0;
    let team = w.clubs[club].first_team();
    let mut mates: Vec<f32> = w.teams[team].squad.iter().filter(|&&q| q != p && w.players.cold[q].best_pos.group() == group).map(|&q| scouting::view(w, club, q).0).collect();
    mates.sort_by(|a, b| b.total_cmp(a));
    let bar = mates.get(1).copied().unwrap_or(ca - 5.0);
    let edge = ((ca - bar) / 10.0).clamp(-1.0, 1.0);
    let body = distance[Channel::Environment.idx()].max(distance[Channel::Routine.idx()]);
    let system = distance[Channel::Tactical.idx()];
    let tolerance = w.clubs[club].manager.get().map_or(0.5, |m| boardroom::tendency(w, w.staff[m].person));
    let unready = 0.6 * body + 0.4 * system;
    let score = 0.4 * urgency + 0.5 * edge + 0.3 * tolerance - 0.9 * unready;
    if body > 0.7 && tolerance < 0.6 {
        Integration::TrainingOnly
    } else if body > 0.5 {
        Integration::ConditioningFirst
    } else if score >= 0.35 {
        Integration::StartImmediately
    } else if system > 0.6 && edge > 0.0 {
        Integration::SimplifiedRole
    } else if score >= 0.15 {
        Integration::ReducedMinutes
    } else if score >= 0.0 {
        Integration::Gradual
    } else {
        Integration::Substitute
    }
}

fn plan_weeks(plan: Integration) -> i32 {
    match plan {
        Integration::StartImmediately => 0,
        Integration::Gradual => 3,
        Integration::Substitute => 4,
        Integration::TrainingOnly => 2,
        Integration::ReducedMinutes => 4,
        Integration::SimplifiedRole => 8,
        Integration::ConditioningFirst => 3,
    }
}

/// A player has joined `to` from `from` (or from nowhere, for a free agent). Signing does not make him available, acclimatised,
/// integrated or performing: he starts settling now.
pub fn begin(w: &mut World, p: PlayerId, from: ClubId, to: ClubId) {
    let today = w.date;
    if w.players.hot[p].status == PlayerStatus::Retired || to.is_none() {
        return;
    }
    let distance = distances(w, p, from, to);
    let sup = club_support(w, to, p);
    let weeks = weeks_for(w, p, &distance, sup);
    let plan = choose_integration(w, to, p, &distance);
    w.adaptation.done.remove(&p);
    w.adaptation.current.insert(
        p,
        Adapting { since: today, club: to, distance, weeks, progress: [0.0; N_CHANNELS], support: sup, plan, plan_until: today.add_days(plan_weeks(plan) * 7), strain: 0.0, form: 0.0 },
    );
}

/// How far he is from playing at his level, on the three things that decide it: body and clock, the football and the system, and his
/// head. Each 0..1; 1 when nothing is left to settle.
pub fn state(a: &Adapting) -> (f32, f32, f32) {
    let g = |c: Channel| a.progress[c.idx()];
    let acclimatised = 0.5 * g(Channel::Environment) + 0.5 * g(Channel::Routine);
    let integrated = 0.3 * g(Channel::Football) + 0.45 * g(Channel::Tactical) + 0.25 * g(Channel::Social);
    (acclimatised, integrated, g(Channel::Mental))
}

/// Multipliers on what he brings to a match: `(body, execution, mind)`. A settled player is at 1.
pub fn effect(w: &World, p: PlayerId) -> (f32, f32, f32) {
    let Some(a) = w.adaptation.current.get(&p) else { return (1.0, 1.0, 1.0) };
    let (acc, integ, mind) = state(a);
    let simple = if a.plan == Integration::SimplifiedRole { 0.6 } else { 1.0 };
    (0.85 + 0.15 * acc, 1.0 - 0.20 * (1.0 - integ) * simple, 0.90 + 0.10 * mind)
}

/// How much the manager wants to hold him out of the starting eleven now, 0 (free to play) .. 1 (not this week).
pub fn hold(w: &World, p: PlayerId) -> f32 {
    let Some(a) = w.adaptation.current.get(&p) else { return 0.0 };
    if w.date > a.plan_until {
        return 0.0;
    }
    match a.plan {
        Integration::StartImmediately => 0.0,
        Integration::Gradual => 0.5,
        Integration::Substitute => 0.9,
        Integration::TrainingOnly | Integration::ConditioningFirst => 1.0,
        Integration::ReducedMinutes => 0.25,
        Integration::SimplifiedRole => 0.15,
    }
}

/// Weekly: everyone still settling moves on, faster or slower for how his football goes and how hard he is being used.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let ids: Vec<PlayerId> = w.adaptation.current.keys().copied().collect();
    for p in ids {
        let Some(mut a) = w.adaptation.current.remove(&p) else { continue };
        let h = &w.players.hot[p];
        if h.status != PlayerStatus::Active || h.club != a.club && w.players.cold[p].loan.is_none() {
            // He moved on or left the game: the settling is over, unfinished.
            continue;
        }
        let acc = 0.5 * (a.progress[Channel::Environment.idx()] + a.progress[Channel::Routine.idx()]);
        let minutes = f32::from(h.minutes_week);
        let avg = h.form_avg().unwrap_or(6.7);
        // Negative loop: heavy minutes on a body and clock that have not settled.
        let overload = ((minutes / 90.0) - 1.2).max(0.0) * (0.5 - acc).max(0.0) * 2.0;
        a.strain = a.strain * 0.9 + overload;
        if overload > 0.0 {
            let hm = &mut w.players.hot[p];
            hm.fatigue = (f32::from(hm.fatigue) + 4.0 * overload).min(100.0) as u8;
            hm.confidence = (f32::from(hm.confidence) - 2.0 * overload).max(5.0) as u8;
            hm.acute += 25.0 * overload;
        }
        // Positive loop: playing well, early, speeds the rest along; playing badly slows it.
        a.form = a.form * 0.8 + (avg - 6.7) * 0.2;
        let boost = if a.form > 0.3 && minutes > 0.0 { 1.25 } else if a.form < -0.4 { 0.8 } else { 1.0 };
        for ch in Channel::ALL {
            let i = ch.idx();
            let mut rate = 1.0 / a.weeks[i];
            if matches!(ch, Channel::Social | Channel::Tactical | Channel::Mental) {
                rate *= boost;
            }
            // Football itself is learned by playing; the body settles whether he plays or not.
            if ch == Channel::Football && minutes == 0.0 {
                rate *= 0.6;
            }
            if matches!(ch, Channel::Environment | Channel::Routine) {
                rate *= 1.0 - 0.3 * a.strain.min(1.0);
            }
            a.progress[i] = (a.progress[i] + rate).min(1.0);
        }
        let weeks = (a.since.days_until(today) / 7) as u16;
        if a.progress.iter().all(|&x| x >= 0.97) {
            let slowest = Channel::ALL.into_iter().max_by(|x, y| a.weeks[x.idx()].total_cmp(&a.weeks[y.idx()])).unwrap_or(Channel::Social);
            let expected = a.weeks.iter().copied().fold(0.0, f32::max);
            w.adaptation.done.insert(p, Settled { date: today, weeks, slowest, struggled: f32::from(weeks) > expected * 1.5 });
        } else if weeks >= 78 {
            let slowest = Channel::ALL.into_iter().min_by(|x, y| a.progress[x.idx()].total_cmp(&a.progress[y.idx()])).unwrap_or(Channel::Social);
            w.adaptation.done.insert(p, Settled { date: today, weeks, slowest, struggled: true });
        } else {
            w.adaptation.current.insert(p, a);
        }
    }
    // Old records of how settling went are not kept for ever.
    if today.month() == 1 && today.day() <= 7 {
        w.adaptation.done.retain(|_, s| s.date.days_until(today) < 3 * 365);
    }
}

/// Whether adaptation was his trouble: still unsettled a long way in, or recorded as having taken far too long.
pub fn struggled(w: &World, p: PlayerId) -> bool {
    if let Some(a) = w.adaptation.current.get(&p) {
        let weeks = a.since.days_until(w.date) / 7;
        let mean = a.progress.iter().sum::<f32>() / N_CHANNELS as f32;
        return weeks >= 30 && mean < 0.75;
    }
    w.adaptation.done.get(&p).is_some_and(|s| s.struggled)
}

/// What a club expects of a signing's settling (section 4.19), from public facts about the places involved and what is known of him,
/// blurred by how good its own staff are at reading a player's adaptability. Not the true state, which lives in [`Adapting`].
pub fn readiness(w: &World, club: ClubId, p: PlayerId) -> Readiness {
    let from = w.players.hot[p].club;
    let truth = distances(w, p, from, club);
    let staff_judge = w.clubs[club]
        .staff
        .iter()
        .copied()
        .filter(|&s| !w.staff[s].retired)
        .map(|s| dossier::competence(w, s, Domain::Adaptability))
        .fold(6.0f32, f32::max);
    let sigma = (0.28 - 0.012 * staff_judge).max(0.04);
    let seen = w.knowledge.seen(club, p).map_or(0, |s| s.minutes);
    // Not knowing him widens it: the club reads a stranger's adaptability much less well.
    let wide = if seen >= 300 { 1.0 } else { 1.8 };
    let mut risks = [pw_world::adaptation::Level::Low; N_CHANNELS];
    let mut est = [0.0f32; N_CHANNELS];
    for ch in Channel::ALL {
        let i = ch.idx();
        est[i] = perceive(truth[i], sigma * wide, Observer::Club(club), p, 7000 + i as u64).clamp(0.0, 1.0);
        risks[i] = pw_world::adaptation::Level::of(est[i]);
    }
    let sup = club_support(w, club, p);
    let weeks = weeks_for(w, p, &est, sup);
    // Useful when the body, the football and the system are mostly there (seven tenths), not when every channel is finished.
    let to_useful = [Channel::Environment, Channel::Routine, Channel::Football, Channel::Tactical].iter().map(|c| weeks[c.idx()] * 0.7).fold(0.0, f32::max);
    let body = est[Channel::Environment.idx()].max(est[Channel::Routine.idx()]);
    let confidence = if sigma * wide < 0.1 { Confidence::High } else if sigma * wide < 0.2 { Confidence::Medium } else { Confidence::Low };
    Readiness {
        risks,
        weeks_to_useful: to_useful,
        immediate_availability: pw_world::adaptation::Level::of(1.0 - body.max(0.05)),
        immediate_effectiveness: pw_world::adaptation::Level::of(1.0 - (0.5 * est[Channel::Tactical.idx()] + 0.3 * est[Channel::Football.idx()] + 0.2 * body)),
        confidence,
    }
}

/// A quick read of how long a move would take to settle, from the two countries alone (no squads, no household): for ranking many
/// candidates. The full estimate is [`readiness`].
pub fn quick_weeks(w: &World, p: PlayerId, club: ClubId) -> f32 {
    let from = w.players.hot[p].club;
    if from.is_none() || w.clubs[from].nation == w.clubs[club].nation {
        return 0.0;
    }
    let (a, b) = (env(w, from), env(w, club));
    let who = w.players.cold[p].person;
    let lang = 1.0 - fluency(w, who, w.clubs[club].nation);
    let gap = |ch: Channel, d: f32| ch.base_weeks() * (0.4 + 1.6 * d);
    let body = gap(Channel::Environment, climate_gap(&a, &b)).max(gap(Channel::Routine, tz_gap(&a, &b)));
    let football = gap(Channel::Football, style_gap(&a, &b) * 0.7 + 0.1);
    (body.max(football).max(gap(Channel::Social, 0.5 * lang + 0.15)) * 0.7).min(60.0)
}

/// How much a club's urgency makes it mind the wait for a signing to become useful, 0..1: a rebuilding club minds little.
pub fn impatience(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let group = w.players.cold[p].best_pos.group();
    let urgency = planning::need_detail(w, club, group).map_or(0.3, |n| f32::from(n.urgency) / 3.0);
    let style = w.governance.get(&club).map_or(pw_world::governance::TransferStyle::Balanced, |g| g.policy.transfer_style);
    let rebuilding = matches!(style, pw_world::governance::TransferStyle::Develop | pw_world::governance::TransferStyle::Homegrown);
    let youth = if w.age(p) <= 21 { 0.5 } else { 1.0 };
    urgency * if rebuilding { 0.35 } else { 1.0 } * youth
}

/// Weeks still to go on a channel, for display.
pub fn weeks_left(a: &Adapting, c: Channel) -> f32 {
    ((1.0 - a.progress[c.idx()]) * a.weeks[c.idx()]).max(0.0)
}
