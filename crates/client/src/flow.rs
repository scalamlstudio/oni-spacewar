//! First-playable scene flow and placeholder UI.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sim::input::{INPUT_MOVE, INPUT_SKILL_Q};
use sim::NetInput;

use crate::mission::{self, MissionOutcome, MissionResult};
use crate::rollback::SimWorld;
use crate::save::{self, SaveError, SaveGame};

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameScreen {
    #[default]
    Title,
    Carrier,
    Battle,
    Result,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlowEvent {
    NewGame,
    Continue,
    LaunchMission,
    FinishMission,
    ReturnToCarrier,
    QuitToTitle,
}

pub fn transition(from: GameScreen, event: FlowEvent, has_save: bool) -> GameScreen {
    match (from, event) {
        (GameScreen::Title, FlowEvent::NewGame) => GameScreen::Carrier,
        (GameScreen::Title, FlowEvent::Continue) if has_save => GameScreen::Carrier,
        (GameScreen::Carrier, FlowEvent::LaunchMission) => GameScreen::Battle,
        (GameScreen::Battle, FlowEvent::FinishMission) => GameScreen::Result,
        (GameScreen::Result, FlowEvent::ReturnToCarrier) => GameScreen::Carrier,
        (_, FlowEvent::QuitToTitle) => GameScreen::Title,
        _ => from,
    }
}

pub struct FlowPlugin;

impl Plugin for FlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameScreen>()
            .insert_resource(SaveSlot::load())
            .init_resource::<PauseMenu>()
            .init_resource::<LastMissionResult>()
            .add_systems(OnEnter(GameScreen::Title), enter_title)
            .add_systems(OnEnter(GameScreen::Carrier), enter_carrier)
            .add_systems(OnEnter(GameScreen::Battle), enter_battle)
            .add_systems(OnEnter(GameScreen::Result), enter_result)
            .add_systems(
                OnExit(GameScreen::Title),
                (cleanup_screen, close_pause_menu).chain(),
            )
            .add_systems(
                OnExit(GameScreen::Carrier),
                (cleanup_screen, close_pause_menu).chain(),
            )
            .add_systems(
                OnExit(GameScreen::Battle),
                (cleanup_screen, remove_battle_world, close_pause_menu).chain(),
            )
            .add_systems(
                OnExit(GameScreen::Result),
                (cleanup_screen, close_pause_menu).chain(),
            )
            .add_systems(
                Update,
                (
                    button_actions,
                    keyboard_actions,
                    toggle_pause_menu,
                    apply_pause_actions,
                    sync_pause_menu,
                ),
            )
            .add_systems(FixedUpdate, step_demo_battle);
    }
}

#[derive(Resource)]
struct SaveSlot {
    path: std::path::PathBuf,
    game: Option<SaveGame>,
    status: String,
}

impl SaveSlot {
    fn load() -> Self {
        let path = save::save_path();
        match save::load(&path) {
            Ok(game) => Self {
                path,
                game: Some(game),
                status: "Save loaded".to_string(),
            },
            Err(SaveError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Self {
                path,
                game: None,
                status: "No save found".to_string(),
            },
            Err(e) => Self {
                path,
                game: None,
                status: format!("Save unavailable: {e}"),
            },
        }
    }

    fn store(&mut self) {
        if let Some(game) = &self.game {
            match save::store(&self.path, game) {
                Ok(()) => self.status = "Saved".to_string(),
                Err(e) => self.status = format!("Save failed: {e}"),
            }
        }
    }
}

#[derive(Resource, Default)]
struct PauseMenu {
    open: bool,
    dirty: bool,
}

#[derive(Resource, Default)]
struct LastMissionResult(Option<MissionResult>);

#[derive(Component)]
struct ScreenEntity;

#[derive(Component)]
struct PauseEntity;

#[derive(Component, Clone, Copy)]
enum DemoButton {
    NewGame,
    Continue,
    QuitGame,
    LaunchMission,
    BuyUpgrade,
    Save,
    Victory,
    Defeat,
    ReturnCarrier,
    Resume,
    QuitToTitle,
}

fn text_style(size: f32, color: Color) -> (TextFont, TextColor) {
    (
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn panel(commands: &mut Commands, title: &str, body: String) {
    commands.spawn((
        ScreenEntity,
        Text::new(format!("{title}\n\n{body}")),
        text_style(24.0, Color::srgb(0.9, 0.92, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            left: px(32),
            top: px(28),
            max_width: px(760),
            ..default()
        },
    ));
}

fn button(commands: &mut Commands, action: DemoButton, label: &str, top: f32, enabled: bool) {
    let bg = if enabled {
        Color::srgb(0.12, 0.22, 0.28)
    } else {
        Color::srgb(0.11, 0.11, 0.11)
    };
    commands
        .spawn((
            ScreenEntity,
            action,
            Button,
            BackgroundColor(bg),
            Node {
                position_type: PositionType::Absolute,
                left: px(34),
                top: px(top),
                width: px(190),
                height: px(42),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.35, 0.52, 0.58)),
        ))
        .with_child((
            Text::new(label),
            text_style(
                18.0,
                if enabled {
                    Color::WHITE
                } else {
                    Color::srgb(0.45, 0.45, 0.45)
                },
            ),
        ));
}

fn enter_title(mut commands: Commands, save: Res<SaveSlot>) {
    panel(
        &mut commands,
        "Oni Spacewar",
        format!(
            "First playable flow skeleton\n{}\n\nN New Game    C Continue    Q Quit",
            save.status
        ),
    );
    button(&mut commands, DemoButton::NewGame, "New Game", 170.0, true);
    button(
        &mut commands,
        DemoButton::Continue,
        "Continue",
        222.0,
        save.game.is_some(),
    );
    button(&mut commands, DemoButton::QuitGame, "Quit", 274.0, true);
}

fn enter_carrier(
    mut commands: Commands,
    mut save: ResMut<SaveSlot>,
    mut pending_result: ResMut<LastMissionResult>,
) {
    if let Some(result) = pending_result.0.take() {
        if let Some(game) = &mut save.game {
            game.record_mission_return(result.credits, &result.resources);
            save.store();
        }
    }
    let status = save.status.clone();
    let game = save.game.get_or_insert_with(SaveGame::default);
    let body = format!(
        "Carrier placeholder\nCredits: {}\nSalvage: {}\nShip: {}\nMissions completed: {}\nUpgrades: {}\n{}\n\nL Launch    U Buy upgrade    S Save    Esc Pause",
        game.credits,
        game.resources.get("salvage").copied().unwrap_or(0),
        game.selected_battleship,
        game.mission_count,
        game.purchased_upgrades.len(),
        status,
    );
    panel(&mut commands, "Carrier", body);
    button(
        &mut commands,
        DemoButton::LaunchMission,
        "Launch Mission",
        260.0,
        true,
    );
    button(
        &mut commands,
        DemoButton::BuyUpgrade,
        "Buy Upgrade",
        312.0,
        true,
    );
    button(&mut commands, DemoButton::Save, "Save", 364.0, true);
}

fn enter_battle(mut commands: Commands, save: Res<SaveSlot>) {
    let game = save.game.clone().unwrap_or_default();
    let config = mission::config_from_save(&game);
    commands.insert_resource(SimWorld(mission::launch_sim(&config)));
    panel(
        &mut commands,
        "Battle",
        format!(
            "Elimination placeholder\nLoadouts: {}\nSeed: {}\n\nLeft mouse move, Q fire\nV Victory    F Defeat    Esc Pause",
            config.ship_loadouts.len(),
            config.seed
        ),
    );
    button(&mut commands, DemoButton::Victory, "Victory", 230.0, true);
    button(&mut commands, DemoButton::Defeat, "Defeat", 282.0, true);
}

fn enter_result(mut commands: Commands, result: Res<LastMissionResult>) {
    let text = match &result.0 {
        Some(result) => format!(
            "Outcome: {:?}\nCredits earned: {}\nSalvage earned: {}\nWaves cleared: {}\n\nEnter Return",
            result.outcome,
            result.credits,
            result.resources.get("salvage").copied().unwrap_or(0),
            result.waves_cleared
        ),
        None => "No result recorded\n\nEnter Return".to_string(),
    };
    panel(&mut commands, "Result", text);
    button(
        &mut commands,
        DemoButton::ReturnCarrier,
        "Return to Carrier",
        230.0,
        true,
    );
}

fn cleanup_screen(mut commands: Commands, entities: Query<Entity, With<ScreenEntity>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

fn remove_battle_world(mut commands: Commands) {
    commands.remove_resource::<SimWorld>();
}

fn close_pause_menu(mut pause: ResMut<PauseMenu>) {
    pause.open = false;
    pause.dirty = true;
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn button_actions(
    mut interactions: Query<(&Interaction, &DemoButton), (Changed<Interaction>, With<Button>)>,
    state: Res<State<GameScreen>>,
    mut save: ResMut<SaveSlot>,
    world: Option<Res<SimWorld>>,
    mut pending_result: ResMut<LastMissionResult>,
    mut next: ResMut<NextState<GameScreen>>,
    mut pause: ResMut<PauseMenu>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            DemoButton::Save => {
                save.store();
                next.set(GameScreen::Carrier);
                continue;
            }
            DemoButton::NewGame => {
                save.game = Some(SaveGame::default());
                save.store();
                next.set(transition(*state.get(), FlowEvent::NewGame, true));
                continue;
            }
            DemoButton::BuyUpgrade => {
                if let Some(game) = &mut save.game {
                    game.buy_placeholder_upgrade();
                    save.store();
                }
                next.set(GameScreen::Carrier);
                continue;
            }
            DemoButton::QuitGame
                if matches!(state.get(), GameScreen::Carrier | GameScreen::Battle) =>
            {
                save.store();
            }
            DemoButton::QuitToTitle => save.store(),
            _ => {}
        }
        handle_action(
            *action,
            state.get(),
            save.game.is_some(),
            world.as_deref(),
            &mut pending_result,
            &mut next,
            &mut pause,
            &mut exit,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn keyboard_actions(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameScreen>>,
    mut save: ResMut<SaveSlot>,
    world: Option<Res<SimWorld>>,
    mut pending_result: ResMut<LastMissionResult>,
    mut next: ResMut<NextState<GameScreen>>,
    mut pause: ResMut<PauseMenu>,
    mut exit: MessageWriter<AppExit>,
) {
    let action = match state.get() {
        GameScreen::Title if keys.just_pressed(KeyCode::KeyN) => Some(DemoButton::NewGame),
        GameScreen::Title if keys.just_pressed(KeyCode::KeyC) => Some(DemoButton::Continue),
        GameScreen::Title if keys.just_pressed(KeyCode::KeyQ) => Some(DemoButton::QuitGame),
        GameScreen::Carrier if keys.just_pressed(KeyCode::KeyL) => Some(DemoButton::LaunchMission),
        GameScreen::Carrier if keys.just_pressed(KeyCode::KeyU) => Some(DemoButton::BuyUpgrade),
        GameScreen::Carrier if keys.just_pressed(KeyCode::KeyS) => Some(DemoButton::Save),
        GameScreen::Battle if keys.just_pressed(KeyCode::KeyV) => Some(DemoButton::Victory),
        GameScreen::Battle if keys.just_pressed(KeyCode::KeyF) => Some(DemoButton::Defeat),
        GameScreen::Result if keys.just_pressed(KeyCode::Enter) => Some(DemoButton::ReturnCarrier),
        _ => None,
    };
    if matches!(action, Some(DemoButton::Save)) {
        save.store();
        next.set(GameScreen::Carrier);
        return;
    }
    if matches!(action, Some(DemoButton::NewGame)) {
        save.game = Some(SaveGame::default());
        save.store();
        next.set(transition(*state.get(), FlowEvent::NewGame, true));
        return;
    }
    if matches!(action, Some(DemoButton::BuyUpgrade)) {
        if let Some(game) = &mut save.game {
            game.buy_placeholder_upgrade();
            save.store();
        }
        next.set(GameScreen::Carrier);
        return;
    }
    if let Some(action) = action {
        handle_action(
            action,
            state.get(),
            save.game.is_some(),
            world.as_deref(),
            &mut pending_result,
            &mut next,
            &mut pause,
            &mut exit,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_action(
    action: DemoButton,
    state: &GameScreen,
    has_save: bool,
    world: Option<&SimWorld>,
    pending_result: &mut LastMissionResult,
    next: &mut NextState<GameScreen>,
    pause: &mut PauseMenu,
    exit: &mut MessageWriter<AppExit>,
) {
    match action {
        DemoButton::NewGame => next.set(transition(*state, FlowEvent::NewGame, has_save)),
        DemoButton::Continue if has_save => {
            next.set(transition(*state, FlowEvent::Continue, has_save));
        }
        DemoButton::QuitGame => {
            exit.write(AppExit::Success);
        }
        DemoButton::LaunchMission => {
            next.set(transition(*state, FlowEvent::LaunchMission, has_save));
        }
        DemoButton::Victory | DemoButton::Defeat => {
            if let Some(world) = world {
                let outcome = if matches!(action, DemoButton::Victory) {
                    MissionOutcome::Victory
                } else {
                    MissionOutcome::Defeat
                };
                pending_result.0 = Some(mission::result_from_sim(world, outcome));
            }
            next.set(transition(*state, FlowEvent::FinishMission, has_save));
        }
        DemoButton::ReturnCarrier => {
            next.set(transition(*state, FlowEvent::ReturnToCarrier, has_save));
        }
        DemoButton::Resume => {
            pause.open = false;
            pause.dirty = true;
        }
        DemoButton::QuitToTitle => {
            pause.open = false;
            pause.dirty = true;
            next.set(transition(*state, FlowEvent::QuitToTitle, has_save));
        }
        DemoButton::Continue | DemoButton::BuyUpgrade | DemoButton::Save => {}
    }
}

fn toggle_pause_menu(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameScreen>>,
    mut pause: ResMut<PauseMenu>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if matches!(state.get(), GameScreen::Carrier | GameScreen::Battle) {
        pause.open = !pause.open;
        pause.dirty = true;
    }
}

fn apply_pause_actions(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameScreen>>,
    mut save: ResMut<SaveSlot>,
    mut pause: ResMut<PauseMenu>,
    mut next: ResMut<NextState<GameScreen>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !pause.open {
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        pause.open = false;
        pause.dirty = true;
    } else if keys.just_pressed(KeyCode::KeyT) {
        save.store();
        pause.open = false;
        pause.dirty = true;
        next.set(transition(
            *state.get(),
            FlowEvent::QuitToTitle,
            save.game.is_some(),
        ));
    } else if keys.just_pressed(KeyCode::KeyQ) {
        save.store();
        exit.write(AppExit::Success);
    }
}

fn sync_pause_menu(
    mut commands: Commands,
    mut pause: ResMut<PauseMenu>,
    entities: Query<Entity, With<PauseEntity>>,
) {
    if !pause.dirty {
        return;
    }
    pause.dirty = false;
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    if !pause.open {
        return;
    }
    commands.spawn((
        PauseEntity,
        Text::new("Paused\n\nR Resume    T Title    Q Quit"),
        text_style(26.0, Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(320),
            top: px(110),
            padding: UiRect::all(px(18)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.92)),
    ));
    pause_button(&mut commands, DemoButton::Resume, "Resume", 240.0);
    pause_button(
        &mut commands,
        DemoButton::QuitToTitle,
        "Quit to Title",
        292.0,
    );
    pause_button(&mut commands, DemoButton::QuitGame, "Quit Game", 344.0);
}

fn pause_button(commands: &mut Commands, action: DemoButton, label: &str, top: f32) {
    commands
        .spawn((
            PauseEntity,
            action,
            Button,
            BackgroundColor(Color::srgb(0.12, 0.22, 0.28)),
            Node {
                position_type: PositionType::Absolute,
                left: px(320),
                top: px(top),
                width: px(190),
                height: px(42),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.35, 0.52, 0.58)),
        ))
        .with_child((Text::new(label), text_style(18.0, Color::WHITE)));
}

#[allow(clippy::too_many_arguments)]
fn step_demo_battle(
    state: Res<State<GameScreen>>,
    pause: Res<PauseMenu>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut world: Option<ResMut<SimWorld>>,
    mut last_cursor: Local<IVec2>,
) {
    if *state.get() != GameScreen::Battle || pause.open {
        return;
    }
    let Some(mut world) = world.take() else {
        return;
    };
    if let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), cameras.single()) {
        if let Some(p) = window
            .cursor_position()
            .and_then(|c| camera.viewport_to_world_2d(cam_tf, c).ok())
        {
            *last_cursor = p.round().as_ivec2();
        }
    }
    let mut buttons = 0;
    if mouse.pressed(MouseButton::Left) {
        buttons |= INPUT_MOVE;
    }
    if keys.pressed(KeyCode::KeyQ) {
        buttons |= INPUT_SKILL_Q;
    }
    world.step(&[NetInput {
        buttons,
        target_x: last_cursor.x,
        target_y: last_cursor.y,
    }]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machine_transitions_through_demo_loop() {
        let mut state = GameScreen::Title;
        state = transition(state, FlowEvent::NewGame, false);
        assert_eq!(state, GameScreen::Carrier);
        state = transition(state, FlowEvent::LaunchMission, true);
        assert_eq!(state, GameScreen::Battle);
        state = transition(state, FlowEvent::FinishMission, true);
        assert_eq!(state, GameScreen::Result);
        state = transition(state, FlowEvent::ReturnToCarrier, true);
        assert_eq!(state, GameScreen::Carrier);
        state = transition(state, FlowEvent::QuitToTitle, true);
        assert_eq!(state, GameScreen::Title);
    }

    #[test]
    fn continue_stays_on_title_without_save() {
        assert_eq!(
            transition(GameScreen::Title, FlowEvent::Continue, false),
            GameScreen::Title
        );
        assert_eq!(
            transition(GameScreen::Title, FlowEvent::Continue, true),
            GameScreen::Carrier
        );
    }
}
