//! Season lifecycle per nation and per confederation: start (entrants,
//! fixtures, budgets), knockout progression, and finish (tables, honours,
//! promotion/relegation, prizes, awards).

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, CompId, Date, NationId, TeamId, Weekday};
use pw_world::calendar::{current_season, season};
use pw_world::comp::{Stage, sort_table};
use pw_world::event::{AwardKind, EventKind, Visibility};
use pw_world::history::{ArchivedTable, AwardRecord, Honour};
use pw_world::nation::Confed;
use pw_world::{CompKind, Format, TeamKind, World};

use crate::schedule;
use crate::{board, finance};

pub fn daily(w: &mut World) {
    let today = w.date;
    for n in w.nations.ids() {
        let cal = w.data.calendars[usize::from(w.nations[n].calendar)].clone();
        let cur = current_season(&cal, today);
        if w.nations[n].season.year == 0 {
            w.nations[n].season = cur;
            start_nation(w, n);
            continue;
        }
        let end = w.nations[n].season.end;
        if today > end && !nation_finished(w, n) {
            finish_nation(w, n);
        }
        if cur.year > w.nations[n].season.year {
            if !nation_finished(w, n) {
                finish_nation(w, n);
            }
            w.nations[n].season = cur;
            start_nation(w, n);
        }
    }
    for confed in Confed::ALL {
        continental(w, confed);
    }
    advance_knockouts(w);
}

fn nation_comps(w: &World, n: NationId) -> Vec<CompId> {
    w.comps.iter_enumerated().filter(|(_, c)| c.nation == n).map(|(id, _)| id).collect()
}

fn nation_finished(w: &World, n: NationId) -> bool {
    w.nations[n].leagues.iter().all(|&c| w.comps[c].state.stage == Stage::Finished || w.comps[c].state.season != w.nations[n].season.year)
}

fn reset_state(w: &mut World, c: CompId, year: i32, start: Date, end: Date) {
    let s = &mut w.comps[c].state;
    s.season = year;
    s.table.clear();
    s.ties.clear();
    s.round = 0;
    s.round_dates.clear();
    s.winner = TeamId::NONE;
    s.runner_up = TeamId::NONE;
    s.stage = Stage::NotStarted;
    s.start = start;
    s.end = end;
}

pub fn start_nation(w: &mut World, n: NationId) {
    let today = w.date;
    let s = w.nations[n].season.clone();
    let from = s.start.max(today.add_days(2));
    let mut rng = Rng::keyed(&[w.seed, stream::DRAW, u64::from(n.0), s.year as u64]);
    for c in nation_comps(w, n) {
        if w.comps[c].state.season == s.year {
            continue;
        }
        reset_state(w, c, s.year, s.start, s.end);
        match w.comps[c].kind {
            CompKind::League => {
                schedule::schedule_league(w, c, from, s.end, s.winter_break);
                if w.comps[c].team_kind == TeamKind::First {
                    for t in w.comps[c].state.entrants.clone() {
                        let club = w.teams[t].club;
                        w.clubs[club].league = c;
                    }
                }
            }
            CompKind::Cup => {
                if w.comps[c].state.entrants.is_empty() {
                    let tiers = usize::from(w.comps[c].tier.max(1));
                    let teams: Vec<TeamId> = w.nations[n].leagues.iter().take(tiers).flat_map(|&l| w.comps[l].state.entrants.clone()).collect();
                    w.comps[c].state.entrants = teams;
                }
                let rounds = (w.comps[c].state.entrants.len().max(2) as f32).log2().ceil() as usize;
                let dates = schedule::cup_round_dates(w, from, s.end, rounds);
                w.comps[c].state.round_dates = dates.clone();
                if let Some(&d) = dates.first() {
                    schedule::draw_cup_opening(w, c, d.max(today.add_days(3)), &mut rng);
                }
            }
            CompKind::SuperCup | CompKind::Continental => {}
        }
    }
    for pid in w.players.ids() {
        let club = w.players.hot[pid].club;
        if club.is_some() && w.clubs[club].nation == n {
            w.players.hot[pid].yellows = 0;
        }
    }
    finance::season_budgets(w, n);
    board::set_targets(w, n);
}

pub fn finish_nation(w: &mut World, n: NationId) {
    let year = w.nations[n].season.year;
    let leagues = w.nations[n].leagues.clone();
    for c in nation_comps(w, n) {
        if w.comps[c].state.season != year || w.comps[c].state.stage == Stage::Finished {
            continue;
        }
        if w.comps[c].is_league() {
            close_league(w, c, year);
        } else if matches!(w.comps[c].kind, CompKind::Cup) {
            // Unfinished cups (short seasons) are closed without a winner.
            archive_stats(w, c, year);
            w.comps[c].state.stage = Stage::Finished;
        }
    }
    // Promotion and relegation between adjacent tiers.
    for pair in leagues.windows(2) {
        let (upper, lower) = (pair[0], pair[1]);
        // Swap the same number both ways so division sizes stay constant.
        let k = usize::from(match (w.comps[upper].relegate, w.comps[lower].promote) {
            (0, p) => p,
            (r, 0) => r,
            (r, p) => r.min(p),
        });
        let upper_rank: Vec<TeamId> = last_table(w, upper, year);
        let lower_rank: Vec<TeamId> = last_table(w, lower, year);
        if upper_rank.is_empty() || lower_rank.is_empty() || k == 0 {
            continue;
        }
        let promoted: Vec<TeamId> = lower_rank.iter().filter(|&&t| !is_b_team_blocked(w, t, upper)).take(k).copied().collect();
        let relegated: Vec<TeamId> = upper_rank.iter().rev().take(promoted.len()).copied().collect();
        let date = w.date;
        let mut new_upper: Vec<TeamId> = upper_rank.iter().copied().filter(|t| !relegated.contains(t)).collect();
        new_upper.extend(promoted.iter().copied());
        let mut new_lower: Vec<TeamId> = w.comps[lower].state.entrants.iter().copied().filter(|t| !promoted.contains(t)).collect();
        new_lower.extend(relegated.iter().copied());
        w.comps[upper].state.entrants = new_upper;
        w.comps[lower].state.entrants = new_lower;
        for &t in &promoted {
            w.events.push(date, Visibility::Public, EventKind::Promoted { comp: upper, team: t });
            w.comps[upper].state.last_moves.push((t, 1));
        }
        for &t in &relegated {
            w.events.push(date, Visibility::Public, EventKind::Relegated { comp: upper, team: t });
            w.comps[lower].state.last_moves.push((t, -1));
        }
    }
    crate::reputation::season_end(w, n);
}

/// A reserve side cannot be promoted into its first team's division (F6).
fn is_b_team_blocked(w: &World, t: TeamId, target: CompId) -> bool {
    let team = &w.teams[t];
    team.kind != TeamKind::First && w.comps[target].state.entrants.iter().any(|&o| w.teams[o].club == team.club)
}

fn last_table(w: &World, c: CompId, year: i32) -> Vec<TeamId> {
    w.history.tables.iter().rev().find(|t| t.comp == c && t.season == year).map(|t| t.rows.iter().map(|r| r.team).collect()).unwrap_or_default()
}

fn close_league(w: &mut World, c: CompId, year: i32) {
    let date = w.date;
    let mut rows = w.comps[c].state.table.clone();
    sort_table(&mut rows);
    w.comps[c].state.last_moves.clear();
    if let (Some(first), second) = (rows.first().copied(), rows.get(1).copied()) {
        let club = w.teams[first.team].club;
        w.comps[c].state.winner = first.team;
        w.comps[c].state.runner_up = second.map_or(TeamId::NONE, |r| r.team);
        w.history.honours.push(Honour { comp: c, season: year, team: first.team, club, runner_up: w.comps[c].state.runner_up });
        w.events.push(date, Visibility::Public, EventKind::Champion { comp: c, team: first.team, season: year });
        if let Some(m) = w.clubs[club].manager.get() {
            w.staff[m].record.trophies += 1;
        }
    }
    // Prize money by finishing position.
    let pool = w.comps[c].prize_pool;
    let n = rows.len().max(1) as f64;
    let weights: Vec<f64> = (0..rows.len()).map(|i| 1.0 - 0.8 * i as f64 / n).collect();
    let total: f64 = weights.iter().sum::<f64>().max(1e-9);
    for (row, wt) in rows.iter().zip(&weights) {
        let club = w.teams[row.team].club;
        let amount = (pool as f64 * wt / total) as i64;
        w.clubs[club].finance.balance += amount;
        w.clubs[club].finance.season_income += amount;
    }
    crate::culture::season_end(w, c, &rows);
    crate::records::league_season(w, c, &rows);
    w.history.tables.push(ArchivedTable { comp: c, season: year, rows });
    archive_stats(w, c, year);
    w.comps[c].state.stage = Stage::Finished;
}

/// Move season statistics into history and hand out individual awards.
fn archive_stats(w: &mut World, c: CompId, year: i32) {
    let lines = w.stats.take_comp(c);
    let date = w.date;
    let games = w.comps[c].state.table.iter().map(|r| u16::from(r.played)).max().unwrap_or(0);
    if w.comps[c].kind == CompKind::League && w.comps[c].team_kind == TeamKind::First && !lines.is_empty() {
        let mut award = |kind: AwardKind, pick: Option<&pw_world::stats::StatLine>, value: f32| {
            if let Some(l) = pick {
                w.history.awards.push(AwardRecord { comp: c, season: year, kind, player: l.player, club: l.club, value });
                w.events.push(date, Visibility::Public, EventKind::Award { player: l.player, comp: c, award: kind, season: year });
            }
        };
        let top = lines.iter().max_by_key(|l| (l.goals, l.assists, std::cmp::Reverse(l.minutes)));
        award(AwardKind::TopScorer, top, top.map_or(0.0, |l| f32::from(l.goals)));
        let min_apps = (f32::from(games) * 0.6) as u16;
        let best = lines.iter().filter(|l| l.apps >= min_apps.max(1)).max_by(|a, b| a.avg_rating().total_cmp(&b.avg_rating()));
        award(AwardKind::PlayerOfSeason, best, best.map_or(0.0, |l| l.avg_rating()));
        let young = lines
            .iter()
            .filter(|l| l.apps >= (min_apps / 2).max(1) && w.people[w.players.cold[l.player].person].dob.age_on(date) <= 21)
            .max_by(|a, b| a.avg_rating().total_cmp(&b.avg_rating()));
        award(AwardKind::YoungPlayerOfSeason, young, young.map_or(0.0, |l| l.avg_rating()));
        crate::honours::season_awards(w, c, year, &lines, games);
    }
    w.history.archive_lines(lines);
}

// ------------------------------------------------------------- knockouts

/// Advance group stages and knockout rounds whose fixtures are complete.
pub fn advance_knockouts(w: &mut World) {
    let today = w.date;
    for c in w.comps.ids() {
        let stage = w.comps[c].state.stage;
        match stage {
            Stage::Groups => {
                let open = w.fixtures.between(w.comps[c].state.start, today.add_days(400)).any(|f| {
                    let fx = w.fixtures.get(f);
                    fx.comp == c && fx.score.is_none() && fx.tie == u16::MAX
                });
                if open {
                    continue;
                }
                let Format::Groups { advance, ko_legs, .. } = w.comps[c].format else { continue };
                let mut rows = w.comps[c].state.table.clone();
                sort_table(&mut rows);
                let mut through = Vec::new();
                let mut g_count = std::collections::BTreeMap::<u8, u8>::new();
                for r in rows {
                    let e = g_count.entry(r.group).or_insert(0);
                    if *e < advance {
                        through.push(r.team);
                    }
                    *e += 1;
                }
                let date = next_round_date(w, c, today);
                let mut rng = Rng::keyed(&[w.seed, stream::DRAW, u64::from(c.0), date.0 as u64]);
                w.comps[c].state.ties.clear();
                schedule::draw_round(w, c, through, date, ko_legs, &mut rng);
            }
            Stage::Knockout(_) => {
                let ties = &w.comps[c].state.ties;
                if ties.is_empty() || !ties.iter().all(|t| t.is_decided()) {
                    continue;
                }
                let winners: Vec<TeamId> = ties.iter().map(|t| t.winner).collect();
                if winners.len() == 1 {
                    crown(w, c, winners[0]);
                    continue;
                }
                let legs = match w.comps[c].format {
                    Format::Knockout { legs, final_legs } | Format::Groups { ko_legs: legs, final_legs, .. } => {
                        if winners.len() == 2 { final_legs } else { legs }
                    }
                    Format::League { .. } => 1,
                };
                w.comps[c].state.round += 1;
                let date = next_round_date(w, c, today);
                let mut rng = Rng::keyed(&[w.seed, stream::DRAW, u64::from(c.0), date.0 as u64]);
                w.comps[c].state.ties.clear();
                schedule::draw_round(w, c, winners, date, legs, &mut rng);
            }
            _ => {}
        }
    }
}

fn next_round_date(w: &World, c: CompId, today: Date) -> Date {
    let s = &w.comps[c].state;
    let planned = s.round_dates.get(usize::from(s.round)).copied().unwrap_or(today.add_days(14));
    let earliest = today.add_days(3);
    if planned >= earliest { planned } else { earliest.next_weekday(Weekday::Wed) }
}

fn crown(w: &mut World, c: CompId, winner: TeamId) {
    let date = w.date;
    let year = w.comps[c].state.season;
    let runner = w
        .fixtures
        .between(date.add_days(-10), date)
        .map(|f| w.fixtures.get(f))
        .find(|f| f.comp == c && f.involves(winner))
        .map_or(TeamId::NONE, |f| f.opponent(winner));
    let club = w.teams[winner].club;
    w.comps[c].state.winner = winner;
    w.comps[c].state.runner_up = runner;
    w.comps[c].state.stage = Stage::Finished;
    w.history.honours.push(Honour { comp: c, season: year, team: winner, club, runner_up: runner });
    w.events.push(date, Visibility::Public, EventKind::Champion { comp: c, team: winner, season: year });
    if let Some(m) = w.clubs[club].manager.get() {
        w.staff[m].record.trophies += 1;
    }
    let prize = w.comps[c].prize_pool / 3;
    w.clubs[club].finance.balance += prize;
    crate::economy::record_continental(w, c);
    archive_stats(w, c, year);
}

// ------------------------------------------------------------- continental

/// Continental club competitions follow the confederation's leading nation's calendar.
fn continental(w: &mut World, confed: Confed) {
    let comps: Vec<CompId> = w.comps.iter_enumerated().filter(|(_, c)| c.kind == CompKind::Continental && c.confed == Some(confed)).map(|(id, _)| id).collect();
    if comps.is_empty() {
        return;
    }
    let Some(lead) = w.nations.iter_enumerated().filter(|(_, n)| n.confed == confed).max_by_key(|(_, n)| n.reputation).map(|(id, _)| id) else {
        return;
    };
    let cal = w.data.calendars[usize::from(w.nations[lead].calendar)].clone();
    let s = current_season(&cal, w.date);
    let group_start = s.start.add_days(35);
    if w.date < group_start.add_days(-10) || w.date > s.end {
        return;
    }
    for (rank, &c) in comps.iter().enumerate() {
        if w.comps[c].state.season == s.year {
            continue;
        }
        reset_state(w, c, s.year, group_start, s.end);
        let entrants = continental_entrants(w, c, confed, rank, s.year);
        w.comps[c].state.entrants = entrants;
        let mut rng = Rng::keyed(&[w.seed, stream::DRAW, u64::from(c.0), s.year as u64]);
        match w.comps[c].format {
            Format::Groups { groups, size, .. } => {
                let fit = w.comps[c].state.entrants.len() / usize::from(size.max(1));
                let groups = (fit.min(usize::from(groups))).max(1);
                // Shrink to a power of two so the knockout bracket is clean.
                let groups = if groups.is_power_of_two() { groups } else { groups.next_power_of_two() / 2 };
                if let Format::Groups { groups: g, .. } = &mut w.comps[c].format {
                    *g = groups as u8;
                }
                let md = schedule::spread_dates(w, group_start.max(w.date.add_days(3)), group_start.add_days(95), 6, Weekday::Tue, Weekday::Wed, None);
                schedule::schedule_groups(w, c, &md, &mut rng);
                let ko_rounds = (groups * 2).max(2).trailing_zeros() as usize;
                w.comps[c].state.round_dates = schedule::cup_round_dates(w, s.end.add_days(-120), s.end, ko_rounds);
            }
            Format::Knockout { legs, .. } => {
                let rounds = (w.comps[c].state.entrants.len().max(2) as f32).log2().ceil() as usize;
                w.comps[c].state.round_dates = schedule::cup_round_dates(w, group_start, s.end, rounds);
                let teams = w.comps[c].state.entrants.clone();
                let d = w.comps[c].state.round_dates[0].max(w.date.add_days(3));
                schedule::draw_round(w, c, teams, d, legs, &mut rng);
            }
            Format::League { .. } => {}
        }
    }
}

/// Qualifiers from last season's top-flight tables, by nation strength.
fn continental_entrants(w: &World, c: CompId, confed: Confed, tier: usize, year: i32) -> Vec<TeamId> {
    let size = usize::from(w.comps[c].size.max(4));
    let mut nations: Vec<NationId> = w.nations.iter_enumerated().filter(|(_, n)| n.confed == confed && !n.leagues.is_empty()).map(|(id, _)| id).collect();
    nations.sort_by_key(|&n| std::cmp::Reverse(w.nations[n].reputation));
    let slots = |rank: usize| -> (usize, usize) {
        // (places in tier-0 competition, places in tier-1 competition)
        match rank {
            0..=3 => (4, 2),
            4..=5 => (3, 2),
            6..=11 => (2, 1),
            _ => (1, 1),
        }
    };
    let imported = &w.comps[c].state.entrants;
    if !imported.is_empty() && w.history.tables.is_empty() {
        return imported.clone();
    }
    let mut out = Vec::new();
    for (rank, &n) in nations.iter().enumerate() {
        let top = w.nations[n].leagues[0];
        let order: Vec<TeamId> = w
            .history
            .tables
            .iter()
            .rev()
            .find(|t| t.comp == top && t.season < year)
            .map(|t| t.rows.iter().map(|r| r.team).collect())
            .unwrap_or_else(|| {
                let mut v = w.comps[top].state.entrants.clone();
                v.sort_by_key(|&t| std::cmp::Reverse(w.clubs[w.teams[t].club].reputation));
                v
            });
        let (a, b) = slots(rank);
        let (skip, take) = if tier == 0 { (0, a) } else { (a, b) };
        out.extend(order.into_iter().skip(skip).take(take));
    }
    out.sort_by_key(|&t| std::cmp::Reverse(w.clubs[w.teams[t].club].reputation));
    out.truncate(size);
    out
}

/// Club that owns a team (helper for callers that only hold a team id).
pub fn club_of(w: &World, t: TeamId) -> ClubId {
    w.teams[t].club
}

pub fn season_dates(w: &World, n: NationId, year: i32) -> (Date, Date) {
    let s = season(&w.data.calendars[usize::from(w.nations[n].calendar)], year);
    (s.start, s.end)
}
