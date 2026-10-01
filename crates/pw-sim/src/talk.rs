//! Conversations (09 §3–4, S6). One resolver for everyone.
//!
//! A meeting happens because somebody decided to have it — a manager who has
//! noticed three weeks of poor training, a player short of minutes, an agent
//! with news. The responder answers in a tone (their AI mind picks one, or a
//! human does), and then the *people* decide what it leads to: the manager
//! weighs the player's standing, their history, the squad, the board and his
//! own temperament; the player weighs theirs. No option has a fixed result.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{ClubId, Date, EventId, Hidden, MeetingId, PersonId, PlayerId, Pos, StaffAttr};
use pw_world::beliefs::{Belief, BeliefKind, Channel};
use pw_world::decision::{Choice, Decision, DecisionKind};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::interaction::{Meeting, MeetingState, Outcome, Tone, Topic};
use pw_world::{MemoryKind, MindKind, PlayerStatus, PromiseKind, SquadStatus, TeamKind, World};
use smallvec::SmallVec;

use crate::consider;
use crate::perception::club_view;

// ------------------------------------------------------------------ requests

/// Ask for a meeting. The responder answers by the meeting date; an external
/// responder gets a decision whose default is their own AI's answer.
#[allow(clippy::too_many_arguments)]
pub fn request(w: &mut World, initiator: PersonId, with: PersonId, player: PlayerId, club: ClubId, topic: Topic, opening: Tone, causes: Causes) -> Option<MeetingId> {
    if initiator == with || with.is_none() || w.meetings.has_pending(initiator, with) {
        return None;
    }
    let today = w.date;
    let date = today.add_days(if w.people[with].mind == MindKind::External { 3 } else { 1 });
    let id = w.meetings.list.next_id();
    let mut m = Meeting {
        id,
        requested: today,
        date,
        initiator,
        with,
        player,
        club,
        topic,
        opening,
        response: None,
        state: MeetingState::Pending,
        outcomes: SmallVec::new(),
        causes,
        decision: pw_core::DecisionId::NONE,
        event: EventId::NONE,
        satisfaction: 0,
    };
    let default = ai_tone(w, with, initiator, topic, false);
    if w.people[with].mind == MindKind::External {
        let options: SmallVec<[Choice; 5]> = Tone::ALL.iter().map(|&t| Choice::Respond(t)).collect();
        let default_idx = Tone::ALL.iter().position(|&t| t == default).unwrap_or(0) as u8;
        let responder_player = w.people[with].player;
        m.decision = w.decisions.push(Decision {
            person: with,
            player: if responder_player.is_some() { responder_player } else { player },
            kind: DecisionKind::Meeting { meeting: id },
            options,
            created: today,
            deadline: date,
            default: default_idx,
            answer: None,
            resolved: false,
        });
    } else {
        m.response = Some(default);
    }
    w.meetings.push(m);
    Some(id)
}

/// Meetings whose date has come are held.
pub fn daily(w: &mut World) {
    let today = w.date;
    prof!("talk::trim", w.meetings.trim_open());
    let due: Vec<MeetingId> = prof!("talk::due", w.meetings.pending().filter(|(_, m)| m.date <= today && m.response.is_some()).map(|(id, _)| id).collect());
    for id in due {
        prof!("talk::hold", hold(w, id));
    }
}

/// An external responder answered (or the default applied).
pub fn respond(w: &mut World, id: MeetingId, tone: Tone) {
    if let Some(m) = w.meetings.list.get_mut(id)
        && m.state == MeetingState::Pending
    {
        m.response = Some(tone);
    }
    hold(w, id);
}

// ------------------------------------------------------------------ tones

/// Which tone a person's own mind would take, from temperament, relationship
/// and what is at stake.
pub fn ai_tone(w: &World, speaker: PersonId, other: PersonId, topic: Topic, opening: bool) -> Tone {
    let p = &w.people[speaker];
    let temp = p.hidden.f(Hidden::Temperament);
    let prof = p.hidden.f(Hidden::Professionalism);
    let press = p.hidden.f(Hidden::Pressure);
    let trust = consider::trust(w, speaker, other);
    let aff = consider::affinity(w, speaker, other);
    let grievance = consider::grievance(w, speaker, other);
    let serious = matches!(topic, Topic::Attitude | Topic::Discipline | Topic::WantAway | Topic::Dropped);
    let weights = [
        // Calm
        1.0 + prof / 20.0 + trust * 0.5,
        // Assertive
        0.6 + press / 25.0 + if opening { 0.4 } else { 0.0 } + grievance * 0.3,
        // Aggressive
        (0.05 + (10.0 - temp).max(0.0) / 12.0 + grievance * 0.6 - trust * 0.3).max(0.02),
        // Humble
        0.3 + (prof - 10.0).max(0.0) / 20.0 + if matches!(topic, Topic::Attitude | Topic::Apology) { 0.8 } else { 0.0 },
        // Joking
        (0.1 + aff * 0.6 + if serious { -0.3 } else { 0.0 }).max(0.02),
    ];
    let mut rng = Rng::keyed(&[w.seed, stream::TALK, u64::from(speaker.0), u64::from(other.0), w.date.0 as u64]);
    Tone::ALL[rng.weighted(&weights)]
}

/// How a tone lands with a listener: (receptiveness −1..1, chance of a row 0..1).
fn tone_lands(w: &World, tone: Tone, speaker: PersonId, listener: PersonId, serious: bool) -> (f32, f32) {
    let l = &w.people[listener];
    let s = &w.people[speaker];
    let l_temp = l.hidden.f(Hidden::Temperament) / 20.0;
    let s_temp = s.hidden.f(Hidden::Temperament) / 20.0;
    let discipline = consider::staff_attr(w, listener, StaffAttr::Discipline) / 20.0;
    let trust = consider::trust(w, listener, speaker);
    let aff = consider::affinity(w, listener, speaker);
    match tone {
        Tone::Calm => (0.12 + trust * 0.1, 0.02),
        Tone::Assertive => (if trust >= 0.5 { 0.12 } else { -0.05 }, 0.05 + discipline * 0.12),
        Tone::Aggressive => (-0.3 - discipline * 0.2, (0.25 + (1.0 - l_temp) * 0.45 + (1.0 - s_temp) * 0.2 + discipline * 0.15 - trust * 0.2).clamp(0.05, 0.95)),
        Tone::Humble => (0.08 + (1.0 - discipline) * 0.05, 0.01),
        Tone::Joking => {
            if serious {
                (-0.15 - discipline * 0.15, 0.08 + discipline * 0.2)
            } else {
                (0.05 + aff * 0.25, 0.03)
            }
        }
    }
}

// ------------------------------------------------------------------ holding

struct Ctx {
    date: Date,
    initiator: PersonId,
    with: PersonId,
    player: PlayerId,
    player_person: PersonId,
    manager: PersonId,
    club: ClubId,
    opening: Tone,
    response: Tone,
    rng: Rng,
    ev: EventId,
    outcomes: SmallVec<[Outcome; 3]>,
    satisfaction: i8,
}

pub fn hold(w: &mut World, id: MeetingId) {
    let Some(m) = w.meetings.list.get(id).cloned() else { return };
    if m.state != MeetingState::Pending {
        return;
    }
    let today = w.date;
    // Did the world move on? Someone left the club or retired.
    let still_there = |w: &World, who: PersonId| w.club_of_person(who) == m.club || m.club.is_none();
    if !still_there(w, m.initiator) || !still_there(w, m.with) {
        w.meetings.list[id].state = MeetingState::Lapsed;
        return;
    }
    let response = m.response.unwrap_or(Tone::Calm);
    let ev = prof!("talk::event", w.events.push_caused(today, Visibility::Between(m.initiator, m.with), EventKind::Meeting { meeting: id, from: m.initiator, with: m.with }, m.causes.clone()));
    let player_person = if m.player.is_some() { w.players.cold[m.player].person } else { m.initiator };
    let manager = if player_person == m.initiator { m.with } else { m.initiator };
    let mut c = Ctx {
        date: today,
        initiator: m.initiator,
        with: m.with,
        player: m.player,
        player_person,
        manager,
        club: m.club,
        opening: m.opening,
        response,
        rng: Rng::keyed(&[w.seed, stream::TALK, u64::from(id.0), hash_key(&[today.0 as u64])]),
        ev,
        outcomes: SmallVec::new(),
        satisfaction: 0,
    };
    // The initiator's tone lands on the responder; the response lands back.
    let serious = matches!(m.topic, Topic::Attitude | Topic::Discipline | Topic::WantAway | Topic::Dropped);
    let (recv_a, row_a) = prof!("talk::tone", tone_lands(w, c.opening, c.initiator, c.with, serious));
    let (recv_b, row_b) = prof!("talk::tone", tone_lands(w, c.response, c.with, c.initiator, serious));
    let row = c.rng.chance(row_a.max(row_b) * 0.8);
    let mood = recv_a + recv_b;

    if m.player.is_some() && w.players.hot[m.player].status == PlayerStatus::Active {
        prof!("talk::topic", match m.topic {
            Topic::PlayingTime => playing_time(w, &mut c, mood),
            Topic::Feedback => feedback(w, &mut c, mood),
            Topic::Position => position(w, &mut c, mood),
            Topic::NewContract => new_contract(w, &mut c, mood),
            Topic::LoanRequest => loan_request(w, &mut c, mood),
            Topic::WantAway => want_away(w, &mut c, mood),
            Topic::PromiseFollowUp => follow_up(w, &mut c, mood),
            Topic::TeammateIssue => teammate_issue(w, &mut c, mood),
            Topic::Apology => apology(w, &mut c, mood),
            Topic::Attitude => attitude(w, &mut c, mood),
            Topic::Discipline => discipline(w, &mut c, mood),
            Topic::Dropped => dropped(w, &mut c, mood),
            Topic::Encouragement => encouragement(w, &mut c, mood),
            Topic::AgentReview => crate::agents::review(w, c.initiator, c.with, c.player, ev),
        })
    }
    if row {
        fall_out(w, &mut c);
    } else if mood > 0.15 && !c.outcomes.contains(&Outcome::Refused) {
        let compat = consider::compat(w, c.with, c.initiator);
        w.social.remember(c.with, c.initiator, MemoryKind::HonestTalk, today, ev, false, 0.8, compat);
    }
    let mm = &mut w.meetings.list[id];
    mm.state = MeetingState::Held;
    mm.response = Some(response);
    mm.outcomes = c.outcomes;
    mm.event = ev;
    mm.satisfaction = c.satisfaction;
}

/// The manager's willingness to give a player something: standing, trust,
/// how the conversation went, man-management, and what the squad can bear.
fn manager_goodwill(w: &World, c: &Ctx, mood: f32) -> f32 {
    let p = c.player;
    let trust = consider::trust(w, c.manager, c.player_person);
    let respect = w.social.get(c.manager, c.player_person).map_or(0.5, |r| f32::from(r.respect) / 100.0);
    let mm = consider::staff_attr(w, c.manager, StaffAttr::ManManagement) / 20.0;
    let status = match w.players.cold[p].status {
        SquadStatus::Star => 0.35,
        SquadStatus::Important => 0.25,
        SquadStatus::Regular => 0.15,
        SquadStatus::Youngster => 0.1,
        _ => 0.0,
    };
    let attitude = consider::memory(w, c.manager, c.player_person, MemoryKind::PoorAttitude);
    let work = consider::memory(w, c.manager, c.player_person, MemoryKind::ExtraWork);
    let infl = crate::social::influence(w, p);
    (trust - 0.5) * 0.8 + (respect - 0.5) * 0.6 + mm * 0.25 + status + infl * 0.2 + mood * 0.6 - attitude * 0.25 + work * 0.2
}

/// How close the player is to the manager's first choice in their position (−1 far … +1 starter).
fn standing_in_squad(w: &World, p: PlayerId) -> f32 {
    let club = w.playing_club(p);
    if club.is_none() {
        return -1.0;
    }
    let team = w.players.hot[p].team;
    let judging = w.club_manager_judging(club).0;
    let me = crate::perception::club_ca_judged(w, club, p, judging);
    let group = w.players.cold[p].best_pos.group();
    let mut rivals: Vec<f32> = w.teams[team].squad.iter().filter(|&&x| x != p && w.players.cold[x].best_pos.group() == group).map(|&x| crate::perception::club_ca_judged(w, club, x, judging)).collect();
    rivals.sort_by(|a, b| b.total_cmp(a));
    let starters: usize = match group {
        pw_core::PosGroup::Gk => 1,
        pw_core::PosGroup::Att => 2,
        _ => 4,
    };
    let bar = rivals.get(starters.saturating_sub(1)).copied().unwrap_or(0.0);
    ((me - bar) / 12.0).clamp(-1.0, 1.0)
}

fn remember_pair(w: &mut World, c: &Ctx, player_kind: Option<MemoryKind>, manager_kind: Option<MemoryKind>, intensity: f32) {
    let compat = consider::compat(w, c.player_person, c.manager);
    if let Some(k) = player_kind {
        w.social.remember(c.player_person, c.manager, k, c.date, c.ev, false, intensity, compat);
    }
    if let Some(k) = manager_kind {
        w.social.remember(c.manager, c.player_person, k, c.date, c.ev, false, intensity, compat);
    }
}

fn promise(w: &mut World, c: &mut Ctx, kind: PromiseKind, days: i32) {
    let due = c.date.add_days(days);
    let id = w.social.make_promise(c.manager, c.player_person, c.club, kind, c.date, due, c.ev);
    w.events.push_caused(c.date, Visibility::Between(c.manager, c.player_person), EventKind::PromiseMade { promise: id, from: c.manager, to: c.player_person }, pw_world::causes![Cause::Event(c.ev)]);
    c.outcomes.push(Outcome::PromiseMade { promise: id });
}

fn player_promises(w: &mut World, c: &mut Ctx, kind: PromiseKind, days: i32) {
    let due = c.date.add_days(days);
    let id = w.social.make_promise(c.player_person, c.manager, c.club, kind, c.date, due, c.ev);
    w.events.push_caused(c.date, Visibility::Between(c.player_person, c.manager), EventKind::PromiseMade { promise: id, from: c.player_person, to: c.manager }, pw_world::causes![Cause::Event(c.ev)]);
    c.outcomes.push(Outcome::PromiseMade { promise: id });
}

fn morale(w: &mut World, p: PlayerId, by: i32) {
    let h = &mut w.players.hot[p];
    h.morale = (i32::from(h.morale) + by).clamp(5, 100) as u8;
}

fn fall_out(w: &mut World, c: &mut Ctx) {
    remember_pair(w, c, Some(MemoryKind::Argument), Some(MemoryKind::Argument), 1.1);
    c.outcomes.push(Outcome::FellOut);
    c.satisfaction = c.satisfaction.saturating_sub(40);
    morale(w, c.player, -8);
    // Strict managers answer a row with a fine or a spell out of the side.
    let discipline = consider::staff_attr(w, c.manager, StaffAttr::Discipline);
    if c.rng.chance((discipline - 8.0).max(0.0) / 24.0) {
        let wage = w.players.cold[c.player].contract.current_wage(c.date);
        let weeks = if discipline >= 16.0 { 2 } else { 1 };
        let amount = wage * i64::from(weeks) / 2;
        w.events.push_caused(c.date, Visibility::Club(c.club), EventKind::Fined { player: c.player, club: c.club, amount }, pw_world::causes![Cause::Event(c.ev)]);
        remember_pair(w, c, Some(MemoryKind::Fined), None, 1.0);
        c.outcomes.push(Outcome::Fined { weeks });
        let f = &mut w.lives[c.player_person].finances;
        f.savings -= amount;
    }
}

fn playing_time(w: &mut World, c: &mut Ctx, mood: f32) {
    let goodwill = manager_goodwill(w, c, mood);
    let standing = standing_in_squad(w, c.player);
    let youth = consider::age(w, c.player_person) < 21.0;
    let noise = c.rng.normal() * 0.15;
    let give = goodwill + standing * 0.6 + noise;
    if give > 0.45 {
        let share = (w.players.cold[c.player].status.expected_minutes().max(0.4) + 0.1).min(0.8);
        promise(w, c, PromiseKind::Minutes { share }, 56);
        remember_pair(w, c, Some(MemoryKind::Backed), None, 0.8);
        morale(w, c.player, 6);
        c.satisfaction = 50;
    } else if give > 0.0 || youth {
        // "Show me in training."
        c.outcomes.push(Outcome::Deferred);
        player_promises(w, c, PromiseKind::ImproveTraining, 28);
        c.satisfaction = 5;
    } else {
        c.outcomes.push(Outcome::Refused);
        remember_pair(w, c, Some(MemoryKind::Refused), None, 0.9);
        morale(w, c.player, -5);
        c.satisfaction = -35;
        // Low-trust, high-discipline managers may decide the player is surplus.
        let discipline = consider::staff_attr(w, c.manager, StaffAttr::Discipline);
        if goodwill < -0.4 && c.rng.chance(discipline / 60.0) {
            list(w, c);
        }
    }
    tell_selection(w, c);
}

fn list(w: &mut World, c: &mut Ctx) {
    w.market.listed.insert(c.player, c.date);
    w.events.push_caused(c.date, Visibility::Public, EventKind::TransferListed { player: c.player, club: c.club }, pw_world::causes![Cause::Event(c.ev)]);
    c.outcomes.push(Outcome::Listed);
}

/// Managers who talk to a player also tell them where they stand — as they
/// see it and are willing to say.
fn tell_selection(w: &mut World, c: &Ctx) {
    let team = w.players.hot[c.player].team;
    if team.is_none() {
        return;
    }
    let (start, _) = crate::selection::forecast(w, team, c.player, c.date.add_days(3), 4);
    let honesty = consider::staff_attr(w, c.manager, StaffAttr::ManManagement) / 20.0;
    // Softer managers round up.
    let said = (start + (1.0 - honesty) * 0.15).clamp(0.0, 1.0);
    w.beliefs.learn(
        c.player_person,
        Belief {
            about: c.player_person,
            kind: BeliefKind::SelectionOutlook { start_pct: (said * 100.0) as u8 },
            channel: Channel::Told(c.manager),
            confidence: (50.0 + honesty * 40.0) as u8,
            date: c.date,
            origin: c.ev,
        },
    );
}

fn feedback(w: &mut World, c: &mut Ctx, mood: f32) {
    let club = w.playing_club(c.player);
    let (ca, band, pa, pa_band) = club_view(w, club, c.player);
    let honesty = consider::staff_attr(w, c.manager, StaffAttr::ManManagement) / 20.0;
    let ceiling = ((pa - ca + pa_band * 0.3) / 12.0).clamp(0.0, 4.0) as u8 + 1;
    w.beliefs.learn(
        c.player_person,
        Belief {
            about: c.player_person,
            kind: BeliefKind::Assessment { ca_lo: (ca - band * 0.5).clamp(1.0, 200.0) as u8, ca_hi: (ca + band * 0.5).clamp(1.0, 200.0) as u8, ceiling },
            channel: Channel::Told(c.manager),
            confidence: (45.0 + honesty * 45.0) as u8,
            date: c.date,
            origin: c.ev,
        },
    );
    let rating = (consider::trust(w, c.manager, c.player_person) * 3.0 + standing_in_squad(w, c.player) + 2.0).clamp(1.0, 5.0) as u8;
    w.beliefs.learn(
        c.player_person,
        Belief { about: c.player_person, kind: BeliefKind::ManagerRating { manager: c.manager, stars: rating }, channel: Channel::Told(c.manager), confidence: 70, date: c.date, origin: c.ev },
    );
    if mood > 0.0 {
        c.outcomes.push(Outcome::Praised);
        remember_pair(w, c, Some(MemoryKind::HonestTalk), Some(MemoryKind::HonestTalk), 0.8);
        c.satisfaction = 30;
    } else {
        c.outcomes.push(Outcome::Warned);
        c.satisfaction = 0;
    }
    tell_selection(w, c);
}

fn position(w: &mut World, c: &mut Ctx, mood: f32) {
    let cold = &w.players.cold[c.player];
    let wanted = match cold.plan.focus {
        pw_world::Focus::Position(p) => p,
        _ => {
            let weights = &w.data.weights;
            Pos::ALL
                .into_iter()
                .filter(|&p| p != cold.best_pos && p != Pos::GK)
                .max_by(|&a, &b| pw_world::player::raw_ability(&cold.attrs, a, weights).total_cmp(&pw_world::player::raw_ability(&cold.attrs, b, weights)))
                .unwrap_or(cold.best_pos)
        }
    };
    let fit = pw_world::player::raw_ability(&cold.attrs, wanted, &w.data.weights) / f32::from(cold.ca.max(1)); // truth-ok: his own attributes against his own overall level, which he knows
    let give = manager_goodwill(w, c, mood) + (fit - 0.9) * 2.0 + c.rng.normal() * 0.1;
    if give > 0.3 {
        promise(w, c, PromiseKind::Position(wanted), 70);
        w.players.cold[c.player].plan.focus = pw_world::Focus::Position(wanted);
        c.satisfaction = 40;
    } else {
        c.outcomes.push(Outcome::Refused);
        remember_pair(w, c, Some(MemoryKind::Refused), None, 0.6);
        c.satisfaction = -15;
    }
}

fn new_contract(w: &mut World, c: &mut Ctx, mood: f32) {
    let left = consider::contract_days_left(w, c.player);
    let goodwill = manager_goodwill(w, c, mood);
    let wanted = standing_in_squad(w, c.player) > -0.3 || consider::age(w, c.player_person) < 21.0;
    let give = goodwill + if left < 540 { 0.3 } else { -0.2 } + if wanted { 0.2 } else { -0.4 } + c.rng.normal() * 0.1;
    if give > 0.45 {
        let causes: Causes = pw_world::causes![Cause::Event(c.ev), Cause::Fact(Fact::ContractRunningDown { player: c.player, days: left.max(0) as u16 })];
        if crate::negotiation::open_renewal(w, c.player, c.club, causes).is_some() {
            c.outcomes.push(Outcome::TalksOpened);
            c.satisfaction = 45;
            return;
        }
    }
    if give > 0.15 {
        promise(w, c, PromiseKind::NewContract, 90);
        c.satisfaction = 15;
    } else {
        c.outcomes.push(Outcome::Refused);
        remember_pair(w, c, Some(MemoryKind::Refused), None, 0.9);
        morale(w, c.player, -4);
        c.satisfaction = -30;
    }
}

fn loan_request(w: &mut World, c: &mut Ctx, mood: f32) {
    let age = consider::age(w, c.player_person);
    let standing = standing_in_squad(w, c.player);
    let give = manager_goodwill(w, c, mood) * 0.5 - standing * 0.6 + if age <= 23.0 { 0.3 } else { -0.2 } + c.rng.normal() * 0.1;
    if give > 0.2 {
        w.market.loan_listed.insert(c.player, c.date);
        promise(w, c, PromiseKind::Loan, 90);
        c.satisfaction = 40;
    } else {
        c.outcomes.push(Outcome::Refused);
        remember_pair(w, c, Some(MemoryKind::Refused), None, 0.7);
        c.satisfaction = -20;
    }
}

fn want_away(w: &mut World, c: &mut Ctx, mood: f32) {
    let status = w.players.cold[c.player].status;
    let key = matches!(status, SquadStatus::Star | SquadStatus::Important);
    let goodwill = manager_goodwill(w, c, mood);
    if key && goodwill > -0.2 {
        // Won't sell now; may promise to let them go later.
        if c.rng.chance(0.4 + goodwill.max(0.0) * 0.4) {
            promise(w, c, PromiseKind::LetLeave, 240);
            c.satisfaction = 10;
        } else {
            c.outcomes.push(Outcome::Refused);
            remember_pair(w, c, Some(MemoryKind::Refused), Some(MemoryKind::LetDown), 1.0);
            c.satisfaction = -40;
        }
    } else {
        list(w, c);
        promise(w, c, PromiseKind::LetLeave, 120);
        remember_pair(w, c, None, Some(MemoryKind::LetDown), 0.7);
        c.satisfaction = 30;
    }
}

fn follow_up(w: &mut World, c: &mut Ctx, mood: f32) {
    let open: Vec<usize> = w.social.promises.iter().enumerate().filter(|(_, p)| p.from == c.manager && p.to == c.player_person && p.state == pw_world::PromiseState::Open).map(|(i, _)| i).collect();
    let Some(&i) = open.first() else {
        c.outcomes.push(Outcome::Refused);
        c.satisfaction = -5;
        return;
    };
    let overdue = w.social.promises[i].due < c.date;
    if overdue {
        crate::social::settle(w, i, false);
        c.outcomes.push(Outcome::FellOut);
        c.satisfaction = -30;
    } else if mood + manager_goodwill(w, c, mood) > 0.2 {
        c.outcomes.push(Outcome::Reconciled);
        remember_pair(w, c, Some(MemoryKind::HonestTalk), None, 0.6);
        c.satisfaction = 20;
    } else {
        c.outcomes.push(Outcome::Warned);
        c.satisfaction = -10;
    }
}

fn teammate_issue(w: &mut World, c: &mut Ctx, mood: f32) {
    // The teammate the player trusts least in the squad.
    let team = w.players.hot[c.player].team;
    let other = w.teams[team]
        .squad
        .iter()
        .copied()
        .filter(|&x| x != c.player)
        .map(|x| (x, consider::grievance(w, c.player_person, consider::person(w, x))))
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
    let Some((other, g)) = other else { return };
    if g <= 0.05 {
        c.outcomes.push(Outcome::Refused);
        return;
    }
    let op = consider::person(w, other);
    let side_with_player = consider::trust(w, c.manager, c.player_person) + mood * 0.3 > consider::trust(w, c.manager, op) + c.rng.normal() * 0.05;
    if side_with_player {
        remember_pair(w, c, Some(MemoryKind::Backed), None, 0.8);
        let compat = consider::compat(w, op, c.manager);
        w.social.remember(op, c.manager, MemoryKind::LetDown, c.date, c.ev, false, 0.6, compat);
        c.satisfaction = 30;
    } else {
        c.outcomes.push(Outcome::Refused);
        remember_pair(w, c, Some(MemoryKind::Refused), None, 0.7);
        c.satisfaction = -20;
    }
}

fn apology(w: &mut World, c: &mut Ctx, mood: f32) {
    let (from, to) = (c.initiator, c.with);
    let compat = consider::compat(w, to, from);
    let accepted = mood > -0.1 || c.rng.chance(0.5);
    if accepted {
        w.social.remember(to, from, MemoryKind::Apologised, c.date, c.ev, false, 1.0, compat);
        c.outcomes.push(Outcome::Reconciled);
        c.satisfaction = 30;
    } else {
        c.outcomes.push(Outcome::Refused);
        c.satisfaction = -10;
    }
}

fn attitude(w: &mut World, c: &mut Ctx, mood: f32) {
    // Manager raised the player's training. The player's reply decides a lot.
    match c.response {
        Tone::Humble | Tone::Calm => {
            player_promises(w, c, PromiseKind::ImproveTraining, 28);
            c.outcomes.push(Outcome::Reconciled);
            c.satisfaction = 20;
        }
        Tone::Assertive if mood > 0.0 => {
            c.outcomes.push(Outcome::Warned);
            c.satisfaction = 0;
        }
        _ => {
            c.outcomes.push(Outcome::Warned);
            remember_pair(w, c, None, Some(MemoryKind::PoorAttitude), 0.8);
            c.satisfaction = -20;
            let discipline = consider::staff_attr(w, c.manager, StaffAttr::Discipline);
            if c.rng.chance(discipline / 40.0) {
                c.outcomes.push(Outcome::Dropped);
                remember_pair(w, c, Some(MemoryKind::Dropped), None, 0.8);
                demote(w, c);
            }
        }
    }
}

fn discipline(w: &mut World, c: &mut Ctx, mood: f32) {
    let strict = consider::staff_attr(w, c.manager, StaffAttr::Discipline);
    let weeks = if strict >= 15.0 { 2 } else { 1 };
    if mood < 0.2 || strict >= 14.0 {
        let wage = w.players.cold[c.player].contract.current_wage(c.date);
        let amount = wage * i64::from(weeks) / 2;
        w.events.push_caused(c.date, Visibility::Club(c.club), EventKind::Fined { player: c.player, club: c.club, amount }, pw_world::causes![Cause::Event(c.ev)]);
        w.lives[c.player_person].finances.savings -= amount;
        remember_pair(w, c, Some(MemoryKind::Fined), None, 1.0);
        c.outcomes.push(Outcome::Fined { weeks });
        c.satisfaction = 20;
    } else {
        c.outcomes.push(Outcome::Warned);
        c.satisfaction = 10;
    }
}

fn dropped(w: &mut World, c: &mut Ctx, mood: f32) {
    tell_selection(w, c);
    if mood > 0.0 {
        c.outcomes.push(Outcome::Reconciled);
        remember_pair(w, c, Some(MemoryKind::HonestTalk), None, 0.7);
    } else {
        c.outcomes.push(Outcome::Warned);
        remember_pair(w, c, Some(MemoryKind::Dropped), None, 0.6);
    }
}

fn encouragement(w: &mut World, c: &mut Ctx, _mood: f32) {
    c.outcomes.push(Outcome::Praised);
    remember_pair(w, c, Some(MemoryKind::Backed), None, 0.9);
    morale(w, c.player, 5);
    let h = &mut w.players.hot[c.player];
    h.confidence = h.confidence.saturating_add(4).min(100);
    c.satisfaction = 30;
}

/// A manager marks a player down the pecking order after a bad meeting.
fn demote(w: &mut World, c: &mut Ctx) {
    let cold = &mut w.players.cold[c.player];
    let from = cold.status;
    let to = match from {
        SquadStatus::Star => SquadStatus::Important,
        SquadStatus::Important => SquadStatus::Regular,
        SquadStatus::Regular => SquadStatus::Squad,
        SquadStatus::Squad | SquadStatus::ImpactSub => SquadStatus::Fringe,
        s => s,
    };
    if from != to && cold.contract.promised_status.is_none() {
        cold.status = to;
        w.events.push_caused(c.date, Visibility::Club(c.club), EventKind::StatusChanged { player: c.player, club: c.club, from, to }, pw_world::causes![Cause::Event(c.ev)]);
        c.outcomes.push(Outcome::StatusChanged);
    }
}

// ------------------------------------------------------------------ manager initiative

/// Weekly: managers decide whom they need to talk to. The same rules apply to
/// every player in every squad; how often they act depends on the manager.
pub fn manager_summons(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let teams: Vec<pw_core::TeamId> = w.teams.ids().filter(|&t| matches!(w.teams[t].kind, TeamKind::First | TeamKind::Reserve | TeamKind::U21)).collect();
    // Long bans in the last week, found once rather than per player.
    let banned: pw_world::FxHashSet<PlayerId> = w
        .events
        .since(today.add_days(-7))
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Suspended { player, matches } if matches >= 3 => Some(player),
            _ => None,
        })
        .collect();
    for team in teams {
        let Some(mgr) = crate::social::team_manager(w, team) else { continue };
        let club = w.teams[team].club;
        let discipline = consider::staff_attr(w, mgr, StaffAttr::Discipline);
        let mm = consider::staff_attr(w, mgr, StaffAttr::ManManagement);
        let motivating = consider::staff_attr(w, mgr, StaffAttr::Motivating);
        let mut held = 0;
        let squad = w.teams[team].squad.clone();
        for p in squad {
            if held >= 2 {
                break;
            }
            if w.players.hot[p].status != PlayerStatus::Active {
                continue;
            }
            let who = consider::person(w, p);
            if w.meetings.days_since_any(mgr, who, today).is_some_and(|d| d < 21) || w.meetings.has_pending(mgr, who) {
                continue;
            }
            let mut rng = Rng::keyed(&[w.seed, stream::TALK, u64::from(mgr.0), u64::from(p.0), week]);
            let life = &w.lives[who];
            let low = life.train_low_weeks;
            let high = life.train_high_weeks;
            let trust = consider::trust(w, mgr, who);
            let banned_recently = banned.contains(&p);
            let (topic, causes, chance): (Topic, Causes, f32) = if banned_recently {
                (Topic::Discipline, Causes::new(), 0.3 + discipline / 30.0)
            } else if low >= 3 {
                (
                    Topic::Attitude,
                    pw_world::causes![Cause::Fact(Fact::TrainingSlump { player: p, weeks: low }), Cause::Fact(Fact::LowTrust { from: mgr, about: who, trust: (trust * 100.0) as u8 })],
                    (discipline / 20.0) * (1.2 - trust) * 0.6,
                )
            } else if high >= 3 && consider::age(w, who) < 24.0 {
                (Topic::Encouragement, pw_world::causes![Cause::Fact(Fact::TrainingSurge { player: p, weeks: high })], (motivating + mm) / 40.0 * 0.35)
            } else if consider::minutes_grievance(w, p) > 0.5 && crate::social::influence(w, p) > 0.45 {
                (
                    Topic::Dropped,
                    pw_world::causes![Cause::Fact(Fact::MinutesShortfall {
                        player: p,
                        share_pct: (consider::minutes_share(w, p).0 * 100.0) as u8,
                        expected_pct: (consider::minutes_share(w, p).1 * 100.0) as u8
                    })],
                    mm / 20.0 * 0.3,
                )
            } else {
                continue;
            };
            if !rng.chance(chance) {
                continue;
            }
            let tone = ai_tone(w, mgr, who, topic, true);
            if request(w, mgr, who, p, club, topic, tone, causes).is_some() {
                held += 1;
            }
        }
    }
}
