//! First-playable scene flow and placeholder UI.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PrimaryWindow;
use sim::input::{INPUT_MOVE, INPUT_SKILL_Q, INPUT_SKILL_W};
use sim::tuning::KILL_TARGET;
use sim::{FxVec2, NetInput, SimState, SUB};

use crate::art::ContentImages;
use crate::carrier::{CarrierArrival, Overlay, Pilot};
use crate::hints::{Hint, Hints};
use crate::mission::{self, MissionOutcome, MissionRequest, MissionResult};
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
            .add_systems(Update, autoplay_flow.run_if(resource_exists::<Autoplay>));
    }
}

/// `--autoplay`: QA mode for the demo loop. Starts a new game (or, with
/// `resume`, Continues the saved one), flies `missions` battles with `ship`
/// and a scripted pilot, buys the first affordable Workshop upgrade between
/// them, returns to the Carrier after the last one and quits (saving, like
/// the pause menu's Quit). With `shots`, saves a window screenshot of every
/// scene to that directory. With `abandon`, the last mission is quit from
/// the pause menu 20 s in (Failed, loot lost). Prints `autoplay:` lines.
#[derive(Resource, Clone, Default)]
pub struct Autoplay {
    pub ship: String,
    pub shots: Option<PathBuf>,
    pub missions: u32,
    pub resume: bool,
    pub abandon: bool,
    /// Missions flown so far in this run.
    pub flown: u32,
}

impl Autoplay {
    fn shoot(&self, commands: &mut Commands, name: &str) {
        let Some(dir) = &self.shots else {
            return;
        };
        let path = dir.join(format!("{name}.png"));
        println!("autoplay: screenshot {}", path.display());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

/// Upgrade preference for the autoplay pilot: the spec's most visible first
/// buy first.
const AUTOPLAY_UPGRADES: [crate::workshop::Upgrade; 3] = [
    crate::workshop::Upgrade::WeaponTuning,
    crate::workshop::Upgrade::HullPlating,
    crate::workshop::Upgrade::ThrusterTuning,
];

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

    pub(crate) fn store(&mut self) {
        if let Some(game) = &self.game {
            match save::store(&self.path, game) {
                Ok(()) => self.status = "Saved".to_string(),
                Err(e) => self.status = format!("Save failed: {e}"),
            }
        }
    }
}

#[cfg(test)]
impl SaveSlot {
    /// A slot holding `game`, stored to a scratch file.
    pub(crate) fn scratch(name: &str, game: SaveGame) -> Self {
        Self {
            path: std::env::temp_dir()
                .join(format!("oni-spacewar-{name}-{}", std::process::id()))
                .join("save.json"),
            game: Some(game),
            status: String::new(),
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

/// Full-window image behind a screen, cropped to keep its aspect ratio.
fn backdrop(commands: &mut Commands, image: Handle<Image>, aspect: f32, tint: Color) {
    commands
        .spawn((
            ScreenEntity,
            ZIndex(-1),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .with_child((
            ImageNode::new(image).with_color(tint),
            Node {
                height: percent(100),
                aspect_ratio: Some(aspect),
                flex_shrink: 0.0,
                ..default()
            },
        ));
}

fn art_backdrop(
    commands: &mut Commands,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    id: &str,
    tint: Color,
) {
    if let Some(image) = art.get(images, id) {
        let px = ContentImages::size(images, &image);
        backdrop(commands, image, px.x / px.y, tint);
    }
}

fn enter_title(
    mut commands: Commands,
    save: Res<SaveSlot>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
) {
    art_backdrop(
        &mut commands,
        &mut art,
        &mut images,
        crate::art::ids::TITLE_KEY_ART,
        Color::WHITE,
    );
    panel(
        &mut commands,
        "Oni Spacewar",
        format!("First Playable Demo\n{}", save.status),
    );
    button(
        &mut commands,
        DemoButton::NewGame,
        "New Game  [N]",
        170.0,
        true,
    );
    button(
        &mut commands,
        DemoButton::Continue,
        "Continue  [C]",
        222.0,
        save.game.is_some(),
    );
    button(
        &mut commands,
        DemoButton::QuitGame,
        "Quit  [Q]",
        274.0,
        true,
    );
}

/// Books the returning mission into the save. The deck itself is spawned by
/// `carrier::CarrierPlugin` after this runs.
pub(crate) fn enter_carrier(
    mut save: ResMut<SaveSlot>,
    mut pending_result: ResMut<LastMissionResult>,
    mut arrival: ResMut<CarrierArrival>,
    mut hints: ResMut<Hints>,
) {
    save.game.get_or_insert_with(SaveGame::default);
    hints.trigger(&mut save, Hint::CarrierWalk);
    if let Some(result) = pending_result.0.take() {
        arrival.from_mission = true;
        if result.credits > 0 || !result.resources.is_empty() {
            hints.trigger(&mut save, Hint::ReturnWithLoot);
        }
        if let Some(game) = &mut save.game {
            game.record_mission_return(result.outcome.success(), result.credits, &result.resources);
            save.store();
        }
    }
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
        Text::new("Left click move    Q / W skills    Esc pause"),
        text_style(16.0, Color::srgb(0.7, 0.75, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            bottom: px(10),
            ..default()
        },
    ));
}

/// `m:ss` from sim ticks (60 per second).
pub fn mission_time(ticks: u32) -> String {
    let secs = ticks / sim::tuning::TICKS_PER_SEC as u32;
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// The Result scene's body text (below the banner).
pub fn result_text(r: &MissionResult, wallet: (u32, u32)) -> String {
    let crystal = r
        .resources
        .get(mission::VOID_CRYSTAL_ID)
        .copied()
        .unwrap_or(0);
    let mut lines = vec![
        format!(
            "Kills {}/{KILL_TARGET}    Time {}    Wave reached {}",
            r.kills,
            mission_time(r.ticks),
            r.wave
        ),
        format!(
            "Collected     {} credits    {} Void Crystal",
            r.collected.credits, r.collected.void_crystal
        ),
    ];
    if r.outcome.success() {
        lines.push(format!(
            "Success bonus {} credits    {} Void Crystal",
            r.bonus.credits, r.bonus.void_crystal
        ));
    }
    lines.push(format!(
        "Kept          {} credits    {crystal} Void Crystal",
        r.credits
    ));
    if r.lost != sim::Loot::default() {
        lines.push(format!(
            "Lost          {} credits    {} Void Crystal  (rewards don't survive failure)",
            r.lost.credits, r.lost.void_crystal
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "Credits {} -> {}    Void Crystal {} -> {}",
        wallet.0,
        wallet.0 + r.credits,
        wallet.1,
        wallet.1 + crystal
    ));
    lines.join("\n")
}

fn enter_result(
    mut commands: Commands,
    result: Res<LastMissionResult>,
    save: Res<SaveSlot>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
) {
    art_backdrop(
        &mut commands,
        &mut art,
        &mut images,
        "core.carrier.interior",
        Color::srgb(0.45, 0.45, 0.5),
    );
    let wallet = save
        .game
        .as_ref()
        .map(|g| (g.credits, crate::workshop::void_crystal(g)))
        .unwrap_or_default();
    let (banner, color, body) = match &result.0 {
        Some(r) => {
            let (banner, color) = match r.outcome {
                MissionOutcome::Victory => ("MISSION SUCCESS", Color::srgb(0.45, 0.95, 0.55)),
                MissionOutcome::Defeat => ("MISSION FAILED", Color::srgb(1.0, 0.4, 0.35)),
                MissionOutcome::Abandoned => {
                    ("MISSION FAILED - abandoned", Color::srgb(1.0, 0.4, 0.35))
                }
            };
            (banner, color, result_text(r, wallet))
        }
        None => ("No result", Color::WHITE, String::new()),
    };
    commands
        .spawn((
            ScreenEntity,
            BackgroundColor(Color::srgba(0.03, 0.05, 0.08, 0.85)),
            BorderColor::all(color),
            Node {
                position_type: PositionType::Absolute,
                left: px(60),
                right: px(60),
                top: px(40),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(20)),
                row_gap: px(14),
                border: UiRect::all(px(2)),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((Text::new(banner), text_style(40.0, color)));
            p.spawn((
                Text::new(body),
                text_style(20.0, Color::srgb(0.88, 0.9, 0.94)),
            ));
            p.spawn((
                Text::new("[Enter] Continue - rewards are saved and you return to the Carrier"),
                text_style(15.0, Color::srgb(0.6, 0.7, 0.75)),
            ));
        });
    button(
        &mut commands,
        DemoButton::ReturnCarrier,
        "Continue",
        470.0,
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
            let result = mission::result_from_sim(&world, outcome);
            if autoplay.is_some() {
                let ship = &world.ships[0];
                println!(
                    "autoplay: mission {:?} kills {} time {} hull {}/{} kept {} cr {} VC",
                    result.outcome,
                    result.kills,
                    mission_time(result.ticks),
                    ship.hull.max(0),
                    ship.stats.max_hull,
                    result.credits,
                    result
                        .resources
                        .get(mission::VOID_CRYSTAL_ID)
                        .copied()
                        .unwrap_or(0),
                );
            }
            pending_result.0 = Some(result);
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

/// Scripted scene steps for `--autoplay`. Each step fires once, when the
/// scene has been up for its time in seconds.
#[allow(clippy::too_many_arguments)]
fn autoplay_flow(
    mut commands: Commands,
    state: Res<State<GameScreen>>,
    mut autoplay: ResMut<Autoplay>,
    time: Res<Time>,
    mut save: ResMut<SaveSlot>,
    mut next: ResMut<NextState<GameScreen>>,
    mut overlay: ResMut<Overlay>,
    mut pilot: Query<&mut Transform, With<Pilot>>,
    mut launch: MessageWriter<MissionRequest>,
    mut exit: MessageWriter<AppExit>,
    world: Option<Res<SimWorld>>,
    mut pending_result: ResMut<LastMissionResult>,
    mut since: Local<f32>,
    mut last: Local<Option<GameScreen>>,
) {
    if *last != Some(*state.get()) {
        *last = Some(*state.get());
        *since = 0.0;
    }
    let before = *since;
    *since += time.delta_secs();
    let at = |t: f32| before < t && *since >= t;
    let n = autoplay.flown;
    match state.get() {
        GameScreen::Title => {
            if at(1.0) {
                autoplay.shoot(
                    &mut commands,
                    if autoplay.resume {
                        "01-title-continue"
                    } else {
                        "01-title"
                    },
                );
            }
            if at(1.5) {
                if autoplay.resume {
                    println!("autoplay: continue ({})", save.status);
                    next.set(transition(
                        GameScreen::Title,
                        FlowEvent::Continue,
                        save.game.is_some(),
                    ));
                } else {
                    save.game = Some(SaveGame {
                        selected_battleship: autoplay.ship.clone(),
                        ..SaveGame::default()
                    });
                    save.store();
                    println!("autoplay: new game");
                    next.set(transition(GameScreen::Title, FlowEvent::NewGame, true));
                }
            }
        }
        GameScreen::Carrier => {
            let tag = if autoplay.resume { "continue" } else { "run" };
            if at(1.0) {
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}-carrier-{tag}-after-{n}", 10 + n * 10),
                );
                if let Some(g) = &save.game {
                    println!(
                        "autoplay: carrier credits {} VC {} upgrades {:?} missions {} won {} last {:?} hints {}",
                        g.credits,
                        crate::workshop::void_crystal(g),
                        g.purchased_upgrades,
                        g.mission_count,
                        g.missions_won,
                        g.last_result,
                        g.tutorial_seen.len()
                    );
                }
            }
            if n >= autoplay.missions {
                if at(2.0) {
                    save.store();
                    println!("autoplay: quit");
                    exit.write(AppExit::Success);
                }
                return;
            }
            if n > 0 && at(1.5) {
                *overlay = Overlay::Workshop {
                    index: 0,
                    message: String::new(),
                };
            }
            if n > 0 && at(2.5) {
                if let Some(game) = &mut save.game {
                    let bought = AUTOPLAY_UPGRADES
                        .iter()
                        .find_map(|&u| crate::workshop::buy(game, u).ok().map(|l| (u, l)));
                    let message = match bought {
                        Some((u, level)) => format!("Bought {} {level}.", u.name()),
                        None => "Nothing affordable.".to_string(),
                    };
                    println!(
                        "autoplay: workshop: {message} credits {} VC {}",
                        game.credits,
                        crate::workshop::void_crystal(game)
                    );
                    let index = bought
                        .and_then(|(u, _)| crate::workshop::UPGRADES.iter().position(|&x| x == u))
                        .unwrap_or(0);
                    *overlay = Overlay::Workshop { index, message };
                }
                save.store();
            }
            if n > 0 && at(3.5) {
                autoplay.shoot(&mut commands, &format!("{:02}-workshop", 12 + n * 10));
            }
            if at(4.0) {
                // Walk-free: put the Pilot at the launch console.
                if let Ok(mut tf) = pilot.single_mut() {
                    tf.translation.x = crate::carrier::LAUNCH_CONSOLE_X;
                }
                *overlay = Overlay::None;
            }
            if at(5.0) {
                autoplay.shoot(&mut commands, &format!("{:02}-dock-console", 13 + n * 10));
            }
            if at(5.5) {
                *overlay = Overlay::ShipSelect {
                    index: crate::carrier::ship_index(&autoplay.ship),
                };
            }
            if at(6.0) {
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}-dock-ship-select", 14 + n * 10),
                );
            }
            if at(6.5) {
                launch.write(MissionRequest {
                    battleship_id: autoplay.ship.clone(),
                });
            }
        }
        GameScreen::Battle => {
            for (i, t) in [6.0, 20.0, 35.0].into_iter().enumerate() {
                if at(t) {
                    autoplay.shoot(
                        &mut commands,
                        &format!("{:02}-battle-{}-{i}", 15 + n * 10, n + 1),
                    );
                }
            }
            if autoplay.abandon && n + 1 == autoplay.missions && at(20.5) {
                if let Some(world) = world {
                    let r = mission::result_from_sim(&world, MissionOutcome::Abandoned);
                    println!(
                        "autoplay: quit mission (abandoned) kills {} lost {} cr {} VC",
                        r.kills, r.lost.credits, r.lost.void_crystal
                    );
                    pending_result.0 = Some(r);
                }
                next.set(transition(
                    GameScreen::Battle,
                    FlowEvent::FinishMission,
                    true,
                ));
            }
        }
        GameScreen::Result => {
            if at(1.5) {
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}-result-{}", 19 + n * 10, n + 1),
                );
            }
            if at(3.0) {
                autoplay.flown += 1;
                next.set(transition(
                    GameScreen::Result,
                    FlowEvent::ReturnToCarrier,
                    true,
                ));
            }
        }
    }
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

    /// Flies one mission with the `--autoplay` pilot, headless.
    fn autoplayed(ship: &str, upgrades: &[&str], seed: u64) -> (MissionResult, i32) {
        let config = mission::MissionConfig {
            mission_type: mission::MissionType::Elimination,
            ship_loadouts: vec![mission::ShipLoadout {
                player_handle: 0,
                battleship_id: ship.to_string(),
                upgrade_ids: upgrades.iter().map(|s| s.to_string()).collect(),
            }],
            seed,
        };
        let mut world = mission::launch_sim(&config);
        for _ in 0..60 * 60 * 5 {
            let input = autopilot(&world);
            world.step(&[input]);
            if let Some(outcome) = mission::sim_outcome(&world) {
                return (
                    mission::result_from_sim(&world, outcome),
                    world.ships[0].hull,
                );
            }
        }
        panic!("mission did not end");
    }

    /// Demo Spec § Workshop upgrades: one successful mission buys about one
    /// upgrade. Checked on the autoplay pilot's runs with both battleships.
    #[test]
    fn one_successful_mission_buys_an_upgrade() {
        for ship in [mission::KITE_ID, mission::BULWARK_ID] {
            for seed in 10_000..10_008 {
                let (r, _) = autoplayed(ship, &[], seed);
                assert!(r.outcome.success(), "{ship} {seed} lost");
                let mut save = SaveGame::default();
                save.record_mission_return(true, r.credits, &r.resources);
                assert!(
                    crate::workshop::can_afford(&save, crate::workshop::COSTS[0]),
                    "{ship} {seed}: {} cr {:?}",
                    r.credits,
                    r.resources
                );
            }
        }
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
