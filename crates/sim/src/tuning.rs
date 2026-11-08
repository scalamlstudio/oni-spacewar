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

// --- Battlefield (Demo Spec § Elimination mission) --------------------------

// There is no arena edge (TAKOAI-58): nothing clamps ships, enemies or
// shots. Shots end by lifetime; fissures are placed on a ring around the
// ships' start.

/// Half the battle view at the start of a mission, centred on the spawn
/// point: the client's default window (1000 × 580) at its battle zoom
/// (1.265), rounded. Only used to keep fissures off the starting screen;
/// the client checks it matches its camera.
pub const INITIAL_VIEW_HALF_W: i32 = 633;
pub const INITIAL_VIEW_HALF_H: i32 = 367;

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
/// Knock-back on an enemy hit by a bolt or a Scatter bolt, px along the
/// shot's flight (TAKOAI-112; design/BATTLE.md #1: "a small knock-back").
pub const BOLT_KNOCKBACK: i32 = 5;
pub const SCATTER_KNOCKBACK: i32 = 4;

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

// --- Carrier rooms (design/READINESS.md § Carrier › Room catalogue) ----------

/// Training Room: Q and W cooldowns × 85 / 100, rounded down. One level.
pub const TRAINING_COOLDOWN_PCT: u32 = 85;
pub const MAX_TRAINING_LEVEL: u8 = 1;

/// A skill cooldown in ticks after `training` Training Room levels.
pub const fn trained(cooldown: u32, training: u8) -> u32 {
    if training > 0 {
        cooldown * TRAINING_COOLDOWN_PCT / 100
    } else {
        cooldown
    }
}

// --- Enemies (Demo Spec § Enemies) ------------------------------------------

pub const ENEMY_MASS: i32 = 1;

/// One basic hit from any ship kills a Swarmer (TAKOAI-113, design/BATTLE.md
/// #2 and B1): Kite's unupgraded bolt does 4. Was 10 (3 Kite hits).
pub const SWARMER_HP: i32 = 4;
/// +20% over the spec's 110 (TAKOAI-58).
pub const SWARMER_SPEED: i32 = 132;
pub const SWARMER_RADIUS: i32 = 10;
pub const SWARMER_CONTACT_DAMAGE: i32 = 5;
pub const SWARMER_CONTACT_COOLDOWN: u32 = ticks(1, 1);
/// Contact reaches this far past touching, because separation keeps the
/// circles from overlapping (engine default).
pub const CONTACT_REACH: i32 = 2;

pub const SPITTER_HP: i32 = 36;
/// +20% over the spec's 60 (TAKOAI-58).
pub const SPITTER_SPEED: i32 = 72;
pub const SPITTER_RADIUS: i32 = 14;
pub const SPITTER_MIN_RANGE: i32 = 200;
pub const SPITTER_MAX_RANGE: i32 = 320;
pub const SPITTER_FIRE_RANGE: i32 = 360;
/// Time from one shot to the next, wind-up included.
pub const SPITTER_FIRE_INTERVAL: u32 = ticks(5, 2);
/// The Spitter stops, glows and shows its aim line for this long before
/// each shot (TAKOAI-113, design/BATTLE.md #3): 0.4 s. The shot's direction
/// is locked when the wind-up starts, so moving off the line dodges it.
pub const SPITTER_WINDUP: u32 = ticks(2, 5);
/// Delay before a fresh Spitter's first shot (engine default).
pub const SPITTER_FIRST_SHOT: u32 = ticks(1, 1);
pub const SPIT_DAMAGE: i32 = 8;
pub const SPIT_SPEED: i32 = 160;
pub const SPIT_RADIUS: i32 = 5;
pub const SPIT_LIFETIME: u32 = ticks(3, 1);

// --- Elimination: void fissures and the spawn director ----------------------

/// Normal play's kill target. Raised from 40 with packs and one-hit
/// Swarmers (TAKOAI-113): the scripted QA pilot wins in 60-77 s over 12
/// seeds per ship (target 60-90 s). The sim reads the
/// mission's own [`Mission::kill_target`](crate::Mission::kill_target), set
/// at launch (`SimState::with_kill_target`); this is only its default.
pub const KILL_TARGET: u32 = 100;
/// `--autoplay` / test runs' kill target (TAKOAI-77): shorter QA battles.
pub const AUTOPLAY_KILL_TARGET: u32 = 10;

/// Each mission places `FISSURES_MIN..=FISSURES_MAX` void fissures.
pub const FISSURES_MIN: i32 = 1;
pub const FISSURES_MAX: i32 = 2;
/// Fissure size for drawing (px). Fissures can't be hit or destroyed.
pub const FISSURE_RADIUS: i32 = 40;
/// Fissures sit on a ring around the ships' start (the origin), this far
/// away (px, inclusive), at a seeded angle. The ring's inner edge is past
/// the starting view everywhere except near its corners; a draw that lands
/// inside the view (plus `FISSURE_OFFSCREEN_MARGIN`) is rejected.
pub const FISSURE_RING_MIN: i32 = 900;
pub const FISSURE_RING_MAX: i32 = 1300;
/// A fissure's centre stays at least this far outside the starting view
/// (its radius plus the widest 4-ship spawn spread, 90 px).
pub const FISSURE_OFFSCREEN_MARGIN: i32 = 130;
/// Two fissures are at least this far apart.
pub const FISSURE_MIN_GAP: i32 = 600;
/// Enemies appear on a ring this far from a fissure's centre.
pub const FISSURE_SPAWN_RING: i32 = 60;

/// First pack this long after the mission starts.
pub const SPAWN_FIRST: u32 = ticks(2, 1);
/// The director releases a pack out of one fissure on a pulse
/// (TAKOAI-113, design/BATTLE.md #2) instead of one enemy at a time. The
/// gap between packs falls linearly from `PACK_INTERVAL_START` to
/// `PACK_INTERVAL_END` over `SPAWN_RAMP`, then stays there.
pub const PACK_INTERVAL_START: u32 = ticks(6, 1);
pub const PACK_INTERVAL_END: u32 = ticks(7, 2);
/// Each pack has `PACK_SWARMERS_MIN..=PACK_SWARMERS_MAX` Swarmers ...
pub const PACK_SWARMERS_MIN: i32 = 6;
pub const PACK_SWARMERS_MAX: i32 = 10;
/// ... and `PACK_SPITTER_ROLLS` chances to add a Spitter, each with
/// `spitter_pct`% chance: `SPITTER_PCT_START`% rising to `SPITTER_PCT_END`%
/// over the same ramp, so early packs rarely bring one and late packs
/// usually bring 1-2.
pub const PACK_SPITTER_ROLLS: u32 = 2;
pub const SPITTER_PCT_START: i32 = 20;
pub const SPITTER_PCT_END: i32 = 60;
pub const SPAWN_RAMP: u32 = ticks(90, 1);
/// Most enemies alive at once. A pack that would go past it brings only
/// what fits; with the field full the director waits. Raised from 36 with
/// packs (TAKOAI-113).
pub const MAX_LIVE_ENEMIES: usize = 60;

/// `start` moved toward `end` by the fraction of the ramp elapsed at tick `t`.
const fn ramp(start: i64, end: i64, t: u32) -> i64 {
    let t = if t < SPAWN_RAMP { t } else { SPAWN_RAMP } as i64;
    start + (end - start) * t / SPAWN_RAMP as i64
}

/// Ticks until the next pack, for a pack at mission tick `t`.
pub const fn pack_interval(t: u32) -> u32 {
    ramp(PACK_INTERVAL_START as i64, PACK_INTERVAL_END as i64, t) as u32
}

/// Chance (percent) of each Spitter roll for a pack at mission tick `t`.
pub const fn spitter_pct(t: u32) -> i32 {
    ramp(SPITTER_PCT_START as i64, SPITTER_PCT_END as i64, t) as i32
}

// --- Loot (Demo Spec § Loot) -------------------------------------------------

/// Lowered from 5 cr / 15% with packs (TAKOAI-113): a win kills ~2.5×
/// the Swarmers, so the haul per win stays where it was (QA pilot:
/// ~170-260 cr and 4-11 VC with the bonus).
pub const SWARMER_CREDITS: u32 = 2;
pub const SWARMER_CRYSTAL_PCT: i32 = 5;
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
