use pw_core::{ClubId, CompId, NationId};
use pw_world::TeamKind;
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref, Tone};
use crate::table::{Key, Source, f_str, f_u32};

pub struct Clubs;

const G: &str = "general";
const FIN: &str = "finance";

impl Source for Clubs {
    type Prep = ();


    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let mut v = vec![
            Col::new("name", "Club", Fmt::Text, 190, &[G, FIN]),
            Col::new("nation", "Nation", Fmt::Text, 120, &[G]).left(),
            Col::new("league", "Competition", Fmt::Text, 160, &[G]).left(),
            Col::new("pos", "Pos", Fmt::Ordinal, 50, &[G]).help("League position"),
            Col::new("pts", "Pts", Fmt::Int, 46, &[G]),
            Col::new("rep", "Reputation", Fmt::Int, 92, &[G, FIN]).help("Standing on a 0 to 10,000 scale"),
            Col::new("manager", "Manager", Fmt::Text, 160, &[G]).left(),
            Col::new("stadium", "Stadium", Fmt::Text, 160, &[G]).left(),
            Col::new("capacity", "Capacity", Fmt::Int, 76, &[G]),
            Col::new("squad", "Squad", Fmt::Int, 56, &[G]).help("First-team squad size"),
            Col::new("founded", "Founded", Fmt::Int, 66, &[G]),
        ];
        if c.observer() {
            v.push(Col::new("balance", "Balance", Fmt::Money, 100, &[FIN]));
            v.push(Col::new("transfer_budget", "Transfer budget", Fmt::Money, 110, &[FIN]));
            v.push(Col::new("wage_bill", "Wage bill / wk", Fmt::Money, 110, &[FIN]));
            v.push(Col::new("wage_budget", "Wage budget / wk", Fmt::Money, 120, &[FIN]));
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("rep", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let nation = f_u32(f, "nation").map(NationId);
        let comp = f_u32(f, "comp").map(CompId);
        let q = f_str(f, "q").map(str::to_lowercase);
        c.w.clubs
            .iter_enumerated()
            .filter(|(_, cl)| nation.is_none_or(|n| cl.nation == n))
            .filter(|(id, _)| {
                comp.is_none_or(|cid| {
                    let ft = c.w.clubs[*id].first_team();
                    c.w.comps[cid].state.entrants.contains(&ft)
                })
            })
            .filter(|(_, cl)| q.as_ref().is_none_or(|q| cl.name.to_lowercase().contains(q.as_str())))
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
        Some(Ref::club(ClubId(id)))
    }

    fn row_tone(&self, c: &Ctx, _p: &(), id: u32) -> Option<Tone> {
        c.same_club(ClubId(id)).then_some(Tone::Info)
    }

    fn note(&self, c: &Ctx, _p: &(), _f: &Value) -> Option<String> {
        (!c.observer()).then(|| "Club finances are private and are not shown.".to_string())
    }
}

impl Clubs {
    fn v(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> (Cell, Key) {
        let w = c.w;
        let club_id = ClubId(id);
        let club = &w.clubs[club_id];
        let num = |n: f64| (Cell::num(n), Key::Num(n));
        let league = w.league_of(club.first_team());
        match col {
            "name" => (Cell::link(Ref::club(club_id), club.name.clone()), Key::text(&club.name)),
            "nation" => {
                let n = c.nation_name(club.nation);
                (Cell::link(Ref::nation(club.nation), n.clone()), Key::text(n))
            }
            "league" => match league {
                Some(l) => {
                    let n = c.comp_short(l);
                    (Cell::link(Ref::comp(l), n.clone()), Key::Num(f64::from(w.comps[l].tier) * 1000.0 + f64::from(l.0)))
                }
                None => (Cell::empty(), Key::None),
            },
            "pos" => match league.and_then(|l| w.comps[l].position_of(club.first_team())) {
                Some(p) => num(p as f64),
                None => (Cell::empty(), Key::None),
            },
            "pts" => match league.and_then(|l| w.comps[l].state.table.iter().find(|r| r.team == club.first_team()).map(|r| r.points)) {
                Some(p) => num(f64::from(p)),
                None => (Cell::empty(), Key::None),
            },
            "rep" => (Cell::num(f64::from(club.reputation)).with_bar(f32::from(club.reputation) / 10_000.0), Key::Num(f64::from(club.reputation))),
            "manager" => {
                if club.manager.is_none() {
                    (Cell::text("Vacant").tone(Tone::Muted), Key::None)
                } else {
                    let person = w.staff[club.manager].person;
                    let n = c.person_name(person);
                    (Cell::link(Ref::person(person), n.clone()), Key::text(n))
                }
            }
            "stadium" => (Cell::text(club.stadium.clone()), Key::text(&club.stadium)),
            "capacity" => num(f64::from(club.capacity)),
            "squad" => {
                let n = w.club_team(club_id, TeamKind::First).map_or(0, |t| w.teams[t].squad.len());
                num(n as f64)
            }
            "founded" => {
                if club.founded == 0 { (Cell::empty(), Key::None) } else { num(f64::from(club.founded)) }
            }
            "balance" => num(club.finance.balance as f64),
            "transfer_budget" => num(club.finance.transfer_budget as f64),
            "wage_bill" => num(club.finance.wage_bill as f64),
            "wage_budget" => num(club.finance.wage_budget as f64),
            _ => (Cell::empty(), Key::None),
        }
    }
}

pub struct Nations;

impl Source for Nations {
    type Prep = ();


    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("name", "Nation", Fmt::Text, 200, &[G]),
            Col::new("code", "Code", Fmt::Text, 60, &[G]).left(),
            Col::new("confed", "Confederation", Fmt::Text, 110, &[G]).left(),
            Col::new("rep", "Reputation", Fmt::Int, 92, &[G]),
            Col::new("clubs", "Clubs", Fmt::Int, 56, &[G]),
            Col::new("leagues", "Divisions", Fmt::Int, 70, &[G]),
            Col::new("season", "Season", Fmt::Text, 80, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("rep", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let q = f_str(f, "q").map(str::to_lowercase);
        c.w.nations
            .iter_enumerated()
            .filter(|(_, n)| q.as_ref().is_none_or(|q| n.name.to_lowercase().contains(q.as_str())))
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
        Some(Ref::nation(NationId(id)))
    }
}

impl Nations {
    fn v(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> (Cell, Key) {
        let w = c.w;
        let n = &w.nations[NationId(id)];
        let num = |x: f64| (Cell::num(x), Key::Num(x));
        match col {
            "name" => (Cell::link(Ref::nation(NationId(id)), n.name.clone()), Key::text(&n.name)),
            "code" => (Cell::text(n.code.clone()), Key::text(&n.code)),
            "confed" => (Cell::text(n.confed.code()), Key::text(n.confed.code())),
            "rep" => (Cell::num(f64::from(n.reputation)).with_bar(f32::from(n.reputation) / 10_000.0), Key::Num(f64::from(n.reputation))),
            "clubs" => num(w.clubs.iter().filter(|cl| cl.nation.0 == id).count() as f64),
            "leagues" => num(n.leagues.len() as f64),
            "season" => (Cell::text(n.season.label()), Key::Num(f64::from(n.season.year))),
            _ => (Cell::empty(), Key::None),
        }
    }
}
