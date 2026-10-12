//! Local input: mouse + keyboard or scripted bots. Runs in `ReadInputs`,
//! outside the rollback schedule; only the resulting `NetInput`s reach the
//! simulation.
//!
//! Controls (`design/DESIGN.md` § Player): left mouse button = click-to-move
//! (hold to keep steering; [`MovePress`] is shared with the carrier's
//! Pilot so the two can't drift apart), Q/W = the battleship's two skills aimed at the
//! cursor (E/R are unused so far); the basic attack fires on its own.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_ggrs::{LocalInputs, LocalPlayers};
use sim::input::{INPUT_MOVE, INPUT_SKILL_E, INPUT_SKILL_Q, INPUT_SKILL_R, INPUT_SKILL_W};
use sim::NetInput;

use crate::rollback::GameConfig;

/// The move button's state this frame, shared by the battle and the
/// carrier: a press starts a move toward the cursor and holding it keeps
/// following the cursor (TAKOAI-87).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MovePress {
    Idle,
    /// Pressed this frame.
    Start,
    /// Held since an earlier frame.
    Hold,
}

impl MovePress {
    pub const BUTTON: MouseButton = MouseButton::Left;

    pub fn read(mouse: &ButtonInput<MouseButton>) -> Self {
        if mouse.just_pressed(Self::BUTTON) {
            MovePress::Start
        } else if mouse.pressed(Self::BUTTON) {
            MovePress::Hold
        } else {
            MovePress::Idle
        }
    }

    /// Whether the player is steering toward the cursor.
    pub fn held(self) -> bool {
        self != MovePress::Idle
    }
}

/// The cursor in world pixels, if it is over the window.
pub fn cursor_world(window: &Window, camera: &Camera, cam_tf: &GlobalTransform) -> Option<Vec2> {
    window
        .cursor_position()
        .and_then(|c| camera.viewport_to_world_2d(cam_tf, c).ok())
}

/// Whether the first local player uses mouse + keyboard (`false` = all bots).
#[derive(Resource, Clone, Copy)]
pub struct KeyboardPlayer(pub bool);

/// Scripted local players for unattended runs: every 30..150 frames a bot
/// clicks a new random destination and toggles using skills toward it.
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
        buttons |= if bot.left % 2 == 0 {
            INPUT_SKILL_Q
        } else {
            INPUT_SKILL_W
        };
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
    if MovePress::read(mouse).held() {
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
        if let Some(p) = cursor_world(window, camera, cam_tf) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_press_starts_then_holds_until_release() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        assert_eq!(MovePress::read(&mouse), MovePress::Idle);
        mouse.press(MovePress::BUTTON);
        assert_eq!(MovePress::read(&mouse), MovePress::Start);
        mouse.clear();
        assert_eq!(MovePress::read(&mouse), MovePress::Hold);
        assert!(MovePress::read(&mouse).held());
        mouse.release(MovePress::BUTTON);
        assert_eq!(MovePress::read(&mouse), MovePress::Idle);
        // Other buttons don't move.
        mouse.press(MouseButton::Right);
        assert!(!MovePress::read(&mouse).held());
    }
}
