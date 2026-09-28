//! Weekly morale drift (03 §8) and the AI players' statistical life model
//! (10 §13), which feeds the same bounded well-being channel the
//! protagonist's detailed life sim writes to.

use pw_core::Hidden;
use pw_core::rng::{Rng, stream};
use pw_world::{MindKind, PlayerStatus, World};
use rayon::prelude::*;

pub fn weekly(w: &mut World) {
    let cold: &[pw_world::PlayerCold] = &w.players.cold;
    w.players.hot.par_iter_mut().enumerate().for_each(|(i, h)| {
        if h.status != PlayerStatus::Active {
            return;
        }
        let c = &cold[i];
        let expected = c.status.expected_minutes() * 360.0;
        let pt = (f32::from(h.minutes_4w) / expected.max(40.0)).min(1.3);
        let target = 55.0 + (pt - 0.8) * 25.0 + (f32::from(h.wellbeing) - 60.0) * 0.2;
        h.morale = pw_core::math::ewma(f32::from(h.morale), target, 0.25).clamp(5.0, 100.0) as u8;
        h.confidence = pw_core::math::ewma(f32::from(h.confidence), 60.0, 0.1).clamp(5.0, 100.0) as u8;
    });
}

/// Monthly well-being for AI-minded players, from personality, age and
/// circumstance. External minds get theirs from the career layer.
pub fn monthly_life(w: &mut World) {
    let seed = w.seed;
    let month = u64::from(w.date.month()) + w.date.year() as u64 * 12;
    let today = w.date;
    let cold: &[pw_world::PlayerCold] = &w.players.cold;
    let people = &w.people;
    let clubs = &w.clubs;
    w.players.hot.par_iter_mut().enumerate().for_each(|(i, h)| {
        if h.status == PlayerStatus::Retired {
            return;
        }
        let c = &cold[i];
        let person = &people[c.person];
        if person.mind == MindKind::External {
            return;
        }
        let mut rng = Rng::keyed(&[seed, stream::LIFE, i as u64, month]);
        let abroad = h.club.is_some() && clubs[h.club].nation != person.nation;
        let adapt = person.hidden.f(Hidden::Adaptability);
        let settled = c.joined.days_until(today) > 180;
        let mut wb = 62.0 + (person.hidden.f(Hidden::Professionalism) - 10.0) * 0.8 + (person.hidden.f(Hidden::Pressure) - 10.0) * 0.5;
        if abroad {
            wb -= (12.0 - adapt).max(0.0) * if settled { 0.6 } else { 1.4 };
        }
        if h.injury_days > 60 {
            wb -= 8.0;
        }
        if h.status == PlayerStatus::FreeAgent {
            wb -= 10.0;
        }
        wb += rng.normal() * 6.0;
        h.wellbeing = pw_core::math::ewma(f32::from(h.wellbeing), wb, 0.5).clamp(10.0, 100.0) as u8;
    });
}
