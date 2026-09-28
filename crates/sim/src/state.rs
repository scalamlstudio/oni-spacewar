//! The whole rolled-back simulation state and its fixed-timestep step.
//!
//! Gameplay here is a placeholder that exercises the foundation: click-to-move
//! battleships, a Q shot, and waves of enemies that chase the nearest ship.
//! Real mission gameplay replaces it; the structure is what stays:
//!
//! - All state is plain data in [`SimState`] (entities in `Vec`s, see
//!   [`crate::entity`]).
//! - [`SimState::step`] runs a fixed sequence of phases, each a method over
//!   whole collections, in the same order on every peer.
//! - Randomness comes only from `self.rng`.

use core::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::collision::{overlaps, separation};
use crate::entity::{Enemy, Projectile, Ship};
use crate::fixed::{mul_q16, FxVec2, SUB};
use crate::input::{NetInput, INPUT_MOVE, INPUT_SKILL_Q};
use crate::rng::SimRng;
use crate::trig::{cos_q16, sin_q16, ANGLE_STEPS};
use crate::tuning::*;

/// Most players a mission can hold (design: up to 4 per mission).
pub const MAX_PLAYERS: usize = 4;

/// Match setup that must be identical on every peer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SimParams {
    pub num_players: usize,
    pub seed: u64,
}

/// Everything that affects gameplay. Saved/loaded wholesale on rollback and
/// hashed for desync detection, so any new gameplay field goes in here.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub params: SimParams,
    /// Number of ticks simulated so far.
    pub frame: u32,
    pub rng: SimRng,
    /// One ship per player, indexed by handle (stable order).
    pub ships: Vec<Ship>,
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    /// Number of enemy waves spawned so far.
    pub wave: u32,
}

const fn px(v: i32) -> i32 {
    v * SUB
}

fn ship_spawn(handle: usize, n: usize) -> FxVec2 {
    let x = (2 * handle as i32 - (n as i32 - 1)) * px(SHIP_SPAWN_GAP) / 2;
    FxVec2::new(x, 0)
}

impl SimState {
    pub fn new(params: SimParams) -> Self {
        assert!(
            (1..=MAX_PLAYERS).contains(&params.num_players),
            "num_players must be 1..={MAX_PLAYERS}"
        );
        let n = params.num_players;
        let ships = (0..n)
            .map(|handle| {
                let pos = ship_spawn(handle, n);
                Ship {
                    handle,
                    pos,
                    target: pos,
                    cooldown: 0,
                }
            })
            .collect();
        Self {
            params,
            frame: 0,
            rng: SimRng::new(params.seed),
            ships,
            enemies: Vec::new(),
            projectiles: Vec::new(),
            wave: 0,
        }
    }

    /// Advance one fixed tick. `inputs[h]` is player `h`'s input.
    pub fn step(&mut self, inputs: &[NetInput]) {
        assert_eq!(inputs.len(), self.ships.len(), "one input per player");
        self.spawn_wave_if_clear();
        self.apply_inputs(inputs);
        self.move_ships();
        self.move_enemies();
        self.move_projectiles();
        self.projectile_hits();
        self.separate_ships_and_enemies();
        self.remove_dead();
        self.frame += 1;
    }

    /// Spawn `WAVE_SIZE` enemies evenly on a ring around the ships' centroid,
    /// at a random rotation.
    fn spawn_wave_if_clear(&mut self) {
        if !self.enemies.is_empty() {
            return;
        }
        let n = self.ships.len() as i32;
        let sum = self.ships.iter().fold(FxVec2::ZERO, |a, s| a + s.pos);
        let centre = FxVec2::new(sum.x / n, sum.y / n);
        let offset = self.rng.range(0, ANGLE_STEPS - 1);
        for i in 0..WAVE_SIZE as i32 {
            let angle = offset + i * ANGLE_STEPS / WAVE_SIZE as i32;
            let r = px(WAVE_RING_RADIUS);
            self.enemies.push(Enemy {
                pos: centre + FxVec2::new(mul_q16(r, cos_q16(angle)), mul_q16(r, sin_q16(angle))),
                hp: ENEMY_HP,
            });
        }
        self.wave += 1;
    }

    /// Click-to-move sets the target; Q fires toward the cursor when off cooldown.
    fn apply_inputs(&mut self, inputs: &[NetInput]) {
        for ship in &mut self.ships {
            let input = inputs[ship.handle];
            let aim = FxVec2::from_px(input.target_x, input.target_y);
            if input.pressed(INPUT_MOVE) {
                ship.target = aim;
            }
            ship.cooldown = ship.cooldown.saturating_sub(1);
            if input.pressed(INPUT_SKILL_Q) && ship.cooldown == 0 {
                let vel = (aim - ship.pos).scale_to(px_per_tick(SHOT_SPEED));
                if vel != FxVec2::ZERO {
                    self.projectiles.push(Projectile {
                        owner: ship.handle,
                        pos: ship.pos,
                        vel,
                        ttl: SHOT_LIFETIME,
                    });
                    ship.cooldown = SHOT_COOLDOWN;
                }
            }
        }
    }

    fn move_ships(&mut self) {
        let speed = px_per_tick(SHIP_SPEED);
        for ship in &mut self.ships {
            let d = ship.target - ship.pos;
            ship.pos = if d.length() <= speed as i64 {
                ship.target
            } else {
                ship.pos + d.scale_to(speed)
            };
        }
    }

    /// Nearest ship to `pos` (lowest handle on ties).
    fn nearest_ship(&self, pos: FxVec2) -> Option<&Ship> {
        self.ships
            .iter()
            .min_by_key(|s| ((s.pos - pos).length_squared(), s.handle))
    }

    fn move_enemies(&mut self) {
        let speed = px_per_tick(ENEMY_SPEED);
        for i in 0..self.enemies.len() {
            let pos = self.enemies[i].pos;
            if let Some(target) = self.nearest_ship(pos).map(|s| s.pos) {
                self.enemies[i].pos = pos + (target - pos).scale_to(speed);
            }
        }
    }

    fn move_projectiles(&mut self) {
        for p in &mut self.projectiles {
            p.pos = p.pos + p.vel;
            p.ttl = p.ttl.saturating_sub(1);
        }
    }

    /// Each projectile damages the first living enemy it overlaps and is spent.
    fn projectile_hits(&mut self) {
        for p in &mut self.projectiles {
            let hit = self
                .enemies
                .iter_mut()
                .find(|e| e.hp > 0 && overlaps(p.pos, px(SHOT_RADIUS), e.pos, px(ENEMY_RADIUS)));
            if let Some(e) = hit {
                e.hp -= SHOT_DAMAGE;
                p.ttl = 0;
            }
        }
    }

    /// Ships and enemies push apart, split by mass (the lighter one moves more).
    /// Ships never collide with each other.
    fn separate_ships_and_enemies(&mut self) {
        let total = SHIP_MASS + ENEMY_MASS;
        for ship in &mut self.ships {
            for e in &mut self.enemies {
                let Some(push) = separation(ship.pos, px(SHIP_RADIUS), e.pos, px(ENEMY_RADIUS))
                else {
                    continue;
                };
                let ship_share =
                    FxVec2::new(push.x * ENEMY_MASS / total, push.y * ENEMY_MASS / total);
                ship.pos = ship.pos + ship_share;
                e.pos = e.pos - (push - ship_share);
            }
        }
    }

    fn remove_dead(&mut self) {
        self.enemies.retain(|e| e.hp > 0);
        self.projectiles.retain(|p| p.ttl > 0);
    }

    /// Platform- and toolchain-independent checksum (FNV-1a over `Hash`), for
    /// tests and logs. GGRS computes its own checksum via the client.
    pub fn checksum(&self) -> u64 {
        let mut h = Fnv1a::default();
        self.hash(&mut h);
        h.finish()
    }
}

struct Fnv1a(u64);

impl Default for Fnv1a {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv1a {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::*;

    /// Deterministic pseudo-players: re-target every 40 ticks, fire in bursts.
    fn scripted_inputs(frame: u32, players: usize) -> Vec<NetInput> {
        (0..players)
            .map(|p| {
                let k = (frame / 40) as i32 + p as i32 * 7;
                let mut buttons = 0;
                if frame.is_multiple_of(40) {
                    buttons |= INPUT_MOVE;
                }
                if !(frame / 25 + p as u32).is_multiple_of(3) {
                    buttons |= INPUT_SKILL_Q;
                }
                NetInput {
                    buttons,
                    target_x: (k * 173) % 900 - 450,
                    target_y: (k * 311) % 900 - 450,
                }
            })
            .collect()
    }

    fn run(params: SimParams, frames: u32) -> SimState {
        let mut s = SimState::new(params);
        for f in 0..frames {
            s.step(&scripted_inputs(f, params.num_players));
        }
        s
    }

    fn params(num_players: usize) -> SimParams {
        SimParams {
            num_players,
            seed: 7,
        }
    }

    fn idle(n: usize) -> Vec<NetInput> {
        vec![NetInput::default(); n]
    }

    fn input(buttons: u8, x: i32, y: i32) -> NetInput {
        NetInput {
            buttons,
            target_x: x,
            target_y: y,
        }
    }

    /// Ships only, no wave yet (the next step would spawn one).
    fn quiet(players: usize) -> SimState {
        let mut s = SimState::new(params(players));
        // A far-away enemy keeps waves from spawning without touching anything.
        s.enemies.push(Enemy {
            pos: FxVec2::from_px(100_000, 100_000),
            hp: ENEMY_HP,
        });
        s
    }

    #[test]
    fn same_inputs_same_state() {
        for players in 1..=MAX_PLAYERS {
            assert_eq!(
                run(params(players), 3600).checksum(),
                run(params(players), 3600).checksum()
            );
        }
    }

    #[test]
    fn restore_and_resimulate_matches() {
        // What rollback does: snapshot, run ahead, restore, re-run.
        let mut s = run(params(4), 500);
        let snapshot = s.clone();
        for f in 500..600 {
            s.step(&scripted_inputs(f, 4));
        }
        let ahead = s.checksum();
        let mut s = snapshot;
        for f in 500..600 {
            s.step(&scripted_inputs(f, 4));
        }
        assert_eq!(s.checksum(), ahead);
    }

    #[test]
    fn scripted_run_exercises_gameplay() {
        // Guard against the tests above passing on a sim that does nothing.
        let s = run(params(2), 3600);
        assert!(s.wave > 1, "waves cleared: {}", s.wave);
        assert_ne!(s.ships[0].pos, ship_spawn(0, 2));
    }

    #[test]
    fn one_ship_per_player_in_handle_order() {
        let s = SimState::new(params(3));
        let handles: Vec<_> = s.ships.iter().map(|s| s.handle).collect();
        assert_eq!(handles, vec![0, 1, 2]);
        assert_ne!(s.ships[0].pos, s.ships[1].pos);
        assert_eq!(SimState::new(params(1)).ships[0].pos, FxVec2::ZERO);
    }

    #[test]
    fn click_to_move_keeps_going_after_release() {
        let mut s = quiet(1);
        s.step(&[input(INPUT_MOVE, 100, 0)]);
        // 200 px/s = 853 sub-px/tick, so 100 px takes 31 ticks.
        for _ in 0..29 {
            s.step(&idle(1));
        }
        assert_ne!(s.ships[0].pos, FxVec2::from_px(100, 0));
        s.step(&idle(1));
        assert_eq!(s.ships[0].pos, FxVec2::from_px(100, 0));
        s.step(&idle(1));
        assert_eq!(s.ships[0].pos, FxVec2::from_px(100, 0));
    }

    #[test]
    fn shot_damages_enemy_and_is_spent() {
        let mut s = quiet(1);
        s.enemies.push(Enemy {
            pos: FxVec2::from_px(60, 0),
            hp: ENEMY_HP,
        });
        s.step(&[input(INPUT_SKILL_Q, 60, 0)]);
        assert_eq!(s.projectiles.len(), 1);
        for _ in 0..10 {
            s.step(&idle(1));
        }
        assert_eq!(s.enemies[1].hp, ENEMY_HP - SHOT_DAMAGE);
        assert!(s.projectiles.is_empty());
    }

    #[test]
    fn q_respects_cooldown() {
        let mut s = quiet(1);
        let q = [input(INPUT_SKILL_Q, 0, 500)];
        for _ in 0..SHOT_COOLDOWN {
            s.step(&q);
        }
        assert_eq!(s.projectiles.len(), 1);
        s.step(&q);
        assert_eq!(s.projectiles.len(), 2);
    }

    #[test]
    fn enemies_chase_the_nearest_ship() {
        let mut s = quiet(2);
        s.enemies.clear();
        // Nearer ship 1 (x = +30) than ship 0 (x = -30).
        s.enemies.push(Enemy {
            pos: FxVec2::from_px(30, -300),
            hp: ENEMY_HP,
        });
        s.step(&idle(2));
        assert_eq!(
            s.enemies[0].pos,
            FxVec2::new(px(30), px(-300) + px_per_tick(ENEMY_SPEED))
        );
    }

    #[test]
    fn ships_never_collide_with_each_other() {
        let mut s = quiet(2);
        s.ships[1].pos = s.ships[0].pos;
        s.ships[1].target = s.ships[0].pos;
        s.step(&idle(2));
        assert_eq!(s.ships[0].pos, s.ships[1].pos);
    }

    #[test]
    fn ships_and_enemies_push_apart_by_mass() {
        let mut s = quiet(1);
        s.enemies.push(Enemy {
            pos: FxVec2::from_px(0, 20),
            hp: ENEMY_HP,
        });
        s.step(&idle(1));
        let (ship, enemy) = (s.ships[0].pos, s.enemies[1].pos);
        assert!(ship.y < 0 && enemy.y > px(20) - px_per_tick(ENEMY_SPEED));
        // The heavier ship moves less.
        assert!(-ship.y < enemy.y - px(20));
        assert!((enemy - ship).length() >= px(SHIP_RADIUS + ENEMY_RADIUS) as i64 - 2);
    }

    #[test]
    fn new_wave_spawns_when_clear() {
        let mut s = SimState::new(params(1));
        s.step(&idle(1));
        assert_eq!((s.wave, s.enemies.len()), (1, WAVE_SIZE));
        for e in &s.enemies {
            let r = e.pos.length() - px(WAVE_RING_RADIUS) as i64;
            assert!(r.abs() < SUB as i64 * 8, "on the ring (moved one tick)");
        }
        s.enemies.clear();
        s.step(&idle(1));
        assert_eq!(s.wave, 2);
    }

    #[test]
    fn checksum_sees_state_changes() {
        let a = run(params(2), 10);
        let mut b = a.clone();
        b.ships[1].pos.x += 1;
        assert_ne!(a.checksum(), b.checksum());
        let mut c = a.clone();
        c.rng.next_u32();
        assert_ne!(a.checksum(), c.checksum());
        let mut d = a.clone();
        d.enemies[0].hp -= 1;
        assert_ne!(a.checksum(), d.checksum());
    }
}
