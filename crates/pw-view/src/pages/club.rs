use pw_core::{ClubId, CompId, NationId};
use pw_world::comp::Stage;
use pw_world::club::Ownership;
use pw_world::{CompKind, StaffRole, TeamKind};
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};
use crate::tables::comp_summary_stage;

fn named(r: Ref, n: String) -> Value {
    serde_json::to_value(Named::new(r, n)).unwrap_or(Value::Null)
}

pub fn get(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = ClubId(args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing club id".into()))? as u32);
    if id.0 as usize >= c.w.clubs.len() {
        return Err(ApiError::NotFound(format!("club {}", id.0)));
    }
    let w = c.w;
    let club = &w.clubs[id];
    let first = club.first_team();
    let league = w.league_of(first);
    let league_json = league.map(|l| {
        let comp = &w.comps[l];
        let (rows, _) = crate::tables::visible_table(c, l);
        let pos = rows.iter().position(|r| r.team == first).map(|p| p + 1);
        let row = rows.iter().find(|r| r.team == first);
        json!({
            "comp": named(Ref::comp(l), comp.name.clone()), "position": pos, "teams": rows.len(),
            "points": row.map(|r| r.points), "played": row.map(|r| r.played),
        })
    });
    let teams: Vec<Value> = club
        .teams
        .iter()
        .map(|&t| {
            let team = &w.teams[t];
            json!({
                "team": t.0, "kind": team.kind.label(), "kind_key": match team.kind { TeamKind::First => "first", TeamKind::Reserve => "reserve", TeamKind::U21 => "u21", TeamKind::U19 => "u19", TeamKind::U18 => "u18", TeamKind::U16 => "u16", TeamKind::U14 => "u14", TeamKind::U12 => "u12" },
                "squad": team.squad.len(),
                "comp": w.league_of(t).map(|l| named(Ref::comp(l), c.comp_short(l))),
                "captain": if team.captain.is_some() { Some(named(c.player_ref(team.captain), c.player_name(team.captain))) } else { None },
            })
        })
        .collect();
    let manager = (club.manager.is_some()).then(|| {
        let s = &w.staff[club.manager];
        json!({"person": named(Ref::person(s.person), c.person_name(s.person)), "since": s.joined.0, "record": {"games": s.record.games, "wins": s.record.wins, "draws": s.record.draws, "losses": s.record.losses}})
    });
    let relation = if c.same_club(id) { "Your club" } else if c.observer() { "Observing" } else { "Not your club" };
    let staff_counts: Vec<Value> = [StaffRole::Manager, StaffRole::Assistant, StaffRole::Coach, StaffRole::GkCoach, StaffRole::FitnessCoach, StaffRole::Scout, StaffRole::Physio, StaffRole::SportsScientist, StaffRole::HeadOfYouth, StaffRole::DirectorOfFootball]
        .iter()
        .map(|r| json!({"role": r.label(), "count": club.staff.iter().filter(|&&s| w.staff[s].role == *r).count()}))
        .collect();
    let internals = c.sees_club_internals(id);
    let followed = club.teams.iter().any(|t| w.followed.contains(t));
    Ok(json!({
        "id": id.0, "name": club.name, "short": club.short_name, "city": club.city,
        "nation": named(Ref::nation(club.nation), c.nation_name(club.nation)),
        "colors": [format!("#{:06x}", club.colors[0] & 0xffffff), format!("#{:06x}", club.colors[1] & 0xffffff)],
        "stadium": club.stadium, "capacity": club.capacity, "founded": club.founded, "reputation": club.reputation,
        "ownership": match club.ownership { Ownership::Private => "Private ownership", Ownership::MemberOwned => "Member owned", Ownership::Benefactor => "Benefactor", Ownership::InvestmentGroup => "Investment group", Ownership::StateBacked => "State backed" },
        "fan_mood": club.fan_mood,
        "league": league_json, "manager": manager, "teams": teams, "relation": relation, "followed": followed,
        "staff_counts": staff_counts,
        "finance": if internals { json!({
            "balance": club.finance.balance, "transfer_budget": club.finance.transfer_budget,
            "wage_budget": club.finance.wage_budget, "wage_bill": club.finance.wage_bill,
            "season_income": club.finance.season_income, "season_spend": club.finance.season_spend, "debt": club.finance.debt,
        }) } else { Value::Null },
        "facilities": if internals || c.same_club(id) { json!({"training": club.facilities.training, "youth": club.facilities.youth, "academy": club.facilities.academy, "medical": club.facilities.medical}) } else { Value::Null },
        "board": if internals { json!({"satisfaction": club.board.satisfaction, "patience": club.board.patience, "target_position": club.board.target_position, "warnings": club.board.warnings}) } else { Value::Null },
        "needs": if internals { json!(club.market.needs.iter().map(|n| json!({"pos": n.pos.code(), "min_ability": n.min_ability, "max_age": n.max_age, "urgency": n.urgency})).collect::<Vec<_>>()) } else { Value::Null },
    }))
}

pub fn follow(c: &mut crate::session::Session, args: &Value) -> ApiResult<Value> {
    let id = ClubId(args.get("club").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing club".into()))? as u32);
    let on = args.get("follow").and_then(Value::as_bool).unwrap_or(true);
    if id.0 as usize >= c.w().clubs.len() {
        return Err(ApiError::NotFound(format!("club {}", id.0)));
    }
    let teams: Vec<_> = c.w().clubs[id].teams.iter().copied().collect();
    let followed = &mut c.game.sim.world.followed;
    for t in teams {
        followed.retain(|x| *x != t);
        if on {
            followed.push(t);
        }
    }
    c.revision += 1;
    Ok(json!({"followed": on}))
}

// ---- competitions and nations ----------------------------------------------------------

pub fn comp(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = CompId(args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing competition id".into()))? as u32);
    if id.0 as usize >= c.w.comps.len() {
        return Err(ApiError::NotFound(format!("competition {}", id.0)));
    }
    let w = c.w;
    let comp = &w.comps[id];
    let st = &comp.state;
    let groups = match comp.format {
        pw_world::Format::Groups { groups, .. } => groups,
        _ => 0,
    };
    // Knockout ties, with unrevealed results held back.
    let concealed: Vec<_> = c.concealed_fixtures().into_iter().filter(|f| f.comp == id).collect();
    let mut tie_round: std::collections::BTreeMap<u16, u8> = Default::default();
    if matches!(comp.format, pw_world::Format::Knockout { .. } | pw_world::Format::Groups { .. }) {
        for fid in w.fixtures.between(st.start.add_days(-1), st.end.add_days(1)) {
            let f = w.fixtures.get(fid);
            if f.comp == id && f.tie != u16::MAX {
                tie_round.entry(f.tie).or_insert(f.round);
            }
        }
    }
    let ties: Vec<Value> = st
        .ties
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let masked = concealed.iter().any(|f| f.tie == i as u16);
            json!({
                "index": i, "round": tie_round.get(&(i as u16)).copied(),
                "a": named(c.team_ref(t.a), c.team_short(t.a)), "b": named(c.team_ref(t.b), c.team_short(t.b)),
                "goals_a": if masked { Value::Null } else { json!(t.goals_a) }, "goals_b": if masked { Value::Null } else { json!(t.goals_b) },
                "legs": t.legs, "played": t.played,
                "winner": if masked || t.winner.is_none() { Value::Null } else { named(c.team_ref(t.winner), c.team_short(t.winner)) },
                "hidden": masked,
            })
        })
        .collect();
    let above = comp.above.get().map(|x| named(Ref::comp(x), c.comp_name(x)));
    let below = comp.below.get().map(|x| named(Ref::comp(x), c.comp_name(x)));
    let spots: Vec<Value> = comp.continental.iter().map(|(x, n)| json!({"comp": named(Ref::comp(*x), c.comp_name(*x)), "places": n})).collect();
    let past_winners = w.history.honours.iter().filter(|h| h.comp == id).count();
    Ok(json!({
        "id": id.0, "name": comp.name, "short": comp.short_name,
        "nation": if comp.nation.is_some() { named(Ref::nation(comp.nation), c.nation_name(comp.nation)) } else { Value::Null },
        "kind": crate::tables::kind_text(comp.kind), "kind_key": match comp.kind { CompKind::League => "league", CompKind::Cup => "cup", CompKind::Continental => "continental", CompKind::SuperCup => "supercup" },
        "tier": comp.tier, "reputation": comp.reputation,
        "format": crate::tables::format_text(&comp.format), "is_league": comp.is_league(), "groups": groups,
        "size": comp.size, "promote": comp.promote, "relegate": comp.relegate,
        "rules": {"yellow_limit": comp.rules.yellow_limit, "bench": comp.rules.bench, "subs": comp.rules.subs, "extra_time": comp.rules.extra_time, "away_goals": comp.rules.away_goals, "foreigner_limit": comp.rules.foreigner_limit},
        "team_kind": comp.team_kind.label(),
        "state": {
            "season": c.season_label(id, st.season), "season_year": st.season, "stage": comp_summary_stage(c, id),
            "knockout": matches!(st.stage, Stage::Knockout(_)), "start": st.start.0, "end": st.end.0,
            "teams": st.entrants.len(), "round": st.round,
            "winner": if st.winner.is_some() && concealed.is_empty() { named(c.team_ref(st.winner), c.team_name(st.winner)) } else { Value::Null },
            "runner_up": if st.runner_up.is_some() && concealed.is_empty() { named(c.team_ref(st.runner_up), c.team_name(st.runner_up)) } else { Value::Null },
        },
        "above": above, "below": below, "continental_places": spots, "ties": ties, "past_editions": past_winners,
        "prize_pool": if c.observer() { json!(comp.prize_pool) } else { Value::Null },
    }))
}

pub fn nation(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = NationId(args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing nation id".into()))? as u32);
    if id.0 as usize >= c.w.nations.len() {
        return Err(ApiError::NotFound(format!("nation {}", id.0)));
    }
    let w = c.w;
    let n = &w.nations[id];
    let leagues: Vec<Value> = n.leagues.iter().map(|&l| json!({"comp": named(Ref::comp(l), c.comp_name(l)), "tier": w.comps[l].tier, "teams": w.comps[l].state.entrants.len()})).collect();
    let cups: Vec<Value> = n.cups.iter().map(|&l| named(Ref::comp(l), c.comp_name(l))).collect();
    let clubs = w.clubs.iter().filter(|cl| cl.nation == id).count();
    let players = w.people.iter().filter(|p| p.nation == id && p.player.is_some()).count();
    Ok(json!({
        "id": id.0, "name": n.name, "code": n.code, "confed": n.confed.code(), "reputation": n.reputation,
        "economy": if c.observer() { json!(n.economy) } else { Value::Null },
        "youth_rating": if c.observer() { json!(n.youth_rating) } else { Value::Null },
        "season": {"label": n.season.label(), "start": n.season.start.0, "end": n.season.end.0,
            "windows": n.season.windows.iter().map(|(a, b)| json!([a.0, b.0])).collect::<Vec<_>>(),
            "winter_break": n.season.winter_break.map(|(a, b)| json!([a.0, b.0]))},
        "leagues": leagues, "cups": cups, "clubs": clubs, "players": players,
        "window_open": n.season.window_open(w.date),
    }))
}
