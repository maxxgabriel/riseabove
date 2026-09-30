//! The inhabited person's private view of themselves and the people around them: how they feel and
//! why, their week and money, who they know, what has been promised, what they have heard, what is
//! said about them, their agent, and their own notes and goals. Every line comes from recorded
//! world state; nothing here reads another person's hidden numbers.

use pw_career::{Goal, GoalKind};
use pw_core::PersonId;
use pw_narrate::fmt::{feeling, level};
use pw_world::beliefs::{BeliefKind, Channel};
use pw_world::life::MoodFactor;
use pw_world::{PlayerStatus, PromiseState};
use serde_json::{Value, json};

use super::me::named;
use crate::ctx::Ctx;
use crate::contract::{Evidence, PeopleView, RelationshipRow, RumourRow, RumoursView};
use crate::model::{ApiError, ApiResult, Named, Ref, Tone, level_band, pull, sureness};
use crate::session::Session;

fn need(c: &Ctx) -> ApiResult<PersonId> {
    c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world. Inhabit someone to use this page.".into()))
}

fn channel_text(c: &Ctx, ch: &Channel) -> String {
    let w = c.w;
    match ch {
        Channel::Witnessed => "you saw it yourself".into(),
        Channel::Told(p) => c.person_name(*p),
        Channel::Agent(p) => format!("your agent {}", c.person_name(*p)),
        Channel::Club(cl) => c.club_name(*cl),
        Channel::Media(s) => pw_narrate::press::outlet_name(w, &w.media.stories[*s]),
        Channel::Public => "common knowledge".into(),
        Channel::Inferred => "your own reading of things".into(),
    }
}

/// One entry per cause: the world may record the same cause twice (the manager, and the mood of one's group), and the reader
/// should not be told the same thing twice.
pub(crate) fn merged(mood: &pw_world::life::Mood) -> Vec<(MoodFactor, i8)> {
    let mut m: Vec<(MoodFactor, i8)> = Vec::new();
    for &(f, x) in mood.iter() {
        match m.iter_mut().find(|(g, _)| *g == f) {
            Some(e) => e.1 = e.1.saturating_add(x),
            None => m.push((f, x)),
        }
    }
    m
}

fn mood_list(mood: &pw_world::life::Mood) -> Vec<Value> {
    let mut m = merged(mood);
    m.sort_by_key(|(_, x)| std::cmp::Reverse(x.unsigned_abs()));
    m.iter().filter(|(_, x)| *x != 0).map(|(f, x)| json!({"text": format!("{} {}", feeling(*x), f.label()), "factor": f.label(), "pull": pull(*x)})).collect()
}

/// What a coach's selection forecast means, in words: they are telling you how it looks, not a probability to the percent.
fn start_outlook(start_pct: u8) -> &'static str {
    match start_pct {
        80.. => "expects you to start",
        60..=79 => "thinks you will probably start",
        40..=59 => "sees it as open whether you start",
        20..=39 => "thinks you will probably not start",
        _ => "does not expect you to start",
    }
}

/// How you feel, what your coaches have told you, and how others describe you.
pub fn self_view(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    let life = &w.lives[me];
    let pe = &w.people[me];
    let mut told: Vec<Value> = Vec::new();
    for b in w.beliefs.about(me, me) {
        let src = channel_text(c, &b.channel);
        let text = match b.kind {
            BeliefKind::Assessment { ca_lo, ca_hi, ceiling } => Some(format!("{src} rates your level at {}-{} and sees a ceiling of {} out of 5.", ca_lo / 10, ca_hi / 10, ceiling)),
            BeliefKind::ManagerRating { manager, stars } => Some(format!("{} gives you {} out of 5.", c.person_name(manager), stars)),
            BeliefKind::SelectionOutlook { start_pct } => Some(format!("{src} {} the next match.", start_outlook(start_pct))),
            _ => None,
        };
        if let Some(text) = text {
            told.push(json!({"text": text, "date": b.date.0, "sureness": sureness(b.confidence)}));
        }
    }
    told.sort_by(|a, b| b["date"].as_i64().cmp(&a["date"].as_i64()));
    let p = pe.player;
    let career = (p.is_some()).then(|| {
        let cold = &w.players.cold[p];
        json!({
            "apps": cold.senior_apps, "goals": cold.senior_goals, "caps": cold.caps, "position": cold.best_pos.code(),
            "status": c.status_label(p), "clubs": w.history.spells.get(&p).map_or(0, Vec::len),
        })
    });
    Ok(json!({
        "name": c.person_name(me), "age": c.age(me), "nation": named(Ref::nation(pe.nation), c.nation_name(pe.nation)),
        "personality": pe.hidden.personality_label(), "hint": pw_career::views::personality_hint(w, me),
        "mood": mood_list(&life.morale_why), "wellbeing": mood_list(&life.wellbeing_why), "told": told, "career": career,
        "stress": level_band(100 - life.stress), "sleep": level_band(life.sleep), "fulfilment": level_band(life.fulfilment),
    }))
}

// ---- life ------------------------------------------------------------------------------------------

/// Someone else's private life is the observer's to see; an inhabited person sees only their own.
pub fn life_of(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = args.get("id").and_then(Value::as_u64).map(|n| PersonId(n as u32));
    let who = match (c.me(), id) {
        (Some(me), None) => me,
        (Some(me), Some(x)) if x == me => me,
        (Some(_), Some(_)) => return Err(ApiError::Unauthorized("You cannot see into someone else's private life.".into())),
        (None, Some(x)) => x,
        (None, None) => return Err(ApiError::Bad("missing person".into())),
    };
    if c.w.people.get(who).is_none() {
        return Err(ApiError::NotFound(format!("person {}", who.0)));
    }
    life_for(c, who)
}

pub fn life(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    life_for(c, me)
}

fn life_for(c: &Ctx, me: PersonId) -> ApiResult<Value> {
    let w = c.w;
    let l = &w.lives[me];
    let aff = w.affairs.of(me);
    let f = &l.finances;
    let partner = l.partner().map(|pt| {
        json!({
            "who": named(Ref::person(pt.person), c.person_name(pt.person)), "status": pt.status.label(), "since": pt.since.0,
            "bond": level_band(pt.bond),
            "lives": if pt.lives != l.home { Some(c.nation_name(pt.lives)) } else { None },
            "occupation": w.lives[pt.person].occupation.label(),
        })
    });
    let par = &l.household.parents;
    let r = l.routine;
    let routine: Vec<Value> = [
        ("rest", "Rest", r.rest),
        ("recovery", "Recovery", r.recovery),
        ("family", "Family", r.family),
        ("partner", "Partner", r.partner),
        ("social", "Friends and social life", r.social),
        ("study", "Study", r.study),
        ("hobbies", "Hobbies", r.hobbies),
        ("media", "Media and public life", r.media),
        ("nightlife", "Nights out", r.nightlife),
        ("language", "Learning the language", r.language),
    ]
    .iter()
    .map(|(k, label, h)| json!({"key": k, "label": label, "hours": h}))
    .collect();
    let languages: Vec<Value> = l.languages.iter().map(|(n, fl)| json!({"nation": named(Ref::nation(*n), c.nation_name(*n)), "level": level_band(*fl)})).collect();
    let home = aff.map(|a| a.home);
    let studying = aff.and_then(|a| a.studying).map(|s| json!({"course": s.course.label(), "done": s.done, "effort": s.course.effort()}));
    let work = aff.and_then(|a| a.work).map(|wk| json!({"path": wk.path.label(), "income": wk.income, "standing": level(wk.standing), "since": wk.since.0}));
    Ok(json!({
        "home": {"nation": named(Ref::nation(l.home), c.nation_name(l.home)), "since": l.home_since.0, "kind": home.map(|h| format!("{:?}", h.kind).to_lowercase()), "quality": home.map(|h| h.quality)},
        "languages": languages, "partner": partner, "children": l.household.children, "siblings": l.household.siblings,
        "parents": {"alive": par.alive, "nation": if par.nation.is_some() { Some(c.nation_name(par.nation)) } else { None }, "health": level(par.health), "closeness": level(par.closeness)},
        "money": {
            "savings": f.savings, "debt": f.debt, "income": f.income, "spending": f.spending, "family_support": f.family_support,
            "lifestyle": f.lifestyle.label(), "invested": aff.map_or(0, |a| a.invested),
        },
        "routine": {"rows": routine, "total": r.total(), "budget": pw_world::life::Routine::BUDGET},
        "occupation": l.occupation.label(), "education": l.education,
        "studying": studying, "qualifications": aff.map(|a| a.quals.iter().map(|(co, d)| json!({"label": co.label(), "date": d.0})).collect::<Vec<_>>()).unwrap_or_default(),
        "helpers": aff.map(|a| a.staff.iter().map(|h| json!({"label": h.helper.label(), "quality": h.quality, "cost": h.cost})).collect::<Vec<_>>()).unwrap_or_default(),
        "giving": aff.map(|a| json!({"pct": a.giving_pct, "community": a.community, "foundation": a.foundation})),
        "work": work,
        "open_to_dating": w.intents.dating.get(&me).copied(),
        "wellbeing": mood_list(&l.wellbeing_why), "morale": mood_list(&l.morale_why),
    }))
}

// ---- people -----------------------------------------------------------------------------------------

fn role_of(c: &Ctx, me: PersonId, p: PersonId) -> String {
    let w = c.w;
    let x = &w.people[p];
    if x.staff.is_some() && w.staff[x.staff].employed() {
        return format!("{}, {}", w.staff[x.staff].role.label(), c.club_short(w.staff[x.staff].club));
    }
    if x.player.is_some() {
        let h = &w.players.hot[x.player];
        if h.status == PlayerStatus::Active {
            return format!("Player, {}", c.club_short(h.club));
        }
    }
    if w.media.journalists.contains_key(&p) {
        return "Journalist".into();
    }
    if w.agents.list.iter().any(|a| a.person == p) {
        return "Agent".into();
    }
    if w.lives[me].partner().is_some_and(|pt| pt.person == p) {
        return "Partner".into();
    }
    String::new()
}

/// The people in your life, as you feel about them, and the memory behind it.
pub fn people(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    let today = w.date;
    let grudge = pw_world::social::grudge_factor(&w.people[me]);
    let mut rels: Vec<(PersonId, pw_world::Rel)> = w.social.relations_of(me).collect();
    rels.sort_by_key(|(p, r)| (std::cmp::Reverse(i32::from(r.affinity).abs() + (i32::from(r.trust) - 50).abs()), *p));
    let ev = |m: &pw_world::social::Memory| Evidence { text: format!("They {}", m.kind.text()), date: m.date.0 };
    let people: Vec<RelationshipRow> = rels
        .into_iter()
        .take(40)
        .map(|(p, r)| {
            // Recent things that shaped it: the evidence, not the score (locked design 8.7).
            let mut mem: Vec<&pw_world::social::Memory> = w.social.recall(me, p).collect();
            mem.sort_by_key(|m| std::cmp::Reverse(m.date));
            RelationshipRow {
                who: Named::new(Ref::person(p), c.person_name(p)),
                role: role_of(c, me, p),
                label: r.label().into(),
                tone: match r.affinity {
                    25.. => Tone::Pos,
                    ..=-25 => Tone::Neg,
                    _ => Tone::Warn,
                },
                trust: level(r.trust).to_string(),
                respect: level(r.respect).to_string(),
                since: r.since.0,
                last: r.last.0,
                why: w.social.defining_memory(me, p, today, grudge).map(ev),
                evidence: mem.iter().take(3).map(|m| ev(m)).collect(),
            }
        })
        .collect();
    typed(&PeopleView { people })
}

pub fn promises(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    // Both indexes: what was promised to me and what I promised. A promise to myself is listed once.
    let mut rows: Vec<Value> = w
        .social
        .promises_to(me)
        .chain(w.social.promises_by(me).filter(|pr| pr.to != me))
        .map(|pr| {
            let mine = pr.from == me;
            let other = if mine { pr.to } else { pr.from };
            let progress = match pr.kind {
                pw_world::PromiseKind::Minutes { share } if pr.team_minutes > 0 => Some(json!({"actual": pr.player_minutes as f32 / pr.team_minutes as f32, "promised": share})),
                _ => None,
            };
            json!({
                "id": pr.id, "mine": mine, "with": named(Ref::person(other), c.person_name(other)), "text": pr.kind.text(),
                "made": pr.made.0, "due": pr.due.0, "progress": progress,
                "state": match pr.state { PromiseState::Open => "open", PromiseState::Kept => "kept", PromiseState::Broken => "broken", PromiseState::Void => "void" },
                "days_left": pr.due.0 - w.date.0,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        let open = |v: &Value| v["state"] == "open";
        open(b).cmp(&open(a)).then_with(|| b["made"].as_i64().cmp(&a["made"].as_i64()))
    });
    Ok(json!({"promises": rows}))
}

pub fn rumours(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    let mut bs: Vec<_> = w.beliefs.of(me).collect();
    bs.sort_by_key(|b| std::cmp::Reverse(b.date));
    let mut rumours: Vec<RumourRow> = Vec::new();
    for b in bs {
        let via = channel_text(c, &b.channel);
        let row = |kind: &str, via: String, text: String, club: Option<Named>, fee: Option<i64>| RumourRow { kind: kind.into(), date: b.date.0, via, sureness: sureness(b.confidence).into(), text, club, fee };
        match b.kind {
            BeliefKind::ClubInterested { club } => rumours.push(row("interest", via, format!("{} are interested in you.", c.club_name(club)), Some(Named::new(Ref::club(club), c.club_name(club))), None)),
            BeliefKind::BidMade { club, fee } => rumours.push(row("bid", via, format!("{} made a bid for you.", c.club_name(club)), Some(Named::new(Ref::club(club), c.club_name(club))), Some(fee as i64))),
            BeliefKind::Rumour { story } => {
                let s = &w.media.stories[story];
                let text = c.headline(s);
                // The same rumour run by two outlets on one day is heard once.
                if rumours.iter().any(|r| r.kind == "rumour" && r.date == b.date.0 && r.text == text) {
                    continue;
                }
                rumours.push(row("rumour", pw_narrate::press::outlet_name(w, s), text, None, None));
            }
            _ => {}
        }
    }
    typed(&RumoursView { rumours })
}

/// A contract payload, serialised.
fn typed<T: serde::Serialize>(v: &T) -> ApiResult<Value> {
    serde_json::to_value(v).map_err(|e| ApiError::Internal(e.to_string()))
}

/// What is said about you, and how the fans see you.
pub fn press(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    let club = w.club_of_person(me);
    let mut seen_headlines: std::collections::HashSet<(i32, String)> = std::collections::HashSet::new();
    let stories: Vec<Value> = w
        .media
        .stories
        .iter()
        .rev()
        .filter(|s| s.person == me || (club.is_some() && (s.club == club || s.other_club == club)))
        .filter(|s| seen_headlines.insert((s.date.0, c.headline(s))))
        .take(30)
        .map(|s| {
            json!({
                "id": s.id.0, "date": s.date.0, "outlet": pw_narrate::press::outlet_name(w, s), "headline": c.headline(s),
                "about_you": s.person == me,
            })
        })
        .collect();
    let reactions: Vec<Value> = w
        .media
        .reactions
        .iter()
        .rev()
        .filter(|r| r.about == me || (club.is_some() && r.club == club))
        .take(20)
        .map(|r| json!({"date": r.date.0, "text": pw_narrate::press::reaction(w, r), "sentiment": r.sentiment}))
        .collect();
    let fans: Vec<Value> = w
        .media
        .fans
        .iter()
        .filter(|((_, p), _)| *p == me)
        .map(|((cl, _), f)| {
            json!({
                "club": named(Ref::club(*cl), c.club_name(*cl)), "label": f.label(), "score": f.score,
                "reasons": f.reasons.iter().map(|r| r.0.label()).collect::<Vec<_>>(),
            })
        })
        .collect();
    let image = w.media.image.get(&me).copied().unwrap_or(0);
    let quotes: Vec<Value> = w
        .pressroom
        .quotes_by(me)
        .rev()
        .take(20)
        .map(|q| {
            let about = if q.about.is_some() && q.about != me { Some(named(Ref::person(q.about), c.person_name(q.about))) } else { None };
            json!({"id": q.id, "date": q.date.0, "stance": pw_narrate::press::stance_label(q.stance), "about": about, "at_conference": q.topic.is_some()})
        })
        .collect();
    Ok(json!({
        "stories": stories, "reactions": reactions, "fans": fans, "quotes": quotes,
        "image": if image > 100 { "positive" } else if image < -100 { "negative" } else { "neutral" },
    }))
}

pub fn story(c: &Ctx, args: &Value) -> ApiResult<Value> {
    need(c)?;
    let id = args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing story".into()))? as u32;
    let sid = pw_core::StoryId(id);
    let w = c.w;
    let s = w.media.stories.get(sid).ok_or_else(|| ApiError::NotFound("story".into()))?;
    Ok(json!({
        "id": id, "date": s.date.0, "outlet": pw_narrate::press::outlet_name(w, s), "headline": c.headline(s),
        "body": c.story_body(s),
    }))
}

// ---- agent ------------------------------------------------------------------------------------------

pub fn agent(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    let p = w.people[me].player;
    if p.is_none() {
        return Ok(json!({"agent": Value::Null, "player": false}));
    }
    let current = w.agents.of_player.get(&p).map(|r| {
        let a = &w.agents.list[r.agent];
        json!({
            "id": r.agent.0, "who": named(Ref::person(a.person), c.person_name(a.person)), "fee_pct": r.fee_pct, "since": r.since.0, "until": r.until.0,
            "satisfaction": level_band(r.satisfaction), "reputation": a.reputation, "clients": a.clients.len(),
            "base": c.nation_name(a.base),
        })
    });
    Ok(json!({"agent": current, "player": true}))
}

// ---- journal ---------------------------------------------------------------------------------------

fn goal_json(w: &pw_world::World, g: &Goal, i: usize) -> Value {
    let me_player = None::<pw_core::PlayerId>;
    let _ = (w, me_player);
    json!({
        "i": i, "text": g.text, "pinned": g.pinned.0, "done": g.done.map(|d| d.0),
        "kind": match g.kind { GoalKind::Appearances(_) => "appearances", GoalKind::Goals(_) => "goals", GoalKind::TopFlight => "top_flight", GoalKind::Personal => "personal" },
    })
}

pub fn journal(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let w = c.w;
    let sess = &c.s.game.session;
    let p = w.people[me].player;
    let (apps, goals) = if p.is_some() { (w.players.cold[p].senior_apps, w.players.cold[p].senior_goals) } else { (0, 0) };
    let goal_rows: Vec<Value> = sess
        .goals
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let mut v = goal_json(w, g, i);
            match g.kind {
                GoalKind::Appearances(n) => v["progress"] = json!({"now": apps, "target": n}),
                GoalKind::Goals(n) => v["progress"] = json!({"now": goals, "target": n}),
                _ => {}
            }
            v
        })
        .collect();
    let notes: Vec<Value> = sess.notes.iter().enumerate().rev().map(|(i, (d, t))| json!({"i": i, "date": d.0, "text": t})).collect();
    let history: Vec<Value> = sess.history.iter().map(|(p, from, to)| json!({"who": named(Ref::person(*p), c.person_name(*p)), "from": from.0, "to": to.map(|d| d.0)})).collect();
    Ok(json!({"goals": goal_rows, "notes": notes, "history": history}))
}

pub fn add_goal(s: &mut Session, args: &Value) -> ApiResult<Value> {
    if s.my_person().is_none() {
        return Err(ApiError::Unauthorized("You are observing the world.".into()));
    }
    let today = s.today();
    let req: crate::contract::GoalReq = crate::contract::request(args.clone())?;
    let n = req.target.map_or(0, |n| n.min(u64::from(u16::MAX)) as u16);
    let text = req.text.as_deref().map(str::trim).unwrap_or("").to_string();
    let (kind, text) = match req.kind.as_deref().unwrap_or("personal") {
        "appearances" if n > 0 => (GoalKind::Appearances(n), format!("Reach {n} senior appearances")),
        "goals" if n > 0 => (GoalKind::Goals(n), format!("Score {n} senior goals")),
        "top_flight" => (GoalKind::TopFlight, "Play for a club in a top-tier league".to_string()),
        "personal" if !text.is_empty() => (GoalKind::Personal, text.chars().take(200).collect()),
        _ => return Err(ApiError::Bad("Say what the goal is.".into())),
    };
    s.game.session.goals.push(Goal { text, pinned: today, kind, done: None });
    s.revision += 1;
    Ok(json!({"ok": true}))
}

pub fn goal_done(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let req: crate::contract::GoalDoneReq = crate::contract::request(args.clone())?;
    let i = req.i as usize;
    let today = s.today();
    let remove = req.remove.unwrap_or(false);
    let goals = &mut s.game.session.goals;
    if i >= goals.len() {
        return Err(ApiError::NotFound("goal".into()));
    }
    if remove {
        goals.remove(i);
    } else if goals[i].done.is_none() {
        goals[i].done = Some(today);
    } else {
        goals[i].done = None;
    }
    s.revision += 1;
    Ok(json!({"ok": true}))
}

pub fn add_note(s: &mut Session, args: &Value) -> ApiResult<Value> {
    if s.my_person().is_none() {
        return Err(ApiError::Unauthorized("You are observing the world.".into()));
    }
    let req: crate::contract::NoteReq = crate::contract::request(args.clone())?;
    let text = req.text.as_deref().map(str::trim).unwrap_or("");
    if text.is_empty() {
        return Err(ApiError::Bad("Write something first.".into()));
    }
    let today = s.today();
    s.game.session.notes.push((today, text.chars().take(2000).collect()));
    s.revision += 1;
    Ok(json!({"ok": true}))
}

pub fn remove_note(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let i = crate::contract::request::<crate::contract::IndexReq>(args.clone())?.i as usize;
    if i >= s.game.session.notes.len() {
        return Err(ApiError::NotFound("note".into()));
    }
    s.game.session.notes.remove(i);
    s.revision += 1;
    Ok(json!({"ok": true}))
}
