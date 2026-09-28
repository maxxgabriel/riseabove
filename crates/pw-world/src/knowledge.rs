//! Perception (03 §11). Clubs store only *evidence* (minutes observed, when);
//! estimates are derived on demand as `truth + σ·bias`, where the bias is a
//! stable hash of (observer, subject, field). As evidence grows σ shrinks and
//! the estimate converges on the truth — consistent over time, no per-attribute
//! storage for hundreds of thousands of (club, player) pairs.

use pw_core::rng::noise;
use pw_core::{ClubId, Date, PlayerId};
use pw_data::Perception;
use serde::{Deserialize, Serialize};

use crate::FxHashMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Seen {
    pub minutes: u16,
    pub last: Date,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Knowledge {
    clubs: Vec<FxHashMap<PlayerId, Seen>>,
}

impl Knowledge {
    pub fn resize(&mut self, n_clubs: usize) {
        self.clubs.resize_with(n_clubs, FxHashMap::default);
    }

    #[inline]
    pub fn observe(&mut self, club: ClubId, player: PlayerId, minutes: u16, today: Date) {
        let e = self.clubs[club.0 as usize].entry(player).or_insert(Seen { minutes: 0, last: today });
        e.minutes = e.minutes.saturating_add(minutes);
        e.last = today;
    }

    #[inline]
    pub fn seen(&self, club: ClubId, player: PlayerId) -> Option<Seen> {
        self.clubs.get(club.0 as usize)?.get(&player).copied()
    }

    pub fn known(&self, club: ClubId) -> impl Iterator<Item = (PlayerId, Seen)> + '_ {
        self.clubs[club.0 as usize].iter().map(|(&p, &s)| (p, s))
    }

    pub fn known_count(&self, club: ClubId) -> usize {
        self.clubs[club.0 as usize].len()
    }

    /// Forget stale, thin evidence so memory stays bounded over decades.
    pub fn forget(&mut self, before: Date, min_minutes: u16) {
        for m in &mut self.clubs {
            m.retain(|_, s| s.last >= before || s.minutes >= min_minutes);
        }
    }

    pub fn clear_player(&mut self, player: PlayerId) {
        for m in &mut self.clubs {
            m.remove(&player);
        }
    }
}

/// Observer identity for bias keys; clubs and individual people never collide.
#[derive(Clone, Copy, Debug)]
pub enum Observer {
    Club(ClubId),
    Person(u32),
}

impl Observer {
    #[inline]
    fn key(self) -> u64 {
        match self {
            Observer::Club(c) => u64::from(c.0),
            Observer::Person(p) => (1 << 40) | u64::from(p),
        }
    }
}

/// Field tags for stable bias keys.
pub mod field {
    pub const CA: u64 = 1000;
    pub const PA: u64 = 1001;
    pub const ATTR: u64 = 0;
}

/// Uncertainty on the 1–20 attribute scale (17 §10).
pub fn sigma(t: &Perception, seen: Option<Seen>, judging: f32, today: Date, famous: bool) -> f32 {
    let (minutes, weeks) = match seen {
        Some(s) => (f32::from(s.minutes), (s.last.days_until(today).max(0) as f32) / 7.0),
        None => (0.0, 26.0),
    };
    let evidence = 1.0 - (minutes / t.full_knowledge_minutes).min(0.9);
    let skill = 1.3 - 0.03 * judging.clamp(1.0, 20.0);
    let s = t.sigma0 * evidence * skill + t.decay_per_week * weeks.min(52.0);
    if famous { s.min(t.sigma0 * 0.35) } else { s }
}

#[inline]
pub fn perceive(truth: f32, sigma: f32, observer: Observer, subject: PlayerId, field: u64) -> f32 {
    truth + sigma * noise(&[observer.key(), u64::from(subject.0), field])
}

/// Perceived current ability (CA scale) and its ± band.
pub fn perceived_ca(true_ca: f32, attr_sigma: f32, observer: Observer, subject: PlayerId) -> (f32, f32) {
    let band = attr_sigma * 6.0;
    (perceive(true_ca, band, observer, subject, field::CA).clamp(1.0, 200.0), band)
}

/// Perceived potential: noisier than CA and never below the perceived CA.
pub fn perceived_pa(true_pa: f32, est_ca: f32, attr_sigma: f32, judging_potential: f32, observer: Observer, subject: PlayerId) -> (f32, f32) {
    let band = attr_sigma * 6.0 + (20.0 - judging_potential.clamp(1.0, 20.0)) * 1.2;
    (perceive(true_pa, band, observer, subject, field::PA).clamp(est_ca, 200.0), band)
}
