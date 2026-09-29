//! Managers' careers and what they do to squads (07 §2, §10).
//!
//! A manager arriving is one of the biggest things that can happen to a
//! player without anything happening to the player: the new man brings his
//! own system, his own kind of player, his own staff and his own favourites,
//! and he has no memories of anyone at his new club. Statuses are
//! re-assessed through his eyes on arrival. Managers resign, get poached by
//! bigger clubs (which then have a vacancy of their own), change systems when
//! things go wrong, and build reputations that follow them.

use pw_core::rng::{Rng, stream};
use pw_core::{Attr, ClubId, Hidden, Money, PersonId, PlayerId, Pos, PosGroup, StaffAttr, StaffId};
use pw_world::careers::{Archetype2, Job, JobEnd, ManagerProfile, MediaStyle};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::{MemoryKind, StaffRole, World};
use smallvec::SmallVec;

use crate::consider;

/// What kind of player this is, as a manager would pigeonhole them.
pub fn archetype_of(w: &World, p: PlayerId) -> Archetype2 {
    let c = &w.players.cold[p];
    let a = |x: Attr| c.attrs.get(x);
    match c.best_pos.group() {
        PosGroup::Gk => Archetype2::SweeperKeeper,
        PosGroup::Def => {
            if matches!(c.best_pos, Pos::DL | Pos::DR | Pos::WBL | Pos::WBR) && a(Attr::Crossing) + a(Attr::Pace) > 26.0 {
                Archetype2::AttackingFullBack
            } else if a(Attr::Passing) + a(Attr::Composure) > a(Attr::Heading) + a(Attr::Strength) {
                Archetype2::BallPlayingDefender
            } else {
                Archetype2::TallDefender
            }
        }
        PosGroup::Mid => {
            if a(Attr::Vision) + a(Attr::Passing) >= 30.0 {
                Archetype2::Playmaker
            } else if a(Attr::Tackling) + a(Attr::Aggression) >= 28.0 {
                Archetype2::Destroyer
            } else if a(Attr::WorkRate) + a(Attr::Stamina) >= 28.0 {
                Archetype2::BoxToBox
            } else if a(Attr::Pace) + a(Attr::Dribbling) >= 28.0 {
                Archetype2::PacyWinger
            } else {
                Archetype2::Technician
            }
        }
        PosGroup::Att => {
            if a(Attr::Heading) + a(Attr::Strength) >= 29.0 {
                Archetype2::TargetMan
            } else if a(Attr::Pace) + a(Attr::Dribbling) >= 29.0 && matches!(c.best_pos, Pos::AML | Pos::AMR) {
                Archetype2::PacyWinger
            } else if a(Attr::WorkRate) + a(Attr::Teamwork) >= 29.0 {
                Archetype2::Workhorse
            } else if a(Attr::Finishing) + a(Attr::OffTheBall) >= 29.0 {
                Archetype2::Poacher
            } else {
                Archetype2::Technician
            }
        }
    }
}

/// How the club's manager feels about this kind of player, −1..1, plus
/// favouritism for players he has managed and rated before.
pub fn preference(w: &World, manager: StaffId, p: PlayerId) -> f32 {
    let Some(prof) = w.careers.managers.get(&manager) else { return 0.0 };
    let a = archetype_of(w, p);
    let mut v = 0.0;
    if prof.likes.contains(&a) {
        v += 0.5;
    }
    if prof.dislikes == Some(a) {
        v -= 0.5;
    }
    if prof.favourites.contains(&w.players.cold[p].person) {
        v += f32::from(prof.favouritism) / 100.0;
    }
    let age = w.age_years(p);
    if age >= 31.0 {
        v += (f32::from(prof.veteran_loyalty) - 50.0) / 150.0;
    }
    v.clamp(-1.0, 1.0)
}

/// Does the club's manager want this player (a former favourite, a type he loves)?
pub fn wants(w: &World, club: ClubId, p: PlayerId) -> f32 {
    match w.clubs[club].manager.get() {
        Some(m) => {
            let infl = w.careers.managers.get(&m).map_or(0.5, |x| f32::from(x.recruitment_influence) / 100.0);
            preference(w, m, p).max(0.0) * infl
        }
        None => 0.0,
    }
}

fn profile_for(w: &World, s: StaffId) -> ManagerProfile {
    let st = &w.staff[s];
    let person = &w.people[st.person];
    let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(s.0), 0x3a9]);
    let mut likes: SmallVec<[Archetype2; 2]> = SmallVec::new();
    // Tastes follow philosophy: pressing managers love workhorses, possession
    // managers love technicians and ball-playing defenders, direct ones target men.
    let ph = st.philosophy;
    let lean: &[Archetype2] = if ph.press >= 65 {
        &[Archetype2::Workhorse, Archetype2::BoxToBox, Archetype2::Destroyer]
    } else if ph.directness <= 35 {
        &[Archetype2::Technician, Archetype2::BallPlayingDefender, Archetype2::Playmaker]
    } else if ph.directness >= 65 {
        &[Archetype2::TargetMan, Archetype2::TallDefender, Archetype2::PacyWinger]
    } else {
        &Archetype2::ALL
    };
    likes.push(lean[rng.index(lean.len())]);
    if rng.chance(0.6) {
        let second = Archetype2::ALL[rng.index(Archetype2::ALL.len())];
        if !likes.contains(&second) {
            likes.push(second);
        }
    }
    let dislikes = rng.chance(0.5).then(|| Archetype2::ALL[rng.index(Archetype2::ALL.len())]).filter(|d| !likes.contains(d));
    let r = |rng: &mut Rng, m: f32| rng.normal_ms(m, 18.0).clamp(0.0, 100.0) as u8;
    let temper = person.hidden.f(Hidden::Temperament);
    let media = st.attrs.f(StaffAttr::MediaHandling);
    let media_style = if temper <= 7.0 {
        MediaStyle::Combative
    } else if media >= 15.0 {
        MediaStyle::Charming
    } else if person.hidden.f(Hidden::Sportsmanship) >= 14.0 {
        MediaStyle::Candid
    } else {
        MediaStyle::Guarded
    };
    let rotation = match ph.archetype {
        pw_world::Archetype::Rotator => 75.0,
        pw_world::Archetype::Loyalist => 25.0,
        _ => 45.0,
    };
    let veteran = if ph.archetype == pw_world::Archetype::Loyalist {
        70.0
    } else if ph.archetype == pw_world::Archetype::Developer {
        30.0
    } else {
        50.0
    };
    ManagerProfile {
        staff: s,
        likes,
        dislikes,
        rotation: r(&mut rng, rotation),
        veteran_loyalty: r(&mut rng, veteran),
        favouritism: r(&mut rng, 40.0 + person.hidden.f(Hidden::Loyalty) * 2.0),
        adaptability: r(&mut rng, 30.0 + st.attrs.f(StaffAttr::TacticalKnowledge) * 2.5),
        ambition: r(&mut rng, person.hidden.f(Hidden::Ambition) * 5.0),
        recruitment_influence: r(&mut rng, 50.0),
        media_style,
        entourage: SmallVec::new(),
        reputation: st.reputation,
        jobs: if st.club.is_some() { vec![Job { club: st.club, from: st.joined, to: None, ended: None, record_at_start: st.record }] } else { Vec::new() },
        systems: ph.formations.iter().copied().collect(),
        favourites: SmallVec::new(),
    }
}

/// Every manager (and would-be manager) has a profile.
pub fn ensure(w: &mut World) {
    let ids: Vec<StaffId> = w.staff.ids().filter(|&s| w.staff[s].role == StaffRole::Manager && !w.careers.managers.contains_key(&s)).collect();
    for s in ids {
        let prof = profile_for(w, s);
        w.careers.managers.insert(s, prof);
    }
    // Entourages: the assistant and a coach he gets on with at his current club.
    let managers: Vec<StaffId> = w.careers.managers.keys().copied().collect();
    let mut managers = managers;
    managers.sort();
    for m in managers {
        if !w.careers.managers[&m].entourage.is_empty() {
            continue;
        }
        let club = w.staff[m].club;
        if club.is_none() {
            continue;
        }
        let mp = w.staff[m].person;
        let mut close: Vec<(StaffId, f32)> = w.clubs[club]
            .staff
            .iter()
            .copied()
            .filter(|&s| s != m && matches!(w.staff[s].role, StaffRole::Assistant | StaffRole::Coach | StaffRole::FitnessCoach))
            .map(|s| (s, f32::from(consider::compat(w, mp, w.staff[s].person))))
            .filter(|&(_, c)| c > 8.0)
            .collect();
        close.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let ent: SmallVec<[StaffId; 3]> = close.into_iter().take(2).map(|(s, _)| s).collect();
        if let Some(p) = w.careers.managers.get_mut(&m) {
            p.entourage = ent;
        }
    }
}

/// A manager takes a job: CV, staff who follow, knowledge he brings, and a
/// fresh look at every player through his own eyes.
pub fn on_appointment(w: &mut World, m: StaffId, club: ClubId) {
    let today = w.date;
    if !w.careers.managers.contains_key(&m) {
        let prof = profile_for(w, m);
        w.careers.managers.insert(m, prof);
    }
    let record = w.staff[m].record;
    let prev_clubs: Vec<(ClubId, pw_core::Date, Option<pw_core::Date>)> = w.careers.managers[&m].jobs.iter().map(|j| (j.club, j.from, j.to)).collect();
    if let Some(p) = w.careers.managers.get_mut(&m) {
        p.jobs.push(Job { club, from: today, to: None, ended: None, record_at_start: record });
    }
    // Staff who follow him (if their own clubs let them go).
    let entourage = w.careers.managers[&m].entourage.clone();
    for s in entourage {
        let st = &w.staff[s];
        // A follower who has since been promoted to run a club of his own stays where he is (a poaching chain can promote an assistant
        // of the very manager being poached, who would otherwise be dragged away from his new chair).
        if st.retired || st.club == club || st.role == StaffRole::Manager {
            continue;
        }
        let old = st.club;
        if old.is_some() {
            w.clubs[old].staff.retain(|&x| x != s);
        }
        let st = &mut w.staff[s];
        st.club = club;
        st.joined = today;
        st.contract_end = today.add_months(24);
        w.clubs[club].staff.push(s);
        w.events.push(today, Visibility::Public, EventKind::StaffFollowed { staff: s, manager: m, club });
        // The old assistant at the new club makes way.
        if w.staff[s].role == StaffRole::Assistant {
            let old_assistant = w.clubs[club].staff.iter().copied().find(|&x| x != s && w.staff[x].role == StaffRole::Assistant);
            if let Some(a) = old_assistant {
                w.clubs[club].staff.retain(|&x| x != a);
                w.staff[a].club = ClubId::NONE;
                w.events.push(today, Visibility::Public, EventKind::StaffLeft { staff: a, club });
            }
        }
    }
    // He knows the players he managed before, and remembers who he rated.
    let mp = w.staff[m].person;
    let mut favourites: SmallVec<[PersonId; 8]> = SmallVec::new();
    let mut known: Vec<PlayerId> = Vec::new();
    for (c, from, to) in prev_clubs {
        let to = to.unwrap_or(today);
        for (p, spells) in &w.history.spells {
            if spells.iter().any(|s| s.club == c && s.from <= to && s.to.is_none_or(|t| t >= from)) {
                known.push(*p);
            }
        }
    }
    known.sort();
    known.dedup();
    for p in known {
        if w.players.hot[p].status == pw_world::PlayerStatus::Retired {
            continue;
        }
        w.knowledge.observe(club, p, 900, today);
        let who = w.players.cold[p].person;
        let rated = consider::trust(w, mp, who) + consider::memory(w, mp, who, MemoryKind::ExtraWork) * 0.2 + consider::memory(w, mp, who, MemoryKind::GaveChance) * 0.2;
        if rated > 0.62 && favourites.len() < 8 {
            favourites.push(who);
        }
    }
    if let Some(p) = w.careers.managers.get_mut(&m) {
        p.favourites = favourites;
    }
    // Everyone is re-assessed through the new man's eyes.
    crate::market::reassess(w, club);
}

/// A manager leaves a club (sacked, resigned, poached, contract end).
pub fn on_departure(w: &mut World, m: StaffId, club: ClubId, how: JobEnd) {
    let today = w.date;
    let record = w.staff[m].record;
    if let Some(p) = w.careers.managers.get_mut(&m) {
        if let Some(j) = p.jobs.iter_mut().rev().find(|j| j.club == club && j.to.is_none()) {
            j.to = Some(today);
            j.ended = Some(how);
        }
        // Reputation follows results relative to expectations.
        let played = record.games.saturating_sub(p.jobs.last().map_or(0, |j| j.record_at_start.games)).max(1);
        let won = record.wins.saturating_sub(p.jobs.last().map_or(0, |j| j.record_at_start.wins));
        let rate = won as f32 / played as f32;
        let change = ((rate - 0.4) * 2000.0) as i32 - if how == JobEnd::Sacked { 400 } else { 0 };
        p.reputation = (i32::from(p.reputation) + change).clamp(100, 10_000) as u16;
        w.staff[m].reputation = p.reputation;
    }
    // Players react according to how they got on with him.
    let mp = w.staff[m].person;
    let squad: Vec<PlayerId> = w.clubs[club].teams.iter().flat_map(|&t| w.teams[t].squad.clone()).collect();
    for p in squad {
        let who = w.players.cold[p].person;
        let aff = consider::affinity(w, who, mp);
        let h = &mut w.players.hot[p];
        h.morale = (i32::from(h.morale) - (aff * 8.0) as i32).clamp(5, 100) as u8;
    }
}

/// Monthly: managers change systems when things go wrong, resign when the
/// relationship upstairs breaks, and see out or renew their contracts.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let month = (today.year() as u64) * 12 + u64::from(today.month());
    let clubs: Vec<ClubId> = w.clubs.ids().collect();
    for club in clubs {
        let Some(m) = w.clubs[club].manager.get() else { continue };
        let Some(prof) = w.careers.managers.get(&m).cloned() else { continue };
        let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(m.0), month]);
        let sat = w.clubs[club].board.satisfaction;
        // Tactical change under pressure, if he's the adaptable kind.
        if sat < 40
            && rng.chance(f32::from(prof.adaptability) / 250.0)
            && let Some(f) = best_formation(w, club)
            && !w.staff[m].philosophy.formations.contains(&f)
        {
            let ph = &mut w.staff[m].philosophy;
            ph.formations = [f, ph.formations[0]];
            if let Some(p) = w.careers.managers.get_mut(&m) {
                p.systems.push(f);
                if p.systems.len() > 4 {
                    p.systems.remove(0);
                }
            }
            let causes: Causes = pw_world::causes![Cause::Fact(Fact::BoardPressure { club, warnings: w.clubs[club].board.warnings })];
            w.events.push_caused(today, Visibility::Public, EventKind::TacticalChange { club, staff: m, formation: f }, causes);
        }
        // Resignation: proud managers walk when the owner turns on them.
        let chair = w.governance.get(&club).map(|g| g.chairman);
        let relation = chair.map_or(0.5, |c| consider::trust(w, w.staff[m].person, c));
        let pride = f32::from(prof.reputation) / 10_000.0 + consider::hid(w, w.staff[m].person, Hidden::Pressure) / 40.0;
        if sat < 25 && relation < 0.35 && rng.chance(pride * 0.15) {
            resign(w, m, club);
            continue;
        }
        // Contract end: renew if the board is happy, otherwise part ways.
        let left = w.staff[m].contract_end.days_until(today);
        let settled_in = w.staff[m].joined.days_until(today) > 180;
        if left >= 0 && !settled_in {
            w.staff[m].contract_end = today.add_months(24);
        } else if left >= 0 {
            if sat >= 55 {
                w.staff[m].contract_end = today.add_months(24);
            } else {
                depart(w, m, club, JobEnd::ContractExpired);
                crate::board::appoint(w, club);
            }
        }
    }
    // Yearly-ish retirement of old managers.
    if today.month() == 6 {
        let old: Vec<StaffId> = w.careers.managers.keys().copied().filter(|&s| !w.staff[s].retired && consider::age(w, w.staff[s].person) > 66.0).collect();
        for s in old {
            let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(s.0), today.year() as u64, 0x77]);
            if rng.chance(0.3) {
                let club = w.staff[s].club;
                // Retired first: the search for a successor must not find him among the unemployed and give him the job back.
                w.staff[s].retired = true;
                if club.is_some() && w.clubs[club].manager == s {
                    depart(w, s, club, JobEnd::Retired);
                    crate::board::appoint(w, club);
                }
            }
        }
    }
}

fn depart(w: &mut World, m: StaffId, club: ClubId, how: JobEnd) {
    w.staff[m].club = ClubId::NONE;
    w.clubs[club].staff.retain(|&s| s != m);
    if w.clubs[club].manager == m {
        w.clubs[club].manager = StaffId::NONE;
    }
    on_departure(w, m, club, how);
}

fn resign(w: &mut World, m: StaffId, club: ClubId) {
    let today = w.date;
    let causes: Causes = pw_world::causes![Cause::Fact(Fact::BoardPressure { club, warnings: w.clubs[club].board.warnings })];
    w.events.push_caused(today, Visibility::Public, EventKind::ManagerResigned { staff: m, club }, causes);
    depart(w, m, club, JobEnd::Resigned);
    crate::board::appoint(w, club);
}

/// The formation that best fits the squad as the manager sees it.
fn best_formation(w: &World, club: ClubId) -> Option<u8> {
    let team = w.clubs[club].first_team();
    let squad = &w.teams[team].squad;
    let mut best: Option<(f32, u8)> = None;
    for (i, f) in w.data.formations.iter().enumerate() {
        let mut used: Vec<PlayerId> = Vec::new();
        let mut total = 0.0;
        for slot in &f.slots {
            let pick = squad
                .iter()
                .copied()
                .filter(|p| !used.contains(p))
                .map(|p| (p, crate::perception::club_view(w, club, p).0 * pw_world::player::familiarity_factor(w.players.cold[p].familiarity[slot.pos.idx()])))
                .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
            if let Some((p, v)) = pick {
                used.push(p);
                total += v;
            }
        }
        if best.is_none_or(|(b, _)| total > b) {
            best = Some((total, i as u8));
        }
    }
    best.map(|(_, i)| i)
}

/// Try to lure an employed manager from a smaller club. Returns the manager
/// if he accepts; his old club is paid compensation and must appoint in turn.
pub fn try_poach(w: &mut World, club: ClubId, rival_fit: f32) -> Option<StaffId> {
    let today = w.date;
    let rep = w.clubs[club].reputation;
    let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(club.0), today.0 as u64, 0x90]);
    let candidates: Vec<(StaffId, f32)> = w
        .careers
        .managers
        .iter()
        .filter(|(s, _)| {
            let st = &w.staff[**s];
            st.employed() && st.role == StaffRole::Manager && st.club != club && w.clubs[st.club].reputation + 800 < rep && w.clubs[st.club].manager == **s
        })
        .map(|(s, p)| {
            let fit = f32::from(p.reputation) / 1000.0 + w.staff[*s].role_rating(StaffRole::Manager) / 4.0 - (f32::from(p.reputation) - f32::from(rep)).abs() / 2500.0;
            (*s, fit)
        })
        .collect();
    let (m, fit) = candidates.into_iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))?;
    if fit <= rival_fit + 0.3 {
        return None;
    }
    // His own decision: step up, loyalty to his club, how things stand there.
    let old = w.staff[m].club;
    let prof = &w.careers.managers[&m];
    let step = (f32::from(rep) - f32::from(w.clubs[old].reputation)) / 3000.0;
    let content = f32::from(w.clubs[old].board.satisfaction) / 100.0;
    let loyal = consider::hid(w, w.staff[m].person, Hidden::Loyalty) / 20.0;
    let accept = step * (0.5 + f32::from(prof.ambition) / 100.0) - loyal * 0.3 - content * 0.2 + rng.normal() * 0.1 > 0.1;
    if !accept {
        return None;
    }
    let compensation: Money = w.staff[m].wage * 52;
    crate::finance::pay_fee(w, club, old, compensation);
    w.events.push(today, Visibility::Public, EventKind::ManagerPoached { staff: m, from: old, to: club, compensation });
    depart(w, m, old, JobEnd::Poached);
    // He is now spoken for. Without this the old club's search below saw an unemployed manager, picked its own departed manager again, and
    // he then also took the new job: one man running two (or three, in a chain of poachings) clubs.
    w.staff[m].club = club;
    // The old club's supporters and players feel it; the old club must replace him.
    crate::board::appoint(w, old);
    Some(m)
}

/// Manager reputation drifts yearly toward what the current job's results say.
pub fn yearly(w: &mut World) {
    let ids: Vec<StaffId> = w.careers.managers.keys().copied().collect();
    for m in ids {
        let club = w.staff[m].club;
        if club.is_none() {
            continue;
        }
        let rep = f32::from(w.clubs[club].reputation);
        let sat = f32::from(w.clubs[club].board.satisfaction);
        if let Some(p) = w.careers.managers.get_mut(&m) {
            let target = rep * (0.6 + sat / 125.0);
            p.reputation = (f32::from(p.reputation) * 0.8 + target * 0.2).clamp(100.0, 10_000.0) as u16;
            w.staff[m].reputation = p.reputation;
        }
    }
}
