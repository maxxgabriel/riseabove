//! AI minds (01 §5, S5). Every AI-controlled person decides, on their own
//! initiative, what to do about their situation — from the same
//! considerations and through the same intents a human uses. A player short of
//! minutes may knock on the manager's door; a player whose manager broke his
//! word may hand in a transfer request; a veteran whose body is going and whose
//! phone has stopped ringing may call it a day. Nothing here reads who is
//! human: external minds simply don't get their intents generated.

use pw_core::math::sigmoid;
use pw_core::rng::{Rng, stream};
use pw_core::{Attr, Hidden, PersonId, PlayerId, Pos};
use pw_world::interaction::{Tone, Topic};
use pw_world::life::Routine;
use pw_world::{Focus, Intensity, Intent, Life, MemoryKind, MindKind, PlayerStatus, SquadStatus, StaffRole, TrainingPlan, World};

use crate::consider;
use crate::talk::ai_tone;

// ------------------------------------------------------------------ routine & training

/// How this person would spend their free week, left to themselves.
pub fn ai_routine(w: &World, who: PersonId, life: &Life) -> Routine {
    let p = &w.people[who];
    let prof = p.hidden.f(Hidden::Professionalism);
    let contro = p.hidden.f(Hidden::Controversy);
    let age = p.dob.age_years(w.date);
    let partner = life.partner().is_some();
    let kids = life.household.children;
    let abroad = life.home.is_some() && life.home != p.nation;
    let r = Routine {
        rest: (10.0 + prof * 0.4) as u8,
        recovery: (1.0 + (prof - 8.0).max(0.0) * 0.4 + (age - 28.0).max(0.0) * 0.4) as u8,
        family: (4 + kids * 3).min(14),
        partner: if partner { 8 } else { 0 },
        social: (8.0 - (age - 25.0).max(0.0) * 0.2) as u8,
        study: if age < 19.0 { 6 } else if prof >= 15.0 && age > 29.0 { 2 } else { 0 },
        hobbies: 6,
        media: if contro >= 13.0 { 4 } else { 1 },
        nightlife: ((contro - prof * 0.5).max(0.0) * 0.8 + if age < 24.0 { 2.0 } else { 0.0 }).min(10.0) as u8,
        language: if abroad && life.fluency(life.home) < 60 { (p.hidden.f(Hidden::Adaptability) / 5.0) as u8 } else { 0 },
    };
    r.normalised()
}

/// The training plan this player would choose, or their coaches would set.
pub fn ai_training(w: &World, p: PlayerId) -> TrainingPlan {
    let c = &w.players.cold[p];
    let h = &w.players.hot[p];
    let who = c.person;
    let prof = consider::hid(w, who, Hidden::Professionalism);
    let det = c.attrs.get(Attr::Determination);
    let age = consider::age(w, who);
    // Promised a new position? Work on it.
    let promised_pos = w.social.open_promises_to(who).find_map(|pr| match pr.kind {
        pw_world::PromiseKind::Position(pos) => Some(pos),
        _ => None,
    });
    let focus = if let Some(pos) = promised_pos {
        Focus::Position(pos)
    } else {
        // Weakest key attribute for the player's main role.
        let role = pw_core::Role::default_for(c.best_pos);
        let weakest = role.key_attrs().iter().map(|&(a, _)| a).min_by(|&a, &b| c.attrs.get(a).total_cmp(&c.attrs.get(b)));
        match weakest {
            Some(a) if c.attrs.get(a) < 14.0 && prof >= 11.0 => Focus::Attribute(a),
            Some(a) if prof >= 9.0 => Focus::Group(a.group()),
            _ => Focus::General,
        }
    };
    let tired = h.fatigue > 45 || h.acwr() > 1.35;
    let intensity = if tired || h.injury_days > 0 {
        Intensity::Light
    } else if prof + det >= 30.0 && age < 31.0 {
        Intensity::High
    } else {
        Intensity::Normal
    };
    let extra = if tired { 0 } else { ((prof + det - 24.0) / 6.0).clamp(0.0, 3.0) as u8 };
    let recovery = ((age - 27.0) / 3.0 + if c.injuries_career > 4 { 1.0 } else { 0.0 } + if prof >= 15.0 { 1.0 } else { 0.0 }).clamp(0.0, 3.0) as u8;
    TrainingPlan { focus, intensity, extra, recovery }
}

pub fn monthly(w: &mut World) {
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status != PlayerStatus::Retired).collect();
    for p in ids {
        let who = w.players.cold[p].person;
        if w.people[who].mind != MindKind::Ai {
            continue;
        }
        let plan = ai_training(w, p);
        w.players.cold[p].plan = plan;
        let routine = ai_routine(w, who, &w.lives[who]);
        let wage = w.players.cold[p].contract.current_wage(w.date);
        let lifestyle = crate::life::lifestyle_for(&w.people[who], wage);
        let l = &mut w.lives[who];
        l.routine = routine;
        l.finances.lifestyle = lifestyle;
    }
}

// ------------------------------------------------------------------ weekly initiative

/// Weekly: AI players act on their situation. Intents are queued and applied
/// by `intents::process`, exactly as a human's are.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect();
    let mut queued: Vec<(PersonId, Intent)> = Vec::new();
    for p in ids {
        let who = w.players.cold[p].person;
        if w.people[who].mind != MindKind::Ai {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::INTENT, u64::from(p.0), week]);
        if let Some(i) = consider_player(w, p, who, &mut rng) {
            queued.push((who, i));
        }
    }
    // Unattached and retired people consider their options too.
    for (who, person) in w.people.iter_enumerated() {
        if person.mind != MindKind::Ai || person.player.is_none() {
            continue;
        }
        let p = person.player;
        let status = w.players.hot[p].status;
        let mut rng = Rng::keyed(&[w.seed, stream::INTENT, u64::from(p.0), week, 1]);
        match status {
            PlayerStatus::FreeAgent => {
                if w.agents.agent_of(p).is_none() && rng.chance(0.15) {
                    if let Some(a) = crate::agents::best_available(w, p) {
                        queued.push((who, Intent::HireAgent(a)));
                    }
                }
                if retirement_choice(w, p, &mut rng) {
                    queued.push((who, Intent::Retire));
                }
            }
            PlayerStatus::Retired => {
                // A working life after playing: coaching (badges first), the
                // media, agency, scouting, a club role or business.
                let idle = person.staff.is_none() && w.affairs.of(who).is_none_or(|a| a.work.is_none() && a.studying.is_none());
                if idle && today.month() == 8 && rng.chance(staff_calling(w, p).max(0.25)) {
                    if let Some(i) = crate::affairs::ai_next_step(w, who, p) {
                        queued.push((who, i));
                    }
                }
            }
            PlayerStatus::Amateur => {
                // Adults in the amateur game may give it up; children don't retire.
                if consider::age(w, who) >= 23.0 && retirement_choice(w, p, &mut rng) {
                    queued.push((who, Intent::Retire));
                }
            }
            PlayerStatus::Active => {}
        }
    }
    for (who, i) in queued {
        w.intents.submit(who, i, today);
    }
}

fn consider_player(w: &World, p: PlayerId, who: PersonId, rng: &mut Rng) -> Option<Intent> {
    let today = w.date;
    let person = &w.people[who];
    let ambition = person.hidden.f(Hidden::Ambition) / 20.0;
    let loyalty = person.hidden.f(Hidden::Loyalty) / 20.0;
    let temper = 1.0 - person.hidden.f(Hidden::Temperament) / 20.0;
    let prof = person.hidden.f(Hidden::Professionalism) / 20.0;
    let age = consider::age(w, who);
    let mgr = w.manager_of_player(p);
    let grievance_mins = consider::minutes_grievance(w, p);

    // An agent first: most professionals have one.
    if w.agents.agent_of(p).is_none() && age >= 16.0 && rng.chance(0.03 + ambition * 0.05) {
        if let Some(a) = crate::agents::best_available(w, p) {
            return Some(Intent::HireAgent(a));
        }
    }
    let Some(mgr) = mgr else { return None };
    let met_recently = w.meetings.days_since_any(who, mgr, today).is_some_and(|d| d < 28) || w.meetings.has_pending(who, mgr);
    let grievance_mgr = consider::grievance(w, who, mgr);
    let failing = consider::failing_promises(w, mgr, who);

    // Withdraw a transfer request once things are good again.
    if w.market.has_requested(p) && grievance_mins < 0.1 && grievance_mgr < 0.2 && w.players.hot[p].morale > 65 && rng.chance(0.2) {
        return Some(Intent::WithdrawTransferRequest);
    }
    // Broken faith plus no football: ask to leave, formally.
    let want_out = grievance_mgr * 0.6 + grievance_mins * 0.8 + ambition * 0.3 + f32::from(consider::heard_interest(w, who).min(3)) * 0.1 - loyalty * 0.6;
    if !w.market.has_requested(p) && want_out > 0.9 && rng.chance(sigmoid((want_out - 1.0) * 4.0) * 0.3) {
        return Some(Intent::TransferRequest);
    }
    if met_recently {
        return None;
    }
    let tone = |topic| ai_tone(w, who, mgr, topic, true);
    if failing > 0 && rng.chance(0.25 + temper * 0.3) {
        return Some(Intent::RequestMeeting { with: mgr, topic: Topic::PromiseFollowUp, tone: tone(Topic::PromiseFollowUp) });
    }
    if grievance_mins > 0.35 && rng.chance(grievance_mins * (0.12 + ambition * 0.25 + temper * 0.1)) {
        let topic = if age <= 22.0 && w.players.cold[p].status >= SquadStatus::Squad && rng.chance(0.35) { Topic::LoanRequest } else { Topic::PlayingTime };
        return Some(Intent::RequestMeeting { with: mgr, topic, tone: tone(topic) });
    }
    let days_left = consider::contract_days_left(w, p);
    if (60..420).contains(&days_left) && matches!(w.players.cold[p].status, SquadStatus::Star | SquadStatus::Important | SquadStatus::Regular) && rng.chance(0.04 + ambition * 0.06) {
        return Some(Intent::RequestMeeting { with: mgr, topic: Topic::NewContract, tone: tone(Topic::NewContract) });
    }
    if consider::memory(w, who, mgr, MemoryKind::Argument) > 0.4 && rng.chance(prof * 0.12) {
        return Some(Intent::RequestMeeting { with: mgr, topic: Topic::Apology, tone: Tone::Humble });
    }
    if age < 23.0 && rng.chance(prof * 0.02) {
        return Some(Intent::RequestMeeting { with: mgr, topic: Topic::Feedback, tone: Tone::Calm });
    }
    None
}

// ------------------------------------------------------------------ retirement

/// Would this person, left to themselves, retire now? Body, opportunities,
/// money, family and motivation — the same inputs a human would weigh.
pub fn retirement_choice(w: &World, p: PlayerId, rng: &mut Rng) -> bool {
    let c = &w.players.cold[p];
    let h = &w.players.hot[p];
    let who = c.person;
    let age = consider::age(w, who);
    if age < 30.0 && h.status != PlayerStatus::FreeAgent {
        return false;
    }
    let body = consider::body_outlook(w, p);
    let unattached = (consider::days_unattached(w, p) as f32 / 240.0).min(1.5);
    let level = f32::from(c.ca) / 200.0;
    let ambition = consider::hid(w, who, Hidden::Ambition) / 20.0;
    let life = &w.lives[who];
    let secure = (life.finances.savings as f32 / (life.finances.spending.max(500) as f32 * 24.0)).min(1.5);
    let family = if life.household.children > 0 { 0.15 } else { 0.0 } + if life.household.parents.health < 35 && life.household.parents.alive > 0 { 0.1 } else { 0.0 };
    let long_injury = if h.injury_days > 150 { 0.4 } else { 0.0 };
    let age_push = ((age - 32.0) / 5.0).max(0.0);
    let x = -2.6 + age_push * 1.6 + (1.0 - body) * 2.2 + unattached * 1.4 + secure * 0.3 + family + long_injury - level * 1.5 - ambition * 0.5;
    rng.chance(sigmoid(x) * 0.5)
}

/// How drawn a retired player is to staying in football, per season.
fn staff_calling(w: &World, p: PlayerId) -> f32 {
    let c = &w.players.cold[p];
    let lead = c.attrs.get(Attr::Leadership);
    let det = c.attrs.get(Attr::Determination);
    let age = w.age_years(p);
    if age > 58.0 {
        return 0.0;
    }
    ((lead + det - 18.0) / 30.0).clamp(0.02, 0.6)
}

pub(crate) fn preferred_staff_role(w: &World, p: PlayerId) -> StaffRole {
    let c = &w.players.cold[p];
    let a = |x: Attr| c.attrs.get(x);
    if a(Attr::Leadership) >= 15.0 {
        StaffRole::Manager
    } else if c.best_pos == Pos::GK {
        StaffRole::GkCoach
    } else if a(Attr::Anticipation) + a(Attr::Vision) >= 28.0 {
        StaffRole::Scout
    } else if a(Attr::Teamwork) >= 14.0 {
        StaffRole::HeadOfYouth
    } else {
        StaffRole::Coach
    }
}
