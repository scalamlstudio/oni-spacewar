//! Local input: mouse + keyboard or scripted bots. Runs in `ReadInputs`,
//! outside the rollback schedule; only the resulting `NetInput`s reach the
//! simulation.
//!
//! Battle controls (`design/DESIGN.md` § Player, design/BATTLE.md B8):
//! twin-stick. WASD moves the battleship, the cursor aims the main gun, the
//! left mouse button fires it (hold: semi-auto; press, hold, release:
//! heavy), and 1/2 cast the ship's two skills toward the cursor (3/4 are
//! reserved). Nothing fires on its own. [`MovePress`] is the Carrier's
//! click-to-walk button; the battle no longer uses it to move.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_ggrs::{LocalInputs, LocalPlayers};
use sim::input::{
    INPUT_DOWN, INPUT_FIRE, INPUT_LEFT, INPUT_RIGHT, INPUT_SKILL_1, INPUT_SKILL_2, INPUT_SKILL_3,
    INPUT_SKILL_4, INPUT_UP,
};
use sim::NetInput;

use crate::rollback::GameConfig;

/// The Carrier's walk button this frame: a press starts a move toward the cursor and holding it keeps
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
/// picks a new random WASD heading and aim point, and whether to hold the
/// gun and use skills toward it.
#[derive(Resource, Default)]
pub struct BotBrains(HashMap<usize, Bot>);

#[derive(Default)]
pub struct Bot {
    rng: u64,
    left: u32,
    target: IVec2,
    keys: u16,
    firing: bool,
}

fn bot_input(brains: &mut BotBrains, handle: usize) -> NetInput {
    let bot = brains.0.entry(handle).or_insert_with(|| Bot {
        rng: (handle as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
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
        bot.keys = [INPUT_UP, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, 0][(a >> 32) as usize % 5]
            | [0, INPUT_LEFT, INPUT_RIGHT][(b >> 32) as usize % 3];
    }
    bot.left -= 1;
    buttons |= bot.keys;
    if bot.firing {
        // Fire held (a heavy charges and lets go when the burst ends).
        buttons |= INPUT_FIRE;
        buttons |= if bot.left % 2 == 0 {
            INPUT_SKILL_1
        } else {
            INPUT_SKILL_2
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
    NetInput {
        buttons: battle_buttons(keys, mouse),
        target_x: cursor.x,
        target_y: cursor.y,
    }
}

/// The battle's buttons from mouse + keyboard (also the offline Demo
/// battle in `flow`): WASD, left mouse = fire, 1/2/3/4 = skills.
pub fn battle_buttons(keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> u16 {
    let mut buttons = 0;
    if mouse.pressed(FIRE_BUTTON) {
        buttons |= INPUT_FIRE;
    }
    for (key, bit) in [
        (KeyCode::KeyW, INPUT_UP),
        (KeyCode::KeyS, INPUT_DOWN),
        (KeyCode::KeyA, INPUT_LEFT),
        (KeyCode::KeyD, INPUT_RIGHT),
        (KeyCode::Digit1, INPUT_SKILL_1),
        (KeyCode::Digit2, INPUT_SKILL_2),
        (KeyCode::Digit3, INPUT_SKILL_3),
        (KeyCode::Digit4, INPUT_SKILL_4),
    ] {
        if keys.pressed(key) {
            buttons |= bit;
        }
    }
    buttons
}

/// The main gun's trigger.
pub const FIRE_BUTTON: MouseButton = MouseButton::Left;

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

    #[test]
    fn battle_keys_are_twin_stick() {
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut mouse = ButtonInput::<MouseButton>::default();
        assert_eq!(battle_buttons(&keys, &mouse), 0);
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::KeyD);
        keys.press(KeyCode::Digit2);
        mouse.press(FIRE_BUTTON);
        assert_eq!(
            battle_buttons(&keys, &mouse),
            INPUT_UP | INPUT_RIGHT | INPUT_SKILL_2 | INPUT_FIRE
        );
        // Q no longer casts anything.
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyQ);
        assert_eq!(battle_buttons(&keys, &ButtonInput::default()), 0);
    }
}
