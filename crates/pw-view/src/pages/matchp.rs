use pw_core::{FixtureId, PlayerId, Pos, TeamId};
use pw_match::{Ev, MatchResult};
use pw_world::Fixture;
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};
use crate::tables::{round_text, score_text};

fn named(r: Ref, n: String) -> Value {
    serde_json::to_value(Named::new(r, n)).unwrap_or(Value::Null)
}

pub fn find<'a>(c: &Ctx<'a>, uid: u64) -> Option<(FixtureId, &'a Fixture)> {
    c.w.fixtures.iter().find(|(_, f)| f.uid == uid)
}

pub fn uid_arg(args: &Value) -> ApiResult<u64> {
    args.get("uid").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing match id".into()))
}

fn ev_label(e: Ev) -> &'static str {
    match e {
        Ev::KickOff => "Kick-off",
        Ev::HalfTime => "Half-time",
        Ev::FullTime => "Full-time",
        Ev::ExtraTimeStart => "Extra time",
        Ev::Goal => "Goal",
        Ev::OwnGoal => "Own goal",
        Ev::PenaltyGoal => "Penalty scored",
        Ev::PenaltyMiss => "Penalty missed",
        Ev::ShotSaved => "Shot saved",
        Ev::ShotWide => "Shot off target",
        Ev::ShotBlocked => "Shot blocked",
        Ev::ShotPost => "Hit the woodwork",
        Ev::Header => "Header",
        Ev::KeyPass => "Key pass",
        Ev::ThroughBall => "Through ball",
        Ev::Cross => "Cross",
        Ev::Dribble => "Dribble",
        Ev::Tackle => "Tackle",
        Ev::Interception => "Interception",
        Ev::Clearance => "Clearance",
        Ev::Corner => "Corner",
        Ev::FreeKick => "Free kick",
        Ev::Offside => "Offside",
        Ev::Foul => "Foul",
        Ev::Yellow => "Yellow card",
        Ev::SecondYellow => "Second yellow",
        Ev::Red => "Red card",
        Ev::Sub => "Substitution",
        Ev::Injury => "Injury",
        Ev::Chain => "Move",
        Ev::ShootoutGoal => "Shootout: scored",
        Ev::ShootoutMiss => "Shootout: missed",
        Ev::TacticChange => "Tactical change",
    }
}

fn is_shot(e: Ev) -> bool {
    matches!(e, Ev::Goal | Ev::OwnGoal | Ev::PenaltyGoal | Ev::PenaltyMiss | Ev::ShotSaved | Ev::ShotWide | Ev::ShotBlocked | Ev::ShotPost)
}

fn pref(c: &Ctx, p: PlayerId) -> Value {
    if p.is_none() { Value::Null } else { named(c.player_ref(p), c.player_short(p)) }
}

/// Everything recorded about a played match with a detailed report.
fn detail(c: &Ctx, fx: &Fixture, r: &MatchResult) -> Value {
    use pw_match::{ZONES_X, ZONES_Y, zone_xy};
    let events: Vec<Value> = r
        .events
        .iter()
        .map(|e| {
            let (zx, zy) = zone_xy(usize::from(e.zone).min(ZONES_X * ZONES_Y - 1));
            let (x, y) = ((zx as f32 + 0.5) / ZONES_X as f32, (zy as f32 + 0.5) / ZONES_Y as f32);
            let (x, y) = if e.side == 0 { (x, y) } else { (1.0 - x, 1.0 - y) };
            let tier = if e.kind.is_key() {
                "key"
            } else if is_shot(e.kind) {
                "shot"
            } else {
                "minor"
            };
            json!({
                "t": e.t, "minute": e.minute(), "side": e.side, "kind": format!("{:?}", e.kind), "label": ev_label(e.kind),
                "tier": tier, "player": pref(c, e.player), "other": pref(c, e.other), "x": x, "y": y, "xg": e.value,
            })
        })
        .collect();
    let lines: Vec<Value> = r
        .lines
        .iter()
        .map(|l| {
            let (bx, by) = l.pos.map_or((0.5, 0.5), Pos::base_xy);
            json!({
                "player": pref(c, l.player), "side": l.side, "started": l.started, "pos": l.pos.map(Pos::code), "keeper": l.is_keeper,
                "minutes": l.minutes, "on_at": l.on_at, "off_at": l.off_at, "rating": l.rating,
                "goals": l.goals, "assists": l.assists, "shots": l.shots, "on_target": l.on_target, "xg": l.xg, "xa": l.xa,
                "key_passes": l.key_passes, "passes": l.passes, "passes_completed": l.passes_completed,
                "tackles": l.tackles, "tackles_won": l.tackles_won, "interceptions": l.interceptions, "clearances": l.clearances,
                "dribbles": l.dribbles, "dribbles_won": l.dribbles_won, "saves": l.saves, "conceded": l.conceded,
                "fouls": l.fouls, "yellows": l.yellows, "reds": l.reds, "injured": l.injured,
                "x": bx, "y": by,
            })
        })
        .collect();
    let stats = |i: usize| {
        let s = &r.stats[i];
        json!({
            "possession": s.possession, "shots": s.shots, "on_target": s.on_target, "xg": s.xg, "big_chances": s.big_chances,
            "corners": s.corners, "fouls": s.fouls, "offsides": s.offsides, "passes": s.passes, "passes_completed": s.passes_completed,
            "tackles": s.tackles, "saves": s.saves, "yellows": s.yellows, "reds": s.reds,
        })
    };
    json!({
        "events": events, "lines": lines, "stats": [stats(0), stats(1)],
        "man_of_the_match": pref(c, r.pom),
        "shape_home": tactic_shape(c, fx.home), "shape_away": tactic_shape(c, fx.away),
    })
}

fn tactic_shape(c: &Ctx, t: TeamId) -> Value {
    let tac = &c.w.teams[t].tactics;
    let name = c.w.data.formations.get(usize::from(tac.formation)).map(|f| f.name.clone());
    json!({"formation": name})
}

fn team_side(c: &Ctx, fx: &Fixture, t: TeamId) -> Value {
    let pos_row = c.w.comps[fx.comp].state.table.iter().find(|r| r.team == t).map(|_| ());
    let _ = pos_row;
    json!({
        "team": named(c.team_ref(t), c.team_name(t)), "short": c.team_short(t),
        "club": c.w.teams[t].club.0, "colors": [
            format!("#{:06x}", c.w.clubs[c.w.teams[t].club].colors[0] & 0xffffff),
            format!("#{:06x}", c.w.clubs[c.w.teams[t].club].colors[1] & 0xffffff),
        ],
        "mine": c.my_team() == t,
    })
}

fn absences(c: &Ctx, t: TeamId) -> Vec<Value> {
    c.w.teams[t]
        .squad
        .iter()
        .filter_map(|&p| {
            let h = &c.w.players.hot[p];
            if h.injury != 0 {
                Some(json!({"player": pref(c, p), "why": crate::fmt::singulars(format!("Injured, about {} days", h.injury_days))}))
            } else if h.ban > 0 {
                Some(json!({"player": pref(c, p), "why": format!("Suspended for {} {}", h.ban, if h.ban == 1 { "match" } else { "matches" })}))
            } else {
                None
            }
        })
        .collect()
}

pub fn get(c: &Ctx, args: &Value, watching: bool) -> ApiResult<Value> {
    let uid = uid_arg(args)?;
    let (_, fx) = find(c, uid).ok_or_else(|| ApiError::NotFound("That match is no longer in the records; only summaries are kept for old seasons.".into()))?;
    let w = c.w;
    let concealed = c.is_concealed(uid);
    let played = fx.score.is_some();
    let report = w.reports.get(&uid);
    let hide = concealed && !watching;
    let (score, detailed) = match (&fx.score, hide) {
        (Some(s), false) => (
            json!({"home": s.home, "away": s.away, "ht_home": s.ht_home, "ht_away": s.ht_away, "extra_time": s.extra_time, "pens": s.pens, "text": score_text(s)}),
            report.map(|r| detail(c, fx, r)).unwrap_or(Value::Null),
        ),
        _ => (Value::Null, Value::Null),
    };
    let comp = &w.comps[fx.comp];
    let club_home = w.teams[fx.home].club;
    // Recent form and meetings, from what has been revealed.
    let (rows, _) = crate::tables::visible_table(c, fx.comp);
    let position = |t: TeamId| rows.iter().position(|r| r.team == t).map(|p| p + 1);
    let previous: Vec<Value> = w
        .fixtures
        .iter()
        .map(|(_, f)| f)
        .filter(|f| f.uid != uid && f.score.is_some() && !c.is_concealed(f.uid) && ((f.home == fx.home && f.away == fx.away) || (f.home == fx.away && f.away == fx.home)))
        .map(|f| (f.date, f))
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_iter()
        .rev()
        .take(5)
        .map(|(_, f)| json!({"uid": f.uid, "date": f.date.0, "comp": c.comp_short(f.comp), "home": c.team_short(f.home), "away": c.team_short(f.away), "score": f.score.as_ref().map(score_text)}))
        .collect();
    let follow_hint = played && report.is_none() && !hide;
    Ok(json!({
        "uid": uid, "comp": named(Ref::comp(fx.comp), comp.name.clone()), "round": round_text(c, fx), "date": fx.date.0,
        "home": team_side(c, fx, fx.home), "away": team_side(c, fx, fx.away),
        "venue": if fx.neutral { json!("Neutral venue") } else { json!(w.clubs[club_home].stadium) },
        "capacity": if fx.neutral { Value::Null } else { json!(w.clubs[club_home].capacity) },
        "neutral": fx.neutral, "decisive": fx.decisive,
        "status": if played { "played" } else if fx.date < w.date { "not played" } else { "scheduled" },
        "concealed": concealed, "watching": watching && concealed,
        "score": score, "detail": detail_or_null(&detailed),
        "detail_kept": report.is_some(),
        "can_follow": follow_hint,
        // Who is missing is today's state, so it says nothing about a match that has already been played.
        "pre": {
            "home": {"position": position(fx.home), "absences": if played { Vec::new() } else { absences(c, fx.home) }},
            "away": {"position": position(fx.away), "absences": if played { Vec::new() } else { absences(c, fx.away) }},
        },
        "previous": previous,
        "rules": {"subs": comp.rules.subs, "bench": comp.rules.bench, "extra_time": comp.rules.extra_time},
    }))
}

fn detail_or_null(v: &Value) -> Value {
    v.clone()
}

pub fn reveal(s: &mut crate::session::Session, args: &Value) -> ApiResult<Value> {
    let uid = uid_arg(args)?;
    s.meta.concealed.remove(&uid);
    s.revision += 1;
    Ok(json!({"revealed": uid}))
}

pub fn reveal_all(s: &mut crate::session::Session) -> ApiResult<Value> {
    s.meta.concealed.clear();
    s.revision += 1;
    Ok(json!({"revealed": "all"}))
}
