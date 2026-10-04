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

/// A ship's numbers after upgrades in human units (px, px/s, ticks), the
/// single source for both [`ShipStats`] and UIs such as the Dock's stat
/// preview, so what the player reads is what the battle uses.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ShipSheet {
    pub hull: i32,
    /// px/s.
    pub speed: i32,
    pub radius: i32,
    pub basic_damage: i32,
    /// Ticks between basic attacks.
    pub basic_interval: u32,
    pub basic_range: i32,
    /// W damage after Weapon Tuning: Kite Scatter per bolt, Bulwark
    /// Shockwave. (Neither Q deals damage.)
    pub w_damage: i32,
    pub q_cooldown: u32,
    pub w_cooldown: u32,
}

impl ShipSheet {
    pub fn new(loadout: Loadout) -> Self {
        let up = loadout.upgrades;
        let weapon = |base| upgraded(base, WEAPON_PCT_PER_LEVEL, up.weapon);
        let (hull, speed, radius, dmg, interval, range, w_dmg, q_cd, w_cd) = match loadout.ship {
            ShipKind::Kite => (
                KITE_HULL,
                KITE_SPEED,
                KITE_RADIUS,
                KITE_BASIC_DAMAGE,
                KITE_BASIC_INTERVAL,
                KITE_BASIC_RANGE,
                SCATTER_DAMAGE,
                AFTERBURN_COOLDOWN,
                SCATTER_COOLDOWN,
            ),
            ShipKind::Bulwark => (
                BULWARK_HULL,
                BULWARK_SPEED,
                BULWARK_RADIUS,
                BULWARK_BASIC_DAMAGE,
                BULWARK_BASIC_INTERVAL,
                BULWARK_BASIC_RANGE,
                SHOCKWAVE_DAMAGE,
                BASTION_COOLDOWN,
                SHOCKWAVE_COOLDOWN,
            ),
        };
        Self {
            hull: upgraded(hull, HULL_PCT_PER_LEVEL, up.hull),
            speed: upgraded(speed, THRUSTER_PCT_PER_LEVEL, up.thruster),
            radius,
            basic_damage: weapon(dmg),
            basic_interval: interval,
            basic_range: range,
            w_damage: weapon(w_dmg),
            q_cooldown: q_cd,
            w_cooldown: w_cd,
        }
    }
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
        let sheet = ShipSheet::new(loadout);
        Self {
            max_hull: sheet.hull,
            speed: px_per_tick(sheet.speed),
            radius: sheet.radius,
            basic_damage: sheet.basic_damage,
            basic_interval: sheet.basic_interval,
            basic_range: sheet.basic_range,
            weapon_level: loadout.upgrades.weapon,
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
    /// `NetInput::buttons` from the previous tick. Skills fire on the tick
    /// their button goes down (edge), not while it is held.
    pub prev_buttons: u8,
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
            prev_buttons: 0,
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
    /// Spitter strafing direction around its target: 1 counter-clockwise,
    /// -1 clockwise. Flips when it runs into the arena edge.
    pub orbit: i32,
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
            orbit: 1,
        }
    }
}

/// A void fissure: enemies come out of it. Fixed for the whole mission and
/// can't be hit or destroyed.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Fissure {
    pub pos: FxVec2,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(ship: ShipKind, hull: u8, weapon: u8, thruster: u8) -> ShipSheet {
        ShipSheet::new(Loadout {
            ship,
            upgrades: Upgrades {
                hull,
                weapon,
                thruster,
            },
        })
    }

    #[test]
    fn sheet_follows_the_upgrade_table() {
        // Demo Spec § Workshop upgrades, both levels of every upgrade.
        let hull: Vec<_> = (0..=2)
            .map(|l| sheet(ShipKind::Kite, l, 0, 0).hull)
            .collect();
        assert_eq!(hull, [60, 75, 90]);
        let hull: Vec<_> = (0..=2)
            .map(|l| sheet(ShipKind::Bulwark, l, 0, 0).hull)
            .collect();
        assert_eq!(hull, [140, 175, 210]);
        let dmg: Vec<_> = (0..=2)
            .map(|l| sheet(ShipKind::Kite, 0, l, 0).basic_damage)
            .collect();
        assert_eq!(dmg, [4, 5, 6]);
        let dmg: Vec<_> = (0..=2)
            .map(|l| sheet(ShipKind::Bulwark, 0, l, 0).basic_damage)
            .collect();
        assert_eq!(dmg, [8, 10, 12]);
        let speed: Vec<_> = (0..=2)
            .map(|l| sheet(ShipKind::Kite, 0, 0, l).speed)
            .collect();
        assert_eq!(speed, [220, 253, 286]);
        let speed: Vec<_> = (0..=2)
            .map(|l| sheet(ShipKind::Bulwark, 0, 0, l).speed)
            .collect();
        assert_eq!(speed, [140, 161, 182]);
    }

    #[test]
    fn stats_are_the_sheet_in_sim_units() {
        let loadout = Loadout {
            ship: ShipKind::Bulwark,
            upgrades: Upgrades {
                hull: 1,
                weapon: 2,
                thruster: 1,
            },
        };
        let (sheet, stats) = (ShipSheet::new(loadout), ShipStats::new(loadout));
        assert_eq!(stats.max_hull, sheet.hull);
        assert_eq!(stats.speed, px_per_tick(sheet.speed));
        assert_eq!(stats.basic_damage, sheet.basic_damage);
        assert_eq!(stats.skill_damage(SHOCKWAVE_DAMAGE), sheet.w_damage);
    }
}
