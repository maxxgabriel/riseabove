//! The working lives of backroom staff (Slice 5): contracts that end, clubs
//! that lose people to bigger clubs, assistants who become managers under a
//! mentor's influence, growth that plateaus, decline, retirement.
//!
//! Before this, only managers had careers: everyone else was permanent once
//! hired and improved every year until a cap, so backroom quality could only
//! ratchet up and a club's coaching character could never change.

use pw_core::rng::stream;
use pw_core::{ClubId, Hidden, StaffId};
use pw_world::event::{EventKind, Visibility};
use pw_world::stafflife::{StaffCareer, StaffEnd, StaffJob};
use pw_world::{StaffRole, World};

use crate::consider;

/// Record the start of a job (a legacy record begins here and invents nothing before it).
pub fn open_job(w: &mut World, s: StaffId, club: ClubId) {
    let today = w.date;
    let role = w.staff[s].role;
    w.ext.staff.careers.entry(s).or_default().jobs.push(StaffJob { club, role, from: today, to: None, ended: None });
}

fn close_job(w: &mut World, s: StaffId, end: StaffEnd) {
    let today = w.date;
    let c = w.ext.staff.careers.entry(s).or_default();
    if let Some(j) = c.jobs.iter_mut().rev().find(|j| j.to.is_none()) {
        j.to = Some(today);
        j.ended = Some(end);
    }
}

fn leave(w: &mut World, s: StaffId, end: StaffEnd) {
    let club = w.staff[s].club;
    if club.is_some() {
        w.clubs[club].staff.retain(|&x| x != s);
        w.events.push(w.date, Visibility::Public, EventKind::StaffLeft { staff: s, club });
    }
    w.staff[s].club = ClubId::NONE;
    // The date of separation: how long a person has been out of work is counted from it (`leave_the_game`).
    w.staff[s].contract_end = w.date;
    close_job(w, s, end);
}

/// An assistant is made manager: he carries the ideas of the man he served under.
pub fn on_promoted(w: &mut World, a: StaffId, club: ClubId) {
    let mentor = w
        .careers
        .managers
        .iter()
        .filter_map(|(&m, p)| p.jobs.iter().rev().find(|j| j.club == club && j.to.is_some()).map(|j| (j.to, m)))
        .filter(|&(_, m)| m != a)
        .max()
        .map(|(_, m)| m);
    close_job(w, a, StaffEnd::Promoted);
    let Some(m) = mentor else { return };
    let mp = w.staff[m].philosophy;
    let ph = &mut w.staff[a].philosophy;
    let pull = |own: u8, theirs: u8| (f32::from(own) * 0.7 + f32::from(theirs) * 0.3).round() as u8;
    ph.press = pull(ph.press, mp.press);
    ph.tempo = pull(ph.tempo, mp.tempo);
    ph.directness = pull(ph.directness, mp.directness);
    ph.youth_trust = pull(ph.youth_trust, mp.youth_trust);
    let c = w.ext.staff.careers.entry(a).or_default();
    if !c.mentors.contains(&m) {
        c.mentors.push(m);
    }
}

/// Monthly: contracts end and are renewed or not by both sides; clubs with gaps
/// they cannot fill from the market tempt people away from smaller clubs.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let month = (today.year() as u64) * 12 + u64::from(today.month());
    let ids: Vec<StaffId> = w.staff.ids().collect();
    for s in ids {
        let st = &w.staff[s];
        if !st.employed() || st.role == StaffRole::Manager || w.intl.managers.contains(&s) {
            continue;
        }
        if st.contract_end <= st.joined {
            // Never had a real contract (imported): stagger the first expiries over the next two years.
            let months = 3 + (w.roll(stream::STAFF, &[u64::from(s.0), 0x7e1]) * 24.0) as i32;
            w.staff[s].contract_end = today.add_months(months);
            continue;
        }
        if st.contract_end > today {
            continue;
        }
        let club = st.club;
        let role = st.role;
        let person = st.person;
        let skill = st.role_rating(role) / 20.0;
        let others: usize = w.clubs[club].staff.iter().filter(|&&x| w.staff[x].role == role).count();
        let surplus = others > crate::staffing::wanted_count(w.clubs[club].reputation, role);
        // The club renews the useful; the surplus and the weak are let go.
        let club_p = (0.45 + 0.5 * skill - if surplus { 0.4 } else { 0.0 }).clamp(0.05, 0.95);
        // The person stays if content: loyal and unambitious people stay; those who have outgrown the club go.
        let loyal = consider::hid(w, person, Hidden::Loyalty);
        let ambition = consider::hid(w, person, Hidden::Ambition);
        let outgrown = (f32::from(w.staff[s].reputation) - f32::from(w.clubs[club].reputation)) / 8000.0;
        let staff_p = (0.8 + (loyal - ambition) / 40.0 - outgrown).clamp(0.1, 0.95);
        let keys = [u64::from(s.0), month, 0x7e2];
        if w.roll(stream::STAFF, &keys) < club_p && w.roll(stream::STAFF, &[keys[0], keys[1], 0x7e3]) < staff_p {
            w.staff[s].contract_end = today.add_months(24);
        } else {
            leave(w, s, StaffEnd::Expired);
        }
    }
    poach(w, month);
}

/// A club short of a role and with no one on the market tries to take the best from a smaller club.
fn poach(w: &mut World, month: u64) {
    let clubs: Vec<ClubId> = w.clubs.ids().collect();
    for club in clubs {
        let rep = w.clubs[club].reputation;
        for role in [StaffRole::Coach, StaffRole::Assistant, StaffRole::FitnessCoach, StaffRole::Physio, StaffRole::HeadOfYouth, StaffRole::Scout, StaffRole::DirectorOfFootball] {
            let have = w.clubs[club].staff.iter().filter(|&&s| w.staff[s].role == role).count();
            if have >= crate::staffing::wanted_count(rep, role) || rep < 2500 {
                continue;
            }
            // Only when the open market has no one (the market gets first pick in `staffing::monthly`; this is the fallback).
            if w.staff.iter().any(|s| s.role == role && !s.employed() && !s.retired && s.reputation + 2000 >= rep) {
                continue;
            }
            let target = w
                .staff
                .iter_enumerated()
                .filter(|(id, s)| s.role == role && s.employed() && s.club != club && !w.intl.managers.contains(id) && w.clubs[s.club].reputation + 800 < rep)
                .max_by(|a, b| a.1.role_rating(role).total_cmp(&b.1.role_rating(role)).then(b.0.cmp(&a.0)))
                .map(|(id, _)| id);
            let Some(s) = target else { continue };
            let person = w.staff[s].person;
            let ambition = consider::hid(w, person, Hidden::Ambition) / 20.0;
            let loyal = consider::hid(w, person, Hidden::Loyalty) / 20.0;
            let step = (f32::from(rep) - f32::from(w.clubs[w.staff[s].club].reputation)) / 4000.0;
            let roll = w.roll(stream::STAFF, &[u64::from(s.0), u64::from(club.0), month, 0x7e4]);
            if step * (0.5 + ambition) - loyal * 0.3 + roll * 0.2 > 0.2 {
                leave(w, s, StaffEnd::Poached);
                crate::staffing::hire(w, club, s);
            }
        }
    }
}

/// Yearly: growth is a chance that plateaus, decline follows age, the oldest retire.
pub fn yearly(w: &mut World) {
    let year = w.date.year() as u64;
    let ids: Vec<StaffId> = w.staff.ids().collect();
    for s in ids {
        if w.staff[s].retired {
            continue;
        }
        let person = w.staff[s].person;
        let age = consider::age(w, person);
        let prof = consider::hid(w, person, Hidden::Professionalism);
        let employed = w.staff[s].employed();
        let mentored = w.ext.staff.careers.get(&s).is_some_and(|c| !c.mentors.is_empty());
        let role = w.staff[s].role;
        for (i, &a) in role.key_attrs().iter().enumerate() {
            let cur = w.staff[s].attrs.get(a);
            let roll = w.roll(stream::STAFF, &[u64::from(s.0), year, i as u64, 0x7e5]);
            if employed && age < 52.0 && cur < 19 {
                let p = (0.15 + 0.02 * prof + if mentored { 0.15 } else { 0.0 }) * (1.0 - f32::from(cur) / 22.0);
                if roll < p {
                    w.staff[s].attrs.set(a, cur + 1);
                }
            } else if age > 55.0 && cur > 4 && roll < 0.05 * (age - 55.0) {
                w.staff[s].attrs.set(a, cur - 1);
            }
        }
        // Out of work for long: people give up on football work and go back to ordinary jobs, so the pool of the unemployed
        // reaches a level where jobs and people balance instead of growing without end.
        if role != StaffRole::Manager && !employed && !w.intl.managers.contains(&s) {
            let months = (w.date.0 - w.staff[s].contract_end.0).max(0) as f32 / 30.0;
            if months >= 10.0 {
                let p = (0.25 + 0.04 * (months - 10.0).min(24.0) + if age >= 50.0 { 0.2 } else { 0.0 }).min(0.9);
                if w.roll(stream::STAFF, &[u64::from(s.0), year, 0x7e7]) < p {
                    w.staff[s].retired = true;
                    continue;
                }
            }
        }
        // Retirement (managers retire through `managers::monthly`).
        if role != StaffRole::Manager && age >= 60.0 && w.roll(stream::STAFF, &[u64::from(s.0), year, 0x7e6]) < 0.12 * (age - 59.0).min(5.0) {
            if employed {
                leave(w, s, StaffEnd::Retired);
            }
            w.staff[s].retired = true;
        }
    }
}

/// Career record for anyone with one; legacy staff have at most their current job.
pub fn career(w: &World, s: StaffId) -> Option<&StaffCareer> {
    w.ext.staff.careers.get(&s)
}
