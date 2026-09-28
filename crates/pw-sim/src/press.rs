//! The wider press (11 §3): what gets said on the record and what gets
//! written about football beyond rumours and news.
//!
//! - Speaking: managers at press conferences, players in interviews — AI
//!   minds choose to speak from their own situation, humans through the
//!   `SpeakToPress` intent. Words are stances; they land as memories on the
//!   people spoken about, move fans, and colour images.
//! - Match reports on the week's marquee games.
//! - Features and analysis pieces from how the media and analysts read
//!   players (`perf` readings).
//! - Lists of young talents, with real effects: fame, and the big clubs'
//!   scouts start watching.
//! - Season reviews, award and milestone news, international news.
//! - Retrospectives: anniversaries of titles, legends, hall of fame.

use pw_core::rng::hash_key;
use pw_core::{ClubId, CompId, Hidden, NationId, PersonId, PlayerId, StoryId};
use pw_world::event::{AwardKind, Cause, EventKind, Fact, MilestoneKind, Visibility};
use pw_world::media::{Quote, Stance, StoryLink};
use pw_world::perf::{Label, Lens};
use pw_world::{CompKind, FanReason, MemoryKind, MindKind, PlayerStatus, StoryKind, TeamKind, World};

use crate::consider;
use crate::media::{big_enough, outlet_journalist, publish};

pub fn weekly(w: &mut World) {
    player_interviews(w);
    match_reports(w);
    features(w);
    news_from_honours(w);
    let today = w.date;
    if today.month() == 10 && today.day() <= 7 {
        wonderkid_lists(w);
    }
    if today.day() <= 7 {
        retrospectives(w);
    }
}

// ---------------------------------------------------------------------------
// On the record
// ---------------------------------------------------------------------------

/// Someone says something publicly. The same path for AI and humans.
/// Returns the story and the quote record it produced.
pub fn speak(w: &mut World, speaker: PersonId, about: PersonId, stance: Stance) -> Option<(StoryId, u32)> {
    let today = w.date;
    let club = w.club_of_person(speaker);
    let nation = if club.is_some() { w.clubs[club].nation } else { w.people[speaker].nation };
    let key = hash_key(&[u64::from(speaker.0), u64::from(about.0), today.0 as u64]);
    let j = outlet_journalist(w, nation, club, key)?;
    let subject_player = if about.is_some() { w.people[about].player } else { w.people[speaker].player };
    let tone: i8 = match stance {
        Stance::Praise | Stance::Support | Stance::Loyalty => 40,
        Stance::Criticise | Stance::Complain => -45,
        Stance::Ambition => -10,
        Stance::Deflect | Stance::Deny => 0,
    };
    let about_or_self = if about.is_some() { about } else { speaker };
    let id = publish(w, j, StoryKind::Interview, subject_player, about_or_self, club, ClubId::NONE, 0, 90, true, Cause::Fact(Fact::Said { person: speaker }), PersonId::NONE, tone);
    w.media.links.insert(id, StoryLink::Quote(Quote { speaker, about, stance }));
    let quote = w.pressroom.quotes.len() as u32;
    w.pressroom.quotes.push(pw_world::pressroom::QuoteRecord { id: quote, speaker, about, stance, topic: None, date: today, conference: u32::MAX, story: id });
    let ev = w.media.stories[id].event;
    let compat = consider::compat(w, about, speaker);
    let fan_club = if about.is_some() { w.club_of_person(about) } else { club };
    match stance {
        Stance::Praise if about.is_some() => {
            w.social.remember(about, speaker, MemoryKind::PublicPraise, today, ev, true, 1.0, compat);
            if let Some(p) = w.people[about].player.get() {
                let h = &mut w.players.hot[p];
                h.morale = (h.morale + 4).min(100);
                h.confidence = (h.confidence + 4).min(100);
            }
        }
        Stance::Criticise if about.is_some() => {
            w.social.remember(about, speaker, MemoryKind::PublicCriticism, today, ev, true, 1.0, compat);
            if let Some(p) = w.people[about].player.get() {
                let h = &mut w.players.hot[p];
                h.morale = h.morale.saturating_sub(6);
                // Teammates who like the criticised player close ranks.
                let team = w.players.hot[p].team;
                if team.is_some() {
                    let mates: Vec<PersonId> = w.teams[team].squad.iter().map(|&x| w.players.cold[x].person).filter(|&x| x != about && consider::affinity(w, x, about) > 0.3).collect();
                    for m in mates {
                        let c = consider::compat(w, m, speaker);
                        w.social.adjust(m, speaker, today, c, -2, -3, 0);
                    }
                }
            }
            w.media.move_fans(fan_club, about, -25, FanReason::Interview, today);
        }
        Stance::Support if about.is_some() => {
            w.social.remember(about, speaker, MemoryKind::DefendedMe, today, ev, true, 1.0, compat);
        }
        Stance::Complain => {
            // A complaint about minutes or treatment is aimed at the manager.
            if let Some(p) = w.people[speaker].player.get() {
                if let Some(m) = w.manager_of_player(p) {
                    let c = consider::compat(w, m, speaker);
                    w.social.remember(m, speaker, MemoryKind::PublicCriticism, today, ev, true, 0.9, c);
                }
            }
            w.media.move_fans(club, speaker, -20, FanReason::Interview, today);
        }
        Stance::Ambition => {
            if let Some(p) = w.people[speaker].player.get() {
                if let Some(m) = w.manager_of_player(p) {
                    let c = consider::compat(w, m, speaker);
                    w.social.remember(m, speaker, MemoryKind::LetDown, today, ev, true, 0.5, c);
                }
            }
            w.media.move_fans(club, speaker, -40, FanReason::Interview, today);
        }
        Stance::Loyalty => {
            w.media.move_fans(club, speaker, 60, FanReason::Loyalty, today);
            if let Some(p) = w.people[speaker].player.get() {
                if let Some(m) = w.manager_of_player(p) {
                    let c = consider::compat(w, m, speaker);
                    w.social.adjust(m, speaker, today, c, 2, 3, 0);
                }
            }
        }
        _ => {}
    }
    // Sportsmanship shows in how people talk.
    let sport = consider::hid(w, speaker, Hidden::Sportsmanship);
    let nudge = match stance {
        Stance::Criticise | Stance::Complain => -((20.0 - sport) as i16),
        Stance::Praise | Stance::Support => 5,
        _ => 0,
    };
    w.media.nudge_image(speaker, nudge);
    Some((id, quote))
}

/// Players with something on their mind sometimes say it.
fn player_interviews(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let ids: Vec<PlayerId> = w
        .players
        .ids()
        .filter(|&p| w.players.hot[p].status == PlayerStatus::Active && w.players.cold[p].rep.current >= 3000 && big_enough(w, w.players.hot[p].club))
        .collect();
    for p in ids {
        let who = w.players.cold[p].person;
        if w.people[who].mind != MindKind::Ai {
            continue;
        }
        let roll = (hash_key(&[w.seed, u64::from(p.0), week, 0x1e7]) % 1000) as f32 / 1000.0;
        let controversy = consider::hid(w, who, Hidden::Controversy) / 20.0;
        if roll > 0.02 + controversy * 0.05 {
            continue;
        }
        let morale = f32::from(w.players.hot[p].morale);
        let grievance = consider::minutes_grievance(w, p);
        let interest = consider::heard_interest(w, who);
        let ambition = consider::hid(w, who, Hidden::Ambition) / 20.0;
        let loyalty = consider::hid(w, who, Hidden::Loyalty) / 20.0;
        let stance = if grievance > 0.4 && morale < 45.0 {
            Stance::Complain
        } else if interest >= 2 && ambition > 0.65 && loyalty < 0.5 {
            Stance::Ambition
        } else if interest >= 1 && loyalty > 0.65 {
            Stance::Loyalty
        } else {
            continue;
        };
        speak(w, who, PersonId::NONE, stance);
    }
}

// ---------------------------------------------------------------------------
// Match reports
// ---------------------------------------------------------------------------

/// Each top league's marquee game of the week gets a report.
fn match_reports(w: &mut World) {
    let today = w.date;
    let from = today.add_days(-7);
    let leagues: Vec<CompId> = w.comps.iter_enumerated().filter(|(_, c)| c.kind == CompKind::League && c.tier == 1 && c.team_kind == TeamKind::First).map(|(id, _)| id).collect();
    for comp in leagues {
        let best = w
            .fixtures
            .between(from, today)
            .map(|f| w.fixtures.get(f))
            .filter(|f| f.comp == comp && f.score.is_some())
            .max_by_key(|f| (u32::from(w.clubs[w.teams[f.home].club].reputation) + u32::from(w.clubs[w.teams[f.away].club].reputation), std::cmp::Reverse(f.uid)))
            .cloned();
        let Some(fx) = best else { continue };
        let score = fx.score.expect("played");
        let (home, away) = (w.teams[fx.home].club, w.teams[fx.away].club);
        // The star of the game, from the appearance records.
        let star = w
            .perf
            .recent
            .iter()
            .filter_map(|(&p, v)| v.iter().rev().find(|a| a.date == fx.date && a.comp == comp && (a.club == home || a.club == away)).map(|a| (p, a.rating)))
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
            .map_or(PlayerId::NONE, |x| x.0);
        let nation = w.clubs[home].nation;
        let Some(j) = outlet_journalist(w, nation, home, fx.uid) else { continue };
        let person = if star.is_some() { w.players.cold[star].person } else { PersonId::NONE };
        let tone = if star.is_some() { 30 } else { 0 };
        let ev_cause = Cause::Fact(Fact::Played { fixture: fx.uid });
        let id = publish(w, j, StoryKind::MatchReport, star, person, home, away, 0, 100, true, ev_cause, PersonId::NONE, tone);
        w.media.links.insert(id, StoryLink::Fixture { uid: fx.uid, home, away, hg: score.home, ag: score.away, star });
    }
}

// ---------------------------------------------------------------------------
// Features and analysis
// ---------------------------------------------------------------------------

fn features(w: &mut World) {
    let fresh = std::mem::take(&mut w.perf.fresh);
    for (p, label, lens, _) in fresh {
        let c = &w.players.cold[p];
        let club = w.players.hot[p].club;
        let notable = c.rep.current >= 2500 || (label == Label::Breakthrough && big_enough(w, club));
        if !notable || w.players.hot[p].status != PlayerStatus::Active {
            continue;
        }
        let who = c.person;
        let nation = if club.is_some() { w.clubs[club].nation } else { w.people[who].nation };
        let key = hash_key(&[u64::from(p.0), label as u64, w.date.0 as u64]);
        let Some(j) = outlet_journalist(w, nation, club, key) else { continue };
        let tone: i8 = match label {
            Label::BigGamePlayer | Label::InForm | Label::Underrated | Label::GoalThreat | Label::Workhorse | Label::Breakthrough | Label::Durable => 45,
            Label::FlatTrackBully | Label::InSlump | Label::Overrated | Label::Unreliable | Label::InjuryProne => -45,
            Label::FrozenOut => -10,
        };
        let kind = if lens == Lens::Analyst { StoryKind::Analysis } else { StoryKind::Feature };
        let id = publish(w, j, kind, p, who, club, ClubId::NONE, 0, 70, true, Cause::Fact(Fact::Reading { player: p }), PersonId::NONE, tone);
        w.media.links.insert(id, StoryLink::Reading(label));
        // Being written up moves the crowd a little; "frozen out" turns them on the manager.
        let by = i16::from(tone) / 3;
        w.media.move_fans(club, who, by, FanReason::Performances, w.date);
        if label == Label::FrozenOut {
            if let Some(m) = w.clubs.get(club).and_then(|c| c.manager.get()) {
                let mp = w.staff[m].person;
                w.media.move_fans(club, mp, -15, FanReason::Performances, w.date);
            }
        }
        if label == Label::Underrated {
            // Clubs read the analysis too.
            let readers: Vec<ClubId> = w.clubs.iter_enumerated().filter(|(id, cl)| cl.reputation >= 6000 && *id != club).map(|(id, _)| id).collect();
            for r in readers {
                w.knowledge.observe(r, p, 45, w.date);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Honours in print
// ---------------------------------------------------------------------------

fn news_from_honours(w: &mut World) {
    let today = w.date;
    let events: Vec<(pw_core::EventId, EventKind)> = w.events.since(today.add_days(-7)).iter().filter(|e| e.date <= today && matches!(e.vis, Visibility::Public)).map(|e| (e.id, e.kind.clone())).collect();
    for (id, kind) in events {
        let (story, player, person, club, other, tone, link): (StoryKind, PlayerId, PersonId, ClubId, ClubId, i8, Option<StoryLink>) = match kind {
            EventKind::Award { player, award, .. } if !matches!(award, AwardKind::TeamOfSeason | AwardKind::PlayerOfMonth) => {
                (StoryKind::AwardNews, player, w.players.cold[player].person, w.players.hot[player].club, ClubId::NONE, 50, Some(StoryLink::Award(award)))
            }
            EventKind::Milestone { player, kind, count, club } if significant(kind, count) => {
                (StoryKind::Milestone, player, w.players.cold[player].person, club, ClubId::NONE, 40, Some(StoryLink::Milestone(kind, count)))
            }
            EventKind::RecordBroken { player, kind, club, value } => {
                let person = if player.is_some() { w.players.cold[player].person } else { PersonId::NONE };
                (StoryKind::Milestone, player, person, club, ClubId::NONE, 40, Some(StoryLink::Record(kind, value)))
            }
            EventKind::TournamentWon { nation, tournament } => {
                let n = nation;
                let Some(j) = outlet_journalist(w, n, ClubId::NONE, u64::from(id.0)) else { continue };
                let sid = publish(w, j, StoryKind::International, PlayerId::NONE, PersonId::NONE, ClubId::NONE, ClubId::NONE, 0, 100, true, Cause::Event(id), PersonId::NONE, 60);
                w.media.links.insert(sid, StoryLink::Tournament(tournament));
                continue;
            }
            EventKind::Champion { comp, team, season } if w.comps[comp].kind == CompKind::League && w.comps[comp].tier == 1 => {
                let list: Vec<PlayerId> = w.history.awards.iter().filter(|a| a.comp == comp && a.season == season && a.kind == AwardKind::TeamOfSeason).map(|a| a.player).collect();
                (StoryKind::SeasonReview, PlayerId::NONE, PersonId::NONE, w.teams[team].club, ClubId::NONE, 30, Some(StoryLink::List(list)))
            }
            EventKind::BecameLegend { person, club } => (StoryKind::Retrospective, w.people[person].player, person, club, ClubId::NONE, 60, None),
            EventKind::InductedHallOfFame { person } => (StoryKind::Retrospective, w.people[person].player, person, ClubId::NONE, ClubId::NONE, 70, None),
            _ => continue,
        };
        let nation = if club.is_some() { w.clubs[club].nation } else if person.is_some() { w.people[person].nation } else { continue };
        let Some(j) = outlet_journalist(w, nation, club, u64::from(id.0) ^ 0x40) else { continue };
        let sid = publish(w, j, story, player, person, club, other, 0, 100, true, Cause::Event(id), PersonId::NONE, tone);
        if let Some(l) = link {
            w.media.links.insert(sid, l);
        }
    }
}

fn significant(kind: MilestoneKind, count: u16) -> bool {
    match kind {
        MilestoneKind::ClubApps => count >= 200,
        MilestoneKind::CareerGoals => count >= 100,
        MilestoneKind::SeniorApps => count >= 500,
        MilestoneKind::Caps => count >= 50,
    }
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

/// Each autumn, outlets rank the best young players they have seen: world
/// and per confederation. Being on the list is fame — and scrutiny.
fn wonderkid_lists(w: &mut World) {
    let today = w.date;
    let mut scopes: Vec<Option<pw_world::Confed>> = vec![None];
    scopes.extend(pw_world::Confed::ALL.iter().map(|&c| Some(c)));
    for scope in scopes {
        let mut v: Vec<(f32, PlayerId)> = w
            .players
            .ids()
            .filter(|&p| w.players.hot[p].status == PlayerStatus::Active && w.age(p) <= 20)
            .filter(|&p| {
                let club = w.players.hot[p].club;
                scope.is_none_or(|c| club.is_some() && w.nations[w.clubs[club].nation].confed == c)
            })
            .filter(|&p| w.players.cold[p].rep.current >= 800)
            .map(|p| {
                let c = &w.players.cold[p];
                let trend = f32::from(crate::growth::trajectory(w, p));
                let minutes = w.perf.season(p, today.year()).map_or(0.0, |s| s.minutes as f32).min(3000.0);
                let hype = if w.perf.has(p, Lens::Media, Label::Breakthrough) { 800.0 } else { 0.0 };
                let youth = (21.0 - w.age_years(p)).max(0.0) * 150.0;
                (f32::from(c.rep.current) + trend * 40.0 + minutes * 0.4 + hype + youth, p)
            })
            .collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let list: Vec<PlayerId> = v.into_iter().take(10).map(|x| x.1).collect();
        if list.len() < 5 {
            continue;
        }
        let nation: NationId = match scope {
            None => w.nations.iter_enumerated().max_by_key(|(_, n)| n.reputation).map_or(NationId::NONE, |x| x.0),
            Some(c) => w.nations.iter_enumerated().filter(|(_, n)| n.confed == c).max_by_key(|(_, n)| n.reputation).map_or(NationId::NONE, |x| x.0),
        };
        if nation.is_none() {
            continue;
        }
        let Some(j) = outlet_journalist(w, nation, ClubId::NONE, hash_key(&[today.year() as u64, scope.map_or(99, |c| c as u64)])) else { continue };
        let sid: StoryId = publish(w, j, StoryKind::WonderkidList, list[0], w.players.cold[list[0]].person, ClubId::NONE, ClubId::NONE, 0, 80, true, Cause::Fact(Fact::Ranking), PersonId::NONE, 50);
        // Consequences: fame, confidence, pressure; the big clubs start looking.
        let watchers: Vec<ClubId> = w.clubs.iter_enumerated().filter(|(_, c)| c.reputation >= 7000).map(|(id, _)| id).collect();
        for &p in &list {
            let who = w.players.cold[p].person;
            let bump = if scope.is_none() { 400 } else { 200 };
            let c = &mut w.players.cold[p];
            c.rep.world = c.rep.world.saturating_add(bump / 2).min(10_000);
            let r = w.renown.people.entry(who).or_default();
            r.fame = r.fame.saturating_add(bump).min(10_000);
            let pressure = consider::hid(w, who, Hidden::Pressure);
            if let Some(l) = w.lives.get_mut(who) {
                l.stress = l.stress.saturating_add(((20.0 - pressure) / 4.0) as u8).min(100);
            }
            for &club in &watchers {
                if club != w.players.hot[p].club {
                    w.knowledge.observe(club, p, 60, today);
                }
            }
        }
        w.media.links.insert(sid, StoryLink::List(list));
    }
}

// ---------------------------------------------------------------------------
// Retrospectives
// ---------------------------------------------------------------------------

/// Anniversaries of titles (10, 20, 25, 50 years) in top leagues and
/// continental competitions.
fn retrospectives(w: &mut World) {
    let today = w.date;
    let year = today.year();
    // Once a year, as seasons end.
    if today.month() != 6 {
        return;
    }
    let due: Vec<(u32, CompId, ClubId, i32)> = w
        .history
        .honours
        .iter()
        .enumerate()
        .filter(|(_, h)| matches!(year - h.season, 10 | 20 | 25 | 50))
        .filter(|(_, h)| {
            let c = &w.comps[h.comp];
            (c.kind == CompKind::League && c.tier == 1) || c.kind == CompKind::Continental
        })
        .map(|(i, h)| (i as u32, h.comp, h.club, h.season))
        .collect();
    for (idx, _comp, club, season) in due {
        let already = w.media.links.iter().any(|(sid, l)| matches!(l, StoryLink::Honour(i) if *i == idx) && w.media.stories[*sid].date.year() == year);
        if already {
            continue;
        }
        let nation = w.clubs[club].nation;
        let Some(j) = outlet_journalist(w, nation, club, u64::from(idx) ^ year as u64) else { continue };
        let sid = publish(w, j, StoryKind::Retrospective, PlayerId::NONE, PersonId::NONE, club, ClubId::NONE, 0, 100, true, Cause::Fact(Fact::Anniversary { year: season }), PersonId::NONE, 40);
        w.media.links.insert(sid, StoryLink::Honour(idx));
        // Nostalgia warms the fans.
        let mood = &mut w.clubs[club].fan_mood;
        *mood = (*mood + 1).min(100);
    }
}
