//! Reputation (11 §5, 17 §8): players drift toward what their ability and
//! stage earn; clubs move with league finishes.

use pw_core::NationId;
use pw_world::comp::sort_table;
use pw_world::{PlayerStatus, TeamKind, World};
use rayon::prelude::*;

pub fn weekly(w: &mut World) {
    let league_rep: Vec<f32> = w
        .teams
        .iter()
        .map(|t| {
            let club = &w.clubs[t.club];
            let base = if club.league.is_some() { f32::from(w.comps[club.league].reputation) } else { 1500.0 };
            if t.kind == TeamKind::First { base } else { base * 0.25 }
        })
        .collect();
    let hot: &[pw_world::PlayerHot] = &w.players.hot;
    w.players.cold.par_iter_mut().enumerate().for_each(|(i, c)| {
        let h = &hot[i];
        if h.status == PlayerStatus::Retired {
            return;
        }
        let stage = if h.team.is_some() { league_rep[h.team.0 as usize] / 10_000.0 } else { 0.1 };
        let play = 0.6 + 0.4 * (f32::from(h.minutes_4w) / 270.0).min(1.0);
        let target = 10_000.0 * (f32::from(c.ca) / 200.0).powf(1.6) * (0.35 + 0.65 * stage) * play;
        let step = |cur: u16, tgt: f32, a: f32| -> u16 { (f32::from(cur) + a * (tgt - f32::from(cur))).clamp(0.0, 10_000.0) as u16 };
        c.rep.current = step(c.rep.current, target, 0.03);
        c.rep.home = step(c.rep.home, target * 1.1, 0.03).max(c.rep.current);
        c.rep.world = step(c.rep.world, target * stage.sqrt(), 0.02);
    });
}

pub fn season_end(w: &mut World, n: NationId) {
    for &league in &w.nations[n].leagues.clone() {
        let mut rows = w.comps[league].state.table.clone();
        sort_table(&mut rows);
        let size = rows.len().max(1) as f32;
        let tier_rep = f32::from(w.comps[league].reputation);
        for (i, r) in rows.iter().enumerate() {
            let club = w.teams[r.team].club;
            let rel = 1.0 - i as f32 / (size - 1.0).max(1.0);
            let c = &mut w.clubs[club];
            // Pull toward the league's standing, pushed by finishing position.
            let target = tier_rep * (0.55 + 0.6 * rel);
            let rep = f32::from(c.reputation);
            c.reputation = (rep + 0.15 * (target - rep) + (rel - 0.5) * 150.0).clamp(100.0, 10_000.0) as u16;
        }
    }
}
