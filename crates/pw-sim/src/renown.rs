//! Renown (11 §5): the audiences beyond a player's football reputation.
//!
//! - Local: how the club's city sees them — fan standing, years at the
//!   club, being one of their own.
//! - Continental: continental club football, international tournaments,
//!   awards.
//! - Fame: celebrity beyond football — world reputation, public image,
//!   media time, partners, controversies.
//! - Followers: grow with fame and with the time a person gives to media.
//!
//! Renown is read by sponsors and brands, by the media's appetite for a
//! person, by stress and privacy in the life model, and by post-career
//! opportunities (punditry, ambassadors).

use pw_core::{CompId, PersonId, PlayerId};
use pw_world::{CompKind, PlayerStatus, World};

pub fn monthly(w: &mut World) {
    let today = w.date;
    let continental: Vec<CompId> = w.comps.iter_enumerated().filter(|(_, c)| c.kind == CompKind::Continental).map(|(id, _)| id).collect();
    let ids: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status != PlayerStatus::Retired || w.renown.people.contains_key(&w.players.cold[p].person)).collect();
    for p in ids {
        let c = &w.players.cold[p];
        let who: PersonId = c.person;
        let h = &w.players.hot[p];
        let retired = h.status == PlayerStatus::Retired;
        let club = h.club;
        // Skip the anonymous: nobody outside a small circle has heard of them.
        if !retired && c.rep.current < 1500 && !w.renown.people.contains_key(&who) {
            continue;
        }
        let fans = if club.is_some() { w.media.fan(club, who).map_or(0.0, |f| f32::from(f.score)) } else { 0.0 };
        let years = c.joined.days_until(today) as f32 / 365.0;
        let local_target = if retired {
            0.0
        } else {
            (f32::from(c.rep.current) * 0.6 + fans * 3.0 + years.min(8.0) * 250.0 + if club.is_some() && c.youth_club == club { 1500.0 } else { 0.0 }).clamp(0.0, 10_000.0)
        };
        let cont_apps = w.perf.recent.get(&p).map_or(0, |v| v.iter().filter(|a| continental.contains(&a.comp)).count()) as f32;
        let intl = crate::intl::standing(w, p);
        let cont_target = (f32::from(c.rep.world) * 0.7 + cont_apps * 250.0 + intl * 1500.0).clamp(0.0, 10_000.0);
        let image = f32::from(w.media.image.get(&who).copied().unwrap_or(0));
        let media_time = w.lives.get(who).map_or(1.0, |l| f32::from(l.routine.media));
        let partner_fame = w.lives.get(who).and_then(|l| l.household.partner).map_or(0.0, |pt| f32::from(w.renown.of(pt.person).fame) * 0.2);
        let fame_target = (f32::from(c.rep.world) * 0.8 + image.abs() * 1.5 + media_time * 250.0 + partner_fame).clamp(0.0, 10_000.0)
            * if retired { 0.7 } else { 1.0 };
        let world_rep = c.rep.world;
        let r = w.renown.people.entry(who).or_default();
        let step = |cur: u16, tgt: f32, a: f32| (f32::from(cur) + a * (tgt - f32::from(cur))).clamp(0.0, 10_000.0) as u16;
        r.local = step(r.local, local_target, 0.15);
        r.continental = step(r.continental, cont_target, 0.08);
        r.fame = step(r.fame, fame_target, 0.06);
        // Followers compound with fame and the effort put into being seen.
        let growth = (f32::from(r.fame) / 10_000.0).powi(2) * 400_000.0 * (0.5 + media_time / 6.0);
        let churn = r.followers as f32 * if retired { 0.02 } else { 0.005 };
        r.followers = (r.followers as f32 + growth - churn).max(0.0) as u32;
        if world_rep > r.peak_world {
            r.peak_world = world_rep;
            r.peak_date = today;
        }
    }
    // Staff (managers) have fame too, driven by their reputation and record.
    let managers: Vec<(PersonId, u16)> = w.staff.iter().filter(|s| !s.retired && s.reputation >= 3000).map(|s| (s.person, s.reputation)).collect();
    for (who, rep) in managers {
        let r = w.renown.people.entry(who).or_default();
        let target = f32::from(rep) * 0.8;
        r.fame = (f32::from(r.fame) + 0.05 * (target - f32::from(r.fame))).clamp(0.0, 10_000.0) as u16;
    }
    // Fame is pressure: the very famous have less privacy and more stress.
    let famous: Vec<PersonId> = w.renown.people.iter().filter(|(_, r)| r.fame >= 7000).map(|(&p, _)| p).collect();
    for who in famous {
        if let Some(l) = w.lives.get_mut(who) {
            l.stress = l.stress.saturating_add(1).min(100);
        }
    }
}
