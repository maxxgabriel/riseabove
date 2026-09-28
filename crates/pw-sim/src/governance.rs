//! Owners and boards at work (07 §9–10). Monthly the board reviews money and
//! mood; yearly it sets policy, commissions facilities, and decides how much
//! the owner puts in. Debt breeds austerity and forced sales; deeper
//! insolvency brings administration and points deductions; struggling or
//! promising clubs attract buyers, and a takeover changes everything
//! downstream: budgets, patience, style and who manages the team.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Hidden, Money, PersonId};
use pw_world::club::Ownership;
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::governance::{BoardConcern, Governance, Owner, Policy, Project, ProjectKind, TransferStyle};
use pw_world::{MindKind, NameId, Person, SquadStatus, World};
use smallvec::SmallVec;

use crate::consider;
use crate::generate as gen_;

fn new_person(w: &mut World, club: ClubId, age: (i32, i32), rng: &mut Rng) -> PersonId {
    let nation = w.clubs[club].nation;
    let (first, last) = crate::people::random_name(w, nation, rng);
    let dob = w.date.add_days(-(365 * rng.range_i32(age.0, age.1)));
    w.people.push(Person {
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
    })
}

fn owner_for(kind: Ownership, rep: f32, rng: &mut Rng, person: PersonId, since: pw_core::Date) -> Owner {
    let r = |rng: &mut Rng, m: f32| rng.normal_ms(m, 15.0).clamp(0.0, 100.0) as u8;
    let (wealth_mult, amb, pat, med, fru, fan) = match kind {
        Ownership::MemberOwned => (0.2, 50.0, 60.0, 20.0, 70.0, 85.0),
        Ownership::Private => (0.8, 55.0, 50.0, 40.0, 55.0, 50.0),
        Ownership::Benefactor => (3.0, 70.0, 45.0, 60.0, 30.0, 55.0),
        Ownership::InvestmentGroup => (2.0, 60.0, 40.0, 35.0, 75.0, 30.0),
        Ownership::StateBacked => (12.0, 85.0, 35.0, 50.0, 10.0, 40.0),
    };
    Owner {
        person,
        kind,
        wealth: (20_000_000.0 * wealth_mult * (0.3 + rep * 2.0) * rng.range_f32(0.5, 1.5)) as Money,
        ambition: r(rng, amb + rep * 20.0),
        patience: r(rng, pat),
        meddling: r(rng, med),
        frugality: r(rng, fru),
        fan_sensitivity: r(rng, fan),
        since,
    }
}

fn policy_for(o: &Owner, rep: f32, rng: &mut Rng) -> Policy {
    let style = match (o.kind, o.frugality > 60, rep > 0.6) {
        (Ownership::InvestmentGroup, _, _) => TransferStyle::Value,
        (Ownership::StateBacked, _, _) | (Ownership::Benefactor, _, true) => TransferStyle::WinNow,
        (Ownership::MemberOwned, true, _) => TransferStyle::Homegrown,
        (_, true, false) => TransferStyle::Develop,
        _ => TransferStyle::Balanced,
    };
    Policy {
        wage_cap_mult: match style {
            TransferStyle::WinNow => 4.0,
            TransferStyle::Homegrown | TransferStyle::Develop => 2.2,
            _ => 3.0,
        } * rng.range_f32(0.85, 1.15),
        youth_investment: match style {
            TransferStyle::Homegrown => 70,
            TransferStyle::Develop => 55,
            TransferStyle::WinNow => 20,
            _ => 35,
        },
        transfer_style: style,
        max_signing_age: match style {
            TransferStyle::Develop | TransferStyle::Value => 26,
            TransferStyle::WinNow => 0,
            _ => 30,
        },
        sell_to_rivals: o.frugality > 70,
        debt_tolerance: (1.2 - f32::from(o.frugality) / 100.0).max(0.2),
        style_mandate: rng.range_i32(-1, 2) as i8,
        youth_minutes_target: match style {
            TransferStyle::Homegrown => 25,
            TransferStyle::Develop => 15,
            _ => 5,
        },
        selling_stance: match style {
            TransferStyle::Value | TransferStyle::Develop => 0.85,
            TransferStyle::WinNow => 1.4,
            _ => 1.1,
        },
    }
}

/// Every club gets an owner, a chairman and a policy (worldgen; new clubs later).
pub fn ensure(w: &mut World) {
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|c| !w.governance.contains_key(c)).collect();
    for club in clubs {
        let mut rng = Rng::keyed(&[w.seed, stream::BOARD, u64::from(club.0), 0x60f]);
        let owner_p = new_person(w, club, (40, 75), &mut rng);
        let rep = f32::from(w.clubs[club].reputation) / 10_000.0;
        let kind = w.clubs[club].ownership;
        let since = w.date.add_days(-rng.range_i32(0, 7300));
        let owner = owner_for(kind, rep, &mut rng, owner_p, since);
        let chairman = if kind == Ownership::MemberOwned { new_person(w, club, (45, 70), &mut rng) } else { owner_p };
        let policy = policy_for(&owner, rep, &mut rng);
        w.clubs[club].board.patience = owner.patience;
        let revenue = crate::finance::season_revenue(w, club);
        let academy_budget = revenue * Money::from(policy.youth_investment) / 1000;
        w.governance.insert(
            club,
            Governance { owner, chairman, policy, projects: Vec::new(), administration: None, red_months: 0, concerns: SmallVec::new(), revenue_history: SmallVec::new(), academy_budget, injected: 0 },
        );
    }
}

/// Monthly board review: money, mood, and the owner's temper.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let clubs: Vec<ClubId> = w.governance.keys().copied().collect();
    let mut clubs = clubs;
    clubs.sort();
    for club in clubs {
        let revenue = crate::finance::season_revenue(w, club).max(1);
        let (debt, balance) = (w.clubs[club].finance.debt, w.clubs[club].finance.balance);
        let g = w.governance.get(&club).unwrap().clone();
        let mut concerns: SmallVec<[(BoardConcern, i8); 6]> = SmallVec::new();

        // Finances.
        let ratio = debt as f32 / revenue as f32;
        if ratio > g.policy.debt_tolerance {
            concerns.push((BoardConcern::Finances, -((ratio - g.policy.debt_tolerance) * 20.0).clamp(1.0, 30.0) as i8));
            austerity(w, club, ratio);
        } else if balance > revenue / 2 {
            concerns.push((BoardConcern::Finances, 5));
        }
        // Deep insolvency for months on end means administration.
        let red = balance < -(revenue + revenue / 5);
        let gm = w.governance.get_mut(&club).unwrap();
        gm.red_months = if red { gm.red_months.saturating_add(1) } else { 0 };
        if gm.red_months >= 3 && gm.administration.is_none() {
            administration(w, club);
        }

        // Youth minutes against the board's wish.
        let target = f32::from(g.policy.youth_minutes_target) / 100.0;
        if target > 0.0 {
            let share = academy_minutes_share(w, club);
            concerns.push((BoardConcern::YouthMinutes, ((share - target) * 60.0).clamp(-15.0, 10.0) as i8));
        }
        // Style: does the manager play the way the board wants?
        if let Some(m) = w.clubs[club].manager.get() {
            let gap = (i32::from(w.staff[m].philosophy.mentality) - i32::from(g.policy.style_mandate)).abs();
            if gap >= 2 {
                concerns.push((BoardConcern::Style, -(gap as i8) * 3));
            }
        }
        // Fans.
        let mood = i32::from(w.clubs[club].fan_mood);
        if mood < 30 {
            concerns.push((BoardConcern::FanUnrest, (-(30 - mood) as f32 * f32::from(g.owner.fan_sensitivity) / 100.0) as i8));
        }
        // Owner patience wearing with ambition unmet.
        let gap = f32::from(w.clubs[club].board.target_position);
        if g.owner.ambition > 70 && gap > 0.0 {
            let league = w.clubs[club].league;
            if league.is_some() {
                let pos = w.comps[league].position_of(w.clubs[club].first_team()).unwrap_or(99) as f32;
                if pos > gap + 2.0 {
                    concerns.push((BoardConcern::OwnerPatience, -((pos - gap) as i8).min(20)));
                }
            }
        }
        let total: i32 = concerns.iter().map(|&(_, v)| i32::from(v)).sum();
        let b = &mut w.clubs[club].board;
        b.satisfaction = (i32::from(b.satisfaction) + total / 3).clamp(0, 100) as u8;
        w.governance.get_mut(&club).unwrap().concerns = concerns;
        let _ = today;
    }
}

fn academy_minutes_share(w: &World, club: ClubId) -> f32 {
    let team = w.clubs[club].first_team();
    let (mut total, mut academy) = (0u32, 0u32);
    for &p in &w.teams[team].squad {
        let m = u32::from(w.players.hot[p].minutes_4w);
        total += m;
        if w.players.cold[p].youth_club == club {
            academy += m;
        }
    }
    if total == 0 { 0.0 } else { academy as f32 / total as f32 }
}

/// Debt beyond tolerance: budgets cut, high earners and surplus made available.
fn austerity(w: &mut World, club: ClubId, ratio: f32) {
    let today = w.date;
    if w.events.since(today.add_days(-90)).iter().any(|e| matches!(e.kind, EventKind::Austerity { club: c } if c == club)) {
        return;
    }
    let f = &mut w.clubs[club].finance;
    f.wage_budget = f.wage_budget * 85 / 100;
    f.transfer_budget = 0;
    let team = w.clubs[club].first_team();
    let mut wages: Vec<(pw_core::PlayerId, Money)> = w.teams[team].squad.iter().map(|&p| (p, w.players.cold[p].contract.current_wage(today))).collect();
    wages.sort_by_key(|&(_, x)| std::cmp::Reverse(x));
    let median = wages.get(wages.len() / 2).map_or(0, |x| x.1);
    for &(p, wage) in wages.iter().take(4) {
        if wage > median * 2 && w.age(p) >= 27 {
            w.market.listed.insert(p, today);
        }
    }
    let causes: Causes = pw_world::causes![Cause::Fact(Fact::BoardPressure { club, warnings: (ratio * 10.0).min(255.0) as u8 })];
    w.events.push_caused(today, Visibility::Public, EventKind::Austerity { club }, causes);
}

fn administration(w: &mut World, club: ClubId) {
    let today = w.date;
    w.governance.get_mut(&club).unwrap().administration = Some(today);
    let ev = w.events.push(today, Visibility::Public, EventKind::Administration { club });
    // Points deduction in the current league season.
    let league = w.clubs[club].league;
    let team = w.clubs[club].first_team();
    if league.is_some()
        && let Some(r) = w.comps[league].state.table.iter_mut().find(|r| r.team == team)
    {
        r.points -= 10;
        w.events.push_caused(today, Visibility::Public, EventKind::PointsDeducted { club, points: 10 }, pw_world::causes![Cause::Event(ev)]);
    }
    // Everyone of value is for sale; wages are frozen.
    let squad = w.teams[team].squad.clone();
    for p in squad {
        w.market.listed.insert(p, today);
    }
    let f = &mut w.clubs[club].finance;
    f.transfer_budget = 0;
    f.wage_budget = f.wage_bill * 80 / 100;
}

/// Yearly: owner money, facilities, projects, policy drift, takeovers.
pub fn yearly(w: &mut World) {
    let today = w.date;
    let year = today.year() as u64;
    let clubs: Vec<ClubId> = {
        let mut v: Vec<ClubId> = w.governance.keys().copied().collect();
        v.sort();
        v
    };
    for club in clubs {
        let mut rng = Rng::keyed(&[w.seed, stream::BOARD, u64::from(club.0), year]);
        let revenue = crate::finance::season_revenue(w, club);
        {
            let g = w.governance.get_mut(&club).unwrap();
            g.revenue_history.push(revenue);
            if g.revenue_history.len() > 5 {
                g.revenue_history.remove(0);
            }
            g.academy_budget = revenue * Money::from(g.policy.youth_investment) / 1000;
        }
        complete_projects(w, club);
        invest(w, club, revenue, &mut rng);
        commission(w, club, revenue, &mut rng);
        maybe_takeover(w, club, revenue, &mut rng);
        recover_from_administration(w, club);
    }
}

fn invest(w: &mut World, club: ClubId, revenue: Money, rng: &mut Rng) {
    let today = w.date;
    let g = w.governance.get(&club).unwrap().clone();
    let o = &g.owner;
    let willing = match o.kind {
        Ownership::StateBacked | Ownership::Benefactor => 1.0,
        Ownership::Private => 0.3,
        Ownership::InvestmentGroup => 0.15,
        Ownership::MemberOwned => 0.0,
    } * f32::from(o.ambition)
        / 100.0
        * (1.0 - f32::from(o.frugality) / 150.0);
    if willing <= 0.05 || !rng.chance(willing) {
        return;
    }
    let amount = ((o.wealth as f32 * rng.range_f32(0.03, 0.12)) as Money).min(revenue * 2);
    if amount <= 0 {
        return;
    }
    let gm = w.governance.get_mut(&club).unwrap();
    gm.owner.wealth -= amount;
    gm.injected += amount;
    let f = &mut w.clubs[club].finance;
    f.balance += amount;
    f.transfer_budget += amount * 2 / 3;
    w.events.push(today, Visibility::Public, EventKind::OwnerInvestment { club, amount });
}

fn commission(w: &mut World, club: ClubId, revenue: Money, rng: &mut Rng) {
    let today = w.date;
    if !w.governance[&club].projects.is_empty() {
        return;
    }
    let balance = w.clubs[club].finance.balance;
    let fac = w.clubs[club].facilities;
    let youth = w.governance[&club].policy.youth_investment;
    let rep = w.clubs[club].reputation;
    // What the club most lacks relative to its standing and priorities.
    let expected = (4.0 + f32::from(rep) / 650.0).round() as i32;
    let mut options: Vec<(ProjectKind, i32)> = vec![
        (ProjectKind::Training, expected - i32::from(fac.training)),
        (ProjectKind::Youth, expected - i32::from(fac.youth) + i32::from(youth) / 25),
        (ProjectKind::AcademyNetwork, expected - i32::from(fac.academy) + i32::from(youth) / 30),
        (ProjectKind::Medical, expected - i32::from(fac.medical)),
    ];
    let demand = f32::from(w.clubs[club].fan_mood) / 100.0 * (0.5 + f32::from(rep) / 10_000.0);
    if demand > 0.6 && w.clubs[club].capacity < 20_000 + u32::from(rep) * 8 {
        options.push((ProjectKind::Stadium, 3));
    }
    options.sort_by_key(|&(_, gap)| std::cmp::Reverse(gap));
    let Some(&(kind, gap)) = options.first() else { return };
    if gap < 1 {
        return;
    }
    let cost = match kind {
        ProjectKind::Stadium => revenue * 2,
        _ => revenue / 5 * (1 + gap as Money),
    };
    if balance < cost / 2 || !rng.chance(0.5) {
        return;
    }
    let years = if kind == ProjectKind::Stadium { rng.range_i32(2, 3) } else { rng.range_i32(1, 2) };
    let target = match kind {
        ProjectKind::Training => u32::from(fac.training) + 1,
        ProjectKind::Youth => u32::from(fac.youth) + 1,
        ProjectKind::AcademyNetwork => u32::from(fac.academy) + 1,
        ProjectKind::Medical => u32::from(fac.medical) + 1,
        ProjectKind::Stadium => (w.clubs[club].capacity / 5).max(3000),
    };
    w.clubs[club].finance.balance -= cost;
    w.governance.get_mut(&club).unwrap().projects.push(Project { kind, target, cost, started: today, completes: today.add_months(12 * years) });
    w.events.push(today, Visibility::Public, EventKind::ProjectStarted { club, kind });
}

fn complete_projects(w: &mut World, club: ClubId) {
    let today = w.date;
    let done: Vec<Project> = {
        let g = w.governance.get_mut(&club).unwrap();
        let (done, rest): (Vec<Project>, Vec<Project>) = g.projects.drain(..).partition(|p| p.completes <= today);
        g.projects = rest;
        done
    };
    for p in done {
        let c = &mut w.clubs[club];
        let lvl = p.target.min(20) as u8;
        match p.kind {
            ProjectKind::Training => c.facilities.training = lvl,
            ProjectKind::Youth => c.facilities.youth = lvl,
            ProjectKind::AcademyNetwork => c.facilities.academy = lvl,
            ProjectKind::Medical => c.facilities.medical = lvl,
            ProjectKind::Stadium => c.capacity += p.target,
        }
        w.events.push(today, Visibility::Public, EventKind::ProjectCompleted { club, kind: p.kind });
    }
}

/// Clubs change hands: distressed clubs attract rescuers, clubs in rich
/// leagues with big fanbases attract investors.
fn maybe_takeover(w: &mut World, club: ClubId, revenue: Money, rng: &mut Rng) {
    let today = w.date;
    let g = w.governance[&club].clone();
    if g.owner.kind == Ownership::MemberOwned {
        return;
    }
    let distressed = w.clubs[club].finance.debt as f32 / revenue.max(1) as f32 > 1.0 || g.administration.is_some();
    let league_rich = w.economy.nations.get(&w.clubs[club].nation).map_or(0.0, |e| e.league_strength);
    let appeal = f32::from(w.clubs[club].reputation) / 10_000.0 * 0.5 + league_rich * 0.4;
    let owner_age = consider::age(w, g.owner.person);
    let p = if distressed { 0.25 } else { 0.02 + appeal * 0.04 } + if owner_age > 75.0 { 0.05 } else { 0.0 };
    if !rng.chance(p) {
        return;
    }
    let kind = match rng.f32() {
        x if x < 0.35 => Ownership::InvestmentGroup,
        x if x < 0.65 => Ownership::Private,
        x if x < 0.92 => Ownership::Benefactor,
        _ => Ownership::StateBacked,
    };
    let person = new_person(w, club, (38, 70), rng);
    let rep = f32::from(w.clubs[club].reputation) / 10_000.0;
    let owner = owner_for(kind, rep, rng, person, today);
    let policy = policy_for(&owner, rep, rng);
    let previous = g.owner.person;
    let rescue = (w.clubs[club].finance.debt).min(owner.wealth / 3);
    {
        let gm = w.governance.get_mut(&club).unwrap();
        gm.owner = owner;
        gm.chairman = person;
        gm.policy = policy;
        gm.administration = None;
        gm.red_months = 0;
        gm.injected += rescue;
    }
    let c = &mut w.clubs[club];
    c.ownership = kind;
    c.finance.balance += rescue;
    c.board.patience = w.governance[&club].owner.patience;
    // New owners bring new expectations: the board resets its view of the manager.
    c.board.satisfaction = 50;
    let causes: Causes = if distressed { pw_world::causes![Cause::Fact(Fact::BoardPressure { club, warnings: 0 })] } else { Causes::new() };
    w.events.push_caused(today, Visibility::Public, EventKind::Takeover { club, owner: person, previous }, causes);
    // Ambitious new owners often want their own manager.
    if let Some(m) = w.clubs[club].manager.get() {
        let meddling = w.governance[&club].owner.meddling;
        if rng.chance(f32::from(meddling) / 150.0) {
            let mgr = w.staff[m].person;
            let compat = consider::compat(w, person, mgr);
            w.social.adjust(person, mgr, today, compat, -10, -15, 0);
            w.clubs[club].board.satisfaction = 20;
        }
    }
}

fn recover_from_administration(w: &mut World, club: ClubId) {
    let today = w.date;
    let Some(since) = w.governance[&club].administration else { return };
    if since.days_until(today) >= 365 && w.clubs[club].finance.balance >= 0 {
        w.governance.get_mut(&club).unwrap().administration = None;
        let team = w.clubs[club].first_team();
        let squad = w.teams[team].squad.clone();
        for p in squad {
            if w.players.cold[p].status != SquadStatus::NotNeeded {
                w.market.listed.remove(&p);
            }
        }
    }
}

/// How eager a club is to sell (multiplies asking prices).
pub fn selling_stance(w: &World, club: ClubId) -> f32 {
    w.governance.get(&club).map_or(1.0, |g| {
        let debt = w.clubs[club].finance.debt as f32 / crate::finance::season_revenue(w, club).max(1) as f32;
        let pressure = (debt - g.policy.debt_tolerance).max(0.0) * 0.3 + if g.administration.is_some() { 0.3 } else { 0.0 };
        (g.policy.selling_stance - pressure).clamp(0.5, 1.8)
    })
}

/// Highest wage the board will sanction for one player at this club.
pub fn wage_ceiling(w: &World, club: ClubId) -> Money {
    let Some(g) = w.governance.get(&club) else { return Money::MAX };
    let team = w.clubs[club].first_team();
    let mut wages: Vec<Money> = w.teams[team].squad.iter().map(|&p| w.players.cold[p].contract.current_wage(w.date)).filter(|&x| x > 0).collect();
    if wages.is_empty() {
        return Money::MAX;
    }
    wages.sort_unstable();
    let median = wages[wages.len() / 2] as f32;
    (median * g.policy.wage_cap_mult) as Money
}

/// Owners and chairmen judge managers through their own temperament too.
pub fn owner_temper(w: &World, club: ClubId) -> f32 {
    w.governance.get(&club).map_or(1.0, |g| {
        let chair = &w.people[g.chairman];
        (1.0 + (10.0 - chair.hidden.f(Hidden::Temperament)) * 0.03 + f32::from(g.owner.ambition) / 400.0).clamp(0.7, 1.5)
    })
}
