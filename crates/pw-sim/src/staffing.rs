//! The staff job market (07 §12). Clubs keep a backroom sized to their
//! ambitions and fill gaps from the pool of people looking for football work:
//! retired players (whoever controlled them), sacked coaches, promoted
//! assistants. Being hired depends on your attributes, your reputation as a
//! player, who you know at the club, and whether they rate you.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, StaffId};
use pw_world::event::{EventKind, Visibility};
use pw_world::{StaffRole, World};

use crate::consider;

/// How many of each role a club of this reputation wants.
fn wanted(rep: u16, role: StaffRole) -> usize {
    let big = rep >= 5000;
    match role {
        StaffRole::Manager | StaffRole::Assistant | StaffRole::GkCoach | StaffRole::FitnessCoach | StaffRole::HeadOfYouth => 1,
        StaffRole::Coach => {
            if big {
                4
            } else {
                2
            }
        }
        StaffRole::Scout => {
            if big {
                4
            } else {
                1
            }
        }
        StaffRole::Physio => {
            if big {
                2
            } else {
                1
            }
        }
        StaffRole::SportsScientist => usize::from(big),
        StaffRole::DirectorOfFootball => usize::from(rep >= 6500),
        StaffRole::Analyst => {
            if rep >= 7000 {
                3
            } else if rep >= 3500 {
                1
            } else {
                0
            }
        }
    }
}

const HIRED_ROLES: [StaffRole; 9] = [
    StaffRole::Analyst,
    StaffRole::Assistant,
    StaffRole::Coach,
    StaffRole::GkCoach,
    StaffRole::FitnessCoach,
    StaffRole::Scout,
    StaffRole::HeadOfYouth,
    StaffRole::SportsScientist,
    StaffRole::DirectorOfFootball,
];

/// Monthly: clubs with gaps interview the pool.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let month = (today.year() as u64) * 12 + u64::from(today.month());
    let clubs: Vec<ClubId> = w.clubs.ids().collect();
    for club in clubs {
        let rep = w.clubs[club].reputation;
        for role in HIRED_ROLES {
            let have = w.clubs[club].staff.iter().filter(|&&s| w.staff[s].role == role).count();
            if have >= wanted(rep, role) {
                continue;
            }
            let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(club.0), month, role as u64]);
            if let Some(s) = best_candidate(w, club, role, &mut rng) {
                hire(w, club, s);
            } else if rng.chance(0.35) {
                // Nobody suitable on the market: someone from outside the game
                // (a coach from lower levels, a graduate analyst) gets a chance.
                let s = fresh(w, club, role, &mut rng);
                hire(w, club, s);
            }
        }
    }
}

pub(crate) fn wanted_count(rep: u16, role: StaffRole) -> usize {
    wanted(rep, role)
}

fn best_candidate(w: &World, club: ClubId, role: StaffRole, rng: &mut Rng) -> Option<StaffId> {
    let rep = i32::from(w.clubs[club].reputation);
    let nation = w.clubs[club].nation;
    let manager = w.clubs[club].manager.get().map(|m| w.staff[m].person);
    w.staff
        .iter_enumerated()
        .filter(|(id, s)| s.role == role && !s.employed() && !s.retired && !w.intl.managers.contains(id))
        .filter(|(_, s)| {
            let need = match role {
                StaffRole::Assistant => 2,
                StaffRole::Coach | StaffRole::GkCoach | StaffRole::HeadOfYouth => 1,
                _ => 0,
            };
            crate::affairs::coaching_level(w, s.person) >= need
        })
        .filter(|(_, s)| i32::from(s.reputation) <= rep + 2000)
        .map(|(id, s)| {
            let skill = s.role_rating(role) / 20.0;
            let fame = f32::from(s.reputation) / 10_000.0;
            // Who you know: the manager's view of you, and whether you played here.
            let known = manager.map_or(0.0, |m| consider::affinity(w, m, s.person) + (consider::trust(w, m, s.person) - 0.5));
            let played_here = w.people[s.person].player.get().is_some_and(|p| w.history.spells.get(&p).is_some_and(|sp| sp.iter().any(|x| x.club == club)));
            let local = if w.people[s.person].nation == nation { 0.1 } else { 0.0 };
            let score = skill * 1.2 + fame * 0.4 + known * 0.5 + if played_here { 0.3 } else { 0.0 } + local + rng.normal() * 0.05;
            (id, score)
        })
        .filter(|(_, sc)| *sc > 0.35)
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(id, _)| id)
}

fn fresh(w: &mut World, club: ClubId, role: StaffRole, rng: &mut Rng) -> StaffId {
    let nation = w.clubs[club].nation;
    let (first, last) = crate::people::random_name(w, nation, rng);
    let dob = w.date.add_days(-(365 * rng.range_i32(26, 55)));
    let person = w.people.push(pw_world::Person {
        first,
        last,
        common: pw_world::NameId::NONE,
        dob,
        nation,
        nation2: Default::default(),
        hidden: crate::generate::hidden_random(rng),
        player: Default::default(),
        staff: Default::default(),
        mind: pw_world::MindKind::Ai,
    });
    let level = 5.0 + f32::from(w.clubs[club].reputation) / 1100.0;
    let mut attrs = pw_core::StaffAttrs::default();
    for a in pw_core::StaffAttr::ALL {
        attrs.set(a, rng.normal_ms(level, 2.5).round().clamp(1.0, 20.0) as u8);
    }
    for &a in role.key_attrs() {
        let v = attrs.get(a);
        attrs.set(a, v.saturating_add(2));
    }
    let id = w.staff.push(pw_world::Staff {
        person,
        role,
        club: ClubId::NONE,
        attrs,
        wage: 0,
        contract_end: w.date,
        reputation: (f32::from(w.clubs[club].reputation) * 0.3) as u16,
        philosophy: Default::default(),
        joined: w.date,
        record: Default::default(),
        retired: false,
    });
    w.people[person].staff = id;
    id
}

pub(crate) fn hire(w: &mut World, club: ClubId, s: StaffId) {
    let today = w.date;
    let revenue = crate::finance::season_revenue(w, club) as f64;
    let st = &mut w.staff[s];
    st.club = club;
    st.joined = today;
    st.contract_end = today.add_months(24);
    st.wage = (revenue * 0.0012 / 52.0 * f64::from(st.role_rating(st.role)) / 10.0).max(5.0) as i64; // no fixed minimum: it would exceed the whole budget of a small club
    let person = st.person;
    w.clubs[club].staff.push(s);
    w.events.push(today, Visibility::Public, EventKind::JoinedStaff { person, staff: s, club });
    crate::stafflife::open_job(w, s, club);
}

/// Staff grow into their jobs, plateau, and decline; the oldest retire. Growth is a chance,
/// not a certainty: professionalism and having a mentor raise it, and no one improves forever.
pub fn yearly_growth(w: &mut World) {
    crate::stafflife::yearly(w);
}
