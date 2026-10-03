//! First-playable scene flow and placeholder UI.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PrimaryWindow;
use sim::input::{INPUT_MOVE, INPUT_SKILL_Q, INPUT_SKILL_W};
use sim::tuning::KILL_TARGET;
use sim::{FxVec2, NetInput, SimState, SUB};

use crate::carrier::CarrierArrival;
use crate::mission::{self, MissionOutcome, MissionRequest, MissionResult};
use crate::rollback::SimWorld;
use crate::save::{self, LastResult, SaveError, SaveGame};

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
            .add_message::<MissionRequest>()
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
                    apply_mission_request,
                ),
            )
            // The demo battle steps the sim at the rollback rate, 60 Hz.
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .add_systems(FixedUpdate, step_demo_battle)
            .add_systems(
                Update,
                (autoplay_flow, autoplay_screenshots).run_if(resource_exists::<Autoplay>),
            );
    }
}

/// `--autoplay`: QA mode. Starts a new game with `ship`, flies one battle
/// with a scripted pilot, optionally saves window screenshots to `shots`
/// every 2 s, and quits a few seconds into the Result scene.
#[derive(Resource, Clone, Default)]
pub struct Autoplay {
    pub ship: String,
    pub shots: Option<PathBuf>,
}

#[derive(Resource)]
pub(crate) struct SaveSlot {
    path: std::path::PathBuf,
    pub(crate) game: Option<SaveGame>,
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
pub(crate) struct PauseMenu {
    pub(crate) open: bool,
    dirty: bool,
}

#[derive(Resource, Default)]
pub(crate) struct LastMissionResult(Option<MissionResult>);

#[derive(Component)]
pub(crate) struct ScreenEntity;

#[derive(Component)]
struct PauseEntity;

#[derive(Component, Clone, Copy)]
enum DemoButton {
    NewGame,
    Continue,
    QuitGame,
    QuitMission,
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

/// Books the returning mission into the save. The deck itself is spawned by
/// `carrier::CarrierPlugin` after this runs.
pub(crate) fn enter_carrier(
    mut save: ResMut<SaveSlot>,
    mut pending_result: ResMut<LastMissionResult>,
    mut arrival: ResMut<CarrierArrival>,
) {
    if let Some(result) = pending_result.0.take() {
        arrival.from_mission = true;
        if let Some(game) = &mut save.game {
            game.record_mission_return(result.credits, &result.resources);
            game.last_result = match result.outcome {
                MissionOutcome::Victory => LastResult::Success,
                MissionOutcome::Defeat | MissionOutcome::Abandoned => LastResult::Failed,
            };
            save.store();
        }
    }
    save.game.get_or_insert_with(SaveGame::default);
}

/// Launch from the Dock: remember the picked battleship and enter Battle.
fn apply_mission_request(
    mut requests: MessageReader<MissionRequest>,
    state: Res<State<GameScreen>>,
    mut save: ResMut<SaveSlot>,
    mut next: ResMut<NextState<GameScreen>>,
) {
    let Some(request) = requests.read().last() else {
        return;
    };
    if let Some(game) = &mut save.game {
        game.selected_battleship = request.battleship_id.clone();
    }
    save.store();
    next.set(transition(
        *state.get(),
        FlowEvent::LaunchMission,
        save.game.is_some(),
    ));
}

fn enter_battle(mut commands: Commands, save: Res<SaveSlot>) {
    let game = save.game.clone().unwrap_or_default();
    let config = mission::config_from_save(&game);
    commands.insert_resource(SimWorld(mission::launch_sim(&config)));
    commands.spawn((
        ScreenEntity,
        Text::new("Click to move. Your guns fire on their own.    Q / W skills    Esc pause"),
        text_style(16.0, Color::srgb(0.7, 0.75, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            bottom: px(10),
            ..default()
        },
    ));
}

fn enter_result(mut commands: Commands, result: Res<LastMissionResult>) {
    let text = match &result.0 {
        Some(r) => {
            let (title, kept) = if r.outcome.success() {
                ("Success", "Loot kept")
            } else {
                ("Failed", "Loot lost")
            };
            let quit = if r.outcome == MissionOutcome::Abandoned {
                " (mission abandoned)"
            } else {
                ""
            };
            format!(
                "{title}{quit}\nKills: {}/{KILL_TARGET}    Wave reached: {}\nCollected: {} credits, {} Void Crystal\nSuccess bonus: {} credits, {} Void Crystal\n{kept}: {} credits, {} Void Crystal\n\nEnter Return",
                r.kills,
                r.wave,
                r.collected.credits,
                r.collected.void_crystal,
                r.bonus.credits,
                r.bonus.void_crystal,
                r.credits,
                r.resources.get(mission::VOID_CRYSTAL_ID).copied().unwrap_or(0),
            )
        }
        None => "No result recorded\n\nEnter Return".to_string(),
    };
    panel(&mut commands, "Result", text);
    button(
        &mut commands,
        DemoButton::ReturnCarrier,
        "Return to Carrier",
        290.0,
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
            DemoButton::NewGame => {
                save.game = Some(SaveGame::default());
                save.store();
                next.set(transition(*state.get(), FlowEvent::NewGame, true));
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
        GameScreen::Result if keys.just_pressed(KeyCode::Enter) => Some(DemoButton::ReturnCarrier),
        _ => None,
    };
    if matches!(action, Some(DemoButton::NewGame)) {
        save.game = Some(SaveGame::default());
        save.store();
        next.set(transition(*state.get(), FlowEvent::NewGame, true));
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
        DemoButton::QuitMission => {
            // Quit Mission counts as Failed (Demo Spec § Elimination mission).
            pause.open = false;
            pause.dirty = true;
            if let Some(world) = world {
                pending_result.0 = Some(mission::result_from_sim(world, MissionOutcome::Abandoned));
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
        DemoButton::Continue => {}
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

#[allow(clippy::too_many_arguments)]
fn apply_pause_actions(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameScreen>>,
    mut save: ResMut<SaveSlot>,
    mut pause: ResMut<PauseMenu>,
    world: Option<Res<SimWorld>>,
    mut pending_result: ResMut<LastMissionResult>,
    mut next: ResMut<NextState<GameScreen>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !pause.open {
        return;
    }
    if keys.just_pressed(KeyCode::KeyM) && *state.get() == GameScreen::Battle {
        handle_action(
            DemoButton::QuitMission,
            state.get(),
            save.game.is_some(),
            world.as_deref(),
            &mut pending_result,
            &mut next,
            &mut pause,
            &mut exit,
        );
    } else if keys.just_pressed(KeyCode::KeyR) {
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
    state: Res<State<GameScreen>>,
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
    let in_battle = *state.get() == GameScreen::Battle;
    commands.spawn((
        PauseEntity,
        Text::new(if in_battle {
            "Paused\n\nR Resume    M Quit Mission (counts as Failed)\nT Title    Q Quit"
        } else {
            "Paused\n\nR Resume    T Title    Q Quit"
        }),
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
    if in_battle {
        pause_button(
            &mut commands,
            DemoButton::QuitMission,
            "Quit Mission",
            396.0,
        );
    }
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

/// Frames the ended battle stays on screen before the Result scene.
const RESULT_DELAY: u32 = 90;

#[allow(clippy::too_many_arguments)]
fn step_demo_battle(
    state: Res<State<GameScreen>>,
    pause: Res<PauseMenu>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut world: Option<ResMut<SimWorld>>,
    mut pending_result: ResMut<LastMissionResult>,
    mut next: ResMut<NextState<GameScreen>>,
    autoplay: Option<Res<Autoplay>>,
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
    let input = if autoplay.is_some() {
        autopilot(&world)
    } else {
        let mut buttons = 0;
        if mouse.pressed(MouseButton::Left) {
            buttons |= INPUT_MOVE;
        }
        if keys.pressed(KeyCode::KeyQ) {
            buttons |= INPUT_SKILL_Q;
        }
        if keys.pressed(KeyCode::KeyW) {
            buttons |= INPUT_SKILL_W;
        }
        NetInput {
            buttons,
            target_x: last_cursor.x,
            target_y: last_cursor.y,
        }
    };
    world.step(&[input]);
    if let Some(outcome) = mission::sim_outcome(&world) {
        if world.frame >= world.mission.end_frame + RESULT_DELAY {
            pending_result.0 = Some(mission::result_from_sim(&world, outcome));
            next.set(transition(
                GameScreen::Battle,
                FlowEvent::FinishMission,
                true,
            ));
        }
    }
}

/// Scripted pilot for `--autoplay`: circles the nearest enemy at range,
/// uses W when one gets close (Q too on Bulwark), and picks up loot when
/// the coast is clear. Input-only, like a player.
fn autopilot(world: &SimState) -> NetInput {
    let mut input = NetInput::default();
    let Some(ship) = world.ships.first() else {
        return input;
    };
    let me = ship.pos;
    let mut aim_at = |p: FxVec2, buttons: u8| {
        input.buttons |= buttons;
        input.target_x = p.x / SUB;
        input.target_y = p.y / SUB;
    };
    let nearest = world
        .enemies
        .iter()
        .min_by_key(|e| (e.pos - me).length_squared())
        .map(|e| e.pos);
    let dist = nearest.map_or(i64::MAX, |e| (e - me).length() / SUB as i64);
    if let Some(e) = nearest.filter(|_| dist <= 300) {
        let away = me - e;
        let side = FxVec2::new(-away.y, away.x).scale_to(100 * SUB);
        if dist < 140 && world.frame % 10 == 5 {
            let q = if ship.kind == sim::ShipKind::Bulwark {
                INPUT_SKILL_Q
            } else {
                0
            };
            aim_at(e, INPUT_SKILL_W | q);
        } else if world.frame.is_multiple_of(10) {
            let goal = if dist < 150 {
                me + away.scale_to(120 * SUB) + side
            } else {
                me + side
            };
            aim_at(goal, INPUT_MOVE);
        }
    } else if let Some(p) = world.pickups.first() {
        aim_at(p.pos, INPUT_MOVE);
    } else if let Some(e) = nearest {
        aim_at(e, INPUT_MOVE);
    }
    input
}

#[allow(clippy::too_many_arguments)]
fn autoplay_flow(
    state: Res<State<GameScreen>>,
    autoplay: Res<Autoplay>,
    time: Res<Time>,
    mut save: ResMut<SaveSlot>,
    mut next: ResMut<NextState<GameScreen>>,
    mut exit: MessageWriter<AppExit>,
    mut since: Local<f32>,
    mut last: Local<Option<GameScreen>>,
) {
    if *last != Some(*state.get()) {
        *last = Some(*state.get());
        *since = 0.0;
    }
    *since += time.delta_secs();
    match state.get() {
        GameScreen::Title => {
            save.game = Some(SaveGame {
                selected_battleship: autoplay.ship.clone(),
                ..SaveGame::default()
            });
            next.set(transition(GameScreen::Title, FlowEvent::NewGame, true));
        }
        GameScreen::Carrier if *since > 0.5 => {
            next.set(transition(
                GameScreen::Carrier,
                FlowEvent::LaunchMission,
                true,
            ));
        }
        GameScreen::Result if *since > 3.0 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

fn autoplay_screenshots(
    mut commands: Commands,
    state: Res<State<GameScreen>>,
    autoplay: Res<Autoplay>,
    time: Res<Time>,
    mut timer: Local<f32>,
    mut seq: Local<u32>,
) {
    let Some(dir) = &autoplay.shots else {
        return;
    };
    if !matches!(state.get(), GameScreen::Battle | GameScreen::Result) {
        return;
    }
    *timer += time.delta_secs();
    if *timer < 2.0 {
        return;
    }
    *timer = 0.0;
    *seq += 1;
    let path = dir.join(format!("{:03}-{:?}.png", *seq, state.get()).to_lowercase());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
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
