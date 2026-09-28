//! Bridges the Bevy-free `sim` crate into bevy_ggrs.
//!
//! The whole simulation is one resource, [`SimWorld`], registered for rollback
//! (saved/loaded by clone) and for checksumming (by `Hash`). One system in
//! `GgrsSchedule` feeds the confirmed/predicted inputs to `SimState::step`.
//! Nothing outside this schedule may mutate `SimWorld`.

use bevy::prelude::*;
use bevy_ggrs::prelude::*;
use sim::{NetInput, SimState};

pub type GameConfig = GgrsConfig<NetInput, matchbox_socket::PeerId>;

/// The rolled-back simulation state.
#[derive(Resource, Clone, Hash, Deref, DerefMut)]
pub struct SimWorld(pub SimState);

/// Negative control (`--inject-desync`): mixes a counter that is NOT rolled
/// back into the RNG every 997 ticks, so SyncTest / desync detection must fire.
#[derive(Resource, Clone, Copy, Default)]
pub struct InjectDesync(pub bool);

pub struct RollbackPlugin;

impl Plugin for RollbackPlugin {
    fn build(&self, app: &mut App) {
        app.rollback_resource_with_clone::<SimWorld>()
            .checksum_resource_with_hash::<SimWorld>()
            .init_resource::<InjectDesync>()
            .add_systems(
                GgrsSchedule,
                (crate::stats::track_sim_frame, step_sim, inject_desync).chain(),
            );
    }
}

fn step_sim(inputs: Res<PlayerInputs<GameConfig>>, mut world: ResMut<SimWorld>) {
    // PlayerInputs is indexed by player handle, matching `SimState::ships`.
    let inputs: Vec<NetInput> = inputs.iter().map(|(input, _status)| *input).collect();
    world.step(&inputs);
}

fn inject_desync(flag: Res<InjectDesync>, mut world: ResMut<SimWorld>, mut calls: Local<u64>) {
    if !flag.0 {
        return;
    }
    *calls += 1;
    if calls.is_multiple_of(997) {
        world.rng.0 ^= *calls;
    }
}
