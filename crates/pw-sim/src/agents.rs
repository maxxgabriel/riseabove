//! Agents at work (08 §7). Agents are people running a business: they take on
//! clients they think are worth it, pitch them to clubs in their network, hear
//! about interest from clubs they have ties with, pass on what they choose to,
//! and negotiate. They do this for every client, AI or human.

use pw_core::rng::{Rng, stream};
use pw_core::{AgentId, ClubId, EventId, NationId, PersonId, PlayerId};
use pw_world::agent::{Agent, Representation};
use pw_world::beliefs::{Belief, BeliefKind, Channel};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::{MindKind, NameId, Person, PlayerStatus, World};

use crate::consider;
use crate::generate as gen_;

/// Make sure every nation has a working agent market (worldgen and yearly).
pub fn ensure_market(w: &mut World) {
    let mut per_nation = vec![0usize; w.nations.len()];
    for p in w.players.ids() {
        let h = &w.players.hot[p];
        if h.status == PlayerStatus::Retired {
            continue;
        }
        let n = if h.club.is_some() { w.clubs[h.club].nation } else { w.people[w.players.cold[p].person].nation };
        if n.is_some() {
            per_nation[n.0 as usize] += 1;
        }
    }
    let mut have = vec![0usize; w.nations.len()];
    for a in w.agents.list.iter().filter(|a| a.active) {
        if a.base.is_some() {
            have[a.base.0 as usize] += 1;
        }
    }
    for n in w.nations.ids() {
        let want = (per_nation[n.0 as usize] / 45).max(if per_nation[n.0 as usize] > 0 { 2 } else { 0 });
        for k in have[n.0 as usize]..want {
            new_agent(w, n, k as u64);
        }
    }
}

fn new_agent(w: &mut World, nation: NationId, k: u64) -> AgentId {
    let mut rng = Rng::keyed(&[w.seed, stream::AGENT, u64::from(nation.0), k, w.date.year() as u64]);
    let (first, last) = crate::people::random_name(w, nation, &mut rng);
    let dob = w.date.add_days(-(365 * rng.range_i32(28, 64)));
    let person = w.people.push(Person {
        first,
        last,
        common: NameId::NONE,
        dob,
        nation,
        nation2: Default::default(),
        hidden: gen_::hidden_random(&mut rng),
        player: Default::default(),
        staff: Default::default(),
        mind: MindKind::Ai,
    });
    // Agency size follows a long tail: a few super-agents, many small operators.
    let tier = pw_core::math::powf(rng.f32(), 2.5);
    let stat = |rng: &mut Rng, base: f32| (base + rng.normal() * 3.0).round().clamp(1.0, 20.0) as u8;
    let mut reach = Vec::new();
    let extra = (tier * 6.0) as usize;
    for _ in 0..extra {
        let n = NationId(rng.below(w.nations.len().max(1) as u32));
        if n != nation && !reach.contains(&n) {
            reach.push(n);
        }
    }
    let a = Agent {
        person,
        base: nation,
        reach,
        negotiating: stat(&mut rng, 8.0 + tier * 9.0),
        network: stat(&mut rng, 6.0 + tier * 12.0),
        diligence: stat(&mut rng, 11.0),
        greed: stat(&mut rng, 10.0),
        honesty: stat(&mut rng, 12.0),
        reputation: (500.0 + tier * 8_500.0 + rng.normal() * 300.0).clamp(100.0, 10_000.0) as u16,
        clients: Vec::new(),
        capacity: (8.0 + tier * 40.0) as u8,
        active: true,
    };
    w.agents.list.push(a)
}

/// The agent this player would approach: one who covers where they play, has
/// room, and is at a level to take them seriously.
pub fn best_available(w: &World, p: PlayerId) -> Option<AgentId> {
    let h = &w.players.hot[p];
    let c = &w.players.cold[p];
    let nation = if h.club.is_some() { w.clubs[h.club].nation } else { w.people[c.person].nation };
    let rep = f32::from(c.rep.current.max(300));
    w.agents
        .list
        .iter_enumerated()
        .filter(|(_, a)| a.active && a.covers(nation) && a.clients.len() < usize::from(a.capacity) + 4)
        .filter(|(_, a)| f32::from(a.reputation) <= rep * 3.0 + 1500.0)
        .max_by(|(ia, a), (ib, b)| {
            let s = |x: &Agent| f32::from(x.reputation) / 1000.0 + f32::from(x.network) * 0.2 - x.clients.len() as f32 / f32::from(x.capacity.max(1));
            s(a).total_cmp(&s(b)).then(ib.cmp(ia))
        })
        .map(|(id, _)| id)
}

/// Engage an agent — if the agent wants the client.
pub fn hire(w: &mut World, p: PlayerId, a: AgentId) {
    let today = w.date;
    let Some(agent) = w.agents.list.get(a) else { return };
    if !agent.active || w.agents.agent_of(p) == Some(a) {
        return;
    }
    let c = &w.players.cold[p];
    let prospect = f32::from(c.rep.current) + f32::from(c.pa.saturating_sub(c.ca)) * 25.0 + f32::from(c.ca) * 20.0;
    let bar = f32::from(agent.reputation) * 0.6;
    let full = agent.clients.len() >= usize::from(agent.capacity) + 4;
    if full || prospect < bar {
        return;
    }
    let fee_pct = (3 + agent.greed / 3).min(10);
    let agent_person = agent.person;
    drop_agent(w, p);
    w.agents.list[a].clients.push(p);
    w.agents.of_player.insert(p, Representation { agent: a, since: today, until: today.add_months(24), fee_pct, last_pitch: today, satisfaction: 60 });
    let who = w.players.cold[p].person;
    w.events.push(today, Visibility::Person(who), EventKind::AgentHired { player: p, agent: a });
    let compat = consider::compat(w, who, agent_person);
    w.social.adjust(who, agent_person, today, compat, 5, 5, 0);
}

pub fn drop_agent(w: &mut World, p: PlayerId) {
    let Some(r) = w.agents.of_player.remove(&p) else { return };
    w.agents.list[r.agent].clients.retain(|&x| x != p);
    let who = w.players.cold[p].person;
    w.events.push(w.date, Visibility::Person(who), EventKind::AgentLeft { player: p, agent: r.agent });
}

/// Which clubs are seriously watching which players this week: clubs with a
/// squad need that have seen a player for at least two matches' worth.
pub type Tracking = pw_world::FxHashMap<PlayerId, smallvec::SmallVec<[(ClubId, u16); 4]>>;

pub fn tracking_index(w: &World) -> Tracking {
    let mut idx: Tracking = Default::default();
    for club in w.clubs.ids() {
        if w.clubs[club].market.needs.is_empty() {
            continue;
        }
        let mut seen: Vec<(PlayerId, u16)> = w.knowledge.known(club).filter(|(_, s)| s.minutes >= 180).map(|(p, s)| (p, s.minutes)).collect();
        seen.sort();
        for (p, m) in seen {
            if w.players.hot[p].club != club && consider::club_need_for(w, club, p) > 0.0 {
                idx.entry(p).or_default().push((club, m));
            }
        }
    }
    idx
}

/// Clubs in an agent's network that are tracking a client, with how much they have seen.
fn interest_known_to(w: &World, a: AgentId, p: PlayerId, idx: &Tracking, rng: &mut Rng) -> Vec<(ClubId, u16)> {
    let agent = &w.agents.list[a];
    let mut v = Vec::new();
    let Some(list) = idx.get(&p) else { return v };
    for &(club, seen) in list {
        if !agent.covers(w.clubs[club].nation) {
            continue;
        }
        let tie = f32::from(w.agents.tie(a, club)) / 100.0;
        let hear = 0.15 + f32::from(agent.network) / 30.0 + tie * 0.5;
        if rng.chance(hear.min(0.95)) {
            v.push((club, seen));
        }
    }
    v
}

/// Tell a client about interest, filtered by the agent's honesty and interests.
fn relay(w: &mut World, a: AgentId, p: PlayerId, found: &[(ClubId, u16)], origin: EventId) {
    let today = w.date;
    let agent = &w.agents.list[a];
    let (honest, greed, agent_person) = (f32::from(agent.honesty) / 20.0, f32::from(agent.greed) / 20.0, agent.person);
    let who = w.players.cold[p].person;
    for &(club, seen) in found {
        // Greedy, less honest agents talk up interest from rich clubs and play down the rest.
        let rich = f32::from(w.clubs[club].reputation) / 10_000.0;
        let shown = honest > 0.45 || rich * greed > 0.25;
        if !shown {
            continue;
        }
        let conf = (40.0 + (f32::from(seen) / 30.0).min(40.0) * honest + if greed > 0.6 { 10.0 } else { 0.0 }).min(95.0) as u8;
        w.beliefs.learn(who, Belief { about: who, kind: BeliefKind::ClubInterested { club }, channel: Channel::Agent(agent_person), confidence: conf, date: today, origin });
    }
}

/// Weekly work for every agent.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let ids: Vec<AgentId> = w.agents.list.ids().collect();
    let idx = tracking_index(w);
    let needy: Vec<ClubId> = w.clubs.ids().filter(|&c| !w.clubs[c].market.needs.is_empty()).collect();
    for a in ids {
        if !w.agents.list[a].active {
            continue;
        }
        let clients = w.agents.list[a].clients.clone();
        let attention = w.agents.list[a].attention();
        for p in clients {
            let mut rng = Rng::keyed(&[w.seed, stream::AGENT, u64::from(a.0), u64::from(p.0), week]);
            if !rng.chance(attention * 0.6) {
                continue;
            }
            let h = &w.players.hot[p];
            if h.status == PlayerStatus::Retired {
                continue;
            }
            let needs_move = h.status == PlayerStatus::FreeAgent
                || w.market.requests.contains_key(&p)
                || w.market.listed.contains_key(&p)
                || consider::minutes_grievance(w, p) > 0.4
                || (0..240).contains(&consider::contract_days_left(w, p));
            if needs_move {
                pitch(w, a, p, &needy, &mut rng);
            }
            let found = interest_known_to(w, a, p, &idx, &mut rng);
            if !found.is_empty() {
                relay(w, a, p, &found, EventId::NONE);
            }
            client_mood(w, a, p, !found.is_empty());
        }
    }
    if today.month() == 7 && today.day() <= 7 {
        ensure_market(w);
    }
}

/// Put a client in front of a club with a need, in the agent's network.
fn pitch(w: &mut World, a: AgentId, p: PlayerId, needy: &[ClubId], rng: &mut Rng) {
    let today = w.date;
    let (network, reach_base) = {
        let ag = &w.agents.list[a];
        (ag.network, ag.base)
    };
    let c = &w.players.cold[p];
    let group = c.best_pos.group();
    let level = f32::from(c.ca);
    let current = w.players.hot[p].club;
    let candidates: Vec<ClubId> = needy
        .iter()
        .copied()
        .filter(|&id| id != current && w.agents.list[a].covers(w.clubs[id].nation))
        .filter(|&id| w.clubs[id].market.needs.iter().any(|n| n.group == group && f32::from(n.min_ability) <= level + 6.0))
        .filter(|&id| crate::market::ideal_ca(w.clubs[id].reputation) <= level + 25.0)
        .collect();
    if candidates.is_empty() {
        return;
    }
    let tie_w: Vec<f32> = candidates.iter().map(|&c| 1.0 + f32::from(w.agents.tie(a, c)) / 20.0 + if w.clubs[c].nation == reach_base { 0.5 } else { 0.0 }).collect();
    let club = candidates[rng.weighted(&tie_w)];
    let minutes = 60 + u16::from(network) * 8;
    w.knowledge.observe(club, p, minutes, today);
    w.agents.strengthen(a, club, 1);
    if let Some(r) = w.agents.of_player.get_mut(&p) {
        r.last_pitch = today;
    }
    let causes: Causes = pw_world::causes![Cause::Fact(Fact::SquadNeed { club })];
    w.events.push_caused(today, Visibility::Club(club), EventKind::AgentPitch { player: p, agent: a, club }, causes);
}

/// An agent who hears a client is unsettled quietly sounds out clubs: a few
/// pitches to clubs with a need, and word that the player might be available
/// — which is itself information that can travel.
pub fn explore(w: &mut World, a: AgentId, p: PlayerId, cause: EventId) {
    let today = w.date;
    let agent = w.agents.list[a].person;
    let needy: Vec<ClubId> = w.clubs.ids().filter(|&c| !w.clubs[c].market.needs.is_empty()).collect();
    let mut rng = w.rng(stream::AGENT, &[u64::from(a.0), u64::from(p.0), pw_core::rng::period::week(today), 0xe8]);
    for _ in 0..3 {
        pitch(w, a, p, &needy, &mut rng);
    }
    let who = w.players.cold[p].person;
    let causes: Causes = if cause.is_some() { pw_world::causes![Cause::Event(cause)] } else { Causes::new() };
    let ev = w.events.push_caused(today, Visibility::Between(agent, who), EventKind::AgentExploring { agent, player: p }, causes);
    crate::grapevine::witness(w, pw_world::info::InfoKind::Exploring { player: p, agent }, ev, 45, &[agent, who], &[]);
}

/// Clients judge their agents; AI clients act on it (humans decide for themselves).
fn client_mood(w: &mut World, a: AgentId, p: PlayerId, news: bool) {
    let today = w.date;
    let who = w.players.cold[p].person;
    let unattached = w.players.hot[p].status == PlayerStatus::FreeAgent;
    let Some(r) = w.agents.of_player.get_mut(&p) else { return };
    let quiet = r.last_pitch.days_until(today) > 60;
    let delta: i32 = if news { 3 } else { 0 } - if unattached && quiet { 6 } else { 0 } - if quiet { 1 } else { 0 };
    r.satisfaction = (i32::from(r.satisfaction) + delta).clamp(0, 100) as u8;
    let sat = r.satisfaction;
    let expired = today > r.until;
    let agent_person = w.agents.list[a].person;
    let compat = consider::compat(w, who, agent_person);
    w.social.adjust(who, agent_person, today, compat, delta.signum(), delta.signum(), 0);
    if w.people[who].mind == MindKind::Ai && (sat < 20 || (expired && sat < 50)) {
        drop_agent(w, p);
    } else if expired && let Some(r) = w.agents.of_player.get_mut(&p) {
        r.until = today.add_months(24);
    }
}

/// A client sits down with their agent: what does the agent know?
pub fn review(w: &mut World, initiator: PersonId, with: PersonId, player: PlayerId, ev: EventId) {
    let Some(a) = w.agents.agent_of(player) else { return };
    let agent_person = w.agents.list[a].person;
    if agent_person != initiator && agent_person != with {
        return;
    }
    let mut rng = Rng::keyed(&[w.seed, stream::AGENT, u64::from(a.0), u64::from(player.0), w.date.0 as u64, 9]);
    // A sit-down gets the agent's full attention: they ring round their contacts.
    let mut idx: Tracking = Default::default();
    for club in w.clubs.ids() {
        let seen = consider::club_tracking(w, club, player);
        if seen >= 180 && w.players.hot[player].club != club && consider::club_need_for(w, club, player) > 0.0 {
            idx.entry(player).or_default().push((club, seen));
        }
    }
    let found = interest_known_to(w, a, player, &idx, &mut rng);
    relay(w, a, player, &found, ev);
}
