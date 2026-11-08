//! The whole rolled-back simulation state and its fixed-timestep step.
//!
//! One Elimination mission (design/READINESS.md § Demo Spec): click-to-move
//! battleships with an auto-firing basic attack and two skills each, void
//! monster packs pouring out of void fissures (Swarmers and Spitters, and from
//! TAKOAI-116 the Lancer, Bloater, Brood and Warden roles), loot drops, and a
//! win/lose result.
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
use crate::event::{Attack, SimEvent, Skill};
use crate::fixed::{mul_q16, FxVec2, SUB};
use crate::input::{NetInput, INPUT_MOVE, INPUT_SKILL_Q, INPUT_SKILL_W};
use crate::mission::{rolled_kill_target, Mission, MissionOutcome, MissionStatus};
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
    /// Bloater bursts waiting to go off.
    pub blasts: Vec<Blast>,
    /// Where enemies come from; placed once at mission start.
    pub fissures: Vec<Fissure>,
    pub mission: Mission,
    /// The next stable entity ID to hand out (enemies, shots, pickups share
    /// one counter, starting at 1).
    pub next_id: u32,
    /// What happened during the last `step` (cleared at its start), for
    /// client feedback; see [`crate::event`].
    pub events: Vec<SimEvent>,
}

/// Hands out the next stable entity ID. A free function over the counter
/// field so it can run while other fields of `SimState` are borrowed.
fn new_id(next: &mut u32) -> u32 {
    let id = *next;
    *next += 1;
    id
}

const fn px(v: i32) -> i32 {
    v * SUB
}

fn ship_spawn(handle: usize, n: usize) -> FxVec2 {
    let x = (2 * handle as i32 - (n as i32 - 1)) * px(SHIP_SPAWN_GAP) / 2;
    FxVec2::new(x, 0)
}

/// Is a fissure at `p` out of the starting view, with margin?
fn offscreen_at_start(p: FxVec2) -> bool {
    p.x.abs() > px(INITIAL_VIEW_HALF_W + FISSURE_OFFSCREEN_MARGIN)
        || p.y.abs() > px(INITIAL_VIEW_HALF_H + FISSURE_OFFSCREEN_MARGIN)
}

/// Place 1-2 fissures on a ring `FISSURE_RING_MIN..=FISSURE_RING_MAX` px
/// around the ships' start (the origin), at seeded angles and apart from
/// each other, outside the starting view. Rejection sampling with a bounded number of tries, all from
/// `rng`, so every peer gets the same result.
fn place_fissures(rng: &mut SimRng) -> Vec<Fissure> {
    let count = rng.range(FISSURES_MIN, FISSURES_MAX);
    let gap = px(FISSURE_MIN_GAP) as i64;
    let mut fissures: Vec<Fissure> = Vec::new();
    for _ in 0..count {
        for _ in 0..256 {
            let angle = rng.range(0, ANGLE_STEPS - 1);
            let r = px(rng.range(FISSURE_RING_MIN, FISSURE_RING_MAX));
            let p = FxVec2::new(mul_q16(r, cos_q16(angle)), mul_q16(r, sin_q16(angle)));
            let apart = fissures
                .iter()
                .all(|f| (f.pos - p).length_squared() >= gap * gap);
            if offscreen_at_start(p) && apart {
                fissures.push(Fissure { pos: p });
                break;
            }
        }
    }
    if fissures.is_empty() {
        // Unreachable with the shipped tuning; never start without a source.
        fissures.push(Fissure {
            pos: FxVec2::from_px(FISSURE_RING_MAX, 0),
        });
    }
    fissures
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

/// The new roles in the order they unlock, with their unlock point
/// (percent of the kill target). `Mission::introduced` has a bit per index.
const ROLES: [(EnemyKind, u32); 4] = [
    (EnemyKind::Lancer, LANCER_UNLOCK_PCT),
    (EnemyKind::Bloater, BLOATER_UNLOCK_PCT),
    (EnemyKind::Warden, WARDEN_UNLOCK_PCT),
    (EnemyKind::Brood, BROOD_UNLOCK_PCT),
];

/// The living Warden shielding enemy `i`, if any: the first (lowest index)
/// within `WARDEN_RANGE`. Wardens don't shield each other.
pub fn warden_of(enemies: &[Enemy], i: usize) -> Option<usize> {
    let e = &enemies[i];
    if e.kind == EnemyKind::Warden || e.hp <= 0 {
        return None;
    }
    let range = px(WARDEN_RANGE) as i64;
    enemies.iter().position(|w| {
        w.kind == EnemyKind::Warden && w.hp > 0 && (w.pos - e.pos).length_squared() <= range * range
    })
}

/// Damage after a Warden's shield: it absorbs half, rounded down, so the
/// enemy takes half rounded up.
pub const fn warded(amount: i32) -> i32 {
    amount - amount / 2
}

/// Damage enemy `i` takes from a hit of `amount`, after any Warden.
fn damage_to(enemies: &[Enemy], i: usize, amount: i32) -> i32 {
    if warden_of(enemies, i).is_some() {
        warded(amount)
    } else {
        amount
    }
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
        let mut rng = SimRng::new(seed);
        let fissures = place_fissures(&mut rng);
        Self {
            params: SimParams {
                num_players: n,
                seed,
            },
            frame: 0,
            rng,
            ships,
            enemies: Vec::new(),
            projectiles: Vec::new(),
            enemy_projectiles: Vec::new(),
            pickups: Vec::new(),
            blasts: Vec::new(),
            fissures,
            mission: Mission {
                kill_target: rolled_kill_target(seed),
                ..Mission::default()
            },
            next_id: 1,
            events: Vec::new(),
        }
    }

    /// The same mission won at `kill_target` kills instead of the rolled
    /// one, capped at `KILL_TARGET_MAX`. A launch setting: call it before the
    /// first `step`.
    pub fn with_kill_target(mut self, kill_target: u32) -> Self {
        assert_eq!(self.frame, 0, "the kill target is set at launch");
        assert!(kill_target > 0, "kill target must be at least 1");
        self.mission.kill_target = kill_target.min(KILL_TARGET_MAX);
        self
    }

    /// The finished mission's result, or `None` while it is still running.
    pub fn outcome(&self) -> Option<MissionOutcome> {
        self.mission.outcome()
    }

    /// Advance one fixed tick. `inputs[h]` is player `h`'s input. Once the
    /// mission has ended the world is frozen and only `frame` advances.
    pub fn step(&mut self, inputs: &[NetInput]) {
        assert_eq!(inputs.len(), self.ships.len(), "one input per player");
        self.events.clear();
        if self.mission.status == MissionStatus::InProgress {
            self.direct_spawns();
            self.apply_inputs(inputs);
            self.move_ships();
            self.basic_attacks();
            self.enemy_behaviour();
            self.move_projectiles();
            self.projectile_hits();
            self.separate();
            self.update_blasts();
            self.kill_and_drop();
            self.update_pickups();
            self.check_end();
        }
        self.frame += 1;
    }

    /// The spawn director: every `pack_interval` ticks a pack comes out of
    /// a random fissure. A plain pack is `PACK_SWARMERS_MIN..=PACK_SWARMERS_MAX`
    /// Swarmers plus a Spitter for each of `PACK_SPITTER_ROLLS` rolls under
    /// `spitter_pct`, both ramping with mission time. Once kills unlock the
    /// new roles (`ROLES`), a pack can be a role pack instead (`role_pack`).
    /// A pack brings only what fits under `MAX_LIVE_ENEMIES`, its role
    /// enemies and Spitters first; with the field full the director waits.
    fn direct_spawns(&mut self) {
        let m = &mut self.mission;
        if m.spawn_timer > 0 {
            m.spawn_timer -= 1;
            if m.spawn_timer > 0 {
                return;
            }
        }
        let room = MAX_LIVE_ENEMIES.saturating_sub(self.enemies.len());
        if room == 0 {
            return;
        }
        let t = self.frame;
        let mut kinds = match self.pick_role() {
            Some(role) => self.role_pack(role),
            None => {
                let swarmers = self.rng.range(PACK_SWARMERS_MIN, PACK_SWARMERS_MAX) as usize;
                let spitters = self.spitter_rolls(PACK_SPITTER_ROLLS);
                core::iter::repeat_n(EnemyKind::Spitter, spitters)
                    .chain(core::iter::repeat_n(EnemyKind::Swarmer, swarmers))
                    .collect()
            }
        };
        kinds.truncate(room);
        self.spawn_pack(kinds);
        self.mission.spawn_timer = pack_interval(t);
    }

    /// How many of `rolls` Spitter rolls come up at the current ramp.
    fn spitter_rolls(&mut self, rolls: u32) -> usize {
        let pct = spitter_pct(self.frame);
        (0..rolls).filter(|_| self.rng.range(0, 99) < pct).count()
    }

    /// Which role this pack is built round, if any. A role unlocks when the
    /// kills reach its share of the kill target; the first pack after that
    /// always brings it, later ones `ROLE_PACK_PCT`% of the time (evenly
    /// among the unlocked roles). No RNG is drawn before the first unlock.
    fn pick_role(&mut self) -> Option<EnemyKind> {
        let m = &mut self.mission;
        let unlocked: Vec<usize> = (0..ROLES.len())
            .filter(|&i| m.kills as u64 * 100 >= ROLES[i].1 as u64 * m.kill_target as u64)
            .collect();
        if let Some(&i) = unlocked.iter().find(|&&i| m.introduced & (1 << i) == 0) {
            m.introduced |= 1 << i;
            return Some(ROLES[i].0);
        }
        if unlocked.is_empty() || self.rng.range(0, 99) >= ROLE_PACK_PCT {
            return None;
        }
        let pick = self.rng.range(0, unlocked.len() as i32 - 1) as usize;
        Some(ROLES[unlocked[pick]].0)
    }

    /// The mixed pack for a role, the role's enemies first: Swarmers with
    /// Lancers; Swarmers with Bloaters among them; a Warden over Swarmers
    /// (and maybe a Spitter); a Brood behind Spitters and a few Swarmers.
    fn role_pack(&mut self, role: EnemyKind) -> Vec<EnemyKind> {
        let mut roll = |(lo, hi): (i32, i32)| self.rng.range(lo, hi) as usize;
        let (leaders, spitters, swarmers) = match role {
            EnemyKind::Lancer => (roll(LANCER_PACK_LANCERS), 0, roll(LANCER_PACK_SWARMERS)),
            EnemyKind::Bloater => (roll(BLOATER_PACK_BLOATERS), 0, roll(BLOATER_PACK_SWARMERS)),
            EnemyKind::Brood => (1, roll(BROOD_PACK_SPITTERS), roll(BROOD_PACK_SWARMERS)),
            _ => {
                let swarmers = roll(WARDEN_PACK_SWARMERS);
                (1, self.spitter_rolls(1), swarmers)
            }
        };
        core::iter::repeat_n(role, leaders)
            .chain(core::iter::repeat_n(EnemyKind::Spitter, spitters))
            .chain(core::iter::repeat_n(EnemyKind::Swarmer, swarmers))
            .collect()
    }

    /// Spawn a pack evenly round the spawn ring of one random fissure,
    /// starting at a random angle. Spitters strafe a random way round.
    fn spawn_pack(&mut self, kinds: Vec<EnemyKind>) {
        let f = self.rng.range(0, self.fissures.len() as i32 - 1) as usize;
        let start = self.rng.range(0, ANGLE_STEPS - 1);
        let r = px(FISSURE_SPAWN_RING);
        let n = kinds.len() as i32;
        for (i, kind) in kinds.into_iter().enumerate() {
            let angle = (start + i as i32 * ANGLE_STEPS / n) % ANGLE_STEPS;
            let pos = self.fissures[f].pos
                + FxVec2::new(mul_q16(r, cos_q16(angle)), mul_q16(r, sin_q16(angle)));
            let mut e = Enemy::new(kind, pos);
            e.id = new_id(&mut self.next_id);
            e.home = self.fissures[f].pos;
            if kind == EnemyKind::Spitter && self.rng.range(0, 1) == 0 {
                e.orbit = -1;
            }
            self.enemies.push(e);
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
                ship.target = aim;
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
                ship.q_cooldown = ship.stats.q_cooldown;
                self.events.push(SimEvent::SkillUsed {
                    ship: ship.handle,
                    skill: Skill::Afterburn,
                    pos: ship.pos,
                });
            }
            ShipKind::Bulwark => {
                // Bastion: absorb the next N damage for a while.
                ship.shield = BASTION_SHIELD;
                ship.shield_ticks = BASTION_DURATION;
                ship.q_cooldown = ship.stats.q_cooldown;
                self.events.push(SimEvent::SkillUsed {
                    ship: ship.handle,
                    skill: Skill::Bastion,
                    pos: ship.pos,
                });
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
                        id: new_id(&mut self.next_id),
                        owner,
                        pos,
                        vel: rotate(dir, angle),
                        ttl: SCATTER_LIFETIME,
                        damage,
                        radius: SCATTER_RADIUS,
                        attack: Attack::Scatter,
                    });
                }
                self.ships[i].w_cooldown = self.ships[i].stats.w_cooldown;
                self.events.push(SimEvent::SkillUsed {
                    ship: owner,
                    skill: Skill::Scatter,
                    pos,
                });
            }
            ShipKind::Bulwark => {
                // Shockwave: damage and push back every enemy in range.
                let (centre, damage) = (ship.pos, ship.stats.skill_damage(SHOCKWAVE_DAMAGE));
                let handle = ship.handle;
                self.events.push(SimEvent::SkillUsed {
                    ship: handle,
                    skill: Skill::Shockwave,
                    pos: centre,
                });
                let dealt: Vec<i32> = (0..self.enemies.len())
                    .map(|j| damage_to(&self.enemies, j, damage))
                    .collect();
                for (e, dealt) in self.enemies.iter_mut().zip(dealt) {
                    if !overlaps(centre, px(SHOCKWAVE_RADIUS), e.pos, px(e.kind.radius())) {
                        continue;
                    }
                    e.hp -= dealt;
                    self.events.push(SimEvent::Hit {
                        enemy: e.id,
                        ship: handle,
                        attack: Attack::Shockwave,
                        source: 0,
                        amount: dealt,
                        pos: e.pos,
                    });
                    let d = e.pos - centre;
                    let push = if d == FxVec2::ZERO {
                        FxVec2::new(px(SHOCKWAVE_PUSH), 0)
                    } else {
                        d.scale_to(px(SHOCKWAVE_PUSH))
                    };
                    e.pos = e.pos + push;
                }
                self.ships[i].w_cooldown = self.ships[i].stats.w_cooldown;
            }
        }
    }

    fn move_ships(&mut self) {
        for ship in self.ships.iter_mut().filter(|s| s.alive()) {
            if ship.dash_ticks > 0 {
                ship.pos = ship.pos + ship.dash_vel;
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
                ship.pos + d.scale_to(speed)
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
            let id = new_id(&mut self.next_id);
            self.projectiles.push(Projectile {
                id,
                owner: ship.handle,
                pos: ship.pos,
                vel,
                ttl: (ship.stats.basic_range * TICKS_PER_SEC / BOLT_SPEED) as u32 + 2,
                damage: ship.stats.basic_damage,
                radius: BOLT_RADIUS,
                attack: Attack::Bolt,
            });
            ship.basic_cooldown = ship.stats.basic_interval;
            self.events.push(SimEvent::Fired {
                ship: ship.handle,
                shot: id,
            });
        }
    }

    /// Each enemy acts on the nearest living ship, by role:
    /// - Swarmers rush it and hit on contact.
    /// - Spitters close to a distance band, then strafe around it, and stop
    ///   only for the 0.4 s wind-up before each shot (aim locked at its start).
    /// - Lancers walk in, stop for a 0.7 s wind-up with the dash locked, then
    ///   dash `LANCER_DASH_DISTANCE` along it, hitting each ship once.
    /// - Bloaters waddle in; touching a ship arms a 1 s fuse (it stops) and
    ///   the burst goes off when it runs out (`kill_and_drop`, `update_blasts`).
    /// - Broods hold near their fissure, toward the ship, and every
    ///   `BROOD_INTERVAL` pulse for `BROOD_WINDUP` and release Swarmers.
    /// - Wardens stay by the nearest ally but out of the ship's reach.
    fn enemy_behaviour(&mut self) {
        let mut releases: Vec<FxVec2> = Vec::new();
        for i in 0..self.enemies.len() {
            // A Warden's ally, read before `e` borrows the list.
            let ally = (self.enemies[i].kind == EnemyKind::Warden)
                .then(|| {
                    let at = self.enemies[i].pos;
                    self.enemies
                        .iter()
                        .filter(|a| a.kind != EnemyKind::Warden && a.hp > 0)
                        .min_by_key(|a| (a.pos - at).length_squared())
                        .map(|a| a.pos)
                })
                .flatten();
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
                        let hull_lost = ship.take_damage(SWARMER_CONTACT_DAMAGE);
                        e.cooldown = SWARMER_CONTACT_COOLDOWN;
                        self.events.push(SimEvent::ShipDamaged {
                            ship: h,
                            source: e.id,
                            amount: SWARMER_CONTACT_DAMAGE,
                            hull_lost,
                            pos: ship.pos,
                        });
                    }
                }
                EnemyKind::Spitter => {
                    // Winding up: hold still, then fire along the locked aim.
                    if e.windup > 0 {
                        e.windup -= 1;
                        if e.windup == 0 {
                            self.enemy_projectiles.push(Projectile {
                                id: new_id(&mut self.next_id),
                                owner: 0,
                                pos: e.pos,
                                vel: e.aim,
                                ttl: SPIT_LIFETIME,
                                damage: SPIT_DAMAGE,
                                radius: SPIT_RADIUS,
                                attack: Attack::Bolt,
                            });
                            e.cooldown = SPITTER_FIRE_INTERVAL - SPITTER_WINDUP;
                        }
                        continue;
                    }
                    let d = to_ship.length();
                    let step = if d < px(SPITTER_MIN_RANGE) as i64 {
                        FxVec2::ZERO - to_ship.scale_to(speed)
                    } else if d > px(SPITTER_MAX_RANGE) as i64 {
                        to_ship.scale_to(speed)
                    } else {
                        // In the band: strafe along the circle round the ship.
                        FxVec2::new(-to_ship.y * e.orbit, to_ship.x * e.orbit).scale_to(speed)
                    };
                    e.pos = e.pos + step;
                    let vel = to_ship.scale_to(px_per_tick(SPIT_SPEED));
                    if e.cooldown == 0 && d <= px(SPITTER_FIRE_RANGE) as i64 && vel != FxVec2::ZERO
                    {
                        e.windup = SPITTER_WINDUP;
                        e.aim = vel;
                    }
                }
                EnemyKind::Lancer => {
                    if e.dash > 0 {
                        e.pos = e.pos + e.aim;
                        e.dash -= 1;
                        for s in self.ships.iter_mut().filter(|s| s.alive()) {
                            let bit = 1 << s.handle;
                            let reach = px(s.stats.radius + CONTACT_REACH);
                            if e.struck & bit != 0
                                || !overlaps(e.pos, px(e.kind.radius()), s.pos, reach)
                            {
                                continue;
                            }
                            e.struck |= bit;
                            let hull_lost = s.take_damage(LANCER_DASH_DAMAGE);
                            self.events.push(SimEvent::ShipDamaged {
                                ship: s.handle,
                                source: e.id,
                                amount: LANCER_DASH_DAMAGE,
                                hull_lost,
                                pos: s.pos,
                            });
                        }
                        if e.dash == 0 {
                            e.cooldown = LANCER_RECOVER;
                        }
                        continue;
                    }
                    if e.windup > 0 {
                        e.windup -= 1;
                        if e.windup == 0 {
                            e.dash = LANCER_DASH_TICKS;
                            e.struck = 0;
                        }
                        continue;
                    }
                    let d = to_ship.length();
                    let vel = to_ship.scale_to(px(LANCER_DASH_DISTANCE) / LANCER_DASH_TICKS as i32);
                    if e.cooldown == 0 && d <= px(LANCER_CHARGE_RANGE) as i64 && vel != FxVec2::ZERO
                    {
                        e.windup = LANCER_WINDUP;
                        e.aim = vel;
                    } else if d > px(LANCER_CHARGE_RANGE / 2) as i64 {
                        e.pos = e.pos + to_ship.scale_to(speed);
                    }
                }
                EnemyKind::Bloater => {
                    if e.struck != 0 {
                        // Armed: swell in place; at 0 it dies and bursts.
                        e.windup = e.windup.saturating_sub(1);
                        if e.windup == 0 {
                            e.hp = 0;
                        }
                        continue;
                    }
                    if to_ship.length() > speed as i64 {
                        e.pos = e.pos + to_ship.scale_to(speed);
                    }
                    let reach = ship.stats.radius + CONTACT_REACH;
                    if overlaps(e.pos, px(e.kind.radius()), ship.pos, px(reach)) {
                        e.struck = 1 << h;
                        e.windup = BLOATER_FUSE;
                    }
                }
                EnemyKind::Brood => {
                    if e.windup > 0 {
                        e.windup -= 1;
                        if e.windup == 0 {
                            releases.push(e.pos);
                            e.cooldown = BROOD_INTERVAL - BROOD_WINDUP;
                        }
                        continue;
                    }
                    let toward = ship.pos - e.home;
                    let hold = if toward == FxVec2::ZERO {
                        e.home
                    } else {
                        e.home + toward.scale_to(px(BROOD_HOLD))
                    };
                    let d = hold - e.pos;
                    e.pos = if d.length() <= speed as i64 {
                        hold
                    } else {
                        e.pos + d.scale_to(speed)
                    };
                    if e.cooldown == 0 {
                        e.windup = BROOD_WINDUP;
                    }
                }
                EnemyKind::Warden => {
                    let d = to_ship.length();
                    let step = if d < px(WARDEN_KEEP_AWAY) as i64 {
                        FxVec2::ZERO - to_ship.scale_to(speed)
                    } else if let Some(ally) = ally {
                        // Sit with the ally, close enough to shield it.
                        let to_ally = ally - e.pos;
                        if to_ally.length() > px(WARDEN_RANGE / 2) as i64 {
                            to_ally.scale_to(speed)
                        } else {
                            FxVec2::ZERO
                        }
                    } else if d > px(WARDEN_KEEP_AWAY + 60) as i64 {
                        to_ship.scale_to(speed)
                    } else {
                        FxVec2::ZERO
                    };
                    e.pos = e.pos + step;
                }
            }
        }
        for at in releases {
            self.release_swarmers(at);
        }
    }

    /// A Brood at `at` releases `BROOD_SWARMERS` Swarmers round itself, as
    /// many as fit under `MAX_LIVE_ENEMIES`.
    fn release_swarmers(&mut self, at: FxVec2) {
        let room = MAX_LIVE_ENEMIES.saturating_sub(self.enemies.len());
        let n = (BROOD_SWARMERS as usize).min(room);
        let start = self.rng.range(0, ANGLE_STEPS - 1);
        let r = px(BROOD_RELEASE_RING);
        for k in 0..n as i32 {
            let angle = (start + k * ANGLE_STEPS / BROOD_SWARMERS) % ANGLE_STEPS;
            let pos = at + FxVec2::new(mul_q16(r, cos_q16(angle)), mul_q16(r, sin_q16(angle)));
            let mut e = Enemy::new(EnemyKind::Swarmer, pos);
            e.id = new_id(&mut self.next_id);
            self.enemies.push(e);
            self.mission.spawned += 1;
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
    /// A ship's shot also knocks the enemy back a little along its flight.
    fn projectile_hits(&mut self) {
        for p in &mut self.projectiles {
            let hit = self.enemies.iter().position(|e| {
                e.hp > 0 && overlaps(p.pos, px(p.radius), e.pos, px(e.kind.radius()))
            });
            if let Some(i) = hit {
                let dealt = damage_to(&self.enemies, i, p.damage);
                let e = &mut self.enemies[i];
                e.hp -= dealt;
                p.ttl = 0;
                let knockback = match p.attack {
                    Attack::Scatter => SCATTER_KNOCKBACK,
                    _ => BOLT_KNOCKBACK,
                };
                e.pos = e.pos + p.vel.scale_to(px(knockback));
                self.events.push(SimEvent::Hit {
                    enemy: e.id,
                    ship: p.owner,
                    attack: p.attack,
                    source: p.id,
                    amount: dealt,
                    pos: e.pos,
                });
            }
        }
        for p in &mut self.enemy_projectiles {
            let hit = self
                .ships
                .iter_mut()
                .find(|s| s.alive() && overlaps(p.pos, px(p.radius), s.pos, px(s.stats.radius)));
            if let Some(s) = hit {
                let hull_lost = s.take_damage(p.damage);
                p.ttl = 0;
                self.events.push(SimEvent::ShipDamaged {
                    ship: s.handle,
                    source: p.id,
                    amount: p.damage,
                    hull_lost,
                    pos: s.pos,
                });
            }
        }
        self.projectiles.retain(|p| p.ttl > 0);
        self.enemy_projectiles.retain(|p| p.ttl > 0);
    }

    /// Ships and enemies push apart, split by mass (the lighter one moves
    /// more); enemies push each other apart evenly. Ships never collide with
    /// each other, and a dashing Lancer runs straight through ships.
    fn separate(&mut self) {
        let total = SHIP_MASS + ENEMY_MASS;
        for ship in self.ships.iter_mut().filter(|s| s.alive()) {
            for e in self.enemies.iter_mut().filter(|e| e.dash == 0) {
                let Some(push) =
                    separation(ship.pos, px(ship.stats.radius), e.pos, px(e.kind.radius()))
                else {
                    continue;
                };
                let ship_share =
                    FxVec2::new(push.x * ENEMY_MASS / total, push.y * ENEMY_MASS / total);
                ship.pos = ship.pos + ship_share;
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
    }

    /// Bloater bursts tick down and go off at 0: every living ship and enemy
    /// within `BLOATER_BLAST_RADIUS` takes damage (Wardens shield enemies as
    /// usual), so a burst can kill more Bloaters and chain.
    fn update_blasts(&mut self) {
        let r = px(BLOATER_BLAST_RADIUS);
        for b in &mut self.blasts {
            b.fuse = b.fuse.saturating_sub(1);
            if b.fuse > 0 {
                continue;
            }
            self.events.push(SimEvent::Burst {
                blast: b.id,
                pos: b.pos,
            });
            for ship in self.ships.iter_mut().filter(|s| s.alive()) {
                if !overlaps(b.pos, r, ship.pos, px(ship.stats.radius)) {
                    continue;
                }
                let hull_lost = ship.take_damage(BLOATER_BLAST_DAMAGE);
                self.events.push(SimEvent::ShipDamaged {
                    ship: ship.handle,
                    source: b.id,
                    amount: BLOATER_BLAST_DAMAGE,
                    hull_lost,
                    pos: ship.pos,
                });
            }
            let dealt: Vec<i32> = (0..self.enemies.len())
                .map(|j| damage_to(&self.enemies, j, BLOATER_BLAST_ENEMY_DAMAGE))
                .collect();
            for (e, dealt) in self.enemies.iter_mut().zip(dealt) {
                if e.hp <= 0 || !overlaps(b.pos, r, e.pos, px(e.kind.radius())) {
                    continue;
                }
                e.hp -= dealt;
                self.events.push(SimEvent::Hit {
                    enemy: e.id,
                    ship: 0,
                    attack: Attack::Blast,
                    source: b.id,
                    amount: dealt,
                    pos: e.pos,
                });
            }
        }
        self.blasts.retain(|b| b.fuse > 0);
    }

    /// Dead enemies count as kills and drop loot rolled with the sim RNG.
    fn kill_and_drop(&mut self) {
        for e in self.enemies.iter().filter(|e| e.hp <= 0) {
            self.mission.kills += 1;
            self.events.push(SimEvent::Killed {
                enemy: e.id,
                kind: e.kind,
                pos: e.pos,
            });
            if e.kind == EnemyKind::Bloater {
                // Killed: the full fuse. Burst on contact: what's left of it.
                self.blasts.push(Blast {
                    id: new_id(&mut self.next_id),
                    pos: e.pos,
                    fuse: if e.struck != 0 {
                        e.windup
                    } else {
                        BLOATER_FUSE
                    },
                });
            }
            let (credits, crystal_pct) = e.kind.loot();
            self.pickups.push(Pickup {
                id: new_id(&mut self.next_id),
                kind: LootKind::Credits,
                amount: credits,
                pos: e.pos,
                ttl: PICKUP_LIFETIME,
            });
            if self.rng.range(0, 99) < crystal_pct {
                self.pickups.push(Pickup {
                    id: new_id(&mut self.next_id),
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
                self.events.push(SimEvent::Collected {
                    ship: h,
                    pickup: p.id,
                    kind: p.kind,
                    amount: p.amount,
                });
            }
        }
        self.pickups.retain(|p| p.ttl > 0);
    }

    /// Success at the kill target (the field is cleared and every pickup
    /// left is collected); failure when every ship is destroyed.
    fn check_end(&mut self) {
        let m = &mut self.mission;
        if m.kills >= m.kill_target {
            m.status = MissionStatus::Success;
            for p in self.pickups.drain(..) {
                m.collected.add(p.kind, p.amount);
            }
            self.enemies.clear();
            self.blasts.clear();
            self.projectiles.clear();
            self.enemy_projectiles.clear();
        } else if self.ships.iter().all(|s| !s.alive()) {
            m.status = MissionStatus::Failed;
        } else {
            return;
        }
        m.end_frame = self.frame;
        self.events.push(SimEvent::MissionEnded {
            success: m.status == MissionStatus::Success,
        });
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
            training: 0,
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

    /// Ships only: the spawn director is held off so nothing spawns.
    fn quiet(loadouts: &[Loadout]) -> SimState {
        let mut s = SimState::with_loadouts(7, loadouts);
        s.mission.spawn_timer = u32::MAX;
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
        assert!(s.mission.spawned > 10, "spawned {}", s.mission.spawned);
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
            training: 0,
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
    fn no_arena_edge_blocks_movement() {
        // Far past where the old 2000 × 1160 arena ended.
        let mut s = quiet(&[KITE]);
        s.step(&[input(INPUT_MOVE, 3000, -2000)]);
        for _ in 0..ticks(20, 1) {
            s.step(&idle(1));
        }
        assert_eq!(s.ships[0].pos, FxVec2::from_px(3000, -2000));
        // Enemies spawned out there aren't pulled in either.
        s.enemies.push(enemy(EnemyKind::Spitter, -5000, 4000));
        s.step(&idle(1));
        assert!(s.enemies[0].pos.x < px(-4900) && s.enemies[0].pos.y > px(3900));
    }

    #[test]
    fn basic_attack_auto_fires_at_nearest_enemy_in_range() {
        let mut s = quiet(&[BULWARK]);
        // A Swarmer flies straight in, so the bolt can't miss it.
        s.enemies.push(enemy(EnemyKind::Swarmer, 230, 0));
        s.enemies.push(enemy(EnemyKind::Spitter, 0, 500)); // out of range
        s.step(&idle(1));
        assert_eq!(s.projectiles.len(), 1);
        assert!(s.projectiles[0].vel.x > 0 && s.projectiles[0].vel.y == 0);
        // Next shot only after the interval.
        for _ in 0..BULWARK_BASIC_INTERVAL - 1 {
            s.step(&idle(1));
        }
        // One bolt, one dead Swarmer; the next shot waits for the Spitter.
        assert_eq!(s.mission.kills, 1);
        assert_eq!(s.enemies.len(), 1);
        s.step(&idle(1));
        assert!(s.projectiles.is_empty());
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
    fn fissures_start_off_screen_on_a_ring() {
        let mut counts = [0; 3];
        for seed in 0..500 {
            let s = SimState::with_loadouts(seed, &mixed(1 + seed as usize % MAX_PLAYERS));
            counts[s.fissures.len()] += 1;
            for f in &s.fissures {
                let p = FxVec2::new(f.pos.x / SUB, f.pos.y / SUB);
                // Outside the starting view, with room for the sprite and
                // any ship's spawn offset.
                assert!(
                    p.x.abs() >= INITIAL_VIEW_HALF_W + FISSURE_RADIUS + 90
                        || p.y.abs() >= INITIAL_VIEW_HALF_H + FISSURE_RADIUS + 90,
                    "seed {seed}: {p:?}"
                );
                // On the ring around the start (1 px of rounding).
                let d = f.pos.length() / SUB as i64;
                assert!(
                    (FISSURE_RING_MIN as i64 - 1..=FISSURE_RING_MAX as i64 + 1).contains(&d),
                    "seed {seed}: {d}"
                );
            }
            if let [a, b] = &s.fissures[..] {
                assert!((a.pos - b.pos).length() >= px(FISSURE_MIN_GAP) as i64);
            }
        }
        // Both counts happen, never zero.
        assert_eq!(counts[0], 0);
        assert!(counts[1] > 100 && counts[2] > 100, "{counts:?}");
        // Seeded: same seed, same fissures; different seeds differ.
        let f = |seed| SimState::with_loadouts(seed, &[KITE]).fissures;
        assert_eq!(f(5), f(5));
        assert!((0..20).any(|seed| f(seed) != f(5)));
    }

    #[test]
    fn enemies_spawn_on_a_ring_around_a_fissure() {
        let mut s = SimState::with_loadouts(3, &[KITE]);
        s.ships[0].basic_cooldown = u32::MAX;
        s.ships[0].hull = 100_000;
        let mut seen = 0;
        for _ in 0..ticks(60, 1) {
            let before = s.enemies.len();
            s.step(&idle(1));
            for e in &s.enemies[before.min(s.enemies.len())..] {
                // Spawned this tick, before it moved: on some fissure's ring.
                let on_ring = s.fissures.iter().any(|f| {
                    let d = (e.pos - f.pos).length();
                    (d - px(FISSURE_SPAWN_RING) as i64).abs() <= px(2) as i64
                });
                let moved = e.kind.speed() as i64 + px(e.kind.radius()) as i64;
                let near = s
                    .fissures
                    .iter()
                    .any(|f| (e.pos - f.pos).length() <= px(FISSURE_SPAWN_RING) as i64 + moved);
                assert!(on_ring || near, "{e:?} vs {:?}", s.fissures);
                seen += 1;
            }
        }
        assert!(seen > 10, "{seen}");
    }

    #[test]
    fn director_releases_packs_faster_with_more_spitters_up_to_a_cap() {
        // The curve: packs closer together and more Spitters over the ramp,
        // flat after.
        assert_eq!(pack_interval(0), PACK_INTERVAL_START);
        assert_eq!(pack_interval(SPAWN_RAMP), PACK_INTERVAL_END);
        assert_eq!(pack_interval(SPAWN_RAMP * 10), PACK_INTERVAL_END);
        assert_eq!(spitter_pct(0), SPITTER_PCT_START);
        assert_eq!(spitter_pct(SPAWN_RAMP * 10), SPITTER_PCT_END);
        for t in 0..SPAWN_RAMP {
            assert!(pack_interval(t + 1) <= pack_interval(t));
            assert!(spitter_pct(t + 1) >= spitter_pct(t));
        }
        // In the sim: each pulse is a whole pack out of one fissure, pulses
        // get closer together, Spitters get commoner, and the live count
        // never passes the cap.
        let mut s = SimState::with_loadouts(9, &[KITE]);
        let mut packs: Vec<(u32, Vec<EnemyKind>)> = Vec::new();
        for _ in 0..ticks(240, 1) {
            s.ships[0].hull = 100_000;
            s.ships[0].basic_cooldown = u32::MAX;
            let before = s.mission.spawned as usize;
            let live = s.enemies.len();
            s.step(&idle(1));
            let new = s.mission.spawned as usize - before;
            if new > 0 {
                let pack = &s.enemies[live..];
                assert_eq!(pack.len(), new);
                let swarmers = pack.iter().filter(|e| e.kind == EnemyKind::Swarmer).count();
                assert!(
                    (PACK_SWARMERS_MIN as usize..=PACK_SWARMERS_MAX as usize).contains(&swarmers)
                );
                assert!(new - swarmers <= PACK_SPITTER_ROLLS as usize);
                // All from the same fissure (pushed apart a little already).
                let near = |f: &Fissure| {
                    pack.iter()
                        .all(|e| (e.pos - f.pos).length() <= px(FISSURE_SPAWN_RING + 40) as i64)
                };
                assert!(s.fissures.iter().any(near), "pack split across fissures");
                packs.push((s.frame - 1, pack.iter().map(|e| e.kind).collect()));
            }
            assert!(s.enemies.len() <= MAX_LIVE_ENEMIES);
            // Thin the field now and then so the cap doesn't stop the curve.
            if s.frame.is_multiple_of(ticks(5, 1)) {
                s.enemies.truncate(2);
            }
        }
        assert_eq!(packs[0].0, SPAWN_FIRST - 1);
        let gaps: Vec<_> = packs.windows(2).map(|w| w[1].0 - w[0].0).collect();
        assert_eq!(gaps[0], pack_interval(SPAWN_FIRST - 1));
        assert!(gaps.iter().rev().take(10).all(|&g| g == PACK_INTERVAL_END));
        let spit = |p: &[(u32, Vec<EnemyKind>)]| {
            p.iter()
                .flat_map(|(_, k)| k)
                .filter(|k| **k == EnemyKind::Spitter)
                .count()
        };
        let half = packs.len() / 2;
        assert!(spit(&packs[half..]) > spit(&packs[..half]), "{packs:?}");
        // With nobody killing anything, the field fills to the cap and stops.
        let mut s = SimState::with_loadouts(9, &[KITE]);
        for _ in 0..ticks(120, 1) {
            s.ships[0].hull = 100_000;
            s.ships[0].basic_cooldown = u32::MAX;
            s.step(&idle(1));
        }
        assert_eq!(s.enemies.len(), MAX_LIVE_ENEMIES);
        assert_eq!(s.mission.spawned as usize, MAX_LIVE_ENEMIES);
    }

    #[test]
    fn one_basic_hit_kills_a_swarmer_from_any_ship() {
        for loadout in [KITE, BULWARK] {
            let stats = ShipStats::new(loadout);
            assert!(stats.basic_damage >= SWARMER_HP, "{loadout:?}");
        }
        let mut s = quiet(&[KITE]);
        s.enemies.push(enemy(EnemyKind::Swarmer, 200, 0));
        let mut f = 0;
        while s.mission.kills == 0 {
            s.step(&idle(1));
            f += 1;
            assert!(f < KITE_BASIC_INTERVAL, "killed by the first bolt");
        }
    }

    #[test]
    fn spitter_winds_up_then_fires_along_the_locked_aim() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        let mut e = enemy(EnemyKind::Spitter, 260, 0);
        e.cooldown = 0;
        s.enemies.push(e);
        s.step(&idle(1));
        // It starts winding up, aimed at the ship, and stops moving.
        let e = s.enemies[0].clone();
        assert_eq!(e.windup, SPITTER_WINDUP);
        assert!(e.aim.x < 0 && e.aim.y == 0, "{:?}", e.aim);
        assert!(s.enemy_projectiles.is_empty());
        // The ship moving away doesn't turn the aim.
        s.ships[0].pos = FxVec2::from_px(0, 300);
        for _ in 1..SPITTER_WINDUP {
            s.step(&idle(1));
            assert_eq!(s.enemies[0].pos, e.pos, "holds still");
            assert_eq!(s.enemies[0].aim, e.aim);
            assert!(s.enemy_projectiles.is_empty(), "no shot before 0.4 s");
        }
        s.step(&idle(1));
        assert_eq!(s.enemy_projectiles.len(), 1);
        assert_eq!(s.enemy_projectiles[0].vel, e.aim);
        assert_eq!(s.enemies[0].windup, 0);
        // The next shot comes a fire interval after this one's wind-up began.
        assert_eq!(
            s.enemies[0].cooldown,
            SPITTER_FIRE_INTERVAL - SPITTER_WINDUP
        );
    }

    /// B2 (design/BATTLE.md): enemy shots are slow enough that the slowest
    /// ship dodges by moving sideways once the wind-up shows, even from the
    /// Spitter's closest band edge.
    #[test]
    fn the_slowest_ship_can_dodge_a_spitter_shot() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        let mut e = enemy(EnemyKind::Spitter, SPITTER_MIN_RANGE, 0);
        e.cooldown = 0;
        s.enemies.push(e);
        s.step(&idle(1));
        assert_eq!(s.enemies[0].windup, SPITTER_WINDUP);
        // A quarter-second reaction, then fly straight across the line.
        for _ in 0..ticks(1, 4) {
            s.step(&idle(1));
        }
        for _ in 0..ticks(3, 1) {
            s.step(&[input(INPUT_MOVE, 0, 2000)]);
        }
        assert_eq!(s.ships[0].hull, BULWARK_HULL, "dodged");
    }

    #[test]
    fn spitters_strafe_inside_their_band() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        s.ships[0].hull = 100_000;
        s.enemies.push(enemy(EnemyKind::Spitter, 260, 0));
        let mut last = s.enemies[0].pos;
        for t in 0..ticks(20, 1) {
            let winding = s.enemies[0].windup > 0;
            s.step(&idle(1));
            let e = &s.enemies[0];
            if !winding && e.windup == 0 {
                assert_ne!(e.pos, last, "tick {t}: Spitter stood still");
            }
            last = e.pos;
            let d = e.pos.length();
            assert!(
                d >= px(SPITTER_MIN_RANGE - 2) as i64 && d <= px(SPITTER_MAX_RANGE + 2) as i64,
                "tick {t}: {}",
                d / SUB as i64
            );
        }
        // It went a good way round the ship, not back and forth on the spot.
        assert!(s.enemies[0].pos.y.abs() > px(100) || s.enemies[0].pos.x < 0);
    }

    #[test]
    fn kill_target_wins_and_collects_the_field() {
        let mut s = quiet(&[KITE]);
        let target = s.mission.kill_target;
        assert_eq!(target, rolled_kill_target(7));
        s.mission.kills = target - 1;
        s.pickups.push(Pickup {
            id: 0,
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
        assert_eq!(out.kills, target);
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
    fn a_ten_kill_mission_ends_at_ten_kills() {
        for loadout in [KITE, BULWARK] {
            let mut s =
                SimState::with_loadouts(11, &[loadout]).with_kill_target(AUTOPLAY_KILL_TARGET);
            let mut f = 0;
            while s.outcome().is_none() && f < 60 * 300 {
                s.ships[0].hull = 10_000;
                assert!(s.mission.kills < AUTOPLAY_KILL_TARGET);
                let e = s.enemies.first().map(|e| e.pos).unwrap_or_default();
                let buttons = if f % 30 == 0 { INPUT_SKILL_W } else { 0 };
                s.step(&[input(buttons, e.x / SUB, e.y / SUB)]);
                f += 1;
            }
            let out = s.outcome().expect("mission over");
            assert!(out.success, "{loadout:?} wins");
            // Several enemies can die on the winning tick.
            assert!((AUTOPLAY_KILL_TARGET..KILL_TARGET_MAX).contains(&out.kills));
            assert!(s.enemies.is_empty());
        }
        // Same seed and inputs, different target: a different state hash.
        let a = SimState::with_loadouts(3, &[KITE]);
        let b = a.clone().with_kill_target(10);
        assert_ne!(a, b);
    }

    #[test]
    fn kill_target_is_rolled_from_the_seed_and_capped_at_100() {
        let mut seen = [false; KILL_TARGET_MAX as usize + 1];
        for seed in 0..5000 {
            let t = SimState::with_loadouts(seed, &[KITE]).mission.kill_target;
            assert!((KILL_TARGET_MIN..=KILL_TARGET_MAX).contains(&t), "{t}");
            assert_eq!(t, rolled_kill_target(seed));
            assert_eq!(
                t,
                SimState::with_loadouts(seed, &[KITE]).mission.kill_target
            );
            seen[t as usize] = true;
        }
        // The whole range comes up, both ends included.
        assert!(seen[1..].iter().all(|&s| s));
        // Rolling the target doesn't shift the mission's own RNG stream.
        let s = SimState::with_loadouts(3, &[KITE]);
        let mut rng = SimRng::new(3);
        assert_eq!(s.fissures, place_fissures(&mut rng));
        // Whatever sets it at launch is capped.
        let s = SimState::with_loadouts(3, &[KITE]).with_kill_target(250);
        assert_eq!(s.mission.kill_target, KILL_TARGET_MAX);
    }

    #[test]
    fn short_missions_end_at_one_and_ten_kills_with_roles_valid() {
        for target in [1, 10] {
            for seed in [5, 11, 23] {
                let mut s = SimState::with_loadouts(seed, &mixed(2)).with_kill_target(target);
                let mut f = 0;
                while s.outcome().is_none() && f < 60 * 300 {
                    for sh in &mut s.ships {
                        sh.hull = sh.hull.max(1000);
                    }
                    s.step(&scripted_inputs(f, 2));
                    f += 1;
                }
                let out = s.outcome().expect("mission over");
                assert!(out.success, "target {target} seed {seed}");
                assert!(out.kills >= target);
                // Only roles whose share of the target was reached came.
                for (i, (_, pct)) in ROLES.iter().enumerate() {
                    if s.mission.introduced & (1 << i) != 0 {
                        assert!(out.kills * 100 >= pct * target);
                    }
                }
            }
        }
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

            // Win: a hull too thick to break, guns and skills going, at
            // the full kill target.
            let mut s = SimState::with_loadouts(11, &[loadout]).with_kill_target(KILL_TARGET_MAX);
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
            assert!(out.kills >= KILL_TARGET_MAX && out.collected.credits > 0);
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
            id: 0,
            kind: LootKind::VoidCrystal,
            amount: 1,
            pos: FxVec2::from_px(70, 0),
            ttl: PICKUP_LIFETIME,
        });
        s.pickups.push(Pickup {
            id: 0,
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
        // Over many Swarmer kills the roll lands near its percentage.
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
        let expected = SWARMER_CRYSTAL_PCT as usize * 10;
        assert!(
            (expected / 2..=expected * 3 / 2).contains(&crystals),
            "{crystals}"
        );
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

    #[test]
    fn entity_ids_are_unique_and_stable() {
        let mut s = SimState::with_loadouts(7, &mixed(4));
        let mut seen: std::collections::BTreeMap<u32, &'static str> = Default::default();
        for f in 0..3000 {
            s.step(&scripted_inputs(f, 4));
            let ids = s
                .enemies
                .iter()
                .map(|e| (e.id, "enemy"))
                .chain(s.projectiles.iter().map(|p| (p.id, "shot")))
                .chain(s.enemy_projectiles.iter().map(|p| (p.id, "spit")))
                .chain(s.pickups.iter().map(|p| (p.id, "pickup")));
            for (id, what) in ids {
                assert_ne!(id, 0, "{what} without an ID");
                // An ID never moves to another entity kind.
                assert_eq!(*seen.entry(id).or_insert(what), what, "id {id}");
            }
            let mut live: Vec<u32> = s.enemies.iter().map(|e| e.id).collect();
            live.extend(s.projectiles.iter().map(|p| p.id));
            live.extend(s.pickups.iter().map(|p| p.id));
            let n = live.len();
            live.sort_unstable();
            live.dedup();
            assert_eq!(live.len(), n, "duplicate live IDs on frame {f}");
        }
        assert!(seen.len() > 100, "{}", seen.len());
    }

    #[test]
    fn bolt_hit_reports_the_hit_and_knocks_the_enemy_back() {
        let mut s = quiet(&[KITE]);
        let mut e = enemy(EnemyKind::Spitter, 100, 0);
        e.id = 42;
        e.cooldown = u32::MAX;
        s.enemies.push(e);
        let mut fired = None;
        let mut hit = None;
        for _ in 0..20 {
            let before = s.enemies[0].pos;
            s.step(&idle(1));
            for ev in &s.events {
                match *ev {
                    SimEvent::Fired { ship: 0, shot } if fired.is_none() => fired = Some(shot),
                    SimEvent::Hit {
                        enemy,
                        ship,
                        attack,
                        source,
                        amount,
                        ..
                    } if hit.is_none() => {
                        hit = Some((enemy, ship, attack, source, amount));
                        // Pushed away from the ship (+x) past its own step.
                        assert!(s.enemies[0].pos.x > before.x, "no knock-back");
                    }
                    _ => {}
                }
            }
        }
        let shot = fired.expect("the ship fired");
        assert_eq!(
            hit,
            Some((42, 0, Attack::Bolt, shot, KITE_BASIC_DAMAGE)),
            "{:?}",
            s.events
        );
    }

    #[test]
    fn kills_damage_skills_and_the_end_are_reported() {
        // Kill: an enemy at 0 HP is reported once, with its ID and kind.
        let mut s = quiet(&[BULWARK]);
        let mut e = enemy(EnemyKind::Swarmer, 500, 500);
        (e.id, e.hp) = (7, 0);
        s.enemies.push(e);
        s.step(&idle(1));
        assert!(s.events.iter().any(|ev| matches!(
            ev,
            SimEvent::Killed {
                enemy: 7,
                kind: EnemyKind::Swarmer,
                ..
            }
        )));
        s.step(&idle(1));
        assert!(!s
            .events
            .iter()
            .any(|ev| matches!(ev, SimEvent::Killed { .. })));

        // Shockwave: the skill and a hit on every enemy in range.
        let mut e = enemy(EnemyKind::Spitter, 100, 0);
        e.id = 8;
        s.enemies.push(e);
        s.step(&[input(INPUT_SKILL_W, 0, 0)]);
        assert!(s.events.contains(&SimEvent::SkillUsed {
            ship: 0,
            skill: Skill::Shockwave,
            pos: FxVec2::ZERO,
        }));
        assert!(s.events.iter().any(|ev| matches!(
            ev,
            SimEvent::Hit {
                enemy: 8,
                attack: Attack::Shockwave,
                amount: SHOCKWAVE_DAMAGE,
                ..
            }
        )));

        // Ship damage: contact from a Swarmer, all of it to the hull.
        let mut s = quiet(&[BULWARK]);
        let mut e = enemy(EnemyKind::Swarmer, 30, 0);
        e.id = 9;
        s.enemies.push(e);
        s.step(&idle(1));
        assert!(s.events.contains(&SimEvent::ShipDamaged {
            ship: 0,
            source: 9,
            amount: SWARMER_CONTACT_DAMAGE,
            hull_lost: SWARMER_CONTACT_DAMAGE,
            pos: s.ships[0].pos,
        }));

        // Mission end: reported on the tick it ends, then never again.
        let mut s = quiet(&[KITE]).with_kill_target(1);
        let mut e = enemy(EnemyKind::Swarmer, 500, 500);
        e.hp = 0;
        s.enemies.push(e);
        s.step(&idle(1));
        assert!(s.events.contains(&SimEvent::MissionEnded { success: true }));
        s.step(&idle(1));
        assert!(s.events.is_empty());
    }

    #[test]
    fn resimulated_frames_report_the_same_events() {
        // Rollback replays frames; the client dedupes by (frame, key), which
        // only works if a replay reports the same keys.
        let mut s = run(4, 1500);
        let snapshot = s.clone();
        let mut first = Vec::new();
        for f in 1500..1800 {
            s.step(&scripted_inputs(f, 4));
            first.extend(s.events.iter().map(|e| (f, e.key())));
        }
        assert!(first.len() > 20, "{}", first.len());
        let mut s = snapshot;
        let mut again = Vec::new();
        for f in 1500..1800 {
            s.step(&scripted_inputs(f, 4));
            again.extend(s.events.iter().map(|e| (f, e.key())));
        }
        assert_eq!(first, again);
    }

    // ---- New roles (TAKOAI-116) --------------------------------------------

    #[test]
    fn lancer_shows_its_line_then_dashes_along_it_hitting_once() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        let mut e = enemy(EnemyKind::Lancer, 300, 0);
        e.cooldown = 0;
        s.enemies.push(e);
        s.step(&idle(1));
        let l = &s.enemies[0];
        assert_eq!(l.windup, LANCER_WINDUP);
        assert!(l.aim.x < 0 && l.aim.y == 0, "locked toward the ship");
        let at = l.pos;
        for _ in 0..LANCER_WINDUP {
            s.step(&idle(1));
        }
        assert_eq!(s.enemies[0].pos, at, "holds still while winding up");
        assert_eq!(s.enemies[0].dash, LANCER_DASH_TICKS);
        for _ in 0..LANCER_DASH_TICKS {
            s.step(&idle(1));
        }
        let travelled = (s.enemies[0].pos - at).length();
        assert!(
            (travelled - px(LANCER_DASH_DISTANCE) as i64).abs() < px(30) as i64,
            "{travelled}"
        );
        assert_eq!(
            s.ships[0].hull,
            BULWARK_HULL - LANCER_DASH_DAMAGE,
            "hit once"
        );
        assert_eq!(s.enemies[0].cooldown, LANCER_RECOVER);
    }

    #[test]
    fn the_slowest_ship_can_sidestep_a_lancer() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        let mut e = enemy(EnemyKind::Lancer, 300, 0);
        e.cooldown = 0;
        s.enemies.push(e);
        s.step(&idle(1));
        // A quarter-second reaction, then move off the line.
        for _ in 0..ticks(1, 4) {
            s.step(&idle(1));
        }
        for _ in 0..ticks(2, 1) {
            s.step(&[input(INPUT_MOVE, 0, 2000)]);
        }
        assert_eq!(s.ships[0].hull, BULWARK_HULL, "sidestepped");
    }

    #[test]
    fn a_killed_bloater_bursts_a_second_later_and_chains() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        let mut b = enemy(EnemyKind::Bloater, 400, 0);
        b.hp = 0;
        s.enemies.push(b);
        // A Swarmer and a second Bloater in the burst, a Swarmer outside it.
        s.enemies.push(enemy(EnemyKind::Swarmer, 400, 60));
        s.enemies.push(enemy(EnemyKind::Bloater, 460, 0));
        s.enemies.push(enemy(EnemyKind::Swarmer, 400, 300));
        s.step(&idle(1));
        assert_eq!(s.mission.kills, 1);
        assert_eq!(s.blasts.len(), 1);
        assert_eq!(s.blasts[0].fuse, BLOATER_FUSE);
        // Freeze the rest so the geometry holds.
        for e in &mut s.enemies {
            e.cooldown = u32::MAX;
        }
        let mut burst_at = None;
        for t in 1..=BLOATER_FUSE {
            for e in &mut s.enemies {
                e.pos = e.home;
            }
            s.step(&idle(1));
            if s.events.iter().any(|e| matches!(e, SimEvent::Burst { .. })) {
                burst_at = Some(t);
                break;
            }
        }
        assert_eq!(burst_at, Some(BLOATER_FUSE));
        // The Swarmer and the Bloater in range died; the far Swarmer lives.
        assert_eq!(s.mission.kills, 3);
        assert_eq!(s.enemies.len(), 1);
        // The chained Bloater left its own burst.
        assert_eq!(s.blasts.len(), 1);
        assert_eq!(s.ships[0].hull, BULWARK_HULL, "the ship was far away");
    }

    #[test]
    fn a_bloater_that_touches_a_ship_swells_then_bursts() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        s.enemies.push(enemy(EnemyKind::Bloater, 36, 0));
        s.step(&idle(1));
        let b = &s.enemies[0];
        assert_ne!(b.struck, 0);
        assert_eq!(b.windup, BLOATER_FUSE);
        let at = b.pos;
        let mut ticks_to_burst = 0;
        while !s.events.iter().any(|e| matches!(e, SimEvent::Burst { .. })) {
            s.step(&idle(1));
            ticks_to_burst += 1;
            assert!(ticks_to_burst < BLOATER_FUSE + 5);
            if let Some(b) = s.enemies.first() {
                // It stops walking (only separation nudges it).
                assert!(
                    (b.pos - at).length() <= px(1) as i64,
                    "holds while swelling"
                );
            }
        }
        assert!(ticks_to_burst >= BLOATER_FUSE);
        assert_eq!(s.ships[0].hull, BULWARK_HULL - BLOATER_BLAST_DAMAGE);
        assert_eq!(s.mission.kills, 1);
    }

    #[test]
    fn brood_pulses_then_releases_swarmers_by_its_fissure() {
        let mut s = quiet(&[BULWARK]);
        s.ships[0].basic_cooldown = u32::MAX;
        let mut b = enemy(EnemyKind::Brood, 1000, 0);
        b.cooldown = 1;
        s.enemies.push(b);
        s.step(&idle(1));
        assert_eq!(s.enemies[0].windup, BROOD_WINDUP);
        for _ in 0..BROOD_WINDUP {
            assert_eq!(s.enemies.len(), 1);
            s.step(&idle(1));
        }
        assert_eq!(s.enemies.len(), 1 + BROOD_SWARMERS as usize);
        assert_eq!(s.mission.spawned, BROOD_SWARMERS as u32);
        assert_eq!(s.enemies[0].cooldown, BROOD_INTERVAL - BROOD_WINDUP);
        // It hangs back: never more than `BROOD_HOLD` out from home.
        for _ in 0..ticks(10, 1) {
            s.step(&idle(1));
            let b = s
                .enemies
                .iter()
                .find(|e| e.kind == EnemyKind::Brood)
                .unwrap();
            // (Plus a little give for its own Swarmers jostling it.)
            assert!((b.pos - b.home).length() <= px(BROOD_HOLD + BROOD_RADIUS) as i64);
        }
    }

    #[test]
    fn a_warden_halves_damage_to_allies_near_it() {
        let mut s = quiet(&[BULWARK]);
        s.enemies.push(enemy(EnemyKind::Spitter, 100, 0));
        s.enemies.push(enemy(EnemyKind::Warden, 100, 100));
        s.enemies.push(enemy(EnemyKind::Spitter, 100, 500));
        assert_eq!(warden_of(&s.enemies, 0), Some(1));
        assert_eq!(warden_of(&s.enemies, 1), None, "not itself");
        assert_eq!(warden_of(&s.enemies, 2), None, "out of range");
        s.ships[0].w_cooldown = 0;
        s.step(&[input(INPUT_SKILL_W, 0, 0)]);
        let hit = |id: usize| {
            s.events.iter().find_map(|e| match *e {
                SimEvent::Hit { enemy, amount, .. } if enemy == s.enemies[id].id => Some(amount),
                _ => None,
            })
        };
        assert_eq!(hit(0), Some(warded(SHOCKWAVE_DAMAGE)));
        assert_eq!(s.enemies[0].hp, SPITTER_HP - warded(SHOCKWAVE_DAMAGE));
        assert_eq!(warded(4), 2);
        assert_eq!(warded(15), 8);
        // A dead Warden shields nobody.
        s.enemies[1].hp = 0;
        assert_eq!(warden_of(&s.enemies, 0), None);
    }

    #[test]
    fn new_roles_join_the_packs_in_order_as_kills_rise() {
        let mut s = SimState::with_loadouts(5, &[KITE]).with_kill_target(100);
        let mut first_pack: Vec<(EnemyKind, u32)> = Vec::new();
        for _ in 0..ticks(400, 1) {
            s.ships[0].hull = 100_000;
            s.ships[0].basic_cooldown = u32::MAX;
            let live = s.enemies.len();
            s.step(&idle(1));
            for e in &s.enemies[live.min(s.enemies.len())..] {
                if !first_pack.iter().any(|(k, _)| *k == e.kind) {
                    first_pack.push((e.kind, s.mission.kills));
                }
            }
            // Fake progress: a kill every second, and keep the field small.
            if s.frame.is_multiple_of(60) {
                s.mission.kills += 1;
                s.enemies.retain(|e| e.kind == EnemyKind::Brood);
            }
        }
        let order: Vec<_> = first_pack.iter().map(|(k, _)| *k).collect();
        let at = |k| first_pack.iter().find(|(x, _)| *x == k).unwrap().1;
        for (kind, pct) in ROLES {
            assert!(order.contains(&kind), "{kind:?} never came: {order:?}");
            assert!(at(kind) >= pct, "{kind:?} at {} kills", at(kind));
        }
        assert!(at(EnemyKind::Lancer) < at(EnemyKind::Bloater));
        assert!(at(EnemyKind::Bloater) < at(EnemyKind::Warden));
        assert!(at(EnemyKind::Warden) < at(EnemyKind::Brood));
    }

    #[test]
    fn every_role_stays_deterministic_under_rollback() {
        // A short kill target unlocks every role early in a scripted run.
        let go = |frames: u32| {
            let mut s = SimState::with_loadouts(11, &mixed(2)).with_kill_target(40);
            for f in 0..frames {
                for sh in &mut s.ships {
                    sh.hull = sh.hull.max(1000);
                }
                s.step(&scripted_inputs(f, 2));
            }
            s
        };
        let a = go(2400);
        assert_eq!(a.checksum(), go(2400).checksum());
        assert!(
            a.mission.introduced.count_ones() >= 2,
            "{}",
            a.mission.introduced
        );
        let mut s = go(1800);
        let snapshot = s.clone();
        for f in 1800..2000 {
            s.step(&scripted_inputs(f, 2));
        }
        let ahead = s.checksum();
        let mut s = snapshot;
        for f in 1800..2000 {
            s.step(&scripted_inputs(f, 2));
        }
        assert_eq!(s.checksum(), ahead);
    }
}
