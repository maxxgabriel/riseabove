//! Chats around the people humans inhabit (`pw_world::chat`). Each message is someone's reaction to something that happened, chosen
//! by who they are to the person: the closest teammates write first, the captain speaks for the squad, family is proud or worried.
//! Runs after the chronicle at the end of the day; it starts for whoever has a chronicle. Nothing reads it to decide anything.

use pw_core::{ClubId, Date, EventId, PersonId, PlayerId, Rng};
use pw_world::World;
use pw_world::chat::{About, Inbox, Room, Said, Sender};
use pw_world::ecosystem::StageKind;
use pw_world::event::EventKind as E;

/// Days between quiet check-ins from the captain or a friend.
const CHECK_IN_GAP: i32 = 30;

pub fn daily(w: &mut World) {
    if w.ext.chronicle.lives.is_empty() {
        return;
    }
    let mut who: Vec<PersonId> = w.ext.chronicle.lives.keys().copied().collect();
    who.sort_unstable();
    for id in who {
        let mut inbox = w.ext.chats.of.remove(&id).unwrap_or_else(|| Inbox { cursor: w.events.last_id(), ..Inbox::default() });
        let p = w.people[id].player;
        if p.is_some() {
            day(w, &mut inbox, id, p);
        }
        inbox.cursor = w.events.last_id();
        w.ext.chats.of.insert(id, inbox);
    }
}

fn person_of(w: &World, q: PlayerId) -> PersonId {
    w.players.cold.get(q).map_or(PersonId::NONE, |c| c.person)
}

/// Teammates at `club` by how close they are to `me` (relationships first, then the rest of the squad), at most `n`.
fn closest(w: &World, me: PersonId, club: ClubId, n: usize, rng: &mut Rng) -> Vec<PersonId> {
    let at_club = |q: PersonId| w.people.get(q).is_some_and(|x| x.player.is_some() && w.players.hot[x.player].club == club);
    let mut rel: Vec<(PersonId, i16)> = w.social.relations_of(me).filter(|(q, r)| *q != me && r.affinity > 0 && at_club(*q)).map(|(q, r)| (q, i16::from(r.affinity))).collect();
    rel.sort_by_key(|(q, a)| (std::cmp::Reverse(*a), *q));
    let mut out: Vec<PersonId> = rel.into_iter().map(|(q, _)| q).take(n).collect();
    if out.len() < n {
        let team = w.clubs.get(club).map(|k| k.first_team());
        if let Some(t) = team.filter(|t| t.is_some()) {
            let mut squad: Vec<PersonId> = w.teams[t].squad.iter().map(|&q| person_of(w, q)).filter(|q| *q != me && !out.contains(q)).collect();
            rng.shuffle(&mut squad);
            out.extend(squad.into_iter().take(n - out.len()));
        }
    }
    out
}

fn captain(w: &World, club: ClubId, me: PersonId) -> Option<PersonId> {
    let t = w.clubs.get(club)?.first_team();
    if t.is_none() {
        return None;
    }
    let c = w.teams[t].captain;
    let q = person_of(w, c);
    (c.is_some() && q != me).then_some(q)
}

/// A parent (or a sibling) of `me`, if any are living.
fn family(w: &World, me: PersonId, rng: &mut Rng) -> Option<Sender> {
    let h = &w.lives.get(me)?.household;
    let mut v = Vec::new();
    if h.parents.alive >= 1 {
        v.push(Sender::Mother);
    }
    if h.parents.alive >= 2 {
        v.push(Sender::Father);
    }
    if h.siblings > 0 {
        v.push(Sender::Sibling);
    }
    (!v.is_empty()).then(|| *rng.pick(&v))
}

fn partner(w: &World, me: PersonId) -> Option<PersonId> {
    w.lives.get(me)?.household.partner.map(|p| p.person)
}

fn agent(w: &World, p: PlayerId) -> Option<PersonId> {
    w.agents.of_player.get(&p).and_then(|r| w.agents.list.get(r.agent)).map(|a| a.person)
}

/// Everyone close reacts to good news: two teammates privately, the family group, the partner.
fn good_news(w: &World, inbox: &mut Inbox, me: PersonId, club: ClubId, about: About, ev: EventId, today: Date, rng: &mut Rng) {
    if club.is_some() {
        for q in closest(w, me, club, 2, rng) {
            inbox.post(Room::Direct { with: q }, today, Sender::Person(q), Said::Congrats { about }, ev);
        }
    }
    if let Some(f) = family(w, me, rng) {
        inbox.post(Room::Family, today, f, Said::Proud { about }, ev);
    }
    if let Some(pt) = partner(w, me) {
        inbox.post(Room::Direct { with: pt }, today, Sender::Person(pt), Said::Proud { about }, ev);
    }
}

fn day(w: &World, inbox: &mut Inbox, me: PersonId, p: PlayerId) {
    let today = w.date;
    let mut rng = Rng::keyed(&[w.seed, 0x6368_6174, u64::from(today.0 as u32), u64::from(me.0)]);
    let club = w.players.hot[p].club;
    let squad_room = Room::Squad { club };
    let fresh: Vec<pw_world::event::Event> = w.events.after(inbox.cursor).to_vec();
    for e in &fresh {
        let ev = e.id;
        match e.kind {
            E::NationalSquad { player, nation, .. } if player == p => good_news(w, inbox, me, club, About::CallUp { nation }, ev, today, &mut rng),
            E::InternationalDebut { player, nation, .. } if player == p => good_news(w, inbox, me, club, About::Capped { nation }, ev, today, &mut rng),
            E::PathwayStep { player, kind, target } if player == p && StageKind::from_code(kind) == StageKind::StateTeam => {
                good_news(w, inbox, me, club, About::StateSide { state: pw_core::RegionId(target) }, ev, today, &mut rng);
            }
            E::Debut { player, .. } if player == p => {
                if let Some(c) = captain(w, club, me) {
                    inbox.post(squad_room, today, Sender::Person(c), Said::Congrats { about: About::Debut }, ev);
                }
                if let Some(f) = family(w, me, &mut rng) {
                    inbox.post(Room::Family, today, f, Said::Proud { about: About::Debut }, ev);
                }
            }
            E::FirstGoal { player, .. } if player == p => {
                for q in closest(w, me, club, 2, &mut rng) {
                    inbox.post(squad_room, today, Sender::Person(q), Said::Congrats { about: About::FirstGoal }, ev);
                }
                if let Some(f) = family(w, me, &mut rng) {
                    inbox.post(Room::Family, today, f, Said::Proud { about: About::FirstGoal }, ev);
                }
            }
            E::ContractSigned { player, club: k, .. } if player == p => {
                if let Some(f) = family(w, me, &mut rng) {
                    inbox.post(Room::Family, today, f, Said::Proud { about: About::Contract { club: k } }, ev);
                }
                if let Some(pt) = partner(w, me) {
                    inbox.post(Room::Direct { with: pt }, today, Sender::Person(pt), Said::Proud { about: About::Contract { club: k } }, ev);
                }
            }
            E::Transfer { player, from, to, .. } | E::LoanMove { player, from, to, .. } if player == p => {
                for q in closest(w, me, from, 2, &mut rng) {
                    inbox.post(Room::Squad { club: from }, today, Sender::Person(q), Said::Farewell { who: me }, ev);
                }
                if let Some(c) = captain(w, to, me) {
                    inbox.post(Room::Squad { club: to }, today, Sender::Person(c), Said::Welcome { who: me }, ev);
                }
                if let Some(f) = family(w, me, &mut rng) {
                    inbox.post(Room::Family, today, f, Said::Proud { about: About::Moved { club: to } }, ev);
                }
            }
            E::EnrolledUniversity { person, institution } if person == me => {
                if let Some(f) = family(w, me, &mut rng) {
                    inbox.post(Room::Family, today, f, Said::Proud { about: About::Scholarship { inst: institution } }, ev);
                }
            }
            E::Award { player, .. } if player == p => {
                if let Some(c) = captain(w, club, me) {
                    inbox.post(squad_room, today, Sender::Person(c), Said::Congrats { about: About::Award }, ev);
                }
            }
            E::Injured { player, days, .. } if player == p && days >= 14 => {
                if let Some(c) = captain(w, club, me) {
                    inbox.post(Room::Direct { with: c }, today, Sender::Person(c), Said::GetWell { days }, ev);
                }
                for q in closest(w, me, club, 1, &mut rng) {
                    inbox.post(Room::Direct { with: q }, today, Sender::Person(q), Said::GetWell { days }, ev);
                }
                if let Some(f) = family(w, me, &mut rng) {
                    inbox.post(Room::Family, today, f, Said::GetWell { days }, ev);
                }
            }
            E::Released { player, club: k } | E::AcademyReleased { player, club: k } if player == p => {
                for q in closest(w, me, k, 1, &mut rng) {
                    inbox.post(Room::Direct { with: q }, today, Sender::Person(q), Said::HeadUp, ev);
                }
                if let Some(f) = family(w, me, &mut rng) {
                    inbox.post(Room::Family, today, f, Said::HeadUp, ev);
                }
            }
            E::AskedIfReady { player, manager } if player == p => {
                inbox.post(Room::Direct { with: manager }, today, Sender::Person(manager), Said::AskedIfReady, ev);
            }
            E::AgentPitch { player, club: k, .. } if player == p => {
                if let Some(a) = agent(w, p) {
                    inbox.post(Room::Direct { with: a }, today, Sender::Person(a), Said::AgentNews { club: k }, ev);
                }
            }
            // A teammate's wedding or a child.
            E::Life { person, kind: pw_world::event::LifeEventKind::Married { .. } | pw_world::event::LifeEventKind::ChildBorn } if person != me && club.is_some() && {
                let q = w.people.get(person).map_or(PlayerId::NONE, |x| x.player);
                q.is_some() && w.players.hot[q].club == club
            } => {
                let child = matches!(e.kind, E::Life { kind: pw_world::event::LifeEventKind::ChildBorn, .. });
                if let Some(c) = captain(w, club, me).filter(|c| *c != person) {
                    inbox.post(squad_room, today, Sender::Person(c), Said::LifeNews { who: person, child }, ev);
                }
            }
            // Someone joins or leaves the squad.
            E::Transfer { player, from, to, .. } if player != p && club.is_some() && (to == club || from == club) => {
                let who = person_of(w, player);
                let said = if to == club { Said::Welcome { who } } else { Said::Farewell { who } };
                if let Some(c) = captain(w, club, me).filter(|c| *c != who) {
                    inbox.post(squad_room, today, Sender::Person(c), said, ev);
                }
            }
            _ => {
                // The club changing: the squad talks about it.
                if let Some((_, news)) = crate::chronicle::club_news(w, p, &e.kind)
                    && let Some(&q) = closest(w, me, club, 1, &mut rng).first()
                {
                    inbox.post(squad_room, today, Sender::Person(q), Said::ClubNews { news }, ev);
                }
            }
        }
    }
    from_the_story(w, inbox, me, club, today, &mut rng);
    if club.is_none() {
        return;
    }
    after_match(w, inbox, me, p, club, today, &mut rng);
    left_out(w, inbox, me, p, club, today);
    birthdays(w, inbox, me, club, today, &mut rng);
    far_from_home(w, inbox, me, p, today, &mut rng);
}

/// What today's chronicle lines bring to the chats: a teammate after the first match back, and the old squad when one of its
/// former members gets a first cap or a manager's job.
fn from_the_story(w: &World, inbox: &mut Inbox, me: PersonId, club: ClubId, today: Date, rng: &mut Rng) {
    use pw_world::chronicle::{Line, Then, TieKind};
    let Some(life) = w.ext.chronicle.of(me) else { return };
    for e in life.entries.iter().rev().take_while(|e| e.date == today) {
        match e.line {
            Line::Comeback { uid, club: k, .. } => {
                if let Some(&q) = closest(w, me, k, 1, rng).first() {
                    inbox.post(Room::Squad { club: k }, today, Sender::Person(q), Said::GoodToHaveYouBack { uid }, EventId::NONE);
                }
            }
            Line::Meanwhile { who, tie, then: then @ (Then::Capped { .. } | Then::BecameManager { .. }) } => {
                let Some(TieKind::Teammate { club: old }) = life.ties.get(usize::from(tie)).map(|t| t.kind) else { continue };
                if old == club || old.is_none() {
                    continue;
                }
                // Someone still at the old club says it, if anyone from your time there is.
                let at_old = |q: PersonId| w.people.get(q).is_some_and(|x| x.player.is_some() && w.players.hot[x.player].club == old);
                let speaker = life.ties.iter().filter(|t| matches!(t.kind, TieKind::Teammate { club: c } if c == old) && t.person != who && at_old(t.person)).map(|t| t.person).min();
                if let Some(q) = speaker {
                    inbox.post(Room::Squad { club: old }, today, Sender::Person(q), Said::OldTeamNews { who, then }, e.event);
                }
            }
            _ => {}
        }
    }
}

/// The squad's chat after today's match: one to three of those who played, the scorer named, the result in their mood.
fn after_match(w: &World, inbox: &mut Inbox, me: PersonId, p: PlayerId, club: ClubId, today: Date, rng: &mut Rng) {
    let Some(m) = w.recent_matches.on(today).find(|m| m.involves(club) && w.teams.get(if m.home == club { m.home_team } else { m.away_team }).is_some_and(|t| t.kind == pw_world::TeamKind::First)) else { return };
    let side = u8::from(m.away == club);
    let result = if side == 0 { m.result() } else { -m.result() };
    let mut scorers: Vec<PlayerId> = m.goals.iter().filter(|g| g.side == side && !g.own_goal).map(|g| g.player).collect();
    scorers.dedup();
    let scored = scorers.first().map_or(PersonId::NONE, |&q| person_of(w, q));
    let played: Vec<PersonId> = {
        let mut v: Vec<PersonId> = closest(w, me, club, 6, rng).into_iter().filter(|q| w.people.get(*q).is_some_and(|x| w.perf.recent.get(&x.player).is_some_and(|a| a.iter().any(|a| a.date == today && a.club == club)))).collect();
        v.truncate(match result {
            1.. => 3,
            0 => 1,
            _ => 1,
        });
        v
    };
    let room = Room::Squad { club };
    for (i, q) in played.iter().enumerate() {
        // After a defeat the captain speaks, if he played; otherwise the first of those who did.
        let from = if result < 0 && i == 0 { captain(w, club, me).unwrap_or(*q) } else { *q };
        let (gf, ga) = if side == 0 { (m.hg, m.ag) } else { (m.ag, m.hg) };
        inbox.post(room, today, Sender::Person(from), Said::AfterMatch { uid: m.uid, result, gf, ga, scored, derby: m.derby }, EventId::NONE);
    }
    // You made the difference: someone says so to you.
    let mine = w.perf.recent.get(&p).and_then(|a| a.iter().rev().find(|a| a.date == today && a.club == club)).copied();
    if let Some(a) = mine
        && (a.goals > 0 || m.pom == p)
        && result >= 0
        && let Some(&q) = played.first()
    {
        inbox.post(room, today, Sender::Person(q), Said::WellDone { uid: m.uid }, EventId::NONE);
    }
}

/// Left out of the squad again: now and then the captain or a friend checks in, quietly.
fn left_out(w: &World, inbox: &mut Inbox, me: PersonId, p: PlayerId, club: ClubId, today: Date) {
    let omitted = w.perf.season(p, today.year()).filter(|l| l.club == club).map_or(0, |l| l.omitted);
    let before = inbox.omitted_seen;
    inbox.omitted_seen = omitted;
    if omitted <= before || omitted < 3 || inbox.checked_in.days_until(today) < CHECK_IN_GAP {
        return;
    }
    let mut rng = Rng::keyed(&[w.seed, 0x6c65_6674, u64::from(today.0 as u32), u64::from(me.0)]);
    let who = captain(w, club, me).or_else(|| closest(w, me, club, 1, &mut rng).first().copied());
    if let Some(q) = who {
        inbox.checked_in = today;
        inbox.post(Room::Direct { with: q }, today, Sender::Person(q), Said::CheckIn, EventId::NONE);
    }
}

/// Birthdays in the squad, and your own.
fn birthdays(w: &World, inbox: &mut Inbox, me: PersonId, club: ClubId, today: Date, rng: &mut Rng) {
    let (_, m, d) = today.ymd();
    let born_today = |q: PersonId| w.people.get(q).is_some_and(|x| {
        let (_, bm, bd) = x.dob.ymd();
        bm == m && bd == d
    });
    if born_today(me) {
        if let Some(f) = family(w, me, rng) {
            inbox.post(Room::Family, today, f, Said::Birthday { who: me }, EventId::NONE);
        }
        if let Some(pt) = partner(w, me) {
            inbox.post(Room::Direct { with: pt }, today, Sender::Person(pt), Said::Birthday { who: me }, EventId::NONE);
        }
        for q in closest(w, me, club, 2, rng) {
            inbox.post(Room::Squad { club }, today, Sender::Person(q), Said::Birthday { who: me }, EventId::NONE);
        }
        return;
    }
    let team = w.clubs.get(club).map(|k| k.first_team()).filter(|t| t.is_some());
    let Some(t) = team else { return };
    // Someone in the squad wishes them well; not every birthday makes the group chat.
    if let Some(b) = w.teams[t].squad.iter().map(|&q| person_of(w, q)).find(|q| *q != me && born_today(*q))
        && rng.chance(0.5)
        && let Some(q) = {
            let mut v: Vec<PersonId> = closest(w, me, club, 4, rng).into_iter().filter(|q| *q != b).collect();
            rng.shuffle(&mut v);
            v.first().copied()
        }
    {
        inbox.post(Room::Squad { club }, today, Sender::Person(q), Said::Birthday { who: b }, EventId::NONE);
    }
}

/// Playing far from home: on the first of the month, sometimes, the family says they miss you.
fn far_from_home(w: &World, inbox: &mut Inbox, me: PersonId, p: PlayerId, today: Date, rng: &mut Rng) {
    if today.day() != 1 || !rng.chance(0.35) {
        return;
    }
    let eco = &w.ext.ecosystem;
    let Some(home) = eco.story.get(&p).map(|s| eco.state_of(s.home)) else { return };
    let club = w.players.hot[p].club;
    let here = eco.club_region.get(&club).map(|&r| eco.state_of(r));
    let abroad = w.clubs.get(club).is_some_and(|k| k.nation != w.people[me].nation);
    if (here.is_some_and(|h| h != home) || abroad)
        && let Some(f) = family(w, me, rng)
    {
        inbox.post(Room::Family, today, f, Said::Missing, EventId::NONE);
    }
}
