//! Club-to-club deals and everything around them (08 §1–6, 16 §C).
//!
//! Recruitment starts from the squad plan: shortlists per need, ranked from
//! the club's reports. A deal opens with an enquiry; the seller may say the
//! player is not for sale. Bids and counters go back and forth over a few
//! days — fees in instalments, appearance and goal add-ons, sell-on
//! percentages, buy-back clauses — shaped by each club's board, finances,
//! rivalries and how close the window is to closing. Another club can gazump
//! a deal. An agreed fee goes to a medical, then to personal terms. Once
//! done, instalments, add-ons and sell-ons become money owed across seasons.
//! Loans carry fees, wage splits, options, obligations, minutes clauses and
//! recalls. Players running down contracts sign pre-contracts; unattached
//! players go on trial.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Money, PlayerId, PosGroup};
use pw_world::contract::Loan;
use pw_world::deals::{AddOn, AddOnKind, ClubDeal, DealEnd, DealLine, DealState, DealTerms, LoanTerms, PayReason, Payable, PendingAddOn, PreContract, Shortlist, Trial};
use pw_world::decision::{Decision, DecisionKind};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::governance::TransferStyle;
use pw_world::negotiation::{TalkKind, Terms};
use pw_world::{MindKind, PlayerStatus, PromiseKind, SquadStatus, World};
use smallvec::SmallVec;

use crate::{consider, market, negotiation, planning, scouting};

// ------------------------------------------------------------------ shortlists

/// Monthly: each club ranks targets for each need from what it knows.
pub fn shortlists(w: &mut World) {
    let today = w.date;
    let clubs: Vec<ClubId> = w.deals.plans.keys().copied().collect();
    let mut clubs = clubs;
    clubs.sort();
    for club in clubs {
        let plan = w.deals.plans[&club].clone();
        let nation = w.clubs[club].nation;
        let prof = pw_world::rules::profile(w, nation);
        for need in plan.needs.iter() {
            let mut cands: Vec<(PlayerId, f32)> = Vec::new();
            for (p, seen) in w.knowledge.known(club) {
                if seen.minutes < 90 {
                    continue;
                }
                let h = &w.players.hot[p];
                let c = &w.players.cold[p];
                if h.club == club || h.status != PlayerStatus::Active || c.loan.is_some() {
                    continue;
                }
                if c.best_pos.group() != need.group || w.age(p) > u32::from(need.max_age) {
                    continue;
                }
                if need.homegrown && pw_world::rules::homegrown_years(w, p, ClubId::NONE, nation, &prof) < f32::from(prof.homegrown_years) {
                    continue;
                }
                let (ca, band, pa, _) = scouting::view(w, club, p);
                if ca < f32::from(need.min_ability) {
                    continue;
                }
                let price = market::asking_price(w, p);
                if price > need.fee_band * 13 / 10 + 100_000 {
                    continue;
                }
                if market::wage_demand(w, p, club) > need.wage_band * 13 / 10 {
                    continue;
                }
                let grade = w.scouting.of(club, p).last().map_or(3.0, |r| f32::from(r.grade));
                let youth = if w.age(p) <= 23 { (pa - ca).max(0.0) * 0.3 } else { 0.0 };
                let available = if w.market.requests.contains_key(&p) || w.market.listed.contains_key(&p) { 4.0 } else { 0.0 };
                let score = ca + youth + (grade - 3.0) * 4.0 + crate::managers::wants(w, club, p) * 6.0 + available - band * 0.15 - (price as f32 / 1e6).sqrt();
                cands.push((p, score));
            }
            cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            let targets: SmallVec<[(PlayerId, f32); 5]> = cands.into_iter().take(5).collect();
            let failures = w.deals.shortlists.get(&(club, need.group)).map_or(0, |s| s.failures);
            w.deals.shortlists.insert((club, need.group), Shortlist { updated: today, targets, failures });
        }
    }
}

/// A club with an open window works down its shortlist. Returns true if it acted.
pub fn pursue(w: &mut World, club: ClubId) -> bool {
    let today = w.date;
    let Some(plan) = w.deals.plans.get(&club).cloned() else { return false };
    for need in plan.needs.iter() {
        let Some(list) = w.deals.shortlists.get(&(club, need.group)).cloned() else { continue };
        for (p, _) in list.targets {
            if w.deals.active(club, p) || w.market.on_cooldown(club, p, today) || w.market.talking.contains_key(&p) {
                continue;
            }
            if w.players.hot[p].club == club || w.players.hot[p].status != PlayerStatus::Active {
                continue;
            }
            enquire(w, club, p, Some(need.group));
            return true;
        }
    }
    false
}

// ------------------------------------------------------------------ deals

fn window_days_left(w: &World, club: ClubId) -> i32 {
    let s = &w.nations[w.clubs[club].nation].season;
    s.windows.iter().find(|&&(a, b)| w.date >= a && w.date <= b).map_or(0, |&(_, b)| w.date.days_until(b))
}

fn urgency(w: &World, club: ClubId, group: Option<PosGroup>) -> f32 {
    let need = group.and_then(|g| planning::need_detail(w, club, g)).map_or(1.0, |n| f32::from(n.urgency));
    let left = window_days_left(w, club) as f32;
    let closing = (1.0 - left / 21.0).clamp(0.0, 1.0);
    let failures = group.and_then(|g| w.deals.shortlists.get(&(club, g))).map_or(0.0, |s| f32::from(s.failures));
    0.4 + need * 0.15 + closing * 0.5 + failures * 0.08
}

fn log(w: &mut World, i: usize, l: DealLine) {
    let today = w.date;
    w.deals.deals[i].log.push((today, l));
}

fn collapse(w: &mut World, i: usize, why: DealEnd) {
    let today = w.date;
    let d = w.deals.deals[i].clone();
    {
        let x = &mut w.deals.deals[i];
        x.state = DealState::Collapsed;
        x.end = Some(why);
    }
    log(w, i, DealLine::Ended(why));
    w.market.cooldown.insert((d.buyer, d.player), today.add_days(if why == DealEnd::NotForSale { 150 } else { 60 }));
    if let Some(g) = d.need
        && let Some(s) = w.deals.shortlists.get_mut(&(d.buyer, g))
    {
        s.failures = s.failures.saturating_add(1);
    }
    let vis = if matches!(why, DealEnd::NotForSale | DealEnd::SellerRefused) { Visibility::Club(d.buyer) } else { Visibility::Public };
    w.events.push_caused(today, vis, EventKind::DealCollapsed { player: d.player, buyer: d.buyer, seller: d.seller, reason: why }, pw_world::causes![Cause::Event(d.event)]);
}

/// An enquiry: is the player available, and at roughly what price?
pub fn enquire(w: &mut World, buyer: ClubId, p: PlayerId, need: Option<PosGroup>) {
    let today = w.date;
    let seller = w.players.hot[p].club;
    if seller.is_none() || seller == buyer {
        return;
    }
    let causes: Causes = pw_world::causes![Cause::Fact(Fact::Tracking { club: buyer, player: p, minutes: consider::club_tracking(w, buyer, p) })];
    let ev = w.events.push_caused(today, Visibility::Club(seller), EventKind::Interest { player: p, club: buyer }, causes);
    let urg = urgency(w, buyer, need);
    let value = w.players.cold[p].value;
    let budget = w.clubs[buyer].finance.transfer_budget;
    let mut rng = Rng::keyed(&[w.seed, stream::MARKET, u64::from(buyer.0), u64::from(p.0), today.0 as u64]);
    let opening = ((value as f32 * rng.range_f32(0.75, 0.95)).min(budget as f32 * 1.1)) as Money;
    // Poorer clubs spread the cost.
    let instalments = if w.clubs[buyer].finance.balance < opening * 2 { 3 } else { 1 };
    let terms = DealTerms { fee: (opening / 50_000).max(1) * 50_000, instalments, add_ons: SmallVec::new(), sell_on: 0, buyback: 0, loan: None };
    w.deals.deals.push(ClubDeal {
        buyer,
        seller,
        player: p,
        need,
        terms,
        ask: None,
        round: 1,
        state: DealState::Enquiry,
        end: None,
        opened: today,
        next: today.add_days(1),
        urgency: urg,
        log: vec![(today, DealLine::Enquired)],
        talk: pw_core::TalkId::NONE,
        event: ev,
    });
}

/// Daily: every deal whose turn has come moves one step.
pub fn daily(w: &mut World) {
    let today = w.date;
    let due: Vec<usize> = w.deals.deals.iter().enumerate().filter(|(_, d)| d.is_open() && d.next <= today && d.state != DealState::Terms).map(|(i, _)| i).collect();
    for i in due {
        let d = &w.deals.deals[i];
        // Windows close on deals that aren't done.
        if !w.nations[w.clubs[d.buyer].nation].season.window_open(today) && d.state != DealState::Medical {
            collapse(w, i, DealEnd::WindowClosed);
            continue;
        }
        if w.players.hot[d.player].club != d.seller {
            collapse(w, i, DealEnd::Hijacked);
            continue;
        }
        match d.state {
            DealState::Enquiry => answer_enquiry(w, i),
            DealState::Bid => seller_turn(w, i),
            DealState::Counter => buyer_turn(w, i),
            DealState::Medical => medical(w, i),
            _ => {}
        }
    }
    // Old closed deals are archived away.
    if today.day() == 1 {
        let cutoff = today.add_days(-400);
        w.deals.deals.retain(|d| d.is_open() || d.opened >= cutoff);
    }
}

fn answer_enquiry(w: &mut World, i: usize) {
    let d = w.deals.deals[i].clone();
    let c = &w.players.cold[d.player];
    let key = matches!(c.status, SquadStatus::Star | SquadStatus::Important);
    let available = w.market.requests.contains_key(&d.player) || w.market.listed.contains_key(&d.player);
    let bigger = w.clubs[d.buyer].reputation > w.clubs[d.seller].reputation + 1500;
    let stance = crate::governance::selling_stance(w, d.seller);
    let rival = w.media.rivalry(d.seller, d.buyer) >= 50 && !w.governance.get(&d.seller).is_some_and(|g| g.policy.sell_to_rivals);
    let not_for_sale = !available && ((key && !bigger && stance > 1.1) || rival);
    if not_for_sale {
        collapse(w, i, DealEnd::NotForSale);
        return;
    }
    let bid = w.deals.deals[i].terms.fee;
    let x = &mut w.deals.deals[i];
    x.state = DealState::Bid;
    x.next = w.date.add_days(2);
    log(w, i, DealLine::Bid(bid));
}

/// What the selling club needs to see.
fn valuation(w: &World, d: &ClubDeal) -> f64 {
    let mut v = market::asking_price(w, d.player) as f64;
    // A rival pays a premium.
    if w.media.rivalry(d.seller, d.buyer) >= 50 {
        v *= 1.4;
    }
    // Close to the deadline a club that wants the money takes less.
    let left = window_days_left(w, d.seller);
    let wants_money = w.market.requests.contains_key(&d.player) || w.market.listed.contains_key(&d.player) || crate::governance::selling_stance(w, d.seller) < 0.9;
    if left <= 3 && wants_money {
        v *= 0.88;
    }
    v
}

fn seller_turn(w: &mut World, i: usize) {
    let d = w.deals.deals[i].clone();
    let today = w.date;
    // Gazumping: a better offer for the same player wins.
    let better = w
        .deals
        .deals
        .iter()
        .enumerate()
        .any(|(j, o)| j != i && o.player == d.player && o.is_open() && matches!(o.state, DealState::Bid | DealState::Counter) && o.terms.value() > d.terms.value() * 1.05);
    if better {
        collapse(w, i, DealEnd::Hijacked);
        return;
    }
    let want = valuation(w, &d);
    if d.terms.value() >= want {
        let x = &mut w.deals.deals[i];
        x.state = DealState::Medical;
        x.next = today.add_days(2);
        let fee = d.terms.fee;
        log(w, i, DealLine::Accepted(fee));
        w.events.push_caused(today, Visibility::Club(d.seller), EventKind::BidAccepted { player: d.player, club: d.buyer, fee }, pw_world::causes![Cause::Event(d.event)]);
        return;
    }
    if d.round >= 4 {
        w.events.push(today, Visibility::Club(d.seller), EventKind::BidRejected { player: d.player, club: d.buyer, fee: d.terms.fee });
        collapse(w, i, DealEnd::SellerRefused);
        return;
    }
    // Counter: a fee, and clauses that reflect the seller's policy.
    let policy = w.governance.get(&d.seller).map(|g| g.policy.transfer_style);
    let young = w.age(d.player) <= 21;
    let sell_on = match policy {
        Some(TransferStyle::Develop) | Some(TransferStyle::Homegrown) => 20,
        Some(TransferStyle::Value) => 15,
        _ => 0,
    };
    let buyback = if young && policy == Some(TransferStyle::Homegrown) { (want * 2.5) as Money } else { 0 };
    let ask = DealTerms { fee: ((want * 1.05) as Money / 50_000).max(1) * 50_000, instalments: d.terms.instalments.min(2), add_ons: SmallVec::new(), sell_on, buyback, loan: None };
    w.events.push(today, Visibility::Club(d.seller), EventKind::BidRejected { player: d.player, club: d.buyer, fee: d.terms.fee });
    let fee = ask.fee;
    let x = &mut w.deals.deals[i];
    x.ask = Some(ask);
    x.state = DealState::Counter;
    x.next = today.add_days(2);
    log(w, i, DealLine::Countered(fee));
}

fn buyer_turn(w: &mut World, i: usize) {
    let d = w.deals.deals[i].clone();
    let today = w.date;
    let Some(ask) = d.ask.clone() else { return };
    let urg = urgency(w, d.buyer, d.need);
    let value = w.players.cold[d.player].value as f64;
    let fee_band = d.need.and_then(|g| planning::need_detail(w, d.buyer, g)).map_or(w.clubs[d.buyer].finance.transfer_budget, |n| n.fee_band);
    let budget = w.clubs[d.buyer].finance.transfer_budget.max(fee_band) as f64;
    let willing = (value * (1.0 + 0.35 * f64::from(urg)).min(1.6)).min(budget * 1.1);
    if ask.value() <= willing {
        let x = &mut w.deals.deals[i];
        x.terms = ask.clone();
        x.state = DealState::Medical;
        x.next = today.add_days(2);
        log(w, i, DealLine::Accepted(ask.fee));
        return;
    }
    if d.round >= 4 || ask.value() > willing * 1.35 {
        collapse(w, i, DealEnd::BuyerWithdrew);
        return;
    }
    // Raise, and bridge the gap with structure the budget can bear.
    let mut t = d.terms.clone();
    let gap = ask.fee as f64 - t.fee as f64;
    t.fee = ((t.fee as f64 + gap * 0.5).min(willing) as Money / 50_000).max(1) * 50_000;
    if (ask.fee as f64) > budget {
        t.instalments = 3;
    }
    let remaining = ask.value() - t.value();
    if remaining > 0.0 && t.add_ons.is_empty() {
        t.add_ons.push(AddOn { kind: AddOnKind::Appearances(25), amount: (remaining * 0.7) as Money });
    }
    t.sell_on = ask.sell_on.min(15);
    let fee = t.fee;
    let x = &mut w.deals.deals[i];
    x.terms = t;
    x.round += 1;
    x.state = DealState::Bid;
    x.urgency = urg;
    x.next = today.add_days(2);
    log(w, i, DealLine::Raised(fee));
}

/// The medical: history and body wear can end a deal or cut the fee.
fn medical(w: &mut World, i: usize) {
    let d = w.deals.deals[i].clone();
    let today = w.date;
    let c = &w.players.cold[d.player];
    let h = &w.players.hot[d.player];
    let wear = f32::from(c.wear.iter().copied().max().unwrap_or(0)) / 100.0;
    let strict = 0.7 + f32::from(w.clubs[d.buyer].facilities.medical) / 40.0;
    let history = f32::from(c.injuries_career.min(10)) / 20.0;
    let mut rng = Rng::keyed(&[w.seed, stream::HEALTH, u64::from(d.player.0), u64::from(d.buyer.0), today.0 as u64]);
    let fail_p = (0.02 + wear * 0.4 + history * 0.3 + if h.injury_days > 21 { 0.35 } else { 0.0 }) * strict;
    if rng.chance(fail_p) {
        log(w, i, DealLine::MedicalFailed);
        if rng.chance(0.4) {
            // Go through at a lower fee with appearance add-ons protecting the buyer.
            let mut t = d.terms.clone();
            let cut = t.fee / 4;
            t.fee -= cut;
            t.add_ons.push(AddOn { kind: AddOnKind::Appearances(40), amount: cut });
            let fee = t.fee;
            w.deals.deals[i].terms = t;
            log(w, i, DealLine::FeeRenegotiated(fee));
        } else {
            collapse(w, i, DealEnd::FailedMedical);
            return;
        }
    } else {
        log(w, i, DealLine::MedicalPassed);
    }
    // Personal terms: the upfront part is paid on completion; the rest is owed.
    let d = w.deals.deals[i].clone();
    let upfront = d.terms.fee / Money::from(d.terms.instalments.max(1));
    let causes: Causes = pw_world::causes![Cause::Event(d.event)];
    match negotiation::open(w, d.player, d.buyer, TalkKind::Transfer, d.seller, upfront, None, causes) {
        Some(t) => {
            let x = &mut w.deals.deals[i];
            x.talk = t;
            x.state = DealState::Terms;
        }
        None => collapse(w, i, DealEnd::PlayerRefused),
    }
}

/// Talks finished and the player signed: money owed, clauses recorded.
pub fn on_completed(w: &mut World, p: PlayerId, buyer: ClubId) {
    let today = w.date;
    let Some(i) = w.deals.deals.iter().position(|d| d.player == p && d.buyer == buyer && d.state == DealState::Terms) else { return };
    let d = w.deals.deals[i].clone();
    w.deals.deals[i].state = DealState::Done;
    let n = Money::from(d.terms.instalments.max(1));
    let part = d.terms.fee / n;
    for k in 1..n {
        w.deals.payables.push(Payable { from: buyer, to: d.seller, amount: part, due: today.add_months(12 * k as i32), reason: PayReason::Instalment, player: p });
    }
    let c = &w.players.cold[p];
    for a in &d.terms.add_ons {
        w.deals.add_ons.push(PendingAddOn { player: p, payer: buyer, payee: d.seller, add_on: *a, since: today, apps_at: c.senior_apps, goals_at: c.senior_goals, caps_at: c.caps });
    }
    if d.terms.sell_on > 0 {
        w.deals.sell_ons.entry(p).or_default().push((d.seller, d.terms.sell_on));
    }
    if d.terms.buyback > 0 {
        w.deals.buybacks.insert(p, (d.seller, d.terms.buyback, today.add_months(24)));
    }
    // The agent's commission is a cost to the buyer.
    if let Some(r) = w.agents.of_player.get(&p) {
        let fee = d.terms.fee * Money::from(r.fee_pct) / 100;
        w.deals.payables.push(Payable { from: buyer, to: ClubId::NONE, amount: fee, due: today, reason: PayReason::AgentFee, player: p });
    }
    // A big signing is promised football.
    let status = w.players.cold[p].contract.promised_status;
    if let (Some(s), Some(mgr)) = (status, w.manager_of_player(p))
        && s <= SquadStatus::Regular
    {
        let who = w.players.cold[p].person;
        let id = w.social.make_promise(mgr, who, buyer, PromiseKind::Minutes { share: s.expected_minutes() }, today, today.add_days(120), d.event);
        w.events.push(today, Visibility::Between(mgr, who), EventKind::PromiseMade { promise: id, from: mgr, to: who });
    }
}

pub fn on_talks_failed(w: &mut World, p: PlayerId, buyer: ClubId) {
    if let Some(i) = w.deals.deals.iter().position(|d| d.player == p && d.buyer == buyer && d.state == DealState::Terms) {
        collapse(w, i, DealEnd::PlayerRefused);
    }
}

/// A fee changed hands for `p`: earlier clubs with sell-on clauses get their share.
pub fn on_transfer_fee(w: &mut World, p: PlayerId, seller: ClubId, fee: Money) {
    let today = w.date;
    if fee <= 0 {
        return;
    }
    if let Some(list) = w.deals.sell_ons.remove(&p) {
        for (club, pct) in list {
            if club == seller || club.is_none() {
                continue;
            }
            let amount = fee * Money::from(pct) / 100;
            w.deals.payables.push(Payable { from: seller, to: club, amount, due: today, reason: PayReason::SellOn, player: p });
        }
    }
    w.deals.buybacks.remove(&p);
}

/// Monthly: money owed falls due; add-ons trigger on the player's real career.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let due: Vec<Payable> = w.deals.payables.iter().filter(|x| x.due <= today).copied().collect();
    w.deals.payables.retain(|x| x.due > today);
    for x in due {
        w.clubs[x.from].finance.balance -= x.amount;
        w.clubs[x.from].finance.season_spend += x.amount;
        if x.to.is_some() {
            w.clubs[x.to].finance.balance += x.amount;
            w.clubs[x.to].finance.season_income += x.amount;
        }
        match x.reason {
            PayReason::SellOn => {
                w.events.push(today, Visibility::Public, EventKind::SellOnPaid { player: x.player, to: x.to, amount: x.amount });
            }
            PayReason::AddOn => {
                w.events.push(today, Visibility::Club(x.to), EventKind::AddOnPaid { player: x.player, from: x.from, to: x.to, amount: x.amount });
            }
            _ => {}
        }
    }
    let pending = std::mem::take(&mut w.deals.add_ons);
    for a in pending {
        let c = &w.players.cold[a.player];
        let hit = match a.add_on.kind {
            AddOnKind::Appearances(n) => c.senior_apps.saturating_sub(a.apps_at) >= n && w.players.hot[a.player].club == a.payer,
            AddOnKind::Goals(n) => c.senior_goals.saturating_sub(a.goals_at) >= n,
            AddOnKind::InternationalCaps(n) => c.caps.saturating_sub(a.caps_at) >= n,
            AddOnKind::Promotion => w.events.since(a.since).iter().any(|e| matches!(e.kind, EventKind::Promoted { team, .. } if w.teams[team].club == a.payer)),
            AddOnKind::Title => w.events.since(a.since).iter().any(|e| matches!(e.kind, EventKind::Champion { team, .. } if w.teams[team].club == a.payer)),
        };
        let expired = a.since.days_until(today) > 365 * 4;
        if hit {
            w.deals.payables.push(Payable { from: a.payer, to: a.payee, amount: a.add_on.amount, due: today, reason: PayReason::AddOn, player: a.player });
        } else if !expired {
            w.deals.add_ons.push(a);
        }
    }
}

// ------------------------------------------------------------------ loans

/// Terms of a development loan, from both clubs' circumstances.
pub fn loan_terms(w: &World, parent: ClubId, dest: ClubId, p: PlayerId) -> (Loan, LoanTerms) {
    let today = w.date;
    let value = w.players.cold[p].value;
    let dest_rich = f32::from(w.clubs[dest].reputation) / 10_000.0;
    let parent_policy = w.governance.get(&parent).map(|g| g.policy.transfer_style);
    let parent_in_debt = w.clubs[parent].finance.debt > 0;
    let age = w.age(p);
    let fee = if age >= 21 { (value as f32 * 0.05 * (0.5 + dest_rich)) as Money } else { 0 };
    let wage_share = (40.0 + dest_rich * 60.0).round().clamp(20.0, 100.0) as u8;
    // Clubs happy to move a player on add an option (or an obligation when they need money).
    let surplus = w.players.cold[p].status >= SquadStatus::Fringe;
    let option = if surplus && parent_policy != Some(TransferStyle::Homegrown) { value } else { 0 };
    let (obligation, obligation_apps) = if parent_in_debt && surplus { (value, 20) } else { (0, 0) };
    let end = w.nations[w.clubs[dest].nation].season.end;
    let loan = Loan { parent, club: dest, start: today, end, wage_share, fee, buy_option: option, recall: !surplus };
    let terms =
        LoanTerms { fee, wage_share, option, obligation, obligation_apps, recall: !surplus, minutes_clause: if age <= 21 && !surplus { 40 } else { 0 }, apps_at_start: w.players.cold[p].senior_apps };
    (loan, terms)
}

/// A loan ends: obligation, option, or home. Returns true if the player moved permanently.
pub fn loan_ends(w: &mut World, p: PlayerId) -> bool {
    let today = w.date;
    let Some(loan) = w.players.cold[p].loan.clone() else { return false };
    let Some(t) = w.deals.loans.remove(&p) else { return false };
    let apps = w.players.cold[p].senior_apps.saturating_sub(t.apps_at_start);
    let obliged = t.obligation > 0 && apps >= t.obligation_apps;
    // An option is taken up if the loan club rates what it saw.
    let exercised = !obliged && t.option > 0 && {
        let (ca, _, _, _) = scouting::view(w, loan.club, p);
        let rating = w.players.hot[p].form_avg().unwrap_or(6.5);
        ca >= market::ideal_ca(w.clubs[loan.club].reputation) - 10.0 && rating >= 6.8 && w.clubs[loan.club].finance.transfer_budget >= t.option
    };
    if !(obliged || exercised) {
        return false;
    }
    let fee = if obliged { t.obligation } else { t.option };
    let contract = market::new_contract(w, p, loan.club, 1.0);
    w.players.cold[p].loan = None;
    market::execute_transfer(w, p, loan.club, loan.parent, fee, contract);
    w.events.push(today, Visibility::Public, EventKind::OptionExercised { player: p, club: loan.club, fee });
    true
}

/// Weekly: parents recall loanees they need or who aren't playing.
pub fn recalls(w: &mut World) {
    let today = w.date;
    let loanees: Vec<PlayerId> = w.deals.loans.keys().copied().collect();
    let mut loanees = loanees;
    loanees.sort();
    for p in loanees {
        let Some(loan) = w.players.cold[p].loan.clone() else { continue };
        let t = w.deals.loans[&p];
        if !t.recall || !w.nations[w.clubs[loan.parent].nation].season.window_open(today) {
            continue;
        }
        let group = w.players.cold[p].best_pos.group();
        let first = w.clubs[loan.parent].first_team();
        let fit = w.teams[first].squad.iter().filter(|&&x| w.players.cold[x].best_pos.group() == group && w.players.hot[x].available()).count();
        let crisis = fit
            < match group {
                PosGroup::Gk => 1,
                PosGroup::Att => 2,
                _ => 4,
            };
        let (share, _) = consider::minutes_share(w, p);
        let benched = t.minutes_clause > 0 && share * 100.0 < f32::from(t.minutes_clause) && loan.start.days_until(today) > 60;
        if crisis || benched {
            w.deals.loans.remove(&p);
            market::end_loan(w, p);
            w.events.push(today, Visibility::Public, EventKind::LoanRecalled { player: p, club: loan.parent });
        }
    }
}

// ------------------------------------------------------------------ pre-contracts & trials

/// Weekly: clubs approach players abroad in the last six months of their deals.
pub fn pre_contracts(w: &mut World) {
    let today = w.date;
    let clubs: Vec<ClubId> = w.deals.plans.keys().copied().collect();
    let mut clubs = clubs;
    clubs.sort();
    for club in clubs {
        if !(club.0 + (today.0 / 7) as u32).is_multiple_of(3) {
            continue;
        }
        let groups: Vec<PosGroup> = w.deals.plans[&club].needs.iter().map(|n| n.group).collect();
        for g in groups {
            let Some(list) = w.deals.shortlists.get(&(club, g)).cloned() else { continue };
            for (p, _) in list.targets {
                let c = &w.players.cold[p];
                let left = c.contract.days_left(today);
                let current = w.players.hot[p].club;
                let abroad = current.is_some() && w.clubs[current].nation != w.clubs[club].nation;
                if !(1..=180).contains(&left) || !abroad || w.market.talking.contains_key(&p) || w.deals.pre_contracts.iter().any(|x| x.player == p) {
                    continue;
                }
                let causes: Causes = pw_world::causes![Cause::Fact(Fact::ContractRunningDown { player: p, days: left as u16 })];
                negotiation::open(w, p, club, TalkKind::PreContract, current, 0, None, causes);
                break;
            }
        }
    }
}

pub fn sign_pre_contract(w: &mut World, p: PlayerId, club: ClubId, terms: Terms) {
    let today = w.date;
    w.deals.pre_contracts.push(PreContract { player: p, club, terms, signed: today });
    w.events.push(today, Visibility::Public, EventKind::PreContractSigned { player: p, club });
    // The current club knows he's leaving.
    let current = w.players.hot[p].club;
    if let Some(m) = w.manager_of_player(p) {
        let who = w.players.cold[p].person;
        let compat = consider::compat(w, m, who);
        w.social.adjust(m, who, today, compat, -4, -8, 0);
    }
    let _ = current;
}

/// On contract expiry: does a pre-contract take the player somewhere? Returns true if so.
pub fn honour_pre_contract(w: &mut World, p: PlayerId) -> bool {
    let Some(i) = w.deals.pre_contracts.iter().position(|x| x.player == p) else { return false };
    let pc = w.deals.pre_contracts.remove(i);
    let today = w.date;
    let contract = pc.terms.to_contract(pc.club, pw_world::ContractKind::Professional, today);
    let old = w.players.hot[p].club;
    market::execute_transfer(w, p, pc.club, old, 0, contract);
    true
}

/// Weekly: unattached players are invited to train with clubs that need them.
pub fn trials(w: &mut World) {
    let today = w.date;
    // Trials that end today.
    let ending: Vec<Trial> = w.deals.trials.iter().filter(|t| t.until <= today).copied().collect();
    w.deals.trials.retain(|t| t.until > today);
    for t in ending {
        let need = w.clubs[t.club].market.needs.iter().find(|n| n.group == w.players.cold[t.player].best_pos.group()).copied();
        let (ca, _, _, _) = scouting::view(w, t.club, t.player);
        let offered = need.is_some_and(|n| ca >= f32::from(n.min_ability)) && w.players.hot[t.player].status != PlayerStatus::Active;
        w.events.push(today, Visibility::Person(w.players.cold[t.player].person), EventKind::TrialEnded { player: t.player, club: t.club, offered });
        if offered {
            let causes: Causes = pw_world::causes![Cause::Fact(Fact::SquadNeed { club: t.club })];
            negotiation::open(w, t.player, t.club, TalkKind::FreeAgent, ClubId::NONE, 0, None, causes);
        }
    }
    // Trialists are watched closely.
    let on: Vec<Trial> = w.deals.trials.clone();
    for t in on {
        w.knowledge.observe(t.club, t.player, 180, today);
    }
    // New invitations: clubs uncertain about an unattached player they've heard of.
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| !w.clubs[c].market.needs.is_empty() && (c.0 + (today.0 / 7) as u32).is_multiple_of(4)).collect();
    for club in clubs {
        let needs = w.clubs[club].market.needs.clone();
        let cands: Vec<PlayerId> = w
            .knowledge
            .known(club)
            .filter(|(p, s)| s.minutes >= 45 && matches!(w.players.hot[*p].status, PlayerStatus::FreeAgent | PlayerStatus::Amateur) && w.age(*p) >= 17)
            .map(|(p, _)| p)
            .filter(|&p| w.deals.on_trial(p).is_none() && !w.market.talking.contains_key(&p) && !w.market.on_cooldown(club, p, today))
            .filter(|&p| {
                let (ca, band, _, _) = scouting::view(w, club, p);
                needs.iter().any(|n| n.group == w.players.cold[p].best_pos.group() && ca + band >= f32::from(n.min_ability)) && band >= 10.0
            })
            .collect();
        let mut cands = cands;
        cands.sort();
        let Some(&p) = cands.first() else { continue };
        invite(w, club, p);
    }
}

fn invite(w: &mut World, club: ClubId, p: PlayerId) {
    let today = w.date;
    let who = w.players.cold[p].person;
    w.market.cooldown.insert((club, p), today.add_days(90));
    if w.people[who].mind == MindKind::External {
        let kind = DecisionKind::Trial { club, days: 14 };
        let options = kind.simple_options();
        w.decisions.push(Decision { person: who, player: p, kind, options, created: today, deadline: today.add_days(4), default: 0, answer: None, resolved: false });
        return;
    }
    start_trial(w, club, p);
}

pub fn start_trial(w: &mut World, club: ClubId, p: PlayerId) {
    let today = w.date;
    if w.deals.on_trial(p).is_some() {
        return;
    }
    w.deals.trials.push(Trial { player: p, club, from: today, until: today.add_days(14) });
    w.events.push(today, Visibility::Person(w.players.cold[p].person), EventKind::TrialStarted { player: p, club });
}
