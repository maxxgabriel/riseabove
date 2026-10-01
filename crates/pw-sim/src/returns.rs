//! Returning from injury before the body is ready (Slice 1).
//!
//! Before a match, a manager whose injured player is nearly back decides on
//! what he *believes*, not on the truth:
//!
//! - the medical room's belief is its own estimate (converging on the truth
//!   as the case runs, sharper in a good room);
//! - the player reports what he feels (pain tracks how much is truly left),
//!   coloured by ambition and pressure against professionalism;
//! - the manager blends the two by how far he trusts his medical people,
//!   wants the player by the stakes, and has a learned tendency to rush.
//!
//! A club's medical authority (an owner who does not meddle) can veto.
//! A rush is a `Ruling`: every stance, the belief error and the accepted risk
//! are kept. The chance then resolves on the true remaining fraction: the
//! player comes back part-fit and, for a window, carries a raised injury
//! hazard that the ordinary injury systems act on. Recurrence in the same
//! region within the window fails the ruling; an uneventful window vindicates
//! it. People learn from *outcome* only, whether or not the call was sound.

use pw_core::{ClubId, Hidden, PersonId, PlayerId, StaffId, TeamId};
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::medical::{ReturnStage, RushedReturn};
use pw_world::ruling::{Outcome, Ruling, RulingKind, Stance, StanceRole};
use pw_world::{Archetype, Fixture, MemoryKind, MindKind, SquadStatus, StaffRole, World};
use smallvec::SmallVec;

use crate::consider;

/// Days after a rush during which a recurrence counts against the decision.
pub const WINDOW_DAYS: i32 = 28;

/// Extra injury hazard while a rushed player is inside the window.
pub fn hazard_factor(w: &World, p: PlayerId) -> f32 {
    if w.ext.medical.rushed.is_empty() {
        return 1.0;
    }
    w.ext.medical.rushed.get(&p).map_or(1.0, |r| if r.from.days_until(w.date) <= WINDOW_DAYS { 1.0 + 4.0 * r.left } else { 1.0 })
}

/// Daily, before selection: vindicate expired rushes, then let managers of
/// teams playing today consider their nearly-fit players.
pub fn pre_match(w: &mut World) {
    resolve_expired(w);
    let fixtures: Vec<Fixture> = w.fixtures.on(w.date).iter().map(|&f| w.fixtures.get(f).clone()).filter(|f| f.score.is_none()).collect();
    for fx in fixtures {
        let stakes = (crate::matchday::importance(w, fx.comp, fx.decisive) + crate::culture::stakes(w, &fx)).min(1.0);
        for t in [fx.home, fx.away] {
            review_team(w, t, stakes);
        }
    }
}

fn status_weight(s: SquadStatus) -> f32 {
    match s {
        SquadStatus::Star => 1.0,
        SquadStatus::Important => 0.85,
        SquadStatus::Regular => 0.6,
        _ => 0.3,
    }
}

fn medical_lead(w: &World, club: ClubId) -> Option<PersonId> {
    let c = &w.clubs[club];
    let pick = |role| c.staff.iter().copied().find(|&s| w.staff[s].role == role);
    pick(StaffRole::Physio).or_else(|| pick(StaffRole::SportsScientist)).map(|s| w.staff[s].person)
}

fn review_team(w: &mut World, team: TeamId, stakes: f32) {
    let club = w.teams[team].club;
    let Some(m) = w.clubs[club].manager.get() else { return };
    let candidates: Vec<PlayerId> = w.teams[team]
        .squad
        .iter()
        .copied()
        .filter(|&p| {
            let h = &w.players.hot[p];
            h.injury != 0 && h.injury_days >= 3 && h.club == club && ReturnStage::of(f32::from(h.injury_days) / f32::from(h.injury_total.max(1))) >= ReturnStage::FullTraining && w.medical.open.get(&p).is_some_and(|c| !c.rushed)
        })
        .collect();
    for p in candidates {
        consider_rush(w, club, m, p, stakes);
    }
}

/// One manager's decision on one nearly-fit player, for a match of the given stakes (0–1).
pub fn consider_rush(w: &mut World, club: ClubId, m: StaffId, p: PlayerId, stakes: f32) {
    let today = w.date;
    let (case, h) = (w.medical.open[&p], w.players.hot[p]);
    let who = w.players.cold[p].person;
    let mp = w.staff[m].person;
    let total = f32::from(h.injury_total.max(1));
    let truth_left = f32::from(h.injury_days) / total;

    // What the medical room believes is left, as a fraction of the case.
    let elapsed = case.date.days_until(today).max(0) as u16;
    let med_left = f32::from(case.estimate.saturating_sub(elapsed)) / f32::from(case.estimate.max(1));
    // What the player reports: he feels the truth, shaded by wanting to play.
    let amb = consider::hid(w, who, Hidden::Ambition);
    let pres = consider::hid(w, who, Hidden::Pressure);
    let prof = consider::hid(w, who, Hidden::Professionalism);
    let optimism = (0.10 + 0.02 * (amb + pres - 20.0) - 0.015 * (prof - 10.0)).clamp(0.0, 0.6);
    let reported_left = truth_left * (1.0 - optimism);

    // The manager weighs the two by how far he trusts his medical people.
    let lead = medical_lead(w, club);
    let trust_med = lead.map_or(0.5, |l| consider::trust(w, mp, l));
    let weight_med = 0.35 + 0.55 * trust_med;
    let believed = med_left * weight_med + reported_left * (1.0 - weight_med);

    // How much he wants him: the stakes, the player's standing, his temperament, the pressure on him.
    let archetype = match w.staff[m].philosophy.archetype {
        Archetype::Pragmatist => 1.15,
        Archetype::Loyalist => 1.0,
        Archetype::Developer => 0.9,
        Archetype::Rotator => 0.8,
    };
    let pressure = w.clubs[club].board.satisfaction < 40;
    let bias = f32::from(w.ext.medical.rush_bias.get(&m).copied().unwrap_or(0)) / 100.0;
    let want = (status_weight(w.players.cold[p].status) * stakes * archetype * (1.0 + if pressure { 0.3 } else { 0.0 }) * (1.0 + bias)).clamp(0.0, 1.0);
    if believed > 0.08 + 0.27 * want {
        return;
    }

    // The player has to be willing; a human's default is to say no unless he has said otherwise.
    let willing = match w.medical.willing_to_rush.get(&p) {
        Some(&b) => b,
        None => w.people[who].mind == MindKind::Ai && amb + pres > 26.0,
    };
    if !willing {
        // Someone who has never said where he stands is asked, once in a case: the manager wants him and the medical room has not
        // cleared him. His answer is his stance on playing through (`medical::set_willing`).
        let undecided = !w.medical.willing_to_rush.contains_key(&p) && w.people[who].mind != MindKind::Ai;
        let asked = w.events.latest_where(today, case.date.days_until(today).max(0) + 1, |e| matches!(e.kind, EventKind::AskedIfReady { player, .. } if player == p)).is_some();
        if undecided && !asked {
            w.events.push(today, Visibility::Between(mp, who), EventKind::AskedIfReady { player: p, manager: mp });
        }
        return;
    }

    // Medical authority: an owner who does not meddle lets the medical room stop it when it is clearly too soon.
    let authority = 1.0 - w.governance.get(&club).map_or(0.5, |g| f32::from(g.owner.meddling) / 100.0);
    let vetoed = authority >= 0.5 && med_left > 0.22 && case.certainty >= 60;

    let lead_stance = lead.map(|l| Stance { who: l, role: StanceRole::Medical, believed_pct: pct(med_left), backing: if med_left > 0.15 { -60 } else { 20 }, authority: authority >= 0.5 });
    let mut stances: SmallVec<[Stance; 4]> = SmallVec::new();
    stances.push(Stance { who: mp, role: StanceRole::Manager, believed_pct: pct(believed), backing: 80, authority: !vetoed });
    stances.extend(lead_stance);
    stances.push(Stance { who, role: StanceRole::Player, believed_pct: pct(reported_left), backing: 70, authority: false });
    let mut causes: Causes = Causes::new();
    if pressure {
        causes.push(Cause::Fact(Fact::BoardPressure { club, warnings: w.clubs[club].board.warnings }));
    }

    if vetoed {
        // Only a contested, high-stakes veto is worth remembering.
        if want > 0.6 {
            record(w, club, p, mp, stances, truth_left, want, Outcome::Vetoed, causes, false);
        }
        return;
    }

    let (ruling, event) = record(w, club, p, mp, stances, truth_left, want, Outcome::Pending, causes, true);
    let region = case.region;
    // Back on the pitch part-fit: condition and sharpness follow how much was truly left.
    {
        let hm = &mut w.players.hot[p];
        hm.injury = 0;
        hm.injury_days = 0;
        hm.injury_total = 0;
        hm.condition = hm.condition.min((90.0 - 80.0 * truth_left).max(30.0) as u8);
        hm.sharpness = hm.sharpness.min((45.0 - 30.0 * truth_left).max(10.0) as u8);
    }
    if let Some(c) = w.medical.open.get_mut(&p) {
        c.rushed = true;
    }
    w.ext.medical.rushed.insert(p, RushedReturn { ruling, event, from: today, left: truth_left, region, manager: m });
    let vis = Visibility::Club(club);
    w.events.push_caused(today, vis, EventKind::RushedBack { player: p }, pw_world::causes![Cause::Event(event)]);
    crate::medical::close(w, p);
}

fn pct(x: f32) -> u8 {
    (x * 100.0).round().clamp(0.0, 100.0) as u8
}

#[allow(clippy::too_many_arguments)]
fn record(w: &mut World, club: ClubId, p: PlayerId, mp: PersonId, stances: SmallVec<[Stance; 4]>, truth_left: f32, want: f32, outcome: Outcome, causes: Causes, _rushed: bool) -> (u32, pw_core::EventId) {
    let today = w.date;
    let id = w.ext.decisions.add(Ruling {
        id: 0,
        kind: RulingKind::ReturnFromInjury,
        date: today,
        club,
        subject: p,
        about: pw_core::PersonId::NONE,
        liability: 0,
        decider: mp,
        stances,
        true_pct: pct(truth_left),
        want: pct(want),
        outcome,
        resolved: if outcome == Outcome::Pending { None } else { Some(today) },
        event: pw_core::EventId::NONE,
    });
    let event = w.events.push_caused(today, Visibility::Club(club), EventKind::Ruling { ruling: id }, causes);
    if let Some(r) = w.ext.decisions.get_mut(id) {
        r.event = event;
    }
    (id, event)
}

/// A new injury: if it is a recurrence in the region of a recent rush, the ruling failed.
pub fn on_new_injury(w: &mut World, p: PlayerId, region: u8) {
    let Some(r) = w.ext.medical.rushed.get(&p).copied() else { return };
    if r.from.days_until(w.date) > WINDOW_DAYS || region != r.region {
        return;
    }
    if let Some(c) = w.medical.open.get_mut(&p) {
        c.recurrence = true;
    }
    conclude(w, p, r, Outcome::Failed);
}

fn resolve_expired(w: &mut World) {
    if w.ext.medical.rushed.is_empty() {
        return;
    }
    let today = w.date;
    let mut due: Vec<(PlayerId, RushedReturn)> = w.ext.medical.rushed.iter().filter(|(_, r)| r.from.days_until(today) > WINDOW_DAYS).map(|(&p, &r)| (p, r)).collect();
    due.sort_by_key(|(p, _)| *p);
    for (p, r) in due {
        conclude(w, p, r, Outcome::Held);
    }
}

/// Close a rush: record the outcome, then let each person learn from it (from the outcome alone).
fn conclude(w: &mut World, p: PlayerId, r: RushedReturn, outcome: Outcome) {
    let today = w.date;
    w.ext.medical.rushed.remove(&p);
    let Some(ruling) = w.ext.decisions.get_mut(r.ruling) else { return };
    ruling.outcome = outcome;
    ruling.resolved = Some(today);
    let stances = ruling.stances.clone();
    let mp = ruling.decider;
    let who = w.players.cold[p].person;
    let bias = w.ext.medical.rush_bias.entry(r.manager).or_default();
    *bias = match outcome {
        Outcome::Held => (*bias + 5).min(30),
        _ => (*bias - 10).max(-40),
    };
    let against: Vec<PersonId> = stances.iter().filter(|s| s.role == StanceRole::Medical && s.backing < 0).map(|s| s.who).collect();
    let compat = consider::compat(w, who, mp);
    match outcome {
        Outcome::Failed => {
            w.social.remember(who, mp, MemoryKind::LetDown, today, r.event, false, 1.0, compat);
            for m in against {
                let c = consider::compat(w, m, mp);
                w.social.remember(m, mp, MemoryKind::LetDown, today, r.event, false, 0.8, c);
            }
        }
        Outcome::Held => {
            // He was right, so the over-cautious medical room was wrong, in his memory.
            for m in against {
                let c = consider::compat(w, mp, m);
                w.social.remember(mp, m, MemoryKind::Blamed, today, r.event, false, 0.5, c);
            }
        }
        _ => {}
    }
}
