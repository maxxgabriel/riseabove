//! Pages that exist only while inhabiting someone: Today, Messages, Calendar,
//! Football and the contract. Each is built from what that person can know.

use pw_core::{Date, PlayerId};
use pw_sim::health::{self, DayKind};
use pw_world::event::EventKind as E;
use pw_world::{Contract, Focus, Intensity, PlayerStatus};
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};
use crate::narrative;
use crate::session::Session;
use crate::tables::{round_text, score_text};

pub(crate) fn named(r: Ref, n: String) -> Value {
    serde_json::to_value(Named::new(r, n)).unwrap_or(Value::Null)
}

pub(crate) fn need_me(c: &Ctx) -> ApiResult<PlayerId> {
    c.my_player().ok_or_else(|| ApiError::State("You are observing the world. Inhabit a player to use this page.".into()))
}

fn word(v: u8, tiers: &[(u8, &str)], floor: &str) -> String {
    tiers.iter().find(|(min, _)| v >= *min).map_or(floor, |(_, s)| s).to_string()
}

fn condition_words(h: &pw_world::PlayerHot) -> Value {
    json!({
        "condition": {"label": word(h.condition, &[(92, "Fresh"), (78, "Good"), (62, "Tired")], "Exhausted"), "value": h.condition},
        "sharpness": {"label": word(h.sharpness, &[(80, "Match sharp"), (60, "Reasonably sharp")], "Rusty"), "value": h.sharpness},
        "morale": {"label": word(h.morale, &[(80, "Excellent"), (65, "Good"), (45, "Okay"), (30, "Low")], "Very low"), "value": h.morale},
        "confidence": {"label": word(h.confidence, &[(80, "Very confident"), (62, "Confident"), (42, "Uncertain")], "Doubting"), "value": h.confidence},
        "wellbeing": {"label": word(h.wellbeing, &[(80, "Thriving"), (62, "Well"), (42, "Strained")], "Struggling"), "value": h.wellbeing},
        "fatigue": {"label": word(100 - h.fatigue.min(100), &[(85, "Light legs"), (65, "Some fatigue"), (40, "Heavy legs")], "Very heavy legs"), "value": h.fatigue},
    })
}

fn day_kind_text(k: DayKind) -> (&'static str, &'static str) {
    match k {
        DayKind::Match => ("Match day", "match"),
        DayKind::AfterMatch => ("Recovery day after the match", "recovery"),
        DayKind::BeforeMatch => ("Preparation for tomorrow's match", "training"),
        DayKind::Training => ("Training day", "training"),
        DayKind::Rest => ("Rest day", "rest"),
        DayKind::Offseason => ("Off-season, individual work", "rest"),
    }
}

fn fixture_brief(c: &Ctx, f: &pw_world::Fixture) -> Value {
    let my = c.my_team();
    let home = f.home == my;
    let opp = f.opponent(my);
    json!({
        "uid": f.uid, "date": f.date.0, "comp": named(Ref::comp(f.comp), c.comp_short(f.comp)), "round": round_text(c, f),
        "opponent": named(c.team_ref(opp), c.team_short(opp)), "home": home, "days": f.date.0 - c.w.date.0,
        "venue": if f.neutral { Some("Neutral venue".to_string()) } else { Some(c.w.clubs[c.w.teams[f.home].club].stadium.clone()).filter(|v| !v.trim().is_empty()) },
    })
}

fn result_brief(c: &Ctx, f: &pw_world::Fixture) -> Value {
    let mut v = fixture_brief(c, f);
    let concealed = c.is_concealed(f.uid);
    v["concealed"] = json!(concealed);
    if let (Some(s), false) = (f.score, concealed) {
        v["score"] = json!(score_text(&s));
        let my = c.my_team();
        v["outcome"] = json!(match s.home_won() {
            Some(h) if h == (f.home == my) => "win",
            Some(_) => "loss",
            None => "draw",
        });
    }
    v
}

fn my_fixtures<'a>(c: &Ctx<'a>, from: Date, to: Date) -> Vec<&'a pw_world::Fixture> {
    let my = c.my_team();
    if my.is_none() {
        return vec![];
    }
    c.w.fixtures.between(from, to).map(|id| c.w.fixtures.get(id)).filter(|f| f.involves(my)).collect()
}

pub(crate) fn contract_rows(c: &Ctx, k: &Contract) -> Value {
    json!([
        {"label": "Wage per week", "money": k.current_wage(c.w.date).max(k.wage)},
        {"label": "Runs until", "date": k.end.0},
        {"label": "Starts", "date": k.start.0},
        {"label": "Release clause", "money": if k.release_clause > 0 { json!(k.release_clause) } else { Value::Null }, "text": if k.release_clause > 0 { Value::Null } else { json!("None") }},
        {"label": "Promised role", "text": k.promised_status.map_or("None".to_string(), |s| s.label().to_string())},
        {"label": "Appearance bonus", "money": k.appearance_bonus},
        {"label": "Goal bonus", "money": k.goal_bonus},
        {"label": "Yearly rise", "text": format!("{}%", k.yearly_rise)},
        {"label": "Cut if relegated", "text": format!("{}%", k.relegation_cut)},
    ])
}

pub fn today(c: &Ctx) -> ApiResult<Value> {
    let p = need_me(c)?;
    let w = c.w;
    let h = &w.players.hot[p];
    let cold = &w.players.cold[p];
    let me = c.me().expect("inhabiting");
    let team = h.team;
    let date = w.date;

    let day_kind = if team.is_some() { health::team_days(w, date)[team.0 as usize] } else { DayKind::Offseason };
    let (day_label, day_key) = day_kind_text(day_kind);

    let next = if team.is_some() { w.fixtures.next_for(team, date, 120).map(|id| w.fixtures.get(id)) } else { None };
    let recent: Vec<Value> = {
        let mut v = my_fixtures(c, date.add_days(-90), date.add_days(-1));
        v.retain(|f| f.score.is_some());
        v.sort_by_key(|f| std::cmp::Reverse((f.date, f.uid)));
        v.iter().take(5).map(|f| result_brief(c, f)).collect()
    };
    let unrevealed: Vec<Value> = c
        .concealed_fixtures()
        .into_iter()
        .filter(|f| f.involves(team))
        .map(|f| fixture_brief(c, f))
        .collect();

    // Commitments.
    let mut commitments: Vec<Value> = Vec::new();
    let mut match_listed = false;
    if let Some(f) = my_fixtures(c, date, date).into_iter().next() {
        if f.score.is_none() {
            let text = format!("Match day: {} {}", if f.home == team { "home to" } else { "away at" }, c.team_short(f.opponent(team)));
            commitments.push(json!({"kind": "match", "text": text, "ref": Ref::fixture(f.uid)}));
            match_listed = true;
        }
    }
    if !(match_listed && day_key == "match") {
        commitments.push(json!({"kind": day_key, "text": day_label}));
    }
    if h.injury != 0 {
        commitments.push(json!({"kind": "medical", "text": format!("Rehabilitation: {}, about {} days to go", health::injury_name(w, h.injury).to_lowercase(), h.injury_days)}));
    }
    if h.ban > 0 {
        commitments.push(json!({"kind": "discipline", "text": format!("Suspended for {} more match(es)", h.ban)}));
    }

    // Decisions waiting.
    let decisions = super::inbox::waiting(c);

    // What reached the viewer since they last looked.
    let since = Date(c.s.meta.last_viewed.max(date.0 - 60));
    let mut changes: Vec<Value> = Vec::new();
    for e in w.events.since(since).iter().rev() {
        if !pw_career::feed::concerns(w, me, e) || !narrative::visible(c, e) {
            continue;
        }
        changes.push(json!({
            "id": format!("e{}", e.id.0), "date": e.date.0, "kind": narrative::label(&e.kind), "parts": narrative::describe(c, e),
            "important": pw_career::feed::is_important(w, me, e),
        }));
        if changes.len() >= 12 {
            break;
        }
    }

    // What is on the person's mind, in their own words, with the world's recorded reasons.
    let life = &w.lives[me];
    let mut mind: Vec<(pw_world::life::MoodFactor, i8)> = life.morale_why.iter().copied().collect();
    mind.sort_by_key(|(_, x)| std::cmp::Reverse(x.unsigned_abs()));
    let mind: Vec<Value> = mind
        .iter()
        .take(4)
        .filter(|(_, x)| x.unsigned_abs() >= 2)
        .map(|(f, x)| json!({"text": format!("{} {}", pw_narrate::fmt::feeling(*x), f.label()), "value": x}))
        .collect();

    // Things the person has set in motion that the world has not yet acted on.
    let mut waiting_on: Vec<Value> = Vec::new();
    for pi in w.intents.queue.iter().filter(|pi| pi.person == me) {
        waiting_on.push(json!({"kind": "intent", "text": super::act::intent_text(c, &pi.intent), "since": pi.date.0}));
    }
    for (_, m) in w.meetings.pending().filter(|(_, m)| m.initiator == me) {
        waiting_on.push(json!({"kind": "meeting", "text": format!("You asked {} to talk about {}", c.person_name(m.with), m.topic.label()), "since": m.requested.0, "date": m.date.0, "ref": Ref::person(m.with)}));
    }
    let open_promises = w.social.promises.iter().filter(|pr| (pr.to == me || pr.from == me) && pr.state == pw_world::PromiseState::Open).count();
    let next_due = w.social.promises.iter().filter(|pr| (pr.to == me || pr.from == me) && pr.state == pw_world::PromiseState::Open).map(|pr| pr.due.0).min();

    let contract = if h.club.is_some() {
        json!({"club": named(Ref::club(cold.contract.club), c.club_name(cold.contract.club)), "end": cold.contract.end.0, "days_left": cold.contract.days_left(date), "status": cold.status.label(), "wage": cold.contract.current_wage(date)})
    } else {
        Value::Null
    };
    let mut position_in_league = Value::Null;
    if let Some(l) = team.get().and_then(|t| w.league_of(t)) {
        let (rows, _) = crate::tables::visible_table(c, l);
        if let Some(pos) = rows.iter().position(|r| r.team == team) {
            position_in_league = json!({"comp": named(Ref::comp(l), c.comp_name(l)), "position": pos + 1, "teams": rows.len(), "points": rows[pos].points});
        }
    }
    Ok(json!({
        "date": date.0, "weekday": date.weekday().index(),
        "me": {
            "person": me.0, "name": c.person_name(me), "age": c.age(me), "position": cold.best_pos.code(),
            "club": if h.club.is_some() { named(Ref::club(h.club), c.club_name(h.club)) } else { Value::Null },
            "team": if team.is_some() { Value::String(w.teams[team].kind.label().into()) } else { Value::Null },
            "status": c.status_label(p), "squad_status": if h.club.is_some() { json!(cold.status.label()) } else { Value::Null },
            "shirt": cold.shirt,
        },
        "day": {"label": day_label, "kind": day_key},
        "commitments": commitments, "decisions": decisions, "changes": changes, "mind": mind, "waiting_on": waiting_on,
        "promises": {"open": open_promises, "next_due": next_due},
        "routine_hours": life.routine.total(), "lifestyle": life.finances.lifestyle.label(),
        "next_match": next.map(|f| fixture_brief(c, f)), "recent": recent, "unrevealed": unrevealed,
        "condition": condition_words(h),
        "availability": {
            "injured": h.injury != 0, "injury": if h.injury != 0 { json!(health::injury_name(w, h.injury)) } else { Value::Null }, "days": h.injury_days, "ban": h.ban,
        },
        "contract": contract, "league": position_in_league,
        "form": h.form.iter().filter(|&&r| r > 0).map(|&r| f64::from(r) / 10.0).collect::<Vec<_>>(),
        "minutes_4w": h.minutes_4w,
        "plan": plan_json(&cold.plan), "plan_pending": plan_pending(c),
        "last_viewed": c.s.meta.last_viewed,
        "conceal_mine": c.s.meta.conceal_mine,
    }))
}

pub fn mark_viewed(s: &mut Session) -> ApiResult<Value> {
    s.meta.last_viewed = s.today().0;
    s.game.session.seen = s.game.sim.world.events.last_id();
    Ok(json!({"ok": true}))
}

// ---- calendar ----------------------------------------------------------------------------------

pub fn calendar(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let p = need_me(c)?;
    let w = c.w;
    let from = Date(args.get("from").and_then(Value::as_i64).map_or(w.date.0, |v| v as i32));
    let to = Date(args.get("to").and_then(Value::as_i64).map_or(from.0 + 34, |v| v as i32).min(from.0 + 120));
    let team = c.my_team();
    let cold = &w.players.cold[p];
    let h = &w.players.hot[p];
    let mut days: Vec<Value> = Vec::new();
    let mut d = from;
    while d <= to {
        let mut entries: Vec<Value> = Vec::new();
        if team.is_some() && d >= w.date {
            let kind = health::team_days(w, d)[team.0 as usize];
            let (label, key) = day_kind_text(kind);
            if !matches!(kind, DayKind::Match) {
                entries.push(json!({"kind": key, "label": label, "source": "Club training schedule", "state": "scheduled"}));
            }
        }
        for f in my_fixtures(c, d, d) {
            let concealed = c.is_concealed(f.uid);
            let mut e = json!({
                "kind": "match", "label": format!("{} {}", if f.home == team { "v" } else { "at" }, c.team_short(f.opponent(team))),
                "sub": c.comp_short(f.comp), "source": "Fixture list", "state": if f.score.is_some() { "played" } else { "confirmed" },
                "ref": Ref::fixture(f.uid), "required": true,
            });
            if let (Some(s), false) = (f.score, concealed) {
                e["result"] = json!(score_text(&s));
            }
            entries.push(e);
        }
        if h.club.is_some() && d == cold.contract.end {
            entries.push(json!({"kind": "contract", "label": "Contract ends", "source": "Contract", "state": "confirmed"}));
        }
        if h.club.is_some() {
            let n = &w.nations[w.clubs[h.club].nation];
            for (a, b) in n.season.windows.iter() {
                if d == *a {
                    entries.push(json!({"kind": "window", "label": "Transfer window opens", "source": "Competition rules", "state": "confirmed"}));
                }
                if d == *b {
                    entries.push(json!({"kind": "window", "label": "Transfer window closes", "source": "Competition rules", "state": "confirmed"}));
                }
            }
            if d == n.season.start {
                entries.push(json!({"kind": "season", "label": "Season starts", "source": "Competition rules", "state": "confirmed"}));
            }
            if d == n.season.end {
                entries.push(json!({"kind": "season", "label": "Season ends", "source": "Competition rules", "state": "confirmed"}));
            }
        }
        for (_, dec) in w.decisions.pending_for(c.me().expect("me")) {
            if dec.deadline == d && dec.answer.is_none() {
                entries.push(json!({"kind": "decision", "label": format!("{} expires", dec.kind.title()), "source": "Messages", "state": "deadline"}));
            }
        }
        days.push(json!({"date": d.0, "entries": entries}));
        d = d.add_days(1);
    }
    Ok(json!({"from": from.0, "to": to.0, "today": w.date.0, "days": days}))
}

// ---- football: squad place, training, contract -------------------------------------------------------

/// The training plan the person has asked for that the world has not applied yet.
fn plan_pending(c: &Ctx) -> Value {
    let Some(me) = c.me() else { return Value::Null };
    c.w.intents.queue.iter().rev().find_map(|pi| match pi.intent {
        pw_world::Intent::SetTraining(plan) if pi.person == me => Some(plan_json(&plan)),
        _ => None,
    }).unwrap_or(Value::Null)
}

fn plan_json(plan: &pw_world::TrainingPlan) -> Value {
    let (fk, fv): (&str, Value) = match plan.focus {
        Focus::General => ("general", Value::Null),
        Focus::Group(g) => ("group", json!(format!("{g:?}").to_lowercase())),
        Focus::Attribute(a) => ("attribute", json!(a.key())),
        Focus::Position(p) => ("position", json!(p.code())),
    };
    json!({
        "focus": {"kind": fk, "value": fv},
        "intensity": match plan.intensity { Intensity::Light => "light", Intensity::Normal => "normal", Intensity::High => "high" },
        "extra": plan.extra, "recovery": plan.recovery,
    })
}

pub fn football(c: &Ctx) -> ApiResult<Value> {
    let p = need_me(c)?;
    let w = c.w;
    let h = &w.players.hot[p];
    let cold = &w.players.cold[p];
    let team = h.team;
    let mut usage: Vec<Value> = Vec::new();
    if team.is_some() {
        let mut fx = my_fixtures(c, w.date.add_days(-120), w.date.add_days(-1));
        fx.retain(|f| f.score.is_some());
        fx.sort_by_key(|f| std::cmp::Reverse((f.date, f.uid)));
        for f in fx.iter().take(8) {
            let mut item = result_brief(c, f);
            if let Some(line) = w.reports.get(&f.uid).and_then(|r| r.line(p)) {
                if !c.is_concealed(f.uid) {
                    item["played"] = json!({"started": line.started, "minutes": line.minutes, "rating": line.rating, "goals": line.goals, "assists": line.assists});
                }
            }
            usage.push(item);
        }
    }
    // Others competing for similar roles, as far as the viewer knows them.
    let rivals: Vec<Value> = if team.is_some() {
        w.teams[team]
            .squad
            .iter()
            .copied()
            .filter(|&q| q != p && w.players.cold[q].best_pos.group() == cold.best_pos.group())
            .map(|q| {
                let qh = &w.players.hot[q];
                json!({"player": named(c.player_ref(q), c.player_name(q)), "pos": w.players.cold[q].best_pos.code(), "age": c.age(w.players.cold[q].person),
                    "available": qh.available(), "minutes_4w": qh.minutes_4w})
            })
            .collect()
    } else {
        vec![]
    };
    let apps = c.w.stats.for_player(p).map(|l| (u32::from(l.apps), u32::from(l.starts), l.minutes)).fold((0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
    Ok(json!({
        "team": if team.is_some() { Value::String(w.teams[team].kind.label().into()) } else { Value::Null },
        "squad_status": cold.status.label(), "promised": cold.contract.promised_status.map(|s| s.label()),
        "usage": usage, "rivals": rivals,
        "season": {"apps": apps.0, "starts": apps.1, "minutes": apps.2},
        "minutes_4w": h.minutes_4w,
        "plan": plan_json(&cold.plan), "plan_pending": plan_pending(c),
        "options": {
            "attributes": pw_core::Attr::ALL.iter().filter(|a| !a.is_goalkeeping() || cold.best_pos == pw_core::Pos::GK).map(|a| json!({"key": a.key(), "label": a.label(), "group": format!("{:?}", a.group())})).collect::<Vec<_>>(),
            "positions": pw_core::Pos::ALL.iter().map(|p| json!({"code": p.code()})).collect::<Vec<_>>(),
        },
    }))
}

pub fn set_plan(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let p = s.my_player().ok_or_else(|| ApiError::State("Inhabit a player to set a training plan.".into()))?;
    if s.w().players.hot[p].status == PlayerStatus::Retired {
        return Err(ApiError::State("You have retired.".into()));
    }
    let mut plan = s.w().players.cold[p].plan;
    if let Some(i) = args.get("intensity").and_then(Value::as_str) {
        plan.intensity = match i {
            "light" => Intensity::Light,
            "normal" => Intensity::Normal,
            "high" => Intensity::High,
            _ => return Err(ApiError::Bad("Unknown intensity.".into())),
        };
    }
    if let Some(e) = args.get("extra").and_then(Value::as_u64) {
        plan.extra = e.min(3) as u8;
    }
    if let Some(r) = args.get("recovery").and_then(Value::as_u64) {
        plan.recovery = r.min(2) as u8;
    }
    if let Some(f) = args.get("focus") {
        let kind = f.get("kind").and_then(Value::as_str).unwrap_or("general");
        let value = f.get("value").and_then(Value::as_str).unwrap_or("");
        plan.focus = match kind {
            "general" => Focus::General,
            "group" => Focus::Group(match value {
                "technical" => pw_core::attr::AttrGroup::Technical,
                "mental" => pw_core::attr::AttrGroup::Mental,
                "physical" => pw_core::attr::AttrGroup::Physical,
                "goalkeeping" => pw_core::attr::AttrGroup::Goalkeeping,
                _ => return Err(ApiError::Bad("Unknown attribute group.".into())),
            }),
            "attribute" => Focus::Attribute(pw_core::Attr::from_key(value).ok_or_else(|| ApiError::Bad("Unknown attribute.".into()))?),
            "position" => Focus::Position(pw_core::Pos::from_code(value).ok_or_else(|| ApiError::Bad("Unknown position.".into()))?),
            _ => return Err(ApiError::Bad("Unknown focus.".into())),
        };
    }
    s.act(pw_world::Intent::SetTraining(plan))?;
    Ok(json!({"plan": plan_json(&plan), "applies": "tomorrow"}))
}

pub fn contract(c: &Ctx) -> ApiResult<Value> {
    let p = need_me(c)?;
    let w = c.w;
    let h = &w.players.hot[p];
    let cold = &w.players.cold[p];
    if h.club.is_none() {
        return Ok(json!({"has_contract": false, "status": c.status_label(p)}));
    }
    let k = &cold.contract;
    let me = c.me().expect("me");
    let offers: Vec<Value> = w
        .decisions
        .pending_for(me)
        .filter(|(_, d)| d.answer.is_none())
        .map(|(id, d)| json!({"id": format!("d{}", id.0), "title": d.kind.title(), "deadline": d.deadline.0}))
        .collect();
    let agent = w.agents.of_player.get(&p).map(|r| {
        let a = &w.agents.list[r.agent];
        json!({"who": named(Ref::person(a.person), c.person_name(a.person)), "fee_pct": r.fee_pct, "until": r.until.0, "satisfaction": pw_narrate::fmt::level(r.satisfaction)})
    });
    let talks = w.market.talking.get(&p).map(|&t| super::inbox::talk_json(c, &w.talks[t]));
    let past: Vec<Value> = w
        .events
        .all()
        .iter()
        .filter(|e| matches!(e.kind, E::ContractSigned { player, .. } if player == p) && narrative::visible(c, e))
        .rev()
        .take(6)
        .map(|e| json!({"date": e.date.0, "parts": narrative::describe(c, e)}))
        .collect();
    Ok(json!({
        "has_contract": true,
        "club": named(Ref::club(k.club), c.club_name(k.club)),
        "summary": {"wage": k.current_wage(w.date), "end": k.end.0, "days_left": k.days_left(w.date), "status": cold.status.label(), "kind": format!("{:?}", k.kind)},
        "terms": contract_rows(c, k),
        "loan": cold.loan.as_ref().map(|l| json!({"parent": named(Ref::club(l.parent), c.club_name(l.parent)), "club": named(Ref::club(l.club), c.club_name(l.club)), "end": l.end.0, "recall": l.recall, "wage_share": l.wage_share, "buy_option": l.buy_option})),
        "offers": offers, "history": past, "agent": agent, "talks": talks,
        "transfer_request": w.market.requests.get(&p).map(|d| d.0), "listed": w.market.listed.contains_key(&p),
        "guaranteed_note": "Bonuses are paid only when earned. The wage shown is the current weekly figure including any yearly rises.",
    }))
}
