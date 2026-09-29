//! The coach's window into the OFM backend: what the touchline can see, and how a side's call reaches the game.

use std::collections::HashMap;

use ofm_engine::event::{DangerBand, EventDetail};

use super::*;

/// What each player's state does to how he plays: focus to attention and decisions, calm to composure and temper, risk to flair,
/// drive to work off the ball, confidence to the form he plays to. Different states move different things.
pub fn with_minds<'a>(inp: &MatchInput<'a>, coach: &dyn Coach) -> MatchInput<'a> {
    let mut out = inp.clone();
    for (side, sheet) in [&mut out.home, &mut out.away].into_iter().enumerate() {
        for p in sheet.xi.iter_mut().chain(sheet.bench.iter_mut()) {
            let m = coach.mind(side as u8, p.id);
            if !m.is_neutral() {
                apply_mind(p, m.clamped());
            }
        }
    }
    out
}

fn apply_mind(p: &mut PlayerSheet, m: Mind) {
    let mut shift = |a: Attr, d: f32| {
        if d != 0.0 {
            p.attrs.set(a, (p.attrs.get(a) + d).clamp(1.0, 20.0));
        }
    };
    shift(Attr::Concentration, 2.5 * m.focus);
    shift(Attr::Decisions, 2.0 * m.focus);
    shift(Attr::Anticipation, m.focus);
    shift(Attr::Composure, 2.5 * m.calm);
    shift(Attr::Aggression, -2.0 * m.calm);
    shift(Attr::Flair, 2.0 * m.risk);
    shift(Attr::OffTheBall, m.risk);
    shift(Attr::WorkRate, 2.5 * m.drive);
    p.morale = (p.morale + 15.0 * m.confidence).clamp(0.0, 100.0);
}

/// Keep of the engine manager's commands only substitutions for players who are visibly spent.
pub fn keep_tired_subs(state: &LiveMatchState, cmds: &mut Vec<MatchCommand>) {
    if !cmds.iter().any(|c| matches!(c, MatchCommand::Substitute { .. })) {
        cmds.clear();
        return;
    }
    let snap = state.snapshot();
    let tired = |side: Side, id: &str| {
        let team = if side == Side::Home { &snap.home_team } else { &snap.away_team };
        team.players.iter().find(|p| p.id == id).is_some_and(|p| p.condition < 60)
    };
    cmds.retain(|c| matches!(c, MatchCommand::Substitute { side, player_off_id, .. } if tired(*side, player_off_id)));
}

#[derive(Default, Clone, Copy)]
struct SideAcc {
    shots: u16,
    on_target: u16,
    big_chances: u16,
    corners: u16,
    fouls: u16,
    yellows: u16,
    reds: u16,
    territory: u16,
    lost_own_third: u16,
}

pub fn default_role(pos: Pos) -> Role {
    match pos.group() {
        PosGroup::Gk => Role::Goalkeeper,
        PosGroup::Def => Role::CentreBack,
        PosGroup::Mid => Role::CentralMidfielder,
        PosGroup::Att => Role::CompleteForward,
    }
}

/// What is visible from the touchline: this is the whole of what a manager's staff are given.
pub fn look_at(snap: &ofm_engine::MatchSnapshot, inp: &MatchInput, live: &[Tactics; 2], minute: u8, half_time: bool) -> Look {
    let mut pos_of: HashMap<u32, (Pos, Role)> = HashMap::new();
    for t in [&inp.home, &inp.away] {
        for (p, s) in t.xi.iter().zip(t.slots.iter()) {
            pos_of.insert(p.id.0, (s.pos, s.role));
        }
        for p in &t.bench {
            let pos = bench_pos(p);
            pos_of.insert(p.id.0, (pos, default_role(pos)));
        }
    }
    let ids = |s: &str| s.parse::<u32>().ok();
    // A substitute takes over the shape of the man he replaced.
    for r in &snap.substitutions {
        if let (Some(on), Some(off)) = (ids(&r.player_on_id), ids(&r.player_off_id))
            && let Some(&v) = pos_of.get(&off)
        {
            pos_of.insert(on, v);
        }
    }
    let mut tallies: HashMap<u32, Tally> = HashMap::new();
    let mut sides = [SideAcc::default(), SideAcc::default()];
    for e in &snap.events {
        let s = usize::from(e.side == Side::Away);
        let att_third = e.zone == Zone::attacking_third(e.side) || e.zone == Zone::attacking_box(e.side);
        let own_third = e.zone == Zone::defensive_third(e.side) || e.zone == Zone::attacking_box(e.side.opposite());
        let p = e.player_id.as_deref().and_then(ids);
        let q = e.secondary_player_id.as_deref().and_then(ids);
        let mut bump = |id: Option<u32>, f: fn(&mut Tally)| {
            if let Some(id) = id {
                f(tallies.entry(id).or_default());
            }
        };
        match e.event_type {
            EventType::PassCompleted => {
                bump(p, |t| t.passes_ok = t.passes_ok.saturating_add(1));
                if att_third {
                    sides[s].territory += 1;
                }
            }
            EventType::PassIntercepted => {
                bump(p, |t| t.passes_lost = t.passes_lost.saturating_add(1));
                if own_third {
                    sides[s].lost_own_third += 1;
                }
            }
            EventType::Tackle | EventType::Interception => bump(p, |t| t.duels_won = t.duels_won.saturating_add(1)),
            EventType::Dribble => {
                bump(p, |t| t.duels_won = t.duels_won.saturating_add(1));
                sides[s].territory += 1;
            }
            EventType::DribbleTackled => bump(p, |t| t.duels_lost = t.duels_lost.saturating_add(1)),
            EventType::Foul => {
                bump(p, |t| t.fouls = t.fouls.saturating_add(1));
                bump(q, |t| t.fouled = t.fouled.saturating_add(1));
                sides[s].fouls += 1;
            }
            EventType::ShotOnTarget | EventType::ShotSaved | EventType::Goal | EventType::PenaltyGoal => {
                bump(p, |t| t.shots = t.shots.saturating_add(1));
                sides[s].shots += 1;
                sides[s].on_target += 1;
                sides[s].territory += 1;
            }
            EventType::ShotOffTarget | EventType::ShotBlocked | EventType::PenaltyMiss => {
                bump(p, |t| t.shots = t.shots.saturating_add(1));
                sides[s].shots += 1;
                sides[s].territory += 1;
            }
            EventType::Corner => sides[s].corners += 1,
            EventType::YellowCard | EventType::SecondYellow => sides[s].yellows += 1,
            EventType::RedCard => sides[s].reds += 1,
            _ => {}
        }
        if matches!(e.detail, Some(EventDetail::Shot { danger: DangerBand::BigChance })) {
            sides[s].big_chances += 1;
        }
    }
    let side_look = |s: usize| -> SideLook {
        let (team, bench, yellows, subs) = if s == 0 { (&snap.home_team, &snap.home_bench, &snap.home_yellows, snap.home_subs_made) } else { (&snap.away_team, &snap.away_bench, &snap.away_yellows, snap.away_subs_made) };
        let a = &sides[s];
        let players = team
            .players
            .iter()
            .filter(|p| !snap.sent_off.contains(&p.id))
            .filter_map(|p| {
                let id = ids(&p.id)?;
                let (pos, role) = pos_of.get(&id).copied().unwrap_or((Pos::MC, Role::CentralMidfielder));
                Some(PlayerLook { player: PlayerId(id), pos, role, condition: p.condition, booked: yellows.contains_key(&p.id), tally: tallies.get(&id).copied().unwrap_or_default() })
            })
            .collect();
        let bench = bench.iter().filter(|p| !snap.sent_off.contains(&p.id)).filter_map(|p| ids(&p.id).map(|id| (PlayerId(id), pos_of.get(&id).map_or(Pos::MC, |x| x.0)))).collect();
        SideLook {
            tactics: live[s],
            possession: (if s == 0 { snap.home_possession_pct } else { snap.away_possession_pct }).round() as u8,
            shots: a.shots,
            on_target: a.on_target,
            big_chances: a.big_chances,
            corners: a.corners,
            fouls: a.fouls,
            yellows: a.yellows,
            reds: a.reds,
            territory: a.territory,
            lost_own_third: a.lost_own_third,
            subs_made: subs,
            subs_max: snap.max_subs,
            players,
            bench,
        }
    };
    Look { minute, half_time, goals: [snap.home_score, snap.away_score], sides: [side_look(0), side_look(1)] }
}

/// Carry a side's call into the game: instructions become the engine's play style, roles and substitutions are commands.
pub fn apply_call(state: &mut LiveMatchState, coach: &mut dyn Coach, side: u8, call: &Call, live: &mut Tactics, style: &mut PlayStyle) {
    if call.is_empty() {
        return;
    }
    let sd = if side == 0 { Side::Home } else { Side::Away };
    if let Some(t) = call.tactics {
        *live = t;
        let next = play_style(&t);
        if next != *style {
            *style = next;
            let _ = state.apply_command(MatchCommand::ChangePlayStyle { side: sd, play_style: next });
        }
    }
    for &(p, role) in &call.roles {
        let _ = state.apply_command(MatchCommand::ChangePlayerRole { side: sd, player_id: p.0.to_string(), role: map_role(role) });
    }
    let made: SmallVec<[bool; 3]> = call
        .subs
        .iter()
        .map(|s| state.apply_command(MatchCommand::Substitute { side: sd, player_off_id: s.out.0.to_string(), player_on_id: s.inn.0.to_string() }).is_ok())
        .collect();
    coach.applied(side, call, &made);
}
