//! Carrier scene: a side-view, one-deck cross-section the Pilot walks around
//! between missions (design/READINESS.md § Demo Spec › Carrier). Client only;
//! nothing here touches the sim.
//!
//! Visuals are placeholder shapes. Every placeholder carries a stable
//! [`ContentId`] so the demo art import swaps sprites in place instead of
//! reworking the layout. Portraits already ship and load by stable ID through
//! the content manifest.

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use content::ContentManifest;

use crate::flow::{self, GameScreen, PauseMenu, SaveSlot, ScreenEntity};
use crate::mission::MissionRequest;
use crate::save::LastResult;

/// Width of one room; the deck is four rooms long.
pub const ROOM_W: f32 = 320.0;
pub const DECK_LEN: f32 = ROOM_W * 4.0;
const ROOM_H: f32 = 240.0;
/// A / D walking speed, px/s.
pub const WALK_SPEED: f32 = 120.0;
/// E reaches the nearest hotspot within this many px.
pub const INTERACT_RANGE: f32 = 40.0;
const PILOT_HALF_W: f32 = 10.0;
const PILOT_SPAWN_NEW_GAME: f32 = 110.0;
const PILOT_SPAWN_AFTER_MISSION: f32 = 1060.0;
const CAMERA_Y: f32 = 50.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Room {
    Bridge,
    CrewQuarters,
    Workshop,
    Dock,
}

const ROOMS: [(Room, &str, &str, Color); 4] = [
    (
        Room::Bridge,
        "Bridge",
        "core.carrier.room.bridge",
        Color::srgb(0.10, 0.14, 0.22),
    ),
    (
        Room::CrewQuarters,
        "Crew Quarters",
        "core.carrier.room.crew_quarters",
        Color::srgb(0.16, 0.13, 0.18),
    ),
    (
        Room::Workshop,
        "Workshop",
        "core.carrier.room.workshop",
        Color::srgb(0.17, 0.15, 0.11),
    ),
    (
        Room::Dock,
        "Dock",
        "core.carrier.room.dock",
        Color::srgb(0.10, 0.17, 0.16),
    ),
];

pub fn room_at(x: f32) -> Room {
    let i = (x / ROOM_W).floor().clamp(0.0, 3.0) as usize;
    ROOMS[i].0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Crew {
    Pilot,
    Gunner,
    Researcher,
    Engineer,
}

impl Crew {
    pub fn name(self) -> &'static str {
        match self {
            Crew::Pilot => "Pilot",
            Crew::Gunner => "Gunner",
            Crew::Researcher => "Researcher",
            Crew::Engineer => "Engineer",
        }
    }

    /// Directory name under `assets/source/core/portraits/`.
    fn key(self) -> &'static str {
        match self {
            Crew::Pilot => "pilot",
            Crew::Gunner => "gunner",
            Crew::Researcher => "researcher",
            Crew::Engineer => "engineer",
        }
    }

    fn color(self) -> Color {
        match self {
            Crew::Pilot => Color::srgb(0.35, 0.75, 1.0),
            Crew::Gunner => Color::srgb(0.95, 0.45, 0.35),
            Crew::Researcher => Color::srgb(0.70, 0.55, 0.95),
            Crew::Engineer => Color::srgb(0.95, 0.75, 0.30),
        }
    }
}

/// One line of dialogue with the portrait expression to show.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Line {
    pub speaker: Crew,
    pub expression: &'static str,
    pub text: &'static str,
}

const fn line(speaker: Crew, expression: &'static str, text: &'static str) -> Line {
    Line {
        speaker,
        expression,
        text,
    }
}

struct CrewScript {
    lines: &'static [Line],
    success: Line,
    failed: Line,
}

// Lines and expressions: design/READINESS.md § Demo Spec › Crew dialogue.
// Bevy's default font has no em dash or ellipsis, so those are ASCII here.
const BRIEFING: [Line; 3] = [
    line(
        Crew::Pilot,
        "serious",
        "Elimination run. Void monsters are nesting in the next segment.",
    ),
    line(
        Crew::Pilot,
        "normal",
        "Twenty kills clears it. They come in three waves.",
    ),
    line(
        Crew::Pilot,
        "excited",
        "Grab whatever they drop - the Workshop runs on it.",
    ),
];

const GUNNER: CrewScript = CrewScript {
    lines: &[
        line(
            Crew::Gunner,
            "serious",
            "Swarmers rush you. Keep moving and let the guns work.",
        ),
        line(
            Crew::Gunner,
            "normal",
            "Spitters hang back. Their shots are slow - sidestep them.",
        ),
    ],
    success: line(Crew::Gunner, "happy", "Clean shooting out there."),
    failed: line(
        Crew::Gunner,
        "angry",
        "We lost the hull, not the crew. Again.",
    ),
};

const RESEARCHER: CrewScript = CrewScript {
    lines: &[
        line(
            Crew::Researcher,
            "excited",
            "Void Crystal hums when you hold it. I'd love a proper lab.",
        ),
        line(
            Crew::Researcher,
            "confused",
            "Kite or Bulwark? Speed or armor - both are valid.",
        ),
    ],
    success: line(
        Crew::Researcher,
        "happy",
        "Fascinating samples! Well, crystals.",
    ),
    failed: line(Crew::Researcher, "sad", "We'll learn from it... next time."),
};

const ENGINEER: CrewScript = CrewScript {
    lines: &[
        line(
            Crew::Engineer,
            "happy",
            "Bring me credits and crystals and I'll make her sing.",
        ),
        line(
            Crew::Engineer,
            "normal",
            "Plating, weapons, thrusters - pick one.",
        ),
    ],
    success: line(Crew::Engineer, "excited", "Haul's in. Let's upgrade."),
    failed: line(
        Crew::Engineer,
        "sleepy",
        "Nothing came back with you. Not even scrap.",
    ),
};

/// A crew member's lines in the order they are shown: the reaction to the
/// last mission first (skipped before the first mission), then the rest.
pub fn crew_lines(crew: Crew, last: LastResult) -> Vec<Line> {
    let script = match crew {
        Crew::Pilot => return BRIEFING.to_vec(),
        Crew::Gunner => &GUNNER,
        Crew::Researcher => &RESEARCHER,
        Crew::Engineer => &ENGINEER,
    };
    let mut lines = Vec::with_capacity(script.lines.len() + 1);
    match last {
        LastResult::None => {}
        LastResult::Success => lines.push(script.success),
        LastResult::Failed => lines.push(script.failed),
    }
    lines.extend_from_slice(script.lines);
    lines
}

/// Battleship stats shown in the Dock (design/READINESS.md § Demo Spec ›
/// Battleships). Display only: the battle reads its own numbers from `sim`.
pub struct ShipInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub role: &'static str,
    pub hull: u32,
    pub speed: u32,
    pub radius: u32,
    pub basic: &'static str,
    pub q: &'static str,
    pub w: &'static str,
}

pub const SHIPS: [ShipInfo; 2] = [
    ShipInfo {
        id: "kite",
        name: "Kite",
        role: "fast, fragile, short cooldowns",
        hull: 60,
        speed: 220,
        radius: 14,
        basic: "4 dmg every 0.35 s, range 220 px",
        q: "Afterburn: dash 160 px toward the cursor. CD 4 s",
        w: "Scatter: 5 bolts in a 40 deg cone, 6 dmg each. CD 6 s",
    },
    ShipInfo {
        id: "bulwark",
        name: "Bulwark",
        role: "slow, tanky, stronger basic attack",
        hull: 140,
        speed: 140,
        radius: 20,
        basic: "8 dmg every 0.6 s, range 240 px",
        q: "Bastion: shield absorbs the next 40 dmg for 5 s. CD 12 s",
        w: "Shockwave: 15 dmg within 150 px, pushes 100 px. CD 9 s",
    },
];

/// Index into [`SHIPS`] for a saved battleship id; unknown ids fall back to
/// the first ship.
pub fn ship_index(id: &str) -> usize {
    SHIPS.iter().position(|s| s.id == id).unwrap_or(0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hotspot {
    Crew(Crew),
    UpgradeBench,
    LaunchConsole,
}

impl Hotspot {
    fn verb(self) -> &'static str {
        match self {
            Hotspot::Crew(_) => "Talk",
            Hotspot::UpgradeBench | Hotspot::LaunchConsole => "Use",
        }
    }
}

/// Interactable things on the deck and their x position.
pub const HOTSPOTS: [(Hotspot, f32); 5] = [
    (Hotspot::Crew(Crew::Gunner), 210.0),
    (Hotspot::Crew(Crew::Researcher), 500.0),
    (Hotspot::Crew(Crew::Engineer), 700.0),
    (Hotspot::UpgradeBench, 840.0),
    (Hotspot::LaunchConsole, 1010.0),
];

/// The hotspot E would use from `x`: the closest one within reach.
pub fn nearest_hotspot(x: f32) -> Option<(Hotspot, f32)> {
    HOTSPOTS
        .iter()
        .copied()
        .map(|(h, hx)| (h, hx, (hx - x).abs()))
        .filter(|&(_, _, d)| d <= INTERACT_RANGE)
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(h, hx, _)| (h, hx))
}

/// One frame of A / D walking; the walls at either end of the deck stop it.
pub fn walk(x: f32, dir: f32, dt: f32) -> f32 {
    (x + dir * WALK_SPEED * dt).clamp(PILOT_HALF_W, DECK_LEN - PILOT_HALF_W)
}

/// What the carrier is showing on top of the deck.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub enum Overlay {
    #[default]
    None,
    /// Dialogue lines; a briefing continues into ship select.
    Dialogue {
        lines: Vec<Line>,
        index: usize,
        briefing: bool,
    },
    ShipSelect {
        index: usize,
    },
    Workshop,
}

/// Set by the flow when the Carrier is entered straight from a mission, so the
/// Pilot spawns at the Dock instead of the Bridge.
#[derive(Resource, Default)]
pub struct CarrierArrival {
    pub from_mission: bool,
}

/// Stable content ID of a placeholder, for the later art swap.
#[derive(Component, Clone, Copy, Debug)]
#[allow(
    dead_code,
    reason = "read by the demo art import that replaces placeholders"
)]
pub struct ContentId(pub &'static str);

#[derive(Component)]
struct Pilot;

#[derive(Component)]
struct Prompt;

#[derive(Component)]
struct CarrierHud;

#[derive(Component)]
struct OverlayUi;

/// Portrait images by stable asset ID, decoded once from the manifest's
/// processed files.
#[derive(Resource, Default)]
struct Portraits {
    manifest: Option<ContentManifest>,
    loaded: std::collections::HashMap<String, Option<Handle<Image>>>,
}

impl Portraits {
    fn get(
        &mut self,
        images: &mut Assets<Image>,
        crew: Crew,
        expression: &str,
    ) -> Option<Handle<Image>> {
        let id = format!("core.portraits.{}.{}", crew.key(), expression);
        if let Some(handle) = self.loaded.get(&id) {
            return handle.clone();
        }
        if self.manifest.is_none() {
            self.manifest = ContentManifest::load("assets/manifest.json").ok();
        }
        let handle = self
            .manifest
            .as_ref()
            .and_then(|m| m.processed_path("assets", &id).ok())
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| {
                Image::from_buffer(
                    &bytes,
                    ImageType::Extension("png"),
                    CompressedImageFormats::NONE,
                    true,
                    ImageSampler::Default,
                    RenderAssetUsages::default(),
                )
                .ok()
            })
            .map(|image| images.add(image));
        if handle.is_none() {
            warn!("portrait {id} unavailable");
        }
        self.loaded.insert(id, handle.clone());
        handle
    }
}

pub struct CarrierPlugin;

impl Plugin for CarrierPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Overlay>()
            .init_resource::<CarrierArrival>()
            .init_resource::<Portraits>()
            .add_systems(
                OnEnter(GameScreen::Carrier),
                spawn_carrier.after(flow::enter_carrier),
            )
            .add_systems(OnExit(GameScreen::Carrier), leave_carrier)
            .add_systems(
                Update,
                (
                    carrier_input,
                    follow_pilot,
                    update_prompt,
                    update_hud,
                    sync_overlay,
                )
                    .chain()
                    .run_if(in_state(GameScreen::Carrier)),
            );
    }
}

fn label(text: &str, size: f32, color: Color, pos: Vec3) -> impl Bundle {
    (
        ScreenEntity,
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Transform::from_translation(pos),
    )
}

fn block(id: &'static str, color: Color, size: Vec2, pos: Vec3) -> impl Bundle {
    (
        ScreenEntity,
        ContentId(id),
        Sprite::from_color(color, size),
        Transform::from_translation(pos),
    )
}

fn spawn_carrier(
    mut commands: Commands,
    arrival: Res<CarrierArrival>,
    mut overlay: ResMut<Overlay>,
) {
    *overlay = Overlay::None;
    let hull = Color::srgb(0.32, 0.36, 0.42);
    // Hull shell: floor, ceiling, end walls, and bulkheads between rooms.
    commands.spawn(block(
        "core.carrier.hull.floor",
        hull,
        Vec2::new(DECK_LEN + 24.0, 16.0),
        Vec3::new(DECK_LEN / 2.0, -8.0, 0.0),
    ));
    commands.spawn(block(
        "core.carrier.hull.ceiling",
        hull,
        Vec2::new(DECK_LEN + 24.0, 12.0),
        Vec3::new(DECK_LEN / 2.0, ROOM_H + 6.0, 0.0),
    ));
    for x in [-6.0, DECK_LEN + 6.0] {
        commands.spawn(block(
            "core.carrier.hull.end_wall",
            hull,
            Vec2::new(12.0, ROOM_H + 28.0),
            Vec3::new(x, ROOM_H / 2.0, 0.0),
        ));
    }
    for (i, (_, name, id, color)) in ROOMS.iter().enumerate() {
        let x0 = i as f32 * ROOM_W;
        commands.spawn(block(
            id,
            *color,
            Vec2::new(ROOM_W, ROOM_H),
            Vec3::new(x0 + ROOM_W / 2.0, ROOM_H / 2.0, -1.0),
        ));
        commands.spawn(label(
            name,
            16.0,
            Color::srgb(0.75, 0.82, 0.9),
            Vec3::new(x0 + ROOM_W / 2.0, ROOM_H - 18.0, 1.0),
        ));
        if i > 0 {
            // Bulkhead with an open doorway at floor level.
            commands.spawn(block(
                "core.carrier.hull.bulkhead",
                hull,
                Vec2::new(8.0, ROOM_H - 90.0),
                Vec3::new(x0, 90.0 + (ROOM_H - 90.0) / 2.0, 0.5),
            ));
        }
    }

    // Props.
    let prop = Color::srgb(0.45, 0.5, 0.58);
    commands.spawn(block(
        "core.carrier.prop.star_map",
        Color::srgb(0.15, 0.35, 0.55),
        Vec2::new(110.0, 70.0),
        Vec3::new(80.0, 130.0, 0.2),
    ));
    commands.spawn(label(
        "Next: Elimination",
        11.0,
        Color::srgb(0.7, 0.9, 1.0),
        Vec3::new(80.0, 130.0, 0.3),
    ));
    for x in [370.0, 600.0] {
        commands.spawn(block(
            "core.carrier.prop.bunk",
            prop,
            Vec2::new(80.0, 22.0),
            Vec3::new(x, 30.0, 0.2),
        ));
    }
    commands.spawn(block(
        "core.carrier.prop.upgrade_bench",
        Color::srgb(0.6, 0.48, 0.25),
        Vec2::new(70.0, 36.0),
        Vec3::new(840.0, 18.0, 0.2),
    ));
    commands.spawn(block(
        "core.carrier.prop.launch_console",
        Color::srgb(0.25, 0.6, 0.5),
        Vec2::new(26.0, 50.0),
        Vec3::new(1010.0, 25.0, 0.2),
    ));
    for (x, ship, size) in [
        (1120.0, &SHIPS[0], Vec2::new(70.0, 28.0)),
        (1220.0, &SHIPS[1], Vec2::new(84.0, 44.0)),
    ] {
        commands.spawn(block(
            "core.carrier.prop.berth",
            prop,
            Vec2::new(92.0, 8.0),
            Vec3::new(x, 4.0, 0.2),
        ));
        let id = if ship.id == "kite" {
            "core.ships.kite.berth"
        } else {
            "core.ships.bulwark.berth"
        };
        commands.spawn(block(
            id,
            Color::srgb(0.55, 0.62, 0.7),
            size,
            Vec3::new(x, 8.0 + size.y / 2.0, 0.3),
        ));
        commands.spawn(label(
            ship.name,
            12.0,
            Color::srgb(0.85, 0.9, 0.95),
            Vec3::new(x, 8.0 + size.y + 12.0, 0.4),
        ));
    }

    // Crew.
    for (hotspot, x) in HOTSPOTS {
        let Hotspot::Crew(crew) = hotspot else {
            continue;
        };
        let id = match crew {
            Crew::Gunner => "core.carrier.crew.gunner",
            Crew::Researcher => "core.carrier.crew.researcher",
            Crew::Engineer => "core.carrier.crew.engineer",
            Crew::Pilot => "core.carrier.crew.pilot",
        };
        commands.spawn(block(
            id,
            crew.color(),
            Vec2::new(20.0, 46.0),
            Vec3::new(x, 23.0, 0.5),
        ));
        commands.spawn(label(
            crew.name(),
            11.0,
            crew.color(),
            Vec3::new(x, 58.0, 0.6),
        ));
    }

    // The Pilot (player character).
    let x = if arrival.from_mission {
        PILOT_SPAWN_AFTER_MISSION
    } else {
        PILOT_SPAWN_NEW_GAME
    };
    commands.spawn((
        block(
            "core.carrier.pilot",
            Crew::Pilot.color(),
            Vec2::new(PILOT_HALF_W * 2.0, 48.0),
            Vec3::new(x, 24.0, 2.0),
        ),
        Pilot,
    ));

    commands.spawn((
        label("", 14.0, Color::WHITE, Vec3::new(0.0, 0.0, 5.0)),
        Prompt,
    ));
    commands.spawn((
        ScreenEntity,
        CarrierHud,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.9, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            right: px(16),
            top: px(36),
            ..default()
        },
    ));
}

fn leave_carrier(
    mut overlay: ResMut<Overlay>,
    mut arrival: ResMut<CarrierArrival>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    *overlay = Overlay::None;
    arrival.from_mission = false;
    if let Ok(mut cam) = camera.single_mut() {
        cam.translation.x = 0.0;
        cam.translation.y = 0.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn carrier_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    pause: Res<PauseMenu>,
    save: Res<SaveSlot>,
    mut overlay: ResMut<Overlay>,
    mut pilot: Query<&mut Transform, With<Pilot>>,
    mut launch: MessageWriter<MissionRequest>,
) {
    if pause.open {
        return;
    }
    let Ok(mut tf) = pilot.single_mut() else {
        return;
    };
    let interact = keys.just_pressed(KeyCode::KeyE);
    let confirm =
        interact || keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    let back = keys.just_pressed(KeyCode::KeyX) || keys.just_pressed(KeyCode::Backspace);
    let left = keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft);
    let right = keys.just_pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight);

    match &mut *overlay {
        Overlay::None => {
            let mut dir = 0.0;
            if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
                dir -= 1.0;
            }
            if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
                dir += 1.0;
            }
            tf.translation.x = walk(tf.translation.x, dir, time.delta_secs());
            if !interact {
                return;
            }
            let last = save
                .game
                .as_ref()
                .map(|g| g.last_result)
                .unwrap_or_default();
            *overlay = match nearest_hotspot(tf.translation.x) {
                Some((Hotspot::Crew(crew), _)) => Overlay::Dialogue {
                    lines: crew_lines(crew, last),
                    index: 0,
                    briefing: false,
                },
                Some((Hotspot::LaunchConsole, _)) => Overlay::Dialogue {
                    lines: crew_lines(Crew::Pilot, last),
                    index: 0,
                    briefing: true,
                },
                Some((Hotspot::UpgradeBench, _)) => Overlay::Workshop,
                None => return,
            };
        }
        Overlay::Dialogue {
            lines,
            index,
            briefing,
        } => {
            if back {
                *overlay = Overlay::None;
            } else if confirm {
                if *index + 1 < lines.len() {
                    *index += 1;
                } else if *briefing {
                    let selected = save
                        .game
                        .as_ref()
                        .map(|g| ship_index(&g.selected_battleship))
                        .unwrap_or(0);
                    *overlay = Overlay::ShipSelect { index: selected };
                } else {
                    *overlay = Overlay::None;
                }
            }
        }
        Overlay::ShipSelect { index } => {
            if back {
                *overlay = Overlay::None;
            } else if left || keys.just_pressed(KeyCode::Digit1) {
                *index = 0;
            } else if right || keys.just_pressed(KeyCode::Digit2) {
                *index = 1;
            } else if confirm {
                launch.write(MissionRequest {
                    battleship_id: SHIPS[*index].id.to_string(),
                });
            }
        }
        Overlay::Workshop => {
            if back || confirm {
                *overlay = Overlay::None;
            }
        }
    }
}

fn follow_pilot(
    pilot: Query<&Transform, (With<Pilot>, Without<Camera2d>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    let (Ok(pilot), Ok(mut cam)) = (pilot.single(), camera.single_mut()) else {
        return;
    };
    let half = windows.single().map(|w| w.width() / 2.0).unwrap_or(500.0);
    let x = if DECK_LEN <= half * 2.0 {
        DECK_LEN / 2.0
    } else {
        pilot
            .translation
            .x
            .clamp(half - 20.0, DECK_LEN - half + 20.0)
    };
    cam.translation.x = x;
    cam.translation.y = CAMERA_Y;
}

fn update_prompt(
    overlay: Res<Overlay>,
    pilot: Query<&Transform, (With<Pilot>, Without<Prompt>)>,
    mut prompt: Query<(&mut Text2d, &mut Transform, &mut Visibility), With<Prompt>>,
) {
    let (Ok(pilot), Ok((mut text, mut tf, mut vis))) = (pilot.single(), prompt.single_mut()) else {
        return;
    };
    let near = nearest_hotspot(pilot.translation.x);
    match near {
        Some((hotspot, x)) if *overlay == Overlay::None => {
            text.0 = format!("[E] {}", hotspot.verb());
            tf.translation.x = x;
            tf.translation.y = 84.0;
            *vis = Visibility::Visible;
        }
        _ => *vis = Visibility::Hidden,
    }
}

fn update_hud(
    save: Res<SaveSlot>,
    pilot: Query<&Transform, With<Pilot>>,
    mut hud: Query<&mut Text, With<CarrierHud>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let (credits, crystal) = save
        .game
        .as_ref()
        .map(|g| {
            (
                g.credits,
                g.resources.get("void_crystal").copied().unwrap_or(0),
            )
        })
        .unwrap_or_default();
    let room = pilot
        .single()
        .map(|tf| {
            ROOMS
                .iter()
                .find(|r| r.0 == room_at(tf.translation.x))
                .map_or("", |r| r.1)
        })
        .unwrap_or("");
    text.0 = format!("Credits {credits}    Void Crystal {crystal}\n{room}");
}

fn sync_overlay(
    mut commands: Commands,
    overlay: Res<Overlay>,
    mut portraits: ResMut<Portraits>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<OverlayUi>>,
) {
    if !overlay.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let panel_bg = BackgroundColor(Color::srgba(0.03, 0.05, 0.08, 0.94));
    let border = BorderColor::all(Color::srgb(0.35, 0.52, 0.58));
    let font = |size: f32, color: Color| {
        (
            TextFont {
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
        )
    };
    match &*overlay {
        Overlay::None => {}
        Overlay::Dialogue {
            lines,
            index,
            briefing,
        } => {
            let line = lines[*index];
            let portrait = portraits.get(&mut images, line.speaker, line.expression);
            let more = if *index + 1 < lines.len() {
                "[E] Next"
            } else if *briefing {
                "[E] Pick battleship"
            } else {
                "[E] Close"
            };
            commands
                .spawn((
                    ScreenEntity,
                    OverlayUi,
                    panel_bg,
                    border,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(24),
                        right: px(24),
                        bottom: px(18),
                        height: px(150),
                        padding: UiRect::all(px(10)),
                        column_gap: px(16),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    match portrait {
                        Some(image) => {
                            p.spawn((
                                ImageNode::new(image),
                                Node {
                                    width: px(84),
                                    height: px(128),
                                    ..default()
                                },
                            ));
                        }
                        None => {
                            p.spawn((
                                BackgroundColor(line.speaker.color()),
                                Node {
                                    width: px(84),
                                    height: px(128),
                                    ..default()
                                },
                            ));
                        }
                    }
                    p.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        row_gap: px(8),
                        ..default()
                    })
                    .with_children(|c| {
                        c.spawn((
                            Text::new(line.speaker.name()),
                            font(20.0, line.speaker.color()),
                        ));
                        c.spawn((Text::new(line.text), font(19.0, Color::WHITE)));
                        c.spawn((
                            Text::new(format!("{more}    [X] Leave")),
                            font(14.0, Color::srgb(0.6, 0.7, 0.75)),
                        ));
                    });
                });
        }
        Overlay::ShipSelect { index } => {
            commands
                .spawn((
                    ScreenEntity,
                    OverlayUi,
                    panel_bg,
                    border,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(60),
                        right: px(60),
                        top: px(100),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(14)),
                        row_gap: px(12),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((Text::new("Dock - pick a battleship"), font(22.0, Color::WHITE)));
                    p.spawn(Node {
                        column_gap: px(14),
                        ..default()
                    })
                    .with_children(|row| {
                        for (i, ship) in SHIPS.iter().enumerate() {
                            let selected = i == *index;
                            row.spawn((
                                BackgroundColor(if selected {
                                    Color::srgb(0.10, 0.22, 0.28)
                                } else {
                                    Color::srgb(0.07, 0.08, 0.10)
                                }),
                                BorderColor::all(if selected {
                                    Color::srgb(0.5, 0.9, 1.0)
                                } else {
                                    Color::srgb(0.25, 0.3, 0.34)
                                }),
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_grow: 1.0,
                                    flex_basis: px(0),
                                    padding: UiRect::all(px(10)),
                                    row_gap: px(4),
                                    border: UiRect::all(px(if selected { 2 } else { 1 })),
                                    ..default()
                                },
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    Text::new(format!("{} {}", i + 1, ship.name)),
                                    font(20.0, Color::WHITE),
                                ));
                                card.spawn((
                                    Text::new(format!(
                                        "{}\nHull {}    Speed {} px/s    Radius {} px\nBasic: {}\nQ {}\nW {}",
                                        ship.role, ship.hull, ship.speed, ship.radius, ship.basic, ship.q, ship.w
                                    )),
                                    font(14.0, Color::srgb(0.82, 0.86, 0.9)),
                                ));
                            });
                        }
                    });
                    p.spawn((
                        Text::new("[A]/[D] or [1]/[2] Choose    [E] Launch    [X] Back"),
                        font(15.0, Color::srgb(0.6, 0.7, 0.75)),
                    ));
                });
        }
        Overlay::Workshop => {
            commands.spawn((
                ScreenEntity,
                OverlayUi,
                panel_bg,
                border,
                Text::new(
                    "Workshop - upgrade bench\n\n\
                     Hull Plating      +25% hull\n\
                     Weapon Tuning     +25% damage\n\
                     Thruster Tuning   +15% move speed\n\n\
                     The upgrade shop opens in a later build.\n\n[E] Close",
                ),
                font(18.0, Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(260),
                    top: px(90),
                    padding: UiRect::all(px(18)),
                    border: UiRect::all(px(1)),
                    ..default()
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walls_stop_walking_at_both_ends() {
        assert_eq!(walk(PILOT_HALF_W, -1.0, 1.0), PILOT_HALF_W);
        assert_eq!(
            walk(DECK_LEN - PILOT_HALF_W, 1.0, 1.0),
            DECK_LEN - PILOT_HALF_W
        );
        assert_eq!(walk(500.0, 1.0, 0.5), 560.0);
    }

    #[test]
    fn whole_deck_is_walkable_and_every_room_reached() {
        let mut x = PILOT_SPAWN_NEW_GAME;
        let mut rooms = vec![room_at(x)];
        let mut hotspots = Vec::new();
        for _ in 0..(20 * 60) {
            x = walk(x, 1.0, 1.0 / 60.0);
            if rooms.last() != Some(&room_at(x)) {
                rooms.push(room_at(x));
            }
            if let Some((h, _)) = nearest_hotspot(x) {
                if !hotspots.contains(&h) {
                    hotspots.push(h);
                }
            }
        }
        assert_eq!(
            rooms,
            [Room::Bridge, Room::CrewQuarters, Room::Workshop, Room::Dock]
        );
        assert_eq!(hotspots.len(), HOTSPOTS.len());
        assert_eq!(x, DECK_LEN - PILOT_HALF_W);
    }

    #[test]
    fn nearest_hotspot_respects_range() {
        assert_eq!(
            nearest_hotspot(1010.0 + INTERACT_RANGE).map(|h| h.0),
            Some(Hotspot::LaunchConsole)
        );
        assert_eq!(nearest_hotspot(1010.0 + INTERACT_RANGE + 1.0), None);
        assert_eq!(nearest_hotspot(PILOT_SPAWN_NEW_GAME), None);
    }

    #[test]
    fn reaction_line_comes_first_after_a_mission() {
        assert_eq!(crew_lines(Crew::Gunner, LastResult::None).len(), 2);
        let won = crew_lines(Crew::Gunner, LastResult::Success);
        assert_eq!(won[0].text, "Clean shooting out there.");
        assert_eq!(won.len(), 3);
        let lost = crew_lines(Crew::Engineer, LastResult::Failed);
        assert_eq!(lost[0].expression, "sleepy");
        assert_eq!(crew_lines(Crew::Pilot, LastResult::Success).len(), 3);
    }

    #[test]
    fn every_line_has_a_shipped_portrait() {
        let manifest = ContentManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/manifest.json"
        ))
        .unwrap();
        for crew in [Crew::Pilot, Crew::Gunner, Crew::Researcher, Crew::Engineer] {
            for last in [LastResult::None, LastResult::Success, LastResult::Failed] {
                for l in crew_lines(crew, last) {
                    let id = format!("core.portraits.{}.{}", l.speaker.key(), l.expression);
                    assert!(manifest.asset(&id).is_ok(), "{id}");
                }
            }
        }
    }

    #[test]
    fn unknown_saved_ship_falls_back_to_kite() {
        assert_eq!(ship_index("bulwark"), 1);
        assert_eq!(SHIPS[ship_index("starter-frigate")].id, "kite");
    }
}
