//! Fixed-point helpers. Positions and velocities are `i32` sub-pixels.

use serde::{Deserialize, Serialize};

/// Sub-pixels per pixel (fixed-point scale for positions and velocities).
pub const SUB: i32 = 256;

/// `1.0` in Q16 fixed point, the format `trig` returns.
pub const Q16_ONE: i32 = 1 << 16;

/// A 2D vector in sub-pixels.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct FxVec2 {
    pub x: i32,
    pub y: i32,
}

impl FxVec2 {
    pub const ZERO: Self = Self { x: 0, y: 0 };

    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// From whole pixels.
    pub const fn from_px(x: i32, y: i32) -> Self {
        Self {
            x: x * SUB,
            y: y * SUB,
        }
    }

    /// Squared length, widened so it cannot overflow.
    pub fn length_squared(self) -> i64 {
        let (x, y) = (self.x as i64, self.y as i64);
        x * x + y * y
    }

    /// Length, via integer square root (deterministic on every platform).
    pub fn length(self) -> i64 {
        self.length_squared().isqrt()
    }

    /// This vector rescaled to length `len` (zero stays zero). Integer-only,
    /// so aiming at a point needs no trig.
    pub fn scale_to(self, len: i32) -> Self {
        let l = self.length();
        if l == 0 {
            return Self::ZERO;
        }
        Self::new(
            (self.x as i64 * len as i64 / l) as i32,
            (self.y as i64 * len as i64 / l) as i32,
        )
    }
}

impl core::ops::Add for FxVec2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl core::ops::Sub for FxVec2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

/// Multiply a value by a Q16 factor (e.g. a `trig` result).
pub fn mul_q16(v: i32, q16: i32) -> i32 {
    ((v as i64 * q16 as i64) >> 16) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_uses_integer_sqrt() {
        assert_eq!(FxVec2::new(3 * SUB, 4 * SUB).length(), (5 * SUB) as i64);
        assert_eq!(mul_q16(1000, Q16_ONE / 2), 500);
        assert_eq!(mul_q16(-1000, Q16_ONE / 2), -500);
        assert_eq!(FxVec2::new(0, -900).scale_to(30), FxVec2::new(0, -30));
        assert_eq!(FxVec2::ZERO.scale_to(30), FxVec2::ZERO);
    }
}
