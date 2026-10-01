//! `me.matchday`: one of your own matches as a day lived, not a scoreline. The trip, the squad, the captain, half-time, your minutes
//! and goals, the treatment room, the recovery day, the group chat and the papers, in order. Every step comes from what the world
//! recorded (the fixture, its report, the regions' map, the club's training days, the chats, the stories); a step it did not record
//! is left out rather than imagined.

use pw_core::{ClubId, Date, TeamId};
use pw_match::Ev;
use pw_sim::health::{self, DayKind};
use pw_world::chat::{Room, Said};
use pw_world::ecosystem::Climate;
use pw_world::event::{EventKind, ordinal};
use serde_json::Value;

use crate::contract::{MatchdayStep, MatchdayView};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Part, Ref};

/// Kilometres per unit of the regions' 0-100 map grid (the grid spans roughly the width of the country).

fn step(when: &str, kind: &str, parts: Vec<Part>) -> MatchdayStep {
    MatchdayStep { when: when.into(), kind: kind.into(), parts }
}

fn t(s: impl Into<String>) -> Vec<Part> {
    vec![Part::t(s)]
}

/// Distance in km between two clubs' home regions, when both are on the map.
fn km(c: &Ctx, a: ClubId, b: ClubId) -> Option<f32> {
    pw_sim::chronicle::club_km(c.w, a, b)
}

fn conditions(c: &Ctx, venue: ClubId, d: Date) -> Option<String> {
    let eco = &c.w.ext.ecosystem;
    let r = eco.regions.get(*eco.club_region.get(&venue)?)?;
    let m = d.month();
    let place = r.name.as_str();
    Some(match r.climate {
        Climate::HeavyMonsoon | Climate::HumidCoastal | Climate::NorthEast if (6..=9).contains(&m) => format!("Monsoon season in {place}: a heavy pitch and wet air"),
        Climate::HotDry if (4..=6).contains(&m) => format!("Pre-monsoon heat in {place}: water breaks and heavy legs"),
        Climate::HumidCoastal => format!("Coastal humidity in {place}"),
        Climate::HighAltitude => format!("At altitude in {place}: the air runs out sooner"),
        Climate::CoolHill if m == 12 || m <= 2 => format!("A cold evening in the hills of {place}"),
        _ => return None,
    })
}

fn travel(c: &Ctx, my_club: ClubId, venue: ClubId, played: bool) -> Option<Vec<Part>> {
    let city = c.w.clubs.get(venue).map(|k| k.city.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| c.club_name(venue));
    let Some(d) = km(c, my_club, venue) else {
        return Some(t(format!("{} to {city}", if played { "Travelled" } else { "Travel" })));
    };
    let km = (d / 10.0).round() * 10.0;
    Some(t(match (d, played) {
        (d, true) if d > 600.0 => format!("Flew {km:.0} km to {city} the day before and spent the night at the team hotel"),
        (d, false) if d > 600.0 => format!("A {km:.0} km flight to {city} the day before, then a night at the team hotel"),
        (d, true) if d > 150.0 => format!("A {:.0}-hour coach trip to {city} the day before, then the team hotel", (d / 55.0).round().max(3.0)),
        (d, false) if d > 150.0 => format!("A {:.0}-hour coach trip to {city} the day before, then the team hotel", (d / 55.0).round().max(3.0)),
        (_, true) => format!("A short bus ride to {city} on the morning of the match"),
        (_, false) => format!("A short bus ride to {city} on the morning of the match"),
    }))
}

pub fn matchday(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let me = c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world; match days belong to someone's life.".into()))?;
    let uid = crate::contract::request::<crate::contract::MatchReq>(args.clone())?.uid;
    let w = c.w;
    let p = w.people[me].player;
    let (_, f) = super::matchp::find(c, uid).ok_or_else(|| ApiError::NotFound("match".into()))?;
    let report = w.reports.get(&uid);
    let line = report.and_then(|r| r.line(p));
    // Yours if you were in its squad, or if it is your team's and has not been played yet.
    let my_team: TeamId = match line {
        Some(l) => if l.side == 0 { f.home } else { f.away },
        None if f.score.is_none() && (f.home == c.my_team() || f.away == c.my_team()) => c.my_team(),
        None if f.home == c.my_team() || f.away == c.my_team() => c.my_team(),
        None => return Err(ApiError::NotFound("not one of your matches".into())),
    };
    let home = f.home == my_team;
    let my_club = w.teams[my_team].club;
    let opp_club = w.teams[f.opponent(my_team)].club;
    let venue = if home { my_club } else { opp_club };
    let played = f.score.is_some();
    let mut steps: Vec<MatchdayStep> = Vec::new();

    // The day before / the morning.
    if home {
        let stadium = w.clubs.get(my_club).map(|k| k.stadium.clone()).filter(|s| !s.is_empty());
        steps.push(step("Morning", "travel", t(match stadium {
            Some(s) => format!("Home at {s}: report to the ground two hours before kick-off"),
            None => "Home: report to the ground two hours before kick-off".to_string(),
        })));
    } else if let Some(trip) = travel(c, my_club, venue, played) {
        let short = km(c, my_club, venue).is_some_and(|d| d <= 150.0);
        steps.push(step(if short { "Morning" } else { "The day before" }, "travel", trip));
    }
    if let Some(cond) = conditions(c, venue, f.date) {
        steps.push(step("Conditions", "weather", t(cond)));
    }
    // A result you chose to keep hidden stays hidden here too: the day stops at kick-off.
    if played && c.is_concealed(uid) {
        steps.push(step("Kick-off", "kickoff", t("You chose to keep this result hidden. Show it on the match page to relive the day")));
        return serde_json::to_value(MatchdayView { uid, played, steps }).map_err(|e| ApiError::Internal(e.to_string()));
    }
    if !played {
        steps.push(step("Matchday", "squad", t("The squad is named on the morning of the match")));
        let view = MatchdayView { uid, played, steps };
        return serde_json::to_value(view).map_err(|e| ApiError::Internal(e.to_string()));
    }
    let Some(r) = report else {
        steps.push(step("Full time", "result", t("Only the result was kept for this match")));
        return serde_json::to_value(MatchdayView { uid, played, steps }).map_err(|e| ApiError::Internal(e.to_string()));
    };
    let side = if home { 0u8 } else { 1u8 };

    // Selection.
    steps.push(step(
        "Team news",
        "squad",
        t(match line {
            Some(l) if l.started => format!("Named in the starting eleven{}", l.pos.map_or(String::new(), |p| format!(", at {}", p.code()))),
            Some(l) if l.minutes > 0 => format!("On the bench; you came on after {} minutes", l.on_at),
            Some(_) => "On the bench, and not called on".to_string(),
            None => "Not in the matchday squad".to_string(),
        }),
    ));
    let captain = w.teams.get(my_team).map(|t| t.captain).filter(|q| q.is_some());
    if let Some(cap) = captain.filter(|q| r.line(*q).is_some_and(|l| l.started)) {
        let cp = w.players.cold[cap].person;
        let parts = if cp == me { t("You led the team out as captain") } else { vec![Part::l(Ref::person(cp), c.person_name(cp)), Part::t(" led the team out")] };
        steps.push(step("Kick-off", "kickoff", parts));
    }

    // Half-time.
    let (hf, ha) = if home { r.ht } else { (r.ht.1, r.ht.0) };
    steps.push(step(
        "Half-time",
        "half",
        t(match hf.cmp(&ha) {
            std::cmp::Ordering::Greater => format!("{hf}-{ha} up at the break"),
            std::cmp::Ordering::Less => format!("{hf}-{ha} down at the break"),
            std::cmp::Ordering::Equal => format!("Level at {hf}-{ha} at the break"),
        }),
    ));

    // Your match.
    if let Some(l) = line.filter(|l| l.minutes > 0) {
        let mut bits: Vec<String> = Vec::new();
        let goals: Vec<u16> = r.events.iter().filter(|e| e.player == p && matches!(e.kind, Ev::Goal | Ev::PenaltyGoal)).map(|e| e.minute()).collect();
        if !goals.is_empty() {
            let mins: Vec<String> = goals.iter().map(|m| ordinal((*m).min(255) as u8)).collect();
            bits.push(format!("scored in the {} minute{}", join(&mins), if goals.len() > 1 { "s" } else { "" }));
        }
        if l.assists > 0 {
            bits.push(format!("{} assist{}", l.assists, if l.assists > 1 { "s" } else { "" }));
        }
        if r.events.iter().any(|e| e.player == p && matches!(e.kind, Ev::Red | Ev::SecondYellow)) {
            bits.push("sent off".into());
        } else if r.events.iter().any(|e| e.player == p && e.kind == Ev::Yellow) {
            bits.push("booked".into());
        }
        let time = if l.started && l.off_at > 0 && l.off_at < 90 { format!("Taken off after {} minutes", l.off_at) } else if l.started { "Played the whole match".into() } else { format!("{} minutes off the bench", l.minutes) };
        let mut text = time;
        if !bits.is_empty() {
            text.push_str(&format!("; {}", join(&bits)));
        }
        text.push_str(&format!(". Rated {:.1}", l.rating));
        if r.pom == p {
            text.push_str(", best on the pitch");
        }
        steps.push(step("Your match", "you", t(text)));
    }

    // Full time.
    let (gf, ga) = if home { (r.home_goals, r.away_goals) } else { (r.away_goals, r.home_goals) };
    let scorers: Vec<String> = r.events.iter().filter(|e| e.side == side && matches!(e.kind, Ev::Goal | Ev::PenaltyGoal)).map(|e| if e.player == p { "you".to_string() } else { w.player_short(e.player) }).collect();
    let mut ft = format!("{} {gf}-{ga}", match gf.cmp(&ga) {
        std::cmp::Ordering::Greater => "Won",
        std::cmp::Ordering::Less => "Lost",
        std::cmp::Ordering::Equal => "Drew",
    });
    if hf < ha && gf > ga {
        ft.push_str(", from behind");
    }
    if !scorers.is_empty() {
        ft.push_str(&format!(". Our goals: {}", join(&dedup(scorers))));
    }
    steps.push(step("Full time", "result", vec![Part::t(format!("{ft} against ")), Part::l(Ref::club(opp_club), c.club_name(opp_club))]));

    // After: treatment, the group chat, the papers, the next day.
    if let Some(e) = w.events.since(f.date).iter().take_while(|e| e.date <= f.date).find(|e| matches!(e.kind, EventKind::Injured { player, .. } if player == p))
        && let EventKind::Injured { injury, days, .. } = e.kind
    {
        let weeks = (u32::from(days) + 6) / 7;
        steps.push(step("After", "injury", t(format!("Into the treatment room: {}, about {weeks} week{} out", health::injury_name(w, injury).to_lowercase(), if weeks == 1 { "" } else { "s" }))));
    }
    if let Some(inbox) = w.ext.chats.of.get(&me)
        && let Some(chat) = inbox.chats.iter().find(|ch| ch.room == Room::Squad { club: my_club })
    {
        let first = (chat.posted as usize).saturating_sub(chat.msgs.len());
        for (i, m) in chat.msgs.iter().enumerate().filter(|(_, m)| matches!(m.said, Said::AfterMatch { uid: u, .. } | Said::WellDone { uid: u } if u == uid)).take(2) {
            let who = match m.from {
                pw_world::chat::Sender::Person(q) if w.people.get(q).is_some() => vec![Part::l(Ref::person(q), c.person_name(q)), Part::t(" in the squad chat: \u{201c}")],
                _ => t("In the squad chat: \u{201c}"),
            };
            let mut parts = who;
            parts.extend(super::chat::words(c, me, m, first + i, false));
            parts.push(Part::t("\u{201d}"));
            steps.push(step("That night", "chat", parts));
        }
    }
    for s in w.media.stories.iter().filter(|s| s.kind == pw_world::media::StoryKind::MatchReport && s.date >= f.date && s.date.days_until(f.date) > -3 && ((s.club == my_club && s.other_club == opp_club) || (s.club == opp_club && s.other_club == my_club))).take(1) {
        steps.push(step("The papers", "press", t(format!("{}: \u{201c}{}\u{201d}", pw_narrate::press::outlet_name(w, s), c.headline(s)))));
    }
    let next = f.date.add_days(1);
    if let Some(&k) = health::team_days(w, next).get(my_team.0 as usize).filter(|_| next <= w.date) {
        let words = match k {
            DayKind::AfterMatch => Some("Recovery the next morning: pool, stretching, and the video of the match"),
            DayKind::Rest => Some("A day off the next day"),
            DayKind::Training | DayKind::BeforeMatch => Some("Back in training the next day"),
            _ => None,
        };
        if let Some(words) = words {
            steps.push(step("Next day", "recovery", t(words)));
        }
    }
    serde_json::to_value(MatchdayView { uid, played, steps }).map_err(|e| ApiError::Internal(e.to_string()))
}

fn dedup(v: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in v {
        if let Some(i) = out.iter().position(|x| x == &s || x.starts_with(&format!("{s} ("))) {
            let n = out[i].trim_start_matches(&s).trim_matches(|c: char| !c.is_ascii_digit()).parse::<u32>().unwrap_or(1) + 1;
            out[i] = format!("{s} ({n})");
        } else {
            out.push(s);
        }
    }
    out
}

fn join(v: &[String]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        n => format!("{} and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}
