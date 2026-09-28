//! Local input: keyboard or scripted bots. Runs in `ReadInputs`, outside the
//! rollback schedule; only the resulting `NetInput`s reach the simulation.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_ggrs::{LocalInputs, LocalPlayers};
use sim::input::{INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_UP};
use sim::NetInput;

use crate::rollback::GameConfig;

/// Whether the first local player uses the keyboard (`false` = all bots).
#[derive(Resource, Clone, Copy)]
pub struct KeyboardPlayer(pub bool);

/// Scripted local players for unattended runs: random 8-way direction changes
/// every 6..40 frames.
#[derive(Resource, Default)]
pub struct BotBrains(HashMap<usize, (u64, u8, u32)>);

fn bot_input(brains: &mut BotBrains, handle: usize) -> u8 {
    let (rng, buttons, left) =
        brains
            .0
            .entry(handle)
            .or_insert(((handle as u64 + 1) * 0x9E37_79B9_7F4A_7C15, 0, 0));
    if *left == 0 {
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        *rng ^= *rng << 17;
        let r = *rng;
        const DIRS: [u8; 9] = [
            0,
            INPUT_UP,
            INPUT_DOWN,
            INPUT_LEFT,
            INPUT_RIGHT,
            INPUT_UP | INPUT_LEFT,
            INPUT_UP | INPUT_RIGHT,
            INPUT_DOWN | INPUT_LEFT,
            INPUT_DOWN | INPUT_RIGHT,
        ];
        *buttons = DIRS[(r % 9) as usize];
        *left = 6 + ((r >> 16) % 35) as u32;
    }
    *left -= 1;
    *buttons
}

fn keyboard_input(keys: &ButtonInput<KeyCode>) -> u8 {
    let mut b = 0;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        b |= INPUT_UP;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        b |= INPUT_DOWN;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        b |= INPUT_LEFT;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        b |= INPUT_RIGHT;
    }
    b
}

pub fn read_local_inputs(
    mut commands: Commands,
    local_players: Res<LocalPlayers>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    keyboard: Res<KeyboardPlayer>,
    mut brains: ResMut<BotBrains>,
) {
    let mut local_inputs = HashMap::new();
    for (i, handle) in local_players.0.iter().enumerate() {
        // The first local player is on the keyboard unless --bot/--headless;
        // any further local players (synctest) are always bots.
        let buttons = match &keys {
            Some(keys) if i == 0 && keyboard.0 => keyboard_input(keys),
            _ => bot_input(&mut brains, *handle),
        };
        local_inputs.insert(*handle, NetInput { buttons });
    }
    commands.insert_resource(LocalInputs::<GameConfig>(local_inputs));
}
