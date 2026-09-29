//! Structural invariants: things that must be true of the world at every
//! moment, checked close to the source rather than discovered seasons later.
//!
//! Unlike `audit` (which guards what text says), these guard the world's own
//! bookkeeping: who is registered where, who is employed by whom, who plays
//! when, and that identities are unique. Every check is read-only and returns
//! data; nothing here repairs. Cost is one pass over players, staff, teams
//! and fixtures, so run it at checkpoints and in tests, not every day.
//!
//! These join the shared metrics/invariant harness when it is visible; the
//! function boundary (`check(&World) -> Vec<Breach>`) is the integration point.

use pw_world::{FxHashSet, PlayerStatus, StaffRole, World};

/// One broken invariant, with enough detail to find the cause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breach {
    pub code: &'static str,
    pub detail: String,
}

fn breach(v: &mut Vec<Breach>, code: &'static str, detail: String) {
    // A corrupt world can breach thousands of times; the first few say enough.
    if v.iter().filter(|b| b.code == code).count() < 8 {
        v.push(Breach { code, detail });
    }
}

pub fn check(w: &World) -> Vec<Breach> {
    let mut v = Vec::new();
    identities(w, &mut v);
    registrations(w, &mut v);
    injuries(w, &mut v);
    staff(w, &mut v);
    fixtures(w, &mut v);
    provenance(w, &mut v);
    v
}

/// One person, one player record, one staff record, both ways round.
fn identities(w: &World, v: &mut Vec<Breach>) {
    for (p, c) in w.players.cold.iter_enumerated() {
        if w.people[c.person].player != p {
            breach(v, "identity.player", format!("player {} → person {} → player {}", p.0, c.person.0, w.people[c.person].player.0));
        }
    }
    for (s, st) in w.staff.iter_enumerated() {
        if w.people[st.person].staff != s {
            breach(v, "identity.staff", format!("staff {} → person {} → staff {}", s.0, st.person.0, w.people[st.person].staff.0));
        }
    }
    for (id, person) in w.people.iter_enumerated() {
        if let Some(p) = person.player.get()
            && w.players.cold[p].person != id
        {
            breach(v, "identity.person_player", format!("person {} → player {} → person {}", id.0, p.0, w.players.cold[p].person.0));
        }
        if let Some(s) = person.staff.get()
            && w.staff[s].person != id
        {
            breach(v, "identity.person_staff", format!("person {} → staff {} → person {}", id.0, s.0, w.staff[s].person.0));
        }
    }
}

/// Squads and players agree, and a team belongs to the club (or loan club) that registers the player.
fn registrations(w: &World, v: &mut Vec<Breach>) {
    let mut listed = vec![0u8; w.players.len()];
    for (t, team) in w.teams.iter_enumerated() {
        for &p in &team.squad {
            if w.players.hot[p].team != t {
                breach(v, "squad.mismatch", format!("player {} listed in team {} but plays for team {}", p.0, t.0, w.players.hot[p].team.0));
            }
            listed[p.0 as usize] = listed[p.0 as usize].saturating_add(1);
        }
    }
    for (p, h) in w.players.hot.iter_enumerated() {
        let n = listed[p.0 as usize];
        let expected = u8::from(h.team.is_some());
        if n != expected {
            breach(v, "squad.membership", format!("player {} ({:?}) is in {} squads, team field says {}", p.0, h.status, n, expected));
        }
        if h.team.is_some() && h.status == PlayerStatus::Active {
            let team_club = w.teams[h.team].club;
            let loan_club = w.players.cold[p].loan.as_ref().map(|l| l.club);
            if team_club != h.club && loan_club != Some(team_club) {
                breach(v, "registration.club", format!("player {} registered to club {} plays in a team of club {}", p.0, h.club.0, team_club.0));
            }
        }
    }
}

/// Rehab bookkeeping: days left never exceed the case total, and a case has days.
fn injuries(w: &World, v: &mut Vec<Breach>) {
    for (p, h) in w.players.hot.iter_enumerated() {
        if h.injury != 0 && (h.injury_days == 0 || h.injury_days > h.injury_total) {
            breach(v, "injury.days", format!("player {} injury {} has {} of {} days", p.0, h.injury, h.injury_days, h.injury_total));
        }
        if h.injury == 0 && h.injury_days != 0 {
            breach(v, "injury.phantom", format!("player {} has {} injury days with no injury", p.0, h.injury_days));
        }
    }
}

/// Employment is consistent both ways: no one on two clubs' books, no dangling manager.
fn staff(w: &World, v: &mut Vec<Breach>) {
    for (c, club) in w.clubs.iter_enumerated() {
        let mut seen: FxHashSet<u32> = FxHashSet::default();
        for &s in &club.staff {
            if !seen.insert(s.0) {
                breach(v, "staff.duplicate", format!("staff {} listed twice at club {}", s.0, c.0));
            }
            if w.staff[s].club != c {
                breach(v, "staff.employer", format!("staff {} listed at club {} but employed by club {}", s.0, c.0, w.staff[s].club.0));
            }
        }
        if let Some(m) = club.manager.get() {
            if w.staff[m].club != c || w.staff[m].role != StaffRole::Manager {
                breach(v, "staff.manager", format!("club {} manager {} is club {} role {:?}", c.0, m.0, w.staff[m].club.0, w.staff[m].role));
            }
            if !club.staff.contains(&m) {
                breach(v, "staff.manager_unlisted", format!("club {} manager {} is not on its staff list", c.0, m.0));
            }
        }
    }
    for (s, st) in w.staff.iter_enumerated() {
        if st.club.is_some() && !w.clubs[st.club].staff.contains(&s) {
            breach(v, "staff.unlisted", format!("staff {} says club {} but is not on its list", s.0, st.club.0));
        }
        if st.club.is_some() && w.intl.managers.contains(&s) {
            breach(v, "staff.double_employment", format!("staff {} is a national manager and employed by club {}", s.0, st.club.0));
        }
    }
}

/// A team plays at most once a day, and never itself.
fn fixtures(w: &World, v: &mut Vec<Breach>) {
    let mut busy: FxHashSet<(i32, u32)> = FxHashSet::default();
    for (_, f) in w.fixtures.iter() {
        if f.home == f.away {
            breach(v, "fixture.self", format!("fixture {} pits team {} against itself", f.uid, f.home.0));
        }
        for t in [f.home, f.away] {
            if !busy.insert((f.date.0, t.0)) {
                breach(v, "fixture.double_booked", format!("team {} has two fixtures on day {} (fixture {})", t.0, f.date.0, f.uid));
            }
        }
    }
}

/// Every player has an origin, and none is dated in the future.
fn provenance(w: &World, v: &mut Vec<Breach>) {
    if w.players.origin.len() != w.players.len() {
        breach(v, "origin.length", format!("{} origins for {} players", w.players.origin.len(), w.players.len()));
        return;
    }
    for (p, o) in w.players.origin.iter_enumerated() {
        if o.date > w.date {
            breach(v, "origin.future", format!("player {} created on day {} but today is {}", p.0, o.date.0, w.date.0));
        }
    }
}
