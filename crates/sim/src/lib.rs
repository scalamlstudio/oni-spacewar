//! Deterministic mission simulation for Oni Spacewar.
//!
//! This crate is deliberately Bevy-free. The client wraps [`SimState`] in a
//! rollback-registered resource and calls [`SimState::step`] once per GGRS
//! tick; everything that affects gameplay lives inside `SimState`, so saving,
//! loading and checksumming it covers the whole simulation.
//!
//! Rules every module here follows (see `ARCHITECTURE.md`):
//! - Fixed timestep driven only by player inputs. No wall clock, no frame
//!   delta, no unseeded randomness: the only randomness is [`SimRng`], which
//!   is part of the state.
//! - Integer / fixed-point math only. No `f32`/`f64` in gameplay state, and no
//!   platform float functions (`sin`, `cos`, `sqrt`, `atan2`, ...): directions
//!   come from the committed lookup table in [`trig`], lengths from
//!   `i64::isqrt`.
//! - Stable iteration order: ships live in a `Vec` indexed by player handle and
//!   every other entity kind in its own `Vec` (see [`entity`]), so their order
//!   is part of the state.
//! - Never assume a single battleship: one ship per player handle, 1..=4.

pub mod collision;
pub mod entity;
pub mod fixed;
pub mod input;
pub mod mission;
pub mod rng;
pub mod state;
pub mod trig;
pub mod tuning;

pub use entity::{
    Enemy, EnemyKind, Fissure, Loadout, Loot, LootKind, Pickup, Projectile, Ship, ShipKind,
    ShipSheet, ShipStats, Upgrades,
};
pub use fixed::{FxVec2, SUB};
pub use input::NetInput;
pub use mission::{Mission, MissionOutcome, MissionStatus};
pub use rng::SimRng;
pub use state::{SimParams, SimState, MAX_PLAYERS};
