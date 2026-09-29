use pw_core::{CompId, NationId, TeamId};
use pw_world::comp::{Stage, TableRow, sort_table};
use pw_world::{CompKind, Format};
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref, Tone};
use crate::table::{Key, Source, f_str, f_u32};

pub fn stage_text(c: &Ctx, comp: CompId) -> String {
    let st = &c.w.comps[comp].state;
    match st.stage {
        Stage::NotStarted => "Not started".into(),
        Stage::League => "In progress".into(),
        Stage::Groups => "Group stage".into(),
        Stage::Knockout(r) => crate::pages::compview::knockout_round_name(r),
        Stage::Finished => "Finished".into(),
    }
}

pub fn kind_text(k: CompKind) -> &'static str {
    match k {
        CompKind::League => "League",
        CompKind::Cup => "Cup",
        CompKind::Continental => "Continental",
        CompKind::SuperCup => "Super cup",
    }
}

pub struct Comps;

const G: &str = "general";

impl Source for Comps {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("name", "Competition", Fmt::Text, 220, &[G]),
            Col::new("nation", "Nation", Fmt::Text, 130, &[G]).left(),
            Col::new("kind", "Type", Fmt::Text, 90, &[G]).left(),
            Col::new("tier", "Level", Fmt::Int, 54, &[G]),
            Col::new("teams", "Teams", Fmt::Int, 56, &[G]),
            Col::new("stage", "Stage", Fmt::Text, 140, &[G]).left(),
            Col::new("season", "Season", Fmt::Text, 76, &[G]).left(),
            Col::new("leader", "Leader / holder", Fmt::Text, 170, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("tier", false)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let nation = f_u32(f, "nation").map(NationId);
        let kind = f_str(f, "kind");
        let q = f_str(f, "q").map(str::to_lowercase);
        c.w.comps
            .iter_enumerated()
            .filter(|(_, x)| x.team_kind == pw_world::TeamKind::First)
            .filter(|(_, x)| nation.is_none_or(|n| x.nation == n))
            .filter(|(_, x)| {
                kind.is_none_or(|k| match k {
                    "league" => x.kind == CompKind::League,
                    "cup" => matches!(x.kind, CompKind::Cup | CompKind::SuperCup),
                    "continental" => x.kind == CompKind::Continental,
                    _ => true,
                })
            })
            .filter(|(_, x)| q.as_ref().is_none_or(|q| x.name.to_lowercase().contains(q.as_str())))
            .map(|(id, _)| id.0)
            .collect()
    }

    fn key(&self, c: &Ctx, p: &(), id: u32, col: &str) -> Key {
        self.v(c, p, id, col).1
    }

    fn cell(&self, c: &Ctx, p: &(), id: u32, col: &str) -> Cell {
        self.v(c, p, id, col).0
    }

    fn open(&self, _c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(Ref::comp(CompId(id)))
    }
}

impl Comps {
    fn v(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> (Cell, Key) {
        let w = c.w;
        let cid = CompId(id);
        let comp = &w.comps[cid];
        let num = |x: f64| (Cell::num(x), Key::Num(x));
        match col {
            "name" => (Cell::link(Ref::comp(cid), comp.name.clone()), Key::text(&comp.name)),
            "nation" => {
                if comp.nation.is_none() {
                    (Cell::text("International"), Key::text("~"))
                } else {
                    let n = c.nation_name(comp.nation);
                    (Cell::link(Ref::nation(comp.nation), n.clone()), Key::text(n))
                }
            }
            "kind" => (Cell::text(kind_text(comp.kind)), Key::text(kind_text(comp.kind))),
            "tier" => {
                if comp.kind == CompKind::League {
                    num(f64::from(comp.tier))
                } else {
                    (Cell::empty(), Key::None)
                }
            }
            "teams" => num(comp.state.entrants.len() as f64),
            "stage" => {
                let t = stage_text(c, cid);
                (Cell::text(t.clone()), Key::text(t))
            }
            "season" => (Cell::text(c.season_label(cid, comp.state.season)), Key::Num(f64::from(comp.state.season))),
            "leader" => {
                let team = if comp.state.winner.is_some() {
                    comp.state.winner
                } else if comp.is_league() {
                    comp.sorted_table().first().map_or(TeamId::NONE, |r| r.team)
                } else {
                    TeamId::NONE
                };
                if team.is_none() {
                    (Cell::empty(), Key::None)
                } else {
                    let n = c.team_short(team);
                    let mut cell = Cell::link(c.team_ref(team), n.clone());
                    if comp.state.winner.is_some() {
                        cell = cell.with_sub("Winner");
                    }
                    (cell, Key::text(n))
                }
            }
            _ => (Cell::empty(), Key::None),
        }
    }
}

// ---- standings ---------------------------------------------------------------------

pub struct StandingsPrep {
    pub rows: Vec<TableRow>,
    pub form: rustc_hash::FxHashMap<TeamId, Vec<char>>,
    pub comp: CompId,
    pub hidden: usize,
}

pub struct Standings;

const ST: &str = "general";

/// Table rows for a competition with any unrevealed results taken back out.
pub fn visible_table(c: &Ctx, comp: CompId) -> (Vec<TableRow>, usize) {
    let mut rows = c.w.comps[comp].state.table.clone();
    let mut hidden = 0;
    for f in c.concealed_fixtures() {
        if f.comp != comp {
            continue;
        }
        let Some(s) = f.score else { continue };
        hidden += 1;
        for (team, gf, ga) in [(f.home, s.home, s.away), (f.away, s.away, s.home)] {
            if let Some(r) = rows.iter_mut().find(|r| r.team == team) {
                r.played = r.played.saturating_sub(1);
                r.gf = r.gf.saturating_sub(u16::from(gf));
                r.ga = r.ga.saturating_sub(u16::from(ga));
                match gf.cmp(&ga) {
                    std::cmp::Ordering::Greater => {
                        r.won = r.won.saturating_sub(1);
                        r.points -= 3;
                    }
                    std::cmp::Ordering::Equal => {
                        r.drawn = r.drawn.saturating_sub(1);
                        r.points -= 1;
                    }
                    std::cmp::Ordering::Less => r.lost = r.lost.saturating_sub(1),
                }
            }
        }
    }
    sort_table(&mut rows);
    (rows, hidden)
}

impl Source for Standings {
    type Prep = StandingsPrep;

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("pos", "Pos", Fmt::Int, 44, &[ST]).nosort(),
            Col::new("team", "Team", Fmt::Text, 200, &[ST]),
            Col::new("played", "P", Fmt::Int, 40, &[ST]).help("Played"),
            Col::new("won", "W", Fmt::Int, 38, &[ST]),
            Col::new("drawn", "D", Fmt::Int, 38, &[ST]),
            Col::new("lost", "L", Fmt::Int, 38, &[ST]),
            Col::new("gf", "GF", Fmt::Int, 42, &[ST]).help("Goals for"),
            Col::new("ga", "GA", Fmt::Int, 42, &[ST]).help("Goals against"),
            Col::new("gd", "GD", Fmt::Int, 46, &[ST]).help("Goal difference"),
            Col::new("points", "Pts", Fmt::Int, 48, &[ST]),
            Col::new("form", "Form", Fmt::Text, 90, &[ST]).center().nosort().help("Last five results, most recent last"),
        ]
    }

    fn natural_order(&self) -> bool {
        true
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("points", true)
    }

    fn prep(&self, c: &Ctx, f: &Value) -> StandingsPrep {
        let comp = CompId(f_u32(f, "comp").unwrap_or(u32::MAX));
        if comp.is_none() || comp.0 as usize >= c.w.comps.len() {
            return StandingsPrep { rows: vec![], form: Default::default(), comp: CompId::NONE, hidden: 0 };
        }
        let (mut rows, hidden) = visible_table(c, comp);
        if let Some(g) = f_u32(f, "group") {
            rows.retain(|r| u32::from(r.group) == g);
        }
        let st = &c.w.comps[comp].state;
        let mut form: rustc_hash::FxHashMap<TeamId, Vec<char>> = rustc_hash::FxHashMap::default();
        let mut played: Vec<_> =
            c.w.fixtures.between(st.start.add_days(-1), c.w.date).map(|id| c.w.fixtures.get(id)).filter(|f| f.comp == comp && f.score.is_some() && !c.is_concealed(f.uid)).collect();
        played.sort_by_key(|f| (f.date, f.uid));
        for f in played {
            let s = f.score.expect("filtered");
            let (h, a) = match s.home.cmp(&s.away) {
                std::cmp::Ordering::Greater => ('W', 'L'),
                std::cmp::Ordering::Less => ('L', 'W'),
                std::cmp::Ordering::Equal => match s.pens {
                    Some((ph, pa)) if ph > pa => ('W', 'L'),
                    Some((ph, pa)) if pa > ph => ('L', 'W'),
                    _ => ('D', 'D'),
                },
            };
            form.entry(f.home).or_default().push(h);
            form.entry(f.away).or_default().push(a);
        }
        StandingsPrep { rows, form, comp, hidden }
    }

    fn ids(&self, _c: &Ctx, p: &StandingsPrep, _f: &Value) -> Vec<u32> {
        (0..p.rows.len() as u32).collect()
    }

    fn key(&self, c: &Ctx, p: &StandingsPrep, id: u32, col: &str) -> Key {
        let r = &p.rows[id as usize];
        match col {
            "team" => Key::text(c.team_short(r.team)),
            "played" => Key::Num(f64::from(r.played)),
            "won" => Key::Num(f64::from(r.won)),
            "drawn" => Key::Num(f64::from(r.drawn)),
            "lost" => Key::Num(f64::from(r.lost)),
            "gf" => Key::Num(f64::from(r.gf)),
            "ga" => Key::Num(f64::from(r.ga)),
            "gd" => Key::Num(f64::from(r.gd())),
            "points" => Key::Num(f64::from(r.points) * 100000.0 + f64::from(r.gd()) * 100.0 + f64::from(r.gf).min(99.0)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, p: &StandingsPrep, id: u32, col: &str) -> Cell {
        let r = &p.rows[id as usize];
        let num = |x: f64| Cell::num(x);
        match col {
            "pos" => num(f64::from(id) + 1.0),
            "team" => {
                let n = c.team_short(r.team);
                let mut cell = Cell::link(c.team_ref(r.team), n);
                if c.my_team() == r.team {
                    cell = cell.with_sub("Your team");
                }
                cell
            }
            "played" => num(f64::from(r.played)),
            "won" => num(f64::from(r.won)),
            "drawn" => num(f64::from(r.drawn)),
            "lost" => num(f64::from(r.lost)),
            "gf" => num(f64::from(r.gf)),
            "ga" => num(f64::from(r.ga)),
            "gd" => num(f64::from(r.gd())),
            "points" => num(f64::from(r.points)),
            "form" => {
                let f = p.form.get(&r.team).map(|v| v.iter().rev().take(5).rev().collect::<String>()).unwrap_or_default();
                Cell::text(f)
            }
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, p: &StandingsPrep, id: u32) -> Option<Ref> {
        Some(c.team_ref(p.rows[id as usize].team))
    }

    fn row_tone(&self, c: &Ctx, p: &StandingsPrep, id: u32) -> Option<Tone> {
        if p.comp.is_none() {
            return None;
        }
        let comp = &c.w.comps[p.comp];
        let r = &p.rows[id as usize];
        if c.my_team() == r.team {
            return Some(Tone::Info);
        }
        if !comp.is_league() {
            return None;
        }
        let n = p.rows.len();
        let pos = id as usize;
        if pos < usize::from(comp.promote) {
            Some(Tone::Pos)
        } else if comp.relegate > 0 && pos >= n.saturating_sub(usize::from(comp.relegate)) {
            Some(Tone::Neg)
        } else {
            None
        }
    }

    fn note(&self, c: &Ctx, p: &StandingsPrep, _f: &Value) -> Option<String> {
        if p.comp.is_none() {
            return None;
        }
        let comp = &c.w.comps[p.comp];
        let mut parts = Vec::new();
        if comp.is_league() && comp.promote > 0 {
            parts.push(format!("Top {} are promoted", comp.promote));
        }
        if comp.is_league() && comp.relegate > 0 {
            parts.push(format!("bottom {} are relegated", comp.relegate));
        }
        if p.hidden > 0 {
            parts.push(format!("{} of your results are not included until you reveal them", p.hidden));
        }
        (!parts.is_empty()).then(|| {
            let mut s = parts.join("; ");
            s.push('.');
            let mut chars = s.chars();
            chars.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + chars.as_str())
        })
    }
}

pub fn format_text(f: &Format) -> String {
    match *f {
        Format::League { rounds } => {
            if rounds >= 2 {
                format!("League, each team plays every other team {rounds} times")
            } else {
                "League, single round".into()
            }
        }
        Format::Knockout { legs, final_legs } => {
            format!(
                "Knockout, {} per tie, {} in the final",
                if legs == 1 { "one leg".to_string() } else { format!("{legs} legs") },
                if final_legs == 1 { "one leg".to_string() } else { format!("{final_legs} legs") }
            )
        }
        Format::Groups { groups, size, advance, .. } => {
            format!("{groups} groups of {size}, top {advance} advance to a knockout stage")
        }
    }
}
