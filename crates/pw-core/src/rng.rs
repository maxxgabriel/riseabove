//! Deterministic RNG. There is no global generator: every consumer derives a
//! stream from `(world seed, system, entity, date, …)`, so results never depend
//! on iteration order or thread scheduling.

/// System stream tags, mixed into keyed seeds so systems never share streams.
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
