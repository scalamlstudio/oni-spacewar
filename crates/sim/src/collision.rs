//! Integer circle overlap and separation. Deterministic on every platform:
//! no floats, lengths via `i64::isqrt`.

use crate::fixed::FxVec2;

/// Do circles (`pa`, `ra`) and (`pb`, `rb`) overlap? Radii in sub-pixels.
pub fn overlaps(pa: FxVec2, ra: i32, pb: FxVec2, rb: i32) -> bool {
    let sum = (ra + rb) as i64;
    (pa - pb).length_squared() < sum * sum
}

/// If the circles overlap, the vector `a` must move by to separate from `b`
/// (or `b` by its negation). `None` if they don't touch.
pub fn separation(pa: FxVec2, ra: i32, pb: FxVec2, rb: i32) -> Option<FxVec2> {
    if !overlaps(pa, ra, pb, rb) {
        return None;
    }
    let d = pa - pb;
    let depth = ra + rb - d.length() as i32;
    // Coincident centres: pick a fixed direction so the result is deterministic.
    Some(if d == FxVec2::ZERO {
        FxVec2::new(depth, 0)
    } else {
        d.scale_to(depth)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::SUB;

    #[test]
    fn circles() {
        let (ra, rb) = (10 * SUB, 5 * SUB);
        let o = FxVec2::ZERO;
        assert_eq!(separation(o, ra, FxVec2::from_px(20, 0), rb), None);
        assert_eq!(
            separation(o, ra, FxVec2::from_px(12, 0), rb),
            Some(FxVec2::from_px(-3, 0))
        );
        assert_eq!(separation(o, ra, o, rb), Some(FxVec2::from_px(15, 0)));
    }
}
