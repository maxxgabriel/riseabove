//! Evidence gathering (03 §11): coaches watch their own squads in training,
//! scouts sample players across nations, stale evidence is forgotten. Club
//! estimates come from `pw_world::knowledge`.

use pw_core::{ClubId, PlayerId};
use pw_world::World;
use pw_world::knowledge::{Observer, perceived_ca, perceived_pa, sigma};

/// A club's view of a player: (ca estimate, ca band, pa estimate, pa band).
pub fn club_view(w: &World, club: ClubId, p: PlayerId) -> (f32, f32, f32, f32) {
    club_view_judged(w, club, p, w.club_manager_judging(club))
}

/// [`club_view`] for a caller that looks at many players of one club and has worked out the club's judging once
/// (`World::club_manager_judging`, which scans the club's staff).
pub fn club_view_judged(w: &World, club: ClubId, p: PlayerId, judging: (f32, f32)) -> (f32, f32, f32, f32) {
    let c = &w.players.cold[p];
    let t = &w.data.tuning.perception;
    let (judge_a, judge_p) = judging;
    let s = sigma(t, w.knowledge.seen(club, p), judge_a, w.date, c.rep.world >= t.famous_reputation);
    let (ca, ca_band) = perceived_ca(f32::from(c.ca), s, Observer::Club(club), p);
    let (pa, pa_band) = perceived_pa(f32::from(c.pa), ca, s, judge_p, Observer::Club(club), p);
    (ca, ca_band, pa, pa_band)
}

/// Only the club's estimate of current ability (the first value of [`club_view`]), for many players of one club.
pub fn club_ca_judged(w: &World, club: ClubId, p: PlayerId, judging_ability: f32) -> f32 {
    let c = &w.players.cold[p];
    let t = &w.data.tuning.perception;
    let s = sigma(t, w.knowledge.seen(club, p), judging_ability, w.date, c.rep.world >= t.famous_reputation);
    perceived_ca(f32::from(c.ca), s, Observer::Club(club), p).0
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
