//! Club finances (07 §9): seasonal budgets, weekly income and wages.

use pw_core::{ClubId, Money, NationId};
use pw_world::club::Ownership;
use pw_world::{PlayerStatus, World};

/// Season revenue from broadcast, commercial and sponsorship (gate is per match).
/// Broadcast share, commercial income and base revenue, scaled by the
/// economy the club lives in (see `economy::club_revenue`).
pub fn season_revenue(w: &World, club: ClubId) -> Money {
    crate::economy::club_revenue(w, club)
}

pub fn season_budgets(w: &mut World, n: NationId) {
    let share = f64::from(w.data.tuning.finance.wage_share);
    for club in w.clubs.ids() {
        if w.clubs[club].nation != n {
            continue;
        }
        let revenue = season_revenue(w, club) as f64;
        let f = &mut w.clubs[club].finance;
        f.wage_budget = (revenue * share / 52.0) as Money;
        let reserves = (f.balance as f64 * 0.35).max(0.0);
        f.transfer_budget = (reserves + revenue * 0.12) as Money;
        f.season_income = 0;
        f.season_spend = 0;
        w.clubs[club].market.signed_this_window = 0;
    }
}

/// How many times a year owners take their share of the cash above the reserve: quarterly. A club's cash settles where the surplus it
/// earns equals what leaves, at the reserve plus the yearly surplus over (this times the owners' share). Taken once a year instead,
/// cash piled up for seasons above where a world starts, and every transfer budget read from it with it. (A constant, not tuning: the
/// tuning table is part of the save layout.)
const DISTRIBUTIONS_PER_YEAR: f64 = 4.0;

pub fn weekly(w: &mut World) {
    let today = w.date;
    let mut wages = vec![0 as Money; w.clubs.len()];
    for p in w.players.ids() {
        if w.players.hot[p].status != PlayerStatus::Active {
            continue;
        }
        let c = &w.players.cold[p];
        let wage = c.contract.current_wage(today);
        let payer = c.contract.club;
        if payer.is_none() {
            continue;
        }
        match &c.loan {
            Some(l) => {
                let loan_part = wage * Money::from(l.wage_share) / 100;
                wages[l.club.0 as usize] += loan_part;
                wages[payer.0 as usize] += wage - loan_part;
            }
            None => wages[payer.0 as usize] += wage,
        }
    }
    for s in w.staff.ids() {
        let st = &w.staff[s];
        if st.employed() {
            wages[st.club.0 as usize] += st.wage;
        }
    }
    for club in w.clubs.ids() {
        let revenue = season_revenue(w, club);
        let income = revenue / 52;
        let t = &w.data.tuning.finance;
        let running = (revenue as f64 * f64::from(t.operating_share) / 52.0) as Money;
        // What lies above the reserve leaves the club: dividends where owners take profit, investment in ground and facilities where they
        // put it back. Nothing else drains a profitable club, and money would be created without end.
        let reserve = (revenue as f64 * f64::from(t.reserve_years)) as Money;
        let payout = match w.clubs[club].ownership {
            Ownership::Private => 0.60,
            Ownership::InvestmentGroup => 0.80,
            Ownership::MemberOwned => 0.30,
            Ownership::Benefactor => 0.20,
            Ownership::StateBacked => 0.15,
        };
        let f = &mut w.clubs[club].finance;
        let bill = wages[club.0 as usize];
        f.wage_bill = bill;
        let excess = (f.balance - reserve).max(0);
        let out = (excess as f64 * payout * DISTRIBUTIONS_PER_YEAR / 52.0) as Money;
        f.balance += income - bill - running - out;
        f.season_income += income;
        f.season_spend += bill + running;
        if f.balance < 0 {
            f.debt = -f.balance;
        } else {
            f.debt = 0;
        }
    }
}

/// Money moves for a transfer fee.
pub fn pay_fee(w: &mut World, buyer: ClubId, seller: ClubId, fee: Money) {
    if fee <= 0 {
        return;
    }
    let b = &mut w.clubs[buyer].finance;
    b.balance -= fee;
    b.transfer_budget = (b.transfer_budget - fee).max(0);
    b.season_spend += fee;
    if seller.is_some() {
        let s = &mut w.clubs[seller].finance;
        s.balance += fee;
        s.transfer_budget += fee / 2;
        s.season_income += fee;
    }
}
