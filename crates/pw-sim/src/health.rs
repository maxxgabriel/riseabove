//! Load, condition, fatigue and injuries (05). Daily, parallel over players.

use pw_core::rng::{Rng, stream};
use pw_core::{Attr, ClubId, Date, Hidden, PlayerId, TeamId};
use pw_data::{InjuryDef, Mechanism};
use pw_world::event::{EventKind, Visibility};
use pw_world::{PlayerStatus, World};
use rayon::prelude::*;

/// What a team does today, derived from its fixtures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DayKind {
    Match,
    AfterMatch,
    BeforeMatch,
    Training,
    Rest,
    Offseason,
}

impl DayKind {
    /// Training load in arbitrary units (matches add their own load).
    fn load(self) -> f32 {
        match self {
            DayKind::Match => 0.0,
            DayKind::AfterMatch => 120.0,
            DayKind::BeforeMatch => 260.0,
            DayKind::Training => 470.0,
            DayKind::Rest => 0.0,
            DayKind::Offseason => 90.0,
        }
    }

    fn recovery(self) -> f32 {
        match self {
            DayKind::Rest | DayKind::Offseason => 1.25,
            DayKind::AfterMatch => 1.15,
            DayKind::BeforeMatch => 1.0,
            DayKind::Training | DayKind::Match => 0.85,
        }
    }
}

pub fn team_days(w: &World, today: Date) -> Vec<DayKind> {
    let mut days = vec![DayKind::Training; w.teams.len()];
    for (tid, _) in w.teams.iter_enumerated() {
        let club = &w.clubs[w.teams[tid].club];
        let season = &w.nations[club.nation].season;
        if today > season.end || today < season.start.add_days(-35) {
            days[tid.0 as usize] = DayKind::Offseason;
        } else if today.weekday() == pw_core::Weekday::Sun {
            days[tid.0 as usize] = DayKind::Rest;
        }
    }
    let mut mark = |d: Date, kind: DayKind| {
        for &f in w.fixtures.on(d) {
            let fx = w.fixtures.get(f);
            for t in [fx.home, fx.away] {
                let slot = &mut days[t.0 as usize];
                if *slot != DayKind::Match {
                    *slot = kind;
                }
            }
        }
    };
    mark(today.add_days(-1), DayKind::AfterMatch);
    mark(today.add_days(1), DayKind::BeforeMatch);
    mark(today, DayKind::Match);
    days
}

/// Multiplier on injury hazard from proneness, load, fatigue, body wear and age (17 §3).
pub fn hazard_mult(w: &World, p: PlayerId) -> f32 {
    let h = &w.players.hot[p];
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    let t = &w.data.tuning.health;
    let acwr = h.acwr();
    let acwr_mult = if acwr > t.acwr_safe_high {
        1.0 + 1.5 * (acwr - t.acwr_safe_high)
    } else if acwr < t.acwr_safe_low {
        1.0 + 0.8 * (t.acwr_safe_low - acwr)
    } else {
        1.0
    };
    let wear = f32::from(c.wear.iter().copied().max().unwrap_or(0));
    let age = person.dob.age_years(w.date);
    let age_mult = if age > 28.0 {
        1.0 + 0.03 * (age - 28.0)
    } else if age < 20.0 {
        1.0 + 0.02 * (20.0 - age)
    } else {
        1.0
    };
    let wellbeing = 1.1 - 0.2 * f32::from(h.wellbeing) / 100.0;
    (0.6 + 0.04 * person.hidden.f(Hidden::InjuryProneness))
        * acwr_mult.min(2.5)
        * (1.0 + 0.8 * f32::from(h.fatigue) / 100.0)
        * (1.0 + 0.6 * wear / 100.0)
        * age_mult
        * wellbeing
        * w.medical.fragility(p)
        * crate::returns::hazard_factor(w, p)
        * crate::affairs::body_care(w, c.person)
}

/// Pick an injury from the catalogue for a mechanism; returns (catalogue index, days).
pub fn sample_injury(defs: &[InjuryDef], mech: &[Mechanism], rng: &mut Rng) -> Option<(usize, u16)> {
    let idx: Vec<usize> = (0..defs.len()).filter(|&i| mech.contains(&defs[i].mechanism)).collect();
    let k = rng.weighted_by(&idx, |&i| defs[i].weight)?;
    let d = &defs[idx[k]];
    // Triangular(min, mode, max).
    let (a, m, b) = (f32::from(d.days[0]), f32::from(d.days[1]), f32::from(d.days[2]));
    let u = rng.f32();
    let fc = (m - a) / (b - a).max(1.0);
    let days = if u < fc { a + (u * (b - a) * (m - a)).sqrt() } else { b - ((1.0 - u) * (b - a) * (b - m)).sqrt() };
    Some((idx[k], days.round().max(1.0) as u16))
}

struct Outcome {
    player: PlayerId,
    club: ClubId,
    injury: Option<(u16, u16)>,
    recovered: bool,
}

/// Daily condition/load/sharpness/injury pass for every active player.
pub fn daily(w: &mut World, days: &[DayKind]) {
    let today = w.date;
    let seed = w.seed;
    let tuning = w.data.tuning.health.clone();
    let winter = matches!(today.month(), 11 | 12 | 1 | 2);
    let medical: Vec<f32> = w.clubs.iter().map(|c| 0.9 + 0.012 * f32::from(c.facilities.medical)).collect();
    let hazards: Vec<f32> = (0..w.players.len())
        .into_par_iter()
        .map(|i| {
            let p = PlayerId(i as u32);
            if w.players.hot[p].status == PlayerStatus::Active { hazard_mult(w, p) } else { 1.0 }
        })
        .collect();
    let injuries = &w.data.injuries;
    let people = &w.people;
    let lives = &w.lives;
    let cold: &[pw_world::PlayerCold] = &w.players.cold;

    let outcomes: Vec<Outcome> = w
        .players
        .hot
        .par_iter_mut()
        .enumerate()
        .filter_map(|(i, h)| {
            if h.status == PlayerStatus::Retired {
                return None;
            }
            let p = PlayerId(i as u32);
            let c = &cold[i];
            let person = &people[c.person];
            let mut rng = Rng::keyed(&[seed, stream::HEALTH, i as u64, today.0 as u64]);
            let kind = if h.team.is_some() { days[h.team.0 as usize] } else { DayKind::Offseason };
            let nf = c.attrs.get(Attr::NaturalFitness);
            let age = person.dob.age_years(today);
            let mut out = Outcome { player: p, club: h.club, injury: None, recovered: false };

            // Injury progression.
            if h.injury != 0 {
                let speed = if h.club.is_some() { medical[h.club.0 as usize] } else { 0.9 };
                let step = if rng.chance(speed.fract()) { speed.ceil() } else { speed.floor() }.max(1.0) as u16;
                h.injury_days = h.injury_days.saturating_sub(step);
                if h.injury_days == 0 {
                    h.injury = 0;
                    h.injury_total = 0;
                    h.sharpness = h.sharpness.min(45);
                    h.condition = h.condition.min(80);
                    out.recovered = true;
                }
                h.acute = pw_core::math::ewma(h.acute, 0.0, 0.25);
                h.chronic = pw_core::math::ewma(h.chronic, 0.0, 0.069);
                return Some(out);
            }

            // Load and fitness: the club's day, shaped by the player's own plan
            // (intensity, extra sessions, recovery work) and their week off the pitch.
            let plan = c.plan;
            let training_day = matches!(kind, DayKind::Training | DayKind::BeforeMatch);
            let own = if training_day { plan.intensity.load_mult() + f32::from(plan.extra) * 0.06 - f32::from(plan.recovery) * 0.03 } else { 1.0 };
            let load = kind.load() * own * if h.status == PlayerStatus::FreeAgent { 0.4 } else { 1.0 };
            let routine = lives.get(c.person).map(|l| l.routine).unwrap_or_default();
            let sleep = lives.get(c.person).map_or(70.0, |l| f32::from(l.sleep));
            h.acute = pw_core::math::ewma(h.acute, load, 0.25);
            h.chronic = pw_core::math::ewma(h.chronic, load, 0.069);
            let capacity = 380.0 + nf * 12.0;
            let debt = f32::from(h.fatigue) + (load - capacity).max(-150.0) / 60.0;
            h.fatigue = debt.clamp(0.0, 100.0) as u8;

            let age_rec = if age > 30.0 { 1.0 - 0.025 * (age - 30.0) } else { 1.0 };
            let habits = 1.0 + f32::from(plan.recovery) * 0.03 + (f32::from(routine.rest + routine.recovery) - 16.0) * 0.004 - f32::from(routine.nightlife) * 0.008 + (sleep - 70.0) * 0.002;
            let recover = tuning.condition_recovery * (0.7 + nf / 40.0) * age_rec * kind.recovery() * (0.9 + 0.2 * f32::from(h.wellbeing) / 100.0) * habits.clamp(0.8, 1.15);
            let cap = 100.0 - f32::from(h.fatigue) * 0.3;
            let cond = (f32::from(h.condition) + recover - load / 60.0).min(cap);
            h.condition = cond.clamp(10.0, 100.0) as u8;

            // Sharpness decays without matches; training holds a floor.
            let since_match = h.last_match.days_until(today);
            if since_match > 3 {
                let floor = if kind == DayKind::Offseason { 20.0 } else { 45.0 };
                let s = f32::from(h.sharpness);
                h.sharpness = if s > floor { (s - 1.2).max(floor) } else { (s + 0.4).min(floor) } as u8;
            }
            h.minutes_4w = (f32::from(h.minutes_4w) * (27.0 / 28.0)) as u16;

            // Training rating as coaches see it.
            if matches!(kind, DayKind::Training | DayKind::BeforeMatch) {
                let prof = person.hidden.f(Hidden::Professionalism);
                let det = c.attrs.get(Attr::Determination);
                let effort = match plan.intensity {
                    pw_world::Intensity::Light => -0.25,
                    pw_world::Intensity::Normal => 0.0,
                    pw_world::Intensity::High => 0.2,
                } + f32::from(plan.extra) * 0.06;
                let base = 5.8
                    + 0.06 * (prof - 10.0)
                    + 0.05 * (det - 10.0)
                    + 0.012 * (f32::from(c.ca) - 100.0).clamp(-40.0, 60.0)
                    + 0.3 * (f32::from(h.wellbeing) - 60.0) / 40.0
                    + 0.4 * (f32::from(h.condition) - 85.0) / 15.0
                    + effort
                    - f32::from(routine.nightlife) * 0.03
                    + rng.normal() * 0.55;
                let r = (base.clamp(4.0, 10.0) * 10.0).round();
                h.training = (f32::from(h.training) * 0.8 + r * 0.2).round() as u8;
            }

            // Training injuries and illness.
            let train_p = if load > 200.0 { tuning.training_session * hazards[i] * (load / 470.0) } else { 0.0 };
            let ill_p = tuning.illness_daily * if winter { 2.0 } else { 0.8 };
            if rng.chance(train_p) {
                let mech = [Mechanism::NonContact, Mechanism::Overuse];
                out.injury = sample_injury(injuries, &mech, &mut rng).map(|(k, d)| (k as u16 + 1, d));
            } else if rng.chance(ill_p) {
                out.injury = sample_injury(injuries, &[Mechanism::Illness], &mut rng).map(|(k, d)| (k as u16 + 1, d));
            }
            if let Some((k, d)) = out.injury {
                h.injury = k;
                h.injury_days = d;
                h.injury_total = d;
            }
            (out.injury.is_some() || out.recovered).then_some(out)
        })
        .collect();

    for o in outcomes {
        let vis = if o.club.is_some() { Visibility::Club(o.club) } else { Visibility::Public };
        if let Some((k, d)) = o.injury {
            apply_injury_effects(w, o.player, k);
            // The world hears the medical team's estimate, not the truth.
            let est = crate::medical::on_injury(w, o.player, k, d);
            w.events.push(today, vis, EventKind::Injured { player: o.player, injury: k, days: est });
        } else if o.recovered {
            w.events.push(today, vis, EventKind::Recovered { player: o.player });
        }
    }
}

/// Record an injury that happened in a match (the engine only flags it).
pub fn match_injury(w: &mut World, p: PlayerId, rng: &mut Rng, noncontact: bool) {
    let mech: &[Mechanism] = if noncontact { &[Mechanism::NonContact, Mechanism::Overuse] } else { &[Mechanism::Contact] };
    if let Some((k, d)) = sample_injury(&w.data.injuries, mech, rng) {
        let h = &mut w.players.hot[p];
        h.injury = k as u16 + 1;
        h.injury_days = d;
        h.injury_total = d;
        let club = h.club;
        apply_injury_effects(w, p, k as u16 + 1);
        let est = crate::medical::on_injury(w, p, k as u16 + 1, d);
        let vis = if club.is_some() { Visibility::Club(club) } else { Visibility::Public };
        w.events.push(w.date, vis, EventKind::Injured { player: p, injury: k as u16 + 1, days: est });
    }
}

/// Body wear and possible permanent loss for serious injuries (05 §4.6).
fn apply_injury_effects(w: &mut World, p: PlayerId, injury: u16) {
    let def = w.data.injuries[usize::from(injury - 1)].clone();
    let mut rng = Rng::keyed(&[w.seed, stream::HEALTH, u64::from(p.0), w.date.0 as u64, 77]);
    let c = &mut w.players.cold[p];
    c.injuries_career += 1;
    if let Some(slot) = def.region.wear_slot() {
        c.wear[slot] = c.wear[slot].saturating_add(def.wear).min(100);
    }
    for (attr, max_loss) in &def.permanent {
        let loss = rng.range_f32(0.0, *max_loss);
        if loss > 0.2 {
            c.attrs.add(*attr, -loss);
        }
    }
    if !def.permanent.is_empty() {
        c.refresh_ca(&w.data.weights);
    }
}

/// Human-readable injury name.
pub fn injury_name(w: &World, injury: u16) -> &str {
    if injury == 0 { "" } else { &w.data.injuries[usize::from(injury - 1)].name }
}

pub fn team_of(w: &World, p: PlayerId) -> TeamId {
    w.players.hot[p].team
}
