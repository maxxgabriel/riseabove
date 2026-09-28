//! Applying intents (S5). Whether an intent came from an AI mind's weekly
//! deliberation or from a human at the keyboard, it lands here and is carried
//! out by the same rules: a meeting is requested and the other side decides; a
//! transfer request goes public and people react; an agent may say no.

use pw_core::{PersonId, PlayerId};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::interaction::Topic;
use pw_world::intent::PendingIntent;
use pw_world::{FanReason, Intent, MemoryKind, PlayerStatus, World};

use crate::consider;

pub fn process(w: &mut World) {
    let queue = w.intents.take();
    for PendingIntent { person, intent, .. } in queue {
        apply(w, person, intent);
    }
}

fn apply(w: &mut World, who: PersonId, intent: Intent) {
    let today = w.date;
    let p = w.people[who].player;
    let playing = p.is_some() && w.players.hot[p].status == PlayerStatus::Active;
    match intent {
        Intent::RequestMeeting { with, topic, tone } => {
            if with.is_none() {
                return;
            }
            let club = w.club_of_person(who);
            let causes = meeting_causes(w, who, p, with, topic);
            crate::talk::request(w, who, with, p, club, topic, tone, causes);
        }
        Intent::TransferRequest if playing => transfer_request(w, who, p),
        Intent::WithdrawTransferRequest if playing => {
            if w.market.requests.remove(&p).is_some() {
                let club = w.players.hot[p].club;
                let ev = w.events.push(today, Visibility::Public, EventKind::TransferRequestWithdrawn { player: p, club });
                if let Some(m) = w.manager_of_player(p) {
                    let compat = consider::compat(w, m, who);
                    w.social.remember(m, who, MemoryKind::Apologised, today, ev, false, 0.7, compat);
                }
                w.media.move_fans(club, who, 40, FanReason::Loyalty, today);
            }
        }
        Intent::SetTraining(plan) if p.is_some() => {
            let mut plan = plan;
            plan.extra = plan.extra.min(3);
            plan.recovery = plan.recovery.min(3);
            w.players.cold[p].plan = plan;
        }
        Intent::SetRoutine(r) => w.lives[who].routine = r.normalised(),
        Intent::SetLifestyle(l) => w.lives[who].finances.lifestyle = l,
        Intent::HireAgent(a) if p.is_some() => crate::agents::hire(w, p, a),
        Intent::DropAgent if p.is_some() => crate::agents::drop_agent(w, p),
        Intent::Retire if p.is_some() && w.players.hot[p].status != PlayerStatus::Retired => crate::people::retire(w, p),
        Intent::SeekStaffJob(role) => {
            if let Some(s) = crate::people::enter_staff_pool(w, who, role) {
                // Hiring happens in the staff market; the person is now a candidate.
                let _ = s;
            }
        }
        Intent::Unretire if p.is_some() && w.players.hot[p].status == PlayerStatus::Retired && consider::age(w, who) < 40.0 => {
            let h = &mut w.players.hot[p];
            h.status = PlayerStatus::FreeAgent;
            h.sharpness = 10;
            h.condition = 70;
            h.fitness = 40;
            w.events.push(today, Visibility::Public, EventKind::CameOutOfRetirement { person: who });
        }
        Intent::AskPartner(ask) => {
            if let Some(pt) = w.lives[who].household.partner {
                // The partner decides with their own mind.
                let partner = pt.person;
                crate::decisions::partner_asks(w, partner, who, ask);
            }
        }
        Intent::OpenToDating(open) => {
            w.intents.dating.insert(who, open);
        }
        Intent::JoinAmateurFootball if p.is_some() && w.players.hot[p].status == PlayerStatus::FreeAgent => {
            w.players.hot[p].status = PlayerStatus::Amateur;
            crate::youth::join_local_near(w, p, who);
        }
        Intent::DeclareForNation(n) if p.is_some() => crate::intl::declare(w, p, n),
        Intent::RetireFromInternational if p.is_some() => crate::intl::retire(w, p),
        Intent::PlayThroughPain(b) if p.is_some() => crate::medical::set_willing(w, p, b),
        Intent::Mentor(mentee) if playing => crate::growth::offer_mentoring(w, who, mentee),
        _ => {}
    }
}

fn meeting_causes(w: &World, who: PersonId, p: PlayerId, with: PersonId, topic: Topic) -> Causes {
    let mut c = Causes::new();
    if p.is_none() {
        return c;
    }
    match topic {
        Topic::PlayingTime | Topic::LoanRequest | Topic::WantAway => {
            let (share, expected) = consider::minutes_share(w, p);
            c.push(Cause::Fact(Fact::MinutesShortfall { player: p, share_pct: (share * 100.0) as u8, expected_pct: (expected * 100.0) as u8 }));
        }
        Topic::NewContract => c.push(Cause::Fact(Fact::ContractRunningDown { player: p, days: consider::contract_days_left(w, p).max(0) as u16 })),
        Topic::PromiseFollowUp => {
            if let Some(pr) = w.social.open_promises_between(with, who).next() {
                c.push(Cause::Fact(Fact::PromiseDue { promise: pr.id }));
            }
        }
        _ => {}
    }
    if consider::trust(w, who, with) < 0.35 {
        c.push(Cause::Fact(Fact::LowTrust { from: who, about: with, trust: (consider::trust(w, who, with) * 100.0) as u8 }));
    }
    c
}

/// A formal transfer request: public, remembered, and felt by the fans.
fn transfer_request(w: &mut World, who: PersonId, p: PlayerId) {
    let today = w.date;
    if w.market.requests.contains_key(&p) {
        return;
    }
    let club = w.players.hot[p].club;
    let mut causes = Causes::new();
    if let Some(m) = w.manager_of_player(p) {
        if consider::grievance(w, who, m) > 0.2 {
            causes.push(Cause::Fact(Fact::LowTrust { from: who, about: m, trust: (consider::trust(w, who, m) * 100.0) as u8 }));
        }
    }
    let (share, expected) = consider::minutes_share(w, p);
    if share + 0.1 < expected {
        causes.push(Cause::Fact(Fact::MinutesShortfall { player: p, share_pct: (share * 100.0) as u8, expected_pct: (expected * 100.0) as u8 }));
    }
    w.market.requests.insert(p, today);
    let ev = w.events.push_caused(today, Visibility::Public, EventKind::TransferRequested { player: p, club }, causes);
    if let Some(m) = w.manager_of_player(p) {
        let compat = consider::compat(w, m, who);
        w.social.remember(m, who, MemoryKind::TransferRequest, today, ev, true, 1.0, compat);
    }
    let years = w.players.cold[p].joined.days_until(today) as f32 / 365.0;
    let by = -(60.0 + years * 25.0) as i16;
    w.media.move_fans(club, who, by, FanReason::TransferRequest, today);
    crate::media::react(w, club, who, by.max(-100) as i8, FanReason::TransferRequest, ev);
}
