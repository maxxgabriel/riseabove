//! The inhabited person's inbox: everything that needs an answer, and everything that reached them.
//! Decisions come from the world's decision port, so every kind the simulation can raise (offers,
//! contract talks, conversations, a partner's question, a treatment choice…) is handled here.
//! Nothing is invented: paragraphs and consequences are stated only where the rules make them true.

use pw_core::{ClubId, DecisionId, EventId};
use pw_narrate::choices;
use pw_world::decision::{Choice, Decision, DecisionKind};
use pw_world::event::{Cause, EventKind as E};
use pw_world::interaction::{Meeting, MeetingState};
use pw_world::negotiation::{Negotiation, TalkLine, TalkState, Terms};
use pw_world::PartnerAsk;
use serde_json::{Value, json};

use super::me::{contract_rows, named, need_me};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Ref};
use crate::narrative::{self, Group};
use crate::session::Session;

pub fn kind_key(k: &DecisionKind) -> &'static str {
    match k {
        DecisionKind::TransferTalks { .. } => "transfer_talks",
        DecisionKind::ContractOffer { renewal: true, .. } => "renewal",
        DecisionKind::ContractOffer { .. } => "contract",
        DecisionKind::LoanOffer { .. } => "loan",
        DecisionKind::FreeAgentOffer { .. } => "free_agent",
        DecisionKind::Negotiation { .. } => "negotiation",
        DecisionKind::Meeting { .. } => "meeting",
        DecisionKind::Partner { .. } => "partner",
        DecisionKind::Trial { .. } => "trial",
        DecisionKind::NationChoice { .. } => "nation",
        DecisionKind::Treatment { .. } => "treatment",
        DecisionKind::Endorsement { .. } => "endorsement",
    }
}

pub fn state_of(d: &Decision) -> &'static str {
    if d.resolved {
        if d.answer.is_some() { "settled" } else { "expired" }
    } else if d.answer.is_some() {
        "answered"
    } else {
        "awaiting"
    }
}

fn folder_of(d: &Decision, state: &str) -> &'static str {
    if state == "awaiting" {
        return "awaiting";
    }
    match d.kind {
        DecisionKind::Meeting { .. } => "conversations",
        DecisionKind::Partner { .. } => "life",
        DecisionKind::Treatment { .. } | DecisionKind::NationChoice { .. } => "work",
        _ => "contracts",
    }
}

/// Who a decision comes from, as a link.
pub fn decision_from(c: &Ctx, d: &Decision) -> Value {
    let w = c.w;
    match &d.kind {
        DecisionKind::TransferTalks { club, .. } | DecisionKind::ContractOffer { club, .. } | DecisionKind::FreeAgentOffer { club, .. } | DecisionKind::Trial { club, .. } => {
            named(Ref::club(*club), c.club_name(*club))
        }
        DecisionKind::LoanOffer { loan } => named(Ref::club(loan.club), c.club_name(loan.club)),
        DecisionKind::Negotiation { talk } => {
            let club = w.talks[*talk].club;
            named(Ref::club(club), c.club_name(club))
        }
        DecisionKind::Meeting { meeting } => {
            let m = &w.meetings.list[*meeting];
            named(Ref::person(m.initiator), c.person_name(m.initiator))
        }
        DecisionKind::Partner { partner, .. } => named(Ref::person(*partner), c.person_name(*partner)),
        DecisionKind::NationChoice { nation, .. } => named(Ref::nation(*nation), c.nation_name(*nation)),
        DecisionKind::Endorsement { brand, .. } => Value::String(w.commerce.brands[*brand as usize].name.clone()),
        DecisionKind::Treatment { .. } => Value::Null,
    }
}

fn from_club(d: &Decision, c: &Ctx) -> ClubId {
    match &d.kind {
        DecisionKind::TransferTalks { club, .. } | DecisionKind::ContractOffer { club, .. } | DecisionKind::FreeAgentOffer { club, .. } | DecisionKind::Trial { club, .. } => *club,
        DecisionKind::LoanOffer { loan } => loan.club,
        DecisionKind::Negotiation { talk } => c.w.talks[*talk].club,
        _ => ClubId::NONE,
    }
}

/// Plain answer labels for what each choice means in this decision.
fn option_label(c: &Ctx, d: &Decision, ch: &Choice) -> String {
    use DecisionKind as K;
    let w = c.w;
    match (&d.kind, ch) {
        (K::TransferTalks { .. }, Choice::Accept) => "Open to talks".into(),
        (K::TransferTalks { .. }, Choice::Reject) => "Not interested".into(),
        (K::ContractOffer { .. } | K::FreeAgentOffer { .. }, Choice::Accept) => "Accept the offer".into(),
        (K::ContractOffer { .. } | K::FreeAgentOffer { .. }, Choice::Reject) => "Turn it down".into(),
        (K::LoanOffer { .. }, Choice::Accept) => "Accept the loan".into(),
        (K::LoanOffer { .. }, Choice::Reject) => "Stay where you are".into(),
        (K::Negotiation { .. }, Choice::Accept) => "Accept these terms".into(),
        (K::Negotiation { .. }, Choice::Reject) => "Walk away".into(),
        (K::Partner { ask: PartnerAsk::MoveIn, .. }, Choice::Accept) => "Move in together".into(),
        (K::Partner { ask: PartnerAsk::MoveIn, .. }, Choice::Reject) => "Not yet".into(),
        (K::Partner { ask: PartnerAsk::Marry, .. }, Choice::Accept) => "Say yes".into(),
        (K::Partner { ask: PartnerAsk::Marry, .. }, Choice::Reject) => "Say no".into(),
        (K::Partner { ask: PartnerAsk::Separate, .. }, Choice::Accept) => "Agree to separate".into(),
        (K::Partner { ask: PartnerAsk::Separate, .. }, Choice::Reject) => "Try to work things out".into(),
        (K::Trial { .. }, Choice::Accept) => "Accept the trial".into(),
        (K::Trial { .. }, Choice::Reject) => "Turn it down".into(),
        (K::NationChoice { nation, .. }, Choice::Accept) => format!("Play for {}", c.nation_name(*nation)),
        (K::NationChoice { other, .. }, Choice::Reject) => format!("Commit to {}", c.nation_name(*other)),
        (K::Treatment { .. }, Choice::Accept) => "Have the surgery".into(),
        (K::Treatment { .. }, Choice::Reject) => "Rehabilitate without surgery".into(),
        (K::Endorsement { .. }, Choice::Accept) => "Sign the deal".into(),
        (K::Endorsement { .. }, Choice::Reject) => "Turn it down".into(),
        (K::Meeting { meeting }, Choice::Respond(t)) => {
            let topic = w.meetings.list[*meeting].topic;
            let _ = topic;
            format!("Answer {}", t.label())
        }
        _ => choices::option(ch),
    }
}

fn option_kind(ch: &Choice) -> &'static str {
    match ch {
        Choice::Accept => "accept",
        Choice::Reject => "reject",
        Choice::Decline => "decline",
        Choice::Respond(_) => "respond",
        Choice::Counter { .. } => "counter",
    }
}

fn options_json(c: &Ctx, d: &Decision) -> Vec<Value> {
    d.options
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let mut v = json!({
                "i": i, "label": option_label(c, d, ch), "kind": option_kind(ch), "positive": ch.is_positive(),
                "default": i == usize::from(d.default),
            });
            match ch {
                Choice::Counter { wage, years, status, release_clause } => {
                    v["counter"] = json!({"wage": wage, "years": years, "status": status.map(|s| s.label()), "release_clause": release_clause});
                }
                Choice::Respond(t) => v["tone"] = json!(t.label()),
                _ => {}
            }
            v
        })
        .collect()
}

fn chosen_label(c: &Ctx, d: &Decision, i: u8) -> String {
    d.options.get(usize::from(i)).map_or_else(|| "an unavailable response".to_string(), |ch| option_label(c, d, ch))
}

// ---- terms and talks ---------------------------------------------------------------------------------

pub fn terms_rows(t: &Terms) -> Value {
    let mut rows = vec![
        json!({"label": "Wage per week", "money": t.wage}),
        json!({"label": "Length", "text": format!("{} year{}", t.years, if t.years == 1 { "" } else { "s" })}),
    ];
    if t.signing_fee > 0 {
        rows.push(json!({"label": "Signing fee", "money": t.signing_fee}));
    }
    if t.appearance_bonus > 0 {
        rows.push(json!({"label": "Appearance bonus", "money": t.appearance_bonus}));
    }
    if t.goal_bonus > 0 {
        rows.push(json!({"label": "Goal bonus", "money": t.goal_bonus}));
    }
    if t.clean_sheet_bonus > 0 {
        rows.push(json!({"label": "Clean sheet bonus", "money": t.clean_sheet_bonus}));
    }
    rows.push(if t.release_clause > 0 { json!({"label": "Release clause", "money": t.release_clause}) } else { json!({"label": "Release clause", "text": "None"}) });
    rows.push(json!({"label": "Promised role", "text": t.status.map_or("None".to_string(), |s| s.label().to_string())}));
    rows.push(json!({"label": "Yearly rise", "text": format!("{}%", t.yearly_rise)}));
    rows.push(json!({"label": "Cut if relegated", "text": format!("{}%", t.relegation_cut)}));
    if t.sell_on_to_player > 0 {
        rows.push(json!({"label": "Share of a future fee", "text": format!("{}%", t.sell_on_to_player)}));
    }
    Value::Array(rows)
}

fn talk_side(l: &TalkLine) -> &'static str {
    match l {
        TalkLine::ClubOffer(_) | TalkLine::ClubImproved(_) | TalkLine::ClubHeldFirm | TalkLine::ClubWalkedAway => "club",
        TalkLine::PlayerCounter(_) | TalkLine::PlayerAccepted | TalkLine::PlayerRejected => "you",
        TalkLine::AgentPressed => "agent",
    }
}

pub fn talk_json(c: &Ctx, n: &Negotiation) -> Value {
    let w = c.w;
    let state = match n.state {
        TalkState::PlayerTurn => "your_turn",
        TalkState::ClubTurn => "club_turn",
        TalkState::Agreed => "agreed",
        TalkState::Collapsed => "collapsed",
    };
    let agent = if n.agent.is_some() { Some(named(Ref::person(w.agents.list[n.agent].person), c.person_name(w.agents.list[n.agent].person))) } else { None };
    json!({
        "kind": n.kind.label(), "club": named(Ref::club(n.club), c.club_name(n.club)), "state": state,
        "round": n.round, "max_rounds": n.max_rounds, "opened": n.opened.0, "deadline": n.deadline.0,
        "agent": agent, "fee": if n.fee > 0 { json!(n.fee) } else { Value::Null },
        "seller": if n.seller.is_some() { Some(named(Ref::club(n.seller), c.club_name(n.seller))) } else { None },
        "offer": terms_rows(&n.offer),
        "log": n.log.iter().map(|(d, l)| json!({"date": d.0, "side": talk_side(l), "text": pw_narrate::talk::talk_line(l)})).collect::<Vec<_>>(),
    })
}

// ---- conversations ------------------------------------------------------------------------------------

fn who(c: &Ctx, p: pw_core::PersonId) -> String {
    if Some(p) == c.me() { "You".to_string() } else { c.person_name(p) }
}

pub fn meeting_json(c: &Ctx, m: &Meeting) -> Value {
    let w = c.w;
    let me = c.me().unwrap_or(pw_core::PersonId::NONE);
    let key = u64::from(m.id.0);
    let mut lines = vec![json!({
        "side": if m.initiator == me { "you" } else { "them" }, "name": who(c, m.initiator), "ref": Ref::person(m.initiator),
        "text": pw_narrate::talk::opener(m.topic, m.opening, key), "tone": m.opening.label(),
    })];
    let state = match m.state {
        MeetingState::Pending => "pending",
        MeetingState::Held => "held",
        MeetingState::Lapsed => "lapsed",
    };
    let mut outcomes: Vec<String> = Vec::new();
    if let (MeetingState::Held, Some(r)) = (m.state, m.response) {
        lines.push(json!({
            "side": if m.with == me { "you" } else { "them" }, "name": who(c, m.with), "ref": Ref::person(m.with),
            "text": pw_narrate::talk::reply(r, key ^ 0x99), "tone": r.label(),
        }));
        outcomes = m.outcomes.iter().map(|o| pw_narrate::talk::outcome(w, o, m, me)).collect();
    }
    json!({
        "topic": m.topic.label(), "date": m.date.0, "state": state, "lines": lines, "outcomes": outcomes,
        "with": named(Ref::person(if m.initiator == me { m.with } else { m.initiator }), c.person_name(if m.initiator == me { m.with } else { m.initiator })),
        "why": m.causes.iter().map(|x| pw_narrate::events::cause(w, x, me)).collect::<Vec<_>>(),
    })
}

// ---- the inbox ---------------------------------------------------------------------------------------

fn event_folder(k: &E) -> &'static str {
    match k {
        E::Meeting { .. } | E::PromiseMade { .. } | E::PromiseKept { .. } | E::PromiseBroken { .. } => "conversations",
        _ => match narrative::group_of(k) {
            Group::Transfers => "contracts",
            Group::Life => "life",
            Group::Media => "press",
            _ => "work",
        },
    }
}

pub fn inbox(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let _ = need_me(c).map(|_| ()).or_else(|e| if c.me().is_some() { Ok(()) } else { Err(e) })?;
    let me = c.me().expect("inhabiting");
    let w = c.w;
    let limit = args.get("limit").and_then(Value::as_u64).map_or(300, |n| n.clamp(20, 1000) as usize);
    let mut out: Vec<Value> = Vec::new();

    for (id, d) in w.decisions.all.iter_enumerated().rev().filter(|(_, d)| d.person == me).take(200) {
        let state = state_of(d);
        out.push(json!({
            "id": format!("d{}", id.0), "kind": "decision", "dkind": kind_key(&d.kind), "date": d.created.0,
            "subject": d.kind.title(), "preview": choices::title(w, d), "from": decision_from(c, d), "state": state,
            "folder": folder_of(d, state), "needs_action": state == "awaiting", "deadline": d.deadline.0, "important": state == "awaiting",
        }));
    }
    let since = w.date.add_days(-180);
    let mut n = 0usize;
    for e in w.events.all().iter().rev() {
        if e.date < since || n >= limit {
            break;
        }
        if !pw_career::feed::concerns(w, me, e) || !narrative::visible(c, e) {
            continue;
        }
        let parts = narrative::describe(c, e);
        n += 1;
        out.push(json!({
            "id": format!("e{}", e.id.0), "kind": "event", "date": e.date.0, "subject": narrative::label(&e.kind), "parts": parts,
            "from": Value::Null, "state": "info", "folder": event_folder(&e.kind), "needs_action": false, "deadline": Value::Null,
            "important": pw_career::feed::is_important(w, me, e), "unread": c.s.game.session.seen.0 < e.id.0,
        }));
    }
    out.sort_by(|a, b| b["date"].as_i64().cmp(&a["date"].as_i64()).then_with(|| b["id"].as_str().cmp(&a["id"].as_str())));
    let awaiting = out.iter().filter(|m| m["needs_action"] == true).count();
    let unread = out.iter().filter(|m| m["unread"] == true).count();
    Ok(json!({"messages": out, "awaiting": awaiting, "unread": unread}))
}

/// Everything a decision needs to be understood and answered.
pub fn decision_detail(c: &Ctx, did: DecisionId, d: &Decision) -> Value {
    let w = c.w;
    let me = c.me().unwrap_or(pw_core::PersonId::NONE);
    let state = state_of(d);
    // Once an offer is settled, "your current contract" and "what accepting means" describe a world that has moved on.
    let live = matches!(state, "awaiting" | "answered");
    let cold = c.my_player().map(|p| &w.players.cold[p]);
    let mut paragraphs: Vec<String> = vec![choices::title(w, d)];
    let mut consequences: Vec<String> = Vec::new();
    let mut terms = Value::Null;
    let mut current = Value::Null;
    let mut talk = Value::Null;
    let mut meeting = Value::Null;

    match &d.kind {
        DecisionKind::ContractOffer { club, contract, renewal } => {
            terms = contract_rows(c, contract);
            if *renewal {
                if let (true, Some(cold)) = (live, cold) {
                    paragraphs.push(format!("Your current contract runs until {}.", crate::fmt::date(cold.contract.end)));
                    current = contract_rows(c, &cold.contract);
                }
                consequences.push("Accepting replaces your current contract with the terms shown.".into());
                consequences.push("Declining leaves your current contract unchanged. The club may approach you again later.".into());
            } else {
                if let Some(deal) = w.market.pending.iter().find(|x| x.decision == did) {
                    paragraphs.push(format!("{} have agreed a fee with {} for you.", c.club_name(*club), c.club_name(deal.seller)));
                    paragraphs.push("The move happens only if you agree personal terms.".into());
                }
                consequences.push(format!("Accepting means moving to {} once the registration is completed.", c.club_name(*club)));
                consequences.push("Declining ends this approach; the club will not come back for you for a while.".into());
            }
        }
        DecisionKind::FreeAgentOffer { club, contract } => {
            terms = contract_rows(c, contract);
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
        DecisionKind::TransferTalks { .. } => consequences.push("Agreeing only opens talks; nothing is signed.".into()),
        DecisionKind::Negotiation { talk: t } => {
            let n = &w.talks[*t];
            talk = talk_json(c, n);
            terms = terms_rows(&n.offer);
            consequences.push("Accepting agrees the terms shown and ends the talks.".into());
            consequences.push("A counter-offer asks for more; the club may improve, hold firm or walk away.".into());
            consequences.push("Walking away ends the talks.".into());
        }
        DecisionKind::Meeting { meeting: m } => {
            meeting = meeting_json(c, &w.meetings.list[*m]);
            consequences.push("The tone you answer in is part of the record; both of you will remember it.".into());
        }
        DecisionKind::Partner { .. } => {
            consequences.push("Your answer changes your home life, and how you feel about your week.".into());
        }
        DecisionKind::Trial { club, days } => {
            consequences.push(format!("Accepting means training with {} for {days} days; they then decide whether to offer a contract.", c.club_name(*club)));
        }
        DecisionKind::NationChoice { .. } => {
            consequences.push("This cannot be undone: the nation you commit to is the one you play for from now on.".into());
        }
        DecisionKind::Treatment { .. } => {}
        DecisionKind::Endorsement { .. } => {}
    }
    if !live {
        consequences.clear();
    }
    let default_label = chosen_label(c, d, d.default);
    let outcome = match (state, d.answer) {
        ("settled", Some(a)) | ("answered", Some(a)) => Some(format!("You chose: {}", chosen_label(c, d, a))),
        ("expired", _) => Some(format!("No response was given. The default was applied: {default_label}.")),
        _ => None,
    };
    let _ = me;
    json!({
        "id": format!("d{}", did.0), "kind": "decision", "dkind": kind_key(&d.kind), "title": d.kind.title(), "from": decision_from(c, d),
        "created": d.created.0, "deadline": d.deadline.0, "state": state,
        "paragraphs": paragraphs, "options": options_json(c, d), "answer": d.answer,
        "default": {"i": d.default, "label": default_label},
        "without_response": if state != "awaiting" { Value::Null } else { json!(format!("If you do not respond by {}, the response your own judgement would give is applied: {}.", crate::fmt::date(d.deadline), default_label.to_lowercase())) },
        "consequences": consequences, "terms": terms, "current_terms": if live { current } else { Value::Null },
        "talk": talk, "meeting": meeting, "outcome": outcome,
    })
}

pub fn message(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let me = c.me().ok_or_else(|| ApiError::State("You are observing the world. Inhabit someone to read their messages.".into()))?;
    let id = args.get("id").and_then(Value::as_str).ok_or_else(|| ApiError::Bad("missing message id".into()))?;
    let w = c.w;
    if let Some(n) = id.strip_prefix('d').and_then(|s| s.parse::<u32>().ok()) {
        let did = DecisionId(n);
        let d = w.decisions.all.get(did).filter(|d| d.person == me).ok_or_else(|| ApiError::NotFound("message".into()))?;
        return Ok(decision_detail(c, did, d));
    }
    if let Some(n) = id.strip_prefix('e').and_then(|s| s.parse::<u32>().ok()) {
        let e = w
            .events
            .get(EventId(n))
            .filter(|e| pw_career::feed::concerns(w, me, e) || e.vis == pw_world::Visibility::Public)
            .filter(|e| narrative::visible(c, e))
            .ok_or_else(|| ApiError::NotFound("message".into()))?;
        let why: Vec<String> = e.causes.iter().map(|x: &Cause| pw_narrate::events::cause(w, x, me)).collect();
        let meeting = if let E::Meeting { meeting, .. } = e.kind { meeting_json(c, &w.meetings.list[meeting]) } else { Value::Null };
        let story = if let E::Published { story } = e.kind {
            let s = &w.media.stories[story];
            json!({"outlet": pw_narrate::press::outlet_name(w, s), "headline": pw_narrate::press::headline(w, s), "body": pw_narrate::press::body(w, s), "date": s.date.0})
        } else {
            Value::Null
        };
        return Ok(json!({
            "id": id, "kind": "event", "title": narrative::label(&e.kind), "date": e.date.0,
            "parts": narrative::describe(c, e), "primary": narrative::primary(c, &e.kind), "why": why,
            "meeting": meeting, "story": story,
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

/// Decisions waiting, briefly, for the Today page and the shell.
pub fn waiting(c: &Ctx) -> Vec<Value> {
    let Some(me) = c.me() else { return Vec::new() };
    c.w.decisions
        .pending_for(me)
        .filter(|(_, d)| d.answer.is_none())
        .map(|(id, d)| {
            json!({
                "id": format!("d{}", id.0), "dkind": kind_key(&d.kind), "title": d.kind.title(), "summary": choices::title(c.w, d),
                "deadline": d.deadline.0, "from": decision_from(c, d), "club": from_club(d, c).0,
            })
        })
        .collect()
}
