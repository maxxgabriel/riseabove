//! The football ecosystem around a nation's professional game (see
//! `pw_world::ecosystem`): regions and associations that move slowly,
//! aggregate participation pools, and the moment a child becomes a person.
//!
//! Timescales: regions and associations change yearly (infrastructure follows
//! investment with a lag of years); pools advance yearly; prospects are drawn
//! from a district's pool when they become competitive or noticed. A child in
//! the mass of participants costs nothing; a prospect is an ordinary player
//! with a recorded origin, home and route.
//!
//! Nothing here is a permanent bonus for a place. Strength comes from
//! participation, coaches, facilities, competition and scouting, all of which
//! move, and from what the region has produced lately (its culture).

use pw_core::rng::stream;
use pw_core::{ClubId, LocalClubId, PersonId, PlayerId, RegionId, Rng};
use pw_world::ecosystem::{Pool, PlayerStory, Provider, RegionKind, Stage, StageKind, POOL_FIRST_AGE};
use pw_world::event::{EventKind, Visibility};
use pw_world::player::PlayerSource;
use pw_world::{Contract, PlayerStatus, World};
use smallvec::SmallVec;

use crate::people::{NewPlayer, spawn_player};

/// Share of a district's participants who are drawn out as visible prospects each year, before coverage.
const PROSPECT_RATE: f32 = 0.0007;
/// Share of leavers from the mass pool who enter organised adult football late, before coverage.
const LATE_RATE: f32 = 0.00015;
/// Age at which a child in the pool becomes a person.
const MATERIALISE_AGE: usize = 11;

fn nudge(x: &mut f32, target: f32, rate: f32) {
    *x = (*x + rate * (target - *x)).clamp(2.0, 98.0);
}

/// Record a step on a player's route (no-op in worlds without an ecosystem) and announce it to them.
pub fn note(w: &mut World, p: PlayerId, kind: StageKind, target: u32) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let today = w.date;
    let region = w.ext.ecosystem.story.get(&p).map_or(RegionId::NONE, |s| s.dev);
    let v = w.ext.ecosystem.stages.entry(p).or_default();
    if v.len() >= 24 {
        v.remove(0);
    }
    v.push(Stage { date: today, kind, target, region });
    let who = w.players.cold[p].person;
    w.events.push(today, Visibility::Person(who), EventKind::PathwayStep { player: p, kind: kind.code(), target });
}

/// The first person outside the family to take a player seriously. Only the first counts.
pub fn note_found(w: &mut World, p: PlayerId, by: PersonId, club: ClubId) {
    let today = w.date;
    if let Some(s) = w.ext.ecosystem.story.get_mut(&p)
        && s.found_by.is_none()
    {
        s.found_by = by;
        s.found_club = club;
        s.found_on = today;
    }
}

/// Move where a player is developing (a school, an academy, a university elsewhere).
pub fn set_dev_region(w: &mut World, p: PlayerId, region: RegionId) {
    if region.is_some()
        && let Some(s) = w.ext.ecosystem.story.get_mut(&p)
    {
        s.dev = region;
    }
}

fn poisson(rng: &mut Rng, lambda: f32) -> u32 {
    if lambda <= 0.0 {
        return 0;
    }
    // Normal approximation once large; Knuth's method below that.
    if lambda > 30.0 {
        return (lambda + rng.normal() * lambda.sqrt()).round().max(0.0) as u32;
    }
    let l = (-lambda).exp();
    let (mut k, mut p) = (0u32, 1.0f32);
    loop {
        p *= rng.f32();
        if p <= l {
            return k;
        }
        k += 1;
    }
}

fn retain(age: usize, coach: f32, comp: f32, econ: f32) -> f32 {
    let drop_out = if age >= 13 { 0.08 * (1.0 - econ / 100.0) } else { 0.0 };
    (0.90 + 0.06 * coach / 100.0 + 0.05 * comp / 100.0 - drop_out).clamp(0.6, 0.99)
}

/// Fill a district's pool at world start as if the conditions had always held.
fn seed_pool(pool: &mut Pool, entrants: f32, coach: f32, comp: f32, econ: f32) {
    let mut x = entrants;
    for i in 0..pool.part.len() {
        pool.part[i] = x;
        x *= retain(POOL_FIRST_AGE + i, coach, comp, econ);
    }
}

/// July 1: regions, associations and pools move on a year; new prospects appear.
pub fn yearly(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let year = w.date.year();
    if w.ext.ecosystem.last_year >= year && w.ext.ecosystem.pools.values().any(|p| p.part[0] > 0.0) {
        return;
    }
    let first = w.ext.ecosystem.last_year >= year;
    w.ext.ecosystem.last_year = year;
    if !first {
        associations(w, year);
        regions(w, year);
    }
    pools(w, year, first);
    if first {
        // Children who exist at the start go to school in their district straight away.
        crate::life::sync(w);
        crate::minor::assign(w);
    }
}

fn associations(w: &mut World, year: i32) {
    let mut keys: Vec<RegionId> = w.ext.ecosystem.assoc.keys().copied().collect();
    keys.sort();
    for r in keys {
        let mut rng = Rng::keyed(&[w.seed, stream::WORLDGEN, 0xa550, u64::from(r.0), year as u64]);
        let a = w.ext.ecosystem.assoc.get_mut(&r).unwrap();
        // Governance drifts; a badly run association can lose ground fast, a well run one compounds slowly.
        a.governance = (a.governance + rng.normal() * 2.5).clamp(5.0, 95.0);
        if a.governance < 35.0 && rng.chance(0.04) {
            a.finance = (a.finance - 12.0).max(5.0);
            a.youth_invest = (a.youth_invest - 8.0).max(5.0);
        }
        let g = a.governance;
        // Money follows governance and commercial strength; what is invested in youth and coaching follows money.
        nudge(&mut a.finance, 30.0 + 0.35 * g + 0.25 * a.commercial, 0.06);
        nudge(&mut a.youth_invest, 25.0 + 0.4 * a.finance + 0.2 * g, 0.07);
        nudge(&mut a.coach_ed, 20.0 + 0.4 * a.youth_invest + 0.2 * a.admin, 0.06);
        nudge(&mut a.comp_quality, 20.0 + 0.35 * a.finance + 0.25 * a.admin + 0.15 * a.referee_dev, 0.05);
        nudge(&mut a.scouting, 15.0 + 0.4 * a.finance + 0.2 * a.coach_ed, 0.05);
        nudge(&mut a.grassroots_reach, 20.0 + 0.5 * a.youth_invest, 0.06);
        nudge(&mut a.referee_dev, 20.0 + 0.4 * a.coach_ed + 0.2 * a.admin, 0.05);
        nudge(&mut a.commercial, 20.0 + 0.3 * a.comp_quality + 0.2 * a.finance, 0.04);
    }
}

fn regions(w: &mut World, year: i32) {
    // Football the region has produced lately: players from here who have become senior regulars.
    let mut produced: pw_world::FxHashMap<RegionId, f32> = Default::default();
    for (&p, s) in &w.ext.ecosystem.story {
        let c = &w.players.cold[p];
        if c.senior_apps >= 10 && w.players.hot[p].status != PlayerStatus::Retired && w.age(p) <= 27 && !s.home.is_none() {
            *produced.entry(s.home).or_default() += 1.0 + f32::from(c.caps.min(20)) / 10.0;
        }
    }
    let n = w.ext.ecosystem.regions.len();
    for i in 0..n {
        let r = RegionId(i as u32);
        let state = w.ext.ecosystem.state_of(r);
        let assoc = w.ext.ecosystem.assoc.get(&state).copied();
        let Some(a) = assoc else { continue };
        let mut rng = Rng::keyed(&[w.seed, stream::WORLDGEN, 0x4e91, u64::from(r.0), year as u64]);
        let made = produced.get(&r).copied().unwrap_or(0.0) + produced.get(&state).copied().unwrap_or(0.0) * 0.1;
        let reg = &mut w.ext.ecosystem.regions[r];
        // Money in this year: public follows the association, private follows wealth and proximity to clubs.
        reg.invest_public = (0.5 * a.youth_invest + 0.3 * a.finance + 0.2 * a.governance + rng.normal() * 4.0).clamp(2.0, 98.0);
        reg.invest_private = (0.3 * reg.economic_access + 0.3 * reg.pro_proximity + 0.2 * reg.culture + 0.2 * a.commercial + rng.normal() * 4.0).clamp(2.0, 98.0);
        let flow = 0.6 * reg.invest_public + 0.4 * reg.invest_private;
        // Infrastructure lags investment by years, so a place cannot be bought good overnight or ruined at once.
        nudge(&mut reg.facilities, 5.0 + 0.85 * flow + 0.1 * a.facilities, 0.10);
        nudge(&mut reg.coach_density, 8.0 + 0.5 * a.coach_ed + 0.3 * flow, 0.10);
        nudge(&mut reg.competition_density, 8.0 + 0.5 * a.comp_quality + 0.3 * reg.participation, 0.10);
        nudge(&mut reg.academy_access, 0.4 * reg.pro_proximity + 0.4 * a.academy_coord + 0.2 * flow, 0.10);
        nudge(&mut reg.scouting_coverage, 0.35 * a.scouting + 0.25 * reg.pro_proximity + 0.25 * reg.competition_density + 0.15 * flow, 0.10);
        // Households slowly get better off.
        reg.economic_access = (reg.economic_access + 0.4).min(98.0);
        // Participation follows football's presence in people's lives, with the longest lag.
        nudge(&mut reg.participation, 0.35 * reg.culture + 0.25 * reg.facilities + 0.2 * reg.economic_access + 0.2 * a.grassroots_reach, 0.06);
        // Culture remembers what the region has produced and fades without it.
        reg.culture = (reg.culture * 0.93 + (made * 1.6).min(9.0)).clamp(2.0, 98.0);
    }
}

fn pools(w: &mut World, year: i32, first: bool) {
    let n = w.ext.ecosystem.regions.len();
    let mut districts: Vec<RegionId> = (0..n).map(|i| RegionId(i as u32)).filter(|&r| w.ext.ecosystem.regions[r].kind == RegionKind::District).collect();
    districts.sort();
    // Grassroots and amateur clubs per district, in id order.
    let mut clubs: pw_world::FxHashMap<RegionId, (Vec<LocalClubId>, Vec<LocalClubId>)> = Default::default();
    let mut locals: Vec<(LocalClubId, RegionId)> = w.ext.ecosystem.local_region.iter().map(|(&l, &r)| (l, r)).collect();
    locals.sort();
    for (l, r) in locals {
        let e = clubs.entry(r).or_default();
        if w.youth.local[l].level == pw_world::youth::LocalLevel::Grassroots {
            e.0.push(l);
        } else {
            e.1.push(l);
        }
    }
    for r in districts {
        let reg = w.ext.ecosystem.regions[r].clone();
        let state = w.ext.ecosystem.state_of(r);
        let reach = w.ext.ecosystem.assoc.get(&state).map_or(50.0, |a| a.grassroots_reach);
        let cohort = reg.population_k as f32 * 1000.0 * 0.0165;
        let entrants = cohort * (reg.participation / 100.0) * (0.5 + 0.5 * reach / 100.0) * 0.9;
        let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, 0xd157, u64::from(r.0), year as u64]);
        let pool = w.ext.ecosystem.pools.entry(r).or_default();
        if first || pool.part[0] == 0.0 {
            seed_pool(pool, entrants, reg.coach_density, reg.competition_density, reg.economic_access);
        }
        // Age everyone a year; the oldest leave the mass game (some to adult football, late).
        let leaving = pool.part[13];
        for i in (1..14).rev() {
            pool.part[i] = pool.part[i - 1] * retain(POOL_FIRST_AGE + i, reg.coach_density, reg.competition_density, reg.economic_access);
        }
        pool.part[0] = entrants;
        // Who becomes visible: more where there is competition to be seen in and people to see it.
        let visible = 0.25 + 0.75 * (0.5 * reg.competition_density + 0.5 * reg.scouting_coverage) / 100.0;
        let ages: Vec<usize> = if first { (MATERIALISE_AGE..=17).collect() } else { vec![MATERIALISE_AGE] };
        let mut wanted: Vec<(usize, u32)> = Vec::new();
        for age in ages {
            let mean = pool.part[age - POOL_FIRST_AGE] * PROSPECT_RATE * visible;
            let k = poisson(&mut rng, mean);
            pool.materialised += k;
            wanted.push((age, k));
        }
        let late = poisson(&mut rng, leaving * LATE_RATE * visible);
        pool.materialised += late;
        let (grass, amateur) = clubs.get(&r).cloned().unwrap_or_default();
        for (age, k) in wanted {
            for _ in 0..k {
                let target = if grass.is_empty() { None } else { Some(grass[rng.index(grass.len())]) };
                materialise(w, r, &reg, age as i32, target, PlayerSource::RegionalPool, &mut rng);
            }
        }
        for _ in 0..late {
            let target = if amateur.is_empty() { None } else { Some(amateur[rng.index(amateur.len())]) };
            let age = rng.range_i32(18, 24);
            materialise(w, r, &reg, age, target, PlayerSource::LateEntry, &mut rng);
        }
    }
}

/// A child or young adult leaves the mass and becomes a player with a home, a route and a recorded origin.
fn materialise(w: &mut World, r: RegionId, reg: &pw_world::ecosystem::Region, age: i32, local: Option<LocalClubId>, source: PlayerSource, rng: &mut Rng) {
    let today = w.date;
    let dob = today.add_days(-(age * 365 + rng.range_i32(0, 364)));
    let years = dob.age_years(today);
    let mut pa = rng.normal_ms(68.0 + 0.06 * reg.culture + 0.05 * reg.coach_density, 20.0);
    if rng.chance(0.004) {
        pa += rng.range_f32(25.0, 60.0);
    }
    let pa = pa.clamp(30.0, 190.0);
    let ca = (pa * crate::generate::ca_share_at(years) * rng.normal_ms(1.0, 0.1)).clamp(8.0, pa);
    let pos = crate::generate::random_position(rng);
    let np = NewPlayer { nation: reg.nation, dob, pos, ca, pa: pa as u8, club: ClubId::NONE, team: pw_core::TeamId::NONE, contract: Contract::default(), source };
    let p = spawn_player(w, np, rng);
    let who = w.players.cold[p].person;
    // Names follow the region's language; the name pools live with the builder, so keep what the world gave.
    w.players.hot[p].status = PlayerStatus::Amateur;
    if let Some(l) = local {
        w.youth.join(p, l);
    }
    if age < 19 {
        w.youth.school.insert(who, pw_world::youth::School::default());
    }
    // Who runs the football a child first plays follows what the region has.
    let weights = [
        (Provider::BlueCubs, 0.2 + 0.6 * (reg.pro_proximity / 100.0) * (reg.competition_density / 100.0)),
        (Provider::School, 0.3 + 0.5 * reg.participation / 100.0),
        (Provider::Community, 0.3 + 0.3 * (1.0 - reg.economic_access / 100.0)),
        (Provider::Municipal, 0.1 + 0.4 * reg.invest_public / 100.0),
        (Provider::Foundation, 0.05 + 0.3 * reg.invest_private / 100.0),
        (Provider::PrivateSchool, 0.05 + 0.4 * reg.economic_access / 100.0),
    ];
    let k = rng.weighted(&weights.map(|x| x.1));
    let provider = weights[k].0;
    w.ext.ecosystem.story.insert(p, PlayerStory { home: r, dev: r, provider, found_by: PersonId::NONE, found_club: ClubId::NONE, found_on: today });
    let kind = if matches!(provider, Provider::School | Provider::PrivateSchool) { StageKind::School } else { StageKind::Grassroots };
    w.ext.ecosystem.stages.entry(p).or_insert_with(SmallVec::new).push(Stage { date: today, kind, target: r.0, region: r });
}

/// A player's route as read back: recorded steps and club spells together, oldest first.
pub fn route(w: &World, p: PlayerId) -> Vec<(pw_core::Date, StageKind, u32)> {
    let mut v: Vec<(pw_core::Date, StageKind, u32)> = w.ext.ecosystem.route(p).iter().map(|s| (s.date, s.kind, s.target)).collect();
    if let Some(spells) = w.history.spells.get(&p) {
        for s in spells {
            if !v.iter().any(|x| x.0 == s.from && matches!(x.1, StageKind::Academy | StageKind::Professional | StageKind::SemiPro)) {
                v.push((s.from, StageKind::Professional, s.club.0));
            }
        }
    }
    v.sort_by_key(|x| x.0);
    v
}
