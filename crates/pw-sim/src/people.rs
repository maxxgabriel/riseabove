//! The flow of people: youth intakes (regens), retirements, players who
//! become coaches.

use pw_core::math::interp;
use pw_core::rng::{Rng, stream};
use pw_core::{Attr, ClubId, Date, Foot, Hidden, NationId, PersonId, PlayerId, Pos, StaffAttr, StaffAttrs};
use pw_world::contract::ContractKind;
use pw_world::event::{EventKind, Visibility};
use pw_world::player::Reputation;
use pw_world::staff::ManagerRecord;
use pw_world::{Contract, MindKind, NameId, Person, Philosophy, PlayerCold, PlayerHot, PlayerStatus, SquadStatus, Staff, StaffRole, TeamKind, World};

use crate::contracts;
use crate::generate as gen_;

pub fn random_name(w: &World, nation: NationId, rng: &mut Rng) -> (NameId, NameId) {
    let n = &w.nations[nation];
    let pick = |pool: &[NameId], rng: &mut Rng| if pool.is_empty() { NameId::NONE } else { pool[rng.index(pool.len())] };
    let first = pick(&n.first_names, rng);
    let last = pick(&n.last_names, rng);
    if first.is_none() && last.is_none() {
        // A nation with no imported people borrows from the whole world.
        let any = w.nations.iter().find(|n| !n.last_names.is_empty());
        if let Some(a) = any {
            return (pick(&a.first_names, rng), pick(&a.last_names, rng));
        }
    }
    (first, last)
}

/// Everything needed to create a player; the rest is derived.
pub struct NewPlayer {
    pub nation: NationId,
    pub dob: Date,
    pub pos: Pos,
    pub ca: f32,
    pub pa: u8,
    pub club: ClubId,
    pub team: pw_core::TeamId,
    pub contract: Contract,
}

pub fn spawn_player(w: &mut World, np: NewPlayer, rng: &mut Rng) -> PlayerId {
    let hidden = gen_::hidden_random(rng);
    let attrs = gen_::attrs_for(&w.data.weights, np.pos, np.ca, rng);
    let familiarity = gen_::familiarity_for(np.pos, hidden.f(Hidden::Versatility), rng);
    let (first, last) = random_name(w, np.nation, rng);
    let foot = match rng.f32() {
        x if x < 0.72 => Foot::Right,
        x if x < 0.93 => Foot::Left,
        _ => Foot::Either,
    };
    let weak = (rng.normal_ms(8.0, 3.0).round().clamp(1.0, 16.0)) as u8;
    let (left, right) = match foot {
        Foot::Right => (weak, 20),
        Foot::Left => (20, weak),
        Foot::Either => (18, 19),
    };
    let height = gen_::height_for(np.pos, rng);
    let player_id = w.players.hot.next_id();
    let person = w.people.push(Person {
        first,
        last,
        common: NameId::NONE,
        dob: np.dob,
        nation: np.nation,
        nation2: NationId::NONE,
        hidden,
        player: player_id,
        staff: Default::default(),
        mind: MindKind::Ai,
    });
    let mut cold = PlayerCold {
        person,
        attrs,
        pa: np.pa,
        ca: 0,
        familiarity,
        best_pos: np.pos,
        left_foot: left,
        right_foot: right,
        height,
        weight: (f32::from(height) * 0.42 + rng.normal() * 4.0).clamp(55.0, 105.0) as u8,
        traits: Default::default(),
        bio_offset: (rng.normal() * 8.0).round().clamp(-20.0, 20.0) as i8,
        pa_rerolled: false,
        wear: [0; pw_data::N_BODY_REGIONS],
        contract: np.contract,
        loan: None,
        value: 0,
        rep: Reputation::default(),
        status: SquadStatus::Youngster,
        shirt: 0,
        caps: 0,
        intl_goals: 0,
        joined: w.date,
        youth_club: np.club,
        injuries_career: 0,
        senior_apps: 0,
        senior_goals: 0,
        plan: Default::default(),
    };
    cold.refresh_ca(&w.data.weights);
    cold.pa = cold.pa.max(cold.ca);
    let hot = PlayerHot {
        club: np.club,
        team: np.team,
        status: if np.club.is_some() { PlayerStatus::Active } else { PlayerStatus::FreeAgent },
        condition: 95,
        sharpness: 50,
        fitness: 75,
        ..PlayerHot::default()
    };
    let id = w.players.push(hot, cold);
    debug_assert_eq!(id, player_id);
    if np.team.is_some() {
        w.teams[np.team].squad.push(id);
    }
    if np.club.is_some() {
        w.history.start_spell(id, np.club, w.date, false, 0);
    }
    id
}

pub fn daily(w: &mut World) {
    let today = w.date;
    for n in w.nations.ids() {
        let s = &w.nations[n].season;
        if s.year != 0 && today == s.start.add_days(210) {
            youth_intake(w, n);
        }
    }
    if today.month() == 7 && today.day() == 1 {
        retirements(w);
    }
}

/// Annual academy intake (04 §6, 07 §6).
fn youth_intake(w: &mut World, n: NationId) {
    let today = w.date;
    let youth_rating = f32::from(w.nations[n].youth_rating);
    let t = w.data.tuning.squad.clone();
    let clubs: Vec<ClubId> = w.clubs.iter_enumerated().filter(|(_, c)| c.nation == n).map(|(id, _)| id).collect();
    for club in clubs {
        let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, u64::from(club.0), today.year() as u64]);
        let c = &w.clubs[club];
        let academy = f32::from(c.facilities.academy + c.facilities.youth) / 40.0;
        let count = f32::from(t.youth_intake_min) + academy * f32::from(t.youth_intake_max - t.youth_intake_min) + rng.normal() * 1.2;
        let count = count.round().clamp(1.0, f32::from(t.youth_intake_max) + 2.0) as usize;
        let team = [TeamKind::U18, TeamKind::U19, TeamKind::U21, TeamKind::Reserve, TeamKind::First].iter().find_map(|&k| w.club_team(club, k));
        let Some(team) = team else { continue };
        let rep = f32::from(c.reputation);
        let youth_fac = f32::from(c.facilities.youth);
        for _ in 0..count {
            let foreign = rng.chance(0.08);
            let nation = if foreign { pw_core::NationId(rng.below(w.nations.len() as u32)) } else { n };
            let age_days = rng.range_i32(15 * 365 + 30, 16 * 365 + 200);
            let dob = today.add_days(-age_days);
            let mean_pa = 58.0 + 2.2 * youth_fac + 1.6 * youth_rating + rep / 250.0;
            let mut pa = rng.normal_ms(mean_pa, 21.0);
            if rng.chance(0.004) {
                pa += rng.range_f32(25.0, 60.0);
            }
            let pa = pa.clamp(35.0, 200.0);
            let age = age_days as f32 / 365.25;
            let ca = (pa * gen_::ca_share_at(age) * rng.normal_ms(1.0, 0.08)).clamp(15.0, pa);
            let contract = Contract {
                club,
                kind: ContractKind::Youth,
                wage: (80.0 + rep / 40.0) as i64,
                start: today,
                end: dob.add_months(12 * 18 + 12),
                ..Default::default()
            };
            let pos = gen_::random_position(&mut rng);
            spawn_player(w, NewPlayer { nation, dob, pos, ca, pa: pa as u8, club, team, contract }, &mut rng);
        }
        w.events.push(today, Visibility::Club(club), EventKind::YouthIntake { club, count: count as u8 });
    }
}

fn retire_chance(age: u32, ca: u8, free_agent: bool, keeper: bool, long_injury: bool) -> f32 {
    let base = interp(&[(31.0, 0.0), (32.0, 0.03), (33.0, 0.08), (34.0, 0.18), (35.0, 0.32), (36.0, 0.5), (37.0, 0.68), (38.0, 0.8), (40.0, 0.95)], age as f32);
    let mut p = base;
    if ca >= 140 {
        p *= 0.6;
    }
    if free_agent {
        p = (p * 2.0).max(if age >= 30 { 0.25 } else { 0.0 });
    }
    if keeper {
        p *= 0.7;
    }
    if long_injury {
        p *= 1.5;
    }
    p.min(0.98)
}

/// Season-end retirements for AI minds. External minds only retire by choice.
fn retirements(w: &mut World) {
    let today = w.date;
    let mut retiring = Vec::new();
    for p in w.players.ids() {
        let h = &w.players.hot[p];
        if h.status == PlayerStatus::Retired {
            continue;
        }
        let c = &w.players.cold[p];
        let person = &w.people[c.person];
        if person.mind == MindKind::External {
            continue;
        }
        let age = person.age(today);
        if age < 30 && h.status != PlayerStatus::FreeAgent {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::RETIREMENT, u64::from(p.0), today.year() as u64]);
        let chance = retire_chance(age, c.ca, h.status == PlayerStatus::FreeAgent, c.best_pos == Pos::GK, h.injury_days > 120);
        if rng.chance(chance) {
            retiring.push(p);
        }
    }
    for p in retiring {
        retire(w, p);
    }
}

pub fn retire(w: &mut World, p: PlayerId) {
    let today = w.date;
    if w.players.hot[p].club.is_some() {
        if w.players.cold[p].loan.is_some() {
            crate::market::end_loan(w, p);
        }
        contracts::release(w, p);
    }
    w.players.hot[p].status = PlayerStatus::Retired;
    w.knowledge.clear_player(p);
    let person = w.players.cold[p].person;
    w.events.push(today, Visibility::Public, EventKind::Retired { person });
    maybe_become_coach(w, p, person);
}

/// Some retiring players take their badges and join the staff pool (07 §12).
fn maybe_become_coach(w: &mut World, p: PlayerId, person: PersonId) {
    let c = &w.players.cold[p];
    let leadership = c.attrs.get(Attr::Leadership);
    let det = c.attrs.get(Attr::Determination);
    let age = w.people[person].age(w.date);
    let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(p.0)]);
    if age > 40 || leadership + det < 24.0 || !rng.chance(0.35) {
        return;
    }
    let a = |x: Attr| c.attrs.get(x);
    let mut attrs = StaffAttrs::default();
    let noisy = |v: f32, rng: &mut Rng| (v * 0.75 + rng.normal() * 2.0).round().clamp(1.0, 20.0) as u8;
    attrs.set(StaffAttr::Attacking, noisy((a(Attr::Finishing) + a(Attr::OffTheBall)) / 2.0, &mut rng));
    attrs.set(StaffAttr::Defending, noisy((a(Attr::Marking) + a(Attr::Positioning)) / 2.0, &mut rng));
    attrs.set(StaffAttr::Technical, noisy(a(Attr::Technique), &mut rng));
    attrs.set(StaffAttr::Tactical, noisy((a(Attr::Decisions) + a(Attr::Vision)) / 2.0, &mut rng));
    attrs.set(StaffAttr::Mental, noisy(det, &mut rng));
    attrs.set(StaffAttr::Fitness, noisy(a(Attr::NaturalFitness), &mut rng));
    attrs.set(StaffAttr::Goalkeeping, noisy((a(Attr::Reflexes) + a(Attr::Handling)) / 2.0, &mut rng));
    attrs.set(StaffAttr::ManManagement, noisy(leadership, &mut rng));
    attrs.set(StaffAttr::Motivating, noisy((leadership + det) / 2.0, &mut rng));
    attrs.set(StaffAttr::TacticalKnowledge, noisy((a(Attr::Decisions) + a(Attr::Anticipation)) / 2.0, &mut rng));
    attrs.set(StaffAttr::JudgingAbility, noisy(a(Attr::Anticipation), &mut rng));
    attrs.set(StaffAttr::JudgingPotential, noisy(a(Attr::Vision), &mut rng));
    attrs.set(StaffAttr::Youngsters, noisy(a(Attr::Teamwork), &mut rng));
    let rep = c.rep.current / 3;
    let staff = w.staff.push(Staff {
        person,
        role: if leadership >= 15.0 { StaffRole::Manager } else { StaffRole::Coach },
        club: ClubId::NONE,
        attrs,
        wage: 0,
        contract_end: w.date,
        reputation: rep,
        philosophy: Philosophy::default(),
        joined: w.date,
        record: ManagerRecord::default(),
        retired: false,
    });
    w.people[person].staff = staff;
}
