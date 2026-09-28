//! Club finances (07 §9): seasonal budgets, weekly income and wages.

use pw_core::{ClubId, Money, NationId};
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
        let income = season_revenue(w, club) / 52;
        let f = &mut w.clubs[club].finance;
        let bill = wages[club.0 as usize];
        f.wage_bill = bill;
        f.balance += income - bill;
        f.season_income += income;
        f.season_spend += bill;
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
