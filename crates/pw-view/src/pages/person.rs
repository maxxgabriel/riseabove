use pw_core::attr::AttrGroup;
use pw_core::{Attr, PersonId, Pos};
use pw_world::player::familiarity_label;
use pw_world::{PlayerStatus, StaffRole};
use serde_json::{Value, json};

use crate::ctx::{AttrView, Ctx};
use crate::visible::Assessment;
use crate::model::{ApiError, ApiResult, Named, Ref, Tone};

pub fn person_id(args: &Value) -> ApiResult<PersonId> {
    Ok(PersonId(crate::contract::request::<crate::contract::PersonReq>(args.clone())?.id))
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
        // What the viewer may know of him comes from the view type; everything below that is private is read from `seen` alone.
        let seen = c.visible_player(p);
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
        } else if let Some(injury) = &seen.injured {
            // Anyone can see he is out; the days and the diagnosis are the club's business (`Injury::medical`).
            match &injury.medical {
                Some(m) => (crate::fmt::singulars(format!("Injured, about {} days", m.days)), Tone::Neg, m.diagnosis.clone()),
                None => ("Injured".to_string(), Tone::Neg, String::new()),
            }
        } else if h.ban > 0 {
            (format!("Suspended for {} {}", h.ban, if h.ban == 1 { "match" } else { "matches" }), Tone::Warn, String::new())
        } else if h.status == PlayerStatus::FreeAgent {
            ("Free agent".to_string(), Tone::Muted, String::new())
        } else {
            ("Available".to_string(), Tone::Pos, String::new())
        };
        let contract = if let Some(terms) = seen.terms.as_ref().filter(|_| h.club.is_some()) {
            let k = &terms.contract;
            json!({
                "club": Named::new(Ref::club(k.club), c.club_name(k.club)),
                "wage": terms.wage_now,
                "start": k.start.0, "end": k.end.0, "days_left": terms.days_left,
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
        let held = c.unrevealed_apps(p);
        let career_unknown = w.origins.person(id).is_some_and(|o| o.get(pw_world::origin::Facet::Career) == pw_world::origin::Origin::Unknown);
        // (`contract` covers the squad status that goes with it.) A `null` above is one of three things; the first two are named here so a client does not have to guess: the viewer may not see it
        // (`hidden`), nobody has a record of it (`unknown`), or it does not apply (a free agent has no contract: in neither list).
        let hidden: Vec<&str> = [
            (seen.terms.is_none(), "contract"),
            (seen.value.is_none(), "value"),
            (seen.body.is_none(), "condition"),
            (seen.engine.is_none(), "internal"),
        ]
        .into_iter()
        .filter_map(|(is, name)| is.then_some(name))
        .collect();
        let unknown: Vec<&str> = if career_unknown { vec!["senior_apps", "senior_goals"] } else { Vec::new() };
        player_json = json!({
            "hidden": hidden, "unknown": unknown,
            "player_id": p.0,
            "best_pos": cold.best_pos.code(),
            "positions": positions,
            "foot": foot, "height": cold.height, "weight": cold.weight, "shirt": cold.shirt,
            "traits": cold.traits.labels().collect::<Vec<_>>(),
            "team": if h.team.is_some() { Value::String(w.teams[h.team].kind.label().into()) } else { Value::Null },
            "loan": loan,
            "availability": {"label": avail_label, "tone": tone_str(avail_tone), "detail": avail_detail},
            "contract": contract,
            "squad_status": match &seen.terms { Some(t) if h.club.is_some() => Value::String(t.squad_status.label().into()), _ => Value::Null },
            "value": match seen.value { Some(v) => json!(v), None => Value::Null },
            "condition": match &seen.body { Some(b) => json!({
                "condition": b.condition, "sharpness": b.sharpness, "fitness": b.fitness, "fatigue": b.fatigue,
                "morale": b.morale, "confidence": b.confidence, "wellbeing": b.wellbeing,
            }), None => Value::Null },
            "form": c.visible_form(p),
            "caps": cold.caps, "intl_goals": cold.intl_goals,
            "senior_apps": if career_unknown { Value::Null } else { json!(cold.senior_apps.saturating_sub(held.len() as u16)) },
            "senior_goals": if career_unknown { Value::Null } else { json!(cold.senior_goals.saturating_sub(held.iter().map(|a| u16::from(a.goals)).sum())) },
            "career_coverage": if career_unknown { "unknown" } else if w.origins.person(id).is_some() { "source records" } else { "complete" },
            "joined": if h.club.is_some() { json!(cold.joined.0) } else { Value::Null },
            "youth_club": if cold.youth_club.is_some() { serde_json::to_value(Named::new(Ref::club(cold.youth_club), c.club_name(cold.youth_club))).unwrap() } else { Value::Null },
            "internal": match &seen.engine { Some(e) => json!({
                "ca": e.ability, "pa": e.potential, "reputation": {"current": e.reputation.current, "home": e.reputation.home, "world": e.reputation.world},
                "personality": e.personality, "bio_offset": e.bio_offset,
                "plan": {"focus": format!("{:?}", e.focus), "intensity": format!("{:?}", e.intensity)},
            }), None => Value::Null },
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

fn attr_json(a: Attr, v: AttrView) -> Value {
    match v {
        AttrView::Exact(v) => json!({"key": a.key(), "label": a.label(), "kind": "exact", "v": v}),
        AttrView::Range { lo, hi, mid } => json!({"key": a.key(), "label": a.label(), "kind": "range", "lo": lo, "hi": hi, "v": mid.round()}),
        AttrView::Unknown => json!({"key": a.key(), "label": a.label(), "kind": "unknown"}),
    }
}

pub fn attributes(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = person_id(args)?;
    let person = c.w.people.get(id).ok_or_else(|| ApiError::NotFound(format!("person {}", id.0)))?;
    let Some(p) = person.player.get() else {
        return Ok(json!({
            "available": false, "reason": "This person is not a player.", "source": "", "known": false, "groups": [], "positions": [],
            "hidden": null, "internal": null, "personality": null,
        }));
    };
    // The readout is built from what this viewer may know of him and from nothing else.
    let ability = c.visible_ability(p);
    let groups: Vec<Value> = [AttrGroup::Technical, AttrGroup::Mental, AttrGroup::Physical, AttrGroup::Goalkeeping]
        .into_iter()
        .filter(|g| *g != AttrGroup::Goalkeeping || ability.shows_goalkeeping)
        .map(|g| {
            let name = match g {
                AttrGroup::Technical => "Technical",
                AttrGroup::Mental => "Mental",
                AttrGroup::Physical => "Physical",
                AttrGroup::Goalkeeping => "Goalkeeping",
            };
            let attrs: Vec<Value> = ability.attrs.iter().filter(|(a, _)| a.group() == g).map(|&(a, v)| attr_json(a, v)).collect();
            json!({"name": name, "attrs": attrs})
        })
        .collect();

    let (source, known) = match ability.assessment {
        Assessment::Omniscient => ("Exact values, as the simulation holds them (observer view).".to_string(), true),
        Assessment::Staff { club, last, minutes } => (format!("Assessed by the coaching staff at {}. Last watched {}; about {minutes} minutes of evidence.", c.club_name(club), crate::fmt::date(last)), true),
        Assessment::Own => ("Your own assessment of yourself.".to_string(), true),
        Assessment::Nobody => ("Nobody at your club has watched this player enough to assess them.".to_string(), false),
    };
    let hidden = match &ability.hidden {
        Some(h) => json!(h.iter().map(|(label, v)| json!({"label": label, "v": v})).collect::<Vec<_>>()),
        None => Value::Null,
    };
    let positions: Vec<Value> = ability.positions.iter().map(|(ps, fam)| json!({"code": ps.code(), "fam": fam, "level": familiarity_label(*fam)})).collect();
    Ok(json!({
        "available": true, "reason": null, "source": source, "known": known, "groups": groups, "positions": positions,
        "hidden": hidden,
        "internal": match &ability.engine { Some(e) => json!({"ca": e.ability, "pa": e.potential}), None => Value::Null },
        "personality": match &ability.engine { Some(e) => json!(e.personality), None => Value::Null },
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
    Ok(crate::contract::wire(crate::contract::Created { person: person.0 }))
}
