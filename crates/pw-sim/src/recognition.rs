//! Being seen. In a country of a billion people a good game does not make a
//! name. What moves a young player on is *credible evidence at the right level*:
//! many games, steady, against real opposition, seen by people who can act, and
//! vouched for. This module keeps that evidence (`pw_world::ecosystem::Repute`)
//! and answers three questions from it:
//!
//! - `standing`: how much a player's name carries beyond their own pitch. It is
//!   weighted by the level the evidence came from, so a park full of goals adds
//!   little and a state-side season adds a lot.
//! - `attention`: how likely a scout at a match is to notice this player among the
//!   others, from a dozen separate aspects (`aspects`).
//! - `recognised_by`: whether an academy of a given standing has enough to invite
//!   them, which needs several sightings, a sample of games and, for the big
//!   academies, proof above grassroots level.
//!
//! Nothing here is a bonus for a player. It only changes how much of what a
//! player does reaches anyone. Cost is per game played and once a year, never
//! per person per day.

use pw_core::{ClubId, PlayerId, PosGroup, RegionId};
use pw_world::ecosystem::{Evidence, Tier, TIERS};
use pw_world::{PlayerStatus, World};

/// Games at a level before half of what they showed is believed.
const HALF_SAMPLE: f32 = 8.0;
/// A performance can never count for more than this in one game, however big.
const GAME_CAP: f32 = 1.0;
/// Two sightings closer than this are one look.
const LOOK_GAP_DAYS: i32 = 10;

fn conf(games: f32, half: f32) -> f32 {
    games / (games + half)
}

fn region_of(w: &World, p: PlayerId) -> RegionId {
    w.ext.ecosystem.story.get(&p).map_or(RegionId::NONE, |s| s.dev)
}

/// How real the opposition was: the state's competition quality, and a park pitch is not a stadium.
fn field_quality(w: &World, region: RegionId) -> f32 {
    let q = if region.is_some() { w.ext.ecosystem.assoc.get(&w.ext.ecosystem.state_of(region)).map_or(50.0, |a| a.comp_quality) } else { 50.0 };
    0.6 + 0.5 * q / 100.0
}

/// One game's worth of evidence. `rating` is on the usual 1–10 scale, `strength` the opposition against
/// the player's side (1 = level). A single enormous game is capped and only nudges a slow-moving average.
pub fn credit(w: &mut World, p: PlayerId, tier: Tier, rating: f32, strength: f32) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let region = region_of(w, p);
    let field = field_quality(w, region) * strength.clamp(0.5, 1.5);
    let perf = ((rating - 6.6) / 1.4).clamp(-1.5, 2.0) * field;
    let spike = perf > 1.3 * field;
    let counted = perf.min(GAME_CAP);
    let r = w.ext.ecosystem.repute.entry(p).or_default();
    let ev = &mut r.at[tier.ix()];
    let alpha = (1.0 / (ev.games + 4.0)).max(0.10);
    let dev = (counted - ev.steady).abs();
    ev.steady += alpha * (counted - ev.steady);
    ev.spread += 0.15 * (dev - ev.spread);
    ev.peak = ev.peak.max(perf);
    if spike && ev.games < 3.0 {
        r.spikes = r.spikes.saturating_add(1);
    }
    ev.games += 1.0;
    let (games, proof_now) = (ev.games, proof_of(ev));
    breakout(w, p, tier, spike, games, proof_now);
}

/// Talk spreads faster than truth. A single spectacular game can travel (a clip, a post) in a place where people
/// are online and the game was at a level that gets filmed; a long steady run gets noticed on its merits, more slowly.
/// Buzz makes scouts look; it adds nothing to what the player has actually shown, and fades within a year.
fn breakout(w: &mut World, p: PlayerId, tier: Tier, spike: bool, games: f32, proof_now: f32) {
    let region = region_of(w, p);
    let (prox, commercial) = if region.is_some() {
        let eco = &w.ext.ecosystem;
        (eco.regions[region].pro_proximity / 100.0, eco.assoc.get(&eco.state_of(region)).map_or(0.4, |a| a.commercial / 100.0))
    } else {
        (0.3, 0.3)
    };
    let media = 0.5 * prox + 0.5 * commercial;
    let key = [u64::from(p.0), games as u64, tier.ix() as u64, 0xb2e];
    let (chance, earned) = if spike && games < 4.0 {
        ((0.01 + 0.08 * media) * tier.weight().sqrt(), false)
    } else if games >= 10.0 && proof_now >= 0.45 {
        (0.03 * media * tier.weight().sqrt(), true)
    } else {
        return;
    };
    if w.roll(pw_core::rng::stream::YOUTH, &key) >= chance {
        return;
    }
    let r = w.ext.ecosystem.repute.entry(p).or_default();
    if r.buzz >= 30 {
        return;
    }
    r.buzz = (r.buzz + 40).min(100);
    let today = w.date;
    w.events.push(today, pw_world::event::Visibility::Public, pw_world::event::EventKind::Breakout { player: p, tier: tier.ix() as u8, earned });
}

/// What a level's evidence is worth in its own right (no level weighting): steady, believable, unbroken.
fn proof_of(ev: &Evidence) -> f32 {
    ev.steady.max(0.0) * conf(ev.games, HALF_SAMPLE) / (1.0 + 1.5 * ev.spread)
}

pub fn proof(w: &World, p: PlayerId, tier: Tier) -> f32 {
    w.ext.ecosystem.repute.get(&p).map_or(0.0, |r| proof_of(&r.at[tier.ix()]))
}

/// Games credited at a level (fades when unseen).
pub fn games(w: &World, p: PlayerId, tier: Tier) -> f32 {
    w.ext.ecosystem.repute.get(&p).map_or(0.0, |r| r.at[tier.ix()].games)
}

/// How far a name carries: level-weighted proof, 0–1. Perfect grassroots evidence alone tops out near 0.3.
pub fn standing(w: &World, p: PlayerId) -> f32 {
    let Some(r) = w.ext.ecosystem.repute.get(&p) else { return 0.0 };
    let sum: f32 = [Tier::Grassroots, Tier::School, Tier::District, Tier::Adult, Tier::Academy, Tier::State].iter().map(|&t| t.weight() * proof_of(&r.at[t.ix()])).sum();
    1.0 - (-3.0 * sum).exp()
}

/// The highest level at which a player has a real sample (at least three games).
pub fn best_tier(w: &World, p: PlayerId) -> Option<Tier> {
    let r = w.ext.ecosystem.repute.get(&p)?;
    [Tier::State, Tier::Academy, Tier::District, Tier::Adult, Tier::School, Tier::Grassroots].into_iter().find(|t| r.at[t.ix()].games >= 3.0)
}

/// Every aspect that decides whether a player is seen, for the record and for any screen that wants to explain it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Aspects {
    /// Highest level with a real sample, as a weight (0 if none).
    pub level: f32,
    /// Belief in what they have shown: games behind it.
    pub sample: f32,
    /// Steadiness: low spread is high.
    pub consistency: f32,
    /// How well they have done at their level.
    pub proof: f32,
    pub standing: f32,
    /// Current run of form, against par.
    pub form: f32,
    /// Share of their evidence that came from a single big game.
    pub spikiness: f32,
    /// Physical maturity as it reads to a watcher (early developers look better, late ones worse).
    pub maturity: f32,
    /// How visible their position makes them.
    pub position: f32,
    /// Someone vouches for them.
    pub sponsor: f32,
    pub sightings: f32,
    /// How much of their district's football anyone with influence watches.
    pub coverage: f32,
    /// How easy it is to get to trials and tournaments (cost, distance).
    pub access: f32,
    /// How thin the attention is: the district's young players per unit of scouting.
    pub crowding: f32,
    /// The coaching around them, which produces recommendations.
    pub network: f32,
}

pub fn aspects(w: &World, p: PlayerId) -> Aspects {
    let eco = &w.ext.ecosystem;
    let r = eco.repute.get(&p).copied().unwrap_or_default();
    let region = region_of(w, p);
    let reg = if region.is_some() { Some(&eco.regions[region]) } else { None };
    let tier = best_tier(w, p);
    let ev = tier.map_or(Evidence::default(), |t| r.at[t.ix()]);
    let total_games: f32 = r.at.iter().map(|e| e.games).sum();
    let hot = &w.players.hot[p];
    let cold = &w.players.cold[p];
    let crowd = reg.map_or(1.0, |rg| {
        let pool = eco.pools.get(&region).map_or(0.0, |x| x.part.iter().sum::<f32>());
        (pool / (2_000.0 + 400.0 * rg.scouting_coverage)).clamp(0.2, 8.0)
    });
    Aspects {
        level: tier.map_or(0.0, Tier::weight),
        sample: conf(total_games, HALF_SAMPLE),
        consistency: 1.0 / (1.0 + 1.5 * ev.spread),
        proof: proof_of(&ev),
        standing: standing(w, p),
        form: hot.form_avg().map_or(0.0, |f| ((f - 6.6) / 1.5).clamp(-1.0, 1.5)),
        spikiness: if total_games > 0.0 { f32::from(r.spikes) / total_games.max(1.0) } else { 0.0 },
        maturity: (-0.04 * f32::from(cold.bio_offset)).clamp(-0.6, 0.6),
        position: match cold.best_pos.group() {
            PosGroup::Att => 0.3,
            PosGroup::Mid => 0.1,
            PosGroup::Def => -0.15,
            PosGroup::Gk => -0.3,
        },
        sponsor: f32::from(r.sponsor.min(3)),
        sightings: f32::from(r.sightings),
        coverage: reg.map_or(0.5, |rg| rg.scouting_coverage / 100.0),
        access: reg.map_or(0.5, |rg| (0.4 * rg.economic_access + 0.3 * rg.pro_proximity + 0.3 * rg.facilities) / 100.0),
        crowding: crowd,
        network: reg.map_or(0.5, |rg| rg.coach_density / 100.0),
    }
}

/// The chance a scout at a game of this level singles this player out, among the others playing.
/// Merit is only part of it: a hot week, a position, being young for the level and someone vouching
/// help; a single spike, a crowded district and a hard-to-reach place hurt.
pub fn attention(w: &World, p: PlayerId, tier: Tier) -> f32 {
    let a = aspects(w, p);
    let r = w.ext.ecosystem.repute.get(&p).copied().unwrap_or_default();
    let ev = r.at[tier.ix()];
    let merit = ev.steady.max(0.0) * conf(ev.games, 3.0);
    let mut x = -2.9 + 2.6 * merit + 0.5 * a.form + a.maturity + a.position + 0.35 * a.sponsor;
    if r.sightings > 0 {
        x += 0.4;
    }
    // Talk gets a scout to look, and a scout who has heard of a spike still discounts it (below).
    x += 0.012 * f32::from(r.buzz);
    // One big game among few is not a pattern, and scouts know it.
    if ev.games < 4.0 && ev.peak > 1.0 {
        x -= 0.5;
    }
    x -= 0.35 * (a.crowding - 1.0).max(0.0).min(3.0);
    x += 0.6 * (a.access - 0.5);
    1.0 / (1.0 + (-x).exp())
}

/// Someone who could act has actually watched this player play. Counts once per look, not per minute.
pub fn sighted(w: &mut World, p: PlayerId) {
    let today = w.date;
    let r = w.ext.ecosystem.repute.entry(p).or_default();
    if r.last_sight.days_until(today) < LOOK_GAP_DAYS && r.sightings > 0 {
        return;
    }
    if r.last_sight.year() != today.year() || r.sightings == 0 {
        r.sight_years = r.sight_years.saturating_add(1);
    }
    r.sightings = r.sightings.saturating_add(1);
    r.last_sight = today;
}

/// How much an academy needs before it will invite a child it has not seen at its own level.
fn need(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let rep = f32::from(w.clubs[club].reputation) / 10_000.0;
    let feeder = w.youth.academies.get(&club).is_some_and(|a| w.youth.member_of.get(&p).is_some_and(|l| a.feeders.contains(l)));
    let base = 0.05 + 0.34 * rep * rep + 0.14 * rep;
    if feeder { base * 0.6 } else { base }
}

/// Has this academy got enough on the child to invite them? Worlds without an ecosystem keep the old rule.
pub fn recognised_by(w: &World, club: ClubId, p: PlayerId) -> bool {
    if !w.ext.ecosystem.is_configured() {
        return true;
    }
    let Some(r) = w.ext.ecosystem.repute.get(&p) else { return false };
    let total: f32 = r.at.iter().map(|e| e.games).sum();
    let rep = w.clubs[club].reputation;
    // Established academies want to have seen a child more than twice, on different days.
    let looks = if rep >= 4500 { 3 } else { 2 };
    if total < 6.0 || r.sightings < looks {
        return false;
    }
    // The big academies want proof above the park: school, district, adult or state football.
    if rep >= 6500 && !best_tier(w, p).is_some_and(|t| t != Tier::Grassroots) {
        return false;
    }
    if rep >= 8500 && !best_tier(w, p).is_some_and(|t| matches!(t, Tier::District | Tier::State | Tier::Academy | Tier::Adult)) {
        return false;
    }
    standing(w, p) >= need(w, club, p)
}

/// Yearly: unseen evidence fades, coaches and teachers start to vouch for the ones who kept showing it.
pub fn yearly(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let mut ids: Vec<PlayerId> = w.ext.ecosystem.repute.keys().copied().collect();
    ids.sort();
    let year = w.date.year() as u64;
    for p in ids {
        if w.players.hot[p].status == PlayerStatus::Retired {
            w.ext.ecosystem.repute.remove(&p);
            continue;
        }
        let region = region_of(w, p);
        let net = if region.is_some() { w.ext.ecosystem.regions[region].coach_density / 100.0 } else { 0.4 };
        let vouch = w.roll(pw_core::rng::stream::YOUTH, &[u64::from(p.0), year, 0x5b0]) < 0.15 + 0.5 * net;
        let strong = proof(w, p, Tier::Grassroots) >= 0.35 || proof(w, p, Tier::School) >= 0.35 || proof(w, p, Tier::District) >= 0.35;
        let r = w.ext.ecosystem.repute.get_mut(&p).unwrap();
        if strong && vouch {
            r.sponsor = (r.sponsor + 1).min(3);
        }
        let mut empty = true;
        for e in &mut r.at {
            e.games *= 0.75;
            e.peak *= 0.7;
            e.steady *= 0.92;
            empty &= e.games < 0.5;
        }
        r.spikes = r.spikes.saturating_sub(1);
        r.buzz = (f32::from(r.buzz) * 0.4) as u8;
        if empty && r.sightings == 0 {
            w.ext.ecosystem.repute.remove(&p);
        }
    }
}

/// A short summary of the tiers with evidence, for lists.
pub fn summary(w: &World, p: PlayerId) -> Vec<(Tier, f32, f32)> {
    let Some(r) = w.ext.ecosystem.repute.get(&p) else { return Vec::new() };
    (0..TIERS)
        .filter(|&i| r.at[i].games >= 1.0)
        .map(|i| {
            let t = [Tier::Grassroots, Tier::School, Tier::District, Tier::Adult, Tier::Academy, Tier::State][i];
            (t, r.at[i].games, proof_of(&r.at[i]))
        })
        .collect()
}
