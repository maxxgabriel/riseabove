//! `institution`: a school, sports school or university and its football. Public facts only: its name, kind, where it is, its standing
//! in words, its teams this season, who plays for it, who went on to play professionally and what it has won. Its coaching quality
//! (a number the table keeps for the omniscient view) is not shown here.

use pw_world::minor::{Entrant, InstKind};
use serde_json::Value;

use crate::contract::{InstitutionPerson, InstitutionTeam, InstitutionTitle, InstitutionView};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};

/// Players listed at most, current and former.
const LISTED: usize = 40;

fn standing_words(prestige: u16) -> &'static str {
    match prestige {
        800.. => "One of the most respected in the country",
        600..=799 => "Well regarded beyond its state",
        400..=599 => "Known across its region",
        200..=399 => "A modest name",
        _ => "Little known outside its town",
    }
}

fn success_words(s: f32) -> &'static str {
    match s {
        x if x >= 70.0 => "Winning: one of the strongest sides at its level",
        x if x >= 45.0 => "Competitive: it holds its own",
        x if x >= 20.0 => "Middling: some good seasons, some poor ones",
        _ => "Struggling: results have been hard to come by",
    }
}

fn facilities_words(f: f32) -> &'static str {
    match f {
        x if x >= 70.0 => "Good pitches, a gym and proper changing rooms",
        x if x >= 40.0 => "A decent pitch of its own",
        x if x >= 15.0 => "Basic: a ground shared with everything else",
        _ => "Very little: the team plays where it can",
    }
}

fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn season_words(season: i32) -> String {
    format!("{}/{:02}", season, (season + 1) % 100)
}

pub fn get(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = crate::contract::request::<crate::contract::IdReq>(args.clone())?.id;
    let w = c.w;
    let Some(inst) = w.minor.institutions.get(id as usize) else { return Err(ApiError::NotFound(format!("institution {id}"))) };
    let eco = &w.ext.ecosystem;
    let profile = eco.inst.get(&id);
    let me_entrant = Entrant::Inst(id);

    let kind = match (inst.kind, profile.is_some_and(|p| p.residential)) {
        (InstKind::School, false) => "School",
        (InstKind::School, true) => "Sports school (residential)",
        (InstKind::University, false) => "University",
        (InstKind::University, true) => "University (residential programme)",
    };
    let region = profile.map(|p| p.region).filter(|r| r.is_some() && eco.regions.get(*r).is_some());
    let state = region.map(|r| eco.state_of(r)).filter(|s| Some(*s) != region).and_then(|s| eco.regions.get(s)).map(|s| s.name.clone());

    // This season's competitions it plays in.
    let mut teams: Vec<InstitutionTeam> = Vec::new();
    for comp in w.minor.comps.iter().filter(|m| m.entrants.contains(&me_entrant)) {
        let name = pw_narrate::history::comp_name(w, comp.kind, comp.nation, &comp.region);
        let (standing, played, points) = if comp.kind.is_cup() {
            let s = if comp.done && comp.alive.len() == 1 && comp.alive[0] == me_entrant {
                "Won it"
            } else if comp.alive.contains(&me_entrant) && !comp.done {
                "Still in it"
            } else {
                "Knocked out"
            };
            (Some(s.to_string()), None, None)
        } else {
            let mut rows: Vec<&(Entrant, pw_world::minor::Row)> = comp.table.iter().collect();
            rows.sort_by_key(|(e, r)| (std::cmp::Reverse(r.pts), std::cmp::Reverse(i32::from(r.gf) - i32::from(r.ga)), std::cmp::Reverse(r.gf), *e));
            let pos = rows.iter().position(|(e, _)| *e == me_entrant);
            let row = rows.iter().find(|(e, _)| *e == me_entrant).map(|(_, r)| *r);
            let played = row.map(|r| u32::from(r.p)).filter(|p| *p > 0);
            (
                pos.filter(|_| played.is_some()).map(|p| format!("{} of {}", ordinal(p + 1), rows.len())),
                played,
                row.filter(|r| r.p > 0).map(|r| u32::from(r.pts)),
            )
        };
        teams.push(InstitutionTeam { comp: name, season: season_words(comp.season), standing, played, points });
    }

    let person_of = |p: pw_core::PlayerId| w.players.cold.get(p).map(|x| x.person).filter(|q| q.is_some() && w.people.get(*q).is_some());
    let mut players: Vec<InstitutionPerson> = inst
        .members
        .iter()
        .filter_map(|&p| person_of(p).map(|q| (p, q)))
        .map(|(p, q)| InstitutionPerson {
            who: Named::new(Ref::person(q), c.person_name(q)),
            age: c.age(q),
            now: w.minor.enrolled.get(&p).map(|y| format!("Enrolled {y}")),
        })
        .collect();
    players.sort_by(|a, b| b.age.cmp(&a.age).then(a.who.name.cmp(&b.who.name)));
    players.truncate(LISTED);
    let mut alumni: Vec<InstitutionPerson> = inst
        .alumni_pros
        .iter()
        .filter(|q| q.is_some() && w.people.get(**q).is_some())
        .map(|&q| InstitutionPerson { who: Named::new(Ref::person(q), c.person_name(q)), age: c.age(q), now: super::chronicle::now_words(c, q) })
        .collect();
    alumni.sort_by(|a, b| a.age.cmp(&b.age).then(a.who.name.cmp(&b.who.name)));
    alumni.truncate(LISTED);

    let mut titles: Vec<(i32, InstitutionTitle)> = w
        .minor
        .history
        .iter()
        .filter(|h| h.winner == me_entrant || h.runner_up == me_entrant)
        .map(|h| {
            (
                h.season,
                InstitutionTitle {
                    comp: pw_narrate::history::comp_name(w, h.kind, h.nation, &h.region),
                    season: season_words(h.season),
                    finish: if h.winner == me_entrant { "won" } else { "runner_up" }.into(),
                },
            )
        })
        .collect();
    titles.sort_by_key(|(s, t)| (std::cmp::Reverse(*s), t.finish != "won"));

    // Your own connection: your team now, or one your story says you played for.
    let yours = c.me().and_then(|me| {
        let mine = c.my_player().is_some_and(|p| w.minor.member_of.get(&p) == Some(&id));
        if mine {
            return Some(if inst.kind == InstKind::University { "Your university: you play for its team" } else { "Your school: you play for its team" }.to_string());
        }
        let life = w.ext.chronicle.of(me)?;
        use pw_world::chronicle::Line;
        life.entries
            .iter()
            .any(|e| match e.line {
                Line::Began { institution, .. } => institution == Some(id),
                Line::Enrolled { inst } | Line::Graduated { inst } | Line::Scholarship { inst, .. } => inst == id,
                _ => false,
            })
            .then(|| "You played here".to_string())
    });

    let view = InstitutionView {
        id,
        name: inst.name.clone(),
        kind: kind.into(),
        nation: Named::new(Ref::nation(inst.nation), c.nation_name(inst.nation)),
        city: (!inst.city.trim().is_empty()).then(|| inst.city.clone()),
        region: region.and_then(|r| eco.regions.get(r)).map(|r| r.name.clone()),
        state,
        founded: (inst.founded > 0).then_some(inst.founded),
        standing: standing_words(inst.prestige).into(),
        football: profile.map(|p| success_words(p.success).to_string()),
        facilities: profile.map(|p| facilities_words(p.facilities).to_string()),
        scholarships: profile.map(|p| u32::from(p.scholarships)).filter(|n| *n > 0),
        origin: if profile.is_some_and(|p| p.real) { "Imported" } else { "Generated" }.into(),
        teams,
        players,
        alumni,
        titles: titles.into_iter().map(|(_, t)| t).take(30).collect(),
        yours,
    };
    serde_json::to_value(view).map_err(|e| ApiError::Internal(e.to_string()))
}
