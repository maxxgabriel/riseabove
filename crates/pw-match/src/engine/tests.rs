use pw_core::{Attrs, HiddenAttrs, N_POS, PlayerTraits, TeamId};
use pw_data::DataPack;

use super::*;

fn sheet(id: u32, pos: Pos, level: f32) -> PlayerSheet {
    let mut attrs = Attrs::splat(100);
    for a in Attr::ALL {
        let v = if a.is_goalkeeping() { if pos == Pos::GK { level } else { 3.0 } } else if pos == Pos::GK { level * 0.6 } else { level };
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
    let xi = std::array::from_fn(|i| sheet(base + i as u32, f.slots[i].pos, level));
    let bench = (0..9).map(|i| sheet(base + 20 + i, if i == 0 { Pos::GK } else { f.slots[1 + i as usize % 10].pos }, level - 1.0)).collect();
    TeamSheet { team: TeamId(base), tactics: Tactics::default(), slots: f.slots, xi, bench, manager_reactivity: 0.5 }
}

fn run(n: u64, home: f32, away: f32) -> (f32, f32, f32, f32) {
    let pack = DataPack::builtin();
    let (mut goals, mut home_wins, mut draws, mut shots) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..n {
        let inp = MatchInput {
            seed,
            home: team(0, home, &pack),
            away: team(100, away, &pack),
            neutral: false,
            decisive: false,
            first_leg: None,
            away_goals_rule: false,
            importance: 0.5,
            referee_strictness: 1.0,
            max_subs: 5,
            lod: Lod::Standard,
            tuning: &pack.tuning.matches,
        };
        let r = simulate(&inp);
        goals += u32::from(r.home_goals + r.away_goals);
        shots += u32::from(r.stats[0].shots + r.stats[1].shots);
        match r.home_goals.cmp(&r.away_goals) {
            std::cmp::Ordering::Greater => home_wins += 1,
            std::cmp::Ordering::Equal => draws += 1,
            _ => {}
        }
    }
    let n = n as f32;
    (goals as f32 / n, home_wins as f32 / n, draws as f32 / n, shots as f32 / n)
}

#[test]
fn deterministic() {
    let pack = DataPack::builtin();
    let mk = || MatchInput {
        seed: 99,
        home: team(0, 12.0, &pack),
        away: team(100, 12.0, &pack),
        neutral: false,
        decisive: true,
        first_leg: None,
        away_goals_rule: false,
        importance: 0.5,
        referee_strictness: 1.0,
        max_subs: 5,
        lod: Lod::Full,
        tuning: &pack.tuning.matches,
    };
    let a = simulate(&mk());
    let b = simulate(&mk());
    assert_eq!((a.home_goals, a.away_goals, a.events.len()), (b.home_goals, b.away_goals, b.events.len()));
    assert!(a.lines.iter().filter(|l| l.minutes > 0).count() >= 22);
    assert!(a.winner().is_some(), "decisive match must produce a winner");
}

#[test]
fn stronger_team_wins_more() {
    let (_, even_home, _, _) = run(300, 12.0, 12.0);
    let (_, strong_home, _, _) = run(300, 15.0, 10.0);
    assert!(strong_home > even_home + 0.15, "even {even_home} strong {strong_home}");
}

#[test]
#[ignore = "calibration report; run with --ignored --nocapture"]
fn calibration_report() {
    let pack = DataPack::builtin();
    let n = 1000u64;
    let mut acc = [0f64; 16];
    let t0 = std::time::Instant::now();
    let (mut hw, mut dr) = (0, 0);
    for seed in 0..n {
        let inp = MatchInput {
            seed,
            home: team(0, 12.0, &pack),
            away: team(100, 12.0, &pack),
            neutral: false,
            decisive: false,
            first_leg: None,
            away_goals_rule: false,
            importance: 0.5,
            referee_strictness: 1.0,
            max_subs: 5,
            lod: Lod::Full,
            tuning: &pack.tuning.matches,
        };
        let r = simulate(&inp);
        let st = |f: fn(&TeamStats) -> f64| f(&r.stats[0]) + f(&r.stats[1]);
        acc[0] += f64::from(r.home_goals + r.away_goals);
        acc[1] += st(|s| f64::from(s.shots));
        acc[2] += st(|s| f64::from(s.on_target));
        acc[3] += st(|s| f64::from(s.xg));
        acc[4] += st(|s| f64::from(s.passes));
        acc[5] += st(|s| f64::from(s.passes_completed));
        acc[6] += st(|s| f64::from(s.fouls));
        acc[7] += st(|s| f64::from(s.yellows));
        acc[8] += st(|s| f64::from(s.reds));
        acc[9] += st(|s| f64::from(s.corners));
        acc[10] += st(|s| f64::from(s.offsides));
        acc[11] += r.events.iter().filter(|e| e.kind == Ev::Chain).count() as f64;
        let rated: Vec<f32> = r.lines.iter().filter(|l| l.minutes > 0).map(|l| l.rating).collect();
        acc[12] += f64::from(rated.iter().sum::<f32>()) / rated.len() as f64;
        acc[13] += r.lines.iter().filter(|l| l.injured).count() as f64;
        acc[14] += r.events.iter().filter(|e| e.kind == Ev::Sub).count() as f64;
        acc[15] += f64::from(r.events.last().map_or(0, |e| e.t)) / 60.0;
        match r.home_goals.cmp(&r.away_goals) {
            std::cmp::Ordering::Greater => hw += 1,
            std::cmp::Ordering::Equal => dr += 1,
            _ => {}
        }
    }
    let el = t0.elapsed();
    let a: Vec<f64> = acc.iter().map(|v| v / n as f64).collect();
    println!(
        "goals {:.2} shots {:.1} sot {:.1} xg {:.2} passes {:.0} pass% {:.1} fouls {:.1} yel {:.2} red {:.2} corners {:.1} offs {:.1} chains {:.0} rating {:.2} inj {:.2} subs {:.1} len {:.0}m",
        a[0], a[1], a[2], a[3], a[4], a[5] / a[4] * 100.0, a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15]
    );
    println!("home {:.3} draw {:.3}  {:.2} ms/match (Full LOD)", hw as f64 / n as f64, dr as f64 / n as f64, el.as_secs_f64() * 1000.0 / n as f64);
}

#[test]
#[ignore = "diagnostic; run with --ignored --nocapture"]
fn diag_zones() {
    let pack = DataPack::builtin();
    let n = 300u64;
    let mut shot_rows = [0u32; 6];
    let mut chain_rows = [0u32; 6];
    let (mut dribbles, mut dribbles_won, mut crosses, mut through) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..n {
        let inp = MatchInput {
            seed,
            home: team(0, 12.0, &pack),
            away: team(100, 12.0, &pack),
            neutral: false,
            decisive: false,
            first_leg: None,
            away_goals_rule: false,
            importance: 0.5,
            referee_strictness: 1.0,
            max_subs: 5,
            lod: Lod::Full,
            tuning: &pack.tuning.matches,
        };
        let r = simulate(&inp);
        for e in &r.events {
            let row = e.zone as usize / 5;
            match e.kind {
                Ev::Goal | Ev::ShotSaved | Ev::ShotWide | Ev::ShotBlocked | Ev::ShotPost => shot_rows[row] += 1,
                Ev::Chain => chain_rows[row] += 1,
                Ev::ThroughBall => through += 1,
                _ => {}
            }
        }
        for l in &r.lines {
            dribbles += u32::from(l.dribbles);
            dribbles_won += u32::from(l.dribbles_won);
            crosses += u32::from(l.crosses);
        }
    }
    let f = |v: u32| v as f64 / n as f64;
    println!("shots by row {:?}", shot_rows.map(f));
    println!("chains by row {:?}", chain_rows.map(f));
    println!("dribbles {:.1} won {:.1} crosses {:.1} through_ok {:.1}", f(dribbles), f(dribbles_won), f(crosses), f(through));
    let g = |a: &[std::sync::atomic::AtomicU64]| a.iter().map(|v| v.load(std::sync::atomic::Ordering::Relaxed) as f64 / n as f64).map(|v| (v * 10.0).round() / 10.0).collect::<Vec<_>>();
    println!("row5 arrivals [short medium long through cross carry-free carry-won restart] {:?}", g(&super::diag::ROW5));
    println!("pass tried {:?}", g(&super::diag::TRIED));
    println!("pass done  {:?}", g(&super::diag::DONE));
}
