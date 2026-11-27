//! Per-player input: the only data peers exchange each tick.
//!
//! Twin-stick controls (design/BATTLE.md B8, design/DESIGN.md § Player):
//! WASD moves the battleship, the cursor aims the main gun, the left mouse
//! button fires it, and 1/2/3/4 cast the ship's skills toward the cursor.

use serde::{Deserialize, Serialize};

/// WASD: thrust up / down / left / right (world y is up). Opposite keys
/// cancel; two neighbouring keys move diagonally at the same speed.
pub const INPUT_UP: u16 = 1 << 0;
pub const INPUT_DOWN: u16 = 1 << 1;
pub const INPUT_LEFT: u16 = 1 << 2;
pub const INPUT_RIGHT: u16 = 1 << 3;
/// Left mouse button held: fire the main gun toward the cursor (semi-auto)
/// or charge it, firing on release (heavy).
pub const INPUT_FIRE: u16 = 1 << 4;
/// Skill hotkeys 1/2/3/4. 1 and 2 use the battleship's two skills, aimed at
/// the cursor; 3 and 4 are reserved for the deferred E/R skills.
pub const INPUT_SKILL_1: u16 = 1 << 5;
pub const INPUT_SKILL_2: u16 = 1 << 6;
pub const INPUT_SKILL_3: u16 = 1 << 7;
pub const INPUT_SKILL_4: u16 = 1 << 8;

/// One player's input for one simulation tick. Must stay `Copy`, small and
/// serialisable: GGRS sends it over the wire and predicts it on rollback.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct NetInput {
    pub buttons: u16,
    /// Cursor position in whole world pixels (y up): where the main gun and
    /// skills aim.
    pub target_x: i32,
    pub target_y: i32,
}

impl NetInput {
    pub fn pressed(self, button: u16) -> bool {
        self.buttons & button != 0
    }

    /// The WASD thrust as a direction with components in -1..=1.
    pub fn thrust(self) -> (i32, i32) {
        let axis = |pos: u16, neg: u16| self.pressed(pos) as i32 - self.pressed(neg) as i32;
        (axis(INPUT_RIGHT, INPUT_LEFT), axis(INPUT_UP, INPUT_DOWN))
    }

    /// WASD bits that steer from `from` toward `to` (world px), each axis
    /// pressed only when it is more than `deadzone` px off. For scripted
    /// pilots that think in destinations.
    pub fn toward(from: (i32, i32), to: (i32, i32), deadzone: i32) -> u16 {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let mut b = 0;
        if dx > deadzone {
            b |= INPUT_RIGHT;
        } else if dx < -deadzone {
            b |= INPUT_LEFT;
        }
        if dy > deadzone {
            b |= INPUT_UP;
        } else if dy < -deadzone {
            b |= INPUT_DOWN;
        }
        b
    }
}
