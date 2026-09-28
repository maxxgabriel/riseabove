//! Evidence gathering (03 §11): coaches watch their own squads in training,
//! scouts sample players across nations, stale evidence is forgotten. Club
//! estimates come from `pw_world::knowledge`.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, PlayerId};
use pw_world::knowledge::{Observer, perceived_ca, perceived_pa, sigma};
use pw_world::{PlayerStatus, StaffRole, World};

/// A club's view of a player: (ca estimate, ca band, pa estimate, pa band).
pub fn club_view(w: &World, club: ClubId, p: PlayerId) -> (f32, f32, f32, f32) {
    let c = &w.players.cold[p];
    let t = &w.data.tuning.perception;
    let (judge_a, judge_p) = w.club_manager_judging(club);
    let s = sigma(t, w.knowledge.seen(club, p), judge_a, w.date, c.rep.world >= t.famous_reputation);
    let (ca, ca_band) = perceived_ca(f32::from(c.ca), s, Observer::Club(club), p);
    let (pa, pa_band) = perceived_pa(f32::from(c.pa), ca, s, judge_p, Observer::Club(club), p);
    (ca, ca_band, pa, pa_band)
}

pub fn weekly(w: &mut World) {
    let today = w.date;
    // Coaches see their own players every week in training.
    for t in w.teams.ids() {
        let club = w.teams[t].club;
        for i in 0..w.teams[t].squad.len() {
            let p = w.teams[t].squad[i];
            w.knowledge.observe(club, p, 120, today);
        }
    }

    // Scouting pools: active and unattached players by the nation they play in.
    let mut by_nation: Vec<Vec<PlayerId>> = vec![Vec::new(); w.nations.len()];
    for p in w.players.ids() {
        let h = &w.players.hot[p];
        if h.status == PlayerStatus::Retired {
            continue;
        }
        let nation = if h.club.is_some() { w.clubs[h.club].nation } else { w.people[w.players.cold[p].person].nation };
        if nation.is_some() {
            by_nation[nation.0 as usize].push(p);
        }
    }
    let nation_weight: Vec<f32> = w.nations.iter().map(|n| (f32::from(n.reputation) / 1000.0).powi(2) + 0.2).collect();

    let week = (today.0 / 7) as u64;
    for club in w.clubs.ids() {
        let scouts = w.clubs[club].staff.iter().filter(|&&s| w.staff[s].role == StaffRole::Scout).count();
        let budget = 1 + (w.clubs[club].reputation / 2500) as usize;
        let n = (2 + 3 * scouts).min(4 + 3 * budget).min(16);
        let home = w.clubs[club].nation;
        let mut rng = Rng::keyed(&[w.seed, stream::PERCEPTION, u64::from(club.0), week]);
        for _ in 0..n {
            let nation = if rng.chance(0.65) || nation_weight.is_empty() { home.0 as usize } else { rng.weighted(&nation_weight) };
            let pool = &by_nation[nation];
            if pool.is_empty() {
                continue;
            }
            let p = pool[rng.index(pool.len())];
            w.knowledge.observe(club, p, 90, today);
        }
    }
}

pub fn monthly(w: &mut World) {
    w.knowledge.forget(w.date.add_days(-540), 360);
}
