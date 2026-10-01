//! Development beyond the weekly attribute step (04): the people and
//! circumstances that shape a player.
//!
//! - Mentoring: experienced professionals take young players under their
//!   wing; character and habits rub off (both ways — a bad influence is an
//!   influence too).
//! - Character drift: hidden personality moves slowly with mentors, peers,
//!   the manager's man-management and hard experiences.
//! - Stagnation: a young player who goes months without football loses some
//!   of what they could have become.
//! - Traits: preferred moves are learned from mentors and adopted as bodies
//!   age.
//! - Trajectory: monthly ability snapshots others read (coaches, media lists).

use pw_core::rng::{Rng, stream};
use pw_core::{Attr, Hidden, PersonId, PlayerId, PlayerTraits, PosGroup, StaffAttr};
use pw_world::event::{EventKind, Visibility};
use pw_world::{MemoryKind, PlayerStatus, TeamKind, World};

use crate::consider;

pub fn monthly(w: &mut World) {
    snapshots(w);
    mentoring(w);
    drift(w);
    stagnation(w);
    traits(w);
}

fn snapshots(w: &mut World) {
    let today = w.date;
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status != PlayerStatus::Retired && w.age(p) <= 26).collect();
    for p in ids {
        let ca = w.players.cold[p].ca;
        let r = w.growth.records.entry(p).or_default();
        r.ca.push((today, ca));
        if r.ca.len() > 12 {
            r.ca.remove(0);
        }
    }
    // Forget records of players who have moved past the age anyone tracks.
    let old: Vec<PlayerId> = w.growth.records.keys().copied().filter(|&p| w.age(p) > 27 || w.players.hot[p].status == PlayerStatus::Retired).collect();
    for p in old {
        if let Some(r) = w.growth.records.remove(&p)
            && r.mentor.is_some()
        {
            let me = w.players.cold[p].person;
            if let Some(v) = w.growth.mentees.get_mut(&r.mentor) {
                v.retain(|x| *x != me);
            }
        }
    }
}

/// Young players find (or are found by) an experienced teammate.
fn mentoring(w: &mut World) {
    let today = w.date;
    for t in w.teams.ids() {
        let kind = w.teams[t].kind;
        if !matches!(kind, TeamKind::First | TeamKind::Reserve | TeamKind::U21 | TeamKind::U19 | TeamKind::U18) {
            continue;
        }
        let club = w.teams[t].club;
        let squad: Vec<PlayerId> = w.teams[t].squad.iter().copied().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect();
        // Seniors at the club (first team) can mentor youngsters in any of its sides.
        let first = w.clubs[club].teams.iter().copied().find(|&x| w.teams[x].kind == TeamKind::First);
        let seniors: Vec<PlayerId> =
            first.map(|f| w.teams[f].squad.clone()).unwrap_or_default().into_iter().filter(|&p| w.players.hot[p].status == PlayerStatus::Active && w.age_years(p) >= 27.0).collect();
        for &y in &squad {
            if w.age_years(y) > 21.0 {
                continue;
            }
            let ym = w.players.cold[y].person;
            let current = w.growth.records.get(&y).map(|r| r.mentor).filter(|m| m.is_some());
            // A mentor who has left the club no longer counts.
            if let Some(m) = current {
                let still = w.people[m].player.get().is_some_and(|mp| w.players.hot[mp].club == club);
                if still {
                    continue;
                }
                end_mentoring(w, m, ym);
            }
            let mut rng = Rng::keyed(&[w.seed, stream::DEVELOPMENT, u64::from(y.0), today.0 as u64, 0x3e7]);
            if !rng.chance(0.25) {
                continue;
            }
            let yg = w.players.cold[y].best_pos.group();
            let best = seniors
                .iter()
                .copied()
                .filter(|&s| w.growth.mentees.get(&w.players.cold[s].person).is_none_or(|v| v.len() < 3))
                .map(|s| {
                    let sm = w.players.cold[s].person;
                    let c = &w.players.cold[s];
                    let prof = consider::hid(w, sm, Hidden::Professionalism);
                    let lead = c.attrs.get(Attr::Leadership);
                    let same_group = c.best_pos.group() == yg;
                    let shared = w.people[sm].nation == w.people[ym].nation;
                    let compat = f32::from(consider::compat(w, sm, ym)) / 100.0;
                    let aff = consider::affinity(w, sm, ym);
                    let score = lead / 20.0 * 0.6 + prof / 20.0 * 0.3 + if same_group { 0.3 } else { 0.0 } + if shared { 0.2 } else { 0.0 } + compat + aff + rng.normal() * 0.1;
                    (s, score)
                })
                .filter(|&(s, score)| score > 0.8 && w.players.cold[s].attrs.get(Attr::Leadership) >= 11.0)
                .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
            if let Some((s, _)) = best {
                begin_mentoring(w, w.players.cold[s].person, ym, y);
            }
        }
    }
}

/// Start a mentoring bond (also reachable through a human's intent).
pub fn begin_mentoring(w: &mut World, mentor: PersonId, mentee: PersonId, mentee_player: PlayerId) {
    let today = w.date;
    let r = w.growth.records.entry(mentee_player).or_default();
    if r.mentor == mentor {
        return;
    }
    r.mentor = mentor;
    r.mentor_since = today;
    w.growth.mentees.entry(mentor).or_default().push(mentee);
    let club = w.players.hot[mentee_player].club;
    let vis = if club.is_some() { Visibility::Club(club) } else { Visibility::Public };
    let ev = w.events.push(today, vis, EventKind::TookUnderWing { mentor, mentee });
    let compat = consider::compat(w, mentee, mentor);
    w.social.remember(mentee, mentor, MemoryKind::Mentored, today, ev, false, 0.9, compat);
    let compat = consider::compat(w, mentor, mentee);
    w.social.adjust(mentor, mentee, today, compat, 4, 3, 0);
}

fn end_mentoring(w: &mut World, mentor: PersonId, mentee: PersonId) {
    if let Some(v) = w.growth.mentees.get_mut(&mentor) {
        v.retain(|x| *x != mentee);
    }
    if let Some(p) = w.people[mentee].player.get()
        && let Some(r) = w.growth.records.get_mut(&p)
    {
        r.mentor = PersonId::NONE;
    }
}

/// A human asks to take someone under their wing; the younger player decides
/// with their own view of the mentor.
pub fn offer_mentoring(w: &mut World, mentor: PersonId, mentee: PersonId) {
    let Some(mp) = w.people[mentee].player.get() else { return };
    let Some(sp) = w.people[mentor].player.get() else { return };
    if w.players.hot[mp].club != w.players.hot[sp].club || w.age_years(mp) > 23.0 {
        return;
    }
    let willing = consider::trust(w, mentee, mentor) + consider::affinity(w, mentee, mentor) > 0.45;
    if willing {
        begin_mentoring(w, mentor, mentee, mp);
    }
}

const DRIFTING: [Hidden; 5] = [Hidden::Professionalism, Hidden::Temperament, Hidden::Sportsmanship, Hidden::Pressure, Hidden::Ambition];

/// Personality moves slowly toward the people one spends time with.
fn drift(w: &mut World) {
    let today = w.date;
    // Sorted, as in `stagnation`: the events are recorded in visiting order, and the map's order differs after a reload.
    let mut ids: Vec<PlayerId> = w.growth.records.keys().copied().collect();
    ids.sort();
    for p in ids {
        if w.age_years(p) > 24.0 || w.players.hot[p].status != PlayerStatus::Active {
            continue;
        }
        let me = w.players.cold[p].person;
        let mut rng = Rng::keyed(&[w.seed, stream::DEVELOPMENT, u64::from(p.0), today.0 as u64, 0xd41f]);
        let mentor = w.growth.records[&p].mentor;
        let mut moved: i32 = 0;
        // The mentor's example.
        if mentor.is_some() {
            for h in DRIFTING {
                let mine = i32::from(w.people[me].hidden.get(h));
                let theirs = i32::from(w.people[mentor].hidden.get(h));
                if (theirs - mine).abs() >= 3 && rng.chance(0.12) {
                    let step = (theirs - mine).signum();
                    w.people[me].hidden.set(h, (mine + step) as u8);
                    moved += if h == Hidden::Professionalism { step } else { 0 };
                }
            }
            // Determination is visible; it moves too.
            let md = w.people[mentor].player.get().map(|mp| w.players.cold[mp].attrs.get(Attr::Determination));
            if let Some(md) = md {
                let mine = w.players.cold[p].attrs.get(Attr::Determination);
                if md - mine >= 3.0 && rng.chance(0.1) {
                    w.players.cold[p].attrs.add(Attr::Determination, 0.5);
                }
            }
        }
        // Bad company: a close teammate who lives for the night.
        let team = w.players.hot[p].team;
        if team.is_some() {
            let bad = w.teams[team].squad.iter().copied().filter(|&x| x != p).any(|x| {
                let xp = w.players.cold[x].person;
                w.lives.get(xp).is_some_and(|l| l.routine.nightlife >= 6) && consider::affinity(w, me, xp) > 0.35 && consider::hid(w, xp, Hidden::Professionalism) <= 8.0
            });
            if bad && rng.chance(0.08) {
                let v = w.people[me].hidden.get(Hidden::Professionalism);
                w.people[me].hidden.set(Hidden::Professionalism, v.saturating_sub(1));
                moved -= 1;
            }
        }
        // A manager good with people steadies temperaments.
        let club = w.players.hot[p].club;
        if club.is_some()
            && let Some(m) = w.clubs[club].manager.get()
            && w.staff[m].attrs.f(StaffAttr::ManManagement) >= 15.0
            && rng.chance(0.05)
        {
            let v = w.people[me].hidden.get(Hidden::Temperament);
            w.people[me].hidden.set(Hidden::Temperament, v + 1);
        }
        if moved != 0 {
            let r = w.growth.records.get_mut(&p).expect("record");
            r.drift = r.drift.saturating_add(moved.unsigned_abs() as u8);
            if r.drift >= 3 {
                r.drift = 0;
                // Character moves in many small steps with no single moment behind them; the one visible bond that shapes it, a mentor,
                // is the cause when there is one.
                let mentor = w.events.latest_where(today, 400, |e| matches!(e.kind, EventKind::TookUnderWing { mentee, .. } if mentee == me));
                let because = mentor.map_or_else(Default::default, |id| pw_world::causes![pw_world::event::Cause::Event(id)]);
                w.events.push_caused(today, Visibility::Public, EventKind::CharacterChanged { person: me, up: moved > 0 }, because);
            }
        }
    }
}

/// Months without football take something away from young players.
fn stagnation(w: &mut World) {
    let today = w.date;
    // Sorted: events are recorded as the players are visited, and the map holds its keys in an order that differs after a reload.
    let mut ids: Vec<PlayerId> = w.growth.records.keys().copied().collect();
    ids.sort();
    for p in ids {
        let age = w.age_years(p);
        let h = &w.players.hot[p];
        if !(17.0..=23.0).contains(&age) || h.status == PlayerStatus::Retired {
            continue;
        }
        let playing = h.minutes_4w >= 120 || h.injury != 0;
        let minutes = h.minutes_4w;
        let (ca, pa) = (w.players.cold[p].ca, w.players.cold[p].pa);
        let r = w.growth.records.get_mut(&p).expect("record");
        if playing {
            r.idle_weeks = r.idle_weeks.saturating_sub(8);
            continue;
        }
        r.idle_weeks = r.idle_weeks.saturating_add(4);
        if r.idle_weeks >= 26 && r.eroded < 12 && pa > ca.saturating_add(5) {
            r.eroded += 1;
            let first = r.eroded == 3;
            w.players.cold[p].pa = pa - 1;
            if first {
                // What the ceiling came down for: the minutes he did not get, read against the bar the rule itself uses (a third of a
                // month's football).
                let share_pct = (u32::from(minutes) * 100 / 360).min(100) as u8;
                let because = pw_world::causes![pw_world::event::Cause::Fact(pw_world::event::Fact::MinutesShortfall { player: p, share_pct, expected_pct: 33 })];
                w.events.push_caused(today, Visibility::Public, EventKind::Stagnated { player: p }, because);
            }
        }
    }
}

/// Preferred moves: learned from a mentor, or adopted as the body changes.
fn traits(w: &mut World) {
    let today = w.date;
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect();
    for p in ids {
        let mut rng = Rng::keyed(&[w.seed, stream::DEVELOPMENT, u64::from(p.0), today.0 as u64, 0x7a17]);
        let c = &w.players.cold[p];
        if c.traits.bits().count_ones() >= 6 {
            continue;
        }
        let age = w.age_years(p);
        let group = c.best_pos.group();
        // From a mentor in a similar role.
        let mentor = w.growth.records.get(&p).map(|r| r.mentor).filter(|m| m.is_some());
        if let Some(mp) = mentor.and_then(|m| w.people[m].player.get()) {
            let theirs = w.players.cold[mp].traits;
            let learnable = theirs - c.traits;
            if !learnable.is_empty() && w.players.cold[mp].best_pos.group() == group && rng.chance(0.05) {
                let options: Vec<PlayerTraits> = learnable.iter().collect();
                let t = options[rng.index(options.len())];
                w.players.cold[p].traits |= t;
                if let Some(r) = w.growth.records.get_mut(&p) {
                    r.last_trait = today;
                }
                continue;
            }
        }
        // Ageing attackers and midfielders adapt their game.
        if age >= 30.0 && group != PosGroup::Gk {
            let pace = c.attrs.get(Attr::Pace);
            if pace < 11.0 && !c.traits.contains(PlayerTraits::SIMPLE_PASSES) && rng.chance(0.03) {
                w.players.cold[p].traits |= PlayerTraits::SIMPLE_PASSES;
                w.players.cold[p].traits.remove(PlayerTraits::KNOCKS_BALL_PAST);
            } else if group == PosGroup::Att && pace < 11.0 && !c.traits.contains(PlayerTraits::COMES_DEEP) && rng.chance(0.03) {
                w.players.cold[p].traits |= PlayerTraits::COMES_DEEP;
            }
        }
    }
}

/// How a coach would describe the trajectory (for talks and media).
pub fn trajectory(w: &World, p: PlayerId) -> i16 {
    w.growth.records.get(&p).map_or(0, |r| r.trend())
}
