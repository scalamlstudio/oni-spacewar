//! Gameplay data ported 1:1 from the LÖVE2D prototype (`src/enemy`,
//! `src/projectile`, `src/world`, `src/class/entity`, `src/class/char.lua`).
//!
//! Values keep the prototype's units: speeds in px/second, lifetimes and
//! effect durations in seconds, sizes in px. [`px_per_tick`] and [`ticks`]
//! convert them once, at use, to the fixed 60 Hz timestep.

use serde::{Deserialize, Serialize};

use crate::fixed::{Q16_ONE, SUB};

/// Simulation ticks per second (bevy_ggrs `RollbackFrameRate`).
pub const TICKS_PER_SEC: i32 = 60;

/// px/second -> sub-pixels/tick.
pub const fn px_per_tick(px_per_sec: i32) -> i32 {
    px_per_sec * SUB / TICKS_PER_SEC
}

/// Seconds -> ticks.
pub const fn ticks(seconds: i32) -> u32 {
    (seconds * TICKS_PER_SEC) as u32
}

/// "Effectively immortal" hp the prototype uses (`1.427e8`).
pub const IMMORTAL_HP: i32 = 142_700_000;

/// Battleship stats (`Char:new`: hp 100, s 400, r 10, m 5).
pub const SHIP_HP: i32 = 100;
pub const SHIP_SPEED: i32 = 400;
pub const SHIP_RADIUS: i32 = 10;
pub const SHIP_MASS: i32 = 5;

/// Mass of everything else (`Config` default `m = 1`).
pub const DEFAULT_MASS: i32 = 1;

/// A stat an [`Effect`] can change (the prototype's `tar`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Stat {
    Hp,
    Speed,
}

/// The prototype's `tmat` effect: `stat = stat * mul + add`. With
/// `ticks == 0` it applies once on hit; otherwise it is re-applied every tick
/// for `ticks` ticks (only battleships keep timed effects, as in `agent.lua`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub stat: Stat,
    /// Multiplier in Q16.
    pub mul_q16: i32,
    /// Added after multiplying; hp points, or px/second for `Speed`.
    pub add: i32,
    pub ticks: u32,
}

impl Effect {
    /// Apply to a value of `stat`'s kind (hp, or speed in sub-px/tick).
    pub fn apply(&self, value: i32) -> i32 {
        let add = match self.stat {
            Stat::Hp => self.add,
            Stat::Speed => px_per_tick(self.add),
        };
        crate::fixed::mul_q16(value, self.mul_q16) + add
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ProjectileKind {
    P1,
    P2,
}

/// Placeholder shape a projectile is drawn with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProjectileShape {
    Circle,
    Triangle,
}

pub struct ProjectileDef {
    pub shape: ProjectileShape,
    pub radius: i32,
    pub speed: i32,
    /// Lifetime in seconds (`Config` default `t = 1`).
    pub lifetime: i32,
    pub effects: &'static [Effect],
}

const DMG_1: Effect = Effect {
    stat: Stat::Hp,
    mul_q16: Q16_ONE,
    add: -1,
    ticks: 0,
};

/// `src/projectile/p1.lua` — the battleship's shot.
const P1: ProjectileDef = ProjectileDef {
    shape: ProjectileShape::Circle,
    radius: 3,
    speed: 600,
    lifetime: 1,
    effects: &[
        DMG_1,
        Effect {
            stat: Stat::Speed,
            mul_q16: 6554, // 0.1
            add: 0,
            ticks: ticks(5),
        },
    ],
};

/// `src/projectile/p2.lua` — the ranged enemy's shot.
const P2: ProjectileDef = ProjectileDef {
    shape: ProjectileShape::Triangle,
    radius: 3,
    speed: 200,
    lifetime: 1,
    effects: &[
        DMG_1,
        Effect {
            stat: Stat::Speed,
            mul_q16: Q16_ONE / 2,
            add: 0,
            ticks: ticks(5),
        },
    ],
};

impl ProjectileKind {
    pub fn def(self) -> &'static ProjectileDef {
        match self {
            Self::P1 => &P1,
            Self::P2 => &P2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum EnemyKind {
    E1,
    E2,
}

pub struct EnemyDef {
    pub speed: i32,
    pub radius: i32,
    pub hp: i32,
    /// `etype = "range"`: fires `projectile` at its target with 2% chance a tick.
    pub ranged: Option<ProjectileKind>,
}

impl EnemyKind {
    pub fn def(self) -> &'static EnemyDef {
        match self {
            // src/enemy/e1.lua
            Self::E1 => &EnemyDef {
                speed: 100,
                radius: 8,
                hp: 10,
                ranged: None,
            },
            // src/enemy/e2.lua
            Self::E2 => &EnemyDef {
                speed: 100,
                radius: 12,
                hp: 20,
                ranged: Some(ProjectileKind::P2),
            },
        }
    }
}

/// Chance per tick that a ranged enemy fires, as 1 in N (`math.random() > 0.98`).
pub const RANGED_FIRE_ONE_IN: u32 = 50;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ObstacleKind {
    O1,
    O2,
}

pub struct ObstacleDef {
    /// Side length of the square, px.
    pub size: i32,
    pub hp: i32,
}

impl ObstacleKind {
    pub fn def(self) -> &'static ObstacleDef {
        // src/class/entity/obstacle.lua
        match self {
            Self::O1 => &ObstacleDef {
                size: 400,
                hp: IMMORTAL_HP,
            },
            Self::O2 => &ObstacleDef { size: 200, hp: 10 },
        }
    }
}

/// `src/class/entity/portal.lua`: radius 50, spawned within ±200 px of the origin.
pub const PORTAL_RADIUS: i32 = 50;
pub const PORTAL_SPAWN_RANGE: i32 = 200;

/// A mission level (`src/world/w*.lua`): how many of each thing to try to spawn.
pub struct LevelDef {
    pub obstacles: &'static [(ObstacleKind, u32)],
    pub enemies: &'static [(EnemyKind, u32)],
}

/// Levels are numbered from 1 like the prototype's `worldmap`.
pub const LEVELS: [LevelDef; 2] = [
    // w1
    LevelDef {
        obstacles: &[(ObstacleKind::O1, 10), (ObstacleKind::O2, 100)],
        enemies: &[(EnemyKind::E1, 10), (EnemyKind::E2, 5)],
    },
    // w2
    LevelDef {
        obstacles: &[(ObstacleKind::O2, 300)],
        enemies: &[(EnemyKind::E2, 40)],
    },
];

/// The prototype boots into `World:new({id=2})`.
pub const START_LEVEL: u8 = 2;

/// Spawn attempts land uniformly in ±`SPAWN_RANGE` px; an attempt closer to the
/// origin than the keep-out radius is skipped (not retried), as in `world.lua`.
pub const SPAWN_RANGE: i32 = 1000;
/// Squared keep-out radius in px² for obstacles / enemies.
pub const OBSTACLE_KEEP_OUT_SQ: i64 = 100_000;
pub const ENEMY_KEEP_OUT_SQ: i64 = 1_000;

/// Horizontal gap between battleship spawn points, px. The prototype has one
/// agent at the origin; N ships spread out along x around it.
pub const SHIP_SPAWN_GAP: i32 = 60;
