//! Per-player input: the only data peers exchange each tick.

use serde::{Deserialize, Serialize};

pub const INPUT_UP: u8 = 1 << 0;
pub const INPUT_DOWN: u8 = 1 << 1;
pub const INPUT_LEFT: u8 = 1 << 2;
pub const INPUT_RIGHT: u8 = 1 << 3;

/// One player's input for one simulation tick. Must stay `Copy`, small and
/// serialisable: GGRS sends it over the wire and predicts it on rollback.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct NetInput {
    pub buttons: u8,
}

impl NetInput {
    pub fn pressed(self, button: u8) -> bool {
        self.buttons & button != 0
    }

    /// Unit direction on each axis (-1, 0 or 1), up = +y.
    pub fn axis(self) -> (i32, i32) {
        let dx = self.pressed(INPUT_RIGHT) as i32 - self.pressed(INPUT_LEFT) as i32;
        let dy = self.pressed(INPUT_UP) as i32 - self.pressed(INPUT_DOWN) as i32;
        (dx, dy)
    }
}
