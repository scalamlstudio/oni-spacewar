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

// --- New enemy roles (TAKOAI-116, design/BATTLE.md § Enemy roles) ---------
//
// Behaviours, warning times and the numbers the design gives (Lancer dash
// ~400 px after 0.7 s; Bloater burst 1 s, ~90 px; Brood 3 Swarmers every
// ~6 s; Warden shield ~120 px, half the damage) are the design's starting
// points. HP, speeds, damage, cooldowns and loot are engine defaults to tune
// in playtests.

/// Lancer (charger): walks in, stops, shows its charge line, then dashes
/// along it. 6 Kite bolts / 3 Bulwark bolts.
pub const LANCER_HP: i32 = 24;
pub const LANCER_SPEED: i32 = 100;
pub const LANCER_RADIUS: i32 = 12;
/// It starts its wind-up once the ship is this close.
pub const LANCER_CHARGE_RANGE: i32 = 340;
/// The charge line shows this long before the dash: 0.7 s. The direction is
/// locked when the wind-up starts.
pub const LANCER_WINDUP: u32 = ticks(7, 10);
/// The dash: `LANCER_DASH_DISTANCE` px over `LANCER_DASH_TICKS`.
pub const LANCER_DASH_DISTANCE: i32 = 400;
pub const LANCER_DASH_TICKS: u32 = ticks(2, 5);
/// Damage to each ship it runs through (once per dash).
pub const LANCER_DASH_DAMAGE: i32 = 12;
/// Time after a dash before it can wind up again.
pub const LANCER_RECOVER: u32 = ticks(5, 2);
/// Delay before a fresh Lancer's first charge.
pub const LANCER_FIRST_CHARGE: u32 = ticks(1, 1);

/// Bloater (death-bomber): slow; bursts `BLOATER_FUSE` after it dies, or
/// after it touches a ship (it stops and swells). The burst hurts ships
/// and enemies alike, so it can be chained into packs.
pub const BLOATER_HP: i32 = 30;
pub const BLOATER_SPEED: i32 = 55;
pub const BLOATER_RADIUS: i32 = 16;
pub const BLOATER_FUSE: u32 = ticks(1, 1);
pub const BLOATER_BLAST_RADIUS: i32 = 90;
pub const BLOATER_BLAST_DAMAGE: i32 = 14;
/// Enough to kill any Swarmer, Lancer or Bloater caught in it.
pub const BLOATER_BLAST_ENEMY_DAMAGE: i32 = 30;

/// Brood (summoner): hangs back by the fissure it came from and releases
/// Swarmers; its sac pulses for `BROOD_WINDUP` first.
pub const BROOD_HP: i32 = 48;
pub const BROOD_SPEED: i32 = 40;
pub const BROOD_RADIUS: i32 = 20;
/// It holds this far out from its fissure, on the side of the nearest ship.
pub const BROOD_HOLD: i32 = 160;
pub const BROOD_SWARMERS: i32 = 3;
pub const BROOD_INTERVAL: u32 = ticks(6, 1);
pub const BROOD_WINDUP: u32 = ticks(1, 1);
/// Delay before a fresh Brood's first release (wind-up included after it).
pub const BROOD_FIRST_RELEASE: u32 = ticks(3, 1);
/// Released Swarmers appear this far from the Brood's centre.
pub const BROOD_RELEASE_RING: i32 = 30;

/// Warden (buffer): stays with its pack, out of the ship's face, and
/// shields every non-Warden ally within `WARDEN_RANGE`: they take half
/// damage (rounded up), shown by a tether.
pub const WARDEN_HP: i32 = 40;
pub const WARDEN_SPEED: i32 = 90;
pub const WARDEN_RADIUS: i32 = 15;
pub const WARDEN_RANGE: i32 = 120;
/// It backs off when a ship is closer than this.
pub const WARDEN_KEEP_AWAY: i32 = 220;

// --- Elimination: void fissures and the spawn director ----------------------

/// Each Elimination's kill target is rolled from the mission seed in
/// `KILL_TARGET_MIN..=KILL_TARGET_MAX` (TAKOAI-128; was a fixed 100). The
/// sim reads the mission's own
/// [`Mission::kill_target`](crate::Mission::kill_target)
/// (`mission::rolled_kill_target`, or `SimState::with_kill_target`).
pub const KILL_TARGET_MIN: u32 = 1;
/// Hard cap: nothing raises a kill target above this.
pub const KILL_TARGET_MAX: u32 = 100;
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

/// Scattered start (TAKOAI-130, Wei's Playtest #2): the field isn't empty
/// when a mission starts. `START_GROUPS_MIN..=START_GROUPS_MAX` small
/// dormant groups sit on a ring `START_RING_MIN..=START_RING_MAX` px round
/// the ships' start (the origin), at seeded angles, at least
/// `START_GROUP_GAP` px apart. The ring's inner edge keeps them far from
/// every ship's spawn (spawns are within 90 px of the origin) and outside
/// `START_WAKE_RANGE`, so nothing is awake on the first tick.
pub const START_GROUPS_MIN: i32 = 3;
pub const START_GROUPS_MAX: i32 = 5;
pub const START_RING_MIN: i32 = 480;
pub const START_RING_MAX: i32 = 1000;
pub const START_GROUP_GAP: i32 = 280;
/// Each group is `START_SWARMERS_MIN..=START_SWARMERS_MAX` Swarmers on a
/// ring `START_GROUP_RING` px round its centre, plus a Spitter
/// `START_SPITTER_PCT`% of the time. Plain enemies only: roles, elites and
/// the Guardian keep coming from the fissures as before.
pub const START_SWARMERS_MIN: i32 = 2;
pub const START_SWARMERS_MAX: i32 = 3;
pub const START_SPITTER_PCT: i32 = 25;
pub const START_GROUP_RING: i32 = 36;
/// A dormant enemy wakes when a ship comes within this many px, when it is
/// hit, or when something is hit or killed within `START_ALERT_RANGE` px of
/// it (its group wakes together).
pub const START_WAKE_RANGE: i32 = 320;
pub const START_ALERT_RANGE: i32 = 160;

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

/// Role packs (TAKOAI-116). Each new role unlocks once the kills reach its
/// share of the kill target (percent), so the roster grows over a mission
/// at any target: Lancer, Bloater, Warden, Brood in that order. The first
/// pack after an unlock always brings the new role; after that a pack is a
/// role pack `ROLE_PACK_PCT`% of the time, picked evenly among the unlocked
/// roles, and a plain Swarmer / Spitter pack otherwise.
pub const LANCER_UNLOCK_PCT: u32 = 10;
pub const BLOATER_UNLOCK_PCT: u32 = 25;
pub const WARDEN_UNLOCK_PCT: u32 = 40;
pub const BROOD_UNLOCK_PCT: u32 = 55;
pub const ROLE_PACK_PCT: i32 = 50;
/// Lancer pack: Swarmers plus 1-2 Lancers.
pub const LANCER_PACK_SWARMERS: (i32, i32) = (4, 6);
pub const LANCER_PACK_LANCERS: (i32, i32) = (1, 2);
/// Bloater pack: Swarmers plus 1-2 Bloaters walking among them.
pub const BLOATER_PACK_SWARMERS: (i32, i32) = (5, 8);
pub const BLOATER_PACK_BLOATERS: (i32, i32) = (1, 2);
/// Warden pack: Swarmers, sometimes a Spitter, and one Warden.
pub const WARDEN_PACK_SWARMERS: (i32, i32) = (6, 9);
/// Brood pack: one Brood behind 1-2 Spitters and a few Swarmers.
pub const BROOD_PACK_SPITTERS: (i32, i32) = (1, 2);
pub const BROOD_PACK_SWARMERS: (i32, i32) = (2, 3);

/// Elite packs (TAKOAI-117, design/BATTLE.md #6 and § Elite affixes).
/// Elites unlock once the kills reach `ELITE_UNLOCK_PCT`% of the target;
/// the first pack after that always has one, later packs
/// `ELITE_PACK_PCT`% of the time. The pack's first enemy (its role enemy
/// or a Spitter, else a Swarmer) becomes the leader: `ELITE_HP_MULT`× HP,
/// `ELITE_SIZE_PCT`% size and `ELITE_AFFIXES` affix(es) (Threat levels
/// will raise it to 2). Engine defaults to tune in playtests.
pub const ELITE_UNLOCK_PCT: u32 = 15;
pub const ELITE_PACK_PCT: i32 = 25;
pub const ELITE_HP_MULT: i32 = 4;
pub const ELITE_SIZE_PCT: i32 = 140;
pub const ELITE_AFFIXES: usize = 1;
/// An elite drops `ELITE_CREDITS_MULT`× its kind's credits and always
/// `ELITE_CRYSTALS` Void Crystal.
pub const ELITE_CREDITS_MULT: u32 = 4;
pub const ELITE_CRYSTALS: u32 = 2;
/// Swift: +40% speed.
pub const SWIFT_SPEED_PCT: i32 = 40;
/// Warding: every `WARDING_INTERVAL` it shields itself and allies within
/// `WARDING_RANGE` px for `WARDING_DURATION` (half damage).
pub const WARDING_FIRST: u32 = ticks(2, 1);
pub const WARDING_INTERVAL: u32 = ticks(5, 1);
pub const WARDING_DURATION: u32 = ticks(2, 1);
pub const WARDING_RANGE: i32 = 160;
/// Blinking: with the ship within `BLINK_RANGE` px it marks a spot
/// `BLINK_GAP` px off the ship's edge (on its own side) and lands there
/// `BLINK_MARK` later; then waits `BLINK_INTERVAL`.
pub const BLINK_FIRST: u32 = ticks(2, 1);
pub const BLINK_INTERVAL: u32 = ticks(5, 1);
pub const BLINK_MARK: u32 = ticks(1, 2);
pub const BLINK_RANGE: i32 = 700;
pub const BLINK_GAP: i32 = 24;
/// Splitting: `SPLIT_COPIES` normal copies, `SPLIT_OFFSET` px from where
/// it died.
pub const SPLIT_COPIES: i32 = 2;
pub const SPLIT_OFFSET: i32 = 20;
/// Scorching: a patch every `SCORCH_DROP_INTERVAL` that burns for
/// `SCORCH_LIFETIME`; a ship in any patch takes `SCORCH_DAMAGE` every
/// `SCORCH_TICK`.
pub const SCORCH_DROP_INTERVAL: u32 = ticks(1, 2);
pub const SCORCH_LIFETIME: u32 = ticks(3, 1);
pub const SCORCH_RADIUS: i32 = 26;
pub const SCORCH_DAMAGE: i32 = 2;
pub const SCORCH_TICK: u32 = ticks(1, 2);
/// Chilling: a hit slows the ship to `CHILL_SPEED_PCT`% for `CHILL_DURATION`.
pub const CHILL_DURATION: u32 = ticks(3, 2);
pub const CHILL_SPEED_PCT: i32 = 60;

/// Fissure Guardian (TAKOAI-118, design/BATTLE.md #7 and § Enemy roles,
/// decision B5): once the kill target is reached the fissures go quiet
/// and the Guardian comes out of the fissure nearest the ships; the
/// mission is won when it dies. Phase 1 fires rings of slow shots with two
/// gaps and summons Swarmer packs; below half HP (phase 2) the rings are
/// faster and denser, it charges like a Lancer and summons Bloaters. Every
/// attack has a wind-up the client draws. Engine defaults to tune.
///
/// HP for one ship; each extra ship adds `GUARDIAN_HP_PER_SHIP_PCT`%.
pub const GUARDIAN_HP: i32 = 400;
pub const GUARDIAN_HP_PER_SHIP_PCT: i32 = 60;
/// Art's suggested 140 px display size over `ROLE_SCALE` (1.6).
pub const GUARDIAN_RADIUS: i32 = 44;
/// It closes in at `GUARDIAN_SPEED`, faster (`GUARDIAN_RUSH_SPEED`) while
/// farther than `GUARDIAN_ATTACK_RANGE`, and stops at `GUARDIAN_HOLD`.
pub const GUARDIAN_SPEED: i32 = 50;
pub const GUARDIAN_RUSH_SPEED: i32 = 130;
pub const GUARDIAN_HOLD: i32 = 240;
/// It only starts an attack with a ship this close.
pub const GUARDIAN_ATTACK_RANGE: i32 = 600;
/// Delay after it comes out before its first attack.
pub const GUARDIAN_FIRST_ATTACK: u32 = ticks(2, 1);
/// Pause after each attack before the next wind-up (phase 1 / 2).
pub const GUARDIAN_REST_P1: u32 = ticks(8, 5);
pub const GUARDIAN_REST_P2: u32 = ticks(1, 1);
/// The phase change: it roars for this long (no attack, still hittable).
pub const GUARDIAN_ROAR: u32 = ticks(3, 2);
/// Shot rings: the ring flashes for the wind-up, then `GUARDIAN_RING_SHOTS`
/// shots leave evenly round it, minus two opposite gaps of
/// `GUARDIAN_RING_GAP` slots each (the gap angle is locked when the flash
/// starts, so the flash can show it).
pub const GUARDIAN_RING_WINDUP_P1: u32 = ticks(4, 5);
pub const GUARDIAN_RING_WINDUP_P2: u32 = ticks(3, 5);
pub const GUARDIAN_RING_SHOTS_P1: i32 = 24;
pub const GUARDIAN_RING_SHOTS_P2: i32 = 32;
pub const GUARDIAN_RING_GAP: i32 = 3;
pub const GUARDIAN_RING_SPEED_P1: i32 = 130;
pub const GUARDIAN_RING_SPEED_P2: i32 = 190;
pub const GUARDIAN_SHOT_DAMAGE: i32 = 8;
pub const GUARDIAN_SHOT_RADIUS: i32 = 8;
pub const GUARDIAN_SHOT_LIFETIME: u32 = ticks(4, 1);
/// Summons: the core glows for the wind-up, then `GUARDIAN_SWARMERS`
/// Swarmers (phase 1) or `GUARDIAN_BLOATERS` Bloaters (phase 2) appear
/// round it.
pub const GUARDIAN_SUMMON_WINDUP: u32 = ticks(4, 5);
pub const GUARDIAN_SWARMERS: i32 = 5;
pub const GUARDIAN_BLOATERS: i32 = 2;
pub const GUARDIAN_SUMMON_RING: i32 = 80;
/// Phase 2 charge: the charge line shows for the wind-up, then it dashes
/// `GUARDIAN_CHARGE_DISTANCE` px along it, hitting each ship once.
pub const GUARDIAN_CHARGE_WINDUP: u32 = ticks(4, 5);
pub const GUARDIAN_CHARGE_DISTANCE: i32 = 460;
pub const GUARDIAN_CHARGE_TICKS: u32 = ticks(1, 2);
pub const GUARDIAN_CHARGE_DAMAGE: i32 = 18;
/// Loot burst: `GUARDIAN_LOOT_PILES` credit piles of
/// `GUARDIAN_LOOT_CREDITS` and `GUARDIAN_LOOT_CRYSTALS` single Void
/// Crystals, flung round where it died (all under the elite light beam).
pub const GUARDIAN_LOOT_PILES: u32 = 10;
pub const GUARDIAN_LOOT_CREDITS: u32 = 10;
pub const GUARDIAN_LOOT_CRYSTALS: u32 = 5;
/// After the kill the battle runs this long before the mission succeeds,
/// pulling every pickup to the nearest ship (the big kill moment).
pub const GUARDIAN_OUTRO: u32 = ticks(5, 2);
/// Outro pickup pull speed.
pub const GUARDIAN_OUTRO_PULL: i32 = 520;

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
pub const LANCER_CREDITS: u32 = 8;
pub const LANCER_CRYSTAL_PCT: i32 = 30;
pub const BLOATER_CREDITS: u32 = 6;
pub const BLOATER_CRYSTAL_PCT: i32 = 20;
pub const BROOD_CREDITS: u32 = 20;
pub const BROOD_CRYSTAL_PCT: i32 = 70;
pub const WARDEN_CREDITS: u32 = 15;
pub const WARDEN_CRYSTAL_PCT: i32 = 60;
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
