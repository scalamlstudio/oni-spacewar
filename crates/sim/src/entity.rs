//! Plain-data entities. Each kind lives in its own `Vec` inside
//! [`SimState`](crate::SimState), so creation order is iteration order and is
//! part of the rolled-back, checksummed state. Positions are sub-pixels.

use serde::{Deserialize, Serialize};

use crate::fixed::FxVec2;

/// One player's battleship. `handle` is the GGRS player handle and equals the
/// ship's index in `SimState::ships`.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Ship {
    pub handle: usize,
    pub pos: FxVec2,
    /// Placeholder hull points. Reaching 0 loses the mission for that player.
    pub hp: i32,
    /// Click-to-move destination; the ship stops when it gets there.
    pub target: FxVec2,
    /// Ticks until the Q skill can fire again.
    pub cooldown: u32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Enemy {
    pub pos: FxVec2,
    pub hp: i32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Projectile {
    /// Handle of the ship that fired it.
    pub owner: usize,
    pub pos: FxVec2,
    /// Sub-pixels per tick.
    pub vel: FxVec2,
    /// Ticks left before it expires.
    pub ttl: u32,
}
