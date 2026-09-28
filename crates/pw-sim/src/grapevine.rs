//! The grapevine: how information travels between people (S15, 11 §2).
//!
//! 1. Absorb: private and club-internal events (fines, unrest, board
//!    warnings, bids, collapsed talks, private life events, injuries worse
//!    than announced) become information items known to the people who were
//!    there. Incidents create items directly with their witnesses.
//! 2. Spread: every day each holder may tell the people around them —
//!    partner, agent, clients, teammates, colleagues, superiors, the board,
//!    journalists who count them as a source. Whether they do depends on
//!    discretion (professionalism, loyalty to the subject), appetite (how
//!    juicy it is, how fresh), closeness, and motive (duty, confiding,
//!    gossip, ego, revenge, an agent's or player's strategy, carelessness,
//!    friendship with a journalist). Each retelling may degrade the version
//!    that is passed on; some tellers plant a misleading version on purpose.
//! 3. React: the people who learn things act — agents start sounding out
//!    clubs, boards ask managers to explain, players hear of interest,
//!    managers learn of trouble in their squad. Journalists who learn things
//!    feed the newsroom.
//! 4. Leaks: when a story built on an item is published, the people it
//!    concerns realise it got out and suspect someone — from what *they* can
//!    see, which may be wrong.

use pw_core::rng::{period, stream};
use pw_core::{AgentId, ClubId, EventId, PersonId, PlayerId, StoryId};
use pw_world::event::{Cause, EventKind, Fact, Visibility};
use pw_world::info::{Fidelity, InfoKind, Knower, Learned, Motive, Tell};
use pw_world::{FxHashMap, LifeEventKind, MemoryKind, PlayerStatus, StaffRole, World};
use smallvec::SmallVec;

use crate::consider;

/// Items stop travelling after this many days.
const SHELF_LIFE: i32 = 21;

pub fn daily(w: &mut World) {
    absorb_events(w);
    spread(w);
    close_old(w);
}

// ---------------------------------------------------------------------------
// Creating items
// ---------------------------------------------------------------------------

/// Create an item known to `people` (who were involved or saw it).
pub fn witness(w: &mut World, kind: InfoKind, event: EventId, sensitivity: u8, involved: &[PersonId], witnesses: &[PersonId]) -> u32 {
    let today = w.date;
    let id = w.grapevine.create(kind, event, today, sensitivity);
    for &p in involved {
        if p.is_some() {
            w.grapevine.learn(id, Knower { person: p, how: Learned::Involved, date: today, fidelity: Fidelity::Accurate, confidence: 100, told: 0 });
        }
    }
    for &p in witnesses {
        if p.is_some() {
            w.grapevine.learn(id, Knower { person: p, how: Learned::Witnessed, date: today, fidelity: Fidelity::Accurate, confidence: 90, told: 0 });
        }
    }
    id
}

fn manager_person(w: &World, club: ClubId) -> PersonId {
    if club.is_none() {
        return PersonId::NONE;
    }
    w.clubs[club].manager.get().map_or(PersonId::NONE, |m| w.staff[m].person)
}

fn chairman(w: &World, club: ClubId) -> PersonId {
    w.governance.get(&club).map_or(PersonId::NONE, |g| g.chairman)
}

fn staff_in(w: &World, club: ClubId, role: StaffRole) -> Vec<PersonId> {
    if club.is_none() {
        return Vec::new();
    }
    w.clubs[club].staff.iter().filter(|&&s| w.staff[s].role == role).map(|&s| w.staff[s].person).collect()
}

fn agent_person(w: &World, p: PlayerId) -> PersonId {
    w.agents.agent_of(p).map_or(PersonId::NONE, |a| w.agents.list[a].person)
}

/// Private and club-internal events become information held by those present.
fn absorb_events(w: &mut World) {
    let events: Vec<(EventId, EventKind, Visibility)> = w.events.after(w.grapevine.absorbed).iter().map(|e| (e.id, e.kind.clone(), e.vis)).collect();
    if let Some(last) = events.last() {
        w.grapevine.absorbed = last.0;
    }
    for (id, kind, vis) in events {
        if matches!(vis, Visibility::Public) {
            continue;
        }
        match kind {
            EventKind::Fined { player, club, .. } => {
                let who = w.players.cold[player].person;
                let m = manager_person(w, club);
                let captain = w.clubs[club].first_team();
                let cap = w.teams[captain].captain;
                let cap = if cap.is_some() && cap != player { w.players.cold[cap].person } else { PersonId::NONE };
                witness(w, InfoKind::Discipline { player, club }, id, 45, &[who, m], &[cap]);
            }
            EventKind::Unrest { club, player } => {
                let who = w.players.cold[player].person;
                let team = w.players.hot[player].team;
                let friends: Vec<PersonId> = if team.is_some() {
                    w.teams[team].squad.iter().map(|&x| w.players.cold[x].person).filter(|&x| x != who && consider::affinity(w, x, who) > 0.2).take(6).collect()
                } else {
                    Vec::new()
                };
                let mut involved = vec![who];
                involved.extend(friends);
                witness(w, InfoKind::DressingRoom { club, leader: player }, id, 70, &involved, &[]);
            }
            EventKind::BoardWarning { club, .. } => {
                let m = manager_person(w, club);
                witness(w, InfoKind::JobInDanger { club, manager: m }, id, 60, &[chairman(w, club), m], &[]);
            }
            EventKind::Interest { player, club } => {
                // The interested club's people know; so does the player's agent if the club asked around.
                let seller = w.players.hot[player].club;
                let mut involved = vec![manager_person(w, club)];
                involved.extend(staff_in(w, club, StaffRole::DirectorOfFootball));
                let mut witnesses = vec![manager_person(w, seller)];
                witnesses.push(agent_person(w, player));
                witness(w, InfoKind::Interest { club, player }, id, 35, &involved, &witnesses);
            }
            EventKind::BidRejected { player, club, fee } | EventKind::BidAccepted { player, club, fee } => {
                let seller = w.players.hot[player].club;
                let mut involved = vec![manager_person(w, club), manager_person(w, seller), chairman(w, seller)];
                involved.extend(staff_in(w, club, StaffRole::DirectorOfFootball));
                witness(w, InfoKind::Bid { club, player, fee }, id, 40, &involved, &[agent_person(w, player)]);
            }
            EventKind::TalksCollapsed { player, club, .. } => {
                let who = w.players.cold[player].person;
                witness(w, InfoKind::ContractTalks { player, club, stalling: true }, id, 40, &[who, agent_person(w, player), manager_person(w, club)], &[]);
            }
            EventKind::Life { person, kind: what } if matches!(what, LifeEventKind::Separated { .. } | LifeEventKind::ParentUnwell | LifeEventKind::FinancialTrouble | LifeEventKind::Bereavement) => {
                let partner = w.lives.get(person).and_then(|l| l.household.partner).map_or(PersonId::NONE, |p| p.person);
                let other = if let LifeEventKind::Separated { partner } = what { partner } else { PersonId::NONE };
                witness(w, InfoKind::Private { person, what }, id, 60, &[person, partner, other], &[]);
            }
            _ => {}
        }
    }
    injuries_worse_than_said(w);
}

/// Weekly: unhappiness that people carry becomes something they can confide.
pub fn feelings(w: &mut World) {
    let today = w.date;
    let candidates: Vec<(PlayerId, PersonId, PersonId)> = w
        .players
        .ids()
        .filter(|&p| w.players.hot[p].status == PlayerStatus::Active && w.players.hot[p].morale < 35)
        .filter_map(|p| {
            let who = w.players.cold[p].person;
            let m = w.manager_of_player(p)?;
            (consider::grievance(w, who, m) > 0.3 || consider::minutes_grievance(w, p) > 0.4).then_some((p, who, m))
        })
        .collect();
    for (p, who, m) in candidates {
        let recent = w.grapevine.known_by(who).any(|i| matches!(i.kind, InfoKind::Unhappy { player, .. } if player == p) && i.date.days_until(today) < 60);
        if !recent {
            witness(w, InfoKind::Unhappy { player: p, with: m }, EventId::NONE, 40, &[who], &[]);
        }
    }
}

/// A medical room that knows an injury is worse than the public estimate.
fn injuries_worse_than_said(w: &mut World) {
    let today = w.date;
    let cases: Vec<(PlayerId, u16, u16, ClubId, EventId)> = w
        .medical
        .open
        .values()
        .filter(|c| c.date == today && c.club.is_some())
        .map(|c| (c.player, c.estimate, w.players.hot[c.player].injury_days, c.club, EventId::NONE))
        .collect();
    for (p, estimate, truth, club, ev) in cases {
        if f32::from(truth) > f32::from(estimate) * 1.4 && truth >= 21 {
            let who = w.players.cold[p].person;
            let physios = staff_in(w, club, StaffRole::Physio);
            let mut involved = vec![who, manager_person(w, club)];
            involved.extend(physios);
            witness(w, InfoKind::InjuryWorse { player: p, days: truth }, ev, 50, &involved, &[]);
        }
    }
}

// ---------------------------------------------------------------------------
// Spreading
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Partner,
    OwnAgent,
    Client,
    Teammate,
    Superior,
    Board,
    Colleague,
    Journalist,
}

/// People a holder might tell.
fn contacts(w: &World, who: PersonId, sources_of: &FxHashMap<PersonId, SmallVec<[PersonId; 2]>>, agent_by_person: &FxHashMap<PersonId, AgentId>) -> SmallVec<[(PersonId, Role); 16]> {
    let mut v: SmallVec<[(PersonId, Role); 16]> = SmallVec::new();
    if let Some(pt) = w.lives.get(who).and_then(|l| l.household.partner) {
        v.push((pt.person, Role::Partner));
    }
    let p = w.people[who].player;
    if p.is_some() && w.players.hot[p].status == PlayerStatus::Active {
        let ap = agent_person(w, p);
        if ap.is_some() {
            v.push((ap, Role::OwnAgent));
        }
        if let Some(m) = w.manager_of_player(p) {
            v.push((m, Role::Superior));
        }
        let team = w.players.hot[p].team;
        if team.is_some() {
            for &x in &w.teams[team].squad {
                let xp = w.players.cold[x].person;
                if xp != who && consider::affinity(w, who, xp) > 0.25 && v.len() < 12 {
                    v.push((xp, Role::Teammate));
                }
            }
        }
    }
    let s = w.people[who].staff;
    if s.is_some() && w.staff[s].employed() {
        let club = w.staff[s].club;
        let m = manager_person(w, club);
        if m == who {
            let ch = chairman(w, club);
            if ch.is_some() {
                v.push((ch, Role::Board));
            }
        } else if m.is_some() {
            v.push((m, Role::Superior));
        }
        for &o in &w.clubs[club].staff {
            let op = w.staff[o].person;
            if op != who && op != m && consider::affinity(w, who, op) > 0.3 && v.len() < 14 {
                v.push((op, Role::Colleague));
            }
        }
    }
    if let Some(&a) = agent_by_person.get(&who) {
        for &c in w.agents.list[a].clients.iter().take(6) {
            v.push((w.players.cold[c].person, Role::Client));
        }
    }
    if let Some(js) = sources_of.get(&who) {
        for &j in js {
            v.push((j, Role::Journalist));
        }
    }
    v
}

/// Who the item is about, for loyalty and grudges.
fn subject_person(w: &World, kind: &InfoKind) -> PersonId {
    match *kind {
        InfoKind::JobInDanger { manager, .. } => manager,
        InfoKind::Unhappy { player, .. } | InfoKind::Discipline { player, .. } | InfoKind::InjuryWorse { player, .. } | InfoKind::ContractTalks { player, .. } | InfoKind::Exploring { player, .. } => w.players.cold[player].person,
        InfoKind::Interest { player, .. } | InfoKind::Bid { player, .. } => w.players.cold[player].person,
        InfoKind::DressingRoom { club, .. } => manager_person(w, club),
        InfoKind::Private { person, .. } => person,
        InfoKind::Incident { incident } => w.incidents.get(incident).and_then(|i| i.parties.first().copied()).unwrap_or(PersonId::NONE),
    }
}

/// How likely `teller` is to tell `to` today, and why.
fn inclination(w: &World, teller: PersonId, to: PersonId, role: Role, kind: &InfoKind, sensitivity: u8, freshness: f32, agent_by_person: &FxHashMap<PersonId, AgentId>) -> (f32, Motive) {
    let prof = consider::hid(w, teller, pw_core::Hidden::Professionalism) / 20.0;
    let loose = consider::hid(w, teller, pw_core::Hidden::Controversy) / 20.0;
    let ambition = consider::hid(w, teller, pw_core::Hidden::Ambition) / 20.0;
    let subject = subject_person(w, kind);
    let loyalty = if subject.is_some() && subject != teller { consider::affinity(w, teller, subject).max(0.0) } else { 0.0 };
    // Told to keep it quiet by someone in authority.
    let hushed = matches!(kind, InfoKind::Incident { incident } if w.incidents.get(*incident).is_some_and(|i| i.hushed));
    let discretion = (0.5 * prof + 0.3 * (1.0 - loose) + 0.3 * loyalty + if hushed { 0.25 } else { 0.0 }).clamp(0.0, 1.0);
    let juicy = 0.5 + f32::from(sensitivity) / 100.0;
    let closeness = (0.3 + consider::affinity(w, teller, to).max(0.0) + consider::trust(w, to, teller) * 0.3).min(1.2);
    let (base, motive) = match role {
        Role::Partner => (0.22, Motive::Confiding),
        Role::OwnAgent => match kind {
            InfoKind::Interest { .. } | InfoKind::Bid { .. } | InfoKind::Unhappy { .. } | InfoKind::ContractTalks { .. } | InfoKind::Discipline { .. } => (0.35, Motive::PlayerStrategy),
            _ => (0.08, Motive::Confiding),
        },
        Role::Client => match kind {
            InfoKind::Interest { player, .. } | InfoKind::Bid { player, .. } if w.players.cold[*player].person == to => (0.45, Motive::AgentStrategy),
            _ => (0.03, Motive::Gossip),
        },
        Role::Teammate => (0.05, Motive::Gossip),
        Role::Colleague => (0.04, Motive::Gossip),
        Role::Superior => match kind {
            InfoKind::Incident { .. } | InfoKind::DressingRoom { .. } | InfoKind::Discipline { .. } | InfoKind::InjuryWorse { .. } => (0.3, Motive::Duty),
            _ => (0.03, Motive::Duty),
        },
        Role::Board => match kind {
            InfoKind::DressingRoom { .. } | InfoKind::Incident { .. } | InfoKind::Bid { .. } => (0.15, Motive::Duty),
            _ => (0.02, Motive::Duty),
        },
        Role::Journalist => {
            // Why would someone talk to the press?
            let grudge = if subject.is_some() { consider::grievance(w, teller, subject) } else { 0.0 };
            let is_agent = agent_by_person.contains_key(&teller);
            let own_story = kind.player().is_some() && w.players.cold[kind.player()].person == teller;
            let friendly = consider::affinity(w, teller, to);
            let mut best = (0.012, Motive::Careless);
            let mut consider_motive = |p: f32, m: Motive| {
                if p > best.0 {
                    best = (p, m);
                }
            };
            if grudge > 0.3 {
                consider_motive(0.02 + grudge * 0.05, Motive::Revenge);
            }
            if is_agent && matches!(kind, InfoKind::Interest { .. } | InfoKind::Bid { .. } | InfoKind::Unhappy { .. } | InfoKind::Exploring { .. }) {
                let greed = agent_by_person.get(&teller).map_or(10, |&a| w.agents.list[a].greed);
                consider_motive(0.03 + f32::from(greed) / 20.0 * 0.06, Motive::AgentStrategy);
            }
            if own_story && matches!(kind, InfoKind::Unhappy { .. } | InfoKind::Interest { .. }) {
                consider_motive(0.03 + ambition * 0.03, Motive::PlayerStrategy);
            }
            if friendly > 0.4 {
                consider_motive(0.02 + friendly * 0.03, Motive::PressFriendship);
            }
            if ambition > 0.7 && loose > 0.6 {
                consider_motive(0.03, Motive::Ego);
            }
            if prof < 0.4 {
                consider_motive(0.025, Motive::Careless);
            }
            (best.0, best.1)
        }
    };
    let p = base * juicy * freshness * (1.25 - discretion) * closeness;
    (p.clamp(0.0, 0.9), motive)
}

fn spread(w: &mut World) {
    let today = w.date;
    // Who counts whom as a source; which people are agents.
    let mut sources_of: FxHashMap<PersonId, SmallVec<[PersonId; 2]>> = FxHashMap::default();
    for j in w.media.journalists.values() {
        for &s in &j.sources {
            sources_of.entry(s).or_default().push(j.person);
        }
    }
    let agent_by_person: FxHashMap<PersonId, AgentId> = w.agents.list.iter_enumerated().filter(|(_, a)| a.active).map(|(id, a)| (a.person, id)).collect();
    let active = w.grapevine.active.clone();
    for info in active {
        let (kind, sensitivity, date, closed) = {
            let it = w.grapevine.get(info);
            (it.kind, it.sensitivity, it.date, it.closed)
        };
        if closed {
            continue;
        }
        let age = date.days_until(today);
        let freshness = (1.0 - age as f32 / SHELF_LIFE as f32).max(0.0);
        if freshness <= 0.0 {
            continue;
        }
        let holders: Vec<pw_world::info::Knower> = w.grapevine.get(info).holders.to_vec();
        for k in holders {
            if k.told >= 6 || k.person.is_none() {
                continue;
            }
            let teller = k.person;
            for (to, role) in contacts(w, teller, &sources_of, &agent_by_person) {
                if to.is_none() || w.grapevine.get(info).knows(to) {
                    continue;
                }
                let (p, motive) = inclination(w, teller, to, role, &kind, sensitivity, freshness, &agent_by_person);
                let roll = w.roll(stream::GRAPEVINE, &[u64::from(info), u64::from(teller.0), u64::from(to.0), period::day(today)]);
                if roll >= p {
                    continue;
                }
                // The version passed on.
                let honest = if let Some(&a) = agent_by_person.get(&teller) {
                    f32::from(w.agents.list[a].honesty) / 20.0
                } else {
                    consider::hid(w, teller, pw_core::Hidden::Sportsmanship) / 20.0
                };
                let roll2 = w.roll(stream::GRAPEVINE, &[u64::from(info), u64::from(teller.0), u64::from(to.0), 0xf1d]);
                let mut fidelity = k.fidelity.degrade(roll2, honest);
                let still_true = w.grapevine.get(info).true_now;
                if !still_true && fidelity != Fidelity::Planted {
                    fidelity = Fidelity::Outdated;
                }
                if role == Role::Journalist && motive == Motive::AgentStrategy && honest < 0.45 {
                    fidelity = Fidelity::Planted;
                }
                tell(w, info, teller, to, fidelity, motive, k.confidence);
            }
        }
    }
}

/// One telling: the recipient learns, the log records it, and they react.
pub fn tell(w: &mut World, info: u32, from: PersonId, to: PersonId, fidelity: Fidelity, motive: Motive, teller_confidence: u8) {
    let today = w.date;
    let trust = consider::trust(w, to, from);
    let confidence = (f32::from(teller_confidence) * (0.5 + 0.5 * trust)).clamp(10.0, 95.0) as u8;
    let fresh = w.grapevine.learn(info, Knower { person: to, how: Learned::Told { by: from }, date: today, fidelity, confidence, told: 0 });
    if !fresh {
        return;
    }
    if let Some(k) = w.grapevine.items[info as usize].holders.iter_mut().find(|k| k.person == from) {
        k.told = k.told.saturating_add(1);
    }
    w.grapevine.record_tell(Tell { info, from, to, date: today, fidelity, motive });
    react(w, to, info);
}

/// People act on what they learn.
fn react(w: &mut World, who: PersonId, info: u32) {
    let today = w.date;
    let item = w.grapevine.get(info).clone();
    let Some(k) = item.knower(who).copied() else { return };
    let ev = item.event;
    match item.kind {
        InfoKind::Interest { club, player } | InfoKind::Bid { club, player, .. } if w.players.cold[player].person == who => {
            // The player hears a club wants them.
            let origin = ev;
            w.beliefs.learn(
                who,
                pw_world::beliefs::Belief {
                    about: who,
                    kind: pw_world::beliefs::BeliefKind::ClubInterested { club },
                    channel: match k.how {
                        Learned::Told { by } => pw_world::beliefs::Channel::Told(by),
                        _ => pw_world::beliefs::Channel::Inferred,
                    },
                    confidence: k.confidence,
                    date: today,
                    origin,
                },
            );
        }
        InfoKind::DressingRoom { club, .. } | InfoKind::Discipline { club, .. } if who == chairman(w, club) => {
            // The board wants an explanation.
            let m = manager_person(w, club);
            if m.is_some() {
                let causes = pw_world::causes![Cause::Fact(Fact::Heard { info, from: who })];
                w.events.push_caused(today, Visibility::Between(who, m), EventKind::BoardQuery { club, manager: m, info }, causes);
                let b = &mut w.clubs[club].board;
                b.satisfaction = b.satisfaction.saturating_sub(2);
                if let Some(l) = w.lives.get_mut(m) {
                    l.stress = l.stress.saturating_add(4).min(100);
                }
            }
        }
        InfoKind::Unhappy { player, .. } | InfoKind::Discipline { player, .. } | InfoKind::DressingRoom { leader: player, .. } | InfoKind::ContractTalks { player, .. } => {
            // An agent who hears a client is unsettled starts sounding out clubs.
            if let Some(a) = w.agents.agent_of(player) {
                if w.agents.list[a].person == who && !w.grapevine.known_by(who).any(|i| matches!(i.kind, InfoKind::Exploring { player: q, .. } if q == player) && i.date.days_until(today) < 60) {
                    crate::agents::explore(w, a, player, ev);
                }
            }
            // A manager who hears a player is unhappy trusts them a little less.
            if let InfoKind::Unhappy { player, with } = item.kind {
                if with == who {
                    let pp = w.players.cold[player].person;
                    let compat = consider::compat(w, who, pp);
                    w.social.adjust(who, pp, today, compat, -1, -3, 0);
                }
            }
        }
        InfoKind::Incident { incident } => crate::responses::on_learn(w, who, incident),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Publication and leak suspicion
// ---------------------------------------------------------------------------

/// A story built on `info` has been published: it is public now, and the
/// people it concerns realise it got out.
pub fn on_published(w: &mut World, info: u32, story: StoryId) {
    let today = w.date;
    let item = w.grapevine.get(info).clone();
    {
        let it = &mut w.grapevine.items[info as usize];
        if it.published.is_none() {
            it.published = Some(story);
        }
        it.closed = true;
    }
    let club = item.kind.club();
    if club.is_none() || item.sensitivity < 30 || item.leak_noticed {
        return;
    }
    w.grapevine.items[info as usize].leak_noticed = true;
    // Who at the club knew? The manager suspects from what he can see.
    let noticer = manager_person(w, club);
    if noticer.is_none() {
        return;
    }
    let suspects: Vec<PersonId> = item
        .holders
        .iter()
        .map(|k| k.person)
        .filter(|&p| p != noticer && w.club_of_person(p) == club)
        .collect();
    let best = suspects
        .iter()
        .map(|&s| {
            let grudge = consider::grievance(w, s, noticer);
            let distrust = 1.0 - consider::trust(w, noticer, s);
            let loose = consider::hid(w, s, pw_core::Hidden::Controversy) / 20.0;
            let past = consider::memory(w, noticer, s, MemoryKind::Leaked);
            let n = w.roll(stream::GRAPEVINE, &[u64::from(info), u64::from(s.0), 0x1ea4]) * 0.3;
            (s, grudge * 1.5 + distrust + loose * 0.5 + past + n)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
    if let Some((suspect, score)) = best {
        if score > 0.8 {
            let causes = pw_world::causes![Cause::Fact(Fact::Heard { info, from: noticer })];
            let ev = w.events.push_caused(today, Visibility::Club(club), EventKind::LeakSuspected { by: noticer, suspect, info }, causes);
            let compat = consider::compat(w, noticer, suspect);
            w.social.remember(noticer, suspect, MemoryKind::Leaked, today, ev, false, (0.5 + f32::from(item.sensitivity) / 100.0).min(1.5), compat);
        }
    }
    // The board wants to know how it got out.
    let ch = chairman(w, club);
    if ch.is_some() && item.sensitivity >= 50 {
        let causes = pw_world::causes![Cause::Fact(Fact::Heard { info, from: ch })];
        w.events.push_caused(today, Visibility::Between(ch, noticer), EventKind::BoardQuery { club, manager: noticer, info }, causes);
        let b = &mut w.clubs[club].board;
        b.satisfaction = b.satisfaction.saturating_sub(2);
    }
}

/// Mark an item as no longer true (a bid withdrawn, a player settled).
pub fn outdate(w: &mut World, info: u32) {
    w.grapevine.items[info as usize].true_now = false;
}

fn close_old(w: &mut World) {
    let today = w.date;
    let items = &mut w.grapevine.items;
    w.grapevine.active.retain(|&i| {
        let it = &mut items[i as usize];
        if it.date.days_until(today) > SHELF_LIFE || it.published.is_some() {
            it.closed = true;
        }
        !it.closed
    });
}

/// Monthly: old, closed items keep only their first holders (for audits).
pub fn compact(w: &mut World) {
    let before = w.date.add_days(-180);
    for it in w.grapevine.items.iter_mut() {
        if it.closed && it.date < before && it.holders.len() > 4 {
            it.holders.truncate(4);
        }
    }
    w.grapevine.by_person.retain(|_, v| !v.is_empty());
}

/// Does `person` know something about `player` that has not been published?
pub fn private_knowledge(w: &World, person: PersonId, player: PlayerId) -> impl Iterator<Item = &pw_world::info::InfoItem> + '_ {
    w.grapevine.known_by(person).filter(move |i| i.published.is_none() && i.kind.player() == player)
}
