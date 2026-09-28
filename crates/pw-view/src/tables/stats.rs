use pw_core::{CompId, PlayerId};
use pw_world::stats::StatLine;
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref};
use crate::table::{Key, Source, f_u32};

const G: &str = "general";

/// One player's season-by-season record (current season first, then history).
pub struct PlayerSeasons;

fn player_of(c: &Ctx, f: &Value) -> Option<PlayerId> {
    let person = pw_core::PersonId(f_u32(f, "person")?);
    c.w.people.get(person)?.player.get()
}

impl Source for PlayerSeasons {
    type Prep = Vec<StatLine>;


    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("season", "Season", Fmt::Text, 80, &[G]).left(),
            Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
            Col::new("comp", "Competition", Fmt::Text, 160, &[G]).left(),
            Col::new("apps", "Apps", Fmt::Int, 52, &[G]),
            Col::new("starts", "Starts", Fmt::Int, 56, &[G]),
            Col::new("mins", "Minutes", Fmt::Int, 66, &[G]),
            Col::new("goals", "Goals", Fmt::Int, 56, &[G]),
            Col::new("assists", "Assists", Fmt::Int, 60, &[G]),
            Col::new("xg", "xG", Fmt::Dec1, 52, &[G]),
            Col::new("rating", "Rating", Fmt::Dec2, 62, &[G]),
            Col::new("cs", "Clean sheets", Fmt::Int, 70, &[G]),
            Col::new("yellows", "Yellow", Fmt::Int, 56, &[G]),
            Col::new("reds", "Red", Fmt::Int, 46, &[G]),
            Col::new("pom", "MotM", Fmt::Int, 52, &[G]).help("Man of the match awards"),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("season", true)
    }

    fn prep(&self, c: &Ctx, f: &Value) -> Vec<StatLine> {
        let Some(p) = player_of(c, f) else { return vec![] };
        let mut v: Vec<StatLine> = c.w.stats.for_player(p).copied().collect();
        v.extend(c.w.history.career(p).copied());
        v
    }

    fn ids(&self, _c: &Ctx, p: &Vec<StatLine>, _f: &Value) -> Vec<u32> {
        (0..p.len() as u32).collect()
    }

    fn key(&self, c: &Ctx, p: &Vec<StatLine>, id: u32, col: &str) -> Key {
        let l = &p[id as usize];
        match col {
            "season" => Key::Num(f64::from(l.season) * 10.0 - f64::from(c.w.comps.get(l.comp).map_or(0, |x| x.tier))),
            "club" => Key::text(c.club_short(l.club)),
            "comp" => Key::text(c.comp_short(l.comp)),
            "apps" => Key::Num(f64::from(l.apps)),
            "starts" => Key::Num(f64::from(l.starts)),
            "mins" => Key::Num(f64::from(l.minutes)),
            "goals" => Key::Num(f64::from(l.goals)),
            "assists" => Key::Num(f64::from(l.assists)),
            "xg" => Key::Num(f64::from(l.xg)),
            "rating" => if l.apps > 0 { Key::Num(f64::from(l.avg_rating())) } else { Key::None },
            "cs" => Key::Num(f64::from(l.clean_sheets)),
            "yellows" => Key::Num(f64::from(l.yellows)),
            "reds" => Key::Num(f64::from(l.reds)),
            "pom" => Key::Num(f64::from(l.pom)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, p: &Vec<StatLine>, id: u32, col: &str) -> Cell {
        let l = &p[id as usize];
        match col {
            "season" => {
                let cur = c.w.comps.get(l.comp).is_some_and(|x| x.state.season == l.season);
                let mut cell = Cell::text(c.season_label(l.comp, l.season));
                if cur {
                    cell = cell.with_sub("Current");
                }
                cell
            }
            "club" => Cell::link(Ref::club(l.club), c.club_short(l.club)),
            "comp" => Cell::link(Ref::comp(l.comp), c.comp_short(l.comp)),
            "apps" => Cell::num(f64::from(l.apps)),
            "starts" => Cell::num(f64::from(l.starts)),
            "mins" => Cell::num(f64::from(l.minutes)),
            "goals" => Cell::num(f64::from(l.goals)),
            "assists" => Cell::num(f64::from(l.assists)),
            "xg" => Cell::num(f64::from(l.xg)),
            "rating" => if l.apps > 0 { Cell::num(f64::from(l.avg_rating())) } else { Cell::empty() },
            "cs" => Cell::num(f64::from(l.clean_sheets)),
            "yellows" => Cell::num(f64::from(l.yellows)),
            "reds" => Cell::num(f64::from(l.reds)),
            "pom" => Cell::num(f64::from(l.pom)),
            _ => Cell::empty(),
        }
    }
}

/// Leaders of one competition this season.
pub struct CompLeaders;

impl Source for CompLeaders {
    type Prep = Vec<StatLine>;


    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("player", "Player", Fmt::Text, 190, &[G]),
            Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
            Col::new("apps", "Apps", Fmt::Int, 52, &[G]),
            Col::new("mins", "Minutes", Fmt::Int, 66, &[G]),
            Col::new("goals", "Goals", Fmt::Int, 56, &[G]),
            Col::new("assists", "Assists", Fmt::Int, 60, &[G]),
            Col::new("xg", "xG", Fmt::Dec1, 52, &[G]),
            Col::new("rating", "Rating", Fmt::Dec2, 62, &[G]).help("Average match rating; players with fewer than three appearances are not ranked"),
            Col::new("cs", "Clean sheets", Fmt::Int, 70, &[G]),
            Col::new("yellows", "Yellow", Fmt::Int, 56, &[G]),
            Col::new("reds", "Red", Fmt::Int, 46, &[G]),
            Col::new("pom", "MotM", Fmt::Int, 52, &[G]),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("goals", true)
    }

    fn prep(&self, c: &Ctx, f: &Value) -> Vec<StatLine> {
        let comp = CompId(f_u32(f, "comp").unwrap_or(u32::MAX));
        if comp.is_none() {
            return vec![];
        }
        let min_apps = f_u32(f, "min_apps").unwrap_or(1) as u16;
        let mut v: Vec<StatLine> = c.w.stats.for_comp(comp).copied().filter(|l| l.apps >= min_apps).collect();
        v.sort_by_key(|l| (l.player, l.club));
        v
    }

    fn ids(&self, _c: &Ctx, p: &Vec<StatLine>, _f: &Value) -> Vec<u32> {
        (0..p.len() as u32).collect()
    }

    fn key(&self, c: &Ctx, p: &Vec<StatLine>, id: u32, col: &str) -> Key {
        let l = &p[id as usize];
        match col {
            "player" => Key::text(c.player_short(l.player)),
            "club" => Key::text(c.club_short(l.club)),
            "apps" => Key::Num(f64::from(l.apps)),
            "mins" => Key::Num(f64::from(l.minutes)),
            "goals" => Key::Num(f64::from(l.goals) * 1000.0 + f64::from(l.assists)),
            "assists" => Key::Num(f64::from(l.assists) * 1000.0 + f64::from(l.goals)),
            "xg" => Key::Num(f64::from(l.xg)),
            "rating" => if l.apps >= 3 { Key::Num(f64::from(l.avg_rating())) } else { Key::None },
            "cs" => Key::Num(f64::from(l.clean_sheets)),
            "yellows" => Key::Num(f64::from(l.yellows)),
            "reds" => Key::Num(f64::from(l.reds)),
            "pom" => Key::Num(f64::from(l.pom)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, p: &Vec<StatLine>, id: u32, col: &str) -> Cell {
        let l = &p[id as usize];
        match col {
            "player" => Cell::link(c.player_ref(l.player), c.player_name(l.player)),
            "club" => Cell::link(Ref::club(l.club), c.club_short(l.club)),
            "apps" => Cell::num(f64::from(l.apps)),
            "mins" => Cell::num(f64::from(l.minutes)),
            "goals" => Cell::num(f64::from(l.goals)),
            "assists" => Cell::num(f64::from(l.assists)),
            "xg" => Cell::num(f64::from(l.xg)),
            "rating" => if l.apps >= 3 { Cell::num(f64::from(l.avg_rating())) } else { Cell::empty() },
            "cs" => Cell::num(f64::from(l.clean_sheets)),
            "yellows" => Cell::num(f64::from(l.yellows)),
            "reds" => Cell::num(f64::from(l.reds)),
            "pom" => Cell::num(f64::from(l.pom)),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, p: &Vec<StatLine>, id: u32) -> Option<Ref> {
        Some(c.player_ref(p[id as usize].player))
    }
}
