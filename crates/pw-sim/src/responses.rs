//! How people in authority respond to incidents (C in the brief).
//!
//! There is no correct response. A manager who learns of a training-ground
//! fight weighs his own disposition (discipline, feel for people, temper,
//! how he deals with the press), his authority (tenure, backing of the
//! dressing room and the board, stature against the players involved), his
//! relationships (trust, favourites), the pressure he is under (board,
//! the next match), the club's culture, the hierarchy of the squad and — not
//! least — how much he actually knows and how reliably. The same event
//! therefore ends in a fine and a dropped instigator at one club, a quiet
//! mediation at another, nothing at all at a third, and a protected
//! favourite and a scapegoat at a fourth.
//!
//! Responses are applied through ordinary state and recorded with the
//! reasons that drove them (for `why`). A human in authority gets a
//! decision with the options their situation allows; the default is what
//! their own mind would have done.

use pw_core::rng::stream;
use pw_core::{ClubId, Hidden, PersonId, PlayerId, StaffAttr, StaffId};
use pw_world::careers::MediaStyle;
use pw_world::decision::{Choice, Decision, DecisionKind, MindKind};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::incident::{Ask, IncidentKind, Reason, Response, ResponseRecord};
use pw_world::info::{Fidelity, Learned, Motive};
use pw_world::media::Stance;
use pw_world::{FanReason, MemoryKind, SquadStatus, StaffRole, World};
use smallvec::SmallVec;

use crate::consider;

fn club_manager(w: &World, club: ClubId) -> (PersonId, StaffId) {
    if club.is_none() {
        return (PersonId::NONE, StaffId::NONE);
    }
    w.clubs[club].manager.get().map_or((PersonId::NONE, StaffId::NONE), |m| (w.staff[m].person, m))
}

/// Who decides what happens about an incident.
pub fn authority(w: &World, id: u32) -> PersonId {
    let Some(inc) = w.incidents.get(id) else { return PersonId::NONE };
    match inc.kind {
        IncidentKind::OwnershipControversy | IncidentKind::SupporterUnrest => w.governance.get(&inc.club).map_or(PersonId::NONE, |g| g.chairman),
        k if k.is_conduct() || k.needs_leave() || k == IncidentKind::FamilyEmergency => {
            // Children in an academy answer to its head.
            let young = inc.players.first().is_some_and(|&p| w.age(p) < 18);
            if young && inc.club.is_some() {
                let head = w.clubs[inc.club].staff.iter().find(|&&s| w.staff[s].role == StaffRole::HeadOfYouth).map(|&s| w.staff[s].person);
                if let Some(h) = head {
                    return h;
                }
            }
            club_manager(w, inc.club).0
        }
        _ => PersonId::NONE,
    }
}

/// Someone learned of an incident: if it is theirs to deal with, they do.
pub fn on_learn(w: &mut World, who: PersonId, id: u32) {
    let Some(inc) = w.incidents.get(id) else { return };
    if inc.resolved || inc.responses.iter().any(|r| r.by == who) {
        return;
    }
    // Leave requests are only decided once the person has asked.
    if inc.kind.needs_leave() && !inc.responses.is_empty() {
        return;
    }
    if authority(w, id) != who || who.is_none() {
        return;
    }
    if inc.kind.needs_leave() && !asked(w, id) {
        return;
    }
    // Already waiting on this person's decision.
    if w.decisions.all.iter().any(|d| !d.resolved && d.person == who && matches!(d.kind, DecisionKind::Incident { incident } if incident == id)) {
        return;
    }
    let (choice, reasons, options) = decide(w, who, id);
    if w.people[who].mind == MindKind::External {
        let today = w.date;
        let default = options.iter().position(|&r| r == choice).unwrap_or(0) as u8;
        let kind = DecisionKind::Incident { incident: id };
        let player = w.people[who].player;
        w.decisions.push(Decision {
            person: who,
            player,
            kind,
            options: options.iter().map(|&r| Choice::Handle(r)).collect(),
            created: today,
            deadline: today.add_days(2),
            default,
            answer: None,
            resolved: false,
        });
        return;
    }
    apply(w, id, who, choice, reasons);
}

/// Has the person concerned asked for leave (for leave-type incidents)?
fn asked(w: &World, id: u32) -> bool {
    let Some(inc) = w.incidents.get(id) else { return false };
    let Some(&a) = inc.parties.first() else { return false };
    inc.info != u32::MAX && w.grapevine.get(inc.info).holders.iter().any(|k| matches!(k.how, Learned::Told { by } if by == a))
}

// ---------------------------------------------------------------------------
// Deciding
// ---------------------------------------------------------------------------

struct Profile {
    discipline: f32,
    empathy: f32,
    temper: f32,
    authority: f32,
    pressure: f32,
    favouritism: f32,
    culture: f32,
    guarded: f32,
    combative: f32,
    evidence: f32,
}

fn profile(w: &World, who: PersonId, id: u32) -> Profile {
    let inc = w.incidents.get(id).expect("incident");
    let s = w.people[who].staff;
    let (disc, mm) = if s.is_some() { (w.staff[s].attrs.f(StaffAttr::Discipline) / 20.0, w.staff[s].attrs.f(StaffAttr::ManManagement) / 20.0) } else { (0.5, 0.5) };
    let temper = 1.0 - consider::hid(w, who, Hidden::Temperament) / 20.0;
    let club = inc.club;
    let tenure = if s.is_some() { (w.staff[s].joined.days_until(w.date) as f32 / 365.0 / 4.0).min(1.0) } else { 0.5 };
    let backing = w.rooms.clubs.get(&club).map_or(0.5, |r| f32::from(r.backing) / 100.0);
    let board = if club.is_some() { f32::from(w.clubs[club].board.satisfaction) / 100.0 } else { 0.5 };
    let stature = if s.is_some() { f32::from(w.staff[s].reputation) / 10_000.0 } else { 0.5 };
    let party_fame = inc.players.iter().map(|&p| f32::from(w.players.cold[p].rep.current) / 10_000.0).fold(0.0f32, f32::max);
    let authority = (0.3 * tenure + 0.3 * backing + 0.2 * board + 0.2 * (stature - party_fame + 0.5)).clamp(0.0, 1.0);
    let warnings = if club.is_some() { f32::from(w.clubs[club].board.warnings) / 3.0 } else { 0.0 };
    let pressure = ((1.0 - board) * 0.6 + warnings * 0.4).clamp(0.0, 1.0);
    let prof = if s.is_some() { w.careers.managers.get(&s) } else { None };
    let favouritism = prof.map_or(0.3, |p| f32::from(p.favouritism) / 100.0);
    let (guarded, combative) = match prof.map(|p| p.media_style) {
        Some(MediaStyle::Guarded) => (1.0, 0.0),
        Some(MediaStyle::Combative) => (0.0, 1.0),
        Some(MediaStyle::Candid) => (0.2, 0.4),
        _ => (0.4, 0.2),
    };
    let culture = if club.is_some() { f32::from(w.culture.club(club).discipline) / 100.0 } else { 0.5 };
    let evidence = if inc.info != u32::MAX {
        w.grapevine.get(inc.info).knower(who).map_or(0.5, |k| {
            let f = match k.fidelity {
                Fidelity::Accurate => 1.0,
                Fidelity::Partial => 0.7,
                Fidelity::Exaggerated => 0.6,
                Fidelity::Outdated => 0.5,
                Fidelity::Garbled | Fidelity::Planted => 0.3,
            };
            f * f32::from(k.confidence) / 100.0
        })
    } else {
        1.0
    };
    Profile { discipline: disc, empathy: mm, temper, authority, pressure, favouritism, culture, guarded, combative, evidence }
}

/// A response with its utility and the three reasons that weigh most.
type Scored = (Response, f32, [(Reason, u8); 3]);

/// Options this incident allows, each with a utility and its reasons.
fn options(w: &World, who: PersonId, id: u32, pr: &Profile) -> SmallVec<[Scored; 10]> {
    let inc = w.incidents.get(id).expect("incident");
    let sev = f32::from(inc.severity) / 100.0;
    let r = |a: (Reason, f32), b: (Reason, f32), c: (Reason, f32)| -> [(Reason, u8); 3] {
        let q = |x: f32| (x.clamp(0.0, 1.0) * 100.0) as u8;
        [(a.0, q(a.1)), (b.0, q(b.1)), (c.0, q(c.1))]
    };
    let mut v: SmallVec<[Scored; 10]> = SmallVec::new();
    let s = w.people[who].staff;
    // How much the next match needs the people involved.
    let needed = inc
        .players
        .iter()
        .map(|&p| match w.players.cold[p].status {
            SquadStatus::Star => 1.0,
            SquadStatus::Important => 0.7,
            SquadStatus::Regular => 0.4,
            _ => 0.1,
        })
        .fold(0.0f32, f32::max);
    // The squad hierarchy: leaders are harder to punish.
    let rank = inc
        .players
        .iter()
        .map(|&p| match w.rooms.clubs.get(&inc.club).and_then(|r| r.standing.get(&p)) {
            Some(pw_world::dressing::Standing::Leader) => 1.0,
            Some(pw_world::dressing::Standing::Influential) => 0.6,
            _ => 0.2,
        })
        .fold(0.0f32, f32::max);
    let noise = |k: u64| w.roll(stream::RESPONSE, &[u64::from(id), u64::from(who.0), k]) * 0.15;
    if inc.kind.is_conduct() {
        v.push((
            Response::Fine,
            0.25 + pr.discipline * 0.6 + pr.culture * 0.3 + sev * 0.3 - rank * 0.3 + pr.evidence * 0.2 - 0.3 + noise(1),
            r((Reason::Discipline, pr.discipline), (Reason::ClubCulture, pr.culture), (Reason::Evidence, pr.evidence)),
        ));
        v.push((
            Response::Drop,
            0.1 + pr.discipline * 0.5 + pr.authority * 0.3 + sev * 0.3 - needed * 0.6 - rank * 0.2 + pr.evidence * 0.2 - 0.3 + noise(2),
            r((Reason::Discipline, pr.discipline), (Reason::Authority, pr.authority), (Reason::Results, 1.0 - needed)),
        ));
        v.push((
            Response::DemandApology,
            0.25 + pr.discipline * 0.3 + pr.empathy * 0.2 + sev * 0.2 - 0.2 + noise(3),
            r((Reason::Discipline, pr.discipline), (Reason::Empathy, pr.empathy), (Reason::Evidence, pr.evidence)),
        ));
        v.push((
            Response::Mediate,
            0.15 + pr.empathy * 0.7 + pr.authority * 0.2 - pr.temper * 0.2 - 0.2 + noise(4),
            r((Reason::Empathy, pr.empathy), (Reason::Authority, pr.authority), (Reason::Temper, 1.0 - pr.temper)),
        ));
        let captain_ok = inc.club.is_some() && {
            let cap = w.teams[w.clubs[inc.club].first_team()].captain;
            cap.is_some() && !inc.players.contains(&cap)
        };
        if captain_ok {
            v.push((
                Response::InvolveCaptain,
                0.15 + pr.empathy * 0.35 + (1.0 - pr.authority) * 0.4 - 0.2 + noise(5),
                r((Reason::Empathy, pr.empathy), (Reason::Authority, 1.0 - pr.authority), (Reason::Hierarchy, rank)),
            ));
        }
        v.push((
            Response::KeepPrivate,
            0.1 + pr.guarded * 0.4 + pr.empathy * 0.2 + sev * 0.2 - 0.2 + noise(6),
            r((Reason::MediaStyle, pr.guarded), (Reason::Empathy, pr.empathy), (Reason::Hierarchy, rank)),
        ));
        v.push((
            Response::Ignore,
            0.1 + pr.pressure * 0.5 + needed * 0.3 - pr.discipline * 0.4 - sev * 0.5 + noise(7),
            r((Reason::Results, needed), (Reason::BoardPressure, pr.pressure), (Reason::Avoidance, 1.0 - pr.authority)),
        ));
        v.push((
            Response::Delay,
            (1.0 - pr.authority) * 0.5 * (1.0 - pr.temper) + (1.0 - pr.evidence) * 0.4 - 0.15 + noise(8),
            r((Reason::Avoidance, 1.0 - pr.authority), (Reason::Evidence, 1.0 - pr.evidence), (Reason::Temper, 1.0 - pr.temper)),
        ));
        v.push((
            Response::Statement,
            pr.combative * 0.4 + pr.temper * 0.3 + sev * 0.2 - pr.empathy * 0.3 - 0.15 + noise(9),
            r((Reason::MediaStyle, pr.combative), (Reason::Temper, pr.temper), (Reason::Discipline, pr.discipline)),
        ));
        // A favourite among the parties can be shielded.
        if inc.players.len() == 2 && s.is_some() {
            let (p0, p1) = (inc.players[0], inc.players[1]);
            let (f0, f1) = (crate::managers::preference(w, s, p0) + consider::trust(w, who, inc.parties[0]), crate::managers::preference(w, s, p1) + consider::trust(w, who, inc.parties[1]));
            let (fav, gap) = if f0 >= f1 { (inc.parties[0], f0 - f1) } else { (inc.parties[1], f1 - f0) };
            if gap > 0.2 {
                v.push((
                    Response::Protect(fav),
                    pr.favouritism * gap * 1.5 + (1.0 - pr.evidence) * 0.2 - 0.1 + noise(10),
                    r((Reason::Favouritism, pr.favouritism), (Reason::Evidence, 1.0 - pr.evidence), (Reason::Hierarchy, rank)),
                ));
            }
        }
    } else if inc.kind.needs_leave() || matches!(inc.kind, IncidentKind::FamilyEmergency) {
        let trust = inc.parties.first().map_or(0.5, |&a| consider::trust(w, who, a));
        v.push((
            Response::GrantLeave,
            0.3 + pr.empathy * 0.6 + sev * 0.5 + trust * 0.3 - pr.pressure * 0.4 - needed * 0.3 + noise(11),
            r((Reason::Empathy, pr.empathy), (Reason::Hierarchy, trust), (Reason::Results, 1.0 - needed)),
        ));
        v.push((
            Response::RefuseLeave,
            0.2 + pr.pressure * 0.5 + needed * 0.5 + pr.discipline * 0.3 - pr.empathy * 0.4 - sev * 0.6 + noise(12),
            r((Reason::Results, needed), (Reason::BoardPressure, pr.pressure), (Reason::Discipline, pr.discipline)),
        ));
    } else if matches!(inc.kind, IncidentKind::OwnershipControversy | IncidentKind::SupporterUnrest) {
        let (fan, meddle) = w.governance.get(&inc.club).map_or((0.5, 0.5), |g| (f32::from(g.owner.fan_sensitivity) / 100.0, f32::from(g.owner.meddling) / 100.0));
        v.push((Response::Apologise, 0.2 + fan * 0.7 - meddle * 0.2 + noise(13), r((Reason::ClubCulture, fan), (Reason::Empathy, pr.empathy), (Reason::Authority, 1.0 - pr.authority))));
        v.push((Response::Defy, 0.2 + meddle * 0.5 + (1.0 - fan) * 0.5 + pr.temper * 0.2 + noise(14), r((Reason::Authority, meddle), (Reason::Temper, pr.temper), (Reason::ClubCulture, 1.0 - fan))));
        v.push((Response::Statement, 0.3 + noise(15), r((Reason::MediaStyle, 0.5), (Reason::Authority, pr.authority), (Reason::Evidence, 0.5))));
        v.push((Response::Inquiry, 0.2 + sev * 0.4 + pr.pressure * 0.3 + noise(16), r((Reason::BoardPressure, pr.pressure), (Reason::Evidence, sev), (Reason::Authority, pr.authority))));
    }
    v
}

fn decide(w: &World, who: PersonId, id: u32) -> (Response, [(Reason, u8); 3], SmallVec<[Response; 8]>) {
    let pr = profile(w, who, id);
    let mut opts = options(w, who, id, &pr);
    opts.sort_by(|a, b| b.1.total_cmp(&a.1));
    let list: SmallVec<[Response; 8]> = opts.iter().take(8).map(|x| x.0).collect();
    let best = opts.first().map_or((Response::Ignore, [(Reason::Avoidance, 50), (Reason::Evidence, 0), (Reason::Results, 0)]), |x| (x.0, x.2));
    (best.0, best.1, list)
}

// ---------------------------------------------------------------------------
// Applying
// ---------------------------------------------------------------------------

/// A human in authority chose a response.
pub fn apply_choice(w: &mut World, id: u32, who: PersonId, response: Response) {
    let pr = profile(w, who, id);
    let reasons = options(w, who, id, &pr).into_iter().find(|x| x.0 == response).map_or([(Reason::Authority, 50), (Reason::Evidence, 50), (Reason::Empathy, 50)], |x| x.2);
    apply(w, id, who, response, reasons);
}

pub fn apply(w: &mut World, id: u32, by: PersonId, response: Response, reasons: [(Reason, u8); 3]) {
    let today = w.date;
    let inc = w.incidents.get(id).expect("incident").clone();
    let sev = f32::from(inc.severity) / 100.0;
    let club = inc.club;
    let a = inc.parties.first().copied().unwrap_or(pw_core::PersonId::NONE);
    let b = inc.parties.get(1).copied().unwrap_or(pw_core::PersonId::NONE);
    let culprits: SmallVec<[PlayerId; 2]> =
        if inc.kind == IncidentKind::TrainingConfrontation && sev > 0.7 { inc.players.clone().into_iter().collect() } else { inc.players.iter().take(1).copied().collect() };
    let vis = match response {
        Response::KeepPrivate => Visibility::Between(by, a),
        Response::Statement | Response::Apologise | Response::Defy => Visibility::Public,
        _ if club.is_some() => Visibility::Club(club),
        _ => Visibility::Between(by, a),
    };
    let mut causes: Causes = Causes::new();
    causes.push(Cause::Event(inc.event));
    for &(reason, level) in reasons.iter().take(2) {
        causes.push(Cause::Fact(Fact::Disposition { person: by, reason, level }));
    }
    let ev = w.events.push_caused(today, vis, EventKind::IncidentResponse { incident: id, by, response }, causes);
    if let Some(x) = w.incidents.list.get_mut(id as usize) {
        x.responses.push(ResponseRecord { by, response, date: today, reasons, event: ev });
    }
    let compat = |w: &World, x: PersonId, y: PersonId| consider::compat(w, x, y);
    match response {
        Response::Fine => {
            for &p in &culprits {
                let wage = w.players.cold[p].contract.current_wage(today);
                let amount = (wage as f32 * (0.5 + sev)) as i64;
                w.events.push_caused(today, Visibility::Club(club), EventKind::Fined { player: p, club, amount }, pw_world::causes![Cause::Event(ev)]);
                let who = w.players.cold[p].person;
                if let Some(l) = w.lives.get_mut(who) {
                    l.finances.savings -= amount;
                }
                let c = compat(w, who, by);
                w.social.remember(who, by, MemoryKind::Fined, today, ev, false, 0.8 + sev * 0.4, c);
            }
            settle(w, &inc, 0.3);
        }
        Response::Drop => {
            for &p in culprits.iter().take(1) {
                w.incidents.dropped.insert(p, today.add_days(7));
                let who = w.players.cold[p].person;
                let c = compat(w, who, by);
                w.social.remember(who, by, MemoryKind::Dropped, today, ev, true, 1.0, c);
                let h = &mut w.players.hot[p];
                h.morale = h.morale.saturating_sub(8);
            }
            settle(w, &inc, 0.2);
        }
        Response::DemandApology => {
            // The instigator's own mind decides; a human is asked.
            if a.is_some() {
                if w.people[a].mind == MindKind::External {
                    ask(w, a, id, Ask::Apologise);
                } else {
                    let willing = consider::hid(w, a, Hidden::Sportsmanship) + consider::hid(w, a, Hidden::Professionalism) + consider::trust(w, a, by) * 10.0 > 22.0;
                    answer_ask(w, id, a, Ask::Apologise, willing);
                }
            }
        }
        Response::Mediate => {
            let ok = w.roll(stream::RESPONSE, &[u64::from(id), 0x3ed]) < 0.5 + profile(w, by, id).empathy * 0.4 - sev * 0.3;
            for &x in &inc.parties {
                if x != by {
                    let c = compat(w, x, by);
                    w.social.remember(x, by, if ok { MemoryKind::Mediated } else { MemoryKind::Argument }, today, ev, false, 0.7, c);
                }
            }
            if ok {
                settle(w, &inc, 0.6);
                if a.is_some() && b.is_some() {
                    let c = compat(w, a, b);
                    w.social.adjust(a, b, today, c, 3, 2, 1);
                    let c = compat(w, b, a);
                    w.social.adjust(b, a, today, c, 3, 2, 1);
                }
            }
        }
        Response::InvolveCaptain => {
            let cap = w.teams[w.clubs[club].first_team()].captain;
            if cap.is_some() {
                let cp = w.players.cold[cap].person;
                for &x in &inc.parties {
                    if x != by && x != cp {
                        let c = compat(w, x, cp);
                        w.social.remember(x, cp, MemoryKind::Mediated, today, ev, false, 0.8, c);
                    }
                }
                let c = compat(w, by, cp);
                w.social.adjust(by, cp, today, c, 2, 3, 3);
                settle(w, &inc, 0.45);
            }
        }
        Response::KeepPrivate => {
            if let Some(x) = w.incidents.list.get_mut(id as usize) {
                x.hushed = true;
            }
        }
        Response::Ignore => {
            if sev > 0.5 {
                for &x in &inc.witnesses {
                    let c = compat(w, x, by);
                    w.social.adjust(x, by, today, c, 0, -2, -2);
                }
            }
            if b.is_some() && b != by && sev > 0.6 {
                let c = compat(w, b, by);
                w.social.remember(b, by, MemoryKind::LetDown, today, ev, false, 0.6, c);
            }
        }
        Response::Protect(fav) => {
            let other = if fav == a { b } else { a };
            let c = compat(w, fav, by);
            w.social.remember(fav, by, MemoryKind::Protected, today, ev, false, 1.0, c);
            if other.is_some() {
                let c = compat(w, other, by);
                w.social.remember(other, by, MemoryKind::Blamed, today, ev, false, 1.0, c);
                w.incidents.add_tension(fav, other, 10);
                // The scapegoat's friends notice.
                let op = w.people[other].player;
                if op.is_some() {
                    let team = w.players.hot[op].team;
                    if team.is_some() {
                        let friends: Vec<PersonId> = w.teams[team].squad.iter().map(|&x| w.players.cold[x].person).filter(|&x| x != other && consider::affinity(w, x, other) > 0.3).collect();
                        for f in friends {
                            let c = compat(w, f, by);
                            w.social.adjust(f, by, today, c, -2, -3, -1);
                        }
                    }
                }
            }
        }
        Response::Delay => {
            w.incidents.deferred.push((id, by, today.add_days(7)));
        }
        Response::Statement => {
            if a.is_some() && a != by {
                crate::press::speak(w, by, a, Stance::Criticise);
            }
            settle(w, &inc, 0.1);
        }
        Response::GrantLeave => {
            if let Some(&p) = inc.players.first() {
                let days = (3.0 + sev * 11.0) as i32;
                w.incidents.away.insert(p, today.add_days(days));
            }
            if a.is_some() {
                let c = compat(w, a, by);
                w.social.remember(a, by, MemoryKind::Supported, today, ev, false, 1.2, c);
                if let Some(l) = w.lives.get_mut(a) {
                    l.stress = l.stress.saturating_sub(8);
                }
            }
            resolve(w, id);
        }
        Response::RefuseLeave => {
            if a.is_some() {
                let c = compat(w, a, by);
                w.social.remember(a, by, MemoryKind::Refused, today, ev, false, 0.8 + sev, c);
                if let Some(l) = w.lives.get_mut(a) {
                    l.stress = l.stress.saturating_add(10).min(100);
                }
                // Some go anyway — and that is a new problem.
                let family = w.lives.get(a).map_or(0.5, |l| f32::from(l.household.parents.closeness) / 100.0);
                let prof = consider::hid(w, a, Hidden::Professionalism) / 20.0;
                if w.people[a].mind == MindKind::Ai
                    && sev * 0.6 + family * 0.4 > prof + 0.2
                    && let Some(&p) = inc.players.first()
                {
                    w.incidents.away.insert(p, today.add_days(3));
                    let d = pw_world::incident::def(IncidentKind::LateArrival);
                    let c = crate::incidents::Ctx { a, pa: p, club, nation: inc.nation, ..crate::incidents::Ctx::default() };
                    crate::incidents::trigger(w, &d, c, SmallVec::new(), Some(id));
                }
            }
            resolve(w, id);
        }
        Response::Apologise => {
            w.clubs[club].fan_mood = (w.clubs[club].fan_mood + 5).min(100);
            w.media.nudge_image(by, 20);
            w.media.move_fans(club, by, 60, FanReason::Interview, today);
            resolve(w, id);
        }
        Response::Defy => {
            w.clubs[club].fan_mood = w.clubs[club].fan_mood.saturating_sub(4);
            w.media.nudge_image(by, -25);
            w.media.move_fans(club, by, -80, FanReason::Interview, today);
        }
        Response::Inquiry => {
            // Someone must answer for it: the board's patience with the manager thins.
            let b = &mut w.clubs[club].board;
            b.satisfaction = b.satisfaction.saturating_sub(5);
            resolve(w, id);
        }
    }
}

/// Tension between the parties eases by a share.
fn settle(w: &mut World, inc: &pw_world::incident::Incident, share: f32) {
    if inc.parties.len() >= 2 {
        let t = w.incidents.tension(inc.parties[0], inc.parties[1]);
        w.incidents.add_tension(inc.parties[0], inc.parties[1], -((f32::from(t) * share) as i16));
    }
    if share >= 0.3 {
        resolve(w, inc.id);
    }
}

fn resolve(w: &mut World, id: u32) {
    if let Some(x) = w.incidents.list.get_mut(id as usize) {
        x.resolved = true;
    }
}

// ---------------------------------------------------------------------------
// The people involved
// ---------------------------------------------------------------------------

/// A person with a personal emergency decides whether to ask for time
/// away; asking is telling the person in authority.
pub fn personal_request(w: &mut World, id: u32) {
    let Some(inc) = w.incidents.get(id) else { return };
    let Some(&a) = inc.parties.first() else { return };
    if inc.players.is_empty() {
        return;
    }
    if w.people[a].mind == MindKind::External {
        ask(w, a, id, Ask::RequestLeave);
        return;
    }
    let sev = f32::from(inc.severity) / 100.0;
    let family = w.lives.get(a).map_or(0.5, |l| f32::from(l.household.parents.closeness) / 100.0);
    let prof = consider::hid(w, a, Hidden::Professionalism) / 20.0;
    let asks = sev * 0.7 + family * 0.3 + if inc.kind == IncidentKind::FamilyEmergency { 0.2 } else { 0.0 } > prof * 0.6 + 0.2;
    answer_ask(w, id, a, Ask::RequestLeave, asks);
}

/// New parents ask for leave the same way.
pub fn birth_leave(w: &mut World, parent: PersonId, pregnancy: u32) {
    let p = w.people[parent].player;
    if p.is_none() {
        return;
    }
    let club = w.club_of_person(parent);
    let d = pw_world::incident::def(IncidentKind::FamilyEmergency);
    let c = crate::incidents::Ctx { a: parent, pa: p, club, nation: w.lives.get(parent).map_or(pw_core::NationId::NONE, |l| l.home), ..crate::incidents::Ctx::default() };
    crate::incidents::trigger(w, &d, c, SmallVec::new(), Some(pregnancy));
}

fn ask(w: &mut World, who: PersonId, id: u32, ask: Ask) {
    let today = w.date;
    let kind = DecisionKind::IncidentAsk { incident: id, ask };
    let options = kind.simple_options();
    let player = w.people[who].player;
    w.decisions.push(Decision { person: who, player, kind, options, created: today, deadline: today.add_days(2), default: 0, answer: None, resolved: false });
}

/// The answer of the person asked (their mind, or a human).
pub fn answer_ask(w: &mut World, id: u32, who: PersonId, ask: Ask, yes: bool) {
    let today = w.date;
    let Some(inc) = w.incidents.get(id).cloned() else { return };
    match ask {
        Ask::RequestLeave => {
            if !yes {
                return;
            }
            // Asking = telling the person in authority.
            let auth = authority(w, id);
            if auth.is_some() && inc.info != u32::MAX {
                crate::grapevine::tell(w, inc.info, who, auth, Fidelity::Accurate, Motive::Duty, 100);
                on_learn(w, auth, id);
            }
        }
        Ask::Apologise => {
            let other = inc.parties.iter().copied().find(|&x| x != who).unwrap_or(pw_core::PersonId::NONE);
            if yes {
                if other.is_some() {
                    let c = consider::compat(w, other, who);
                    w.social.remember(other, who, MemoryKind::Apologised, today, inc.event, false, 0.8, c);
                    w.incidents.add_tension(who, other, -25);
                }
                resolve(w, id);
            } else {
                let auth = authority(w, id);
                if auth.is_some() {
                    let c = consider::compat(w, auth, who);
                    w.social.remember(auth, who, MemoryKind::PoorAttitude, today, inc.event, false, 1.0, c);
                }
                if other.is_some() {
                    w.incidents.add_tension(who, other, 10);
                }
            }
        }
    }
}

/// Club crises: the chairman (or owner) responds.
pub fn club_response(w: &mut World, id: u32) {
    let auth = authority(w, id);
    if auth.is_some() {
        on_learn(w, auth, id);
    }
}

/// Deferred decisions come due; some captains step in on their own.
pub fn deferred(w: &mut World) {
    let today = w.date;
    let due: Vec<(u32, PersonId)> = w.incidents.deferred.iter().filter(|x| x.2 <= today).map(|x| (x.0, x.1)).collect();
    w.incidents.deferred.retain(|x| x.2 > today);
    for (id, who) in due {
        let Some(inc) = w.incidents.get(id) else { continue };
        if inc.resolved {
            continue;
        }
        // A follow-up incident while the decision was put off forces the issue.
        let escalated = inc.led_to.is_some();
        let (mut choice, reasons, _) = decide(w, who, id);
        if choice == Response::Delay {
            choice = if escalated { Response::Fine } else { Response::Ignore };
        }
        apply(w, id, who, choice, reasons);
    }
    captains_step_in(w);
}

/// A respected captain may settle bad blood nobody in authority dealt with.
fn captains_step_in(w: &mut World) {
    let today = w.date;
    if today.weekday() != pw_core::Weekday::Tue {
        return;
    }
    let pairs: Vec<((PersonId, PersonId), u8)> = w.incidents.tension.iter().filter(|&(_, &v)| v >= 50).map(|(&k, &v)| (k, v)).collect();
    for ((a, b), _) in pairs {
        let (pa, pb) = (w.people[a].player, w.people[b].player);
        if pa.is_none() || pb.is_none() || w.players.hot[pa].team != w.players.hot[pb].team || w.players.hot[pa].team.is_none() {
            continue;
        }
        let team = w.players.hot[pa].team;
        let cap = w.teams[team].captain;
        if cap.is_none() || cap == pa || cap == pb {
            continue;
        }
        let lead = w.players.cold[cap].attrs.get(pw_core::Attr::Leadership) / 20.0;
        let cp = w.players.cold[cap].person;
        let roll = w.roll(stream::RESPONSE, &[u64::from(a.0), u64::from(b.0), pw_core::rng::period::week(today)]);
        if roll < lead * 0.3 {
            for x in [a, b] {
                let c = consider::compat(w, x, cp);
                w.social.remember(x, cp, MemoryKind::Mediated, today, pw_core::EventId::NONE, false, 0.7, c);
            }
            w.incidents.add_tension(a, b, -30);
            let club = w.players.hot[pa].club;
            w.events.push(today, Visibility::Club(club), EventKind::CaptainMediated { captain: cp, a, b });
        }
    }
}
