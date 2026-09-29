//! How the world abroad sees the country's players, by market and by kind of football (the India brief, items 17 and 18).
//!
//! There is no single "export reputation". A market is a group of nations whose clubs look at the same things the same way
//! (`Scenario::markets`, from the pack); each holds a separate regard for the country's youth football, its senior national side,
//! its domestic league and its university football. A market watches only what it has reason to: not every club abroad looks at
//! this country, and one that does is looking at one kind of football, through its own scouts.
//!
//! Regard is earned. It follows how the players who went to that market's clubs have done there, slowly, and a few good exports
//! open the door for the next. A scenario may seed it; a seed is labelled as one and nothing else moves it up.

use pw_core::rng::stream;
use pw_core::{ClubId, PlayerId};
use pw_world::ecosystem::{StageKind, Tier};
use pw_world::recog::{Learned, Org, Regard, Segment};
use pw_world::{PlayerStatus, World};

/// Regard of a segment as a share of a market's stated starting reputation: youth football and senior football are seen a little less
/// than the league, and university football is nearly invisible abroad.
const SEED_SHARE: [(Segment, f32); 4] = [(Segment::Youth, 0.7), (Segment::Senior, 0.8), (Segment::League, 1.0), (Segment::University, 0.25)];

/// The market a club belongs to: the first whose nations include the club's own. A scenario with no markets has one anonymous
/// market that every foreign club belongs to. None when the club's nation is in no market (its clubs do not look here).
pub fn market_of(w: &World, club: ClubId) -> Option<u8> {
    let m = &w.ext.scenario.markets;
    if m.is_empty() {
        return Some(0);
    }
    let code = &w.nations[w.clubs[club].nation].code;
    m.iter().position(|d| d.nations.iter().any(|n| n == code)).map(|i| i as u8)
}

fn market_count(w: &World) -> u8 {
    w.ext.scenario.markets.len().max(1) as u8
}

/// A market's name for screens.
pub fn market_name(w: &World, m: u8) -> String {
    w.ext.scenario.markets.get(usize::from(m)).map_or_else(|| "abroad".to_string(), |d| d.key.clone())
}

/// World start: a scenario's seed for each market and segment, where nothing has been earned yet.
pub fn seed(w: &mut World) {
    for m in 0..market_count(w) {
        let start = w.ext.scenario.markets.get(usize::from(m)).map_or(12.0, |d| d.start);
        for (seg, share) in SEED_SHARE {
            w.ext.recog.export.entry((m, seg)).or_insert(Regard { level: start * share, ..Regard::default() });
        }
    }
    summary(w);
}

/// The one number older readers keep: the mean regard for the domestic league across markets.
fn summary(w: &mut World) {
    let n = market_count(w);
    let sum: f32 = (0..n).map(|m| w.ext.recog.regard(m, Segment::League)).sum();
    w.ext.ecosystem.export = sum / f32::from(n);
}

/// Which kind of the country's football an export came out of. Read from the route he took, and from age and caps.
fn segment_of(w: &World, p: PlayerId) -> Segment {
    let stages = w.ext.ecosystem.stages.get(&p);
    let has = |k: StageKind| stages.is_some_and(|v| v.iter().any(|s| s.kind == k));
    if w.players.cold[p].caps >= 3 {
        Segment::Senior
    } else if has(StageKind::University) {
        Segment::University
    } else if has(StageKind::Academy) || w.age(p) <= 21 {
        Segment::Youth
    } else {
        Segment::League
    }
}

/// Yearly: each market's regard for each kind of the country's football follows how the players it took from that kind are doing.
pub fn yearly(w: &mut World) {
    let Some(home) = w.ext.ecosystem.regions.iter().next().map(|r| r.nation) else { return };
    let markets = market_count(w);
    // (exports, successes, good) per market and segment.
    let mut tally: pw_world::FxHashMap<(u8, Segment), (u16, u16, f32)> = Default::default();
    for (p, h) in w.players.hot.iter_enumerated() {
        if h.status == PlayerStatus::Retired || h.club.is_none() || w.clubs[h.club].nation == home || w.people[w.players.cold[p].person].nation != home {
            continue;
        }
        let Some(m) = market_of(w, h.club) else { continue };
        let seg = segment_of(w, p);
        // How good he looks to the world: the market's reading of him, not his hidden ability.
        let good = ((crate::market::public_view(w, p).0 - 90.0) / 40.0).max(0.0);
        let e = tally.entry((m, seg)).or_default();
        e.0 = e.0.saturating_add(1);
        e.1 = e.1.saturating_add(u16::from(h.minutes_4w >= 200));
        e.2 += good;
    }
    for m in 0..markets {
        for seg in Segment::ALL {
            let (exports, successes, good) = tally.get(&(m, seg)).copied().unwrap_or_default();
            // University football is seen by almost no one abroad: what a market makes of it comes only from the ones it took.
            let ceiling = if seg == Segment::University { 45.0 } else { 75.0 };
            let target = 10.0 + (ceiling - 10.0) * (1.0 - (-good / 8.0).exp());
            let r = w.ext.recog.export.entry((m, seg)).or_default();
            r.level = if r.level <= 0.0 { 12.0 } else { r.level + 0.15 * (target - r.level) };
            r.exports = exports;
            r.successes = successes;
            r.legacy = false;
        }
    }
    summary(w);
}

/// Clubs abroad send people to the events where the country's players are on show (the state championship, the national camps).
/// Whether a market's clubs come depends on that market's regard for *this kind of football*; whom they notice depends on each
/// player's evidence at that level. A foreign scout at a state game is looking at a handful of players, and a market that does not
/// rate what is on show sends almost no one.
pub fn eyes(w: &mut World, seg: Segment, tier: Tier, pool: &[PlayerId]) {
    let Some(home) = w.ext.ecosystem.regions.iter().next().map(|r| r.nation) else { return };
    let t = w.ext.scenario.scouting.clone();
    let year = w.date.year() as u64;
    let today = w.date;
    let mut clubs: Vec<ClubId> = w.clubs.iter_enumerated().filter(|(_, c)| c.nation != home && c.reputation >= t.foreign_min_rep).map(|(id, _)| id).collect();
    clubs.sort();
    for club in clubs {
        let Some(m) = market_of(w, club) else { continue };
        let Some(scout) = crate::ecosystem::scout_of(w, club) else { continue };
        let regard = w.ext.recog.regard(m, seg).max(8.0) / 100.0;
        let rep = f32::from(w.clubs[club].reputation) / 10_000.0;
        if w.roll(stream::INTL, &[u64::from(club.0), year, tier.ix() as u64, 0xf0e]) >= t.foreign_base + t.foreign_share * regard * (1.0 - 0.5 * rep) {
            continue;
        }
        if let Some(r) = w.ext.recog.export.get_mut(&(m, seg)) {
            r.visits = r.visits.saturating_add(1);
        }
        let by = w.staff[scout].person;
        let mut cands: Vec<(f32, PlayerId)> = pool.iter().copied().filter(|&p| w.age(p) >= 16).map(|p| (crate::recognition::attention_for(w, Some(Org::Market(m)), p, tier), p)).collect();
        cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for (a, p) in cands.into_iter().take(usize::from(t.looks_per_visit)) {
            if w.roll(stream::INTL, &[u64::from(club.0), u64::from(p.0), year, 0xf0f]) >= a {
                continue;
            }
            w.knowledge.observe(club, p, 90, today);
            crate::recognition::sighted_by(w, Org::Market(m), by, p, Learned::Watched);
            crate::recognition::sighted_by(w, Org::Club(club), by, p, Learned::Watched);
            let r = crate::scouting::judge(w, scout, club, p, 300);
            w.scouting.file(club, p, r);
            if let Some(rep) = w.ext.ecosystem.repute.get_mut(&p) {
                rep.foreign = rep.foreign.saturating_add(1);
            }
            w.ext.ecosystem.foreign_looks += 1;
        }
    }
}
