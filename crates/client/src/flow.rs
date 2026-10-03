//! First playable scene shell: Title -> Carrier -> Battle -> Result.

use bevy::app::AppExit;
use bevy::prelude::*;
use sim::input::{INPUT_MOVE, INPUT_SKILL_Q};
use sim::{MissionStatus, NetInput, SimParams, SimState};

use crate::rollback::SimWorld;
use crate::save::{self, SaveData, SaveSlot};

const BUTTON_NORMAL: Color = Color::srgb(0.12, 0.14, 0.18);
const BUTTON_HOVERED: Color = Color::srgb(0.22, 0.25, 0.32);
const BUTTON_DISABLED: Color = Color::srgb(0.07, 0.08, 0.1);
const TEXT: Color = Color::srgb(0.9, 0.94, 0.96);
const MUTED: Color = Color::srgb(0.55, 0.62, 0.68);

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameState {
    #[default]
    Title,
    Carrier,
    Battle,
    Result,
}

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum PauseState {
    #[default]
    Running,
    Paused,
}

#[derive(Resource, Clone, Debug)]
pub struct MissionRequest {
    pub ship_id: String,
    pub mission_id: String,
}

impl Default for MissionRequest {
    fn default() -> Self {
        Self {
            ship_id: "starter_battleship".to_string(),
            mission_id: "elimination_placeholder".to_string(),
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct MissionOutcome {
    pub success: bool,
    pub kills: u32,
    pub loot: u32,
}

#[derive(Resource, Default)]
struct SaveError(Option<String>);

#[derive(Resource, Default)]
struct LastCursor(IVec2);

#[derive(Resource, Default)]
struct BattleTick {
    accumulator: f32,
}

#[derive(Component)]
struct SceneRoot;

#[derive(Component)]
struct PauseRoot;

#[derive(Component)]
struct UiButton(Action);

#[derive(Clone, Copy, Debug)]
enum Action {
    NewGame,
    Continue,
    Quit,
    LaunchBattle,
    ResultContinue,
    Resume,
    ReturnToTitle,
}

pub struct DemoFlowPlugin;

impl Plugin for DemoFlowPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SaveSlot::new())
            .init_resource::<SaveError>()
            .init_resource::<LastCursor>()
            .init_state::<PauseState>()
            .add_plugins((
                TitlePlugin,
                CarrierPlugin,
                BattlePlugin,
                ResultPlugin,
                PausePlugin,
            ))
            .add_systems(Update, button_visuals)
            .add_systems(Last, save_on_exit);
    }
}

struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Title), setup_title)
            .add_systems(Update, title_keys.run_if(in_state(GameState::Title)))
            .add_systems(OnExit(GameState::Title), teardown_scene);
    }
}

struct CarrierPlugin;

impl Plugin for CarrierPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Carrier), setup_carrier)
            .add_systems(
                Update,
                carrier_keys
                    .run_if(in_state(GameState::Carrier))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(OnExit(GameState::Carrier), teardown_scene);
    }
}

struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Battle), setup_battle)
            .add_systems(
                Update,
                (track_cursor, step_local_battle, battle_keys)
                    .chain()
                    .run_if(in_state(GameState::Battle))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(OnExit(GameState::Battle), cleanup_battle);
    }
}

struct ResultPlugin;

impl Plugin for ResultPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Result), setup_result)
            .add_systems(Update, result_keys.run_if(in_state(GameState::Result)))
            .add_systems(OnExit(GameState::Result), teardown_scene);
    }
}

struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, pause_toggle)
            .add_systems(OnEnter(PauseState::Paused), setup_pause)
            .add_systems(Update, pause_keys.run_if(in_state(PauseState::Paused)))
            .add_systems(OnExit(PauseState::Paused), teardown_pause);
    }
}

fn panel(commands: &mut Commands, title: &str, lines: &[String], buttons: &[(Action, &str, bool)]) {
    commands
        .spawn((
            SceneRoot,
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(14),
                ..default()
            },
            BackgroundColor(Color::srgb(0.03, 0.04, 0.06)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(42.0),
                    ..default()
                },
                TextColor(TEXT),
            ));
            for line in lines {
                parent.spawn((
                    Text::new(line.clone()),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(MUTED),
                ));
            }
            for &(action, label, enabled) in buttons {
                parent
                    .spawn((
                        UiButton(action),
                        Button,
                        Node {
                            width: px(260),
                            height: px(46),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(if enabled {
                            BUTTON_NORMAL
                        } else {
                            BUTTON_DISABLED
                        }),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(if enabled { TEXT } else { MUTED }),
                        ));
                    });
            }
        });
}

fn setup_title(mut commands: Commands, slot: Res<SaveSlot>, error: Res<SaveError>) {
    let mut lines = vec!["First playable shell".to_string()];
    if let Some(error) = &error.0 {
        lines.push(format!("Save error: {error}"));
    }
    let mut buttons = vec![(Action::NewGame, "New Game", true)];
    if slot.exists() {
        buttons.push((Action::Continue, "Continue", true));
    }
    buttons.push((Action::Quit, "Quit", true));
    panel(&mut commands, "Oni Spacewar", &lines, &buttons);
}

fn title_keys(
    keys: Res<ButtonInput<KeyCode>>,
    slot: Res<SaveSlot>,
    mut save_data: Option<ResMut<SaveData>>,
    mut error: ResMut<SaveError>,
    mut next: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::KeyN) {
        if let Some(mut save_data) = save_data.take() {
            *save_data = SaveData::default();
        }
        next.set(GameState::Carrier);
    } else if keys.just_pressed(KeyCode::KeyC) && slot.exists() {
        match save::load(&slot) {
            Ok(save) => {
                if let Some(mut save_data) = save_data.take() {
                    *save_data = save;
                }
                error.0 = None;
                next.set(GameState::Carrier);
            }
            Err(e) => error.0 = Some(e),
        }
    } else if keys.just_pressed(KeyCode::KeyQ) {
        exit.write(AppExit::Success);
    }
}

fn setup_carrier(mut commands: Commands, save: Res<SaveData>) {
    let alloy = save.resources.get("alloy").copied().unwrap_or(0);
    panel(
        &mut commands,
        "Carrier",
        &[
            format!("Credits: {}  Alloy: {}", save.credits, alloy),
            "Selected ship: starter_battleship".to_string(),
        ],
        &[
            (Action::LaunchBattle, "Launch Battle", true),
            (Action::ReturnToTitle, "Title", true),
            (Action::Quit, "Quit", true),
        ],
    );
}

fn carrier_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut next: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        commands.insert_resource(MissionRequest::default());
        next.set(GameState::Battle);
    } else if keys.just_pressed(KeyCode::KeyT) {
        next.set(GameState::Title);
    } else if keys.just_pressed(KeyCode::KeyQ) {
        exit.write(AppExit::Success);
    }
}

fn setup_battle(mut commands: Commands, request: Option<Res<MissionRequest>>) {
    let request = request.as_deref().cloned().unwrap_or_default();
    let label = format!(
        "Battle: {} / {} | left click to move, Q to fire, Esc to pause",
        request.ship_id, request.mission_id
    );
    commands.insert_resource(request);
    commands.insert_resource(SimWorld(SimState::new(SimParams {
        num_players: 1,
        seed: 42,
    })));
    commands.insert_resource(BattleTick::default());
    commands.spawn((
        SceneRoot,
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(TEXT),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            bottom: px(12),
            ..default()
        },
    ));
}

fn track_cursor(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut cursor: ResMut<LastCursor>,
) {
    if let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), cameras.single()) {
        if let Some(p) = window
            .cursor_position()
            .and_then(|c| camera.viewport_to_world_2d(cam_tf, c).ok())
        {
            cursor.0 = p.round().as_ivec2();
        }
    }
}

fn step_local_battle(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Res<LastCursor>,
    mut tick: ResMut<BattleTick>,
    mut world: ResMut<SimWorld>,
    mut commands: Commands,
    mut next: ResMut<NextState<GameState>>,
) {
    tick.accumulator += time.delta_secs();
    let input = local_input(&keys, &mouse, cursor.0);
    while tick.accumulator >= 1.0 / 60.0 {
        world.step(&[input]);
        tick.accumulator -= 1.0 / 60.0;
        match world.outcome {
            MissionStatus::Running => {}
            MissionStatus::Won | MissionStatus::Lost => {
                let success = world.outcome == MissionStatus::Won;
                let kills = world.kills;
                commands.insert_resource(MissionOutcome {
                    success,
                    kills,
                    loot: if success { 50 + kills * 5 } else { kills * 2 },
                });
                next.set(GameState::Result);
                return;
            }
        }
    }
}

fn local_input(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    cursor: IVec2,
) -> NetInput {
    let mut buttons = 0;
    if mouse.pressed(MouseButton::Left) {
        buttons |= INPUT_MOVE;
    }
    if keys.pressed(KeyCode::KeyQ) {
        buttons |= INPUT_SKILL_Q;
    }
    NetInput {
        buttons,
        target_x: cursor.x,
        target_y: cursor.y,
    }
}

fn battle_keys(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::KeyF) {
        next.set(GameState::Result);
    }
}

fn cleanup_battle(
    mut commands: Commands,
    roots: Query<Entity, With<SceneRoot>>,
    children: Query<&Children>,
    world: Option<Res<SimWorld>>,
) {
    teardown_roots(&mut commands, &roots, &children);
    if world.is_some() {
        commands.remove_resource::<SimWorld>();
    }
    commands.remove_resource::<BattleTick>();
}

fn setup_result(mut commands: Commands, outcome: Option<Res<MissionOutcome>>) {
    let outcome = outcome.as_deref().cloned().unwrap_or_default();
    panel(
        &mut commands,
        if outcome.success {
            "Mission Complete"
        } else {
            "Mission Failed"
        },
        &[
            format!("Kills: {}", outcome.kills),
            format!("Loot: {} credits", outcome.loot),
        ],
        &[
            (Action::ResultContinue, "Continue", true),
            (Action::ReturnToTitle, "Title", true),
            (Action::Quit, "Quit", true),
        ],
    );
}

fn result_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut save_data: ResMut<SaveData>,
    outcome: Option<Res<MissionOutcome>>,
    slot: Res<SaveSlot>,
    mut error: ResMut<SaveError>,
    mut next: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        if let Some(outcome) = outcome {
            apply_outcome(&mut save_data, &outcome);
        }
        match save::store(&slot, &save_data) {
            Ok(()) => error.0 = None,
            Err(e) => error.0 = Some(e),
        }
        next.set(GameState::Carrier);
    }
}

fn pause_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    pause: Option<Res<State<PauseState>>>,
    mut next: ResMut<NextState<PauseState>>,
) {
    if !matches!(state.get(), GameState::Carrier | GameState::Battle) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if pause.is_some_and(|p| *p.get() == PauseState::Paused) {
            next.set(PauseState::Running);
        } else {
            next.set(PauseState::Paused);
        }
    }
}

fn setup_pause(mut commands: Commands) {
    commands
        .spawn((
            PauseRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Paused"),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(TEXT),
            ));
            for &(action, label) in &[
                (Action::Resume, "Resume"),
                (Action::ReturnToTitle, "Title"),
                (Action::Quit, "Quit"),
            ] {
                parent
                    .spawn((
                        UiButton(action),
                        Button,
                        Node {
                            width: px(220),
                            height: px(42),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(BUTTON_NORMAL),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(17.0),
                                ..default()
                            },
                            TextColor(TEXT),
                        ));
                    });
            }
        });
}

fn pause_keys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut commands: Commands,
    mut pause: ResMut<NextState<PauseState>>,
    mut next: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        pause.set(PauseState::Running);
    } else if keys.just_pressed(KeyCode::KeyT) {
        if *state.get() == GameState::Battle {
            commands.insert_resource(MissionOutcome::default());
        }
        pause.set(PauseState::Running);
        next.set(GameState::Title);
    } else if keys.just_pressed(KeyCode::KeyQ) {
        exit.write(AppExit::Success);
    }
}

fn button_visuals(
    mut interaction: Query<(&Interaction, &mut BackgroundColor, &UiButton), Changed<Interaction>>,
    mut commands: Commands,
    mut save_data: Option<ResMut<SaveData>>,
    slot: Res<SaveSlot>,
    state: Res<State<GameState>>,
    mut pause: ResMut<NextState<PauseState>>,
    mut next: ResMut<NextState<GameState>>,
    mut error: ResMut<SaveError>,
    outcome: Option<Res<MissionOutcome>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, mut color, button) in &mut interaction {
        *color = match *interaction {
            Interaction::Hovered => BUTTON_HOVERED.into(),
            _ => BUTTON_NORMAL.into(),
        };
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.0 {
            Action::NewGame => {
                if let Some(mut save_data) = save_data.take() {
                    *save_data = SaveData::default();
                }
                next.set(GameState::Carrier);
            }
            Action::Continue => match save::load(&slot) {
                Ok(save) => {
                    if let Some(mut save_data) = save_data.take() {
                        *save_data = save;
                    }
                    error.0 = None;
                    next.set(GameState::Carrier);
                }
                Err(e) => error.0 = Some(e),
            },
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            Action::LaunchBattle => {
                commands.insert_resource(MissionRequest::default());
                next.set(GameState::Battle);
            }
            Action::ResultContinue => {
                if let (Some(mut save_data), Some(outcome)) = (save_data.take(), outcome.as_deref())
                {
                    apply_outcome(&mut save_data, outcome);
                    match save::store(&slot, &save_data) {
                        Ok(()) => error.0 = None,
                        Err(e) => error.0 = Some(e),
                    }
                }
                next.set(GameState::Carrier);
            }
            Action::Resume => pause.set(PauseState::Running),
            Action::ReturnToTitle => {
                if *state.get() == GameState::Battle {
                    commands.insert_resource(MissionOutcome::default());
                }
                pause.set(PauseState::Running);
                next.set(GameState::Title);
            }
        }
    }
}

fn apply_outcome(save: &mut SaveData, outcome: &MissionOutcome) {
    save.credits = save.credits.saturating_add(outcome.loot);
    *save.resources.entry("alloy".to_string()).or_default() += outcome.kills;
    save.tutorial_flags_seen
        .push("first_mission_result_seen".to_string());
    save.tutorial_flags_seen.sort();
    save.tutorial_flags_seen.dedup();
}

fn save_on_exit(
    mut exits: MessageReader<AppExit>,
    save_data: Option<Res<SaveData>>,
    slot: Option<Res<SaveSlot>>,
) {
    if exits.read().next().is_none() {
        return;
    }
    if let (Some(save_data), Some(slot)) = (save_data, slot) {
        if let Err(e) = save::store(&slot, &save_data) {
            error!("save on quit failed: {e}");
        }
    }
}

fn teardown_scene(
    mut commands: Commands,
    roots: Query<Entity, With<SceneRoot>>,
    children: Query<&Children>,
) {
    teardown_roots(&mut commands, &roots, &children);
}

fn teardown_pause(
    mut commands: Commands,
    roots: Query<Entity, With<PauseRoot>>,
    children: Query<&Children>,
) {
    teardown_roots(&mut commands, &roots, &children);
}

fn teardown_roots<T: Component>(
    commands: &mut Commands,
    roots: &Query<Entity, With<T>>,
    children: &Query<&Children>,
) {
    for entity in roots {
        despawn_tree(commands, entity, children);
    }
}

fn despawn_tree(commands: &mut Commands, entity: Entity, child_query: &Query<&Children>) {
    if let Ok(children) = child_query.get(entity) {
        for child in children.iter() {
            despawn_tree(commands, child, child_query);
        }
    }
    commands.entity(entity).despawn();
}
