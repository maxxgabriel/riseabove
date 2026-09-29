//! What the clauses of a contract do once it is signed (locked design 5.3, 5.14-5.19).
//!
//! Bonuses are paid when their conditions are met, out of the club's money and into the player's; a relegation cut and a relegation
//! release take effect when the club goes down; options and automatic extensions are decided or triggered by whoever holds them; a
//! release clause lets another club take the player without the seller's consent. Important contracts leave a file so they can be
//! judged later on what was known at the time as well as how they turned out, and a status promise is a promise that is remembered.

use pw_core::{ClubId, EventId, Money, PlayerId, PosGroup};
use pw_world::boardroom::{ContractFile, Outcome, Verdict, Voice};
use pw_world::contract::{SquadStatus, Trigger};
use pw_world::event::{EventKind, Visibility};
use pw_world::negotiation::{Negotiation, TalkKind};
use pw_world::social::PromiseKind;
use pw_world::{MindKind, PlayerStatus, World};
use smallvec::SmallVec;

use crate::{boardroom, consider, market, package};

// ------------------------------------------------------------------ money

/// The club pays a bonus; the player keeps what tax and the agent leave, in his savings.
fn pay(w: &mut World, club: ClubId, p: PlayerId, amount: Money) {
    if amount <= 0 || club.is_none() {
        return;
    }
    let f = &mut w.clubs[club].finance;
    f.balance -= amount;
    f.season_spend += amount;
    let tax = i64::from(w.nations[w.clubs[club].nation].env.tax);
    let who = w.players.cold[p].person;
    w.lives[who].finances.savings += amount * (100 - tax) / 100 * 6 / 10;
}

/// After a senior match: appearance, goal, assist and clean-sheet bonuses.
pub fn match_bonuses(w: &mut World, p: PlayerId, minutes: u8, goals: u8, assists: u8, kept_clean: bool) {
    if minutes == 0 {
        return;
    }
    let c = &w.players.cold[p].contract;
    let club = c.club;
    if club.is_none() {
        return;
    }
    let keeper_or_defender = matches!(w.players.cold[p].best_pos.group(), PosGroup::Gk | PosGroup::Def);
    let mut total = c.appearance_bonus + c.goal_bonus * Money::from(goals) + c.assist_bonus * Money::from(assists);
    if kept_clean && minutes >= 60 && keeper_or_defender {
        total += c.clean_sheet_bonus;
    }
    pay(w, club, p, total);
    check_auto(w, p, None);
}

/// A senior international cap.
pub fn on_cap(w: &mut World, p: PlayerId) {
    let c = &w.players.cold[p].contract;
    let (club, bonus) = (c.club, c.cap_bonus);
    pay(w, club, p, bonus);
    check_auto(w, p, None);
}

/// A continental competition's entrants are known: those clubs' players earn the qualification bonus.
pub fn on_continental_entry(w: &mut World, comp: pw_core::CompId) {
    let teams = w.comps[comp].state.entrants.clone();
    for t in teams {
        for p in w.teams[t].squad.clone() {
            let c = &w.players.cold[p].contract;
            let (club, bonus) = (c.club, c.continental_bonus);
            pay(w, club, p, bonus);
            check_auto(w, p, Some(Trigger::Continental));
        }
    }
}

// ------------------------------------------------------------------ season results

/// Weekly: read what happened since last time (titles, promotions, relegations), pay what is owed, apply what falls due, and settle options.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let from = if w.boardroom.event_cursor == 0 { EventId::NONE } else { EventId(w.boardroom.event_cursor) };
    let news: Vec<(EventKind, u32)> = w.events.after(from).iter().map(|e| (e.kind.clone(), e.id.0)).collect();
    if let Some(&(_, last)) = news.last() {
        w.boardroom.event_cursor = last;
    }
    for (kind, _) in news {
        match kind {
            EventKind::Champion { team, .. } => {
                for p in w.teams[team].squad.clone() {
                    let c = &w.players.cold[p].contract;
                    let (club, bonus) = (c.club, c.title_bonus);
                    pay(w, club, p, bonus);
                    check_auto(w, p, Some(Trigger::Title));
                }
            }
            EventKind::Promoted { team, .. } => {
                for p in w.teams[team].squad.clone() {
                    let c = &w.players.cold[p].contract;
                    let (club, bonus) = (c.club, c.promotion_bonus);
                    pay(w, club, p, bonus);
                    check_auto(w, p, Some(Trigger::Promotion));
                }
            }
            EventKind::Relegated { team, .. } => relegated(w, team),
            _ => {}
        }
    }
    // Loyalty bonuses on each anniversary of signing.
    for p in w.players.ids().collect::<Vec<_>>() {
        let c = &w.players.cold[p].contract;
        if c.loyalty_bonus > 0 && w.players.hot[p].status == PlayerStatus::Active && c.club.is_some() {
            let days = c.start.days_until(today);
            // The anniversary fell in the last seven days.
            if days >= 365 && days / 365 > (days - 7) / 365 {
                let (club, bonus) = (c.club, c.loyalty_bonus);
                pay(w, club, p, bonus);
            }
        }
    }
    resolve_options(w);
    check_appearance_triggers(w);
}

/// The club goes down: wages fall by the cut in each contract, and a relegation release becomes a release clause.
fn relegated(w: &mut World, team: pw_core::TeamId) {
    for p in w.teams[team].squad.clone() {
        let c = &mut w.players.cold[p].contract;
        if c.relegation_cut > 0 {
            c.wage = (c.wage as f64 * (1.0 - f64::from(c.relegation_cut) / 100.0)) as Money;
            c.relegation_cut = 0;
        }
        if c.relegation_release > 0 {
            c.release_clause = c.relegation_release;
            c.relegation_release = 0;
        }
    }
}

// ------------------------------------------------------------------ options and extensions

fn extend(w: &mut World, p: PlayerId, years: u8, how: pw_world::event::OptionKind) {
    let today = w.date;
    let c = &mut w.players.cold[p].contract;
    c.end = c.end.add_months(12 * i32::from(years));
    c.options.used = true;
    let club = c.club;
    w.events.push(today, Visibility::Public, EventKind::ContractOption { player: p, club, kind: how, taken: true });
}

/// An automatic extension whose trigger has been met (`None` checks the counting ones).
fn check_auto(w: &mut World, p: PlayerId, event: Option<Trigger>) {
    let cold = &w.players.cold[p];
    let c = &cold.contract;
    let Some((trigger, extra)) = c.options.auto else { return };
    if c.options.used || c.club.is_none() {
        return;
    }
    let met = match (trigger, event) {
        (Trigger::Appearances(n), _) => cold.senior_apps.saturating_sub(c.apps_base) >= n,
        (Trigger::Caps(n), _) => cold.caps.saturating_sub(c.caps_base) >= n,
        (t, Some(e)) => t == e,
        _ => false,
    };
    if met {
        extend(w, p, extra, pw_world::event::OptionKind::Automatic);
    }
}

fn check_appearance_triggers(w: &mut World) {
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| matches!(w.players.cold[p].contract.options.auto, Some((Trigger::Appearances(_) | Trigger::Caps(_), _)))).collect();
    for p in ids {
        check_auto(w, p, None);
    }
}

/// Does the club want to keep him on as things are: he is useful and affordable.
fn club_wants(w: &World, club: ClubId, p: PlayerId) -> bool {
    let (ca, _, pa, _) = crate::scouting::view(w, club, p);
    let ideal = market::ideal_ca(w.clubs[club].reputation);
    let useful = ca.max(pa * 0.9) >= ideal - 10.0 && w.players.cold[p].status != SquadStatus::NotNeeded;
    let f = &w.clubs[club].finance;
    useful && (f.wage_budget == 0 || f.wage_bill <= f.wage_budget + f.wage_budget / 10)
}

/// Does the player want to stay for another year on the same terms: he wants security, or nobody else is interested.
fn player_wants(w: &World, p: PlayerId) -> bool {
    let who = w.players.cold[p].person;
    let pr = package::priorities(w, p);
    let security = pr[pw_world::negotiation::Priority::Security.idx()] * 13.0;
    let ambition = w.people[who].hidden.f(pw_core::Hidden::Ambition);
    let interest = consider::heard_interest(w, who);
    security > 1.1 || interest == 0 || (w.age(p) >= 31 && ambition < 15.0)
}

/// Options fall due in the last months of a contract: the holder decides (section 5.14).
fn resolve_options(w: &mut World) {
    let today = w.date;
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| {
        let c = &w.players.cold[p].contract;
        c.options.any() && !c.options.used && c.club.is_some() && (30..=100).contains(&c.days_left(today)) && w.players.hot[p].status == PlayerStatus::Active
    }).collect();
    for p in ids {
        let (opts, club) = {
            let c = &w.players.cold[p].contract;
            (c.options, c.club)
        };
        if w.people[w.players.cold[p].person].mind != MindKind::Ai {
            continue;
        }
        let club_yes = club_wants(w, club, p);
        let player_yes = player_wants(w, p);
        if opts.club_years > 0 && club_yes {
            extend(w, p, opts.club_years, pw_world::event::OptionKind::Club);
        } else if opts.player_years > 0 && player_yes {
            extend(w, p, opts.player_years, pw_world::event::OptionKind::Player);
        } else if opts.mutual_years > 0 && club_yes && player_yes {
            extend(w, p, opts.mutual_years, pw_world::event::OptionKind::Mutual);
        } else if opts.club_years > 0 || opts.player_years > 0 || opts.mutual_years > 0 {
            // The holder passes; say so once, and leave the deal to run out.
            let kind = if opts.club_years > 0 { pw_world::event::OptionKind::Club } else if opts.player_years > 0 { pw_world::event::OptionKind::Player } else { pw_world::event::OptionKind::Mutual };
            let c = &mut w.players.cold[p].contract;
            c.options.used = true;
            w.events.push(today, Visibility::Public, EventKind::ContractOption { player: p, club, kind, taken: false });
        }
    }
}

// ------------------------------------------------------------------ release clauses

/// Monthly, in a window: a club with the need and the money can pay a player's release clause and go straight to him (section 5.15).
pub fn release_clause_bids(w: &mut World) {
    let today = w.date;
    let holders: Vec<PlayerId> = w
        .players
        .ids()
        .filter(|&p| {
            let c = &w.players.cold[p].contract;
            c.release_clause > 0 && c.club.is_some() && w.players.hot[p].status == PlayerStatus::Active && w.players.cold[p].loan.is_none() && !crate::negotiation::in_talks(w, p)
        })
        .collect();
    for p in holders {
        let clause = w.players.cold[p].contract.release_clause;
        let seller = w.players.hot[p].club;
        let group = w.players.cold[p].best_pos.group();
        let mut best: Option<(ClubId, u16)> = None;
        for buyer in w.clubs.ids() {
            if buyer == seller || !w.nations[w.clubs[buyer].nation].season.window_open(today) || w.market.on_cooldown(buyer, p, today) {
                continue;
            }
            let fin = &w.clubs[buyer].finance;
            if fin.transfer_budget < clause || fin.balance < clause {
                continue;
            }
            if !w.clubs[buyer].market.needs.iter().any(|n| n.group == group) {
                continue;
            }
            let (ca, ..) = crate::scouting::view(w, buyer, p);
            if ca < market::ideal_ca(w.clubs[buyer].reputation) - 4.0 {
                continue;
            }
            // Being able to pay the clause is not a reason to: nobody pays far above what they think he is worth.
            if clause as f64 > market::fair_value(w, buyer, p) as f64 * 1.25 {
                continue;
            }
            let rep = w.clubs[buyer].reputation;
            if best.is_none_or(|b| rep > b.1) {
                best = Some((buyer, rep));
            }
        }
        let Some((buyer, _)) = best else { continue };
        w.market.cooldown.insert((buyer, p), today.add_days(120));
        // The club's own people still have to want him at that price.
        if !boardroom::consider_target(w, buyer, p, Some(group), clause, w.clubs[buyer].finance.transfer_budget) {
            continue;
        }
        let causes = pw_world::causes![pw_world::Cause::Fact(pw_world::Fact::SquadNeed { club: buyer })];
        crate::negotiation::open(w, p, buyer, TalkKind::Transfer, seller, clause, None, causes);
    }
}

// ------------------------------------------------------------------ what a contract remembers

fn leader_voice(w: &World, club: ClubId) -> Voice {
    match boardroom::structure(w, club) {
        pw_world::boardroom::Structure::ManagerLed => Voice::Manager,
        pw_world::boardroom::Structure::OwnerLed => Voice::Owner,
        pw_world::boardroom::Structure::DataLed => Voice::Analyst,
        _ => Voice::Director,
    }
}

/// The file of an important contract, to be kept once it is signed: what the club believed, what it conceded, what the player wanted,
/// what was promised (section 5.19). `None` for ordinary deals.
pub fn file_contract(w: &World, t: &Negotiation) -> Option<ContractFile> {
    if matches!(t.kind, TalkKind::Loan | TalkKind::PreContract) {
        return None;
    }
    let club = t.club;
    let p = t.player;
    let revenue = crate::finance::season_revenue(w, club).max(1);
    let commitment = package::club_cost(w, club, p, &t.offer) as Money;
    let burden = (commitment as f64 / (revenue as f64 * f64::from(t.offer.years.max(1))) * 100.0).clamp(0.0, 100.0) as u8;
    let excess = package::hierarchy_excess(w, club, t.offer.wage);
    let important = burden >= 3 || excess > 0.0 || t.offer.status.is_some_and(|s| s <= SquadStatus::Important);
    if !important {
        return None;
    }
    let group = w.players.cold[p].best_pos.group();
    let risks: SmallVec<[pw_world::dossier::RiskKind; 4]> = w.dossiers.get(club, p).map(|d| d.risks.iter().filter(|r| r.known).map(|r| r.kind).take(4).collect()).unwrap_or_default();
    Some(ContractFile {
        club,
        player: p,
        date: w.date,
        terms: t.offer,
        believed_worth: market::fair_value(w, club, p),
        role: t.offer.status.unwrap_or(SquadStatus::Squad),
        risks,
        competing: consider::heard_interest(w, w.players.cold[p].person).min(9) as u8,
        alternatives: w.deals.shortlists.get(&(club, group)).map_or(0, |s| s.targets.len().min(9) as u8),
        exception: (excess > 0.0).then(|| leader_voice(w, club)),
        concessions: t.moves.clone(),
        priorities: t.priorities,
        promised: t.offer.status,
        commitment,
        burden_pct: burden,
        apps_at: w.players.cold[p].senior_apps,
        outcome: None,
    })
}

pub fn keep_file(w: &mut World, file: Option<ContractFile>) {
    if let Some(f) = file
        && w.boardroom.contracts.len() < 1500
    {
        w.boardroom.contracts.push(f);
    }
}

/// A promised status is a promise like any other: remembered, checked, and costly to break (section 5.16).
pub fn promise_status(w: &mut World, club: ClubId, p: PlayerId, s: SquadStatus) {
    let today = w.date;
    let Some(mgr) = w.clubs[club].manager.get().map(|m| w.staff[m].person) else { return };
    let who = w.players.cold[p].person;
    if mgr == who {
        return;
    }
    let id = w.social.make_promise(mgr, who, club, PromiseKind::Status(s), today, today.add_days(365), EventId::NONE);
    w.events.push(today, Visibility::Between(mgr, who), EventKind::PromiseMade { promise: id, from: mgr, to: who });
}

/// Judge important contracts a season on, by what was known at the time and by how they turned out; a burden that did not pay off
/// costs the club with its board and supporters, and those who approved it (sections 5.17, 5.18).
pub fn review_contracts(w: &mut World) {
    let today = w.date;
    let due: Vec<usize> = w.boardroom.contracts.iter().enumerate().filter(|(_, f)| f.outcome.is_none() && f.date.days_until(today) >= 300).map(|(i, _)| i).collect();
    for i in due {
        let f = w.boardroom.contracts[i].clone();
        let p = f.player;
        let with_club = w.players.hot[p].club == f.club;
        let months = (f.date.days_until(today) as f32 / 30.0).max(1.0);
        let apps = w.players.cold[p].senior_apps.saturating_sub(f.apps_at) as f32;
        let expected = (months / 12.0 * 38.0 * f.role.expected_minutes()).max(1.0);
        let minutes = if with_club { ((apps - expected) / expected).clamp(-1.0, 1.0) } else { -0.6 };
        let now = market::value_of(w, p) as f32;
        let value = if f.believed_worth > 0 { (now / f.believed_worth as f32).max(0.05).ln().clamp(-1.0, 1.0) } else { 0.0 };
        let success = (0.65 * minutes + 0.35 * value).clamp(-1.0, 1.0);
        // What was known at the time: were the risks seen, was the price within the range, was the burden sensible.
        let sound = f.burden_pct <= 12 && f.exception.is_none();
        let verdict = if success >= 0.15 {
            if sound { Verdict::Sound } else { Verdict::Lucky }
        } else if success > -0.15 {
            if sound { Verdict::Sound } else { Verdict::Mistake }
        } else if sound {
            Verdict::Unlucky
        } else {
            Verdict::Mistake
        };
        w.boardroom.contracts[i].outcome = Some(Outcome { date: today, success, verdict, materialised: [None, None] });
        if success <= -0.15 && f.burden_pct >= 8 && !sound {
            // The club is stuck with it: the board notices the money, the supporters notice the player.
            if let Some(g) = w.governance.get_mut(&f.club) {
                bump_concern(g, pw_world::governance::BoardConcern::Finances, -10);
                bump_concern(g, pw_world::governance::BoardConcern::FanUnrest, -8);
            }
            if let Some(v) = f.exception {
                let e = w.boardroom.authority.entry((f.club, v)).or_insert(0.0);
                *e = (*e - 0.05).clamp(-0.3, 0.3);
            }
            let vis = if f.burden_pct >= 15 { Visibility::Public } else { Visibility::Club(f.club) };
            w.events.push(today, vis, EventKind::SigningReviewed { player: p, club: f.club, verdict, overruled: false });
        }
    }
    w.boardroom.contracts.retain(|f| f.outcome.is_none() || f.outcome.is_some_and(|o| o.date.days_until(today) < 4 * 365));
}

fn bump_concern(g: &mut pw_world::governance::Governance, kind: pw_world::governance::BoardConcern, by: i8) {
    match g.concerns.iter_mut().find(|(k, _)| *k == kind) {
        Some((_, v)) => *v = (i16::from(*v) + i16::from(by)).clamp(-30, 30) as i8,
        None => g.concerns.push((kind, by.clamp(-30, 30))),
    }
}

