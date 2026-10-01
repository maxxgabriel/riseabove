//! The competition overview: one screen that says what is going on in a competition right now.
//! Recent and coming results, the current round or table, who leads which statistic, and the
//! stories written about it. Everything comes from recorded state. Results the viewer has not
//! revealed yet are left out or masked, and sections that cannot be masked are held back.

use std::collections::BTreeMap;

use pw_core::{ClubId, CompId, Date, PlayerId, TeamId};
use pw_world::comp::{Format, Stage};
use pw_world::media::StoryLink;
use pw_world::{CompKind, Fixture};
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};

fn named(r: Ref, n: String) -> Value {
    serde_json::to_value(Named::new(r, n)).unwrap_or(Value::Null)
}

fn hex(c: u32) -> String {
    format!("#{:06x}", c & 0xffffff)
}

/// A team as the overview draws it: name, crest colours and whether it is the viewer's own.
fn team(c: &Ctx, t: TeamId) -> Value {
    let club = c.w.teams[t].club;
    let cl = &c.w.clubs[club];
    json!({
        "k": "club", "id": club.0, "name": c.team_short(t), "full": c.team_name(t),
        "colors": [hex(cl.colors[0]), hex(cl.colors[1])],
        "me": c.my_club() == club,
    })
}

fn club_team(c: &Ctx, club: ClubId) -> Value {
    let cl = &c.w.clubs[club];
    json!({
        "k": "club", "id": club.0, "name": cl.short_name, "full": cl.name,
        "colors": [hex(cl.colors[0]), hex(cl.colors[1])],
        "me": c.my_club() == club,
    })
}

/// What a competition's standing is called, on a scale of 0 to 10,000.
pub fn reputation_word(rep: u16) -> &'static str {
    match rep {
        0..=1249 => "Poor",
        1250..=2499 => "Modest",
        2500..=3749 => "Fair",
        3750..=4999 => "Fairly good",
        5000..=6249 => "Good",
        6250..=7499 => "Very good",
        7500..=8749 => "Excellent",
        _ => "World class",
    }
}

/// The name of a knockout round from the number of teams still in it.
pub fn knockout_round_name(teams: u16) -> String {
    match teams {
        0..=2 => "Final".into(),
        3..=4 => "Semi-finals".into(),
        5..=8 => "Quarter-finals".into(),
        n => format!("Round of {n}"),
    }
}

fn ordinal(n: usize) -> String {
    let suffix = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn kind_rank(k: CompKind) -> u8 {
    match k {
        CompKind::League => 0,
        CompKind::Cup => 1,
        CompKind::SuperCup => 2,
        CompKind::Continental => 3,
    }
}

fn kind_key(k: CompKind) -> &'static str {
    match k {
        CompKind::League => "league",
        CompKind::Cup => "cup",
        CompKind::SuperCup => "supercup",
        CompKind::Continental => "continental",
    }
}

fn kind_plural(k: CompKind) -> &'static str {
    match k {
        CompKind::League => "leagues",
        CompKind::Cup => "cups",
        CompKind::SuperCup => "super cups",
        CompKind::Continental => "continental competitions",
    }
}

/// The previous and next competition in the same nation (or the same kind, for continental ones).
fn siblings(c: &Ctx, id: CompId) -> (Value, Value) {
    let w = c.w;
    let me = &w.comps[id];
    let mut all: Vec<(u8, u8, CompId)> = w
        .comps
        .iter_enumerated()
        .filter(|(_, x)| if me.nation.is_some() { x.nation == me.nation } else { x.nation.is_none() && x.kind == me.kind })
        .map(|(i, x)| (kind_rank(x.kind), x.tier, i))
        .collect();
    all.sort();
    let Some(pos) = all.iter().position(|x| x.2 == id) else { return (Value::Null, Value::Null) };
    let pick = |i: Option<usize>| i.and_then(|i| all.get(i)).map_or(Value::Null, |x| named(Ref::comp(x.2), c.comp_name(x.2)));
    (pick(pos.checked_sub(1)), pick(pos.checked_add(1)))
}

/// One block of leaders: the page of the panel it sits on, its title, its top rows and whether it may be shown.
struct Block {
    page: u8,
    title: &'static str,
    rows: Vec<Value>,
    show: bool,
}

impl Block {
    fn new(page: u8, title: &'static str, rows: Vec<Value>) -> Self {
        Self { page, title, rows, show: true }
    }

    fn show_if(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    fn json(&self) -> Value {
        json!({"page": self.page, "title": self.title, "rows": self.rows})
    }
}

/// A team's value to rank by and the text to show for it, if it has one.
type Ranked = Option<(f64, String)>;

/// Sums of player lines for one club that cannot be split by match.
#[derive(Default)]
struct ClubAgg {
    xg: f32,
    shots: u32,
    yellows: u32,
    reds: u32,
}

#[derive(Default, Clone, Copy)]
struct TeamAgg {
    played: u16,
    won: u16,
    gf: u16,
    ga: u16,
    cs: u16,
    /// Best win by margin: (margin, goals for, goals against, opponent).
    best: Option<(u8, u8, u8, TeamId)>,
}

fn round_of(fx: &Fixture) -> u8 {
    fx.round
}

pub fn overview(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let req: crate::contract::CompOverviewReq = crate::contract::request(args.clone())?;
    let id = CompId(req.id);
    if id.0 as usize >= c.w.comps.len() {
        return Err(ApiError::NotFound(format!("competition {}", id.0)));
    }
    let w = c.w;
    let comp = &w.comps[id];
    let st = &comp.state;
    let today = w.date;

    let stage_label = match st.stage {
        Stage::NotStarted => "Not started".to_string(),
        Stage::League => "Season in progress".to_string(),
        Stage::Groups => "Group stage".to_string(),
        Stage::Knockout(n) => knockout_round_name(n),
        Stage::Finished => "Season finished".to_string(),
    };

    // ---- the header ----
    // A title decided by a match the viewer has not revealed yet is not announced in the header.
    let undecided = c.concealed_fixtures().iter().any(|f| f.comp == id);
    let holders = w.history.honours.iter().filter(|h| h.comp == id && !(undecided && h.season >= st.season)).max_by_key(|h| h.season);
    let ranked: Vec<u16> = {
        let mut v: Vec<u16> = w.comps.iter().filter(|x| x.kind == comp.kind).map(|x| x.reputation).collect();
        v.sort_unstable_by(|a, b| b.cmp(a));
        v
    };
    let rank = ranked.iter().position(|&r| r <= comp.reputation).map_or(1, |p| p + 1);
    let (prev, next) = siblings(c, id);
    let mut meta: Vec<Value> = Vec::new();
    if comp.nation.is_some() {
        meta.push(json!({"label": "Based in", "value": w.nations[comp.nation].code, "ref": named(Ref::nation(comp.nation), c.nation_name(comp.nation))}));
    }
    if let Some(h) = holders {
        meta.push(json!({"label": "Current holders", "value": club_team(c, h.club), "sub": c.season_label(id, h.season)}));
    }
    meta.push(json!({"label": "Ranking", "value": format!("{} of {} {}", ordinal(rank), ranked.len(), kind_plural(comp.kind))}));
    meta.push(json!({"label": "Reputation", "value": reputation_word(comp.reputation)}));

    let head = json!({
        "id": id.0, "name": comp.name, "short": comp.short_name, "kind": crate::tables::kind_text(comp.kind), "kind_key": kind_key(comp.kind), "tier": comp.tier,
        "teams": st.entrants.len(), "season": c.season_label(id, if st.season > 0 { st.season } else { w.date.year() }), "stage": stage_label,
        "prev": prev, "next": next, "meta": meta,
    });
    if req.light.unwrap_or(false) {
        return Ok(head);
    }

    // Fixtures of this season, in date order.
    let mut fixtures: Vec<&Fixture> = w.fixtures.between(st.start.add_days(-1), st.end.add_days(1)).map(|f| w.fixtures.get(f)).filter(|f| f.comp == id).collect();
    fixtures.sort_by_key(|f| (f.date, f.uid));
    let held: Vec<&Fixture> = fixtures.iter().copied().filter(|f| f.score.is_some() && c.is_concealed(f.uid)).collect();
    let n_held = held.len();

    // ---- the strip of results ----
    let played: Vec<&Fixture> = fixtures.iter().copied().filter(|f| f.score.is_some()).collect();
    let recent_from = played.len().saturating_sub(10);
    let mut ticker: Vec<Value> = played[recent_from..]
        .iter()
        .map(|f| {
            let masked = c.is_concealed(f.uid);
            let s = f.score.filter(|_| !masked);
            json!({
                "uid": f.uid, "date": f.date.0, "round": f.round, "status": if masked { "held" } else { "ft" },
                "home": team(c, f.home), "away": team(c, f.away),
                "hs": s.map(|s| s.home), "as": s.map(|s| s.away),
                "pens": s.and_then(|s| s.pens).map(|(h, a)| json!([h, a])),
            })
        })
        .collect();
    let upcoming: Vec<&Fixture> = fixtures.iter().copied().filter(|f| f.score.is_none() && f.date >= today).take(4).collect();
    ticker.extend(upcoming.iter().map(|f| {
        json!({
            "uid": f.uid, "date": f.date.0, "round": f.round, "status": "next",
            "home": team(c, f.home), "away": team(c, f.away), "hs": Value::Null, "as": Value::Null, "pens": Value::Null,
        })
    }));

    // ---- the round or the table ----
    let groups = match comp.format {
        Format::Groups { groups, .. } => groups,
        _ => 0,
    };
    let left = if comp.is_league() && st.table.is_empty() && !st.entrants.is_empty() {
        // Before the first round the table does not exist yet: the entrants, in name order, with nothing played.
        let mut names: Vec<(String, TeamId)> = st.entrants.iter().map(|&t| (c.team_name(t), t)).collect();
        names.sort();
        let rows: Vec<Value> = names.iter().enumerate().map(|(i, (_, t))| json!({"pos": i + 1, "team": team(c, *t), "played": 0, "gd": 0, "points": 0, "zone": Value::Null})).collect();
        let total = rows.len();
        json!({"kind": "table", "title": "Table", "rows": rows, "total": total, "shown": total})
    } else if (comp.is_league() || matches!(st.stage, Stage::Groups)) && !st.table.is_empty() {
        let (rows, _) = crate::tables::visible_table(c, id);
        let mine = st.entrants.iter().copied().find(|&t| c.w.teams[t].club == c.my_club()).and_then(|t| rows.iter().find(|r| r.team == t)).map_or(0, |r| r.group);
        let group = if groups > 1 { mine } else { 0 };
        let rows: Vec<_> = rows.into_iter().filter(|r| groups <= 1 || r.group == group).collect();
        let total = rows.len();
        let zones = |pos: usize| -> Option<&'static str> {
            if !comp.is_league() {
                return None;
            }
            if pos <= usize::from(comp.promote) && comp.tier > 1 {
                Some("up")
            } else if pos > total.saturating_sub(usize::from(comp.relegate)) && comp.relegate > 0 {
                Some("down")
            } else {
                None
            }
        };
        let keep: Vec<usize> = {
            let my = rows.iter().position(|r| c.w.teams[r.team].club == c.my_club());
            if total <= 24 {
                (0..total).collect()
            } else {
                let mut k: Vec<usize> = (0..14).collect();
                if let Some(m) = my.filter(|&m| m >= 14) {
                    k.extend(m.saturating_sub(1)..(m + 2).min(total));
                }
                k
            }
        };
        let out: Vec<Value> = keep
            .iter()
            .map(|&i| {
                let r = &rows[i];
                json!({
                    "pos": i + 1, "team": team(c, r.team), "played": r.played, "gd": i32::from(r.gf) - i32::from(r.ga),
                    "points": r.points, "zone": zones(i + 1),
                })
            })
            .collect();
        json!({
            "kind": "table", "title": if groups > 1 { format!("Group {}", char::from(b'A' + group)) } else { "Table".to_string() },
            "rows": out, "total": total, "shown": keep.len(),
        })
    } else {
        // Knockout: the round now being played, or the last one that was.
        let mut tie_round: BTreeMap<u16, u8> = BTreeMap::new();
        for f in &fixtures {
            if f.tie != u16::MAX {
                tie_round.entry(f.tie).or_insert(round_of(f));
            }
        }
        let cur = tie_round.values().copied().max();
        let mut ties: Vec<Value> = Vec::new();
        let mut date: Option<Date> = None;
        for (i, t) in st.ties.iter().enumerate() {
            // A slot whose sides are not decided yet has no team to draw.
            if t.a.is_none() || t.b.is_none() || Some(tie_round.get(&(i as u16)).copied().unwrap_or(0)) != cur {
                continue;
            }
            let masked = held.iter().any(|f| f.tie == i as u16);
            let first = fixtures.iter().find(|f| f.tie == i as u16).map(|f| f.date);
            date = date.or(first);
            let complete = t.winner.is_some() && !masked;
            ties.push(json!({
                "a": team(c, t.a), "b": team(c, t.b),
                "goals_a": if complete { json!(t.goals_a) } else { Value::Null }, "goals_b": if complete { json!(t.goals_b) } else { Value::Null },
                "played": t.played, "legs": t.legs, "hidden": masked,
                "winner": if complete { json!(if t.winner == t.a { "a" } else { "b" }) } else { Value::Null },
            }));
        }
        let total = ties.len();
        json!({"kind": "ties", "title": comp.short_name, "round": stage_label, "ties": ties, "total": total, "date": date.map(|d| d.0)})
    };

    // ---- team statistics from the results the viewer can see ----
    let mut agg: BTreeMap<TeamId, TeamAgg> = BTreeMap::new();
    for f in fixtures.iter().filter(|f| !c.is_concealed(f.uid)) {
        let Some(s) = f.score else { continue };
        for (t, gf, ga, opp) in [(f.home, s.home, s.away, f.away), (f.away, s.away, s.home, f.home)] {
            let a = agg.entry(t).or_default();
            a.played += 1;
            a.gf += u16::from(gf);
            a.ga += u16::from(ga);
            a.cs += u16::from(ga == 0);
            if gf > ga {
                a.won += 1;
                let margin = gf - ga;
                if a.best.is_none_or(|b| margin > b.0) {
                    a.best = Some((margin, gf, ga, opp));
                }
            }
        }
    }
    // Statistics that cannot be taken back out of a hidden match: shots, chances and cards.
    let mut xg: BTreeMap<ClubId, ClubAgg> = BTreeMap::new();
    let lines: Vec<_> = w.stats.for_comp(id).filter(|l| l.season == st.season).copied().collect();
    if n_held == 0 {
        for l in &lines {
            let e = xg.entry(l.club).or_default();
            e.xg += l.xg;
            e.shots += u32::from(l.shots);
            e.yellows += u32::from(l.yellows);
            e.reds += u32::from(l.reds);
        }
    }

    let club_of = |t: TeamId| w.teams[t].club;
    let team_top = |key: &dyn Fn(&TeamAgg, TeamId) -> Ranked, asc: bool| -> Vec<Value> {
        let mut v: Vec<(f64, TeamId, String)> = agg.iter().filter_map(|(&t, a)| key(a, t).map(|(x, s)| (x, t, s))).collect();
        v.sort_by(|a, b| if asc { a.0.total_cmp(&b.0) } else { b.0.total_cmp(&a.0) }.then(a.1.cmp(&b.1)));
        v.into_iter().take(3).map(|(_, t, s)| json!({"team": team(c, t), "value": s})).collect()
    };
    let club_top = |m: &dyn Fn(&ClubAgg) -> f64, dec: bool| -> Vec<Value> {
        let mut v: Vec<(f64, ClubId)> = xg.iter().map(|(&cl, e)| (m(e), cl)).filter(|x| x.0 > 0.0).collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        v.into_iter().take(3).map(|(x, cl)| json!({"team": club_team(c, cl), "value": if dec { format!("{x:.2}") } else { format!("{}", x as u32) }})).collect()
    };
    let mut team_sections: Vec<Block> = vec![
        Block::new(1, "Goals", team_top(&|a, _| (a.gf > 0).then(|| (f64::from(a.gf), a.gf.to_string())), false)),
        Block::new(1, "Fewest Goals Conceded", team_top(&|a, _| (a.played > 0).then(|| (f64::from(a.ga), a.ga.to_string())), true)),
        Block::new(1, "Clean Sheets", team_top(&|a, _| (a.cs > 0).then(|| (f64::from(a.cs), a.cs.to_string())), false)),
        Block::new(1, "Biggest Win", team_top(&|a, _| a.best.filter(|b| b.0 >= 2).map(|b| (f64::from(b.0) * 100.0 + f64::from(b.1), format!("{}-{}", b.1, b.2))), false)),
        Block::new(1, "Expected Goals For", club_top(&|e| f64::from(e.xg), true)),
        Block::new(2, "Wins", team_top(&|a, _| (a.won > 0).then(|| (f64::from(a.won), a.won.to_string())), false)),
        Block::new(2, "Shots", club_top(&|e| f64::from(e.shots), false)),
        Block::new(2, "Yellow Cards", club_top(&|e| f64::from(e.yellows), false)),
        Block::new(2, "Red Cards", club_top(&|e| f64::from(e.reds), false)),
    ];
    team_sections.retain(|s| !s.rows.is_empty());
    let _ = club_of;

    // ---- player statistics, with unrevealed matches taken out where that is possible ----
    struct P {
        player: PlayerId,
        club: ClubId,
        apps: i64,
        goals: i64,
        assists: i64,
        rating: i64,
        minutes: i64,
        cs: i64,
        pom: i64,
        yellows: i64,
        reds: i64,
        xg: f64,
        shots: i64,
        key: i64,
        tackles: i64,
    }
    let mut rows: Vec<P> = Vec::with_capacity(lines.len());
    for l in &lines {
        let mut p = P {
            player: l.player,
            club: l.club,
            apps: i64::from(l.apps),
            goals: i64::from(l.goals),
            assists: i64::from(l.assists),
            rating: i64::from(l.rating_sum),
            minutes: i64::from(l.minutes),
            cs: i64::from(l.clean_sheets),
            pom: i64::from(l.pom),
            yellows: i64::from(l.yellows),
            reds: i64::from(l.reds),
            xg: f64::from(l.xg),
            shots: i64::from(l.shots),
            key: i64::from(l.key_passes),
            tackles: i64::from(l.tackles),
        };
        if n_held > 0 {
            for a in c.unrevealed_apps(l.player).into_iter().filter(|a| a.comp == id && a.club == l.club && a.minutes > 0) {
                p.apps -= 1;
                p.goals -= i64::from(a.goals);
                p.assists -= i64::from(a.assists);
                p.rating -= i64::from(a.rating);
                p.minutes -= i64::from(a.minutes);
            }
        }
        rows.push(p);
    }
    let max_apps = rows.iter().map(|p| p.apps).max().unwrap_or(0);
    let min_apps = (max_apps / 4).clamp(1, 5);
    let player_row = |p: &P, value: String, pill: bool| -> Value {
        json!({
            "p": named(Ref::person(w.players.cold[p.player].person), c.player_short(p.player)),
            "team": club_team(c, p.club), "value": value, "pill": pill,
        })
    };
    let top = |rows: &[&P], key: &dyn Fn(&P) -> f64, fmt: &dyn Fn(f64) -> String, pill: bool| -> Vec<Value> {
        let mut v: Vec<(&P, f64)> = rows.iter().map(|&p| (p, key(p))).filter(|x| x.1 > 0.0).collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.minutes.cmp(&b.0.minutes)).then(a.0.player.cmp(&b.0.player)));
        v.into_iter().take(3).map(|(p, x)| player_row(p, fmt(x), pill)).collect()
    };
    let all: Vec<&P> = rows.iter().filter(|p| p.apps > 0).collect();
    let int = |x: f64| format!("{}", x as i64);
    let mut player_sections: Vec<Block> = vec![
        Block::new(1, "Goals", top(&all, &|p| p.goals as f64, &int, false)).show_if(true),
        Block::new(1, "Assists", top(&all, &|p| p.assists as f64, &int, false)).show_if(true),
        Block::new(1, "Average Rating", top(&all.iter().copied().filter(|p| p.apps >= min_apps).collect::<Vec<_>>(), &|p| p.rating as f64 / 10.0 / p.apps as f64, &|x| format!("{x:.2}"), true))
            .show_if(true),
        Block::new(1, "Player of the Match", top(&all, &|p| p.pom as f64, &int, false)).show_if(n_held == 0),
        Block::new(1, "Clean Sheets", top(&all, &|p| p.cs as f64, &int, false)).show_if(n_held == 0),
        Block::new(2, "Expected Goals", top(&all, &|p| p.xg, &|x| format!("{x:.2}"), false)).show_if(n_held == 0),
        Block::new(2, "Shots", top(&all, &|p| p.shots as f64, &int, false)).show_if(n_held == 0),
        Block::new(2, "Key Passes", top(&all, &|p| p.key as f64, &int, false)).show_if(n_held == 0),
        Block::new(2, "Tackles Won", top(&all, &|p| p.tackles as f64, &int, false)).show_if(n_held == 0),
        Block::new(2, "Yellow Cards", top(&all, &|p| p.yellows as f64, &int, false)).show_if(n_held == 0),
        Block::new(2, "Red Cards", top(&all, &|p| p.reds as f64, &int, false)).show_if(n_held == 0),
    ];
    // What was left out because it cannot be taken back out of a hidden match.
    let withheld: Vec<&str> = player_sections.iter().filter(|s| !s.show).map(|s| s.title).collect();
    player_sections.retain(|s| s.show && !s.rows.is_empty());

    // ---- what is written about it ----
    let cutoff = today.add_days(-45);
    let uids: std::collections::HashSet<u64> = played.iter().map(|f| f.uid).collect();
    let mut news: Vec<Value> = Vec::new();
    let stories = &w.media.stories;
    let mut i = stories.len();
    while i > 0 && news.len() < 4 {
        i -= 1;
        let s = &stories[pw_core::StoryId(i as u32)];
        if s.date < cutoff {
            break;
        }
        let Some(StoryLink::Fixture { uid, home, away, hg, ag, .. }) = w.media.links.get(&s.id) else { continue };
        if !uids.contains(uid) {
            continue;
        }
        let spoils = c.story_spoils(s);
        // Two outlets running the same headline for one match are one piece of news.
        let headline = c.headline(s);
        if news.iter().any(|n| n["match"]["id"] == json!(*uid as u32) && n["headline"] == json!(headline)) {
            continue;
        }
        news.push(json!({
            "headline": c.headline(s), "date": s.date.0, "days_ago": today.0 - s.date.0, "outlet": pw_narrate::press::outlet_name(w, s),
            "match": Ref::fixture(*uid), "spoils": spoils,
            "home": club_team(c, *home), "away": club_team(c, *away),
            "score": if spoils { Value::Null } else { json!([hg, ag]) },
        }));
    }

    let mut out = head;
    out["ticker"] = json!(ticker);
    out["left"] = left;
    out["players"] = json!(player_sections.iter().map(Block::json).collect::<Vec<_>>());
    out["teams_stats"] = json!(team_sections.iter().map(Block::json).collect::<Vec<_>>());
    out["held"] = json!({"results": n_held, "withheld": if n_held > 0 { withheld } else { Vec::new() }});
    out["news"] = json!(news);
    Ok(out)
}

/// Badge colours of every club, so any list can draw a crest without carrying colours on each row.
pub fn crest_colors(c: &Ctx) -> ApiResult<Value> {
    let map: serde_json::Map<String, Value> = c.w.clubs.iter_enumerated().map(|(id, cl)| (id.0.to_string(), json!([hex(cl.colors[0]), hex(cl.colors[1])]))).collect();
    Ok(json!({"colors": map}))
}
