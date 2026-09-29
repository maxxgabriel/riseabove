//! Structural validation of a world, and a census of its persistent identities.
//!
//! Locked design §10.10, §10.6: after loading, and above all after a migration, every reference must still resolve and every persistent
//! id must still mean the same entity. [`problems`] lists what is wrong (never panics, never repairs); [`Census`] records identities so a
//! migration test can prove none moved. Tests live in `crates/pw-cli/tests/saves.rs`, where worlds can be built.

use pw_core::rng::hash_key;
use pw_world::{PlayerStatus, World};

/// Everything structurally wrong with the world, at most 40 lines.
pub fn problems(w: &World) -> Vec<String> {
    const LIMIT: usize = 40;
    let mut out: Vec<String> = Vec::new();
    let mut bad = |msg: String| {
        if out.len() < LIMIT {
            out.push(msg);
        }
    };
    let (n_people, n_clubs, n_teams, n_comps, n_staff, n_nations) = (w.people.len(), w.clubs.len(), w.teams.len(), w.comps.len(), w.staff.len(), w.nations.len());
    let year = w.date.year();
    if !(1900..=2300).contains(&year) {
        bad(format!("the world's date is not sane: year {year}"));
    }
    if w.lives.len() != n_people {
        bad(format!("{} lives for {} people", w.lives.len(), n_people));
    }
    if w.players.hot.len() != w.players.cold.len() {
        bad(format!("{} hot player rows for {} cold", w.players.hot.len(), w.players.cold.len()));
    }

    for (id, p) in w.people.iter_enumerated() {
        if p.nation.is_some() && p.nation.0 as usize >= n_nations {
            bad(format!("person {id:?} has an unknown nation"));
        }
        if p.player.is_some() {
            match w.players.cold.get(p.player) {
                Some(c) if c.person == id => {}
                _ => bad(format!("person {id:?} names player {:?} who does not name them back", p.player)),
            }
        }
        if p.staff.is_some() && p.staff.0 as usize >= n_staff {
            bad(format!("person {id:?} names an unknown staff record"));
        }
        if p.dob.0 > w.date.0 {
            bad(format!("person {id:?} is born after today"));
        }
    }

    let mut listed = pw_world::FxHashMap::<pw_core::PlayerId, pw_core::TeamId>::default();
    for (tid, t) in w.teams.iter_enumerated() {
        if t.club.0 as usize >= n_clubs {
            bad(format!("team {tid:?} belongs to unknown club {:?}", t.club));
            continue;
        }
        if !w.clubs[t.club].teams.contains(&tid) {
            bad(format!("team {tid:?} is not listed by its club {:?}", t.club));
        }
        for &p in &t.squad {
            if p.0 as usize >= w.players.hot.len() {
                bad(format!("team {tid:?} lists unknown player {p:?}"));
            } else if let Some(other) = listed.insert(p, tid) {
                bad(format!("player {p:?} is in two squads: {other:?} and {tid:?}"));
            }
        }
        if t.captain.is_some() && t.captain.0 as usize >= w.players.hot.len() {
            bad(format!("team {tid:?} has an unknown captain"));
        }
    }

    for (id, h) in w.players.hot.iter_enumerated() {
        let c = &w.players.cold[id];
        if c.person.0 as usize >= n_people || w.people[c.person].player != id {
            bad(format!("player {id:?} and his person do not name each other"));
        }
        if h.status == PlayerStatus::Active {
            if h.club.0 as usize >= n_clubs {
                bad(format!("active player {id:?} is registered to an unknown club"));
                continue;
            }
            match listed.get(&id) {
                Some(&t) if t == h.team => {}
                Some(&t) => bad(format!("active player {id:?} is in the squad of {t:?} but plays for {:?}", h.team)),
                None => bad(format!("active player {id:?} is in no squad")),
            }
            if c.contract.club.0 as usize >= n_clubs {
                bad(format!("active player {id:?} has a contract with an unknown club"));
            }
            if c.contract.end.0 < c.contract.start.0 {
                bad(format!("active player {id:?} has a contract that ends before it starts"));
            }
        }
        if let Some(l) = &c.loan
            && (l.parent.0 as usize >= n_clubs || l.club.0 as usize >= n_clubs)
        {
            bad(format!("player {id:?} is on loan between unknown clubs"));
        }
    }

    for (id, s) in w.staff.iter_enumerated() {
        if s.person.0 as usize >= n_people {
            bad(format!("staff {id:?} names an unknown person"));
        }
        if s.club.is_some() && s.club.0 as usize >= n_clubs {
            bad(format!("staff {id:?} works for an unknown club"));
        }
    }

    let mut chairs = pw_world::FxHashMap::<pw_core::StaffId, pw_core::ClubId>::default();
    for (id, c) in w.clubs.iter_enumerated() {
        // A club's manager works for that club, is not retired, and runs no other club.
        if c.manager.is_some() && (c.manager.0 as usize) < n_staff {
            let s = &w.staff[c.manager];
            if s.club != id {
                bad(format!("club {id:?}'s manager {:?} works for {:?}", c.manager, s.club));
            }
            if s.retired {
                bad(format!("club {id:?}'s manager {:?} is retired", c.manager));
            }
            if let Some(other) = chairs.insert(c.manager, id) {
                bad(format!("manager {:?} runs both {other:?} and {id:?}", c.manager));
            }
        }
        if c.nation.is_some() && c.nation.0 as usize >= n_nations {
            bad(format!("club {id:?} is in an unknown nation"));
        }
        if c.league.is_some() && c.league.0 as usize >= n_comps {
            bad(format!("club {id:?} plays in an unknown league"));
        }
        if c.manager.is_some() && c.manager.0 as usize >= n_staff {
            bad(format!("club {id:?} has an unknown manager"));
        }
        let f = &c.finance;
        if f.balance.abs() > 1_000_000_000_000_000 || !f.wage_scale.is_finite() {
            bad(format!("club {id:?} has insane finances (balance {}, wage scale {})", f.balance, f.wage_scale));
        }
    }

    for (id, comp) in w.comps.iter_enumerated() {
        for &t in &comp.state.entrants {
            if t.0 as usize >= n_teams {
                bad(format!("competition {id:?} has an unknown entrant {t:?}"));
            }
        }
    }
    for (id, f) in w.fixtures.iter() {
        if f.comp.0 as usize >= n_comps || f.home.0 as usize >= n_teams || f.away.0 as usize >= n_teams {
            bad(format!("fixture {id:?} refers to an unknown competition or team"));
        }
    }

    for (a, b) in w.social.endpoints() {
        if a.0 as usize >= n_people || b.0 as usize >= n_people {
            bad(format!("a relationship joins unknown people {a:?} and {b:?}"));
            break;
        }
    }
    for (&p, j) in &w.media.journalists {
        if p.0 as usize >= n_people || j.person != p {
            bad(format!("journalist record {p:?} is misfiled"));
        }
    }
    for s in w.media.stories.iter() {
        if s.journalist.is_some() && s.journalist.0 as usize >= n_people {
            bad(format!("story {:?} was written by an unknown person", s.id));
        }
    }
    for a in &w.net.accounts {
        if a.person.is_some() && a.person.0 as usize >= n_people {
            bad(format!("account {} belongs to an unknown person", a.handle));
        }
    }
    let mut last = None;
    for e in w.events.all() {
        if let Some(prev) = last
            && e.id.0 <= prev
        {
            bad(format!("event ids are not strictly increasing at {:?}", e.id));
            break;
        }
        last = Some(e.id.0);
    }
    out
}

/// `Ok` when nothing is wrong, else the first problems as one message.
pub fn check(w: &World) -> Result<(), String> {
    let p = problems(w);
    if p.is_empty() { Ok(()) } else { Err(format!("{} problem(s): {}", p.len(), p.iter().take(8).cloned().collect::<Vec<_>>().join("; "))) }
}

/// The identity of every persistent entity, by id. A migration may add entities but must not renumber or replace any (§10.6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Census {
    people: Vec<u64>,
    players: Vec<u64>,
    staff: Vec<u64>,
    clubs: Vec<u64>,
    teams: Vec<u64>,
    comps: Vec<u64>,
    accounts: Vec<u64>,
    stories: Vec<u64>,
    events: Vec<u64>,
}

fn text(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4u64, |h, b| hash_key(&[h, u64::from(b)]))
}

pub fn census(w: &World) -> Census {
    Census {
        people: w.people.iter().map(|p| hash_key(&[u64::from(p.first.0), u64::from(p.last.0), p.dob.0 as u64, u64::from(p.nation.0)])).collect(),
        players: w.players.cold.iter().map(|c| u64::from(c.person.0)).collect(),
        staff: w.staff.iter().map(|s| hash_key(&[u64::from(s.person.0), s.role as u64])).collect(),
        clubs: w.clubs.iter().map(|c| text(&c.name)).collect(),
        teams: w.teams.iter().map(|t| hash_key(&[u64::from(t.club.0), t.kind as u64])).collect(),
        comps: w.comps.iter().map(|c| text(&c.name)).collect(),
        accounts: w.net.accounts.iter().map(|a| text(&a.handle)).collect(),
        stories: w.media.stories.iter().map(|s| hash_key(&[u64::from(s.id.0), s.date.0 as u64, u64::from(s.journalist.0)])).collect(),
        events: w.events.all().iter().map(|e| hash_key(&[u64::from(e.id.0), e.date.0 as u64])).collect(),
    }
}

impl Census {
    /// `Ok` when every entity present before is still present, at the same id, with the same identity. New entities are allowed.
    pub fn preserved_in(&self, after: &Census) -> Result<(), String> {
        let sets: [(&str, &Vec<u64>, &Vec<u64>); 9] = [
            ("people", &self.people, &after.people),
            ("players", &self.players, &after.players),
            ("staff", &self.staff, &after.staff),
            ("clubs", &self.clubs, &after.clubs),
            ("teams", &self.teams, &after.teams),
            ("competitions", &self.comps, &after.comps),
            ("accounts", &self.accounts, &after.accounts),
            ("stories", &self.stories, &after.stories),
            ("events", &self.events, &after.events),
        ];
        for (name, before, now) in sets {
            if now.len() < before.len() {
                return Err(format!("{name}: {} before, only {} after", before.len(), now.len()));
            }
            if let Some(i) = before.iter().zip(now.iter()).position(|(a, b)| a != b) {
                return Err(format!("{name}: the entity at id {i} changed identity"));
            }
        }
        Ok(())
    }
}
