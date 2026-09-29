//! The board deciding a manager's fate (Slice 4).
//!
//! Three warnings put the question; the seats answer it. Each seat is an
//! existing person (owner, chairman, director of football) who forms their
//! *own* backing of the manager from their own pressures. There is no
//! averaged board opinion: the ruling keeps who supported, who opposed and
//! who had the authority to decide. The board acts only when those with
//! authority turn on him *and* a better alternative exists at a cost it can
//! stomach; otherwise it backs him, and that is remembered too.
//!
//! Compensation is computed and recorded as the ruling's liability but is
//! not paid while `PAY_COMPENSATION` is off: the economy baseline has to be
//! understood before another money flow is introduced.

use pw_core::{ClubId, EventId, PersonId, StaffId};
use pw_world::club::Ownership;
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::ruling::{Outcome, Ruling, RulingKind, Stance, StanceRole};
use pw_world::{MemoryKind, StaffRole, World};
use smallvec::SmallVec;

use crate::consider;

/// Deferred by decision: sacking pays no compensation yet.
pub const PAY_COMPENSATION: bool = false;

struct Seat {
    who: PersonId,
    role: StanceRole,
    backing: f32,
    authority: bool,
}

fn seats(w: &World, club: ClubId, m: StaffId) -> Vec<Seat> {
    let Some(g) = w.governance.get(&club) else { return Vec::new() };
    let mp = w.staff[m].person;
    let sat = f32::from(w.clubs[club].board.satisfaction);
    let mut v = Vec::new();

    // Owner: results against ambition, filtered through his temper and his trust in the man.
    let temper = crate::governance::owner_temper(w, club);
    let raw = (sat - 50.0) * 2.0 - (f32::from(g.owner.ambition) - 50.0) * 0.4 + (consider::trust(w, g.owner.person, mp) - 0.5) * 40.0;
    let backing = if raw < 0.0 { raw * temper } else { raw };
    v.push(Seat { who: g.owner.person, role: StanceRole::Owner, backing: backing.clamp(-100.0, 100.0), authority: true });

    // Chairman (where distinct): the mood of the supporters he answers to.
    if g.chairman != g.owner.person {
        let fans = (f32::from(w.clubs[club].fan_mood) - 50.0) * 1.6 * (0.5 + f32::from(g.owner.fan_sensitivity) / 100.0);
        let b = fans + (consider::trust(w, g.chairman, mp) - 0.5) * 40.0;
        v.push(Seat { who: g.chairman, role: StanceRole::Chair, backing: b.clamp(-100.0, 100.0), authority: w.clubs[club].ownership == Ownership::MemberOwned });
    }

    // Director of football: does he trust the man, and does the style match what the club wants?
    if let Some(d) = w.clubs[club].staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::DirectorOfFootball) {
        let dp = w.staff[d].person;
        let gap = (i32::from(w.staff[m].philosophy.mentality) - i32::from(g.policy.style_mandate)).abs() as f32;
        let b = (consider::trust(w, dp, mp) - 0.5) * 80.0 - gap * 12.0;
        // An owner who leaves football to others gives him a say.
        v.push(Seat { who: dp, role: StanceRole::Director, backing: b.clamp(-100.0, 100.0), authority: g.owner.meddling < 40 });
    }
    v
}

/// Should the board act now? Returns the ruling's event when it does (the caller carries out the sacking).
pub fn decide(w: &mut World, club: ClubId) -> Option<EventId> {
    let m = w.clubs[club].manager.get()?;
    let today = w.date;
    let seats = seats(w, club, m);
    if seats.is_empty() {
        // No governance record: fall back to the old rule (three warnings).
        return Some(EventId::NONE);
    }
    let holders: Vec<&Seat> = seats.iter().filter(|s| s.authority).collect();
    let support = holders.iter().map(|s| s.backing).sum::<f32>() / holders.len().max(1) as f32;

    // What acting costs and what it could buy: remaining contract against the best man available.
    let weeks_left = (w.staff[m].contract_end.days_until(today).max(0) as f32 / 7.0).max(0.0);
    let liability = (w.staff[m].wage as f64 * f64::from(weeks_left)) as i64;
    let revenue = crate::finance::season_revenue(w, club).max(1);
    let frugality = w.governance.get(&club).map_or(50.0, |g| f32::from(g.owner.frugality)) / 100.0;
    let cost_weight = (liability as f32 / revenue as f32) * frugality * 8.0;
    let incumbent = w.staff[m].role_rating(StaffRole::Manager);
    let rep = i32::from(w.clubs[club].reputation);
    let alternative = w
        .staff
        .iter_enumerated()
        .filter(|(id, s)| s.role == StaffRole::Manager && !s.employed() && !s.retired && !w.intl.managers.contains(id) && i32::from(s.reputation) <= rep + 1500)
        .map(|(_, s)| s.role_rating(StaffRole::Manager))
        .fold(0.0f32, f32::max);
    let gain = if alternative > 0.0 { (alternative - incumbent) / 20.0 + 0.35 } else { 0.0 };
    let act = support < -20.0 && gain > cost_weight;

    let mp = w.staff[m].person;
    let mut stances: SmallVec<[Stance; 4]> = seats.iter().map(|s| Stance { who: s.who, role: s.role, believed_pct: 255, backing: s.backing as i8, authority: s.authority }).collect();
    stances.push(Stance { who: mp, role: StanceRole::Subject, believed_pct: 255, backing: 0, authority: false });
    let kind = if act { RulingKind::SackManager } else { RulingKind::BackManager };
    let id = w.ext.decisions.add(Ruling {
        id: 0,
        kind,
        date: today,
        club,
        subject: pw_core::PlayerId::NONE,
        about: mp,
        liability,
        decider: holders.first().map_or(PersonId::NONE, |s| s.who),
        stances,
        true_pct: 0,
        want: (-support).clamp(0.0, 100.0) as u8,
        outcome: Outcome::Enacted,
        resolved: Some(today),
        event: EventId::NONE,
    });
    let warnings = w.clubs[club].board.warnings;
    let causes: Causes = pw_world::causes![Cause::Fact(Fact::BoardPressure { club, warnings })];
    let event = w.events.push_caused(today, Visibility::Club(club), EventKind::Ruling { ruling: id }, causes);
    if let Some(r) = w.ext.decisions.get_mut(id) {
        r.event = event;
    }

    // Who backed him remembers it; who wanted him gone is remembered by him.
    for s in &seats {
        let c = consider::compat(w, mp, s.who);
        if s.backing > 20.0 {
            w.social.remember(mp, s.who, MemoryKind::Backed, today, event, false, 0.8, c);
        } else if s.backing < -20.0 && act {
            w.social.remember(mp, s.who, MemoryKind::Blamed, today, event, false, 1.0, c);
        }
    }

    if act {
        if PAY_COMPENSATION {
            w.clubs[club].finance.balance -= liability;
        }
        Some(event)
    } else {
        // Backed for now: the pressure eases, but the warnings stand so the question returns.
        let b = &mut w.clubs[club].board;
        b.satisfaction = 35;
        b.warnings = 2;
        None
    }
}
