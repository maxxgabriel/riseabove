//! The press and the fans (11, S16–S17).
//!
//! Outlets and journalists are actors with sources. A rumour exists only
//! because a club really is watching a player and someone who knows — a scout,
//! a director, an agent with a motive — told a journalist. How it reads
//! depends on the outlet: careful papers say "monitoring", tabloids say "bid
//! imminent". Outlets whose rumours come true earn credibility; the ones that
//! cry wolf lose it. News also follows public events — transfers, sackings,
//! long injuries, transfer requests — and leaks from inside clubs when someone
//! inside talks. Fans react to what they see and read, per club, with reasons.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{ClubId, Date, EventId, Hidden, Money, NationId, OutletId, PersonId, PlayerId, StoryId};
use pw_world::beliefs::{Belief, BeliefKind, Channel};
use pw_world::event::{Cause, EventKind, Fact, Visibility};
use pw_world::media::{Journalist, Outlet, OutletKind, Reaction, Story};
use pw_world::{CompKind, FanReason, MindKind, NameId, Person, PlayerStatus, StoryKind, TeamKind, World};
use smallvec::SmallVec;

use crate::consider;
use crate::generate as gen_;

// ------------------------------------------------------------------ worldgen

/// Every nation with football gets a press: a serious national title, a
/// tabloid, a broadcaster, a data site, and local papers for its big clubs.
/// Journalists are people with beats and sources. Rivalries come from shared
/// cities and shared title races.
pub fn ensure_media(w: &mut World) {
    let have: Vec<NationId> = w.media.outlets.iter().map(|o| o.nation).collect();
    for n in w.nations.ids() {
        if w.nations[n].leagues.is_empty() || have.contains(&n) {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::MEDIA, u64::from(n.0)]);
        let code = w.nations[n].code.clone();
        let rep = f32::from(w.nations[n].reputation) / 10_000.0;
        let reach = |base: f32| (base + rep * 8.0).clamp(1.0, 20.0) as u8;
        let mut outlets = vec![
            (format!("{code} Sport"), OutletKind::National, reach(8.0), 15u8, 6u8, ClubId::NONE),
            (format!("The {code} Sun"), OutletKind::Tabloid, reach(9.0), 6, 17, ClubId::NONE),
            (format!("{code} TV Football"), OutletKind::Broadcaster, reach(10.0), 13, 9, ClubId::NONE),
            (format!("{code} Numbers"), OutletKind::DataSite, reach(3.0), 18, 3, ClubId::NONE),
        ];
        let mut big: Vec<ClubId> = w.clubs.iter_enumerated().filter(|(_, c)| c.nation == n).map(|(id, _)| id).collect();
        big.sort_by_key(|&c| std::cmp::Reverse(w.clubs[c].reputation));
        for &c in big.iter().take(6) {
            let city = if w.clubs[c].city.is_empty() { w.clubs[c].short_name.clone() } else { w.clubs[c].city.clone() };
            outlets.push((format!("{city} Evening Post"), OutletKind::Local, reach(4.0), 11, 9, c));
            outlets.push((format!("{} Fan Channel", w.clubs[c].short_name), OutletKind::FanChannel, reach(2.0), 5, 14, c));
        }
        for (name, kind, reach, accuracy, sensationalism, leaning) in outlets {
            let oid = w.media.outlets.push(Outlet { name, nation: n, kind, reach, accuracy, sensationalism, leaning, credibility: 55 });
            let staff = if kind == OutletKind::FanChannel { 1 } else { 1 + rng.below(3) as usize };
            for _ in 0..staff {
                let (first, last) = crate::people::random_name(w, n, &mut rng);
                let dob = w.date.add_days(-(365 * rng.range_i32(24, 62)));
                let person = w.people.push(Person {
                    first,
                    last,
                    common: NameId::NONE,
                    dob,
                    nation: n,
                    nation2: Default::default(),
                    hidden: gen_::hidden_random(&mut rng),
                    player: Default::default(),
                    staff: Default::default(),
                    mind: MindKind::Ai,
                });
                let mut beat: SmallVec<[ClubId; 4]> = SmallVec::new();
                if leaning.is_some() {
                    beat.push(leaning);
                } else {
                    for _ in 0..3 {
                        if big.is_empty() {
                            break;
                        }
                        let c = big[rng.index(big.len().min(10))];
                        if !beat.contains(&c) {
                            beat.push(c);
                        }
                    }
                }
                w.media.journalists.insert(person, Journalist { person, outlet: oid, beat, sources: SmallVec::new(), credibility: 50 });
            }
        }
        // Rivalries: clubs sharing a city, and the top of each league.
        for (i, &a) in big.iter().enumerate() {
            for &b in &big[i + 1..] {
                let same_city = !w.clubs[a].city.is_empty() && w.clubs[a].city == w.clubs[b].city;
                let same_league = w.clubs[a].league == w.clubs[b].league;
                let intensity = if same_city { 90 } else if same_league && i < 3 { 55 } else { 0 };
                if intensity > 0 {
                    w.media.rivals.insert((a, b), intensity);
                    w.media.rivals.insert((b, a), intensity);
                }
            }
        }
    }
    refresh_sources(w);
}

/// Journalists cultivate sources at the clubs they cover: staff with loose
/// lips, and agents who find the press useful.
pub fn refresh_sources(w: &mut World) {
    let year = w.date.year() as u64;
    let js: Vec<PersonId> = w.media.journalists.keys().copied().collect();
    let mut js = js;
    js.sort();
    for j in js {
        let beat = w.media.journalists[&j].beat.clone();
        let mut rng = Rng::keyed(&[w.seed, stream::MEDIA, u64::from(j.0), year]);
        let mut sources: SmallVec<[PersonId; 8]> = SmallVec::new();
        for &club in &beat {
            let staff: Vec<PersonId> = w.clubs[club].staff.iter().map(|&s| w.staff[s].person).collect();
            for s in staff {
                let p = &w.people[s];
                let loose = (p.hidden.f(Hidden::Controversy) + 20.0 - p.hidden.f(Hidden::Professionalism)) / 40.0;
                if rng.chance(loose * 0.35) && sources.len() < 8 {
                    sources.push(s);
                }
            }
        }
        let agents: Vec<PersonId> = w.agents.list.iter().filter(|a| a.active && a.greed >= 12).map(|a| a.person).collect();
        for _ in 0..2 {
            if agents.is_empty() || sources.len() >= 8 {
                break;
            }
            let a = agents[rng.index(agents.len())];
            if !sources.contains(&a) {
                sources.push(a);
            }
        }
        if let Some(jj) = w.media.journalists.get_mut(&j) {
            jj.sources = sources;
        }
    }
}

// ------------------------------------------------------------------ publishing

#[allow(clippy::too_many_arguments)]
fn publish(
    w: &mut World,
    journalist: PersonId,
    kind: StoryKind,
    player: PlayerId,
    person: PersonId,
    club: ClubId,
    other_club: ClubId,
    fee: Money,
    claim: u8,
    grounded: bool,
    source: Cause,
    leaker: PersonId,
    tone: i8,
) -> StoryId {
    let today = w.date;
    let outlet = w.media.journalists.get(&journalist).map_or(OutletId::NONE, |j| j.outlet);
    let id = w.media.stories.next_id();
    let mut causes = pw_world::Causes::new();
    causes.push(source);
    let ev = w.events.push_caused(today, Visibility::Public, EventKind::Published { story: id }, causes);
    w.media.stories.push(Story { id, date: today, outlet, journalist, kind, player, person, club, other_club, fee, claim, grounded, source, leaker, tone, event: ev });
    // The subject reads it (or hears about it) — at the outlet's credibility.
    if person.is_some() {
        let cred = if outlet.is_some() { w.media.outlets[outlet].credibility } else { 40 };
        w.beliefs.learn(person, Belief { about: person, kind: BeliefKind::Rumour { story: id }, channel: Channel::Media(id), confidence: cred, date: today, origin: ev });
        if kind == StoryKind::TransferRumour && other_club.is_some() {
            w.beliefs.learn(person, Belief { about: person, kind: BeliefKind::ClubInterested { club: other_club }, channel: Channel::Media(id), confidence: cred / 2 + claim / 4, date: today, origin: ev });
        }
        if tone <= -40 {
            let l = &mut w.lives[person];
            let press = w.people[person].hidden.f(Hidden::Pressure);
            l.stress = l.stress.saturating_add(((20.0 - press) / 3.0) as u8).min(100);
        }
    }
    id
}

fn outlet_journalist(w: &World, nation: NationId, club: ClubId, key: u64) -> Option<PersonId> {
    let mut cands: Vec<PersonId> = w
        .media
        .journalists
        .values()
        .filter(|j| w.media.outlets[j.outlet].nation == nation && (club.is_none() || j.beat.contains(&club) || w.media.outlets[j.outlet].kind != OutletKind::Local))
        .map(|j| j.person)
        .collect();
    if cands.is_empty() {
        return None;
    }
    cands.sort();
    Some(cands[(hash_key(&[w.seed, key]) % cands.len() as u64) as usize])
}

fn big_enough(w: &World, club: ClubId) -> bool {
    if club.is_none() {
        return false;
    }
    let league = w.clubs[club].league;
    league.is_some() && w.comps[league].tier <= 2
}

pub fn weekly(w: &mut World) {
    rumours(w);
    news_from_events(w);
    form_and_pressure(w);
    fan_performance(w);
    settle_credibility(w);
    if w.date.month() == 7 && w.date.day() <= 7 {
        ensure_media(w);
    }
    w.media.compact(w.date.add_days(-120));
}

/// Leaks: sources who know a club is watching a player tell a journalist.
fn rumours(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let idx = crate::agents::tracking_index(w);
    let mut js: Vec<PersonId> = w.media.journalists.keys().copied().collect();
    js.sort();
    for j in js {
        let (sources, outlet) = {
            let jj = &w.media.journalists[&j];
            (jj.sources.clone(), jj.outlet)
        };
        let mut rng = Rng::keyed(&[w.seed, stream::MEDIA, u64::from(j.0), week]);
        for s in sources {
            let src = &w.people[s];
            let loose = (src.hidden.f(Hidden::Controversy) + 20.0 - src.hidden.f(Hidden::Professionalism)) / 40.0;
            let bond = consider::affinity(w, s, j).max(0.0) + 0.3;
            if !rng.chance(loose * bond * 0.25) {
                continue;
            }
            // What does this source actually know?
            let club = w.club_of_person(s);
            let agent = w.agents.list.iter_enumerated().find(|(_, a)| a.person == s).map(|(id, _)| id);
            let known: Vec<(PlayerId, ClubId, u16)> = if club.is_some() {
                w.knowledge
                    .known(club)
                    .filter(|(p, seen)| seen.minutes >= 180 && w.players.hot[*p].club != club && w.players.hot[*p].status == PlayerStatus::Active)
                    .filter(|(p, _)| consider::club_need_for(w, club, *p) > 0.0)
                    .map(|(p, seen)| (p, club, seen.minutes))
                    .collect()
            } else if let Some(a) = agent {
                // An agent talks up interest in their own clients.
                w.agents.list[a]
                    .clients
                    .iter()
                    .filter_map(|&p| idx.get(&p).and_then(|l| l.iter().max_by_key(|x| x.1)).map(|&(c, m)| (p, c, m)))
                    .collect()
            } else {
                Vec::new()
            };
            if known.is_empty() {
                continue;
            }
            let mut known = known;
            known.sort();
            // The most newsworthy: famous players, big clubs.
            let (p, interested, minutes) = *known
                .iter()
                .max_by_key(|(p, c, _)| u32::from(w.players.cold[*p].rep.current) + u32::from(w.clubs[*c].reputation) / 2)
                .unwrap();
            if w.media.stories.iter().rev().take(500).any(|st| st.player == p && st.other_club == interested && st.date.days_until(today) < 45) {
                continue;
            }
            let o = &w.media.outlets[outlet];
            let sens = f32::from(o.sensationalism) / 20.0;
            let acc = f32::from(o.accuracy) / 20.0;
            let truth = (f32::from(minutes) / 900.0).min(1.0) * 60.0;
            let claim = (truth + sens * 40.0 * (1.0 - acc * 0.5) + if agent.is_some() { 15.0 } else { 0.0 }).clamp(10.0, 100.0) as u8;
            let value = w.players.cold[p].value as f32;
            let fee = (value * (1.0 + sens * 0.6) / 50_000.0).round() as Money * 50_000;
            // A claim of an imminent bid with no bid is ungrounded.
            let bid_made = w.events.since(today.add_days(-30)).iter().any(|e| matches!(e.kind, EventKind::BidRejected { player, club, .. } | EventKind::BidAccepted { player, club, .. } if player == p && club == interested));
            let grounded = claim < 70 || bid_made;
            let person = w.players.cold[p].person;
            let current = w.players.hot[p].club;
            publish(w, j, StoryKind::TransferRumour, p, person, current, interested, fee, claim, grounded, Cause::Fact(Fact::Tracking { club: interested, player: p, minutes }), s, 0);
            // Fans of the player's club fret if a rival is circling.
            if w.media.rivalry(current, interested) > 50 {
                let ev = w.events.last_id();
                react(w, current, person, -20, FanReason::Leaving, ev);
            }
        }
    }
}

/// Public events (and leaked private ones) become news.
fn news_from_events(w: &mut World) {
    let today = w.date;
    let from = today.add_days(-7);
    let events: Vec<(EventId, EventKind, Visibility, pw_world::Causes)> =
        w.events.since(from).iter().filter(|e| e.date <= today).map(|e| (e.id, e.kind.clone(), e.vis, e.causes.clone())).collect();
    for (id, kind, vis, _causes) in events {
        let (story, player, club, other, tone): (StoryKind, PlayerId, ClubId, ClubId, i8) = match kind {
            EventKind::Transfer { player, from, to, fee } if big_enough(w, to) || big_enough(w, from) => {
                let _ = fee;
                (StoryKind::TransferNews, player, to, from, 10)
            }
            EventKind::ManagerSacked { club, .. } if big_enough(w, club) => (StoryKind::ManagerChange, PlayerId::NONE, club, ClubId::NONE, -20),
            EventKind::ManagerAppointed { club, .. } if big_enough(w, club) => (StoryKind::ManagerChange, PlayerId::NONE, club, ClubId::NONE, 10),
            EventKind::Injured { player, days, .. } if days >= 28 && big_enough(w, w.players.hot[player].club) && is_senior(w, player) => {
                (StoryKind::Injury, player, w.players.hot[player].club, ClubId::NONE, -5)
            }
            EventKind::TransferRequested { player, club } if big_enough(w, club) => (StoryKind::Unhappy, player, club, ClubId::NONE, -30),
            EventKind::Debut { player, team, .. } if w.teams[team].kind == TeamKind::First && w.age(player) <= 19 && big_enough(w, w.teams[team].club) => {
                (StoryKind::Praise, player, w.teams[team].club, ClubId::NONE, 40)
            }
            EventKind::Champion { comp, team, .. } if w.comps[comp].kind != CompKind::SuperCup => (StoryKind::Season, PlayerId::NONE, w.teams[team].club, ClubId::NONE, 50),
            EventKind::Retired { person } => {
                let p = w.people[person].player;
                if p.is_some() && w.players.cold[p].rep.current >= 3000 {
                    (StoryKind::Praise, p, ClubId::NONE, ClubId::NONE, 30)
                } else {
                    continue;
                }
            }
            // Club-internal matters only reach the press through a leak.
            EventKind::Fined { player, club, .. } | EventKind::Unrest { player, club } if big_enough(w, club) => {
                let Some(leaker) = inside_source(w, club, id) else { continue };
                let kind = if matches!(kind, EventKind::Fined { .. }) { StoryKind::Discipline } else { StoryKind::Unhappy };
                let person = w.players.cold[player].person;
                let nation = w.clubs[club].nation;
                if let Some(j) = source_journalist(w, leaker, nation, club, id) {
                    publish(w, j, kind, player, person, club, ClubId::NONE, 0, 60, true, Cause::Event(id), leaker, -35);
                    w.media.nudge_image(person, -20);
                }
                continue;
            }
            EventKind::Life { person, kind: pw_world::LifeEventKind::Married { .. } } if matches!(vis, Visibility::Public) => {
                let p = w.people[person].player;
                if p.is_some() && w.players.cold[p].rep.current >= 4000 {
                    (StoryKind::Personal, p, w.players.hot[p].club, ClubId::NONE, 25)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        let nation = if club.is_some() { w.clubs[club].nation } else if player.is_some() { w.people[w.players.cold[player].person].nation } else { continue };
        let Some(j) = outlet_journalist(w, nation, club, u64::from(id.0)) else { continue };
        let person = if player.is_some() { w.players.cold[player].person } else { PersonId::NONE };
        let fee = if let EventKind::Transfer { fee, .. } = kind { fee } else { 0 };
        publish(w, j, story, player, person, club, other, fee, 90, true, Cause::Event(id), PersonId::NONE, tone);
    }
}

fn is_senior(w: &World, p: PlayerId) -> bool {
    let t = w.players.hot[p].team;
    t.is_some() && w.teams[t].kind == TeamKind::First
}

/// Someone at the club who would talk, if anyone.
fn inside_source(w: &World, club: ClubId, key: EventId) -> Option<PersonId> {
    let mut cands: Vec<PersonId> = w
        .media
        .journalists
        .values()
        .flat_map(|j| j.sources.iter().copied())
        .filter(|&s| w.club_of_person(s) == club)
        .collect();
    cands.sort();
    cands.dedup();
    let r = hash_key(&[w.seed, stream::MEDIA, u64::from(key.0)]);
    let pick = *cands.get((r % cands.len().max(1) as u64) as usize)?;
    let p = &w.people[pick];
    let loose = (p.hidden.f(Hidden::Controversy) + 20.0 - p.hidden.f(Hidden::Professionalism)) / 40.0;
    if ((r >> 20) % 100) < (loose * 60.0) as u64 { Some(pick) } else { None }
}

fn source_journalist(w: &World, source: PersonId, nation: NationId, club: ClubId, key: EventId) -> Option<PersonId> {
    let mut js: Vec<PersonId> = w.media.journalists.values().filter(|j| j.sources.contains(&source)).map(|j| j.person).collect();
    js.sort();
    js.first().copied().or_else(|| outlet_journalist(w, nation, club, u64::from(key.0)))
}

/// Form streaks and board pressure at the top two tiers make columns.
fn form_and_pressure(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| big_enough(w, c)).collect();
    for club in clubs {
        let nation = w.clubs[club].nation;
        let b = w.clubs[club].board;
        if b.warnings >= 1 && today.weekday() == pw_core::Weekday::Mon && (u64::from(club.0) + week) % 3 == 0 {
            if let Some(j) = outlet_journalist(w, nation, club, hash_key(&[u64::from(club.0), week])) {
                publish(w, j, StoryKind::ManagerPressure, PlayerId::NONE, PersonId::NONE, club, ClubId::NONE, 0, 60, true, Cause::Fact(Fact::BoardPressure { club, warnings: b.warnings }), PersonId::NONE, -30);
            }
        }
        let first = w.clubs[club].first_team();
        let squad = w.teams[first].squad.clone();
        for p in squad {
            let who = w.players.cold[p].person;
            if w.players.cold[p].rep.current < 2500 {
                continue;
            }
            let low = w.lives[who].form_low_weeks;
            let hot = w.players.hot[p].form_avg().unwrap_or(0.0);
            let key = hash_key(&[u64::from(p.0), week, 0x51]);
            if low == 3 && key % 2 == 0 {
                if let Some(j) = outlet_journalist(w, nation, club, key) {
                    publish(w, j, StoryKind::Criticism, p, who, club, ClubId::NONE, 0, 60, true, Cause::Fact(Fact::FormSlump { player: p }), PersonId::NONE, -45);
                    w.media.move_fans(club, who, -30, FanReason::Performances, today);
                }
            } else if hot >= 7.6 && w.players.hot[p].form.iter().all(|&r| r >= 70) && key % 3 == 0 {
                if let Some(j) = outlet_journalist(w, nation, club, key) {
                    publish(w, j, StoryKind::Praise, p, who, club, ClubId::NONE, 0, 70, true, Cause::Fact(Fact::FormSurge { player: p }), PersonId::NONE, 45);
                }
            }
        }
    }
}

/// Fans watch the matches: this week's performances move how they feel.
fn fan_performance(w: &mut World) {
    let today = w.date;
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].last_match.days_until(today) <= 7 && is_senior(w, p)).collect();
    for p in ids {
        let h = w.players.hot[p];
        let club = w.playing_club(p);
        let who = w.players.cold[p].person;
        let r = f32::from(h.form[0]) / 10.0;
        let by = ((r - 6.8) * 25.0) as i16;
        w.media.move_fans(club, who, by, FanReason::Performances, today);
        // Settle, over time, toward neutral-with-memory.
        if let Some(f) = w.media.fans.get_mut(&(club, who)) {
            f.score -= f.score / 60;
        }
    }
}

/// Outlets are judged by whether their rumours came true.
fn settle_credibility(w: &mut World) {
    let today = w.date;
    let window: Vec<(StoryId, OutletId, PlayerId, ClubId, Date)> = w
        .media
        .stories
        .iter()
        .rev()
        .take(20_000)
        .filter(|s| s.kind == StoryKind::TransferRumour)
        .filter(|s| {
            let age = s.date.days_until(today);
            (120..127).contains(&age)
        })
        .map(|s| (s.id, s.outlet, s.player, s.other_club, s.date))
        .collect();
    for (_, outlet, player, club, date) in window {
        if outlet.is_none() {
            continue;
        }
        let happened = w.events.since(date).iter().any(|e| matches!(e.kind, EventKind::Transfer { player: x, to, .. } | EventKind::LoanMove { player: x, to, .. } if x == player && to == club));
        let o = &mut w.media.outlets[outlet];
        o.credibility = if happened { o.credibility.saturating_add(3).min(100) } else { o.credibility.saturating_sub(1) };
    }
}

/// Public reaction to a real event.
pub fn react(w: &mut World, club: ClubId, about: PersonId, sentiment: i8, reason: FanReason, event: EventId) {
    let volume = if club.is_some() { (w.clubs[club].reputation / 500).clamp(1, 20) as u8 } else { 5 };
    w.media.reactions.push(Reaction { date: w.date, club, about, sentiment, reason, event, volume });
}

/// A player moves: both fanbases react, and rivals remember.
pub fn on_move(w: &mut World, p: PlayerId, seller: ClubId, buyer: ClubId, ev: EventId) {
    let today = w.date;
    let who = w.players.cold[p].person;
    if seller.is_some() {
        let rival = w.media.rivalry(seller, buyer);
        let requested = w.events.since(today.add_days(-365)).iter().any(|e| matches!(e.kind, EventKind::TransferRequested { player, club } if player == p && club == seller));
        let years = w.history.spells.get(&p).and_then(|s| s.iter().rev().nth(1)).map_or(0.0, |s| s.from.days_until(today) as f32 / 365.0);
        let (by, reason) = if rival >= 50 {
            (-(200 + i16::from(rival) * 4), FanReason::JoinedRival)
        } else if requested {
            (-120, FanReason::Leaving)
        } else if years >= 4.0 {
            (80, FanReason::Loyalty)
        } else {
            (-10, FanReason::Leaving)
        };
        w.media.move_fans(seller, who, by, reason, today);
        react(w, seller, who, (by / 5).clamp(-100, 100) as i8, reason, ev);
        if rival >= 50 {
            w.media.nudge_image(who, -10);
        }
    }
    let rep = f32::from(w.players.cold[p].rep.current) / 100.0;
    let welcome = (rep * 0.8) as i16 + 20;
    w.media.move_fans(buyer, who, welcome, FanReason::Performances, today);
    react(w, buyer, who, (welcome / 3).clamp(-100, 100) as i8, FanReason::Performances, ev);
}
