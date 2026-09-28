//! Seeded RNG that is part of the rolled-back simulation state.

use serde::{Deserialize, Serialize};

/// xorshift64*. Rolled back and checksummed like any other state; never seed
/// it from the clock.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SimRng(pub u64);

impl SimRng {
    pub fn new(seed: u64) -> Self {
        // splitmix64 so neighbouring seeds give unrelated streams, then avoid
        // the all-zero state. (The spike's `seed ^ C | 1` made seeds 2k and
        // 2k+1 identical.)
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Self(if z == 0 { 1 } else { z })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// Uniform-ish integer in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_u32() % (hi - lo + 1) as u32) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (SimRng::new(42), SimRng::new(42));
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        // Neighbouring seeds must not collide.
        for seed in 0..64 {
            assert_ne!(SimRng::new(seed), SimRng::new(seed + 1));
        }
    }

    #[test]
    fn range_is_inclusive() {
        let mut r = SimRng::new(1);
        for _ in 0..1000 {
            let v = r.range(-4, 4);
            assert!((-4..=4).contains(&v));
        }
    }
}
