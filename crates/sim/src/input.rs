//! Per-player input: the only data peers exchange each tick.

use serde::{Deserialize, Serialize};

/// Primary mouse button held: set the move target to `target`.
pub const INPUT_MOVE: u8 = 1 << 0;
/// Skill hotkeys Q/W/E/R (design/DESIGN.md § Player). Only Q does anything
/// yet: a placeholder shot toward `target`.
pub const INPUT_SKILL_Q: u8 = 1 << 1;
pub const INPUT_SKILL_W: u8 = 1 << 2;
pub const INPUT_SKILL_E: u8 = 1 << 3;
pub const INPUT_SKILL_R: u8 = 1 << 4;

/// One player's input for one simulation tick. Must stay `Copy`, small and
/// serialisable: GGRS sends it over the wire and predicts it on rollback.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct NetInput {
    pub buttons: u8,
    /// Cursor position in whole world pixels (y up). Used as the move target
    /// while `INPUT_MOVE` is held and as the aim point for skills.
    pub target_x: i32,
    pub target_y: i32,
}

impl NetInput {
    pub fn pressed(self, button: u8) -> bool {
        self.buttons & button != 0
    }
}
