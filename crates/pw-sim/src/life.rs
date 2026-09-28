//! The life model (10, S19–S20). One set of rules for every person with a
//! football role: households form and break, parents age, money comes in and
//! goes out, languages are learned, people settle or don't — and all of it
//! feeds well-being and stress with the reasons kept.
//!
//! The model never looks at who controls a person. A human changes the
//! *inputs* (routine, lifestyle, whom they ask what); the rules are the same.

use pw_core::math::{ewma, sigmoid};
use pw_core::rng::{Rng, stream};
use pw_core::{Date, Hidden, NationId, PersonId};
use pw_world::event::{Cause, Causes, EventKind, Fact, LifeEventKind, Visibility};
use pw_world::life::{Household, Lifestyle, Mood, MoodFactor, Occupation, Parents, Partner, PartnerStatus, Routine};
use pw_world::{Life, MemoryKind, MindKind, NameId, Person, PlayerStatus, World};

use crate::consider;
use crate::generate as gen_;

// ------------------------------------------------------------------ sync

/// Every person has a life. New people (regens, staff, partners) get one the
/// day they appear, generated from `(seed, person id)` so what exists never
/// depends on who is being watched (S20).
pub fn sync(w: &mut World) {
    let before = w.lives.len();
    while w.lives.len() < w.people.len() {
        w.lives.push(Life::default());
    }
    if before == w.people.len() {
        return;
    }
    let fresh: Vec<PersonId> = (before..w.people.len()).map(|i| PersonId(i as u32)).collect();
    for id in fresh {
        if !w.lives[id].ready {
            init(w, id);
        }
    }
}

fn nation_of(w: &World, who: PersonId) -> NationId {
    let club = w.club_of_person(who);
    if club.is_some() { w.clubs[club].nation } else { w.people[who].nation }
}

fn init(w: &mut World, who: PersonId) {
    let today = w.date;
    let mut rng = Rng::keyed(&[w.seed, stream::FAMILY, u64::from(who.0)]);
    let person = w.people[who].clone();
    let age = person.dob.age_years(today);
    let home = nation_of(w, who);
    let footballer = person.player.is_some() || person.staff.is_some();

    let mut life = Life { home, ready: true, ..Life::default() };
    life.home_since = if home == person.nation { person.dob } else { today.add_days(-rng.range_i32(30, 900)) };
    if person.nation.is_some() {
        life.languages.push((person.nation, 100));
    }
    if person.nation2.is_some() && person.nation2 != person.nation {
        life.languages.push((person.nation2, rng.range_i32(40, 100) as u8));
    }
    if home.is_some() && home != person.nation {
        let years = life.years_here(today);
        let adapt = person.hidden.f(Hidden::Adaptability);
        life.learn_language(home, (years * 12.0 * (0.5 + adapt / 20.0)).min(90.0));
    }
    life.education = match age as u32 {
        0..=15 => 1,
        16..=18 => 2,
        _ => rng.range_i32(2, 4) as u8,
    };
    life.occupation = if footballer { Occupation::Athlete } else { random_occupation(&mut rng, age) };

    // Parents: roughly 28 years older than the person, ageing by the same curve.
    let parent_age = age + rng.normal_ms(29.0, 4.0);
    let alive_p = sigmoid((82.0 - parent_age) / 6.0);
    let alive = u8::from(rng.chance(alive_p)) + u8::from(rng.chance(alive_p));
    life.household.parents = Parents {
        nation: person.nation,
        alive,
        health: (100.0 - (parent_age - 45.0).max(0.0) * 1.4 + rng.normal() * 8.0).clamp(10.0, 100.0) as u8,
        closeness: rng.normal_ms(62.0, 18.0).clamp(5.0, 100.0) as u8,
        support: rng.normal_ms(11.0, 4.0).clamp(1.0, 20.0) as u8,
        means: rng.weighted(&[0.18, 0.3, 0.3, 0.15, 0.07]) as u8 + 1,
    };
    life.household.siblings = rng.weighted(&[0.18, 0.38, 0.26, 0.12, 0.06]) as u8;

    // Finances: savings consistent with the career so far.
    if person.player.is_some() {
        let c = &w.players.cold[person.player];
        let weekly = c.contract.current_wage(today).max(0);
        let years_paid = (age - 18.0).clamp(0.0, 18.0);
        let prof = person.hidden.f(Hidden::Professionalism);
        life.finances.savings = (weekly as f32 * 52.0 * years_paid * 0.12 * (0.5 + prof / 20.0)) as i64;
        life.finances.lifestyle = lifestyle_for(&person, weekly);
    }
    life.routine = crate::mind::ai_routine(w, who, &life);
    w.lives[who] = life;

    // Partner and children for adults, by age — a real person when they exist.
    if footballer && age >= 18.0 {
        let p_partner = sigmoid((age - 24.0) / 3.0) * 0.8;
        if rng.chance(p_partner) {
            let years_together = (rng.f32() * (age - 18.0).min(12.0)).max(0.1);
            let status = match years_together {
                y if y > 5.0 && rng.chance(0.7) => PartnerStatus::Married,
                y if y > 1.5 => PartnerStatus::Living,
                _ => PartnerStatus::Dating,
            };
            let since = today.add_days(-(years_together * 365.0) as i32);
            let partner = create_partner(w, who, since, status, &mut rng);
            if status == PartnerStatus::Married && age >= 25.0 {
                let kids = rng.weighted(&[0.35, 0.35, 0.22, 0.08]) as u8;
                w.lives[who].household.children = kids;
                w.lives[partner].household.children = kids;
                if kids > 0 {
                    let youngest = today.add_days(-rng.range_i32(60, 2400));
                    w.lives[who].household.youngest_born = youngest;
                    w.lives[partner].household.youngest_born = youngest;
                }
            }
        }
    }
}

fn random_occupation(rng: &mut Rng, age: f32) -> Occupation {
    if age < 22.0 && rng.chance(0.55) {
        return Occupation::Student;
    }
    let w = [0.06, 0.0, 0.24, 0.14, 0.3, 0.12, 0.04, 0.1];
    Occupation::ALL[rng.weighted(&w)]
}

pub fn lifestyle_for(p: &Person, weekly_wage: i64) -> Lifestyle {
    let prof = p.hidden.f(Hidden::Professionalism);
    let contro = p.hidden.f(Hidden::Controversy);
    let base = match weekly_wage {
        ..=800 => 0,
        801..=5_000 => 1,
        5_001..=40_000 => 2,
        _ => 3,
    };
    let lean = if contro >= 14.0 && prof <= 10.0 { 1 } else if prof >= 16.0 { -1 } else { 0 };
    Lifestyle::ALL[(base + lean).clamp(0, 3) as usize]
}

/// A new partner: a full person with their own nationality, career and roots.
fn create_partner(w: &mut World, who: PersonId, since: Date, status: PartnerStatus, rng: &mut Rng) -> PersonId {
    let me = w.people[who].clone();
    let home = w.lives[who].home;
    // Mostly someone from where they grew up or live now; sometimes elsewhere.
    let nation = match rng.f32() {
        x if x < 0.62 => me.nation,
        x if x < 0.9 && home.is_some() => home,
        _ => pw_core::NationId(rng.below(w.nations.len().max(1) as u32)),
    };
    let nation = if nation.is_some() { nation } else { me.nation };
    let (first, last) = crate::people::random_name(w, nation, rng);
    let dob = me.dob.add_days((rng.normal() * 3.0 * 365.0) as i32);
    let pid = w.people.push(Person {
        first,
        last,
        common: NameId::NONE,
        dob,
        nation,
        nation2: Default::default(),
        hidden: gen_::hidden_random(rng),
        player: Default::default(),
        staff: Default::default(),
        mind: MindKind::Ai,
    });
    while w.lives.len() < w.people.len() {
        w.lives.push(Life::default());
    }
    let age = dob.age_years(w.date);
    let mut plife = Life { home: if status == PartnerStatus::Dating { nation } else { home }, ready: true, ..Life::default() };
    plife.home_since = since;
    plife.languages.push((nation, 100));
    if home.is_some() && home != nation {
        plife.learn_language(home, rng.range_f32(10.0, 70.0));
    }
    plife.occupation = random_occupation(rng, age);
    plife.education = rng.range_i32(2, 5) as u8;
    plife.household = Household {
        partner: Some(Partner { person: who, since, status, bond: rng.normal_ms(68.0, 12.0).clamp(30.0, 100.0) as u8, lives: home }),
        parents: Parents { nation, ..Parents::default() },
        siblings: rng.below(4) as u8,
        ..Household::default()
    };
    let bond = plife.household.partner.as_ref().map_or(60, |p| p.bond);
    let lives_at = if status == PartnerStatus::Dating && nation != home { nation } else { home };
    w.lives[pid] = plife;
    w.lives[who].household.partner = Some(Partner { person: pid, since, status, bond, lives: lives_at });
    pid
}

// ------------------------------------------------------------------ monthly

/// Everyone's month: money, family, relationships, languages, stress and
/// well-being. Deterministic per (seed, person, month).
pub fn monthly(w: &mut World) {
    let month = u64::from(w.date.month()) + w.date.year() as u64 * 12;
    let people: Vec<PersonId> = w
        .people
        .iter_enumerated()
        .filter(|(_, p)| p.player.is_some() || p.staff.is_some())
        .map(|(id, _)| id)
        .collect();
    for who in people {
        if !w.lives[who].ready {
            init(w, who);
        }
        let mut rng = Rng::keyed(&[w.seed, stream::LIFE, u64::from(who.0), month]);
        finances(w, who);
        parents(w, who, &mut rng);
        relationship(w, who, &mut rng);
        languages(w, who);
        wellbeing(w, who, &mut rng);
    }
}

fn tax_rate(w: &World, nation: NationId) -> f32 {
    if nation.is_none() { 0.3 } else { (0.18 + 0.25 * w.nations[nation].economy).clamp(0.12, 0.48) }
}

fn finances(w: &mut World, who: PersonId) {
    let today = w.date;
    // Children's lives are paid for by their families.
    if consider::age(w, who) < 18.0 {
        return;
    }
    let person = &w.people[who];
    let mut gross_week: i64 = 0;
    // Amateur footballers have day jobs.
    if person.player.is_some() && w.players.hot[person.player].status == PlayerStatus::Amateur {
        let econ = w.nations.get(person.nation).map_or(0.5, |n| n.economy);
        gross_week += (500.0 * econ * w.economy.global()) as i64;
    }
    if person.player.is_some() && w.players.hot[person.player].status == PlayerStatus::Active {
        gross_week += w.players.cold[person.player].contract.current_wage(today);
    }
    // Club staff, or a federation's manager.
    if person.staff.is_some() && (w.staff[person.staff].employed() || w.intl.managers.contains(&person.staff)) {
        gross_week += w.staff[person.staff].wage;
    }
    let home = w.lives[who].home;
    let tax = tax_rate(w, home);
    // Endorsements, post-playing work, homes, helpers, giving.
    let (extra_in, extra_out) = crate::affairs::money(w, who);
    let income = ((gross_week as f32 * 52.0 / 12.0 + extra_in as f32) * (1.0 - tax)) as i64;
    let (share, floor) = w.lives[who].finances.lifestyle.spend();
    let kids = i64::from(w.lives[who].household.children) * 350;
    let floor = (floor as f32 * crate::affairs::cost_index(w, home) / 0.7) as i64;
    let spend = ((income as f32 * share) as i64).max(floor) + kids + extra_out;
    let parents = w.lives[who].household.parents;
    let support = if parents.alive > 0 && parents.means <= 2 && income > 6_000 {
        (income as f32 * 0.04 * f32::from(parents.closeness) / 60.0) as i64
    } else {
        0
    };
    let f = &mut w.lives[who].finances;
    f.income = income;
    f.spending = spend;
    f.family_support = support;
    let net = income - spend - support;
    if net >= 0 {
        let repay = net.min(f.debt);
        f.debt -= repay;
        f.savings += net - repay;
    } else {
        let draw = (-net).min(f.savings);
        f.savings -= draw;
        f.debt += -net - draw;
    }
    let trouble = f.debt > income.max(1_000) * 3;
    if trouble && today.month() % 3 == 0 {
        w.events.push_caused(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::FinancialTrouble }, Causes::new());
    }
}

fn parents(w: &mut World, who: PersonId, rng: &mut Rng) {
    let today = w.date;
    let age = consider::age(w, who);
    let p = w.lives[who].household.parents;
    if p.alive == 0 {
        return;
    }
    let parent_age = age + 29.0;
    let decline = ((parent_age - 55.0).max(0.0) * 0.06 + rng.normal() * 0.8).max(-1.0);
    let mut health = (f32::from(p.health) - decline).clamp(0.0, 100.0);
    let mut alive = p.alive;
    let ill_p = if parent_age > 60.0 { 0.004 * (parent_age - 55.0) / 5.0 } else { 0.002 };
    if rng.chance(ill_p) {
        health = (health - rng.range_f32(15.0, 40.0)).max(0.0);
        w.events.push(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::ParentUnwell });
        w.lives[who].stress = w.lives[who].stress.saturating_add(12).min(100);
    } else if p.health < 45 && health >= 45.0 {
        w.events.push(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::ParentRecovered });
    }
    if health < 12.0 && rng.chance(0.25) {
        alive -= 1;
        health = if alive > 0 { rng.range_f32(55.0, 85.0) } else { 0.0 };
        w.events.push(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::Bereavement });
        let l = &mut w.lives[who];
        l.stress = l.stress.saturating_add(25).min(100);
        l.fulfilment = l.fulfilment.saturating_sub(10);
    } else {
        health = (health + if health < 60.0 { rng.range_f32(0.0, 3.0) } else { 0.0 }).min(100.0);
    }
    let l = &mut w.lives[who].household.parents;
    l.health = health as u8;
    l.alive = alive;
    // Closeness follows time spent with family.
    let fam = f32::from(w.lives[who].routine.family);
    let l = &mut w.lives[who].household.parents;
    l.closeness = ewma(f32::from(l.closeness), (40.0 + fam * 4.0).min(95.0), 0.05) as u8;
}

fn relationship(w: &mut World, who: PersonId, rng: &mut Rng) {
    let today = w.date;
    let Some(pt) = w.lives[who].household.partner else {
        maybe_meet_someone(w, who, rng);
        return;
    };
    let me = w.people[who].clone();
    let partner = w.people[pt.person].clone();
    let life = w.lives[who].clone();
    // Time together, distance, stress spill-over, compatibility.
    let hours = f32::from(life.routine.partner);
    let apart = pt.lives != life.home;
    let compat = f32::from(pw_world::social::compatibility(&me, &partner, today)) / 30.0;
    let stress = f32::from(life.stress) / 100.0;
    let night = f32::from(life.routine.nightlife) / 10.0;
    let target = 55.0 + hours * 2.2 + compat * 15.0 - stress * 25.0 - night * 10.0 - if apart { 22.0 } else { 0.0 }
        + (me.hidden.f(Hidden::Loyalty) - 10.0) * 0.8
        + rng.normal() * 6.0;
    let bond = ewma(f32::from(pt.bond), target.clamp(0.0, 100.0), 0.18).clamp(0.0, 100.0) as u8;
    set_bond(w, who, pt.person, bond);

    // The partner's own decisions: take things further, or end it.
    let years = pt.since.days_until(today) as f32 / 365.0;
    let partner_mind = partner.mind;
    if bond < 22 && rng.chance(0.35) {
        separate(w, who, pt.person, Causes::new());
        return;
    }
    let next = match pt.status {
        PartnerStatus::Dating if years > 1.0 && bond > 65 && !apart => Some(pw_world::PartnerAsk::MoveIn),
        PartnerStatus::Living if years > 2.5 && bond > 75 && consider::age(w, pt.person) > 23.0 => Some(pw_world::PartnerAsk::Marry),
        _ => None,
    };
    if let Some(ask) = next {
        // Either side may raise it; the partner does so here (a person the
        // partner's own AI speaks for), and it becomes a decision for `who`.
        let p_raise = 0.05 + 0.1 * f32::from(bond) / 100.0;
        if partner_mind == MindKind::Ai && rng.chance(p_raise) {
            crate::decisions::partner_asks(w, who, pt.person, ask);
        }
    }
    // Children for settled couples (a household outcome, same odds for everyone).
    if pt.status == PartnerStatus::Married {
        let age = consider::age(w, who);
        let kids = life.household.children;
        let recent = life.household.youngest_born.days_until(today) < 540 && kids > 0;
        let want = (bond as f32 / 100.0) * if (24.0..=38.0).contains(&age) { 1.0 } else { 0.2 } / (1.0 + f32::from(kids) * 0.8);
        if !recent && rng.chance(0.012 * want) {
            // Expecting: the birth comes in about nine months (incidents).
            crate::incidents::conceive(w, who, pt.person);
        }
    }
}

fn set_bond(w: &mut World, a: PersonId, b: PersonId, bond: u8) {
    if let Some(p) = w.lives[a].household.partner.as_mut() {
        p.bond = bond;
    }
    if let Some(p) = w.lives[b].household.partner.as_mut() {
        p.bond = bond;
    }
}

/// Change a relationship's status after both sides agreed.
pub fn advance_relationship(w: &mut World, a: PersonId, b: PersonId, ask: pw_world::PartnerAsk) {
    let today = w.date;
    let (status, kind) = match ask {
        pw_world::PartnerAsk::MoveIn => (PartnerStatus::Living, LifeEventKind::MovedIn { partner: b }),
        pw_world::PartnerAsk::Marry => (PartnerStatus::Married, LifeEventKind::Married { partner: b }),
        pw_world::PartnerAsk::Separate => {
            separate(w, a, b, Causes::new());
            return;
        }
    };
    let home = w.lives[a].home;
    for (x, other) in [(a, b), (b, a)] {
        if let Some(p) = w.lives[x].household.partner.as_mut() {
            if p.person == other {
                p.status = status;
                p.bond = p.bond.saturating_add(6).min(100);
                p.lives = home;
            }
        }
        w.lives[x].fulfilment = w.lives[x].fulfilment.saturating_add(6).min(100);
    }
    w.lives[b].home = home;
    let vis = if status == PartnerStatus::Married { Visibility::Public } else { Visibility::Person(a) };
    w.events.push(today, vis, EventKind::Life { person: a, kind });
}

pub fn separate(w: &mut World, a: PersonId, b: PersonId, causes: Causes) {
    let today = w.date;
    for (x, other) in [(a, b), (b, a)] {
        let h = &mut w.lives[x].household;
        if h.partner.as_ref().is_some_and(|p| p.person == other) {
            h.partner = None;
            h.past_partners.push(other);
        }
        let l = &mut w.lives[x];
        l.stress = l.stress.saturating_add(18).min(100);
        l.fulfilment = l.fulfilment.saturating_sub(8);
    }
    let married = w.events.all().iter().rev().take(20_000).any(|e| matches!(e.kind, EventKind::Life { person, kind: LifeEventKind::Married { partner } } if (person == a && partner == b) || (person == b && partner == a)));
    let vis = if married { Visibility::Public } else { Visibility::Person(a) };
    let ev = w.events.push_caused(today, vis, EventKind::Life { person: a, kind: LifeEventKind::Separated { partner: b } }, causes);
    let compat = consider::compat(w, a, b);
    w.social.remember(b, a, MemoryKind::LetDown, today, ev, false, 1.0, compat);
}

fn maybe_meet_someone(w: &mut World, who: PersonId, rng: &mut Rng) {
    let age = consider::age(w, who);
    if age < 17.0 {
        return;
    }
    let explicit = w.intents.dating.get(&who).copied();
    let open = explicit.unwrap_or_else(|| age < 45.0 && rng.chance(0.6));
    if !open {
        return;
    }
    let social = f32::from(w.lives[who].routine.social + w.lives[who].routine.nightlife) / 12.0;
    let settled = consider::settledness(w, who);
    let p = 0.02 * (0.4 + social) * (0.5 + settled) * if (19.0..=34.0).contains(&age) { 1.0 } else { 0.5 };
    if rng.chance(p) {
        let partner = create_partner(w, who, w.date, PartnerStatus::Dating, rng);
        w.events.push(w.date, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::StartedDating { partner } });
    }
}

fn languages(w: &mut World, who: PersonId) {
    let home = w.lives[who].home;
    if home.is_none() {
        return;
    }
    let native = w.people[who].nation == home;
    if native {
        return;
    }
    let adapt = w.people[who].hidden.f(Hidden::Adaptability);
    let hours = f32::from(w.lives[who].routine.language);
    let gain = 1.2 * (0.5 + adapt / 20.0) + hours * 0.9;
    w.lives[who].learn_language(home, gain);
    if let Some(pt) = w.lives[who].household.partner {
        if pt.lives == home {
            let pa = w.people[pt.person].hidden.f(Hidden::Adaptability);
            w.lives[pt.person].learn_language(home, 1.0 * (0.5 + pa / 20.0));
        }
    }
}

/// Monthly well-being with reasons (bounded effects, 10 §2).
fn wellbeing(w: &mut World, who: PersonId, rng: &mut Rng) {
    let today = w.date;
    let person = w.people[who].clone();
    let life = w.lives[who].clone();
    let r = life.routine;
    let player = person.player;
    let playing = player.is_some() && w.players.hot[player].status != PlayerStatus::Retired;
    let mut mood = Mood::new();
    let mut push = |f: MoodFactor, v: f32| {
        let v = v.round().clamp(-40.0, 40.0) as i8;
        if v != 0 {
            mood.push((f, v));
        }
    };

    // Balance of the week: rest and recovery against nightlife; some of
    // everything that matters to this person.
    let rest = f32::from(r.rest + r.recovery);
    let balance = (rest - 12.0) * 0.5 - f32::from(r.nightlife) * 0.9 + f32::from(r.hobbies.min(10)) * 0.4 + f32::from(r.social.min(10)) * 0.3;
    push(MoodFactor::Routine, balance.clamp(-10.0, 8.0));

    // Stress: career pressure, money, home, loss.
    let mut stress = f32::from(life.stress) * 0.7;
    if playing {
        let grievance = consider::minutes_grievance(w, player);
        let left = consider::contract_days_left(w, player);
        let insecure = if w.players.hot[player].status == PlayerStatus::FreeAgent { 1.0 } else if left < 180 && left >= 0 { 0.5 } else { 0.0 };
        let injured = w.players.hot[player].injury_days > 30;
        stress += grievance * 15.0 + insecure * 18.0 + if injured { 12.0 } else { 0.0 };
        push(MoodFactor::PlayingTime, -grievance * 12.0);
        push(MoodFactor::Contract, -insecure * 10.0);
        if injured {
            push(MoodFactor::Injury, -8.0 * (1.2 - person.hidden.f(Hidden::Pressure) / 20.0));
        }
    }
    if life.finances.debt > life.finances.income.max(500) * 2 {
        stress += 12.0;
        push(MoodFactor::Money, -8.0);
    } else if life.finances.savings > life.finances.spending * 12 {
        push(MoodFactor::Money, 3.0);
    }
    if let Some(pt) = life.partner() {
        let b = f32::from(pt.bond);
        push(MoodFactor::Partner, (b - 55.0) * 0.2 - if pt.lives != life.home { 6.0 } else { 0.0 });
        stress += (55.0 - b).max(0.0) * 0.2;
    }
    let par = life.household.parents;
    if par.alive > 0 && par.health < 40 {
        push(MoodFactor::Family, -6.0);
        stress += 6.0;
    } else {
        push(MoodFactor::Family, (f32::from(r.family) - 4.0).clamp(-4.0, 5.0));
    }
    let settled = consider::settledness(w, who);
    push(MoodFactor::Settling, (settled - 0.7) * 20.0);
    let relief = rest * 0.35 + f32::from(r.hobbies) * 0.4 + f32::from(r.family + r.partner) * 0.25;
    stress = (stress - relief + (20.0 - person.hidden.f(Hidden::Pressure)) * 0.4).clamp(0.0, 100.0);
    push(MoodFactor::Stress, -(stress - 30.0).max(0.0) * 0.3);

    let sleep = (70.0 + (rest - 12.0) * 1.5 - f32::from(r.nightlife) * 3.0 - stress * 0.25
        - if life.household.youngest_born.days_until(today) < 180 && life.household.children > 0 { 15.0 } else { 0.0 })
    .clamp(10.0, 100.0);

    let total: f32 = mood.iter().map(|&(_, v)| f32::from(v)).sum();
    let target = (62.0 + total + (person.hidden.f(Hidden::Professionalism) - 10.0) * 0.5 + rng.normal() * 4.0).clamp(5.0, 100.0);

    let l = &mut w.lives[who];
    l.stress = stress as u8;
    l.sleep = sleep as u8;
    l.wellbeing_why = mood;
    let fulfil_target = if playing { 55.0 + total * 0.5 } else { 45.0 + f32::from(r.study + r.hobbies + r.family) * 0.8 };
    l.fulfilment = ewma(f32::from(l.fulfilment), fulfil_target.clamp(5.0, 100.0), 0.2) as u8;
    if r.study > 0 && l.education < 5 && today.month() == 6 && r.study >= 4 {
        l.education += 1;
        w.events.push(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::Graduated });
    }
    if player.is_some() {
        let h = &mut w.players.hot[player];
        h.wellbeing = ewma(f32::from(h.wellbeing), target, 0.5).clamp(5.0, 100.0) as u8;
    }
}

// ------------------------------------------------------------------ moves

/// A person's working life moves to another nation (transfer, loan, job).
/// Their partner decides for themselves whether to come.
pub fn relocate(w: &mut World, who: PersonId, to: NationId, cause: Cause) {
    if to.is_none() || w.lives[who].home == to {
        return;
    }
    let today = w.date;
    let mut causes = Causes::new();
    causes.push(cause);
    let ev = w.events.push_caused(today, Visibility::Public, EventKind::Life { person: who, kind: LifeEventKind::Relocated { nation: to } }, causes);
    {
        let l = &mut w.lives[who];
        l.home = to;
        l.home_since = today;
        l.stress = l.stress.saturating_add(10).min(100);
    }
    crate::affairs::on_relocate(w, who);
    if let Some(pt) = w.lives[who].household.partner {
        let p = consider::partner_would_move(w, pt.person, to, pt.bond);
        let mut rng = Rng::keyed(&[w.seed, stream::FAMILY, u64::from(pt.person.0), today.0 as u64]);
        let mut cs = Causes::new();
        cs.push(Cause::Event(ev));
        cs.push(Cause::Fact(Fact::Household { person: pt.person }));
        if rng.chance(p) {
            w.lives[pt.person].home = to;
            w.lives[pt.person].home_since = today;
            if let Some(x) = w.lives[who].household.partner.as_mut() {
                x.lives = to;
            }
            if let Some(x) = w.lives[pt.person].household.partner.as_mut() {
                x.lives = to;
            }
            w.events.push_caused(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::PartnerJoinedMove { partner: pt.person } }, cs);
        } else {
            let bond = pt.bond.saturating_sub(10);
            set_bond(w, who, pt.person, bond);
            w.events.push_caused(today, Visibility::Person(who), EventKind::Life { person: who, kind: LifeEventKind::PartnerStayedBehind { partner: pt.person } }, cs);
        }
    }
}

/// Default routine hours for a person's circumstances (used when a life is
/// created and by AI minds); humans set theirs directly.
pub fn baseline_routine() -> Routine {
    Routine::default()
}
