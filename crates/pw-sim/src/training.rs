//! The training week (Slice 2): fixture density, the manager's philosophy and
//! staff advice decide each team's week; the week decides daily load,
//! development emphasis and how well the side has drilled its shapes.
//!
//! Resolved per team on Mondays. A shape the squad has not practised is
//! harder to execute (selection prefers drilled formations, and an undrilled
//! one costs sharpness in the match sheet), which is also what limits national
//! sides that meet for a few days.

use pw_core::TeamId;
use pw_world::ruling::{Outcome, Ruling, RulingKind, Stance, StanceRole};
use pw_world::training::LoadPlan;
use pw_world::{Archetype, StaffRole, TeamKind, World};
use smallvec::SmallVec;

use crate::consider;

/// How drilled a national side is when it has only had a camp.
pub const CAMP_DRILL: f32 = 0.4;

/// Mondays: plan every team's week, then update what they have practised.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let teams: Vec<TeamId> = w.teams.ids().collect();
    for t in teams {
        let club = w.teams[t].club;
        let matches = w.fixtures.of_team_between(t, today, today.add_days(6)).filter(|&f| w.fixtures.get(f).score.is_none()).count() as u8;
        let manager = w.clubs[club].manager.get();
        let phil = manager.map(|m| w.staff[m].philosophy).unwrap_or_default();

        // What the manager wants, before staff have their say.
        let (mut tac, mut phys, mut rec) = (0.40f32, 0.35f32, 0.25f32);
        match phil.archetype {
            Archetype::Pragmatist => tac += 0.10,
            Archetype::Rotator => rec += 0.10,
            Archetype::Developer => phys += 0.05,
            Archetype::Loyalist => {}
        }
        phys += (f32::from(phil.press) - 50.0) / 300.0;
        if matches >= 2 {
            rec += 0.20;
            phys -= 0.15;
        }
        let days = (6i32 - 2 * i32::from(matches)).clamp(1, 6) as u8;
        let wanted = normalise(tac, phys, rec);

        // Staff advice: a tired squad, and someone whose job is to say so.
        let (advised, adviser) = advise(w, t, club, wanted, matches);
        let chosen = if advised != wanted { decide(w, t, club, manager, wanted, advised, adviser, matches) } else { wanted };
        w.ext.training.plans.insert(t, LoadPlan { matches, days, tactical: chosen.0, physical: chosen.1, recovery: chosen.2 });

        if w.teams[t].kind == TeamKind::First || w.teams[t].kind == TeamKind::Reserve {
            drill(w, t, phil.formations, f32::from(chosen.0) / 100.0 * f32::from(days));
        }
    }
}

fn normalise(a: f32, b: f32, c: f32) -> (u8, u8, u8) {
    let (a, b, c) = (a.max(0.05), b.max(0.05), c.max(0.05));
    let s = a + b + c;
    let (x, y) = ((a / s * 100.0).round() as u8, (b / s * 100.0).round() as u8);
    (x, y, 100 - x - y)
}

/// The fitness staff's version of the week: lighter and more recovery when the squad's workload ratio is high.
fn advise(w: &World, team: TeamId, club: pw_core::ClubId, wanted: (u8, u8, u8), matches: u8) -> ((u8, u8, u8), Option<pw_core::PersonId>) {
    let adviser = w.clubs[club].staff.iter().copied().find(|&s| matches!(w.staff[s].role, StaffRole::FitnessCoach | StaffRole::SportsScientist)).map(|s| w.staff[s].person);
    let squad = &w.teams[team].squad;
    if adviser.is_none() || squad.is_empty() || matches == 0 && w.teams[team].kind != TeamKind::First {
        return (wanted, None);
    }
    let acwr = squad.iter().map(|&p| w.players.hot[p].acwr()).sum::<f32>() / squad.len() as f32;
    if acwr <= 1.30 {
        return (wanted, adviser);
    }
    let shift = ((acwr - 1.30) * 40.0).min(15.0) as i32;
    let phys = (i32::from(wanted.1) - shift).max(10) as u8;
    let rec = (i32::from(wanted.2) + shift).min(60) as u8;
    (normalise(f32::from(wanted.0), f32::from(phys), f32::from(rec)), adviser)
}

/// The manager takes or ignores the advice; an overruled warning is a material decision and is remembered.
#[allow(clippy::too_many_arguments)]
fn decide(w: &mut World, team: TeamId, club: pw_core::ClubId, manager: Option<pw_core::StaffId>, wanted: (u8, u8, u8), advised: (u8, u8, u8), adviser: Option<pw_core::PersonId>, matches: u8) -> (u8, u8, u8) {
    let Some(m) = manager else { return advised };
    let mp = w.staff[m].person;
    // Trust in the adviser against his own stubbornness (pragmatists and pressed managers ignore warnings).
    let trust = adviser.map_or(0.5, |a| consider::trust(w, mp, a));
    let stubborn = match w.staff[m].philosophy.archetype {
        Archetype::Pragmatist => 0.25,
        Archetype::Loyalist => 0.15,
        Archetype::Developer => 0.05,
        Archetype::Rotator => -0.1,
    } + if w.clubs[club].board.satisfaction < 40 { 0.15 } else { 0.0 };
    let roll = w.roll(pw_core::rng::stream::TRAINING, &[u64::from(team.0), (w.date.0 / 7) as u64, 0x10ad]);
    let listens = roll < (0.55 + 0.5 * (trust - 0.5) - stubborn).clamp(0.05, 0.95);
    if listens {
        return advised;
    }
    if matches >= 1 {
        let today = w.date;
        let mut stances: SmallVec<[Stance; 4]> = SmallVec::new();
        stances.push(Stance { who: mp, role: StanceRole::Manager, believed_pct: 255, backing: 80, authority: true });
        if let Some(a) = adviser {
            stances.push(Stance { who: a, role: StanceRole::Medical, believed_pct: 255, backing: -70, authority: false });
        }
        let id = w.ext.decisions.add(Ruling {
            id: 0,
            kind: RulingKind::LoadPlan,
            date: today,
            club,
            subject: pw_core::PlayerId::NONE,
            about: mp,
            liability: 0,
            decider: mp,
            stances,
            true_pct: 0,
            want: wanted.1,
            outcome: Outcome::Enacted,
            resolved: Some(today),
            event: pw_core::EventId::NONE,
        });
        let ev = w.events.push(today, pw_world::event::Visibility::Club(club), pw_world::event::EventKind::Ruling { ruling: id });
        if let Some(r) = w.ext.decisions.get_mut(id) {
            r.event = ev;
        }
        if let Some(a) = adviser {
            let c = consider::compat(w, a, mp);
            w.social.remember(a, mp, pw_world::MemoryKind::LetDown, today, ev, false, 0.4, c);
        }
    }
    wanted
}

/// Practice raises the drill of the manager's two formations and lets others fade.
fn drill(w: &mut World, team: TeamId, formations: [u8; 2], tactical_days: f32) {
    let v = w.ext.training.familiarity.entry(team).or_default();
    for x in v.iter_mut() {
        if !formations.contains(&x.0) {
            x.1 = (f32::from(x.1) * 0.94) as u8;
        }
    }
    for (i, &f) in formations.iter().enumerate() {
        let gain = tactical_days * if i == 0 { 5.0 } else { 2.5 };
        match v.iter_mut().find(|x| x.0 == f) {
            Some(x) => x.1 = (f32::from(x.1) + gain * (1.0 - f32::from(x.1) / 100.0)).min(100.0) as u8,
            None => {
                if v.len() >= 3 {
                    let weakest = v.iter().enumerate().min_by_key(|(_, x)| x.1).map(|(i, _)| i).unwrap_or(0);
                    v.remove(weakest);
                }
                v.push((f, (gain.min(100.0)) as u8));
            }
        }
    }
    v.retain(|x| x.1 > 0);
}
