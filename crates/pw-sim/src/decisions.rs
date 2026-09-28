//! Minds and the DecisionPort (01 §5, S5–S6). Every request to a person goes
//! through here: AI minds answer now; external minds receive a decision whose
//! options were generated from state and whose default is what their own AI
//! mind would have chosen (P1). Contract talks and conversations have their
//! own resolvers (`negotiation`, `talk`); this module routes answers to them.

use pw_core::rng::{Rng, hash_key, noise, stream};
use pw_core::{ClubId, Hidden, Money, PersonId, PlayerId};
use pw_world::decision::{Choice, Decision, DecisionKind};
use pw_world::event::{Cause, Causes, EventKind, Visibility};
use pw_world::negotiation::TalkKind;
use pw_world::world::PendingDeal;
use pw_world::{Loan, MemoryKind, MindKind, PartnerAsk, TeamKind, World};

use crate::{consider, market, negotiation};

#[derive(Clone, Debug)]
pub enum Proposal {
    /// Clubs agreed a fee; personal terms follow in talks.
    Transfer { buyer: ClubId, seller: ClubId, fee: Money },
    /// The club wants to extend the player's deal.
    Renewal,
    Loan { loan: Loan },
    FreeAgent { club: ClubId },
}

/// Share of first-team minutes a player of ability `ca` could expect at `club`.
pub fn expected_share(w: &World, club: ClubId, ca: u8, exclude: PlayerId) -> f32 {
    let Some(team) = w.club_team(club, TeamKind::First) else { return 0.1 };
    let rank = w.teams[team].squad.iter().filter(|&&p| p != exclude && w.players.cold[p].ca > ca).count();
    match rank {
        0..=10 => 0.9 - rank as f32 * 0.03,
        11..=15 => 0.35,
        16..=20 => 0.15,
        _ => 0.05,
    }
}

/// Player-side utility of moving to `club` for `wage` vs staying put (08 §4, 17 §6).
/// Weighs level, minutes, money, trophies, loyalty, the household's ties and
/// how things stand with the current manager.
pub fn move_utility(w: &World, p: PlayerId, club: ClubId, wage: Money) -> f32 {
    let h = &w.players.hot[p];
    let c = &w.players.cold[p];
    let who = c.person;
    let person = &w.people[who];
    let hid = |x: Hidden| person.hidden.f(x);
    let cur = h.club;
    let rep = |cl: ClubId| if cl.is_some() { f32::from(w.clubs[cl].reputation) / 10_000.0 } else { 0.0 };
    let level = rep(club) - rep(cur);
    let pt = expected_share(w, club, c.ca, p) - if cur.is_some() { expected_share(w, cur, c.ca, p) } else { 0.0 };
    let cur_wage = if cur.is_some() { c.contract.current_wage(w.date) } else { 0 };
    let money = (pw_core::math::ln((wage as f32 + 50.0) / (cur_wage as f32 + 50.0)) / 2.0).clamp(-1.0, 1.0);
    let years_here = if cur.is_some() { (c.joined.days_until(w.date) as f32 / 365.0).min(10.0) } else { 0.0 };
    let loyalty = years_here * hid(Hidden::Loyalty) / 20.0 * 0.08;
    let to_nation = w.clubs[club].nation;
    let life = -consider::household_move_cost(w, who, to_nation);
    // Unhappiness with the current manager or fans makes leaving more attractive.
    let push = match w.manager_of_player(p) {
        Some(m) if cur.is_some() => consider::grievance(w, who, m) * 0.15 + consider::minutes_grievance(w, p) * 0.2,
        _ => 0.0,
    } + if w.market.has_requested(p) { 0.2 } else { 0.0 };

    let (wl, wp, wm, wt, wy, wf) = if hid(Hidden::Ambition) >= 15.0 {
        (0.30, 0.20, 0.15, 0.20, 0.05, 0.10)
    } else if hid(Hidden::Loyalty) >= 15.0 {
        (0.15, 0.20, 0.10, 0.10, 0.30, 0.15)
    } else if hid(Hidden::Adaptability) <= 7.0 {
        (0.15, 0.20, 0.15, 0.05, 0.10, 0.35)
    } else if hid(Hidden::Professionalism) <= 8.0 {
        (0.15, 0.10, 0.40, 0.10, 0.05, 0.20)
    } else {
        (0.22, 0.22, 0.2, 0.12, 0.1, 0.14)
    };
    wl * level * 3.0 + wp * pt * 2.0 + wm * money + wt * level * 2.0 - wy * loyalty * 3.0 + wf * life * 2.0 + push
}

/// What this player's own AI mind would do with a loan proposal.
fn ai_accepts_loan(w: &World, p: PlayerId, loan: &Loan) -> bool {
    let jitter = 0.05 * noise(&[w.seed, stream::MIND, u64::from(p.0), w.date.0 as u64]);
    let c = &w.players.cold[p];
    let gain = expected_share(w, loan.club, c.ca, p) - expected_share(w, loan.parent, c.ca, p);
    let cost = consider::household_move_cost(w, c.person, w.clubs[loan.club].nation) * 0.3;
    gain - cost + jitter > 0.1
}

/// Put an offer to a player. Returns `Some(accepted)` when resolved now, `None` when pending.
pub fn propose(w: &mut World, p: PlayerId, prop: Proposal) -> Option<bool> {
    let today = w.date;
    match prop {
        Proposal::Transfer { buyer, seller, fee } => {
            let causes: Causes = pw_world::causes![Cause::Fact(pw_world::Fact::Tracking { club: buyer, player: p, minutes: consider::club_tracking(w, buyer, p) })];
            negotiation::open(w, p, buyer, TalkKind::Transfer, seller, fee, None, causes).map(|_| true)
        }
        Proposal::FreeAgent { club } => {
            let causes: Causes = pw_world::causes![Cause::Fact(pw_world::Fact::SquadNeed { club })];
            negotiation::open(w, p, club, TalkKind::FreeAgent, ClubId::NONE, 0, None, causes).map(|_| true)
        }
        Proposal::Renewal => {
            let club = w.players.hot[p].club;
            let causes: Causes = pw_world::causes![Cause::Fact(pw_world::Fact::ContractRunningDown { player: p, days: consider::contract_days_left(w, p).max(0) as u16 })];
            negotiation::open_renewal(w, p, club, causes).map(|_| true)
        }
        Proposal::Loan { loan } => {
            let accept = ai_accepts_loan(w, p, &loan);
            let person = w.players.cold[p].person;
            if w.people[person].mind != MindKind::External {
                if accept {
                    market::execute_loan(w, p, loan);
                } else {
                    refuse_loan(w, p, &loan);
                }
                return Some(accept);
            }
            let kind = DecisionKind::LoanOffer { loan: loan.clone() };
            let options = kind.simple_options();
            let id = w.decisions.push(Decision {
                person,
                player: p,
                kind,
                options,
                created: today,
                deadline: today.add_days(5),
                default: if accept { 0 } else { 1 },
                answer: None,
                resolved: false,
            });
            let (buyer, seller) = (loan.club, loan.parent);
            w.market.pending.push(PendingDeal { player: p, buyer, seller, fee: loan.fee, contract: w.players.cold[p].contract.clone(), loan: Some(loan), decision: id });
            None
        }
    }
}

fn refuse_loan(w: &mut World, p: PlayerId, loan: &Loan) {
    let today = w.date;
    w.market.cooldown.insert((loan.club, p), today.add_days(90));
    // The parent club's manager remembers being turned down.
    if let Some(m) = w.clubs[loan.parent].manager.get().map(|m| w.staff[m].person) {
        let who = w.players.cold[p].person;
        let compat = consider::compat(w, m, who);
        let ev = w.events.push(today, Visibility::Club(loan.parent), EventKind::Interest { player: p, club: loan.club });
        w.social.remember(m, who, MemoryKind::RefusedLoan, today, ev, false, 0.8, compat);
    }
}

/// A partner raises the next step (or the end). `who` answers with their mind.
pub fn partner_asks(w: &mut World, who: PersonId, partner: PersonId, ask: PartnerAsk) {
    let today = w.date;
    let accept = ai_partner_answer(w, who, partner, ask);
    if w.people[who].mind != MindKind::External {
        apply_partner_answer(w, who, partner, ask, accept);
        return;
    }
    let kind = DecisionKind::Partner { partner, ask };
    let options = kind.simple_options();
    let player = w.people[who].player;
    w.decisions.push(Decision {
        person: who,
        player,
        kind,
        options,
        created: today,
        deadline: today.add_days(10),
        default: if accept { 0 } else { 1 },
        answer: None,
        resolved: false,
    });
}

fn ai_partner_answer(w: &World, who: PersonId, partner: PersonId, ask: PartnerAsk) -> bool {
    let bond = w.lives[who].partner().filter(|p| p.person == partner).map_or(0, |p| p.bond);
    let loyal = w.people[who].hidden.f(Hidden::Loyalty) / 20.0;
    let age = consider::age(w, who);
    let r = (hash_key(&[w.seed, stream::FAMILY, u64::from(who.0), w.date.0 as u64]) % 1000) as f32 / 1000.0;
    match ask {
        PartnerAsk::MoveIn => f32::from(bond) / 100.0 + loyal * 0.2 > 0.7 + r * 0.2,
        PartnerAsk::Marry => f32::from(bond) / 100.0 + loyal * 0.3 + if age > 26.0 { 0.1 } else { -0.1 } > 0.85 + r * 0.2,
        PartnerAsk::Separate => bond < 40,
    }
}

pub fn apply_partner_answer(w: &mut World, who: PersonId, partner: PersonId, ask: PartnerAsk, accept: bool) {
    let today = w.date;
    if w.lives[who].partner().is_none_or(|p| p.person != partner) {
        return;
    }
    if accept {
        crate::life::advance_relationship(w, who, partner, ask);
    } else if ask != PartnerAsk::Separate {
        let compat = consider::compat(w, partner, who);
        w.social.remember(partner, who, MemoryKind::Refused, today, pw_core::EventId::NONE, false, 1.0, compat);
        if let Some(p) = w.lives[who].household.partner.as_mut() {
            p.bond = p.bond.saturating_sub(8);
        }
        if let Some(p) = w.lives[partner].household.partner.as_mut() {
            p.bond = p.bond.saturating_sub(8);
        }
    }
}

/// Apply every answered or expired decision (01 §5: defaults at deadlines).
pub fn resolve_due(w: &mut World) {
    for id in w.decisions.due(w.date) {
        let (p, person, kind, choice) = {
            let d = &w.decisions.all[id];
            (d.player, d.person, d.kind.clone(), d.chosen())
        };
        w.decisions.resolve(id);
        match kind {
            DecisionKind::Negotiation { talk } => negotiation::answer(w, talk, choice),
            DecisionKind::Meeting { meeting } => {
                let tone = match choice {
                    Choice::Respond(t) => t,
                    _ => pw_world::Tone::Calm,
                };
                crate::talk::respond(w, meeting, tone);
            }
            DecisionKind::Partner { partner, ask } => apply_partner_answer(w, person, partner, ask, choice == Choice::Accept),
            DecisionKind::LoanOffer { loan } => {
                let pending_idx = w.market.pending.iter().position(|d| d.decision == id);
                if let Some(i) = pending_idx {
                    w.market.pending.remove(i);
                }
                let valid = w.players.hot[p].club == loan.parent && w.players.cold[p].loan.is_none();
                if choice == Choice::Accept && valid {
                    market::execute_loan(w, p, loan);
                } else {
                    refuse_loan(w, p, &loan);
                }
            }
            DecisionKind::ContractOffer { contract, renewal, .. } => {
                if choice == Choice::Accept && renewal && w.players.hot[p].club == contract.club {
                    crate::contracts::renew(w, p, contract);
                }
            }
            DecisionKind::FreeAgentOffer { club, contract } => {
                if choice == Choice::Accept && w.players.hot[p].status == pw_world::PlayerStatus::FreeAgent {
                    market::execute_transfer(w, p, club, ClubId::NONE, 0, contract);
                }
            }
            DecisionKind::Trial { club, .. } => {
                if choice == Choice::Accept && w.players.hot[p].status != pw_world::PlayerStatus::Active {
                    if w.age(p) < 16 {
                        crate::youth::start_trial(w, club, p);
                    } else {
                        crate::deals::start_trial(w, club, p);
                    }
                }
            }
            DecisionKind::TransferTalks { .. } => {}
        }
    }
}

/// Stable per-(player, key) coin for AI choices that aren't utility-driven.
pub fn coin(w: &World, p: PlayerId, key: u64) -> f32 {
    (hash_key(&[w.seed, stream::MIND, u64::from(p.0), key]) % 10_000) as f32 / 10_000.0
}

/// Mind-seeded RNG for a person's own choices this day.
pub fn mind_rng(w: &World, person: PersonId, key: u64) -> Rng {
    Rng::keyed(&[w.seed, stream::MIND, u64::from(person.0), w.date.0 as u64, key])
}
