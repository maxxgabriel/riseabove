//! Contract talks (08 §5). Every renewal, transfer's personal terms, free-agent
//! signing and first professional deal runs through here, for every player.
//!
//! The club opens with an offer below a private ceiling it will not show. The
//! player's side (their agent, if they have one) answers: accept, counter, or
//! walk. AI players answer in the same day; a human answers through a decision
//! whose options are built from the talks as they stand, with their own AI's
//! answer as the default. Clubs improve, hold firm or walk away depending on
//! how far apart the sides are, how much they want the player, and how many
//! rounds have passed.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Hidden, Money, PlayerId, StaffAttr, TalkId};
use pw_world::contract::{ContractKind, Loan, SquadStatus};
use pw_world::decision::{Choice, Decision, DecisionKind};
use pw_world::event::{Cause, Causes, EventKind, Visibility};
use pw_world::negotiation::{Negotiation, TalkEnd, TalkKind, TalkLine, TalkState, Terms};
use pw_world::{MemoryKind, MindKind, PlayerStatus, StaffRole, World};
use smallvec::SmallVec;

use crate::{consider, contracts, market};

/// The person negotiating on the club's side (director of football, else manager).
fn club_negotiator(w: &World, club: ClubId) -> Option<pw_core::PersonId> {
    let c = &w.clubs[club];
    let s = c.staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::DirectorOfFootball).or(c.manager.get())?;
    Some(w.staff[s].person)
}

fn negotiating_skill(w: &World, club: ClubId) -> f32 {
    club_negotiator(w, club).map_or(10.0, |p| consider::staff_attr(w, p, StaffAttr::Negotiating))
}

fn agent_skill(w: &World, p: PlayerId) -> (pw_core::AgentId, f32) {
    match w.agents.agent_of(p) {
        Some(a) => (a, f32::from(w.agents.list[a].negotiating)),
        None => (pw_core::AgentId::NONE, 6.0),
    }
}

/// Open talks. AI players settle the same day; an external player gets a decision.
#[allow(clippy::too_many_arguments)]
pub fn open(w: &mut World, p: PlayerId, club: ClubId, kind: TalkKind, seller: ClubId, fee: Money, loan: Option<Loan>, causes: Causes) -> Option<TalkId> {
    if w.market.talking.contains_key(&p) {
        return None;
    }
    let today = w.date;
    let premium = {
        let mut rng = Rng::keyed(&[w.seed, stream::NEGOTIATION, u64::from(p.0), u64::from(club.0), today.0 as u64]);
        rng.range_f32(0.88, 1.02)
    };
    let mut first = market::new_contract(w, p, club, premium);
    if kind == TalkKind::Renewal || kind == TalkKind::FirstPro {
        let current = w.players.cold[p].contract.current_wage(today);
        first.wage = first.wage.max(current);
    }
    let years = market::contract_years(w.age(p)).min(pw_world::rules::max_contract_years_for(w, w.clubs[club].nation, w.age(p)));
    let offer = Terms { signing_fee: first.wage * 2, ..Terms::from_contract(&first, years) };
    let limit = club_limit(w, club, p, &offer);
    let (agent, _) = agent_skill(w, p);
    let player_person = w.players.cold[p].person;
    let id = w.talks.next_id();
    let ev = w.events.push_caused(today, Visibility::Person(player_person), EventKind::TalksOpened { talk: id, player: p, club }, causes.clone());
    w.market.talking.insert(p, id);
    w.talks.push(Negotiation {
        player: p,
        club,
        kind,
        seller,
        fee,
        loan,
        offer,
        ask: None,
        limit,
        agent,
        round: 1,
        max_rounds: 4,
        opened: today,
        deadline: today.add_days(if kind == TalkKind::Renewal { 21 } else { 10 }),
        state: TalkState::PlayerTurn,
        end: None,
        decision: pw_core::DecisionId::NONE,
        log: vec![(today, TalkLine::ClubOffer(offer))],
        causes,
        event: ev,
    });
    player_turn(w, id);
    Some(id)
}

pub fn open_renewal(w: &mut World, p: PlayerId, club: ClubId, causes: Causes) -> Option<TalkId> {
    let youth = w.players.cold[p].contract.kind == ContractKind::Youth && w.age(p) >= u32::from(pw_world::rules::MIN_PRO_AGE);
    open(w, p, club, if youth { TalkKind::FirstPro } else { TalkKind::Renewal }, ClubId::NONE, 0, None, causes)
}

/// What a club will go to, never shown to the player: wage room, wage
/// structure and how much they want this player.
fn club_limit(w: &World, club: ClubId, p: PlayerId, offer: &Terms) -> Terms {
    let f = &w.clubs[club].finance;
    let room = (f.wage_budget - f.wage_bill).max(f.wage_budget / 25).max(offer.wage);
    let (ca, _, pa, _) = crate::perception::club_view(w, club, p);
    let ideal = market::ideal_ca(w.clubs[club].reputation);
    let want = ((ca - ideal) / 20.0 + (pa - ca).max(0.0) / 60.0 + consider::club_need_for(w, club, p) * 0.3).clamp(-0.5, 1.0);
    let stretch = (1.12 + 0.35 * want).clamp(1.02, 1.6);
    let status = if want > 0.5 { Some(SquadStatus::Important) } else if want > 0.1 { Some(SquadStatus::Regular) } else { None };
    // The board's wage structure caps what anyone earns, unless this is a signing
    // the club badly wants.
    let ceiling = if want > 0.6 { Money::MAX } else { crate::governance::wage_ceiling(w, club).max(offer.wage) };
    Terms {
        wage: ((offer.wage as f32 * stretch) as Money).min(offer.wage + room).min(ceiling),
        signing_fee: (offer.signing_fee as f32 * stretch * 1.5) as Money,
        release_clause: 0,
        status,
        years: offer.years,
        ..*offer
    }
}

/// The player's side: what they would settle for, from demands, personality,
/// their agent, how badly they want this, and alternatives they know about.
fn reservation(w: &World, t: &Negotiation) -> (Money, Option<SquadStatus>) {
    let p = t.player;
    let pid = person_id(w, p);
    let person = &w.people[pid];
    let demand = market::wage_demand(w, p, t.club) as f32;
    let ambition = person.hidden.f(Hidden::Ambition);
    let loyalty = person.hidden.f(Hidden::Loyalty);
    let (_, agent) = agent_skill(w, p);
    let interest = f32::from(consider::heard_interest(w, pid).min(3));
    let mut k = 0.95 + (ambition - 10.0) * 0.012 + (agent - 10.0) * 0.008 + interest * 0.04;
    if t.kind == TalkKind::Renewal {
        k -= (loyalty - 10.0) * 0.01;
        // A player who feels let down by the club asks for more.
        if let Some(mgr) = w.clubs[t.club].manager.get().map(|m| w.staff[m].person) {
            k += consider::grievance(w, pid, mgr) * 0.05;
        }
    }
    if w.players.hot[p].status == PlayerStatus::FreeAgent {
        k -= (consider::days_unattached(w, p) as f32 / 365.0).min(1.0) * 0.35;
    }
    let status = {
        let (ca, _, _, _) = crate::perception::club_view(w, t.club, p);
        let ideal = market::ideal_ca(w.clubs[t.club].reputation);
        if ambition >= 14.0 && ca >= ideal + 5.0 { Some(SquadStatus::Regular) } else { None }
    };
    ((demand * k.clamp(0.5, 1.6)) as Money, status)
}

fn person_id(w: &World, p: PlayerId) -> pw_core::PersonId {
    w.players.cold[p].person
}

/// What the player's own AI would do with the current offer.
pub fn ai_choice(w: &World, id: TalkId) -> Choice {
    let t = &w.talks[id];
    let (res, want_status) = reservation(w, t);
    let status_ok = want_status.is_none_or(|s| t.offer.status.is_some_and(|o| o <= s));
    // For moves, the move itself must appeal.
    if matches!(t.kind, TalkKind::Transfer | TalkKind::FreeAgent) {
        let u = crate::decisions::move_utility(w, t.player, t.club, t.offer.wage);
        if u < -0.25 && w.players.hot[t.player].status != PlayerStatus::FreeAgent {
            return Choice::Reject;
        }
    }
    if t.offer.wage >= res && status_ok {
        return Choice::Accept;
    }
    if t.round >= t.max_rounds {
        return if t.offer.wage as f32 >= res as f32 * 0.9 { Choice::Accept } else { Choice::Reject };
    }
    let ask = (res as f32 * 1.06) as Money;
    Choice::Counter { wage: ask.max(t.offer.wage + t.offer.wage / 20), years: t.offer.years, status: want_status.or(t.offer.status), release_clause: 0 }
}

/// Options a player can put across the table right now.
fn options(w: &World, id: TalkId) -> SmallVec<[Choice; 5]> {
    let t = &w.talks[id];
    let mut v: SmallVec<[Choice; 5]> = SmallVec::new();
    v.push(Choice::Accept);
    if t.round < t.max_rounds {
        let o = t.offer;
        v.push(Choice::Counter { wage: o.wage + o.wage / 10, years: o.years, status: o.status, release_clause: 0 });
        v.push(Choice::Counter { wage: o.wage + o.wage / 4, years: o.years, status: o.status, release_clause: 0 });
        let better = match o.status {
            None | Some(SquadStatus::Squad) | Some(SquadStatus::ImpactSub) | Some(SquadStatus::Fringe) | Some(SquadStatus::Backup) | Some(SquadStatus::Youngster) | Some(SquadStatus::NotNeeded) => SquadStatus::Regular,
            Some(SquadStatus::Regular) => SquadStatus::Important,
            Some(s) => s,
        };
        v.push(Choice::Counter { wage: o.wage + o.wage / 20, years: o.years, status: Some(better), release_clause: (w.players.cold[t.player].value as f32 * 2.5) as Money });
    }
    v.push(Choice::Reject);
    v
}

fn player_turn(w: &mut World, id: TalkId) {
    let t = &w.talks[id];
    if t.state != TalkState::PlayerTurn {
        return;
    }
    let person = w.players.cold[t.player].person;
    if w.people[person].mind == MindKind::External {
        let opts = options(w, id);
        let ai = ai_choice(w, id);
        let default = match ai {
            Choice::Accept => 0,
            Choice::Reject | Choice::Decline | Choice::Respond(_) => (opts.len() - 1) as u8,
            Choice::Counter { wage, .. } => opts
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Choice::Counter { .. }))
                .min_by_key(|(_, c)| match c {
                    Choice::Counter { wage: x, .. } => (x - wage).abs(),
                    _ => Money::MAX,
                })
                .map_or(0, |(i, _)| i as u8),
        };
        let deadline = w.talks[id].deadline.min(w.date.add_days(5));
        let d = w.decisions.push(Decision {
            person,
            player: w.talks[id].player,
            kind: DecisionKind::Negotiation { talk: id },
            options: opts,
            created: w.date,
            deadline,
            default,
            answer: None,
            resolved: false,
        });
        w.talks[id].decision = d;
        return;
    }
    let choice = ai_choice(w, id);
    answer(w, id, choice);
}

/// Apply the player's side of a round (from an AI mind or a human decision).
pub fn answer(w: &mut World, id: TalkId, choice: Choice) {
    if !w.talks[id].is_open() {
        return;
    }
    if !still_valid(w, id) {
        end(w, id, TalkEnd::Overtaken);
        return;
    }
    let today = w.date;
    match choice {
        Choice::Accept => {
            w.talks[id].log.push((today, TalkLine::PlayerAccepted));
            complete(w, id);
        }
        Choice::Counter { wage, years, status, release_clause } => {
            let t = &mut w.talks[id];
            let ask = Terms { wage, years, status, release_clause, ..t.offer };
            t.ask = Some(ask);
            t.state = TalkState::ClubTurn;
            t.log.push((today, TalkLine::PlayerCounter(ask)));
            club_turn(w, id);
        }
        _ => {
            w.talks[id].log.push((today, TalkLine::PlayerRejected));
            end(w, id, TalkEnd::PlayerRejected);
        }
    }
}

fn club_turn(w: &mut World, id: TalkId) {
    let today = w.date;
    let (club, player, ask, offer, limit, round, agent) = {
        let t = &w.talks[id];
        (t.club, t.player, t.ask.unwrap_or(t.offer), t.offer, t.limit, t.round, t.agent)
    };
    let club_skill = negotiating_skill(w, club);
    let agent_n = if agent.is_some() { f32::from(w.agents.list[agent].negotiating) } else { 6.0 };
    let mut rng = Rng::keyed(&[w.seed, stream::NEGOTIATION, u64::from(id.0), u64::from(round)]);
    let status_ok = ask.status.is_none_or(|s| limit.status.is_some_and(|l| l <= s));
    let gap = (ask.wage - limit.wage) as f32 / limit.wage.max(1) as f32;
    if gap > 0.3 && round >= 2 && rng.chance(0.5 + gap) {
        w.talks[id].log.push((today, TalkLine::ClubWalkedAway));
        end(w, id, TalkEnd::ClubWalkedAway);
        return;
    }
    // Move toward the ask; better agents extract more, better club negotiators give less.
    let pull = (0.45 + (agent_n - club_skill) * 0.025 + rng.normal() * 0.05).clamp(0.15, 0.9);
    let wage = if ask.wage <= limit.wage && rng.chance(0.35 + pull * 0.4) {
        ask.wage
    } else {
        (offer.wage + ((ask.wage.min(limit.wage) - offer.wage) as f32 * pull) as Money).max(offer.wage)
    };
    let status = if status_ok { ask.status } else { offer.status.or(limit.status) };
    let release_clause = if ask.release_clause > 0 && rng.chance(0.4) { ask.release_clause } else { offer.release_clause };
    let improved = Terms { wage, status, release_clause, signing_fee: offer.signing_fee.max(ask.signing_fee.min(limit.signing_fee)), ..offer };
    let t = &mut w.talks[id];
    t.log.push((today, if improved.wage > offer.wage || improved.status != offer.status { TalkLine::ClubImproved(improved) } else { TalkLine::ClubHeldFirm }));
    t.offer = improved;
    t.round += 1;
    t.state = TalkState::PlayerTurn;
    let _ = player;
    player_turn(w, id);
}

fn still_valid(w: &World, id: TalkId) -> bool {
    let t = &w.talks[id];
    let h = &w.players.hot[t.player];
    match t.kind {
        TalkKind::Transfer => h.club == t.seller && h.status == PlayerStatus::Active,
        TalkKind::Renewal | TalkKind::FirstPro => h.club == t.club && h.status == PlayerStatus::Active,
        TalkKind::FreeAgent => h.status == PlayerStatus::FreeAgent,
        TalkKind::Loan => h.club == t.seller,
        TalkKind::PreContract => h.club == t.seller && h.status == PlayerStatus::Active,
    }
}

fn complete(w: &mut World, id: TalkId) {
    let today = w.date;
    let t = w.talks[id].clone();
    // Registration, work permit, quota and minors rules can still sink a deal.
    let check = match t.kind {
        TalkKind::Transfer | TalkKind::FreeAgent => Some(pw_world::rules::can_sign(w, t.club, t.player, today)),
        TalkKind::Loan => Some(pw_world::rules::can_loan(w, t.club, t.seller, t.player, today)),
        _ => None,
    };
    if let Some(o) = check {
        if let Some(&reason) = o.reasons.first() {
            w.talks[id].log.push((today, TalkLine::ClubWalkedAway));
            let person = w.players.cold[t.player].person;
            let causes: Causes = pw_world::causes![Cause::Event(t.event), Cause::Fact(pw_world::Fact::Rule { reason })];
            w.events.push_caused(today, Visibility::Person(person), EventKind::TalksCollapsed { talk: id, player: t.player, club: t.club }, causes);
            let x = &mut w.talks[id];
            x.state = TalkState::Collapsed;
            x.end = Some(TalkEnd::Blocked);
            w.market.talking.remove(&t.player);
            return;
        }
    }
    let kind = if w.age(t.player) < 17 { ContractKind::Youth } else { ContractKind::Professional };
    let contract = t.offer.to_contract(t.club, kind, today);
    match t.kind {
        TalkKind::Transfer => market::execute_transfer(w, t.player, t.club, t.seller, t.fee, contract),
        TalkKind::FreeAgent => market::execute_transfer(w, t.player, t.club, ClubId::NONE, 0, contract),
        TalkKind::Renewal | TalkKind::FirstPro => contracts::renew(w, t.player, contract),
        TalkKind::Loan => {
            if let Some(l) = t.loan.clone() {
                market::execute_loan(w, t.player, l);
            }
        }
        TalkKind::PreContract => crate::deals::sign_pre_contract(w, t.player, t.club, t.offer),
    }
    if t.kind == TalkKind::Transfer {
        crate::deals::on_completed(w, t.player, t.club);
    }
    // Signing-on fee reaches the player's bank account, net of the agent's cut.
    let person = w.players.cold[t.player].person;
    let fee_pct = w.agents.of_player.get(&t.player).map_or(0, |r| r.fee_pct);
    let agent_cut = t.offer.signing_fee * Money::from(fee_pct) / 100;
    w.lives[person].finances.savings += (t.offer.signing_fee - agent_cut) * 6 / 10;
    if t.agent.is_some() {
        w.agents.strengthen(t.agent, t.club, 8);
        if let Some(r) = w.agents.of_player.get_mut(&t.player) {
            r.satisfaction = r.satisfaction.saturating_add(8).min(100);
        }
    }
    if let Some(s) = t.offer.status.filter(|_| t.kind != TalkKind::PreContract) {
        w.players.cold[t.player].contract.promised_status = Some(s);
    }
    let x = &mut w.talks[id];
    x.state = TalkState::Agreed;
    x.end = Some(TalkEnd::Signed);
    w.market.talking.remove(&t.player);
}

fn end(w: &mut World, id: TalkId, how: TalkEnd) {
    let today = w.date;
    let t = w.talks[id].clone();
    {
        let x = &mut w.talks[id];
        x.state = TalkState::Collapsed;
        x.end = Some(how);
    }
    w.market.talking.remove(&t.player);
    if t.kind == TalkKind::Transfer {
        crate::deals::on_talks_failed(w, t.player, t.club);
    }
    if how == TalkEnd::Overtaken {
        return;
    }
    let person = w.players.cold[t.player].person;
    let causes: Causes = pw_world::causes![Cause::Event(t.event)];
    w.events.push_caused(today, Visibility::Person(person), EventKind::TalksCollapsed { talk: id, player: t.player, club: t.club }, causes);
    w.market.cooldown.insert((t.club, t.player), today.add_days(if t.kind == TalkKind::Renewal { 90 } else { 120 }));
    // A renewal that breaks down sours things with the club's decision-makers.
    if t.kind == TalkKind::Renewal {
        if let Some(n) = club_negotiator(w, t.club) {
            let compat = consider::compat(w, person, n);
            let kind = if how == TalkEnd::ClubWalkedAway { MemoryKind::LetDown } else { MemoryKind::HardBargain };
            w.social.remember(person, n, kind, today, t.event, false, 0.8, compat);
            w.social.remember(n, person, MemoryKind::HardBargain, today, t.event, false, 0.6, compat);
        }
    }
}

/// Talks that ran past their deadline without agreement collapse.
pub fn daily(w: &mut World) {
    let today = w.date;
    let mut expired: Vec<TalkId> = w
        .market
        .talking
        .values()
        .copied()
        .filter(|&id| {
            let t = &w.talks[id];
            t.is_open() && today > t.deadline && w.decisions.all.get(t.decision).is_none_or(|d| d.resolved)
        })
        .collect();
    expired.sort();
    for id in expired {
        end(w, id, TalkEnd::TimedOut);
    }
}

/// Is this player in open talks with anyone?
pub fn in_talks(w: &World, p: PlayerId) -> bool {
    w.market.talking.contains_key(&p)
}
