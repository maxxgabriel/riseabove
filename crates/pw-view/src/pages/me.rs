//! Pages that exist only while inhabiting someone: Today, Messages, Calendar,
//! Football and the contract. Each is built from what that person can know.

use pw_core::{Date, DecisionId, PlayerId};
use pw_sim::health::{self, DayKind};
use pw_world::decision::{Decision, DecisionKind};
use pw_world::event::EventKind as E;
use pw_world::{Contract, Focus, Intensity, PlayerStatus};
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};
use crate::narrative;
use crate::session::Session;
use crate::tables::{round_text, score_text};

fn named(r: Ref, n: String) -> Value {
    serde_json::to_value(Named::new(r, n)).unwrap_or(Value::Null)
}

fn need_me(c: &Ctx) -> ApiResult<PlayerId> {
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

pub fn decision_summary(c: &Ctx, id: DecisionId, d: &Decision) -> (String, Vec<pw_world::Contract>) {
    let _ = (c, id);
    match &d.kind {
        DecisionKind::ContractOffer { club, contract, renewal } => (
            format!("{} {} you a contract.", c.club_name(*club), if *renewal { "have offered" } else { "would like to sign" }),
            vec![contract.clone()],
        ),
        DecisionKind::FreeAgentOffer { club, contract } => (format!("{} have offered you a contract.", c.club_name(*club)), vec![contract.clone()]),
        DecisionKind::LoanOffer { loan } => (
            format!("{} would like to borrow you from {} until {}.", c.club_name(loan.club), c.club_name(loan.parent), crate::fmt::date(loan.end)),
            vec![],
        ),
        DecisionKind::TransferTalks { club, .. } => (format!("{} would like to talk to you.", c.club_name(*club)), vec![]),
    }
}

fn decision_club(d: &Decision) -> pw_core::ClubId {
    match &d.kind {
        DecisionKind::ContractOffer { club, .. } | DecisionKind::FreeAgentOffer { club, .. } | DecisionKind::TransferTalks { club, .. } => *club,
        DecisionKind::LoanOffer { loan } => loan.club,
    }
}

fn state_of(d: &Decision) -> &'static str {
    if d.resolved {
        if d.answer.is_some() { "settled" } else { "expired" }
    } else if d.answer.is_some() {
        "answered"
    } else {
        "awaiting"
    }
}

fn contract_rows(c: &Ctx, k: &Contract) -> Value {
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
    let decisions: Vec<Value> = w
        .decisions
        .pending_for(me)
        .filter(|(_, d)| d.answer.is_none())
        .map(|(id, d)| {
            let (summary, _) = decision_summary(c, id, d);
            json!({"id": format!("d{}", id.0), "title": d.kind.title(), "summary": summary, "deadline": d.deadline.0, "from": named(Ref::club(decision_club(d)), c.club_name(decision_club(d)))})
        })
        .collect();

    // What changed since the viewer last looked.
    let since = Date(c.s.meta.last_viewed.max(date.0 - 60));
    let mut changes: Vec<Value> = Vec::new();
    for e in w.events.since(since).iter().rev() {
        if !narrative::visible(c, e) || !relevant(c, p, &e.kind) {
            continue;
        }
        changes.push(json!({"date": e.date.0, "kind": narrative::label(&e.kind), "parts": narrative::describe(c, &e.kind)}));
        if changes.len() >= 12 {
            break;
        }
    }

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
        "commitments": commitments, "decisions": decisions, "changes": changes,
        "next_match": next.map(|f| fixture_brief(c, f)), "recent": recent, "unrevealed": unrevealed,
        "condition": condition_words(h),
        "availability": {
            "injured": h.injury != 0, "injury": if h.injury != 0 { json!(health::injury_name(w, h.injury)) } else { Value::Null }, "days": h.injury_days, "ban": h.ban,
        },
        "contract": contract, "league": position_in_league,
        "form": h.form.iter().filter(|&&r| r > 0).map(|&r| f64::from(r) / 10.0).collect::<Vec<_>>(),
        "minutes_4w": h.minutes_4w,
        "plan": plan_json(&cold.plan),
        "last_viewed": c.s.meta.last_viewed,
        "conceal_mine": c.s.meta.conceal_mine,
    }))
}

fn relevant(c: &Ctx, p: PlayerId, k: &E) -> bool {
    let club = c.w.players.hot[p].club;
    if k.player() == Some(p) {
        return true;
    }
    match *k {
        E::Retired { person } => Some(person) == c.me(),
        E::ManagerSacked { club: x, .. } | E::ManagerAppointed { club: x, .. } | E::YouthIntake { club: x, .. } => x == club,
        E::Champion { team, .. } | E::Promoted { team, .. } | E::Relegated { team, .. } => c.w.teams[team].club == club,
        _ => false,
    }
}

pub fn mark_viewed(s: &mut Session) -> ApiResult<Value> {
    s.meta.last_viewed = s.today().0;
    Ok(json!({"ok": true}))
}

// ---- messages ---------------------------------------------------------------------------------

pub fn messages(c: &Ctx) -> ApiResult<Value> {
    let p = need_me(c)?;
    let me = c.me().expect("inhabiting");
    let w = c.w;
    let mut out: Vec<Value> = Vec::new();

    for (id, d) in w.decisions.all.iter_enumerated().filter(|(_, d)| d.person == me) {
        let (summary, _) = decision_summary(c, id, d);
        let state = state_of(d);
        let club = decision_club(d);
        out.push(json!({
            "id": format!("d{}", id.0), "kind": "decision", "date": d.created.0, "subject": d.kind.title(), "preview": summary,
            "from": named(Ref::club(club), c.club_name(club)), "state": state, "deadline": d.deadline.0,
            "folder": match (&d.kind, state) { (_, "awaiting") => "awaiting", (DecisionKind::LoanOffer { .. }, _) => "work", _ => "contracts" },
            "needs_action": state == "awaiting",
        }));
    }
    let since = w.date.add_days(-180);
    for (i, e) in w.events.all().iter().enumerate().rev() {
        if e.date < since {
            break;
        }
        if !narrative::visible(c, e) || !relevant(c, p, &e.kind) {
            continue;
        }
        let club = w.players.hot[p].club;
        let folder = match e.kind {
            E::ContractSigned { .. } | E::Transfer { .. } | E::Released { .. } | E::LoanMove { .. } | E::Interest { .. } | E::BidAccepted { .. } | E::BidRejected { .. } | E::TransferListed { .. } => "contracts",
            E::CallUp { .. } => "invitations",
            _ => "work",
        };
        out.push(json!({
            "id": format!("e{i}"), "kind": "event", "date": e.date.0, "subject": narrative::label(&e.kind),
            "preview": Value::Null, "parts": narrative::describe(c, &e.kind),
            "from": if club.is_some() { named(Ref::club(club), c.club_name(club)) } else { Value::Null },
            "state": "info", "folder": folder, "needs_action": false, "deadline": Value::Null,
        }));
        if out.len() > 400 {
            break;
        }
    }
    out.sort_by(|a, b| b["date"].as_i64().cmp(&a["date"].as_i64()));
    let awaiting = out.iter().filter(|m| m["needs_action"] == true).count();
    Ok(json!({"messages": out, "awaiting": awaiting}))
}

pub fn message(c: &Ctx, args: &Value) -> ApiResult<Value> {
    need_me(c)?;
    let id = args.get("id").and_then(Value::as_str).ok_or_else(|| ApiError::Bad("missing message id".into()))?;
    let me = c.me().expect("inhabiting");
    let w = c.w;
    if let Some(n) = id.strip_prefix('d').and_then(|s| s.parse::<u32>().ok()) {
        let did = DecisionId(n);
        let d = w.decisions.all.get(did).filter(|d| d.person == me).ok_or_else(|| ApiError::NotFound("message".into()))?;
        let (summary, contracts) = decision_summary(c, did, d);
        let options: Vec<Value> = d.kind.options().iter().enumerate().map(|(i, l)| json!({"i": i, "label": l})).collect();
        let pending = w.market.pending.iter().find(|x| x.decision == did);
        let cold = &w.players.cold[c.my_player().expect("me")];
        let state = state_of(d);
        // Once an offer is settled, "your current contract" and "what accepting means" describe a world that has moved on.
        let live = matches!(state, "awaiting" | "answered");
        let mut paragraphs: Vec<String> = vec![summary];
        let mut consequences: Vec<String> = Vec::new();
        let default_label = d.kind.options()[usize::from(d.default).min(d.kind.options().len() - 1)];
        match &d.kind {
            DecisionKind::ContractOffer { renewal: true, .. } => {
                if live {
                    paragraphs.push(format!("Your current contract runs until {}.", crate::fmt::date(cold.contract.end)));
                }
                consequences.push("Accepting replaces your current contract with the terms shown.".into());
                consequences.push("Declining leaves your current contract unchanged. The club may approach you again later.".into());
            }
            DecisionKind::ContractOffer { club, .. } => {
                if let Some(deal) = pending {
                    paragraphs.push(format!("{} have agreed a fee with {} for you.", c.club_name(*club), c.club_name(deal.seller)));
                    paragraphs.push("The move happens only if you agree personal terms.".into());
                }
                consequences.push(format!("Accepting means moving to {} once the registration is completed.", c.club_name(*club)));
                consequences.push("Declining ends this approach; the club will not come back for you for a while.".into());
            }
            DecisionKind::FreeAgentOffer { club, .. } => {
                consequences.push(format!("Accepting means signing for {}.", c.club_name(*club)));
                consequences.push("Declining leaves you free to consider other offers.".into());
            }
            DecisionKind::LoanOffer { loan } => {
                paragraphs.push(format!("The loan would run until {}. You would remain a {} player.", crate::fmt::date(loan.end), c.club_name(loan.parent)));
                if loan.buy_option > 0 {
                    paragraphs.push("The borrowing club has an option to buy you at the end of the loan.".into());
                }
                consequences.push("Accepting sends you to the borrowing club for the period shown.".into());
                consequences.push("Declining keeps you where you are.".into());
            }
            DecisionKind::TransferTalks { .. } => consequences.push("Agreeing only opens talks.".into()),
        }
        if !live {
            consequences.clear();
        }
        let outcome = match (state, d.answer) {
            ("settled", Some(a)) | ("answered", Some(a)) => Some(format!("You chose: {}", d.kind.options()[usize::from(a).min(d.kind.options().len() - 1)])),
            ("expired", _) => Some(format!("No response was given. The default was applied: {default_label}.")),
            _ => None,
        };
        let terms: Vec<Value> = contracts.iter().map(|k| contract_rows(c, k)).collect();
        let current = if live && matches!(d.kind, DecisionKind::ContractOffer { renewal: true, .. }) { contract_rows(c, &cold.contract) } else { Value::Null };
        let club = decision_club(d);
        return Ok(json!({
            "id": id, "kind": "decision", "title": d.kind.title(), "from": named(Ref::club(club), c.club_name(club)),
            "created": d.created.0, "deadline": d.deadline.0, "state": state,
            "paragraphs": paragraphs, "options": options, "answer": d.answer,
            "default": {"i": d.default, "label": default_label},
            "without_response": if state != "awaiting" { Value::Null } else { json!(format!("If you do not respond by {}, the response your own judgement would give is applied: {}.", crate::fmt::date(d.deadline), default_label.to_lowercase())) },
            "consequences": consequences, "terms": terms.first().cloned().unwrap_or(Value::Null), "current_terms": current,
            "outcome": outcome,
        }));
    }
    if let Some(i) = id.strip_prefix('e').and_then(|s| s.parse::<usize>().ok()) {
        let e = w.events.all().get(i).filter(|e| narrative::visible(c, e)).ok_or_else(|| ApiError::NotFound("message".into()))?;
        return Ok(json!({
            "id": id, "kind": "event", "title": narrative::label(&e.kind), "date": e.date.0,
            "parts": narrative::describe(c, &e.kind), "primary": narrative::primary(c, &e.kind),
        }));
    }
    Err(ApiError::NotFound("message".into()))
}

pub fn answer(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let id = args.get("id").and_then(Value::as_str).and_then(|s| s.strip_prefix('d')).and_then(|s| s.parse::<u32>().ok()).ok_or_else(|| ApiError::Bad("missing decision".into()))?;
    let choice = args.get("choice").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing choice".into()))? as u8;
    s.answer(DecisionId(id), choice)?;
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
        "plan": plan_json(&cold.plan),
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
    s.sim.world.players.cold[p].plan = plan;
    s.revision += 1;
    Ok(plan_json(&plan))
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
    let past: Vec<Value> = w
        .events
        .all()
        .iter()
        .filter(|e| matches!(e.kind, E::ContractSigned { player, .. } if player == p) && narrative::visible(c, e))
        .rev()
        .take(6)
        .map(|e| json!({"date": e.date.0, "parts": narrative::describe(c, &e.kind)}))
        .collect();
    Ok(json!({
        "has_contract": true,
        "club": named(Ref::club(k.club), c.club_name(k.club)),
        "summary": {"wage": k.current_wage(w.date), "end": k.end.0, "days_left": k.days_left(w.date), "status": cold.status.label(), "kind": format!("{:?}", k.kind)},
        "terms": contract_rows(c, k),
        "loan": cold.loan.as_ref().map(|l| json!({"parent": named(Ref::club(l.parent), c.club_name(l.parent)), "club": named(Ref::club(l.club), c.club_name(l.club)), "end": l.end.0, "recall": l.recall, "wage_share": l.wage_share, "buy_option": l.buy_option})),
        "offers": offers, "history": past,
        "guaranteed_note": "Bonuses are paid only when earned. The wage shown is the current weekly figure including any yearly rises.",
    }))
}
