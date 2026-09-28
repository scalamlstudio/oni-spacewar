//! Local input: mouse + keyboard or scripted bots. Runs in `ReadInputs`,
//! outside the rollback schedule; only the resulting `NetInput`s reach the
//! simulation.
//!
//! Controls (from `src/general/control.lua`, movement per `design/DESIGN.md`):
//! left mouse button = click-to-move (hold to keep steering), Q = fire toward
//! the cursor, W/E/R = unused skill slots.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_ggrs::{LocalInputs, LocalPlayers};
use sim::input::{INPUT_MOVE, INPUT_SKILL_E, INPUT_SKILL_Q, INPUT_SKILL_R, INPUT_SKILL_W};
use sim::NetInput;

use crate::rollback::GameConfig;

/// Whether the first local player uses mouse + keyboard (`false` = all bots).
#[derive(Resource, Clone, Copy)]
pub struct KeyboardPlayer(pub bool);

/// Scripted local players for unattended runs: every 30..150 frames a bot
/// clicks a new random destination and toggles firing at it.
#[derive(Resource, Default)]
pub struct BotBrains(HashMap<usize, Bot>);

#[derive(Default)]
pub struct Bot {
    rng: u64,
    left: u32,
    target: IVec2,
    firing: bool,
}

fn bot_input(brains: &mut BotBrains, handle: usize) -> NetInput {
    let bot = brains.0.entry(handle).or_insert_with(|| Bot {
        rng: (handle as u64 + 1) * 0x9E37_79B9_7F4A_7C15,
        ..default()
    });
    let mut buttons = 0;
    if bot.left == 0 {
        let mut next = || {
            bot.rng ^= bot.rng << 13;
            bot.rng ^= bot.rng >> 7;
            bot.rng ^= bot.rng << 17;
            bot.rng
        };
        let (a, b, c) = (next(), next(), next());
        bot.target = IVec2::new((a % 1201) as i32 - 600, (b % 1201) as i32 - 600);
        bot.firing = c % 3 != 0;
        bot.left = 30 + (c >> 8) as u32 % 121;
        buttons |= INPUT_MOVE;
    }
    bot.left -= 1;
    if bot.firing {
        buttons |= INPUT_SKILL_Q;
    }
    NetInput {
        buttons,
        target_x: bot.target.x,
        target_y: bot.target.y,
    }
}

fn human_input(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    cursor: IVec2,
) -> NetInput {
    let mut buttons = 0;
    if mouse.pressed(MouseButton::Left) {
        buttons |= INPUT_MOVE;
    }
    for (key, bit) in [
        (KeyCode::KeyQ, INPUT_SKILL_Q),
        (KeyCode::KeyW, INPUT_SKILL_W),
        (KeyCode::KeyE, INPUT_SKILL_E),
        (KeyCode::KeyR, INPUT_SKILL_R),
    ] {
        if keys.pressed(key) {
            buttons |= bit;
        }
    }
    NetInput {
        buttons,
        target_x: cursor.x,
        target_y: cursor.y,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn read_local_inputs(
    mut commands: Commands,
    local_players: Res<LocalPlayers>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    keyboard: Res<KeyboardPlayer>,
    mut brains: ResMut<BotBrains>,
    mut last_cursor: Local<IVec2>,
) {
    // Cursor in world pixels; keep the last value while it is off-window.
    if let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), cameras.single()) {
        if let Some(p) = window
            .cursor_position()
            .and_then(|c| camera.viewport_to_world_2d(cam_tf, c).ok())
        {
            *last_cursor = p.round().as_ivec2();
        }
    }
    let mut local_inputs = HashMap::new();
    for (i, handle) in local_players.0.iter().enumerate() {
        // The first local player is on mouse + keyboard unless --bot/--headless;
        // any further local players (synctest) are always bots.
        let input = match (&keys, &mouse) {
            (Some(keys), Some(mouse)) if i == 0 && keyboard.0 => {
                human_input(keys, mouse, *last_cursor)
            }
            _ => bot_input(&mut brains, *handle),
        };
        local_inputs.insert(*handle, input);
    }
    commands.insert_resource(LocalInputs::<GameConfig>(local_inputs));
}
