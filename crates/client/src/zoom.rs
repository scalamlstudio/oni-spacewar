//! Mouse-wheel zoom for the local camera, shared by the Carrier and the
//! battle (TAKOAI-94). Client only: it moves and scales the camera and never
//! touches sim, rollback or network state, so no clock rule applies.
//!
//! Zoom is screen px per world px, relative to a scene's base view (the
//! Carrier's 1:1, the battle's `render::BATTLE_ZOOM`). The wheel sets a
//! target in [`MIN`]..=[`MAX`]; the shown zoom eases toward it, and every
//! frame's change is made about the cursor, so the world point under the
//! cursor stays put. The camera then follows its target (the Pilot, our
//! ship, or the Build-mode pan) plus the `offset` that zooming about the
//! cursor leaves, kept small enough that the target stays on screen.
//!
//! The UI (HUD, panels) is not drawn by this camera's projection, so it
//! keeps its size. Picking goes through `Camera::viewport_to_world_2d`,
//! which reads the same transform and projection; [`world_at`] is the same
//! maths for tests and for this module's own anchoring.

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

/// Zoom limits. 0.5 is the spec's Overview (design/READINESS.md § Carrier).
pub const MIN: f32 = 0.5;
pub const MAX: f32 = 2.0;
/// One wheel notch multiplies the target by this (2^¼: eight notches from
/// `MIN` to `MAX`).
pub const STEP: f32 = 1.189_207_1;
/// Pixel-unit scroll (trackpads) per notch.
const PX_PER_NOTCH: f32 = 100.0;
/// Easing rate, 1/s: the shown zoom covers ~1 − e^(−EASE·dt) of the way
/// (in log space) to the target each frame.
const EASE: f32 = 14.0;
/// The followed target stays inside this share of the half view.
const KEEP_IN_VIEW: f32 = 0.8;

/// A camera's zoom: the target the wheel sets, the zoom shown now, and the
/// camera's offset from what it follows (world px).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Zoom {
    pub target: f32,
    pub shown: f32,
    pub offset: Vec2,
}

impl Default for Zoom {
    fn default() -> Self {
        Self::at(1.0)
    }
}

impl Zoom {
    /// Settled at `zoom`, centred on the target.
    pub fn at(zoom: f32) -> Self {
        let zoom = zoom.clamp(MIN, MAX);
        Self {
            target: zoom,
            shown: zoom,
            offset: Vec2::ZERO,
        }
    }

    /// Jumps straight to `zoom` (Build mode's Overview), recentred.
    pub fn set(&mut self, zoom: f32) {
        *self = Self::at(zoom);
    }

    /// `notches` of wheel (positive: zoom in) on the target.
    pub fn scroll(&mut self, notches: f32) {
        self.target = (self.target * STEP.powf(notches)).clamp(MIN, MAX);
    }

    /// One frame: eases the shown zoom toward the target over `dt` s,
    /// keeping the world point under `cursor` (screen px from the window's
    /// top left) put. `base` is the scene's screen px per world px at zoom
    /// one. `centre` is where the camera is now and the returned value
    /// where it goes; `offset` moves with it.
    pub fn update(
        &mut self,
        dt: f32,
        base: f32,
        centre: Vec2,
        cursor: Option<Vec2>,
        window: Vec2,
    ) -> Vec2 {
        let next = ease(self.shown, self.target, dt);
        if next == self.shown {
            return centre;
        }
        let moved = match cursor {
            Some(c) => zoom_about(centre, c, window, base * self.shown, base * next),
            None => centre,
        };
        self.offset += moved - centre;
        self.shown = next;
        moved
    }

    /// Clamps `offset` so the followed target stays on screen at `half`
    /// (world px) view.
    pub fn keep_in_view(&mut self, half: Vec2) {
        self.offset = self.offset.clamp(-half * KEEP_IN_VIEW, half * KEEP_IN_VIEW);
    }
}

/// Wheel notches this frame (positive: away from the user, zoom in).
pub fn notches(scroll: &AccumulatedMouseScroll) -> f32 {
    match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / PX_PER_NOTCH,
    }
}

/// `shown` eased toward `target` over `dt` s, in log space so zooming in
/// and out feel alike; snaps once within 0.1 %.
pub fn ease(shown: f32, target: f32, dt: f32) -> f32 {
    if (shown / target - 1.0).abs() < 1e-3 {
        return target;
    }
    let t = 1.0 - (-EASE * dt).exp();
    shown * (target / shown).powf(t)
}

/// The world point (Bevy, y up) at `screen` px (from the top left of a
/// `window`-sized view) when the camera is at `centre` with `zoom` screen
/// px per world px.
pub fn world_at(screen: Vec2, window: Vec2, centre: Vec2, zoom: f32) -> Vec2 {
    let d = screen - window / 2.0;
    centre + Vec2::new(d.x, -d.y) / zoom
}

/// The camera centre that keeps the world point under `cursor` where it is
/// when the zoom changes from `from` to `to`.
pub fn zoom_about(centre: Vec2, cursor: Vec2, window: Vec2, from: f32, to: f32) -> Vec2 {
    let anchor = world_at(cursor, window, centre, from);
    let d = cursor - window / 2.0;
    anchor - Vec2::new(d.x, -d.y) / to
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::{ComputedCameraValues, RenderTargetInfo};

    const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

    #[test]
    fn wheel_steps_and_clamps() {
        let mut z = Zoom::default();
        z.scroll(2.0);
        assert!((z.target - 2f32.sqrt()).abs() < 1e-4, "{}", z.target);
        z.scroll(100.0);
        assert_eq!(z.target, MAX);
        z.scroll(-100.0);
        assert_eq!(z.target, MIN);
        // Eight notches span the whole range.
        z.scroll(8.0);
        assert!((z.target - MAX).abs() < 1e-4);
    }

    #[test]
    fn easing_is_smooth_and_settles() {
        let mut shown = 1.0;
        let mut last = shown;
        for _ in 0..120 {
            shown = ease(shown, 2.0, 1.0 / 60.0);
            assert!(shown >= last && shown <= 2.0);
            // No single frame jumps more than a notch.
            assert!(shown / last <= STEP);
            last = shown;
        }
        assert_eq!(shown, 2.0);
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor() {
        let centre = Vec2::new(300.0, -200.0);
        for cursor in [Vec2::ZERO, Vec2::new(1000.0, 100.0), WINDOW / 2.0, WINDOW] {
            let before = world_at(cursor, WINDOW, centre, 1.0);
            for to in [MIN, 0.8, 1.5, MAX] {
                let c = zoom_about(centre, cursor, WINDOW, 1.0, to);
                let after = world_at(cursor, WINDOW, c, to);
                assert!(before.distance(after) < 1e-3, "{cursor} {to}");
            }
        }
        // Through `update`, frame by frame while it eases.
        let mut z = Zoom::default();
        z.scroll(-8.0);
        let cursor = Vec2::new(200.0, 600.0);
        let anchor = world_at(cursor, WINDOW, centre, 1.0);
        let mut c = centre;
        for _ in 0..90 {
            c = z.update(1.0 / 60.0, 1.0, c, Some(cursor), WINDOW);
            assert!(world_at(cursor, WINDOW, c, z.shown).distance(anchor) < 0.01);
        }
        assert_eq!(z.shown, MIN);
        assert_eq!(z.offset, c - centre);
        // At the battle's base scale too.
        let base = 1.0 / crate::render::BATTLE_ZOOM;
        let mut z = Zoom::default();
        z.scroll(8.0);
        let anchor = world_at(cursor, WINDOW, centre, base);
        let mut c = centre;
        for _ in 0..90 {
            c = z.update(1.0 / 60.0, base, c, Some(cursor), WINDOW);
            assert!(world_at(cursor, WINDOW, c, base * z.shown).distance(anchor) < 0.01);
        }
        assert_eq!(z.shown, MAX);
        // The cursor at the centre zooms straight in.
        let c = zoom_about(centre, WINDOW / 2.0, WINDOW, 1.0, 2.0);
        assert_eq!(c, centre);
    }

    #[test]
    fn the_followed_target_stays_on_screen() {
        let mut z = Zoom {
            offset: Vec2::new(5000.0, -5000.0),
            ..Zoom::default()
        };
        let half = WINDOW / 2.0 / MAX;
        z.keep_in_view(half);
        assert!(z.offset.x < half.x && -z.offset.y < half.y);
        z.set(MIN);
        assert_eq!(z, Zoom::at(MIN));
    }

    /// A camera as Bevy sets it up for a `WINDOW` view at `zoom`.
    fn camera(zoom: f32) -> Camera {
        let mut ortho = OrthographicProjection {
            scale: 1.0 / zoom,
            ..OrthographicProjection::default_2d()
        };
        bevy::camera::CameraProjection::update(&mut ortho, WINDOW.x, WINDOW.y);
        Camera {
            computed: ComputedCameraValues {
                clip_from_view: bevy::camera::CameraProjection::get_clip_from_view(&ortho),
                target_info: Some(RenderTargetInfo {
                    physical_size: WINDOW.as_uvec2(),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        }
    }

    #[test]
    fn picking_matches_bevy_at_every_zoom() {
        let centre = Vec2::new(700.0, -450.0);
        let tf = GlobalTransform::from(Transform::from_translation(centre.extend(0.0)));
        for zoom in [MIN, 1.0, MAX] {
            let cam = camera(zoom);
            for screen in [Vec2::ZERO, Vec2::new(17.0, 650.0), WINDOW / 2.0, WINDOW] {
                let bevy = cam.viewport_to_world_2d(&tf, screen).unwrap();
                let ours = world_at(screen, WINDOW, centre, zoom);
                assert!(bevy.distance(ours) < 0.01, "{zoom} {screen}: {bevy} {ours}");
            }
            // A click on a tile's centre on screen picks that tile.
            for tile in [(9, 6), (10, 7), (12, 5)] {
                let tile_centre = Vec2::new(
                    (tile.0 as f32 + 0.5) * crate::layout::TILE,
                    -(tile.1 as f32 + 0.5) * crate::layout::TILE,
                );
                let d = (tile_centre - centre) * zoom;
                let screen = WINDOW / 2.0 + Vec2::new(d.x, -d.y);
                let p = cam.viewport_to_world_2d(&tf, screen).unwrap();
                assert_eq!(crate::layout::tile_at(p.x, -p.y), tile, "{zoom}");
            }
        }
    }
}
