//! Bringing a new person into the world (09 §1, S4, S20).
//!
//! Most of the time a human simply picks someone who already exists. When
//! they want someone new, the world creates them with its own generator —
//! the same code that produces every regen — and the human then inhabits
//! them. Talent is never chosen: potential is drawn from the club's own
//! academy distribution and stays hidden, like everyone's.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Date, NationId, PersonId, PlayerId, Pos};
use pw_sim::generate as gen_;
use pw_sim::people::{NewPlayer, intake_pa, spawn_player};
use pw_world::contract::ContractKind;
use pw_world::{Contract, TeamKind, World};

#[derive(Clone, Debug)]
pub struct NewPerson {
    pub first: String,
    pub last: String,
    pub nation: NationId,
    /// Club whose academy or squad they belong to; `NONE` = unattached.
    pub club: ClubId,
    /// Age in years at creation (any age the world supports).
    pub age: u8,
    pub pos: Pos,
    /// Seed for the draw (recorded, so creation is reproducible).
    pub salt: u64,
}

pub fn create_person(w: &mut World, np: NewPerson) -> (PersonId, PlayerId) {
    let today = w.date;
    let mut rng = Rng::keyed(&[w.seed, stream::WORLDGEN, 0xc4ea7e, np.salt]);
    let nation = if np.nation.is_some() {
        np.nation
    } else if np.club.is_some() {
        w.clubs[np.club].nation
    } else {
        NationId(0)
    };
    let (fac, rep) = if np.club.is_some() {
        let c = &w.clubs[np.club];
        (f32::from(c.facilities.youth), f32::from(c.reputation))
    } else {
        (6.0, 800.0)
    };
    let youth = f32::from(w.nations[nation].youth_rating);
    let pa = intake_pa(fac, youth, rep, &mut rng);
    let age = f32::from(np.age.clamp(8, 40));
    let ca = (pa * gen_::ca_share_at(age) * rng.normal_ms(1.0, 0.08)).clamp(15.0, pa);
    let dob = Date(today.0 - (age * 365.25) as i32 - rng.range_i32(0, 300));

    // Children start where every child in the world starts: at a local club.
    // Academies find them (or don't) through their own scouting and trials.
    let child = age < 16.0;
    let (team, contract) = if np.club.is_some() && !child {
        let kinds: &[TeamKind] = if age < 18.0 {
            &[TeamKind::U18, TeamKind::U19, TeamKind::U21, TeamKind::Reserve, TeamKind::First]
        } else if age < 21.0 {
            &[TeamKind::U21, TeamKind::Reserve, TeamKind::U19, TeamKind::First]
        } else {
            &[TeamKind::First]
        };
        let team = kinds.iter().find_map(|&k| w.club_team(np.club, k)).unwrap_or_else(|| w.clubs[np.club].first_team());
        let kind = if age < 17.0 { ContractKind::Youth } else { ContractKind::Professional };
        let years = if age < 18.0 { 2 } else { 3 };
        let contract = Contract { club: np.club, kind, wage: (60.0 + rep / 30.0 * if age < 18.0 { 1.0 } else { 3.0 }) as i64, start: today, end: today.add_months(12 * years), ..Default::default() };
        (team, contract)
    } else {
        (pw_core::TeamId::NONE, Contract::default())
    };
    let club = if child { ClubId::NONE } else { np.club };
    let p = spawn_player(w, NewPlayer { nation, dob, pos: np.pos, ca, pa: pa as u8, club, team, contract }, &mut rng);
    let person = w.players.cold[p].person;
    if !np.first.trim().is_empty() {
        w.people[person].first = w.names.intern(&np.first);
    }
    if !np.last.trim().is_empty() {
        w.people[person].last = w.names.intern(&np.last);
    }
    pw_sim::life::sync(w);
    if child {
        w.players.hot[p].status = pw_world::PlayerStatus::Amateur;
        w.youth.school.insert(person, Default::default());
        // Near the chosen club's town if one was given.
        let city = if np.club.is_some() { Some(w.clubs[np.club].city.clone()) } else { None };
        let local = city.and_then(|c| w.youth.local.ids().find(|&l| w.youth.local[l].city == c && w.youth.local[l].level == pw_world::youth::LocalLevel::Grassroots));
        match local {
            Some(l) => w.youth.join(p, l),
            None => pw_sim::youth::join_local_near(w, p, person),
        }
    } else if np.club.is_some() {
        w.knowledge.observe(np.club, p, 300, today);
    }
    (person, p)
}
