//! Boards judge managers (07 §10): season targets, weekly satisfaction,
//! warnings, sackings and appointments.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Date, NationId, StaffAttr, StaffId};
use pw_world::event::{EventKind, Visibility};
use pw_world::staff::ManagerRecord;
use pw_world::{Person, Philosophy, Staff, StaffRole, World};

use crate::generate as gen_;

pub fn set_targets(w: &mut World, n: NationId) {
    for &league in &w.nations[n].leagues.clone() {
        let mut clubs: Vec<ClubId> = w.comps[league].state.entrants.iter().map(|&t| w.teams[t].club).collect();
        clubs.sort_by_key(|&c| std::cmp::Reverse(w.clubs[c].reputation));
        let size = clubs.len();
        for (rank, c) in clubs.into_iter().enumerate() {
            let b = &mut w.clubs[c].board;
            // Boards expect roughly their reputation rank, with a little slack.
            b.target_position = ((rank + 1) as f32 * 1.1 + 1.0).round().min(size as f32) as u8;
            b.satisfaction = b.satisfaction.max(55);
            b.warnings = 0;
        }
    }
}

pub fn weekly(w: &mut World) {
    let today = w.date;
    let mut sack: Vec<ClubId> = Vec::new();
    for club in w.clubs.ids() {
        let c = &w.clubs[club];
        if c.manager.is_none() {
            sack.push(club);
            continue;
        }
        let league = c.league;
        if league.is_none() {
            continue;
        }
        let comp = &w.comps[league];
        let team = c.first_team();
        let Some(row) = comp.state.table.iter().find(|r| r.team == team) else { continue };
        if row.played < 6 {
            continue;
        }
        let pos = comp.position_of(team).unwrap_or(comp.state.table.len()) as f32;
        let size = comp.state.table.len().max(1) as f32;
        let target = f32::from(c.board.target_position);
        let gap = (target - pos) / size; // + ahead of target
        let relegation = pos > size - f32::from(comp.relegate) - 0.5;
        let delta = gap * 6.0 - if relegation { 2.0 } else { 0.0 } + 0.5;
        let patience = 0.6 + f32::from(w.clubs[club].board.patience) / 100.0;
        // Hot-headed or ambitious chairmen feel bad results more sharply.
        let temper = crate::governance::owner_temper(w, club);
        let felt = if delta < 0.0 { delta * temper } else { delta };
        let b = &mut w.clubs[club].board;
        b.satisfaction = (f32::from(b.satisfaction) + felt / patience).clamp(0.0, 100.0) as u8;
        if b.satisfaction < 15 {
            b.warnings += 1;
            b.satisfaction = 40;
            if b.warnings >= 3 {
                sack.push(club);
            }
        }
    }
    for club in sack {
        if let Some(m) = w.clubs[club].manager.get() {
            w.staff[m].club = pw_core::ClubId::NONE;
            w.staff[m].record.sackings += 1;
            w.clubs[club].staff.retain(|&s| s != m);
            w.clubs[club].manager = StaffId::NONE;
            let warnings = w.clubs[club].board.warnings;
            let causes: pw_world::Causes = pw_world::causes![pw_world::Cause::Fact(pw_world::Fact::BoardPressure { club, warnings })];
            w.events.push_caused(today, Visibility::Public, EventKind::ManagerSacked { staff: m, club }, causes);
            crate::managers::on_departure(w, m, club, pw_world::careers::JobEnd::Sacked);
        }
        appoint(w, club);
    }
}

/// Fill a vacancy: best-fitting unemployed manager, else promote the
/// assistant, else a newly qualified coach.
pub fn appoint(w: &mut World, club: ClubId) {
    let today = w.date;
    let rep = i32::from(w.clubs[club].reputation);
    let nation = w.clubs[club].nation;
    let best = w
        .staff
        .iter_enumerated()
        .filter(|(id, s)| s.role == StaffRole::Manager && !s.employed() && !s.retired && !w.intl.managers.contains(id))
        // Licensing: bigger clubs need higher coaching badges.
        .filter(|(_, s)| crate::affairs::coaching_level(w, s.person) >= crate::affairs::required_level(w.clubs[club].reputation))
        .filter(|(_, s)| i32::from(s.reputation) <= rep + 1500)
        .map(|(id, s)| {
            let fit = -((i32::from(s.reputation) - rep).abs() as f32) / 1000.0 + s.role_rating(StaffRole::Manager) / 4.0
                + if w.people[s.person].nation == nation { 0.5 } else { 0.0 };
            (id, fit)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
    let best_fit = best.map_or(-9.0, |b| b.1);
    let best = best.map(|b| b.0);
    // A bigger club may prefer to lure a manager doing well elsewhere.
    let best = crate::managers::try_poach(w, club, best_fit).or(best);
    let chosen = best.or_else(|| {
        let a = w.clubs[club].staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::Assistant)?;
        w.staff[a].role = StaffRole::Manager;
        w.clubs[club].staff.retain(|&s| s != a);
        Some(a)
    });
    let m = chosen.unwrap_or_else(|| new_manager(w, club));
    let wage = (crate::finance::season_revenue(w, club) as f64 * 0.006 / 52.0) as i64;
    let s = &mut w.staff[m];
    s.club = club;
    s.joined = today;
    s.contract_end = today.add_months(24);
    s.wage = wage;
    w.clubs[club].manager = m;
    if !w.clubs[club].staff.contains(&m) {
        w.clubs[club].staff.push(m);
    }
    w.clubs[club].board.satisfaction = 60;
    w.clubs[club].board.warnings = 0;
    w.events.push(today, Visibility::Public, EventKind::ManagerAppointed { staff: m, club });
    crate::managers::on_appointment(w, m, club);
}

/// A newly qualified manager when the market is empty (F7).
fn new_manager(w: &mut World, club: ClubId) -> StaffId {
    let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(club.0), w.date.0 as u64]);
    let nation = w.clubs[club].nation;
    let level = 6.0 + f32::from(w.clubs[club].reputation) / 1000.0;
    let (first, last) = crate::people::random_name(w, nation, &mut rng);
    let dob = w.date.add_days(-(365 * rng.range_i32(35, 52)));
    let person = w.people.push(Person {
        first,
        last,
        common: Default::default(),
        dob: Date(dob.0),
        nation,
        nation2: Default::default(),
        hidden: gen_::hidden_random(&mut rng),
        player: Default::default(),
        staff: Default::default(),
        mind: Default::default(),
    });
    let mut attrs = pw_core::StaffAttrs::default();
    for a in StaffAttr::ALL {
        attrs.set(a, rng.normal_ms(level, 2.5).round().clamp(1.0, 20.0) as u8);
    }
    let phil = Philosophy {
        formations: [rng.below(w.data.formations.len() as u32) as u8, rng.below(w.data.formations.len() as u32) as u8],
        mentality: rng.range_i32(-1, 1) as i8,
        press: rng.range_i32(30, 75) as u8,
        tempo: rng.range_i32(35, 70) as u8,
        directness: rng.range_i32(25, 75) as u8,
        youth_trust: rng.range_i32(20, 80) as u8,
        archetype: [pw_world::Archetype::Pragmatist, pw_world::Archetype::Developer, pw_world::Archetype::Rotator, pw_world::Archetype::Loyalist][rng.index(4)],
    };
    let id = w.staff.push(Staff {
        person,
        role: StaffRole::Manager,
        club: ClubId::NONE,
        attrs,
        wage: 0,
        contract_end: w.date,
        reputation: (f32::from(w.clubs[club].reputation) * 0.6) as u16,
        philosophy: phil,
        joined: w.date,
        record: ManagerRecord::default(),
        retired: false,
    });
    w.people[person].staff = id;
    id
}
