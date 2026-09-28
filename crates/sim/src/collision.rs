//! Integer collision shapes and overlap tests, replacing the prototype's Box2D
//! (`src/general/physics.lua`). Only what the prototype actually uses: circles
//! and axis-aligned squares, "hit" on overlap, and pushing bodies apart so
//! nothing walks through obstacles. No rotation, restitution or impulses.

use crate::fixed::FxVec2;

/// A collision shape centred on the body position. Sizes in sub-pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Circle {
        r: i32,
    },
    /// Axis-aligned square with half side `half`.
    Square {
        half: i32,
    },
}

impl Shape {
    /// Half extent of the bounding box.
    pub fn extent(self) -> i32 {
        match self {
            Shape::Circle { r } => r,
            Shape::Square { half } => half,
        }
    }
}

/// `v` rescaled to length `len` (zero if `v` is zero). Integer-only.
pub fn scale_to(v: FxVec2, len: i32) -> FxVec2 {
    let l = v.length();
    if l == 0 {
        return FxVec2::ZERO;
    }
    FxVec2::new(
        (v.x as i64 * len as i64 / l) as i32,
        (v.y as i64 * len as i64 / l) as i32,
    )
}

/// If the shapes overlap, the vector `a` must move by to separate from `b`
/// (or `b` by its negation). `None` if they don't touch.
pub fn contact(a: Shape, pa: FxVec2, b: Shape, pb: FxVec2) -> Option<FxVec2> {
    match (a, b) {
        (Shape::Circle { r: ra }, Shape::Circle { r: rb }) => {
            let d = pa - pb;
            let sum = (ra + rb) as i64;
            if d.length_squared() >= sum * sum {
                return None;
            }
            let dist = d.length();
            let depth = (sum - dist) as i32;
            // Coincident centres: pick a fixed direction so the result is deterministic.
            Some(if dist == 0 {
                FxVec2::new(depth, 0)
            } else {
                scale_to(d, depth)
            })
        }
        (Shape::Circle { r }, Shape::Square { half }) => circle_square(pa, r, pb, half),
        (Shape::Square { half }, Shape::Circle { r }) => {
            circle_square(pb, r, pa, half).map(|v| FxVec2::ZERO - v)
        }
        // Squares are only ever static obstacles, which never collide with
        // each other (Box2D skips static/static pairs).
        (Shape::Square { .. }, Shape::Square { .. }) => None,
    }
}

/// Push-out vector for circle (`c`, `r`) against square (`s`, `half`).
fn circle_square(c: FxVec2, r: i32, s: FxVec2, half: i32) -> Option<FxVec2> {
    let q = FxVec2::new(
        c.x.clamp(s.x - half, s.x + half),
        c.y.clamp(s.y - half, s.y + half),
    );
    let d = c - q;
    if d == FxVec2::ZERO {
        // Centre inside the square: leave through the nearest edge.
        let rel = c - s;
        let (ex, ey) = (half - rel.x.abs(), half - rel.y.abs());
        return Some(if ex <= ey {
            FxVec2::new(if rel.x >= 0 { ex + r } else { -(ex + r) }, 0)
        } else {
            FxVec2::new(0, if rel.y >= 0 { ey + r } else { -(ey + r) })
        });
    }
    let dist_sq = d.length_squared();
    if dist_sq >= r as i64 * r as i64 {
        return None;
    }
    Some(scale_to(d, r - d.length() as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::SUB;

    #[test]
    fn circles() {
        let a = Shape::Circle { r: 10 * SUB };
        let b = Shape::Circle { r: 5 * SUB };
        assert_eq!(
            contact(a, FxVec2::from_px(0, 0), b, FxVec2::from_px(20, 0)),
            None
        );
        let push = contact(a, FxVec2::from_px(0, 0), b, FxVec2::from_px(12, 0)).unwrap();
        assert_eq!(push, FxVec2::from_px(-3, 0));
    }

    #[test]
    fn circle_vs_square() {
        let c = Shape::Circle { r: 10 * SUB };
        let s = Shape::Square { half: 100 * SUB };
        // Touching the right face.
        let push = contact(c, FxVec2::from_px(105, 0), s, FxVec2::ZERO).unwrap();
        assert_eq!(push, FxVec2::from_px(5, 0));
        // Same pair, other order: opposite vector.
        let push2 = contact(s, FxVec2::ZERO, c, FxVec2::from_px(105, 0)).unwrap();
        assert_eq!(push2, FxVec2::from_px(-5, 0));
        // Centre inside, nearest edge is the top.
        let push = contact(c, FxVec2::from_px(0, 95), s, FxVec2::ZERO).unwrap();
        assert_eq!(push, FxVec2::from_px(0, 15));
        // Clear of the corner.
        assert_eq!(contact(c, FxVec2::from_px(108, 108), s, FxVec2::ZERO), None);
    }
}
