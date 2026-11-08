//! What happened during one tick, for the client's feedback (hit flash,
//! bursts, damage numbers, shake, sound). [`SimState::events`](crate::SimState)
//! holds the events of the last `step` only; it is cleared at the start of
//! every step. Events never feed back into the simulation.
//!
//! Under rollback a frame can be simulated more than once, so the client
//! keys each event by `(frame, SimEvent::key)` and drops repeats. Keys use
//! stable entity IDs (`Enemy::id`, `Projectile::id`, `Pickup::id`, ship
//! handles), never list indices.

use serde::{Deserialize, Serialize};

use crate::entity::{EnemyKind, LootKind};
use crate::fixed::FxVec2;

/// Which attack hit an enemy.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Attack {
    /// Basic-attack bolt.
    Bolt,
    /// One of Kite's Scatter bolts.
    Scatter,
    /// Bulwark's Shockwave.
    Shockwave,
}

/// A ship skill, named per ship.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Skill {
    Afterburn,
    Scatter,
    Bastion,
    Shockwave,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum SimEvent {
    /// A ship's attack hit an enemy. `source` is the shot's ID (0 for a
    /// Shockwave); `pos` is where the enemy was hit.
    Hit {
        enemy: u32,
        ship: usize,
        attack: Attack,
        source: u32,
        amount: i32,
        pos: FxVec2,
    },
    /// An enemy died this tick.
    Killed {
        enemy: u32,
        kind: EnemyKind,
        pos: FxVec2,
    },
    /// A ship took damage. `source` is the enemy (contact) or shot (spit)
    /// ID; `hull_lost` is what got past the shield.
    ShipDamaged {
        ship: usize,
        source: u32,
        amount: i32,
        hull_lost: i32,
        pos: FxVec2,
    },
    /// A ship fired its basic attack; `shot` is the new bolt's ID.
    Fired { ship: usize, shot: u32 },
    /// A ship used a skill at `pos` (the ship's position).
    SkillUsed {
        ship: usize,
        skill: Skill,
        pos: FxVec2,
    },
    /// A ship flew over a pickup.
    Collected {
        ship: usize,
        pickup: u32,
        kind: LootKind,
        amount: u32,
    },
    /// The mission ended this tick.
    MissionEnded { success: bool },
}

impl SimEvent {
    /// Identifies the event within its frame: the same thing happening on
    /// a re-simulated frame gives the same key.
    pub fn key(&self) -> (u8, u32, u32) {
        match *self {
            // A Shockwave hit has no shot: key it by the ship instead.
            Self::Hit {
                enemy,
                ship,
                source: 0,
                ..
            } => (7, enemy, ship as u32),
            Self::Hit { enemy, source, .. } => (0, enemy, source),
            Self::Killed { enemy, .. } => (1, enemy, 0),
            Self::ShipDamaged { ship, source, .. } => (2, ship as u32, source),
            Self::Fired { ship, shot } => (3, ship as u32, shot),
            Self::SkillUsed { ship, skill, .. } => (4, ship as u32, skill as u32),
            Self::Collected { ship, pickup, .. } => (5, ship as u32, pickup),
            Self::MissionEnded { .. } => (6, 0, 0),
        }
    }
}
