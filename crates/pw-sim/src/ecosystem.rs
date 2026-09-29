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
use pw_world::minor::InstKind;
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
    let mut counts: pw_world::FxHashMap<RegionId, (i64, i64)> = Default::default();
    for (&p, s) in &w.ext.ecosystem.story {
        let c = &w.players.cold[p];
        if c.senior_apps >= 10 && w.players.hot[p].status != PlayerStatus::Retired && w.age(p) <= 27 && !s.home.is_none() {
            *produced.entry(s.home).or_default() += 1.0 + f32::from(c.caps.min(20)) / 10.0;
            let e = counts.entry(s.home).or_default();
            e.0 += 1;
            e.1 += i64::from(c.caps > 0);
            let st = w.ext.ecosystem.state_of(s.home);
            if st != s.home && st.is_some() {
                let e = counts.entry(st).or_default();
                e.0 += 1;
                e.1 += i64::from(c.caps > 0);
            }
        }
    }
    let mut marks: Vec<(RegionId, (i64, i64))> = counts.into_iter().collect();
    marks.sort_by_key(|m| m.0);
    for (r, (players, caps)) in marks {
        crate::almanac::region_marks(w, r, players, caps);
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

/// Potential and current ability for someone drawn from a region's pool: the same draw for everyone, human or not.
fn draw_talent(reg: &pw_world::ecosystem::Region, years: f32, rng: &mut Rng) -> (f32, f32) {
    let mut pa = rng.normal_ms(68.0 + 0.06 * reg.culture + 0.05 * reg.coach_density, 20.0);
    if rng.chance(0.004) {
        pa += rng.range_f32(25.0, 60.0);
    }
    let pa = pa.clamp(30.0, 190.0);
    let ca = (pa * crate::generate::ca_share_at(years) * rng.normal_ms(1.0, 0.1)).clamp(8.0, pa);
    (pa, ca)
}

/// A child or young adult leaves the mass and becomes a player with a home, a route and a recorded origin.
fn materialise(w: &mut World, r: RegionId, reg: &pw_world::ecosystem::Region, age: i32, local: Option<LocalClubId>, source: PlayerSource, rng: &mut Rng) {
    let today = w.date;
    let dob = today.add_days(-(age * 365 + rng.range_i32(0, 364)));
    let (pa, ca) = draw_talent(reg, dob.age_years(today), rng);
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

// ---------------------------------------------------------------------------
// Being seen: district selection, school football, universities, national camps
// ---------------------------------------------------------------------------

fn scout_of(w: &World, club: ClubId) -> Option<pw_core::StaffId> {
    let c = &w.clubs[club];
    c.staff.iter().copied().find(|&s| w.staff[s].role == pw_world::StaffRole::HeadOfYouth).or_else(|| c.staff.iter().copied().find(|&s| w.staff[s].role == pw_world::StaffRole::Scout))
}

/// Monthly: the ways players get seen outside a club's own academy.
pub fn monthly(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    match w.date.month() {
        10 => district_selection(w),
        1 => university_scouting(w),
        5 => camps(w),
        m if m >= 8 || m <= 4 => school_scouting(w),
        _ => {}
    }
}

/// October: each district picks its best young players for a district side, on what its selectors have seen.
fn district_selection(w: &mut World) {
    let year = w.date.year();
    let mut by_district: pw_world::FxHashMap<RegionId, Vec<PlayerId>> = Default::default();
    for (&p, s) in &w.ext.ecosystem.story {
        let age = w.age(p);
        if (12..=17).contains(&age) && w.players.hot[p].status == PlayerStatus::Amateur && s.dev.is_some() {
            by_district.entry(s.dev).or_default().push(p);
        }
    }
    let mut districts: Vec<RegionId> = by_district.keys().copied().collect();
    districts.sort();
    let academies: Vec<ClubId> = {
        let mut v: Vec<ClubId> = w.youth.academies.keys().copied().collect();
        v.sort();
        v
    };
    for r in districts {
        let state = w.ext.ecosystem.state_of(r);
        let scouting = w.ext.ecosystem.assoc.get(&state).map_or(40.0, |a| a.scouting);
        let sigma = (14.0 - scouting / 10.0).max(3.0);
        let mut cands: Vec<(f32, PlayerId)> = by_district[&r]
            .iter()
            .map(|&p| {
                let h = &w.players.hot[p];
                let form = h.form_avg().map_or(0.0, |f| (f - 6.6) * 3.0);
                (perceive_ca(w, p, sigma * 2.0, 3_000_000 + r.0, 8000 + year as u64) + form, p)
            })
            .collect();
        cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for &(_, p) in cands.iter().take(16) {
            note(w, p, StageKind::District, r.0);
            // The district side is played in front of academy people who can get there.
            for &club in &academies {
                let ar = w.ext.ecosystem.region_of_club(club);
                let near = w.ext.ecosystem.travel_burden(ar, r) < 0.3 || matches!(w.youth.academies[&club].reach, pw_world::youth::Reach::National | pw_world::youth::Reach::International);
                if near && w.roll(stream::YOUTH, &[u64::from(club.0), u64::from(p.0), year as u64, 0xd15]) < 0.35 {
                    w.knowledge.observe(club, p, 120, w.date);
                    if let Some(s) = scout_of(w, club) {
                        let finder = w.staff[s].person;
                        note_found(w, p, finder, club);
                    }
                }
            }
        }
    }
}

fn perceive_ca(w: &World, p: PlayerId, sigma: f32, observer: u32, field: u64) -> f32 {
    pw_world::knowledge::perceive(f32::from(w.players.cold[p].ca), sigma, pw_world::knowledge::Observer::Person(observer), p, field)
}

/// School term: academy people watch the school football of the districts near them.
fn school_scouting(w: &mut World) {
    let today = w.date;
    let month = u64::from(today.month());
    let academies: Vec<ClubId> = {
        let mut v: Vec<ClubId> = w.youth.academies.keys().copied().collect();
        v.sort();
        v
    };
    for club in academies {
        let Some(scout) = scout_of(w, club) else { continue };
        let ar = w.ext.ecosystem.region_of_club(club);
        let mut insts: Vec<u32> = w.ext.ecosystem.inst.iter().filter(|(i, p)| w.minor.institutions[**i as usize].kind == pw_world::minor::InstKind::School && p.region.is_some()).map(|(i, _)| *i).collect();
        insts.sort();
        for i in insts {
            let region = w.ext.ecosystem.inst[&i].region;
            let cov = w.ext.ecosystem.regions[region].scouting_coverage / 100.0;
            let near = (1.0 - 1.5 * w.ext.ecosystem.travel_burden(ar, region)).max(0.0);
            let p_visit = 0.02 + 0.10 * cov * near;
            if w.roll(stream::YOUTH, &[u64::from(club.0), u64::from(i), month, w.date.year() as u64, 0x5c0]) >= p_visit {
                continue;
            }
            let mut kids: Vec<(u8, PlayerId)> = w.minor.institutions[i as usize].members.iter().copied().filter(|&p| (12..=18).contains(&w.age(p))).map(|p| (w.players.hot[p].form[0], p)).collect();
            kids.sort_by(|a, b| b.cmp(a));
            for (_, p) in kids.into_iter().take(3) {
                w.knowledge.observe(club, p, 70, today);
                let r = crate::scouting::judge(w, scout, club, p, 300);
                w.scouting.file(club, p, r);
                let finder = w.staff[scout].person;
                note_found(w, p, finder, club);
            }
        }
    }
}

/// January: professional clubs watch the university game; those who shine are seen.
fn university_scouting(w: &mut World) {
    let mut seen: Vec<PlayerId> = w
        .minor
        .lines
        .iter()
        .filter(|(_, l)| l.apps >= 4 && l.rating / u32::from(l.apps) >= 71 && matches!(l.entrant, pw_world::minor::Entrant::Inst(i) if w.minor.institutions[i as usize].kind == pw_world::minor::InstKind::University))
        .map(|((p, _), _)| *p)
        .collect();
    seen.sort();
    seen.dedup();
    for p in seen {
        crate::statepath::notice(w, p, 90);
    }
}

/// May: the national federation's identification camps: a hundred called, then forty, then twenty-five.
/// Selectors see only what they have seen; being called is not being picked.
fn camps(w: &mut World) {
    let year = w.date.year();
    let scouting = w.ext.ecosystem.federation.map_or(50.0, |f| f.scouting);
    let sigma = (12.0 - scouting / 10.0).max(3.0);
    let mut cands: Vec<(f32, PlayerId)> = Vec::new();
    for (p, h) in w.players.hot.iter_enumerated() {
        if h.status == PlayerStatus::Retired || !(14..=17).contains(&w.age(p)) {
            continue;
        }
        let Some(s) = w.ext.ecosystem.story.get(&p) else { continue };
        // Visible through an academy or youth league; less through a school or district side.
        let vis = if h.club.is_some() { 0.9 } else { 0.35 + 0.4 * w.ext.ecosystem.regions[s.dev].scouting_coverage / 100.0 };
        let seen_p = (0.10 + 0.6 * scouting / 100.0) * vis;
        if w.roll(stream::INTL, &[u64::from(p.0), year as u64, 0xca3]) >= seen_p {
            continue;
        }
        cands.push((perceive_ca(w, p, sigma * 2.0, 4_000_000, 9000 + year as u64), p));
    }
    cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    for (rank, &(_, p)) in cands.iter().take(100).enumerate() {
        w.ext.ecosystem.camp.insert(p, (year, 1));
        note(w, p, StageKind::NationalCamp, 1);
        // Cuts, on a sharper second and third look.
        if rank < 40 {
            w.ext.ecosystem.camp.insert(p, (year, 2));
        }
        if rank < 25 {
            w.ext.ecosystem.camp.insert(p, (year, 3));
        }
    }
    w.ext.ecosystem.camp.retain(|_, (y, _)| *y >= year - 3);
}

/// What being in a national camp is worth when the youth sides are named (not permanent status).
pub fn camp_bonus(w: &World, p: PlayerId) -> f32 {
    match w.ext.ecosystem.camp.get(&p) {
        Some(&(y, stage)) if w.date.year() - y <= 2 => f32::from(stage) * 1.3,
        _ => 0.0,
    }
}

/// How the ecosystem is doing, for the shared development metrics.
#[derive(Clone, Debug, Default)]
pub struct DevMetrics {
    pub regions: usize,
    /// Children in the participation pools.
    pub participants: f64,
    pub prospects_drawn: u32,
    /// Mean coach density across districts (0–100), a proxy for licensed-coach coverage.
    pub coach_density: f32,
    pub mean_facilities: f32,
    pub mean_participation: f32,
    /// Share of districts with real academy access.
    pub academy_coverage: f32,
    pub university_players: usize,
    pub scholarships_held: usize,
    pub national_pool_depth: usize,
    pub camp_called: usize,
    pub state_titles: usize,
}

pub fn metrics(w: &World) -> DevMetrics {
    let e = &w.ext.ecosystem;
    let districts: Vec<&pw_world::ecosystem::Region> = e.regions.iter().filter(|r| r.kind == RegionKind::District).collect();
    let n = districts.len().max(1) as f32;
    let nation = e.regions.iter().next().map(|r| r.nation);
    DevMetrics {
        regions: e.regions.len(),
        participants: e.pools.values().map(|p| p.part.iter().map(|&x| f64::from(x)).sum::<f64>()).sum(),
        prospects_drawn: e.pools.values().map(|p| p.materialised).sum(),
        coach_density: districts.iter().map(|r| r.coach_density).sum::<f32>() / n,
        mean_facilities: districts.iter().map(|r| r.facilities).sum::<f32>() / n,
        mean_participation: districts.iter().map(|r| r.participation).sum::<f32>() / n,
        academy_coverage: districts.iter().filter(|r| r.academy_access >= 50.0).count() as f32 / n,
        university_players: w.minor.member_of.iter().filter(|(_, i)| w.minor.institutions[**i as usize].kind == pw_world::minor::InstKind::University).count(),
        scholarships_held: e.scholarship.len(),
        national_pool_depth: nation.map_or(0, |nt| w.players.hot.iter_enumerated().filter(|(p, h)| h.status != PlayerStatus::Retired && w.people[w.players.cold[*p].person].nation == nt && (17..=30).contains(&w.age(*p)) && w.players.cold[*p].ca >= 70).count()),
        camp_called: e.camp.len(),
        state_titles: e.tournament_titles.len(),
    }
}

// ---------------------------------------------------------------------------
// Starting somewhere else on the route (for a person a human will inhabit)
// ---------------------------------------------------------------------------

/// Where on the route a new person begins. Talent is never chosen: it is drawn from the region's
/// pool like everyone's, and hidden. Everything after the start follows the ordinary systems.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Start {
    /// Fifteen or sixteen, standing out at a school in the district.
    SchoolStandout,
    /// Eighteen, released by an academy near home.
    ReleasedAcademy,
    /// Eighteen, just arrived at a university on a tuition scholarship.
    UniversityFreshman,
    /// Twenty-one, three years into a university place.
    UniversityStar,
    /// Twenty, playing in the state's premier league.
    StateLeague,
    /// Twenty-two, in the national fourth tier.
    SemiPro,
}

impl Start {
    pub const fn age(self) -> i32 {
        match self {
            Start::SchoolStandout => 16,
            Start::ReleasedAcademy | Start::UniversityFreshman => 18,
            Start::StateLeague => 20,
            Start::UniversityStar => 21,
            Start::SemiPro => 22,
        }
    }
}

/// Create a person at a place on the route, in the given district (or the first district if none is given).
/// Returns `None` if the world has no suitable place (no school, university or club there).
pub fn begin(w: &mut World, start: Start, region: RegionId, salt: u64) -> Option<PlayerId> {
    if !w.ext.ecosystem.is_configured() {
        return None;
    }
    let today = w.date;
    let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, 0x57a27, salt]);
    let region = if region.is_some() { region } else { RegionId(w.ext.ecosystem.regions.iter_enumerated().find(|(_, r)| r.kind == RegionKind::District)?.0.0) };
    let reg = w.ext.ecosystem.regions[region].clone();
    let state = w.ext.ecosystem.state_of(region);
    let age = start.age();
    let dob = today.add_days(-(age * 365 + rng.range_i32(0, 364)));
    let (pa, ca) = draw_talent(&reg, dob.age_years(today), &mut rng);
    let pos = crate::generate::random_position(&mut rng);
    let base = |club, team, contract| NewPlayer { nation: reg.nation, dob, pos, ca, pa: pa as u8, club, team, contract, source: PlayerSource::HumanCreated };
    // The club of a given league tier nearest the region's state.
    let club_in = |w: &World, min_tier: u8, max_tier: u8| -> Option<ClubId> {
        w.clubs
            .iter_enumerated()
            .filter(|(id, c)| {
                let l = c.league;
                l.is_some() && (min_tier..=max_tier).contains(&w.comps[l].tier) && w.comps[l].team_kind == pw_world::TeamKind::First && w.ext.ecosystem.state_of(w.ext.ecosystem.region_of_club(*id)) == state
            })
            .map(|(id, _)| id)
            .next()
    };
    let p = match start {
        Start::StateLeague | Start::SemiPro => {
            let (lo, hi) = if start == Start::StateLeague { (50, 51) } else { (4, 4) };
            let club = club_in(w, lo, hi)?;
            let team = w.clubs[club].first_team();
            let contract = Contract { club, kind: pw_world::contract::ContractKind::Professional, wage: 0, start: today, end: today.add_months(24), yearly_rise: 3, ..Default::default() };
            let p = spawn_player(w, base(club, team, contract), &mut rng);
            let wage = crate::market::wage_demand(w, p, club);
            w.players.cold[p].contract.wage = wage;
            p
        }
        _ => {
            let p = spawn_player(w, base(ClubId::NONE, pw_core::TeamId::NONE, Contract::default()), &mut rng);
            w.players.hot[p].status = PlayerStatus::Amateur;
            p
        }
    };
    let who = w.players.cold[p].person;
    w.ext.ecosystem.story.insert(p, PlayerStory { home: region, dev: region, provider: Provider::Community, found_by: PersonId::NONE, found_club: ClubId::NONE, found_on: today });
    let place = |w: &mut World, kind: InstKind| -> Option<u32> {
        let mut c: Vec<(f32, u32)> = w.ext.ecosystem.inst.iter().filter(|(i, _)| w.minor.institutions[**i as usize].kind == kind).map(|(i, pr)| (w.ext.ecosystem.travel_burden(region, pr.region), *i)).collect();
        c.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        c.first().map(|x| x.1)
    };
    match start {
        Start::SchoolStandout => {
            let s = place(w, InstKind::School)?;
            w.minor.join(p, s);
            w.youth.school.insert(who, pw_world::youth::School::default());
            note(w, p, StageKind::School, s);
        }
        Start::ReleasedAcademy => {
            let mut academies: Vec<(f32, ClubId)> = w.youth.academies.keys().map(|&c| (w.ext.ecosystem.travel_burden(region, w.ext.ecosystem.region_of_club(c)), c)).collect();
            academies.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let club = academies.first()?.1;
            w.players.cold[p].youth_club = club;
            w.youth.released.entry(p).or_default().push(pw_world::youth::Release { club, date: today.add_days(-60), age: 17 });
            w.ext.ecosystem.stages.entry(p).or_default().push(Stage { date: today.add_days(-365 * 3), kind: StageKind::Academy, target: club.0, region });
            w.ext.ecosystem.stages.entry(p).or_default().push(Stage { date: today.add_days(-60), kind: StageKind::Released, target: club.0, region });
            w.players.hot[p].morale = 35;
        }
        Start::UniversityFreshman | Start::UniversityStar => {
            let u = place(w, InstKind::University)?;
            w.minor.join(p, u);
            let year = today.year() - if start == Start::UniversityStar { 2 } else { 0 };
            w.minor.enrolled.insert(p, year);
            w.ext.ecosystem.scholarship.insert(p, pw_world::ecosystem::Scholarship { inst: u, tier: if start == Start::UniversityStar { 2 } else { 1 }, from: today });
            let r = w.ext.ecosystem.inst[&u].region;
            set_dev_region(w, p, r);
            note(w, p, StageKind::University, u);
        }
        Start::StateLeague | Start::SemiPro => note(w, p, if start == Start::SemiPro { StageKind::SemiPro } else { StageKind::StateLeague }, w.players.hot[p].club.0),
    }
    crate::life::sync(w);
    Some(p)
}
