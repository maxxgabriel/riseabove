//! Considerations (S7): small, pure functions of world state that decisions
//! across the world weigh. Selection, meetings, transfer requests, contract
//! talks, relocation, retirement and dozens of other choices are weighted sums
//! of these, with weights coming from the decider's personality and role.
//!
//! Depth grows by adding a consideration here and wiring it into the decisions
//! it plausibly affects — not by writing events. Every function returns a
//! value in a documented range and never mutates anything.

use pw_core::math::{interp, sigmoid};
use pw_core::{Attr, ClubId, Date, Hidden, NationId, PersonId, PlayerId, StaffAttr, TeamId};
use pw_world::social::grudge_factor;
use pw_world::{MemoryKind, Person, PlayerStatus, SquadStatus, World};

// ------------------------------------------------------------ people & roles

#[inline]
pub fn person(w: &World, p: PlayerId) -> PersonId {
    w.players.cold[p].person
}

#[inline]
pub fn hid(w: &World, person: PersonId, h: Hidden) -> f32 {
    w.people[person].hidden.f(h)
}

/// Staff attribute of a person, if they hold a staff role (else a middling 10).
pub fn staff_attr(w: &World, person: PersonId, a: StaffAttr) -> f32 {
    let s = w.people[person].staff;
    if s.is_some() { w.staff[s].attrs.f(a) } else { 10.0 }
}

pub fn grudge(w: &World, person: PersonId) -> f32 {
    grudge_factor(&w.people[person])
}

pub fn age(w: &World, person: PersonId) -> f32 {
    w.people[person].dob.age_years(w.date)
}

pub fn compat(w: &World, a: PersonId, b: PersonId) -> i8 {
    pw_world::social::compatibility(&w.people[a], &w.people[b], w.date)
}

// ------------------------------------------------------------ training & form

/// What a player's coaches consider normal training for them (×10 scale):
/// driven by professionalism, determination and ability.
pub fn training_norm(w: &World, p: PlayerId) -> f32 {
    let c = &w.players.cold[p];
    let prof = hid(w, c.person, Hidden::Professionalism);
    let det = c.attrs.get(Attr::Determination);
    (58.0 + 0.6 * (prof - 10.0) + 0.5 * (det - 10.0) + 0.12 * (f32::from(c.ca) - 100.0).clamp(-40.0, 60.0)).clamp(40.0, 90.0)
}

/// Current training against the player's norm, in rating points (−3..+3).
pub fn training_delta(w: &World, p: PlayerId) -> f32 {
    ((f32::from(w.players.hot[p].training) - training_norm(w, p)) / 10.0).clamp(-3.0, 3.0)
}

/// Form (recent ratings) against 6.8, −2..+2; `None` without matches.
pub fn form_delta(w: &World, p: PlayerId) -> Option<f32> {
    w.players.hot[p].form_avg().map(|f| (f - 6.8).clamp(-2.0, 2.0))
}

// ------------------------------------------------------------ minutes & status

/// League-level minutes a team has had available over roughly four weeks.
pub fn team_minutes_4w(w: &World, team: TeamId) -> u32 {
    if team.is_none() {
        return 0;
    }
    let from = w.date.add_days(-28);
    let n = w.fixtures.between(from, w.date).filter(|&f| {
        let fx = w.fixtures.get(f);
        fx.score.is_some() && fx.involves(team)
    });
    n.count() as u32 * 90
}

/// Share of available minutes played recently (0..1), and the share the
/// player's status leads them to expect.
pub fn minutes_share(w: &World, p: PlayerId) -> (f32, f32) {
    let h = &w.players.hot[p];
    let avail = team_minutes_4w(w, h.team).max(1) as f32;
    let share = (f32::from(h.minutes_4w) / avail).clamp(0.0, 1.0);
    let expected = w.players.cold[p].status.expected_minutes();
    (share, expected)
}

/// How far short of expectations the player's minutes are (0 = fine, 1 = none at all
/// when regular football was expected).
pub fn minutes_grievance(w: &World, p: PlayerId) -> f32 {
    let (share, expected) = minutes_share(w, p);
    if team_minutes_4w(w, w.players.hot[p].team) < 180 {
        return 0.0;
    }
    ((expected - share) / expected.max(0.1)).clamp(0.0, 1.0) * expected.max(0.2).sqrt()
}

/// Matches for a team in the next `days`.
pub fn fixtures_ahead(w: &World, team: TeamId, days: i32) -> u8 {
    if team.is_none() {
        return 0;
    }
    w.fixtures.between(w.date, w.date.add_days(days)).filter(|&f| w.fixtures.get(f).involves(team)).count() as u8
}

pub fn contract_days_left(w: &World, p: PlayerId) -> i32 {
    let c = &w.players.cold[p];
    if c.contract.club.is_some() { c.contract.days_left(w.date) } else { 0 }
}

/// Wage relative to first-team peers of the same or lower status (percent).
pub fn wage_vs_peers(w: &World, p: PlayerId) -> u16 {
    let club = w.players.hot[p].club;
    let Some(team) = (club.is_some()).then(|| w.clubs[club].first_team()) else { return 100 };
    let me = &w.players.cold[p];
    let mine = me.contract.current_wage(w.date).max(1) as f32;
    let peers: Vec<f32> = w.teams[team]
        .squad
        .iter()
        .filter(|&&x| x != p && w.players.cold[x].status <= me.status.max(SquadStatus::Regular))
        .map(|&x| w.players.cold[x].contract.current_wage(w.date) as f32)
        .filter(|&v| v > 0.0)
        .collect();
    if peers.is_empty() {
        return 100;
    }
    let mut v = peers;
    v.sort_by(f32::total_cmp);
    let median = v[v.len() / 2].max(1.0);
    (mine / median * 100.0).clamp(0.0, 999.0) as u16
}

// ------------------------------------------------------------ relationships

/// Trust `from` has in `about`, 0..1 (0.5 neutral).
pub fn trust(w: &World, from: PersonId, about: PersonId) -> f32 {
    w.social.get(from, about).map_or(0.5, |r| f32::from(r.trust) / 100.0)
}

pub fn affinity(w: &World, from: PersonId, about: PersonId) -> f32 {
    w.social.get(from, about).map_or(0.0, |r| f32::from(r.affinity) / 100.0)
}

/// Weighted memory strength (0..~3) of one kind.
pub fn memory(w: &World, from: PersonId, about: PersonId, kind: MemoryKind) -> f32 {
    w.social.weight_of(from, about, kind, w.date, grudge(w, from)) / 60.0
}

/// Net grievance (negative memories outweigh positive), 0..~3.
pub fn grievance(w: &World, from: PersonId, about: PersonId) -> f32 {
    (w.social.grievance(from, about, w.date, grudge(w, from)) / 60.0).max(0.0)
}

/// Open promises from `from` to `to` that are now overdue or failing.
pub fn failing_promises(w: &World, from: PersonId, to: PersonId) -> usize {
    w.social.open_promises_between(from, to).filter(|pr| pr.due <= w.date.add_days(21)).count()
}

/// A manager's reputation for keeping their word, 0..1 (1 = never breaks).
pub fn word_kept(w: &World, person: PersonId) -> f32 {
    let broken = w.social.broken_by(person, w.date, 730) as f32;
    (1.0 - broken * 0.12).clamp(0.2, 1.0)
}

// ------------------------------------------------------------ club context

pub fn board_pressure(w: &World, club: ClubId) -> f32 {
    if club.is_none() {
        return 0.0;
    }
    let b = &w.clubs[club].board;
    ((60.0 - f32::from(b.satisfaction)) / 60.0).clamp(0.0, 1.0) + f32::from(b.warnings) * 0.2
}

/// How badly a club needs players in this player's group (0..1).
pub fn club_need_for(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let g = w.players.cold[p].best_pos.group();
    w.clubs[club].market.needs.iter().filter(|n| n.group == g).map(|n| f32::from(n.urgency) / 2.0).fold(0.0, f32::max)
}

/// Does the club currently know (enough about) the player to be interested?
pub fn club_tracking(w: &World, club: ClubId, p: PlayerId) -> u16 {
    w.knowledge.seen(club, p).map_or(0, |s| s.minutes)
}

// ------------------------------------------------------------ life

/// How hard a move to `to` would be for this person's household, 0..1+.
/// Partner career and roots, children, closeness to parents, language.
pub fn household_move_cost(w: &World, who: PersonId, to: NationId) -> f32 {
    let life = &w.lives[who];
    let here = if life.home.is_some() { life.home } else { w.people[who].nation };
    if to.is_none() || to == here {
        return 0.0;
    }
    let mut cost = 0.0;
    if let Some(pt) = life.partner() {
        let partner = &w.people[pt.person];
        let plife = &w.lives[pt.person];
        let roots = f32::from(plife.occupation.rootedness()) / 20.0;
        let lang = 1.0 - f32::from(plife.fluency(to).max(if partner.nation == to { 100 } else { 0 })) / 100.0;
        let adapt = 1.0 - partner.hidden.f(Hidden::Adaptability) / 20.0;
        let bond = f32::from(pt.bond) / 100.0;
        let status = match pt.status {
            pw_world::life::PartnerStatus::Dating => 0.4,
            pw_world::life::PartnerStatus::Living => 0.8,
            pw_world::life::PartnerStatus::Married => 1.0,
        };
        cost += status * bond * (0.35 * roots + 0.25 * lang + 0.2 * adapt + if partner.nation == to { -0.3 } else { 0.0 });
    }
    if life.household.children > 0 {
        cost += 0.12 + 0.05 * f32::from(life.household.children.min(4));
    }
    let par = &life.household.parents;
    if par.alive > 0 && par.nation == here {
        cost += f32::from(par.closeness) / 100.0 * 0.15 + if par.health < 40 { 0.2 } else { 0.0 };
    }
    let adapt = w.people[who].hidden.f(Hidden::Adaptability);
    let lang = 1.0 - f32::from(life.fluency(to)) / 100.0;
    cost += (1.0 - adapt / 20.0) * 0.25 * (0.5 + lang);
    cost.max(0.0)
}

/// Would this partner go with them? Probability-like 0..1 from the partner's own
/// circumstances (the partner's decision is made with this at the time).
pub fn partner_would_move(w: &World, partner: PersonId, to: NationId, bond: u8) -> f32 {
    let p = &w.people[partner];
    let life = &w.lives[partner];
    let roots = f32::from(life.occupation.rootedness()) / 20.0;
    let lang = f32::from(life.fluency(to)) / 100.0 + if p.nation == to { 1.0 } else { 0.0 };
    let adapt = p.hidden.f(Hidden::Adaptability) / 20.0;
    let parents_close = if life.household.parents.alive > 0 { f32::from(life.household.parents.closeness) / 100.0 } else { 0.0 };
    let x = 2.2 * (f32::from(bond) / 100.0 - 0.5) + 1.2 * adapt + 0.8 * lang.min(1.0) - 1.5 * roots - 0.8 * parents_close + 0.3;
    sigmoid(x * 2.0)
}

/// Time settled in the current country, 0..1.
pub fn settledness(w: &World, who: PersonId) -> f32 {
    let life = &w.lives[who];
    let native = w.people[who].nation == life.home || life.home.is_none();
    if native {
        return 1.0;
    }
    let years = life.years_here(w.date);
    let lang = f32::from(life.fluency(life.home)) / 100.0;
    let adapt = w.people[who].hidden.f(Hidden::Adaptability) / 20.0;
    (0.35 * (years / 2.0).min(1.0) + 0.4 * lang + 0.25 * adapt).clamp(0.0, 1.0)
}

// ------------------------------------------------------------ career stage (as inputs only)

/// How much of a career is left physically, 0..1 — an input to retirement,
/// contract length and move decisions. Never a trigger.
pub fn body_outlook(w: &World, p: PlayerId) -> f32 {
    let c = &w.players.cold[p];
    let a = w.age_years(p);
    let nf = c.attrs.get(Attr::NaturalFitness);
    let wear = f32::from(c.wear.iter().copied().max().unwrap_or(0)) / 100.0;
    let keeper = c.best_pos == pw_core::Pos::GK;
    let peak_end = if keeper { 35.0 } else { 31.0 } + (nf - 10.0) * 0.25;
    (interp(&[(peak_end - 12.0, 1.0), (peak_end, 0.75), (peak_end + 5.0, 0.15), (peak_end + 8.0, 0.0)], a) - wear * 0.35).clamp(0.0, 1.0)
}

/// Unattached for how long (days), for any player.
pub fn days_unattached(w: &World, p: PlayerId) -> i32 {
    if w.players.hot[p].status != PlayerStatus::FreeAgent {
        return 0;
    }
    w.history.spells.get(&p).and_then(|s| s.last()).and_then(|s| s.to).map_or(365, |d| d.days_until(w.date))
}

/// Something is known publicly about interest in this player (media or agent chatter).
pub fn heard_interest(w: &World, who: PersonId) -> u8 {
    w.beliefs.of(who).filter(|b| matches!(b.kind, pw_world::beliefs::BeliefKind::ClubInterested { .. }) && b.date.days_until(w.date) <= 60).count() as u8
}

/// Days since a date (saturating for sentinel dates).
pub fn days_since(w: &World, d: Date) -> i32 {
    d.days_until(w.date).clamp(0, 100_000)
}

pub fn person_ref(w: &World, id: PersonId) -> &Person {
    &w.people[id]
}
