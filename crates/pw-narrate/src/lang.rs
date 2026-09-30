//! The bridge from world events and stories to the language engine (`pw-lang`).
//!
//! The engine writes text from one semantic event, filtered through what the speaker knows and how sure they are. This module is the only
//! place that reads the world for it: it turns a real event into an engine event carrying only **public** facts (names, ages, positions,
//! fees the outlet reports), decides the speaker's certainty from what the story actually rests on, and hands back text. It never supplies a
//! fact the world did not record and never a firmer certainty than the story's own claim, so the engine's guarantees carry through.
//!
//! Text is produced only where the engine has an event for it and the world's money is rupees (a world with an ecosystem); everywhere
//! else the callers fall back to the older templates. Text is a pure function of world state and the story, so it reads the same each time.

use std::sync::OnceLock;

use pw_core::{ClubId, CompId, Date, PlayerId, PosGroup, TeamId};
use pw_lang::{Certainty, Engine, Event as LEvent, Know, Ref as LRef, Request, Speaker, Tracker};
use pw_world::event::EventKind as E;
use pw_world::media::{OutletKind, Story, StoryKind};
use pw_world::World;

fn engine() -> &'static Engine {
    static E: OnceLock<Engine> = OnceLock::new();
    E.get_or_init(Engine::builtin)
}

/// Whether the engine writes for this world. Its money is rupees and its examples are Indian football; other worlds keep their own templates.
pub fn enabled(w: &World) -> bool {
    w.ext.ecosystem.is_configured()
}

// ------------------------------------------------------------------------------------------------- references

pub fn club_ref(w: &World, c: ClubId) -> LRef {
    let club = &w.clubs[c];
    let short = if club.short_name.is_empty() { club.name.clone() } else { club.short_name.clone() };
    LRef::new(&format!("club.{}", c.0), &club.name, &short)
}

fn role_word(g: PosGroup) -> &'static str {
    match g {
        PosGroup::Gk => "goalkeeper",
        PosGroup::Def => "defender",
        PosGroup::Mid => "midfielder",
        PosGroup::Att => "forward",
    }
}

/// A player as the public knows him: name, age and position (never ability).
pub fn player_ref(w: &World, p: PlayerId) -> LRef {
    let person = w.person_of(p);
    let full = w.player_name(p);
    let short = w.names.get(person.last).to_string();
    let short = if short.is_empty() { full.clone() } else { short };
    let role = role_word(w.players.cold[p].best_pos.group());
    let age = w.age(p);
    let mut r = LRef::new(&format!("person.{}", w.players.cold[p].person.0), &full, &short);
    r = r.with_desc("role", role);
    if (15..=45).contains(&age) {
        r = r.with_desc("age_role", &format!("{age}-year-old {role}"));
    }
    r
}

fn manager_ref(w: &World, s: pw_core::StaffId) -> LRef {
    let name = w.staff_name(s);
    let short = name.split_whitespace().last().unwrap_or(&name).to_string();
    LRef::new(&format!("person.{}", w.staff[s].person.0), &name, &short).with_desc("title", "manager")
}

fn comp_ref(w: &World, c: CompId) -> LRef {
    let comp = &w.comps[c];
    LRef::new(&format!("comp.{}", c.0), &comp.name, if comp.short_name.is_empty() { &comp.name } else { &comp.short_name })
}

fn team_club(w: &World, t: TeamId) -> ClubId {
    w.teams[t].club
}

// ------------------------------------------------------------------------------------------------- events

/// The engine event for a world event, when the engine has one and the world recorded every fact it needs.
pub fn event(w: &World, k: &E, date: Date) -> Option<LEvent> {
    Some(match *k {
        E::Transfer { player, from, to, fee } if from.is_some() && to.is_some() => {
            let mut ev = LEvent::new("transfer.completed", date).ent("player", player_ref(w, player)).ent("from", club_ref(w, from)).ent("to", club_ref(w, to));
            if fee > 0 {
                ev = ev.money("fee", fee);
            }
            ev
        }
        E::ContractSigned { player, club, until, renewal: true, .. } if club.is_some() => {
            let years = (until.0 - date.0).max(0) / 365;
            LEvent::new("contract.renewed", date).ent("player", player_ref(w, player)).ent("club", club_ref(w, club)).num("contract_years", i64::from(years.max(1)))
        }
        E::Released { player, club } if club.is_some() => LEvent::new("player.released", date).ent("player", player_ref(w, player)).ent("club", club_ref(w, club)),
        E::Injured { player, injury, days } => {
            let club = w.players.hot[player].club;
            let name = if injury > 0 { w.data.injuries.get(usize::from(injury - 1)).map(|d| d.name.to_lowercase()) } else { None };
            let mut ev = LEvent::new("injury.suffered", date).ent("player", player_ref(w, player)).num("weeks_out", i64::from(days.div_ceil(7).max(1)));
            if club.is_some() {
                ev = ev.ent("club", club_ref(w, club));
            }
            if let Some(n) = name {
                ev = ev.text("injury", &n);
            }
            ev
        }
        E::ManagerSacked { staff, club } if club.is_some() => LEvent::new("manager.departed", date).ent("manager", manager_ref(w, staff)).ent("club", club_ref(w, club)).text("manner", "sacked"),
        E::ManagerAppointed { staff, club } if club.is_some() => LEvent::new("manager.appointed", date).ent("manager", manager_ref(w, staff)).ent("club", club_ref(w, club)),
        E::Promoted { comp, team } => LEvent::new("competition.promotion", date).ent("club", club_ref(w, team_club(w, team))).ent("competition", comp_ref(w, comp)),
        E::Relegated { comp, team } => LEvent::new("competition.relegation", date).ent("club", club_ref(w, team_club(w, team))).ent("competition", comp_ref(w, comp)),
        E::BidRejected { player, club, fee } if club.is_some() => {
            let seller = w.players.hot[player].club;
            if seller.is_none() {
                return None;
            }
            LEvent::new("transfer.bid_rejected", date).ent("buyer", club_ref(w, club)).ent("seller", club_ref(w, seller)).ent("player", player_ref(w, player)).money("fee", fee)
        }
        _ => return None,
    })
}

// ------------------------------------------------------------------------------------------------- speakers

/// The voice an outlet writes in.
fn voice_of(w: &World, s: &Story) -> &'static str {
    if s.outlet.is_none() {
        return "wire_reporter";
    }
    match w.media.outlets[s.outlet].kind {
        OutletKind::National => "sports_desk",
        OutletKind::Local => "regional_daily",
        OutletKind::Tabloid => "tabloid",
        OutletKind::Broadcaster => "wire_reporter",
        OutletKind::DataSite => "analyst",
        OutletKind::FanChannel => "fan_hype",
    }
}

/// How firmly an outlet may state what it published: what the story's own claim strength supports, never more.
fn certainty_of(s: &Story) -> Certainty {
    match s.kind {
        StoryKind::TransferRumour | StoryKind::Leak => match s.claim {
            75..=100 => Certainty::SourceClaim,
            30..=74 => Certainty::Rumour,
            _ => Certainty::Speculation,
        },
        _ => Certainty::Fact,
    }
}

fn speaker(s: &Story, w: &World, ev: &LEvent, cert: Certainty) -> Speaker {
    let eng = engine();
    let id = format!("outlet.{}", s.outlet.0);
    let mut sp = eng.witness(&id, "journalist", voice_of(w, s), ev);
    if cert != Certainty::Fact {
        let keys: Vec<String> = sp.knows.keys().cloned().collect();
        for k in keys {
            sp.knows.insert(k, Know::of(cert, "sources"));
        }
    }
    sp
}

// ------------------------------------------------------------------------------------------------- stories

pub struct Text {
    pub headline: String,
    pub body: String,
}

/// The engine's article for a story, when it has one that does not say more than the story does.
pub fn story(w: &World, s: &Story) -> Option<Text> {
    try_story(w, s).ok()
}

/// As [`story`], saying why the engine did not write it (no event for it, or the engine reported the rendering incomplete).
pub fn try_story(w: &World, s: &Story) -> Result<Text, String> {
    if !enabled(w) {
        return Err("engine not enabled for this world".into());
    }
    let ev = story_event(w, s).ok_or_else(|| "no engine event for this story".to_string())?;
    let cert = certainty_of(s);
    let sp = speaker(s, w, &ev, cert);
    let seed = pw_core::rng::hash_key(&[pw_core::rng::stream::NARRATION, u64::from(s.id.0), 0x1a9]);
    let mut tr = Tracker::new();
    let r = engine().render(&Request::new(&ev, &sp, "news", s.date, seed), &mut tr);
    // An optional item with no frame is skipped and noted; only a missing required item makes the article incomplete.
    if r.notes.iter().any(|n| n.contains("incomplete") || n.contains("required") || n.contains("unknown channel")) {
        return Err(format!("{} {}: {}", ev.kind, s.id.0, r.notes.join("; ")));
    }
    let headline = r.part("headline").ok_or("no headline")?.text.clone();
    let body: Vec<&str> = r.parts.iter().filter(|p| p.slot != "headline").map(|p| p.text.as_str()).collect();
    Ok(Text { headline, body: body.join(" ") })
}

fn story_event(w: &World, s: &Story) -> Option<LEvent> {
    match s.kind {
        // A rumour is a bid only when the story says a move is close; a club merely "monitoring" has bid for nobody.
        StoryKind::TransferRumour => {
            if s.claim < 75 || s.other_club.is_none() || s.club.is_none() || s.player.is_none() {
                return None;
            }
            let mut ev = LEvent::new("transfer.bid_made", s.date).ent("buyer", club_ref(w, s.other_club)).ent("seller", club_ref(w, s.club)).ent("player", player_ref(w, s.player));
            if s.fee > 0 {
                ev = ev.money("fee", s.fee);
            }
            Some(ev)
        }
        StoryKind::MatchReport => match w.media.links.get(&s.id) {
            Some(pw_world::media::StoryLink::Fixture { home, away, hg, ag, .. }) if home.is_some() && away.is_some() => Some(
                LEvent::new("match.result", s.date).ent("home", club_ref(w, *home)).ent("away", club_ref(w, *away)).num("home_goals", i64::from(*hg)).num("away_goals", i64::from(*ag)),
            ),
            _ => None,
        },
        StoryKind::TransferNews | StoryKind::ManagerChange | StoryKind::Injury | StoryKind::Season | StoryKind::Contract => underlying(w, s),
        _ => None,
    }
}

/// The world event a story is about: the story's own publication event points at it through its causes (looking a couple of steps back).
fn underlying(w: &World, s: &Story) -> Option<LEvent> {
    let mut frontier = vec![s.event];
    for _ in 0..3 {
        let mut next = Vec::new();
        for id in frontier {
            let Some(e) = w.events.get(id) else { continue };
            if let Some(ev) = event(w, &e.kind, e.date) {
                return Some(ev);
            }
            next.extend(e.causes.iter().filter_map(|c| if let pw_world::event::Cause::Event(x) = c { Some(*x) } else { None }));
        }
        frontier = next;
    }
    None
}
