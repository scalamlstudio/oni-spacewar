//! The deterministic mission simulation (everything that runs in `GgrsSchedule`).
//!
//! Rules this module follows:
//! - Only integer math. Positions/velocities are fixed-point `i32` in `SUB`
//!   sub-pixels; directions come from `trig` (committed lookup table); vector
//!   lengths use `i64::isqrt`. No `f32` ever touches gameplay state.
//! - No wall clock, no frame delta, no unseeded randomness: the only inputs are
//!   `PlayerInputs` and the rolled-back `SimRng`.
//! - Stable iteration order: ships are sorted by player handle before use, and
//!   bullets live in one `Vec` resource whose order is part of the state.
//! - Every piece of gameplay state is registered for rollback and checksumming
//!   in `SimPlugin`.

use bevy::prelude::*;
use bevy_ggrs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::trig::{cos_q16, sin_q16, ANGLE_STEPS};

/// Sub-pixels per pixel (fixed-point scale for positions and velocities).
pub const SUB: i32 = 256;
/// Arena half extents in pixels (the arena is centred on the origin).
pub const ARENA_HALF_W: i32 = 480;
pub const ARENA_HALF_H: i32 = 270;
/// Ship hitbox radius in pixels (bullet-hell style: tiny).
pub const SHIP_HITBOX: i32 = 3;
pub const BULLET_RADIUS: i32 = 4;
const SHIP_SPEED: i32 = 4 * SUB;
const SHIP_FOCUS_SPEED: i32 = 2 * SUB;
const INVULN_FRAMES: u16 = 30;

pub const INPUT_UP: u8 = 1 << 0;
pub const INPUT_DOWN: u8 = 1 << 1;
pub const INPUT_LEFT: u8 = 1 << 2;
pub const INPUT_RIGHT: u8 = 1 << 3;
pub const INPUT_FOCUS: u8 = 1 << 4;

/// The only data exchanged between peers each frame.
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub struct NetInput {
    pub buttons: u8,
}

pub type SpikeConfig = GgrsConfig<NetInput, matchbox_socket::PeerId>;

/// Parameters that must be identical on every peer (part of the "match setup").
#[derive(Resource, Clone, Copy, Debug)]
pub struct SimParams {
    pub num_players: usize,
    pub seed: u64,
    /// Emitters stop spawning while this many bullets are alive.
    pub bullet_target: usize,
}

#[derive(Component, Clone, Copy, Hash, Debug)]
pub struct Ship {
    pub handle: usize,
}

/// Fixed-point position in sub-pixels.
#[derive(Component, Clone, Copy, Hash, Debug, Default)]
pub struct FxPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Clone, Copy, Hash, Debug, Default)]
pub struct ShipHealth {
    pub hits_taken: u32,
    pub invuln: u16,
}

#[derive(Clone, Copy, Hash, Debug)]
pub struct Bullet {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub kind: u8,
}

pub const BULLET_KIND_RING: u8 = 0;
pub const BULLET_KIND_AIMED: u8 = 1;

/// All bullets, in a single Vec so order is part of the (rolled-back) state.
#[derive(Resource, Clone, Hash, Default)]
pub struct Bullets(pub Vec<Bullet>);

/// Seeded xorshift64* RNG. Rolled back and checksummed like any other state.
#[derive(Resource, Clone, Copy, Hash)]
pub struct SimRng(pub u64);

impl SimRng {
    pub fn new(seed: u64) -> Self {
        // Avoid the all-zero state.
        Self(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
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

/// Simulation frame counter (independent of bevy_ggrs' own frame count so the
/// sim never depends on plugin internals).
#[derive(Resource, Clone, Copy, Hash, Default)]
pub struct SimFrame(pub u32);

/// Emitter state that evolves over time.
#[derive(Resource, Clone, Copy, Hash, Default)]
pub struct Emitters {
    /// Spiral angle of the "Maw" ring emitter, in `ANGLE_STEPS` units.
    pub maw_angle: i32,
    /// Direction the spiral is currently rotating (+1 / -1), flipped randomly.
    pub maw_spin: i32,
}

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.rollback_component_with_copy::<Ship>()
            .rollback_component_with_copy::<FxPos>()
            .rollback_component_with_copy::<ShipHealth>()
            .rollback_resource_with_clone::<Bullets>()
            .rollback_resource_with_copy::<SimRng>()
            .rollback_resource_with_copy::<SimFrame>()
            .rollback_resource_with_copy::<Emitters>()
            .checksum_component_with_hash::<Ship>()
            .checksum_component_with_hash::<FxPos>()
            .checksum_component_with_hash::<ShipHealth>()
            .checksum_resource_with_hash::<Bullets>()
            .checksum_resource_with_hash::<SimRng>()
            .checksum_resource_with_hash::<SimFrame>()
            .checksum_resource_with_hash::<Emitters>()
            .init_resource::<InjectDesync>()
            .add_systems(Startup, spawn_match)
            .add_systems(
                GgrsSchedule,
                (
                    crate::stats::track_sim_frame,
                    move_ships,
                    move_bullets,
                    emit_maw_ring,
                    emit_swarmling_streams,
                    collide_bullets_with_ships,
                    inject_desync,
                    advance_frame,
                )
                    .chain(),
            );
    }
}

/// Negative control for the SyncTest check (`--inject-desync`): deliberately
/// breaks determinism by mixing a counter that is NOT rolled back into the RNG
/// once every 997 sim steps (prime, so it is not aligned with the
/// synctest's 8-steps-per-update pattern). A correct checker must report mismatches.
#[derive(Resource, Clone, Copy, Default)]
pub struct InjectDesync(pub bool);

fn inject_desync(flag: Res<InjectDesync>, mut rng: ResMut<SimRng>, mut calls: Local<u64>) {
    if !flag.0 {
        return;
    }
    *calls += 1;
    if *calls % 997 == 0 {
        rng.0 ^= *calls;
    }
}

fn spawn_match(mut commands: Commands, params: Res<SimParams>) {
    commands.insert_resource(SimRng::new(params.seed));
    commands.insert_resource(SimFrame(0));
    commands.insert_resource(Bullets(Vec::with_capacity(params.bullet_target + 256)));
    commands.insert_resource(Emitters {
        maw_angle: 0,
        maw_spin: 1,
    });
    // Nothing here assumes a single battleship: one ship per player handle.
    let n = params.num_players as i32;
    for handle in 0..params.num_players {
        let x = (-(n - 1) * 120 + handle as i32 * 240) * SUB;
        commands.spawn((
            Ship { handle },
            FxPos { x, y: -180 * SUB },
            ShipHealth::default(),
            Rollback,
        ));
    }
}

/// Ships sorted by handle: the stable iteration order the sim relies on.
fn sorted_ships<'a>(
    iter: impl Iterator<Item = (&'a Ship, &'a FxPos)>,
) -> Vec<(usize, FxPos)> {
    let mut v: Vec<_> = iter.map(|(s, p)| (s.handle, *p)).collect();
    v.sort_unstable_by_key(|(h, _)| *h);
    v
}

fn move_ships(inputs: Res<PlayerInputs<SpikeConfig>>, mut ships: Query<(&Ship, &mut FxPos)>) {
    for (ship, mut pos) in &mut ships {
        let b = inputs[ship.handle].0.buttons;
        let dx = (b & INPUT_RIGHT != 0) as i32 - (b & INPUT_LEFT != 0) as i32;
        let dy = (b & INPUT_UP != 0) as i32 - (b & INPUT_DOWN != 0) as i32;
        let speed = if b & INPUT_FOCUS != 0 {
            SHIP_FOCUS_SPEED
        } else {
            SHIP_SPEED
        };
        // 8-direction movement; diagonals scaled by ~1/sqrt(2) (46341/65536).
        let (vx, vy) = if dx != 0 && dy != 0 {
            let d = ((speed as i64 * 46341) >> 16) as i32;
            (dx * d, dy * d)
        } else {
            (dx * speed, dy * speed)
        };
        pos.x = (pos.x + vx).clamp(-ARENA_HALF_W * SUB, ARENA_HALF_W * SUB);
        pos.y = (pos.y + vy).clamp(-ARENA_HALF_H * SUB, ARENA_HALF_H * SUB);
    }
}

fn move_bullets(mut bullets: ResMut<Bullets>) {
    let margin = 16 * SUB;
    let (w, h) = (ARENA_HALF_W * SUB + margin, ARENA_HALF_H * SUB + margin);
    bullets.0.retain_mut(|b| {
        b.x += b.vx;
        b.y += b.vy;
        b.x.abs() <= w && b.y.abs() <= h
    });
}

fn emit_maw_ring(
    frame: Res<SimFrame>,
    params: Res<SimParams>,
    mut rng: ResMut<SimRng>,
    mut emitters: ResMut<Emitters>,
    mut bullets: ResMut<Bullets>,
) {
    // The Maw: a rotating 20-way ring every 5 frames from the arena centre-top.
    const ARMS: i32 = 20;
    const SPEED: i32 = 2 * SUB + SUB / 2;
    emitters.maw_angle = (emitters.maw_angle + 7 * emitters.maw_spin).rem_euclid(ANGLE_STEPS);
    if frame.0 % 240 == 239 && rng.next_u32() % 2 == 0 {
        emitters.maw_spin = -emitters.maw_spin;
    }
    if frame.0 % 5 != 0 || bullets.0.len() >= params.bullet_target {
        return;
    }
    let (ox, oy) = (0, 120 * SUB);
    for arm in 0..ARMS {
        let a = emitters.maw_angle + arm * (ANGLE_STEPS / ARMS);
        bullets.0.push(Bullet {
            x: ox,
            y: oy,
            vx: ((SPEED as i64 * cos_q16(a) as i64) >> 16) as i32,
            vy: ((SPEED as i64 * sin_q16(a) as i64) >> 16) as i32,
            kind: BULLET_KIND_RING,
        });
    }
}

/// Position of swarmling `i` at `frame`: they drift along the top edge.
pub fn swarmling_pos(i: u32, frame: u32) -> (i32, i32) {
    let a = (frame as i32 * 3 + i as i32 * (ANGLE_STEPS / 3)).rem_euclid(ANGLE_STEPS);
    let x = ((ARENA_HALF_W - 60) as i64 * SUB as i64 * cos_q16(a) as i64 >> 16) as i32;
    let y = (ARENA_HALF_H - 40) * SUB + ((30 * SUB) as i64 * sin_q16(a * 2) as i64 >> 16) as i32;
    (x, y)
}

pub const SWARMLINGS: u32 = 3;

fn emit_swarmling_streams(
    frame: Res<SimFrame>,
    params: Res<SimParams>,
    mut rng: ResMut<SimRng>,
    ships: Query<(&Ship, &FxPos)>,
    mut bullets: ResMut<Bullets>,
) {
    // Swarmlings: 3 emitters firing aimed 3-shot fans at the nearest ship every 4 frames.
    const SPEED: i64 = 4 * SUB as i64;
    if frame.0 % 4 != 0 || bullets.0.len() >= params.bullet_target {
        return;
    }
    let ships = sorted_ships(ships.iter());
    if ships.is_empty() {
        return;
    }
    for i in 0..SWARMLINGS {
        let (sx, sy) = swarmling_pos(i, frame.0);
        // Nearest ship; ties resolved by lowest handle (ships are sorted).
        let target = ships
            .iter()
            .min_by_key(|(_, p)| {
                let dx = (p.x - sx) as i64;
                let dy = (p.y - sy) as i64;
                dx * dx + dy * dy
            })
            .map(|(_, p)| *p)
            .unwrap();
        let dx = (target.x - sx) as i64;
        let dy = (target.y - sy) as i64;
        let len = (dx * dx + dy * dy).isqrt().max(1);
        let (vx, vy) = ((dx * SPEED / len) as i32, (dy * SPEED / len) as i32);
        for spread in -1..=1 {
            // Small rotation per fan arm plus seeded jitter (angle units -> Q16 rotation).
            let rot = spread * 24 + rng.range(-4, 4);
            let (c, s) = (cos_q16(rot) as i64, sin_q16(rot) as i64);
            bullets.0.push(Bullet {
                x: sx,
                y: sy,
                vx: ((vx as i64 * c - vy as i64 * s) >> 16) as i32,
                vy: ((vx as i64 * s + vy as i64 * c) >> 16) as i32,
                kind: BULLET_KIND_AIMED,
            });
        }
    }
}

fn collide_bullets_with_ships(
    mut bullets: ResMut<Bullets>,
    mut ships: Query<(&Ship, &FxPos, &mut ShipHealth)>,
) {
    let r = ((SHIP_HITBOX + BULLET_RADIUS) * SUB) as i64;
    let r2 = r * r;
    let mut ships: Vec<_> = ships.iter_mut().collect();
    ships.sort_unstable_by_key(|(s, _, _)| s.handle);
    for (_, _, health) in ships.iter_mut() {
        health.invuln = health.invuln.saturating_sub(1);
    }
    bullets.0.retain(|b| {
        for (_, pos, health) in ships.iter_mut() {
            let dx = (b.x - pos.x) as i64;
            let dy = (b.y - pos.y) as i64;
            if dx * dx + dy * dy <= r2 {
                if health.invuln == 0 {
                    health.hits_taken += 1;
                    health.invuln = INVULN_FRAMES;
                }
                return false;
            }
        }
        true
    });
}

fn advance_frame(mut frame: ResMut<SimFrame>) {
    frame.0 += 1;
}
