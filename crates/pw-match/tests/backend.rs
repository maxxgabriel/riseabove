//! Tests through the public `simulate` with the default (data-pack) backend.

use pw_core::{Attr, Attrs, HiddenAttrs, N_POS, PlayerId, PlayerTraits, Pos, Tactics, TeamId};
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

fn input<'a>(seed: u64, home: f32, away: f32, pack: &DataPack, tuning: &'a MatchTuning, decisive: bool) -> MatchInput<'a> {
    MatchInput {
        seed,
        home: team(0, home, pack),
        away: team(100, away, pack),
        neutral: false,
        decisive,
        first_leg: None,
        away_goals_rule: false,
        importance: 0.5,
        referee_strictness: 1.0,
        max_subs: 5,
        lod: Lod::Full,
        tuning,
    }
}

#[test]
fn deterministic_and_complete() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let a = simulate(&input(7, 12.0, 12.0, &pack, t, true));
    let b = simulate(&input(7, 12.0, 12.0, &pack, t, true));
    assert_eq!((a.home_goals, a.away_goals, a.events.len(), a.pens), (b.home_goals, b.away_goals, b.events.len(), b.pens));
    assert!(a.winner().is_some());
    assert!(a.lines.iter().filter(|l| l.minutes > 0).count() >= 22);
    assert!(a.lines.iter().all(|l| l.minutes == 0 || (3.0..=10.0).contains(&l.rating)));
}

#[test]
fn stronger_side_wins_more() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let wins = |h: f32, a: f32| {
        (0..300)
            .filter(|&s| {
                let r = simulate(&input(s, h, a, &pack, t, false));
                r.home_goals > r.away_goals
            })
            .count()
    };
    let (even, strong) = (wins(12.0, 12.0), wins(15.0, 10.0));
    assert!(strong > even + 45, "even {even} strong {strong}");
}

#[test]
#[ignore = "calibration report; run with --ignored --nocapture"]
fn calibration_report() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let n = 2000u64;
    let mut acc = [0f64; 10];
    let (mut hw, mut dr) = (0, 0);
    let t0 = std::time::Instant::now();
    for s in 0..n {
        let r = simulate(&input(s, 12.0, 12.0, &pack, t, false));
        let sum = |f: fn(&TeamStats) -> f64| f(&r.stats[0]) + f(&r.stats[1]);
        acc[0] += f64::from(r.home_goals + r.away_goals);
        acc[1] += sum(|s| f64::from(s.shots));
        acc[2] += sum(|s| f64::from(s.on_target));
        acc[3] += sum(|s| f64::from(s.fouls));
        acc[4] += sum(|s| f64::from(s.yellows));
        acc[5] += sum(|s| f64::from(s.reds));
        acc[6] += sum(|s| f64::from(s.corners));
        let rated: Vec<f32> = r.lines.iter().filter(|l| l.minutes > 0).map(|l| l.rating).collect();
        acc[7] += f64::from(rated.iter().sum::<f32>()) / rated.len() as f64;
        acc[8] += r.events.iter().filter(|e| e.kind == Ev::Sub).count() as f64;
        acc[9] += sum(|s| f64::from(s.xg));
        match r.home_goals.cmp(&r.away_goals) {
            std::cmp::Ordering::Greater => hw += 1,
            std::cmp::Ordering::Equal => dr += 1,
            _ => {}
        }
    }
    let a: Vec<f64> = acc.iter().map(|v| v / n as f64).collect();
    println!("goals {:.2} shots {:.1} sot {:.1} fouls {:.1} yel {:.2} red {:.2} corners {:.1} rating {:.2} subs {:.1} xg {:.2}", a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9]);
    println!("home {:.3} draw {:.3}  {:.3} ms/match", hw as f64 / n as f64, dr as f64 / n as f64, t0.elapsed().as_secs_f64() * 1000.0 / n as f64);
}
