//! Brands and sponsorship (11 §6).
//!
//! Brands are generated per nation (plus a few global giants) with budgets
//! that follow the economy. Each summer they sponsor clubs; through the year
//! they look for faces — famous, well-behaved, the right age — and offer
//! endorsements. People accept or refuse with their own minds (or through a
//! decision). Deals pay monthly (clubs may take an image-rights share), cost
//! time (appearance days add stress), clash with club partners in the same
//! sector, and end when image collapses, fame fades or careers end.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{ClubId, Hidden, Money, NationId, PersonId, PlayerId};
use pw_world::commerce::{Brand, ClubDeal, ClubSlot, DealEnd, Endorsement, Sector};
use pw_world::decision::{Choice, Decision, DecisionKind, MindKind};
use pw_world::event::{EventKind, Visibility};
use pw_world::{PlayerStatus, SquadStatus, World};
use smallvec::SmallVec;

use crate::consider;

const NOUNS: [(Sector, &[&str]); 9] = [
    (Sector::Sportswear, &["Athletic", "Sport", "Strike"]),
    (Sector::Drinks, &["Brewing", "Drinks", "Springs"]),
    (Sector::Betting, &["Bet", "Odds", "Play"]),
    (Sector::Automotive, &["Motors", "Auto", "Cars"]),
    (Sector::Finance, &["Bank", "Capital", "Finance"]),
    (Sector::Fashion, &["& Co", "Couture", "Wear"]),
    (Sector::Technology, &["Tech", "Digital", "Systems"]),
    (Sector::Airline, &["Air", "Airways", "Skies"]),
    (Sector::Food, &["Foods", "Kitchen", "Farms"]),
];

/// Every nation's market gets brands; the biggest economies host giants.
pub fn ensure(w: &mut World) {
    let have: Vec<NationId> = w.commerce.brands.iter().map(|b| b.nation).collect();
    let mut by_econ: Vec<(NationId, f32)> = w.nations.iter_enumerated().map(|(id, n)| (id, n.economy)).collect();
    by_econ.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let giants: Vec<NationId> = by_econ.iter().take(5).map(|x| x.0).collect();
    for (n, econ) in by_econ {
        if have.contains(&n) || w.nations[n].last_names.is_empty() {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::MEDIA, u64::from(n.0), 0xb4a]);
        let count = (3.0 + econ * 9.0) as usize;
        for i in 0..count {
            let sector = Sector::ALL[(i + rng.index(Sector::ALL.len())) % Sector::ALL.len()];
            let giant = giants.contains(&n) && i < 2;
            let size = if giant { rng.range_i32(17, 20) as u8 } else { (3.0 + econ * 10.0 + rng.normal() * 3.0).clamp(1.0, 16.0) as u8 };
            let last = w.nations[n].last_names[rng.index(w.nations[n].last_names.len())];
            let nouns = NOUNS.iter().find(|x| x.0 == sector).map_or(&["Group"][..], |x| x.1);
            let name = format!("{} {}", w.names.get(last), nouns[rng.index(nouns.len())]);
            let budget = (f32::from(size).powi(3) * 2_000.0 * (0.3 + econ)) as Money;
            w.commerce.brands.push(Brand { name, nation: n, sector, size, budget, committed: 0, sensitivity: rng.range_i32(4, 18) as u8, youthful: rng.chance(0.4) });
        }
    }
}

/// July: budgets follow the economy; clubs sign shirt and kit deals.
pub fn yearly(w: &mut World) {
    let today = w.date;
    for b in w.commerce.brands.iter_mut() {
        b.committed = 0;
    }
    let infl = w.economy.global();
    for i in 0..w.commerce.brands.len() {
        let econ = w.nations.get(w.commerce.brands[i].nation).map_or(0.5, |n| n.economy);
        let b = &mut w.commerce.brands[i];
        b.budget = (f32::from(b.size).powi(3) * 2_000.0 * (0.3 + econ) * infl) as Money;
    }
    // Expired club deals go; open slots are filled.
    w.commerce.club_deals.retain(|d| d.end > today);
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].league.is_some()).collect();
    for club in clubs {
        for (slot, sector) in [(ClubSlot::Shirt, None), (ClubSlot::Kit, Some(Sector::Sportswear))] {
            if w.commerce.club_deals.iter().any(|d| d.club == club && d.slot == slot) {
                continue;
            }
            club_offer(w, club, slot, sector);
        }
    }
}

fn club_offer(w: &mut World, club: ClubId, slot: ClubSlot, sector: Option<Sector>) {
    let today = w.date;
    let c = &w.clubs[club];
    let rep = f32::from(c.reputation) / 10_000.0;
    let nation = c.nation;
    let econ = w.nations[nation].economy;
    // Bigger clubs attract brands from further away.
    let pick = w
        .commerce
        .brands
        .iter()
        .enumerate()
        .filter(|(_, b)| sector.is_none_or(|s| b.sector == s))
        .filter(|(_, b)| b.nation == nation || (b.size >= 15 && rep >= 0.6))
        .filter(|(_, b)| f32::from(b.size) / 20.0 <= rep + 0.35)
        .filter(|(_, b)| b.budget - b.committed > 0)
        .max_by(|a, b| {
            let fit = |x: &Brand| -(f32::from(x.size) / 20.0 - rep).abs() + if x.nation == nation { 0.1 } else { 0.0 };
            fit(a.1).total_cmp(&fit(b.1)).then(b.0.cmp(&a.0))
        })
        .map(|(i, _)| i);
    let Some(bi) = pick else { return };
    let base = rep * rep * 40_000_000.0 * (0.3 + econ) * w.economy.global();
    let fee = (base * if slot == ClubSlot::Shirt { 1.0 } else { 0.8 }) as Money;
    let fee = fee.min(w.commerce.brands[bi].budget - w.commerce.brands[bi].committed).max(20_000);
    w.commerce.brands[bi].committed += fee;
    let years = 3;
    w.commerce.club_deals.push(ClubDeal { brand: bi as u32, club, slot, fee_year: fee, start: today, end: today.add_months(12 * years) });
    w.events.push(today, Visibility::Public, EventKind::ClubSponsor { club, brand: bi as u32, slot, fee_year: fee });
}

/// Monthly: club sponsorship income, personal offers, clashes, endings.
pub fn monthly(w: &mut World) {
    pay_clubs(w);
    offers(w);
    review(w);
}

fn pay_clubs(w: &mut World) {
    let deals: Vec<(ClubId, Money)> = w.commerce.club_deals.iter().map(|d| (d.club, d.fee_year / 12)).collect();
    for (club, m) in deals {
        let f = &mut w.clubs[club].finance;
        f.balance += m;
        f.season_income += m;
    }
    // Image-rights shares of personal deals.
    let shares: Vec<(PersonId, Money, u8)> = w.commerce.endorsements.iter().filter(|e| e.ended.is_none() && e.club_share > 0).map(|e| (e.person, e.fee_year / 12, e.club_share)).collect();
    for (who, m, share) in shares {
        let club = w.club_of_person(who);
        if club.is_some() {
            let cut = m * Money::from(share) / 100;
            w.clubs[club].finance.balance += cut;
            w.clubs[club].finance.season_income += cut;
        }
    }
}

/// What a person is worth to a brand per year.
fn value_to(w: &World, b: &Brand, who: PersonId, p: PlayerId) -> Money {
    let fame = f32::from(w.renown.of(who).fame) / 10_000.0;
    let world = if p.is_some() { f32::from(w.players.cold[p].rep.world) / 10_000.0 } else { 0.0 };
    let reach = (fame * 0.6 + world * 0.4).powi(2);
    let young = p.is_some() && w.age(p) <= 23;
    let fit = if b.youthful == young { 1.2 } else { 0.8 };
    (reach * 4_000_000.0 * f32::from(b.size) / 20.0 * fit * w.economy.global()) as Money
}

fn offers(w: &mut World) {
    let today = w.date;
    let month = (today.year() * 12) as u64 + u64::from(today.month());
    // Faces worth approaching: the famous and well-regarded.
    let faces: Vec<(PersonId, PlayerId)> = w
        .renown
        .people
        .iter()
        .filter(|(_, r)| r.fame >= 2500)
        .map(|(&who, _)| (who, w.people[who].player))
        .filter(|&(who, p)| p.is_none() || w.players.hot[p].status != PlayerStatus::Retired || w.renown.of(who).fame >= 6000)
        .filter(|&(who, _)| w.media.image.get(&who).copied().unwrap_or(0) > -150)
        .collect();
    for (who, p) in faces {
        let mut rng = Rng::keyed(&[w.seed, stream::MEDIA, u64::from(who.0), month, 0xe0d]);
        if !rng.chance(0.15) {
            continue;
        }
        let home = w.lives.get(who).map_or(w.people[who].nation, |l| l.home);
        let held: SmallVec<[Sector; 4]> = w.commerce.active_for(who).map(|e| w.commerce.brands[e.brand as usize].sector).collect();
        if held.len() >= 4 {
            continue;
        }
        let cand: Vec<usize> = w
            .commerce
            .brands
            .iter()
            .enumerate()
            .filter(|(_, b)| !held.contains(&b.sector) && (b.nation == home || b.nation == w.people[who].nation || b.size >= 15))
            .filter(|(_, b)| b.budget - b.committed > 0)
            .map(|(i, _)| i)
            .collect();
        if cand.is_empty() {
            continue;
        }
        let bi = cand[rng.index(cand.len())];
        let fee = value_to(w, &w.commerce.brands[bi], who, p).min(w.commerce.brands[bi].budget - w.commerce.brands[bi].committed);
        if fee < 10_000 {
            continue;
        }
        let days = rng.range_i32(1, 4) as u8;
        let years = rng.range_i32(1, 3) as u8;
        propose(w, who, bi as u32, fee, years, days);
    }
}

/// An endorsement offer: the person's mind decides, or a human is asked.
fn propose(w: &mut World, who: PersonId, brand: u32, fee_year: Money, years: u8, days: u8) {
    let today = w.date;
    let accept = ai_accepts(w, who, brand, fee_year, days);
    if w.people[who].mind != MindKind::External {
        if accept {
            sign(w, who, brand, fee_year, years, days);
        }
        return;
    }
    let kind = DecisionKind::Endorsement { brand, fee_year, years, days };
    let options = kind.simple_options();
    w.decisions.push(Decision {
        person: who,
        player: w.people[who].player,
        kind,
        options,
        created: today,
        deadline: today.add_days(10),
        default: if accept { 0 } else { 1 },
        answer: None,
        resolved: false,
    });
}

fn ai_accepts(w: &World, who: PersonId, brand: u32, fee_year: Money, days: u8) -> bool {
    let b = &w.commerce.brands[brand as usize];
    let income = w.lives.get(who).map_or(1, |l| l.finances.income.max(1));
    let worth = fee_year as f32 / 12.0 / income as f32;
    let prof = consider::hid(w, who, Hidden::Professionalism) / 20.0;
    let sport = consider::hid(w, who, Hidden::Sportsmanship) / 20.0;
    let ambition = consider::hid(w, who, Hidden::Ambition) / 20.0;
    // Betting and drinks sit badly with some; time costs the professional.
    let taste = match b.sector {
        Sector::Betting => -0.4 * sport,
        Sector::Drinks => -0.2 * prof,
        _ => 0.0,
    };
    let score = worth * 2.0 + ambition * 0.3 + taste - f32::from(days) * 0.05 * prof;
    score > 0.25
}

/// Apply an accepted endorsement (AI or decision).
pub fn sign(w: &mut World, who: PersonId, brand: u32, fee_year: Money, years: u8, days: u8) {
    let today = w.date;
    let club = w.club_of_person(who);
    // Big clubs take a share of image rights.
    let club_share = if club.is_some() && w.clubs[club].reputation >= 7000 { 20 } else { 0 };
    let idx = w.commerce.endorsements.len() as u32;
    w.commerce.endorsements.push(Endorsement { brand, person: who, fee_year, start: today, end: today.add_months(12 * i32::from(years)), days, club_share, ended: None });
    w.commerce.by_person.entry(who).or_default().push(idx);
    w.commerce.brands[brand as usize].committed += fee_year;
    w.events.push(today, Visibility::Public, EventKind::Endorsed { person: who, brand, fee_year });
    let r = w.renown.people.entry(who).or_default();
    r.fame = r.fame.saturating_add(100).min(10_000);
}

/// Answer to a human's endorsement decision.
pub fn answer(w: &mut World, who: PersonId, kind: &DecisionKind, choice: Choice) {
    if let DecisionKind::Endorsement { brand, fee_year, years, days } = *kind {
        if choice == Choice::Accept {
            sign(w, who, brand, fee_year, years, days);
        }
    }
}

/// Deals end for scandal, clashes, retirement or when they run out; the
/// obligations cost time and calm.
fn review(w: &mut World) {
    let today = w.date;
    let n = w.commerce.endorsements.len();
    for i in 0..n {
        let e = w.commerce.endorsements[i];
        if e.ended.is_some() {
            continue;
        }
        let b = &w.commerce.brands[e.brand as usize];
        let (sector, sensitivity) = (b.sector, b.sensitivity);
        let image = w.media.image.get(&e.person).copied().unwrap_or(0);
        let p = w.people[e.person].player;
        let retired = p.is_some() && w.players.hot[p].status == PlayerStatus::Retired;
        let fame = w.renown.of(e.person).fame;
        let why = if image < -(i16::from(sensitivity) * 25) {
            Some(DealEnd::Scandal)
        } else if retired && fame < 6000 {
            Some(DealEnd::Retired)
        } else if e.end <= today {
            Some(if fame >= 3000 { DealEnd::Expired } else { DealEnd::Faded })
        } else {
            None
        };
        // Clashes with the club's own partner in a protected sector.
        let club = w.club_of_person(e.person);
        let clash = club.is_some() && matches!(sector, Sector::Sportswear | Sector::Drinks | Sector::Betting) && w.commerce.club_partner(club, sector).is_some_and(|d| d.brand != e.brand);
        let why = why.or_else(|| {
            if !clash {
                return None;
            }
            let star = p.is_some() && w.players.cold[p].status == SquadStatus::Star;
            let key = hash_key(&[w.seed, u64::from(e.person.0), i as u64, today.year() as u64]);
            if key % 3 == 0 {
                w.events.push(today, Visibility::Public, EventKind::SponsorClash { person: e.person, brand: e.brand, club });
            }
            // Stars are tolerated; others are told to drop the deal.
            if star { None } else { Some(DealEnd::Conflict) }
        });
        if let Some(why) = why {
            let renew = why == DealEnd::Expired && !retired;
            w.commerce.endorsements[i].ended = Some((today, why));
            if let Some(v) = w.commerce.by_person.get_mut(&e.person) {
                v.retain(|&x| x != i as u32);
            }
            w.events.push(today, Visibility::Public, EventKind::EndorsementEnded { person: e.person, brand: e.brand, why });
            if why == DealEnd::Conflict && p.is_some() {
                let h = &mut w.players.hot[p];
                h.morale = h.morale.saturating_sub(3);
            }
            if renew {
                let fee = value_to(w, &w.commerce.brands[e.brand as usize], e.person, p);
                if fee >= 10_000 {
                    propose(w, e.person, e.brand, fee, 2, e.days);
                }
            }
            continue;
        }
        // Appearance days cost calm.
        if let Some(l) = w.lives.get_mut(e.person) {
            l.stress = l.stress.saturating_add(e.days / 2).min(100);
        }
    }
}
