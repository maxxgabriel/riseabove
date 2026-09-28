//! Referees, controversies, appeals, charges and atmosphere. See
//! `pw_world::officials`.

use pw_core::rng::{period, stream};
use pw_core::{ClubId, Hidden, NationId, PersonId, PlayerId};
use pw_match::{Ev, MatchResult};
use pw_world::careers::MediaStyle;
use pw_world::decision::{Choice, Decision, DecisionKind, MindKind};
use pw_world::event::{EventKind, Visibility};
use pw_world::names::NameId;
use pw_world::officials::{Appeal, AppealOutcome, Atmosphere, CallKind, Charge, ChargeKind, Controversy, Referee};
use pw_world::{Fixture, MemoryKind, Person, TeamKind, World};

use crate::consider;

// ---------------------------------------------------------------------------
// The referee pool
// ---------------------------------------------------------------------------

fn new_referee(w: &mut World, nation: NationId, tier: u8, key: u64) -> u32 {
    let today = w.date;
    let mut rng = w.rng(stream::STAFF, &[u64::from(nation.0), key, 0x4ef]);
    let (first, last) = crate::people::random_name(w, nation, &mut rng);
    let dob = today.add_days(-(365 * rng.range_i32(28, 44) + rng.range_i32(0, 364)));
    let person = w.people.push(Person {
        first,
        last,
        common: NameId::NONE,
        dob,
        nation,
        nation2: Default::default(),
        hidden: crate::generate::hidden_random(&mut rng),
        player: Default::default(),
        staff: Default::default(),
        mind: MindKind::Ai,
    });
    let id = w.officials.referees.len() as u32;
    let base = match tier {
        1 => 72.0,
        2 => 62.0,
        _ => 52.0,
    };
    w.officials.referees.push(Referee {
        id,
        person,
        nation,
        tier,
        strictness: rng.normal_ms(50.0, 15.0).clamp(10.0, 95.0) as u8,
        accuracy: rng.normal_ms(base, 8.0).clamp(20.0, 98.0) as u8,
        composure: rng.normal_ms(55.0, 15.0).clamp(10.0, 95.0) as u8,
        matches: 0,
        reds: 0,
        penalties: 0,
        big_calls: 0,
        wrong: 0,
        active: true,
        since: today,
    });
    w.officials.by_nation.entry(nation).or_default().push(id);
    w.officials.by_person.insert(person, id);
    id
}

/// Enough referees for every nation with leagues, by tier.
pub fn ensure(w: &mut World) {
    let nations: Vec<NationId> = w.nations.ids().filter(|&n| !w.nations[n].leagues.is_empty()).collect();
    for n in nations {
        let leagues = w.nations[n].leagues.clone();
        for (t, &comp) in leagues.iter().enumerate().take(3) {
            let tier = t as u8 + 1;
            let clubs = w.comps[comp].state.table.len().max(8);
            let want = clubs / 2 + 3;
            let have = w.officials.by_nation.get(&n).map_or(0, |v| v.iter().filter(|&&r| w.officials.referees[r as usize].tier == tier && w.officials.referees[r as usize].active).count());
            for k in have..want {
                new_referee(w, n, tier, (u64::from(tier) << 16) | k as u64 | (period::year(w.date) << 24));
            }
        }
    }
}

/// Who referees a fixture: a pure function of the fixture (so it can be
/// used while matches are simulated in parallel). Domestic fixtures get a
/// referee of the nation at the right tier; others a top referee from a
/// neutral nation.
pub fn referee_for(w: &World, fx: &Fixture) -> Option<u32> {
    let comp = &w.comps[fx.comp];
    let (hn, an) = (w.clubs[w.teams[fx.home].club].nation, w.clubs[w.teams[fx.away].club].nation);
    let pool: Vec<u32> = if comp.nation.is_some() {
        let tier = comp.tier.clamp(1, 3);
        let v: Vec<u32> = w.officials.by_nation.get(&comp.nation).map_or_else(Vec::new, |v| v.iter().copied().filter(|&r| w.officials.referees[r as usize].active && w.officials.referees[r as usize].tier == tier).collect());
        if v.is_empty() { w.officials.by_nation.get(&comp.nation).map_or_else(Vec::new, |v| v.iter().copied().filter(|&r| w.officials.referees[r as usize].active).collect()) } else { v }
    } else {
        w.officials.referees.iter().filter(|r| r.active && r.tier == 1 && r.nation != hn && r.nation != an).map(|r| r.id).collect()
    };
    if pool.is_empty() {
        return None;
    }
    let k = pw_core::rng::hash_key(&[w.seed, stream::STAFF, fx.uid, 0x4ef]) as usize % pool.len();
    Some(pool[k])
}

/// The strictness the match engine applies (0.75–1.25).
pub fn strictness(w: &World, fx: &Fixture) -> Option<f32> {
    referee_for(w, fx).map(|r| 0.75 + 0.5 * f32::from(w.officials.referees[r as usize].strictness) / 100.0)
}

// ---------------------------------------------------------------------------
// Atmosphere (before kick-off)
// ---------------------------------------------------------------------------

/// Today's senior fixtures get an atmosphere; its effects are small and
/// bounded (at most ±3 confidence, once per match), and the home crowd sings.
pub fn pre_match(w: &mut World) {
    let today = w.date;
    let fixtures: Vec<pw_core::FixtureId> = w.fixtures.on(today).to_vec();
    for f in fixtures {
        let fx = w.fixtures.get(f).clone();
        if w.teams[fx.home].kind != TeamKind::First || w.teams[fx.away].kind != TeamKind::First || fx.neutral {
            continue;
        }
        let club = w.teams[fx.home].club;
        let meaning = crate::culture::meaning(w, &fx);
        let groups: Vec<(u32, i8, u8)> = w.net.groups.iter().filter(|g| g.club == club).map(|g| (g.size, g.team, g.voice)).collect();
        let size: f32 = groups.iter().map(|g| g.0 as f32).sum::<f32>().max(1.0);
        let mood = groups.iter().map(|g| f32::from(g.1) * g.0 as f32).sum::<f32>() / size;
        let voice = groups.iter().map(|g| f32::from(g.2) * g.0 as f32).sum::<f32>() / size;
        let capacity = w.clubs[club].capacity.min(90_000) as f32 / 90_000.0;
        let occasion = f32::from(meaning.significance) / 100.0 + if meaning.derby { 0.3 } else { 0.0 };
        let level = (25.0 + capacity * 25.0 + voice * 0.25 + occasion * 30.0 + mood * 0.1).clamp(0.0, 100.0) as u8;
        let tribal = f32::from(w.culture.club(club).tribalism) / 100.0;
        let hostility = ((if meaning.derby { 50.0 } else { 15.0 }) + tribal * 30.0 + occasion * 15.0).clamp(0.0, 100.0) as u8;
        // The home crowd sings its most popular songs.
        let mut songs: Vec<(u8, u32)> = w.net.chants.iter().filter(|c| c.club == club && c.popularity >= 20).map(|c| (c.popularity, c.id)).collect();
        songs.sort_by(|a, b| b.cmp(a));
        let mut chants = [u32::MAX; 3];
        for (i, &(_, id)) in songs.iter().take(3).enumerate() {
            chants[i] = id;
            let c = &mut w.net.chants[id as usize];
            c.last_sung = today;
            c.popularity = (c.popularity + 1).min(100);
        }
        w.officials.atmosphere.insert(fx.uid, Atmosphere { level, hostility, chants });
        // Bounded effects: home players lifted a little; visitors who handle
        // pressure badly unsettled a little in hostile grounds.
        let lift = ((f32::from(level) - 50.0) / 50.0 * 3.0).clamp(-1.0, 3.0) as i16;
        let home_squad = w.teams[fx.home].squad.clone();
        for p in home_squad {
            let h = &mut w.players.hot[p];
            h.confidence = (i16::from(h.confidence) + lift).clamp(5, 100) as u8;
        }
        if hostility >= 50 {
            let away_squad = w.teams[fx.away].squad.clone();
            for p in away_squad {
                let who = w.players.cold[p].person;
                let nerve = consider::hid(w, who, Hidden::Pressure) / 20.0;
                let d = ((1.0 - nerve) * f32::from(hostility) / 100.0 * 3.0).min(3.0) as u8;
                let h = &mut w.players.hot[p];
                h.confidence = h.confidence.saturating_sub(d).max(5);
            }
        }
    }
    // Keep a month of atmospheres.
    if today.day() == 1 {
        let recent: std::collections::HashSet<u64> = w.recent_matches.list.iter().map(|m| m.uid).collect();
        w.officials.atmosphere.retain(|uid, _| recent.contains(uid));
    }
}

// ---------------------------------------------------------------------------
// After the match: big calls, grievances, charges
// ---------------------------------------------------------------------------

pub fn after_match(w: &mut World, fx: &Fixture, r: &MatchResult) {
    let today = w.date;
    let Some(rid) = referee_for(w, fx) else { return };
    w.officials.assigned.insert(fx.uid, rid);
    let clubs = [w.teams[fx.home].club, w.teams[fx.away].club];
    let senior = w.teams[fx.home].kind == TeamKind::First && w.teams[fx.away].kind == TeamKind::First;
    let (accuracy, composure) = {
        let x = &mut w.officials.referees[rid as usize];
        x.matches += 1;
        (f32::from(x.accuracy) / 100.0, f32::from(x.composure) / 100.0)
    };
    let hostility = w.officials.atmosphere.get(&fx.uid).map_or(0.0, |a| f32::from(a.hostility) / 100.0);
    let calls: Vec<(CallKind, PlayerId, u8, u8)> = r
        .events
        .iter()
        .filter_map(|e| match e.kind {
            Ev::Red => Some((CallKind::RedCard, e.player, e.side, (e.t / 60).min(130) as u8)),
            Ev::SecondYellow => Some((CallKind::SecondYellow, e.player, e.side, (e.t / 60).min(130) as u8)),
            // A penalty goes against the defending side.
            Ev::PenaltyGoal | Ev::PenaltyMiss if (e.t / 60) <= 120 => Some((CallKind::Penalty, e.player, 1 - e.side, (e.t / 60).min(130) as u8)),
            _ => None,
        })
        .collect();
    for (i, (kind, player, against_side, minute)) in calls.into_iter().enumerate() {
        let against = clubs[usize::from(against_side)];
        let benefited = clubs[1 - usize::from(against_side)];
        {
            let x = &mut w.officials.referees[rid as usize];
            x.big_calls += 1;
            match kind {
                CallKind::Penalty => x.penalties += 1,
                _ => x.reds += 1,
            }
        }
        // The truth, independent of which side: accuracy only.
        let p_right = 0.80 + 0.17 * accuracy;
        let correct = w.roll(stream::STAFF, &[fx.uid, i as u64, 0xca11]) < p_right;
        if !senior {
            continue;
        }
        // How wronged the losing side's supporters feel.
        let tribal = f32::from(w.culture.club(against).tribalism) / 100.0;
        let late = if minute >= 80 { 15.0 } else { 0.0 };
        let grievance = ((if correct { 20.0 } else { 60.0 }) + tribal * 25.0 + late + hostility * (1.0 - composure) * 10.0).clamp(0.0, 100.0) as u8;
        let id = w.officials.controversies.len() as u32;
        w.officials.controversies.push(Controversy { id, uid: fx.uid, date: today, referee: rid, kind, player, against, benefited, minute, correct, grievance, appeal: None });
        let g = w.officials.grievance.entry((against, rid)).or_default();
        let was_biased = *g >= 400;
        *g = (*g + u16::from(grievance) * 2).min(1000);
        let now_biased = *g >= 400;
        if grievance >= 50 {
            w.events.push(today, Visibility::Public, EventKind::RefereeControversy { controversy: id });
        }
        if now_biased && !was_biased {
            // The supporters now believe the referee is against them.
            let ref_person = w.officials.referees[rid as usize].person;
            w.media.move_fans(against, ref_person, -200, pw_world::FanReason::Performances, today);
        }
        // A red card may be appealed by the club.
        if matches!(kind, CallKind::RedCard) {
            consider_appeal(w, id);
        }
        // A manager who feels wronged may say so — and be charged for it.
        if grievance >= 60 {
            referee_comments(w, against, rid, id);
        }
    }
    // Too many cards: failing to control the players.
    if senior {
        for side in 0..2u8 {
            let yellows: u32 = r.lines.iter().filter(|l| l.side == side).map(|l| u32::from(l.yellows)).sum();
            let reds = r.events.iter().filter(|e| e.side == side && matches!(e.kind, Ev::Red | Ev::SecondYellow)).count();
            if yellows >= 6 || reds >= 2 {
                charge(w, clubs[usize::from(side)], ChargeKind::FailingToControl, fx.uid);
            }
        }
        // Supporters' views of referees decay slowly between meetings.
        if today.day() == 1 {
            for g in w.officials.grievance.values_mut() {
                *g = g.saturating_sub(20);
            }
        }
    }
}

fn charge(w: &mut World, club: ClubId, kind: ChargeKind, uid: u64) {
    let today = w.date;
    let revenue = w.clubs[club].finance.balance.max(0);
    let fine = match kind {
        ChargeKind::FailingToControl => 20_000 + revenue / 5000,
        ChargeKind::RefereeComments { .. } => 10_000 + revenue / 10_000,
    };
    let id = w.officials.charges.len() as u32;
    w.officials.charges.push(Charge { id, club, kind, date: today, uid, fine });
    w.clubs[club].finance.balance -= fine;
    w.events.push(today, Visibility::Public, EventKind::Charged { charge: id, club });
}

/// A manager who feels their side was wronged may criticise the referee —
/// from their own media style and temperament; saying it costs a charge.
fn referee_comments(w: &mut World, club: ClubId, rid: u32, controversy: u32) {
    let Some(m) = w.clubs[club].manager.get() else { return };
    let speaker = w.staff[m].person;
    if w.people[speaker].mind == MindKind::External {
        // A human manager says what they choose, at their press conference.
        return;
    }
    let style = w.careers.managers.get(&m).map(|p| p.media_style);
    let temper = 1.0 - consider::hid(w, speaker, Hidden::Temperament) / 20.0;
    let roll = w.roll(stream::PRESS, &[u64::from(controversy), u64::from(speaker.0), 0x4ef]);
    let inclined = match style {
        Some(MediaStyle::Combative) => 0.6,
        Some(MediaStyle::Candid) => 0.3,
        _ => 0.08,
    } * (0.5 + temper);
    if roll >= inclined {
        return;
    }
    let ref_person = w.officials.referees[rid as usize].person;
    crate::press::speak(w, speaker, ref_person, pw_world::media::Stance::Criticise);
    charge(w, club, ChargeKind::RefereeComments { person: speaker }, w.officials.controversies[controversy as usize].uid);
}

/// Whether the club appeals a red card: the manager judges from what they
/// saw (the truth with noise) and how much the player matters.
fn consider_appeal(w: &mut World, controversy: u32) {
    let today = w.date;
    let c = w.officials.controversies[controversy as usize];
    let Some(m) = w.clubs[c.against].manager.get() else { return };
    let speaker = w.staff[m].person;
    let seen = if c.correct { 0.25 } else { 0.75 } + (w.roll(stream::STAFF, &[u64::from(controversy), 0xa99]) - 0.5) * 0.4;
    let matters = matches!(w.players.cold[c.player].status, pw_world::SquadStatus::Star | pw_world::SquadStatus::Important);
    let ai_appeals = seen > 0.6 || (seen > 0.45 && matters);
    if w.people[speaker].mind == MindKind::External {
        w.decisions.push(Decision {
            person: speaker,
            player: w.people[speaker].player,
            kind: DecisionKind::Appeal { controversy },
            options: [Choice::Accept, Choice::Reject].into_iter().collect(),
            created: today,
            deadline: today.add_days(1),
            default: if ai_appeals { 0 } else { 1 },
            answer: None,
            resolved: false,
        });
        return;
    }
    if ai_appeals {
        lodge(w, controversy);
    }
}

/// A human's decision on an appeal.
pub fn decide_appeal(w: &mut World, controversy: u32, appeal: bool) {
    if appeal {
        lodge(w, controversy);
    }
}

fn lodge(w: &mut World, controversy: u32) {
    let today = w.date;
    let c = w.officials.controversies[controversy as usize];
    if c.appeal.is_some() {
        return;
    }
    let id = w.officials.appeals.len() as u32;
    w.officials.appeals.push(Appeal { id, controversy, club: c.against, player: c.player, lodged: today, decided: None, outcome: None });
    w.officials.controversies[controversy as usize].appeal = Some(id);
}

/// Daily: appeal panels sit two days after lodging.
pub fn daily(w: &mut World) {
    let today = w.date;
    let open: Vec<u32> = w.officials.appeals.iter().rev().take(500).filter(|a| a.decided.is_none() && a.lodged.days_until(today) >= 2).map(|a| a.id).collect();
    for id in open {
        let a = w.officials.appeals[id as usize];
        let c = w.officials.controversies[a.controversy as usize];
        let roll = w.roll(stream::STAFF, &[u64::from(id), 0xa9e]);
        // The panel sees the incident imperfectly.
        let outcome = if !c.correct {
            if roll < 0.8 { AppealOutcome::Rescinded } else { AppealOutcome::Upheld }
        } else if roll < 0.05 {
            AppealOutcome::Rescinded
        } else if roll < 0.15 {
            AppealOutcome::Extended
        } else {
            AppealOutcome::Upheld
        };
        let nation = w.clubs[a.club].nation;
        let red_ban = pw_world::rules::profile(w, nation).red_ban_straight;
        let h = &mut w.players.hot[a.player];
        match outcome {
            AppealOutcome::Rescinded => h.ban = h.ban.saturating_sub(red_ban),
            AppealOutcome::Extended => h.ban = h.ban.saturating_add(1),
            AppealOutcome::Upheld => {}
        }
        if matches!(outcome, AppealOutcome::Rescinded) {
            w.officials.referees[c.referee as usize].wrong += 1;
        }
        let x = &mut w.officials.appeals[id as usize];
        x.decided = Some(today);
        x.outcome = Some(outcome);
        w.events.push(today, Visibility::Public, EventKind::AppealDecided { appeal: id, player: a.player });
        // The player remembers who stood up for them.
        if let Some(m) = w.clubs[a.club].manager.get() {
            let mp = w.staff[m].person;
            let who = w.players.cold[a.player].person;
            let compat = consider::compat(w, who, mp);
            w.social.remember(who, mp, MemoryKind::DefendedMe, today, pw_core::EventId::NONE, true, 0.4, compat);
        }
    }
}

/// July: referees are reviewed on their season — accurate ones move up,
/// error-prone ones down; the oldest retire and are replaced.
pub fn season_review(w: &mut World) {
    let today = w.date;
    for i in 0..w.officials.referees.len() {
        let (person, tier, calls, wrong, active) = {
            let r = &w.officials.referees[i];
            (r.person, r.tier, r.big_calls, r.wrong, r.active)
        };
        if !active {
            continue;
        }
        let age = w.people[person].age(today);
        if age >= 46 {
            w.officials.referees[i].active = false;
            continue;
        }
        if calls < 5 {
            continue;
        }
        let rate = wrong as f32 / calls as f32;
        let r = &mut w.officials.referees[i];
        if rate < 0.06 && tier > 1 {
            r.tier -= 1;
        } else if rate > 0.2 && tier < 3 {
            r.tier += 1;
        }
        // Experience sharpens judgement a little.
        r.accuracy = (r.accuracy + 1).min(98);
    }
    ensure(w);
}

/// A referee's person, for text and for supporters' opinions.
pub fn referee_person(w: &World, id: u32) -> PersonId {
    w.officials.referees.get(id as usize).map_or(PersonId::NONE, |r| r.person)
}
