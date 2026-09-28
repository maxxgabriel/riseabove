//! Adapter for OpenFootManager's live match engine.
//!
//! Maps Pathway's 46-attribute 1–20 players onto OFM's 19-attribute 0–100
//! model, derives OFM's archetype traits from attributes, drives
//! `LiveMatchState` minute by minute with OFM's own in-game AI managers, and
//! converts the report into a `MatchResult` with Pathway ratings.

use ofm_engine::ai::{self, AiPersonality, AiProfile};
use ofm_engine::{
    DefensiveLine, EventType, LiveMatchState, MatchConfig, MatchEvent as OfmEvent, PlayStyle, PlayerData, PlayerRole, Position, PressingIntensity, Side, TacticsBuildUpStyle, TacticsConfig,
    TacticsPitchWidth, TeamData, Tempo, Zone,
};
use pw_core::rng::hash2;
use pw_core::{Attr, Hidden, Mentality, PlayerId, Pos, PosGroup, Role, Tactics};
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::pitch::{N_ZONES, zone_id};
use crate::types::*;

pub fn simulate(inp: &MatchInput) -> MatchResult {
    let home = team(&inp.home, inp);
    let away = team(&inp.away, inp);
    let home_bench = inp.home.bench.iter().map(|p| player(p, bench_pos(p), None, inp)).collect();
    let away_bench = inp.away.bench.iter().map(|p| player(p, bench_pos(p), None, inp)).collect();

    let t = inp.tuning;
    let config = MatchConfig {
        home_advantage: if inp.neutral { 1.0 } else { 1.0 + f64::from(t.home_advantage) * 0.23 },
        goal_conversion_base: f64::from(t.ofm_conversion),
        yellow_card_probability: f64::from(t.yellow_rate) * f64::from(inp.referee_strictness),
        fatigue_per_minute: f64::from(t.ofm_fatigue),
        ..MatchConfig::default()
    };
    // Aggregate ties are settled after 90' by `settle_aggregate`, not by the engine's ET.
    let decisive_today = inp.decisive && inp.first_leg.is_none();
    let mut state = LiveMatchState::new(home, away, config, home_bench, away_bench, decisive_today);
    let profiles = [profile(&inp.home), profile(&inp.away)];
    let mut rng = StdRng::seed_from_u64(inp.seed);

    // Planned rotation: how many subs each side wants made by each checkpoint.
    let mut plan_rng = pw_core::Rng::keyed(&[inp.seed, 0x5ab5]);
    let total = [2 + plan_rng.below(3) as u8, 2 + plan_rng.below(3) as u8];
    let planned: [(u8, [u8; 2]); 4] = [(60, [1, 1]), (70, [total[0].min(2), total[1].min(2)]), (78, [total[0].min(3), total[1].min(3)]), (86, total)];
    let mut guard = 0;
    while !state.is_finished() && guard < 400 {
        let r = state.step_minute(&mut rng);
        // The AI manager snapshots the whole match; consult it only when a
        // decision is plausible (second half on, every other minute, or after a card).
        let carded = r.events.iter().any(|e| matches!(e.event_type, EventType::RedCard | EventType::SecondYellow | EventType::Injury));
        if carded || (r.minute >= 55 && r.minute % 2 == 0) {
            for (side, prof) in [(Side::Home, &profiles[0]), (Side::Away, &profiles[1])] {
                for cmd in ai::ai_decide(&state, side, prof, &mut rng) {
                    let _ = state.apply_command(cmd);
                }
            }
        }
        if let Some(wanted) = planned.iter().position(|&(m, _)| m == r.minute).map(|i| planned[i].1) {
            planned_subs(&mut state, Side::Home, wanted[0]);
            planned_subs(&mut state, Side::Away, wanted[1]);
        }
        guard += 1;
    }
    let report = state.into_report();
    convert(inp, report)
}

// ------------------------------------------------------------------ inputs

fn profile(t: &TeamSheet) -> AiProfile {
    AiProfile {
        reputation: 500,
        experience: 70,
        personality: match t.manager_reactivity {
            r if r >= 0.66 => AiPersonality::Reactive,
            r if r >= 0.4 => AiPersonality::Visionary,
            _ => AiPersonality::Pragmatist,
        },
    }
}

fn team(t: &TeamSheet, inp: &MatchInput) -> TeamData {
    let players: Vec<PlayerData> = t.xi.iter().zip(t.slots.iter()).map(|(p, s)| player(p, s.pos, Some(s.role), inp)).collect();
    let (d, m, f) = t.slots.iter().fold((0, 0, 0), |(d, m, f), s| match group(s.pos) {
        Position::Defender => (d + 1, m, f),
        Position::Midfielder => (d, m + 1, f),
        Position::Forward => (d, m, f + 1),
        Position::Goalkeeper => (d, m, f),
    });
    TeamData { id: t.team.0.to_string(), name: String::new(), formation: format!("{d}-{m}-{f}"), play_style: play_style(&t.tactics), players, tactics: tactics(&t.tactics) }
}

fn play_style(t: &Tactics) -> PlayStyle {
    match () {
        _ if t.press >= 70 => PlayStyle::HighPress,
        _ if t.mentality >= Mentality::Positive => PlayStyle::Attacking,
        _ if t.mentality <= Mentality::Defensive => PlayStyle::Defensive,
        _ if t.directness <= 30 && t.tempo <= 50 => PlayStyle::Possession,
        _ if t.directness >= 70 => PlayStyle::Counter,
        _ => PlayStyle::Balanced,
    }
}

fn tactics(t: &Tactics) -> TacticsConfig {
    TacticsConfig {
        pressing_intensity: match t.press {
            0..=35 => PressingIntensity::Passive,
            36..=65 => PressingIntensity::Medium,
            _ => PressingIntensity::Aggressive,
        },
        defensive_line: match t.line {
            0..=20 => DefensiveLine::VeryLow,
            21..=40 => DefensiveLine::Low,
            41..=65 => DefensiveLine::Medium,
            _ => DefensiveLine::High,
        },
        width: match t.width {
            0..=35 => TacticsPitchWidth::Narrow,
            36..=65 => TacticsPitchWidth::Normal,
            _ => TacticsPitchWidth::Wide,
        },
        build_up_style: match t.directness {
            0..=35 => TacticsBuildUpStyle::Short,
            36..=65 => TacticsBuildUpStyle::Mixed,
            _ => TacticsBuildUpStyle::Long,
        },
        tempo: if t.tempo >= 55 { Tempo::Direct } else { Tempo::Patient },
        ..TacticsConfig::default()
    }
}

fn group(p: Pos) -> Position {
    match p.group() {
        PosGroup::Gk => Position::Goalkeeper,
        PosGroup::Def => Position::Defender,
        PosGroup::Mid => Position::Midfielder,
        PosGroup::Att => Position::Forward,
    }
}

/// Where a substitute would naturally play.
fn bench_pos(p: &PlayerSheet) -> Pos {
    Pos::ALL.into_iter().max_by_key(|q| p.familiarity[q.idx()]).unwrap_or(Pos::MC)
}

fn player(p: &PlayerSheet, pos: Pos, role: Option<Role>, inp: &MatchInput) -> PlayerData {
    let a = |x: Attr| p.attrs.get(x);
    let form = (0.97 + 0.06 * p.morale / 100.0) * (0.9 + 0.1 * p.sharpness / 100.0) * (1.0 + inp.importance * (p.hidden.f(Hidden::ImportantMatches) - 10.0) / 10.0 * 0.03);
    let scale = |v: f32| (v * 5.0 * form).round().clamp(1.0, 100.0) as u8;
    let fam = pw_core::math::lerp(0.7, 1.0, (f32::from(p.familiarity[pos.idx()]) / 20.0).clamp(0.0, 1.0));
    let fit = |v: f32| scale(v * fam);
    let outfield_pos = pos.group() != PosGroup::Gk;
    let positioning =
        if matches!(pos.group(), PosGroup::Att | PosGroup::Mid) { a(Attr::OffTheBall) * 0.5 + a(Attr::Anticipation) * 0.5 } else { a(Attr::Positioning) * 0.6 + a(Attr::Anticipation) * 0.4 };
    let aerial = if outfield_pos { (a(Attr::Heading) + a(Attr::JumpingReach)) * 0.5 } else { a(Attr::AerialReach) };
    let mut data = PlayerData {
        id: p.id.0.to_string(),
        name: String::new(),
        position: group(pos),
        ovr: 0,
        condition: p.condition.clamp(1.0, 100.0) as u8,
        fitness: (p.sharpness * 0.5 + a(Attr::NaturalFitness) * 2.0).clamp(1.0, 100.0) as u8,
        pace: fit(a(Attr::Pace) * 0.6 + a(Attr::Acceleration) * 0.4),
        stamina: scale(a(Attr::Stamina)),
        strength: scale(a(Attr::Strength)),
        agility: fit(a(Attr::Agility) * 0.6 + a(Attr::Balance) * 0.4),
        passing: fit(a(Attr::Passing) * 0.6 + a(Attr::Technique) * 0.2 + a(Attr::FirstTouch) * 0.2),
        shooting: fit(a(Attr::Finishing) * 0.6 + a(Attr::LongShots) * 0.2 + a(Attr::Technique) * 0.2),
        tackling: fit(a(Attr::Tackling)),
        dribbling: fit(a(Attr::Dribbling) * 0.7 + a(Attr::Technique) * 0.15 + a(Attr::Flair) * 0.15),
        defending: fit(a(Attr::Marking) * 0.4 + a(Attr::Positioning) * 0.3 + a(Attr::Concentration) * 0.3),
        positioning: fit(positioning),
        vision: fit(a(Attr::Vision)),
        decisions: fit(a(Attr::Decisions)),
        composure: scale(a(Attr::Composure)),
        aggression: scale(a(Attr::Aggression)),
        teamwork: scale(a(Attr::Teamwork)),
        leadership: scale(a(Attr::Leadership)),
        handling: fit(a(Attr::Handling)),
        reflexes: fit(a(Attr::Reflexes)),
        aerial: fit(aerial),
        traits: derive_traits(p),
        role: role.map(map_role).unwrap_or_default(),
    };
    data.ovr = data.overall().round() as u8;
    data
}

/// OFM's archetype traits, derived from the attributes they summarise.
fn derive_traits(p: &PlayerSheet) -> Vec<String> {
    let a = |x: Attr| p.attrs.get(x);
    let h = |x: Hidden| p.hidden.f(x);
    let rules: [(&str, bool); 19] = [
        ("Sharpshooter", a(Attr::Finishing) >= 17.0),
        ("CoolHead", a(Attr::Composure) >= 17.0),
        ("Dribbler", a(Attr::Dribbling) >= 17.0),
        ("Speedster", a(Attr::Pace) >= 17.0 && a(Attr::Acceleration) >= 15.0),
        ("Agile", a(Attr::Agility) >= 17.0),
        ("AerialDominance", a(Attr::Heading) + a(Attr::JumpingReach) >= 34.0),
        ("BallWinner", a(Attr::Tackling) >= 17.0),
        ("CatReflexes", a(Attr::Reflexes) >= 17.0),
        ("Engine", a(Attr::Stamina) >= 17.0 && a(Attr::WorkRate) >= 15.0),
        ("HotHead", h(Hidden::Temperament) <= 5.0),
        ("Playmaker", a(Attr::Passing) + a(Attr::Vision) >= 34.0),
        ("Rock", a(Attr::Marking) + a(Attr::Positioning) >= 34.0),
        ("SafeHands", a(Attr::Handling) >= 17.0),
        ("SetPieceSpecialist", a(Attr::FreeKicks).max(a(Attr::Corners)) >= 17.0),
        ("Tank", a(Attr::Strength) >= 17.0),
        ("TeamPlayer", a(Attr::Teamwork) >= 17.0),
        ("Tireless", a(Attr::NaturalFitness) >= 17.0),
        ("Visionary", a(Attr::Vision) >= 17.0),
        ("CompleteForward", [Attr::Finishing, Attr::Heading, Attr::Dribbling, Attr::Passing].iter().all(|&x| a(x) >= 15.0)),
    ];
    rules.iter().filter(|(_, on)| *on).map(|(n, _)| (*n).to_string()).collect()
}

fn map_role(r: Role) -> PlayerRole {
    match r {
        Role::Goalkeeper => PlayerRole::Standard,
        Role::SweeperKeeper => PlayerRole::SweeperKeeper,
        Role::CentreBack => PlayerRole::Stopper,
        Role::BallPlayingDefender => PlayerRole::BallPlayingCB,
        Role::FullBack => PlayerRole::DefensiveFB,
        Role::WingBack => PlayerRole::WingBack,
        Role::InvertedWingBack => PlayerRole::InvertedFB,
        Role::Anchor => PlayerRole::AnchorMan,
        Role::DeepLyingPlaymaker => PlayerRole::DeepLyingPlaymaker,
        Role::BallWinner => PlayerRole::BallWinner,
        Role::CentralMidfielder => PlayerRole::Carrilero,
        Role::BoxToBox => PlayerRole::BoxToBox,
        Role::Mezzala => PlayerRole::Mezzala,
        Role::AdvancedPlaymaker => PlayerRole::AdvancedPlaymaker,
        Role::WideMidfielder | Role::Winger => PlayerRole::WideForward,
        Role::InvertedWinger => PlayerRole::InvertedWinger,
        Role::InsideForward => PlayerRole::InsideForward,
        Role::ShadowStriker => PlayerRole::ShadowStriker,
        Role::TargetForward => PlayerRole::TargetMan,
        Role::Poacher => PlayerRole::Poacher,
        Role::PressingForward => PlayerRole::PressingForward,
        Role::CompleteForward => PlayerRole::CompleteForward,
        Role::FalseNine => PlayerRole::False9,
    }
}

/// Bring on fresh legs for the most tired outfielders until `wanted` subs are made.
fn planned_subs(state: &mut LiveMatchState, side: Side, wanted: u8) {
    let snap = state.snapshot();
    let (team, bench, made) = match side {
        Side::Home => (&snap.home_team, &snap.home_bench, snap.home_subs_made),
        Side::Away => (&snap.away_team, &snap.away_bench, snap.away_subs_made),
    };
    let mut made = made;
    let mut used: Vec<&str> = Vec::new();
    let mut outs: Vec<&PlayerData> = team.players.iter().filter(|p| p.position != Position::Goalkeeper && !snap.sent_off.contains(&p.id) && p.condition < 88).collect();
    outs.sort_by_key(|p| p.condition);
    for off in outs {
        if made >= wanted || made >= snap.max_subs {
            break;
        }
        let Some(on) = bench.iter().filter(|b| b.position == off.position && !used.contains(&b.id.as_str())).max_by(|a, b| a.overall().total_cmp(&b.overall())) else {
            continue;
        };
        let cmd = ofm_engine::MatchCommand::Substitute { side, player_off_id: off.id.clone(), player_on_id: on.id.clone() };
        if state.apply_command(cmd).is_ok() {
            used.push(&on.id);
            made += 1;
        }
    }
}

// ----------------------------------------------------------------- outputs

struct Who {
    sheet: usize,
    side: u8,
    started: bool,
    pos: Option<Pos>,
    on: u8,
    off: Option<u8>,
    line: PlayerLine,
}

fn convert(inp: &MatchInput, rep: ofm_engine::MatchReport) -> MatchResult {
    let sheets: Vec<(&PlayerSheet, u8, Option<Pos>)> = [&inp.home, &inp.away]
        .into_iter()
        .enumerate()
        .flat_map(|(s, t)| t.xi.iter().zip(t.slots.iter()).map(move |(p, sl)| (p, s as u8, Some(sl.pos))).chain(t.bench.iter().map(move |p| (p, s as u8, None))))
        .collect();
    let index = |id: &str| -> Option<usize> {
        let n: u32 = id.parse().ok()?;
        sheets.iter().position(|(p, _, _)| p.id.0 == n)
    };
    let mut who: Vec<Who> = sheets
        .iter()
        .enumerate()
        .map(|(i, &(p, side, pos))| Who { sheet: i, side, started: pos.is_some(), pos, on: 0, off: None, line: PlayerLine { player: p.id, side, started: pos.is_some(), pos, ..Default::default() } })
        .collect();

    let side_of = |s: Side| -> u8 { if s == Side::Home { 0 } else { 1 } };
    let mut events = Vec::with_capacity(rep.events.len());
    let mut stats = [TeamStats::default(), TeamStats::default()];
    let mut ht_minute = 45u8;
    let mut goal_minutes: Vec<(u8, u8)> = Vec::new();

    for e in &rep.events {
        let side = side_of(e.side);
        let p = e.player_id.as_deref().and_then(index);
        let q = e.secondary_player_id.as_deref().and_then(index);
        let kind = match e.event_type {
            EventType::KickOff => Some(Ev::KickOff),
            EventType::HalfTime => {
                ht_minute = e.minute;
                Some(Ev::HalfTime)
            }
            EventType::FullTime => Some(Ev::FullTime),
            EventType::Goal => Some(Ev::Goal),
            EventType::PenaltyGoal => Some(Ev::PenaltyGoal),
            EventType::PenaltyMiss => Some(Ev::PenaltyMiss),
            EventType::ShotSaved | EventType::ShotOnTarget => Some(Ev::ShotSaved),
            EventType::ShotOffTarget => Some(Ev::ShotWide),
            EventType::ShotBlocked => Some(Ev::ShotBlocked),
            EventType::Cross => Some(Ev::Cross),
            EventType::Dribble => Some(Ev::Dribble),
            EventType::Tackle | EventType::DribbleTackled => Some(Ev::Tackle),
            EventType::Interception => Some(Ev::Interception),
            EventType::Clearance => Some(Ev::Clearance),
            EventType::Corner => Some(Ev::Corner),
            EventType::FreeKick => Some(Ev::FreeKick),
            EventType::Foul => Some(Ev::Foul),
            EventType::YellowCard => Some(Ev::Yellow),
            EventType::SecondYellow => Some(Ev::SecondYellow),
            EventType::RedCard => Some(Ev::Red),
            EventType::Substitution => Some(Ev::Sub),
            EventType::Injury => Some(Ev::Injury),
            EventType::ShootoutGoal => Some(Ev::ShootoutGoal),
            EventType::ShootoutMiss => Some(Ev::ShootoutMiss),
            EventType::PassCompleted | EventType::PassIntercepted => Some(Ev::Chain),
            EventType::SecondHalfStart | EventType::GoalKick | EventType::PenaltyAwarded => None,
        };

        // Per-event bookkeeping the OFM report does not aggregate for us.
        let st = &mut stats[side as usize];
        match e.event_type {
            EventType::Goal | EventType::PenaltyGoal => goal_minutes.push((e.minute, side)),
            EventType::Corner => st.corners += 1,
            EventType::ShotSaved | EventType::ShotOnTarget => {
                stats[1 - side as usize].saves += 1;
                if let Some(k) = keeper_on(&who, 1 - side, e.minute) {
                    who[k].line.saves += 1;
                }
            }
            EventType::Dribble => {
                if let Some(i) = p {
                    who[i].line.dribbles += 1;
                    who[i].line.dribbles_won += 1;
                }
            }
            EventType::DribbleTackled => {
                if let Some(i) = q {
                    who[i].line.dribbles += 1;
                }
            }
            EventType::Cross => {
                if let Some(i) = p {
                    who[i].line.crosses += 1;
                }
            }
            EventType::Clearance => {
                if let Some(i) = p {
                    who[i].line.clearances += 1;
                }
            }
            EventType::ShotBlocked => {
                if let Some(i) = q {
                    who[i].line.blocks += 1;
                }
            }
            EventType::Foul => {
                if let Some(i) = q {
                    who[i].line.fouled += 1;
                }
            }
            EventType::Injury => {
                if let Some(i) = p {
                    who[i].line.injured = true;
                }
            }
            EventType::Substitution => {
                if let (Some(on), Some(off)) = (p, q) {
                    who[on].on = e.minute;
                    who[on].pos = who[off].pos;
                    who[on].line.pos = who[off].pos;
                    who[off].off = Some(e.minute);
                }
            }
            EventType::RedCard | EventType::SecondYellow => {
                if let Some(i) = p {
                    who[i].off = Some(e.minute);
                }
            }
            _ => {}
        }

        if let Some(kind) = kind {
            if inp.lod == Lod::Full || kind.is_key() {
                let xg = shot_xg(e);
                events.push(MatchEvent {
                    t: u16::from(e.minute.saturating_sub(1)) * 60 + (hash2(inp.seed, events.len() as u64) % 60) as u16,
                    side,
                    kind,
                    player: p.map_or(PlayerId::NONE, |i| sheets[i].0.id),
                    other: q.map_or(PlayerId::NONE, |i| sheets[i].0.id),
                    zone: zone_index(e.zone, e.side) as u8,
                    value: xg,
                });
            }
            if matches!(kind, Ev::Goal | Ev::ShotSaved | Ev::ShotWide | Ev::ShotBlocked | Ev::PenaltyGoal | Ev::PenaltyMiss) {
                let xg = shot_xg(e);
                stats[side as usize].xg += xg;
                if xg >= 0.3 {
                    stats[side as usize].big_chances += 1;
                }
                if let Some(i) = p {
                    who[i].line.xg += xg;
                }
            }
        }
    }

    // Aggregate stats from OFM's own team and player counters.
    for (s, ts) in [&rep.home_stats, &rep.away_stats].into_iter().enumerate() {
        let st = &mut stats[s];
        st.shots = ts.shots;
        st.on_target = ts.shots_on_target;
        st.fouls = ts.fouls;
        st.passes = ts.passes_completed + ts.passes_intercepted;
        st.passes_completed = ts.passes_completed;
        st.tackles = ts.tackles;
        st.yellows = u16::from(ts.yellow_cards);
        st.reds = u16::from(ts.red_cards);
    }
    let home_poss = rep.home_possession.round().clamp(0.0, 100.0) as u8;
    stats[0].possession = home_poss;
    stats[1].possession = 100 - home_poss;

    let total = rep.total_minutes.max(90);
    let (hg, ag) = (rep.home_goals, rep.away_goals);
    for w in &mut who {
        let (sheet, _, _) = sheets[w.sheet];
        if let Some(ps) = rep.player_stats.get(&sheet.id.0.to_string()) {
            let l = &mut w.line;
            l.goals = ps.goals;
            l.assists = ps.assists;
            l.shots = ps.shots;
            l.on_target = ps.shots_on_target;
            l.passes = u16::from(ps.passes_attempted);
            l.passes_completed = u16::from(ps.passes_completed);
            l.tackles = ps.tackles_won;
            l.tackles_won = ps.tackles_won;
            l.interceptions = ps.interceptions;
            l.fouls = ps.fouls_committed;
            l.yellows = ps.yellow_cards;
            l.reds = ps.red_cards;
        }
        let played = w.started || w.on > 0;
        let from = if w.started { 0 } else { w.on };
        let to = w.off.unwrap_or(total);
        w.line.minutes = if played { to.saturating_sub(from).max(1) } else { 0 };
        w.line.on_at = from;
        w.line.off_at = w.off.unwrap_or(0);
        w.line.is_keeper = w.pos == Some(Pos::GK);
        let stamina = sheet.attrs.get(Attr::Stamina);
        w.line.condition_end = (sheet.condition - f32::from(w.line.minutes) * (0.45 - stamina * 0.012)).clamp(15.0, 100.0) as u8;
    }

    // Goals conceded while each player was on the pitch.
    let mut conceded = vec![0u8; who.len()];
    for &(minute, scoring_side) in &goal_minutes {
        for (i, w) in who.iter().enumerate() {
            let on = (w.started || w.on > 0) && w.on <= minute && w.off.is_none_or(|o| o >= minute);
            if on && w.side != scoring_side {
                conceded[i] += 1;
            }
        }
    }
    let ht = goal_minutes.iter().filter(|(m, _)| *m <= ht_minute).fold((0u8, 0u8), |(h, a), &(_, s)| if s == 0 { (h + 1, a) } else { (h, a + 1) });

    let mut pens = rep.home_penalties.zip(rep.away_penalties);
    if pens.is_none() && inp.decisive {
        pens = settle_aggregate(inp, hg, ag);
    }
    let sign = [f32::from(i8::from(hg > ag) - i8::from(hg < ag)), f32::from(i8::from(ag > hg) - i8::from(ag < hg))];

    let mut lines = Vec::with_capacity(who.len());
    let mut best: Option<(f32, u8, PlayerId)> = None;
    for (i, w) in who.into_iter().enumerate() {
        let mut l = w.line;
        if l.is_keeper {
            l.conceded = conceded[i];
        }
        if l.minutes > 0 {
            l.rating = rating(&l, conceded[i], sign[l.side as usize]);
            let key = (l.rating, l.goals, l.player);
            if best.is_none_or(|b| (key.0, key.1) > (b.0, b.1)) {
                best = Some(key);
            }
        }
        lines.push(l);
    }

    MatchResult {
        home: inp.home.team,
        away: inp.away.team,
        home_goals: hg,
        away_goals: ag,
        ht,
        extra_time: rep.total_minutes > 100,
        pens,
        stats,
        lines,
        events,
        pom: best.map_or(PlayerId::NONE, |b| b.2),
    }
}

fn keeper_on(who: &[Who], side: u8, minute: u8) -> Option<usize> {
    who.iter().position(|w| w.side == side && w.pos == Some(Pos::GK) && (w.started || w.on > 0) && w.on <= minute && w.off.is_none_or(|o| o >= minute))
}

fn shot_xg(e: &OfmEvent) -> f32 {
    use ofm_engine::event::{DangerBand, EventDetail};
    match e.event_type {
        EventType::PenaltyGoal | EventType::PenaltyMiss => 0.76,
        _ => match &e.detail {
            Some(EventDetail::Shot { danger }) => match danger {
                DangerBand::Speculative => 0.04,
                DangerBand::Decent => 0.12,
                DangerBand::BigChance => 0.38,
            },
            _ => match e.zone {
                Zone::HomeBox | Zone::AwayBox => 0.14,
                _ => 0.04,
            },
        },
    }
}

/// OFM's five-band pitch mapped onto our 6×5 grid in the acting side's frame.
fn zone_index(z: Zone, side: Side) -> usize {
    let row = match (z, side) {
        (Zone::AwayBox, Side::Home) | (Zone::HomeBox, Side::Away) => 5,
        (Zone::AwayDefense, Side::Home) | (Zone::HomeDefense, Side::Away) => 4,
        (Zone::Midfield, _) => 3,
        (Zone::HomeDefense, Side::Home) | (Zone::AwayDefense, Side::Away) => 1,
        (Zone::HomeBox, Side::Home) | (Zone::AwayBox, Side::Away) => 0,
    };
    zone_id(row, 2).min(N_ZONES - 1)
}

/// Second legs level on aggregate after 90': away goals (if used), then a shootout.
fn settle_aggregate(inp: &MatchInput, hg: u8, ag: u8) -> Option<(u8, u8)> {
    let (fh, fa) = inp.first_leg?;
    let (th, ta) = (u16::from(hg) + u16::from(fh), u16::from(ag) + u16::from(fa));
    if th != ta || (inp.away_goals_rule && ag != fh) {
        return None;
    }
    let taker = |t: &TeamSheet| -> f32 {
        let mut v: Vec<f32> = t.xi.iter().map(|p| p.attrs.get(Attr::PenaltyTaking) + p.attrs.get(Attr::Composure) * 0.5).collect();
        v.sort_by(|a, b| b.total_cmp(a));
        v.iter().take(5).sum::<f32>() / 5.0
    };
    let mut rng = pw_core::Rng::keyed(&[inp.seed, 0x5007]);
    let (mut h, mut a) = (0u8, 0u8);
    let p = |s: f32| (0.66 + 0.008 * (s - 15.0)).clamp(0.55, 0.85);
    for round in 0..30 {
        h += u8::from(rng.chance(p(taker(&inp.home))));
        a += u8::from(rng.chance(p(taker(&inp.away))));
        if round >= 4 && h != a {
            break;
        }
    }
    if h == a {
        h += 1;
    }
    Some((h, a))
}

/// Pathway rating from the player's line (OFM leaves ratings to its caller).
fn rating(l: &PlayerLine, conceded: u8, result: f32) -> f32 {
    let g = l.pos.map(Pos::group);
    let base = match g {
        Some(PosGroup::Gk) => 6.4,
        Some(PosGroup::Def) => 6.35,
        Some(PosGroup::Mid) => 6.3,
        _ => 6.2,
    };
    let completion = if l.passes > 0 { f32::from(l.passes_completed) / f32::from(l.passes) } else { 0.8 };
    let mut v = f32::from(l.goals) * 1.05 + f32::from(l.assists) * 0.7 + f32::from(l.on_target) * 0.12 - f32::from(l.shots.saturating_sub(l.on_target)) * 0.04
        + f32::from(l.tackles_won) * 0.09
        + f32::from(l.interceptions) * 0.08
        + f32::from(l.clearances) * 0.04
        + f32::from(l.blocks) * 0.08
        + f32::from(l.dribbles_won) * 0.06
        + (completion - 0.78) * 0.8 * (f32::from(l.passes).min(40.0) / 40.0)
        - f32::from(l.fouls) * 0.05
        - f32::from(l.yellows) * 0.25
        - f32::from(l.reds) * 1.4;
    match g {
        Some(PosGroup::Gk) => {
            v += f32::from(l.saves) * 0.22 - f32::from(conceded) * 0.32 + if conceded == 0 && l.minutes >= 60 { 0.6 } else { 0.0 };
        }
        Some(PosGroup::Def) => {
            v += -f32::from(conceded) * 0.14 + if conceded == 0 && l.minutes >= 60 { 0.35 } else { 0.0 };
        }
        _ => v -= f32::from(conceded) * 0.03,
    }
    let minutes_weight = (f32::from(l.minutes.max(15)) / 90.0).sqrt();
    let r = base + v * (1.0 / minutes_weight).min(1.6) + 0.25 * result;
    ((r * 10.0).round() / 10.0).clamp(3.0, 10.0)
}
