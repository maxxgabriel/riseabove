//! Football culture (11 §7): club identities, national trends, rivalries
//! that grow from history, and the meaning of a fixture.
//!
//! Everything is seeded from the world seed (`stream::CULTURE`) and then
//! moved by what happens: meetings, eliminations, title races, relegation
//! fights, players and managers crossing between rivals, academy graduates,
//! trophies and droughts. Match meaning is computed on demand and read by
//! matchday (stakes), the press (newsworthiness and questions), supporters
//! and social media (salience and memories).

use pw_core::rng::{noise, stream};
use pw_core::{ClubId, CompId, Date, EventId, NationId, PersonId, PlayerId, StaffId, TeamId};
use pw_world::culture::{ClubCulture, Identity, MatchMeaning, Moment, MomentKind, NationCulture, RivalryKind, Side};
use pw_world::event::{Cause, Causes, EventKind, Visibility};
use pw_world::{CompKind, Fixture, FxHashMap, TeamKind, World};

fn n01(w: &World, keys: &[u64]) -> f32 {
    let mut k: Vec<u64> = vec![w.seed, stream::CULTURE];
    k.extend_from_slice(keys);
    noise(&k)
}

fn pct(v: f32) -> u8 {
    v.round().clamp(0.0, 100.0) as u8
}

pub fn side_of_team(w: &World, t: TeamId) -> Option<Side> {
    (t.is_some() && w.teams[t].kind == TeamKind::First).then(|| Side::Club(w.teams[t].club))
}

// ---------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------

pub fn ensure(w: &mut World) {
    let today = w.date;
    let nations: Vec<NationId> = w.nations.ids().filter(|n| !w.culture.nations.contains_key(n)).collect();
    for n in nations {
        let k = u64::from(n.0);
        let rep = f32::from(w.nations[n].reputation) / 100.0;
        let econ = w.nations[n].economy;
        let fervour = 40.0 + rep * 0.45 + n01(w, &[k, 1]) * 18.0;
        let c = NationCulture {
            fervour: pct(fervour),
            media_intensity: pct(35.0 + econ * 30.0 + n01(w, &[k, 2]) * 18.0),
            patience: pct(62.0 - fervour / 3.0 + n01(w, &[k, 3]) * 12.0),
            youth_faith: pct(30.0 + f32::from(w.nations[n].youth_rating) * 2.0 + n01(w, &[k, 4]) * 15.0),
            trend_press: pct(50.0 + n01(w, &[k, 5]) * 25.0),
            trend_tempo: pct(50.0 + n01(w, &[k, 6]) * 25.0),
            trend_direct: pct(50.0 + n01(w, &[k, 7]) * 25.0),
            trend_since: today,
        };
        w.culture.nations.insert(n, c);
    }
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|c| !w.culture.clubs.contains_key(c)).collect();
    for club in clubs {
        let k = u64::from(club.0);
        let c = &w.clubs[club];
        let nc = w.culture.nation(c.nation);
        let rep = f32::from(c.reputation) / 100.0;
        let academy = w.youth.academies.contains_key(&club);
        let flair = 45.0 + n01(w, &[k, 11]) * 30.0;
        let culture = ClubCulture {
            identity: Identity {
                youth: pct(20.0 + f32::from(c.facilities.youth) * 2.5 + if academy { 10.0 } else { 0.0 } + n01(w, &[k, 12]) * 15.0),
                local: pct(35.0 + (60.0 - rep).max(0.0) * 0.3 + n01(w, &[k, 13]) * 20.0),
                flair: pct(flair),
                grit: pct(100.0 - flair + n01(w, &[k, 14]) * 10.0),
                underdog: pct((70.0 - rep).max(0.0) + n01(w, &[k, 15]) * 15.0),
                glamour: pct(rep * 0.9 + n01(w, &[k, 16]) * 10.0),
            },
            discipline: pct(45.0 + n01(w, &[k, 17]) * 30.0),
            expectations: pct(rep),
            patience: pct(f32::from(nc.patience) + n01(w, &[k, 18]) * 15.0),
            tribalism: pct(f32::from(nc.fervour) * 0.6 + n01(w, &[k, 19]) * 20.0 + 15.0),
            graduates: 0,
            drought: 0,
        };
        w.culture.clubs.insert(club, culture);
    }
    seed_rivalries(w);
}

fn seed_rivalries(w: &mut World) {
    let today = w.date;
    if !w.culture.rivalries.list.is_empty() {
        return;
    }
    // Derbies: clubs sharing a city.
    let mut by_city: FxHashMap<(NationId, String), Vec<ClubId>> = FxHashMap::default();
    for (id, c) in w.clubs.iter_enumerated() {
        if !c.city.is_empty() && c.league.is_some() {
            by_city.entry((c.nation, c.city.clone())).or_default().push(id);
        }
    }
    let mut cities: Vec<((NationId, String), Vec<ClubId>)> = by_city.into_iter().filter(|(_, v)| v.len() >= 2).collect();
    cities.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, mut clubs) in cities {
        clubs.sort();
        for (i, &a) in clubs.iter().enumerate() {
            for &b in &clubs[i + 1..] {
                let base = 70.0 + n01(w, &[u64::from(a.0), u64::from(b.0), 21]) * 20.0;
                // Clubs far apart in stature have a lopsided, cooler derby.
                let gap = (f32::from(w.clubs[a].reputation) - f32::from(w.clubs[b].reputation)).abs() / 200.0;
                w.culture.rivalries.ensure(Side::Club(a), Side::Club(b), RivalryKind::Derby, pct(base - gap), today);
            }
        }
    }
    // The leading clubs of each top league: historic rivals.
    let leagues: Vec<CompId> = w.comps.iter_enumerated().filter(|(_, c)| c.kind == CompKind::League && c.tier == 1 && c.team_kind == TeamKind::First).map(|(id, _)| id).collect();
    for comp in leagues {
        let mut clubs: Vec<ClubId> = w.comps[comp].state.entrants.iter().map(|&t| w.teams[t].club).collect();
        clubs.sort_by(|&a, &b| w.clubs[b].reputation.cmp(&w.clubs[a].reputation).then(a.cmp(&b)));
        clubs.truncate(4);
        for (i, &a) in clubs.iter().enumerate() {
            for &b in &clubs[i + 1..] {
                let v = 40.0 + n01(w, &[u64::from(a.0), u64::from(b.0), 22]) * 15.0 - (i as f32) * 4.0;
                w.culture.rivalries.ensure(Side::Club(a), Side::Club(b), RivalryKind::Historic, pct(v), today);
            }
        }
    }
    // Nations: neighbours in standing within a confederation.
    for confed in pw_world::Confed::ALL {
        let mut ns: Vec<NationId> = w.nations.iter_enumerated().filter(|(_, n)| n.confed == confed).map(|(id, _)| id).collect();
        ns.sort_by(|&a, &b| w.nations[b].reputation.cmp(&w.nations[a].reputation).then(a.cmp(&b)));
        ns.truncate(8);
        for pair in ns.windows(2) {
            let v = 35.0 + n01(w, &[u64::from(pair[0].0), u64::from(pair[1].0), 23]) * 25.0;
            w.culture.rivalries.ensure(Side::Nation(pair[0]), Side::Nation(pair[1]), RivalryKind::International, pct(v), today);
        }
    }
    sync_media(w);
}

/// Keep the press's quick lookup in step with the rivalry registry.
fn sync_media(w: &mut World) {
    let pairs: Vec<(ClubId, ClubId, u8)> = w
        .culture
        .rivalries
        .list
        .iter()
        .filter_map(|r| match (r.a, r.b) {
            (Side::Club(a), Side::Club(b)) => Some((a, b, r.intensity)),
            _ => None,
        })
        .collect();
    for (a, b, v) in pairs {
        w.media.rivals.insert((a, b), v);
        w.media.rivals.insert((b, a), v);
    }
}

// ---------------------------------------------------------------------------
// History moves rivalries
// ---------------------------------------------------------------------------

/// A rivalry begins, or takes on a character it did not have: an event of its own, caused by what made it (the tie, the title race, the
/// scrap), so that what the press and the supporters later say about it traces back to something that happened.
fn kindle(w: &mut World, a: Side, b: Side, kind: RivalryKind, intensity: u8, because: Causes) {
    let today = w.date;
    let new = w.culture.rivalries.get(a, b).is_none_or(|r| !r.kinds.contains(&kind));
    w.culture.rivalries.ensure(a, b, kind, intensity, today);
    if new {
        w.events.push_caused(today, Visibility::Public, EventKind::RivalryKindled { a, b, kind }, because);
    }
}

fn remember(w: &mut World, a: Side, b: Side, m: Moment) {
    if let Some(r) = w.culture.rivalries.get_mut(a, b) {
        r.moments.push(m);
        if r.moments.len() > 6 {
            r.moments.remove(0);
        }
    }
}

/// After every senior result between two clubs (or nations).
pub fn after_result(w: &mut World, fx: &Fixture, hg: u8, ag: u8, pens: Option<(u8, u8)>, ev: EventId) {
    let today = w.date;
    let (Some(h), Some(a)) = (side_of_team(w, fx.home), side_of_team(w, fx.away)) else { return };
    let winner = match hg.cmp(&ag) {
        std::cmp::Ordering::Greater => Some(h),
        std::cmp::Ordering::Less => Some(a),
        std::cmp::Ordering::Equal => pens.map(|(x, y)| if x > y { h } else { a }),
    };
    let margin = hg.abs_diff(ag);
    let knockout = fx.decisive && w.comps[fx.comp].kind != CompKind::League;
    let both_big = |w: &World| {
        let big = |c: ClubId| {
            let l = w.clubs[c].league;
            l.is_some() && w.comps[l].tier <= 2
        };
        matches!((h, a), (Side::Club(x), Side::Club(y)) if big(x) && big(y))
    };
    if knockout && winner.is_some() && both_big(w) && w.culture.rivalries.get(h, a).is_none() {
        let because = if ev.is_some() { pw_world::causes![Cause::Event(ev)] } else { Causes::new() };
        kindle(w, h, a, RivalryKind::CupRevenge, 20, because);
    }
    let Some(r) = w.culture.rivalries.get_mut(h, a) else { return };
    let a_is_home = r.a == h;
    match hg.cmp(&ag) {
        std::cmp::Ordering::Greater => {
            if a_is_home {
                r.h2h.0 += 1
            } else {
                r.h2h.2 += 1
            }
        }
        std::cmp::Ordering::Less => {
            if a_is_home {
                r.h2h.2 += 1
            } else {
                r.h2h.0 += 1
            }
        }
        std::cmp::Ordering::Equal => r.h2h.1 += 1,
    }
    r.last_meeting = today;
    let mut bump: u8 = 1;
    if margin >= 4 {
        bump += 3;
    }
    // Revenge taken (or not).
    if let (Some(due), Some(win)) = (r.revenge_due, winner)
        && due == win
    {
        r.revenge_due = None;
        bump += 2;
    }
    if knockout && let Some(win) = winner {
        r.revenge_due = Some(if win == h { a } else { h });
        if !r.kinds.contains(&RivalryKind::CupRevenge) {
            r.kinds.push(RivalryKind::CupRevenge);
        }
        bump += 3;
    }
    r.intensity = r.intensity.saturating_add(bump).min(100);
    let memorable = margin >= 3 || r.intensity >= 60 || knockout;
    let intensity = r.intensity;
    if memorable {
        let kind = if knockout { winner.map_or(MomentKind::Meeting { winner, margin }, |s| MomentKind::Elimination { winner: s }) } else { MomentKind::Meeting { winner, margin } };
        remember(w, h, a, Moment { date: today, kind, event: ev });
    }
    if let (Side::Club(x), Side::Club(y)) = (h, a) {
        w.media.rivals.insert((x, y), intensity);
        w.media.rivals.insert((y, x), intensity);
    }
}

/// A player moved directly between two clubs.
pub fn on_transfer(w: &mut World, p: PlayerId, from: ClubId, to: ClubId, ev: EventId) {
    let today = w.date;
    let (a, b) = (Side::Club(from), Side::Club(to));
    let Some(r) = w.culture.rivalries.get_mut(a, b) else { return };
    if r.intensity < 30 {
        return;
    }
    if !r.kinds.contains(&RivalryKind::BadBlood) {
        r.kinds.push(RivalryKind::BadBlood);
    }
    r.intensity = r.intensity.saturating_add(4).min(100);
    remember(w, a, b, Moment { date: today, kind: MomentKind::Transfer { player: p, to: b }, event: ev });
}

/// A manager took a job at a rival of a former club.
pub fn on_manager_move(w: &mut World, staff: StaffId, to: ClubId, ev: EventId) {
    let today = w.date;
    crate::evolution::on_appointed(w, staff);
    let former: Vec<ClubId> = w.careers.managers.get(&staff).map(|p| p.jobs.iter().map(|j| j.club).filter(|&c| c != to).collect()).unwrap_or_default();
    for from in former {
        let (a, b) = (Side::Club(from), Side::Club(to));
        let Some(r) = w.culture.rivalries.get_mut(a, b) else { continue };
        if r.intensity < 40 {
            continue;
        }
        r.intensity = r.intensity.saturating_add(5).min(100);
        if !r.kinds.contains(&RivalryKind::BadBlood) {
            r.kinds.push(RivalryKind::BadBlood);
        }
        remember(w, a, b, Moment { date: today, kind: MomentKind::ManagerMove { staff, to: b }, event: ev });
    }
}

/// When a league closes: title races and relegation fights become rivalries.
pub fn season_end(w: &mut World, comp: CompId, rows: &[pw_world::TableRow]) {
    let today = w.date;
    if rows.len() < 4 || w.comps[comp].team_kind != TeamKind::First {
        return;
    }
    let tier = w.comps[comp].tier;
    let clubs_of: pw_world::FxHashMap<TeamId, ClubId> = rows.iter().map(|r| (r.team, w.teams[r.team].club)).collect();
    let club = |t: TeamId| clubs_of[&t];
    // Title (tier 1) or promotion (lower tiers) decided by a few points.
    let (first, second) = (rows[0], rows[1]);
    if first.points - second.points <= 3 {
        let kind = if tier == 1 { RivalryKind::TitleRace } else { RivalryKind::Promotion };
        let (a, b) = (Side::Club(club(first.team)), Side::Club(club(second.team)));
        let decided = w.events.latest_where(today, 30, |e| matches!(e.kind, EventKind::Champion { comp: c, .. } if c == comp));
        let because = decided.map_or_else(Causes::new, |id| pw_world::causes![Cause::Event(id)]);
        kindle(w, a, b, kind, 30, because);
        let r = w.culture.rivalries.ensure(a, b, kind, 30, today);
        r.intensity = r.intensity.saturating_add(8).min(100);
        remember(w, a, b, Moment { date: today, kind: MomentKind::TitleDecided { winner: a }, event: EventId::NONE });
    }
    // The last two above and below the drop, separated by a whisker.
    let n = rows.len();
    let relegate = usize::from(w.comps[comp].relegate);
    if relegate > 0 && relegate < n {
        let safe = rows[n - relegate - 1];
        let down = rows[n - relegate];
        if safe.points - down.points <= 2 {
            let (a, b) = (Side::Club(club(safe.team)), Side::Club(club(down.team)));
            let dropped = w.events.latest_where(today, 30, |e| matches!(e.kind, EventKind::Relegated { team, .. } if team == down.team));
            let because = dropped.map_or_else(Causes::new, |id| pw_world::causes![Cause::Event(id)]);
            kindle(w, a, b, RivalryKind::Relegation, 25, because);
            let r = w.culture.rivalries.ensure(a, b, RivalryKind::Relegation, 25, today);
            r.intensity = r.intensity.saturating_add(6).min(100);
        }
    }
    sync_media(w);
}

/// July: trends drift, identities and expectations follow what happened,
/// rivalries without meetings cool.
pub fn yearly(w: &mut World) {
    let today = w.date;
    let year = today.year() as u64;
    let nations: Vec<NationId> = w.culture.nations.keys().copied().collect();
    for n in nations {
        let k = u64::from(n.0);
        let drift = |x: u8, salt: u64, w: &World| pct(f32::from(x) + n01(w, &[k, year, salt]) * 7.0);
        let c = w.culture.nations[&n];
        let press = drift(c.trend_press, 31, w);
        let tempo = drift(c.trend_tempo, 32, w);
        let direct = drift(c.trend_direct, 33, w);
        let shifted = press.abs_diff(c.trend_press) + tempo.abs_diff(c.trend_tempo) + direct.abs_diff(c.trend_direct) >= 12;
        let e = w.culture.nations.get_mut(&n).expect("nation");
        e.trend_press = press;
        e.trend_tempo = tempo;
        e.trend_direct = direct;
        if shifted {
            e.trend_since = today;
        }
    }
    // Academy graduates who debuted in the last year.
    let mut grads: FxHashMap<ClubId, u16> = FxHashMap::default();
    for e in w.events.since(today.add_days(-365)) {
        if let EventKind::Debut { player, team, .. } = e.kind {
            let club = w.teams[team].club;
            if w.players.cold[player].youth_club == club {
                *grads.entry(club).or_default() += 1;
            }
        }
    }
    let clubs: Vec<ClubId> = w.culture.clubs.keys().copied().collect();
    for club in clubs {
        let rep = f32::from(w.clubs[club].reputation) / 100.0;
        let recent_trophies = w.history.honours.iter().filter(|h| h.club == club && h.season >= today.year() - 5).count() as f32;
        let won_last = w.history.honours.iter().any(|h| h.club == club && h.season >= today.year() - 1);
        let g = grads.get(&club).copied().unwrap_or(0);
        let c = w.culture.clubs.get_mut(&club).expect("club culture");
        c.graduates = c.graduates.saturating_add(g);
        c.identity.youth = pct(f32::from(c.identity.youth) * 0.9 + (30.0 + f32::from(c.graduates.min(30)) * 2.0) * 0.1);
        c.expectations = pct(f32::from(c.expectations) * 0.7 + (rep + recent_trophies * 5.0) * 0.3);
        c.drought = if won_last { 0 } else { c.drought.saturating_add(1) };
        // Long droughts make supporters restless; success makes them demanding.
        if c.drought >= 10 {
            c.patience = c.patience.saturating_sub(1);
        }
    }
    for r in w.culture.rivalries.list.iter_mut() {
        let quiet = r.last_meeting.days_until(today) > 3 * 365;
        let floor = if r.kinds.contains(&RivalryKind::Derby) { 50 } else { 5 };
        if quiet && r.intensity > floor {
            r.intensity -= 1;
        }
    }
    sync_media(w);
}

/// The fashionable style blended into a newly generated manager's philosophy.
pub fn fashion(w: &World, nation: NationId, press: u8, tempo: u8, direct: u8) -> (u8, u8, u8) {
    let Some(c) = w.culture.nations.get(&nation) else { return (press, tempo, direct) };
    let blend = |own: u8, trend: u8| ((f32::from(own) * 0.6 + f32::from(trend) * 0.4).round()) as u8;
    (blend(press, c.trend_press), blend(tempo, c.trend_tempo), blend(direct, c.trend_direct))
}

// ---------------------------------------------------------------------------
// Meaning
// ---------------------------------------------------------------------------

/// What a fixture means beyond the points.
pub fn meaning(w: &World, fx: &Fixture) -> MatchMeaning {
    let mut m = MatchMeaning::default();
    let (Some(h), Some(a)) = (side_of_team(w, fx.home), side_of_team(w, fx.away)) else { return m };
    if let Some(r) = w.culture.rivalries.get(h, a) {
        m.rivalry = r.intensity;
        m.derby = r.has(RivalryKind::Derby);
        m.revenge = r.revenge_due;
    }
    let comp = &w.comps[fx.comp];
    if comp.kind == CompKind::League && comp.state.end.days_until(fx.date) > -70 {
        let rows = comp.sorted_table();
        let pos = |t: TeamId| rows.iter().position(|r| r.team == t);
        let pts = |t: TeamId| rows.iter().find(|r| r.team == t).map_or(0, |r| r.points);
        if let (Some(ph), Some(pa)) = (pos(fx.home), pos(fx.away)) {
            let n = rows.len();
            let close = (pts(fx.home) - pts(fx.away)).abs() <= 6;
            m.title_race = comp.tier == 1 && ph < 3 && pa < 3 && close;
            m.promotion = comp.tier > 1 && ph < 4 && pa < 4 && close;
            m.relegation = ph + 4 >= n && pa + 4 >= n;
        }
    }
    // People returning to a former club.
    let clubs = [w.teams[fx.home].club, w.teams[fx.away].club];
    for (i, &team) in [fx.home, fx.away].iter().enumerate() {
        let other = clubs[1 - i];
        for &p in &w.teams[team].squad {
            let came_from = w.history.spells.get(&p).is_some_and(|v| v.iter().any(|s| s.club == other && !s.loan && s.to.is_some_and(|d| d.days_until(w.date) < 6 * 365)));
            if came_from && m.returns.len() < 3 {
                m.returns.push((w.players.cold[p].person, other));
            }
        }
        if let Some(mgr) = w.clubs[clubs[i]].manager.get()
            && w.careers.managers.get(&mgr).is_some_and(|pr| pr.jobs.iter().any(|j| j.club == other))
            && m.returns.len() < 3
        {
            m.returns.push((w.staff[mgr].person, other));
        }
    }
    let s = f32::from(m.rivalry) * 0.6
        + if m.title_race { 25.0 } else { 0.0 }
        + if m.relegation { 20.0 } else { 0.0 }
        + if m.promotion { 15.0 } else { 0.0 }
        + if m.revenge.is_some() { 10.0 } else { 0.0 }
        + m.returns.len() as f32 * 5.0;
    m.significance = pct(s);
    m
}

/// Extra stakes a fixture's meaning adds to its importance (0–0.2).
pub fn stakes(w: &World, fx: &Fixture) -> f32 {
    f32::from(meaning(w, fx).significance) / 500.0
}

/// A person's history with a club as the terraces see it (for posts and stories).
pub fn returning(w: &World, person: PersonId, club: ClubId) -> bool {
    let p = w.people[person].player;
    p.is_some() && w.history.spells.get(&p).is_some_and(|v| v.iter().any(|s| s.club == club && s.to.is_some()))
}

/// The date of the last meeting between two sides, if they are rivals.
pub fn last_meeting(w: &World, a: Side, b: Side) -> Option<Date> {
    w.culture.rivalries.get(a, b).map(|r| r.last_meeting).filter(|d| d.0 > 0)
}
