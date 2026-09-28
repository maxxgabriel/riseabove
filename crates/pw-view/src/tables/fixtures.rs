use pw_core::{ClubId, CompId, Date, FixtureId, TeamId};
use pw_world::comp::Format;
use pw_world::{Fixture, Score};
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref, Tone};
use crate::table::{Key, Source, f_bool, f_i32, f_u32};

pub struct Fixtures;

const G: &str = "general";

pub fn score_text(s: &Score) -> String {
    let mut t = format!("{}–{}", s.home, s.away);
    if s.extra_time {
        t.push_str(" aet");
    }
    if let Some((h, a)) = s.pens {
        t.push_str(&format!(" ({h}–{a} p)"));
    }
    t
}

pub fn round_text(c: &Ctx, f: &Fixture) -> String {
    let comp = &c.w.comps[f.comp];
    match comp.format {
        Format::League { .. } => format!("Matchday {}", f.round + 1),
        Format::Knockout { .. } => {
            if f.leg >= 1 && matches!(comp.format, Format::Knockout { legs, .. } if legs > 1) {
                format!("Round {}, leg {}", f.round + 1, f.leg)
            } else {
                format!("Round {}", f.round + 1)
            }
        }
        Format::Groups { .. } => {
            if comp.state.stage == pw_world::comp::Stage::Groups || f.group != u8::MAX && f.tie == u16::MAX {
                format!("Group {}, matchday {}", (b'A' + f.group.min(25)) as char, f.round + 1)
            } else {
                format!("Knockout round {}", f.round + 1)
            }
        }
    }
}

impl Source for Fixtures {
    type Prep = ();

    fn id(&self) -> &'static str {
        "fixtures"
    }

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("comp", "Competition", Fmt::Text, 150, &[G]).left(),
            Col::new("round", "Round", Fmt::Text, 130, &[G]).left(),
            Col::new("home", "Home", Fmt::Text, 180, &[G]).left(),
            Col::new("score", "Result", Fmt::Text, 96, &[G]).center().nosort(),
            Col::new("away", "Away", Fmt::Text, 180, &[G]).left(),
            Col::new("venue", "Venue", Fmt::Text, 150, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", false)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let w = c.w;
        let comp = f_u32(f, "comp").map(CompId);
        let club = f_u32(f, "club").map(ClubId);
        let team = f_u32(f, "team").map(TeamId);
        let from = f_i32(f, "from").map(Date);
        let to = f_i32(f, "to").map(Date);
        let played = f_bool(f, "played");
        let mine = f_bool(f, "mine").unwrap_or(false);
        let my_team = c.my_team();
        // A date window keeps this cheap on worlds with many fixtures.
        let lo = from.unwrap_or(Date(i32::MIN / 2));
        let hi = to.unwrap_or(Date(i32::MAX / 2));
        let iter: Box<dyn Iterator<Item = FixtureId>> = if from.is_some() || to.is_some() {
            Box::new(w.fixtures.between(lo, hi))
        } else {
            Box::new(w.fixtures.iter().map(|(id, _)| id))
        };
        iter.filter(|&id| {
            let fx = w.fixtures.get(id);
            if comp.is_some_and(|x| fx.comp != x) {
                return false;
            }
            if let Some(cl) = club {
                if w.teams[fx.home].club != cl && w.teams[fx.away].club != cl {
                    return false;
                }
            }
            if team.is_some_and(|t| !fx.involves(t)) {
                return false;
            }
            if mine && (my_team.is_none() || !fx.involves(my_team)) {
                return false;
            }
            match played {
                Some(true) if fx.score.is_none() => return false,
                Some(false) if fx.score.is_some() => return false,
                _ => {}
            }
            true
        })
        .map(|id| id.0)
        .collect()
    }

    fn key(&self, c: &Ctx, p: &(), id: u32, col: &str) -> Key {
        let fx = c.w.fixtures.get(FixtureId(id));
        match col {
            "date" => Key::Num(f64::from(fx.date.0) * 1000.0 + fx.uid as f64 % 1000.0),
            "comp" => Key::text(c.comp_short(fx.comp)),
            "round" => Key::Num(f64::from(fx.round) * 10.0 + f64::from(fx.leg)),
            "home" => Key::text(c.team_short(fx.home)),
            "away" => Key::text(c.team_short(fx.away)),
            "venue" => Key::text(&c.w.clubs[c.w.teams[fx.home].club].stadium),
            _ => {
                let _ = p;
                Key::None
            }
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let fx = c.w.fixtures.get(FixtureId(id));
        match col {
            "date" => Cell::num(f64::from(fx.date.0)),
            "comp" => Cell::link(Ref::comp(fx.comp), c.comp_short(fx.comp)),
            "round" => Cell::text(round_text(c, fx)),
            "home" => team_cell(c, fx, fx.home),
            "away" => team_cell(c, fx, fx.away),
            "score" => match fx.score {
                None => {
                    if fx.date < c.w.date { Cell::text("Not played").tone(Tone::Muted) } else { Cell::text("v").tone(Tone::Muted) }
                }
                Some(_) if c.is_concealed(fx.uid) => Cell::text("? – ?").tone(Tone::Muted).with_ref(Ref::fixture(fx.uid)).with_sub("Not revealed"),
                Some(s) => {
                    let mut cell = Cell::text(score_text(&s)).with_ref(Ref::fixture(fx.uid));
                    if let Some(t) = my_tone(c, fx, &s) {
                        cell = cell.tone(t);
                    }
                    cell
                }
            },
            "venue" => {
                let club = c.w.teams[fx.home].club;
                if fx.neutral { Cell::text("Neutral venue").tone(Tone::Muted) } else { Cell::text(c.w.clubs[club].stadium.clone()) }
            }
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(Ref::fixture(c.w.fixtures.get(FixtureId(id)).uid))
    }

    fn row_tone(&self, c: &Ctx, _p: &(), id: u32) -> Option<Tone> {
        let fx = c.w.fixtures.get(FixtureId(id));
        (c.my_team().is_some() && fx.involves(c.my_team())).then_some(Tone::Info)
    }
}

fn team_cell(c: &Ctx, fx: &Fixture, t: TeamId) -> Cell {
    let mut cell = Cell::link(c.team_ref(t), c.team_short(t));
    if c.my_team() == t {
        cell = cell.with_sub("Your team");
    }
    if let Some(s) = fx.score {
        if !c.is_concealed(fx.uid) && s.home_won() == Some(fx.home == t) && !(s.home == s.away && s.pens.is_none()) {
            cell = cell.tone(Tone::Pos);
        }
    }
    cell
}

fn my_tone(c: &Ctx, fx: &Fixture, s: &Score) -> Option<Tone> {
    let my = c.my_team();
    if my.is_none() || !fx.involves(my) {
        return None;
    }
    let mine_home = fx.home == my;
    match s.home_won() {
        Some(h) if h == mine_home => Some(Tone::Pos),
        Some(_) => Some(Tone::Neg),
        None => Some(Tone::Warn),
    }
}
