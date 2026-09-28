//! The whole rolled-back simulation state and its fixed-timestep step.
//!
//! A 1:1 port of the LÖVE2D mission prototype in `src/` (see `content.rs` for
//! the data), with two deliberate changes:
//! - Battleship movement is click-to-move (`design/DESIGN.md` § Player): the
//!   primary button sets a target the ship keeps flying to, instead of the
//!   prototype's "move toward the cursor only while held".
//! - Everything is per-player: N battleships, enemies chase the nearest living
//!   one, any battleship entering the portal moves the whole party on.
//!
//! Tick order mirrors `love.update` in `src/main.lua`: pending level switch,
//! entity updates (input, AI, timers), movement, collisions, then removal of
//! anything dead or expired.

use core::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::collision::{contact, scale_to, Shape};
use crate::content::*;
use crate::fixed::{FxVec2, SUB};
use crate::input::{NetInput, INPUT_MOVE, INPUT_SKILL_Q};
use crate::rng::SimRng;

/// Most players a mission can hold (design: up to 4 per mission).
pub const MAX_PLAYERS: usize = 4;

/// Match setup that must be identical on every peer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SimParams {
    pub num_players: usize,
    pub seed: u64,
}

/// Which side a body is on. Bodies on the same side never collide (the
/// prototype's Box2D category/mask `alias`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Side {
    Player,
    Enemy,
    Neutral,
}

/// One player's battleship (`src/class/entity/agent.lua`). `handle` is the
/// GGRS player handle. Dead ships stay in the list so indices keep matching
/// handles.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Ship {
    pub handle: usize,
    pub alive: bool,
    pub pos: FxVec2,
    /// Click-to-move destination.
    pub target: FxVec2,
    pub hp: i32,
    /// Current speed after effects, sub-px/tick.
    pub speed: i32,
    /// Timed effects still running (e.g. the p2 slow).
    pub effects: Vec<Effect>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub pos: FxVec2,
    pub vel: FxVec2,
    pub hp: i32,
    /// sub-px/tick
    pub speed: i32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Projectile {
    pub kind: ProjectileKind,
    pub side: Side,
    pub pos: FxVec2,
    pub vel: FxVec2,
    pub hp: i32,
    /// Ticks left before it expires.
    pub ttl: u32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Obstacle {
    pub kind: ObstacleKind,
    pub pos: FxVec2,
    pub hp: i32,
}

/// Touching it with a battleship moves the party to a random level.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Portal {
    pub pos: FxVec2,
}

/// Pick-up (`src/class/entity/item.lua`). Ported for completeness; the
/// prototype defines map items but never spawns or uses them.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Item {
    pub pos: FxVec2,
    pub map_id: u8,
}

/// Everything that affects gameplay. Saved/loaded wholesale on rollback and
/// hashed for desync detection, so any new gameplay field goes in here.
/// Every collection is a `Vec` so its order is part of the state.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub params: SimParams,
    /// Number of ticks simulated so far.
    pub frame: u32,
    pub rng: SimRng,
    /// Current level, numbered from 1 (index into `LEVELS` + 1).
    pub level: u8,
    /// Level to switch to at the start of the next tick (set by the portal).
    pub pending_level: Option<u8>,
    /// One ship per player, indexed by handle (stable order).
    pub ships: Vec<Ship>,
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub obstacles: Vec<Obstacle>,
    pub portals: Vec<Portal>,
    pub items: Vec<Item>,
}

/// A reference to one body, for the collision pass.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Body {
    Ship(usize),
    Enemy(usize),
    Projectile(usize),
    Obstacle(usize),
    Portal(usize),
    Item(usize),
}

fn ship_spawn(handle: usize, n: usize) -> FxVec2 {
    let gap = SHIP_SPAWN_GAP * SUB;
    FxVec2::new((2 * handle as i32 - (n as i32 - 1)) * gap / 2, 0)
}

fn px(v: i32) -> i32 {
    v * SUB
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
                    alive: true,
                    pos,
                    target: pos,
                    hp: SHIP_HP,
                    speed: px_per_tick(SHIP_SPEED),
                    effects: Vec::new(),
                }
            })
            .collect();
        let mut s = Self {
            params,
            frame: 0,
            rng: SimRng::new(params.seed),
            level: START_LEVEL,
            pending_level: None,
            ships,
            enemies: Vec::new(),
            projectiles: Vec::new(),
            obstacles: Vec::new(),
            portals: Vec::new(),
            items: Vec::new(),
        };
        s.load_level(START_LEVEL);
        s
    }

    /// Clear the level and spawn a new one (`world:clear` + `world:initLevel`
    /// + `agent:resetPosition`). Ship hp, effects and deaths carry over.
    fn load_level(&mut self, level: u8) {
        self.level = level;
        self.enemies.clear();
        self.projectiles.clear();
        self.obstacles.clear();
        self.portals.clear();
        self.items.clear();
        let def = &LEVELS[level as usize - 1];

        let r = PORTAL_SPAWN_RANGE;
        let (x, y) = (self.rng.range(-r, r), self.rng.range(-r, r));
        self.portals.push(Portal {
            pos: FxVec2::from_px(x, y),
        });

        for &(kind, count) in def.obstacles {
            for _ in 0..count {
                if let Some(pos) = self.spawn_point(OBSTACLE_KEEP_OUT_SQ) {
                    self.obstacles.push(Obstacle {
                        kind,
                        pos,
                        hp: kind.def().hp,
                    });
                }
            }
        }
        for &(kind, count) in def.enemies {
            for _ in 0..count {
                if let Some(pos) = self.spawn_point(ENEMY_KEEP_OUT_SQ) {
                    let d = kind.def();
                    self.enemies.push(Enemy {
                        kind,
                        pos,
                        vel: FxVec2::ZERO,
                        hp: d.hp,
                        speed: px_per_tick(d.speed),
                    });
                }
            }
        }

        let n = self.ships.len();
        for ship in &mut self.ships {
            ship.pos = ship_spawn(ship.handle, n);
            ship.target = ship.pos;
        }
    }

    /// One spawn attempt; `None` if it lands inside the keep-out radius.
    fn spawn_point(&mut self, keep_out_sq: i64) -> Option<FxVec2> {
        let x = self.rng.range(-SPAWN_RANGE, SPAWN_RANGE);
        let y = self.rng.range(-SPAWN_RANGE, SPAWN_RANGE);
        let d2 = x as i64 * x as i64 + y as i64 * y as i64;
        (d2 > keep_out_sq).then(|| FxVec2::from_px(x, y))
    }

    /// Advance one fixed tick. `inputs[h]` is player `h`'s input.
    pub fn step(&mut self, inputs: &[NetInput]) {
        assert_eq!(inputs.len(), self.ships.len(), "one input per player");
        if let Some(level) = self.pending_level.take() {
            self.load_level(level);
        }
        self.update_ships(inputs);
        self.update_enemies();
        self.integrate();
        self.collide();
        self.cull();
        self.frame += 1;
    }

    fn fire(&mut self, kind: ProjectileKind, side: Side, from: FxVec2, toward: FxVec2) {
        let def = kind.def();
        let vel = scale_to(toward - from, px_per_tick(def.speed));
        if vel == FxVec2::ZERO {
            return;
        }
        self.projectiles.push(Projectile {
            kind,
            side,
            pos: from,
            vel,
            hp: 1,
            ttl: ticks(def.lifetime),
        });
    }

    fn update_ships(&mut self, inputs: &[NetInput]) {
        for h in 0..self.ships.len() {
            let ship = &mut self.ships[h];
            if !ship.alive {
                continue;
            }
            // Speed is recomputed from base every tick, then timed effects apply.
            ship.speed = px_per_tick(SHIP_SPEED);
            let mut speed = ship.speed;
            ship.effects.retain_mut(|e| {
                e.ticks -= 1;
                if e.ticks > 0 && e.stat == Stat::Speed {
                    speed = e.apply(speed);
                }
                e.ticks > 0
            });
            ship.speed = speed.max(0);

            let input = inputs[ship.handle];
            let aim = FxVec2::from_px(input.target_x, input.target_y);
            if input.pressed(INPUT_MOVE) {
                ship.target = aim;
            }
            if input.pressed(INPUT_SKILL_Q) {
                let from = ship.pos;
                self.fire(ProjectileKind::P1, Side::Player, from, aim);
            }
        }
    }

    /// Nearest living ship to `pos` (lowest handle on ties).
    fn nearest_ship(&self, pos: FxVec2) -> Option<FxVec2> {
        self.ships
            .iter()
            .filter(|s| s.alive)
            .min_by_key(|s| ((s.pos - pos).length_squared(), s.handle))
            .map(|s| s.pos)
    }

    fn update_enemies(&mut self) {
        for i in 0..self.enemies.len() {
            let e = &self.enemies[i];
            let Some(target) = self.nearest_ship(e.pos) else {
                self.enemies[i].vel = FxVec2::ZERO;
                continue;
            };
            let vel = scale_to(target - e.pos, e.speed);
            let (pos, ranged) = (e.pos, e.kind.def().ranged);
            self.enemies[i].vel = vel;
            if let Some(kind) = ranged {
                if self.rng.next_u32().is_multiple_of(RANGED_FIRE_ONE_IN) {
                    self.fire(kind, Side::Enemy, pos, target);
                }
            }
        }
    }

    fn integrate(&mut self) {
        for ship in self.ships.iter_mut().filter(|s| s.alive) {
            let d = ship.target - ship.pos;
            if d.length() <= ship.speed as i64 {
                ship.pos = ship.target;
            } else {
                ship.pos = ship.pos + scale_to(d, ship.speed);
            }
        }
        for e in &mut self.enemies {
            e.pos = e.pos + e.vel;
        }
        for p in &mut self.projectiles {
            p.pos = p.pos + p.vel;
            p.ttl = p.ttl.saturating_sub(1);
        }
    }

    fn body_info(&self, b: Body) -> (FxVec2, Shape, Side, bool) {
        let circle = |r: i32| Shape::Circle { r: px(r) };
        match b {
            Body::Ship(i) => (self.ships[i].pos, circle(SHIP_RADIUS), Side::Player, false),
            Body::Enemy(i) => {
                let e = &self.enemies[i];
                (e.pos, circle(e.kind.def().radius), Side::Enemy, false)
            }
            Body::Projectile(i) => {
                let p = &self.projectiles[i];
                (p.pos, circle(p.kind.def().radius), p.side, false)
            }
            Body::Obstacle(i) => {
                let o = &self.obstacles[i];
                let half = Shape::Square {
                    half: px(o.kind.def().size) / 2,
                };
                (o.pos, half, Side::Neutral, true)
            }
            Body::Portal(i) => (
                self.portals[i].pos,
                circle(PORTAL_RADIUS),
                Side::Neutral,
                true,
            ),
            // Items are dynamic in the prototype but never move; static here.
            Body::Item(i) => (
                self.items[i].pos,
                circle(PORTAL_RADIUS / 2),
                Side::Neutral,
                true,
            ),
        }
    }

    fn set_pos(&mut self, b: Body, pos: FxVec2) {
        match b {
            Body::Ship(i) => self.ships[i].pos = pos,
            Body::Enemy(i) => self.enemies[i].pos = pos,
            Body::Projectile(i) => self.projectiles[i].pos = pos,
            Body::Obstacle(_) | Body::Portal(_) | Body::Item(_) => {}
        }
    }

    fn mass(b: Body) -> i32 {
        match b {
            Body::Ship(_) => SHIP_MASS,
            _ => DEFAULT_MASS,
        }
    }

    /// Broad phase (sort and sweep on x) + narrow phase + contact handling.
    /// Pair order comes from a total sort key, so it is identical on every peer.
    fn collide(&mut self) {
        let mut bodies: Vec<Body> = Vec::new();
        bodies.extend(
            self.ships
                .iter()
                .enumerate()
                .filter(|(_, s)| s.alive)
                .map(|(i, _)| Body::Ship(i)),
        );
        bodies.extend((0..self.enemies.len()).map(Body::Enemy));
        bodies.extend((0..self.projectiles.len()).map(Body::Projectile));
        bodies.extend((0..self.obstacles.len()).map(Body::Obstacle));
        bodies.extend((0..self.portals.len()).map(Body::Portal));
        bodies.extend((0..self.items.len()).map(Body::Item));

        // (min_x, max_x, min_y, max_y, body)
        let mut boxes: Vec<(i32, i32, i32, i32, Body)> = bodies
            .into_iter()
            .map(|b| {
                let (p, shape, _, _) = self.body_info(b);
                let e = shape.extent();
                (p.x - e, p.x + e, p.y - e, p.y + e, b)
            })
            .collect();
        boxes.sort_unstable_by_key(|&(min_x, _, _, _, b)| (min_x, b));

        for i in 0..boxes.len() {
            let (_, max_x, min_y, max_y, a) = boxes[i];
            for &(min_x2, _, min_y2, max_y2, b) in &boxes[i + 1..] {
                if min_x2 > max_x {
                    break;
                }
                if min_y2 > max_y || max_y2 < min_y {
                    continue;
                }
                self.pair(a, b);
            }
        }
    }

    fn pair(&mut self, a: Body, b: Body) {
        let (pa, sa, side_a, static_a) = self.body_info(a);
        let (pb, sb, side_b, static_b) = self.body_info(b);
        if side_a == side_b || (static_a && static_b) {
            return;
        }
        let Some(push) = contact(sa, pa, sb, pb) else {
            return;
        };

        // "hit" events: projectiles lose hp and apply their effects; a ship
        // touching a portal queues a level switch.
        let a_proj = matches!(a, Body::Projectile(_));
        let b_proj = matches!(b, Body::Projectile(_));
        if let Body::Projectile(i) = a {
            self.projectile_hit(i, b);
        }
        if let Body::Projectile(i) = b {
            self.projectile_hit(i, a);
        }
        if matches!(
            (a, b),
            (Body::Ship(_), Body::Portal(_)) | (Body::Portal(_), Body::Ship(_))
        ) {
            let n = LEVELS.len() as i32;
            self.pending_level = Some(self.rng.range(1, n) as u8);
        }

        // Physical separation. Projectiles vanish on hit, so they don't push.
        if a_proj || b_proj {
            return;
        }
        if static_b {
            self.set_pos(a, pa + push);
        } else if static_a {
            self.set_pos(b, pb - push);
        } else {
            // Split by mass: the lighter body moves more.
            let (ma, mb) = (Self::mass(a), Self::mass(b));
            let share_a = FxVec2::new(push.x * mb / (ma + mb), push.y * mb / (ma + mb));
            self.set_pos(a, pa + share_a);
            self.set_pos(b, pb - (push - share_a));
        }
    }

    fn projectile_hit(&mut self, i: usize, other: Body) {
        self.projectiles[i].hp -= 1;
        for effect in self.projectiles[i].kind.def().effects {
            self.apply_effect(other, *effect);
        }
    }

    /// The target's `handle(tmat)`: instant effects apply to hp (speed is
    /// recomputed each tick, so an instant speed change has no lasting
    /// effect); only battleships keep timed effects.
    fn apply_effect(&mut self, target: Body, e: Effect) {
        let hp = match target {
            Body::Ship(i) => {
                if e.ticks > 0 {
                    self.ships[i].effects.push(e);
                    return;
                }
                &mut self.ships[i].hp
            }
            Body::Enemy(i) if e.ticks == 0 => {
                if e.stat == Stat::Speed {
                    let en = &mut self.enemies[i];
                    en.speed = e.apply(en.speed);
                    return;
                }
                &mut self.enemies[i].hp
            }
            Body::Obstacle(i) if e.ticks == 0 => &mut self.obstacles[i].hp,
            _ => return,
        };
        if e.stat == Stat::Hp {
            *hp = e.apply(*hp);
        }
    }

    /// Remove whatever died or expired this tick (`world:update`'s sweep).
    fn cull(&mut self) {
        for ship in &mut self.ships {
            if ship.hp <= 0 {
                ship.alive = false;
            }
        }
        self.enemies.retain(|e| e.hp > 0);
        self.projectiles.retain(|p| p.hp > 0 && p.ttl > 0);
        self.obstacles.retain(|o| o.hp > 0);
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
                if (frame / 25 + p as u32).is_multiple_of(3) {
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

    fn idle(n: usize) -> Vec<NetInput> {
        vec![NetInput::default(); n]
    }

    /// A state with ships only, for targeted tests.
    fn empty(players: usize) -> SimState {
        let mut s = SimState::new(SimParams {
            num_players: players,
            seed: 0,
        });
        s.enemies.clear();
        s.obstacles.clear();
        s.portals.clear();
        s
    }

    #[test]
    fn same_inputs_same_state() {
        for players in 1..=MAX_PLAYERS {
            let p = SimParams {
                num_players: players,
                seed: 7,
            };
            assert_eq!(run(p, 3600).checksum(), run(p, 3600).checksum());
        }
    }

    #[test]
    fn restore_and_resimulate_matches() {
        // What rollback does: snapshot, run ahead, restore, re-run.
        let p = SimParams {
            num_players: 4,
            seed: 42,
        };
        let mut s = run(p, 500);
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
        let p = SimParams {
            num_players: 2,
            seed: 3,
        };
        let start = SimState::new(p);
        let mut s = start.clone();
        let mut max_proj = 0;
        for f in 0..1200 {
            s.step(&scripted_inputs(f, 2));
            max_proj = max_proj.max(s.projectiles.len());
        }
        assert!(max_proj > 10, "projectiles fired");
        assert!(
            s.enemies.len() < start.enemies.len()
                || s.level != start.level
                || s.ships.iter().any(|sh| sh.hp < SHIP_HP),
            "something got hit"
        );
        assert_ne!(s.ships[0].pos, start.ships[0].pos);
    }

    #[test]
    fn level_spawns_like_prototype() {
        let s = SimState::new(SimParams {
            num_players: 1,
            seed: 11,
        });
        assert_eq!(s.level, START_LEVEL);
        assert_eq!(s.portals.len(), 1);
        // Keep-out attempts are skipped, not retried.
        assert!(s.obstacles.len() <= 300 && s.obstacles.len() > 250);
        assert!(s.enemies.len() <= 40 && s.enemies.len() > 35);
        assert!(s.enemies.iter().all(|e| e.kind == EnemyKind::E2));
        for o in &s.obstacles {
            let (x, y) = (o.pos.x as i64 / SUB as i64, o.pos.y as i64 / SUB as i64);
            assert!(x * x + y * y > OBSTACLE_KEEP_OUT_SQ);
        }
        assert_eq!(s.ships[0].pos, FxVec2::ZERO);
    }

    #[test]
    fn one_ship_per_player_in_handle_order() {
        let s = SimState::new(SimParams {
            num_players: 3,
            seed: 0,
        });
        let handles: Vec<_> = s.ships.iter().map(|s| s.handle).collect();
        assert_eq!(handles, vec![0, 1, 2]);
        assert_ne!(s.ships[0].pos, s.ships[1].pos);
    }

    #[test]
    fn click_to_move_keeps_going_after_release() {
        let mut s = empty(1);
        let click = NetInput {
            buttons: INPUT_MOVE,
            target_x: 100,
            target_y: 0,
        };
        s.step(&[click]);
        // Released: keeps flying to the target, then stops exactly on it.
        for _ in 0..120 {
            s.step(&idle(1));
        }
        assert_eq!(s.ships[0].pos, FxVec2::from_px(100, 0));
        // 400 px/s = 1706 sub-px/tick, so 100 px takes 16 ticks.
        let mut s = empty(1);
        s.step(&[click]);
        for _ in 0..14 {
            s.step(&idle(1));
        }
        assert_ne!(s.ships[0].pos, FxVec2::from_px(100, 0));
        s.step(&idle(1));
        assert_eq!(s.ships[0].pos, FxVec2::from_px(100, 0));
    }

    #[test]
    fn projectile_damages_enemy_and_is_consumed() {
        let mut s = empty(1);
        s.enemies.push(Enemy {
            kind: EnemyKind::E1,
            pos: FxVec2::from_px(30, 0),
            vel: FxVec2::ZERO,
            hp: 10,
            speed: 0,
        });
        let fire = NetInput {
            buttons: INPUT_SKILL_Q,
            target_x: 30,
            target_y: 0,
        };
        s.step(&[fire]);
        assert_eq!(s.projectiles.len(), 1);
        for _ in 0..5 {
            s.step(&idle(1));
        }
        assert_eq!(s.enemies[0].hp, 9);
        assert!(s.projectiles.is_empty());
    }

    #[test]
    fn enemy_shot_damages_and_slows_ship() {
        let mut s = empty(1);
        s.projectiles.push(Projectile {
            kind: ProjectileKind::P2,
            side: Side::Enemy,
            pos: FxVec2::from_px(5, 0),
            vel: FxVec2::ZERO,
            hp: 1,
            ttl: 60,
        });
        s.step(&idle(1));
        assert_eq!(s.ships[0].hp, SHIP_HP - 1);
        assert_eq!(s.ships[0].effects.len(), 1);
        s.step(&idle(1));
        assert_eq!(s.ships[0].speed, px_per_tick(SHIP_SPEED) / 2);
        for _ in 0..ticks(5) {
            s.step(&idle(1));
        }
        assert!(s.ships[0].effects.is_empty());
        assert_eq!(s.ships[0].speed, px_per_tick(SHIP_SPEED));
    }

    #[test]
    fn own_side_never_collides() {
        let mut s = empty(2);
        s.ships[1].pos = s.ships[0].pos;
        s.ships[1].target = s.ships[0].pos;
        s.projectiles.push(Projectile {
            kind: ProjectileKind::P1,
            side: Side::Player,
            pos: s.ships[0].pos,
            vel: FxVec2::ZERO,
            hp: 1,
            ttl: 60,
        });
        s.step(&idle(2));
        assert_eq!(s.ships[0].pos, s.ships[1].pos);
        assert_eq!(s.projectiles.len(), 1);
        assert_eq!(s.ships[0].hp, SHIP_HP);
    }

    #[test]
    fn obstacles_block_ships() {
        let mut s = empty(1);
        s.obstacles.push(Obstacle {
            kind: ObstacleKind::O2,
            pos: FxVec2::from_px(200, 0),
            hp: 10,
        });
        let go = NetInput {
            buttons: INPUT_MOVE,
            target_x: 400,
            target_y: 0,
        };
        s.step(&[go]);
        for _ in 0..120 {
            s.step(&idle(1));
        }
        // Stopped against the square's left face (x = 100) minus the radius.
        assert_eq!(s.ships[0].pos, FxVec2::from_px(100 - SHIP_RADIUS, 0));
    }

    #[test]
    fn portal_moves_every_ship_to_a_new_level() {
        let mut s = empty(2);
        s.portals.push(Portal {
            pos: FxVec2::from_px(0, 200),
        });
        let go = NetInput {
            buttons: INPUT_MOVE,
            target_x: 0,
            target_y: 200,
        };
        s.step(&[go, NetInput::default()]);
        for _ in 0..60 {
            if s.pending_level.is_some() {
                break;
            }
            s.step(&idle(2));
        }
        let next = s.pending_level.expect("portal touched");
        s.step(&idle(2));
        assert_eq!(s.level, next);
        assert_eq!(s.portals.len(), 1);
        assert!(!s.obstacles.is_empty());
        assert_eq!(s.ships[0].pos, ship_spawn(0, 2));
        assert_eq!(s.ships[1].pos, ship_spawn(1, 2));
    }

    #[test]
    fn dead_ship_stays_in_place_and_is_ignored() {
        let mut s = empty(2);
        s.ships[0].hp = 1;
        s.projectiles.push(Projectile {
            kind: ProjectileKind::P2,
            side: Side::Enemy,
            pos: s.ships[0].pos,
            vel: FxVec2::ZERO,
            hp: 1,
            ttl: 60,
        });
        s.enemies.push(Enemy {
            kind: EnemyKind::E1,
            pos: FxVec2::from_px(-30, -500),
            vel: FxVec2::ZERO,
            hp: 10,
            speed: px_per_tick(100),
        });
        s.step(&idle(2));
        // Chased the nearest ship (0, straight up) until it died this tick.
        assert_eq!(s.enemies[0].vel, FxVec2::new(0, px_per_tick(100)));
        assert!(!s.ships[0].alive);
        assert_eq!(s.ships.len(), 2);
        // Now it chases ship 1, to the right.
        s.step(&idle(2));
        assert!(s.enemies[0].vel.x > 0);
    }

    #[test]
    fn checksum_sees_state_changes() {
        let p = SimParams {
            num_players: 2,
            seed: 1,
        };
        let a = SimState::new(p);
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
