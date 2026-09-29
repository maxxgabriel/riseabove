//! University football as a second-chance route (Wave 4).
//!
//! Universities scout the school championships, state youth football,
//! district football and, above all, academy releases. What a university
//! sees depends on how much of a district's football is watched, how far it is
//! and its own outreach; what it thinks of a player is a noisy reading, and
//! nothing guarantees it is right. A player, with their family, weighs the
//! scholarship against distance, the university's football, and what else
//! their ability might get them; there is no correct answer.
//!
//! Universities are institutions with a budget that follows results, alumni
//! outcomes and sponsors, so programmes rise and fall over the years.

use pw_core::rng::stream;
use pw_core::{PlayerId, RegionId};
use pw_world::ecosystem::{Scholarship, StageKind};
use pw_world::event::{EventKind, Visibility};
use pw_world::knowledge::{Observer, perceive};
use pw_world::minor::InstKind;
use pw_core::Hidden;
use pw_world::{PlayerStatus, World};

use crate::consider;

const AGES: std::ops::RangeInclusive<u32> = 17..=22;

fn tier_value(t: u8) -> f32 {
    match t {
        0 => 0.1,
        1 => 0.3,
        2 => 0.55,
        _ => 0.85,
    }
}

/// September: universities look for players and offer places.
pub fn recruit(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let today = w.date;
    let year = today.year() as u64;
    // Who is available: not with a professional club, not already studying, still playing.
    let mut pool: Vec<PlayerId> = w
        .players
        .hot
        .iter_enumerated()
        .filter(|(_, h)| matches!(h.status, PlayerStatus::Amateur | PlayerStatus::FreeAgent))
        .map(|(p, _)| p)
        .filter(|&p| AGES.contains(&w.age(p)) && !w.minor.member_of.get(&p).is_some_and(|&i| w.minor.institutions[i as usize].kind == InstKind::University))
        .collect();
    pool.sort();
    if pool.is_empty() {
        return;
    }
    let mut unis: Vec<u32> = w.minor.institutions.iter().filter(|i| i.kind == InstKind::University).map(|i| i.id).collect();
    unis.sort();
    // Offers, from what each university believes it has seen.
    let mut offers: Vec<(PlayerId, u32, u8, f32)> = Vec::new();
    for &u in &unis {
        let Some(prof) = w.ext.ecosystem.inst.get(&u).cloned() else { continue };
        let inst = w.minor.institutions[u as usize].clone();
        let held = w.ext.ecosystem.scholarship.values().filter(|s| s.inst == u).count();
        let mut slots = usize::from(prof.scholarships).saturating_sub(held);
        if slots == 0 {
            continue;
        }
        let bar = 35.0 + 25.0 * f32::from(inst.prestige) / 1000.0 + f32::from(inst.coaching);
        let mut seen: Vec<(f32, PlayerId)> = Vec::new();
        for &p in &pool {
            let dev = w.ext.ecosystem.story.get(&p).map_or(RegionId::NONE, |s| s.dev);
            if dev.is_none() {
                continue;
            }
            let cov = w.ext.ecosystem.regions[dev].scouting_coverage / 100.0;
            let near = (1.0 - 0.8 * w.ext.ecosystem.travel_burden(prof.region, dev)).max(0.05);
            let released = w.youth.released.get(&p).is_some_and(|v| v.iter().any(|r| r.date.days_until(today) <= 400));
            let form = w.players.hot[p].form_avg().map_or(0.0, |f| ((f - 6.6) * 0.15).clamp(-0.1, 0.2));
            let p_see = ((0.10 + 0.55 * cov) * (0.4 + 0.6 * prof.resources / 100.0) * near + form + if released { 0.25 } else { 0.0 }).clamp(0.0, 0.95);
            if w.roll(stream::MINOR, &[u64::from(u), u64::from(p.0), year, 0x5ee]) >= p_see {
                continue;
            }
            // What the coaches make of him: a reading, not the truth.
            let sigma = 3.0 + (20.0 - f32::from(inst.coaching)) * 0.4;
            let c = &w.players.cold[p];
            let est_ca = perceive(f32::from(c.ca), sigma * 2.0, Observer::Person(1_000_000 + u), p, 5000); // truth-ok: a scout's noisy reading, with an observer-specific bias
            let est_pa = perceive(f32::from(c.pa), sigma * 4.0, Observer::Person(1_000_000 + u), p, 5001).max(est_ca); // truth-ok: a scout's noisy reading, with an observer-specific bias
            let score = 0.55 * est_ca + 0.45 * est_pa * (w.age(p) as f32 / 22.0).min(1.0).max(0.6);
            if score >= bar {
                seen.push((score, p));
            }
        }
        seen.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for (score, p) in seen {
            if slots == 0 {
                break;
            }
            let tier = if prof.resources > 60.0 && score > bar + 25.0 { 3 } else if prof.resources > 40.0 && score > bar + 10.0 { 2 } else { 1 };
            offers.push((p, u, tier, score));
            slots -= 1;
        }
    }
    // Each player weighs what he was offered against what else he might do.
    offers.sort_by_key(|o| (o.0, o.1));
    let mut k = 0;
    while k < offers.len() {
        let p = offers[k].0;
        let mine: Vec<(u32, u8, f32)> = offers[k..].iter().take_while(|o| o.0 == p).map(|o| (o.1, o.2, o.3)).collect();
        k += mine.len();
        choose(w, p, &mine, year);
    }
}

fn choose(w: &mut World, p: PlayerId, offers: &[(u32, u8, f32)], year: u64) {
    let today = w.date;
    let who = w.players.cold[p].person;
    let story = w.ext.ecosystem.story.get(&p).copied();
    let home = story.map_or(RegionId::NONE, |s| s.home);
    let lang = if home.is_some() { w.ext.ecosystem.regions[home].language } else { 0 };
    let support = w.lives.get(who).map_or(0.5, |l| f32::from(l.household.parents.support) / 20.0);
    let ambition = consider::hid(w, who, Hidden::Ambition) / 20.0;
    let score = |&(u, tier, _): &(u32, u8, f32)| -> f32 {
        let inst = &w.minor.institutions[u as usize];
        let prof = &w.ext.ecosystem.inst[&u];
        let travel = w.ext.ecosystem.travel_burden(home, prof.region);
        let same_language = if home.is_some() && prof.region.is_some() && w.ext.ecosystem.regions[prof.region].language == lang { 0.05 } else { 0.0 };
        0.5 * tier_value(tier) + 0.25 * prof.success / 100.0 + 0.15 * f32::from(inst.prestige) / 1000.0 - 0.6 * travel * (1.2 - support) + same_language
    };
    let best = offers.iter().copied().max_by(|a, b| score(a).total_cmp(&score(b)).then(b.0.cmp(&a.0))).unwrap();
    // What else he could do: the better he is, the more the professional route tempts, more so if he is ambitious.
    // How good he thinks he is, not how good he is.
    let ca = crate::consider::self_view(w, p);
    let alternative = (0.15 + 0.6 * ((ca - 55.0) / 60.0).clamp(0.0, 1.0)) * (0.7 + 0.6 * ambition);
    let noise = (w.roll(stream::MINOR, &[u64::from(p.0), year, 0x5c0]) - 0.5) * 0.1;
    if score(&best) + noise <= alternative {
        return;
    }
    let (u, tier, _) = best;
    w.minor.join(p, u);
    w.minor.enrolled.insert(p, today.year());
    w.ext.ecosystem.scholarship.insert(p, Scholarship { inst: u, tier, from: today });
    crate::ecosystem::note(w, p, StageKind::University, u);
    let region = w.ext.ecosystem.inst[&u].region;
    crate::ecosystem::set_dev_region(w, p, region);
    w.events.push(today, Visibility::Person(who), EventKind::EnrolledUniversity { person: who, institution: u });
}

/// July: programmes take stock. Results, alumni and sponsors set the budget; the budget sets coaching and places.
pub fn yearly(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let season = w.date.year() - 1;
    // Scholarships end when studies do.
    let gone: Vec<PlayerId> = w.ext.ecosystem.scholarship.iter().filter(|(p, s)| w.minor.member_of.get(p) != Some(&s.inst)).map(|(&p, _)| p).collect();
    for p in gone {
        w.ext.ecosystem.scholarship.remove(&p);
    }
    let mut ids: Vec<u32> = w.ext.ecosystem.inst.keys().copied().collect();
    ids.sort();
    for u in ids {
        let (kind, prestige, coaching, alumni) = {
            let i = &w.minor.institutions[u as usize];
            (i.kind, i.prestige, i.coaching, i.alumni_pros.len())
        };
        // This season's results: titles and finals, from the history books.
        let mut earned = 0.0f32;
        for h in w.minor.history.iter().rev().take(200).filter(|h| h.season == season) {
            if matches!(h.winner, pw_world::minor::Entrant::Inst(i) if i == u) {
                earned += if h.kind.is_cup() { 30.0 } else { 20.0 };
            } else if matches!(h.runner_up, pw_world::minor::Entrant::Inst(i) if i == u) {
                earned += 10.0;
            }
        }
        let region = w.ext.ecosystem.inst[&u].region;
        let private = if region.is_some() { w.ext.ecosystem.regions[region].invest_private } else { 30.0 };
        let prof = w.ext.ecosystem.inst.get_mut(&u).unwrap();
        prof.success = (prof.success * 0.75 + earned).clamp(0.0, 100.0);
        // Budget follows results, what alumni have gone on to, and sponsors; it never jumps.
        let target = 18.0 + 0.35 * prof.success + 0.25 * private + (alumni as f32 * 2.5).min(15.0);
        prof.resources = (prof.resources + 0.12 * (target - prof.resources)).clamp(5.0, 95.0);
        prof.facilities = (prof.facilities + 0.10 * (0.9 * prof.resources - prof.facilities)).clamp(5.0, 95.0);
        if kind == InstKind::University {
            prof.scholarships = (2.0 + prof.resources / 7.0).round().clamp(1.0, 16.0) as u8;
        }
        let res = prof.resources;
        // Coaches come and go with the money.
        let roll = w.roll(stream::MINOR, &[u64::from(u), season as u64, 0xc0a]);
        let i = &mut w.minor.institutions[u as usize];
        if res > 65.0 && coaching < 16 && roll < 0.35 {
            i.coaching += 1;
        } else if res < 25.0 && coaching > 3 && roll < 0.35 {
            i.coaching -= 1;
        }
        i.prestige = (f32::from(prestige) * 0.95 + 0.05 * (400.0 + 6.0 * w.ext.ecosystem.inst[&u].success)).clamp(50.0, 990.0) as u16;
    }
}
