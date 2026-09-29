//! Feeding the almanac and the record book from every match and season:
//! fastest goals and hat-tricks, hat-tricks and streaks, how quickly players
//! reach their first goals, clean sheets, cards, top speed and distance, and
//! for teams the runs, seasons and attendances that make up a competition's
//! history. Everything is written when it happens, at every level of the game.
//!
//! Top speed and distance are a measurement model, not tracked play: they follow
//! pace and acceleration, stamina and work rate, age, condition, role and
//! minutes, with a keyed match-to-match variation, so the fastest man in a
//! league is usually a quick one but not always the same one.

use pw_core::rng::hash_key;
use pw_core::{Attr, ClubId, CompId, NationId, PersonId, PlayerId, PosGroup};
use pw_match::{Ev, MatchResult};
use pw_world::almanac::{MILESTONES, CareerLine};
use pw_world::minor::Level;
use pw_world::records::{Holder, Mark, RecordKey, Scope, Stat};
use pw_world::stats::StatLine;
use pw_world::{TableRow, TeamKind, World};

use crate::records::{note, note_quiet};

/// Which records a match feeds and how.
pub struct Ctx {
    pub comp: Option<CompId>,
    /// Scopes shared by both sides (competition, nation, the world).
    pub shared: Vec<Scope>,
    pub level: Level,
    /// Counts toward professional careers (first teams in club competitions).
    pub pro: bool,
}

fn key(scope: Scope, stat: Stat, level: Level) -> RecordKey {
    RecordKey { scope, stat, level }
}

/// Offer a mark to several scopes. Wide scopes (the world, a nation) announce when a record falls; narrow ones are quiet.
fn offer(w: &mut World, scopes: &[Scope], level: Level, stat: Stat, holder: Holder, value: i64, min: i64) {
    let mark = Mark { holder, value, date: w.date, against: None };
    for &s in scopes {
        let wide = matches!(s, Scope::World | Scope::Nation(_) | Scope::Event(..) | Scope::Region(_));
        if wide && level != Level::Youth {
            note(w, key(s, stat, level), mark, min, true);
        } else {
            note_quiet(w, key(s, stat, level), mark, min);
        }
    }
}

fn top_speed(w: &World, p: PlayerId, minutes: u8, condition: u8, seed: u64) -> i64 {
    let c = &w.players.cold[p];
    let age = w.age_years(p);
    let base = 25.0 + 0.40 * c.attrs.get(Attr::Pace) + 0.15 * c.attrs.get(Attr::Acceleration) - 0.07 * (age - 27.0).max(0.0);
    let fit = 0.96 + 0.04 * f32::from(condition) / 100.0;
    // Longer on the pitch, more chances to reach top speed.
    let chances = 0.4 * (f32::from(minutes) / 90.0).min(1.0);
    let noise = ((hash_key(&[seed, u64::from(p.0), 0x59d]) % 2000) as f32 / 1000.0 - 1.0) * 0.9;
    ((base * fit + chances + noise) * 10.0).round() as i64
}

fn distance(w: &World, p: PlayerId, line: &pw_match::PlayerLine, seed: u64) -> i64 {
    let c = &w.players.cold[p];
    let role = match line.pos.map_or(PosGroup::Mid, |p| p.group()) {
        PosGroup::Gk => 0.25,
        PosGroup::Def => 0.95,
        PosGroup::Mid => 1.15,
        PosGroup::Att => 1.0,
    };
    let base = 6500.0 + 260.0 * c.attrs.get(Attr::Stamina) + 200.0 * c.attrs.get(Attr::WorkRate);
    let noise = ((hash_key(&[seed, u64::from(p.0), 0xd15]) % 2000) as f32 - 1000.0) * 0.4;
    (base * role * f32::from(line.minutes) / 90.0 * (0.85 + 0.15 * f32::from(line.condition_end) / 100.0) + noise).max(0.0) as i64
}

/// Everything one match adds to the book. `side` is each side's own scope (a club, a nation, a state).
pub fn report(w: &mut World, ctx: &Ctx, r: &MatchResult, side: [Scope; 2], squads: [&[PlayerId]; 2], seed: u64) {
    let goals = [r.home_goals, r.away_goals];
    for i in 0..2 {
        let holder = match side[i] {
            Scope::Club(c) => Holder::Club(c),
            Scope::Nation(n) => Holder::Nation(n),
            Scope::Region(g) => Holder::Region(g),
            _ => continue,
        };
        let mut scopes = ctx.shared.clone();
        scopes.push(side[i]);
        offer(w, &scopes, ctx.level, Stat::MostGoalsInMatch, holder, i64::from(goals[i]), 6);
    }
    let mut appeared: pw_world::FxHashSet<PlayerId> = Default::default();
    for line in r.lines.iter().filter(|l| l.minutes > 0) {
        let p = line.player;
        appeared.insert(p);
        let s = usize::from(line.side);
        let who = w.players.cold[p].person;
        let holder = Holder::Person(who);
        let mut scopes = ctx.shared.clone();
        scopes.push(side[s]);
        let age = i64::from(w.people[who].dob.days_until(w.date));
        // Goals: when, and how quickly a hat-trick came.
        let mut times: Vec<u16> = r.events.iter().filter(|e| matches!(e.kind, Ev::Goal | Ev::PenaltyGoal) && e.player == p).map(|e| e.t).collect();
        times.sort_unstable();
        if let Some(&t) = times.first() {
            offer(w, &scopes, ctx.level, Stat::FastestGoal, holder, i64::from(t.max(1)), 240);
        }
        if times.len() >= 3 {
            offer(w, &scopes, ctx.level, Stat::FastestHatTrick, holder, i64::from(times[2] - times[0]), 50 * 60);
            offer(w, &scopes, ctx.level, Stat::YoungestHatTrick, holder, age, 23 * 365);
        }
        offer(w, &scopes, ctx.level, Stat::GoalsInMatch, holder, i64::from(line.goals), 3);
        offer(w, &scopes, ctx.level, Stat::AssistsInMatch, holder, i64::from(line.assists), 3);
        offer(w, &scopes, ctx.level, Stat::MatchRating, holder, (line.rating * 10.0).round() as i64, 96);
        if let Some(t) = r.events.iter().find(|e| matches!(e.kind, Ev::Red | Ev::SecondYellow) && e.player == p).map(|e| e.t) {
            offer(w, &scopes, ctx.level, Stat::FastestRedCard, holder, i64::from(t.max(1)), 25 * 60);
        }
        // Measured physical marks (players with a real spell on the pitch).
        if line.minutes >= 20 {
            let v = top_speed(w, p, line.minutes, line.condition_end, seed);
            offer(w, &scopes, ctx.level, Stat::TopSpeed, holder, v, 345);
            let d = distance(w, p, line, seed);
            offer(w, &scopes, ctx.level, Stat::DistanceCovered, holder, d, 12_000);
            if ctx.pro {
                let c = w.ext.almanac.career.entry(who).or_default();
                c.top_speed = c.top_speed.max(v.clamp(0, 65_000) as u16);
                c.best_distance_m = c.best_distance_m.max(d.clamp(0, 65_000) as u16);
            }
        }
        if ctx.pro {
            career(w, ctx, r, line, p, who, side[s], &times, age);
        } else if let Some(comp) = ctx.comp {
            // Youth and other competitions keep tallies for their own tables only.
            let t = w.ext.almanac.comp_tally.entry((comp, who)).or_default();
            t.0 = t.0.saturating_add(1);
            t.1 = t.1.saturating_add(u16::from(line.goals));
            t.2 = t.2.saturating_add(u16::from(line.assists));
        }
    }
    if ctx.pro {
        // A streak of appearances ends when the side plays and the player does not.
        for sq in squads {
            for &p in sq {
                if !appeared.contains(&p) {
                    let who = w.players.cold[p].person;
                    if let Some(c) = w.ext.almanac.career.get_mut(&who) {
                        c.app_streak = 0;
                    }
                }
            }
        }
        // Runs for clubs: defeats in a row, home games unbeaten.
        for i in 0..2 {
            if let Scope::Club(c) = side[i] {
                let (f, a) = (goals[i], goals[1 - i]);
                let run = w.ext.almanac.runs.entry(c).or_default();
                run.0 = if f < a { run.0 + 1 } else { 0 };
                if i == 0 {
                    run.1 = if f < a { 0 } else { run.1 + 1 };
                }
                let (losing, home) = *run;
                let mut scopes = ctx.shared.clone();
                scopes.push(side[i]);
                offer(w, &scopes, ctx.level, Stat::LosingRun, Holder::Club(c), i64::from(losing), 8);
                if i == 0 {
                    offer(w, &scopes, ctx.level, Stat::HomeUnbeaten, Holder::Club(c), i64::from(home), 20);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn career(w: &mut World, ctx: &Ctx, r: &MatchResult, line: &pw_match::PlayerLine, p: PlayerId, who: PersonId, side: Scope, times: &[u16], age: i64) {
    let today = w.date;
    let comp = ctx.comp;
    let c: &mut CareerLine = w.ext.almanac.career.entry(who).or_default();
    if c.apps == 0 {
        c.debut = today;
    }
    c.apps += 1;
    let goals_before = c.goals;
    c.goals += u32::from(line.goals);
    c.assists += u32::from(line.assists);
    c.yellows += u16::from(line.yellows);
    c.reds += u16::from(line.reds);
    c.motm += u16::from(r.pom == p);
    c.best_rating = c.best_rating.max((line.rating * 10.0).round().clamp(0.0, 255.0) as u8);
    let hat = times.len() >= 3;
    c.hat_tricks += u16::from(hat);
    c.app_streak += 1;
    c.best_app_streak = c.best_app_streak.max(c.app_streak);
    if line.goals > 0 {
        c.goal_streak += 1;
        c.best_goal_streak = c.best_goal_streak.max(c.goal_streak);
    } else {
        c.goal_streak = 0;
    }
    let clean = line.is_keeper && line.conceded == 0 && line.minutes >= 60;
    c.clean_sheets += u32::from(clean);
    if line.is_keeper {
        if line.conceded == 0 {
            c.gk_run += u32::from(line.minutes);
        } else {
            c.gk_run = 0;
        }
        c.best_gk_run = c.best_gk_run.max(c.gk_run);
    }
    // How fast the milestones came.
    let mut reached: Vec<(usize, u16, u16)> = Vec::new();
    for (i, &m) in MILESTONES.iter().enumerate() {
        if c.apps_to[i] == 0 && goals_before < m && c.goals >= m {
            let apps = c.apps.min(u32::from(u16::MAX)) as u16;
            let days = c.debut.days_until(today).clamp(1, i32::from(u16::MAX)) as u16;
            c.apps_to[i] = apps;
            c.days_to[i] = days;
            reached.push((i, apps, days));
        }
    }
    let snap = *c;
    let holder = Holder::Person(who);
    let nation = comp.map_or(NationId::NONE, |cp| w.comps[cp].nation);
    let wide: Vec<Scope> = [Scope::World, if nation.is_some() { Scope::Nation(nation) } else { Scope::World }].into_iter().collect();
    let all_scopes: Vec<Scope> = { let mut v = wide.clone(); v.extend(comp.map(Scope::Comp)); v.push(side); v };
    let lvl = Level::Professional;
    if line.goals > 0 {
        offer(w, &wide, lvl, Stat::Goals, holder, i64::from(snap.goals), 60);
        offer(w, &wide, lvl, Stat::GoalStreak, holder, i64::from(snap.goal_streak), 6);
        for &s in &all_scopes {
            w.ext.almanac.offer(s, Stat::Goals, who, i64::from(snap.goals));
        }
    }
    if line.assists > 0 {
        offer(w, &wide, lvl, Stat::Assists, holder, i64::from(snap.assists), 40);
        for &s in &all_scopes {
            w.ext.almanac.offer(s, Stat::Assists, who, i64::from(snap.assists));
        }
    }
    if snap.apps % 5 == 0 {
        offer(w, &wide, lvl, Stat::Apps, holder, i64::from(snap.apps), 150);
        offer(w, &wide, lvl, Stat::AppStreak, holder, i64::from(snap.best_app_streak), 60);
        if snap.apps >= 100 {
            offer(w, &wide, lvl, Stat::GoalRate, holder, i64::from(snap.goals) * 1000 / i64::from(snap.apps), 400);
        }
        for &s in &all_scopes {
            w.ext.almanac.offer(s, Stat::Apps, who, i64::from(snap.apps));
        }
    }
    if clean {
        offer(w, &wide, lvl, Stat::CleanSheets, holder, i64::from(snap.clean_sheets), 40);
        for &s in &all_scopes {
            w.ext.almanac.offer(s, Stat::CleanSheets, who, i64::from(snap.clean_sheets));
        }
        offer(w, &wide, lvl, Stat::KeeperMinutesUnbeaten, holder, i64::from(snap.best_gk_run), 600);
    }
    if hat {
        offer(w, &wide, lvl, Stat::HatTricks, holder, i64::from(snap.hat_tricks), 3);
    }
    if line.yellows + line.reds > 0 {
        offer(w, &wide, lvl, Stat::YellowCards, holder, i64::from(snap.yellows), 30);
        offer(w, &wide, lvl, Stat::RedCards, holder, i64::from(snap.reds), 4);
    }
    if r.pom == p {
        offer(w, &wide, lvl, Stat::ManOfTheMatch, holder, i64::from(snap.motm), 25);
    }
    // Fastest to N goals, by appearances and by days; only reaching the milestone counts.
    let stats = [
        (Stat::AppsToFirstGoal, Stat::DaysToFirstGoal),
        (Stat::AppsTo10Goals, Stat::DaysTo10Goals),
        (Stat::AppsTo50Goals, Stat::DaysTo50Goals),
        (Stat::AppsTo100Goals, Stat::DaysTo100Goals),
    ];
    for (i, apps, days) in reached {
        // The value to beat is generous so the first few reaching a milestone set the mark.
        let (a_max, d_max) = [(12, 200), (60, 700), (200, 1800), (400, 3600)][i];
        offer(w, &wide, lvl, stats[i].0, holder, i64::from(apps), a_max);
        offer(w, &wide, lvl, stats[i].1, holder, i64::from(days), d_max);
        for &s in &all_scopes {
            w.ext.almanac.offer(s, stats[i].0, who, i64::from(apps));
        }
    }
    let _ = age;
    if let Some(cp) = comp {
        let t = w.ext.almanac.comp_tally.entry((cp, who)).or_default();
        t.0 = t.0.saturating_add(1);
        t.1 = t.1.saturating_add(u16::from(line.goals));
        t.2 = t.2.saturating_add(u16::from(line.assists));
    }
}

/// What a region has produced: players who became senior regulars, and internationals.
pub fn region_marks(w: &mut World, r: pw_core::RegionId, players: i64, internationals: i64) {
    let scopes = [Scope::Region(r)];
    offer(w, &scopes, Level::Professional, Stat::PlayersProduced, Holder::Region(r), players, 5);
    offer(w, &scopes, Level::International, Stat::InternationalsProduced, Holder::Region(r), internationals, 2);
}

/// The record scopes and level for a club competition match.
pub fn comp_ctx(w: &World, comp: CompId) -> Ctx {
    let c = &w.comps[comp];
    let pro = c.team_kind == TeamKind::First;
    let mut shared = vec![Scope::Comp(comp)];
    if c.nation.is_some() {
        shared.push(Scope::Nation(c.nation));
    }
    if pro {
        shared.push(Scope::World);
    }
    Ctx { comp: Some(comp), shared, level: if pro { Level::Professional } else { Level::Youth }, pro }
}

/// A league or cup match between club sides.
pub fn club_match(w: &mut World, comp: CompId, clubs: [ClubId; 2], squads: [&[PlayerId]; 2], r: &MatchResult, seed: u64) {
    let ctx = comp_ctx(w, comp);
    report(w, &ctx, r, [Scope::Club(clubs[0]), Scope::Club(clubs[1])], squads, seed);
}

/// A national-team match.
pub fn national_match(w: &mut World, nations: [NationId; 2], r: &MatchResult, seed: u64) {
    let ctx = Ctx { comp: None, shared: vec![Scope::World], level: Level::International, pro: false };
    report(w, &ctx, r, [Scope::Nation(nations[0]), Scope::Nation(nations[1])], [&[], &[]], seed);
}

/// Attendance is a record of its own.
pub fn attendance(w: &mut World, club: ClubId, comp: CompId, spectators: i64) {
    let mut scopes = vec![Scope::Club(club), Scope::Comp(comp)];
    let n = w.clubs[club].nation;
    if n.is_some() {
        scopes.push(Scope::Nation(n));
    }
    offer(w, &scopes, Level::Professional, Stat::HighestAttendance, Holder::Club(club), spectators, 1000);
}

/// A competition's finished season, players first: season marks, the golden boot.
pub fn season_lines(w: &mut World, comp: CompId, lines: &[StatLine], games: u16) {
    if lines.is_empty() {
        return;
    }
    let ctx = comp_ctx(w, comp);
    let lvl = ctx.level;
    let min_apps = ((f32::from(games) * 0.6) as u16).max(10);
    let boot = lines.iter().max_by_key(|l| (l.goals, l.assists, std::cmp::Reverse(l.minutes)));
    for l in lines {
        let who = w.players.cold[l.player].person;
        let holder = Holder::Person(who);
        offer(w, &ctx.shared, lvl, Stat::GoalsInSeason, holder, i64::from(l.goals), 12);
        offer(w, &ctx.shared, lvl, Stat::AssistsInSeason, holder, i64::from(l.assists), 8);
        offer(w, &ctx.shared, lvl, Stat::CleanSheetsInSeason, holder, i64::from(l.clean_sheets), 10);
        if l.apps >= min_apps {
            offer(w, &ctx.shared, lvl, Stat::AvgRatingInSeason, holder, (l.avg_rating() * 100.0).round() as i64, 720);
        }
        for (stat, v) in [(Stat::GoalsInSeason, i64::from(l.goals)), (Stat::AssistsInSeason, i64::from(l.assists)), (Stat::CleanSheetsInSeason, i64::from(l.clean_sheets))] {
            if v >= 5 {
                w.ext.almanac.offer(Scope::Comp(comp), stat, who, v);
            }
        }
    }
    if let Some(b) = boot.filter(|b| b.goals >= 5 && ctx.pro) {
        let who = w.players.cold[b.player].person;
        let n = {
            let e = w.ext.almanac.golden_boots.entry(who).or_default();
            *e += 1;
            *e
        };
        offer(w, &[Scope::World], lvl, Stat::GoldenBoots, Holder::Person(who), i64::from(n), 3);
        w.ext.almanac.offer(Scope::World, Stat::GoldenBoots, who, i64::from(n));
    }
}

/// A league's final table: team season marks and runs of titles.
pub fn league_season(w: &mut World, comp: CompId, rows: &[TableRow]) {
    let ctx = comp_ctx(w, comp);
    let lvl = ctx.level;
    for r in rows {
        let club = w.teams[r.team].club;
        let mut scopes = ctx.shared.clone();
        scopes.push(Scope::Club(club));
        let h = Holder::Club(club);
        offer(w, &scopes, lvl, Stat::MostGoalsInSeason, h, i64::from(r.gf), 60);
        offer(w, &scopes, lvl, Stat::FewestConcededInSeason, h, i64::from(r.ga), 30);
        offer(w, &scopes, lvl, Stat::BestGoalDifference, h, i64::from(r.gd()), 30);
        offer(w, &scopes, lvl, Stat::WinsInSeason, h, i64::from(r.won), 18);
        if r.lost == 0 {
            offer(w, &scopes, lvl, Stat::UnbeatenSeason, h, i64::from(r.played), 20);
        }
    }
    let Some(first) = rows.first() else { return };
    let champ = w.teams[first.team].club;
    let run = w.ext.almanac.title_run.entry(comp).or_insert((champ, 0));
    if run.0 == champ {
        run.1 += 1;
    } else {
        *run = (champ, 1);
    }
    let streak = run.1;
    offer(w, &ctx.shared, lvl, Stat::TitleStreak, Holder::Club(champ), i64::from(streak), 3);
    let titles = w.history.honours.iter().filter(|h| h.comp == comp && h.club == champ).count() as i64;
    let mut scopes = vec![Scope::Comp(comp)];
    if ctx.pro {
        scopes.push(Scope::World);
    }
    offer(w, &scopes, lvl, Stat::Titles, Holder::Club(champ), titles, 3);
}

/// Every club's league titles in a competition, most first (a championship table).
pub fn champions_table(w: &World, comp: CompId) -> Vec<(ClubId, u16)> {
    let mut m: pw_world::FxHashMap<ClubId, u16> = Default::default();
    for h in w.history.honours.iter().filter(|h| h.comp == comp) {
        *m.entry(h.club).or_default() += 1;
    }
    let mut v: Vec<(ClubId, u16)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}
