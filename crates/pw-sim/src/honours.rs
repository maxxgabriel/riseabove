//! Awards, records, milestones, legends and the hall of fame (11 §8).
//!
//! Everything here is computed from what happened: appearance tallies from
//! matches, fees from transfers, votes from the voters' own lenses. The
//! outputs are events and archive entries — and consequences: award winners
//! gain reputation and fame (and value), legends are loved by their fans for
//! good, record signings carry the weight of their fee.

use pw_core::{ClubId, CompId, Date, NationId, PersonId, PlayerId, PosGroup, StaffId};
use pw_world::event::{AwardKind, EventKind, MilestoneKind, RecordKind, Visibility};
use pw_world::history::AwardRecord;
use pw_world::honours::{Holder, Inductee, Vote};
use pw_world::intl::Level;
use pw_world::nation::Confed;
use pw_world::stats::StatLine;
use pw_world::{CompKind, FanReason, FxHashMap, PlayerStatus, TeamKind, World};

const CLUB_APPS: [u16; 5] = [100, 200, 300, 400, 500];
const CAREER_GOALS: [u16; 5] = [50, 100, 200, 300, 400];
const SENIOR_APPS: [u16; 4] = [250, 500, 750, 1000];
const CAPS: [u16; 4] = [25, 50, 100, 150];

// ---------------------------------------------------------------------------
// Per match
// ---------------------------------------------------------------------------

/// After a senior appearance: tallies, milestones and club records.
pub fn on_appearance(w: &mut World, p: PlayerId, club: ClubId, goals: u8) {
    let today = w.date;
    let t = w.honours.tallies.entry((club, p)).or_default();
    if t.apps == 0 {
        t.first = today;
    }
    t.apps += 1;
    t.goals += u16::from(goals);
    t.last = today;
    let (apps, club_goals) = (t.apps, t.goals);
    if CLUB_APPS.contains(&apps) {
        w.events.push(today, Visibility::Public, EventKind::Milestone { player: p, kind: MilestoneKind::ClubApps, count: apps, club });
    }
    let career_goals = w.players.cold[p].senior_goals;
    if goals > 0 {
        for &m in &CAREER_GOALS {
            if career_goals >= m && career_goals - u16::from(goals) < m {
                w.events.push(today, Visibility::Public, EventKind::Milestone { player: p, kind: MilestoneKind::CareerGoals, count: m, club });
            }
        }
    }
    let senior = w.players.cold[p].senior_apps;
    if SENIOR_APPS.contains(&senior) {
        w.events.push(today, Visibility::Public, EventKind::Milestone { player: p, kind: MilestoneKind::SeniorApps, count: senior, club });
    }
    // Club records.
    let rec = w.honours.clubs.entry(club).or_default();
    let mut broke: Option<(RecordKind, i64)> = None;
    if i64::from(club_goals) > rec.top_scorer.value && club_goals >= 20 {
        let was = rec.top_scorer.player;
        rec.top_scorer = Holder { player: p, value: i64::from(club_goals), date: today };
        if was != p {
            broke = Some((RecordKind::ClubTopScorer, i64::from(club_goals)));
        }
    }
    if i64::from(apps) > rec.most_apps.value && apps >= 100 {
        let was = rec.most_apps.player;
        rec.most_apps = Holder { player: p, value: i64::from(apps), date: today };
        if was != p && broke.is_none() {
            broke = Some((RecordKind::ClubMostApps, i64::from(apps)));
        }
    }
    if let Some((kind, value)) = broke {
        w.events.push(today, Visibility::Public, EventKind::RecordBroken { player: p, kind, club, value });
        let who = w.players.cold[p].person;
        w.media.move_fans(club, who, 120, FanReason::Loyalty, today);
    }
}

/// After a senior result: biggest wins.
pub fn on_result(w: &mut World, winner: ClubId, loser: ClubId, margin: u8) {
    if margin < 4 {
        return;
    }
    let today = w.date;
    let rec = w.honours.clubs.entry(winner).or_default();
    if i64::from(margin) > rec.biggest_win.value {
        let first = rec.biggest_win.value == 0;
        rec.biggest_win = Holder { player: PlayerId::NONE, value: i64::from(margin), date: today };
        rec.biggest_win_against = loser;
        if !first {
            w.events.push(today, Visibility::Public, EventKind::RecordBroken { player: PlayerId::NONE, kind: RecordKind::ClubBiggestWin, club: winner, value: i64::from(margin) });
        }
    }
}

/// After an international cap.
pub fn on_cap(w: &mut World, p: PlayerId, n: NationId) {
    let today = w.date;
    let caps = w.intl.caps_for(p, n, Level::Senior);
    let goals = w.intl.caps.get(&p).and_then(|v| v.iter().find(|c| c.nation == n && c.level == Level::Senior)).map_or(0, |c| c.goals);
    if CAPS.contains(&caps) {
        w.events.push(today, Visibility::Public, EventKind::Milestone { player: p, kind: MilestoneKind::Caps, count: caps, club: ClubId::NONE });
    }
    let rec = w.honours.nations.entry(n).or_default();
    let mut broke = Vec::new();
    if i64::from(caps) > rec.most_caps.value && caps >= 40 {
        if rec.most_caps.player != p {
            broke.push((RecordKind::NationMostCaps, i64::from(caps)));
        }
        rec.most_caps = Holder { player: p, value: i64::from(caps), date: today };
    }
    if i64::from(goals) > rec.top_scorer.value && goals >= 15 {
        if rec.top_scorer.player != p {
            broke.push((RecordKind::NationTopScorer, i64::from(goals)));
        }
        rec.top_scorer = Holder { player: p, value: i64::from(goals), date: today };
    }
    for (kind, value) in broke {
        w.events.push(today, Visibility::Public, EventKind::RecordBroken { player: p, kind, club: ClubId::NONE, value });
    }
}

/// After a transfer: record signings, sales, and the world record.
pub fn on_transfer(w: &mut World, p: PlayerId, buyer: ClubId, seller: ClubId, fee: i64) {
    if fee <= 0 {
        return;
    }
    let today = w.date;
    let mut events = Vec::new();
    if buyer.is_some() {
        let r = w.honours.clubs.entry(buyer).or_default();
        if fee > r.record_signing.value {
            let first = r.record_signing.value == 0;
            r.record_signing = Holder { player: p, value: fee, date: today };
            if !first {
                events.push((RecordKind::ClubRecordSigning, buyer));
            }
        }
    }
    if seller.is_some() {
        let r = w.honours.clubs.entry(seller).or_default();
        if fee > r.record_sale.value {
            let first = r.record_sale.value == 0;
            r.record_sale = Holder { player: p, value: fee, date: today };
            if !first {
                events.push((RecordKind::ClubRecordSale, seller));
            }
        }
    }
    if fee > w.honours.world_fee.value {
        let first = w.honours.world_fee.value == 0;
        w.honours.world_fee = Holder { player: p, value: fee, date: today };
        if !first {
            events.push((RecordKind::WorldRecordFee, buyer));
        }
    }
    for (kind, club) in events {
        w.events.push(today, Visibility::Public, EventKind::RecordBroken { player: p, kind, club, value: fee });
    }
    // A record fee is a weight: fans expect, the media watch.
    if w.honours.clubs.get(&buyer).is_some_and(|r| r.record_signing.player == p) {
        let who = w.players.cold[p].person;
        let l = &mut w.lives[who];
        l.stress = l.stress.saturating_add(5).min(100);
    }
}

// ---------------------------------------------------------------------------
// Season awards (called from season::archive_stats)
// ---------------------------------------------------------------------------

/// Awards beyond the scorer / player / young player trio: playmaker, golden
/// glove, team of the season, manager of the season, league scoring record.
pub fn season_awards(w: &mut World, c: CompId, year: i32, lines: &[StatLine], games: u16) {
    let date = w.date;
    let min_apps = (f32::from(games) * 0.6) as u16;
    let mut give = |w: &mut World, kind: AwardKind, l: &StatLine, value: f32| {
        w.history.awards.push(AwardRecord { comp: c, season: year, kind, player: l.player, club: l.club, value });
        w.events.push(date, Visibility::Public, EventKind::Award { player: l.player, comp: c, award: kind, season: year });
        award_effects(w, l.player, 1.0);
    };
    if let Some(l) = lines.iter().filter(|l| l.assists > 0).max_by_key(|l| (l.assists, l.key_passes, std::cmp::Reverse(l.minutes))) {
        give(w, AwardKind::Playmaker, l, f32::from(l.assists));
    }
    let keepers: Vec<&StatLine> = lines.iter().filter(|l| l.apps >= min_apps.max(1) && w.players.cold[l.player].best_pos.group() == PosGroup::Gk).collect();
    if let Some(l) = keepers.iter().max_by(|a, b| (a.clean_sheets, (a.avg_rating() * 100.0) as i32).cmp(&(b.clean_sheets, (b.avg_rating() * 100.0) as i32))) {
        give(w, AwardKind::GoldenGlove, *l, f32::from(l.clean_sheets));
    }
    // Team of the season: 1 keeper, 4 defenders, 3 midfielders, 3 forwards.
    let mut team: Vec<&StatLine> = Vec::new();
    for (group, n) in [(PosGroup::Gk, 1usize), (PosGroup::Def, 4), (PosGroup::Mid, 3), (PosGroup::Att, 3)] {
        let mut v: Vec<&StatLine> = lines.iter().filter(|l| l.apps >= min_apps.max(1) && w.players.cold[l.player].best_pos.group() == group).collect();
        v.sort_by(|a, b| b.avg_rating().total_cmp(&a.avg_rating()).then(a.player.cmp(&b.player)));
        team.extend(v.into_iter().take(n));
    }
    for l in team {
        give(w, AwardKind::TeamOfSeason, l, l.avg_rating());
    }
    // League scoring record.
    if let Some(top) = lines.iter().max_by_key(|l| l.goals) {
        let rec = w.honours.comps.entry(c).or_default();
        if i64::from(top.goals) > rec.goals_in_season.value {
            let first = rec.goals_in_season.value == 0;
            rec.goals_in_season = Holder { player: top.player, value: i64::from(top.goals), date };
            rec.goals_in_season_year = year;
            if !first {
                w.events.push(date, Visibility::Public, EventKind::RecordBroken { player: top.player, kind: RecordKind::LeagueGoalsInSeason, club: top.club, value: i64::from(top.goals) });
            }
        }
    }
    // Manager of the season: the champion's manager, unless someone beat
    // their board's target by far more.
    let rows = w.comps[c].state.table.clone();
    let mut best: Option<(StaffId, f32)> = None;
    for (pos, r) in rows.iter().enumerate() {
        let club = w.teams[r.team].club;
        let Some(m) = w.clubs[club].manager.get() else { continue };
        let target = f32::from(w.clubs[club].board.target_position.max(1));
        let over = target - (pos as f32 + 1.0) + if pos == 0 { 3.0 } else { 0.0 };
        if best.is_none_or(|b| over > b.1) {
            best = Some((m, over));
        }
    }
    if let Some((m, _)) = best {
        w.honours.managers_of_season.push((year, c, m));
        w.staff[m].reputation = w.staff[m].reputation.saturating_add(400).min(10_000);
        w.events.push(date, Visibility::Public, EventKind::ManagerOfSeason { staff: m, comp: c, season: year });
    }
}

/// What winning an award does to a player's standing.
fn award_effects(w: &mut World, p: PlayerId, weight: f32) {
    let who = w.players.cold[p].person;
    let c = &mut w.players.cold[p];
    c.rep.current = (f32::from(c.rep.current) + 150.0 * weight).min(10_000.0) as u16;
    c.rep.world = (f32::from(c.rep.world) + 200.0 * weight).min(10_000.0) as u16;
    let r = w.renown.people.entry(who).or_default();
    r.fame = (f32::from(r.fame) + 250.0 * weight).min(10_000.0) as u16;
    let h = &mut w.players.hot[p];
    h.morale = (h.morale + 6).min(100);
    h.confidence = (h.confidence + 6).min(100);
}

// ---------------------------------------------------------------------------
// Monthly: player of the month, legends, hall of fame
// ---------------------------------------------------------------------------

pub fn monthly(w: &mut World) {
    player_of_month(w);
    legends_and_hall(w);
}

/// Top-tier leagues name a player of the month from the month's appearances.
fn player_of_month(w: &mut World) {
    let today = w.date;
    let since = today.add_months(-1);
    let (year, month) = (since.year(), since.month() as u8);
    let leagues: Vec<CompId> = w.comps.iter_enumerated().filter(|(_, c)| c.kind == CompKind::League && c.tier == 1 && c.team_kind == TeamKind::First).map(|(id, _)| id).collect();
    for comp in leagues {
        let mut best: Option<(PlayerId, f32, ClubId)> = None;
        for (&p, apps) in &w.perf.recent {
            let month: Vec<_> = apps.iter().filter(|a| a.comp == comp && a.date >= since && a.date < today).collect();
            if month.len() < 3 {
                continue;
            }
            let avg = month.iter().map(|a| f32::from(a.rating)).sum::<f32>() / month.len() as f32 / 10.0;
            let goals: u8 = month.iter().map(|a| a.goals + a.assists).sum();
            let score = avg + f32::from(goals) * 0.08;
            if best.is_none_or(|b| score > b.1 || (score == b.1 && p < b.0)) {
                best = Some((p, score, month[0].club));
            }
        }
        if let Some((p, score, club)) = best {
            w.honours.monthly.push((year, month, comp, p));
            w.history.awards.push(AwardRecord { comp, season: year, kind: AwardKind::PlayerOfMonth, player: p, club, value: score });
            w.events.push(today, Visibility::Public, EventKind::Award { player: p, comp, award: AwardKind::PlayerOfMonth, season: year });
            award_effects(w, p, 0.4);
        }
    }
}

/// Club legends are recognised when a long, loved spell ends (or goes on
/// long enough); the hall of fame weighs whole careers after retirement.
fn legends_and_hall(w: &mut World) {
    let today = w.date;
    let candidates: Vec<((ClubId, PlayerId), pw_world::honours::Tally)> =
        w.honours.tallies.iter().filter(|(_, t)| t.apps >= 150 || t.goals >= 60).map(|(&k, &t)| (k, t)).collect();
    for ((club, p), t) in candidates {
        let who = w.players.cold[p].person;
        if w.honours.is_legend(club, who) {
            continue;
        }
        let fans = w.media.fan(club, who).map_or(0, |f| f.score);
        let trophies = w.history.honours.iter().filter(|h| h.club == club && h.season >= t.first.year() && h.season <= t.last.year()).count() as f32;
        let score = f32::from(t.apps) / 300.0 + f32::from(t.goals) / 120.0 + trophies * 0.25 + f32::from(fans) / 1000.0;
        let left = w.players.hot[p].club != club || w.players.hot[p].status == PlayerStatus::Retired;
        if score >= 1.2 && (left || t.apps >= 350) && fans > -100 {
            w.honours.clubs.entry(club).or_default().legends.push(who);
            w.events.push(today, Visibility::Public, EventKind::BecameLegend { person: who, club });
            w.media.move_fans(club, who, 300, FanReason::Loyalty, today);
        }
    }
    // Hall of fame: a year after retiring.
    let retired: Vec<PlayerId> = w
        .events
        .since(today.add_days(-395))
        .iter()
        .filter(|e| e.date <= today.add_days(-365))
        .filter_map(|e| if let EventKind::Retired { person } = e.kind { w.people[person].player.get() } else { None })
        .collect();
    for p in retired {
        let who = w.players.cold[p].person;
        if w.honours.in_hall(who) {
            continue;
        }
        let score = career_score(w, p);
        if score >= 1000 {
            w.honours.hall.push(Inductee { person: who, date: today, score });
            w.events.push(today, Visibility::Public, EventKind::InductedHallOfFame { person: who });
            let r = w.renown.people.entry(who).or_default();
            r.fame = r.fame.saturating_add(800).min(10_000);
        }
    }
}

/// A whole career in one number: trophies, awards, caps, appearances, peak.
pub fn career_score(w: &World, p: PlayerId) -> u32 {
    let trophies = w.honours.tallies.iter().filter(|((_, x), _)| *x == p).map(|(&(c, _), t)| {
        w.history.honours.iter().filter(|h| h.club == c && h.season >= t.first.year() && h.season <= t.last.year()).count() as u32
    }).sum::<u32>();
    let awards: u32 = w
        .history
        .awards
        .iter()
        .filter(|a| a.player == p)
        .map(|a| match a.kind {
            AwardKind::WorldPlayer { rank: 1 } => 400,
            AwardKind::WorldPlayer { .. } => 80,
            AwardKind::PlayerOfSeason | AwardKind::ContinentalPlayer(_) => 120,
            AwardKind::TopScorer | AwardKind::GoldenGlove | AwardKind::WorldYoungPlayer => 60,
            AwardKind::TeamOfSeason | AwardKind::Playmaker | AwardKind::YoungPlayerOfSeason => 25,
            AwardKind::PlayerOfMonth => 5,
        })
        .sum();
    let intl_wins = w.intl.tournaments.iter().filter(|t| t.winner.is_some()).filter(|t| {
        w.intl.caps.get(&p).is_some_and(|v| v.iter().any(|c| c.nation == t.winner && c.level == Level::Senior && c.last.year() >= t.year))
    }).count() as u32;
    let c = &w.players.cold[p];
    let peak = u32::from(w.renown.of(c.person).peak_world.max(c.rep.world)) / 20;
    trophies * 60 + awards + intl_wins * 250 + u32::from(c.caps) * 3 + u32::from(c.senior_apps) / 2 + peak
}

// ---------------------------------------------------------------------------
// Yearly: global votes (December)
// ---------------------------------------------------------------------------

/// The world player of the year: national team captains-and-managers and
/// journalists each vote for three players, each through their own lens.
pub fn yearly_votes(w: &mut World) {
    let today = w.date;
    let year = today.year();
    for young in [false, true] {
        let pool = candidates(w, year, young);
        if pool.len() < 3 {
            continue;
        }
        let mut tally: FxHashMap<PlayerId, u32> = FxHashMap::default();
        // National managers vote on performances and trophies.
        let managers: Vec<StaffId> = w.intl.sides.values().filter(|s| s.level == Level::Senior).map(|s| s.manager).collect();
        for m in managers {
            let key = u64::from(m.0);
            let mut ranked: Vec<(PlayerId, f32)> = pool.iter().map(|&(p, perf, trophies, _)| (p, perf * 1.0 + trophies * 0.35 + noise(w, key, p) * 0.25)).collect();
            vote(&mut ranked, &mut tally);
        }
        // Journalists vote on fame, goals and moments.
        let journalists: Vec<PersonId> = w.media.journalists.keys().copied().collect();
        for j in journalists {
            let key = u64::from(j.0) | 1 << 40;
            let mut ranked: Vec<(PlayerId, f32)> = pool.iter().map(|&(p, perf, trophies, fame)| (p, perf * 0.6 + trophies * 0.3 + fame * 1.2 + noise(w, key, p) * 0.3)).collect();
            vote(&mut ranked, &mut tally);
        }
        let mut ranking: Vec<(PlayerId, u32)> = tally.into_iter().collect();
        ranking.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        ranking.truncate(20);
        for (i, &(p, pts)) in ranking.iter().enumerate().take(if young { 1 } else { 3 }) {
            let kind = if young { AwardKind::WorldYoungPlayer } else { AwardKind::WorldPlayer { rank: i as u8 + 1 } };
            let club = w.players.hot[p].club;
            w.history.awards.push(AwardRecord { comp: CompId::NONE, season: year, kind, player: p, club, value: pts as f32 });
            w.events.push(today, Visibility::Public, EventKind::Award { player: p, comp: CompId::NONE, award: kind, season: year });
            award_effects(w, p, if i == 0 { 3.0 } else { 1.5 });
        }
        w.honours.votes.push(Vote { year, young, ranking });
    }
    continental_awards(w, year);
}

/// (player, performance, trophies, fame), normalised 0–~3.
fn candidates(w: &World, year: i32, young: bool) -> Vec<(PlayerId, f32, f32, f32)> {
    let mut v: Vec<(PlayerId, f32, f32, f32)> = w
        .players
        .ids()
        .filter(|&p| w.players.hot[p].status == PlayerStatus::Active && w.players.cold[p].rep.world >= if young { 2500 } else { 5000 })
        .filter(|&p| !young || w.age(p) <= 21)
        .filter_map(|p| {
            let s = w.perf.season(p, year).or_else(|| w.perf.season(p, year - 1))?;
            if s.apps < 15 {
                return None;
            }
            let perf = (s.avg() - 6.5).max(0.0) + f32::from(s.goals + s.assists) / 40.0;
            let club = w.players.hot[p].club;
            let trophies = w.history.honours.iter().filter(|h| h.club == club && (h.season == year || h.season == year - 1)).count() as f32
                + w.intl.tournaments.iter().filter(|t| t.year == year && t.winner.is_some() && w.intl.locked_to(p) == Some(t.winner)).count() as f32 * 1.5;
            let fame = f32::from(w.players.cold[p].rep.world) / 10_000.0 + f32::from(w.renown.of(w.players.cold[p].person).fame) / 20_000.0;
            Some((p, perf, trophies, fame))
        })
        .collect();
    v.sort_by(|a, b| (b.1 + b.2 + b.3).total_cmp(&(a.1 + a.2 + a.3)).then(a.0.cmp(&b.0)));
    v.truncate(60);
    v
}

fn noise(w: &World, key: u64, p: PlayerId) -> f32 {
    pw_core::rng::noise(&[w.seed, key, u64::from(p.0), 0xba11])
}

fn vote(ranked: &mut [(PlayerId, f32)], tally: &mut FxHashMap<PlayerId, u32>) {
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    for (i, pts) in [5u32, 3, 1].iter().enumerate() {
        if let Some(&(p, _)) = ranked.get(i) {
            *tally.entry(p).or_default() += pts;
        }
    }
}

/// Best player at each confederation's clubs, by performance and trophies.
fn continental_awards(w: &mut World, year: i32) {
    let today = w.date;
    let pool = candidates(w, year, false);
    for confed in Confed::ALL {
        let best = pool
            .iter()
            .filter(|(p, ..)| {
                let club = w.players.hot[*p].club;
                club.is_some() && w.nations[w.clubs[club].nation].confed == confed
            })
            .max_by(|a, b| (a.1 + a.2 * 0.5).total_cmp(&(b.1 + b.2 * 0.5)).then(b.0.cmp(&a.0)))
            .map(|x| x.0);
        if let Some(p) = best {
            let kind = AwardKind::ContinentalPlayer(confed);
            let club = w.players.hot[p].club;
            w.history.awards.push(AwardRecord { comp: CompId::NONE, season: year, kind, player: p, club, value: 0.0 });
            w.events.push(today, Visibility::Public, EventKind::Award { player: p, comp: CompId::NONE, award: kind, season: year });
            award_effects(w, p, 1.5);
            let who = w.players.cold[p].person;
            let r = w.renown.people.entry(who).or_default();
            r.continental = r.continental.saturating_add(700).min(10_000);
        }
    }
}

/// How much a player is part of a club's story (for fans, media, legends).
pub fn club_bond(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let t = w.honours.tally(club, p);
    let years = t.first.days_until(if t.last == Date(0) { w.date } else { t.last }) as f32 / 365.0;
    f32::from(t.apps) / 200.0 + years / 8.0 + if w.honours.is_legend(club, w.players.cold[p].person) { 1.0 } else { 0.0 }
}
