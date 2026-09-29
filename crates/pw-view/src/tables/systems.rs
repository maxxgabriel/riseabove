//! Tables for the systems that arrived with the world: press stories, agents, contract talks, club
//! deals, international football, boards and sponsors. What an inhabited person may see is decided
//! here: private talks and internal boardroom numbers stay with the observer.

use pw_core::{ClubId, NationId, StoryId};
use pw_world::deals::DealState;
use pw_world::intl::{Level, MatchKind, Stage};
use pw_world::negotiation::TalkState;
use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{Cell, Col, Fmt, Ref, Tone};
use crate::table::{Key, Source, f_str, f_u32};

const G: &str = "general";
const INT: &str = "internal";

fn nation_cell(c: &Ctx, n: NationId) -> Cell {
    if n.is_some() { Cell::link(Ref::nation(n), c.nation_name(n)) } else { Cell::empty() }
}

fn club_cell(c: &Ctx, x: ClubId) -> Cell {
    if x.is_some() { Cell::link(Ref::club(x), c.club_short(x)) } else { Cell::empty() }
}

// ---- press -------------------------------------------------------------------------------------

pub struct Stories;

fn kind_word(k: pw_world::media::StoryKind) -> String {
    let s = format!("{k:?}");
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

impl Source for Stories {
    type Prep = ();

    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let mut v = vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("outlet", "Outlet", Fmt::Text, 150, &[G]).left(),
            Col::new("headline", "Headline", Fmt::Text, 420, &[G]).left(),
            Col::new("kind", "Kind", Fmt::Text, 130, &[G]).left(),
        ];
        if c.observer() {
            v.push(Col::new("grounded", "Grounded", Fmt::Text, 90, &[INT, G]).left().help("Whether the claim matches the truth when it was printed. Readers cannot tell."));
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let person = f_u32(f, "person").map(pw_core::PersonId);
        let club = f_u32(f, "club").map(ClubId);
        let q = f_str(f, "q").map(str::to_lowercase);
        c.w.media
            .stories
            .iter_enumerated()
            .filter(|(_, s)| person.is_none_or(|p| s.person == p))
            .filter(|(_, s)| club.is_none_or(|x| s.club == x || s.other_club == x))
            .filter(|(_, s)| q.as_ref().is_none_or(|q| c.headline(s).to_lowercase().contains(q.as_str())))
            .map(|(id, _)| id.0)
            .collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let s = &c.w.media.stories[StoryId(id)];
        match col {
            "date" => Key::Num(f64::from(s.date.0) * 1_000_000.0 + f64::from(id)),
            "outlet" => Key::text(pw_narrate::press::outlet_name(c.w, s)),
            "headline" => Key::text(c.headline(s)),
            "kind" => Key::text(kind_word(s.kind)),
            "grounded" => Key::Num(if s.grounded { 1.0 } else { 0.0 }),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let s = &c.w.media.stories[StoryId(id)];
        match col {
            "date" => Cell::num(f64::from(s.date.0)),
            "outlet" => Cell::text(pw_narrate::press::outlet_name(c.w, s)),
            "headline" => Cell::text(c.headline(s)),
            "kind" => Cell::text(kind_word(s.kind)),
            "grounded" => {
                if s.grounded {
                    Cell::text("Yes").tone(Tone::Pos)
                } else {
                    Cell::text("No").tone(Tone::Warn)
                }
            }
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        let s = &c.w.media.stories[StoryId(id)];
        if s.person.is_some() {
            Some(Ref::person(s.person))
        } else if s.club.is_some() {
            Some(Ref::club(s.club))
        } else {
            None
        }
    }
}

// ---- agents ------------------------------------------------------------------------------------

pub struct AgentsTable;

impl Source for AgentsTable {
    type Prep = ();

    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let mut v = vec![
            Col::new("name", "Agent", Fmt::Text, 190, &[G]).left(),
            Col::new("base", "Based in", Fmt::Text, 130, &[G]).left(),
            Col::new("rep", "Reputation", Fmt::Int, 90, &[G]),
            Col::new("clients", "Clients", Fmt::Int, 70, &[G]),
        ];
        if c.observer() {
            v.push(Col::new("negotiating", "Negotiating", Fmt::Int, 90, &[INT]));
            v.push(Col::new("network", "Network", Fmt::Int, 80, &[INT]));
            v.push(Col::new("diligence", "Diligence", Fmt::Int, 80, &[INT]));
            v.push(Col::new("greed", "Greed", Fmt::Int, 70, &[INT]));
            v.push(Col::new("honesty", "Honesty", Fmt::Int, 80, &[INT]));
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("rep", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let nation = f_u32(f, "nation").map(NationId);
        c.w.agents.list.iter_enumerated().filter(|(_, a)| a.active).filter(|(_, a)| nation.is_none_or(|n| a.covers(n))).map(|(id, _)| id.0).collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let a = &c.w.agents.list[pw_core::AgentId(id)];
        match col {
            "name" => Key::text(c.person_name(a.person)),
            "base" => Key::text(c.nation_name(a.base)),
            "rep" => Key::Num(f64::from(a.reputation)),
            "clients" => Key::Num(a.clients.len() as f64),
            "negotiating" => Key::Num(f64::from(a.negotiating)),
            "network" => Key::Num(f64::from(a.network)),
            "diligence" => Key::Num(f64::from(a.diligence)),
            "greed" => Key::Num(f64::from(a.greed)),
            "honesty" => Key::Num(f64::from(a.honesty)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let a = &c.w.agents.list[pw_core::AgentId(id)];
        match col {
            "name" => Cell::link(Ref::person(a.person), c.person_name(a.person)),
            "base" => nation_cell(c, a.base),
            "rep" => Cell::num(f64::from(a.reputation)),
            "clients" => Cell::num(a.clients.len() as f64),
            "negotiating" => Cell::num(f64::from(a.negotiating)),
            "network" => Cell::num(f64::from(a.network)),
            "diligence" => Cell::num(f64::from(a.diligence)),
            "greed" => Cell::num(f64::from(a.greed)),
            "honesty" => Cell::num(f64::from(a.honesty)),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(Ref::person(c.w.agents.list[pw_core::AgentId(id)].person))
    }
}

// ---- contract talks (private) -----------------------------------------------------------------

pub struct Talks;

fn talk_state(s: TalkState) -> (&'static str, Option<Tone>) {
    match s {
        TalkState::PlayerTurn => ("Player to answer", Some(Tone::Info)),
        TalkState::ClubTurn => ("Club to answer", Some(Tone::Info)),
        TalkState::Agreed => ("Agreed", Some(Tone::Pos)),
        TalkState::Collapsed => ("Collapsed", Some(Tone::Neg)),
    }
}

impl Source for Talks {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("opened", "Opened", Fmt::Date, 100, &[G]),
            Col::new("player", "Player", Fmt::Text, 180, &[G]).left(),
            Col::new("club", "Club", Fmt::Text, 150, &[G]).left(),
            Col::new("kind", "About", Fmt::Text, 190, &[G]).left(),
            Col::new("state", "State", Fmt::Text, 130, &[G]).left(),
            Col::new("round", "Round", Fmt::Text, 70, &[G]),
            Col::new("wage", "Offer / wk", Fmt::Money, 100, &[G]),
            Col::new("years", "Years", Fmt::Int, 60, &[G]),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("opened", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        if !c.observer() {
            return vec![];
        }
        let open_only = f.get("open").and_then(Value::as_bool).unwrap_or(false);
        c.w.talks.iter_enumerated().filter(|(_, t)| !open_only || t.is_open()).map(|(id, _)| id.0).collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let t = &c.w.talks[pw_core::TalkId(id)];
        match col {
            "opened" => Key::Num(f64::from(t.opened.0) * 1_000_000.0 + f64::from(id)),
            "player" => Key::text(c.player_short(t.player)),
            "club" => Key::text(c.club_short(t.club)),
            "kind" => Key::text(t.kind.label()),
            "state" => Key::text(talk_state(t.state).0),
            "round" => Key::Num(f64::from(t.round)),
            "wage" => Key::Num(t.offer.wage as f64),
            "years" => Key::Num(f64::from(t.offer.years)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let t = &c.w.talks[pw_core::TalkId(id)];
        match col {
            "opened" => Cell::num(f64::from(t.opened.0)),
            "player" => Cell::link(c.player_ref(t.player), c.player_name(t.player)),
            "club" => club_cell(c, t.club),
            "kind" => Cell::text(t.kind.label()),
            "state" => {
                let (s, tone) = talk_state(t.state);
                let cell = Cell::text(s);
                tone.map_or(cell.clone(), |x| cell.tone(x))
            }
            "round" => Cell::text(format!("{}/{}", t.round, t.max_rounds)),
            "wage" => Cell::num(t.offer.wage as f64),
            "years" => Cell::num(f64::from(t.offer.years)),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(c.player_ref(c.w.talks[pw_core::TalkId(id)].player))
    }

    fn note(&self, c: &Ctx, _p: &(), _f: &Value) -> Option<String> {
        (!c.observer()).then(|| "Contract talks are private. The ones that involve you appear in your messages.".to_string())
    }
}

// ---- club-to-club deals (private) --------------------------------------------------------------

pub struct Bids;

fn deal_state(s: DealState) -> (&'static str, Tone) {
    match s {
        DealState::Enquiry => ("Enquiry", Tone::Muted),
        DealState::Bid => ("Bid made", Tone::Info),
        DealState::Counter => ("Counter-offer", Tone::Info),
        DealState::Medical => ("Medical", Tone::Info),
        DealState::Terms => ("Personal terms", Tone::Info),
        DealState::Done => ("Done", Tone::Pos),
        DealState::Collapsed => ("Collapsed", Tone::Neg),
    }
}

impl Source for Bids {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("opened", "Opened", Fmt::Date, 100, &[G]),
            Col::new("player", "Player", Fmt::Text, 180, &[G]).left(),
            Col::new("buyer", "Buyer", Fmt::Text, 140, &[G]).left(),
            Col::new("seller", "Seller", Fmt::Text, 140, &[G]).left(),
            Col::new("state", "State", Fmt::Text, 120, &[G]).left(),
            Col::new("fee", "Offer", Fmt::Money, 100, &[G]),
            Col::new("ask", "Asking", Fmt::Money, 100, &[G]),
            Col::new("end", "Ended because", Fmt::Text, 220, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("opened", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        if !c.observer() {
            return vec![];
        }
        let club = f_u32(f, "club").map(ClubId);
        let live = f.get("open").and_then(Value::as_bool).unwrap_or(false);
        c.w.deals
            .deals
            .iter()
            .enumerate()
            .filter(|(_, d)| club.is_none_or(|x| d.buyer == x || d.seller == x))
            .filter(|(_, d)| !live || !matches!(d.state, DealState::Done | DealState::Collapsed))
            .map(|(i, _)| i as u32)
            .collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let d = &c.w.deals.deals[id as usize];
        match col {
            "opened" => Key::Num(f64::from(d.opened.0) * 1_000_000.0 + f64::from(id)),
            "player" => Key::text(c.player_short(d.player)),
            "buyer" => Key::text(c.club_short(d.buyer)),
            "seller" => Key::text(c.club_short(d.seller)),
            "state" => Key::text(deal_state(d.state).0),
            "fee" => Key::Num(d.terms.fee as f64),
            "ask" => d.ask.as_ref().map_or(Key::None, |a| Key::Num(a.fee as f64)),
            "end" => d.end.map_or(Key::None, |e| Key::text(e.label())),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let d = &c.w.deals.deals[id as usize];
        match col {
            "opened" => Cell::num(f64::from(d.opened.0)),
            "player" => Cell::link(c.player_ref(d.player), c.player_name(d.player)),
            "buyer" => club_cell(c, d.buyer),
            "seller" => club_cell(c, d.seller),
            "state" => {
                let (s, t) = deal_state(d.state);
                Cell::text(s).tone(t)
            }
            "fee" => Cell::num(d.terms.fee as f64),
            "ask" => d.ask.as_ref().map_or(Cell::empty(), |a| Cell::num(a.fee as f64)),
            "end" => d.end.map_or(Cell::empty(), |e| Cell::text(e.label())),
            _ => Cell::empty(),
        }
    }

    fn open(&self, c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(c.player_ref(c.w.deals.deals[id as usize].player))
    }

    fn note(&self, c: &Ctx, _p: &(), _f: &Value) -> Option<String> {
        (!c.observer()).then(|| "Negotiations between clubs are private until a deal is done.".to_string())
    }
}

// ---- international football --------------------------------------------------------------------

pub struct IntlMatches;

fn level_label(l: Level) -> &'static str {
    l.label()
}

fn kind_label(k: MatchKind) -> &'static str {
    match k {
        MatchKind::Friendly => "Friendly",
        MatchKind::Qualifier { .. } => "Qualifier",
        MatchKind::Group { .. } => "Group stage",
        MatchKind::Knockout { .. } => "Knockout",
    }
}

impl Source for IntlMatches {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("date", "Date", Fmt::Date, 100, &[G]),
            Col::new("level", "Level", Fmt::Text, 70, &[G]).left(),
            Col::new("home", "Home", Fmt::Text, 150, &[G]).left(),
            Col::new("score", "Score", Fmt::Text, 80, &[G]).center(),
            Col::new("away", "Away", Fmt::Text, 150, &[G]).left(),
            Col::new("kind", "Match", Fmt::Text, 110, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("date", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let nation = f_u32(f, "nation").map(NationId);
        let level = f_str(f, "level").and_then(|s| Level::ALL.iter().copied().find(|l| l.label().eq_ignore_ascii_case(s)));
        let tournament = f_u32(f, "tournament");
        c.w.intl
            .matches
            .iter()
            .enumerate()
            .filter(|(_, m)| nation.is_none_or(|n| m.home == n || m.away == n))
            .filter(|(_, m)| level.is_none_or(|l| m.level == l))
            .filter(|(_, m)| {
                tournament.is_none_or(|t| match m.kind {
                    MatchKind::Qualifier { tournament } | MatchKind::Group { tournament } | MatchKind::Knockout { tournament, .. } => tournament == t,
                    MatchKind::Friendly => false,
                })
            })
            .map(|(i, _)| i as u32)
            .collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let m = &c.w.intl.matches[id as usize];
        match col {
            "date" => Key::Num(f64::from(m.date.0) * 1_000_000.0 + f64::from(id)),
            "level" => Key::text(level_label(m.level)),
            "home" => Key::text(c.nation_name(m.home)),
            "away" => Key::text(c.nation_name(m.away)),
            "score" => Key::Num(f64::from(m.home_goals) + f64::from(m.away_goals)),
            "kind" => Key::text(kind_label(m.kind)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let m = &c.w.intl.matches[id as usize];
        match col {
            "date" => Cell::num(f64::from(m.date.0)),
            "level" => Cell::text(level_label(m.level)),
            "home" => nation_cell(c, m.home),
            "away" => nation_cell(c, m.away),
            "score" => {
                let mut s = format!("{}–{}", m.home_goals, m.away_goals);
                if let Some((a, b)) = m.pens {
                    s += &format!(" ({a}–{b} pens)");
                }
                Cell::text(s)
            }
            "kind" => Cell::text(kind_label(m.kind)),
            _ => Cell::empty(),
        }
    }
}

pub struct Tournaments;

fn stage_label(s: Stage) -> &'static str {
    match s {
        Stage::Qualifying => "Qualifying",
        Stage::Drawn => "Drawn",
        Stage::Groups => "Group stage",
        Stage::Knockout => "Knockout",
        Stage::Done => "Finished",
    }
}

impl Source for Tournaments {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("year", "Year", Fmt::Int, 70, &[G]),
            Col::new("name", "Tournament", Fmt::Text, 190, &[G]).left(),
            Col::new("stage", "Stage", Fmt::Text, 120, &[G]).left(),
            Col::new("winner", "Winner", Fmt::Text, 160, &[G]).left(),
            Col::new("runner_up", "Runner-up", Fmt::Text, 160, &[G]).left(),
            Col::new("best", "Best player", Fmt::Text, 180, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("year", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), _f: &Value) -> Vec<u32> {
        (0..c.w.intl.tournaments.len() as u32).collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let t = &c.w.intl.tournaments[id as usize];
        match col {
            "year" => Key::Num(f64::from(t.year) * 100.0 + f64::from(id)),
            "name" => Key::text(t.kind.label()),
            "stage" => Key::text(stage_label(t.stage)),
            "winner" => {
                if t.winner.is_some() {
                    Key::text(c.nation_name(t.winner))
                } else {
                    Key::None
                }
            }
            "runner_up" => {
                if t.runner_up.is_some() {
                    Key::text(c.nation_name(t.runner_up))
                } else {
                    Key::None
                }
            }
            "best" => {
                if t.best_player.is_some() {
                    Key::text(c.player_short(t.best_player))
                } else {
                    Key::None
                }
            }
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let t = &c.w.intl.tournaments[id as usize];
        match col {
            "year" => Cell::num(f64::from(t.year)),
            "name" => Cell::text(t.kind.label()),
            "stage" => Cell::text(stage_label(t.stage)),
            "winner" => nation_cell(c, t.winner),
            "runner_up" => nation_cell(c, t.runner_up),
            "best" => {
                if t.best_player.is_some() {
                    Cell::link(c.player_ref(t.best_player), c.player_name(t.best_player))
                } else {
                    Cell::empty()
                }
            }
            _ => Cell::empty(),
        }
    }
}

// ---- boards (private) --------------------------------------------------------------------------

pub struct Boards;

impl Source for Boards {
    type Prep = ();

    fn cols(&self, _c: &Ctx) -> Vec<Col> {
        vec![
            Col::new("club", "Club", Fmt::Text, 170, &[G]).left(),
            Col::new("owner", "Owner", Fmt::Text, 170, &[G]).left(),
            Col::new("kind", "Ownership", Fmt::Text, 130, &[G]).left(),
            Col::new("wealth", "Owner wealth", Fmt::Money, 110, &[G]),
            Col::new("ambition", "Ambition", Fmt::Int, 80, &[G]),
            Col::new("patience", "Patience", Fmt::Int, 80, &[G]),
            Col::new("meddling", "Meddling", Fmt::Int, 80, &[G]),
            Col::new("style", "Transfer style", Fmt::Text, 160, &[G]).left(),
            Col::new("youth", "Youth spend %", Fmt::Int, 100, &[G]),
            Col::new("projects", "Projects", Fmt::Int, 70, &[G]),
            Col::new("concern", "Main concern", Fmt::Text, 190, &[G]).left(),
        ]
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("wealth", true)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        if !c.observer() {
            return vec![];
        }
        let nation = f_u32(f, "nation").map(NationId);
        c.w.governance.keys().filter(|k| nation.is_none_or(|n| c.w.clubs[**k].nation == n)).map(|k| k.0).collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let g = &c.w.governance[&ClubId(id)];
        match col {
            "club" => Key::text(c.club_short(ClubId(id))),
            "owner" => Key::text(c.person_name(g.owner.person)),
            "kind" => Key::text(format!("{:?}", g.owner.kind)),
            "wealth" => Key::Num(g.owner.wealth as f64),
            "ambition" => Key::Num(f64::from(g.owner.ambition)),
            "patience" => Key::Num(f64::from(g.owner.patience)),
            "meddling" => Key::Num(f64::from(g.owner.meddling)),
            "style" => Key::text(g.policy.transfer_style.label()),
            "youth" => Key::Num(f64::from(g.policy.youth_investment)),
            "projects" => Key::Num(g.projects.len() as f64),
            "concern" => Key::Num(g.concerns.iter().map(|x| x.1.unsigned_abs()).max().map_or(0.0, f64::from)),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let g = &c.w.governance[&ClubId(id)];
        match col {
            "club" => club_cell(c, ClubId(id)),
            "owner" => {
                if g.owner.person.is_some() {
                    Cell::link(Ref::person(g.owner.person), c.person_name(g.owner.person))
                } else {
                    Cell::empty()
                }
            }
            "kind" => Cell::text(ownership_label(g.owner.kind)),
            "wealth" => Cell::num(g.owner.wealth as f64),
            "ambition" => Cell::num(f64::from(g.owner.ambition)),
            "patience" => Cell::num(f64::from(g.owner.patience)),
            "meddling" => Cell::num(f64::from(g.owner.meddling)),
            "style" => Cell::text(g.policy.transfer_style.label()),
            "youth" => Cell::num(f64::from(g.policy.youth_investment)),
            "projects" => Cell::num(g.projects.len() as f64),
            "concern" => match g.concerns.iter().max_by_key(|x| x.1.unsigned_abs()) {
                Some((k, v)) if *v != 0 => {
                    let cell = Cell::text(k.label());
                    if *v < 0 { cell.tone(Tone::Neg) } else { cell.tone(Tone::Pos) }
                }
                _ => Cell::empty(),
            },
            _ => Cell::empty(),
        }
    }

    fn open(&self, _c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(Ref::club(ClubId(id)))
    }

    fn note(&self, c: &Ctx, _p: &(), _f: &Value) -> Option<String> {
        (!c.observer()).then(|| "Boardroom numbers are private to the clubs.".to_string())
    }
}

pub fn ownership_label(k: pw_world::club::Ownership) -> &'static str {
    use pw_world::club::Ownership as O;
    match k {
        O::Private => "Private",
        O::MemberOwned => "Member owned",
        O::Benefactor => "Benefactor",
        O::InvestmentGroup => "Investment group",
        O::StateBacked => "State backed",
    }
}

// ---- sponsors ----------------------------------------------------------------------------------

pub struct Sponsors;

fn slot_label(s: pw_world::commerce::ClubSlot) -> &'static str {
    use pw_world::commerce::ClubSlot as S;
    match s {
        S::Shirt => "Shirt",
        S::Kit => "Kit",
        S::Stadium => "Stadium",
        S::Sleeve => "Sleeve",
    }
}

impl Source for Sponsors {
    type Prep = ();

    fn cols(&self, c: &Ctx) -> Vec<Col> {
        let mut v = vec![
            Col::new("club", "Club", Fmt::Text, 160, &[G]).left(),
            Col::new("brand", "Brand", Fmt::Text, 170, &[G]).left(),
            Col::new("slot", "Deal", Fmt::Text, 90, &[G]).left(),
            Col::new("end", "Until", Fmt::Date, 100, &[G]),
        ];
        if c.observer() {
            v.push(Col::new("fee", "Fee / year", Fmt::Money, 110, &[G]));
        }
        v
    }

    fn default_sort(&self) -> (&'static str, bool) {
        ("end", false)
    }

    fn prep(&self, _c: &Ctx, _f: &Value) {}

    fn ids(&self, c: &Ctx, _p: &(), f: &Value) -> Vec<u32> {
        let club = f_u32(f, "club").map(ClubId);
        c.w.commerce.club_deals.iter().enumerate().filter(|(_, d)| club.is_none_or(|x| d.club == x)).filter(|(_, d)| d.end >= c.w.date).map(|(i, _)| i as u32).collect()
    }

    fn key(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Key {
        let d = &c.w.commerce.club_deals[id as usize];
        match col {
            "club" => Key::text(c.club_short(d.club)),
            "brand" => Key::text(&c.w.commerce.brands[d.brand as usize].name),
            "slot" => Key::text(slot_label(d.slot)),
            "end" => Key::Num(f64::from(d.end.0)),
            "fee" => Key::Num(d.fee_year as f64),
            _ => Key::None,
        }
    }

    fn cell(&self, c: &Ctx, _p: &(), id: u32, col: &str) -> Cell {
        let d = &c.w.commerce.club_deals[id as usize];
        match col {
            "club" => club_cell(c, d.club),
            "brand" => Cell::text(&c.w.commerce.brands[d.brand as usize].name),
            "slot" => Cell::text(slot_label(d.slot)),
            "end" => Cell::num(f64::from(d.end.0)),
            "fee" => Cell::num(d.fee_year as f64),
            _ => Cell::empty(),
        }
    }

    fn open(&self, _c: &Ctx, _p: &(), id: u32) -> Option<Ref> {
        Some(Ref::club(_c.w.commerce.club_deals[id as usize].club))
    }
}
