use pw_core::attr::AttrGroup;
use pw_core::{Attr, Hidden, PersonId, PlayerId, Pos};
use pw_sim::health;
use pw_world::player::familiarity_label;
use pw_world::{PlayerStatus, StaffRole};
use serde_json::{Value, json};

use crate::ctx::{AttrView, Ctx};
use crate::model::{ApiError, ApiResult, Named, Ref, Tone};

pub fn person_id(args: &Value) -> ApiResult<PersonId> {
    args.get("id").and_then(Value::as_u64).map(|v| PersonId(v as u32)).ok_or_else(|| ApiError::Bad("missing person id".into()))
}

fn tone_str(t: Tone) -> &'static str {
    match t {
        Tone::Pos => "pos",
        Tone::Neg => "neg",
        Tone::Warn => "warn",
        Tone::Muted => "muted",
        Tone::Info => "info",
    }
}

fn nation_named(c: &Ctx, n: pw_core::NationId) -> Value {
    if n.is_none() {
        return Value::Null;
    }
    serde_json::to_value(Named::new(Ref::nation(n), c.nation_name(n))).unwrap_or(Value::Null)
}

pub fn get(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = person_id(args)?;
    let person = c.w.people.get(id).ok_or_else(|| ApiError::NotFound(format!("person {}", id.0)))?;
    let w = c.w;
    let name = c.person_name(id);
    let initials: String = name.split_whitespace().filter_map(|s| s.chars().next()).take(2).collect();
    let mut nations = vec![nation_named(c, person.nation)];
    if person.nation2.is_some() {
        nations.push(nation_named(c, person.nation2));
    }

    let mut roles = Vec::new();
    let mut player_json = Value::Null;
    let mut staff_json = Value::Null;
    let mut status = "Person".to_string();
    let mut can_inhabit = false;

    if let Some(p) = person.player.get() {
        let h = &w.players.hot[p];
        let cold = &w.players.cold[p];
        status = c.status_label(p).to_string();
        can_inhabit = h.status == PlayerStatus::Active && h.club.is_some() && !c.is_me(p);
        if h.status == PlayerStatus::Active {
            let team = if h.team.is_some() { w.teams[h.team].kind.label() } else { "" };
            roles.push(json!({
                "label": format!("Player{}", if team.is_empty() { String::new() } else { format!(" · {team}") }),
                "org": if h.club.is_some() { serde_json::to_value(Named::new(Ref::club(h.club), c.club_name(h.club))).unwrap() } else { Value::Null },
            }));
        } else if h.status == PlayerStatus::FreeAgent {
            roles.push(json!({"label": "Player · without a club", "org": Value::Null}));
        } else {
            roles.push(json!({"label": "Retired player", "org": Value::Null}));
        }

        let positions: Vec<Value> = Pos::ALL
            .iter()
            .filter(|ps| cold.familiarity[ps.idx()] >= 8)
            .map(|ps| json!({"code": ps.code(), "level": familiarity_label(cold.familiarity[ps.idx()]), "fam": cold.familiarity[ps.idx()]}))
            .collect();
        let foot = {
            let (l, r) = (cold.left_foot, cold.right_foot);
            if l >= 15 && r >= 15 {
                "Either foot"
            } else if l > r {
                "Left foot"
            } else {
                "Right foot"
            }
        };
        let (avail_label, avail_tone, avail_detail) = if h.status == PlayerStatus::Retired {
            ("Retired".to_string(), Tone::Muted, String::new())
        } else if h.injury != 0 {
            (crate::fmt::singulars(format!("Injured, about {} days", h.injury_days)), Tone::Neg, health::injury_name(w, h.injury).to_string())
        } else if h.ban > 0 {
            (format!("Suspended for {} {}", h.ban, if h.ban == 1 { "match" } else { "matches" }), Tone::Warn, String::new())
        } else if h.status == PlayerStatus::FreeAgent {
            ("Free agent".to_string(), Tone::Muted, String::new())
        } else {
            ("Available".to_string(), Tone::Pos, String::new())
        };
        let contract = if c.sees_contract(p) && h.club.is_some() {
            let k = &cold.contract;
            json!({
                "club": Named::new(Ref::club(k.club), c.club_name(k.club)),
                "wage": k.current_wage(w.date),
                "start": k.start.0, "end": k.end.0, "days_left": k.days_left(w.date),
                "release_clause": k.release_clause, "promised_status": k.promised_status.map(|s| s.label()),
                "appearance_bonus": k.appearance_bonus, "goal_bonus": k.goal_bonus,
                "assist_bonus": k.assist_bonus, "clean_sheet_bonus": k.clean_sheet_bonus, "loyalty_bonus": k.loyalty_bonus,
                "title_bonus": k.title_bonus, "promotion_bonus": k.promotion_bonus, "continental_bonus": k.continental_bonus, "cap_bonus": k.cap_bonus,
                "relegation_release": k.relegation_release,
                "options": crate::pages::inbox::options_text(&k.options).into_iter().map(|(l, t)| json!({"label": l, "text": t})).collect::<Vec<_>>(),
                "yearly_rise": k.yearly_rise, "relegation_cut": k.relegation_cut,
                "kind": format!("{:?}", k.kind),
            })
        } else {
            Value::Null
        };
        let loan = cold.loan.as_ref().map(|l| json!({"parent": Named::new(Ref::club(l.parent), c.club_name(l.parent)), "club": Named::new(Ref::club(l.club), c.club_name(l.club)), "end": l.end.0}));
        let visible_state = c.sees_condition(p);
        let held = c.unrevealed_apps(p);
        let career_unknown = w.origins.person(id).is_some_and(|o| o.get(pw_world::origin::Facet::Career) == pw_world::origin::Origin::Unknown);
        player_json = json!({
            "player_id": p.0,
            "best_pos": cold.best_pos.code(),
            "positions": positions,
            "foot": foot, "height": cold.height, "weight": cold.weight, "shirt": cold.shirt,
            "traits": cold.traits.labels().collect::<Vec<_>>(),
            "team": if h.team.is_some() { Value::String(w.teams[h.team].kind.label().into()) } else { Value::Null },
            "loan": loan,
            "availability": {"label": avail_label, "tone": tone_str(avail_tone), "detail": avail_detail},
            "contract": contract,
            "squad_status": if c.sees_contract(p) && h.club.is_some() { Value::String(cold.status.label().into()) } else { Value::Null },
            "value": if c.sees_value(p) { json!(cold.value) } else { Value::Null },
            "condition": if visible_state { json!({
                "condition": h.condition, "sharpness": h.sharpness, "fitness": h.fitness, "fatigue": h.fatigue,
                "morale": h.morale, "confidence": h.confidence, "wellbeing": h.wellbeing,
            }) } else { Value::Null },
            "form": c.visible_form(p),
            "caps": cold.caps, "intl_goals": cold.intl_goals,
            "senior_apps": if career_unknown { Value::Null } else { json!(cold.senior_apps.saturating_sub(held.len() as u16)) },
            "senior_goals": if career_unknown { Value::Null } else { json!(cold.senior_goals.saturating_sub(held.iter().map(|a| u16::from(a.goals)).sum())) },
            "career_coverage": if career_unknown { "unknown" } else if w.origins.person(id).is_some() { "source records" } else { "complete" },
            "joined": if h.club.is_some() { json!(cold.joined.0) } else { Value::Null },
            "youth_club": if cold.youth_club.is_some() { serde_json::to_value(Named::new(Ref::club(cold.youth_club), c.club_name(cold.youth_club))).unwrap() } else { Value::Null },
            "internal": if c.sees_internal_state() { json!({
                "ca": cold.ca, "pa": cold.pa, "reputation": {"current": cold.rep.current, "home": cold.rep.home, "world": cold.rep.world},
                "personality": person.hidden.personality_label(), "bio_offset": cold.bio_offset,
                "plan": {"focus": format!("{:?}", cold.plan.focus), "intensity": format!("{:?}", cold.plan.intensity)},
            }) } else { Value::Null },
        });
    }

    if let Some(s) = person.staff.get() {
        let st = &w.staff[s];
        if st.employed() {
            roles.push(json!({"label": st.role.label(), "org": serde_json::to_value(Named::new(Ref::club(st.club), c.club_name(st.club))).unwrap()}));
        } else if st.retired {
            roles.push(json!({"label": format!("Former {}", st.role.label().to_lowercase()), "org": Value::Null}));
        } else {
            roles.push(json!({"label": format!("{} · without a club", st.role.label()), "org": Value::Null}));
        }
        if person.player.is_none() {
            status = if st.employed() {
                "Employed"
            } else if st.retired {
                "Retired"
            } else {
                "Unemployed"
            }
            .into();
        }
        let ph = &st.philosophy;
        let formation = |i: u8| w.data.formations.get(usize::from(i)).map(|f| f.name.clone());
        staff_json = json!({
            "staff_id": s.0, "role": st.role.label(),
            "club": if st.club.is_some() { serde_json::to_value(Named::new(Ref::club(st.club), c.club_name(st.club))).unwrap() } else { Value::Null },
            "reputation": st.reputation, "joined": st.joined.0,
            "record": {"games": st.record.games, "wins": st.record.wins, "draws": st.record.draws, "losses": st.record.losses, "trophies": st.record.trophies, "sackings": st.record.sackings},
            "style": if st.role == StaffRole::Manager { json!({
                "formations": ph.formations.iter().filter_map(|&f| formation(f)).collect::<Vec<_>>(),
                "mentality": ph.mentality, "press": ph.press, "tempo": ph.tempo, "directness": ph.directness,
                "archetype": format!("{:?}", ph.archetype),
            }) } else { Value::Null },
            "contract": if c.observer() { json!({"wage": st.wage, "end": st.contract_end.0}) } else { Value::Null },
            "attrs": if c.observer() { json!(pw_core::StaffAttr::ALL.iter().map(|a| json!({"label": a.label(), "v": st.attrs.get(*a)})).collect::<Vec<_>>()) } else { Value::Null },
            "role_rating": if c.observer() { json!(st.role_rating(st.role)) } else { Value::Null },
        });
    }

    let is_me = c.me() == Some(id);
    Ok(json!({
        "id": id.0, "name": name, "short": c.person_short(id), "initials": initials.to_uppercase(),
        "age": person.age(w.date), "dob": person.dob.0, "nations": nations,
        "status": status, "roles": roles, "player": player_json, "staff": staff_json,
        "is_me": is_me, "can_inhabit": can_inhabit,
        "perspective": match c.view_mode() { "omniscient" => "observer", m => m },
        "provenance": if c.observer() { provenance(c, id) } else { Value::Null },
    }))
}

/// Where an imported person's record came from and which of its facts the importer had to fill in. This describes the data,
/// not the person, so only an observer sees it. People the game made itself have none.
fn provenance(c: &Ctx, id: PersonId) -> Value {
    let book = &c.w.origins;
    let Some(o) = book.person(id) else { return Value::Null };
    json!({
        "source": book.sources.get(usize::from(o.src.source)).map(|s| s.name.clone()),
        "id": o.src.id,
        "snapshot": book.sources.get(usize::from(o.src.source)).and_then(|s| s.snapshot).map(|d| d.0),
        "facts": pw_world::origin::Facet::ALL.iter().map(|f| json!({"group": f.label(), "origin": o.get(*f).label()})).collect::<Vec<_>>(),
    })
}

fn attr_json(c: &Ctx, p: PlayerId, a: Attr) -> Value {
    match c.attr_view(p, a) {
        AttrView::Exact(v) => json!({"key": a.key(), "label": a.label(), "kind": "exact", "v": v}),
        AttrView::Range { lo, hi, mid } => json!({"key": a.key(), "label": a.label(), "kind": "range", "lo": lo, "hi": hi, "v": mid.round()}),
        AttrView::Unknown => json!({"key": a.key(), "label": a.label(), "kind": "unknown"}),
    }
}

pub fn attributes(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = person_id(args)?;
    let person = c.w.people.get(id).ok_or_else(|| ApiError::NotFound(format!("person {}", id.0)))?;
    let Some(p) = person.player.get() else {
        return Ok(json!({"available": false, "reason": "This person is not a player."}));
    };
    let w = c.w;
    let cold = &w.players.cold[p];
    let keeper = cold.best_pos == Pos::GK;
    let groups: Vec<Value> = [AttrGroup::Technical, AttrGroup::Mental, AttrGroup::Physical, AttrGroup::Goalkeeping]
        .into_iter()
        .filter(|g| *g != AttrGroup::Goalkeeping || keeper || c.observer())
        .map(|g| {
            let name = match g {
                AttrGroup::Technical => "Technical",
                AttrGroup::Mental => "Mental",
                AttrGroup::Physical => "Physical",
                AttrGroup::Goalkeeping => "Goalkeeping",
            };
            let attrs: Vec<Value> = Attr::ALL.iter().filter(|a| a.group() == g).map(|a| attr_json(c, p, *a)).collect();
            json!({"name": name, "attrs": attrs})
        })
        .collect();

    let (source, known) = if c.observer() {
        ("Exact values, as the simulation holds them (observer view).".to_string(), true)
    } else {
        let club = c.my_club();
        let seen = if club.is_some() { w.knowledge.seen(club, p) } else { None };
        match seen {
            Some(s) => (format!("Assessed by the coaching staff at {}. Last watched {}; about {} minutes of evidence.", c.club_name(club), crate::fmt::date(s.last), s.minutes), true),
            None if c.is_me(p) => ("Your own assessment of yourself.".to_string(), true),
            None => ("Nobody at your club has watched this player enough to assess them.".to_string(), false),
        }
    };
    let hidden = if c.sees_internal_state() { json!(Hidden::ALL.iter().map(|h| json!({"label": h.label(), "v": person.hidden.get(*h)})).collect::<Vec<_>>()) } else { Value::Null };
    let positions: Vec<Value> = Pos::ALL.iter().map(|ps| json!({"code": ps.code(), "fam": cold.familiarity[ps.idx()], "level": familiarity_label(cold.familiarity[ps.idx()])})).collect();
    Ok(json!({
        "available": true, "source": source, "known": known, "groups": groups, "positions": positions,
        "hidden": hidden,
        "internal": if c.sees_internal_state() { json!({"ca": cold.ca, "pa": cold.pa}) } else { Value::Null },
        "personality": if c.sees_internal_state() { json!(person.hidden.personality_label()) } else { Value::Null },
    }))
}

/// Bring a new person into the world with the world's own generator, then step into them.
/// Talent is never chosen: potential comes from the club's own intake and stays hidden.
pub fn create(s: &mut crate::session::Session, args: &Value) -> ApiResult<Value> {
    let req: crate::contract::CreatePersonReq = crate::contract::request(args.clone())?;
    let first = req.first.as_deref().unwrap_or("").trim().to_string();
    let last = req.last.as_deref().unwrap_or("").trim().to_string();
    if first.is_empty() || last.is_empty() {
        return Err(ApiError::Bad("Give them a first and a last name.".into()));
    }
    let age = req.age.unwrap_or(17).clamp(8, 40) as u8;
    let pos = req.pos.as_deref().and_then(pw_core::Pos::from_code).ok_or_else(|| ApiError::Bad("Choose a position.".into()))?;
    let club = req.club.map_or(pw_core::ClubId::NONE, pw_core::ClubId);
    if club.is_some() && club.0 as usize >= s.w().clubs.len() {
        return Err(ApiError::NotFound(format!("club {}", club.0)));
    }
    let nation = req.nation.map_or(pw_core::NationId::NONE, pw_core::NationId);
    if nation.is_some() && nation.0 as usize >= s.w().nations.len() {
        return Err(ApiError::NotFound(format!("nation {}", nation.0)));
    }
    let salt = s.w().seed ^ u64::from(s.today().0 as u32).rotate_left(21) ^ s.w().people.len() as u64;
    let (person, _) = pw_career::create_person(&mut s.game.sim.world, pw_career::NewPerson { first, last, nation, club, age, pos, salt });
    s.inhabit(person, salt)?;
    Ok(json!({"person": person.0}))
}
