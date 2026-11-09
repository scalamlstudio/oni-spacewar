//! First-playable scene flow and placeholder UI.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::ui::FocusPolicy;
use bevy::window::PrimaryWindow;
use sim::input::{INPUT_MOVE, INPUT_SKILL_Q, INPUT_SKILL_W};
use sim::{FxVec2, NetInput, SimState, SUB};

use crate::art::ContentImages;
use crate::carrier::{
    self, BuildCursor, CarrierArrival, CarrierScene, CarrierView, Entry, Hotspot, Overlay, Pilot,
    PilotDrag, Tool, WalkOrder,
};
use crate::codex::CodexScreen;
use crate::input::{self, MovePress};
use crate::layout::{ItemId, RoomSize};
use crate::mission::{self, MissionOutcome, MissionRequest, MissionResult};
use crate::rollback::SimWorld;
use crate::save::{self, SaveError, SaveGame};
use crate::settings::{self, SettingsScreen};

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameScreen {
    #[default]
    Title,
    Carrier,
    /// The mission transition screen (`crate::loading`).
    Loading,
    Battle,
    Result,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlowEvent {
    NewGame,
    Continue,
    LaunchMission,
    /// Every battle asset is loaded (`crate::loading`).
    AssetsLoaded,
    FinishMission,
    ReturnToCarrier,
    QuitToTitle,
}

pub fn transition(from: GameScreen, event: FlowEvent, has_save: bool) -> GameScreen {
    match (from, event) {
        (GameScreen::Title, FlowEvent::NewGame) => GameScreen::Carrier,
        (GameScreen::Title, FlowEvent::Continue) if has_save => GameScreen::Carrier,
        (GameScreen::Carrier, FlowEvent::LaunchMission) => GameScreen::Loading,
        (GameScreen::Loading, FlowEvent::AssetsLoaded) => GameScreen::Battle,
        (GameScreen::Loading, FlowEvent::ReturnToCarrier) => GameScreen::Carrier,
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
            .init_resource::<SettingsScreen>()
            .init_resource::<LastMissionResult>()
            .init_resource::<KillTarget>()
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
            .add_systems(OnExit(GameScreen::Loading), cleanup_screen)
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
/// and a scripted pilot; between them it builds the spec's example Small
/// Room at (14,10), waits for it, places a Salvage Sorter at (19,11) and
/// waits again when it can afford both, else buys the first affordable
/// Workshop upgrade; returns to the Carrier after the last one and quits (saving, like
/// the pause menu's Quit). With `shots`, saves a window screenshot of every
/// scene to that directory. With `abandon`, the last mission is quit from
/// the pause menu 20 s in (Failed, loot lost). Before each mission the
/// Pilot is sent to the ship's Dock with click-to-move and boards it on
/// arrival. After the last mission it quits to the Title, reloads the save
/// from disk and Continues once (a restart inside the same process) before
/// quitting. Prints `autoplay:` lines.
/// Kills that win each Elimination in this run: `None` in normal play (each
/// mission rolls its own from its seed, 1..=100), `AUTOPLAY_KILL_TARGET`
/// under `--autoplay`, or `--kill-target N`. Goes into the `MissionConfig`
/// at launch.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KillTarget(pub Option<u32>);

impl KillTarget {
    /// The kill target of the next mission launched from `save`.
    pub fn for_mission(self, save: &crate::save::SaveGame) -> u32 {
        mission::config_from_save(save, self.0).kill_target
    }
}

#[derive(Resource, Clone, Default)]
pub struct Autoplay {
    pub ship: String,
    pub shots: Option<PathBuf>,
    pub missions: u32,
    pub resume: bool,
    pub abandon: bool,
    /// `--clip`: the first battle also shoots a frame every 0.1 s from 7 s
    /// to 10 s (`15c-battle-clip-NN`), for a short clip of the feedback.
    pub clip: bool,
    /// Missions flown so far in this run.
    pub flown: u32,
    /// This Carrier visit builds (decided on arrival), and what: the spec's
    /// example Small Room, its Salvage Sorter.
    pub building: bool,
    pub build_plan: (bool, bool),
    /// When (scene seconds) the Pilot boarded the ship this Carrier visit.
    pub boarded: Option<f32>,
    /// When (scene seconds) the Codex opened from the Sample Analyzer on
    /// the last Carrier visit.
    pub codex_station: Option<f32>,
    /// Already quit to the Title and Continued from the reloaded save.
    pub reloaded: bool,
    /// Role telegraphs already shot this run (bit per `role_shot` index).
    pub role_shots: u16,
}

/// The first time each new role's warning is on screen, the autoplay shoots
/// it (`15d-battle-<name>`): a Lancer halfway through its wind-up, an armed
/// Bloater or a pending burst, a Warden shielding an ally, a Brood about to
/// release; then the elites (TAKOAI-117, `15e-battle-<name>`): an elite on
/// the field, a Blinking elite's mark halfway, elite loot's light beam;
/// then the Fissure Guardian (TAKOAI-118, `15f-battle-guardian-<name>`):
/// 1 s after it came out, a ring flash halfway, its phase change, a charge
/// line halfway, and its loot burst from 0.5 s after the kill.
/// Index = bit in `Autoplay::role_shots`.
fn role_shot(world: &SimState, done: u16) -> Option<(usize, &'static str)> {
    use sim::tuning::{
        BLINK_MARK, BLOATER_FUSE, BROOD_WINDUP, GUARDIAN_CHARGE_WINDUP, GUARDIAN_OUTRO,
        LANCER_WINDUP, PICKUP_LIFETIME, TICKS_PER_SEC,
    };
    use sim::{BossAttack, EnemyKind, MissionStage};
    let enemies = &world.enemies;
    let sec = TICKS_PER_SEC as u32;
    let boss = world.guardian().and_then(|g| g.boss.map(|b| (g, b)));
    let winding = |attack: BossAttack, at: u32| {
        boss.is_some_and(|(g, b)| b.attack == attack && g.windup == at)
    };
    let checks = [
        enemies
            .iter()
            .any(|e| e.kind == EnemyKind::Lancer && e.windup == LANCER_WINDUP / 2),
        world.blasts.iter().any(|b| b.fuse == BLOATER_FUSE / 3)
            || enemies.iter().any(|e| {
                e.kind == EnemyKind::Bloater && e.struck != 0 && e.windup == BLOATER_FUSE / 3
            }),
        (0..enemies.len()).any(|i| sim::state::warden_of(enemies, i).is_some()),
        enemies
            .iter()
            .any(|e| e.kind == EnemyKind::Brood && e.windup == BROOD_WINDUP / 2),
        // Once it is close enough to be well inside the view.
        enemies.iter().any(|e| {
            let near = (350 * sim::SUB) as i64;
            e.is_elite()
                && world
                    .ships
                    .iter()
                    .any(|s| (s.pos - e.pos).length_squared() <= near * near)
        }),
        enemies
            .iter()
            .any(|e| e.elite.is_some_and(|el| el.blink == BLINK_MARK / 2)),
        // 0.2 s after the drop: past the kill's hit-stop, before the ship
        // has pulled it in.
        world
            .pickups
            .iter()
            .any(|p| p.elite && p.ttl == PICKUP_LIFETIME - TICKS_PER_SEC as u32 / 5),
        world.mission.stage == MissionStage::Guardian
            && world.frame == world.mission.guardian_frame + sec,
        boss.is_some_and(|(_, b)| winding(BossAttack::Ring, b.ring().2 / 2)),
        boss.is_some_and(|(_, b)| b.phase_frame > 0 && world.frame == b.phase_frame + sec / 5),
        winding(BossAttack::Charge, GUARDIAN_CHARGE_WINDUP / 2),
        world.mission.stage == MissionStage::Outro
            && world.mission.outro <= GUARDIAN_OUTRO - sec / 2,
    ];
    let names = [
        "lancer",
        "bloater",
        "warden",
        "brood",
        "e-elite",
        "e-elite-blink",
        "e-elite-loot",
        "g-guardian",
        "g-guardian-ring",
        "g-guardian-enraged",
        "g-guardian-charge",
        "g-guardian-kill",
    ];
    (0..names.len())
        .find(|&i| checks[i] && done & (1 << i) == 0)
        .map(|i| (i, names[i]))
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

/// The Codex tabs the autoplay shoots, in order, and the entry it selects
/// in each (Spitter, Berth, Aquarium (scrolled into view), Gunner,
/// Bulwark).
const CODEX_SHOTS: [(&str, usize); 5] = [
    ("enemies", 1),
    ("stations", 2),
    ("furniture", 11),
    ("characters", 1),
    ("battleships", 1),
];

/// First Carrier visit: when the autoplay's 8-direction Pilot tour starts,
/// and the time per walk (the longest, a 209 px diagonal, takes 1.3 s).
const TOUR_START: f32 = 5.6;
const TOUR_LEG: f32 = 1.4;
/// Then the hold-to-move drag: pressed after the tour, released once the
/// cursor has swept all its legs.
const DRAG_START: f32 = TOUR_START + carrier::PILOT_TOUR.len() as f32 * TOUR_LEG + 0.2;
const DRAG_END: f32 =
    DRAG_START + (carrier::PILOT_DRAG.len() - 1) as f32 * carrier::PILOT_DRAG_LEG + 0.3;

/// The spec's example edits (§ Starting layout): a Small Room anchored at
/// (14,10), and the Salvage Sorter at (19,11), which is also its Build-mode
/// mouse tile (a 2 × 2 footprint's middle rounds to its north-west tile).
const EXAMPLE_ROOM: (i32, i32) = (14, 10);
const EXAMPLE_SORTER_MOUSE: (i32, i32) = (19, 11);
/// Art batch B pieces the autoplay adds to the example room with the
/// sorter, so its screenshots show them: a sofa against the north wall,
/// a wall screen over it, a rug in front and a cat tree in the south-east
/// corner, clear of both doorways and the sorter's front strip. Each tile
/// is the piece's mouse tile, which for these footprints is its anchor.
const EXAMPLE_FURNISHING: [(ItemId, (i32, i32)); 4] = [
    (ItemId::Sofa, (15, 11)),
    (ItemId::WallScreen, (15, 10)),
    (ItemId::Rug, (15, 12)),
    (ItemId::CatTree, (20, 14)),
];

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

    /// Reads the save back from disk, as a restart would.
    pub(crate) fn reload(&mut self) {
        *self = Self::load();
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

impl PauseMenu {
    /// Opens or closes the pause, redrawing the menu (hidden while the
    /// Settings screen holds the pause).
    pub(crate) fn hold(&mut self, open: bool) {
        self.open = open;
        self.dirty = true;
    }
}

#[derive(Resource, Default)]
pub(crate) struct LastMissionResult(Option<MissionResult>);

#[derive(Component)]
pub(crate) struct ScreenEntity;

#[derive(Component)]
struct PauseEntity;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum DemoButton {
    NewGame,
    Continue,
    QuitGame,
    QuitMission,
    ReturnCarrier,
    Resume,
    QuitToTitle,
    /// The Carrier pause menu's Settings (S).
    Settings,
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

/// The game name's height as a share of the window height, so it keeps the
/// same place on the key art at every window size (the art is scaled to the
/// window height).
const TITLE_VH: f32 = 9.0;
/// Outline thickness around the game name, also in window-height percent.
const TITLE_OUTLINE_VH: f32 = 0.35;
const TITLE_SUBTITLE_VH: f32 = 3.0;

/// "ONI SPACEWAR" in the display font, drawn in the empty upper right of the
/// text-free key art: eight dark copies around it make an outline, and a drop
/// shadow lifts it off the nebula. Below it, the build name and save status.
fn title_logo(commands: &mut Commands, font: Option<Handle<Font>>, status: &str) {
    let name_font = |color: Color| {
        (
            TextFont {
                font: font.clone().map(Into::into).unwrap_or_default(),
                font_size: FontSize::Vh(TITLE_VH),
                ..default()
            },
            TextColor(color),
        )
    };
    let outline = Color::srgb(0.04, 0.02, 0.1);
    let o = TITLE_OUTLINE_VH;
    commands
        .spawn((
            ScreenEntity,
            Node {
                position_type: PositionType::Absolute,
                right: vw(4),
                top: vh(5),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: vh(1),
                ..default()
            },
        ))
        .with_children(|logo| {
            logo.spawn(Node::default()).with_children(|name| {
                for (dx, dy) in [
                    (-o, -o),
                    (0.0, -o),
                    (o, -o),
                    (-o, 0.0),
                    (o, 0.0),
                    (-o, o),
                    (0.0, o),
                    (o, o),
                ] {
                    name.spawn((
                        Text::new(TITLE_TEXT),
                        name_font(outline),
                        Node {
                            position_type: PositionType::Absolute,
                            left: vh(dx),
                            top: vh(dy),
                            ..default()
                        },
                    ));
                }
                name.spawn((
                    Text::new(TITLE_TEXT),
                    name_font(Color::srgb(1.0, 0.86, 0.55)),
                    TextShadow {
                        offset: Vec2::new(0.0, 6.0),
                        color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                    },
                ));
            });
            logo.spawn((
                Text::new(format!("First Playable Demo\n{status}")),
                TextFont {
                    font_size: FontSize::Vh(TITLE_SUBTITLE_VH),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.92, 0.95)),
                TextLayout::justify(Justify::Right),
                TextShadow {
                    offset: Vec2::splat(2.0),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.85),
                },
            ));
        });
}

const TITLE_TEXT: &str = "ONI SPACEWAR";

fn enter_title(
    mut commands: Commands,
    save: Res<SaveSlot>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
) {
    art_backdrop(
        &mut commands,
        &mut art,
        &mut images,
        crate::art::ids::TITLE_KEY_ART,
        Color::WHITE,
    );
    let font = art.font(&mut fonts, crate::art::ids::FONT_DISPLAY);
    title_logo(&mut commands, font, &save.status);
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
    commands.spawn((
        ScreenEntity,
        settings::gear_button(&mut images, gear_node()),
    ));
}

/// Where the Title's gear (Settings) button sits: the window's
/// bottom-right corner, clear of the logo.
fn gear_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        right: px(16),
        bottom: px(16),
        width: px(36),
        height: px(36),
        padding: UiRect::all(px(4)),
        border: UiRect::all(px(1)),
        ..default()
    }
}

/// Books the returning mission into the save. The deck itself is spawned by
/// `carrier::CarrierPlugin` after this runs.
pub(crate) fn enter_carrier(
    mut save: ResMut<SaveSlot>,
    mut pending_result: ResMut<LastMissionResult>,
    mut arrival: ResMut<CarrierArrival>,
) {
    save.game.get_or_insert_with(SaveGame::default);
    if let Some(result) = pending_result.0.take() {
        arrival.from_mission = true;
        if let Some(game) = &mut save.game {
            game.record_mission_return(result.outcome.success(), result.credits, &result.resources);
            save.store();
        }
    }
}

/// Launch from the Dock: remember the picked battleship and enter the
/// transition screen, which loads the battle's assets.
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

fn enter_battle(
    mut commands: Commands,
    save: Res<SaveSlot>,
    kill_target: Res<KillTarget>,
    mut events: ResMut<crate::fx::BattleEvents>,
) {
    // A new battle: frame numbers and entity IDs start over.
    events.reset();
    let game = save.game.clone().unwrap_or_default();
    let config = mission::config_from_save(&game, kill_target.0);
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

/// The Result scene's body text (below the banner). `salvage` is the
/// Salvage Sorter's extra credits and Void Crystal (zero without a finished
/// one or on failure).
pub fn result_text(r: &MissionResult, wallet: (u32, u32), salvage: (u32, u32)) -> String {
    let crystal = r
        .resources
        .get(mission::VOID_CRYSTAL_ID)
        .copied()
        .unwrap_or(0);
    let mut lines = vec![
        format!(
            "Kills {}/{}    Time {}",
            r.kills,
            r.kill_target,
            mission_time(r.ticks)
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
    if salvage != (0, 0) {
        lines.push(format!(
            "Salvage Sorter +{} cr +{} VC",
            salvage.0, salvage.1
        ));
    }
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
        wallet.0 + r.credits + salvage.0,
        wallet.1,
        wallet.1 + crystal + salvage.1
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
        crate::art::ids::RESULT_BACKDROP,
        Color::srgb(0.8, 0.8, 0.85),
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
            let salvage = save
                .game
                .as_ref()
                .map(|g| {
                    let (credits, res) =
                        g.salvage_bonus(r.outcome.success(), r.credits, &r.resources);
                    (
                        credits,
                        res.get(mission::VOID_CRYSTAL_ID).copied().unwrap_or(0),
                    )
                })
                .unwrap_or_default();
            (banner, color, result_text(r, wallet, salvage))
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
    mut settings: ResMut<SettingsScreen>,
) {
    if settings.open {
        return;
    }
    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            DemoButton::Settings => {
                settings.open(&mut pause, true);
                continue;
            }
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
    settings: Res<SettingsScreen>,
) {
    if settings.open {
        return;
    }
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
        DemoButton::Continue | DemoButton::Settings => {}
    }
}

pub(crate) fn toggle_pause_menu(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameScreen>>,
    overlay: Res<Overlay>,
    mut pause: ResMut<PauseMenu>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // On the Carrier, Esc first backs out of an open panel or Build mode
    // (`carrier::carrier_input`, which runs after this).
    if *state.get() == GameScreen::Carrier && !pause.open && *overlay != Overlay::None {
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
    mut settings: ResMut<SettingsScreen>,
    codex: Res<CodexScreen>,
) {
    if !pause.open || settings.open || codex.open {
        return;
    }
    if keys.just_pressed(KeyCode::KeyS)
        && matches!(state.get(), GameScreen::Carrier | GameScreen::Battle)
    {
        settings.open(&mut pause, true);
    } else if keys.just_pressed(KeyCode::KeyM) && *state.get() == GameScreen::Battle {
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
    settings: Res<SettingsScreen>,
    codex: Res<CodexScreen>,
    entities: Query<Entity, With<PauseEntity>>,
) {
    if !pause.dirty {
        return;
    }
    pause.dirty = false;
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    // The Settings screen, or the Codex opened from the Sample Analyzer,
    // holds the pause without showing the menu.
    if !pause.open || settings.open || codex.open {
        return;
    }
    let in_battle = *state.get() == GameScreen::Battle;
    // One centred panel: the title, then a button per action, each showing
    // its hotkey (`apply_pause_actions`).
    let mut buttons = vec![
        (DemoButton::Resume, "Resume [R]"),
        (DemoButton::Settings, "Settings [S]"),
        (DemoButton::QuitToTitle, "Quit to Title [T]"),
        (DemoButton::QuitGame, "Quit Game [Q]"),
    ];
    if in_battle {
        buttons.push((
            DemoButton::QuitMission,
            "Quit Mission [M] (counts as Failed)",
        ));
    }
    commands
        .spawn((
            PauseEntity,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            FocusPolicy::Block,
            GlobalZIndex(40),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(10),
                    padding: UiRect::all(px(18)),
                    max_width: percent(90),
                    max_height: percent(90),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(settings::PANEL_BG),
                BorderColor::all(settings::PANEL_BORDER),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Paused"),
                    text_style(26.0, Color::WHITE),
                    Node {
                        align_self: AlignSelf::Center,
                        margin: UiRect::bottom(px(4)),
                        ..default()
                    },
                ));
                for (action, label) in buttons {
                    pause_button(panel, action, label);
                }
            });
        });
}

fn pause_button(panel: &mut ChildSpawnerCommands, action: DemoButton, label: &str) {
    panel
        .spawn((
            action,
            Button,
            BackgroundColor(Color::srgb(0.12, 0.22, 0.28)),
            Node {
                min_width: px(220),
                height: px(42),
                padding: UiRect::horizontal(px(14)),
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
    mut events: ResMut<crate::fx::BattleEvents>,
    mut last_cursor: Local<IVec2>,
) {
    if *state.get() != GameScreen::Battle || pause.open {
        return;
    }
    let Some(mut world) = world.take() else {
        return;
    };
    if let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), cameras.single()) {
        if let Some(p) = input::cursor_world(window, camera, cam_tf) {
            *last_cursor = p.round().as_ivec2();
        }
    }
    let input = if autoplay.is_some() {
        autopilot(&world)
    } else {
        let mut buttons = 0;
        if MovePress::read(&mouse).held() {
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
    events.record(&world);
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
/// the coast is clear; against the Guardian, see `guardian_pilot`.
/// Input-only, like a player.
fn autopilot(world: &SimState) -> NetInput {
    let mut input = NetInput::default();
    let Some(ship) = world.ships.first() else {
        return input;
    };
    if let Some(input) = guardian_pilot(world, ship) {
        return input;
    }
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

/// The autopilot against the Fissure Guardian, reading its telegraphs the
/// way a player would: through a ring's wind-up it flies to the nearer gap
/// (just outside its basic range of the Guardian's centre), off a charge
/// line it steps sideways, and otherwise it circles the Guardian inside
/// its range, using W when chaff gets close.
fn guardian_pilot(world: &SimState, ship: &sim::Ship) -> Option<NetInput> {
    use sim::trig::{cos_q16, sin_q16, ANGLE_STEPS};
    use sim::BossAttack;
    let g = world.guardian()?;
    let b = g.boss?;
    let me = ship.pos;
    let away = me - g.pos;
    let range = (ship.stats.basic_range - 30) * SUB;
    let unit = |a: i32| FxVec2::new(cos_q16(a), sin_q16(a));
    // A ring takes about a second to reach it after it fires: keep to the
    // gap until it has passed.
    let rest = if b.phase >= 2 {
        sim::tuning::GUARDIAN_REST_P2
    } else {
        sim::tuning::GUARDIAN_REST_P1
    };
    let ring_out = b.attack == BossAttack::Ring && g.cooldown + 70 > rest;
    let goal = match b.attack {
        BossAttack::Ring if g.windup > 0 || ring_out => {
            let gaps = [b.gap, (b.gap + ANGLE_STEPS / 2) % ANGLE_STEPS];
            let dir = gaps
                .into_iter()
                .map(unit)
                .min_by_key(|d| (g.pos + d.scale_to(range) - me).length_squared())?;
            g.pos + dir.scale_to(range)
        }
        _ if g.dash > 0 || g.windup == 0 => {
            // Circle: a quarter-turn step round it at range.
            let side = FxVec2::new(-away.y, away.x).scale_to(80 * SUB);
            g.pos + (away.scale_to(range) + side).scale_to(range)
        }
        BossAttack::Ring => g.pos + away.scale_to(range),
        BossAttack::Charge => me + FxVec2::new(-g.aim.y, g.aim.x).scale_to(160 * SUB),
        BossAttack::Summon => g.pos + away.scale_to(range),
    };
    // Chaff close by: W at it (on a frame it isn't steering).
    let chaff = world
        .enemies
        .iter()
        .filter(|e| e.boss.is_none())
        .map(|e| e.pos)
        .find(|p| (*p - me).length() < 120 * SUB as i64);
    let (buttons, at) = match chaff {
        Some(p) if world.frame % 10 == 5 => (INPUT_SKILL_W, p),
        _ => (INPUT_MOVE, goal),
    };
    Some(NetInput {
        buttons,
        target_x: at.x / SUB,
        target_y: at.y / SUB,
    })
}

/// A smaller window than `DEFAULT_WINDOW`, for the pause-menu shots.
const SMALL_WINDOW: Vec2 = Vec2::new(800.0, 460.0);

/// Autoplay: from `t0`, opens the pause menu and shoots it (`name`), then
/// at `SMALL_WINDOW` (`name-small`), then restores the window and resumes.
#[allow(clippy::too_many_arguments)]
fn pause_shots(
    at: &dyn Fn(f32) -> bool,
    t0: f32,
    autoplay: &Autoplay,
    commands: &mut Commands,
    pause: &mut PauseMenu,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
    name: &str,
) {
    let mut resize = |size: Vec2| {
        if let Ok(mut window) = windows.single_mut() {
            window.resolution.set(size.x, size.y);
        }
    };
    if at(t0) {
        pause.hold(true);
    }
    if at(t0 + 0.3) {
        autoplay.shoot(commands, name);
    }
    if at(t0 + 0.4) {
        resize(SMALL_WINDOW);
    }
    if at(t0 + 1.0) {
        autoplay.shoot(commands, &format!("{name}-small"));
    }
    if at(t0 + 1.4) {
        resize(crate::DEFAULT_WINDOW.as_vec2());
        pause.hold(false);
    }
}

/// Scripted scene steps for `--autoplay`. Each step fires once, when the
/// scene has been up for its time in seconds.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn autoplay_flow(
    mut commands: Commands,
    state: Res<State<GameScreen>>,
    mut autoplay: ResMut<Autoplay>,
    time: Res<Time>,
    mut save: ResMut<SaveSlot>,
    mut next: ResMut<NextState<GameScreen>>,
    mut overlay: ResMut<Overlay>,
    (mut view, mut cursor, mut walk, scene, mut settings_screen, mut pause, mut codex): (
        ResMut<CarrierView>,
        ResMut<BuildCursor>,
        ResMut<WalkOrder>,
        Res<CarrierScene>,
        ResMut<SettingsScreen>,
        ResMut<PauseMenu>,
        ResMut<CodexScreen>,
    ),
    (mut drag, mut windows): (ResMut<PilotDrag>, Query<&mut Window, With<PrimaryWindow>>),
    pilot: Query<(&Transform, &Pilot)>,
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
            let name = if autoplay.reloaded {
                "90-title-reloaded"
            } else if autoplay.resume {
                "01-title-continue"
            } else {
                "01-title"
            };
            if at(1.0) {
                autoplay.shoot(&mut commands, name);
            }
            // The Settings screen over the first Title, then its Codex:
            // every tab, each with an entry's details.
            let first = !autoplay.reloaded && !autoplay.resume;
            if first {
                if at(1.8) {
                    settings_screen.open(&mut pause, false);
                }
                if at(2.4) {
                    autoplay.shoot(&mut commands, "02-title-settings");
                }
                for (i, (slug, entry)) in CODEX_SHOTS.iter().enumerate() {
                    let t = 2.6 + 0.7 * i as f32;
                    if at(t) {
                        codex.show(i, *entry);
                    }
                    if at(t + 0.5) {
                        autoplay.shoot(&mut commands, &format!("{:02}-codex-{slug}", 3 + i));
                    }
                }
                if at(6.2) {
                    codex.close();
                }
                if at(6.4) {
                    settings_screen.close(&mut pause);
                }
            }
            // The same view later, to show the nebula sky drifting.
            let later = if first { 7.0 } else { 3.5 };
            if at(later) {
                autoplay.shoot(&mut commands, &format!("{name}-later"));
            }
            if at(later + 0.5) {
                if autoplay.resume || autoplay.reloaded {
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
            let tag = if autoplay.reloaded {
                "reloaded"
            } else if autoplay.resume {
                "continue"
            } else {
                "run"
            };
            if at(1.0) {
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}-carrier-{tag}-after-{n}", 10 + n * 10),
                );
                if let Some(g) = &save.game {
                    println!(
                        "autoplay: carrier credits {} VC {} upgrades {:?} missions {} won {} last {:?}",
                        g.credits,
                        crate::workshop::void_crystal(g),
                        g.purchased_upgrades,
                        g.mission_count,
                        g.missions_won,
                        g.last_result
                    );
                    println!(
                        "autoplay: layout rooms {:?} items {:?}",
                        g.carrier
                            .rooms
                            .iter()
                            .map(|r| format!("{}@{},{}", r.size.id(), r.x, r.y))
                            .collect::<Vec<_>>(),
                        g.carrier
                            .items
                            .iter()
                            .filter(|i| !i.starter)
                            .map(|i| format!("{}@{},{}", i.id.key(), i.x, i.y))
                            .collect::<Vec<_>>()
                    );
                }
            }
            // The whole carrier at Overview zoom.
            if at(1.3) {
                view.zoom.set(carrier::OVERVIEW);
            }
            if at(1.7) {
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}-carrier-overview-{tag}-after-{n}", 11 + n * 10),
                );
            }
            if at(2.0) {
                view.zoom.set(1.0);
            }
            if n >= autoplay.missions {
                // Last visit: the Settings screen over the carrier, from
                // the pause menu, before quitting.
                if autoplay.reloaded {
                    pause_shots(
                        &at,
                        2.3,
                        &autoplay,
                        &mut commands,
                        &mut pause,
                        &mut windows,
                        "91a-carrier-pause",
                    );
                    if at(3.9) {
                        pause.hold(true);
                        settings_screen.open(&mut pause, true);
                    }
                    if at(4.5) {
                        autoplay.shoot(&mut commands, "91-carrier-settings");
                    }
                    if at(4.8) {
                        settings_screen.close(&mut pause);
                        pause.hold(false);
                    }
                    // Then click the Sample Analyzer (TAKOAI-129): the
                    // Pilot walks there and its use opens the Codex over
                    // the deck; closing it is back on the Carrier.
                    if at(5.0) {
                        autoplay.codex_station = None;
                        let order = pilot.single().ok().and_then(|(tf, _)| {
                            carrier::order_walk(
                                &scene.walkable,
                                &scene.hotspots,
                                carrier::grid(tf.translation.truncate()),
                                Vec2::ZERO,
                                Some(Hotspot::SampleAnalyzer),
                            )
                        });
                        match order {
                            Some(o) => *walk = o,
                            None => println!("autoplay: WARNING no path to the Sample Analyzer"),
                        }
                    }
                    if *since >= 5.0 && autoplay.codex_station.is_none() {
                        if codex.open {
                            println!("autoplay: codex opened from the Sample Analyzer");
                            autoplay.codex_station = Some(*since);
                        } else if *since >= 25.0 {
                            println!("autoplay: WARNING walk to the Sample Analyzer timed out");
                            autoplay.codex_station = Some(*since);
                        }
                    }
                }
                let quit_at = if autoplay.reloaded {
                    let Some(opened) = autoplay.codex_station else {
                        return;
                    };
                    let after = |t: f32| before < opened + t && *since >= opened + t;
                    if after(0.5) {
                        autoplay.shoot(&mut commands, "92-carrier-codex-station");
                    }
                    if after(0.8) {
                        codex.leave(&mut pause);
                    }
                    if after(1.1) {
                        println!(
                            "autoplay: codex closed: codex {} pause {} settings {}",
                            codex.open, pause.open, settings_screen.open
                        );
                        autoplay.shoot(&mut commands, "92b-carrier-codex-closed");
                    }
                    opened + 1.4
                } else {
                    2.5
                };
                if at(quit_at) {
                    save.store();
                    if autoplay.reloaded {
                        println!("autoplay: quit");
                        exit.write(AppExit::Success);
                    } else {
                        // Quit to the Title and Continue from what is on
                        // disk, like a restart, without a second launch.
                        autoplay.reloaded = true;
                        save.reload();
                        println!("autoplay: quit to title, reloaded ({})", save.status);
                        next.set(transition(
                            GameScreen::Carrier,
                            FlowEvent::QuitToTitle,
                            true,
                        ));
                    }
                }
                return;
            }
            // First visit (a new game or a Continued save): the builder
            // screen with a ghost, then walk the Pilot once in each of the 8
            // directions across the Bridge, shooting mid-stride (the Dock
            // walk adds more).
            let first_visit = n == 0 && !autoplay.reloaded;
            if first_visit {
                if at(2.3) {
                    *overlay = Overlay::building(Tool::Place(Entry::Room(RoomSize::Small)));
                    cursor.0 = Some(carrier::room_mouse(RoomSize::Small, EXAMPLE_ROOM));
                }
                if at(2.9) {
                    autoplay.shoot(&mut commands, "11b-build-mode-first-visit");
                }
                // Then the room ghost snapping (TAKOAI-126): the mouse two
                // tiles south of flush east of the Workshop snaps it to
                // (21,5); four or more tiles from any door-matched spot it stays put,
                // invalid. Then
                // the other flyouts and a tool, each with its ghost on the
                // Bridge.
                let small = Tool::Place(Entry::Room(RoomSize::Small));
                let flyouts = [
                    (
                        small,
                        carrier::room_mouse(RoomSize::Small, (21, 7)),
                        "room-snap",
                    ),
                    (
                        small,
                        carrier::room_mouse(RoomSize::Small, (24, 9)),
                        "room-no-doors",
                    ),
                    (Tool::Place(Entry::Item(ItemId::Stool)), (3, 3), "furniture"),
                    (Tool::Place(Entry::Item(ItemId::Rug)), (2, 3), "decor"),
                    (Tool::Remove, (6, 1), "remove"),
                ];
                for (i, (tool, at_tile, name)) in flyouts.into_iter().enumerate() {
                    let t = 3.1 + i as f32 * 0.42;
                    if at(t) {
                        *overlay = Overlay::building(tool);
                        cursor.0 = Some(at_tile);
                    }
                    if at(t + 0.35) {
                        autoplay.shoot(&mut commands, &format!("11b-build-mode-{name}"));
                    }
                }
                if at(5.3) {
                    *overlay = Overlay::None;
                }
                for (i, (to, name)) in carrier::PILOT_TOUR.into_iter().enumerate() {
                    let start = TOUR_START + i as f32 * TOUR_LEG;
                    if at(start) {
                        let order = pilot.single().ok().and_then(|(tf, _)| {
                            carrier::order_walk(
                                &scene.walkable,
                                &scene.hotspots,
                                carrier::grid(tf.translation.truncate()),
                                to,
                                None,
                            )
                        });
                        match order {
                            Some(o) => *walk = o,
                            None => println!("autoplay: WARNING no path for walk {name}"),
                        }
                    }
                    // Capture the full gait in every facing, including
                    // diagonals and mirrored views (TAKOAI-130).
                    if autoplay.clip {
                        for k in 0..11 {
                            if at(start + 0.04 + k as f32 * 0.08) {
                                autoplay.shoot(&mut commands, &format!("11c-stride-{name}-{k:02}"));
                            }
                        }
                    }
                    if at(start + 0.45) {
                        if let Ok((tf, p)) = pilot.single() {
                            println!(
                                "autoplay: pilot walk {name} at {} facing {:?}",
                                carrier::grid(tf.translation.truncate()).round(),
                                p.facing
                            );
                        }
                        autoplay.shoot(&mut commands, &format!("11c-pilot-walk-{name}"));
                    }
                }
                // Then hold the button and sweep the cursor around the
                // Bridge (TAKOAI-87): the Pilot follows it, shot mid-drag.
                if before >= DRAG_START && *since < DRAG_END {
                    let held = Some(carrier::drag_cursor(*since - DRAG_START));
                    if drag.cursor != held {
                        drag.cursor = held;
                    }
                }
                if at(DRAG_START + 1.0) {
                    if let Ok((tf, p)) = pilot.single() {
                        println!(
                            "autoplay: pilot drag at {} cursor {} facing {:?} walking {}",
                            carrier::grid(tf.translation.truncate()).round(),
                            drag.cursor.unwrap_or_default().round(),
                            p.facing,
                            !walk.path.is_empty()
                        );
                    }
                    autoplay.shoot(&mut commands, "11d-pilot-drag");
                }
                if at(DRAG_END) {
                    *drag = PilotDrag::default();
                }
            }
            // Between missions: build the spec's example Small Room, wait
            // for it, then place the Salvage Sorter in it and wait again,
            // each as soon as it is affordable (a 10-kill mission pays for
            // the room, the next one for the sorter); else buy the first
            // affordable upgrade.
            let room = Tool::Place(Entry::Room(RoomSize::Small));
            let sorter = Tool::Place(Entry::Item(ItemId::SalvageSorter));
            let room_mouse = carrier::room_mouse(RoomSize::Small, EXAMPLE_ROOM);
            if at(2.5) {
                let plan = save
                    .game
                    .as_ref()
                    .map(|g| autoplay_build_plan(g, room_mouse))
                    .unwrap_or_default();
                autoplay.building = n > 0 && (plan.0 || plan.1);
                autoplay.build_plan = plan;
            }
            let build_run = autoplay.building;
            if n > 0 && build_run {
                let shot = |name: &str| format!("{:02}{name}", 12 + n * 10);
                let (build_room, build_sorter) = autoplay.build_plan;
                if build_room {
                    if at(2.5) {
                        *overlay = Overlay::building(room);
                        cursor.0 = Some(room_mouse);
                    }
                    if at(3.1) {
                        autoplay.shoot(&mut commands, &shot("a-build-preview-room"));
                    }
                    if at(3.4) {
                        autoplay_build(&mut save, &mut overlay, room, room_mouse);
                    }
                    if at(4.4) {
                        autoplay.shoot(&mut commands, &shot("b-room-under-construction"));
                    }
                }
                // The room takes 5 s; timers run in Build mode too.
                if build_sorter {
                    if at(9.0) {
                        *overlay = Overlay::building(sorter);
                        cursor.0 = Some(EXAMPLE_SORTER_MOUSE);
                    }
                    if at(9.5) {
                        autoplay.shoot(&mut commands, &shot("c-build-preview-sorter"));
                    }
                    if at(9.8) {
                        autoplay_build(&mut save, &mut overlay, sorter, EXAMPLE_SORTER_MOUSE);
                    }
                    for (k, &(id, mouse)) in EXAMPLE_FURNISHING.iter().enumerate() {
                        if at(10.0 + k as f32 * 0.1) {
                            let piece = Tool::Place(Entry::Item(id));
                            autoplay_build(&mut save, &mut overlay, piece, mouse);
                        }
                    }
                    if at(10.6) {
                        autoplay.shoot(&mut commands, &shot("d-sorter-under-construction"));
                    }
                }
                // The sorter takes 4 s, the furniture 2 s.
                if at(14.3) {
                    *overlay = Overlay::None;
                    view.zoom.set(carrier::OVERVIEW);
                    if let Some(g) = &save.game {
                        println!(
                            "autoplay: built: room {} sorter {}",
                            g.carrier
                                .room_at(room_mouse)
                                .is_some_and(|i| g.carrier.rooms[i].finished()),
                            g.carrier.has_finished(ItemId::SalvageSorter)
                        );
                    }
                }
                if at(14.8) {
                    autoplay.shoot(&mut commands, &shot("e-expanded-carrier"));
                }
                if at(15.2) {
                    view.zoom.set(1.0);
                }
            } else if n > 0 {
                if at(3.5) {
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
                            .and_then(|(u, _)| {
                                crate::workshop::UPGRADES.iter().position(|&x| x == u)
                            })
                            .unwrap_or(0);
                        *overlay = Overlay::workshop(index, message);
                    }
                    save.store();
                }
                if at(4.5) {
                    autoplay.shoot(&mut commands, &format!("{:02}-workshop", 12 + n * 10));
                }
                if at(5.0) {
                    *overlay = Overlay::None;
                }
            }
            let berth = carrier::ship_index(&autoplay.ship);
            // The first visit's Pilot tour and a build visit run past the
            // usual 7.5 s.
            let dock = if first_visit {
                DRAG_END + 0.6
            } else if n > 0 && build_run {
                15.6
            } else {
                7.5
            };
            if at(dock) {
                // Click the Command Console: the Pilot walks to the mission
                // station and opens mission select on arrival (TAKOAI-130).
                autoplay.boarded = None;
                let order = pilot.single().ok().and_then(|(tf, _)| {
                    carrier::order_walk(
                        &scene.walkable,
                        &scene.hotspots,
                        carrier::grid(tf.translation.truncate()),
                        Vec2::ZERO,
                        Some(Hotspot::CommandConsole),
                    )
                });
                match order {
                    Some(o) => {
                        println!(
                            "autoplay: click the Command Console: walk {} waypoints",
                            o.path.len()
                        );
                        *walk = o;
                    }
                    None => println!("autoplay: no path to the Command Console"),
                }
            }
            if at(dock + 0.5) {
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}e-walk-to-dock-a", 12 + n * 10),
                );
            }
            if at(dock + 1.2) {
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}e-walk-to-dock-b", 12 + n * 10),
                );
            }
            if *since >= dock && autoplay.boarded.is_none() {
                if matches!(*overlay, Overlay::Missions { .. }) {
                    println!(
                        "autoplay: mission select after a {:.1} s walk",
                        *since - dock
                    );
                    autoplay.boarded = Some(*since);
                } else if *since >= dock + 20.0 {
                    // Never arrived: open it in place so the run goes on.
                    println!("autoplay: WARNING walk to the Command Console timed out");
                    *walk = WalkOrder::default();
                    carrier::interact_with(
                        Hotspot::CommandConsole,
                        save.game.as_mut(),
                        &mut overlay,
                    );
                    autoplay.boarded = Some(*since);
                }
            }
            let Some(boarded) = autoplay.boarded else {
                return;
            };
            let after = |t: f32| before < boarded + t && *since >= boarded + t;
            if after(0.4) {
                // Pick this run's ship, as 1 / 2 would.
                *overlay = Overlay::Missions { ship: berth };
            }
            if after(0.8) {
                autoplay.shoot(&mut commands, &format!("{:02}-mission-select", 13 + n * 10));
            }
            if after(1.3) {
                *overlay = carrier::launch_prep(berth, save.game.as_mut());
            }
            if after(1.8) {
                autoplay.shoot(&mut commands, &format!("{:02}a-briefing", 14 + n * 10));
            }
            if after(2.2) {
                *overlay = Overlay::None;
            }
            if after(2.5) {
                autoplay.shoot(&mut commands, &format!("{:02}-dock-berths", 14 + n * 10));
            }
            if after(2.8) {
                launch.write(MissionRequest {
                    battleship_id: autoplay.ship.clone(),
                });
            }
        }
        GameScreen::Loading => {
            // The first frame: loading takes a handful of frames at most.
            if before == 0.0 {
                println!("autoplay: loading mission {}", n + 1);
                autoplay.shoot(
                    &mut commands,
                    &format!("{:02}b-loading-{}", 14 + n * 10, n + 1),
                );
            }
        }
        GameScreen::Battle => {
            if before == 0.0 {
                println!("autoplay: battle {} started", n + 1);
            }
            if at(0.05) {
                if let Some(w) = world.as_deref() {
                    println!(
                        "autoplay: populated start: {} enemies, {} dormant, target {}",
                        w.enemies.len(),
                        w.enemies.iter().filter(|e| e.dormant).count(),
                        w.mission.kill_target
                    );
                }
                autoplay.shoot(&mut commands, "15a-populated-start");
            }
            for (i, t) in [1.0, 6.0, 20.0, 35.0].into_iter().enumerate() {
                if at(t) {
                    autoplay.shoot(
                        &mut commands,
                        &format!("{:02}-battle-{}-{i}", 15 + n * 10, n + 1),
                    );
                }
            }
            let done = autoplay.role_shots;
            if let Some((i, name)) = world.as_deref().and_then(|w| role_shot(w, done)) {
                autoplay.role_shots |= 1 << i;
                let name = if let Some(elite) = name.strip_prefix("e-") {
                    format!("15e-battle-{elite}")
                } else if let Some(boss) = name.strip_prefix("g-") {
                    format!("15f-battle-{boss}")
                } else {
                    format!("15d-battle-{name}")
                };
                autoplay.shoot(&mut commands, &name);
            }
            if autoplay.clip && n == 0 {
                for k in 0..30 {
                    if at(7.0 + k as f32 * 0.1) {
                        autoplay.shoot(&mut commands, &format!("15c-battle-clip-{k:02}"));
                    }
                }
            }
            if autoplay.abandon && n + 1 == autoplay.missions {
                pause_shots(
                    &at,
                    18.8,
                    &autoplay,
                    &mut commands,
                    &mut pause,
                    &mut windows,
                    &format!("{:02}d-battle-pause", 15 + n * 10),
                );
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

/// What the autoplay builds this Carrier visit: the example Small Room
/// (mouse tile `room_mouse`) if it isn't there and is affordable, and the
/// Salvage Sorter if there is none and it is affordable after the room (it
/// goes into that room once it's finished).
fn autoplay_build_plan(g: &SaveGame, room_mouse: (i32, i32)) -> (bool, bool) {
    let room_cost = RoomSize::Small.cost();
    let sorter_cost = Entry::Item(ItemId::SalvageSorter).cost();
    let have_room = g.carrier.room_at(room_mouse).is_some();
    let room = !have_room && crate::workshop::can_afford(g, room_cost);
    let mut left = g.clone();
    if room {
        left.credits -= room_cost.credits;
        if let Some(vc) = left.resources.get_mut(mission::VOID_CRYSTAL_ID) {
            *vc -= room_cost.void_crystal;
        }
    }
    let sorter = (have_room || room)
        && g.carrier.find(ItemId::SalvageSorter).is_none()
        && crate::workshop::can_afford(&left, sorter_cost);
    (room, sorter)
}

/// One autoplay Build-mode click on the mouse tile `at`, logged and saved
/// like a player's.
fn autoplay_build(save: &mut SaveSlot, overlay: &mut Overlay, tool: Tool, at: (i32, i32)) {
    if let Some(game) = &mut save.game {
        let out = carrier::apply_build_click(game, tool, at, None);
        println!(
            "autoplay: build {at:?}: {} credits {} VC {}",
            out.message,
            game.credits,
            crate::workshop::void_crystal(game)
        );
        *overlay = Overlay::Build {
            category: carrier::BuildCategory::of(tool),
            tool: out.tool,
            message: out.message,
        };
    }
    save.store();
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
        assert_eq!(state, GameScreen::Loading);
        state = transition(state, FlowEvent::AssetsLoaded, true);
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
            kill_target: sim::tuning::KILL_TARGET_MAX,
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
    /// upgrade. Checked on the autoplay pilot's won runs with both
    /// battleships. The autopilot is no player: it flies at the nearest
    /// enemy, so it wakes dormant groups of the scattered start
    /// (TAKOAI-130) and can lose some runs. Check the payout on wins and
    /// require a majority of this fixed seed set to remain winnable.
    #[test]
    fn one_successful_mission_buys_an_upgrade() {
        for ship in [mission::KITE_ID, mission::BULWARK_ID] {
            let mut won = 0;
            for seed in 10_000..10_008 {
                let (r, _) = autoplayed(ship, &[], seed);
                if !r.outcome.success() {
                    continue;
                }
                won += 1;
                let mut save = SaveGame::default();
                save.record_mission_return(true, r.credits, &r.resources);
                assert!(
                    crate::workshop::can_afford(&save, crate::workshop::COSTS[0]),
                    "{ship} {seed}: {} cr {:?}",
                    r.credits,
                    r.resources
                );
            }
            assert!(won >= 5, "{ship}: won {won} of 8");
        }
    }

    /// The autoplay builds the spec's example Small Room, then its Salvage
    /// Sorter, each as soon as it can pay for it.
    #[test]
    fn autoplay_builds_the_example_edits_when_affordable() {
        let wallet = |credits, vc| {
            let mut g = SaveGame {
                credits,
                ..Default::default()
            };
            g.resources.insert(mission::VOID_CRYSTAL_ID.into(), vc);
            g
        };
        let mouse = carrier::room_mouse(RoomSize::Small, EXAMPLE_ROOM);
        assert_eq!(autoplay_build_plan(&wallet(0, 0), mouse), (false, false));
        // One 10-kill run (150 cr + 5 VC) pays for the room only.
        assert_eq!(autoplay_build_plan(&wallet(150, 5), mouse), (true, false));
        assert_eq!(autoplay_build_plan(&wallet(180, 4), mouse), (true, true));
        // The next visit places the sorter in the finished room.
        let mut g = wallet(240, 9);
        crate::workshop::build_room(&mut g, RoomSize::Small, 14, 10).unwrap();
        g.carrier.tick(5000);
        assert_eq!(autoplay_build_plan(&g, mouse), (false, true));
        let out = carrier::apply_build_click(
            &mut g,
            Tool::Place(Entry::Item(ItemId::SalvageSorter)),
            EXAMPLE_SORTER_MOUSE,
            None,
        );
        assert!(out.changed, "{}", out.message);
        assert_eq!(autoplay_build_plan(&g, mouse), (false, false));
        // Then furnishes the room around it, every piece placed and the
        // layout still valid once they are finished.
        g.credits += 200;
        for (id, at) in EXAMPLE_FURNISHING {
            let out = carrier::apply_build_click(&mut g, Tool::Place(Entry::Item(id)), at, None);
            assert!(out.changed, "{id:?} at {at:?}: {}", out.message);
        }
        g.carrier.tick(4000);
        assert_eq!(g.carrier.validate(), Ok(()));
        for (id, at) in EXAMPLE_FURNISHING {
            let placed = g
                .carrier
                .items
                .iter()
                .find(|i| i.id == id && (i.x, i.y) == at);
            assert!(placed.is_some_and(|i| i.finished()), "{id:?} at {at:?}");
        }
    }

    /// TAKOAI-98: tips belong only on a transition screen, never in a scene.
    /// No client module spawns a "Tip:" box or registers the retired
    /// tutorial-hint UI, so neither the Carrier nor the Battle has one.
    #[test]
    fn no_scene_spawns_a_tip_box() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = vec![src];
        let mut checked = 0;
        while let Some(path) = files.pop() {
            if path.is_dir() {
                files.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for marker in [
                concat!("\"Tip", ": "),
                concat!("Hint", "Ui"),
                concat!("Hints", "Plugin"),
            ] {
                assert!(!text.contains(marker), "{} has {marker}", path.display());
            }
            checked += 1;
        }
        assert!(checked > 10, "only {checked} source files found");
    }

    /// TAKOAI-103: every mission goes through the transition screen, which
    /// only leaves for the Battle once loaded, or back to the Carrier.
    #[test]
    fn every_launch_passes_through_loading() {
        assert_eq!(
            transition(GameScreen::Carrier, FlowEvent::LaunchMission, true),
            GameScreen::Loading
        );
        for screen in [GameScreen::Title, GameScreen::Carrier, GameScreen::Result] {
            assert_ne!(
                transition(screen, FlowEvent::AssetsLoaded, true),
                GameScreen::Battle,
                "{screen:?} skips the transition screen"
            );
        }
        for event in [
            FlowEvent::NewGame,
            FlowEvent::Continue,
            FlowEvent::LaunchMission,
            FlowEvent::FinishMission,
        ] {
            assert_eq!(
                transition(GameScreen::Loading, event, true),
                GameScreen::Loading
            );
        }
        assert_eq!(
            transition(GameScreen::Loading, FlowEvent::ReturnToCarrier, true),
            GameScreen::Carrier
        );
    }

    /// TAKOAI-127: the pause menu is one panel (a single root), with a
    /// button per action; Battle adds Quit Mission.
    #[test]
    fn pause_menu_is_one_panel_of_buttons() {
        use bevy::ecs::system::RunSystemOnce;
        for (screen, quit_mission) in [(GameScreen::Carrier, false), (GameScreen::Battle, true)] {
            let mut world = World::new();
            world.insert_resource(State::new(screen));
            world.insert_resource(SettingsScreen::default());
            world.insert_resource(CodexScreen::default());
            world.insert_resource(PauseMenu {
                open: true,
                dirty: true,
            });
            world.run_system_once(sync_pause_menu).unwrap();
            let roots = world
                .query_filtered::<Entity, With<PauseEntity>>()
                .iter(&world)
                .count();
            assert_eq!(roots, 1, "{screen:?}");
            let mut buttons: Vec<DemoButton> = world
                .query_filtered::<&DemoButton, With<Button>>()
                .iter(&world)
                .copied()
                .collect();
            buttons.sort_by_key(|b| *b as u8);
            let mut expected = vec![
                DemoButton::Resume,
                DemoButton::Settings,
                DemoButton::QuitToTitle,
                DemoButton::QuitGame,
            ];
            if quit_mission {
                expected.push(DemoButton::QuitMission);
            }
            expected.sort_by_key(|b| *b as u8);
            assert_eq!(buttons, expected, "{screen:?}");
            // Closing despawns the whole panel.
            world.resource_mut::<PauseMenu>().hold(false);
            world.run_system_once(sync_pause_menu).unwrap();
            assert_eq!(
                world.query::<&DemoButton>().iter(&world).count(),
                0,
                "{screen:?}"
            );
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
