use std::cell::OnceCell;

use pw_core::{ClubId, PlayerId, Pos, PosGroup, TeamId};
use pw_sim::health;
use pw_world::{PlayerStatus, TeamKind};
use rustc_hash::FxHashMap;
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref, Tone};
use crate::table::{Key, Source, f_bool, f_i32, f_str, f_u32};

#[derive(Clone, Copy, Default)]
pub struct StatAgg {
    pub apps: u32,
    pub starts: u32,
    pub minutes: u32,
    pub goals: u32,
    pub assists: u32,
    pub yellows: u32,
    pub reds: u32,
    pub rating_sum: u32,
    pub xg: f32,
}

impl StatAgg {
    pub fn avg(&self) -> Option<f64> {
        (self.apps > 0).then(|| f64::from(self.rating_sum) / 10.0 / f64::from(self.apps))
    }
}

pub struct Prep {
    stats: OnceCell<FxHashMap<PlayerId, StatAgg>>,
}

impl Prep {
    fn agg(&self, c: &Ctx, p: PlayerId) -> StatAgg {
        let m = self.stats.get_or_init(|| {
            let mut m: FxHashMap<PlayerId, StatAgg> = FxHashMap::default();
            for l in c.w.stats.iter() {
                let a = m.entry(l.player).or_default();
                a.apps += u32::from(l.apps);
                a.starts += u32::from(l.starts);
                a.minutes += l.minutes;
                a.goals += u32::from(l.goals);
                a.assists += u32::from(l.assists);
                a.yellows += u32::from(l.yellows);
                a.reds += u32::from(l.reds);
                a.rating_sum += l.rating_sum;
                a.xg += l.xg;
            }
            m
        });
        m.get(&p).copied().unwrap_or_default()
    }
}

pub struct Players;

const G: &str = "general";
const SEL: &str = "selection";
const PERF: &str = "performance";
const CON: &str = "contracts";
const DEV: &str = "development";
const INH: &str = "inhabit";

pub fn avail_cell(c: &Ctx, p: PlayerId) -> (Cell, Key) {
    let h = &c.w.players.hot[p];
    match h.status {
        PlayerStatus::Retired => (Cell::text("Retired").tone(Tone::Muted), Key::Num(-1.0)),
        PlayerStatus::FreeAgent => (Cell::text("Free agent").tone(Tone::Muted), Key::Num(0.5)),
        PlayerStatus::Active => {
            if h.injury != 0 {
                let name = health::injury_name(c.w, h.injury);
                (
                    Cell::text(format!("Injured · {} d", h.injury_days)).tone(Tone::Neg).with_sub(name.to_string()).with_num(f64::from(h.injury_days)),
                    Key::Num(100.0 + f64::from(h.injury_days)),
                )
            } else if h.ban > 0 {
                (Cell::text(format!("Suspended · {}", h.ban)).tone(Tone::Warn).with_num(f64::from(h.ban)), Key::Num(50.0 + f64::from(h.ban)))
            } else {
                (Cell::text("Available"), Key::Num(0.0))
            }
        }
    }
}

fn pos_text(c: &Ctx, p: PlayerId) -> String {
    let cold = &c.w.players.cold[p];
    let mut v: Vec<&str> = vec![cold.best_pos.code()];
    for pos in cold.natural_positions() {
        if pos != cold.best_pos && v.len() < 3 {
            v.push(pos.code());
        }
    }
    v.join(", ")
}

impl Source for Players {
    type Prep = Prep;

    fn id(&self) -> &'static str {
        "players"
    }

    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let obs = c.observer();
        let mut v = vec![
            Col::new("name", "Name", Fmt::Text, 190, &[G, SEL, PERF, CON, DEV, INH]),
            Col::new("age", "Age", Fmt::Int, 46, &[G, SEL, PERF, CON, DEV, INH]),
            Col::new("pos", "Position", Fmt::Text, 96, &[G, SEL, PERF, DEV, INH]).left(),
            Col::new("nation", "Nationality", Fmt::Text, 120, &[G, INH, CON]).left(),
            Col::new("club", "Club", Fmt::Text, 160, &[G, INH, CON, DEV]).left(),
            Col::new("team", "Team", Fmt::Text, 90, &[INH, DEV]).left(),
            Col::new("league", "Competition", Fmt::Text, 150, &[INH]).left(),
            Col::new("avail", "Availability", Fmt::Text, 130, &[G, SEL, INH]).left(),
            Col::new("foot", "Foot", Fmt::Text, 52, &[DEV]).center(),
            Col::new("height", "Height", Fmt::Int, 60, &[DEV]).help("Centimetres"),
            Col::new("shirt", "No.", Fmt::Int, 44, &[SEL]),
            Col::new("apps", "Apps", Fmt::Int, 52, &[G, PERF, SEL]).help("Appearances this season"),
            Col::new("starts", "Starts", Fmt::Int, 56, &[PERF, SEL]),
            Col::new("mins", "Minutes", Fmt::Int, 66, &[G, PERF, SEL]),
            Col::new("goals", "Goals", Fmt::Int, 56, &[G, PERF]),
            Col::new("assists", "Assists", Fmt::Int, 60, &[G, PERF]),
            Col::new("xg", "xG", Fmt::Dec1, 52, &[PERF]).help("Expected goals, where recorded"),
            Col::new("rating", "Rating", Fmt::Dec2, 62, &[G, PERF, SEL]).help("Average match rating this season"),
            Col::new("cards", "Cards", Fmt::Text, 62, &[PERF]).center(),
            Col::new("joined", "Joined", Fmt::Date, 96, &[DEV, CON]),
        ];
        if obs {
            v.extend([
                Col::new("ca", "Ability", Fmt::Int, 62, &[DEV, G]).help("Current ability (observer only)"),
                Col::new("pa", "Potential", Fmt::Int, 66, &[DEV]).help("Potential ability (observer only)"),
                Col::new("status", "Squad status", Fmt::Text, 130, &[SEL, CON]).left(),
                Col::new("wage", "Wage / wk", Fmt::Money, 90, &[CON]),
                Col::new("contract_end", "Contract ends", Fmt::Date, 100, &[CON, G]),
                Col::new("value", "Value", Fmt::Money, 92, &[CON, DEV]),
                Col::new("condition", "Condition", Fmt::Int, 70, &[SEL]),
                Col::new("sharpness", "Sharpness", Fmt::Int, 72, &[SEL]),
                Col::new("morale", "Morale", Fmt::Int, 60, &[SEL]),
                Col::new("clause", "Release clause", Fmt::Money, 100, &[CON]),
            ]);
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("name", false)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) -> Prep {
        Prep { stats: OnceCell::new() }
    }

    fn ids(&self, c: &Ctx, _p: &Prep, f: &Value) -> Vec<u32> {
        let w = c.w;
        let q = f_str(f, "q").map(str::to_lowercase);
        let club = f_u32(f, "club").map(ClubId);
        let team = f_u32(f, "team").map(TeamId);
        let kind = f_str(f, "kind");
        let nation = f_u32(f, "nation");
        let group = f_str(f, "group");
        let pos = f_str(f, "pos").and_then(Pos::from_code);
        let (amin, amax) = (f_i32(f, "age_min"), f_i32(f, "age_max"));
        let status = f_str(f, "status");
        let comp = f_u32(f, "comp").map(pw_core::CompId);
        let injured = f_bool(f, "injured");
        let loaned = f_bool(f, "loaned");
        let inhabitable = f_bool(f, "inhabitable").unwrap_or(false);
        let ids_only: Option<Vec<u32>> = f.get("ids").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_u64().map(|x| x as u32)).collect());
        let obs = c.observer();
        let expiring = if obs { f_i32(f, "expiring_days") } else { None };
        let min_ca = if obs { f_i32(f, "min_ca") } else { None };
        let entrants = comp.map(|cid| w.comps[cid].state.entrants.clone());

        let mut out = Vec::new();
        let iter: Box<dyn Iterator<Item = PlayerId>> = if let Some(t) = team {
            Box::new(w.teams[t].squad.clone().into_iter())
        } else if let Some(ids) = &ids_only {
            Box::new(ids.clone().into_iter().map(PlayerId))
        } else {
            Box::new(w.players.ids())
        };
        for p in iter {
            let h = &w.players.hot[p];
            match status {
                Some("active") => {
                    if h.status != PlayerStatus::Active {
                        continue;
                    }
                }
                Some("free") => {
                    if h.status != PlayerStatus::FreeAgent {
                        continue;
                    }
                }
                Some("retired") => {
                    if h.status != PlayerStatus::Retired {
                        continue;
                    }
                }
                Some("any") => {}
                _ => {
                    if h.status == PlayerStatus::Retired && team.is_none() && ids_only.is_none() {
                        continue;
                    }
                }
            }
            if inhabitable && (h.status != PlayerStatus::Active || h.club.is_none()) {
                continue;
            }
            if let Some(cl) = club {
                let on_loan_here = w.players.cold[p].loan.as_ref().is_some_and(|l| l.club == cl);
                if h.club != cl && !on_loan_here {
                    continue;
                }
            }
            if let Some(k) = kind {
                if h.team.is_none() || !kind_matches(w.teams[h.team].kind, k) {
                    continue;
                }
            }
            let cold = &w.players.cold[p];
            let person = &w.people[cold.person];
            if let Some(n) = nation {
                if person.nation.0 != n && person.nation2.0 != n {
                    continue;
                }
            }
            if let Some(g) = group {
                let want = match g {
                    "gk" => PosGroup::Gk,
                    "def" => PosGroup::Def,
                    "mid" => PosGroup::Mid,
                    _ => PosGroup::Att,
                };
                if cold.best_pos.group() != want {
                    continue;
                }
            }
            if let Some(ps) = pos {
                if cold.familiarity[ps.idx()] < 15 {
                    continue;
                }
            }
            if amin.is_some() || amax.is_some() {
                let age = person.age(w.date) as i32;
                if amin.is_some_and(|m| age < m) || amax.is_some_and(|m| age > m) {
                    continue;
                }
            }
            if injured == Some(true) && h.injury == 0 {
                continue;
            }
            if loaned == Some(true) && cold.loan.is_none() {
                continue;
            }
            if let Some(ent) = &entrants {
                if h.team.is_none() || !ent.contains(&h.team) {
                    continue;
                }
            }
            if let Some(days) = expiring {
                if h.club.is_none() || cold.contract.days_left(w.date) > days {
                    continue;
                }
            }
            if let Some(m) = min_ca {
                if i32::from(cold.ca) < m {
                    continue;
                }
            }
            if let Some(q) = &q {
                if !person.display_name(&w.names).to_lowercase().contains(q.as_str()) {
                    continue;
                }
            }
            out.push(p.0);
        }
        out
    }

    fn key(&self, c: &Ctx, p: &Prep, id: u32, col: &str) -> Key {
        self.v(c, p, id, col).1
    }

    fn cell(&self, c: &Ctx, p: &Prep, id: u32, col: &str) -> Cell {
        self.v(c, p, id, col).0
    }

    fn open(&self, c: &Ctx, _p: &Prep, id: u32) -> Option<Ref> {
        Some(c.player_ref(PlayerId(id)))
    }

    fn row_tone(&self, c: &Ctx, _p: &Prep, id: u32) -> Option<Tone> {
        c.is_me(PlayerId(id)).then_some(Tone::Info)
    }

    fn note(&self, c: &Ctx, _p: &Prep, _f: &Value) -> Option<String> {
        (!c.observer()).then(|| "Contract, condition and ability figures are not shown: you cannot see other people's private details.".to_string())
    }
}

fn kind_matches(k: TeamKind, s: &str) -> bool {
    matches!((k, s), (TeamKind::First, "first") | (TeamKind::Reserve, "reserve") | (TeamKind::U21, "u21") | (TeamKind::U19, "u19") | (TeamKind::U18, "u18"))
        || (s == "youth" && k.is_youth())
}

impl Players {
    fn v(&self, c: &Ctx, prep: &Prep, id: u32, col: &str) -> (Cell, Key) {
        let w = c.w;
        let p = PlayerId(id);
        let h = &w.players.hot[p];
        let cold = &w.players.cold[p];
        let person = &w.people[cold.person];
        let num = |n: f64| (Cell::num(n), Key::Num(n));
        match col {
            "name" => {
                let n = person.display_name(&w.names).into_owned();
                let mut cell = Cell::link(Ref::person(cold.person), n.clone());
                if c.is_me(p) {
                    cell = cell.with_sub("You".to_string());
                }
                (cell, Key::text(n))
            }
            "age" => num(f64::from(person.age(w.date))),
            "pos" => {
                let t = pos_text(c, p);
                (Cell::text(t.clone()), Key::Num(cold.best_pos.idx() as f64 * 100.0 + f64::from(255 - cold.ca.min(200))))
            }
            "nation" => {
                let n = c.nation_name(person.nation);
                (Cell::link(Ref::nation(person.nation), n.clone()), Key::text(n))
            }
            "club" => {
                if h.club.is_none() {
                    (Cell::text("—").tone(Tone::Muted), Key::None)
                } else {
                    let n = c.club_short(h.club);
                    let mut cell = Cell::link(Ref::club(h.club), n.clone());
                    if let Some(l) = &cold.loan {
                        if l.club == h.club {
                            cell = cell.with_sub(format!("Loan from {}", c.club_short(l.parent)));
                        }
                    }
                    (cell, Key::text(n))
                }
            }
            "team" => {
                if h.team.is_none() {
                    (Cell::empty(), Key::None)
                } else {
                    let l = w.teams[h.team].kind.label();
                    (Cell::text(l), Key::text(l))
                }
            }
            "league" => match (h.team.get(), h.club.get()) {
                (Some(t), _) => match w.league_of(t) {
                    Some(cid) => {
                        let n = c.comp_short(cid);
                        (Cell::link(Ref::comp(cid), n.clone()), Key::text(n))
                    }
                    None => (Cell::empty(), Key::None),
                },
                _ => (Cell::empty(), Key::None),
            },
            "avail" => avail_cell(c, p),
            "foot" => {
                let (l, r) = (cold.left_foot, cold.right_foot);
                let t = if l >= 15 && r >= 15 { "Both" } else if l > r { "Left" } else { "Right" };
                (Cell::text(t), Key::text(t))
            }
            "height" => num(f64::from(cold.height)),
            "shirt" => {
                if cold.shirt == 0 { (Cell::empty(), Key::None) } else { num(f64::from(cold.shirt)) }
            }
            "apps" | "starts" | "mins" | "goals" | "assists" | "xg" | "rating" | "cards" => {
                let a = prep.agg(c, p);
                match col {
                    "apps" => num(f64::from(a.apps)),
                    "starts" => num(f64::from(a.starts)),
                    "mins" => num(f64::from(a.minutes)),
                    "goals" => num(f64::from(a.goals)),
                    "assists" => num(f64::from(a.assists)),
                    "xg" => num(f64::from(a.xg)),
                    "rating" => match a.avg() {
                        Some(r) if a.apps >= 1 => (Cell::num(r), Key::Num(r)),
                        _ => (Cell::empty(), Key::None),
                    },
                    _ => {
                        let t = if a.yellows + a.reds == 0 { String::from("—") } else { format!("{}Y {}R", a.yellows, a.reds) };
                        (Cell::text(t), Key::Num(f64::from(a.yellows + a.reds * 3)))
                    }
                }
            }
            "joined" => {
                if h.club.is_none() { (Cell::empty(), Key::None) } else { num(f64::from(cold.joined.0)) }
            }
            // ---- observer-only columns (the column list omits them for other viewers) ----
            "ca" => num(f64::from(cold.ca)),
            "pa" => num(f64::from(cold.pa)),
            "status" => {
                let l = cold.status.label();
                (Cell::text(l), Key::Num(cold.status as u8 as f64))
            }
            "wage" => {
                if h.club.is_none() { (Cell::empty(), Key::None) } else { num(cold.contract.current_wage(w.date) as f64) }
            }
            "contract_end" => {
                if h.club.is_none() {
                    (Cell::empty(), Key::None)
                } else {
                    let left = cold.contract.days_left(w.date);
                    let mut cell = Cell::num(f64::from(cold.contract.end.0));
                    if left < 180 {
                        cell = cell.tone(Tone::Warn);
                    }
                    (cell, Key::Num(f64::from(cold.contract.end.0)))
                }
            }
            "value" => num(cold.value as f64),
            "condition" => num(f64::from(h.condition)),
            "sharpness" => num(f64::from(h.sharpness)),
            "morale" => num(f64::from(h.morale)),
            "clause" => {
                if cold.contract.release_clause > 0 { num(cold.contract.release_clause as f64) } else { (Cell::empty(), Key::None) }
            }
            _ => (Cell::empty(), Key::None),
        }
    }
}
