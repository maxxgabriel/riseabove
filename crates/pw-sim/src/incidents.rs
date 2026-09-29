//! The stochastic incident engine (S9, S11).
//!
//! State makes an incident plausible; a roll on the incidents stream decides
//! whether it happens now. For every eligible subject the hazard is
//!
//! ```text
//! p = base · exp(Σ wᵢ · fᵢ)
//! ```
//!
//! where each pressure fᵢ ∈ [0, 1] is read from the world (grievances,
//! position rivalry, temperament, morale, public criticism, training load,
//! dressing-room tension, unresolved rows, finances, weather, fame…). The
//! largest contributions are stored on the incident and written as causes,
//! so `why` shows what made it likely.
//!
//! An incident then: gets witnesses from where it happened; becomes an
//! information item that travels (unless it was public); has immediate
//! consequences through ordinary state (memories, tension, availability,
//! money, fixtures, finances); and reaches whoever has authority, whose
//! response (`responses`) depends on who they are. Unresolved tension feeds
//! the next evaluation — incidents have follow-ups because state does.

use pw_core::rng::{period, stream};
use pw_core::{ClubId, FixtureId, Hidden, NationId, PersonId, PlayerId, StaffId, TeamId};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::incident::{Exposure, Incident, IncidentDef, IncidentKind, Location, Pressure, def};
use pw_world::info::InfoKind;
use pw_world::{FanReason, LifeEventKind, MemoryKind, PlayerStatus, StaffRole, World};
use smallvec::SmallVec;

use crate::consider;

/// Who and what an evaluation is about.
#[derive(Clone, Copy, Default)]
pub struct Ctx {
    pub a: PersonId,
    pub b: PersonId,
    pub pa: PlayerId,
    pub pb: PlayerId,
    pub club: ClubId,
    pub nation: NationId,
    pub staff: StaffId,
    pub fixture: FixtureId,
}

fn club_manager(w: &World, club: ClubId) -> PersonId {
    if club.is_none() {
        return PersonId::NONE;
    }
    w.clubs[club].manager.get().map_or(PersonId::NONE, |m| w.staff[m].person)
}

fn first_squad(w: &World, club: ClubId) -> Vec<PlayerId> {
    let t = w.clubs[club].first_team();
    if t.is_none() {
        return Vec::new();
    }
    w.teams[t].squad.iter().copied().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect()
}

/// How wintry it is for football in a nation this month, 0–1.
pub fn winter(w: &World, n: NationId) -> f32 {
    if n.is_none() {
        return 0.0;
    }
    let m = w.date.month();
    let key = w.data.calendars.get(usize::from(w.nations[n].calendar)).map_or("autumn_spring", |c| c.key.as_str());
    if key.starts_with("autumn") {
        match m {
            12 | 1 | 2 => 1.0,
            11 | 3 => 0.5,
            _ => 0.0,
        }
    } else if key.ends_with("nordic") {
        match m {
            11 | 3 | 4 => 0.6,
            12 | 1 | 2 => 1.0,
            _ => 0.0,
        }
    } else {
        match m {
            6..=8 => 0.7,
            _ => 0.0,
        }
    }
}

/// One pressure, read from state, in [0, 1].
pub fn pressure(w: &World, pr: Pressure, c: &Ctx) -> f32 {
    let hid = |p: PersonId, h: Hidden| if p.is_some() { consider::hid(w, p, h) / 20.0 } else { 0.5 };
    let v = match pr {
        Pressure::Resentment => {
            if c.a.is_some() && c.b.is_some() {
                (consider::grievance(w, c.a, c.b) + consider::grievance(w, c.b, c.a)) * 0.5 + consider::memory(w, c.a, c.b, MemoryKind::Fought) * 0.5
            } else if c.a.is_some() {
                let m = club_manager(w, c.club);
                if m.is_some() { consider::grievance(w, c.a, m) } else { 0.0 }
            } else {
                0.0
            }
        }
        Pressure::PositionRivalry => {
            if c.pa.is_some() && c.pb.is_some() {
                let (x, y) = (&w.players.cold[c.pa], &w.players.cold[c.pb]);
                let same = x.best_pos.group() == y.best_pos.group();
                let close = (crate::market::public_view(w, c.pa).0 - crate::market::public_view(w, c.pb).0).abs() <= 10.0;
                if same && close { 0.4 + consider::minutes_grievance(w, c.pa).max(consider::minutes_grievance(w, c.pb)) * 0.6 } else { 0.0 }
            } else {
                0.0
            }
        }
        Pressure::Temper => (1.0 - hid(c.a, Hidden::Temperament)).max(if c.b.is_some() { 1.0 - hid(c.b, Hidden::Temperament) } else { 0.0 }),
        Pressure::LowMorale => {
            if c.pa.is_some() {
                ((50.0 - f32::from(w.players.hot[c.pa].morale)) / 50.0).max(0.0)
            } else {
                0.0
            }
        }
        Pressure::PublicCriticism => {
            let recent = w.media.stories.iter().rev().take(400).any(|s| s.person == c.a && s.tone <= -40 && s.date.days_until(w.date) <= 21);
            if recent { 1.0 } else { 0.0 }
        }
        Pressure::TrainingLoad => {
            if c.pa.is_some() {
                f32::from(w.players.hot[c.pa].fatigue) / 100.0
            } else {
                0.0
            }
        }
        Pressure::RoomTension => w.rooms.clubs.get(&c.club).map_or(0.2, |r| 1.0 - f32::from(r.harmony) / 100.0),
        Pressure::Unresolved => {
            if c.a.is_some() && c.b.is_some() {
                f32::from(w.incidents.tension(c.a, c.b)) / 100.0
            } else if c.a.is_some() {
                let m = club_manager(w, c.club);
                f32::from(w.incidents.tension(c.a, m)) / 100.0
            } else {
                0.0
            }
        }
        Pressure::Unprofessional => 1.0 - hid(c.a, Hidden::Professionalism),
        Pressure::Nightlife => w.lives.get(c.a).map_or(0.0, |l| f32::from(l.routine.nightlife) / 10.0),
        Pressure::Stress => w.lives.get(c.a).map_or(0.0, |l| f32::from(l.stress) / 100.0),
        Pressure::NewAbroad => {
            if c.pa.is_some() {
                let pc = &w.players.cold[c.pa];
                let club = w.players.hot[c.pa].club;
                let abroad = club.is_some() && w.people[pc.person].nation != w.clubs[club].nation;
                if abroad && pc.joined.days_until(w.date) < 120 { 1.0 } else { 0.0 }
            } else {
                0.0
            }
        }
        Pressure::Winter => winter(w, c.nation),
        Pressure::ClubFinances => {
            if c.club.is_none() {
                0.0
            } else {
                let red = w.governance.get(&c.club).map_or(0, |g| g.red_months);
                let broke = if w.clubs[c.club].finance.balance < 0 { 0.4 } else { 0.0 };
                (f32::from(red) / 12.0 + broke).min(1.0)
            }
        }
        Pressure::OwnerMeddling => w.governance.get(&c.club).map_or(0.0, |g| f32::from(g.owner.meddling) / 100.0),
        Pressure::PoorResults => {
            if c.club.is_none() {
                0.0
            } else {
                ((60.0 - f32::from(w.clubs[c.club].board.satisfaction)) / 60.0).max(0.0)
            }
        }
        Pressure::Fame => f32::from(w.renown.of(c.a).fame) / 10_000.0,
        Pressure::ManagerPressure => {
            if c.club.is_none() {
                0.0
            } else {
                f32::from(w.clubs[c.club].board.warnings) / 3.0
            }
        }
        Pressure::StaffDiscontent => {
            let m = club_manager(w, c.club);
            if m.is_some() && c.a.is_some() && m != c.a { 1.0 - consider::trust(w, c.a, m) } else { 0.0 }
        }
        Pressure::Overqualified => {
            if c.staff.is_some() && c.club.is_some() {
                ((f32::from(w.staff[c.staff].reputation) - f32::from(w.clubs[c.club].reputation) * 0.6) / 3000.0).clamp(0.0, 1.0)
            } else {
                0.0
            }
        }
        Pressure::Household => w.lives.get(c.a).map_or(0.0, |l| {
            let young = l.household.children > 0 && l.household.youngest_born.days_until(w.date) < 5 * 365;
            (f32::from(l.household.children.min(3)) / 3.0) * if young { 1.0 } else { 0.4 }
        }),
        Pressure::Studying => {
            let adult = w.affairs.of(c.a).is_some_and(|a| a.studying.is_some());
            let school = w.youth.school.contains_key(&c.a) && consider::age(w, c.a) >= 15.0;
            let exam_season = matches!(w.date.month(), 5 | 6 | 12 | 1);
            if (adult || school) && exam_season {
                1.0
            } else if adult || school {
                0.2
            } else {
                0.0
            }
        }
        Pressure::WeakEconomy => {
            let g = w.economy.nations.get(&c.nation).map_or(0.02, |e| e.growth);
            ((0.01 - g) * 25.0).clamp(0.0, 1.0)
        }
        Pressure::OldFacilities => {
            if c.club.is_none() {
                0.0
            } else {
                (1.0 - f32::from(w.clubs[c.club].facilities.training) / 20.0).clamp(0.0, 1.0)
            }
        }
        Pressure::Congestion => {
            let t = if c.pa.is_some() { w.players.hot[c.pa].team } else { TeamId::NONE };
            let team = if t.is_some() {
                t
            } else if c.club.is_some() {
                w.clubs[c.club].first_team()
            } else {
                TeamId::NONE
            };
            if team.is_none() {
                0.0
            } else {
                let n = w.fixtures.of_team_between(team, w.date.add_days(-10), w.date).count();
                (n as f32 / 4.0).min(1.0)
            }
        }
        Pressure::StrainedRelationship => w.lives.get(c.a).and_then(|l| l.household.partner).map_or(0.0, |p| ((60.0 - f32::from(p.bond)) / 60.0).max(0.0)),
        Pressure::RecentMove => w.lives.get(c.a).map_or(0.0, |l| if l.home_since.days_until(w.date) < 90 { 1.0 } else { 0.0 }),
        Pressure::Wealth => w.lives.get(c.a).map_or(0.0, |l| ((l.finances.savings as f32).max(1.0).log10() - 4.0).clamp(0.0, 3.0) / 3.0),
        Pressure::FanAnger => {
            if c.club.is_none() {
                0.0
            } else {
                ((45.0 - f32::from(w.clubs[c.club].fan_mood)) / 45.0).max(0.0)
            }
        }
    };
    v.clamp(0.0, 1.0)
}

/// Hazard for one subject: probability and the contributions behind it.
fn hazard(w: &World, d: &IncidentDef, c: &Ctx) -> (f32, SmallVec<[(Pressure, u8); 4]>) {
    let mut sum = 0.0f32;
    let mut parts: SmallVec<[(Pressure, f32); 8]> = SmallVec::new();
    for &(pr, weight) in d.pressures {
        let f = pressure(w, pr, c);
        let contrib = weight * f;
        sum += contrib;
        if contrib > 0.0 {
            parts.push((pr, contrib));
        }
    }
    parts.sort_by(|x, y| y.1.total_cmp(&x.1));
    let top: SmallVec<[(Pressure, u8); 4]> = parts.iter().filter(|x| x.1 >= 0.15).take(4).map(|&(p, v)| (p, (v * 100.0).min(255.0) as u8)).collect();
    ((d.hazard * pw_core::math::exp(sum)).min(0.5), top)
}

/// Roll for an incident; trigger it if it happens.
fn consider_incident(w: &mut World, kind: IncidentKind, c: Ctx, keys: &[u64]) -> Option<u32> {
    let d = def(kind);
    if d.hazard <= 0.0 {
        return None;
    }
    let (p, top) = hazard(w, &d, &c);
    let mut k: SmallVec<[u64; 6]> = SmallVec::new();
    k.push(kind as u64);
    k.extend_from_slice(keys);
    if w.roll(stream::INCIDENTS, &k) >= p {
        return None;
    }
    Some(trigger(w, &d, c, top, None))
}

/// Who saw it, from where it happened.
fn witnesses(w: &World, loc: Location, c: &Ctx, key: u64) -> SmallVec<[PersonId; 6]> {
    let mut v: SmallVec<[PersonId; 6]> = SmallVec::new();
    let mut rng = w.rng(stream::INCIDENTS, &[key, 0x517]);
    match loc {
        Location::TrainingGround | Location::DressingRoom | Location::Travel => {
            if c.club.is_some() {
                let squad = first_squad(w, c.club);
                let others: Vec<PersonId> = squad.iter().map(|&p| w.players.cold[p].person).filter(|&x| x != c.a && x != c.b).collect();
                for _ in 0..4 {
                    if others.is_empty() {
                        break;
                    }
                    let x = others[rng.index(others.len())];
                    if !v.contains(&x) {
                        v.push(x);
                    }
                }
                if loc == Location::TrainingGround {
                    let coaches: Vec<PersonId> =
                        w.clubs[c.club].staff.iter().filter(|&&s| matches!(w.staff[s].role, StaffRole::Assistant | StaffRole::Coach | StaffRole::FitnessCoach)).map(|&s| w.staff[s].person).collect();
                    if let Some(&x) = coaches.first() {
                        v.push(x);
                    }
                }
            }
        }
        Location::Home => {
            if let Some(pt) = w.lives.get(c.a).and_then(|l| l.household.partner)
                && pt.person != c.b
            {
                v.push(pt.person);
            }
        }
        Location::Office => {
            if c.club.is_some() {
                for &s in w.clubs[c.club].staff.iter().filter(|&&s| matches!(w.staff[s].role, StaffRole::DirectorOfFootball | StaffRole::Assistant)).take(2) {
                    v.push(w.staff[s].person);
                }
            }
        }
        Location::Stadium | Location::City | Location::Nationwide => {}
    }
    v
}

/// Record an incident, make it known, apply its consequences.
pub fn trigger(w: &mut World, d: &IncidentDef, c: Ctx, top: SmallVec<[(Pressure, u8); 4]>, follows: Option<u32>) -> u32 {
    let today = w.date;
    let id = w.incidents.list.len() as u32;
    let key = pw_core::rng::hash_key(&[w.seed, u64::from(id), d.kind as u64]);
    let pressure_sum: f32 = top.iter().map(|x| f32::from(x.1) / 100.0).sum();
    let noise = pw_core::rng::noise(&[w.seed, stream::INCIDENTS, u64::from(id), 0x5e7]);
    let severity = (30.0 + pressure_sum * 18.0 + noise * 20.0).clamp(5.0, 100.0) as u8;
    let mut parties: SmallVec<[PersonId; 3]> = SmallVec::new();
    for p in [c.a, c.b] {
        if p.is_some() {
            parties.push(p);
        }
    }
    let mut players: SmallVec<[PlayerId; 3]> = SmallVec::new();
    for p in [c.pa, c.pb] {
        if p.is_some() {
            players.push(p);
        }
    }
    let wit = witnesses(w, d.location, &c, key);
    let vis = match d.exposure {
        Exposure::Public => Visibility::Public,
        Exposure::Club if c.club.is_some() => Visibility::Club(c.club),
        _ if c.a.is_some() && c.b.is_some() => Visibility::Between(c.a, c.b),
        _ if c.a.is_some() => Visibility::Person(c.a),
        _ => Visibility::Public,
    };
    let mut causes = Causes::new();
    for &(pr, level) in top.iter().take(3) {
        causes.push(Cause::Fact(Fact::Pressure { pressure: pr, level }));
    }
    if let Some(prev) = follows.and_then(|f| w.incidents.get(f)) {
        let e = prev.event;
        if causes.len() < 3 {
            causes.push(Cause::Event(e));
        }
    }
    let ev = w.events.push_caused(today, vis, EventKind::Incident { incident: id, kind: d.kind }, causes);
    w.incidents.list.push(Incident {
        id,
        kind: d.kind,
        date: today,
        club: c.club,
        nation: c.nation,
        location: d.location,
        parties: parties.clone(),
        players,
        staff: c.staff,
        fixture: c.fixture,
        witnesses: wit.clone(),
        severity,
        pressures: top,
        event: ev,
        info: u32::MAX,
        responses: SmallVec::new(),
        hushed: false,
        resolved: false,
        led_to: None,
        follows,
    });
    if let Some(f) = follows
        && let Some(prev) = w.incidents.list.get_mut(f as usize)
    {
        prev.led_to = Some(id);
    }
    // Information: private and club matters travel from those who know.
    if d.exposure != Exposure::Public {
        let sens = ((u16::from(d.sensitivity) + u16::from(severity)) / 2) as u8;
        let info = crate::grapevine::witness(w, InfoKind::Incident { incident: id }, ev, sens, &parties, &wit);
        w.incidents.list[id as usize].info = info;
    }
    consequences(w, id);
    // Anyone with authority who already knows responds.
    let mut knowers: Vec<PersonId> = parties.to_vec();
    knowers.extend(wit.iter().copied());
    for k in knowers {
        crate::responses::on_learn(w, k, id);
    }
    id
}

// ---------------------------------------------------------------------------
// Evaluation passes
// ---------------------------------------------------------------------------

/// Daily: fixtures (postponements, travel), births, deferred decisions,
/// investigations, and the end of leave and suspensions.
pub fn daily(w: &mut World) {
    let today = w.date;
    // Fixtures tomorrow may be postponed; today's away sides may be delayed.
    let tomorrow: Vec<FixtureId> = w.fixtures.on(today.add_days(1)).to_vec();
    for f in tomorrow {
        let fx = w.fixtures.get(f).clone();
        if fx.score.is_some() || fx.decisive {
            continue;
        }
        let club = w.teams[fx.home].club;
        let nation = w.clubs[club].nation;
        let weather = w.incidents.national_active(nation, IncidentKind::SevereWeather, today).is_some();
        let pitch = w.incidents.list.iter().rev().take(200).any(|i| i.kind == IncidentKind::PitchDamage && i.club == club && i.date.days_until(today) < 5);
        let c = Ctx { club, nation, fixture: f, ..Ctx::default() };
        let d = def(IncidentKind::Postponement);
        let (mut p, top) = hazard(w, &d, &c);
        if weather {
            p = (p * 25.0).min(0.35);
        }
        // Where the monsoon breaks over a home ground, matches are called off more often.
        if w.ext.ecosystem.is_configured() {
            let r = w.ext.ecosystem.region_of_club(club);
            if r.is_some() && w.ext.ecosystem.regions[r].climate == pw_world::ecosystem::Climate::HeavyMonsoon && (6..=9).contains(&today.month()) {
                p = (p * 6.0).min(0.3);
            }
        }
        if pitch {
            p = (p * 40.0).min(0.6);
        }
        if w.roll(stream::INCIDENTS, &[IncidentKind::Postponement as u64, fx.uid]) < p {
            let prev = if weather { w.incidents.national_active(nation, IncidentKind::SevereWeather, today) } else { None };
            trigger(w, &d, c, top, prev);
        }
    }
    let todays: Vec<FixtureId> = w.fixtures.on(today).to_vec();
    for f in todays {
        let fx = w.fixtures.get(f).clone();
        if fx.score.is_some() {
            continue;
        }
        let club = w.teams[fx.away].club;
        let nation = w.clubs[w.teams[fx.home].club].nation;
        let disrupted = w.incidents.national_active(nation, IncidentKind::TransportDisruption, today);
        let c = Ctx { club, nation, fixture: f, ..Ctx::default() };
        let d = def(IncidentKind::TravelDelay);
        let (mut p, top) = hazard(w, &d, &c);
        if disrupted.is_some() {
            p = (p * 15.0).min(0.4);
        }
        if w.roll(stream::INCIDENTS, &[IncidentKind::TravelDelay as u64, fx.uid]) < p {
            trigger(w, &d, c, top, disrupted);
        }
    }
    births(w);
    crate::responses::deferred(w);
    investigations(w);
    w.incidents.away.retain(|_, &mut d| d > today);
    w.incidents.dropped.retain(|_, &mut d| d > today);
    w.incidents.national.retain(|x| x.2 >= today);
}

/// Weekly: the squads (rows, disagreements, walk-outs, lateness, visas)
/// and the grounds (pitches, equipment).
pub fn weekly(w: &mut World) {
    let today = w.date;
    let week = period::week(today);
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].league.is_some()).collect();
    for club in clubs {
        let nation = w.clubs[club].nation;
        let squad = first_squad(w, club);
        if squad.len() < 11 {
            continue;
        }
        let people: Vec<PersonId> = squad.iter().map(|&p| w.players.cold[p].person).collect();
        // Each player's sharpest point of friction in the squad.
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for i in 0..squad.len() {
            let mut best: Option<(usize, f32)> = None;
            for j in 0..squad.len() {
                if i == j {
                    continue;
                }
                let (a, b) = (people[i], people[j]);
                let mut f = consider::grievance(w, a, b) + f32::from(w.incidents.tension(a, b)) / 100.0 - consider::affinity(w, a, b).min(0.0);
                let (x, y) = (&w.players.cold[squad[i]], &w.players.cold[squad[j]]);
                if x.best_pos == y.best_pos {
                    f += 0.2;
                }
                if best.is_none_or(|bb| f > bb.1) {
                    best = Some((j, f));
                }
            }
            if let Some((j, f)) = best
                && f > 0.25
            {
                let pair = if i < j { (i, j) } else { (j, i) };
                if !pairs.contains(&pair) {
                    pairs.push(pair);
                }
            }
        }
        for (i, j) in pairs {
            // The instigator is the one with the shorter fuse.
            let (x, y) = if consider::hid(w, people[i], Hidden::Temperament) <= consider::hid(w, people[j], Hidden::Temperament) { (i, j) } else { (j, i) };
            let c = Ctx { a: people[x], b: people[y], pa: squad[x], pb: squad[y], club, nation, ..Ctx::default() };
            let prev = last_between(w, people[x], people[y]);
            if let Some(id) = consider_incident(w, IncidentKind::TrainingConfrontation, c, &[u64::from(squad[x].0), u64::from(squad[y].0), week])
                && prev.is_some()
            {
                w.incidents.list[id as usize].follows = prev;
            }
        }
        for (k, &p) in squad.iter().enumerate() {
            let c = Ctx { a: people[k], pa: p, club, nation, ..Ctx::default() };
            for kind in [IncidentKind::TacticalDisagreement, IncidentKind::StormedOut, IncidentKind::LateArrival, IncidentKind::VisaProblem] {
                if kind == IncidentKind::VisaProblem && pressure(w, Pressure::NewAbroad, &c) <= 0.0 {
                    continue;
                }
                if kind == IncidentKind::TacticalDisagreement {
                    let m = club_manager(w, club);
                    if m.is_none() || w.people[m].mind != pw_world::MindKind::Ai && w.people[people[k]].mind != pw_world::MindKind::Ai {
                        // Two humans argue in their own time; nothing to simulate.
                        continue;
                    }
                    let c2 = Ctx { b: m, ..c };
                    consider_incident(w, kind, c2, &[u64::from(p.0), week]);
                    continue;
                }
                consider_incident(w, kind, c, &[u64::from(p.0), week]);
            }
        }
        let c = Ctx { club, nation, ..Ctx::default() };
        for kind in [IncidentKind::PitchDamage, IncidentKind::EquipmentProblem] {
            consider_incident(w, kind, c, &[u64::from(club.0), week]);
        }
    }
}

/// Monthly: staff, personal lives, clubs and nations.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let month = period::month(today);
    // Staff.
    let staff: Vec<StaffId> = w.staff.ids().filter(|&s| w.staff[s].employed() && w.staff[s].role != StaffRole::Manager).collect();
    for s in staff {
        let club = w.staff[s].club;
        let c = Ctx { a: w.staff[s].person, staff: s, club, nation: w.clubs[club].nation, ..Ctx::default() };
        for kind in [IncidentKind::CoachResigned, IncidentKind::StaffPoached] {
            if consider_incident(w, kind, c, &[u64::from(s.0), month]).is_some() {
                break;
            }
        }
    }
    // Personal lives: adults with a football life.
    let people: Vec<PersonId> = w
        .people
        .iter_enumerated()
        .filter(|(id, p)| {
            let playing = p.player.is_some() && matches!(w.players.hot[p.player].status, PlayerStatus::Active | PlayerStatus::FreeAgent);
            let working = p.staff.is_some() && w.staff[p.staff].employed();
            (playing || working) && consider::age(w, *id) >= 16.0
        })
        .map(|(id, _)| id)
        .collect();
    for who in people {
        let p = w.people[who].player;
        let club = w.club_of_person(who);
        let nation = w.lives.get(who).map_or(w.people[who].nation, |l| l.home);
        let c = Ctx { a: who, pa: p, club, nation, ..Ctx::default() };
        for kind in [
            IncidentKind::FamilyEmergency,
            IncidentKind::RelationshipConflict,
            IncidentKind::MovingProblem,
            IncidentKind::Burglary,
            IncidentKind::ExamClash,
            IncidentKind::ChildcareClash,
            IncidentKind::UnexpectedBill,
        ] {
            if kind == IncidentKind::RelationshipConflict && w.lives.get(who).is_none_or(|l| l.household.partner.is_none()) {
                continue;
            }
            if consider_incident(w, kind, c, &[u64::from(who.0), month]).is_some() {
                break;
            }
        }
    }
    // Clubs.
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].league.is_some()).collect();
    for club in clubs {
        let c = Ctx { club, nation: w.clubs[club].nation, a: club_manager(w, club), ..Ctx::default() };
        for kind in [
            IncidentKind::RegistrationError,
            IncidentKind::PaperworkProblem,
            IncidentKind::OwnershipControversy,
            IncidentKind::SponsorCollapse,
            IncidentKind::FacilityDamage,
            IncidentKind::Investigation,
            IncidentKind::SupporterUnrest,
        ] {
            if kind == IncidentKind::Investigation && w.incidents.investigations.contains_key(&club) {
                continue;
            }
            if kind == IncidentKind::SponsorCollapse && !w.commerce.club_deals.iter().any(|d| d.club == club) {
                continue;
            }
            consider_incident(w, kind, c, &[u64::from(club.0), month]);
        }
    }
    // Nations.
    let nations: Vec<NationId> = w.nations.ids().filter(|&n| !w.nations[n].leagues.is_empty()).collect();
    for n in nations {
        let c = Ctx { nation: n, ..Ctx::default() };
        for kind in [IncidentKind::EconomicDownturn, IncidentKind::SevereWeather, IncidentKind::TransportDisruption, IncidentKind::FederationDispute] {
            if (kind == IncidentKind::SevereWeather || kind == IncidentKind::TransportDisruption) && w.incidents.national_active(n, kind, today).is_some() {
                continue;
            }
            consider_incident(w, kind, c, &[u64::from(n.0), month]);
        }
    }
    // Unresolved tension fades slowly on its own.
    let keys: Vec<(PersonId, PersonId)> = w.incidents.tension.keys().copied().collect();
    for (a, b) in keys {
        w.incidents.add_tension(a, b, -4);
    }
}

fn last_between(w: &World, a: PersonId, b: PersonId) -> Option<u32> {
    w.incidents.list.iter().rev().take(2000).find(|i| i.parties.contains(&a) && i.parties.contains(&b) && i.date.days_until(w.date) < 365).map(|i| i.id)
}

// ---------------------------------------------------------------------------
// Consequences
// ---------------------------------------------------------------------------

fn consequences(w: &mut World, id: u32) {
    let today = w.date;
    let inc = w.incidents.list[id as usize].clone();
    let ev = inc.event;
    let sev = f32::from(inc.severity) / 100.0;
    let a = inc.parties.first().copied().unwrap_or(PersonId::NONE);
    let b = inc.parties.get(1).copied().unwrap_or(PersonId::NONE);
    let mut rng = w.rng(stream::INCIDENTS, &[u64::from(id), 0xc0]);
    match inc.kind {
        IncidentKind::TrainingConfrontation => {
            let compat_ab = consider::compat(w, a, b);
            let compat_ba = consider::compat(w, b, a);
            w.social.remember(a, b, MemoryKind::Fought, today, ev, false, 0.6 + sev, compat_ab);
            w.social.remember(b, a, MemoryKind::Fought, today, ev, false, 0.6 + sev, compat_ba);
            w.incidents.add_tension(a, b, (20.0 + sev * 50.0) as i16);
            for &p in &inc.players {
                let h = &mut w.players.hot[p];
                h.morale = h.morale.saturating_sub((4.0 + sev * 8.0) as u8);
            }
            // Witnesses side with whoever they like.
            for &x in &inc.witnesses {
                let (fa, fb) = (consider::affinity(w, x, a), consider::affinity(w, x, b));
                let target = if fa < fb { a } else { b };
                let compat = consider::compat(w, x, target);
                w.social.adjust(x, target, today, compat, -3, -2, 0);
            }
            // The worst rows leave a mark.
            if sev > 0.85 && inc.players.len() == 2 {
                let hurt = inc.players[1];
                let h = &mut w.players.hot[hurt];
                h.condition = h.condition.saturating_sub(15);
            }
        }
        IncidentKind::TacticalDisagreement => {
            let compat = consider::compat(w, a, b);
            w.social.remember(a, b, MemoryKind::Argument, today, ev, false, 0.5 + sev * 0.5, compat);
            let compat = consider::compat(w, b, a);
            w.social.remember(b, a, MemoryKind::Argument, today, ev, false, 0.4 + sev * 0.5, compat);
            w.incidents.add_tension(a, b, (10.0 + sev * 30.0) as i16);
        }
        IncidentKind::StormedOut => {
            let m = club_manager(w, inc.club);
            w.incidents.add_tension(a, m, (15.0 + sev * 30.0) as i16);
            if let Some(&p) = inc.players.first() {
                let h = &mut w.players.hot[p];
                h.training = h.training.saturating_sub(10);
            }
        }
        IncidentKind::LateArrival => {
            if let Some(&p) = inc.players.first() {
                let h = &mut w.players.hot[p];
                h.training = h.training.saturating_sub(4);
            }
        }
        IncidentKind::EquipmentProblem => {
            // A lost session: the squad trains lightly.
            for p in first_squad(w, inc.club) {
                let h = &mut w.players.hot[p];
                h.sharpness = h.sharpness.saturating_sub(2);
            }
            let cost = (20_000.0 * (0.5 + sev)) as i64;
            w.clubs[inc.club].finance.balance -= cost;
        }
        IncidentKind::PitchDamage | IncidentKind::FacilityDamage => {
            let cost = (80_000.0 * (0.5 + sev) * (f32::from(w.clubs[inc.club].reputation) / 5000.0 + 0.3)) as i64;
            w.clubs[inc.club].finance.balance -= cost;
            if inc.kind == IncidentKind::FacilityDamage && sev > 0.6 {
                let f = &mut w.clubs[inc.club].facilities;
                f.training = f.training.saturating_sub(1).max(1);
            }
        }
        IncidentKind::Postponement => {
            if inc.fixture.is_some() {
                let d = w.fixtures.get(inc.fixture).date;
                let (h, a) = (w.fixtures.get(inc.fixture).home, w.fixtures.get(inc.fixture).away);
                let to = w.fixtures.first_free_date(h, a, d.add_days(7).next_weekday(pw_core::Weekday::Wed), 21, Some(inc.fixture));
                w.fixtures.reschedule(inc.fixture, to);
            }
        }
        IncidentKind::TravelDelay => {
            if inc.fixture.is_some() {
                let team = w.fixtures.get(inc.fixture).away;
                let squad = w.teams[team].squad.clone();
                let hit = (4.0 + sev * 8.0) as u8;
                for p in squad {
                    let h = &mut w.players.hot[p];
                    h.condition = h.condition.saturating_sub(hit);
                }
            }
        }
        IncidentKind::VisaProblem | IncidentKind::RegistrationError => {
            // The player cannot be fielded for a while.
            let p = if let Some(&p) = inc.players.first() {
                p
            } else {
                // Registration: the club's most recent signing.
                let recent = first_squad(w, inc.club).into_iter().filter(|&p| w.players.cold[p].joined.days_until(today) < 60).max_by_key(|&p| w.players.cold[p].joined);
                match recent {
                    Some(p) => p,
                    None => return,
                }
            };
            let days = (7.0 + sev * 28.0) as i32;
            w.incidents.away.insert(p, today.add_days(days));
            if inc.kind == IncidentKind::RegistrationError {
                w.clubs[inc.club].finance.balance -= 50_000;
                let b = &mut w.clubs[inc.club].board;
                b.satisfaction = b.satisfaction.saturating_sub(3);
                if let Some(x) = w.incidents.list.get_mut(id as usize) {
                    x.players.push(p);
                    x.parties.push(w.players.cold[p].person);
                }
            }
        }
        IncidentKind::PaperworkProblem => {
            // Open talks at the club drag on.
            let club = inc.club;
            for t in w.talks.iter_mut() {
                if t.club == club && t.is_open() {
                    t.deadline = t.deadline.add_days(rng.range_i32(3, 10));
                }
            }
        }
        IncidentKind::CoachResigned | IncidentKind::StaffPoached => {
            let s = inc.staff;
            if s.is_some() && w.staff[s].club == inc.club {
                w.staff[s].club = ClubId::NONE;
                w.clubs[inc.club].staff.retain(|&x| x != s);
                if inc.kind == IncidentKind::StaffPoached {
                    // A bigger club with a weaker holder of the role takes them.
                    let role = w.staff[s].role;
                    let rating = w.staff[s].role_rating(role);
                    let rep = w.clubs[inc.club].reputation;
                    let buyer = w
                        .clubs
                        .ids()
                        .filter(|&c| c != inc.club && w.clubs[c].reputation > rep && w.clubs[c].nation == inc.nation)
                        .filter(|&c| w.clubs[c].staff.iter().filter(|&&x| w.staff[x].role == role).all(|&x| w.staff[x].role_rating(role) < rating))
                        .min_by_key(|&c| w.clubs[c].reputation);
                    if let Some(c) = buyer {
                        w.staff[s].club = c;
                        w.staff[s].joined = today;
                        w.clubs[c].staff.push(s);
                    }
                }
                let m = club_manager(w, inc.club);
                let sp = w.staff[s].person;
                let compat = consider::compat(w, m, sp);
                w.social.remember(m, sp, MemoryKind::LetDown, today, ev, false, 0.5, compat);
            }
        }
        IncidentKind::FamilyEmergency => {
            if let Some(l) = w.lives.get_mut(a) {
                l.stress = l.stress.saturating_add((10.0 + sev * 20.0) as u8).min(100);
            }
            crate::responses::personal_request(w, id);
        }
        IncidentKind::RelationshipConflict => {
            if let Some(l) = w.lives.get_mut(a) {
                l.stress = l.stress.saturating_add((6.0 + sev * 12.0) as u8).min(100);
                if let Some(p) = l.household.partner.as_mut() {
                    p.bond = p.bond.saturating_sub((5.0 + sev * 15.0) as u8);
                }
            }
            let partner = w.lives.get(a).and_then(|l| l.household.partner).map(|p| p.person);
            if let Some(pt) = partner {
                if let Some(p) = w.lives.get_mut(pt).and_then(|l| l.household.partner.as_mut()) {
                    p.bond = p.bond.saturating_sub((5.0 + sev * 15.0) as u8);
                }
                let compat = consider::compat(w, pt, a);
                w.social.remember(pt, a, MemoryKind::Argument, today, ev, false, 0.5 + sev, compat);
            }
        }
        IncidentKind::MovingProblem => {
            if let Some(l) = w.lives.get_mut(a) {
                l.stress = l.stress.saturating_add(6).min(100);
                l.sleep = l.sleep.saturating_sub(8);
            }
        }
        IncidentKind::Burglary => {
            let loss = w.lives.get(a).map_or(0, |l| (l.finances.savings as f32 * (0.01 + sev * 0.03)) as i64);
            if let Some(l) = w.lives.get_mut(a) {
                l.finances.savings -= loss;
                l.stress = l.stress.saturating_add((10.0 + sev * 15.0) as u8).min(100);
            }
            if let Some(pt) = w.lives.get(a).and_then(|l| l.household.partner).map(|p| p.person)
                && let Some(l) = w.lives.get_mut(pt)
            {
                l.stress = l.stress.saturating_add(10).min(100);
            }
        }
        IncidentKind::ExamClash | IncidentKind::ChildcareClash => crate::responses::personal_request(w, id),
        IncidentKind::UnexpectedBill => {
            let bill = (1_500.0 + sev * 6_000.0) as i64;
            if let Some(l) = w.lives.get_mut(a) {
                l.finances.savings -= bill;
                if l.finances.savings < 0 {
                    l.finances.debt += -l.finances.savings;
                    l.finances.savings = 0;
                }
                l.stress = l.stress.saturating_add(4).min(100);
            }
        }
        IncidentKind::Pregnancy => {}
        IncidentKind::OwnershipControversy => {
            let club = inc.club;
            w.clubs[club].fan_mood = w.clubs[club].fan_mood.saturating_sub((5.0 + sev * 10.0) as u8);
            if let Some(g) = w.governance.get(&club) {
                let owner = g.owner.person;
                w.media.nudge_image(owner, -(20.0 + sev * 60.0) as i16);
                w.media.move_fans(club, owner, -(50.0 + sev * 150.0) as i16, FanReason::Interview, today);
            }
            crate::responses::club_response(w, id);
        }
        IncidentKind::SupporterUnrest => {
            let club = inc.club;
            let b = &mut w.clubs[club].board;
            b.satisfaction = b.satisfaction.saturating_sub((3.0 + sev * 6.0) as u8);
            let m = club_manager(w, club);
            if let Some(l) = w.lives.get_mut(m) {
                l.stress = l.stress.saturating_add(8).min(100);
            }
            crate::responses::club_response(w, id);
        }
        IncidentKind::SponsorCollapse => {
            let club = inc.club;
            let brand = w.commerce.club_deals.iter().find(|d| d.club == club).map(|d| d.brand);
            if let Some(brand) = brand {
                // Every club and person the brand paid loses the deal.
                w.commerce.club_deals.retain(|d| d.brand != brand);
                let people: Vec<u32> =
                    (0..w.commerce.endorsements.len() as u32).filter(|&i| w.commerce.endorsements[i as usize].brand == brand && w.commerce.endorsements[i as usize].ended.is_none()).collect();
                for i in people {
                    let e = &mut w.commerce.endorsements[i as usize];
                    e.ended = Some((today, pw_world::commerce::DealEnd::Faded));
                    let person = e.person;
                    if let Some(v) = w.commerce.by_person.get_mut(&person) {
                        v.retain(|&x| x != i);
                    }
                }
                w.commerce.brands[brand as usize].budget = 0;
                let b = &mut w.clubs[club].board;
                b.satisfaction = b.satisfaction.saturating_sub(4);
            }
        }
        IncidentKind::EconomicDownturn => {
            if let Some(e) = w.economy.nations.get_mut(&inc.nation) {
                e.growth -= 0.01 + sev * 0.03;
                e.wage_index *= 1.0 - sev * 0.03;
            }
            w.nations[inc.nation].economy = (w.nations[inc.nation].economy * (1.0 - sev * 0.04)).max(0.05);
        }
        IncidentKind::SevereWeather | IncidentKind::TransportDisruption => {
            let days = (3.0 + sev * 10.0) as i32;
            w.incidents.national.push((inc.nation, inc.kind, today.add_days(days), id));
        }
        IncidentKind::FederationDispute => {
            // National team and clubs at odds over players.
            let national_manager = w.intl.sides.get(&(inc.nation, pw_world::intl::Level::Senior)).map(|s| w.staff[s.manager].person);
            if let Some(nm) = national_manager {
                let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].nation == inc.nation && w.clubs[c].reputation >= 5000).collect();
                for c in clubs {
                    let m = club_manager(w, c);
                    if m.is_some() {
                        let compat = consider::compat(w, m, nm);
                        w.social.adjust(m, nm, today, compat, -3, -4, 0);
                    }
                }
            }
        }
        IncidentKind::Investigation => {
            let verdict = today.add_days(90 + rng.range_i32(0, 90));
            w.incidents.investigations.insert(inc.club, (verdict, id));
            let b = &mut w.clubs[inc.club].board;
            b.satisfaction = b.satisfaction.saturating_sub(5);
        }
    }
}

/// Investigations reach a verdict.
fn investigations(w: &mut World) {
    let today = w.date;
    let due: Vec<(ClubId, u32)> = w.incidents.investigations.iter().filter(|(_, v)| v.0 <= today).map(|(&c, &(_, i))| (c, i)).collect();
    for (club, id) in due {
        w.incidents.investigations.remove(&club);
        let red = w.governance.get(&club).map_or(0, |g| g.red_months);
        let guilty = red >= 6 || w.clubs[club].finance.balance < -(w.clubs[club].finance.season_income / 2);
        let prev = w.incidents.list[id as usize].event;
        let causes = pw_world::causes![Cause::Event(prev)];
        if guilty {
            let league = w.clubs[club].league;
            let team = w.clubs[club].first_team();
            let points: i16 = 6;
            if league.is_some()
                && let Some(r) = w.comps[league].state.table.iter_mut().find(|r| r.team == team)
            {
                r.points -= points;
            }
            w.events.push_caused(today, Visibility::Public, EventKind::PointsDeducted { club, points: points as u8 }, causes);
        } else {
            w.events.push_caused(today, Visibility::Public, EventKind::InvestigationCleared { club }, causes);
        }
        if let Some(x) = w.incidents.list.get_mut(id as usize) {
            x.resolved = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Pregnancy and birth
// ---------------------------------------------------------------------------

/// A couple are expecting (called by the life model instead of an instant birth).
pub fn conceive(w: &mut World, a: PersonId, b: PersonId) {
    let today = w.date;
    if w.incidents.expecting.iter().any(|x| x.0 == a || x.1 == a) {
        return;
    }
    let d = def(IncidentKind::Pregnancy);
    let c = Ctx { a, b, club: w.club_of_person(a), nation: w.lives.get(a).map_or(NationId::NONE, |l| l.home), ..Ctx::default() };
    let id = trigger(w, &d, c, SmallVec::new(), None);
    let due = today.add_days(270 + w.rng(stream::FAMILY, &[u64::from(a.0), u64::from(id)]).range_i32(-14, 14));
    w.incidents.expecting.push((a, b, due, id));
    for x in [a, b] {
        if let Some(l) = w.lives.get_mut(x) {
            l.fulfilment = l.fulfilment.saturating_add(6).min(100);
        }
    }
}

fn births(w: &mut World) {
    let today = w.date;
    let due: Vec<(PersonId, PersonId, u32)> = w.incidents.expecting.iter().filter(|x| x.2 <= today).map(|x| (x.0, x.1, x.3)).collect();
    w.incidents.expecting.retain(|x| x.2 > today);
    for (a, b, id) in due {
        for x in [a, b] {
            if let Some(l) = w.lives.get_mut(x) {
                l.household.children += 1;
                l.household.youngest_born = today;
                l.fulfilment = l.fulfilment.saturating_add(15).min(100);
                l.sleep = l.sleep.saturating_sub(20);
            }
        }
        let prev = w.incidents.list[id as usize].event;
        w.events.push_caused(today, Visibility::Public, EventKind::Life { person: a, kind: LifeEventKind::ChildBorn }, pw_world::causes![Cause::Event(prev)]);
        // Parental leave is asked for like any other leave.
        if let Some(x) = w.incidents.list.get_mut(id as usize) {
            x.resolved = true;
        }
        crate::responses::birth_leave(w, a, id);
    }
}

/// Whether a player is unavailable because of an incident (leave, visa,
/// registration, a disciplinary omission).
pub fn unavailable(w: &World, p: PlayerId) -> bool {
    w.incidents.is_away(p, w.date)
}

/// A short account of an incident for other systems (press, inbox).
pub fn involves(w: &World, id: u32, who: PersonId) -> bool {
    w.incidents.get(id).is_some_and(|i| i.parties.contains(&who) || i.witnesses.contains(&who))
}
