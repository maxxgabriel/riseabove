use pw_core::{ClubId, CompId, NationId};
use pw_world::club::Ownership;
use pw_world::comp::Stage;
use pw_world::culture::Side;
use pw_world::{CompKind, StaffRole, TeamKind};
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};
use crate::tables::comp_summary_stage;

fn named(r: Ref, n: String) -> Value {
    serde_json::to_value(Named::new(r, n)).unwrap_or(Value::Null)
}

pub fn get(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = ClubId(crate::contract::request::<crate::contract::IdReq>(args.clone())?.id);
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
    let relation = if c.same_club(id) {
        "Your club"
    } else if c.observer() {
        "Observing"
    } else {
        "Not your club"
    };
    let staff_counts: Vec<Value> = [
        StaffRole::Manager,
        StaffRole::Assistant,
        StaffRole::Coach,
        StaffRole::GkCoach,
        StaffRole::FitnessCoach,
        StaffRole::Scout,
        StaffRole::Physio,
        StaffRole::SportsScientist,
        StaffRole::HeadOfYouth,
        StaffRole::DirectorOfFootball,
    ]
    .iter()
    .map(|r| json!({"role": r.label(), "count": club.staff.iter().filter(|&&s| w.staff[s].role == *r).count()}))
    .collect();
    let internals = c.sees_club_internals(id);
    let followed = club.teams.iter().any(|t| w.followed.contains(t));
    // What the club is called and known by, and its academy and partners abroad, from reference records (public identity).
    let lore = &w.ext.lore;
    let words = |s: &str| s.replace('_', " ");
    let also_known: Vec<Value> = lore
        .aliases
        .get(&id)
        .into_iter()
        .flatten()
        .filter(|a| matches!(a.kind.as_str(), "nickname" | "supporter_group" | "former_name" | "native_script" | "short_form"))
        .map(|a| json!({"text": a.text, "kind": words(&a.kind), "origin": a.origin.label()}))
        .collect();
    let academy = lore.academies.get(&id).map(|a| json!({"name": a.name, "kind": words(&a.kind), "residential": a.residential, "age_groups": a.age_groups, "origin": a.source.origin.label()}));
    let partners: Vec<Value> = lore
        .partnerships
        .iter()
        .filter(|p| p.clubs.contains(&id))
        .map(|p| json!({"with": p.foreign.iter().map(|(n, nat)| format!("{n} ({nat})")).collect::<Vec<_>>().join(", "), "what": p.components.iter().map(|x| words(x)).collect::<Vec<_>>().join(", "), "purpose": p.purpose, "active": p.active, "origin": p.source.origin.label()}))
        .collect();
    let channels: Vec<Value> = w.media.outlets.iter_enumerated().filter(|(_, o)| o.leaning == id).map(|(oid, o)| json!({"name": o.name, "real": lore.outlets.contains_key(&oid)})).collect();
    Ok(json!({
        "id": id.0, "name": club.name, "short": club.short_name, "city": club.city,
        "nation": named(Ref::nation(club.nation), c.nation_name(club.nation)),
        "colors": [format!("#{:06x}", club.colors[0] & 0xffffff), format!("#{:06x}", club.colors[1] & 0xffffff)],
        "stadium": club.stadium, "capacity": club.capacity, "founded": club.founded, "reputation": club.reputation,
        "ownership": match club.ownership { Ownership::Private => "Private ownership", Ownership::MemberOwned => "Member owned", Ownership::Benefactor => "Benefactor", Ownership::InvestmentGroup => "Investment group", Ownership::StateBacked => "State backed" },
        "fan_mood": club.fan_mood,
        "league": league_json, "manager": manager, "teams": teams, "relation": relation, "followed": followed,
        "staff_counts": staff_counts,
        "also_known": also_known, "academy": academy, "partners": partners, "channels": channels, "place": place(c, id), "country": country(c, id),
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
    let req: crate::contract::FollowReq = crate::contract::request(args.clone())?;
    let id = ClubId(req.club);
    let on = req.follow.unwrap_or(true);
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
    Ok(crate::contract::wire(crate::contract::Followed { followed: on }))
}

// ---- competitions and nations ----------------------------------------------------------

/// A date the world has set, or null for one it has not (day zero is the unset value, not 1970).
fn day(d: pw_core::Date) -> Value {
    if d.0 == 0 { Value::Null } else { json!(d.0) }
}

pub fn comp(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = CompId(crate::contract::request::<crate::contract::IdReq>(args.clone())?.id);
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
        // A tie whose sides are not decided yet (a slot waiting for an earlier round) is not listed: it has no team to name.
        .filter(|(_, t)| t.a.is_some() && t.b.is_some())
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
            "knockout": matches!(st.stage, Stage::Knockout(_)), "start": day(st.start), "end": day(st.end),
            "teams": st.entrants.len(), "round": st.round,
            "winner": if st.winner.is_some() && concealed.is_empty() { named(c.team_ref(st.winner), c.team_name(st.winner)) } else { Value::Null },
            "runner_up": if st.runner_up.is_some() && concealed.is_empty() { named(c.team_ref(st.runner_up), c.team_name(st.runner_up)) } else { Value::Null },
        },
        "above": above, "below": below, "continental_places": spots, "ties": ties, "past_editions": past_winners,
        "prize_pool": if c.observer() { json!(comp.prize_pool) } else { Value::Null },
    }))
}

pub fn nation(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = NationId(crate::contract::request::<crate::contract::IdReq>(args.clone())?.id);
    if id.0 as usize >= c.w.nations.len() {
        return Err(ApiError::NotFound(format!("nation {}", id.0)));
    }
    let w = c.w;
    let n = &w.nations[id];
    let leagues: Vec<Value> = n.leagues.iter().map(|&l| json!({"comp": named(Ref::comp(l), c.comp_name(l)), "tier": w.comps[l].tier, "teams": w.comps[l].state.entrants.len()})).collect();
    let cups: Vec<Value> = n.cups.iter().map(|&l| named(Ref::comp(l), c.comp_name(l))).collect();
    let clubs = w.clubs.iter().filter(|cl| cl.nation == id).count();
    let players = w.people.iter().filter(|p| p.nation == id && p.player.is_some()).count();
    let sides: Vec<Value> = pw_world::intl::Level::ALL
        .iter()
        .filter_map(|&lv| w.intl.sides.get(&(id, lv)).map(|s| (lv, s)))
        .map(|(lv, s)| {
            let mgr = (s.manager.is_some()).then(|| w.staff[s.manager].person);
            json!({
                "level": lv.label(), "manager": mgr.map(|m| named(Ref::person(m), c.person_name(m))), "since": s.since.0,
                "captain": if s.captain.is_some() { Some(named(c.player_ref(s.captain), c.player_name(s.captain))) } else { None },
                "squad": s.squad.len(), "selected": s.selected.0, "record": {"won": s.record.0, "drawn": s.record.1, "lost": s.record.2},
            })
        })
        .collect();
    let econ = w.economy.nations.get(&id);
    let ranking = {
        let mut all: Vec<(NationId, f32)> = w.economy.nations.keys().map(|&n| (n, w.economy.coefficient(n))).collect();
        all.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        all.iter().position(|x| x.0 == id).map(|i| i + 1)
    };
    Ok(json!({
        "sides": sides,
        "world_economy": econ.map(|e| json!({
            "coefficient": w.economy.coefficient(id), "rank": ranking, "league_strength": e.league_strength,
            "wage_index": if c.observer() { json!(w.economy.wage_index(id)) } else { Value::Null },
            "broadcast_pool": if c.observer() { json!(e.broadcast_pool) } else { Value::Null },
            "deal_until": if c.observer() { json!(e.deal_until) } else { Value::Null },
            "growth": if c.observer() { json!(e.growth) } else { Value::Null },
        })),
        "id": id.0, "name": n.name, "code": n.code, "confed": n.confed.code(), "reputation": n.reputation,
        "economy": if c.observer() { json!(n.economy) } else { Value::Null },
        "youth_rating": if c.observer() { json!(n.youth_rating) } else { Value::Null },
        "season": {"label": n.season.label(), "start": day(n.season.start), "end": day(n.season.end),
            "windows": n.season.windows.iter().map(|(a, b)| json!([a.0, b.0])).collect::<Vec<_>>(),
            "winter_break": n.season.winter_break.map(|(a, b)| json!([a.0, b.0]))},
        "leagues": leagues, "cups": cups, "clubs": clubs, "players": players,
        "window_open": n.season.window_open(w.date),
    }))
}

// ---- ownership, planning, scouting, the dressing room and sponsors ---------------------------------

fn bond_text(c: &Ctx, b: pw_world::dressing::Bond) -> String {
    use pw_world::dressing::Bond;
    match b {
        Bond::Nationality(n) => format!("Players from {}", c.nation_name(n)),
        Bond::Generation => "The same generation".into(),
        Bond::Veterans => "The senior players".into(),
        Bond::Academy => "Academy graduates".into(),
        Bond::Friendship => "Close friends".into(),
    }
}

pub(crate) fn rivalry_word(k: pw_world::culture::RivalryKind) -> &'static str {
    use pw_world::culture::RivalryKind as R;
    match k {
        R::Derby => "Derby",
        R::Regional => "Regional",
        R::Historic => "Historic",
        R::TitleRace => "Title race",
        R::Promotion => "Promotion battle",
        R::Relegation => "Relegation battle",
        R::CupRevenge => "Cup revenge",
        R::BadBlood => "Bad blood",
        R::Institutional => "Institutional",
        R::International => "International",
    }
}

pub(crate) fn group_word(k: pw_world::socialnet::GroupKind) -> &'static str {
    use pw_world::socialnet::GroupKind as G;
    match k {
        G::SeasonTicket => "Season-ticket holders",
        G::Online => "Online community",
        G::International => "Overseas supporters",
        G::Academy => "Academy followers",
        G::Ultras => "Ultras",
        G::Numbers => "Numbers and data fans",
        G::Trust => "Supporters' trust",
    }
}

/// Who owns and runs the club, what it is building, what its staff are planning. Public facts (owner,
/// announced projects, sponsors) are shown to everyone; boardroom numbers, the squad plan, scouting and
/// the dressing room only to the observer.
pub fn systems(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = ClubId(crate::contract::request::<crate::contract::IdReq>(args.clone())?.id);
    if id.0 as usize >= c.w.clubs.len() {
        return Err(ApiError::NotFound(format!("club {}", id.0)));
    }
    let w = c.w;
    let internals = c.sees_club_internals(id);
    let person = |p: pw_core::PersonId| if p.is_some() { named(Ref::person(p), c.person_name(p)) } else { Value::Null };
    let board = w.governance.get(&id).map(|g| {
        json!({
            "owner": person(g.owner.person), "kind": crate::tables::ownership_label(g.owner.kind), "since": g.owner.since.0, "chairman": person(g.chairman),
            "administration": g.administration.map(|d| d.0),
            "projects": g.projects.iter().map(|p| json!({"kind": p.kind.label(), "target": p.target, "started": p.started.0, "completes": p.completes.0, "cost": if internals { json!(p.cost) } else { Value::Null }})).collect::<Vec<_>>(),
            "owner_traits": if internals { json!({"wealth": g.owner.wealth, "ambition": g.owner.ambition, "patience": g.owner.patience, "meddling": g.owner.meddling, "frugality": g.owner.frugality, "fan_sensitivity": g.owner.fan_sensitivity}) } else { Value::Null },
            "policy": if internals { json!({
                "wage_cap_mult": g.policy.wage_cap_mult, "youth_investment": g.policy.youth_investment, "transfer_style": g.policy.transfer_style.label(),
                "max_signing_age": g.policy.max_signing_age, "sell_to_rivals": g.policy.sell_to_rivals, "debt_tolerance": g.policy.debt_tolerance,
                "style_mandate": g.policy.style_mandate, "youth_minutes_target": g.policy.youth_minutes_target, "selling_stance": g.policy.selling_stance,
            }) } else { Value::Null },
            "concerns": if internals { json!(g.concerns.iter().filter(|x| x.1 != 0).map(|(k, v)| json!({"label": k.label(), "value": v})).collect::<Vec<_>>()) } else { Value::Null },
            "revenue_history": if internals { json!(g.revenue_history) } else { Value::Null },
            "red_months": if internals { json!(g.red_months) } else { Value::Null },
        })
    });
    let plan = if internals {
        w.deals.plans.get(&id).map(|p| {
            json!({
                "built": p.built.0, "homegrown_gap": p.homegrown_gap,
                "groups": p.groups.iter().map(|g| json!({
                    "group": g.group.label(), "depth": g.depth, "target_depth": g.target_depth, "quality": g.quality, "target_quality": g.target_quality,
                    "avg_age": g.avg_age, "expiring": g.expiring, "injured": g.injured, "prospects": g.prospects, "ageing_starters": g.ageing_starters,
                })).collect::<Vec<_>>(),
                "needs": p.needs.iter().map(|n| json!({
                    "group": n.group.label(), "role": n.role.label(), "min_ability": n.min_ability, "max_age": n.max_age, "homegrown": n.homegrown,
                    "wage_band": n.wage_band, "fee_band": n.fee_band, "urgency": n.urgency,
                })).collect::<Vec<_>>(),
                "sell": p.sell.iter().map(|&x| named(c.player_ref(x), c.player_name(x))).collect::<Vec<_>>(),
                "promote": p.promote.iter().map(|&x| named(c.player_ref(x), c.player_name(x))).collect::<Vec<_>>(),
            })
        })
    } else {
        None
    };
    let scouting = if internals {
        let scouts: Vec<Value> = w.clubs[id]
            .staff
            .iter()
            .filter(|&&s| w.staff[s].role == StaffRole::Scout)
            .map(|&s| {
                let st = &w.staff[s];
                let prof = w.scouting.profiles.get(&s);
                let briefs: Vec<String> = w
                    .scouting
                    .assignments
                    .iter()
                    .filter(|a| a.scout == s && a.club == id)
                    .map(|a| match a.brief {
                        pw_world::scouting::Brief::Nation(n) => format!("Covering {}", c.nation_name(n)),
                        pw_world::scouting::Brief::Competition(x) => format!("Covering {}", c.comp_name(x)),
                        pw_world::scouting::Brief::Youth(n) => format!("Youth football in {}", c.nation_name(n)),
                        pw_world::scouting::Brief::Player(p) => format!("Watching {}", c.player_name(p)),
                        pw_world::scouting::Brief::Need(g) => format!("Looking for {}", g.label().to_lowercase()),
                    })
                    .collect();
                json!({"who": person(st.person), "based": prof.map(|p| c.nation_name(p.based)), "capacity": prof.map(|p| p.capacity), "briefs": briefs})
            })
            .collect();
        let reports = w.scouting.reports.keys().filter(|(cl, _)| *cl == id).count();
        Some(json!({"scouts": scouts, "reports": reports}))
    } else {
        None
    };
    let room = if internals {
        w.rooms.clubs.get(&id).map(|r| {
            let mut leaders: Vec<(&pw_core::PlayerId, &u8)> = r.influence.iter().collect();
            leaders.sort_by_key(|(p, v)| (std::cmp::Reverse(**v), **p));
            json!({
                "harmony": r.harmony, "backing": r.backing, "updated": r.updated.0,
                "groups": r.groups.iter().map(|g| json!({
                    "bond": bond_text(c, g.bond), "cohesion": g.cohesion, "stance": g.stance, "size": g.members.len(),
                    "leader": if g.leader.is_some() { named(c.player_ref(g.leader), c.player_name(g.leader)) } else { Value::Null },
                })).collect::<Vec<_>>(),
                "influential": leaders.iter().take(6).map(|(p, v)| json!({"who": named(c.player_ref(**p), c.player_name(**p)), "influence": v, "standing": r.standing.get(*p).map(|s| s.label())})).collect::<Vec<_>>(),
            })
        })
    } else {
        None
    };
    let sponsors: Vec<Value> = w
        .commerce
        .club_deals
        .iter()
        .filter(|d| d.club == id && d.end >= w.date)
        .map(|d| json!({"brand": w.commerce.brands[d.brand as usize].name, "slot": format!("{:?}", d.slot).to_lowercase(), "until": d.end.0, "fee": if internals { json!(d.fee_year) } else { Value::Null }}))
        .collect();
    let rivalries: Vec<Value> = w
        .culture
        .rivalries
        .list
        .iter()
        .filter(|r| r.a == Side::Club(id) || r.b == Side::Club(id))
        .map(|r| {
            let other = if r.a == Side::Club(id) { r.b } else { r.a };
            // Stored from the first side's point of view: wins, draws, losses for this club.
            let record = if r.a == Side::Club(id) { [r.h2h.0, r.h2h.1, r.h2h.2] } else { [r.h2h.2, r.h2h.1, r.h2h.0] };
            let (name, target) = match other {
                Side::Club(x) => (c.club_name(x), Some(Ref::club(x))),
                Side::Nation(n) => (c.nation_name(n), Some(Ref::nation(n))),
                Side::Institution(i) => (pw_narrate::history::institution(w, i), Some(Ref::inst(i))),
            };
            json!({
                "with": target.map_or_else(|| Value::String(name.clone()), |t| named(t, name.clone())), "intensity": r.intensity,
                "why": r.kinds.iter().map(|k| rivalry_word(*k)).collect::<Vec<_>>(), "record": record, "since": r.since.0,
                "last_met": if r.last_meeting.0 > 0 { json!(r.last_meeting.0) } else { Value::Null },
            })
        })
        .collect();
    let supporters: Vec<Value> = w
        .net
        .groups
        .iter()
        .filter(|g| g.club == id)
        .map(|g| json!({"kind": group_word(g.kind), "size": g.size, "manager": g.manager, "board": g.board, "team": g.team, "voice": g.voice, "last_acted": g.last_action.0}))
        .collect();
    let culture = if internals {
        w.culture.clubs.get(&id).map(|k| {
            json!({
                "identity": {"youth": k.identity.youth, "local": k.identity.local, "flair": k.identity.flair, "grit": k.identity.grit, "underdog": k.identity.underdog, "glamour": k.identity.glamour},
                "discipline": k.discipline, "expectations": k.expectations, "patience": k.patience, "tribalism": k.tribalism, "graduates": k.graduates, "drought": k.drought,
            })
        })
    } else {
        None
    };
    Ok(json!({"board": board, "plan": plan, "scouting": scouting, "room": room, "sponsors": sponsors, "rivalries": rivalries, "supporters": supporters, "culture": culture, "internal": internals}))
}

/// The club's place, from the regions of the world (clubs in a world without regions have none).
fn place(c: &Ctx, id: ClubId) -> Value {
    use pw_world::ecosystem::Climate;
    let w = c.w;
    let eco = &w.ext.ecosystem;
    let Some(&rid) = eco.club_region.get(&id) else { return Value::Null };
    let Some(r) = eco.regions.get(rid) else { return Value::Null };
    let state = eco.state_of(rid);
    let state_name = (state != rid).then(|| eco.regions.get(state).map(|s| s.name.clone())).flatten();
    let climate = match r.climate {
        Climate::HumidCoastal => "Humid and coastal",
        Climate::HotDry => "Hot and dry, with a fierce summer",
        Climate::HighAltitude => "High up: the air is thin",
        Climate::CoolHill => "Cool hill country, cold in winter",
        Climate::HeavyMonsoon => "Heavy monsoon rains from June to September",
        Climate::NorthEast => "Wet and green, the hills of the north-east",
        Climate::Temperate => "Temperate",
    };
    let population = match r.population_k {
        0 => "Not known".to_string(),
        k if k >= 1_000 => format!("About {:.1} million people", f64::from(k) / 1_000.0),
        k => format!("About {k} thousand people"),
    };
    let football = match r.culture {
        x if x >= 60.0 => "Football runs deep here",
        x if x >= 35.0 => "A real football following",
        _ => "Football is still finding its place here",
    };
    let mut nearby: Vec<(u16, ClubId)> = eco.club_region.iter().filter(|(k, rr)| **k != id && eco.state_of(**rr) == state).map(|(k, _)| (w.clubs[*k].reputation, *k)).collect();
    nearby.sort_by_key(|(rep, k)| (std::cmp::Reverse(*rep), *k));
    let nearby: Vec<Value> = nearby.into_iter().take(6).map(|(_, k)| named(Ref::club(k), c.club_name(k))).collect();
    let mut unis: Vec<(u16, String, u32)> = eco
        .inst
        .iter()
        .filter(|(_, p)| eco.state_of(p.region) == state)
        .filter_map(|(i, _)| w.minor.institutions.get(*i as usize).map(|x| (*i, x)))
        .filter(|(_, i)| i.kind == pw_world::minor::InstKind::University)
        .map(|(id, i)| (i.prestige, i.name.clone(), id))
        .collect();
    unis.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let universities: Vec<Value> = unis.into_iter().take(4).map(|(_, n, i)| named(Ref::inst(i), n)).collect();
    // The papers and channels of the place: those that lean to the club, and those based in its state (reference data).
    let media: Vec<Value> = {
        let lore = &w.ext.lore;
        let mut list: Vec<(bool, u8, pw_core::OutletId)> = w
            .media
            .outlets
            .iter_enumerated()
            .filter(|(oid, o)| o.leaning == id || lore.outlets.get(oid).is_some_and(|l| l.home.is_some() && (l.home == state || l.home == rid)))
            .map(|(oid, o)| (o.leaning == id, o.reach, oid))
            .collect();
        list.sort_by_key(|(own, reach, oid)| (std::cmp::Reverse(*own), std::cmp::Reverse(*reach), *oid));
        list.into_iter().take(8).map(|(_, _, oid)| outlet_json(c, oid, id)).collect()
    };
    // How far it is from home, for the person you live as.
    let from_home = c.my_player().and_then(|p| eco.story.get(&p)).and_then(|s| eco.regions.get(s.home)).map(|h| {
        let (dx, dy) = (f32::from(h.x) - f32::from(r.x), f32::from(h.y) - f32::from(r.y));
        let km = (dx * dx + dy * dy).sqrt() * 30.0;
        let n = ((km / 10.0).round() * 10.0) as u32;
        let grouped = if n >= 1_000 { format!("{},{:03}", n / 1_000, n % 1_000) } else { n.to_string() };
        if km < 40.0 { "Home ground: this is where you grew up".to_string() } else { format!("About {grouped} km from home") }
    });
    // Where the work is done and where the matches are played, in words (levels are 1-20).
    let k = &w.clubs[id];
    let training = match k.facilities.training {
        16.. => "A modern training centre: several pitches, a gym and a medical room",
        11..=15 => "A proper training ground of the club's own",
        6..=10 => "Basic: one or two pitches and a changing room",
        _ => "Borrowed pitches: the club trains where it can",
    };
    let ground = (!k.stadium.is_empty()).then(|| if k.capacity > 0 { format!("{}, about {} seats", k.stadium, crate::fmt::thousands(k.capacity)) } else { k.stadium.clone() });
    json!({
        "region": r.name, "state": state_name, "climate": climate, "language": eco.languages.get(usize::from(r.language)).cloned(),
        "population": population, "football": football, "nearby": nearby, "universities": universities, "from_home": from_home,
        "training": training, "ground": ground, "media": media,
    })
}

fn outlet_kind_words(k: pw_world::media::OutletKind) -> &'static str {
    use pw_world::media::OutletKind as K;
    match k {
        K::National => "National paper",
        K::Local => "Local paper",
        K::Tabloid => "Tabloid",
        K::Broadcaster => "Broadcaster",
        K::DataSite => "Data site",
        K::FanChannel => "Fan channel",
    }
}

/// A language code of the reference data, by name where it is a common one.
fn language_name(code: &str) -> String {
    match code {
        "en" => "English",
        "hi" => "Hindi",
        "bn" => "Bengali",
        "ml" => "Malayalam",
        "ta" => "Tamil",
        "te" => "Telugu",
        "kn" => "Kannada",
        "mr" => "Marathi",
        "gu" => "Gujarati",
        "pa" => "Punjabi",
        "or" => "Odia",
        "as" => "Assamese",
        "ur" => "Urdu",
        "kok" => "Konkani",
        "mni" => "Manipuri",
        "lus" => "Mizo",
        "ne" => "Nepali",
        "es" => "Spanish",
        "pt" => "Portuguese",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "ja" => "Japanese",
        "ar" => "Arabic",
        other => return other.to_uppercase(),
    }
    .to_string()
}

/// An outlet as a reader meets it: what it is, how far it reaches, the languages it writes in (reference data), and its latest piece
/// about the club, to open.
fn outlet_json(c: &Ctx, oid: pw_core::OutletId, club: ClubId) -> Value {
    let w = c.w;
    let o = &w.media.outlets[oid];
    let lore = w.ext.lore.outlets.get(&oid);
    let reach = match lore.map(|l| l.reach.as_str()) {
        Some("national") => "Read across the country",
        Some("multi_state") => "Read in several states",
        Some("state") => "Read across the state",
        Some("local") => "Read locally",
        _ => match o.reach {
            15.. => "A big audience",
            8..=14 => "A fair audience",
            _ => "A small audience",
        },
    };
    let languages = lore.filter(|l| !l.languages.is_empty()).map(|l| l.languages.iter().map(|x| language_name(x)).collect::<Vec<_>>().join(", "));
    let latest = w.media.stories.iter().filter(|s| s.outlet == oid && s.club == club && !c.story_spoils(s)).max_by_key(|s| (s.date, s.id));
    json!({
        "name": o.name, "kind": outlet_kind_words(o.kind), "reach": reach, "languages": languages, "own": o.leaning == club,
        "origin": if lore.is_some() { "Imported" } else { "Generated" },
        "story": latest.map(|s| s.id.0), "headline": latest.map(|s| c.headline(s)),
    })
}

/// The country a club is in, as a place to live: for a club abroad from the person you live as (or, for an observer, a club with no
/// region on the map). The nation's environment is reference data when `known`, otherwise an inference from its region.
fn country(c: &Ctx, id: ClubId) -> Value {
    let w = c.w;
    let k = &w.clubs[id];
    let Some(n) = w.nations.get(k.nation) else { return Value::Null };
    let home = c.me().map(|me| w.people[me].nation).filter(|h| h.is_some() && w.nations.get(*h).is_some());
    match home {
        Some(h) if h == k.nation => return Value::Null,
        None if w.ext.ecosystem.club_region.contains_key(&id) => return Value::Null,
        _ => {}
    }
    let e = &n.env;
    let mut climate = match e.climate {
        i8::MIN..=-3 => "Very cold winters",
        -2 => "Cold",
        -1 => "Cool",
        0 => "Mild",
        1 => "Warm",
        2 => "Hot",
        _ => "Very hot",
    }
    .to_string();
    if e.humidity >= 70 {
        climate.push_str(" and humid");
    } else if e.humidity <= 30 {
        climate.push_str(" and dry");
    }
    let altitude = (e.altitude >= 1_200).then(|| format!("High up: football is played about {} m above the sea", crate::fmt::thousands(u32::from(e.altitude / 100 * 100))));
    let football = {
        let mut bits = Vec::new();
        if e.pace >= 62 {
            bits.push("quick");
        }
        if e.physical >= 62 {
            bits.push("physical");
        }
        if e.tempo >= 62 {
            bits.push("played at a high tempo");
        } else if e.tempo <= 38 {
            bits.push("patient on the ball");
        }
        if bits.is_empty() { "A balanced kind of football".to_string() } else { format!("The football is {}", bits.join(", ")) }
    };
    let mut living = format!("Income tax about {}%", e.tax);
    let (mut language, mut clock, mut from_home) = (None, None, None);
    if let (Some(h), Some(me)) = (home, c.me()) {
        let he = &w.nations[h].env;
        let (a, b) = (i32::from(e.climate), i32::from(he.climate));
        if a > b {
            climate.push_str(", warmer than home");
        } else if a < b {
            climate.push_str(", colder than home");
        }
        living.push_str(match i32::from(e.living) - i32::from(he.living) {
            d if d >= 15 => "; living costs more than at home",
            d if d <= -15 => "; living costs less than at home",
            _ => "; living costs about what they do at home",
        });
        // Your own languages: how well you speak the one spoken there.
        let fluent = w.lives.get(me).map_or(0, |l| {
            l.languages.iter().filter(|(x, _)| *x == k.nation || w.nations.get(*x).is_some_and(|y| y.env.language == e.language)).map(|(_, f)| *f).max().unwrap_or(0)
        });
        language = Some(
            if e.language == he.language {
                "The language of home: you will be understood"
            } else {
                match fluent {
                    80.. => "Another language from home, and you speak it well",
                    40..=79 => "Another language from home; you get by in it",
                    1..=39 => "Another language from home; you have a few words",
                    _ => "Another language from home, and you do not speak it yet",
                }
            }
            .to_string(),
        );
        let gap = i32::from(e.tz) - i32::from(he.tz);
        clock = Some(match gap {
            0 => "The same time as home".to_string(),
            g if g > 0 => format!("{g} hour{} ahead of home", if g == 1 { "" } else { "s" }),
            g => format!("{} hour{} behind home", -g, if g == -1 { "" } else { "s" }),
        });
        let same_confed = w.nations[h].confed == n.confed;
        from_home = Some(
            match (same_confed, gap.abs()) {
                (true, 0..=1) => "Not far: a short flight from home",
                (true, _) => "The same part of the world, a long flight from home",
                (false, 0..=3) => "Another continent, though the clock is close to home's",
                (false, _) => "Another continent: far from home, and a long way round the clock",
            }
            .to_string(),
        );
    }
    let mut national: Vec<(u8, pw_core::OutletId)> = w
        .media
        .outlets
        .iter_enumerated()
        .filter(|(_, o)| o.nation == k.nation && matches!(o.kind, pw_world::media::OutletKind::National | pw_world::media::OutletKind::Broadcaster))
        .map(|(oid, o)| (o.reach, oid))
        .collect();
    national.sort_by_key(|(r, oid)| (std::cmp::Reverse(*r), *oid));
    let media: Vec<Value> = national.into_iter().take(4).map(|(_, oid)| outlet_json(c, oid, id)).collect();
    json!({
        "nation": named(Ref::nation(k.nation), n.name.clone()), "climate": climate, "altitude": altitude, "language": language, "clock": clock,
        "from_home": from_home, "football": football, "living": living, "media": media, "origin": if e.known { "Imported" } else { "Inferred" },
    })
}
