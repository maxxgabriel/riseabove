//! Club-to-club bargaining under uncertainty (locked design 3.12-3.14, 3.27-3.28).
//!
//! Neither side sees the other's limit; each holds a range it believes, narrowed by what the other does. Either side can say things
//! to move the other (rivals are bidding, the budget is spent, we will walk away, we are in no hurry). Whether that works depends on the
//! negotiator's skill, the speaker's record for honesty, what the listener can check from public facts, and how real the alternatives
//! are. Bluffs that are found out cost the club credibility. Nothing here is shown as a game, only through its effects and, later, the
//! history it leaves.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Money, PlayerId, PosGroup, StaffAttr};
use pw_world::deals::{ClubDeal, DealLine, Limit, RivalInfo, Signal, Source};
use pw_world::{PlayerStatus, StaffRole, World};

use crate::market;

/// How good the club's negotiator is, 0..1: the sporting director if there is one, else the manager.
pub fn skill(w: &World, club: ClubId) -> f32 {
    let c = &w.clubs[club];
    let who = c.staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::DirectorOfFootball && !w.staff[s].retired).or(c.manager.get());
    who.map_or(0.4, |s| w.staff[s].attrs.f(StaffAttr::Negotiating) / 20.0)
}

/// What each side believes about the other's limit when the enquiry is made: wide, and centred on the public estimate.
pub fn initial_limits(w: &World, buyer: ClubId, seller: ClubId, p: PlayerId) -> (Limit, Limit) {
    let public = market::value_of(w, p) as f64;
    let available = w.market.requests.contains_key(&p) || w.market.listed.contains_key(&p);
    // The buyer's skill narrows what it thinks the seller wants; a listed player is expected to cost less.
    let width = 0.55 - 0.3 * f64::from(skill(w, buyer));
    let lean = if available { 0.85 } else { 1.0 };
    let seller_min = Limit { lo: (public * (0.85 - width * 0.6) * lean) as Money, hi: (public * (1.15 + width) * lean) as Money };
    // The seller reads the buyer's means from what is public: its revenue and reputation.
    let revenue = crate::finance::season_revenue(w, buyer) as f64;
    let means = (revenue * 0.35).max(public * 0.5);
    let swidth = 0.5 - 0.3 * f64::from(skill(w, seller));
    let buyer_max = Limit { lo: (public * (0.9 - swidth * 0.4)) as Money, hi: (public * (1.2 + swidth)).min(means.max(public)) as Money };
    (seller_min, buyer_max)
}

fn honesty(w: &World, club: ClubId) -> f32 {
    w.boardroom.honesty_of(club)
}

fn open_rivals(w: &World, d: &ClubDeal) -> Vec<(ClubId, Money)> {
    w.deals.deals.iter().filter(|o| o.player == d.player && o.buyer != d.buyer && o.is_open() && matches!(o.state, pw_world::deals::DealState::Bid | pw_world::deals::DealState::Counter | pw_world::deals::DealState::Medical)).map(|o| (o.buyer, o.terms.fee)).collect()
}

/// At the enquiry: the player's agent, who hears things, tells the buyer about real competing interest, as far as the agent's
/// network and honesty go (section 3.28).
pub fn agent_tip(w: &mut World, i: usize) {
    let d = w.deals.deals[i].clone();
    let Some(agent) = w.agents.agent_of(d.player) else { return };
    let a = &w.agents.list[agent];
    let rivals = open_rivals(w, &d);
    let today = w.date;
    let tie = f32::from(w.agents.tie(agent, d.buyer)) / 100.0;
    let reliability = (f32::from(a.network) / 20.0 * 0.4 + f32::from(a.honesty) / 20.0 * 0.3 + tie * 0.3).clamp(0.1, 0.9);
    for (club, fee) in rivals.into_iter().take(1) {
        w.deals.deals[i].rival_info.push(RivalInfo { club, claimed: fee, source: Source::Agent, reliability, date: today, real: true });
    }
}

/// The seller's move before it answers a bid: it may say that others are bidding. If they are, it is true and believed; if not, it
/// is a bluff that works only as far as the seller's skill and record carry it. Returns true when a signal was sent.
pub fn seller_signals(w: &mut World, i: usize) -> bool {
    let d = w.deals.deals[i].clone();
    let today = w.date;
    if d.signals.rivals_claimed || d.round < 2 {
        return false;
    }
    let real = open_rivals(w, &d);
    let mut rng = Rng::keyed(&[w.seed, stream::MARKET, u64::from(d.seller.0), u64::from(d.buyer.0), u64::from(d.player.0), 0xb1, today.0 as u64]);
    let (claim, bluff) = if let Some(&(club, fee)) = real.first() {
        (Some((club, fee)), false)
    } else {
        // A bluff is tempting for a skilled negotiator who thinks the buyer could pay more, and risky for one with a record.
        let p = 0.10 + 0.35 * skill(w, d.seller) - 0.35 * (honesty(w, d.seller) - 0.6).max(0.0);
        (rng.chance(p.clamp(0.02, 0.5)).then_some((ClubId::NONE, d.terms.fee + d.terms.fee / 10)), true)
    };
    let Some((club, fee)) = claim else { return false };
    // How far the buyer believes it: the seller's record, discounted by what the buyer can check from public facts, and the buyer's own
    // negotiator seeing through it.
    let mut reliability = 0.25 + 0.5 * honesty(w, d.seller) - 0.25 * skill(w, d.buyer);
    if !bluff {
        reliability = reliability.max(0.6);
    }
    let x = &mut w.deals.deals[i];
    x.signals.rivals_claimed = true;
    x.rival_info.push(RivalInfo { club, claimed: fee, source: Source::Claim, reliability: reliability.clamp(0.05, 0.95), date: today, real: !bluff });
    x.log.push((today, DealLine::Signalled(Signal::RivalInterest)));
    true
}

/// The buyer's move before it answers a counter: when the gap is small it may say the budget is spent, when it has real alternatives
/// it may say it will walk. Both are believed or not by the seller in [`seller_ask`]. Returns true when a signal was sent.
pub fn buyer_signals(w: &mut World, i: usize, ask: Money, willing: f64, alternatives: usize) -> bool {
    let d = w.deals.deals[i].clone();
    let today = w.date;
    let mut sent = false;
    let gap = ask as f64 / willing.max(1.0);
    let mut rng = Rng::keyed(&[w.seed, stream::MARKET, u64::from(d.buyer.0), u64::from(d.seller.0), u64::from(d.player.0), 0xb2, today.0 as u64]);
    if d.round >= 2 && d.signals.budget_claimed == 0 && (1.0..1.2).contains(&gap) && rng.chance((0.15 + 0.4 * skill(w, d.buyer)).clamp(0.05, 0.6)) {
        // It says the money is gone: truthfully if the budget really is nearly spent, a bluff if not.
        let claimed = (willing * 0.95) as Money;
        let x = &mut w.deals.deals[i];
        x.signals.budget_claimed = claimed;
        x.log.push((today, DealLine::Signalled(Signal::BudgetGone)));
        sent = true;
    }
    if alternatives >= 2 && !d.signals.walked && gap > 1.0 {
        let x = &mut w.deals.deals[i];
        x.signals.walked = true;
        x.log.push((today, DealLine::Signalled(Signal::WalkAway)));
        sent = true;
    }
    sent
}

/// Whether the buyer has a real budget behind its claim to have run out: the seller can compare it with the buyer's public means.
fn budget_claim_credibility(w: &World, d: &ClubDeal) -> f32 {
    let claimed = d.signals.budget_claimed as f64;
    if claimed <= 0.0 {
        return 0.0;
    }
    let f = &w.clubs[d.buyer].finance;
    // A club sitting on far more cash than it claims to have is not believed.
    let rich = (f.balance.max(0) as f64 / claimed.max(1.0)).min(6.0) as f32;
    let cred = 0.25 + 0.5 * honesty(w, d.buyer) - 0.25 * skill(w, d.seller) - 0.08 * (rich - 1.0).max(0.0);
    cred.clamp(0.05, 0.9)
}

/// What the buyer is prepared to pay after hearing about rivals and how badly it needs to sign someone: rival interest raises the
/// ceiling by the reliability of what it heard (section 3.28); disbelieved claims add little.
pub fn buyer_ceiling(d: &ClubDeal, willing: f64) -> f64 {
    let heard: f32 = d.rival_info.iter().map(|r| r.reliability).sum::<f32>().min(1.5);
    willing * (1.0 + 0.10 * f64::from(heard))
}

/// The seller's asking price after everything it believes about the buyer: never below its own reservation, pulled up by what it thinks
/// the buyer can pay, and down by a credible claim of a spent budget or a credible threat to walk (section 3.13, 3.14).
pub fn seller_ask(w: &World, d: &ClubDeal, want: f64) -> f64 {
    let believed_max = f64::midpoint(d.seller_thinks_buyer_max.lo as f64, d.seller_thinks_buyer_max.hi as f64);
    // The seller opens at least at its reservation, and dares to ask more when it thinks the buyer can afford it.
    let mut ask = (want * 1.05).max((believed_max * 0.97).min(want * 1.35));
    if d.signals.budget_claimed > 0 {
        let cred = f64::from(budget_claim_credibility(w, d));
        let claimed = d.signals.budget_claimed as f64;
        // Believed, the seller comes down towards what the buyer says it has (never under the reservation).
        ask -= (ask - claimed.max(want)).max(0.0) * cred;
    }
    if d.signals.walked {
        let cred = f64::from((0.2 + 0.5 * honesty(w, d.buyer) - 0.2 * skill(w, d.seller)).clamp(0.05, 0.8));
        ask -= (ask - want).max(0.0) * 0.4 * cred;
    }
    // A buyer in no hurry is read as one who can wait.
    if d.signals.delays > 0 {
        ask -= (ask - want).max(0.0) * 0.12 * f64::from(d.signals.delays.min(3));
    }
    ask.max(want)
}

/// Public facts that change what a player is worth to this buyer, overnight (section 3.27): injuries to the buyer's own players in that
/// position group are reported, so the seller reads its bargaining position off them.
pub fn buyer_need_premium(w: &World, d: &ClubDeal) -> f64 {
    let group: PosGroup = w.players.cold[d.player].best_pos.group();
    let team = w.clubs[d.buyer].first_team();
    let hurt = w.teams[team].squad.iter().filter(|&&q| w.players.cold[q].best_pos.group() == group && w.players.hot[q].status == PlayerStatus::Active && w.players.hot[q].injury_days > 21).count();
    1.0 + 0.05 * hurt.min(3) as f64 * f64::from(0.6 + 0.4 * skill(w, d.seller))
}

/// The buyer narrows its belief about the seller after a counter, and the seller about the buyer after a raise: a counter says the
/// seller will take no more than it asked and more than was offered; a bid that fails says the buyer can pay at least that much.
pub fn learn_from_counter(d: &mut ClubDeal, bid: Money, ask: Money) {
    let m = &mut d.buyer_thinks_seller_min;
    m.lo = m.lo.max(bid);
    m.hi = m.hi.min(ask).max(m.lo);
}

pub fn learn_from_bid(d: &mut ClubDeal, bid: Money) {
    let m = &mut d.seller_thinks_buyer_max;
    m.lo = m.lo.max(bid);
    m.hi = m.hi.max(m.lo);
}

/// The fee at which the buyer aims its next bid: towards the middle of where it thinks the seller will settle.
pub fn next_bid(d: &ClubDeal, ask: Money, willing: f64) -> f64 {
    let anchor = f64::midpoint(d.buyer_thinks_seller_min.lo as f64, d.buyer_thinks_seller_min.hi.min(ask) as f64);
    let bid = d.terms.fee as f64;
    let step = if anchor > bid { (anchor - bid) * 0.6 } else { (ask as f64 - bid) * 0.5 };
    (bid + step).min(willing)
}

/// The deal was done. Anything said that turned out false is now known to the other side.
pub fn settle_bluffs(w: &mut World, i: usize, fee: Money) {
    let d = w.deals.deals[i].clone();
    // The buyer claimed a spent budget and then paid clearly more.
    if d.signals.budget_claimed > 0 && fee as f64 > d.signals.budget_claimed as f64 * 1.08 {
        let h = w.boardroom.honesty.entry(d.buyer).or_insert(0.6);
        *h = (*h - 0.08).max(0.05);
        w.deals.deals[i].signals.caught = true;
    }
    // The seller claimed rivals that never existed: the buyer sees nobody else sign him or bid.
    if d.rival_info.iter().any(|r| r.source == Source::Claim && !r.real) {
        let h = w.boardroom.honesty.entry(d.seller).or_insert(0.6);
        *h = (*h - 0.08).max(0.05);
        w.deals.deals[i].signals.caught = true;
    }
    // Honest dealing slowly rebuilds standing.
    for club in [d.buyer, d.seller] {
        if !w.deals.deals[i].signals.caught
            && let Some(h) = w.boardroom.honesty.get_mut(&club)
        {
            *h = (*h + 0.01).min(1.0);
        }
    }
}
