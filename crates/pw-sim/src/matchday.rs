//! Today's fixtures: select, simulate in parallel, apply in fixture order.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{ClubId, CompId, FixtureId, PlayerId, TeamId};
use pw_match::{Ev, Lod, MatchInput, MatchResult, simulate};
use pw_world::event::{EventKind, Visibility};
use pw_world::{CompKind, FxHashSet, Score, TeamKind, World};
use rayon::prelude::*;

use crate::health;
use crate::selection::{self, Selection};

enum Outcome {
    Played {
        fixture: FixtureId,
        home: Selection,
        away: Selection,
        result: Box<MatchResult>,
    },
    /// A side could not field eleven: awarded 3–0 (D2/D11 simplification).
    Walkover {
        fixture: FixtureId,
        home_forfeits: bool,
    },
}

pub fn importance(w: &World, comp: CompId, decisive: bool) -> f32 {
    let c = &w.comps[comp];
    let base: f32 = match c.kind {
        CompKind::Continental => 0.8,
        CompKind::Cup | CompKind::SuperCup => 0.6,
        CompKind::League => {
            if c.team_kind == TeamKind::First {
                0.5
            } else {
                0.25
            }
        }
    };
    (base + if decisive { 0.15 } else { 0.0 }).min(1.0)
}

pub fn play_today(w: &mut World) {
    let today = w.date;
    let todo: Vec<FixtureId> = w.fixtures.on(today).iter().copied().filter(|&f| w.fixtures.get(f).score.is_none()).collect();
    if todo.is_empty() {
        return;
    }
    let watched = w.watched_teams();
    let world: &World = w;
    let outcomes: Vec<Outcome> = todo.par_iter().map(|&f| play_one(world, f, &watched)).collect();
    for o in outcomes {
        match o {
            Outcome::Played { fixture, home, away, result } => apply(w, fixture, &home, &away, *result, &watched),
            Outcome::Walkover { fixture, home_forfeits } => walkover(w, fixture, home_forfeits),
        }
    }
}

fn play_one(w: &World, f: FixtureId, watched: &FxHashSet<TeamId>) -> Outcome {
    let fx = w.fixtures.get(f);
    let comp = &w.comps[fx.comp];
    // Derbies, title races and relegation fights raise the stakes.
    let imp = (importance(w, fx.comp, fx.decisive) + crate::culture::stakes(w, fx)).min(1.0);
    let home = selection::select_in(w, fx.home, fx.comp, w.date, imp, comp.rules.bench, 0);
    let away = selection::select_in(w, fx.away, fx.comp, w.date, imp, comp.rules.bench, 0);
    let (home, away) = match (home, away) {
        (Some(h), Some(a)) => (h, a),
        (None, _) => return Outcome::Walkover { fixture: f, home_forfeits: true },
        (_, None) => return Outcome::Walkover { fixture: f, home_forfeits: false },
    };
    let first_leg = (fx.leg == 2).then(|| {
        let t = &comp.state.ties[usize::from(fx.tie)];
        // This match's home side is the tie's `b` (away in leg one).
        (t.goals_b, t.goals_a)
    });
    // The appointed referee's strictness; unrefereed levels vary by match.
    let strict = crate::officials::strictness(w, fx).unwrap_or_else(|| 0.75 + 0.5 * (hash_key(&[w.seed, fx.uid, 0x7ef]) % 1000) as f32 / 1000.0);
    let lod = if watched.contains(&fx.home) || watched.contains(&fx.away) { Lod::Full } else { Lod::Standard };
    let input = MatchInput {
        seed: hash_key(&[w.seed, stream::MATCH, fx.uid]),
        home: selection::team_sheet(w, &home),
        away: selection::team_sheet(w, &away),
        neutral: fx.neutral,
        decisive: fx.decisive,
        first_leg,
        away_goals_rule: comp.rules.away_goals,
        importance: imp,
        referee_strictness: strict,
        max_subs: comp.rules.subs,
        lod,
        tuning: &w.data.tuning.matches,
    };
    let result = Box::new(simulate(&input));
    Outcome::Played { fixture: f, home, away, result }
}

fn record_table(w: &mut World, comp: CompId, home: TeamId, away: TeamId, hg: u8, ag: u8) {
    let table = &mut w.comps[comp].state.table;
    if let Some(r) = table.iter_mut().find(|r| r.team == home) {
        r.record(hg, ag);
    }
    if let Some(r) = table.iter_mut().find(|r| r.team == away) {
        r.record(ag, hg);
    }
}

fn record_tie(w: &mut World, f: FixtureId, hg: u8, ag: u8, pens: Option<(u8, u8)>) {
    let fx = w.fixtures.get(f).clone();
    if fx.tie == u16::MAX {
        return;
    }
    let away_rule = w.comps[fx.comp].rules.away_goals;
    let seed = w.seed;
    let t = &mut w.comps[fx.comp].state.ties[usize::from(fx.tie)];
    if fx.leg <= 1 {
        t.goals_a += hg;
        t.goals_b += ag;
        t.away_b += ag;
    } else {
        t.goals_b += hg;
        t.goals_a += ag;
        t.away_a += ag;
    }
    t.played += 1;
    if !fx.decisive {
        return;
    }
    let by_pens = |home_team: TeamId, away_team: TeamId| pens.map(|(h, a)| if h > a { home_team } else { away_team });
    t.winner = if t.legs <= 1 {
        match hg.cmp(&ag) {
            std::cmp::Ordering::Greater => fx.home,
            std::cmp::Ordering::Less => fx.away,
            std::cmp::Ordering::Equal => by_pens(fx.home, fx.away).unwrap_or(fx.home),
        }
    } else if t.goals_a != t.goals_b {
        if t.goals_a > t.goals_b { t.a } else { t.b }
    } else if away_rule && t.away_a != t.away_b {
        if t.away_a > t.away_b { t.a } else { t.b }
    } else {
        by_pens(fx.home, fx.away).unwrap_or(if hash_key(&[seed, fx.uid]) % 2 == 0 { t.a } else { t.b })
    };
}

fn walkover(w: &mut World, f: FixtureId, home_forfeits: bool) {
    let (hg, ag) = if home_forfeits { (0, 3) } else { (3, 0) };
    let fx = w.fixtures.get(f).clone();
    w.fixtures.get_mut(f).score = Some(Score { home: hg, away: ag, ht_home: 0, ht_away: 0, extra_time: false, pens: None });
    record_table(w, fx.comp, fx.home, fx.away, hg, ag);
    record_tie(w, f, hg, ag, None);
}

fn apply(w: &mut World, f: FixtureId, home: &Selection, away: &Selection, r: MatchResult, watched: &FxHashSet<TeamId>) {
    let today = w.date;
    let fx = w.fixtures.get(f).clone();
    let (hg, ag) = (r.home_goals, r.away_goals);
    w.fixtures.get_mut(f).score = Some(Score { home: hg, away: ag, ht_home: r.ht.0, ht_away: r.ht.1, extra_time: r.extra_time, pens: r.pens });
    let group_or_league = fx.tie == u16::MAX;
    if group_or_league {
        record_table(w, fx.comp, fx.home, fx.away, hg, ag);
    }
    record_tie(w, f, hg, ag, r.pens);

    let comp_kind = w.comps[fx.comp].kind;
    let season = w.comps[fx.comp].state.season;
    let yellow_limit = w.comps[fx.comp].rules.yellow_limit;
    let clubs = [w.teams[fx.home].club, w.teams[fx.away].club];
    let senior = [w.teams[fx.home].kind == TeamKind::First, w.teams[fx.away].kind == TeamKind::First];
    let result_sign = [i32::from(hg).cmp(&i32::from(ag)) as i32, i32::from(ag).cmp(&i32::from(hg)) as i32];
    let mut rng = Rng::keyed(&[w.seed, stream::HEALTH, fx.uid]);

    let red_players: Vec<(PlayerId, bool)> = r.events.iter().filter(|e| matches!(e.kind, Ev::Red | Ev::SecondYellow)).map(|e| (e.player, e.kind == Ev::Red)).collect();

    for line in &r.lines {
        let p = line.player;
        let side = usize::from(line.side);
        let club = clubs[side];
        if line.minutes == 0 {
            // Unused substitute.
            let h = &mut w.players.hot[p];
            h.morale = h.morale.saturating_sub(1);
            continue;
        }
        {
            let h = &mut w.players.hot[p];
            h.condition = line.condition_end;
            h.sharpness = (f32::from(h.sharpness) + f32::from(line.minutes) / 90.0 * 14.0).min(100.0) as u8;
            h.minutes_4w = h.minutes_4w.saturating_add(u16::from(line.minutes));
            h.minutes_week = h.minutes_week.saturating_add(u16::from(line.minutes));
            h.push_rating(line.rating);
            h.last_match = today;
            let load = f32::from(line.minutes) * 9.0;
            h.acute += 0.25 * load;
            h.chronic += 0.069 * load;
            let mood = (line.rating - 6.7) * 3.0 + result_sign[side] as f32 * 2.5;
            h.morale = (f32::from(h.morale) + mood).clamp(5.0, 100.0) as u8;
            h.confidence = (f32::from(h.confidence) + (line.rating - 6.7) * 5.0).clamp(5.0, 100.0) as u8;
            h.yellows = h.yellows.saturating_add(line.yellows);
            if yellow_limit > 0 && h.yellows >= yellow_limit {
                h.yellows -= yellow_limit;
                h.ban = h.ban.saturating_add(1);
            }
        }
        if let Some(&(_, straight)) = red_players.iter().find(|(x, _)| *x == p) {
            let prof = pw_world::rules::profile(w, w.clubs[club].nation);
            let matches = if straight { prof.red_ban_straight } else { prof.red_ban_second_yellow };
            w.players.hot[p].ban = w.players.hot[p].ban.saturating_add(matches);
            w.events.push(today, Visibility::Public, EventKind::Suspended { player: p, matches });
        }
        if line.injured {
            health::match_injury(w, p, &mut rng);
        }
        let pom = r.pom == p;
        w.stats.record(fx.comp, club, season, line, pom);
        w.knowledge.observe(clubs[0], p, u16::from(line.minutes), today);
        w.knowledge.observe(clubs[1], p, u16::from(line.minutes), today);

        if senior[side] {
            let c = &mut w.players.cold[p];
            let debut = c.senior_apps == 0;
            let first_goal = line.goals > 0 && c.senior_goals == 0;
            c.senior_apps += 1;
            c.senior_goals += u16::from(line.goals);
            let team = if side == 0 { fx.home } else { fx.away };
            if debut {
                w.events.push(today, Visibility::Public, EventKind::Debut { player: p, team, comp: fx.comp });
            }
            if first_goal {
                w.events.push(today, Visibility::Public, EventKind::FirstGoal { player: p, team, comp: fx.comp });
            }
        }
    }

    // Suspended players serve a match whenever their team plays without them.
    for sel in [home, away] {
        for &p in &w.teams[sel.team].squad {
            if !sel.xi.contains(&p) && !sel.bench.contains(&p) {
                let h = &mut w.players.hot[p];
                if h.ban > 0 && h.injury == 0 {
                    h.ban -= 1;
                }
            }
        }
    }

    for (side, &club) in clubs.iter().enumerate() {
        if let Some(m) = w.clubs[club].manager.get() {
            let rec = &mut w.staff[m].record;
            rec.games += 1;
            match result_sign[side] {
                1 => rec.wins += 1,
                0 => rec.draws += 1,
                _ => rec.losses += 1,
            }
        }
        let mood = &mut w.clubs[club].fan_mood;
        *mood = (i32::from(*mood) + result_sign[side] * 3).clamp(0, 100) as u8;
    }
    gate_receipts(w, clubs[0], comp_kind, senior[0]);
    let imp = (importance(w, fx.comp, fx.decisive) + crate::culture::stakes(w, &fx)).min(1.0);
    crate::interpret::record(w, &fx, home, away, &r, imp);
    crate::culture::after_result(w, &fx, hg, ag, r.pens, pw_core::EventId::NONE);
    crate::facts::record(w, &fx, f, &r);
    crate::officials::after_match(w, &fx, &r);

    if watched.contains(&fx.home) || watched.contains(&fx.away) {
        w.reports.insert(fx.uid, r);
    }
}

fn gate_receipts(w: &mut World, club: ClubId, kind: CompKind, senior: bool) {
    if !senior {
        return;
    }
    let c = &w.clubs[club];
    let econ = f64::from(w.nations[c.nation].economy);
    let rep = f64::from(c.reputation) / 10_000.0;
    let demand = (0.45 + 0.4 * rep + 0.15 * f64::from(c.fan_mood) / 100.0 + if kind == CompKind::Continental { 0.15 } else { 0.0 }).min(1.0);
    let price = f64::from(w.data.tuning.finance.ticket_price_top) * (0.25 + 0.75 * rep) * econ;
    let income = (f64::from(c.capacity) * demand * price) as i64;
    let f = &mut w.clubs[club].finance;
    f.balance += income;
    f.season_income += income;
}
