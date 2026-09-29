//! Match injuries follow the body: risk and tiredness raise them, and they come
//! from both contact and non-contact causes (OFM alone only injures the fouled).

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


fn injuries(risk: f32, condition: f32, n: u64) -> (usize, usize) {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    let (mut all, mut noncontact) = (0, 0);
    for s in 0..n {
        let mut inp = input(s, 12.0, 12.0, &pack, t, false);
        for side in [&mut inp.home, &mut inp.away] {
            for p in side.xi.iter_mut().chain(side.bench.iter_mut()) {
                p.injury_risk = risk;
                p.condition = condition;
            }
        }
        let r = simulate(&inp);
        all += r.lines.iter().filter(|l| l.injured).count();
        noncontact += r.lines.iter().filter(|l| l.injury_noncontact).count();
    }
    (all, noncontact)
}

#[test]
fn injuries_rise_with_risk_and_tiredness() {
    let (robust, _) = injuries(0.4, 100.0, 400);
    let (typical, _) = injuries(1.5, 100.0, 400);
    let (fragile, nc) = injuries(3.0, 60.0, 400);
    assert!(robust < typical && typical < fragile, "robust {robust} typical {typical} fragile {fragile}");
    assert!(nc > 0, "no non-contact injuries at high risk");
}

/// Injuries per 1000 player-hours of match play at ordinary risk and condition, for calibration.
/// Professional football runs at roughly 8 per 1000 match-hours (time-loss injuries, all causes).
/// `cargo test --release -p pw-match --test injury rate -- --ignored --nocapture`
#[test]
#[ignore = "report"]
fn rate() {
    let pack = DataPack::builtin();
    let t = &pack.tuning.matches;
    for (risk, cond) in [(1.0f32, 90.0f32), (1.5, 85.0)] {
        for lod in [Lod::Full, Lod::Standard] {
            let (mut inj, mut nc, mut minutes) = (0usize, 0usize, 0.0f64);
            for s in 0..1500u64 {
                let mut inp = input(s, 12.0, 12.0, &pack, t, false);
                inp.lod = lod;
                for side in [&mut inp.home, &mut inp.away] {
                    for p in side.xi.iter_mut().chain(side.bench.iter_mut()) {
                        p.injury_risk = risk;
                        p.condition = cond;
                    }
                }
                let r = simulate(&inp);
                for l in &r.lines {
                    minutes += f64::from(l.minutes);
                    inj += usize::from(l.injured);
                    nc += usize::from(l.injury_noncontact);
                }
            }
            let hours = minutes / 60.0;
            eprintln!("risk {risk} cond {cond} {lod:?}: {:.1} injuries per 1000 h ({:.0}% non-contact), {inj} in {hours:.0} h", inj as f64 / hours * 1000.0, 100.0 * nc as f64 / inj.max(1) as f64);
        }
    }
}
