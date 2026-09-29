//! Who decides a signing, and what the club remembers of it (locked design 3.6, 3.7, 3.19-3.24).
//!
//! A signing is an organisational decision. Each voice (manager, sporting director, head of recruitment, owner, captain, analysts,
//! supporters) forms a stance from what it believes and cares about; how much each counts depends on how the club is actually run,
//! and on the standing each voice has earned or lost through earlier decisions. The bar the result must clear moves with the club's
//! appetite for risk, which itself follows ownership, money, results and recent luck. Important deals leave a case file, and months
//! later are judged on the process as well as the outcome: a risk the club saw and accepted is not a risk it never noticed.

use pw_core::{ClubId, Hidden, Money, PersonId, PlayerId, PosGroup, StaffId};
use pw_world::boardroom::{Appetite, Case, CaseState, Driver, Outcome, Stance, Structure, Verdict, Voice, Why};
use pw_world::dossier::{Confidence, Dossier, RiskKind};
use pw_world::{EventKind, MemoryKind, PlayerStatus, SquadStatus, StaffRole, TeamKind, Visibility, World};
use smallvec::SmallVec;

use crate::{dossier, market, scouting};

/// A signing this expensive (a share of the buyer's yearly revenue), or a highly rated youngster, is kept as a case file.
const IMPORTANT_SHARE: f64 = 0.02;

/// Cases older than this are dropped once judged.
const KEEP_DAYS: i32 = 4 * 365;

fn director(w: &World, club: ClubId) -> Option<StaffId> {
    w.clubs[club].staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::DirectorOfFootball && !w.staff[s].retired)
}

fn analyst(w: &World, club: ClubId) -> Option<StaffId> {
    w.clubs[club].staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::Analyst && !w.staff[s].retired)
}

fn scouts(w: &World, club: ClubId) -> Vec<StaffId> {
    w.clubs[club].staff.iter().copied().filter(|&s| w.staff[s].role == StaffRole::Scout && !w.staff[s].retired).collect()
}

/// How football decisions are actually made at this club.
pub fn structure(w: &World, club: ClubId) -> Structure {
    let Some(g) = w.governance.get(&club) else { return Structure::ManagerLed };
    if g.owner.meddling >= 70 {
        Structure::OwnerLed
    } else if g.policy.transfer_style == pw_world::governance::TransferStyle::Value && analyst(w, club).is_some() {
        Structure::DataLed
    } else if director(w, club).is_some() {
        if scouts(w, club).len() >= 3 && w.clubs[club].reputation >= 6000 { Structure::Committee } else { Structure::DirectorLed }
    } else {
        Structure::ManagerLed
    }
}

/// How much each voice counts for a signing at this club, summing to 1 over the voices that exist.
pub fn powers(w: &World, club: ClubId, present: &[bool; 7]) -> [f32; 7] {
    // Manager, Director, Recruitment, Owner, Captain, Analyst, Supporters
    let base: [f32; 7] = match structure(w, club) {
        Structure::ManagerLed => [0.50, 0.10, 0.10, 0.10, 0.05, 0.05, 0.10],
        Structure::DirectorLed => [0.20, 0.40, 0.15, 0.10, 0.03, 0.07, 0.05],
        Structure::OwnerLed => [0.10, 0.10, 0.05, 0.55, 0.03, 0.02, 0.15],
        Structure::Committee => [0.20, 0.25, 0.25, 0.10, 0.03, 0.07, 0.10],
        Structure::DataLed => [0.15, 0.20, 0.10, 0.08, 0.02, 0.35, 0.10],
    };
    let meddling = w.governance.get(&club).map_or(0.0, |g| f32::from(g.owner.meddling) / 100.0);
    let mut p = [0.0f32; 7];
    for v in Voice::ALL {
        let i = v.idx();
        if !present[i] {
            continue;
        }
        let mut x = base[i] + w.boardroom.authority_of(club, v);
        if v == Voice::Owner {
            x += meddling * 0.2;
        }
        p[i] = x.max(0.01);
    }
    let total: f32 = p.iter().sum();
    if total > 0.0 {
        for x in &mut p {
            *x /= total;
        }
    }
    p
}

/// A person's own taste for a gamble, 0 (cautious) .. 1 (aggressive), from who they are (section 3.22).
pub fn tendency(w: &World, person: PersonId) -> f32 {
    if person.is_none() {
        return 0.5;
    }
    let h = &w.people[person].hidden;
    ((h.f(Hidden::Ambition) + h.f(Hidden::Controversy) - 0.3 * h.f(Hidden::Professionalism)) / 34.0).clamp(0.0, 1.0)
}

fn owner_tendency(w: &World, club: ClubId) -> f32 {
    w.governance.get(&club).map_or(0.5, |g| (f32::from(g.owner.ambition) / 100.0 * 0.5 + (1.0 - f32::from(g.owner.frugality) / 100.0) * 0.5).clamp(0.0, 1.0))
}

/// The club's present willingness to gamble, from its circumstances (section 3.21). Not stored by the caller; see [`monthly`].
pub fn appetite(w: &World, club: ClubId, trophies: u32) -> Appetite {
    let today = w.date;
    let mut d: SmallVec<[(Driver, f32); 6]> = SmallVec::new();
    let mut add = |driver: Driver, v: f32| {
        if v.abs() >= 0.01 {
            d.push((driver, v));
        }
    };
    let revenue = crate::finance::season_revenue(w, club).max(1) as f32;
    let f = &w.clubs[club].finance;
    add(Driver::Cash, ((f.balance as f32 / revenue) - 0.5).clamp(-0.6, 1.0) * 0.16);
    if let Some(g) = w.governance.get(&club) {
        if g.owner.since.days_until(today) < 365 {
            add(Driver::NewOwner, 0.10 + f32::from(g.owner.ambition) / 100.0 * 0.06);
        }
        if g.injected > 0 && w.clubs[club].finance.balance > 0 {
            add(Driver::Investment, ((g.injected as f32 / revenue).min(2.0)) * 0.05);
        }
        let debt = f.debt as f32 / (revenue * g.policy.debt_tolerance.max(0.1));
        add(Driver::Debt, -(debt.min(1.5)) * 0.14);
    }
    let rec = w.boardroom.record.get(&club).copied().unwrap_or_default();
    add(Driver::RecentFlops, -f32::from(rec.flops.min(3)) * 0.09);
    add(Driver::RecentSuccess, f32::from(rec.hits.min(3)) * 0.04);
    let mgr_tenure = w.clubs[club].manager.get().map_or(0.0, |m| w.staff[m].joined.days_until(today) as f32 / 365.0);
    let dir_tenure = director(w, club).map_or(mgr_tenure, |s| w.staff[s].joined.days_until(today) as f32 / 365.0);
    add(Driver::Tenure, ((mgr_tenure.min(dir_tenure)).min(4.0) - 1.0) * 0.02);
    let board = w.clubs[club].board;
    add(Driver::BoardPressure, -((50.0 - f32::from(board.satisfaction)).max(0.0) / 50.0) * 0.14);
    if let Some(g) = w.governance.get(&club)
        && let Some(&(_, fan)) = g.concerns.iter().find(|(c, _)| *c == pw_world::governance::BoardConcern::FanUnrest)
    {
        add(Driver::SupporterMood, f32::from(fan) / 30.0 * 0.06);
    }
    let league = w.clubs[club].league;
    if league.is_some() {
        let comp = &w.comps[league];
        let team = w.clubs[club].first_team();
        if let Some(pos) = comp.position_of(team) {
            let n = comp.state.entrants.len().max(1);
            if pos <= 2 && comp.state.table.iter().any(|r| r.played >= 10) {
                add(Driver::LeaguePosition, 0.07);
            } else if usize::from(comp.relegate) > 0 && pos + usize::from(comp.relegate) > n && comp.state.table.iter().any(|r| r.played >= 10) {
                add(Driver::LeaguePosition, -0.06);
            }
        }
    }
    add(Driver::Trophies, (trophies.min(2) as f32) * 0.04);
    let left = w.nations[w.clubs[club].nation].season.windows.iter().find(|&&(a, b)| today >= a && today <= b).map_or(99, |&(_, b)| today.days_until(b));
    if left <= 10 {
        add(Driver::WindowTiming, 0.05);
    }
    let level = (0.5 + d.iter().map(|x| x.1).sum::<f32>()).clamp(0.05, 0.95);
    Appetite { level, drivers: d, as_of: today }
}

/// The weakest first-choice player of a position group as the club reads its own squad.
fn starter_bar(w: &World, club: ClubId, group: PosGroup) -> f32 {
    let team = w.clubs[club].first_team();
    let starters = match group {
        PosGroup::Gk => 1,
        PosGroup::Def => 4,
        PosGroup::Mid => 3,
        PosGroup::Att => 3,
    };
    let mut cas: Vec<f32> = w.teams[team].squad.iter().filter(|&&q| w.players.cold[q].best_pos.group() == group && w.players.hot[q].status == PlayerStatus::Active).map(|&q| scouting::view(w, club, q).0).collect();
    cas.sort_by(|a, b| b.total_cmp(a));
    cas.get(starters - 1).copied().or(cas.last().copied()).unwrap_or(60.0)
}

/// How worrying the known risks are to someone of a given taste for gambles, given the club's appetite.
fn risk_penalty(d: &Dossier, tendency: f32, appetite: f32) -> f32 {
    let sum: f32 = d.risks.iter().filter(|r| r.known).map(|r| r.level).sum();
    sum * (1.2 - appetite) * (1.3 - tendency) * 0.6
}

struct Setting<'a> {
    club: ClubId,
    p: PlayerId,
    group: PosGroup,
    dossier: &'a Dossier,
    bar: f32,
    fee: Money,
    fair: Money,
    appetite: f32,
    patience: f32,
    alternatives: u8,
    rank: Option<usize>,
}

fn manager_stance(w: &World, s: &Setting) -> Option<Stance> {
    let m = w.clubs[s.club].manager.get()?;
    let person = w.staff[m].person;
    let worth = dossier::worth(w, s.club, s.p, s.patience).unwrap_or(s.dossier.current.mid);
    let fit = crate::managers::wants(w, s.club, s.p).clamp(0.0, 1.0);
    // A player who will take months to be useful is worth less to a manager who needs someone now (section 4.22), and a signing that
    // fits the way he wants to play is worth more (section 4.11).
    let wait = (crate::adaptation::readiness(w, s.club, s.p).weeks_to_useful / 26.0).min(1.0) * crate::adaptation::impatience(w, s.club, s.p);
    let support = (worth - s.bar) / 8.0 + 0.3 * fit + 0.25 * dossier::system_fit(w, s.club, s.p) - 0.5 * wait - risk_penalty(s.dossier, tendency(w, person), s.appetite);
    Some(Stance { voice: Voice::Manager, who: person, support: support.clamp(-1.0, 1.0), power: 0.0, why: Why::FootballValue })
}

fn director_stance(w: &World, s: &Setting) -> Option<Stance> {
    let dir = director(w, s.club)?;
    let person = w.staff[dir].person;
    let value = if s.fair > 0 { (s.fair as f32 - s.fee as f32 * 1.25) / s.fair as f32 } else { 0.0 };
    let young = w.age(s.p) <= 23;
    let resale = if young { ((s.dossier.ceiling.mid - s.dossier.current.mid) / 40.0).clamp(0.0, 0.6) } else { 0.0 };
    let style = w.governance.get(&s.club).map_or(pw_world::governance::TransferStyle::Balanced, |g| g.policy.transfer_style);
    let taste = match style {
        pw_world::governance::TransferStyle::Develop | pw_world::governance::TransferStyle::Homegrown => 1.0,
        pw_world::governance::TransferStyle::WinNow => 0.0,
        _ => 0.5,
    };
    let support = 0.8 * value + resale * taste + (s.dossier.current.mid - s.bar) / 16.0 - risk_penalty(s.dossier, tendency(w, person), s.appetite);
    let why = if value.abs() > resale { Why::Price } else { Why::FootballValue };
    Some(Stance { voice: Voice::Director, who: person, support: support.clamp(-1.0, 1.0), power: 0.0, why })
}

fn recruitment_stance(w: &World, s: &Setting) -> Option<Stance> {
    let head = scouts(w, s.club).into_iter().max_by(|&a, &b| w.staff[a].reputation.cmp(&w.staff[b].reputation).then(b.0.cmp(&a.0)))?;
    let person = w.staff[head].person;
    let grade = w.scouting.of(s.club, s.p).last().map_or(0.0, |r| (f32::from(r.grade) - 3.0) / 3.0);
    let place = match s.rank {
        Some(0) => 0.6,
        Some(1) => 0.2,
        Some(_) => -0.3,
        None => -0.1,
    };
    Some(Stance { voice: Voice::Recruitment, who: person, support: (grade * 0.6 + place).clamp(-1.0, 1.0), power: 0.0, why: if s.alternatives > 0 && place < 0.0 { Why::Alternative } else { Why::FootballValue } })
}

fn owner_stance(w: &World, s: &Setting) -> Option<Stance> {
    let g = w.governance.get(&s.club)?;
    let fame = crate::attention::commercial_appeal(w, s.p);
    let revenue = crate::finance::season_revenue(w, s.club).max(1) as f32;
    let means = (w.clubs[s.club].finance.balance.max(0) as f32).max(revenue * 0.2);
    let cost = (s.fee as f32 / means).min(2.0);
    let frugal = f32::from(g.owner.frugality) / 100.0;
    let marquee = if w.age(s.p) <= 29 { 1.0 } else { 0.6 };
    let football = f32::from(g.owner.ambition) / 100.0 * ((s.dossier.current.mid - s.bar) / 12.0).clamp(-1.0, 1.0);
    let support = 1.1 * fame * marquee - frugal * cost * 0.8 + football * 0.4 - (1.0 - owner_tendency(w, s.club)) * s.dossier.risks.iter().filter(|r| r.known).map(|r| r.level).sum::<f32>() * 0.3;
    Some(Stance { voice: Voice::Owner, who: g.owner.person, support: support.clamp(-1.0, 1.0), power: 0.0, why: Why::Commercial })
}

fn captain_stance(w: &World, s: &Setting) -> Option<Stance> {
    let team = w.clubs[s.club].first_team();
    let cap = w.teams[team].captain.get()?;
    let person = w.players.cold[cap].person;
    if w.people[person].mind != pw_world::MindKind::Ai {
        return None;
    }
    let same_group = w.players.cold[cap].best_pos.group() == s.group;
    let better = market::public_view(w, s.p).0 > market::public_view(w, cap).0 + 3.0;
    let wage = market::wage_demand(w, s.p, s.club);
    let hierarchy = wage as f32 > w.players.cold[cap].contract.current_wage(w.date) as f32 * 1.3;
    let mut support: f32 = 0.05;
    let mut why = Why::FootballValue;
    if same_group && better {
        support -= 0.45;
        why = Why::RoleCompetition;
    }
    if hierarchy {
        support -= 0.3;
        if support < -0.3 {
            why = Why::WageHierarchy;
        }
    }
    Some(Stance { voice: Voice::Captain, who: person, support: support.clamp(-1.0, 0.4), power: 0.0, why })
}

fn analyst_stance(w: &World, s: &Setting) -> Option<Stance> {
    let a = analyst(w, s.club)?;
    // The analysts speak only when there are numbers to speak from.
    if s.dossier.evidence.minutes_seen < 300 {
        return None;
    }
    let person = w.staff[a].person;
    let support = (s.dossier.current.mid - s.bar) / 8.0 - s.dossier.current.band / 30.0 - risk_penalty(s.dossier, 0.3, s.appetite) * 0.5;
    Some(Stance { voice: Voice::Analyst, who: person, support: support.clamp(-1.0, 1.0), power: 0.0, why: Why::Data })
}

fn supporters_stance(w: &World, s: &Setting) -> Stance {
    let c = &w.players.cold[s.p];
    let image = f32::from(w.media.image.get(&c.person).copied().unwrap_or(0)) / 1000.0;
    let fame = f32::from(c.rep.world) / 10_000.0;
    let unrest = w.governance.get(&s.club).and_then(|g| g.concerns.iter().find(|(k, _)| *k == pw_world::governance::BoardConcern::FanUnrest).map(|x| f32::from(x.1))).unwrap_or(0.0) / 30.0;
    // Angry fans want something done; content ones are not fussed.
    let support = 0.8 * image + (fame - 0.25) * 0.6 - unrest * 0.2;
    Stance { voice: Voice::Supporters, who: PersonId::NONE, support: support.clamp(-1.0, 1.0), power: 0.0, why: Why::PublicMood }
}

/// What the club's people made of a target: every stance, the power-weighted result, the bar it faced, and the verdict.
pub struct Decision {
    pub dossier: Dossier,
    pub group: PosGroup,
    pub stances: SmallVec<[Stance; 7]>,
    pub score: f32,
    pub threshold: f32,
    pub approved: bool,
    pub owner_veto: bool,
    pub champion: Voice,
    pub overruled: SmallVec<[Voice; 3]>,
    pub known: SmallVec<[RiskKind; 4]>,
    pub unknown: SmallVec<[RiskKind; 4]>,
    pub alternatives: u8,
    pub bar: f32,
    pub fair: Money,
    pub appetite: f32,
}

/// Every voice takes a stance, institutional power turns them into a decision at the club's present appetite for risk. Pure: nothing
/// is recorded. `None` when the club has no way to form a view at all.
pub fn decide(w: &World, buyer: ClubId, p: PlayerId, group: Option<PosGroup>, opening: Money) -> Option<Decision> {
    let today = w.date;
    let group = group.unwrap_or_else(|| w.players.cold[p].best_pos.group());
    let d = w.dossiers.get(buyer, p).cloned().or_else(|| dossier::build(w, buyer, p))?;
    let fair = market::fair_value(w, buyer, p);
    let appetite = w.boardroom.appetite_of(buyer);
    let board = w.clubs[buyer].board;
    let danger = w.clubs[buyer].league.is_some() && board.satisfaction < 35;
    let patience = if danger { 0.5 } else if board.satisfaction < 50 { 0.75 } else { 1.0 };
    let (alternatives, rank) = group_lists(w, buyer, group, p, today);
    let bar = starter_bar(w, buyer, group);
    let set = Setting { club: buyer, p, group, dossier: &d, bar, fee: opening, fair, appetite, patience, alternatives, rank };

    let mut stances: SmallVec<[Stance; 7]> = SmallVec::new();
    let mut present = [false; 7];
    for s in [manager_stance(w, &set), director_stance(w, &set), recruitment_stance(w, &set), owner_stance(w, &set), captain_stance(w, &set), analyst_stance(w, &set), Some(supporters_stance(w, &set))].into_iter().flatten() {
        present[s.voice.idx()] = true;
        stances.push(s);
    }
    if stances.is_empty() {
        return None;
    }
    let pw = powers(w, buyer, &present);
    for s in &mut stances {
        s.power = pw[s.voice.idx()];
    }
    let score: f32 = stances.iter().map(|s| s.support * s.power).sum();
    let known: SmallVec<[RiskKind; 4]> = d.risks.iter().filter(|r| r.known && r.level >= 0.25).map(|r| r.kind).collect();
    let unknown: SmallVec<[RiskKind; 4]> = d.risks.iter().filter(|r| !r.known).map(|r| r.kind).collect();
    let worst = d.risks.iter().filter(|r| r.known).map(|r| r.level).fold(0.0f32, f32::max);
    let threshold = 0.05 - 0.30 * (appetite - 0.5) + 0.10 * worst;
    // An owner who meddles and is firmly against can stop a deal outright.
    let owner_veto = stances.iter().any(|s| s.voice == Voice::Owner && s.support < -0.6) && w.governance.get(&buyer).is_some_and(|g| g.owner.meddling >= 60);
    let approved = score >= threshold && !owner_veto;
    let champion = stances.iter().filter(|s| s.support > 0.0).max_by(|a, b| (a.support * a.power).total_cmp(&(b.support * b.power))).map_or(Voice::Manager, |s| s.voice);
    let overruled: SmallVec<[Voice; 3]> = if approved { stances.iter().filter(|s| s.support < -0.4).map(|s| s.voice).take(3).collect() } else { SmallVec::new() };
    Some(Decision { dossier: d, group, stances, score, threshold, approved, owner_veto, champion, overruled, known, unknown, alternatives, bar, fair, appetite })
}

/// The club looks at a target before it approaches, and an important decision is kept as a case file. Returns whether the approach
/// goes ahead.
pub fn consider_target(w: &mut World, buyer: ClubId, p: PlayerId, group: Option<PosGroup>, opening: Money, budget: Money) -> bool {
    let today = w.date;
    let Some(x) = decide(w, buyer, p, group, opening) else { return true };
    let d = &x.dossier;
    let planned = group.is_some();
    // A club that has just lost someone it expected to keep, and buys in that position soon after, is in a hurry (section 4.23).
    let panic = w.boardroom.scramble.get(&buyer).is_some_and(|&(g, when)| g == x.group && when.days_until(today) <= 90);
    let revenue = crate::finance::season_revenue(w, buyer).max(1) as f64;
    let important = opening as f64 >= revenue * IMPORTANT_SHARE || (w.age(p) <= 21 && d.ceiling.mid >= d.current.mid + 15.0 && opening as f64 >= revenue * 0.005);
    if important {
        let role = if d.current.mid >= x.bar + 8.0 { SquadStatus::Important } else if d.current.mid >= x.bar { SquadStatus::Regular } else { SquadStatus::Squad };
        let seller = w.players.hot[p].club;
        let price_high = (budget as f64).min(x.fair as f64 * 1.35) as Money;
        let case = Case {
            buyer,
            seller,
            player: p,
            date: today,
            structure: structure(w, buyer),
            stances: x.stances.clone(),
            score: x.score,
            threshold: x.threshold,
            state: if x.approved { CaseState::Pursuing } else { CaseState::Declined },
            champion: x.champion,
            overruled: x.overruled.clone(),
            current: d.current,
            ceiling: d.ceiling,
            confidence: d.current_confidence,
            risks_accepted: if x.approved { x.known.clone() } else { SmallVec::new() },
            risks_known: x.known.clone(),
            risks_unknown: x.unknown.clone(),
            role,
            alternatives: x.alternatives,
            price_low: (x.fair as f64 * 0.75) as Money,
            price_high,
            price_paid: 0,
            appetite: x.appetite,
            signed: None,
            outcome: None,
            covers: PlayerId::NONE,
            apps_at: w.players.cold[p].senior_apps,
            injuries_at: w.players.cold[p].injuries_career,
            planned,
            panic,
        };
        w.boardroom.cases.push(case);
    }
    x.approved
}

/// How many other targets the club has for the need, and where this one stands among them.
fn group_lists(w: &World, club: ClubId, group: PosGroup, p: PlayerId, today: pw_core::Date) -> (u8, Option<usize>) {
    let Some(list) = w.deals.shortlists.get(&(club, group)) else { return (0, None) };
    let alt = list.targets.iter().filter(|(q, _)| *q != p && !w.market.on_cooldown(club, *q, today)).count();
    (alt.min(255) as u8, list.targets.iter().position(|(q, _)| *q == p))
}

/// The deal was completed: note the price and what came with the arrival.
pub fn on_signed(w: &mut World, buyer: ClubId, p: PlayerId, fee: Money) {
    let today = w.date;
    let Some(i) = w.boardroom.case_for(buyer, p) else { return };
    if w.boardroom.cases[i].state != CaseState::Pursuing || w.boardroom.cases[i].date.days_until(today) > 240 {
        return;
    }
    {
        let c = &mut w.boardroom.cases[i];
        c.state = CaseState::Signed;
        c.signed = Some(today);
        c.price_paid = fee;
        c.apps_at = w.players.cold[p].senior_apps;
        c.injuries_at = w.players.cold[p].injuries_career;
    }
    note_blocked(w, buyer, p);
    // A senior player who opposed the signing carries it into the dressing room: friction, not a veto (section 3.19).
    let newcomer = w.players.cold[p].person;
    let c = w.boardroom.cases[i].clone();
    for s in c.stances.iter().filter(|s| s.voice == Voice::Captain && s.support < -0.2 && s.who.is_some()) {
        w.social.remember(s.who, newcomer, MemoryKind::Rivalry, today, pw_core::EventId::NONE, false, 0.4, 0);
    }
    // The manager who was overruled remembers who forced it through.
    if let (Some(m), Some(d)) = (w.clubs[buyer].manager.get(), director(w, buyer))
        && c.overruled.contains(&Voice::Manager)
        && c.champion == Voice::Director
    {
        w.social.remember(w.staff[m].person, w.staff[d].person, MemoryKind::Argument, today, pw_core::EventId::NONE, false, 0.5, 0);
    }
}

/// A contract runs out and the player walks. If the club had counted on keeping a key man, its plan has failed: it is remembered, and the
/// scramble to replace him marks whatever it signs next in that position (section 4.23).
pub fn on_expiry(w: &mut World, p: PlayerId) {
    let today = w.date;
    let club = w.players.hot[p].club;
    if club.is_none() {
        return;
    }
    let (status, age) = (w.players.cold[p].status, w.age_years(p));
    if !matches!(status, SquadStatus::Star | SquadStatus::Important | SquadStatus::Regular) || crate::planning::renewal_chance(status, age) < 0.7 {
        return;
    }
    w.events.push(today, Visibility::Club(club), EventKind::PlanFailed { club, player: p, kind: pw_world::boardroom::PlanFailure::RenewalCollapsed });
    w.boardroom.scramble.insert(club, (w.players.cold[p].best_pos.group(), today));
}

/// The club signed over a promising youngster of its own at the same position: if he leaves and thrives, that is a plan that failed.
fn note_blocked(w: &mut World, club: ClubId, signing: PlayerId) {
    let today = w.date;
    let group = w.players.cold[signing].best_pos.group();
    let Some(plan) = w.deals.plans.get(&club) else { return };
    let mine = scouting::view(w, club, signing).0;
    let found = plan.promote.iter().copied().filter(|&y| y != signing && w.players.cold[y].best_pos.group() == group && w.age(y) <= 21).find(|&y| scouting::view(w, club, y).2 >= mine - 5.0);
    if let Some(y) = found {
        let level_then = scouting::view(w, club, y).0;
        if w.boardroom.blocked.len() < 400 {
            w.boardroom.blocked.push(pw_world::boardroom::Blocked { club, youngster: y, signing, date: today, level_then });
        }
    }
}

/// Monthly: look back at plans that may have failed and let them become history (section 4.23).
pub fn check_plans(w: &mut World) {
    use pw_world::boardroom::PlanFailure;
    let today = w.date;
    // Academy players the club counted on being ready, a year later.
    let due: Vec<pw_world::deals::CountedOn> = w.deals.counted_on.iter().copied().filter(|c| c.date.days_until(today) >= 330).collect();
    w.deals.counted_on.retain(|c| c.date.days_until(today) < 330);
    for c in due {
        let there = w.players.hot[c.player].club == c.club && w.players.hot[c.player].status == PlayerStatus::Active;
        if there && scouting::view(w, c.club, c.player).0 < c.expected - 8.0 {
            w.events.push(today, Visibility::Club(c.club), EventKind::PlanFailed { club: c.club, player: c.player, kind: PlanFailure::ProspectOverestimated });
        }
    }
    // Youngsters blocked by a signing: gone, and thriving elsewhere.
    let mut keep = Vec::new();
    for b in std::mem::take(&mut w.boardroom.blocked) {
        let age = b.date.days_until(today);
        let left = w.players.hot[b.youngster].club != b.club;
        if age >= 300 && left && w.players.hot[b.youngster].status != PlayerStatus::Retired {
            let now = market::public_view(w, b.youngster).0;
            if now >= b.level_then + 15.0 && w.players.cold[b.youngster].rep.world >= 3000 {
                w.events.push(today, Visibility::Public, EventKind::PlanFailed { club: b.club, player: b.youngster, kind: PlanFailure::YouthBlocked });
                continue;
            }
        }
        if age < 3 * 365 {
            keep.push(b);
        }
    }
    w.boardroom.blocked = keep;
    w.boardroom.scramble.retain(|_, &mut (_, when)| when.days_until(today) <= 90);
}

/// The deal never happened (the seller refused, the player would not sign): the case is closed without a verdict.
pub fn on_collapsed(w: &mut World, buyer: ClubId, p: PlayerId) {
    if let Some(i) = w.boardroom.case_for(buyer, p)
        && w.boardroom.cases[i].state == CaseState::Pursuing
    {
        w.boardroom.cases[i].state = CaseState::Collapsed;
    }
}

/// What happened with a signing, from what anyone could see: minutes against what was promised, what the market now thinks of him,
/// and his health and attitude.
fn assess(w: &World, c: &Case) -> (f32, [Option<RiskKind>; 2]) {
    let p = c.player;
    let cold = &w.players.cold[p];
    let hot = &w.players.hot[p];
    let signed = c.signed.unwrap_or(c.date);
    let months = (signed.days_until(w.date) as f32 / 30.0).max(1.0);
    let with_club = hot.club == c.buyer;
    let apps = cold.senior_apps.saturating_sub(c.apps_at) as f32;
    let expected = (months / 12.0 * 38.0 * c.role.expected_minutes()).max(1.0);
    let minutes = if with_club { ((apps - expected) / expected).clamp(-1.0, 1.0) } else { -0.6 };
    let now = market::value_of(w, p) as f32;
    let value = if c.price_paid > 0 { (now / c.price_paid as f32).max(0.05).ln().clamp(-1.0, 1.0) } else { 0.0 };
    let sick = if hot.injury_days > 90 { -0.4 } else { 0.0 };
    let success = (0.6 * minutes + 0.3 * value + 0.1 * sick).clamp(-1.0, 1.0);
    let mut hit: [Option<RiskKind>; 2] = [None, None];
    let mut n = 0;
    let mut note = |k: RiskKind, hit: &mut [Option<RiskKind>; 2]| {
        if n < 2 {
            hit[n] = Some(k);
            n += 1;
        }
    };
    if cold.injuries_career >= c.injuries_at + 2 || hot.injury_days > 60 {
        note(RiskKind::Injuries, &mut hit);
    }
    if w.market.has_requested(p) || hot.morale < 35 {
        note(RiskKind::Attitude, &mut hit);
    }
    if let Some(avg) = hot.form_avg()
        && avg < 6.1
        && with_club
    {
        note(RiskKind::Decisions, &mut hit);
    }
    if with_club && minutes < -0.5 {
        note(RiskKind::Physical, &mut hit);
    }
    if crate::adaptation::struggled(w, p) {
        note(RiskKind::Adaptation, &mut hit);
    }
    (success, hit)
}

/// How a signing is judged: the result together with the process. See [`Verdict`].
pub fn verdict(c: &Case, success: f32, hit: &[Option<RiskKind>; 2]) -> Verdict {
    let sound = c.confidence >= Confidence::Medium && c.risks_unknown.is_empty() && c.price_paid <= c.price_high + c.price_high / 7 && !c.panic;
    let hit: Vec<RiskKind> = hit.iter().flatten().copied().collect();
    if success >= 0.15 {
        return if sound { Verdict::Sound } else { Verdict::Lucky };
    }
    if success > -0.15 {
        return if sound { Verdict::Sound } else { Verdict::Lucky };
    }
    if hit.iter().any(|k| c.risks_accepted.contains(k)) {
        Verdict::AcceptedRisk
    } else if hit.iter().any(|k| !c.risks_known.contains(k)) && !hit.is_empty() {
        Verdict::MissedRisk
    } else if sound {
        Verdict::Unlucky
    } else {
        Verdict::Mistake
    }
}

fn nudge(w: &mut World, club: ClubId, v: Voice, by: f32) {
    let e = w.boardroom.authority.entry((club, v)).or_insert(0.0);
    *e = (*e + by).clamp(-0.3, 0.3);
}

/// Judge signings that have had a season to show what they were: outcome, verdict, and what the club learns about whom to listen to.
pub fn review(w: &mut World) {
    let today = w.date;
    let due: Vec<usize> = w.boardroom.cases.iter().enumerate().filter(|(_, c)| c.state == CaseState::Signed && c.outcome.is_none() && c.signed.is_some_and(|d| d.days_until(today) >= 300)).map(|(i, _)| i).collect();
    for i in due {
        let c = w.boardroom.cases[i].clone();
        let (success, hit) = assess(w, &c);
        let v = verdict(&c, success, &hit);
        w.boardroom.cases[i].outcome = Some(Outcome { date: today, success, verdict: v, materialised: hit });
        let club = c.buyer;
        let overruled = !c.overruled.is_empty();
        if success >= 0.15 {
            nudge(w, club, c.champion, 0.04);
            for &v in &c.overruled {
                nudge(w, club, v, -0.03);
            }
            w.boardroom.record.entry(club).or_default().hits = w.boardroom.record.get(&club).map_or(0, |r| r.hits).saturating_add(1);
        } else if success <= -0.15 {
            // What the club learns depends on the kind of failure: a risk it accepted is a lesson about appetite, one it missed is a
            // lesson about who to trust, and a decision that was sound is mostly luck and changes little (section 3.23).
            let (champ, opp) = match v {
                Verdict::Unlucky => (-0.01, 0.01),
                Verdict::AcceptedRisk => (-0.03, 0.02),
                Verdict::MissedRisk => (-0.05, 0.0),
                _ => (-0.05, 0.05),
            };
            nudge(w, club, c.champion, champ);
            for &v in &c.overruled {
                nudge(w, club, v, opp);
            }
            let r = w.boardroom.record.entry(club).or_default();
            r.flops = r.flops.saturating_add(u8::from(v != Verdict::Unlucky));
        }
        // The people who backed or opposed it are judged by their own managers over time.
        if let Some(staff) = match c.champion {
            Voice::Manager => w.clubs[club].manager.get(),
            Voice::Director => director(w, club),
            Voice::Recruitment => scouts(w, club).into_iter().next(),
            Voice::Analyst => analyst(w, club),
            _ => None,
        } {
            let t = w.dossiers.track.entry(staff).or_default();
            if success >= 0.15 {
                t.right = t.right.saturating_add(1);
            } else if success <= -0.15 && v != Verdict::Unlucky {
                t.wrong = t.wrong.saturating_add(1);
            }
        }
        if overruled || success.abs() >= 0.3 {
            let vis = if c.price_paid as f64 >= crate::finance::season_revenue(w, club) as f64 * 0.05 { Visibility::Public } else { Visibility::Club(club) };
            w.events.push(today, vis, EventKind::SigningReviewed { player: c.player, club, verdict: v, overruled });
        }
    }
}

/// Monthly: refresh every club's appetite for risk, judge signings a season on, and forget old cases.
pub fn monthly(w: &mut World) {
    let today = w.date;
    // Trophies won in the last two years, per club.
    let mut trophies: rustc_hash::FxHashMap<ClubId, u32> = rustc_hash::FxHashMap::default();
    for e in w.events.since(today.add_days(-730)) {
        if let EventKind::Champion { team, .. } = e.kind {
            *trophies.entry(w.teams[team].club).or_default() += 1;
        }
    }
    for club in w.clubs.ids().collect::<Vec<_>>() {
        if w.clubs[club].teams.iter().all(|&t| w.teams[t].kind != TeamKind::First) {
            continue;
        }
        let a = appetite(w, club, trophies.get(&club).copied().unwrap_or(0));
        w.boardroom.appetite.insert(club, a);
    }
    review(w);
    check_plans(w);
    w.boardroom.cases.retain(|c| c.outcome.is_none() && c.date.days_until(today) < KEEP_DAYS || c.outcome.is_some_and(|o| o.date.days_until(today) < KEEP_DAYS));
    // Standing earned or lost fades; so does the memory of hits and flops.
    if today.month() == 1 {
        for v in w.boardroom.authority.values_mut() {
            *v *= 0.8;
        }
        for r in w.boardroom.record.values_mut() {
            r.hits = r.hits.saturating_sub(1);
            r.flops = r.flops.saturating_sub(1);
        }
    }
}
