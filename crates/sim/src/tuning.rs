//! Gameplay numbers, in human units (px, px/second, seconds). Values come
//! from design/READINESS.md § Demo Spec; anything the spec doesn't give is
//! marked "engine default" and is ours to tune. Convert to sim units at use
//! with [`px_per_tick`] and [`ticks`].

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

/// `base` raised by `pct_per_level`% per level, integer math rounded down
/// (Demo Spec § Workshop upgrades).
pub const fn upgraded(base: i32, pct_per_level: i32, level: u8) -> i32 {
    base * (100 + pct_per_level * level as i32) / 100
}

// --- Arena (Demo Spec § Elimination mission) --------------------------------

/// The arena is a rectangle centred on the origin; edges block movement.
pub const ARENA_HALF_W: i32 = 800;
pub const ARENA_HALF_H: i32 = 600;

// --- Battleships (Demo Spec § Battleships) ----------------------------------

/// Ship/enemy separation weighting (engine default).
pub const SHIP_MASS: i32 = 5;
/// Horizontal gap between battleship spawn points (engine default).
pub const SHIP_SPAWN_GAP: i32 = 60;

pub const KITE_HULL: i32 = 60;
pub const KITE_SPEED: i32 = 220;
pub const KITE_RADIUS: i32 = 14;
pub const KITE_BASIC_DAMAGE: i32 = 4;
/// Basic attack interval as seconds `num/den`: 0.35 s.
pub const KITE_BASIC_INTERVAL: u32 = ticks(35, 100);
pub const KITE_BASIC_RANGE: i32 = 220;

pub const BULWARK_HULL: i32 = 140;
pub const BULWARK_SPEED: i32 = 140;
pub const BULWARK_RADIUS: i32 = 20;
pub const BULWARK_BASIC_DAMAGE: i32 = 8;
pub const BULWARK_BASIC_INTERVAL: u32 = ticks(6, 10);
pub const BULWARK_BASIC_RANGE: i32 = 240;

/// Basic-attack bolt (engine default: the spec gives damage/rate/range only).
pub const BOLT_SPEED: i32 = 900;
pub const BOLT_RADIUS: i32 = 4;

/// Kite Q — Afterburn: dash 160 px toward the cursor.
pub const AFTERBURN_DISTANCE: i32 = 160;
/// The dash is spread over this many ticks so it reads on screen (engine default).
pub const AFTERBURN_TICKS: u32 = 8;
pub const AFTERBURN_COOLDOWN: u32 = ticks(4, 1);

/// Kite W — Scatter: 5 bolts in a 40° cone.
pub const SCATTER_BOLTS: i32 = 5;
/// Angle between neighbouring bolts in `trig::ANGLE_STEPS` units:
/// 10° = 1024 * 10 / 360 ≈ 28 steps (9.84°), so the cone is ~39.4°.
pub const SCATTER_STEP: i32 = 28;
pub const SCATTER_DAMAGE: i32 = 6;
pub const SCATTER_SPEED: i32 = 600;
pub const SCATTER_LIFETIME: u32 = ticks(1, 2);
pub const SCATTER_RADIUS: i32 = 3;
pub const SCATTER_COOLDOWN: u32 = ticks(6, 1);

/// Bulwark Q — Bastion: shield absorbs the next 40 damage for 5 s.
pub const BASTION_SHIELD: i32 = 40;
pub const BASTION_DURATION: u32 = ticks(5, 1);
pub const BASTION_COOLDOWN: u32 = ticks(12, 1);

/// Bulwark W — Shockwave: 15 damage within 150 px, pushes 100 px away.
pub const SHOCKWAVE_DAMAGE: i32 = 15;
pub const SHOCKWAVE_RADIUS: i32 = 150;
pub const SHOCKWAVE_PUSH: i32 = 100;
pub const SHOCKWAVE_COOLDOWN: u32 = ticks(9, 1);

// --- Workshop upgrades (Demo Spec § Workshop upgrades) -----------------------

pub const HULL_PCT_PER_LEVEL: i32 = 25;
pub const WEAPON_PCT_PER_LEVEL: i32 = 25;
pub const THRUSTER_PCT_PER_LEVEL: i32 = 15;
pub const MAX_UPGRADE_LEVEL: u8 = 2;

// --- Enemies (Demo Spec § Enemies) ------------------------------------------

pub const ENEMY_MASS: i32 = 1;

pub const SWARMER_HP: i32 = 10;
pub const SWARMER_SPEED: i32 = 110;
pub const SWARMER_RADIUS: i32 = 10;
pub const SWARMER_CONTACT_DAMAGE: i32 = 5;
pub const SWARMER_CONTACT_COOLDOWN: u32 = ticks(1, 1);
/// Contact reaches this far past touching, because separation keeps the
/// circles from overlapping (engine default).
pub const CONTACT_REACH: i32 = 2;

pub const SPITTER_HP: i32 = 36;
pub const SPITTER_SPEED: i32 = 60;
pub const SPITTER_RADIUS: i32 = 14;
pub const SPITTER_MIN_RANGE: i32 = 200;
pub const SPITTER_MAX_RANGE: i32 = 320;
pub const SPITTER_FIRE_RANGE: i32 = 360;
pub const SPITTER_FIRE_INTERVAL: u32 = ticks(5, 2);
/// Delay before a fresh Spitter's first shot (engine default).
pub const SPITTER_FIRST_SHOT: u32 = ticks(1, 1);
pub const SPIT_DAMAGE: i32 = 8;
pub const SPIT_SPEED: i32 = 160;
pub const SPIT_RADIUS: i32 = 5;
pub const SPIT_LIFETIME: u32 = ticks(3, 1);

// --- Elimination waves (Demo Spec § Elimination mission) --------------------

pub const KILL_TARGET: u32 = 20;
pub const WAVE_COUNT: u32 = 3;
/// "Wave N" banner pause before each wave spawns.
pub const WAVE_BANNER: u32 = ticks(4, 1);
/// The next wave comes this long after the current one spawned, even if
/// it isn't cleared.
pub const WAVE_TIMEOUT: u32 = ticks(25, 1);
pub const WAVE_RING_RADIUS: i32 = 450;

/// One spawn group of a wave: `(delay after the wave starts, swarmers, spitters)`.
pub type SpawnGroup = (u32, u32, u32);

/// Spawn groups for wave `n` (1-based).
pub const fn wave_groups(n: u32) -> &'static [SpawnGroup] {
    const WAVE_1: [SpawnGroup; 2] = [(0, 4, 0), (ticks(2, 1), 4, 0)];
    const WAVE_2_3: [SpawnGroup; 1] = [(0, 6, 2)];
    match n {
        1 => &WAVE_1,
        _ => &WAVE_2_3,
    }
}

// --- Loot (Demo Spec § Loot) -------------------------------------------------

pub const SWARMER_CREDITS: u32 = 5;
pub const SWARMER_CRYSTAL_PCT: i32 = 15;
pub const SPITTER_CREDITS: u32 = 15;
pub const SPITTER_CRYSTAL_PCT: i32 = 60;
/// Mission success bonus. Tuned up from the spec's 50 cr + 2 VC (TAKOAI-42):
/// played runs kill mostly Swarmers and lose a few expired drops, so they
/// collected ~70-100 cr and 1-5 VC and the spec's 50 + 2 bonus left a win
/// short of the first upgrade (150 cr + 4 VC). With 100 + 3 a win pays
/// ~170-200 cr and 4-8 VC, the spec's expected ~180 cr / ~6 VC haul.
pub const SUCCESS_BONUS_CREDITS: u32 = 100;
pub const SUCCESS_BONUS_CRYSTAL: u32 = 3;
pub const PICKUP_COLLECT_RANGE: i32 = 32;
pub const PICKUP_DRIFT_RANGE: i32 = 80;
/// Drift speed toward a nearby ship (engine default).
pub const PICKUP_DRIFT_SPEED: i32 = 240;
pub const PICKUP_LIFETIME: u32 = ticks(20, 1);
/// A Void Crystal drop lands this far from the credits drop (visual, engine default).
pub const CRYSTAL_DROP_OFFSET: i32 = 8;
