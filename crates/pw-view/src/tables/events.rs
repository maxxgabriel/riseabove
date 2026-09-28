use pw_core::{ClubId, CompId, Date, PersonId};
use pw_world::event::{AwardKind, Event, EventKind as E};
use pw_world::history::Spell;
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref, Tone};
use crate::narrative::{self, Group};
use crate::table::{Key, Source, f_i32, f_str, f_u32};

const G: &str = "general";

fn person_filter(c: &Ctx, f: &Value) -> Option<pw_core::PlayerId> {
    let person = PersonId(f_u32(f, "person")?);
    c.w.people.get(person)?.player.get()
}

fn event_ids(c: &Ctx, f: &Value, only: impl Fn(&E) -> bool) -> Vec<u32> {
    let group = f_str(f, "group").and_then(Group::parse);
    let club = f_u32(f, "club").map(ClubId);
    let comp = f_u32(f, "comp").map(CompId);
    let player = person_filter(c, f);
    let from = f_i32(f, "from").map(Date);
    let to = f_i32(f, "to").map(Date);
    let all = c.w.events.all();
    let start = from.map_or(0, |d| all.partition_point(|e| e.date < d));
    let mut out = Vec::new();
    for (i, e) in all.iter().enumerate().skip(start) {
        if to.is_some_and(|d| e.date > d) {
            break;
        }
        if !only(&e.kind) || group.is_some_and(|g| narrative::group_of(&e.kind) != g) {
            continue;
        }
        if let Some(cl) = club {
            if !narrative::clubs_of(&e.kind).contains(&cl) {
                continue;
            }
        }
        if comp.is_some_and(|x| narrative::comp_of(&e.kind) != Some(x)) {
            continue;
        }
        if let Some(p) = player {
            if e.kind.player() != Some(p) {
                continue;
            }
        }
        if !narrative::visible(c, e) {
            continue;
        }
        out.push(i as u32);
    }
    out
}

pub struct Events;

impl Source for Events {
    type Prep = ();


    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("kind", "Kind", Fmt::Text, 110, &[G]).left(),
            Col::new("what", "What happened", Fmt::Text, 640, &[G]).left().nosort(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        event_ids(c, f, |_| true)
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let e = &c.w.events.all()[id as usize];
        match col {
            "date" => Key::Num(f64::from(e.date.0) * 1_000_000.0 + f64::from(id)),
            "kind" => Key::text(narrative::label(&e.kind)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let e: &Event = &c.w.events.all()[id as usize];
        match col {
            "date" => Cell::num(f64::from(e.date.0)),
            "kind" => Cell::text(narrative::label(&e.kind)),
            "what" => Cell::empty().with_parts(narrative::describe(c, &e.kind)),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        narrative::primary(c, &c.w.events.all()[id as usize].kind)
    }
}

pub struct Transfers;

impl Source for Transfers {
    type Prep = ();


    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("player", "Player", Fmt::Text, 190, &[G]),
            Col::new("from", "From", Fmt::Text, 160, &[G]).left(),
            Col::new("to", "To", Fmt::Text, 160, &[G]).left(),
            Col::new("type", "Type", Fmt::Text, 110, &[G]).left(),
            Col::new("fee", "Fee", Fmt::Money, 100, &[G]),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let kind = f_str(f, "type").map(str::to_owned);
        event_ids(c, f, move |k| match (&kind.as_deref(), k) {
            (Some("transfer"), E::Transfer { fee, .. }) => *fee > 0 || true,
            (Some("loan"), E::LoanMove { .. } | E::LoanReturn { .. }) => true,
            (Some("free"), E::Transfer { fee: 0, .. }) => true,
            (Some("release"), E::Released { .. }) => true,
            (Some("renewal"), E::ContractSigned { renewal: true, .. }) => true,
            (None | Some("all"), E::Transfer { .. } | E::LoanMove { .. } | E::LoanReturn { .. } | E::Released { .. } | E::ContractSigned { renewal: false, .. }) => true,
            _ => false,
        })
    }

    fn key(&self, c: &Ctx, p: &(), id: u32, col: &str) -> Key {
        let e = &c.w.events.all()[id as usize];
        match col {
            "date" => Key::Num(f64::from(e.date.0) * 1_000_000.0 + f64::from(id)),
            "player" => e.kind.player().map_or(Key::None, |p| Key::text(c.player_short(p))),
            "type" => Key::text(narrative::label(&e.kind)),
            "fee" => match e.kind {
                E::Transfer { fee, .. } => Key::Num(fee as f64),
                _ => Key::None,
            },
            "from" | "to" => {
                let (a, b) = ends(&e.kind);
                let x = if col == "from" { a } else { b };
                if x.is_some() { Key::text(c.club_short(x)) } else { Key::None }
            }
            _ => {
                let _ = p;
                Key::None
            }
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let e = &c.w.events.all()[id as usize];
        match col {
            "date" => Cell::num(f64::from(e.date.0)),
            "player" => e.kind.player().map_or(Cell::empty(), |p| Cell::link(c.player_ref(p), c.player_name(p))),
            "from" | "to" => {
                let (a, b) = ends(&e.kind);
                let x = if col == "from" { a } else { b };
                if x.is_some() { Cell::link(Ref::club(x), c.club_short(x)) } else { Cell::text("Free agent").tone(Tone::Muted) }
            }
            "type" => {
                let mut cell = Cell::text(narrative::label(&e.kind));
                if let E::LoanMove { until, .. } = e.kind {
                    cell = cell.with_num(f64::from(until.0)).with_sub("Until".to_string());
                }
                cell
            }
            "fee" => match e.kind {
                E::Transfer { fee, .. } if fee > 0 => Cell::num(fee as f64),
                E::Transfer { .. } => Cell::text("Free"),
                E::LoanMove { .. } => Cell::text("Loan").tone(Tone::Muted),
                _ => Cell::empty(),
            },
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        narrative::primary(c, &c.w.events.all()[id as usize].kind)
    }
}

fn ends(k: &E) -> (ClubId, ClubId) {
    match *k {
        E::Transfer { from, to, .. } | E::LoanMove { from, to, .. } => (from, to),
        E::LoanReturn { to, .. } => (ClubId::NONE, to),
        E::ContractSigned { club, .. } => (ClubId::NONE, club),
        E::Released { club, .. } => (club, ClubId::NONE),
        _ => (ClubId::NONE, ClubId::NONE),
    }
}

// ---- honours ---------------------------------------------------------------------------

pub struct Honours;

impl Source for Honours {
    type Prep = ();


    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("season", "Season", Fmt::Text, 80, &[G]).left(),
            Col::new("comp", "Competition", Fmt::Text, 200, &[G]).left(),
            Col::new("winner", "Winner", Fmt::Text, 190, &[G]).left(),
            Col::new("runner_up", "Runner-up", Fmt::Text, 190, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("season", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let club = f_u32(f, "club").map(ClubId);
        let comp = f_u32(f, "comp").map(CompId);
        c.w.history
            .honours
            .iter()
            .enumerate()
            .filter(|(_, h)| club.is_none_or(|x| h.club == x))
            .filter(|(_, h)| comp.is_none_or(|x| h.comp == x))
            .map(|(i, _)| i as u32)
            .collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let h = &c.w.history.honours[id as usize];
        match col {
            "season" => Key::Num(f64::from(h.season) * 1000.0 + f64::from(id % 1000)),
            "comp" => Key::text(c.comp_short(h.comp)),
            "winner" => Key::text(c.team_short(h.team)),
            "runner_up" => if h.runner_up.is_some() { Key::text(c.team_short(h.runner_up)) } else { Key::None },
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let h = &c.w.history.honours[id as usize];
        match col {
            "season" => Cell::text(c.season_label(h.comp, h.season)),
            "comp" => Cell::link(Ref::comp(h.comp), c.comp_name(h.comp)),
            "winner" => Cell::link(c.team_ref(h.team), c.team_name(h.team)),
            "runner_up" => if h.runner_up.is_some() { Cell::link(c.team_ref(h.runner_up), c.team_name(h.runner_up)) } else { Cell::empty() },
            _ => Cell::empty(),
        }
    }
}

pub struct Awards;

impl Source for Awards {
    type Prep = ();


    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("season", "Season", Fmt::Text, 80, &[G]).left(),
            Col::new("comp", "Competition", Fmt::Text, 180, &[G]).left(),
            Col::new("award", "Award", Fmt::Text, 190, &[G]).left(),
            Col::new("player", "Winner", Fmt::Text, 190, &[G]),
            Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
            Col::new("value", "Figure", Fmt::Dec1, 66, &[G]).help("Goals for the top scorer; average rating otherwise"),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("season", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let comp = f_u32(f, "comp").map(CompId);
        let player = person_filter(c, f);
        c.w.history
            .awards
            .iter()
            .enumerate()
            .filter(|(_, a)| comp.is_none_or(|x| a.comp == x))
            .filter(|(_, a)| player.is_none_or(|x| a.player == x))
            .map(|(i, _)| i as u32)
            .collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let a = &c.w.history.awards[id as usize];
        match col {
            "season" => Key::Num(f64::from(a.season) * 1000.0 + f64::from(id % 1000)),
            "comp" => Key::text(c.comp_short(a.comp)),
            "award" => Key::text(award_text(a.kind)),
            "player" => Key::text(c.player_short(a.player)),
            "club" => Key::text(c.club_short(a.club)),
            "value" => Key::Num(f64::from(a.value)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let a = &c.w.history.awards[id as usize];
        match col {
            "season" => Cell::text(c.season_label(a.comp, a.season)),
            "comp" => Cell::link(Ref::comp(a.comp), c.comp_short(a.comp)),
            "award" => Cell::text(award_text(a.kind)),
            "player" => Cell::link(c.player_ref(a.player), c.player_name(a.player)),
            "club" => Cell::link(Ref::club(a.club), c.club_short(a.club)),
            "value" => Cell::num(f64::from(a.value)),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(c.player_ref(c.w.history.awards[id as usize].player))
    }
}

pub fn award_text(k: AwardKind) -> &'static str {
    match k {
        AwardKind::PlayerOfSeason => "Player of the Season",
        AwardKind::YoungPlayerOfSeason => "Young Player of the Season",
        AwardKind::TopScorer => "Top scorer",
        AwardKind::TeamOfSeason => "Team of the Season",
        AwardKind::PlayerOfMonth => "Player of the Month",
    }
}

// ---- employment history ------------------------------------------------------------------

pub struct Spells;

impl Source for Spells {
    type Prep = Vec<Spell>;


    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let mut v = vec![
            Col::new("club", "Club", Fmt::Text, 200, &[G]).left(),
            Col::new("from", "From", Fmt::Date, 100, &[G]),
            Col::new("to", "To", Fmt::Date, 100, &[G]),
            Col::new("kind", "Arrangement", Fmt::Text, 120, &[G]).left(),
        ];
        if c.observer() {
            v.push(Col::new("fee", "Fee", Fmt::Money, 100, &[G]));
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("from", true)
    }

    fn prep(&self, c: &Ctx, f: &Value) -> Vec<Spell> {
        let Some(p) = person_filter(c, f) else { return vec![] };
        let mut v = c.w.history.spells.get(&p).cloned().unwrap_or_default();
        let h = &c.w.players.hot[p];
        if v.is_empty() && h.club.is_some() {
            let cold = &c.w.players.cold[p];
            v.push(Spell { club: h.club, from: cold.joined, to: None, loan: cold.loan.is_some(), fee: 0 });
        }
        v
    }

    fn ids(&self, _c: &Ctx, p: &Vec<Spell>, _f: &Value) -> Vec<u32> {
        (0..p.len() as u32).collect()
    }

    fn key(&self, c: &Ctx, p: &Vec<Spell>, id: u32, col: &str) -> Key {
        let s = &p[id as usize];
        match col {
            "club" => Key::text(c.club_short(s.club)),
            "from" => Key::Num(f64::from(s.from.0)),
            "to" => Key::Num(s.to.map_or(f64::from(i32::MAX), |d| f64::from(d.0))),
            "kind" => Key::text(if s.loan { "Loan" } else { "Permanent" }),
            "fee" => Key::Num(s.fee as f64),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, p: &Vec<Spell>, id: u32, col: &str) -> Cell {
        let s = &p[id as usize];
        match col {
            "club" => Cell::link(Ref::club(s.club), c.club_name(s.club)),
            "from" => Cell::num(f64::from(s.from.0)),
            "to" => s.to.map_or(Cell::text("Present").tone(Tone::Info), |d| Cell::num(f64::from(d.0))),
            "kind" => Cell::text(if s.loan { "Loan" } else { "Permanent" }),
            "fee" => if s.fee > 0 { Cell::num(s.fee as f64) } else { Cell::empty() },
            _ => Cell::empty(),
        }
    }
}
