//! The whole rolled-back simulation state and its fixed-timestep step.
//!
//! One Elimination mission (design/READINESS.md § Demo Spec): click-to-move
//! battleships with an auto-firing basic attack and two skills each, two void
//! monster types arriving in three waves, loot drops, and a win/lose result.
//!
//! - All state is plain data in [`SimState`] (entities in `Vec`s, see
//!   [`crate::entity`]).
//! - [`SimState::step`] runs a fixed sequence of phases, each a method over
//!   whole collections, in the same order on every peer.
//! - Randomness comes only from `self.rng`.

use core::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::collision::{overlaps, separation};
use crate::entity::*;
use crate::fixed::{mul_q16, FxVec2, SUB};
use crate::input::{NetInput, INPUT_MOVE, INPUT_SKILL_Q, INPUT_SKILL_W};
use crate::mission::{Mission, MissionOutcome, MissionStatus};
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
    /// Shots fired by ships.
    pub projectiles: Vec<Projectile>,
    /// Shots fired by enemies.
    pub enemy_projectiles: Vec<Projectile>,
    pub pickups: Vec<Pickup>,
    pub mission: Mission,
}

const fn px(v: i32) -> i32 {
    v * SUB
}

fn ship_spawn(handle: usize, n: usize) -> FxVec2 {
    let x = (2 * handle as i32 - (n as i32 - 1)) * px(SHIP_SPAWN_GAP) / 2;
    FxVec2::new(x, 0)
}

/// Keep a circle of `radius` px inside the arena.
fn clamp_to_arena(p: FxVec2, radius: i32) -> FxVec2 {
    let (hw, hh) = (px(ARENA_HALF_W - radius), px(ARENA_HALF_H - radius));
    FxVec2::new(p.x.clamp(-hw, hw), p.y.clamp(-hh, hh))
}

/// `v` rotated by `angle` (`trig::ANGLE_STEPS` units, counter-clockwise).
fn rotate(v: FxVec2, angle: i32) -> FxVec2 {
    let (c, s) = (cos_q16(angle), sin_q16(angle));
    FxVec2::new(
        mul_q16(v.x, c) - mul_q16(v.y, s),
        mul_q16(v.x, s) + mul_q16(v.y, c),
    )
}

/// Index of the nearest living ship to `pos` (lowest handle on ties).
fn nearest_ship(ships: &[Ship], pos: FxVec2) -> Option<usize> {
    ships
        .iter()
        .filter(|s| s.alive())
        .min_by_key(|s| ((s.pos - pos).length_squared(), s.handle))
        .map(|s| s.handle)
}

impl SimState {
    /// A mission where every player flies the default loadout.
    pub fn new(params: SimParams) -> Self {
        assert!(
            (1..=MAX_PLAYERS).contains(&params.num_players),
            "num_players must be 1..={MAX_PLAYERS}"
        );
        Self::with_loadouts(params.seed, &vec![Loadout::default(); params.num_players])
    }

    /// A mission with one ship per loadout; `loadouts[h]` is player `h`'s.
    pub fn with_loadouts(seed: u64, loadouts: &[Loadout]) -> Self {
        let n = loadouts.len();
        assert!(
            (1..=MAX_PLAYERS).contains(&n),
            "num_players must be 1..={MAX_PLAYERS}"
        );
        let ships = loadouts
            .iter()
            .enumerate()
            .map(|(handle, l)| Ship::new(handle, *l, ship_spawn(handle, n)))
            .collect();
        Self {
            params: SimParams {
                num_players: n,
                seed,
            },
            frame: 0,
            rng: SimRng::new(seed),
            ships,
            enemies: Vec::new(),
            projectiles: Vec::new(),
            enemy_projectiles: Vec::new(),
            pickups: Vec::new(),
            mission: Mission::default(),
        }
    }

    /// The finished mission's result, or `None` while it is still running.
    pub fn outcome(&self) -> Option<MissionOutcome> {
        self.mission.outcome()
    }

    /// Advance one fixed tick. `inputs[h]` is player `h`'s input. Once the
    /// mission has ended the world is frozen and only `frame` advances.
    pub fn step(&mut self, inputs: &[NetInput]) {
        assert_eq!(inputs.len(), self.ships.len(), "one input per player");
        if self.mission.status == MissionStatus::InProgress {
            self.advance_waves();
            self.apply_inputs(inputs);
            self.move_ships();
            self.basic_attacks();
            self.enemy_behaviour();
            self.move_projectiles();
            self.projectile_hits();
            self.separate();
            self.kill_and_drop();
            self.update_pickups();
            self.check_end();
        }
        self.frame += 1;
    }

    /// Banner countdown, then the current wave's spawn groups; the next wave
    /// starts when this one is cleared or timed out.
    fn advance_waves(&mut self) {
        let m = &mut self.mission;
        if m.banner_ticks > 0 {
            m.banner_ticks -= 1;
            if m.banner_ticks > 0 {
                return;
            }
            m.wave_ticks = 0;
        }
        let groups = wave_groups(m.wave);
        let t = m.wave_ticks;
        for &(delay, swarmers, spitters) in groups {
            if delay == t {
                self.spawn_group(swarmers, spitters);
            }
        }
        let m = &mut self.mission;
        m.wave_ticks += 1;
        let all_spawned = groups.iter().all(|g| g.0 < m.wave_ticks);
        let cleared = self.enemies.is_empty();
        if m.wave < WAVE_COUNT && all_spawned && (cleared || m.wave_ticks >= WAVE_TIMEOUT) {
            m.wave += 1;
            m.banner_ticks = WAVE_BANNER;
        }
    }

    /// Spawn evenly on a ring around the living ships' centroid, at a random
    /// rotation, clamped into the arena.
    fn spawn_group(&mut self, swarmers: u32, spitters: u32) {
        let living: Vec<FxVec2> = self
            .ships
            .iter()
            .filter(|s| s.alive())
            .map(|s| s.pos)
            .collect();
        if living.is_empty() {
            return;
        }
        let n = living.len() as i32;
        let sum = living.iter().fold(FxVec2::ZERO, |a, p| a + *p);
        let centre = FxVec2::new(sum.x / n, sum.y / n);
        let count = (swarmers + spitters) as i32;
        let offset = self.rng.range(0, ANGLE_STEPS - 1);
        let r = px(WAVE_RING_RADIUS);
        for i in 0..count {
            // Spitters are spread between the swarmers, not bunched together.
            let kind = if spitters > 0 && i % (count / spitters as i32) == 0 {
                EnemyKind::Spitter
            } else {
                EnemyKind::Swarmer
            };
            let angle = offset + i * ANGLE_STEPS / count;
            let pos = centre + FxVec2::new(mul_q16(r, cos_q16(angle)), mul_q16(r, sin_q16(angle)));
            self.enemies
                .push(Enemy::new(kind, clamp_to_arena(pos, kind.radius())));
            self.mission.spawned += 1;
        }
    }

    /// Click-to-move sets the target while held; Q / W use the ship's skills
    /// toward the cursor once per key press (the tick the button goes down)
    /// when off cooldown. A press during the cooldown is spent, so holding a
    /// key never re-fires a skill by itself.
    fn apply_inputs(&mut self, inputs: &[NetInput]) {
        for i in 0..self.ships.len() {
            let ship = &mut self.ships[i];
            ship.basic_cooldown = ship.basic_cooldown.saturating_sub(1);
            ship.q_cooldown = ship.q_cooldown.saturating_sub(1);
            ship.w_cooldown = ship.w_cooldown.saturating_sub(1);
            ship.shield_ticks = ship.shield_ticks.saturating_sub(1);
            if ship.shield_ticks == 0 {
                ship.shield = 0;
            }
            if !ship.alive() {
                continue;
            }
            let input = inputs[ship.handle];
            let down = input.buttons & !ship.prev_buttons;
            ship.prev_buttons = input.buttons;
            let aim = FxVec2::from_px(input.target_x, input.target_y);
            if input.pressed(INPUT_MOVE) {
                ship.target = clamp_to_arena(aim, ship.stats.radius);
            }
            if down & INPUT_SKILL_Q != 0 && ship.q_cooldown == 0 {
                self.skill_q(i, aim);
            }
            if down & INPUT_SKILL_W != 0 && self.ships[i].w_cooldown == 0 {
                self.skill_w(i, aim);
            }
        }
    }

    fn skill_q(&mut self, i: usize, aim: FxVec2) {
        let ship = &mut self.ships[i];
        match ship.kind {
            ShipKind::Kite => {
                // Afterburn: dash toward the cursor.
                let vel =
                    (aim - ship.pos).scale_to(px(AFTERBURN_DISTANCE) / AFTERBURN_TICKS as i32);
                if vel == FxVec2::ZERO {
                    return;
                }
                ship.dash_vel = vel;
                ship.dash_ticks = AFTERBURN_TICKS;
                ship.q_cooldown = AFTERBURN_COOLDOWN;
            }
            ShipKind::Bulwark => {
                // Bastion: absorb the next N damage for a while.
                ship.shield = BASTION_SHIELD;
                ship.shield_ticks = BASTION_DURATION;
                ship.q_cooldown = BASTION_COOLDOWN;
            }
        }
    }

    fn skill_w(&mut self, i: usize, aim: FxVec2) {
        let ship = &self.ships[i];
        match ship.kind {
            ShipKind::Kite => {
                // Scatter: a cone of bolts toward the cursor.
                let dir = (aim - ship.pos).scale_to(px_per_tick(SCATTER_SPEED));
                if dir == FxVec2::ZERO {
                    return;
                }
                let damage = ship.stats.skill_damage(SCATTER_DAMAGE);
                let (owner, pos) = (ship.handle, ship.pos);
                for k in 0..SCATTER_BOLTS {
                    let angle = (k - SCATTER_BOLTS / 2) * SCATTER_STEP;
                    self.projectiles.push(Projectile {
                        owner,
                        pos,
                        vel: rotate(dir, angle),
                        ttl: SCATTER_LIFETIME,
                        damage,
                        radius: SCATTER_RADIUS,
                    });
                }
                self.ships[i].w_cooldown = SCATTER_COOLDOWN;
            }
            ShipKind::Bulwark => {
                // Shockwave: damage and push back every enemy in range.
                let (centre, damage) = (ship.pos, ship.stats.skill_damage(SHOCKWAVE_DAMAGE));
                for e in &mut self.enemies {
                    if !overlaps(centre, px(SHOCKWAVE_RADIUS), e.pos, px(e.kind.radius())) {
                        continue;
                    }
                    e.hp -= damage;
                    let d = e.pos - centre;
                    let push = if d == FxVec2::ZERO {
                        FxVec2::new(px(SHOCKWAVE_PUSH), 0)
                    } else {
                        d.scale_to(px(SHOCKWAVE_PUSH))
                    };
                    e.pos = clamp_to_arena(e.pos + push, e.kind.radius());
                }
                self.ships[i].w_cooldown = SHOCKWAVE_COOLDOWN;
            }
        }
    }

    fn move_ships(&mut self) {
        for ship in self.ships.iter_mut().filter(|s| s.alive()) {
            let r = ship.stats.radius;
            if ship.dash_ticks > 0 {
                ship.pos = clamp_to_arena(ship.pos + ship.dash_vel, r);
                ship.dash_ticks -= 1;
                if ship.dash_ticks == 0 {
                    // Stop where the dash ends instead of walking back.
                    ship.target = ship.pos;
                }
                continue;
            }
            let speed = ship.stats.speed;
            let d = ship.target - ship.pos;
            ship.pos = if d.length() <= speed as i64 {
                ship.target
            } else {
                clamp_to_arena(ship.pos + d.scale_to(speed), r)
            };
        }
    }

    /// Each living ship fires a bolt at the nearest living enemy in range.
    fn basic_attacks(&mut self) {
        for ship in self.ships.iter_mut().filter(|s| s.alive()) {
            if ship.basic_cooldown > 0 {
                continue;
            }
            let range = px(ship.stats.basic_range) as i64;
            let Some(e) = self
                .enemies
                .iter()
                .filter(|e| e.hp > 0 && (e.pos - ship.pos).length_squared() <= range * range)
                .min_by_key(|e| (e.pos - ship.pos).length_squared())
            else {
                continue;
            };
            let vel = (e.pos - ship.pos).scale_to(px_per_tick(BOLT_SPEED));
            if vel == FxVec2::ZERO {
                continue;
            }
            self.projectiles.push(Projectile {
                owner: ship.handle,
                pos: ship.pos,
                vel,
                ttl: (ship.stats.basic_range * TICKS_PER_SEC / BOLT_SPEED) as u32 + 2,
                damage: ship.stats.basic_damage,
                radius: BOLT_RADIUS,
            });
            ship.basic_cooldown = ship.stats.basic_interval;
        }
    }

    /// Swarmers rush the nearest ship and hit on contact; Spitters hold a
    /// distance band and shoot.
    fn enemy_behaviour(&mut self) {
        for i in 0..self.enemies.len() {
            let e = &mut self.enemies[i];
            e.cooldown = e.cooldown.saturating_sub(1);
            let Some(h) = nearest_ship(&self.ships, e.pos) else {
                continue;
            };
            let ship = &mut self.ships[h];
            let to_ship = ship.pos - e.pos;
            let speed = e.kind.speed();
            match e.kind {
                EnemyKind::Swarmer => {
                    if to_ship.length() > speed as i64 {
                        e.pos = e.pos + to_ship.scale_to(speed);
                    }
                    let reach = ship.stats.radius + CONTACT_REACH;
                    if e.cooldown == 0 && overlaps(e.pos, px(e.kind.radius()), ship.pos, px(reach))
                    {
                        ship.take_damage(SWARMER_CONTACT_DAMAGE);
                        e.cooldown = SWARMER_CONTACT_COOLDOWN;
                    }
                }
                EnemyKind::Spitter => {
                    let d = to_ship.length();
                    if d < px(SPITTER_MIN_RANGE) as i64 {
                        e.pos = e.pos - to_ship.scale_to(speed);
                    } else if d > px(SPITTER_MAX_RANGE) as i64 {
                        e.pos = e.pos + to_ship.scale_to(speed);
                    }
                    e.pos = clamp_to_arena(e.pos, e.kind.radius());
                    let vel = to_ship.scale_to(px_per_tick(SPIT_SPEED));
                    if e.cooldown == 0 && d <= px(SPITTER_FIRE_RANGE) as i64 && vel != FxVec2::ZERO
                    {
                        self.enemy_projectiles.push(Projectile {
                            owner: 0,
                            pos: e.pos,
                            vel,
                            ttl: SPIT_LIFETIME,
                            damage: SPIT_DAMAGE,
                            radius: SPIT_RADIUS,
                        });
                        e.cooldown = SPITTER_FIRE_INTERVAL;
                    }
                }
            }
        }
    }

    fn move_projectiles(&mut self) {
        for p in self
            .projectiles
            .iter_mut()
            .chain(&mut self.enemy_projectiles)
        {
            p.pos = p.pos + p.vel;
            p.ttl = p.ttl.saturating_sub(1);
        }
    }

    /// Each shot damages the first living target it overlaps and is spent.
    fn projectile_hits(&mut self) {
        for p in &mut self.projectiles {
            let hit = self
                .enemies
                .iter_mut()
                .find(|e| e.hp > 0 && overlaps(p.pos, px(p.radius), e.pos, px(e.kind.radius())));
            if let Some(e) = hit {
                e.hp -= p.damage;
                p.ttl = 0;
            }
        }
        for p in &mut self.enemy_projectiles {
            let hit = self
                .ships
                .iter_mut()
                .find(|s| s.alive() && overlaps(p.pos, px(p.radius), s.pos, px(s.stats.radius)));
            if let Some(s) = hit {
                s.take_damage(p.damage);
                p.ttl = 0;
            }
        }
        self.projectiles.retain(|p| p.ttl > 0);
        self.enemy_projectiles.retain(|p| p.ttl > 0);
    }

    /// Ships and enemies push apart, split by mass (the lighter one moves
    /// more); enemies push each other apart evenly. Ships never collide with
    /// each other.
    fn separate(&mut self) {
        let total = SHIP_MASS + ENEMY_MASS;
        for ship in self.ships.iter_mut().filter(|s| s.alive()) {
            for e in &mut self.enemies {
                let Some(push) =
                    separation(ship.pos, px(ship.stats.radius), e.pos, px(e.kind.radius()))
                else {
                    continue;
                };
                let ship_share =
                    FxVec2::new(push.x * ENEMY_MASS / total, push.y * ENEMY_MASS / total);
                ship.pos = clamp_to_arena(ship.pos + ship_share, ship.stats.radius);
                e.pos = e.pos - (push - ship_share);
            }
        }
        for i in 0..self.enemies.len() {
            for j in i + 1..self.enemies.len() {
                let (a, b) = (&self.enemies[i], &self.enemies[j]);
                let Some(push) = separation(a.pos, px(a.kind.radius()), b.pos, px(b.kind.radius()))
                else {
                    continue;
                };
                let half = FxVec2::new(push.x / 2, push.y / 2);
                self.enemies[i].pos = self.enemies[i].pos + half;
                self.enemies[j].pos = self.enemies[j].pos - (push - half);
            }
        }
        for e in &mut self.enemies {
            e.pos = clamp_to_arena(e.pos, e.kind.radius());
        }
    }

    /// Dead enemies count as kills and drop loot rolled with the sim RNG.
    fn kill_and_drop(&mut self) {
        for e in self.enemies.iter().filter(|e| e.hp <= 0) {
            self.mission.kills += 1;
            let (credits, crystal_pct) = match e.kind {
                EnemyKind::Swarmer => (SWARMER_CREDITS, SWARMER_CRYSTAL_PCT),
                EnemyKind::Spitter => (SPITTER_CREDITS, SPITTER_CRYSTAL_PCT),
            };
            self.pickups.push(Pickup {
                kind: LootKind::Credits,
                amount: credits,
                pos: e.pos,
                ttl: PICKUP_LIFETIME,
            });
            if self.rng.range(0, 99) < crystal_pct {
                self.pickups.push(Pickup {
                    kind: LootKind::VoidCrystal,
                    amount: 1,
                    pos: e.pos + FxVec2::from_px(CRYSTAL_DROP_OFFSET, 0),
                    ttl: PICKUP_LIFETIME,
                });
            }
        }
        self.enemies.retain(|e| e.hp > 0);
    }

    /// Pickups drift toward a nearby ship, are collected on contact, and
    /// expire.
    fn update_pickups(&mut self) {
        let drift = px_per_tick(PICKUP_DRIFT_SPEED);
        for p in &mut self.pickups {
            p.ttl = p.ttl.saturating_sub(1);
            let Some(h) = nearest_ship(&self.ships, p.pos) else {
                continue;
            };
            let to_ship = self.ships[h].pos - p.pos;
            let d = to_ship.length();
            if d <= px(PICKUP_DRIFT_RANGE) as i64 {
                p.pos = p.pos
                    + if d <= drift as i64 {
                        to_ship
                    } else {
                        to_ship.scale_to(drift)
                    };
            }
            if (self.ships[h].pos - p.pos).length() <= px(PICKUP_COLLECT_RANGE) as i64 {
                self.mission.collected.add(p.kind, p.amount);
                p.ttl = 0;
            }
        }
        self.pickups.retain(|p| p.ttl > 0);
    }

    /// Success at the kill target (the field is cleared and every pickup
    /// left is collected); failure when every ship is destroyed.
    fn check_end(&mut self) {
        let m = &mut self.mission;
        if m.kills >= KILL_TARGET {
            m.status = MissionStatus::Success;
            for p in self.pickups.drain(..) {
                m.collected.add(p.kind, p.amount);
            }
            self.enemies.clear();
            self.projectiles.clear();
            self.enemy_projectiles.clear();
        } else if self.ships.iter().all(|s| !s.alive()) {
            m.status = MissionStatus::Failed;
        } else {
            return;
        }
        m.end_frame = self.frame;
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

    const KITE: Loadout = Loadout {
        ship: ShipKind::Kite,
        upgrades: Upgrades {
            hull: 0,
            weapon: 0,
            thruster: 0,
        },
    };
    const BULWARK: Loadout = Loadout {
        ship: ShipKind::Bulwark,
        ..KITE
    };

    /// Deterministic pseudo-players: re-target every 40 ticks, use skills in bursts.
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
                if (frame / 30 + p as u32).is_multiple_of(2) {
                    buttons |= INPUT_SKILL_W;
                }
                NetInput {
                    buttons,
                    target_x: (k * 173) % 900 - 450,
                    target_y: (k * 311) % 900 - 450,
                }
            })
            .collect()
    }

    /// Mixed fleet: even handles Kite, odd Bulwark.
    fn mixed(players: usize) -> Vec<Loadout> {
        (0..players)
            .map(|h| if h % 2 == 0 { KITE } else { BULWARK })
            .collect()
    }

    fn run(players: usize, frames: u32) -> SimState {
        let mut s = SimState::with_loadouts(7, &mixed(players));
        for f in 0..frames {
            s.step(&scripted_inputs(f, players));
        }
        s
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

    /// Ships only: the wave banner is held off so nothing spawns.
    fn quiet(loadouts: &[Loadout]) -> SimState {
        let mut s = SimState::with_loadouts(7, loadouts);
        s.mission.banner_ticks = u32::MAX;
        s
    }

    fn enemy(kind: EnemyKind, x: i32, y: i32) -> Enemy {
        Enemy::new(kind, FxVec2::from_px(x, y))
    }

    #[test]
    fn same_inputs_same_state() {
        for players in 1..=MAX_PLAYERS {
            assert_eq!(run(players, 3600).checksum(), run(players, 3600).checksum());
        }
    }

    #[test]
    fn restore_and_resimulate_matches() {
        // What rollback does: snapshot, run ahead, restore, re-run.
        let mut s = run(4, 500);
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
        let s = run(2, 3600);
        assert!(s.mission.wave > 1, "wave {}", s.mission.wave);
        assert!(s.mission.kills > 0);
        assert_ne!(s.ships[0].pos, ship_spawn(0, 2));
    }

    #[test]
    fn one_ship_per_player_in_handle_order() {
        let s = SimState::with_loadouts(1, &mixed(3));
        let handles: Vec<_> = s.ships.iter().map(|s| s.handle).collect();
        assert_eq!(handles, vec![0, 1, 2]);
        assert_eq!(s.ships[1].kind, ShipKind::Bulwark);
        assert_ne!(s.ships[0].pos, s.ships[1].pos);
        let solo = SimState::new(SimParams {
            num_players: 1,
            seed: 1,
        });
        assert_eq!(solo.ships[0].pos, FxVec2::ZERO);
    }

    #[test]
    fn ship_stats_follow_spec_and_upgrades() {
        let kite = ShipStats::new(KITE);
        assert_eq!((kite.max_hull, kite.basic_damage), (60, 4));
        assert_eq!(kite.speed, px_per_tick(220));
        let up = |l: u8| Upgrades {
            hull: l,
            weapon: l,
            thruster: l,
        };
        let k2 = ShipStats::new(Loadout {
            upgrades: up(2),
            ..KITE
        });
        assert_eq!((k2.max_hull, k2.basic_damage), (90, 6));
        assert_eq!(k2.speed, px_per_tick(286));
        let b1 = ShipStats::new(Loadout {
            upgrades: up(1),
            ..BULWARK
        });
        assert_eq!((b1.max_hull, b1.basic_damage), (175, 10));
        assert_eq!(b1.speed, px_per_tick(161));
        assert_eq!(b1.skill_damage(SHOCKWAVE_DAMAGE), 18);
    }

    #[test]
    fn click_to_move_keeps_going_after_release() {
        let mut s = quiet(&[BULWARK]);
        s.step(&[input(INPUT_MOVE, 100, 0)]);
        // 140 px/s = 597 sub-px/tick, so 100 px takes 43 ticks.
        for _ in 0..41 {
            s.step(&idle(1));
        }
        assert_ne!(s.ships[0].pos, FxVec2::from_px(100, 0));
        s.step(&idle(1));
        assert_eq!(s.ships[0].pos, FxVec2::from_px(100, 0));
    }

    #[test]
    fn arena_edges_block_movement() {
        let mut s = quiet(&[KITE]);
        s.step(&[input(INPUT_MOVE, 5000, -5000)]);
        for _ in 0..600 {
            s.step(&idle(1));
        }
        assert_eq!(
            s.ships[0].pos,
            FxVec2::from_px(ARENA_HALF_W - KITE_RADIUS, -(ARENA_HALF_H - KITE_RADIUS))
        );
    }

    #[test]
    fn basic_attack_auto_fires_at_nearest_enemy_in_range() {
        let mut s = quiet(&[BULWARK]);
        s.enemies.push(enemy(EnemyKind::Spitter, 230, 0));
        s.enemies.push(enemy(EnemyKind::Spitter, 0, 500)); // out of range
        s.step(&idle(1));
        assert_eq!(s.projectiles.len(), 1);
        assert!(s.projectiles[0].vel.x > 0 && s.projectiles[0].vel.y == 0);
        // Next shot only after the interval.
        for _ in 0..BULWARK_BASIC_INTERVAL - 1 {
            s.step(&idle(1));
        }
        assert!(s.enemies[0].hp < SPITTER_HP);
        let fired_before = s.enemies[0].hp;
        s.step(&idle(1));
        assert!(s.projectiles.len() == 1 || s.enemies[0].hp < fired_before);
    }

    #[test]
    fn kite_afterburn_dashes_toward_cursor() {
        let mut s = quiet(&[KITE]);
        s.step(&[input(INPUT_SKILL_Q, 1000, 0)]);
        for _ in 0..AFTERBURN_TICKS {
            s.step(&idle(1));
        }
        assert_eq!(s.ships[0].pos, FxVec2::from_px(AFTERBURN_DISTANCE, 0));
        assert_eq!(s.ships[0].target, s.ships[0].pos);
        assert_eq!(s.ships[0].q_cooldown, AFTERBURN_COOLDOWN - AFTERBURN_TICKS);
    }

    #[test]
    fn held_q_dashes_once_and_a_new_press_dashes_again() {
        let mut s = quiet(&[KITE]);
        let q = input(INPUT_SKILL_Q, 1000, 0);
        let mut dashes = 0;
        let mut was_dashing = false;
        // Hold Q for 10 s: more than two Afterburn cooldowns.
        for _ in 0..ticks(10, 1) {
            s.step(&[q]);
            let dashing = s.ships[0].dash_ticks > 0;
            dashes += (dashing && !was_dashing) as u32;
            was_dashing = dashing;
        }
        assert_eq!(dashes, 1);
        assert_eq!(s.ships[0].pos, FxVec2::from_px(AFTERBURN_DISTANCE, 0));
        // Release, then press again (the cooldown is long over).
        s.step(&idle(1));
        s.step(&[q]);
        // The dash starts this tick (its first step is already taken).
        assert_eq!(s.ships[0].dash_ticks, AFTERBURN_TICKS - 1);
        assert_eq!(s.ships[0].q_cooldown, AFTERBURN_COOLDOWN);
    }

    #[test]
    fn held_skills_never_refire() {
        // Every skill, both ships: holding Q+W for 30 s uses each once.
        for loadout in [KITE, BULWARK] {
            let mut s = quiet(&[loadout]);
            let (mut q_uses, mut w_uses) = (0, 0);
            for _ in 0..ticks(30, 1) {
                s.step(&[input(INPUT_SKILL_Q | INPUT_SKILL_W, 0, 300)]);
                let ship = &s.ships[0];
                q_uses += (ship.q_cooldown == ship.q_cooldown_max()) as u32;
                w_uses += (ship.w_cooldown == ship.w_cooldown_max()) as u32;
            }
            assert_eq!((q_uses, w_uses), (1, 1), "{loadout:?}");
        }
    }

    #[test]
    fn idle_ship_never_moves_by_itself() {
        // No input at all: no skill, upgrade or timer moves the ship.
        for loadout in [KITE, BULWARK] {
            let mut l = loadout;
            l.upgrades.thruster = MAX_UPGRADE_LEVEL;
            let mut s = quiet(&[l]);
            let start = s.ships[0].pos;
            for _ in 0..ticks(30, 1) {
                s.step(&idle(1));
                assert_eq!(s.ships[0].pos, start, "{l:?}");
            }
        }
    }

    #[test]
    fn kite_scatter_fires_a_cone() {
        let mut s = quiet(&[KITE]);
        s.step(&[input(INPUT_SKILL_W, 0, 300)]);
        assert_eq!(s.projectiles.len(), SCATTER_BOLTS as usize);
        let xs: Vec<i32> = s.projectiles.iter().map(|p| p.vel.x).collect();
        assert!(xs[0] > 0 && xs[2] == 0 && xs[4] < 0, "{xs:?}");
        assert_eq!(s.ships[0].w_cooldown, SCATTER_COOLDOWN);
    }

    #[test]
    fn bulwark_bastion_absorbs_damage() {
        let mut s = quiet(&[BULWARK]);
        s.step(&[input(INPUT_SKILL_Q, 0, 0)]);
        s.ships[0].take_damage(30);
        assert_eq!((s.ships[0].hull, s.ships[0].shield), (BULWARK_HULL, 10));
        s.ships[0].take_damage(30);
        assert_eq!((s.ships[0].hull, s.ships[0].shield), (BULWARK_HULL - 20, 0));
        // The shield expires.
        s.step(&[input(INPUT_SKILL_Q, 0, 0)]);
        assert!(s.ships[0].q_cooldown > 0);
        s.ships[0].shield = 5;
        s.ships[0].shield_ticks = 1;
        s.step(&idle(1));
        assert_eq!(s.ships[0].shield, 0);
    }

    #[test]
    fn bulwark_shockwave_damages_and_pushes() {
        let mut s = quiet(&[BULWARK]);
        s.enemies.push(enemy(EnemyKind::Spitter, 100, 0));
        s.enemies.push(enemy(EnemyKind::Spitter, 400, 0));
        s.step(&[input(INPUT_SKILL_W, 0, 0)]);
        assert_eq!(s.enemies[0].hp, SPITTER_HP - SHOCKWAVE_DAMAGE);
        assert!(s.enemies[0].pos.x >= px(195));
        assert_eq!(s.enemies[1].hp, SPITTER_HP);
    }

    #[test]
    fn swarmer_contact_damage_has_a_cooldown() {
        let mut s = quiet(&[KITE]);
        s.enemies.push(enemy(EnemyKind::Swarmer, 30, 0));
        // Don't let the basic attack kill it.
        s.ships[0].basic_cooldown = u32::MAX;
        let mut hits = Vec::new();
        for t in 0..200 {
            let before = s.ships[0].hull;
            s.step(&idle(1));
            if s.ships[0].hull < before {
                assert_eq!(before - s.ships[0].hull, SWARMER_CONTACT_DAMAGE);
                hits.push(t);
            }
        }
        assert!(hits.len() >= 3, "{hits:?}");
        assert!(hits
            .windows(2)
            .all(|w| w[1] - w[0] == SWARMER_CONTACT_COOLDOWN));
    }

    #[test]
    fn spitter_keeps_its_distance_and_fires() {
        let mut s = quiet(&[KITE]);
        s.ships[0].basic_cooldown = u32::MAX;
        s.enemies.push(enemy(EnemyKind::Spitter, 100, 0));
        for _ in 0..240 {
            s.step(&idle(1));
        }
        let d = s.enemies[0].pos.length();
        assert!(
            d >= px(SPITTER_MIN_RANGE) as i64 - px(2) as i64,
            "{}",
            d / SUB as i64
        );
        assert!(s.ships[0].hull < KITE_HULL, "spit hit");
    }

    #[test]
    fn enemies_chase_the_nearest_living_ship() {
        let mut s = quiet(&[KITE, KITE]);
        // Nearer ship 1 (x = +30) than ship 0 (x = -30).
        s.enemies.push(enemy(EnemyKind::Swarmer, 30, -500));
        s.step(&idle(2));
        assert_eq!(
            s.enemies[0].pos,
            FxVec2::new(px(30), px(-500) + px_per_tick(SWARMER_SPEED))
        );
        s.ships[1].hull = 0;
        let before = s.enemies[0].pos;
        s.step(&idle(2));
        assert!(s.enemies[0].pos.x < before.x, "now chases ship 0");
    }

    #[test]
    fn waves_follow_the_schedule() {
        let mut s = SimState::with_loadouts(3, &[KITE]);
        s.ships[0].basic_cooldown = u32::MAX;
        for _ in 0..WAVE_BANNER - 1 {
            s.step(&idle(1));
        }
        assert!(s.enemies.is_empty() && s.mission.banner_up());
        s.step(&idle(1));
        assert_eq!(s.enemies.len(), 4);
        for _ in 0..ticks(2, 1) {
            s.step(&idle(1));
        }
        assert_eq!(s.mission.spawned, 8);
        // Clearing the wave brings the next banner, then 6 + 2.
        s.ships[0].hull = 1000;
        s.enemies.clear();
        s.step(&idle(1));
        assert_eq!((s.mission.wave, s.mission.banner_up()), (2, true));
        for _ in 0..WAVE_BANNER {
            s.step(&idle(1));
        }
        let spitters = s
            .enemies
            .iter()
            .filter(|e| e.kind == EnemyKind::Spitter)
            .count();
        assert_eq!((s.enemies.len(), spitters), (8, 2));
        // Wave 3 comes on the timeout even if wave 2 isn't cleared.
        for _ in 0..WAVE_TIMEOUT {
            s.step(&idle(1));
        }
        assert_eq!(s.mission.wave, 3);
        for _ in 0..WAVE_BANNER + WAVE_TIMEOUT + 10 {
            s.ships[0].hull = 1000;
            s.step(&idle(1));
        }
        assert_eq!((s.mission.wave, s.mission.spawned), (3, 24));
    }

    #[test]
    fn spawns_stay_inside_the_arena() {
        let mut s = quiet(&[KITE]);
        s.ships[0].pos = FxVec2::from_px(780, 580);
        s.spawn_group(6, 2);
        for e in &s.enemies {
            let r = px(e.kind.radius());
            assert!(e.pos.x.abs() <= px(ARENA_HALF_W) - r && e.pos.y.abs() <= px(ARENA_HALF_H) - r);
        }
    }

    #[test]
    fn kill_target_wins_and_collects_the_field() {
        let mut s = quiet(&[KITE]);
        s.mission.kills = KILL_TARGET - 1;
        s.pickups.push(Pickup {
            kind: LootKind::Credits,
            amount: 7,
            pos: FxVec2::from_px(500, 500),
            ttl: PICKUP_LIFETIME,
        });
        let mut last = enemy(EnemyKind::Spitter, 0, 400);
        last.hp = 0;
        s.enemies.push(last);
        s.enemies.push(enemy(EnemyKind::Swarmer, -400, 0));
        s.step(&idle(1));
        let out = s.outcome().expect("mission over");
        assert!(out.success);
        assert_eq!(out.kills, KILL_TARGET);
        // 7 on the field + the Spitter's 15 (+ maybe a crystal).
        assert_eq!(out.collected.credits, 7 + SPITTER_CREDITS);
        assert_eq!(out.bonus.credits, SUCCESS_BONUS_CREDITS);
        assert_eq!(out.bonus.void_crystal, SUCCESS_BONUS_CRYSTAL);
        assert!(s.enemies.is_empty() && s.pickups.is_empty());
        // The world is frozen afterwards.
        let frozen = s.clone();
        s.step(&[input(INPUT_MOVE, 300, 0)]);
        assert_eq!(s.ships, frozen.ships);
        assert_eq!(s.mission.end_frame, frozen.frame - 1);
    }

    #[test]
    fn mission_fails_only_when_every_ship_is_destroyed() {
        let mut s = quiet(&[KITE, BULWARK]);
        s.mission.collected.credits = 40;
        s.ships[0].hull = 0;
        s.step(&idle(2));
        assert_eq!(s.outcome(), None);
        s.ships[1].hull = 0;
        s.step(&idle(2));
        let out = s.outcome().expect("mission over");
        assert!(!out.success);
        assert_eq!(out.collected.credits, 40);
        assert_eq!(out.bonus, Loot::default());
    }

    #[test]
    fn both_ships_can_win_and_lose_a_full_mission() {
        for loadout in [KITE, BULWARK] {
            // Lose: sit still and let the waves arrive.
            let mut s = SimState::with_loadouts(11, &[loadout]);
            s.ships[0].basic_cooldown = u32::MAX;
            let mut f = 0;
            while s.outcome().is_none() && f < 60 * 300 {
                s.ships[0].basic_cooldown = u32::MAX;
                s.step(&idle(1));
                f += 1;
            }
            assert_eq!(
                s.outcome().map(|o| o.success),
                Some(false),
                "{loadout:?} loses"
            );

            // Win: a hull too thick to break, guns and skills going.
            let mut s = SimState::with_loadouts(11, &[loadout]);
            let mut f = 0;
            while s.outcome().is_none() && f < 60 * 300 {
                s.ships[0].hull = 10_000;
                let e = s.enemies.first().map(|e| e.pos).unwrap_or_default();
                let buttons = if f % 30 == 0 { INPUT_SKILL_W } else { 0 };
                s.step(&[input(buttons, e.x / SUB, e.y / SUB)]);
                f += 1;
            }
            let out = s.outcome().expect("mission over");
            assert!(out.success, "{loadout:?} wins");
            assert!(out.kills >= KILL_TARGET && out.collected.credits > 0);
        }
    }

    #[test]
    fn kills_drop_loot_and_flying_over_collects_it() {
        let mut s = quiet(&[KITE]);
        let mut dead = enemy(EnemyKind::Swarmer, 300, 0);
        dead.hp = 0;
        s.enemies.push(dead);
        s.step(&idle(1));
        assert_eq!(s.mission.kills, 1);
        assert!(!s.pickups.is_empty());
        assert_eq!(s.pickups[0].kind, LootKind::Credits);
        // Out of drift range: it stays put.
        let pos = s.pickups[0].pos;
        s.step(&idle(1));
        assert_eq!(s.pickups[0].pos, pos);
        // Fly over it.
        s.step(&[input(INPUT_MOVE, 300, 0)]);
        for _ in 0..120 {
            s.step(&idle(1));
        }
        assert!(s.pickups.is_empty());
        assert_eq!(s.mission.collected.credits, SWARMER_CREDITS);
    }

    #[test]
    fn pickups_drift_in_and_expire() {
        let mut s = quiet(&[KITE]);
        s.pickups.push(Pickup {
            kind: LootKind::VoidCrystal,
            amount: 1,
            pos: FxVec2::from_px(70, 0),
            ttl: PICKUP_LIFETIME,
        });
        s.pickups.push(Pickup {
            kind: LootKind::Credits,
            amount: 5,
            pos: FxVec2::from_px(-500, 0),
            ttl: 3,
        });
        s.step(&idle(1));
        assert!(s.pickups[0].pos.x < px(70));
        for _ in 0..10 {
            s.step(&idle(1));
        }
        assert!(s.pickups.is_empty());
        assert_eq!(
            s.mission.collected,
            Loot {
                credits: 0,
                void_crystal: 1
            }
        );
    }

    #[test]
    fn crystal_drop_rate_follows_the_rng() {
        // Over many Swarmer kills the 15% roll lands near 15%.
        let mut s = quiet(&[KITE]);
        let mut crystals = 0;
        for _ in 0..1000 {
            let mut e = enemy(EnemyKind::Swarmer, 500, 500);
            e.hp = 0;
            s.enemies.push(e);
            s.mission.kills = 0;
            s.step(&idle(1));
            crystals += s
                .pickups
                .iter()
                .filter(|p| p.kind == LootKind::VoidCrystal)
                .count();
            s.pickups.clear();
        }
        assert!((100..=200).contains(&crystals), "{crystals}");
    }

    #[test]
    fn checksum_sees_state_changes() {
        let a = run(2, 400);
        let mut b = a.clone();
        b.ships[1].pos.x += 1;
        assert_ne!(a.checksum(), b.checksum());
        let mut c = a.clone();
        c.rng.next_u32();
        assert_ne!(a.checksum(), c.checksum());
        let mut d = a.clone();
        d.mission.collected.credits += 1;
        assert_ne!(a.checksum(), d.checksum());
    }
}
