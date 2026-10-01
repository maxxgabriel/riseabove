//! Tables for the wider world that grew around the game: what supporters say and sing, incidents,
//! press conferences, referees and their big calls, records and honours, tactical schools, rule
//! changes, and football below the professional game. Every sentence is rendered by narration from
//! recorded state; columns that would give away what the world hides are for observers only.

use std::fmt::Debug;

use pw_core::{ClubId, PersonId};
use pw_world::culture::Side;
use pw_world::officials::AppealOutcome;
use pw_world::records::Holder;
use pw_world::socialnet::Post;
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref, Tone};
use crate::table::{Key, Source, f_str, f_u32};

const G: &str = "general";
const INT: &str = "internal";

/// "SeasonTicket" → "Season ticket".
fn words<T: Debug>(v: &T) -> String {
    let s = format!("{v:?}");
    let s = s.split(['{', '(', ' ']).next().unwrap_or("").to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push(' ');
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn club_cell(c: &Ctx, x: ClubId) -> Cell {
    if x.is_some() { Cell::link(Ref::club(x), c.club_short(x)) } else { Cell::empty() }
}

fn person_cell(c: &Ctx, p: PersonId) -> Cell {
    if p.is_some() { Cell::link(Ref::person(p), c.person_name(p)) } else { Cell::empty() }
}

fn date_key(d: pw_core::Date, id: u32) -> Key {
    Key::Num(f64::from(d.0) * 1_000_000.0 + f64::from(id))
}

// ---- a small generic table ------------------------------------------------------------------------------------

/// A row built up front, for lists small enough to assemble on every request.
pub struct Row {
    cells: Vec<(&'static str, Cell, Key)>,
    open: Option<Ref>,
    tone: Option<Tone>,
}

impl Row {
    fn new() -> Self {
        Self { cells: Vec::new(), open: None, tone: None }
    }
    fn text(mut self, k: &'static str, s: impl Into<String>) -> Self {
        let s = s.into();
        self.cells.push((k, Cell::text(s.clone()), Key::text(&s)));
        self
    }
    fn num(mut self, k: &'static str, n: f64) -> Self {
        self.cells.push((k, Cell::num(n), Key::Num(n)));
        self
    }
    fn date(mut self, k: &'static str, d: pw_core::Date) -> Self {
        self.cells.push((k, Cell::num(f64::from(d.0)), Key::Num(f64::from(d.0))));
        self
    }
    fn cell(mut self, k: &'static str, cell: Cell, key: Key) -> Self {
        self.cells.push((k, cell, key));
        self
    }
    fn open(mut self, r: Ref) -> Self {
        self.open = Some(r);
        self
    }
}

pub struct Grid {
    cols: fn(&Ctx) -> Vec<Col>,
    sort: (&'static str, bool),
    build: fn(&Ctx, &Value) -> Vec<Row>,
}

impl Source for Grid {
    type Prep = Vec<Row>;

    fn cols(&self, c: &Ctx) -> Vec<Col> {
        (self.cols)(c)
    }

    fn default_sort(&self) -> (&'static str, bool) {
        self.sort
    }

    fn prep(&self, c: &Ctx, f: &Value) -> Vec<Row> {
        let mut rows = (self.build)(c, f);
        if let Some(q) = f_str(f, "q").map(str::to_lowercase) {
            rows.retain(|r| r.cells.iter().any(|(_, cell, _)| cell.s.as_ref().is_some_and(|s| s.to_lowercase().contains(&q))));
        }
        rows
    }

    fn ids(&self, _c: &Ctx, p: &Vec<Row>, _f: &Value) -> Vec<u32> {
        (0..p.len() as u32).collect()
    }

    fn key(&self, _c: &Ctx, p: &Vec<Row>, id: u32, col: &str) -> Key {
        p[id as usize].cells.iter().find(|(k, _, _)| *k == col).map_or(Key::None, |(_, _, key)| key.clone())
    }

    fn cell(&self, _c: &Ctx, p: &Vec<Row>, id: u32, col: &str) -> Cell {
        p[id as usize].cells.iter().find(|(k, _, _)| *k == col).map_or_else(Cell::empty, |(_, cell, _)| cell.clone())
    }

    fn open(&self, _c: &Ctx, p: &Vec<Row>, id: u32) -> Option<Ref> {
        p[id as usize].open
    }

    fn row_tone(&self, _c: &Ctx, p: &Vec<Row>, id: u32) -> Option<Tone> {
        p[id as usize].tone
    }
}

fn side_name(c: &Ctx, s: Side) -> (String, Option<Ref>) {
    match s {
        Side::Club(x) => (c.club_name(x), Some(Ref::club(x))),
        Side::Nation(n) => (c.nation_name(n), Some(Ref::nation(n))),
        Side::Institution(i) => (pw_narrate::history::institution(c.w, i), Some(Ref::inst(i))),
    }
}

fn holder_ref(h: Holder) -> Option<Ref> {
    match h {
        Holder::Person(p) => Some(Ref::person(p)),
        Holder::Club(x) => Some(Ref::club(x)),
        Holder::Nation(n) => Some(Ref::nation(n)),
        _ => None,
    }
}

// ---- supporters ------------------------------------------------------------------------------------------------

pub fn chants() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
                Col::new("text", "Chant", Fmt::Text, 380, &[G]).left().nosort(),
                Col::new("kind", "Kind", Fmt::Text, 120, &[G]).left(),
                Col::new("popularity", "Popularity", Fmt::Int, 90, &[G]).help("How widely it is sung at the club's matches, 0 to 100."),
                Col::new("born", "First sung", Fmt::Date, 100, &[G]),
                Col::new("last", "Last sung", Fmt::Date, 100, &[G]),
            ]
        },
        sort: ("popularity", true),
        build: |c, f| {
            let club = f_u32(f, "club").map(ClubId);
            c.w.net
                .chants
                .iter()
                .filter(|ch| club.is_none_or(|x| ch.club == x))
                .map(|ch| {
                    let mut r = Row::new()
                        .cell("club", club_cell(c, ch.club), Key::text(c.club_short(ch.club)))
                        .text("text", pw_narrate::social::chant(c.w, ch))
                        .text("kind", words(&ch.kind))
                        .num("popularity", f64::from(ch.popularity))
                        .date("born", ch.born)
                        .date("last", ch.last_sung);
                    if ch.club.is_some() {
                        r = r.open(Ref::club(ch.club));
                    }
                    r
                })
                .collect()
        },
    }
}

pub fn memes() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("text", "Meme", Fmt::Text, 420, &[G]).left().nosort(),
                Col::new("club", "Club", Fmt::Text, 150, &[G]).left(),
                Col::new("recognition", "Recognised", Fmt::Int, 90, &[G]).help("How many people know it, 0 to 100."),
                Col::new("uses", "Uses", Fmt::Int, 80, &[G]),
                Col::new("born", "Born", Fmt::Date, 100, &[G]),
                Col::new("alive", "Still used", Fmt::Text, 90, &[G]),
            ]
        },
        sort: ("uses", true),
        build: |c, f| {
            let club = f_u32(f, "club").map(ClubId);
            c.w.net
                .memes
                .iter()
                .filter(|m| club.is_none_or(|x| m.club == x))
                .map(|m| {
                    let mut r = Row::new()
                        .text("text", pw_narrate::social::meme(c.w, m))
                        .cell("club", club_cell(c, m.club), Key::text(c.club_short(m.club)))
                        .num("recognition", f64::from(m.recognition))
                        .num("uses", f64::from(m.uses))
                        .date("born", m.born)
                        .text("alive", if m.alive { "Yes" } else { "Faded" });
                    if m.about.is_some() {
                        r = r.open(Ref::person(m.about));
                    }
                    r
                })
                .collect()
        },
    }
}

fn mood(v: i8) -> Cell {
    let t = if v >= 25 {
        Tone::Pos
    } else if v <= -25 {
        Tone::Neg
    } else {
        Tone::Muted
    };
    Cell::num(f64::from(v)).tone(t)
}

pub fn groups() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("club", "Club", Fmt::Text, 170, &[G]).left(),
                Col::new("kind", "Group", Fmt::Text, 130, &[G]).left(),
                Col::new("size", "Members", Fmt::Int, 90, &[G]),
                Col::new("manager", "On the manager", Fmt::Int, 110, &[G]).help("How they feel, from -100 to 100."),
                Col::new("board", "On the board", Fmt::Int, 100, &[G]),
                Col::new("team", "On the team", Fmt::Int, 100, &[G]),
                Col::new("voice", "Voice", Fmt::Int, 70, &[G]).help("How loudly they are heard, 0 to 100."),
                Col::new("last", "Last acted", Fmt::Date, 100, &[G]),
            ]
        },
        sort: ("size", true),
        build: |c, f| {
            let club = f_u32(f, "club").map(ClubId);
            c.w.net
                .groups
                .iter()
                .filter(|g| club.is_none_or(|x| g.club == x))
                .map(|g| {
                    Row::new()
                        .cell("club", club_cell(c, g.club), Key::text(c.club_short(g.club)))
                        .text("kind", crate::pages::club::group_word(g.kind))
                        .num("size", f64::from(g.size))
                        .cell("manager", mood(g.manager), Key::Num(f64::from(g.manager)))
                        .cell("board", mood(g.board), Key::Num(f64::from(g.board)))
                        .cell("team", mood(g.team), Key::Num(f64::from(g.team)))
                        .num("voice", f64::from(g.voice))
                        .date("last", g.last_action)
                        .open(Ref::club(g.club))
                })
                .collect()
        },
    }
}

pub fn rivalries() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("a", "Side", Fmt::Text, 160, &[G]).left(),
                Col::new("b", "Against", Fmt::Text, 160, &[G]).left(),
                Col::new("kind", "Why", Fmt::Text, 220, &[G]).left(),
                Col::new("intensity", "Intensity", Fmt::Int, 90, &[G]).help("0 to 100."),
                Col::new("record", "Record", Fmt::Text, 110, &[G]).nosort().help("Won, drawn and lost by the first side."),
                Col::new("since", "Since", Fmt::Date, 100, &[G]),
                Col::new("last", "Last met", Fmt::Date, 100, &[G]),
            ]
        },
        sort: ("intensity", true),
        build: |c, f| {
            let club = f_u32(f, "club").map(ClubId);
            c.w.culture
                .rivalries
                .list
                .iter()
                .filter(|r| club.is_none_or(|x| r.a == Side::Club(x) || r.b == Side::Club(x)))
                .map(|r| {
                    let (a, ar) = side_name(c, r.a);
                    let (b, br) = side_name(c, r.b);
                    let kinds = r.kinds.iter().map(|k| crate::pages::club::rivalry_word(*k)).collect::<Vec<_>>().join(", ");
                    let mut row = Row::new()
                        .cell("a", ar.map_or_else(|| Cell::text(a.clone()), |x| Cell::link(x, a.clone())), Key::text(&a))
                        .cell("b", br.map_or_else(|| Cell::text(b.clone()), |x| Cell::link(x, b.clone())), Key::text(&b))
                        .text("kind", kinds)
                        .num("intensity", f64::from(r.intensity))
                        .text("record", format!("{}-{}-{}", r.h2h.0, r.h2h.1, r.h2h.2))
                        .date("since", r.since);
                    row = if r.last_meeting.0 > 0 { row.date("last", r.last_meeting) } else { row.cell("last", Cell::empty(), Key::None) };
                    if let Some(x) = ar {
                        row = row.open(x);
                    }
                    row
                })
                .collect()
        },
    }
}

// ---- posts -----------------------------------------------------------------------------------------------------

pub struct Posts;

impl Posts {
    fn ids_of<'a>(c: &Ctx<'a>) -> impl Iterator<Item = &'a Post> + 'a {
        c.w.net.posts.iter().chain(c.w.net.kept.values())
    }
}

impl Source for Posts {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("author", "Account", Fmt::Text, 160, &[G]).left(),
            Col::new("text", "Post", Fmt::Text, 460, &[G]).left().nosort(),
            Col::new("about", "About", Fmt::Text, 150, &[G]).left(),
            Col::new("likes", "Likes", Fmt::Int, 70, &[G]),
            Col::new("reposts", "Reposts", Fmt::Int, 80, &[G]),
            Col::new("replies", "Replies", Fmt::Int, 80, &[G]),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let club = f_u32(f, "club").map(ClubId);
        let person = f_u32(f, "person").map(PersonId);
        let q = f_str(f, "q").map(str::to_lowercase);
        Self::ids_of(c)
            .filter(|p| club.is_none_or(|x| p.club == x || c.w.net.accounts[p.author as usize].club == x))
            .filter(|p| person.is_none_or(|x| p.about == x || c.w.net.accounts[p.author as usize].person == x))
            .filter(|p| {
                q.as_ref()
                    .is_none_or(|q| c.w.net.accounts[p.author as usize].handle.to_lowercase().contains(q.as_str()) || c.w.net.accounts[p.author as usize].display.to_lowercase().contains(q.as_str()))
            })
            .map(|p| p.id)
            .collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let Some(p) = c.w.net.post(id) else { return Key::None };
        match col {
            "date" => Key::Num(f64::from(p.date.0) * 100_000.0 + f64::from(p.minute)),
            "author" => Key::text(&c.w.net.accounts[p.author as usize].display),
            "about" => Key::text(if p.about.is_some() { c.person_name(p.about) } else { String::new() }),
            "likes" => Key::Num(f64::from(p.likes)),
            "reposts" => Key::Num(f64::from(p.reposts)),
            "replies" => Key::Num(f64::from(p.replies)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let Some(p) = c.w.net.post(id) else { return Cell::empty() };
        let a = &c.w.net.accounts[p.author as usize];
        match col {
            "date" => Cell::num(f64::from(p.date.0)),
            "author" => {
                let cell = Cell::text(a.display.clone()).with_sub(format!("@{}", a.handle));
                if a.person.is_some() { cell.with_ref(Ref::person(a.person)) } else { cell }
            }
            "text" => Cell::text(c.post_text(p)),
            "about" => person_cell(c, p.about),
            "likes" => Cell::num(f64::from(p.likes)),
            "reposts" => Cell::num(f64::from(p.reposts)),
            "replies" => Cell::num(f64::from(p.replies)),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        let p = c.w.net.post(id)?;
        if p.about.is_some() { Some(Ref::person(p.about)) } else { None }
    }
}

// ---- incidents -------------------------------------------------------------------------------------------------

pub struct Incidents;

impl Incidents {
    /// An inhabited person hears about what involves them or reached them; the rest stays unseen.
    fn known(c: &Ctx, i: &pw_world::incident::Incident) -> bool {
        let Some(me) = c.me() else { return true };
        i.parties.contains(&me) || i.witnesses.contains(&me) || (i.info != u32::MAX && c.w.grapevine.items.get(i.info as usize).is_some_and(|it| it.knower(me).is_some()))
    }
}

impl Source for Incidents {
    type Prep = ();

    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let mut v = vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("what", "What happened", Fmt::Text, 460, &[G]).left().nosort(),
            Col::new("kind", "Kind", Fmt::Text, 170, &[G]).left(),
            Col::new("club", "Club", Fmt::Text, 150, &[G]).left(),
            Col::new("place", "Where", Fmt::Text, 120, &[G]).left(),
            Col::new("status", "Status", Fmt::Text, 90, &[G]).left(),
        ];
        if c.observer() {
            v.push(Col::new("severity", "Severity", Fmt::Int, 80, &[INT, G]).help("How serious it was, 0 to 100."));
            v.push(Col::new("witnesses", "Witnesses", Fmt::Int, 80, &[INT, G]));
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let club = f_u32(f, "club").map(ClubId);
        let person = f_u32(f, "person").map(PersonId);
        let open = f.get("open").and_then(Value::as_bool);
        c.w.incidents
            .list
            .iter()
            .filter(|i| club.is_none_or(|x| i.club == x))
            .filter(|i| person.is_none_or(|p| i.parties.contains(&p)))
            .filter(|i| open.is_none_or(|o| i.resolved != o))
            .filter(|i| Self::known(c, i))
            .map(|i| i.id)
            .collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let Some(i) = c.w.incidents.get(id) else { return Key::None };
        match col {
            "date" => date_key(i.date, id),
            "kind" => Key::text(words(&i.kind)),
            "club" => Key::text(c.club_short(i.club)),
            "place" => Key::text(pw_narrate::incidents::place(i.location)),
            "status" => Key::Num(if i.resolved { 1.0 } else { 0.0 }),
            "severity" => Key::Num(f64::from(i.severity)),
            "witnesses" => Key::Num(i.witnesses.len() as f64),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let Some(i) = c.w.incidents.get(id) else { return Cell::empty() };
        match col {
            "date" => Cell::num(f64::from(i.date.0)),
            "what" => Cell::text(pw_narrate::incidents::sentence(c.w, id, c.me().unwrap_or(PersonId::NONE))),
            "kind" => Cell::text(words(&i.kind)),
            "club" => club_cell(c, i.club),
            "place" => Cell::text(pw_narrate::incidents::place(i.location)),
            "status" => {
                if i.resolved {
                    Cell::text("Settled").tone(Tone::Muted)
                } else {
                    Cell::text("Open").tone(Tone::Warn)
                }
            }
            "severity" => Cell::num(f64::from(i.severity)),
            "witnesses" => Cell::num(i.witnesses.len() as f64),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        let i = c.w.incidents.get(id)?;
        i.parties.first().map(|&p| Ref::person(p)).or_else(|| (i.club.is_some()).then(|| Ref::club(i.club)))
    }
}

// ---- press conferences and quotes -----------------------------------------------------------------------------

pub struct Conferences;

impl Source for Conferences {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
            Col::new("speaker", "Spoke", Fmt::Text, 170, &[G]).left(),
            Col::new("questions", "Questions", Fmt::Int, 90, &[G]),
            Col::new("answered", "Answered", Fmt::Int, 90, &[G]),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let club = f_u32(f, "club").map(ClubId);
        let person = f_u32(f, "person").map(PersonId);
        c.w.pressroom.conferences.iter().filter(|x| club.is_none_or(|k| x.club == k)).filter(|x| person.is_none_or(|p| x.speaker == p)).map(|x| x.id).collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let Some(x) = c.w.pressroom.conferences.get(id as usize) else { return Key::None };
        match col {
            "date" => date_key(x.date, id),
            "club" => Key::text(c.club_short(x.club)),
            "speaker" => Key::text(c.person_name(x.speaker)),
            "questions" => Key::Num(x.questions.len() as f64),
            "answered" => Key::Num(x.answers.iter().flatten().count() as f64),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let Some(x) = c.w.pressroom.conferences.get(id as usize) else { return Cell::empty() };
        match col {
            "date" => Cell::num(f64::from(x.date.0)),
            "club" => club_cell(c, x.club),
            "speaker" => person_cell(c, x.speaker),
            "questions" => Cell::num(x.questions.len() as f64),
            "answered" => Cell::num(x.answers.iter().flatten().count() as f64),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        c.w.pressroom.conferences.get(id as usize).map(|x| Ref::person(x.speaker))
    }
}

pub struct Quotes;

impl Source for Quotes {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("speaker", "Who", Fmt::Text, 170, &[G]).left(),
            Col::new("stance", "Said", Fmt::Text, 150, &[G]).left(),
            Col::new("about", "About", Fmt::Text, 170, &[G]).left(),
            Col::new("headline", "Reported as", Fmt::Text, 420, &[G]).left().nosort(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let person = f_u32(f, "person").map(PersonId);
        c.w.pressroom.quotes.iter().filter(|q| person.is_none_or(|p| q.speaker == p || q.about == p)).map(|q| q.id).collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let Some(q) = c.w.pressroom.quotes.get(id as usize) else { return Key::None };
        match col {
            "date" => date_key(q.date, id),
            "speaker" => Key::text(c.person_name(q.speaker)),
            "stance" => Key::text(pw_narrate::press::stance_label(q.stance)),
            "about" => Key::text(if q.about.is_some() { c.person_name(q.about) } else { String::new() }),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let Some(q) = c.w.pressroom.quotes.get(id as usize) else { return Cell::empty() };
        match col {
            "date" => Cell::num(f64::from(q.date.0)),
            "speaker" => person_cell(c, q.speaker),
            "stance" => Cell::text(pw_narrate::press::stance_label(q.stance)),
            "about" => person_cell(c, q.about),
            "headline" => c.w.media.stories.get(q.story).map_or_else(Cell::empty, |s| Cell::text(c.headline(s))),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        c.w.pressroom.quotes.get(id as usize).map(|q| Ref::person(q.speaker))
    }
}

// ---- referees and big calls ------------------------------------------------------------------------------------

pub fn referees() -> Grid {
    Grid {
        cols: |c| {
            let mut v = vec![
                Col::new("name", "Referee", Fmt::Text, 190, &[G]).left(),
                Col::new("nation", "Nation", Fmt::Text, 130, &[G]).left(),
                Col::new("tier", "Level", Fmt::Int, 70, &[G]).help("Higher levels take bigger matches."),
                Col::new("matches", "Matches", Fmt::Int, 80, &[G]),
                Col::new("reds", "Red cards", Fmt::Int, 80, &[G]),
                Col::new("penalties", "Penalties", Fmt::Int, 80, &[G]),
                Col::new("big", "Big calls", Fmt::Int, 80, &[G]),
            ];
            if c.observer() {
                v.push(Col::new("strictness", "Strictness", Fmt::Int, 90, &[INT]));
                v.push(Col::new("accuracy", "Accuracy", Fmt::Int, 80, &[INT]).help("How often the big calls are right."));
                v.push(Col::new("wrong", "Overturned", Fmt::Int, 90, &[INT]));
            }
            v
        },
        sort: ("matches", true),
        build: |c, _| {
            c.w.officials
                .referees
                .iter()
                .filter(|r| r.active)
                .map(|r| {
                    Row::new()
                        .cell("name", person_cell(c, r.person), Key::text(c.person_name(r.person)))
                        .cell("nation", Cell::link(Ref::nation(r.nation), c.nation_name(r.nation)), Key::text(c.nation_name(r.nation)))
                        .num("tier", f64::from(r.tier))
                        .num("matches", f64::from(r.matches))
                        .num("reds", f64::from(r.reds))
                        .num("penalties", f64::from(r.penalties))
                        .num("big", f64::from(r.big_calls))
                        .num("strictness", f64::from(r.strictness))
                        .num("accuracy", f64::from(r.accuracy))
                        .num("wrong", f64::from(r.wrong))
                        .open(Ref::person(r.person))
                })
                .collect()
        },
    }
}

pub fn controversies() -> Grid {
    Grid {
        cols: |c| {
            let mut v = vec![
                Col::new("date", "Date", Fmt::Date, 100, &[G]),
                Col::new("call", "Call", Fmt::Text, 280, &[G]).left().nosort(),
                Col::new("referee", "Referee", Fmt::Text, 170, &[G]).left(),
                Col::new("against", "Went against", Fmt::Text, 150, &[G]).left(),
                Col::new("minute", "Minute", Fmt::Int, 70, &[G]),
                Col::new("anger", "Supporters' anger", Fmt::Int, 110, &[G]).help("0 to 100."),
                Col::new("appeal", "Appeal", Fmt::Text, 130, &[G]).left(),
            ];
            if c.observer() {
                v.push(Col::new("right", "Call was right", Fmt::Text, 110, &[INT, G]).left().help("Hidden from everyone in the game until a panel rules."));
            }
            v
        },
        sort: ("date", true),
        build: |c, f| {
            let club = f_u32(f, "club").map(ClubId);
            let w = c.w;
            w.officials
                .controversies
                .iter()
                .filter(|x| club.is_none_or(|k| x.against == k || x.benefited == k))
                .map(|x| {
                    let appeal = x.appeal.and_then(|a| w.officials.appeals.get(a as usize)).map_or("None".to_string(), |a| match a.outcome {
                        Some(AppealOutcome::Rescinded) => "Card rescinded".into(),
                        Some(AppealOutcome::Upheld) => "Rejected".into(),
                        Some(AppealOutcome::Extended) => "Ban extended".into(),
                        None => "Pending".into(),
                    });
                    Row::new()
                        .cell("date", Cell::num(f64::from(x.date.0)), date_key(x.date, x.id))
                        .text("call", pw_narrate::officiating::call(w, x))
                        .text("referee", pw_narrate::officiating::referee(w, x.referee))
                        .cell("against", club_cell(c, x.against), Key::text(c.club_short(x.against)))
                        .num("minute", f64::from(x.minute))
                        .num("anger", f64::from(x.grievance))
                        .text("appeal", appeal)
                        .text("right", if x.correct { "Yes" } else { "No" })
                        .open(Ref::club(x.against))
                })
                .collect()
        },
    }
}

pub fn charges() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("date", "Date", Fmt::Date, 100, &[G]),
                Col::new("text", "Charge", Fmt::Text, 520, &[G]).left().nosort(),
                Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
                Col::new("fine", "Fine", Fmt::Money, 100, &[G]),
            ]
        },
        sort: ("date", true),
        build: |c, f| {
            let club = f_u32(f, "club").map(ClubId);
            c.w.officials
                .charges
                .iter()
                .filter(|x| club.is_none_or(|k| x.club == k))
                .map(|x| {
                    Row::new()
                        .cell("date", Cell::num(f64::from(x.date.0)), date_key(x.date, x.id))
                        .text("text", pw_narrate::officiating::charge(c.w, x))
                        .cell("club", club_cell(c, x.club), Key::text(c.club_short(x.club)))
                        .num("fine", x.fine as f64)
                        .open(Ref::club(x.club))
                })
                .collect()
        },
    }
}

// ---- records and honours ---------------------------------------------------------------------------------------

pub fn record_book() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("name", "Record", Fmt::Text, 380, &[G]).left(),
                Col::new("holder", "Held by", Fmt::Text, 200, &[G]).left(),
                Col::new("value", "Mark", Fmt::Text, 110, &[G]).nosort(),
                Col::new("date", "Set", Fmt::Date, 100, &[G]),
                Col::new("broken", "Times broken", Fmt::Int, 100, &[G]),
            ]
        },
        sort: ("name", false),
        build: |c, _| {
            let w = c.w;
            let mut recs: Vec<_> = w.records.records.values().map(|r| (pw_narrate::history::record_name(w, r.key), r)).collect();
            recs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.current.date.cmp(&b.1.current.date)));
            recs.into_iter()
                .map(|(name, r)| {
                    let name = name.strip_prefix("the ").unwrap_or(&name);
                    let name = format!("{}{}", name.chars().next().map_or(String::new(), |f| f.to_uppercase().collect()), name.chars().skip(1).collect::<String>());
                    let holder = pw_narrate::history::holder(w, r.current.holder);
                    let mut row = Row::new()
                        .text("name", name)
                        .cell("holder", holder_ref(r.current.holder).map_or_else(|| Cell::text(holder.clone()), |x| Cell::link(x, holder.clone())), Key::text(&holder))
                        .text("value", pw_narrate::history::value(r.key.stat, r.current.value))
                        .date("date", r.current.date)
                        .num("broken", f64::from(r.broken));
                    if let Some(x) = holder_ref(r.current.holder) {
                        row = row.open(x);
                    }
                    row
                })
                .collect()
        },
    }
}

pub fn records_broken() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("date", "Date", Fmt::Date, 100, &[G]),
                Col::new("text", "Record broken", Fmt::Text, 620, &[G]).left().nosort(),
                Col::new("stood", "Stood for", Fmt::Int, 90, &[G]).help("Days."),
            ]
        },
        sort: ("date", true),
        build: |c, _| {
            c.w.records
                .broken
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    let mut row = Row::new()
                        .cell("date", Cell::num(f64::from(b.new.date.0)), date_key(b.new.date, i as u32))
                        .text("text", pw_narrate::history::broken(c.w, b))
                        .num("stood", f64::from(b.stood_days));
                    if let Some(x) = holder_ref(b.new.holder) {
                        row = row.open(x);
                    }
                    row
                })
                .collect()
        },
    }
}

pub fn votes() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("year", "Year", Fmt::Int, 70, &[G]),
                Col::new("ballot", "Vote", Fmt::Text, 320, &[G]).left(),
                Col::new("winner", "Winner", Fmt::Text, 190, &[G]).left(),
                Col::new("share", "Named on", Fmt::Text, 130, &[G]).nosort(),
                Col::new("voters", "Voters", Fmt::Int, 80, &[G]),
            ]
        },
        sort: ("year", true),
        build: |c, _| {
            let w = c.w;
            w.acclaim
                .votes
                .iter()
                .map(|v| {
                    let winner = v.result.first().map(|x| x.0);
                    let mut row = Row::new().num("year", f64::from(v.year)).text("ballot", pw_narrate::history::ballot(w, v.ballot)).num("voters", f64::from(v.voters));
                    row = match winner {
                        Some(p) => row
                            .cell("winner", person_cell(c, p), Key::text(c.person_name(p)))
                            .text("share", if v.casts.is_empty() { String::new() } else { format!("{} of {}", v.named_by(p), v.voters) })
                            .open(Ref::person(p)),
                        None => row.text("winner", "").text("share", ""),
                    };
                    row
                })
                .collect()
        },
    }
}

pub fn hall_members() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("hall", "Hall of fame", Fmt::Text, 300, &[G]).left(),
                Col::new("person", "Inducted", Fmt::Text, 190, &[G]).left(),
                Col::new("year", "Year", Fmt::Int, 70, &[G]),
                Col::new("share", "Share of vote", Fmt::Int, 100, &[G]).help("Per cent of the voters who named them."),
            ]
        },
        sort: ("year", true),
        build: |c, _| {
            let w = c.w;
            w.acclaim
                .halls
                .iter()
                .flat_map(|h| {
                    let name = pw_narrate::history::hall_name(w, h.scope);
                    let name = format!("{}{}", name.chars().next().map_or(String::new(), |f| f.to_uppercase().collect()), name.chars().skip(1).collect::<String>());
                    h.members.iter().map(move |m| (name.clone(), m))
                })
                .map(|(hall, m)| {
                    Row::new()
                        .text("hall", hall)
                        .cell("person", person_cell(c, m.person), Key::text(c.person_name(m.person)))
                        .num("year", f64::from(m.year))
                        .num("share", f64::from(m.share))
                        .open(Ref::person(m.person))
                })
                .collect()
        },
    }
}

pub fn chronicle() -> Grid {
    Grid {
        cols: |_| vec![Col::new("date", "Date", Fmt::Date, 100, &[G]), Col::new("text", "In football's history", Fmt::Text, 720, &[G]).left().nosort()],
        sort: ("date", true),
        build: |c, _| {
            c.w.acclaim.chronicle.iter().map(|e| Row::new().cell("date", Cell::num(f64::from(e.date.0)), date_key(e.date, e.id)).text("text", pw_narrate::history::chronicle(c.w, e))).collect()
        },
    }
}

// ---- evolution -------------------------------------------------------------------------------------------------

pub fn schools() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("name", "School of thought", Fmt::Text, 180, &[G]).left(),
                Col::new("style", "Principles", Fmt::Text, 420, &[G]).left().nosort(),
                Col::new("founder", "Founder", Fmt::Text, 180, &[G]).left(),
                Col::new("origin", "From", Fmt::Text, 130, &[G]).left(),
                Col::new("born", "Founded", Fmt::Date, 100, &[G]),
                Col::new("adherents", "Managers", Fmt::Int, 80, &[G]),
                Col::new("titles", "Titles", Fmt::Int, 70, &[G]),
                Col::new("prestige", "Standing", Fmt::Int, 80, &[G]),
            ]
        },
        sort: ("prestige", true),
        build: |c, _| {
            let w = c.w;
            w.evolution
                .schools
                .iter()
                .map(|s| {
                    Row::new()
                        .text("name", pw_narrate::history::school_name(w, s))
                        .text("style", pw_narrate::history::school_style(s))
                        .cell("founder", person_cell(c, s.founder), Key::text(c.person_name(s.founder)))
                        .cell("origin", Cell::link(Ref::nation(s.origin), c.nation_name(s.origin)), Key::text(c.nation_name(s.origin)))
                        .date("born", s.born)
                        .num("adherents", s.adherents.len() as f64)
                        .num("titles", f64::from(s.titles))
                        .num("prestige", f64::from(s.prestige))
                        .open(Ref::person(s.founder))
                })
                .collect()
        },
    }
}

pub fn rule_changes() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("date", "Date", Fmt::Date, 100, &[G]),
                Col::new("text", "Change", Fmt::Text, 600, &[G]).left().nosort(),
                Col::new("nation", "Nation", Fmt::Text, 140, &[G]).left(),
                Col::new("from", "From season", Fmt::Int, 100, &[G]),
            ]
        },
        sort: ("date", true),
        build: |c, _| {
            c.w.evolution
                .changes
                .iter()
                .map(|x| {
                    Row::new()
                        .cell("date", Cell::num(f64::from(x.date.0)), date_key(x.date, x.id))
                        .text("text", pw_narrate::history::rule_change(c.w, x))
                        .cell("nation", Cell::link(Ref::nation(x.nation), c.nation_name(x.nation)), Key::text(c.nation_name(x.nation)))
                        .num("from", f64::from(x.from_season))
                        .open(Ref::nation(x.nation))
                })
                .collect()
        },
    }
}

// ---- below the professional game -------------------------------------------------------------------------------

pub fn institutions() -> Grid {
    Grid {
        cols: |c| {
            let mut v = vec![
                Col::new("name", "Name", Fmt::Text, 240, &[G]).left(),
                Col::new("kind", "Kind", Fmt::Text, 100, &[G]).left(),
                Col::new("nation", "Nation", Fmt::Text, 130, &[G]).left(),
                Col::new("city", "City", Fmt::Text, 130, &[G]).left(),
                Col::new("founded", "Founded", Fmt::Int, 80, &[G]),
                Col::new("prestige", "Prestige", Fmt::Int, 80, &[G]),
                Col::new("members", "Players", Fmt::Int, 80, &[G]),
                Col::new("alumni", "Went professional", Fmt::Int, 120, &[G]),
            ];
            if c.observer() {
                v.push(Col::new("coaching", "Coaching", Fmt::Int, 80, &[INT]));
            }
            v
        },
        sort: ("prestige", true),
        build: |c, f| {
            let kind = f_str(f, "kind");
            c.w.minor
                .institutions
                .iter()
                .enumerate()
                .filter(|(_, i)| kind.is_none_or(|k| pw_narrate::history::inst_kind(i.kind).eq_ignore_ascii_case(k)))
                .map(|(n, i)| {
                    Row::new()
                        .cell("name", Cell::link(Ref::inst(n as u32), i.name.clone()), Key::text(i.name.clone()))
                        .text("kind", pw_narrate::history::inst_kind(i.kind))
                        .cell("nation", Cell::link(Ref::nation(i.nation), c.nation_name(i.nation)), Key::text(c.nation_name(i.nation)))
                        .text("city", i.city.clone())
                        .num("founded", f64::from(i.founded))
                        .num("prestige", f64::from(i.prestige))
                        .num("members", i.members.len() as f64)
                        .num("alumni", i.alumni_pros.len() as f64)
                        .num("coaching", f64::from(i.coaching))
                        .open(Ref::inst(n as u32))
                })
                .collect()
        },
    }
}

pub fn minor_seasons() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("season", "Season", Fmt::Int, 80, &[G]),
                Col::new("text", "Result", Fmt::Text, 640, &[G]).left().nosort(),
                Col::new("nation", "Nation", Fmt::Text, 140, &[G]).left(),
                Col::new("top", "Top scorer", Fmt::Text, 160, &[G]).left(),
            ]
        },
        sort: ("season", true),
        build: |c, _| {
            c.w.minor
                .history
                .iter()
                .map(|s| {
                    let mut row = Row::new()
                        .num("season", f64::from(s.season))
                        .text("text", pw_narrate::history::season_line(c.w, s))
                        .cell("nation", Cell::link(Ref::nation(s.nation), c.nation_name(s.nation)), Key::text(c.nation_name(s.nation)))
                        .open(Ref::nation(s.nation));
                    row = if s.top_scorer.is_some() {
                        row.cell("top", Cell::link(c.player_ref(s.top_scorer), c.player_name(s.top_scorer)), Key::text(c.player_name(s.top_scorer)))
                    } else {
                        row.text("top", "")
                    };
                    row
                })
                .collect()
        },
    }
}

// ---- the press corps and the grapevine --------------------------------------------------------------------------

pub fn outlets() -> Grid {
    Grid {
        cols: |c| {
            let mut v = vec![
                Col::new("name", "Outlet", Fmt::Text, 220, &[G]).left(),
                Col::new("kind", "Kind", Fmt::Text, 120, &[G]).left(),
                Col::new("nation", "Nation", Fmt::Text, 130, &[G]).left(),
                Col::new("reach", "Reach", Fmt::Int, 80, &[G]).help("How many people read it, 0 to 100."),
                Col::new("staff", "Journalists", Fmt::Int, 90, &[G]),
            ];
            if c.observer() {
                v.push(Col::new("accuracy", "Accuracy", Fmt::Int, 80, &[INT]));
                v.push(Col::new("sensation", "Sensationalism", Fmt::Int, 110, &[INT]));
                v.push(Col::new("credibility", "Credibility", Fmt::Int, 90, &[INT]).help("What readers have learned to think of it."));
            }
            v
        },
        sort: ("reach", true),
        build: |c, _| {
            let w = c.w;
            w.media
                .outlets
                .iter_enumerated()
                .map(|(id, o)| {
                    let staff = w.media.journalists.values().filter(|j| j.outlet == id).count();
                    Row::new()
                        .text("name", o.name.clone())
                        .text("kind", words(&o.kind))
                        .cell("nation", Cell::link(Ref::nation(o.nation), c.nation_name(o.nation)), Key::text(c.nation_name(o.nation)))
                        .num("reach", f64::from(o.reach))
                        .num("staff", staff as f64)
                        .num("accuracy", f64::from(o.accuracy))
                        .num("sensation", f64::from(o.sensationalism))
                        .num("credibility", f64::from(o.credibility))
                        .open(Ref::nation(o.nation))
                })
                .collect()
        },
    }
}

pub fn journalists() -> Grid {
    Grid {
        cols: |c| {
            let mut v = vec![
                Col::new("name", "Journalist", Fmt::Text, 190, &[G]).left(),
                Col::new("outlet", "Writes for", Fmt::Text, 200, &[G]).left(),
                Col::new("beat", "Covers", Fmt::Text, 300, &[G]).left().nosort(),
            ];
            if c.observer() {
                v.push(Col::new("credibility", "Track record", Fmt::Int, 100, &[INT]).help("How often what they print turns out true, 0 to 100."));
                v.push(Col::new("sources", "Sources", Fmt::Int, 80, &[INT]));
            }
            v
        },
        sort: ("name", false),
        build: |c, f| {
            let w = c.w;
            let club = f_u32(f, "club").map(ClubId);
            w.media
                .journalists
                .values()
                .filter(|j| club.is_none_or(|x| j.beat.contains(&x)))
                .map(|j| {
                    let outlet = if j.outlet.is_some() { w.media.outlets[j.outlet].name.clone() } else { "Freelance".to_string() };
                    let beat = j.beat.iter().map(|&x| c.club_short(x)).collect::<Vec<_>>().join(", ");
                    Row::new()
                        .cell("name", person_cell(c, j.person), Key::text(c.person_name(j.person)))
                        .text("outlet", outlet)
                        .text("beat", beat)
                        .num("credibility", f64::from(j.credibility))
                        .num("sources", j.sources.len() as f64)
                        .open(Ref::person(j.person))
                })
                .collect()
        },
    }
}

/// What is going round, and how far. Observers only: nobody in the game sees the whole of it.
pub fn grapevine() -> Grid {
    Grid {
        cols: |_| {
            vec![
                Col::new("date", "Date", Fmt::Date, 100, &[INT, G]),
                Col::new("what", "The truth of it", Fmt::Text, 520, &[INT, G]).left().nosort(),
                Col::new("knowers", "Know", Fmt::Int, 70, &[INT, G]),
                Col::new("live", "Still spreading", Fmt::Text, 110, &[INT, G]).left(),
            ]
        },
        sort: ("date", true),
        build: |c, _| {
            if !c.observer() {
                return Vec::new();
            }
            let w = c.w;
            w.grapevine
                .items
                .iter()
                .enumerate()
                .rev()
                .take(3000)
                .map(|(i, it)| {
                    Row::new()
                        .cell("date", Cell::num(f64::from(it.date.0)), date_key(it.date, i as u32))
                        .text("what", pw_narrate::grapevine::what(w, i as u32))
                        .num("knowers", it.holders.len() as f64)
                        .text("live", if it.closed { "No" } else { "Yes" })
                })
                .collect()
        },
    }
}
