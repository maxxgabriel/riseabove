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
use pw_core::{ClubId, PersonId};
use pw_world::ecosystem::{Scholarship, StageKind};
use pw_world::event::{EventKind, Visibility};
use pw_world::recog::{Learned, Org};
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
    // What each university believes it has seen, best first.
    let mut recruiters: Vec<Recruiter> = Vec::new();
    for &u in &unis {
        let Some(prof) = w.ext.ecosystem.inst.get(&u).cloned() else { continue };
        let inst = w.minor.institutions[u as usize].clone();
        let held = w.ext.ecosystem.scholarship.values().filter(|s| s.inst == u).count();
        let slots = usize::from(prof.scholarships).saturating_sub(held);
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
            // A university knows a boy through its own people: it has watched him before, or a coach it trusts sent word.
            let org = Org::Institution(u);
            let known = if crate::recognition::looks_by(w, org, p) >= 1 { 0.20 } else { 0.0 };
            let word = 0.30 * crate::recognition::vouch_weight(w, Some(org), p);
            let p_see = ((0.10 + 0.55 * cov) * (0.4 + 0.6 * prof.resources / 100.0) * near + form + if released { 0.25 } else { 0.0 } + known + word).clamp(0.0, 0.95);
            if w.roll(stream::MINOR, &[u64::from(u), u64::from(p.0), year, 0x5ee]) >= p_see {
                continue;
            }
            let how = if word > 0.0 { Learned::Recommended } else { Learned::Watched };
            crate::recognition::sighted_by(w, org, PersonId::NONE, p, how);
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
        recruiters.push(Recruiter { inst: u, bar, resources: prof.resources, ranked: seen, next: 0, slots });
    }
    // Rounds: each programme offers its open places to the best it has seen and not yet lost; each player answers; a place turned
    // down goes to the next name on the list. Programmes that want the same player bid against each other once.
    let mut settled: pw_world::FxHashSet<PlayerId> = Default::default();
    let mut lost: pw_world::FxHashSet<(u32, PlayerId)> = Default::default();
    for round in 0..ROUNDS {
        let mut offers: Vec<Offer> = Vec::new();
        for r in &mut recruiters {
            let mut open = r.slots;
            while open > 0 && r.next < r.ranked.len() {
                let (score, p) = r.ranked[r.next];
                r.next += 1;
                if settled.contains(&p) || lost.contains(&(r.inst, p)) {
                    continue;
                }
                offers.push(Offer { player: p, inst: r.inst, tier: first_tier(r, score), score, raised: false });
                open -= 1;
            }
        }
        if offers.is_empty() {
            break;
        }
        offers.sort_by_key(|o| (o.player, o.inst));
        let mut k = 0;
        while k < offers.len() {
            let p = offers[k].player;
            let n = offers[k..].iter().take_while(|o| o.player == p).count();
            let mut mine: Vec<Offer> = offers[k..k + n].to_vec();
            k += n;
            if mine.len() > 1 {
                counter_offers(w, p, &mut mine, &recruiters);
            }
            match choose(w, p, &mine, year, round) {
                Some(u) => {
                    settled.insert(p);
                    if let Some(r) = recruiters.iter_mut().find(|r| r.inst == u) {
                        r.slots -= 1;
                    }
                    for o in mine.iter().filter(|o| o.inst != u) {
                        lost.insert((o.inst, p));
                    }
                }
                // The professional route, or home: nobody gets the player this year.
                None => {
                    settled.insert(p);
                }
            }
        }
    }
}

/// Recruiting rounds in a September: offers, answers, and places turned down offered again.
const ROUNDS: usize = 3;

/// A programme's view of its recruiting: what it holds itself to, what it can spend, whom it has seen (best first) and how many places
/// are still open.
struct Recruiter {
    inst: u32,
    bar: f32,
    resources: f32,
    ranked: Vec<(f32, PlayerId)>,
    next: usize,
    slots: usize,
}

#[derive(Clone, Copy)]
struct Offer {
    player: PlayerId,
    inst: u32,
    tier: u8,
    score: f32,
    raised: bool,
}

/// The scholarship a programme opens with: the better it thinks the player and the more it has, the more it offers.
fn first_tier(r: &Recruiter, score: f32) -> u8 {
    if r.resources > 60.0 && score > r.bar + 25.0 {
        3
    } else if r.resources > 40.0 && score > r.bar + 10.0 {
        2
    } else {
        1
    }
}

/// The most a programme will stretch to for a player it rates `score`: one step above its opening offer, within its means.
fn ceiling(r: &Recruiter, score: f32) -> u8 {
    let top = if r.resources > 60.0 { 3 } else if r.resources > 40.0 { 2 } else { 1 };
    let wanted = if score > r.bar + 15.0 { 3 } else if score > r.bar + 5.0 { 2 } else { 1 };
    top.min(wanted).max(first_tier(r, score))
}

/// A player holding several offers: each programme that is not the player's first choice may raise its scholarship once, if it rates the player highly enough
/// and can afford to. The family hears the improved offers before deciding.
fn counter_offers(w: &World, p: PlayerId, mine: &mut [Offer], recruiters: &[Recruiter]) {
    let ctx = Family::of(w, p);
    let best = mine.iter().map(|o| ctx.appeal(w, o.inst, o.tier)).fold(f32::MIN, f32::max);
    for o in mine.iter_mut() {
        if ctx.appeal(w, o.inst, o.tier) >= best {
            continue;
        }
        let Some(r) = recruiters.iter().find(|r| r.inst == o.inst) else { continue };
        let cap = ceiling(r, o.score);
        if cap > o.tier {
            o.tier = cap;
            o.raised = true;
        }
    }
}

/// What a family weighs in an offer: the scholarship, the programme's football and name, how far from home, in which language.
struct Family {
    home: RegionId,
    lang: u8,
    support: f32,
}

impl Family {
    fn of(w: &World, p: PlayerId) -> Family {
        let who = w.players.cold[p].person;
        let home = w.ext.ecosystem.story.get(&p).map_or(RegionId::NONE, |s| s.home);
        let lang = if home.is_some() { w.ext.ecosystem.regions[home].language } else { 0 };
        let support = w.lives.get(who).map_or(0.5, |l| f32::from(l.household.parents.support) / 20.0);
        Family { home, lang, support }
    }

    fn appeal(&self, w: &World, u: u32, tier: u8) -> f32 {
        let inst = &w.minor.institutions[u as usize];
        let prof = &w.ext.ecosystem.inst[&u];
        let travel = w.ext.ecosystem.travel_burden(self.home, prof.region);
        let same_language = if self.home.is_some() && prof.region.is_some() && w.ext.ecosystem.regions[prof.region].language == self.lang { 0.05 } else { 0.0 };
        0.5 * tier_value(tier) + 0.25 * prof.success / 100.0 + 0.15 * f32::from(inst.prestige) / 1000.0 - 0.6 * travel * (1.2 - self.support) + same_language
    }
}

/// The player answers: the best offer, or none of them if the alternative looks better. Returns the programme chosen.
fn choose(w: &mut World, p: PlayerId, offers: &[Offer], year: u64, round: usize) -> Option<u32> {
    let today = w.date;
    let who = w.players.cold[p].person;
    let ctx = Family::of(w, p);
    let ambition = consider::hid(w, who, Hidden::Ambition) / 20.0;
    let mut scored: Vec<(f32, Offer)> = offers.iter().map(|o| (ctx.appeal(w, o.inst, o.tier), *o)).collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.inst.cmp(&b.1.inst)));
    let (appeal, best) = *scored.first()?;
    let rival = scored.get(1).map(|x| x.1.inst);
    // What else he could do: the better he is, the more the professional route tempts, more so if he is ambitious.
    // How good he thinks he is, not how good he is.
    let ca = crate::consider::self_view(w, p);
    let alternative = (0.15 + 0.6 * ((ca - 55.0) / 60.0).clamp(0.0, 1.0)) * (0.7 + 0.6 * ambition);
    let noise = (w.roll(stream::MINOR, &[u64::from(p.0), year, 0x5c0]) - 0.5) * 0.1;
    if appeal + noise <= alternative {
        return None;
    }
    let u = best.inst;
    w.minor.join(p, u);
    w.minor.enrolled.insert(p, today.year());
    w.ext.ecosystem.scholarship.insert(p, Scholarship { inst: u, tier: best.tier, from: today });
    crate::ecosystem::note(w, p, StageKind::University, u);
    let region = w.ext.ecosystem.inst[&u].region;
    crate::ecosystem::set_dev_region(w, p, region);
    w.events.push(today, Visibility::Person(who), EventKind::EnrolledUniversity { person: who, institution: u });
    // A contested recruit is news around the programmes: who won the player, against whom, and whether it took a better scholarship.
    if let Some(over) = rival {
        w.events.push(today, Visibility::Public, EventKind::RecruitWon { person: who, institution: u, over, raised: best.raised, round: round as u8 + 1 });
    }
    Some(u)
}

/// July: programmes take stock. Results, alumni and sponsors set the budget; the budget sets coaching and places.
pub fn yearly(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let season = w.date.year() - 1;
    // Scholarships end when studies do.
    let mut gone: Vec<PlayerId> = w.ext.ecosystem.scholarship.iter().filter(|(p, s)| w.minor.member_of.get(p) != Some(&s.inst)).map(|(&p, _)| p).collect();
    gone.sort();
    for p in gone {
        let inst = w.ext.ecosystem.scholarship.remove(&p).map(|s| s.inst);
        // Nothing is guaranteed: a place is a chance. When it ends, how it went is recorded (a club took him, or nobody did), and the
        // source that recommended him to this university is trusted a little more or less next time.
        if let Some(inst) = inst {
            let signed = w.players.hot[p].club.is_some();
            crate::recognition::referral_outcome(w, Org::Institution(inst), p, signed);
        }
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

/// Professional clubs and state selectors watch university football, each through its own people and only where it can reach:
/// a club's scouts cover the university zone near it (a state, a city), not every campus in the country. A standout is seen by
/// the clubs whose coverage includes his ground, and by nobody else (the India brief, item 15).
pub(crate) fn watched_by_clubs(w: &mut World, p: PlayerId, minutes: u8) {
    let today = w.date;
    let Some(inst) = w.minor.member_of.get(&p).copied() else { return };
    let Some(zone) = w.ext.ecosystem.inst.get(&inst).map(|i| i.region) else { return };
    let sample = crate::recognition::games(w, p, pw_world::ecosystem::Tier::Adult) + crate::recognition::games(w, p, pw_world::ecosystem::Tier::State);
    let mut clubs: Vec<ClubId> = w.clubs.ids().collect();
    clubs.sort();
    for c in clubs {
        // Only clubs that sign senior players from the amateur game look at it.
        if crate::statepath::tier_of(w, c) < 2 {
            continue;
        }
        let scouts = w.clubs[c].staff.iter().filter(|&&s| w.staff[s].role == pw_world::StaffRole::Scout).count() as f32;
        // Covering a zone: near enough to attend, or a club that scouts nationally.
        let from = w.ext.ecosystem.region_of_club(c);
        let reach = if from.is_some() && zone.is_some() { (1.0 - 2.5 * w.ext.ecosystem.travel_burden(from, zone)).max(0.0) } else { 0.3 };
        let prob = ((0.06 + 0.05 * scouts).min(0.4)) * (0.15 + 0.85 * reach) * (0.45 + 0.55 * sample / (sample + 2.0));
        if (pw_core::rng::hash_key(&[w.seed, u64::from(c.0), u64::from(p.0), today.0 as u64, 0x5a3]) % 1000) as f32 / 1000.0 < prob {
            w.knowledge.observe(c, p, u16::from(minutes), today);
            let by = crate::ecosystem::scout_of(w, c).map_or(PersonId::NONE, |s| w.staff[s].person);
            crate::recognition::sighted_by(w, Org::Club(c), by, p, Learned::Watched);
        }
    }
    // The state's selectors follow the university teams of their own state.
    let state = w.ext.ecosystem.state_of(zone);
    if state.is_some() {
        crate::recognition::sighted_by(w, Org::State(state), PersonId::NONE, p, Learned::Watched);
    }
}
