//! The coach seam of the match engine (locked design 7.1, 7.35-7.36): staff are given football evidence and never parameters; a call
//! reaches the game as instructions, roles and substitutions; a player's state changes how he plays without a flat bonus; and a match
//! with nobody coaching is exactly the old match.

use pw_core::{Attr, Attrs, HiddenAttrs, Mentality, N_POS, PlayerId, PlayerTraits, Pos, Tactics, TeamId};
use pw_data::{DataPack, MatchTuning};
use pw_match::*;

fn sheet(id: u32, pos: Pos, level: f32) -> PlayerSheet {
    let mut attrs = Attrs::splat(100);
    for a in Attr::ALL {
        let v = match (a.is_goalkeeping(), pos == Pos::GK) {
            (true, true) => level,
            (true, false) => 3.0,
            (false, true) => level * 0.6,
            (false, false) => level,
        };
        attrs.set(a, v);
    }
    let mut familiarity = [1u8; N_POS];
    familiarity[pos.idx()] = 20;
    PlayerSheet {
        id: PlayerId(id),
        attrs,
        hidden: HiddenAttrs::default(),
        traits: PlayerTraits::empty(),
        familiarity,
        left_foot: 8,
        right_foot: 18,
        height: 182,
        condition: 100.0,
        sharpness: 90.0,
        morale: 70.0,
        injury_risk: 1.0,
    }
}

fn team(base: u32, level: f32, pack: &DataPack) -> TeamSheet {
    let f = &pack.formations[0];
    TeamSheet {
        team: TeamId(base),
        tactics: Tactics::default(),
        slots: f.slots,
        xi: std::array::from_fn(|i| sheet(base + i as u32, f.slots[i].pos, level)),
        bench: (0..9).map(|i| sheet(base + 20 + i, if i == 0 { Pos::GK } else { f.slots[1 + i as usize % 10].pos }, level - 1.0)).collect(),
        manager_reactivity: 0.5,
    }
}

fn input<'a>(seed: u64, pack: &DataPack, tuning: &'a MatchTuning) -> MatchInput<'a> {
    MatchInput {
        seed,
        home: team(0, 12.0, pack),
        away: team(100, 12.0, pack),
        neutral: false,
        decisive: false,
        first_leg: None,
        away_goals_rule: false,
        importance: 0.5,
        referee_strictness: 1.0,
        max_subs: 5,
        lod: Lod::Standard,
        tuning,
    }
}

fn signature(r: &MatchResult) -> (u8, u8, usize, Vec<u8>) {
    (r.home_goals, r.away_goals, r.events.len(), r.lines.iter().map(|l| l.minutes).collect())
}

#[test]
fn a_match_with_nobody_coaching_is_the_match_the_engine_always_played() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    for seed in 0..12 {
        let plain = simulate(&input(seed, &pack, t));
        let with = simulate_with(&input(seed, &pack, t), &mut Nobody);
        assert_eq!(signature(&plain), signature(&with), "seed {seed}");
    }
}

/// Records what it is shown and answers with a fixed call at its first window.
#[derive(Default)]
struct Watcher {
    looks: Vec<Look>,
    answer: Option<Call>,
    applied: Vec<(u8, Vec<bool>)>,
    end: Option<Look>,
    finished: bool,
}

impl Coach for Watcher {
    fn call(&mut self, look: &Look) -> [Call; 2] {
        self.looks.push(look.clone());
        if self.looks.len() == 1 { [self.answer.clone().unwrap_or_default(), Call::default()] } else { [Call::default(), Call::default()] }
    }
    fn applied(&mut self, side: u8, _call: &Call, made: &[bool]) {
        self.applied.push((side, made.to_vec()));
    }
    fn full_time(&mut self, look: &Look) {
        self.end = Some(look.clone());
    }
    fn finished(&mut self, _r: &MatchResult) {
        self.finished = true;
    }
}

#[test]
fn staff_are_shown_football_evidence_at_the_windows_and_at_half_time_and_full_time() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let mut w = Watcher::default();
    let r = simulate_with(&input(3, &pack, t), &mut w);
    let minutes: Vec<(u8, bool)> = w.looks.iter().map(|l| (l.minute, l.half_time)).collect();
    assert!(minutes.iter().any(|&(_, h)| h), "half-time is a window: {minutes:?}");
    assert!(minutes.iter().any(|&(m, h)| !h && m == 30) && minutes.iter().any(|&(m, h)| !h && m == 60), "and so are the half-hour and the hour: {minutes:?}");
    assert!(w.finished && w.end.is_some(), "the match ends with a last look and a finish");
    // The evidence is cumulative and about football: no numbers of the engine's own.
    for pair in w.looks.windows(2) {
        assert!(pair[1].sides[0].shots >= pair[0].sides[0].shots && pair[1].sides[1].territory >= pair[0].sides[1].territory);
    }
    let last = w.end.as_ref().unwrap();
    let shots: u16 = last.sides[0].shots + last.sides[1].shots;
    assert!(shots > 0 && last.goals == [r.home_goals, r.away_goals]);
    // What is visible of a player: where he plays, how fresh he looks, whether he is booked, what he has done.
    let p = &last.sides[0].players[3];
    assert!(p.condition > 0 && p.condition <= 100);
    assert_eq!(last.sides[0].players.len(), 11);
}

#[test]
fn a_call_reaches_the_game_as_a_substitution_and_a_change_of_approach() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let home = team(0, 12.0, &pack);
    // Take a tired-looking outfielder off for the first bench outfielder, and go for it.
    let out = home.xi[5].id;
    let inn = home.bench[1].id;
    let mut attack = Tactics::default();
    attack.mentality = Mentality::Attacking;
    attack.press = 80;
    let call = Call { tactics: Some(attack), roles: Default::default(), subs: [SubCall { out, inn }].into_iter().collect() };
    let mut w = Watcher { answer: Some(call), ..Default::default() };
    let r = simulate_with(&input(5, &pack, t), &mut w);
    assert_eq!(w.applied, vec![(0, vec![true])], "the substitution was made");
    let line_out = r.line(out).unwrap();
    let line_in = r.line(inn).unwrap();
    assert!(line_out.minutes > 0 && line_out.off_at > 0, "he came off");
    assert!(line_in.minutes > 0 && line_in.on_at > 0, "and the other came on");
    // The next look sees the new instructions the side is playing to.
    assert_eq!(w.looks.get(1).map(|l| l.sides[0].tactics.press), Some(80));
    assert!(w.looks[1].sides[0].players.iter().any(|p| p.player == inn) && !w.looks[1].sides[0].players.iter().any(|p| p.player == out));
}

#[test]
fn a_substitution_the_engine_cannot_make_is_reported_as_not_made() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let call = Call { tactics: None, roles: Default::default(), subs: [SubCall { out: PlayerId(4242), inn: PlayerId(4243) }].into_iter().collect() };
    let mut w = Watcher { answer: Some(call), ..Default::default() };
    simulate_with(&input(6, &pack, t), &mut w);
    assert_eq!(w.applied, vec![(0, vec![false])]);
}

struct Minded(Mind);

impl Coach for Minded {
    fn mind(&self, side: u8, _p: PlayerId) -> Mind {
        if side == 0 { self.0 } else { Mind::NEUTRAL }
    }
}

fn mean_goal_difference(mind: Mind, n: u64) -> f32 {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let mut d = 0i32;
    for seed in 0..n {
        let r = simulate_with(&input(seed, &pack, t), &mut Minded(mind));
        d += i32::from(r.home_goals) - i32::from(r.away_goals);
    }
    d as f32 / n as f32
}

#[test]
fn a_players_state_changes_how_he_plays_without_being_a_flat_bonus() {
    // Over many matches between equal sides: a side that walks out unfocused and short-tempered does worse than the same side calm and
    // attentive, and both differ from the ordinary day.
    let bad = mean_goal_difference(Mind { focus: -1.0, calm: -1.0, confidence: -1.0, ..Mind::NEUTRAL }, 500);
    let good = mean_goal_difference(Mind { focus: 1.0, calm: 1.0, confidence: 1.0, ..Mind::NEUTRAL }, 500);
    assert!(good > bad + 0.15, "an attentive, calm, confident side ({good}) beats one that is none of those ({bad})");
    // Behaviour, not a stat line: mind can also make a player worse without lowering his skill. The engine reads attention, composure,
    // work rate and flair; a state that touches none of them does nothing.
    let inert = mean_goal_difference(Mind::NEUTRAL, 500);
    assert!(bad < inert && inert < good + 0.05, "{bad} < {inert}");
}

#[test]
fn a_neutral_mind_changes_nothing_and_a_mind_stays_inside_its_range() {
    let m = Mind { focus: 4.0, calm: -9.0, risk: 0.5, drive: 0.0, confidence: 1.0 }.clamped();
    assert_eq!((m.focus, m.calm, m.risk), (1.0, -1.0, 0.5));
    assert!(Mind::NEUTRAL.is_neutral() && !m.is_neutral());
}
