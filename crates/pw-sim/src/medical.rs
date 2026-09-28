//! Injury cases (05 §4): diagnosis with uncertainty, treatment choices,
//! setbacks, rushed returns, fragility left behind and chronic conditions.
//!
//! `health` decides *that* someone is hurt; this module decides what that
//! means over the following weeks and years. Everything it leaves behind is
//! read elsewhere: `health::hazard_mult` (fragility), selection (managed
//! players need rest between games), valuations and media (histories), and
//! the player's own mind (treatment choices, pushing to return).

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{ClubId, Hidden, PlayerId, StaffAttr};
use pw_data::BodyRegion;
use pw_world::decision::{Decision, DecisionKind, MindKind};
use pw_world::event::{EventKind, Visibility};
use pw_world::medical::{Case, Chronic, Fragility, Treatment};
use pw_world::{PlayerStatus, SquadStatus, StaffRole, World};

use crate::consider;

/// Regions where an operation is a real option.
fn surgical(r: BodyRegion) -> bool {
    matches!(r, BodyRegion::Knee | BodyRegion::Ankle | BodyRegion::Shoulder | BodyRegion::Groin | BodyRegion::Foot | BodyRegion::Back)
}

/// Quality of the medical room a player is treated in, 1–20.
pub fn medical_quality(w: &World, club: ClubId) -> f32 {
    if club.is_none() {
        return 6.0;
    }
    let c = &w.clubs[club];
    let physio = c
        .staff
        .iter()
        .filter(|&&s| matches!(w.staff[s].role, StaffRole::Physio | StaffRole::SportsScientist))
        .map(|&s| w.staff[s].attrs.f(StaffAttr::Physiotherapy).max(w.staff[s].attrs.f(StaffAttr::SportsScience)))
        .fold(5.0f32, f32::max);
    0.6 * physio + 0.4 * f32::from(c.facilities.medical)
}

/// Open a case for a fresh injury. Adjusts the true duration for recurrence
/// and returns the medical team's estimate (what the world is told).
pub fn on_injury(w: &mut World, p: PlayerId, injury: u16, days: u16) -> u16 {
    let today = w.date;
    let def = &w.data.injuries[usize::from(injury - 1)];
    let region = def.region;
    let slot = region.wear_slot().map_or(u8::MAX, |s| s as u8);
    let long_injury = def.days[2] >= 60;
    let club = w.players.hot[p].club;
    let q = medical_quality(w, club);

    // Recurrence: the same region within a year heals slower.
    let recurrence = slot != u8::MAX && w.medical.history_of(p).iter().any(|c| c.region == slot && c.date.days_until(today) <= 365);
    let fragile = w.medical.fragile.get(&p).and_then(|v| v.iter().find(|f| f.region == slot)).map_or(0.0, |f| f.level);
    let mut truth = f32::from(days) * if recurrence { 1.3 } else { 1.0 } * (1.0 + 0.2 * fragile);
    // Illness in someone with a systemic condition drags on.
    if slot == u8::MAX && w.medical.chronic.get(&p).is_some_and(|v| v.contains(&Chronic::Systemic)) {
        truth *= 1.4;
    }
    let truth = truth.round().clamp(1.0, 600.0) as u16;
    {
        let h = &mut w.players.hot[p];
        h.injury_days = truth;
        h.injury_total = truth;
    }

    // Diagnosis: a good medical room is close; a poor one guesses.
    let sigma = (0.4 - 0.016 * q).clamp(0.05, 0.4);
    let noise = pw_core::rng::noise(&[w.seed, stream::HEALTH, u64::from(p.0), today.0 as u64, 0xd1a]);
    let estimate = (f32::from(truth) * (1.0 + sigma * noise)).round().clamp(1.0, 700.0) as u16;
    let certainty = (100.0 - sigma * 200.0).clamp(10.0, 95.0) as u8;
    let on_duty = w.intl.duty.contains(&p);
    let case =
        Case { player: p, injury, region: slot, club, date: today, estimate, certainty, treatment: Treatment::Conservative, rushed: false, setbacks: 0, recurrence, on_duty, actual: 0, closed: None };
    w.medical.open.insert(p, case);
    let vis = if club.is_some() { Visibility::Club(club) } else { Visibility::Public };
    w.events.push(today, vis, EventKind::Diagnosed { player: p, injury, estimate, treatment: Treatment::Conservative });

    // A serious injury in an operable region: surgery or rehabilitation.
    if long_injury && surgical(region) && truth >= 42 {
        offer_treatment(w, p, truth);
    }
    estimate
}

/// Surgery takes longer but leaves the tissue sounder; rehabilitation is
/// quicker on paper but setbacks and re-injury are likelier. The player
/// decides (their own mind, or at the keyboard), advised by the club.
fn offer_treatment(w: &mut World, p: PlayerId, truth: u16) {
    let today = w.date;
    let who = w.players.cold[p].person;
    let surgery_days = (f32::from(truth) * 1.15).round() as u16;
    let rehab_days = truth;
    let prefers_surgery = ai_prefers_surgery(w, p);
    if w.people[who].mind != MindKind::External {
        answer_treatment(w, p, prefers_surgery);
        return;
    }
    let kind = DecisionKind::Treatment { surgery_days, rehab_days };
    let options = kind.simple_options();
    w.decisions.push(Decision { person: who, player: p, kind, options, created: today, deadline: today.add_days(3), default: if prefers_surgery { 0 } else { 1 }, answer: None, resolved: false });
}

fn ai_prefers_surgery(w: &World, p: PlayerId) -> bool {
    let who = w.players.cold[p].person;
    let prof = consider::hid(w, who, Hidden::Professionalism) / 20.0;
    let age = w.age_years(p);
    let years_left = consider::contract_days_left(w, p).max(0) as f32 / 365.0;
    let club_q = medical_quality(w, w.players.hot[p].club) / 20.0;
    // The long view: young, professional, secure — fix it properly. Out of
    // contract or near the end of a career, get back quickly.
    let score = prof * 0.5 + (30.0 - age) / 20.0 + (years_left / 3.0).min(1.0) * 0.3 + club_q * 0.2 - 0.6;
    score + (crate::decisions::coin(w, p, 0x5e6) - 0.5) * 0.2 > 0.0
}

/// Apply a treatment choice to an open case.
pub fn answer_treatment(w: &mut World, p: PlayerId, surgery: bool) {
    let Some(case) = w.medical.open.get_mut(&p) else { return };
    if !surgery {
        case.treatment = Treatment::Conservative;
        return;
    }
    case.treatment = Treatment::Surgery;
    case.certainty = case.certainty.max(85);
    let h = &mut w.players.hot[p];
    let extra = (f32::from(h.injury_days) * 0.15).round() as u16;
    h.injury_days = h.injury_days.saturating_add(extra);
    h.injury_total = h.injury_total.saturating_add(extra);
    case.estimate = case.estimate.saturating_add(extra);
    let (club, injury, estimate) = (case.club, case.injury, case.estimate);
    let vis = if club.is_some() { Visibility::Club(club) } else { Visibility::Public };
    w.events.push(w.date, vis, EventKind::Diagnosed { player: p, injury, estimate, treatment: Treatment::Surgery });
}

/// Weekly: estimates firm up, setbacks happen, some are rushed back, cases
/// close and leave their mark.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let open: Vec<PlayerId> = w.medical.open.keys().copied().collect();
    for p in open {
        let h = &w.players.hot[p];
        if h.injury == 0 || h.status == PlayerStatus::Retired {
            close(w, p);
            continue;
        }
        let club = h.club;
        let q = medical_quality(w, club);
        let remaining = h.injury_days;
        let total = h.injury_total.max(1);
        let mut rng = Rng::keyed(&[w.seed, stream::HEALTH, u64::from(p.0), today.0 as u64, 0x3ed]);

        // Estimates converge on the truth as recovery goes on.
        if let Some(c) = w.medical.open.get_mut(&p) {
            let elapsed = c.date.days_until(today).max(0) as u16;
            let est_left = c.estimate.saturating_sub(elapsed);
            let blended = (f32::from(est_left) * 0.5 + f32::from(remaining) * 0.5).round() as u16;
            c.estimate = elapsed + blended;
            c.certainty = (c.certainty + 8).min(98);
        }

        // Setbacks.
        let case = w.medical.open[&p];
        let conservative = if case.treatment == Treatment::Conservative { 1.3 } else { 0.7 };
        let setback_p = 0.02 * (1.0 + f32::from(u8::from(case.recurrence))) * conservative * (1.25 - q / 20.0 * 0.5) * if total >= 28 { 1.0 } else { 0.3 };
        if rng.chance(setback_p) {
            let add = rng.range_i32(7, 21) as u16;
            let h = &mut w.players.hot[p];
            h.injury_days = h.injury_days.saturating_add(add);
            h.injury_total = h.injury_total.saturating_add(add);
            if let Some(c) = w.medical.open.get_mut(&p) {
                c.setbacks += 1;
                c.estimate = c.estimate.saturating_add(add);
            }
            let vis = if club.is_some() { Visibility::Club(club) } else { Visibility::Public };
            w.events.push(today, vis, EventKind::InjurySetback { player: p, days: add });
            continue;
        }

        // Rushing back: late in recovery, when the club needs the player and
        // the player is willing, a less careful medical room signs them off.
        if !case.rushed && f32::from(remaining) <= f32::from(total) * 0.3 && remaining >= 4 && club.is_some() {
            let status = w.players.cold[p].status;
            let needed = matches!(status, SquadStatus::Star | SquadStatus::Important);
            let pressure = w.clubs[club].board.satisfaction < 40 || w.clubs[club].manager.get().is_some_and(|m| w.staff[m].philosophy.archetype == pw_world::Archetype::Pragmatist);
            let who = w.players.cold[p].person;
            let willing = match w.medical.willing_to_rush.get(&p) {
                Some(&b) => b,
                None => w.people[who].mind == MindKind::Ai && consider::hid(w, who, Hidden::Ambition) + consider::hid(w, who, Hidden::Pressure) > 26.0,
            };
            let careful = q / 20.0;
            let roll = (hash_key(&[w.seed, u64::from(p.0), today.0 as u64, 0x7a5]) % 1000) as f32 / 1000.0;
            if needed && willing && (pressure || roll > careful) && roll < 0.6 {
                let h = &mut w.players.hot[p];
                h.injury_days = 0;
                h.injury = 0;
                h.injury_total = 0;
                h.condition = h.condition.min(75);
                h.sharpness = h.sharpness.min(40);
                if let Some(c) = w.medical.open.get_mut(&p) {
                    c.rushed = true;
                }
                w.events.push(today, Visibility::Club(club), EventKind::RushedBack { player: p });
                close(w, p);
            }
        }
    }
}

fn close(w: &mut World, p: PlayerId) {
    let today = w.date;
    let Some(mut case) = w.medical.open.remove(&p) else { return };
    case.actual = case.date.days_until(today).max(0) as u16;
    case.closed = Some(today);
    // What the injury leaves behind.
    if case.region != u8::MAX {
        let gain = f32::from(case.actual) / 120.0 * if case.treatment == Treatment::Surgery { 0.5 } else { 1.0 } + if case.rushed { 0.5 } else { 0.0 } + if case.recurrence { 0.2 } else { 0.0 };
        if gain > 0.02 {
            let v = w.medical.fragile.entry(p).or_default();
            if let Some(f) = v.iter_mut().find(|f| f.region == case.region) {
                f.level = (f.level + gain).min(1.5);
                f.since = today;
            } else {
                v.push(Fragility { region: case.region, level: gain.min(1.5), since: today });
            }
        }
        // Worn, repeatedly injured regions become something to manage.
        let wear = w.players.cold[p].wear.get(usize::from(case.region)).copied().unwrap_or(0);
        let repeats = w.medical.history_of(p).iter().filter(|c| c.region == case.region).count();
        let already = w.medical.needs_managing(p);
        if !already && (wear >= 70 || repeats >= 3) {
            w.medical.chronic.entry(p).or_default().push(Chronic::Managed { region: case.region });
            w.events.push(today, Visibility::Public, EventKind::ChronicCondition { player: p });
        }
    } else if case.actual >= 30 && !w.medical.chronic.get(&p).is_some_and(|v| v.contains(&Chronic::Systemic)) {
        let roll = (hash_key(&[w.seed, u64::from(p.0), today.0 as u64, 0x5c]) % 100) as u8;
        if roll < 15 {
            w.medical.chronic.entry(p).or_default().push(Chronic::Systemic);
            w.events.push(today, Visibility::Public, EventKind::ChronicCondition { player: p });
        }
    }
    let hist = w.medical.history.entry(p).or_default();
    hist.push(case);
    if hist.len() > 12 {
        hist.remove(0);
    }
}

/// Monthly: fragility fades with time and good habits.
pub fn monthly(w: &mut World) {
    for (_, v) in w.medical.fragile.iter_mut() {
        for f in v.iter_mut() {
            f.level *= 0.93;
        }
        v.retain(|f| f.level >= 0.05);
    }
    w.medical.fragile.retain(|_, v| !v.is_empty());
}

/// The player's own stance on playing through (intent).
pub fn set_willing(w: &mut World, p: PlayerId, willing: bool) {
    w.medical.willing_to_rush.insert(p, willing);
}

/// Record-based risk for valuations and media: 1.0 = clean bill of health.
pub fn history_risk(w: &World, p: PlayerId) -> f32 {
    let serious = w.medical.serious_recent(p, w.date, 730) as f32;
    w.medical.fragility(p) + serious * 0.2
}
