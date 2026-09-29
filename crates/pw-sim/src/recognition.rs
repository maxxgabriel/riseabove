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

use pw_core::{ClubId, PersonId, PlayerId, PosGroup, RegionId};
use pw_world::ecosystem::{Evidence, Tier, TIERS};
use pw_world::recog::{Acquaintance, Learned, Org, Referrals, Source, Vouch, VouchBasis, Watch};
use pw_world::scenario::RecognitionTuning;
use pw_world::{PlayerStatus, World};

/// The scenario's tuning: tier weights, sample sizes and thresholds are initial values that live in the pack (`Scenario`), not here.
fn tune(w: &World) -> &RecognitionTuning {
    &w.ext.scenario.recognition
}

/// What evidence at a level is worth to people deciding who to look at next.
pub fn weight(w: &World, t: Tier) -> f32 {
    tune(w).tier_weight[t.ix()]
}

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
    let counted = perf.min(tune(w).game_cap);
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
    let (games, seen) = (ev.games, *ev);
    let proof_now = proof_of(w, &seen);
    breakout(w, p, tier, spike, games, proof_now);
    watched_game(w, p);
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
        ((0.01 + 0.08 * media) * weight(w, tier).sqrt(), false)
    } else if games >= 10.0 && proof_now >= 0.45 {
        (0.03 * media * weight(w, tier).sqrt(), true)
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
    open_watches(w, p);
}

/// Most academies that will send someone because of talk about one player.
const MAX_WATCHERS: usize = 3;
/// Games a sent scout watches before any judgement forms.
const WATCH_GAMES: u8 = 3;
/// Looks that were set going and never finished are dropped after this long.
const WATCH_EXPIRES_DAYS: i32 = 150;

/// Talk sends people to look; it does not tell them what they will see. Each academy that hears (near enough, and with a scout to
/// send) starts a watch: some games are watched, and only then does a judgement form (`watched_game`). The buzz itself never
/// enters the judgement.
fn open_watches(w: &mut World, p: PlayerId) {
    let today = w.date;
    let region = region_of(w, p);
    let mut clubs: Vec<ClubId> = w.youth.academies.keys().copied().collect();
    clubs.sort();
    let mut heard: Vec<(f32, ClubId)> = Vec::new();
    for club in clubs {
        if w.ext.recog.watching.iter().any(|x| x.org == Org::Club(club) && x.player == p) {
            continue;
        }
        let reach = w.youth.academies[&club].reach;
        let near = region.is_none()
            || w.ext.ecosystem.travel_burden(w.ext.ecosystem.region_of_club(club), region) < 0.3
            || matches!(reach, pw_world::youth::Reach::National | pw_world::youth::Reach::International);
        if !near {
            continue;
        }
        let rep = f32::from(w.clubs[club].reputation) / 10_000.0;
        let roll = w.roll(pw_core::rng::stream::YOUTH, &[u64::from(club.0), u64::from(p.0), today.0 as u64, 0xb3f]);
        // A bigger name hears of more, and is more likely to send someone.
        if roll < 0.15 + 0.45 * rep {
            heard.push((roll, club));
        }
    }
    heard.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, club) in heard.into_iter().take(MAX_WATCHERS) {
        let Some(scout) = crate::ecosystem::scout_of(w, club) else { continue };
        let by = w.staff[scout].person;
        w.ext.recog.watching.push(Watch { org: Org::Club(club), player: p, by, left: WATCH_GAMES, since: today });
        // The academy now knows the name and no more.
        let a = w.ext.recog.acquaint.entry((Org::Club(club), p)).or_insert(Acquaintance { sightings: 0, years: 0, first: today, last: today, first_by: by, how: Learned::Buzz });
        a.last = today;
    }
}

/// A player who has people sent to watch him has just played: one game watched. When the last is done, the scout forms a view the
/// way any scout does, from what he saw of the play (`scouting::judge`), and files it.
fn watched_game(w: &mut World, p: PlayerId) {
    if w.ext.recog.watching.is_empty() {
        return;
    }
    let today = w.date;
    let mut finished: Vec<Watch> = Vec::new();
    for x in w.ext.recog.watching.iter_mut().filter(|x| x.player == p && x.left > 0) {
        x.left -= 1;
        if x.left == 0 {
            finished.push(*x);
        }
    }
    for x in finished {
        let Org::Club(club) = x.org else { continue };
        sighted_by(w, x.org, x.by, p, Learned::Watched);
        w.knowledge.observe(club, p, 90 * u16::from(WATCH_GAMES), today);
        if let Some(scout) = crate::ecosystem::scout_of(w, club)
            && w.scouting.profiles.contains_key(&scout)
        {
            let r = crate::scouting::judge(w, scout, club, p, 300);
            w.scouting.file(club, p, r);
        }
    }
    w.ext.recog.watching.retain(|x| x.left > 0);
}

/// Monthly: watches that were never finished (the player stopped playing, the scout left) lapse.
pub fn lapse_watches(w: &mut World) {
    let today = w.date;
    w.ext.recog.watching.retain(|x| x.since.days_until(today) <= WATCH_EXPIRES_DAYS);
}

/// What a level's evidence is worth in its own right (no level weighting): steady, believable, unbroken.
fn proof_of(w: &World, ev: &Evidence) -> f32 {
    ev.steady.max(0.0) * conf(ev.games, tune(w).half_sample) / (1.0 + 1.5 * ev.spread)
}

pub fn proof(w: &World, p: PlayerId, tier: Tier) -> f32 {
    w.ext.ecosystem.repute.get(&p).map_or(0.0, |r| proof_of(w, &r.at[tier.ix()]))
}

/// Games credited at a level (fades when unseen).
pub fn games(w: &World, p: PlayerId, tier: Tier) -> f32 {
    w.ext.ecosystem.repute.get(&p).map_or(0.0, |r| r.at[tier.ix()].games)
}

/// How far a name carries: level-weighted proof, 0–1. Perfect grassroots evidence alone tops out near 0.3.
pub fn standing(w: &World, p: PlayerId) -> f32 {
    let Some(r) = w.ext.ecosystem.repute.get(&p) else { return 0.0 };
    let sum: f32 = [Tier::Grassroots, Tier::School, Tier::District, Tier::Adult, Tier::Academy, Tier::State].iter().map(|&t| weight(w, t) * proof_of(w, &r.at[t.ix()])).sum();
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
        level: tier.map_or(0.0, |t| weight(w, t)),
        sample: conf(total_games, tune(w).half_sample),
        consistency: 1.0 / (1.0 + 1.5 * ev.spread),
        proof: proof_of(w, &ev),
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
        sponsor: 3.0 * vouch_weight(w, None, p),
        sightings: f32::from(r.sightings),
        coverage: reg.map_or(0.5, |rg| rg.scouting_coverage / 100.0),
        access: reg.map_or(0.5, |rg| (0.4 * rg.economic_access + 0.3 * rg.pro_proximity + 0.3 * rg.facilities) / 100.0),
        crowding: crowd,
        network: reg.map_or(0.5, |rg| rg.coach_density / 100.0),
    }
}

/// How much a scout gives what a coach said about this child, 0-1. It is the recommender's own rating of him against the children
/// they coach (`strength`), how far their word counts (`credibility`), and how far *this organisation* trusts that source: a prior,
/// moved by how its past recommendations turned out for it. With no organisation, the prior alone.
pub fn vouch_weight(w: &World, org: Option<Org>, p: PlayerId) -> f32 {
    let Some(v) = w.ext.recog.vouch.get(&p) else { return 0.0 };
    let t = tune(w);
    let record = org.and_then(|o| w.ext.recog.referrals.get(&(o, v.from)).copied()).unwrap_or_default().record();
    let trust = (t.vouch_prior_trust + t.vouch_record_weight * record).clamp(0.0, 1.0);
    // An old save knew only that someone vouched: it counts for half.
    let basis = if v.basis == VouchBasis::Legacy { 0.5 } else { 1.0 };
    (v.strength * v.credibility * trust * 2.0 * basis).clamp(0.0, 1.0)
}

/// The chance a scout at a game of this level singles this player out, among the others playing.
/// Merit is only part of it: a hot week, a position, being young for the level and someone vouching
/// help; a single spike, a crowded district and a hard-to-reach place hurt.
pub fn attention(w: &World, p: PlayerId, tier: Tier) -> f32 {
    attention_for(w, None, p, tier)
}

/// As `attention`, for one organisation's scout: how much the child's coach is believed is that organisation's own call.
pub fn attention_for(w: &World, org: Option<Org>, p: PlayerId, tier: Tier) -> f32 {
    let a = aspects(w, p);
    let t = tune(w);
    let r = w.ext.ecosystem.repute.get(&p).copied().unwrap_or_default();
    let ev = r.at[tier.ix()];
    let merit = ev.steady.max(0.0) * conf(ev.games, 3.0);
    let sponsor = 3.0 * vouch_weight(w, org, p);
    let known = org.and_then(|o| w.ext.recog.known(o, p)).map_or(r.sightings > 0, |k| k.sightings > 0);
    let mut x = t.attention_base + t.attention_merit * merit + 0.5 * a.form + a.maturity + a.position + 0.35 * sponsor;
    if known {
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

/// One of an organisation's people has actually watched this player play. Counts once per look, not per minute, and is recorded twice:
/// as what that organisation now knows, and (for the world at large) that someone who could act has seen him.
pub fn sighted_by(w: &mut World, org: Org, by: PersonId, p: PlayerId, how: Learned) -> bool {
    let today = w.date;
    let gap = tune(w).look_gap_days;
    {
        let r = w.ext.ecosystem.repute.entry(p).or_default();
        if !(r.last_sight.days_until(today) < gap && r.sightings > 0) {
            if r.last_sight.year() != today.year() || r.sightings == 0 {
                r.sight_years = r.sight_years.saturating_add(1);
            }
            r.sightings = r.sightings.saturating_add(1);
            r.last_sight = today;
        }
    }
    let a = w.ext.recog.acquaint.entry((org, p)).or_insert(Acquaintance { sightings: 0, years: 0, first: today, last: today, first_by: by, how });
    if a.how == Learned::Buzz && how != Learned::Buzz {
        a.how = how;
    }
    if a.sightings > 0 && a.last.days_until(today) < gap {
        return false;
    }
    if a.sightings == 0 || a.last.year() != today.year() {
        a.years = a.years.saturating_add(1);
    }
    if a.sightings == 0 {
        a.first = today;
        a.first_by = by;
    }
    a.sightings = a.sightings.saturating_add(1);
    a.last = today;
    true
}

/// How many distinct looks this organisation's people have had of the player.
pub fn looks_by(w: &World, org: Org, p: PlayerId) -> u8 {
    w.ext.recog.known(org, p).map_or(0, |a| a.sightings)
}

/// How much an academy needs before it will invite a child it has not seen at its own level.
fn need(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let t = tune(w);
    let rep = f32::from(w.clubs[club].reputation) / 10_000.0;
    let feeder = w.youth.academies.get(&club).is_some_and(|a| w.youth.member_of.get(&p).is_some_and(|l| a.feeders.contains(l)));
    let base = t.need_base + t.need_sq * rep * rep + t.need_lin * rep;
    if feeder { base * t.feeder_discount } else { base }
}

/// Has this academy got enough on the child to invite him? It knows him through its own people: the looks *its* scouts have had,
/// plus a coach's recommendation it believes. Worlds without an ecosystem keep the old rule.
pub fn recognised_by(w: &World, club: ClubId, p: PlayerId) -> bool {
    if !w.ext.ecosystem.is_configured() {
        return true;
    }
    let Some(r) = w.ext.ecosystem.repute.get(&p) else { return false };
    let t = tune(w);
    let org = Org::Club(club);
    let total: f32 = r.at.iter().map(|e| e.games).sum();
    let rep = w.clubs[club].reputation;
    // Established academies want to have seen a child more than twice, on different days.
    let mut looks = if rep >= t.established_rep { t.looks_established } else { t.looks_other };
    // A recommendation the academy believes stands for one look; it is never the whole case.
    if vouch_weight(w, Some(org), p) >= 0.45 {
        looks = looks.saturating_sub(1).max(1);
    }
    if total < t.min_games || looks_by(w, org, p) < looks {
        return false;
    }
    // The big academies want proof above the park: school, district, adult or state football.
    if rep >= t.gate_above_park && !best_tier(w, p).is_some_and(|t| t != Tier::Grassroots) {
        return false;
    }
    if rep >= t.gate_above_school && !best_tier(w, p).is_some_and(|t| matches!(t, Tier::District | Tier::State | Tier::Academy | Tier::Adult)) {
        return false;
    }
    standing(w, p) >= need(w, club, p)
}

/// A recommendation was followed by an organisation's decision, and it went well or badly: the organisation will weigh that
/// source's next recommendation accordingly.
pub fn referral_outcome(w: &mut World, org: Org, p: PlayerId, hit: bool) {
    let Some(from) = w.ext.recog.vouch.get(&p).map(|v| v.from) else { return };
    let e: &mut Referrals = w.ext.recog.referrals.entry((org, from)).or_default();
    if hit {
        e.hits = e.hits.saturating_add(1);
    } else {
        e.misses = e.misses.saturating_add(1);
    }
}

/// Who coaches this child, and how good a judge that place is (0-1): his school or university, his local club, or the district's
/// selectors when nothing closer knows him. None when nobody has a relationship with him.
fn coach_of(w: &World, p: PlayerId) -> Option<(Source, f32, VouchBasis)> {
    let region = region_of(w, p);
    let density = if region.is_some() { w.ext.ecosystem.regions[region].coach_density / 100.0 } else { 0.4 };
    if let Some(&i) = w.minor.member_of.get(&p) {
        let q = f32::from(w.minor.institutions[i as usize].coaching) / 20.0;
        return Some((Source::Institution(i), 0.5 * q + 0.5 * density, VouchBasis::Trained { months: 12 }));
    }
    if let Some(&l) = w.youth.member_of.get(&p) {
        return Some((Source::Club(l), density, VouchBasis::Trained { months: 12 }));
    }
    let seen = games(w, p, Tier::District) + games(w, p, Tier::School);
    if region.is_some() && seen >= 4.0 {
        return Some((Source::District(region), 0.6 * density + 0.2, VouchBasis::Watched { games: seen.round() as u16 }));
    }
    None
}

/// Vouches older than this lapse unless the person has kept earning them.
const VOUCH_LIFE_DAYS: i32 = 2 * 365;

/// Yearly: coaches and teachers recommend the children they have coached or watched and rate clearly above the rest of their group,
/// and unseen evidence fades. A recommendation has a cause (the relationship and how many months or games it rests on), a strength
/// (how far above the others he stands *among those the coach knows*, not in the country) and a credibility (the coaching around
/// him, and how past recommendations from that source have turned out). Nothing is random: the same children rise the same way.
pub fn yearly(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let today = w.date;
    let mut ids: Vec<PlayerId> = w.ext.ecosystem.repute.keys().copied().collect();
    ids.sort();
    // How each source's recommendations have gone, across every organisation that acted on them.
    let mut record: pw_world::FxHashMap<Source, Referrals> = Default::default();
    for (&(_, src), r) in &w.ext.recog.referrals {
        let e = record.entry(src).or_default();
        e.hits = e.hits.saturating_add(r.hits);
        e.misses = e.misses.saturating_add(r.misses);
    }
    // Each child against the others his coach has.
    let mut groups: pw_world::FxHashMap<Source, Vec<(f32, PlayerId, f32, VouchBasis)>> = Default::default();
    for &p in &ids {
        if w.players.hot[p].status == PlayerStatus::Retired {
            continue;
        }
        let Some((src, quality, basis)) = coach_of(w, p) else { continue };
        let score = proof(w, p, Tier::Grassroots) + proof(w, p, Tier::School) + proof(w, p, Tier::District);
        groups.entry(src).or_default().push((score, p, quality, basis));
    }
    let mut sources: Vec<Source> = groups.keys().copied().collect();
    sources.sort();
    let mut new: Vec<(PlayerId, Vouch)> = Vec::new();
    for src in sources {
        let g = groups.get_mut(&src).unwrap();
        if g.len() < 3 {
            continue;
        }
        g.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let n = g.len();
        let rec = record.get(&src).copied().unwrap_or_default().record();
        for (i, &(score, p, quality, basis)) in g.iter().enumerate() {
            let pct = i as f32 / (n - 1) as f32;
            if pct < 0.75 || score < 0.35 {
                continue;
            }
            let credibility = (0.15 + 0.45 * quality + 0.4 * (0.5 + 0.5 * rec)).clamp(0.05, 1.0);
            new.push((p, Vouch { from: src, basis, strength: pct, credibility, date: today }));
        }
    }
    for (p, v) in new {
        w.ext.recog.vouch.insert(p, v);
    }
    for p in ids {
        if w.players.hot[p].status == PlayerStatus::Retired {
            w.ext.ecosystem.repute.remove(&p);
            w.ext.recog.vouch.remove(&p);
            continue;
        }
        if w.ext.recog.vouch.get(&p).is_some_and(|v| v.date.days_until(today) > VOUCH_LIFE_DAYS) {
            w.ext.recog.vouch.remove(&p);
        }
        // The count kept for older readers is what the recommendation is worth, not a random draw.
        let sponsor = (3.0 * vouch_weight(w, None, p)).round().clamp(0.0, 3.0) as u8;
        let r = w.ext.ecosystem.repute.get_mut(&p).unwrap();
        r.sponsor = sponsor;
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
    // What an organisation knew of a player who has retired, or last saw years ago, is dropped.
    let hot = &w.players.hot;
    w.ext.recog.acquaint.retain(|&(_, p), a| a.last.days_until(today) <= 5 * 365 && hot[p].status != PlayerStatus::Retired);
    w.ext.recog.vouch.retain(|p, _| hot[*p].status != PlayerStatus::Retired);
}

/// A short summary of the tiers with evidence, for lists.
pub fn summary(w: &World, p: PlayerId) -> Vec<(Tier, f32, f32)> {
    let Some(r) = w.ext.ecosystem.repute.get(&p) else { return Vec::new() };
    (0..TIERS)
        .filter(|&i| r.at[i].games >= 1.0)
        .map(|i| {
            let t = [Tier::Grassroots, Tier::School, Tier::District, Tier::Adult, Tier::Academy, Tier::State][i];
            (t, r.at[i].games, proof_of(w, &r.at[i]))
        })
        .collect()
}
