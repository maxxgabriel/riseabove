//! Beginning a life somewhere on the route, in a world that has one (the India pathway).
//!
//! The world makes the person with its own generator, in a district the human picks, at one of the places a child or young
//! adult can be on the way up. Talent is never chosen. Everything after the start follows the ordinary systems.

use pw_core::RegionId;
use pw_sim::ecosystem::{Start, begin};
use pw_world::ecosystem::RegionKind;
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult};
use crate::session::Session;

/// Every start, in the order of the route: its key, its name and one plain sentence.
const STARTS: [(Start, &str, &str, &str); 6] = [
    (Start::SchoolStandout, "school_standout", "School standout", "Standing out at a school in the district. No club has noticed you yet."),
    (Start::ReleasedAcademy, "released_academy", "Released by an academy", "An academy near home has let you go. You still want this."),
    (Start::UniversityFreshman, "university_freshman", "University freshman", "Just arrived at a university on a tuition scholarship."),
    (Start::UniversityStar, "university_star", "University star", "Three years into a university place and playing for it."),
    (Start::StateLeague, "state_league", "State league", "Playing in the state's premier league."),
    (Start::SemiPro, "semi_pro", "Semi-professional", "In the national fourth tier, one step from the leagues people watch."),
];

/// What can be chosen: the starts, and the states with their districts. `available` is false in a world without a route.
pub fn options(c: &Ctx) -> ApiResult<Value> {
    let e = &c.w.ext.ecosystem;
    if !e.is_configured() {
        return Ok(json!({"available": false, "starts": [], "states": []}));
    }
    let starts: Vec<Value> = STARTS.iter().map(|(s, key, label, blurb)| json!({"key": key, "label": label, "age": s.age(), "blurb": blurb})).collect();
    let states: Vec<Value> = e
        .regions
        .iter_enumerated()
        .filter(|(_, r)| r.kind == RegionKind::State)
        .map(|(sid, s)| {
            let districts: Vec<Value> = e
                .regions
                .iter_enumerated()
                .filter(|(_, d)| d.kind == RegionKind::District && d.parent == sid)
                .map(|(did, d)| json!({"id": did.0, "name": d.name, "population_k": d.population_k}))
                .collect();
            json!({"id": sid.0, "name": s.name, "districts": districts})
        })
        .filter(|s| s["districts"].as_array().is_some_and(|d| !d.is_empty()))
        .collect();
    Ok(json!({"available": true, "starts": starts, "states": states}))
}

/// Make a person at the chosen start and district, then step into them.
pub fn begin_route(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let key = args.get("start").and_then(Value::as_str).ok_or_else(|| ApiError::Bad("Choose where to begin.".into()))?;
    let start = STARTS.iter().find(|(_, k, _, _)| *k == key).map(|x| x.0).ok_or_else(|| ApiError::Bad("That is not a place to begin.".into()))?;
    if !s.w().ext.ecosystem.is_configured() {
        return Err(ApiError::State("This world has no route to begin on.".into()));
    }
    let region = match args.get("district").and_then(Value::as_u64) {
        Some(n) => {
            let id = RegionId(n as u32);
            let e = &s.w().ext.ecosystem;
            if (n as usize) >= e.regions.len() || e.regions[id].kind != RegionKind::District {
                return Err(ApiError::Bad("Choose a district.".into()));
            }
            id
        }
        None => RegionId::NONE,
    };
    let text = |k: &str| args.get(k).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()).map(str::to_string);
    let (first, last) = (text("first"), text("last"));
    let salt = s.w().seed ^ u64::from(s.today().0 as u32).rotate_left(21) ^ s.w().people.len() as u64;
    let w = &mut s.game.sim.world;
    let p = begin(w, start, region, salt).ok_or_else(|| ApiError::State("There is no school, university or club in that district for that start. Try another district or start.".into()))?;
    let person = w.players.cold[p].person;
    if let Some(f) = first {
        let id = w.names.intern(&f);
        w.people[person].first = id;
    }
    if let Some(l) = last {
        let id = w.names.intern(&l);
        w.people[person].last = id;
    }
    s.inhabit(person, salt)?;
    Ok(json!({"person": person.0}))
}
