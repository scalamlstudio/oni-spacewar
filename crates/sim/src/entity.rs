//! Plain-data entities. Each kind lives in its own `Vec` inside
//! [`SimState`](crate::SimState), so creation order is iteration order and is
//! part of the rolled-back, checksummed state. Positions are sub-pixels.

use serde::{Deserialize, Serialize};

use crate::fixed::FxVec2;
use crate::tuning::*;

/// Which battleship a player flies (Demo Spec § Battleships).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum ShipKind {
    /// Fast, fragile, short cooldowns. Q Afterburn, W Scatter.
    #[default]
    Kite,
    /// Slow, tanky, stronger basic attack. Q Bastion, W Shockwave.
    Bulwark,
}

/// Workshop upgrade levels (0..=`MAX_UPGRADE_LEVEL`), applied to every ship.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Upgrades {
    pub hull: u8,
    pub weapon: u8,
    pub thruster: u8,
}

/// What one player brings into a mission. Must be identical on every peer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Loadout {
    pub ship: ShipKind,
    pub upgrades: Upgrades,
}

/// A ship's numbers after upgrades, in sim units.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ShipStats {
    pub max_hull: i32,
    /// Sub-pixels per tick.
    pub speed: i32,
    /// Pixels.
    pub radius: i32,
    pub basic_damage: i32,
    pub basic_interval: u32,
    /// Pixels.
    pub basic_range: i32,
    /// Weapon Tuning level, applied to skill damage.
    pub weapon_level: u8,
}

impl ShipStats {
    pub fn new(loadout: Loadout) -> Self {
        let up = loadout.upgrades;
        let (hull, speed, radius, dmg, interval, range) = match loadout.ship {
            ShipKind::Kite => (
                KITE_HULL,
                KITE_SPEED,
                KITE_RADIUS,
                KITE_BASIC_DAMAGE,
                KITE_BASIC_INTERVAL,
                KITE_BASIC_RANGE,
            ),
            ShipKind::Bulwark => (
                BULWARK_HULL,
                BULWARK_SPEED,
                BULWARK_RADIUS,
                BULWARK_BASIC_DAMAGE,
                BULWARK_BASIC_INTERVAL,
                BULWARK_BASIC_RANGE,
            ),
        };
        Self {
            max_hull: upgraded(hull, HULL_PCT_PER_LEVEL, up.hull),
            speed: px_per_tick(upgraded(speed, THRUSTER_PCT_PER_LEVEL, up.thruster)),
            radius,
            basic_damage: upgraded(dmg, WEAPON_PCT_PER_LEVEL, up.weapon),
            basic_interval: interval,
            basic_range: range,
            weapon_level: up.weapon,
        }
    }

    /// Skill damage with Weapon Tuning applied.
    pub fn skill_damage(&self, base: i32) -> i32 {
        upgraded(base, WEAPON_PCT_PER_LEVEL, self.weapon_level)
    }
}

/// One player's battleship. `handle` is the GGRS player handle and equals the
/// ship's index in `SimState::ships`. Destroyed ships stay in the `Vec` (so
/// indices keep matching handles) with `hull <= 0`.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Ship {
    pub handle: usize,
    pub kind: ShipKind,
    pub stats: ShipStats,
    pub pos: FxVec2,
    /// Click-to-move destination; the ship stops when it gets there.
    pub target: FxVec2,
    pub hull: i32,
    /// Ticks until the basic attack can fire again.
    pub basic_cooldown: u32,
    /// Ticks until Q / W can be used again.
    pub q_cooldown: u32,
    pub w_cooldown: u32,
    /// Bastion: damage the shield still absorbs, and ticks it has left.
    pub shield: i32,
    pub shield_ticks: u32,
    /// Afterburn: per-tick dash velocity and ticks left.
    pub dash_vel: FxVec2,
    pub dash_ticks: u32,
}

impl Ship {
    pub fn new(handle: usize, loadout: Loadout, pos: FxVec2) -> Self {
        let stats = ShipStats::new(loadout);
        Self {
            handle,
            kind: loadout.ship,
            stats,
            pos,
            target: pos,
            hull: stats.max_hull,
            basic_cooldown: 0,
            q_cooldown: 0,
            w_cooldown: 0,
            shield: 0,
            shield_ticks: 0,
            dash_vel: FxVec2::ZERO,
            dash_ticks: 0,
        }
    }

    pub fn alive(&self) -> bool {
        self.hull > 0
    }

    /// Full cooldown of Q / W in ticks (for HUDs).
    pub fn q_cooldown_max(&self) -> u32 {
        match self.kind {
            ShipKind::Kite => AFTERBURN_COOLDOWN,
            ShipKind::Bulwark => BASTION_COOLDOWN,
        }
    }

    pub fn w_cooldown_max(&self) -> u32 {
        match self.kind {
            ShipKind::Kite => SCATTER_COOLDOWN,
            ShipKind::Bulwark => SHOCKWAVE_COOLDOWN,
        }
    }

    /// Apply incoming damage: the shield absorbs first, then the hull.
    pub fn take_damage(&mut self, amount: i32) {
        let absorbed = amount.min(self.shield);
        self.shield -= absorbed;
        self.hull -= amount - absorbed;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum EnemyKind {
    /// Rushes the nearest battleship, contact damage.
    Swarmer,
    /// Keeps its distance and fires slow projectiles.
    Spitter,
}

impl EnemyKind {
    pub const fn hp(self) -> i32 {
        match self {
            Self::Swarmer => SWARMER_HP,
            Self::Spitter => SPITTER_HP,
        }
    }

    /// Pixels.
    pub const fn radius(self) -> i32 {
        match self {
            Self::Swarmer => SWARMER_RADIUS,
            Self::Spitter => SPITTER_RADIUS,
        }
    }

    /// Sub-pixels per tick.
    pub const fn speed(self) -> i32 {
        match self {
            Self::Swarmer => px_per_tick(SWARMER_SPEED),
            Self::Spitter => px_per_tick(SPITTER_SPEED),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub pos: FxVec2,
    pub hp: i32,
    /// Ticks until it can attack again (contact or shot).
    pub cooldown: u32,
}

impl Enemy {
    pub fn new(kind: EnemyKind, pos: FxVec2) -> Self {
        Self {
            kind,
            pos,
            hp: kind.hp(),
            cooldown: match kind {
                EnemyKind::Swarmer => 0,
                EnemyKind::Spitter => SPITTER_FIRST_SHOT,
            },
        }
    }
}

/// A shot. Ship shots live in `SimState::projectiles` and hit enemies;
/// enemy shots live in `SimState::enemy_projectiles` and hit ships.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Projectile {
    /// Handle of the ship that fired it (enemy shots: 0, unused).
    pub owner: usize,
    pub pos: FxVec2,
    /// Sub-pixels per tick.
    pub vel: FxVec2,
    /// Ticks left before it expires.
    pub ttl: u32,
    pub damage: i32,
    /// Pixels.
    pub radius: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LootKind {
    Credits,
    VoidCrystal,
}

/// A loot drop on the field, collected by flying over it.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Pickup {
    pub kind: LootKind,
    pub amount: u32,
    pub pos: FxVec2,
    /// Ticks left before it disappears.
    pub ttl: u32,
}

/// Loot totals.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Loot {
    pub credits: u32,
    pub void_crystal: u32,
}

impl Loot {
    pub fn add(&mut self, kind: LootKind, amount: u32) {
        match kind {
            LootKind::Credits => self.credits += amount,
            LootKind::VoidCrystal => self.void_crystal += amount,
        }
    }
}
