//! The whole rolled-back simulation state and its fixed-timestep step.
//!
//! Gameplay is not ported yet. The only behaviour is a placeholder: each
//! ship drifts 8-way from its input so the rollback / netcode plumbing has
//! something to exercise. Replace it when mission gameplay lands.

use core::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::fixed::{FxVec2, SUB};
use crate::input::NetInput;
use crate::rng::SimRng;

/// Most players a mission can hold (design: up to 4 per mission).
pub const MAX_PLAYERS: usize = 4;

/// Arena half extents in pixels (centred on the origin). Placeholder.
pub const ARENA_HALF_W: i32 = 480;
pub const ARENA_HALF_H: i32 = 270;
const PLACEHOLDER_SPEED: i32 = 4 * SUB;

/// Match setup that must be identical on every peer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SimParams {
    pub num_players: usize,
    pub seed: u64,
}

/// One player's battleship. `handle` is the GGRS player handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Ship {
    pub handle: usize,
    pub pos: FxVec2,
}

/// Everything that affects gameplay. Saved/loaded wholesale on rollback and
/// hashed for desync detection, so any new gameplay field goes in here.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SimState {
    pub params: SimParams,
    /// Number of ticks simulated so far.
    pub frame: u32,
    pub rng: SimRng,
    /// One ship per player, indexed by handle (stable order).
    pub ships: Vec<Ship>,
}

impl SimState {
    pub fn new(params: SimParams) -> Self {
        assert!(
            (1..=MAX_PLAYERS).contains(&params.num_players),
            "num_players must be 1..={MAX_PLAYERS}"
        );
        let n = params.num_players as i32;
        let ships = (0..params.num_players)
            .map(|handle| Ship {
                handle,
                pos: FxVec2::from_px(-(n - 1) * 120 + handle as i32 * 240, -180),
            })
            .collect();
        Self {
            params,
            frame: 0,
            rng: SimRng::new(params.seed),
            ships,
        }
    }

    /// Advance one fixed tick. `inputs[h]` is player `h`'s input.
    pub fn step(&mut self, inputs: &[NetInput]) {
        assert_eq!(inputs.len(), self.ships.len(), "one input per player");
        for ship in &mut self.ships {
            let (dx, dy) = inputs[ship.handle].axis();
            // Placeholder 8-way movement; diagonals scaled by ~1/sqrt(2) in Q16.
            let d = if dx != 0 && dy != 0 {
                crate::fixed::mul_q16(PLACEHOLDER_SPEED, 46341)
            } else {
                PLACEHOLDER_SPEED
            };
            ship.pos.x = (ship.pos.x + dx * d).clamp(-ARENA_HALF_W * SUB, ARENA_HALF_W * SUB);
            ship.pos.y = (ship.pos.y + dy * d).clamp(-ARENA_HALF_H * SUB, ARENA_HALF_H * SUB);
        }
        self.frame += 1;
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

    fn scripted_inputs(frame: u32, players: usize) -> Vec<NetInput> {
        const DIRS: [u8; 5] = [
            0,
            INPUT_UP,
            INPUT_RIGHT | INPUT_DOWN,
            INPUT_LEFT,
            INPUT_UP | INPUT_LEFT,
        ];
        (0..players)
            .map(|p| NetInput {
                buttons: DIRS[((frame / 17) as usize + p) % DIRS.len()],
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
    fn one_ship_per_player_in_handle_order() {
        let s = SimState::new(SimParams {
            num_players: 3,
            seed: 0,
        });
        let handles: Vec<_> = s.ships.iter().map(|s| s.handle).collect();
        assert_eq!(handles, vec![0, 1, 2]);
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
    }
}
