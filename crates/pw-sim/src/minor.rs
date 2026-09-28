//! Schools, universities and minor competitions. See `pw_world::minor`.
//!
//! September: institutions take their members (children at school age in
//! their town; school leavers with the grades who are not professionals go
//! to university), competitions are drawn. Weekly through the season: one
//! league round, a cup round every third week, with scores from team
//! strength and goals credited to real members. June: winners, top scorers
//! and best players go into the history books; standout players are noticed
//! by professional clubs nearby; amateur clubs move up and down the local
//! pyramid through their standing.

use pw_core::rng::stream;
use pw_core::{Attr, ClubId, LocalClubId, NationId, PersonId, PlayerId, Rng};
use pw_world::event::{EventKind, Visibility};
use pw_world::minor::{Biggest, Entrant, InstKind, Institution, MinorComp, MinorKind, MinorLine, MinorSeason, Row};
use pw_world::youth::LocalLevel;
use pw_world::{FxHashMap, PlayerStatus, World};
use smallvec::SmallVec;

use crate::consider;

const SCHOOL_AGES: std::ops::RangeInclusive<u32> = 11..=18;
const UNI_YEARS: i32 = 3;

// ---------------------------------------------------------------------------
// Institutions
// ---------------------------------------------------------------------------

/// Cities per nation (from the local clubs, which come from real club towns).
fn cities(w: &World) -> FxHashMap<NationId, Vec<String>> {
    let mut m: FxHashMap<NationId, Vec<String>> = FxHashMap::default();
    for l in w.youth.local.iter() {
        let v = m.entry(l.nation).or_default();
        if !v.contains(&l.city) {
            v.push(l.city.clone());
        }
    }
    for v in m.values_mut() {
        v.sort();
    }
    m
}

/// Create schools and universities once, from the world's towns.
pub fn ensure(w: &mut World) {
    if !w.minor.institutions.is_empty() {
        return;
    }
    let by_nation = cities(w);
    let mut nations: Vec<NationId> = by_nation.keys().copied().collect();
    nations.sort();
    for n in nations {
        let towns = &by_nation[&n];
        for (i, city) in towns.iter().enumerate() {
            let mut rng = Rng::keyed(&[w.seed, stream::MINOR, u64::from(n.0), i as u64, 0x5c]);
            let names = ["High School", "Grammar School", "Community School", "College", "Academy School", "Secondary School"];
            let a = rng.index(names.len());
            let b = (a + 1 + rng.index(names.len() - 1)) % names.len();
            for suffix in [names[a], names[b]] {
                let id = w.minor.institutions.len() as u32;
                w.minor.institutions.push(Institution {
                    id,
                    kind: InstKind::School,
                    name: format!("{city} {suffix}"),
                    nation: n,
                    city: city.clone(),
                    founded: 1870 + rng.below(120) as i32,
                    prestige: rng.range_i32(100, 800) as u16,
                    coaching: rng.range_i32(2, 10) as u8,
                    members: Vec::new(),
                    alumni_pros: SmallVec::new(),
                });
            }
        }
        // Universities in the towns with the most football.
        let mut ranked: Vec<(usize, &String)> = towns.iter().map(|c| (w.youth.local.iter().filter(|l| l.nation == n && &l.city == c).count(), c)).collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
        let count = (towns.len() / 4).clamp(1, 8);
        for (k, (_, city)) in ranked.into_iter().take(count).enumerate() {
            let mut rng = Rng::keyed(&[w.seed, stream::MINOR, u64::from(n.0), k as u64, 0x0a1]);
            let name = match rng.below(3) {
                0 => format!("University of {city}"),
                1 => format!("{city} Technical University"),
                _ => format!("{city} Metropolitan University"),
            };
            let id = w.minor.institutions.len() as u32;
            w.minor.institutions.push(Institution {
                id,
                kind: InstKind::University,
                name,
                nation: n,
                city: city.clone(),
                founded: 1700 + rng.below(290) as i32,
                prestige: rng.range_i32(200, 950) as u16,
                coaching: rng.range_i32(5, 14) as u8,
                members: Vec::new(),
                alumni_pros: SmallVec::new(),
            });
        }
    }
    assign(w);
}

fn city_of(w: &World, p: PlayerId) -> Option<(NationId, String)> {
    if let Some(&l) = w.youth.member_of.get(&p) {
        let lc = &w.youth.local[l];
        return Some((lc.nation, lc.city.clone()));
    }
    let club = w.players.hot[p].club;
    if club.is_some() {
        let c = &w.clubs[club];
        return Some((c.nation, if c.city.is_empty() { c.short_name.clone() } else { c.city.clone() }));
    }
    None
}

/// September: children go to school in their town; school leavers with the
/// grades who are not professionals may go to university; graduates leave;
/// members who turned professional become alumni.
pub fn assign(w: &mut World) {
    let today = w.date;
    let year = today.year();
    // Leavers first.
    let members: Vec<(PlayerId, u32)> = w.minor.member_of.iter().map(|(&p, &i)| (p, i)).collect();
    for (p, inst) in members {
        let kind = w.minor.institutions[inst as usize].kind;
        let age = w.age(p);
        let pro = w.players.hot[p].status == PlayerStatus::Active && w.players.hot[p].club.is_some();
        let done = match kind {
            InstKind::School => !SCHOOL_AGES.contains(&age),
            InstKind::University => w.minor.enrolled.get(&p).is_some_and(|&y| year - y >= UNI_YEARS),
        };
        if pro && kind == InstKind::University || done {
            let who = w.players.cold[p].person;
            w.minor.leave(p);
            if kind == InstKind::University {
                w.minor.enrolled.remove(&p);
                w.events.push(today, Visibility::Person(who), EventKind::Graduated { person: who, institution: inst, early: !done });
            }
        }
        if pro {
            let who = w.players.cold[p].person;
            let a = &mut w.minor.institutions[inst as usize].alumni_pros;
            if !a.contains(&who) {
                a.push(who);
            }
        }
    }
    // Schools by town.
    let mut schools: FxHashMap<(NationId, String), SmallVec<[u32; 2]>> = FxHashMap::default();
    let mut unis: FxHashMap<NationId, Vec<u32>> = FxHashMap::default();
    for i in &w.minor.institutions {
        match i.kind {
            InstKind::School => schools.entry((i.nation, i.city.clone())).or_default().push(i.id),
            InstKind::University => unis.entry(i.nation).or_default().push(i.id),
        }
    }
    let mut kids: Vec<PersonId> = w.youth.school.keys().copied().collect();
    kids.sort();
    for who in kids {
        let p = w.people[who].player;
        if p.is_none() || w.minor.member_of.contains_key(&p) || !SCHOOL_AGES.contains(&w.age(p)) {
            continue;
        }
        let Some(town) = city_of(w, p) else { continue };
        let Some(options) = schools.get(&town) else { continue };
        let k = pw_core::rng::hash_key(&[w.seed, stream::MINOR, u64::from(who.0)]) as usize % options.len();
        w.minor.join(p, options[k]);
    }
    // University: school leavers aged 18–19 with qualifications, not professionals.
    let leavers: Vec<PlayerId> = w
        .players
        .hot
        .iter_enumerated()
        .filter(|(_, h)| matches!(h.status, PlayerStatus::Amateur | PlayerStatus::FreeAgent))
        .map(|(p, _)| p)
        .filter(|&p| (18..=19).contains(&w.age(p)) && !w.minor.member_of.contains_key(&p))
        .collect();
    for p in leavers {
        let who = w.players.cold[p].person;
        let edu = w.lives.get(who).map_or(0, |l| l.education);
        if edu < 2 {
            continue;
        }
        let study = w.lives.get(who).map_or(0.0, |l| f32::from(l.routine.study));
        let prob = 0.15 + study * 0.03 + consider::hid(w, who, pw_core::Hidden::Professionalism) / 100.0;
        if w.roll(stream::MINOR, &[u64::from(p.0), year as u64, 0x0e]) >= prob {
            continue;
        }
        let Some(town) = city_of(w, p) else { continue };
        let Some(list) = unis.get(&town.0) else { continue };
        // Home town first, otherwise the most prestigious that takes them.
        let pick = list.iter().copied().find(|&u| w.minor.institutions[u as usize].city == town.1).unwrap_or_else(|| {
            let k = pw_core::rng::hash_key(&[w.seed, stream::MINOR, u64::from(p.0), 0x17]) as usize % list.len();
            list[k]
        });
        w.minor.join(p, pick);
        w.minor.enrolled.insert(p, year);
        w.events.push(today, Visibility::Person(who), EventKind::EnrolledUniversity { person: who, institution: pick });
    }
}

// ---------------------------------------------------------------------------
// Competitions
// ---------------------------------------------------------------------------

/// Circle-method double round robin.
fn double_round_robin(teams: &[Entrant]) -> Vec<SmallVec<[(Entrant, Entrant); 8]>> {
    let mut t: Vec<Option<Entrant>> = teams.iter().copied().map(Some).collect();
    if t.len() % 2 == 1 {
        t.push(None);
    }
    let n = t.len();
    if n < 2 {
        return Vec::new();
    }
    let mut first = Vec::new();
    for r in 0..n - 1 {
        let mut round: SmallVec<[(Entrant, Entrant); 8]> = SmallVec::new();
        for i in 0..n / 2 {
            if let (Some(a), Some(b)) = (t[i], t[n - 1 - i]) {
                round.push(if r % 2 == 0 { (a, b) } else { (b, a) });
            }
        }
        first.push(round);
        let last = t.pop().expect("n >= 2");
        t.insert(1, last);
    }
    let second: Vec<SmallVec<[(Entrant, Entrant); 8]>> = first.iter().map(|r| r.iter().map(|&(a, b)| (b, a)).collect()).collect();
    first.extend(second);
    first
}

fn new_comp(w: &mut World, kind: MinorKind, nation: NationId, region: String, entrants: Vec<Entrant>) {
    if entrants.len() < 2 {
        return;
    }
    let id = w.minor.comps.len() as u32;
    let season = w.minor.season;
    let rounds = if kind.is_cup() { Vec::new() } else { double_round_robin(&entrants) };
    let table = if kind.is_cup() { Vec::new() } else { entrants.iter().map(|&e| (e, Row::default())).collect() };
    let alive = if kind.is_cup() { entrants.clone() } else { Vec::new() };
    w.minor.comps.push(MinorComp { id, kind, nation, region, season, entrants, rounds, next_round: 0, table, alive, done: false });
}

/// September: draw this season's competitions.
pub fn season_start(w: &mut World) {
    let year = w.date.year();
    if w.minor.season == year && !w.minor.comps.is_empty() {
        return;
    }
    season_end(w);
    w.minor.season = year;
    w.minor.comps.clear();
    w.minor.biggest.clear();
    assign(w);
    let mut nations: Vec<NationId> = w.minor.institutions.iter().map(|i| i.nation).collect();
    nations.sort();
    nations.dedup();
    for n in nations {
        // School leagues: towns grouped into regions of about eight schools.
        let mut schools: Vec<(String, u32)> = w.minor.institutions.iter().filter(|i| i.nation == n && i.kind == InstKind::School && !i.members.is_empty()).map(|i| (i.city.clone(), i.id)).collect();
        schools.sort();
        for chunk in schools.chunks(8) {
            let region = if chunk.first().map(|c| &c.0) == chunk.last().map(|c| &c.0) { chunk[0].0.clone() } else { format!("{} and district", chunk[0].0) };
            new_comp(w, MinorKind::SchoolLeague, n, region, chunk.iter().map(|c| Entrant::Inst(c.1)).collect());
        }
        new_comp(w, MinorKind::SchoolCup, n, String::new(), schools.iter().map(|c| Entrant::Inst(c.1)).collect());
        let unis: Vec<Entrant> = w.minor.institutions.iter().filter(|i| i.nation == n && i.kind == InstKind::University && i.members.len() >= 11).map(|i| Entrant::Inst(i.id)).collect();
        new_comp(w, MinorKind::UniversityLeague, n, String::new(), unis);
        // The amateur pyramid: standing orders the tiers.
        let mut am: Vec<(u16, LocalClubId)> = w.youth.local.iter_enumerated().filter(|(_, l)| l.nation == n && l.level == LocalLevel::Amateur && l.members.len() >= 11).map(|(id, l)| (u16::MAX - l.standing, id)).collect();
        am.sort();
        for (t, chunk) in am.chunks(12).enumerate() {
            new_comp(w, MinorKind::AmateurLeague { tier: t as u8 + 1 }, n, String::new(), chunk.iter().map(|c| Entrant::Local(c.1)).collect());
        }
        let gr: Vec<Entrant> = w.youth.local.iter_enumerated().filter(|(_, l)| l.nation == n && l.level == LocalLevel::Grassroots && !l.members.is_empty()).map(|(id, _)| Entrant::Local(id)).collect();
        new_comp(w, MinorKind::GrassrootsCup, n, String::new(), gr);
    }
}

/// Who plays for an entrant in a competition of this kind.
fn squad(w: &World, e: Entrant, kind: MinorKind) -> SmallVec<[PlayerId; 16]> {
    let members: &[PlayerId] = match e {
        Entrant::Inst(i) => &w.minor.institutions[i as usize].members,
        Entrant::Local(l) => &w.youth.local[l].members,
    };
    let mut v: SmallVec<[(u8, PlayerId); 32]> = members
        .iter()
        .copied()
        .filter(|&p| w.players.hot[p].injury_days == 0 && (kind != MinorKind::GrassrootsCup || w.age(p) >= 13))
        .map(|p| (w.players.cold[p].ca, p))
        .collect();
    v.sort_by(|a, b| b.cmp(a));
    v.into_iter().take(14).map(|x| x.1).collect()
}

fn coaching(w: &World, e: Entrant) -> f32 {
    match e {
        Entrant::Inst(i) => f32::from(w.minor.institutions[i as usize].coaching),
        Entrant::Local(l) => f32::from(w.youth.local[l].coaching),
    }
}

fn strength(w: &World, sq: &[PlayerId], e: Entrant) -> f32 {
    if sq.is_empty() {
        return 0.0;
    }
    let n = sq.len().min(11);
    let mean = sq.iter().take(11).map(|&p| f32::from(w.players.cold[p].ca)).sum::<f32>() / n as f32;
    // Short-handed sides suffer.
    mean * (n as f32 / 11.0) + coaching(w, e) * 1.5
}

fn poisson(rng: &mut Rng, lambda: f32) -> u8 {
    let l = (-lambda).exp();
    let mut k = 0u8;
    let mut p = 1.0f32;
    loop {
        p *= rng.f32();
        if p <= l || k >= 15 {
            return k;
        }
        k += 1;
    }
}

/// Play one game; credit appearances, goals and ratings to real members.
fn play(w: &mut World, comp: u32, a: Entrant, b: Entrant, key: u64) -> (u8, u8) {
    let kind = w.minor.comps[comp as usize].kind;
    let sa = squad(w, a, kind);
    let sb = squad(w, b, kind);
    let (xa, xb) = (strength(w, &sa, a), strength(w, &sb, b));
    let mut rng = w.rng(stream::MINOR, &[u64::from(comp), key]);
    let diff = (xa - xb) / 35.0;
    let (mut ga, mut gb) = (poisson(&mut rng, (1.45 * diff.exp()).clamp(0.2, 6.0)), poisson(&mut rng, (1.15 * (-diff).exp()).clamp(0.2, 6.0)));
    if sa.is_empty() {
        ga = 0;
    }
    if sb.is_empty() {
        gb = 0;
    }
    let season = w.minor.season;
    for (sq, e, goals, conceded) in [(&sa, a, ga, gb), (&sb, b, gb, ga)] {
        // Scorers: weighted by attacking ability.
        let weights: SmallVec<[f32; 16]> = sq
            .iter()
            .take(11)
            .map(|&p| {
                let c = &w.players.cold[p];
                let f = c.attrs.get(Attr::Finishing) + c.attrs.get(Attr::OffTheBall) * 0.5;
                let pos = match c.best_pos {
                    pw_core::Pos::ST | pw_core::Pos::AMC | pw_core::Pos::AML | pw_core::Pos::AMR => 3.0,
                    pw_core::Pos::MC | pw_core::Pos::ML | pw_core::Pos::MR | pw_core::Pos::DM => 1.2,
                    pw_core::Pos::GK => 0.02,
                    _ => 0.5,
                };
                (f * pos).max(0.01)
            })
            .collect();
        let mut scored: SmallVec<[u16; 16]> = SmallVec::from_elem(0, weights.len());
        for _ in 0..goals {
            let i = rng.weighted(&weights);
            if i < scored.len() {
                scored[i] += 1;
            }
        }
        for (i, &p) in sq.iter().take(11).enumerate() {
            let rating = (60 + scored[i] as i32 * 8 + (i32::from(goals) - i32::from(conceded)) * 3 + (rng.normal() * 5.0) as i32).clamp(30, 100) as u32;
            let l = w.minor.lines.entry((p, comp)).or_insert(MinorLine { season, kind, entrant: e, apps: 0, goals: 0, rating: 0 });
            l.apps += 1;
            l.goals += scored[i];
            l.rating += rating;
        }
    }
    let margin = (i32::from(ga) - i32::from(gb)).unsigned_abs();
    if margin >= 3 {
        let best = w.minor.biggest.get(&comp).map_or(0, |b| (i32::from(b.score.0) - i32::from(b.score.1)).unsigned_abs());
        if margin > best {
            let (winner, loser, score) = if ga > gb { (a, b, (ga, gb)) } else { (b, a, (gb, ga)) };
            w.minor.biggest.insert(comp, Biggest { date: w.date, winner, loser, score });
        }
    }
    (ga, gb)
}

/// Weekly in season (September–May).
pub fn weekly(w: &mut World) {
    let today = w.date;
    if matches!(today.month(), 6..=8) {
        return;
    }
    if w.minor.season != today.year() && today.month() >= 9 {
        season_start(w);
    }
    let week = u64::from(today.0 as u32 / 7);
    for comp in 0..w.minor.comps.len() as u32 {
        let c = &w.minor.comps[comp as usize];
        if c.done {
            continue;
        }
        if c.kind.is_cup() {
            if week % 3 != 0 {
                continue;
            }
            cup_round(w, comp, week);
        } else {
            let r = usize::from(c.next_round);
            let Some(fixtures) = c.rounds.get(r).cloned() else {
                w.minor.comps[comp as usize].done = true;
                continue;
            };
            for (i, (a, b)) in fixtures.into_iter().enumerate() {
                let (ga, gb) = play(w, comp, a, b, (week << 8) | i as u64);
                let t = &mut w.minor.comps[comp as usize].table;
                for (e, f, ag) in [(a, ga, gb), (b, gb, ga)] {
                    if let Some(row) = t.iter_mut().find(|x| x.0 == e).map(|x| &mut x.1) {
                        row.p += 1;
                        row.gf += u16::from(f);
                        row.ga += u16::from(ag);
                        match f.cmp(&ag) {
                            std::cmp::Ordering::Greater => {
                                row.w += 1;
                                row.pts += 3;
                            }
                            std::cmp::Ordering::Equal => {
                                row.d += 1;
                                row.pts += 1;
                            }
                            std::cmp::Ordering::Less => row.l += 1,
                        }
                    }
                }
            }
            w.minor.comps[comp as usize].next_round += 1;
        }
    }
}

fn cup_round(w: &mut World, comp: u32, week: u64) {
    let mut alive = w.minor.comps[comp as usize].alive.clone();
    if alive.len() <= 1 {
        w.minor.comps[comp as usize].done = true;
        return;
    }
    let mut rng = w.rng(stream::MINOR, &[u64::from(comp), week, 0xc0]);
    rng.shuffle(&mut alive);
    let mut next = Vec::with_capacity(alive.len() / 2 + 1);
    let mut i = 0;
    while i + 1 < alive.len() {
        let (a, b) = (alive[i], alive[i + 1]);
        let (ga, gb) = play(w, comp, a, b, (week << 16) | i as u64);
        let through = match ga.cmp(&gb) {
            std::cmp::Ordering::Greater => a,
            std::cmp::Ordering::Less => b,
            // Penalties: a coin with a little weight for the better coached.
            std::cmp::Ordering::Equal => {
                if rng.chance(0.5 + (coaching(w, a) - coaching(w, b)) / 100.0) { a } else { b }
            }
        };
        next.push(through);
        i += 2;
    }
    if i < alive.len() {
        next.push(alive[i]);
    }
    let c = &mut w.minor.comps[comp as usize];
    // The runner-up is the last side knocked out in the final.
    if next.len() == 1 {
        c.table = alive.iter().map(|&e| (e, Row::default())).collect();
        c.done = true;
    }
    c.alive = next;
}

fn ranked(c: &MinorComp) -> Vec<Entrant> {
    if c.kind.is_cup() {
        let winner = c.alive.first().copied();
        let mut v: Vec<Entrant> = winner.into_iter().collect();
        v.extend(c.table.iter().map(|x| x.0).filter(|&e| Some(e) != winner));
        return v;
    }
    let mut t = c.table.clone();
    t.sort_by(|a, b| {
        let (x, y) = (&a.1, &b.1);
        y.pts.cmp(&x.pts).then((i32::from(y.gf) - i32::from(y.ga)).cmp(&(i32::from(x.gf) - i32::from(x.ga)))).then(y.gf.cmp(&x.gf)).then(a.0.cmp(&b.0))
    });
    t.into_iter().map(|x| x.0).collect()
}

/// June (or when a new season is drawn): history, honours, notice, pyramid.
pub fn season_end(w: &mut World) {
    let today = w.date;
    if w.minor.comps.is_empty() {
        return;
    }
    let comps = std::mem::take(&mut w.minor.comps);
    let lines = std::mem::take(&mut w.minor.lines);
    for c in &comps {
        if c.entrants.len() < 2 {
            continue;
        }
        let order = ranked(c);
        let (Some(&winner), runner_up) = (order.first(), order.get(1).copied()) else { continue };
        let played: Vec<(PlayerId, MinorLine)> = lines.iter().filter(|((_, id), _)| *id == c.id).map(|((p, _), l)| (*p, *l)).collect();
        let top = played.iter().max_by(|a, b| a.1.goals.cmp(&b.1.goals).then(b.0.cmp(&a.0))).map_or((PlayerId::NONE, 0), |x| (x.0, x.1.goals));
        let best = played.iter().filter(|x| x.1.apps >= 3).max_by(|a, b| (a.1.rating / u32::from(a.1.apps)).cmp(&(b.1.rating / u32::from(b.1.apps))).then(b.0.cmp(&a.0))).map_or(PlayerId::NONE, |x| x.0);
        let season = MinorSeason {
            kind: c.kind,
            nation: c.nation,
            region: c.region.clone(),
            season: c.season,
            winner,
            runner_up: runner_up.unwrap_or(winner),
            top_scorer: top.0,
            top_goals: top.1,
            best,
            biggest: w.minor.biggest.get(&c.id).copied(),
        };
        let hidx = w.minor.history.len() as u32;
        w.minor.history.push(season);
        w.events.push(today, Visibility::Public, EventKind::MinorTitle { history: hidx });
        crate::records::minor_season(w, hidx);
        // Standout players are noticed by professional clubs nearby.
        for p in [top.0, best] {
            if p.is_some() {
                noticed(w, p, c.nation);
            }
        }
        // The amateur pyramid moves through standing.
        if matches!(c.kind, MinorKind::AmateurLeague { .. }) {
            let n = order.len();
            for (i, e) in order.iter().enumerate() {
                if let Entrant::Local(l) = *e {
                    let s = &mut w.youth.local[l].standing;
                    if i < 2 {
                        *s = (*s + 60).min(1000);
                    } else if i + 2 >= n {
                        *s = s.saturating_sub(60);
                    }
                }
            }
        }
    }
    // Lines become careers.
    let mut by_player: Vec<(PlayerId, MinorLine)> = lines.into_iter().map(|((p, _), l)| (p, l)).collect();
    by_player.sort_by_key(|x| (x.0, x.1.season));
    crate::records::minor_lines(w, &by_player);
    for (p, l) in by_player {
        let v = w.minor.careers.entry(p).or_default();
        v.push(l);
        if v.len() > 16 {
            v.remove(0);
        }
    }
}

/// Academies and scouts near a standout see them.
fn noticed(w: &mut World, p: PlayerId, nation: NationId) {
    let today = w.date;
    let clubs: Vec<ClubId> = w.youth.academies.values().filter(|a| w.clubs[a.club].nation == nation || matches!(a.reach, pw_world::youth::Reach::International)).map(|a| a.club).collect();
    for c in clubs {
        w.knowledge.observe(c, p, 180, today);
    }
}
