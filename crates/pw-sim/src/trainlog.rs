//! The training ground, week by week, around the people humans inhabit (`pw_world::trainlog`). On Mondays, after the week's
//! training has been judged, it writes down where the person trained, how their week went, which coach worked with them, and the
//! traces the week left: the manager's word, teammates flying or off the pace, extra work, fines, players back from injury,
//! arrivals. Read from what the world recorded that week; nothing decides from it.

use pw_core::{PersonId, PlayerId, Rng};
use pw_world::World;
use pw_world::event::{CoachNote, EventKind as E};
use pw_world::medical::ReturnStage;
use pw_world::social::MemoryKind;
use pw_world::staff::StaffRole;
use pw_world::trainlog::{Group, Trace, TrainingWeek, Week, KEEP};
use smallvec::SmallVec;

pub fn weekly(w: &mut World) {
    if w.ext.chronicle.lives.is_empty() {
        return;
    }
    let mut who: Vec<PersonId> = w.ext.chronicle.lives.keys().copied().collect();
    who.sort_unstable();
    for me in who {
        let p = w.people[me].player;
        if p.is_none() || w.players.hot[p].club.is_none() {
            continue;
        }
        let week = week_of(w, me, p);
        let v = w.ext.trainlog.of.entry(me).or_default();
        v.push(week);
        if v.len() > KEEP {
            v.remove(0);
        }
    }
}

fn person(w: &World, q: PlayerId) -> PersonId {
    w.players.cold.get(q).map_or(PersonId::NONE, |c| c.person)
}

fn week_of(w: &World, me: PersonId, p: PlayerId) -> TrainingWeek {
    let today = w.date;
    let since = today.add_days(-7);
    let h = &w.players.hot[p];
    let club = h.club;
    let squad: Vec<PlayerId> = w.clubs[club].teams.first().map(|&t| w.teams[t].squad.clone()).unwrap_or_default();
    let group = if h.injury == 0 {
        Group::FirstTeam
    } else if ReturnStage::of(f32::from(h.injury_days) / f32::from(h.injury_total.max(1))) >= ReturnStage::PartialTeam {
        Group::Partial
    } else {
        Group::Rehab
    };
    let life = &w.lives[me];
    let week = if life.train_high_weeks > 0 {
        Week::Sharp
    } else if life.train_low_weeks > 0 {
        Week::Flat
    } else {
        Week::Ordinary
    };
    // The coach who worked with you: the physio when you are inside, a goalkeeping coach for a keeper, otherwise one of the coaches.
    let want = match group {
        Group::Rehab => StaffRole::Physio,
        Group::Partial => StaffRole::FitnessCoach,
        Group::FirstTeam if w.players.cold[p].best_pos.group() == pw_core::PosGroup::Gk => StaffRole::GkCoach,
        Group::FirstTeam => StaffRole::Coach,
    };
    let staff: Vec<PersonId> = w.clubs[club].staff.iter().map(|&s| &w.staff[s]).filter(|s| s.role == want && s.employed()).map(|s| s.person).collect();
    let staff = if staff.is_empty() { w.clubs[club].staff.iter().map(|&s| &w.staff[s]).filter(|s| s.role == StaffRole::Assistant && s.employed()).map(|s| s.person).collect() } else { staff };
    let mut rng = Rng::keyed(&[w.seed, 0x7472_6169_6e, u64::from(today.0 as u32), u64::from(me.0)]);
    let coach = if staff.is_empty() { PersonId::NONE } else { *rng.pick(&staff) };

    let mut traces: SmallVec<[Trace; 6]> = SmallVec::new();
    for e in w.events.since(since) {
        if traces.len() >= 6 {
            break;
        }
        match e.kind {
            E::CoachNote { player, by, note } if player == p => traces.push(Trace::PulledAside { by, good: note == CoachNote::ExcellentTraining }),
            E::Fined { player, club: k, .. } if k == club && player != p => traces.push(Trace::Fined { who: person(w, player) }),
            E::Recovered { player } if player != p && squad.contains(&player) => traces.push(Trace::BackInTraining { who: person(w, player) }),
            E::Transfer { player, to, .. } | E::LoanMove { player, to, .. } if to == club && player != p => traces.push(Trace::Arrived { who: person(w, player) }),
            _ => {}
        }
    }
    // The squad's week: who was flying, who looked off the pace (when a streak starts, and now and then while it lasts), who stayed
    // behind for extra work.
    let mut flying: Vec<(u8, PersonId)> = squad.iter().filter(|&&q| q != p).map(|&q| person(w, q)).filter_map(|q| w.lives.get(q).map(|l| (l.train_high_weeks, q))).filter(|(n, _)| *n == 2 || (*n > 2 && n % 4 == 0)).collect();
    flying.sort_by_key(|(n, q)| (std::cmp::Reverse(*n), *q));
    for (_, q) in flying.into_iter().take(2) {
        if traces.len() < 6 {
            traces.push(Trace::Flying { who: q });
        }
    }
    if let Some((_, q)) = squad.iter().filter(|&&q| q != p).map(|&q| person(w, q)).filter_map(|q| w.lives.get(q).map(|l| (l.train_low_weeks, q))).filter(|(n, _)| *n == 3 || (*n > 3 && n % 6 == 0)).max_by_key(|(n, q)| (*n, std::cmp::Reverse(*q)))
        && traces.len() < 6
    {
        traces.push(Trace::OffThePace { who: q });
    }
    let mgr = w.clubs[club].manager;
    if mgr.is_some() {
        let m = w.staff[mgr].person;
        let extra: Vec<PersonId> = w.social.memories_of(m).filter(|x| x.kind == MemoryKind::ExtraWork && x.date > since && x.about != me).map(|x| x.about).take(1).collect();
        for q in extra {
            if traces.len() < 6 {
                traces.push(Trace::StayedBehind { who: q });
            }
        }
    }
    TrainingWeek { date: today, group, week, coach, traces }
}
