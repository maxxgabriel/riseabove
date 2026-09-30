//! The flow of people: youth intakes (regens), retirements, players who
//! become coaches.

use pw_core::rng::{Rng, stream};
use pw_core::{Attr, ClubId, Date, Foot, Hidden, NationId, PersonId, PlayerId, Pos, StaffAttr, StaffAttrs, StaffId, TeamId};
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
    /// Why this player exists (recorded for population metrics).
    pub source: pw_world::player::PlayerSource,
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
    let person =
        w.people.push(Person { first, last, common: NameId::NONE, dob: np.dob, nation: np.nation, nation2: NationId::NONE, hidden, player: player_id, staff: Default::default(), mind: MindKind::Ai });
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
    let id = w.players.push(hot, cold, pw_world::player::Origin { source: np.source, date: w.date });
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
            crate::youth::academy_intake(w, n);
        }
    }
    if today.month() == 7 && today.day() == 1 {
        retirements(w);
        age_out(w);
    }
    if today.day() == 1 {
        trim_squads(w);
    }
}

/// Monthly: a first team over the squad limit sheds the players the club values least, from its own reading of them (never their hidden
/// ability): a young one moves down to a side that will take him, anyone else is let go. Nothing else stops squads growing without bound.
fn trim_squads(w: &mut World) {
    let max = usize::from(w.data.tuning.squad.first_team_max);
    for club in w.clubs.ids().collect::<Vec<_>>() {
        let first = w.clubs[club].first_team();
        while w.teams[first].squad.len() > max {
            let worst = w.teams[first]
                .squad
                .iter()
                .copied()
                .filter(|&p| w.people[w.players.cold[p].person].mind == MindKind::Ai && w.players.cold[p].loan.is_none() && w.players.hot[p].club == club)
                .map(|p| {
                    let (ca, _, pa, _) = crate::scouting::view(w, club, p);
                    let growth = if w.age(p) < 23 { 0.4 * (pa - ca).max(0.0) } else { 0.0 };
                    (p, ca + growth)
                })
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let Some((p, _)) = worst else { break };
            let age = w.age(p);
            let lower = [TeamKind::Reserve, TeamKind::U21, TeamKind::U19, TeamKind::U18].iter().find_map(|&k| w.club_team(club, k).filter(|_| w.teams[first].kind != k && TeamKind::max_age(k).is_none_or(|m| age <= m)));
            match lower {
                Some(t) => {
                    w.teams[first].squad.retain(|&x| x != p);
                    w.teams[t].squad.push(p);
                    w.players.hot[p].team = t;
                }
                None => contracts::release(w, p),
            }
        }
    }
}

/// July 1: players who have outgrown the side they play in step up or are let go. The club decides from its own reading of him (never
/// his hidden ability): a player good enough for the club's standard joins the first team (or the reserves if that is full), everyone
/// else is released and drifts into the amateur game. Without this, youth and reserve sides fill with adults nobody wants and the
/// world's population grows without bound.
///
/// Graduates step up only into the room the club plans for (`first_team_target`), best first. Filling the first team to its hard limit
/// every summer made the monthly trim (`trim_squads`) push out whoever the club rated least, and with a growth premium on the young that
/// was the senior professionals: dozens of players in their twenties at second-tier standard were released into the amateur game each
/// year, first teams grew years younger, and every price read from age and potential rose with them.
fn age_out(w: &mut World) {
    let mut moves: Vec<(PlayerId, TeamId)> = Vec::new();
    let mut releases: Vec<PlayerId> = Vec::new();
    // Graduates by club, with the club's reading of each: (player, their side, standard met, score).
    let mut graduates: pw_world::FxHashMap<ClubId, Vec<(PlayerId, TeamId, bool, f32)>> = Default::default();
    for t in w.teams.ids() {
        let kind = w.teams[t].kind;
        if kind == TeamKind::First {
            continue;
        }
        let club = w.teams[t].club;
        let bar = crate::market::ideal_ca(w.clubs[club].reputation) - 12.0;
        for &p in &w.teams[t].squad {
            let h = &w.players.hot[p];
            let who = w.players.cold[p].person;
            if h.status != PlayerStatus::Active || w.people[who].mind != MindKind::Ai || w.players.cold[p].loan.is_some() {
                continue;
            }
            let age = w.age(p);
            let outgrown = match kind.max_age() {
                Some(max) => age > max,
                None => age >= 23,
            };
            if !outgrown {
                continue;
            }
            let (ca, _, pa, _) = crate::scouting::view(w, club, p);
            let good = ca >= bar - 8.0 || (age <= 23 && pa >= bar + 4.0);
            let growth = if age < 23 { 0.4 * (pa - ca).max(0.0) } else { 0.0 };
            graduates.entry(club).or_default().push((p, t, good, ca + growth));
        }
    }
    let mut clubs: Vec<ClubId> = graduates.keys().copied().collect();
    clubs.sort();
    let target = usize::from(w.data.tuning.squad.first_team_target);
    for club in clubs {
        let mut list = graduates.remove(&club).unwrap_or_default();
        list.sort_by(|a, b| b.3.total_cmp(&a.3).then(a.0.cmp(&b.0)));
        let first = w.clubs[club].first_team();
        let mut room = target.saturating_sub(w.teams[first].squad.len());
        for (p, t, good, _) in list {
            if !good {
                releases.push(p);
            } else if room > 0 {
                room -= 1;
                moves.push((p, first));
            } else if let Some(reserve) = w.club_team(club, TeamKind::Reserve).filter(|&r| r != t) {
                moves.push((p, reserve));
            } else {
                releases.push(p);
            }
        }
    }
    for (p, to) in moves {
        let from = w.players.hot[p].team;
        if from.is_some() {
            w.teams[from].squad.retain(|&x| x != p);
        }
        w.teams[to].squad.push(p);
        w.players.hot[p].team = to;
    }
    for p in releases {
        contracts::release(w, p);
    }
}

/// Potential of a youngster coming through a club's academy: the world's own
/// distribution, used for every intake and for anyone newly created into the
/// world (a human's new person is drawn from exactly this).
pub fn intake_pa(youth_facilities: f32, nation_youth_rating: f32, club_rep: f32, rng: &mut Rng) -> f32 {
    let mean_pa = 58.0 + 2.2 * youth_facilities + 1.6 * nation_youth_rating + club_rep / 250.0;
    let mut pa = rng.normal_ms(mean_pa, 21.0);
    if rng.chance(0.004) {
        pa += rng.range_f32(25.0, 60.0);
    }
    pa.clamp(35.0, 200.0)
}

/// Season's end: every AI-minded player weighs whether to carry on
/// (`mind::retirement_choice`). An external mind retires when its human
/// decides to, through the same `Retire` intent — the choice is the only
/// difference, never the rules.
fn retirements(w: &mut World) {
    let today = w.date;
    let mut retiring = Vec::new();
    for p in w.players.ids() {
        let h = &w.players.hot[p];
        if h.status == PlayerStatus::Retired {
            continue;
        }
        let person = &w.people[w.players.cold[p].person];
        if person.mind != MindKind::Ai {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::RETIREMENT, u64::from(p.0), today.year() as u64]);
        if crate::mind::retirement_choice(w, p, &mut rng) {
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
    if let Some(a) = w.agents.of_player.remove(&p) {
        w.agents.list[a.agent].clients.retain(|&x| x != p);
    }
    w.market.requests.remove(&p);
    w.market.listed.remove(&p);
}

/// A person (retired player or anyone) enters the staff job market with
/// attributes grown from their playing career. Clubs hire from this pool.
pub fn enter_staff_pool(w: &mut World, person: PersonId, role: StaffRole) -> Option<StaffId> {
    if w.people[person].staff.is_some() {
        let s = w.people[person].staff;
        w.staff[s].role = role;
        w.staff[s].retired = false;
        return Some(s);
    }
    let p = w.people[person].player;
    if p.is_none() {
        return None;
    }
    Some(maybe_become_coach(w, p, person, role))
}

/// Staff attributes grown from a playing career (07 §12).
fn maybe_become_coach(w: &mut World, p: PlayerId, person: PersonId, role: StaffRole) -> StaffId {
    let c = &w.players.cold[p];
    let leadership = c.attrs.get(Attr::Leadership);
    let det = c.attrs.get(Attr::Determination);
    let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(p.0)]);
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
    let _ = (leadership, det);
    let staff = w.staff.push(Staff {
        person,
        role,
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
    staff
}
