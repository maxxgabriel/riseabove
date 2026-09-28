//! Dressing rooms (07 §6): hierarchy, groups, integration and the spread of
//! feeling. Built from what already exists — influence, relationships,
//! nationality, age, time at the club — and feeding back into morale (the
//! `Settling`, `Teammates` and `Manager` factors), unrest, and the board's
//! sense of whether the manager still has the players.

use pw_core::{ClubId, Hidden, NationId, PlayerId};
use pw_world::dressing::{Bond, Group, Room, Standing};
use pw_world::event::{EventKind, Visibility};
use pw_world::{FxHashMap, MemoryKind, PlayerStatus, SquadStatus, TeamKind, World};
use smallvec::SmallVec;

use crate::consider;

fn first_team(w: &World, club: ClubId) -> Option<pw_core::TeamId> {
    w.clubs[club].teams.iter().copied().find(|&t| w.teams[t].kind == TeamKind::First)
}

fn squad(w: &World, club: ClubId) -> Vec<PlayerId> {
    first_team(w, club).map_or_else(Vec::new, |t| w.teams[t].squad.iter().copied().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect())
}

/// Influence, 0–100: the older measure plus status and international standing.
fn influence(w: &World, p: PlayerId) -> u8 {
    let base = crate::social::influence(w, p) * 100.0;
    let status = match w.players.cold[p].status {
        SquadStatus::Star => 15.0,
        SquadStatus::Important => 8.0,
        SquadStatus::Regular => 3.0,
        SquadStatus::Youngster | SquadStatus::Backup | SquadStatus::NotNeeded => -6.0,
        _ => 0.0,
    };
    let caps = crate::intl::standing(w, p) * 8.0;
    (base + status + caps).clamp(0.0, 100.0) as u8
}

/// Weekly: newcomers settle (or don't).
pub fn weekly(w: &mut World) {
    let today = w.date;
    let clubs: Vec<ClubId> = w.rooms.clubs.keys().copied().collect();
    for club in clubs {
        let players = squad(w, club);
        let home = w.clubs[club].nation;
        let arrivals: Vec<PlayerId> = w.rooms.clubs[&club].integration.keys().copied().collect();
        for p in arrivals {
            if !players.contains(&p) {
                w.rooms.clubs.get_mut(&club).expect("room").integration.remove(&p);
                continue;
            }
            let me = w.players.cold[p].person;
            let adapt = consider::hid(w, me, Hidden::Adaptability);
            let my_nation = w.people[me].nation;
            let compatriots = players.iter().filter(|&&x| x != p && w.people[w.players.cold[x].person].nation == my_nation).count() as f32;
            let language = w.lives.get(me).map_or(0.0, |l| f32::from(l.routine.language)) * 0.3;
            let friends = players.iter().filter(|&&x| x != p && consider::affinity(w, me, w.players.cold[x].person) > 0.3).count() as f32;
            let foreign_penalty = if my_nation != home { 0.6 } else { 1.0 };
            let step = (1.0 + adapt / 6.0 + compatriots.min(3.0) * 0.6 + language + friends.min(4.0) * 0.5) * foreign_penalty;
            let room = w.rooms.clubs.get_mut(&club).expect("room");
            let v = room.integration.get_mut(&p).expect("entry");
            *v = (f32::from(*v) + step).min(100.0) as u8;
            if *v >= 80 {
                room.integration.remove(&p);
                w.events.push(today, Visibility::Club(club), EventKind::PlayerSettled { player: p, club });
                // Someone helped; they are remembered for it.
                if let Some(helper) = players.iter().copied().filter(|&x| x != p).max_by(|&a, &b| {
                    let fa = consider::affinity(w, me, w.players.cold[a].person);
                    let fb = consider::affinity(w, me, w.players.cold[b].person);
                    fa.total_cmp(&fb).then(b.cmp(&a))
                }) {
                    let hp = w.players.cold[helper].person;
                    let compat = consider::compat(w, me, hp);
                    w.social.remember(me, hp, MemoryKind::Settled, today, pw_core::EventId::NONE, false, 0.7, compat);
                }
            }
        }
    }
}

/// Monthly: rebuild hierarchy and groups, spread feeling, tell the board.
pub fn monthly(w: &mut World) {
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| first_team(w, c).is_some()).collect();
    for club in clubs {
        rebuild(w, club);
    }
}

fn rebuild(w: &mut World, club: ClubId) {
    let today = w.date;
    let players = squad(w, club);
    if players.len() < 11 {
        return;
    }
    let manager = w.clubs[club].manager.get().map(|m| w.staff[m].person);
    let prev = w.rooms.clubs.remove(&club).unwrap_or_else(|| Room { club, ..Default::default() });

    // Influence and newcomers.
    let mut infl: FxHashMap<PlayerId, u8> = FxHashMap::default();
    let mut integration = prev.integration.clone();
    for &p in &players {
        infl.insert(p, influence(w, p));
        let tenure = w.players.cold[p].joined.days_until(today);
        if tenure < 60 && !prev.standing.contains_key(&p) && !integration.contains_key(&p) {
            let me = w.players.cold[p].person;
            let local = w.people[me].nation == w.clubs[club].nation;
            let start = 20.0 + consider::hid(w, me, Hidden::Adaptability) * 1.5 + if local { 25.0 } else { 0.0 };
            integration.insert(p, start.min(79.0) as u8);
        }
    }

    // Hierarchy.
    let mut ranked: Vec<(PlayerId, u8)> = infl.iter().map(|(&p, &v)| (p, v)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut standing: FxHashMap<PlayerId, Standing> = FxHashMap::default();
    for (i, &(p, v)) in ranked.iter().enumerate() {
        let tenure = w.players.cold[p].joined.days_until(today);
        let s = if integration.contains_key(&p) {
            Standing::Newcomer
        } else if i < 3 && v >= 55 {
            Standing::Leader
        } else if v >= 40 {
            Standing::Influential
        } else if tenure >= 365 {
            Standing::Established
        } else {
            Standing::Peripheral
        };
        if s == Standing::Leader && prev.standing.get(&p) != Some(&Standing::Leader) && !prev.standing.is_empty() {
            w.events.push(today, Visibility::Club(club), EventKind::LeaderEmerged { player: p, club });
        }
        standing.insert(p, s);
    }

    // Leaders who have gone leave a hole their friends feel.
    for (&p, &s) in &prev.standing {
        if s == Standing::Leader && !players.contains(&p) {
            let lp = w.players.cold[p].person;
            for &x in &players {
                let xp = w.players.cold[x].person;
                if consider::affinity(w, xp, lp) > 0.3 {
                    let h = &mut w.players.hot[x];
                    h.morale = h.morale.saturating_sub(4);
                }
            }
        }
    }

    let groups = form_groups(w, club, &players, &infl, manager);

    // Feeling spreads from group leaders to members.
    if let Some(m) = manager {
        for g in &groups {
            let lp = w.players.cold[g.leader].person;
            let lt = consider::trust(w, lp, m);
            let li = f32::from(infl.get(&g.leader).copied().unwrap_or(0)) / 100.0;
            for &x in &g.members {
                if x == g.leader {
                    continue;
                }
                let xp = w.players.cold[x].person;
                let xt = consider::trust(w, xp, m);
                let pull = (lt - xt) * 0.12 * f32::from(g.cohesion) / 100.0 * li;
                if pull.abs() > 0.01 {
                    let compat = consider::compat(w, xp, m);
                    w.social.adjust(xp, m, today, compat, 0, (pull * 100.0) as i32, 0);
                }
            }
            let was = prev.groups.iter().find(|pg| pg.leader == g.leader).map_or(50, |pg| pg.stance);
            if g.stance < 30 && was >= 30 && li >= 0.55 {
                w.events.push(today, Visibility::Club(club), EventKind::DressingRoomSplit { club, leader: g.leader });
            }
        }
    }

    // Collective measures.
    let total_infl: f32 = infl.values().map(|&v| f32::from(v)).sum::<f32>().max(1.0);
    let backing = manager.map_or(50.0, |m| players.iter().map(|&p| consider::trust(w, w.players.cold[p].person, m) * f32::from(infl[&p])).sum::<f32>() / total_infl * 100.0);
    let spread = if groups.len() >= 2 {
        let lo = groups.iter().map(|g| g.stance).min().unwrap_or(50);
        let hi = groups.iter().map(|g| g.stance).max().unwrap_or(50);
        f32::from(hi - lo)
    } else {
        0.0
    };
    let cohesion = if groups.is_empty() { 50.0 } else { groups.iter().map(|g| f32::from(g.cohesion)).sum::<f32>() / groups.len() as f32 };
    let harmony = (cohesion * 0.6 + backing * 0.4 - spread * 0.3).clamp(0.0, 100.0);

    // The board hears when the players have stopped listening.
    if backing < 30.0 {
        let b = &mut w.clubs[club].board;
        b.satisfaction = b.satisfaction.saturating_sub(3);
    }

    w.rooms.clubs.insert(club, Room { club, groups, standing, influence: infl, integration, harmony: harmony as u8, backing: backing.clamp(0.0, 100.0) as u8, updated: today });
}

/// Group the squad around what binds people: a shared foreign language, the
/// academy, the veterans, the young — and then friendship.
fn form_groups(w: &World, club: ClubId, players: &[PlayerId], infl: &FxHashMap<PlayerId, u8>, manager: Option<pw_core::PersonId>) -> Vec<Group> {
    let home = w.clubs[club].nation;
    let mut by_nation: FxHashMap<NationId, Vec<PlayerId>> = FxHashMap::default();
    for &p in players {
        let n = w.people[w.players.cold[p].person].nation;
        if n != home {
            by_nation.entry(n).or_default().push(p);
        }
    }
    let mut assigned: FxHashMap<PlayerId, usize> = FxHashMap::default();
    let mut buckets: Vec<(Bond, Vec<PlayerId>)> = Vec::new();
    let mut nations: Vec<(NationId, Vec<PlayerId>)> = by_nation.into_iter().filter(|(_, v)| v.len() >= 3).collect();
    nations.sort_by(|a, b| a.0.cmp(&b.0));
    for (n, v) in nations {
        buckets.push((Bond::Nationality(n), v));
    }
    let academy: Vec<PlayerId> = players.iter().copied().filter(|&p| w.players.cold[p].youth_club == club).collect();
    if academy.len() >= 3 {
        buckets.push((Bond::Academy, academy));
    }
    let veterans: Vec<PlayerId> = players.iter().copied().filter(|&p| w.age_years(p) >= 30.0).collect();
    if veterans.len() >= 3 {
        buckets.push((Bond::Veterans, veterans));
    }
    let young: Vec<PlayerId> = players.iter().copied().filter(|&p| w.age_years(p) <= 23.0).collect();
    if young.len() >= 3 {
        buckets.push((Bond::Generation, young));
    }
    for (i, (_, v)) in buckets.iter().enumerate() {
        for &p in v {
            assigned.entry(p).or_insert(i);
        }
    }
    // Everyone else gravitates to the group of the teammate they like most.
    for &p in players {
        if assigned.contains_key(&p) {
            continue;
        }
        let me = w.players.cold[p].person;
        let best = players
            .iter()
            .copied()
            .filter(|&x| x != p && assigned.contains_key(&x))
            .map(|x| (x, consider::affinity(w, me, w.players.cold[x].person)))
            .filter(|&(_, a)| a > 0.2)
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
        if let Some((x, _)) = best {
            let g = assigned[&x];
            assigned.insert(p, g);
        }
    }
    let mut members: Vec<SmallVec<[PlayerId; 8]>> = vec![SmallVec::new(); buckets.len()];
    let mut sorted: Vec<(&PlayerId, &usize)> = assigned.iter().collect();
    sorted.sort();
    for (&p, &g) in sorted {
        members[g].push(p);
    }
    buckets
        .into_iter()
        .zip(members)
        .filter(|(_, m)| m.len() >= 3)
        .map(|((bond, _), m)| {
            let leader = m.iter().copied().max_by(|&a, &b| infl.get(&a).cmp(&infl.get(&b)).then(b.cmp(&a))).unwrap_or(m[0]);
            let mut aff = 0.0f32;
            let mut n = 0.0f32;
            for (i, &a) in m.iter().enumerate() {
                for &b in &m[i + 1..] {
                    aff += consider::affinity(w, w.players.cold[a].person, w.players.cold[b].person);
                    n += 1.0;
                }
            }
            let cohesion = (50.0 + aff / n.max(1.0) * 80.0 + m.len() as f32).clamp(0.0, 100.0) as u8;
            let stance = manager.map_or(50.0, |mgr| m.iter().map(|&x| consider::trust(w, w.players.cold[x].person, mgr)).sum::<f32>() / m.len() as f32 * 100.0);
            Group { members: m, leader, bond, cohesion, stance: stance.clamp(0.0, 100.0) as u8 }
        })
        .collect()
}

/// Settling and group feeling as morale inputs (read by `morale::compose`).
pub fn mood_inputs(w: &World, p: PlayerId) -> (Option<f32>, Option<f32>) {
    let club = w.players.hot[p].club;
    let Some(room) = w.rooms.clubs.get(&club) else { return (None, None) };
    let settling = room.integration.get(&p).map(|&v| {
        let months = w.players.cold[p].joined.days_until(w.date) as f32 / 30.0;
        // Early days are exciting; being unsettled after months is not.
        if months < 2.0 { 2.0 } else { (f32::from(v) - 60.0) / 6.0 }
    });
    let group = room.group_of(p).map(|g| (f32::from(g.stance) - 50.0) / 10.0 * f32::from(g.cohesion) / 100.0);
    (settling, group)
}
