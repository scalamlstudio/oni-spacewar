//! Plain-data entities. Each kind lives in its own `Vec` inside
//! [`SimState`](crate::SimState), so creation order is iteration order and is
//! part of the rolled-back, checksummed state. Positions are sub-pixels.

use serde::{Deserialize, Serialize};

use crate::event::Attack;
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
    /// Training Room (0 or 1): shorter Q / W cooldowns. A carrier room, not
    /// a Workshop purchase, but it reaches the sim the same way.
    #[serde(default)]
    pub training: u8,
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
            q_cooldown: trained(q_cd, up.training),
            w_cooldown: trained(w_cd, up.training),
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
    /// Q / W cooldowns in ticks (Training Room applied).
    pub q_cooldown: u32,
    pub w_cooldown: u32,
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
            q_cooldown: sheet.q_cooldown,
            w_cooldown: sheet.w_cooldown,
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
    /// Ticks left slowed by a Chilling elite's hit (0 = not chilled).
    pub chill: u32,
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
            chill: 0,
            prev_buttons: 0,
        }
    }

    pub fn alive(&self) -> bool {
        self.hull > 0
    }

    /// Full cooldown of Q / W in ticks (for HUDs).
    pub fn q_cooldown_max(&self) -> u32 {
        self.stats.q_cooldown
    }

    pub fn w_cooldown_max(&self) -> u32 {
        self.stats.w_cooldown
    }

    /// Apply incoming damage: the shield absorbs first, then the hull.
    /// Returns the hull lost.
    pub fn take_damage(&mut self, amount: i32) -> i32 {
        let absorbed = amount.min(self.shield);
        self.shield -= absorbed;
        self.hull -= amount - absorbed;
        amount - absorbed
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum EnemyKind {
    /// Rushes the nearest battleship, contact damage.
    Swarmer,
    /// Keeps its distance and fires slow projectiles.
    Spitter,
    /// Charger: stops, shows a charge line for 0.7 s, then dashes along it.
    Lancer,
    /// Death-bomber: bursts 1 s after it dies or touches a ship.
    Bloater,
    /// Summoner: hangs back by its fissure and releases Swarmers.
    Brood,
    /// Buffer: allies near it take half damage.
    Warden,
}

impl EnemyKind {
    /// Every kind, in declaration order.
    pub const ALL: [EnemyKind; 6] = [
        Self::Swarmer,
        Self::Spitter,
        Self::Lancer,
        Self::Bloater,
        Self::Brood,
        Self::Warden,
    ];

    pub const fn hp(self) -> i32 {
        match self {
            Self::Swarmer => SWARMER_HP,
            Self::Spitter => SPITTER_HP,
            Self::Lancer => LANCER_HP,
            Self::Bloater => BLOATER_HP,
            Self::Brood => BROOD_HP,
            Self::Warden => WARDEN_HP,
        }
    }

    /// Pixels.
    pub const fn radius(self) -> i32 {
        match self {
            Self::Swarmer => SWARMER_RADIUS,
            Self::Spitter => SPITTER_RADIUS,
            Self::Lancer => LANCER_RADIUS,
            Self::Bloater => BLOATER_RADIUS,
            Self::Brood => BROOD_RADIUS,
            Self::Warden => WARDEN_RADIUS,
        }
    }

    /// px/s (codex, HUDs).
    pub const fn speed_px(self) -> i32 {
        match self {
            Self::Swarmer => SWARMER_SPEED,
            Self::Spitter => SPITTER_SPEED,
            Self::Lancer => LANCER_SPEED,
            Self::Bloater => BLOATER_SPEED,
            Self::Brood => BROOD_SPEED,
            Self::Warden => WARDEN_SPEED,
        }
    }

    /// Sub-pixels per tick.
    pub const fn speed(self) -> i32 {
        px_per_tick(self.speed_px())
    }

    /// Credits dropped on death, and the Void Crystal chance (percent).
    pub const fn loot(self) -> (u32, i32) {
        match self {
            Self::Swarmer => (SWARMER_CREDITS, SWARMER_CRYSTAL_PCT),
            Self::Spitter => (SPITTER_CREDITS, SPITTER_CRYSTAL_PCT),
            Self::Lancer => (LANCER_CREDITS, LANCER_CRYSTAL_PCT),
            Self::Bloater => (BLOATER_CREDITS, BLOATER_CRYSTAL_PCT),
            Self::Brood => (BROOD_CREDITS, BROOD_CRYSTAL_PCT),
            Self::Warden => (WARDEN_CREDITS, WARDEN_CRYSTAL_PCT),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Enemy {
    /// Stable entity ID (unique within the mission; 0 = not yet assigned).
    pub id: u32,
    pub kind: EnemyKind,
    pub pos: FxVec2,
    pub hp: i32,
    /// Ticks until it can attack again (contact, a Spitter's or Lancer's
    /// next wind-up, a Brood's next release).
    pub cooldown: u32,
    /// Spitter strafing direction around its target: 1 counter-clockwise,
    /// -1 clockwise. Picked at spawn.
    pub orbit: i32,
    /// Telegraph: ticks left before the attack lands (0 = none). Spitter
    /// shot, Lancer dash, Brood release, Bloater burst (its fuse, once it
    /// touched a ship). It holds still meanwhile; the client draws the
    /// warning from it.
    pub windup: u32,
    /// Spitter: the shot's velocity; Lancer: the dash's velocity (both
    /// sub-px/tick, locked when the wind-up starts).
    pub aim: FxVec2,
    /// Lancer: dash ticks left.
    pub dash: u32,
    /// Lancer: ships already hit by this dash (bit per handle).
    pub struck: u8,
    /// Brood: the fissure it hangs back by.
    pub home: FxVec2,
    /// Ticks left of a Warding elite's shield on it (0 = none): it takes
    /// half damage meanwhile, like next to a Warden (they don't stack).
    pub ward: u32,
    /// Set on a pack's elite leader (design/BATTLE.md § Elite affixes).
    pub elite: Option<Elite>,
}

impl Enemy {
    pub fn new(kind: EnemyKind, pos: FxVec2) -> Self {
        Self {
            id: 0,
            kind,
            pos,
            hp: kind.hp(),
            cooldown: match kind {
                EnemyKind::Spitter => SPITTER_FIRST_SHOT,
                EnemyKind::Lancer => LANCER_FIRST_CHARGE,
                EnemyKind::Brood => BROOD_FIRST_RELEASE,
                _ => 0,
            },
            orbit: 1,
            windup: 0,
            aim: FxVec2::ZERO,
            dash: 0,
            struck: 0,
            home: pos,
            ward: 0,
            elite: None,
        }
    }

    /// Promote to an elite leader: `ELITE_HP_MULT`× HP, bigger, with
    /// `affixes` (bit per [`Affix`]) and a generated name (`elite_name`).
    pub fn make_elite(&mut self, affixes: u8, name: u32) {
        self.hp = self.kind.hp() * ELITE_HP_MULT;
        self.elite = Some(Elite {
            affixes,
            name,
            ward_cd: WARDING_FIRST,
            blink_cd: BLINK_FIRST,
            blink: 0,
            mark: FxVec2::ZERO,
            scorch_cd: SCORCH_DROP_INTERVAL,
        });
    }

    pub fn is_elite(&self) -> bool {
        self.elite.is_some()
    }

    /// Does this elite have `affix`? (Always false for a normal enemy.)
    pub fn has(&self, affix: Affix) -> bool {
        self.elite.is_some_and(|e| e.affixes & affix.bit() != 0)
    }

    /// HP it spawned with (elites: `ELITE_HP_MULT`× the kind's), for bars.
    pub fn max_hp(&self) -> i32 {
        if self.is_elite() {
            self.kind.hp() * ELITE_HP_MULT
        } else {
            self.kind.hp()
        }
    }

    /// Pixels; an elite is `ELITE_SIZE_PCT`% of its kind.
    pub fn radius(&self) -> i32 {
        if self.is_elite() {
            self.kind.radius() * ELITE_SIZE_PCT / 100
        } else {
            self.kind.radius()
        }
    }

    /// Sub-pixels per tick; Swift adds `SWIFT_SPEED_PCT`.
    pub fn speed(&self) -> i32 {
        if self.has(Affix::Swift) {
            self.kind.speed() * (100 + SWIFT_SPEED_PCT) / 100
        } else {
            self.kind.speed()
        }
    }
}

/// An elite affix (design/BATTLE.md § Elite affixes). Elites roll
/// `ELITE_AFFIXES` of them; `Elite::affixes` is a bitmask so Threat levels
/// can add a second.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Affix {
    /// Moves faster.
    Swift,
    /// Bursts on death like a Bloater.
    Volatile,
    /// Every few seconds shields itself and nearby allies (half damage).
    Warding,
    /// Teleports next to the ship after a visible mark.
    Blinking,
    /// Splits into two normal copies on death.
    Splitting,
    /// Leaves short-lived burning patches behind it.
    Scorching,
    /// Its hits slow the ship briefly.
    Chilling,
}

impl Affix {
    pub const ALL: [Affix; 7] = [
        Self::Swift,
        Self::Volatile,
        Self::Warding,
        Self::Blinking,
        Self::Splitting,
        Self::Scorching,
        Self::Chilling,
    ];

    pub const fn bit(self) -> u8 {
        1 << self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Swift => "Swift",
            Self::Volatile => "Volatile",
            Self::Warding => "Warding",
            Self::Blinking => "Blinking",
            Self::Splitting => "Splitting",
            Self::Scorching => "Scorching",
            Self::Chilling => "Chilling",
        }
    }

    /// Can a `kind` elite roll this? A Bloater already bursts (no
    /// Volatile); a Brood or Warden hangs back by design (no Blinking).
    pub const fn fits(self, kind: EnemyKind) -> bool {
        !matches!(
            (self, kind),
            (Self::Volatile, EnemyKind::Bloater)
                | (Self::Blinking, EnemyKind::Brood | EnemyKind::Warden)
        )
    }

    /// The affixes in a bitmask, in declaration order.
    pub fn in_mask(mask: u8) -> impl Iterator<Item = Affix> {
        Self::ALL.into_iter().filter(move |a| mask & a.bit() != 0)
    }
}

/// An elite's affixes, name and affix timers (ticks).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Elite {
    /// Bit per [`Affix`].
    pub affixes: u8,
    /// Seed of its generated name ([`elite_name`]), drawn from the sim RNG.
    pub name: u32,
    /// Warding: until the next shield pulse.
    pub ward_cd: u32,
    /// Blinking: until it can mark again.
    pub blink_cd: u32,
    /// Blinking: ticks left on the mark (0 = not marking); it lands on
    /// `mark` when this reaches 0.
    pub blink: u32,
    pub mark: FxVec2,
    /// Scorching: until it drops the next burning patch.
    pub scorch_cd: u32,
}

const NAME_HEADS: [&str; 12] = [
    "Gloom", "Rift", "Hex", "Null", "Shard", "Umbra", "Vex", "Murk", "Wisp", "Ash", "Grim", "Nyx",
];
const NAME_TAILS: [&str; 10] = [
    "whisker", "fang", "maw", "purr", "claw", "tail", "eye", "howl", "paw", "mane",
];
const NAME_TITLES: [&str; 8] = [
    "the Hollow",
    "the Starving",
    "the Unbound",
    "the Pale",
    "the Restless",
    "the Cracked",
    "the Hungry",
    "the Old",
];

/// An elite's display name from its seed, e.g. "Hexfang the Hollow".
/// Cosmetic, but derived only from sim state, so every peer shows the same.
pub fn elite_name(seed: u32) -> String {
    let head = NAME_HEADS[(seed % 12) as usize];
    let tail = NAME_TAILS[(seed / 12 % 10) as usize];
    let title = NAME_TITLES[(seed / 120 % 8) as usize];
    format!("{head}{tail} {title}")
}

/// A Scorching elite's burning patch: hurts ships standing in it.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Patch {
    /// Stable entity ID (unique within the mission).
    pub id: u32,
    pub pos: FxVec2,
    /// Ticks left.
    pub ttl: u32,
}

/// A Bloater's burst waiting to go off: left behind when a Bloater dies, so
/// killing it doesn't cancel the burst. Hurts ships and enemies within
/// `BLOATER_BLAST_RADIUS` when `fuse` runs out.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Blast {
    /// Stable entity ID (unique within the mission).
    pub id: u32,
    pub pos: FxVec2,
    /// Ticks left; it bursts on the tick this reaches 0.
    pub fuse: u32,
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
    /// Stable entity ID (unique within the mission).
    pub id: u32,
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
    /// What kind of attack it is (ship shots; enemy shots: `Bolt`, unused).
    pub attack: Attack,
    /// Enemy shots: fired by a Chilling elite, so a hit slows the ship.
    pub chill: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LootKind {
    Credits,
    VoidCrystal,
}

/// A loot drop on the field, collected by flying over it.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Pickup {
    /// Stable entity ID (unique within the mission).
    pub id: u32,
    pub kind: LootKind,
    pub amount: u32,
    pub pos: FxVec2,
    /// Ticks left before it disappears.
    pub ttl: u32,
    /// Dropped by an elite: the client marks it with a light beam.
    pub elite: bool,
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
                training: 0,
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
                training: 0,
            },
        };
        let (sheet, stats) = (ShipSheet::new(loadout), ShipStats::new(loadout));
        assert_eq!(stats.max_hull, sheet.hull);
        assert_eq!(stats.speed, px_per_tick(sheet.speed));
        assert_eq!(stats.basic_damage, sheet.basic_damage);
        assert_eq!(stats.skill_damage(SHOCKWAVE_DAMAGE), sheet.w_damage);
        assert_eq!(
            (stats.q_cooldown, stats.w_cooldown),
            (sheet.q_cooldown, sheet.w_cooldown)
        );
    }

    #[test]
    fn training_room_cuts_q_and_w_cooldowns_by_15_percent() {
        let trained = |ship| {
            let s = ShipSheet::new(Loadout {
                ship,
                upgrades: Upgrades {
                    training: 1,
                    ..Default::default()
                },
            });
            (s.q_cooldown, s.w_cooldown)
        };
        // Kite Q 4 -> 3.4 s, W 6 -> 5.1 s; Bulwark Q 12 -> 10.2 s, W 9 -> 7.65 s.
        assert_eq!(trained(ShipKind::Kite), (204, 306));
        assert_eq!(trained(ShipKind::Bulwark), (612, 459));
        let base = sheet(ShipKind::Kite, 0, 0, 0);
        assert_eq!((base.q_cooldown, base.w_cooldown), (240, 360));
    }
}
