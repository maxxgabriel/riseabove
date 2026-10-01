//! Fixture generation: round-robins, knockout draws and date selection.

use pw_core::rng::Rng;
use pw_core::{CompId, Date, TeamId, Weekday};
use pw_world::calendar::{in_international_window, span};
use pw_world::comp::{Stage, TableRow, Tie};
use pw_world::{Fixture, World};

/// Circle-method round-robin: `legs` passes over every pairing, alternating
/// home and away. Pairings are indices into the entrant list.
pub fn round_robin(n: usize, legs: u8) -> Vec<Vec<(usize, usize)>> {
    if n < 2 {
        return Vec::new();
    }
    let m = if n.is_multiple_of(2) { n } else { n + 1 };
    let mut ring: Vec<usize> = (0..m).collect();
    let mut first: Vec<Vec<(usize, usize)>> = Vec::with_capacity(m - 1);
    for r in 0..m - 1 {
        let mut round = Vec::with_capacity(m / 2);
        for i in 0..m / 2 {
            let (a, b) = (ring[i], ring[m - 1 - i]);
            if a >= n || b >= n {
                continue;
            }
            // Alternate venue by round so nobody plays long home/away runs.
            round.push(if (r + i) % 2 == 0 { (a, b) } else { (b, a) });
        }
        first.push(round);
        let last = ring.pop().expect("non-empty ring");
        ring.insert(1, last);
    }
    let mut out = first.clone();
    for leg in 1..legs {
        for round in &first {
            out.push(round.iter().map(|&(h, a)| if leg % 2 == 1 { (a, h) } else { (h, a) }).collect());
        }
    }
    out
}

fn blocked(w: &World, nation_break: Option<(Date, Date)>, d: Date) -> bool {
    in_international_window(&w.data.international_windows, d) || nation_break.is_some_and(|(a, b)| d >= a && d <= b)
}

/// `count` dates between `from` and `to` on `preferred` weekdays, evenly
/// spread, skipping international windows and the winter break. Falls back to
/// `fallback` weekdays when there are not enough preferred ones.
pub fn spread_dates(w: &World, from: Date, to: Date, count: usize, preferred: Weekday, fallback: Weekday, brk: Option<(Date, Date)>) -> Vec<Date> {
    if count == 0 || to < from {
        return Vec::new();
    }
    let collect = |wd: Weekday| -> Vec<Date> {
        let mut v = Vec::new();
        let mut d = from.next_weekday(wd);
        while d <= to {
            if !blocked(w, brk, d) {
                v.push(d);
            }
            d = d.add_days(7);
        }
        v
    };
    let mut pool = collect(preferred);
    if pool.len() < count {
        pool.extend(collect(fallback));
        pool.sort();
    }
    if pool.len() < count {
        // Compress into consecutive days if the window is simply too short.
        let mut d = from;
        while pool.len() < count && d <= to.add_days(60) {
            if !pool.contains(&d) {
                pool.push(d);
            }
            d = d.add_days(3);
        }
        pool.sort();
    }
    if pool.len() <= count {
        return pool;
    }
    (0..count).map(|k| pool[(k * (pool.len() - 1)) / (count - 1).max(1)]).collect()
}

pub fn add_fixture(w: &mut World, comp: CompId, round: u8, leg: u8, group: u8, date: Date, home: TeamId, away: TeamId, tie: u16, decisive: bool, neutral: bool) {
    // A team never plays twice in a day: a clash moves this fixture to the next free day.
    let date = w.fixtures.first_free_date(home, away, date, 14, None);
    w.fixtures.add(Fixture { uid: 0, comp, round, leg, group, date, home, away, tie, decisive, neutral, score: None });
}

/// League season fixtures from `from` to `to`.
pub fn schedule_league(w: &mut World, comp: CompId, from: Date, to: Date, brk: Option<(Date, Date)>) {
    let c = &w.comps[comp];
    let legs = match c.format {
        pw_world::Format::League { rounds } => rounds.max(1),
        _ => 2,
    };
    let entrants = c.state.entrants.clone();
    let rounds = round_robin(entrants.len(), legs);
    let dates = spread_dates(w, from, to, rounds.len(), Weekday::Sat, Weekday::Wed, brk);
    for (r, (pairs, &date)) in rounds.iter().zip(dates.iter()).enumerate() {
        for &(h, a) in pairs {
            add_fixture(w, comp, r as u8, 0, 0, date, entrants[h], entrants[a], u16::MAX, false, false);
        }
    }
    let comp_mut = &mut w.comps[comp];
    comp_mut.state.table = entrants.iter().map(|&t| TableRow::new(t, 0)).collect();
    comp_mut.state.stage = Stage::League;
}

/// Group stage for continental competitions: pots by reputation, double round-robin.
pub fn schedule_groups(w: &mut World, comp: CompId, dates: &[Date], rng: &mut Rng) {
    let (groups, size) = match w.comps[comp].format {
        pw_world::Format::Groups { groups, size, .. } => (usize::from(groups), usize::from(size)),
        _ => return,
    };
    let mut entrants = w.comps[comp].state.entrants.clone();
    entrants.sort_by(|&a, &b| {
        let ra = w.clubs[w.teams[a].club].reputation;
        let rb = w.clubs[w.teams[b].club].reputation;
        rb.cmp(&ra).then(a.cmp(&b))
    });
    entrants.truncate(groups * size);
    let mut table = Vec::new();
    let mut members: Vec<Vec<TeamId>> = vec![Vec::new(); groups];
    for pot in entrants.chunks(groups) {
        let mut pot = pot.to_vec();
        rng.shuffle(&mut pot);
        for (g, t) in pot.into_iter().enumerate() {
            members[g].push(t);
        }
    }
    for (g, teams) in members.iter().enumerate() {
        for &t in teams {
            table.push(TableRow::new(t, g as u8));
        }
        for (md, pairs) in round_robin(teams.len(), 2).iter().enumerate() {
            let Some(&date) = dates.get(md) else { continue };
            for &(h, a) in pairs {
                add_fixture(w, comp, md as u8, 0, g as u8, date, teams[h], teams[a], u16::MAX, false, false);
            }
        }
    }
    let c = &mut w.comps[comp];
    c.state.table = table;
    c.state.stage = Stage::Groups;
}

/// Draw a knockout round among `teams` and schedule it on `date` (first leg).
pub fn draw_round(w: &mut World, comp: CompId, mut teams: Vec<TeamId>, date: Date, legs: u8, rng: &mut Rng) {
    rng.shuffle(&mut teams);
    let round = w.comps[comp].state.round;
    let is_final = teams.len() == 2;
    let base = w.comps[comp].state.ties.len();
    for (k, pair) in teams.chunks(2).enumerate() {
        let &[a, b] = pair else {
            // Odd team out gets a bye straight into the next round.
            let mut tie = Tie::new(pair[0], TeamId::NONE, 0);
            tie.winner = pair[0];
            w.comps[comp].state.ties.push(tie);
            continue;
        };
        let tie_idx = (base + k) as u16;
        w.comps[comp].state.ties.push(Tie::new(a, b, legs));
        if legs >= 2 {
            add_fixture(w, comp, round, 1, 0, date, a, b, tie_idx, false, false);
            add_fixture(w, comp, round, 2, 0, date.add_days(7), b, a, tie_idx, true, false);
        } else {
            add_fixture(w, comp, round, 1, 0, date, a, b, tie_idx, true, is_final);
        }
    }
    w.comps[comp].state.stage = pw_world::comp::Stage::Knockout(teams.len() as u16);
}

/// First knockout round with byes so the next round is a power of two.
pub fn draw_cup_opening(w: &mut World, comp: CompId, date: Date, rng: &mut Rng) {
    let mut teams = w.comps[comp].state.entrants.clone();
    if teams.len() < 2 {
        return;
    }
    let pow = teams.len().next_power_of_two();
    let target = if pow == teams.len() { pow } else { pow / 2 };
    let prelim_matches = teams.len() - target;
    // Strongest clubs take the byes.
    teams.sort_by(|&a, &b| w.clubs[w.teams[b].club].reputation.cmp(&w.clubs[w.teams[a].club].reputation).then(a.cmp(&b)));
    if prelim_matches == 0 {
        draw_round(w, comp, teams, date, 1, rng);
        return;
    }
    let byes = teams.len() - 2 * prelim_matches;
    for &t in &teams[..byes] {
        let mut tie = Tie::new(t, TeamId::NONE, 0);
        tie.winner = t;
        w.comps[comp].state.ties.push(tie);
    }
    let playing = teams[byes..].to_vec();
    draw_round(w, comp, playing, date, 1, rng);
}

/// International-window-aware anchors for a season: returns dates for
/// `count` midweek knockout rounds evenly spread across the season.
pub fn cup_round_dates(w: &World, start: Date, end: Date, count: usize) -> Vec<Date> {
    let from = start.add_days(28);
    let to = end.add_days(-8);
    let mut d = spread_dates(w, from, to, count.saturating_sub(1), Weekday::Wed, Weekday::Tue, None);
    // Final on the last Saturday.
    d.push(end.add_days(-7).next_weekday(Weekday::Sat).min(end));
    d
}

pub fn windows_label(w: &World, year: i32) -> Vec<(Date, Date)> {
    w.data.international_windows.iter().map(|&s| span(year, s)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_covers_all_pairs_once_per_leg() {
        for n in [4usize, 5, 18, 20] {
            let rr = round_robin(n, 2);
            let m = if n % 2 == 0 { n } else { n + 1 };
            assert_eq!(rr.len(), 2 * (m - 1));
            let mut seen = std::collections::HashMap::new();
            for round in &rr {
                let mut in_round = std::collections::HashSet::new();
                for &(h, a) in round {
                    assert!(in_round.insert(h) && in_round.insert(a), "team twice in a round");
                    *seen.entry((h, a)).or_insert(0) += 1;
                }
            }
            assert_eq!(seen.len(), n * (n - 1));
            assert!(seen.values().all(|&v| v == 1));
        }
    }
}
