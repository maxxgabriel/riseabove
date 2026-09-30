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
pub fn player_ref(w: &World, p: PlayerId, on: Date) -> LRef {
    let person = w.person_of(p);
    let full = w.player_name(p);
    let short = w.names.get(person.last).to_string();
    let short = if short.is_empty() { full.clone() } else { short };
    let role = role_word(w.players.cold[p].best_pos.group());
    // His age when the text is about, not today: an old story keeps the age he was.
    let age = person.dob.age_on(on);
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

/// Any person as the public knows them: a player by name, age and position, staff by name and job.
fn person_ref(w: &World, p: pw_core::PersonId, on: Date) -> Option<LRef> {
    let person = w.people.get(p)?;
    if let Some(pl) = person.player.get() {
        return Some(player_ref(w, pl, on));
    }
    let st = person.staff.get()?;
    let name = w.staff_name(st);
    let short = name.split_whitespace().last().unwrap_or(&name).to_string();
    Some(LRef::new(&format!("person.{}", p.0), &name, &short).with_desc("title", &w.staff[st].role.label().to_lowercase()))
}

/// The club a player was with on a date, from his recorded spells (today's club only when the record says nothing). A story about last
/// season says which club he was at then, not where he is now.
fn club_at(w: &World, p: PlayerId, on: Date) -> ClubId {
    match w.history.spells.get(&p) {
        Some(v) if !v.is_empty() => v.iter().rev().find(|s| s.from <= on && s.to.is_none_or(|t| t >= on)).map_or(ClubId::NONE, |s| s.club),
        _ => w.players.hot[p].club,
    }
}

fn person_club(w: &World, p: pw_core::PersonId, on: Date) -> ClubId {
    let Some(person) = w.people.get(p) else { return ClubId::NONE };
    if let Some(pl) = person.player.get() {
        return club_at(w, pl, on);
    }
    person.staff.get().map_or(ClubId::NONE, |s| w.staff[s].club)
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
            let mut ev = LEvent::new("transfer.completed", date).ent("player", player_ref(w, player, date)).ent("from", club_ref(w, from)).ent("to", club_ref(w, to));
            if fee > 0 {
                ev = ev.money("fee", fee);
            }
            ev
        }
        // The length of a contract is between the player and the club; a published story says he has signed, not for how long.
        E::ContractSigned { player, club, renewal: true, .. } if club.is_some() => LEvent::new("contract.renewed", date).ent("player", player_ref(w, player, date)).ent("club", club_ref(w, club)),
        E::Released { player, club } if club.is_some() => LEvent::new("player.released", date).ent("player", player_ref(w, player, date)).ent("club", club_ref(w, club)),
        // The diagnosis and the time out are the club's business (the medical room), not the public's: a published story says a player is hurt,
        // not with what or for how long. Those facts are simply not in the event, so no text can carry them.
        E::Injured { player, .. } => {
            let club = club_at(w, player, date);
            let mut ev = LEvent::new("injury.suffered", date).ent("player", player_ref(w, player, date));
            if club.is_some() {
                ev = ev.ent("club", club_ref(w, club));
            }
            ev
        }
        E::ManagerSacked { staff, club } if club.is_some() => LEvent::new("manager.departed", date).ent("manager", manager_ref(w, staff)).ent("club", club_ref(w, club)).text("manner", "sacked"),
        E::ManagerAppointed { staff, club } if club.is_some() => LEvent::new("manager.appointed", date).ent("manager", manager_ref(w, staff)).ent("club", club_ref(w, club)),
        E::Promoted { comp, team } => LEvent::new("competition.promotion", date).ent("club", club_ref(w, team_club(w, team))).ent("competition", comp_ref(w, comp)),
        E::Relegated { comp, team } => LEvent::new("competition.relegation", date).ent("club", club_ref(w, team_club(w, team))).ent("competition", comp_ref(w, comp)),
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
        // A leak is somebody's word; a quote or a public matter is stated plainly.
        StoryKind::Unhappy if s.leaker.is_some() => Certainty::SourceClaim,
        _ => Certainty::Fact,
    }
}

fn speaker(s: &Story, w: &World, ev: &LEvent, cert: Certainty) -> Speaker {
    let eng = engine();
    let id = format!("outlet.{}", s.outlet.0);
    let mut sp = eng.witness(&id, "journalist", voice_of(w, s), ev);
    if cert != Certainty::Fact {
        let keys: Vec<String> = sp.knows.keys().cloned().collect();
        // What the outlet has behind it: sources for a claim, nothing checkable for talk or a guess.
        for k in keys {
            let know = if cert == Certainty::SourceClaim { Know::of(cert, "unnamed") } else { Know { certainty: cert, source: None } };
            sp.knows.insert(k, know);
        }
    }
    sp
}

// ------------------------------------------------------------------------------------------------- stories

#[derive(Clone)]
pub struct Text {
    pub headline: String,
    pub body: String,
    /// The ids of the frames used, for checking how varied the writing is.
    pub frames: Vec<String>,
}

thread_local! {
    /// The last story written. A page asks for a story's headline and then its body; the second is answered from here.
    static LAST: std::cell::RefCell<Option<((usize, u64, usize, u32, i32), Option<Text>)>> = const { std::cell::RefCell::new(None) };
}

/// The engine's article for a story, when it has one that does not say more than the story does.
pub fn story(w: &World, s: &Story) -> Option<Text> {
    let key = (std::ptr::from_ref(w) as usize, w.seed, w.media.stories.len(), s.id.0, w.date.0);
    if let Some(hit) = LAST.with(|l| l.borrow().as_ref().filter(|(k, _)| *k == key).map(|(_, t)| t.clone())) {
        return hit;
    }
    let t = try_story(w, s).ok();
    LAST.with(|l| *l.borrow_mut() = Some((key, t.clone())));
    t
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
    Ok(Text { headline, body: body.join(" "), frames: r.parts.iter().map(|p| p.frame.clone()).collect() })
}

fn story_event(w: &World, s: &Story) -> Option<LEvent> {
    match s.kind {
        // A rumour is a bid only when the story says a move is close; a club merely "monitoring" has bid for nobody.
        StoryKind::TransferRumour => {
            if s.other_club.is_none() || s.club.is_none() || s.player.is_none() {
                return None;
            }
            if s.claim < 75 {
                // Interest, at the stage the story says it has reached; never a bid.
                let stage = match s.claim {
                    0..=29 => "watching",
                    30..=54 => "keen",
                    _ => "preparing",
                };
                let mut ev = LEvent::new("transfer.interest", s.date).ent("buyer", club_ref(w, s.other_club)).ent("seller", club_ref(w, s.club)).ent("player", player_ref(w, s.player, s.date)).text("stage", stage);
                if s.fee > 0 && s.claim >= 55 {
                    ev = ev.money("fee", s.fee);
                }
                return Some(ev);
            }
            let mut ev = LEvent::new("transfer.bid_made", s.date).ent("buyer", club_ref(w, s.other_club)).ent("seller", club_ref(w, s.club)).ent("player", player_ref(w, s.player, s.date));
            if s.fee > 0 {
                ev = ev.money("fee", s.fee);
            }
            Some(ev)
        }
        StoryKind::MatchReport => match w.media.links.get(&s.id) {
            Some(pw_world::media::StoryLink::Fixture { home, away, hg, ag, star, .. }) if home.is_some() && away.is_some() => Some(
                {
                    let mut ev = LEvent::new("match.result", s.date).ent("home", club_ref(w, *home)).ent("away", club_ref(w, *away)).num("home_goals", i64::from(*hg)).num("away_goals", i64::from(*ag));
                    if star.is_some() {
                        ev = ev.ent("star", player_ref(w, *star, s.date));
                    }
                    ev
                },
            ),
            _ => None,
        },
        StoryKind::Milestone => match w.media.links.get(&s.id) {
            Some(pw_world::media::StoryLink::Milestone(k, count)) if s.player.is_some() => {
                use pw_world::event::MilestoneKind as M;
                let kind = match k {
                    M::ClubApps => "club_apps",
                    M::CareerGoals => "career_goals",
                    M::SeniorApps => "senior_apps",
                    M::Caps => "caps",
                };
                let mut ev = LEvent::new("milestone.reached", s.date).ent("player", player_ref(w, s.player, s.date)).text("kind", kind).num("count", i64::from(*count));
                if *k == M::ClubApps && s.club.is_some() {
                    ev = ev.ent("club", club_ref(w, s.club));
                }
                Some(ev)
            }
            Some(pw_world::media::StoryLink::Record(k, v)) if s.player.is_some() => {
                use pw_world::event::RecordKind as R;
                // A club's record signing or sale, or the highest fee anywhere: the fee is the record.
                if let Some(kind) = match k {
                    R::ClubRecordSigning => Some("signing"),
                    R::ClubRecordSale => Some("sale"),
                    R::WorldRecordFee => Some("world"),
                    _ => None,
                } && s.club.is_some()
                    && *v > 0
                {
                    return Some(LEvent::new("transfer.record", s.date).ent("player", player_ref(w, s.player, s.date)).ent("club", club_ref(w, s.club)).text("kind", kind).money("fee", *v));
                }
                // Only records a player can be said to have set or broken; a signing, a sale or a fee is not his to break.
                let (record, value) = match k {
                    R::ClubTopScorer if s.club.is_some() => (format!("{} scoring record", w.clubs[s.club].short_name), None),
                    R::ClubMostApps if s.club.is_some() => (format!("{} appearance record", w.clubs[s.club].short_name), None),
                    R::LeagueGoalsInSeason => ("league scoring record for a season".to_string(), Some(format!("{v} goals"))),
                    R::NationMostCaps => ("national appearance record".to_string(), None),
                    R::NationTopScorer => ("national scoring record".to_string(), None),
                    _ => return None,
                };
                let mut ev = LEvent::new("record.broken", s.date).ent("person", player_ref(w, s.player, s.date)).text("record", &record);
                if let Some(v) = value {
                    ev = ev.text("value", &v);
                }
                Some(ev)
            }
            _ => None,
        },
        StoryKind::Unhappy | StoryKind::Praise if s.player.is_some() => {
            let kind = if s.kind == StoryKind::Unhappy { "player.unhappy" } else { "player.praise" };
            let mut ev = LEvent::new(kind, s.date).ent("player", player_ref(w, s.player, s.date));
            if s.club.is_some() {
                ev = ev.ent("club", club_ref(w, s.club));
            }
            Some(ev)
        }
        StoryKind::AwardNews => match w.media.links.get(&s.id) {
            Some(pw_world::media::StoryLink::Award(a)) if s.player.is_some() => Some(LEvent::new("award.won", s.date).ent("player", player_ref(w, s.player, s.date)).text("award", &a.label())),
            _ => None,
        },
        StoryKind::Interview => match w.media.links.get(&s.id) {
            Some(pw_world::media::StoryLink::Quote(q)) => {
                let speaker = person_ref(w, q.speaker, s.date)?;
                let stance = match q.stance {
                    pw_world::media::Stance::Praise => "praise",
                    pw_world::media::Stance::Criticise => "criticise",
                    pw_world::media::Stance::Deflect => "deflect",
                    pw_world::media::Stance::Ambition => "ambition",
                    pw_world::media::Stance::Loyalty => "loyalty",
                    pw_world::media::Stance::Complain => "complain",
                    pw_world::media::Stance::Support => "support",
                    pw_world::media::Stance::Deny => "deny",
                };
                let mut ev = LEvent::new("interview.quote", s.date).ent("speaker", speaker).text("stance", stance);
                if let Some(about) = if q.about.is_some() && q.about != q.speaker { person_ref(w, q.about, s.date) } else { None } {
                    ev = ev.ent("about", about);
                }
                let club = person_club(w, q.speaker, s.date);
                if club.is_some() {
                    ev = ev.ent("club", club_ref(w, club));
                }
                Some(ev)
            }
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

// ------------------------------------------------------------------------------------------------- inbox

/// The engine's message for a decision put to a person: its subject and what has happened. The options stay the simulation's own (accept,
/// reject, ...): the engine only words the situation, so nothing offered here is something the world cannot do.
pub fn decision(w: &World, d: &pw_world::decision::Decision) -> Option<(String, String)> {
    use pw_world::decision::DecisionKind as K;
    if !enabled(w) || d.player.is_none() {
        return None;
    }
    let date = d.created;
    let ev = match &d.kind {
        K::Trial { club, .. } if club.is_some() => LEvent::new("academy.invitation", date)
            .with("to_player", pw_lang::Value::Bool(true))
            .ent("player", player_ref(w, d.player, date))
            .ent("academy", club_ref(w, *club))
            .text("offer_kind", "trial"),
        K::TransferTalks { club, fee } if club.is_some() => {
            let seller = w.players.hot[d.player].club;
            let mut ev = LEvent::new("transfer.bid_made", date).with("ours", pw_lang::Value::Bool(false)).ent("buyer", club_ref(w, *club)).ent("player", player_ref(w, d.player, date));
            if seller.is_some() {
                ev = ev.ent("seller", club_ref(w, seller));
            }
            if *fee > 0 {
                ev = ev.money("fee", *fee);
            }
            ev
        }
        _ => return None,
    };
    let sp = engine().witness("inbox", "staff", "club_official", &ev);
    let seed = pw_core::rng::hash_key(&[pw_core::rng::stream::NARRATION, u64::from(d.person.0), date.0 as u64, 0x1b0]);
    let mut tr = Tracker::new();
    let r = engine().render(&Request::new(&ev, &sp, "inbox", w.date, seed), &mut tr);
    if r.notes.iter().any(|n| n.contains("incomplete") || n.contains("required")) {
        return None;
    }
    let subject = r.part("subject")?.text.clone();
    let body: Vec<&str> = r.parts.iter().filter(|p| p.slot != "subject" && p.slot != "closing" && p.slot != "opening").map(|p| p.text.as_str()).collect();
    if body.is_empty() {
        return None;
    }
    Some((subject, body.join(" ")))
}

// ------------------------------------------------------------------------------------------------- social

fn account_voice(k: pw_world::socialnet::AccountKind) -> &'static str {
    use pw_world::socialnet::AccountKind as A;
    match k {
        A::Supporter | A::Hardcore | A::Ultra | A::Casual | A::Local | A::International => "fan_hype",
        A::Provocateur => "fan_sceptic",
        A::RumourMill => "tabloid",
        A::Stats => "analyst",
        A::ClubOfficial => "club_official",
        A::FanNews | A::AcademyWatcher | A::Neutral | A::Person => "sports_desk",
    }
}

/// A post that relays a story, in the account's own voice. It can be no firmer than the story was and no firmer than the post's own claim;
/// anything else about the post (opinion, banter, chants) stays with the personality-driven text.
pub fn post(w: &World, p: &pw_world::socialnet::Post) -> Option<String> {
    use pw_world::media::ClaimType;
    use pw_world::socialnet::{Concept, Frame};
    if !enabled(w) || p.concept != Concept::Relay {
        return None;
    }
    let Frame::Story { story } = p.frame else { return None };
    let st = w.media.stories.get(story)?;
    let ev = story_event(w, st)?;
    let post_cert = match p.claim {
        ClaimType::Fact => Certainty::Fact,
        ClaimType::Report => Certainty::SourceClaim,
        ClaimType::Rumour => Certainty::Rumour,
        ClaimType::Speculation | ClaimType::Opinion => Certainty::Speculation,
        _ => return None,
    };
    let cert = Certainty::combine([certainty_of(st), post_cert]);
    let a = w.net.accounts.get(p.author as usize)?;
    let mut sp = engine().witness(&format!("account.{}", p.author), "fan", account_voice(a.kind), &ev);
    if cert != Certainty::Fact {
        let keys: Vec<String> = sp.knows.keys().cloned().collect();
        for k in keys {
            let know = if cert == Certainty::SourceClaim { Know::of(cert, "unnamed") } else { Know { certainty: cert, source: None } };
            sp.knows.insert(k, know);
        }
    }
    let seed = pw_core::rng::hash_key(&[pw_core::rng::stream::NARRATION, u64::from(p.id), 0x50c]);
    let mut tr = Tracker::new();
    let r = engine().render(&Request::new(&ev, &sp, "social", p.date, seed), &mut tr);
    if r.notes.iter().any(|n| n.contains("incomplete") || n.contains("required")) {
        return None;
    }
    let text: Vec<&str> = r.parts.iter().filter(|x| x.slot != "tag").map(|x| x.text.as_str()).collect();
    if text.is_empty() { None } else { Some(text.join(" ")) }
}
