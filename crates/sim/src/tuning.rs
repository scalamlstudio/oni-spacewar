//! Placeholder gameplay numbers, in human units (px, px/second, seconds).
//! None of these are design values: they exist so the foundation has
//! something to simulate, and will be replaced as real gameplay lands.
//! Convert to sim units at use with [`px_per_tick`] and [`ticks`].

use crate::fixed::SUB;

/// Simulation ticks per second (bevy_ggrs `RollbackFrameRate`).
pub const TICKS_PER_SEC: i32 = 60;

/// px/second -> sub-pixels/tick.
pub const fn px_per_tick(px_per_sec: i32) -> i32 {
    px_per_sec * SUB / TICKS_PER_SEC
}

/// Seconds (as a fraction `num/den`) -> ticks.
pub const fn ticks(num: i32, den: i32) -> u32 {
    (num * TICKS_PER_SEC / den) as u32
}

/// Battleship: slow, click-to-move (design/DESIGN.md § Player).
pub const SHIP_RADIUS: i32 = 16;
pub const SHIP_SPEED: i32 = 200;
pub const SHIP_MASS: i32 = 5;
/// Horizontal gap between battleship spawn points.
pub const SHIP_SPAWN_GAP: i32 = 60;

/// Placeholder skill on Q: a straight shot toward the cursor.
pub const SHOT_RADIUS: i32 = 3;
pub const SHOT_SPEED: i32 = 600;
pub const SHOT_LIFETIME: u32 = ticks(1, 1);
pub const SHOT_COOLDOWN: u32 = ticks(1, 6);
pub const SHOT_DAMAGE: i32 = 1;

/// Placeholder enemy: chases the nearest battleship.
pub const ENEMY_RADIUS: i32 = 10;
pub const ENEMY_SPEED: i32 = 80;
pub const ENEMY_HP: i32 = 3;
pub const ENEMY_MASS: i32 = 1;

/// When no enemies are left, a new wave spawns on a ring around the ships.
pub const WAVE_SIZE: usize = 8;
pub const WAVE_RING_RADIUS: i32 = 400;
