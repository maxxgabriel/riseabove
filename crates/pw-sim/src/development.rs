//! Weekly attribute development (04, 17 §2): potential room × age curve ×
//! (training + match experience) × wellness × professionalism, plus decline.

use pw_core::attr::{CurveGroup, N_CURVE_GROUPS};
use pw_core::rng::{Rng, stream};
use pw_core::{Attr, Hidden, Pos, StaffAttr};
use pw_world::{PlayerStatus, StaffRole, TeamKind, World};
use rayon::prelude::*;

/// What surrounds a player's growth beyond training and minutes, each read from state the world already keeps.
#[derive(Clone, Copy, Debug)]
pub struct Circumstances {
    /// Ambition (1–20): how hard the player drives themselves.
    pub ambition: f32,
    /// Confidence (0–100).
    pub confidence: f32,
    /// Has an experienced teammate taken them under their wing.
    pub mentored: bool,
    /// Current ability minus the usual standard at their club: positive means playing below their level.
    pub level_gap: f32,
    /// Months since arriving at a club in another country, if that is where they are now.
    pub months_abroad: Option<f32>,
    /// Adaptability (1–20).
    pub adaptability: f32,
}

/// A multiplier near 1.0 on weekly growth. Drive and belief speed it up, a mentor helps a little, playing well below one's
/// level teaches less than being stretched, and the first months in a new country slow everything until the player settles
/// (faster for the adaptable).
pub fn circumstance_factor(c: &Circumstances) -> f32 {
    let drive = 0.90 + 0.01 * c.ambition;
    let belief = 0.92 + 0.0016 * c.confidence;
    let mentor = if c.mentored { 1.06 } else { 1.0 };
    let challenge = 1.0 - 0.25 * (c.level_gap / 40.0).clamp(-1.0, 1.0);
    let settle = c.months_abroad.map_or(1.0, |m| {
        let months_to_settle = 9.0 - 0.25 * c.adaptability;
        0.80 + 0.20 * (m / months_to_settle.max(2.0)).min(1.0)
    });
    (drive * belief * mentor * challenge * settle).clamp(0.6, 1.4)
}

/// One team's coaching environment: quality per attribute group, facilities, the
/// best specialist for each unit (keeper, defence, midfield, attack), what the
/// week's plan emphasises, the club's usual standard and its nation.
struct Env {
    coach: [f32; N_CURVE_GROUPS],
    facility: f32,
    unit: [f32; 4],
    emphasis: [f32; N_CURVE_GROUPS],
    level: f32,
    nation: pw_core::NationId,
}

fn team_environment(w: &World) -> Vec<Env> {
    w.teams
        .iter()
        .enumerate()
        .map(|(ti, t)| {
            let club = &w.clubs[t.club];
            let youth = t.kind.is_youth();
            let mut best = [8.0f32; N_CURVE_GROUPS];
            let mut unit = [8.0f32; 4];
            for &s in &club.staff {
                let st = &w.staff[s];
                if !matches!(st.role, StaffRole::Coach | StaffRole::GkCoach | StaffRole::FitnessCoach | StaffRole::Manager | StaffRole::Assistant | StaffRole::HeadOfYouth) {
                    continue;
                }
                let yb = if youth { st.attrs.f(StaffAttr::Youngsters) * 0.3 } else { 0.0 };
                let g = |a: StaffAttr| st.attrs.f(a) * if youth { 0.7 } else { 1.0 } + yb;
                best[CurveGroup::Technical as usize] = best[CurveGroup::Technical as usize].max(g(StaffAttr::Technical));
                best[CurveGroup::Mental as usize] = best[CurveGroup::Mental as usize].max(g(StaffAttr::Mental));
                best[CurveGroup::Speed as usize] = best[CurveGroup::Speed as usize].max(g(StaffAttr::Fitness));
                best[CurveGroup::Power as usize] = best[CurveGroup::Power as usize].max(g(StaffAttr::Fitness));
                best[CurveGroup::Goalkeeping as usize] = best[CurveGroup::Goalkeeping as usize].max(g(StaffAttr::Goalkeeping));
                unit[0] = unit[0].max(g(StaffAttr::Goalkeeping));
                unit[1] = unit[1].max(g(StaffAttr::Defending));
                unit[2] = unit[2].max(g(StaffAttr::Tactical));
                unit[3] = unit[3].max(g(StaffAttr::Attacking));
            }
            let coach = best.map(|v| 0.6 + 0.8 * (v / 20.0));
            let fac = if youth { club.facilities.youth } else { club.facilities.training };
            // The week's plan: more drilling favours technical and mental growth, more conditioning speed and power.
            let plan = w.ext.training.plans.get(&pw_core::TeamId(ti as u32)).copied().unwrap_or_default();
            let (tac, phys) = (f32::from(plan.tactical) / 100.0, f32::from(plan.physical) / 100.0);
            let mut emphasis = [1.0f32; N_CURVE_GROUPS];
            emphasis[CurveGroup::Technical as usize] = 1.0 + 0.3 * (tac - 0.4);
            emphasis[CurveGroup::Mental as usize] = 1.0 + 0.3 * (tac - 0.4);
            emphasis[CurveGroup::Speed as usize] = 1.0 + 0.3 * (phys - 0.35);
            emphasis[CurveGroup::Power as usize] = 1.0 + 0.3 * (phys - 0.35);
            Env { coach, facility: 0.85 + 0.3 * f32::from(fac) / 20.0, unit, emphasis, level: crate::market::ideal_ca(club.reputation), nation: club.nation }
        })
        .collect()
}

pub fn weekly(w: &mut World) {
    let today = w.date;
    let seed = w.seed;
    let env = team_environment(w);
    let dev = w.data.tuning.development.clone();
    let curves = &w.data.curves;
    let weights = &w.data.weights;
    let hot: &[pw_world::PlayerHot] = &w.players.hot;
    let people = &w.people;
    let mentored = &w.growth.records;
    let team_kind: Vec<TeamKind> = w.teams.iter().map(|t| t.kind).collect();
    // Players outside professional teams develop where they are: their school or university, else their district.
    let story = &w.ext.ecosystem.story;
    let regions = &w.ext.ecosystem.regions;
    let member = &w.minor.member_of;
    let insts = &w.minor.institutions;
    let inst_profile = &w.ext.ecosystem.inst;
    let local_env = |p: pw_core::PlayerId| -> Option<([f32; N_CURVE_GROUPS], f32)> {
        let s = story.get(&p)?;
        if s.dev.is_none() {
            return None;
        }
        let r = &regions[s.dev];
        let (mut coach, mut fac) = (r.coach_density / 20.0, r.facilities / 100.0);
        if let Some(&i) = member.get(&p) {
            coach = coach.max(f32::from(insts[i as usize].coaching));
            fac = fac.max(inst_profile.get(&i).map_or(0.0, |x| x.facilities / 100.0));
        }
        let q = 0.6 + 0.8 * (coach.clamp(1.0, 20.0) / 20.0);
        Some(([q; N_CURVE_GROUPS], 0.85 + 0.3 * fac.clamp(0.0, 1.0)))
    };

    w.players.cold.par_iter_mut().enumerate().for_each(|(i, c)| {
        let h = &hot[i];
        if h.status == PlayerStatus::Retired {
            return;
        }
        let person = &people[c.person];
        let age = person.dob.age_years(today) + f32::from(c.bio_offset) / 10.0;
        let mut rng = Rng::keyed(&[seed, stream::DEVELOPMENT, i as u64, today.0 as u64]);
        let (coach, facility, unit, emphasis, level, club_nation) = if h.team.is_some() {
            let e = &env[h.team.0 as usize];
            (e.coach, e.facility, e.unit, e.emphasis, e.level, e.nation)
        } else {
            let ca_now = f32::from(c.ca);
            let (cg, f) = local_env(pw_core::PlayerId(i as u32)).unwrap_or(([0.75; N_CURVE_GROUPS], 0.85));
            (cg, f, [8.0; 4], [1.0; N_CURVE_GROUPS], ca_now, pw_core::NationId::NONE)
        };
        let abroad = club_nation.is_some() && person.nation != club_nation && person.nation2 != club_nation;
        let circumstances = circumstance_factor(&Circumstances {
            ambition: person.hidden.f(Hidden::Ambition),
            confidence: f32::from(h.confidence),
            mentored: mentored.get(&pw_core::PlayerId(i as u32)).is_some_and(|r| r.mentor.is_some()),
            // A youth side does not play to the first team's standard, so it has no gap to speak of.
            level_gap: if h.team.is_some() && !team_kind[h.team.0 as usize].is_youth() { f32::from(c.ca) - level } else { 0.0 },
            months_abroad: abroad.then(|| c.joined.days_until(today).max(0) as f32 / 30.0),
            adaptability: person.hidden.f(Hidden::Adaptability),
        });
        // The specialist for this player's unit matters a little: a good striker coach helps strikers.
        let unit_mult = 0.9 + 0.2 * unit[match c.best_pos.group() {
            pw_core::PosGroup::Gk => 0,
            pw_core::PosGroup::Def => 1,
            pw_core::PosGroup::Mid => 2,
            pw_core::PosGroup::Att => 3,
        }] / 20.0;
        let level_fit = if h.team.is_some() && team_kind[h.team.0 as usize].is_youth() { 0.7 } else { 1.0 };
        let minutes = f32::from(h.minutes_4w);
        let rating = h.form_avg().unwrap_or(6.5);
        let match_f = (minutes / 360.0).min(1.2) * level_fit * (0.8 + 0.04 * (rating - 6.0));
        let wellness = 0.8 + 0.2 * f32::from(h.wellbeing) / 100.0;
        let prof = 0.7 + 0.03 * person.hidden.f(Hidden::Professionalism) + 0.015 * c.attrs.get(Attr::Determination);
        let room = (f32::from(c.pa) - f32::from(c.ca)).max(0.0) / 200.0;
        let injured = if h.injury != 0 { 0.35 } else { 1.0 };
        let keeper = c.best_pos == Pos::GK;
        let row = weights.row(c.best_pos);
        let max_w = row.iter().copied().fold(1e-6f32, f32::max);
        let nf = c.attrs.get(Attr::NaturalFitness);
        let wear = f32::from(c.wear.iter().copied().max().unwrap_or(0));
        let cap = if age <= 18.0 { dev.weekly_cap_youth } else { dev.weekly_cap };
        let before = c.attrs;
        let ca_before = c.ca;

        for a in Attr::ALL {
            if a.is_goalkeeping() != keeper && (a.is_goalkeeping() || a == Attr::Eccentricity) {
                continue;
            }
            let g = a.curve();
            let age_f = curves.factor(g, age);
            let train_f = 0.4 + 0.6 * coach[g as usize] * facility * unit_mult * emphasis[g as usize];
            let emph = 0.5 + 0.5 * row[a.idx()] / max_w;
            let plan = c.plan;
            let own = plan.intensity.growth_mult() * plan.focus.weight(a) * (1.0 + 0.04 * f32::from(plan.extra));
            let grow = dev.growth * room * age_f.max(0.0) * (train_f * own + match_f) * wellness * prof * emph * injured * circumstances;
            let physical = matches!(g, CurveGroup::Speed | CurveGroup::Power);
            let decline = age_f.min(0.0) * dev.decline * if physical { (1.3 - nf / 20.0 * 0.6) * (1.0 + wear / 200.0) } else { 1.0 };
            let noise = rng.normal() * dev.noise * if age_f > 0.0 { 1.0 } else { 0.5 };
            let delta = (grow + decline + noise).clamp(-cap, cap);
            if delta.abs() > 1e-4 {
                c.attrs.add(a, delta);
            }
        }
        // Learning a position: familiarity grows with focused work and versatility.
        if let pw_world::Focus::Position(pos) = c.plan.focus {
            let versatility = person.hidden.f(Hidden::Versatility);
            let f = &mut c.familiarity[pos.idx()];
            if *f < 18 && rng.chance(0.15 + versatility / 40.0) {
                *f += 1;
            }
        }
        c.refresh_ca(weights);

        // Growth never pushes current ability past potential.
        if c.ca > c.pa && c.ca > ca_before {
            let over = f32::from(c.ca - c.pa);
            let gained = f32::from(c.ca - ca_before).max(1.0);
            let keep = (1.0 - over / gained).clamp(0.0, 1.0);
            for a in Attr::ALL {
                let old = before.get(a);
                let new = c.attrs.get(a);
                if new > old {
                    c.attrs.set(a, old + (new - old) * keep);
                }
            }
            c.refresh_ca(weights);
        }

        // Late bloomers: potential is re-rolled once around 19 (03 §7).
        if !c.pa_rerolled && age >= 19.0 {
            c.pa_rerolled = true;
            let bias = f32::from(c.bio_offset) * 0.8 + (prof - 1.0) * 10.0 + (match_f - 0.6) * 6.0;
            let delta = (rng.normal() * 5.0 + bias).clamp(-dev.pa_reroll_max, dev.pa_reroll_max);
            c.pa = (f32::from(c.pa) + delta).round().clamp(f32::from(c.ca), 200.0) as u8;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Circumstances {
        Circumstances { ambition: 10.0, confidence: 50.0, mentored: false, level_gap: 0.0, months_abroad: None, adaptability: 10.0 }
    }

    #[test]
    fn an_ordinary_player_in_ordinary_circumstances_grows_at_about_the_normal_rate() {
        let f = circumstance_factor(&base());
        assert!((0.95..=1.10).contains(&f), "{f}");
    }

    #[test]
    fn drive_belief_and_a_mentor_speed_growth_and_their_absence_slows_it() {
        let f = |c: Circumstances| circumstance_factor(&c);
        assert!(f(Circumstances { ambition: 19.0, ..base() }) > f(base()) && f(base()) > f(Circumstances { ambition: 3.0, ..base() }));
        assert!(f(Circumstances { confidence: 90.0, ..base() }) > f(Circumstances { confidence: 15.0, ..base() }));
        assert!(f(Circumstances { mentored: true, ..base() }) > f(base()));
    }

    #[test]
    fn being_stretched_teaches_more_than_coasting_below_your_level() {
        let stretched = circumstance_factor(&Circumstances { level_gap: -30.0, ..base() });
        let coasting = circumstance_factor(&Circumstances { level_gap: 30.0, ..base() });
        assert!(stretched > circumstance_factor(&base()) && circumstance_factor(&base()) > coasting);
    }

    #[test]
    fn a_new_country_slows_growth_until_the_player_settles_and_the_adaptable_settle_sooner() {
        let at = |months: f32, adaptability: f32| circumstance_factor(&Circumstances { months_abroad: Some(months), adaptability, ..base() });
        assert!(at(0.0, 10.0) < at(3.0, 10.0) && at(3.0, 10.0) < at(12.0, 10.0));
        assert!((at(24.0, 10.0) - circumstance_factor(&base())).abs() < 1e-6, "settled: no effect");
        assert!(at(3.0, 19.0) > at(3.0, 2.0));
    }

    #[test]
    fn the_factor_never_leaves_its_bounds() {
        let extreme = Circumstances { ambition: 20.0, confidence: 100.0, mentored: true, level_gap: -200.0, months_abroad: None, adaptability: 20.0 };
        assert!(circumstance_factor(&extreme) <= 1.4);
        let worst = Circumstances { ambition: 1.0, confidence: 0.0, mentored: false, level_gap: 200.0, months_abroad: Some(0.0), adaptability: 1.0 };
        assert!(circumstance_factor(&worst) >= 0.6);
    }
}
