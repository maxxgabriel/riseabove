//! Deterministic RNG. There is no global generator: every consumer derives a
//! stream from `(world seed, subsystem, stable entity ids, period…)`, so
//! results never depend on iteration order or thread scheduling, and adding a
//! random draw in one subsystem cannot shift the future of another.
//!
//! Architecture (see `docs/MEDIA_SOCIAL_HISTORY_SYSTEMS.md` §1):
//! - **World seed.** A new world gets a fresh seed from the operating system
//!   (`fresh_seed`) unless one is given; the seed is stored in the save, so
//!   the same save replays exactly and different seeds give different worlds.
//! - **Named subsystem streams** (`stream::*`). Every subsystem has its own
//!   tag; streams are never shared between subsystems.
//! - **Stable keys.** Within a subsystem, a stream is keyed by stable entity
//!   ids (person, club, account, journalist…) and, where the draw belongs to
//!   a period, by that period (`period::day/week/month/year`). A draw for one
//!   entity in one week therefore depends only on the seed, the subsystem,
//!   that entity and that week.
//! - **Playthroughs.** Taking control of someone for the first time mixes a
//!   fresh salt into the seed, so two playthroughs of one world diverge while
//!   each stays reproducible.

/// Subsystem stream tags, mixed into keyed seeds so subsystems never share
/// streams. Values are part of the save format's behaviour: never renumber.
pub mod stream {
    pub const WORLDGEN: u64 = 0x01;
    pub const MATCH: u64 = 0x02;
    pub const TRAINING: u64 = 0x03;
    pub const HEALTH: u64 = 0x04;
    pub const DEVELOPMENT: u64 = 0x05;
    pub const MARKET: u64 = 0x06;
    pub const SELECTION: u64 = 0x07;
    pub const PERCEPTION: u64 = 0x08;
    pub const CONTRACTS: u64 = 0x09;
    pub const YOUTH: u64 = 0x0a;
    pub const RETIREMENT: u64 = 0x0b;
    pub const STAFF: u64 = 0x0c;
    pub const DRAW: u64 = 0x0d;
    pub const LIFE: u64 = 0x0e;
    pub const MEDIA: u64 = 0x0f;
    pub const MIND: u64 = 0x10;
    pub const BOARD: u64 = 0x11;
    pub const SOCIAL: u64 = 0x12;
    pub const TALK: u64 = 0x13;
    pub const AGENT: u64 = 0x14;
    pub const FAMILY: u64 = 0x15;
    pub const INTENT: u64 = 0x16;
    pub const PLAYTHROUGH: u64 = 0x17;
    pub const NEGOTIATION: u64 = 0x18;
    pub const NARRATION: u64 = 0x19;
    pub const INTL: u64 = 0x1a;
    /// Names, birthdays and personalities of generated people.
    pub const IDENTITY: u64 = 0x1b;
    /// Supporter populations and social accounts.
    pub const SUPPORTERS: u64 = 0x1c;
    /// Journalists and outlets (generation and careers).
    pub const JOURNALISTS: u64 = 0x1d;
    /// Who posts, replies, shares and likes, and when.
    pub const SOCIAL_ACTIVITY: u64 = 0x1e;
    /// Press conferences, questions and answers.
    pub const PRESS: u64 = 0x1f;
    /// Contextual incidents (training rows, travel delays, burglaries…).
    pub const INCIDENTS: u64 = 0x20;
    /// World-level shocks (downturns, sponsor collapses, severe weather).
    pub const SHOCKS: u64 = 0x21;
    /// Voting noise in awards.
    pub const AWARDS: u64 = 0x22;
    /// Information passing from person to person.
    pub const GRAPEVINE: u64 = 0x23;
    /// Club cultures and rivalries.
    pub const CULTURE: u64 = 0x24;
    /// School, university, amateur and grassroots competitions.
    pub const MINOR: u64 = 0x25;
    /// Editorial decisions: which story runs, with what angle.
    pub const NEWSROOM: u64 = 0x26;
    /// Responses of people in authority to incidents.
    pub const RESPONSE: u64 = 0x27;
    /// Managers' and staffs' reading of matches.
    pub const TACTICS: u64 = 0x28;
    /// What people carry from their lives onto the pitch.
    pub const LIFESTATE: u64 = 0x29;

    // Descriptive aliases for the subsystem names used in design documents.
    pub const IDENTITY_GENERATION: u64 = IDENTITY;
    pub const SUPPORTER_GENERATION: u64 = SUPPORTERS;
    pub const JOURNALIST_GENERATION: u64 = JOURNALISTS;
    pub const PRESS_ACTIVITY: u64 = PRESS;
    pub const MATCH_RANDOMNESS: u64 = MATCH;
    pub const INJURIES: u64 = HEALTH;
    pub const RELATIONSHIPS: u64 = SOCIAL;
    pub const WORLD_SHOCKS: u64 = SHOCKS;
    pub const YOUTH_GENERATION: u64 = YOUTH;
    pub const MARKET_BEHAVIOUR: u64 = MARKET;

    /// Every stream with its name, for debugging tools and documentation.
    pub const ALL: [(u64, &str); 41] = [
        (WORLDGEN, "worldgen"),
        (MATCH, "match_randomness"),
        (TRAINING, "training"),
        (HEALTH, "injuries"),
        (DEVELOPMENT, "development"),
        (MARKET, "market_behaviour"),
        (SELECTION, "selection"),
        (PERCEPTION, "perception"),
        (CONTRACTS, "contracts"),
        (YOUTH, "youth_generation"),
        (RETIREMENT, "retirement"),
        (STAFF, "staff"),
        (DRAW, "draws"),
        (LIFE, "life"),
        (MEDIA, "media"),
        (MIND, "minds"),
        (BOARD, "boards"),
        (SOCIAL, "relationships"),
        (TALK, "conversations"),
        (AGENT, "agents"),
        (FAMILY, "family"),
        (INTENT, "intents"),
        (PLAYTHROUGH, "playthrough"),
        (NEGOTIATION, "negotiation"),
        (NARRATION, "narration"),
        (INTL, "international"),
        (IDENTITY, "identity_generation"),
        (SUPPORTERS, "supporter_generation"),
        (JOURNALISTS, "journalist_generation"),
        (SOCIAL_ACTIVITY, "social_activity"),
        (PRESS, "press_activity"),
        (INCIDENTS, "incidents"),
        (SHOCKS, "world_shocks"),
        (AWARDS, "awards"),
        (GRAPEVINE, "information_propagation"),
        (CULTURE, "culture"),
        (MINOR, "minor_football"),
        (NEWSROOM, "newsroom"),
        (RESPONSE, "responses"),
        (TACTICS, "tactics"),
        (LIFESTATE, "life_state"),
    ];

    pub fn name(tag: u64) -> &'static str {
        ALL.iter().find(|x| x.0 == tag).map_or("unknown", |x| x.1)
    }
}

/// Period keys for streams whose draws belong to a time window.
pub mod period {
    use crate::Date;

    #[inline]
    pub fn day(d: Date) -> u64 {
        d.0 as u64
    }

    #[inline]
    pub fn week(d: Date) -> u64 {
        (d.0.div_euclid(7)) as u64
    }

    #[inline]
    pub fn month(d: Date) -> u64 {
        (d.year() as u64) * 12 + u64::from(d.month())
    }

    #[inline]
    pub fn year(d: Date) -> u64 {
        d.year() as u64
    }
}

/// A fresh, unpredictable 64-bit seed from the operating system's hashing
/// entropy and the clock (no external crates). Used for new worlds and new
/// playthroughs; never called inside the simulation itself.
pub fn fresh_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let os = std::collections::hash_map::RandomState::new();
    let mut h = os.build_hasher();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    h.write_u128(now);
    h.write_u32(std::process::id());
    let local = 0u8;
    h.write_usize(&local as *const u8 as usize);
    mix64(h.finish())
}

/// A seed as the player sees it: 16 hex digits.
pub fn seed_label(seed: u64) -> String {
    format!("{seed:016x}")
}

/// Parse a seed typed by a person: hex (with or without `0x`), decimal, or
/// any other word (hashed, so `--seed banana` works and is repeatable).
pub fn parse_seed(s: &str) -> u64 {
    let t = s.trim();
    if let Some(h) = t.strip_prefix("0x")
        && let Ok(v) = u64::from_str_radix(h, 16)
    {
        return v;
    }
    if t.len() == 16
        && let Ok(v) = u64::from_str_radix(t, 16)
    {
        return v;
    }
    if let Ok(v) = t.parse::<u64>() {
        return v;
    }
    hash_key(&t.bytes().map(u64::from).collect::<Vec<u64>>())
}

#[inline]
pub const fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[inline]
pub const fn hash2(a: u64, b: u64) -> u64 {
    mix64(a ^ mix64(b.wrapping_add(0x9e37_79b9_7f4a_7c15)))
}

#[inline]
pub fn hash_key(parts: &[u64]) -> u64 {
    parts.iter().fold(0x243f_6a88_85a3_08d3, |h, &p| hash2(h, p))
}

/// Stable noise in `[-1, 1]` for a key. Used for fixed per-(observer, subject)
/// perception bias so estimates converge instead of flickering.
#[inline]
pub fn noise(parts: &[u64]) -> f32 {
    let h = hash_key(parts);
    ((h >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
}

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self { state: mix64(seed ^ 0x6a09_e667_f3bc_c908) }
    }

    #[inline]
    pub fn keyed(parts: &[u64]) -> Self {
        Self::new(hash_key(parts))
    }

    /// Derive an independent child stream.
    #[inline]
    pub fn fork(&mut self, tag: u64) -> Rng {
        Rng::new(hash2(self.next_u64(), tag))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0xa076_1d64_78bd_642f);
        let t = u128::from(self.state) * u128::from(self.state ^ 0xe703_7ed1_a0b4_28db);
        (t >> 64) as u64 ^ t as u64
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 * (1.0 / (1u64 << 24) as f32)
    }

    #[inline]
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Uniform in `[0, n)`; Lemire's method.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        debug_assert!(n > 0);
        ((u64::from(self.next_u32()) * u64::from(n)) >> 32) as u32
    }

    #[inline]
    pub fn index(&mut self, len: usize) -> usize {
        self.below(len as u32) as usize
    }

    /// Inclusive integer range.
    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(hi >= lo);
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    /// Approximately standard normal (Irwin–Hall, n = 4); bounded at ±3.46σ,
    /// which suits game tuning better than unbounded tails.
    #[inline]
    pub fn normal(&mut self) -> f32 {
        let s = self.f32() + self.f32() + self.f32() + self.f32();
        (s - 2.0) * 1.732_050_8
    }

    #[inline]
    pub fn normal_ms(&mut self, mean: f32, sd: f32) -> f32 {
        mean + sd * self.normal()
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.index(items.len())]
    }

    /// Index sampled proportionally to non-negative weights. Returns 0 when all are zero.
    pub fn weighted(&mut self, weights: &[f32]) -> usize {
        let total: f32 = weights.iter().sum();
        if total <= 0.0 {
            return 0;
        }
        let mut t = self.f32() * total;
        for (i, &w) in weights.iter().enumerate() {
            t -= w;
            if t < 0.0 {
                return i;
            }
        }
        weights.len() - 1
    }

    pub fn weighted_by<T>(&mut self, items: &[T], weight: impl Fn(&T) -> f32) -> Option<usize> {
        let total: f32 = items.iter().map(&weight).sum();
        if total <= 0.0 {
            return None;
        }
        let mut t = self.f32() * total;
        for (i, item) in items.iter().enumerate() {
            t -= weight(item);
            if t < 0.0 {
                return Some(i);
            }
        }
        Some(items.len() - 1)
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_streams_are_reproducible_and_distinct() {
        let a: Vec<u64> = (0..8).map(|_| 0).scan(Rng::keyed(&[1, 2, 3]), |r, _| Some(r.next_u64())).collect();
        let b: Vec<u64> = (0..8).map(|_| 0).scan(Rng::keyed(&[1, 2, 3]), |r, _| Some(r.next_u64())).collect();
        let c: Vec<u64> = (0..8).map(|_| 0).scan(Rng::keyed(&[1, 2, 4]), |r, _| Some(r.next_u64())).collect();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn uniform_moments() {
        let mut r = Rng::new(7);
        let n = 200_000;
        let mean: f32 = (0..n).map(|_| r.f32()).sum::<f32>() / n as f32;
        assert!((mean - 0.5).abs() < 0.01);
        let nm: f32 = (0..n).map(|_| r.normal()).sum::<f32>() / n as f32;
        assert!(nm.abs() < 0.02);
    }
}
