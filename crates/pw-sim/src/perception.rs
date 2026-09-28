//! Evidence gathering (03 §11): coaches watch their own squads in training,
//! scouts sample players across nations, stale evidence is forgotten. Club
//! estimates come from `pw_world::knowledge`.

use pw_core::{ClubId, PlayerId};
use pw_world::World;
use pw_world::knowledge::{Observer, perceived_ca, perceived_pa, sigma};

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

    // Everything beyond a club's own squad now comes through its scouting
    // network, analysts, agents, recommendations and matches it plays in.
    crate::scouting::weekly(w);
}

pub fn monthly(w: &mut World) {
    w.knowledge.forget(w.date.add_days(-540), 360);
}
