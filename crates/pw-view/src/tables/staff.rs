use pw_core::{ClubId, StaffId};
use pw_world::StaffRole;
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref};
use crate::table::{Key, Source, f_str, f_u32};

pub struct StaffTable;

const G: &str = "general";
const CON: &str = "contracts";

fn role_from(s: &str) -> Option<StaffRole> {
    use StaffRole::*;
    Some(match s {
        "manager" => Manager,
        "assistant" => Assistant,
        "coach" => Coach,
        "gk_coach" => GkCoach,
        "fitness" => FitnessCoach,
        "scout" => Scout,
        "physio" => Physio,
        "science" => SportsScientist,
        "youth" => HeadOfYouth,
        "director" => DirectorOfFootball,
        _ => return None,
    })
}

impl Source for StaffTable {
    type Prep = ();

    fn id(&self) -> &'static str {
        "staff"
    }

    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let mut v = vec![
            Col::new("name", "Name", Fmt::Text, 190, &[G, CON]),
            Col::new("role", "Role", Fmt::Text, 170, &[G, CON]).left(),
            Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
            Col::new("age", "Age", Fmt::Int, 46, &[G, CON]),
            Col::new("nation", "Nationality", Fmt::Text, 120, &[G]).left(),
            Col::new("rep", "Reputation", Fmt::Int, 86, &[G]).help("Standing in the game on a 0 to 10,000 scale"),
            Col::new("games", "Games", Fmt::Int, 56, &[G]).help("Games in charge (managers)"),
            Col::new("win", "Win %", Fmt::Pct, 60, &[G]),
        ];
        if c.observer() {
            v.push(Col::new("wage", "Wage / wk", Fmt::Money, 90, &[CON]));
            v.push(Col::new("contract_end", "Contract ends", Fmt::Date, 100, &[CON]));
            v.push(Col::new("rating", "Role rating", Fmt::Dec1, 80, &[CON]).help("Average of the role's key attributes (observer only)"));
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("rep", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let club = f_u32(f, "club").map(ClubId);
        let role = f_str(f, "role").and_then(role_from);
        let q = f_str(f, "q").map(str::to_lowercase);
        let unemployed = f.get("unemployed").and_then(Value::as_bool).unwrap_or(false);
        c.w.staff
            .iter_enumerated()
            .filter(|(_, s)| !s.retired)
            .filter(|(_, s)| club.is_none_or(|cl| s.club == cl))
            .filter(|(_, s)| role.is_none_or(|r| s.role == r))
            .filter(|(_, s)| !unemployed || s.club.is_none())
            .filter(|(_, s)| {
                q.as_ref().is_none_or(|q| c.person_name(s.person).to_lowercase().contains(q.as_str()))
            })
            .map(|(id, _)| id.0)
            .collect()
    }

    fn key(&self, c: &Ctx, p: &(), id: u32, col: &str) -> Key {
        let (cell, k) = self.v(c, p, id, col);
        let _ = cell;
        k
    }

    fn cell(&self, c: &Ctx, p: &(), id: u32, col: &str) -> Cell {
        self.v(c, p, id, col).0
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(Ref::person(c.w.staff[StaffId(id)].person))
    }
}

impl StaffTable {
    fn v(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> (Cell, Key) {
        let w = c.w;
        let s = &w.staff[StaffId(id)];
        let person = &w.people[s.person];
        let num = |n: f64| (Cell::num(n), Key::Num(n));
        match col {
            "name" => {
                let n = c.person_name(s.person);
                (Cell::link(Ref::person(s.person), n.clone()), Key::text(n))
            }
            "role" => (Cell::text(s.role.label()), Key::Num(s.role as u8 as f64)),
            "club" => {
                if s.club.is_none() {
                    (Cell::text("Unemployed").tone(crate::model::Tone::Muted), Key::None)
                } else {
                    let n = c.club_short(s.club);
                    (Cell::link(Ref::club(s.club), n.clone()), Key::text(n))
                }
            }
            "age" => num(f64::from(person.age(w.date))),
            "nation" => {
                let n = c.nation_name(person.nation);
                (Cell::link(Ref::nation(person.nation), n.clone()), Key::text(n))
            }
            "rep" => num(f64::from(s.reputation)),
            "games" => {
                if s.record.games == 0 { (Cell::empty(), Key::None) } else { num(f64::from(s.record.games)) }
            }
            "win" => {
                if s.record.games == 0 {
                    (Cell::empty(), Key::None)
                } else {
                    num(f64::from(s.record.wins) / f64::from(s.record.games))
                }
            }
            "wage" => num(s.wage as f64),
            "contract_end" => num(f64::from(s.contract_end.0)),
            "rating" => num(f64::from(s.role_rating(s.role))),
            _ => (Cell::empty(), Key::None),
        }
    }
}
