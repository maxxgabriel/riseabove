use pw_core::CompId;
use pw_world::event::EventKind as E;
use pw_world::{CompKind, PlayerStatus, TeamKind};
use serde_json::{Value, json};

use crate::ctx::Ctx;
use crate::model::{ApiResult, Named, Ref};
use crate::narrative;

fn named(r: Ref, n: String) -> Value {
    serde_json::to_value(Named::new(r, n)).unwrap_or(Value::Null)
}

fn parts_json(c: &Ctx, e: &pw_world::event::Event) -> Value {
    serde_json::to_value(narrative::describe(c, e)).unwrap_or(Value::Null)
}

/// The observer's landing page, also shown as "World" while inhabiting.
pub fn overview(c: &Ctx) -> ApiResult<Value> {
    let w = c.w;
    let today = w.date;

    // Top leagues by standing, with the leader.
    let mut leagues: Vec<(CompId, &pw_world::Competition)> = w.comps.iter_enumerated().filter(|(_, x)| x.kind == CompKind::League && x.tier == 1 && x.team_kind == TeamKind::First).collect();
    leagues.sort_by_key(|(_, x)| std::cmp::Reverse(x.reputation));
    let league_rows: Vec<Value> = leagues
        .iter()
        .take(10)
        .map(|(id, x)| {
            let (rows, _) = crate::tables::visible_table(c, *id);
            let leader = rows.first();
            json!({
                "comp": named(Ref::comp(*id), x.name.clone()),
                "stage": crate::tables::comp_summary_stage(c, *id),
                "leader": leader.map(|r| json!({"team": named(c.team_ref(r.team), c.team_short(r.team)), "points": r.points, "played": r.played})),
            })
        })
        .collect();

    // Upcoming fixtures in the most reputable competitions.
    let upcoming: Vec<Value> = {
        let mut v: Vec<_> = w.fixtures.between(today, today.add_days(6)).map(|id| w.fixtures.get(id)).filter(|f| f.score.is_none() && w.comps[f.comp].team_kind == TeamKind::First).collect();
        v.sort_by_key(|f| (std::cmp::Reverse(w.comps[f.comp].reputation), f.date, f.uid));
        v.iter()
            .take(12)
            .map(|f| json!({"uid": f.uid, "date": f.date.0, "comp": c.comp_short(f.comp), "home": named(c.team_ref(f.home), c.team_short(f.home)), "away": named(c.team_ref(f.away), c.team_short(f.away))}))
            .collect()
    };

    // Recent things worth knowing, newest first.
    let since = today.add_days(-21);
    let mut recent: Vec<Value> = Vec::new();
    for e in w.events.since(since).iter().rev() {
        if !narrative::visible(c, e) {
            continue;
        }
        let interesting = matches!(
            e.kind,
            E::Transfer { fee, .. } if fee > 0
        ) || matches!(
            e.kind,
            E::LoanMove { .. } | E::ManagerSacked { .. } | E::ManagerAppointed { .. } | E::Champion { .. } | E::Promoted { .. } | E::Relegated { .. } | E::Retired { .. } | E::Award { .. }
        );
        if !interesting {
            continue;
        }
        recent.push(json!({"date": e.date.0, "kind": narrative::label(&e.kind), "parts": parts_json(c, e)}));
        if recent.len() >= 14 {
            break;
        }
    }

    let followed: Vec<Value> = w.followed.iter().map(|&t| named(c.team_ref(t), c.team_name(t))).collect();
    let active = w.players.hot.iter().filter(|h| h.status == PlayerStatus::Active).count();

    Ok(json!({
        "date": today.0, "name": c.s.meta.name,
        "counts": {"players": active, "clubs": w.clubs.len(), "competitions": w.comps.iter().filter(|x| x.team_kind == TeamKind::First).count(), "nations": w.nations.len()},
        "leagues": league_rows, "upcoming": upcoming, "recent": recent, "followed": followed,
    }))
}

/// Entity search obeying the current perspective.
pub fn search(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let q = args.get("q").and_then(Value::as_str).unwrap_or("").trim().to_lowercase();
    if q.chars().count() < 2 {
        return Ok(json!({"groups": []}));
    }
    let w = c.w;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(8) as usize;
    let score = |name: &str| -> Option<u8> {
        let n = name.to_lowercase();
        if n == q {
            Some(0)
        } else if n.starts_with(&q) {
            Some(1)
        } else if n.split_whitespace().any(|t| t.starts_with(&q)) {
            Some(2)
        } else if n.contains(&q) {
            Some(3)
        } else {
            None
        }
    };

    let mut people: Vec<(u8, i64, Value)> = Vec::new();
    for (pid, person) in w.people.iter_enumerated() {
        let name = person.display_name(&w.names);
        let Some(sc) = score(&name) else { continue };
        let (sub, weight) = if let Some(p) = person.player.get() {
            let h = &w.players.hot[p];
            let cold = &w.players.cold[p];
            let club = if h.club.is_some() { c.club_short(h.club) } else { c.status_label(p).to_string() };
            (format!("{} · {} · {}", person.age(w.date), cold.best_pos.code(), club), i64::from(cold.rep.world))
        } else if let Some(s) = person.staff.get() {
            let st = &w.staff[s];
            (format!("{} · {}", st.role.label(), if st.club.is_some() { c.club_short(st.club) } else { "no club".into() }), i64::from(st.reputation))
        } else {
            (String::new(), 0)
        };
        people.push((sc, -weight, json!({"k": "person", "id": pid.0, "title": name, "sub": sub})));
    }
    people.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    let mut clubs: Vec<(u8, i64, Value)> = w
        .clubs
        .iter_enumerated()
        .filter_map(|(id, cl)| {
            let sc = score(&cl.name).or_else(|| score(&cl.short_name))?;
            Some((sc, -i64::from(cl.reputation), json!({"k": "club", "id": id.0, "title": cl.name, "sub": format!("{} · {}", c.nation_name(cl.nation), c.comp_short(cl.league))})))
        })
        .collect();
    clubs.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    let mut comps: Vec<(u8, i64, Value)> = w
        .comps
        .iter_enumerated()
        .filter(|(_, x)| x.team_kind == TeamKind::First)
        .filter_map(|(id, x)| {
            let sc = score(&x.name)?;
            Some((sc, -i64::from(x.reputation), json!({"k": "comp", "id": id.0, "title": x.name, "sub": crate::tables::kind_text(x.kind)})))
        })
        .collect();
    comps.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    let nations: Vec<Value> = w
        .nations
        .iter_enumerated()
        .filter(|(_, n)| score(&n.name).is_some() || n.code.to_lowercase() == q)
        .map(|(id, n)| json!({"k": "nation", "id": id.0, "title": n.name, "sub": n.confed.code()}))
        .take(limit)
        .collect();

    let take = |v: Vec<(u8, i64, Value)>, n: usize| -> Vec<Value> { v.into_iter().take(n).map(|x| x.2).collect() };
    let mut groups = Vec::new();
    for (label, items) in [("People", take(people, limit)), ("Clubs", take(clubs, limit)), ("Competitions", take(comps, 5)), ("Nations", nations)] {
        if !items.is_empty() {
            groups.push(json!({"label": label, "items": items}));
        }
    }
    Ok(json!({"groups": groups}))
}

pub fn capabilities() -> Value {
    json!([
        {"area": "World browsing", "status": "ready", "note": "People, clubs, competitions, fixtures, tables, transfers, honours, events, press, agents and international football read the live simulation. So do supporters and their chants, rivalries, incidents, press conferences, referees and their big calls, records, halls of fame, tactical schools, rule changes and football below the professional game."},
        {"area": "Inhabiting a footballer", "status": "ready", "note": "Any active or retired player can be inhabited, or a new one created. The world keeps running around you and your decisions arrive through the same route the AI uses."},
        {"area": "Decisions", "status": "ready", "note": "Offers, loans, contract talks, conversations, a partner's question, trials, allegiance, treatment, endorsements, and the leave and apology requests that follow an incident. The options are the ones the simulation generated; if you do not answer, your own judgement is applied at the deadline."},
        {"area": "Inbox and replies", "status": "ready", "note": "Conversations are grouped by person or story. What you can reply with depends on what reached you (thank someone, ask to talk it through, keep quiet, answer a story, reply to a post). A reply is not typed text: it becomes an intent the world applies on the next day, and the other side answers only if their own mind does."},
        {"area": "Things you do", "status": "ready", "note": "Meetings, transfer requests, agents, retirement, the week's routine, lifestyle, study, home, helpers, giving, investing, press stances, posting online and a second career are queued and applied by the world on the next simulated day. Some are refused by the other side."},
        {"area": "Contract negotiation", "status": "partial", "note": "Multi-round talks with counters work. You choose among the counter-offers the world generates; free-form terms are not accepted."},
        {"area": "Conversations and promises", "status": "ready", "note": "Meetings carry a subject and tone, leave a record on both sides and can produce promises that the world later keeps or breaks."},
        {"area": "Rumours and the grapevine", "status": "ready", "note": "People pass things on, imperfectly. What you hear is framed as what you were told and how sure you are, never as fact. Observers see the truth of every item; an inhabited player never does."},
        {"area": "Life away from football", "status": "ready", "note": "Household, money, routine, well-being and the reasons behind your mood are simulated for everyone. Housing is a kind and a quality, personal staff are effects rather than people, and charity has no named causes."},
        {"area": "Media and public standing", "status": "ready", "note": "Outlets, journalists, stories, story threads, fan standing and social reactions exist. Pundits do not write opinion pieces yet."},
        {"area": "Social media", "status": "ready", "note": "A large population of accounts posts, replies, quotes and builds memes and chants from what happens. You see a personal feed, not the whole network, and can post, reply and quote from your own account."},
        {"area": "Incidents", "status": "partial", "note": "Disputes in the squad, personal trouble and club-wide problems arise from pressures in the world. As a player you are told of the ones that involve you and can ask for time away or apologise. Deciding how to handle them is for a manager, chairman or head of youth, and those cannot be inhabited yet."},
        {"area": "Press conferences and appeals", "status": "partial", "note": "Managers face questions and decide whether to appeal a red card. Both run on the AI's judgement today because only players can be inhabited. Quotes and appeals are recorded and browsable."},
        {"area": "Football below the professional game", "status": "partial", "note": "Schools, universities and amateur leagues have competitions, seasons and records, and players can enrol. Their matches are simulated as results, not played out."},
        {"area": "National teams", "status": "partial", "note": "Squads, call-ups, dual nationality, qualifiers and tournaments run. National managers pick squads by AI only, and youth sides play friendlies."},
        {"area": "Youth pipeline", "status": "partial", "note": "Grassroots and academy football below U18 is statistical, not played out as matches."},
        {"area": "Match watching", "status": "partial", "note": "Recorded events and pitch zones only. There is no continuous player tracking, so the pitch view is an event map."},
        {"area": "Training plan", "status": "partial", "note": "Your individual plan changes your workload and development. Club-wide programmes are not simulated."},
        {"area": "Sponsorship", "status": "partial", "note": "Club shirt and kit deals and personal endorsements exist. Stadium naming and sleeve deals are not generated."},
        {"area": "Staff, owners and national managers", "status": "missing", "note": "Only people who play (or played) can be inhabited. Boards, owners, managers and federations run on AI logic, and their decisions (incident handling, press answers, appeals) have no screen yet."},
        {"area": "Imported data", "status": "partial", "note": "Real-data import has been checked on small hand-made data only. Worlds generated by the game are marked as such, and their history before the start is generated, not recorded."},
        {"area": "Balance", "status": "partial", "note": "All numbers are tunable and none is calibrated against real football yet; expect odd results over long runs (for instance, incidents about exams and house moves are far more common than they should be)."},
        {"area": "Same-day decisions", "status": "missing", "note": "The day pipeline has no safe checkpoint inside a day; decisions resolve at day boundaries."},
    ])
}

pub fn diagnostics(c: &Ctx) -> ApiResult<Value> {
    let w = c.w;
    let t = &c.s.timings;
    let (avg, worst) = if t.is_empty() { (0.0, 0.0) } else { (t.iter().map(|x| f64::from(x.1)).sum::<f64>() / t.len() as f64 / 1000.0, t.iter().map(|x| f64::from(x.1)).fold(0.0, f64::max) / 1000.0) };
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "world": {
            "name": c.s.meta.name, "date": w.date.0, "days_simulated": w.days_simulated, "seed": w.seed,
            "people": w.people.len(), "players": w.players.len(), "staff": w.staff.len(), "clubs": w.clubs.len(), "teams": w.teams.len(),
            "competitions": w.comps.len(), "fixtures": w.fixtures.len(), "events": w.events.len(), "reports": w.reports.len(),
            "decisions": w.decisions.all.len(), "followed": w.followed.len(),
        },
        "timings": {"samples": t.len(), "avg_ms": avg, "worst_ms": worst, "recent": t.iter().rev().take(60).rev().map(|x| json!([x.0, f64::from(x.1) / 1000.0])).collect::<Vec<_>>()},
        "revision": c.s.revision,
        "perspective": if c.observer() { "observer" } else { "inhabit" },
        "capabilities": capabilities(),
    }))
}
