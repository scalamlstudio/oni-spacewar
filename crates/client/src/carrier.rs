//! Carrier scene: a side-view, one-deck cross-section the Pilot walks around
//! between missions (design/READINESS.md § Demo Spec › Carrier). Client only;
//! nothing here touches the sim.
//!
//! Rooms, crew and the Pilot's walk cycle are shipped art (design/art/demo,
//! imported by `demo-art-import`), loaded by stable content ID through the
//! manifest (`crate::art`).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use sim::tuning::{
    AFTERBURN_DISTANCE, BASTION_DURATION, BASTION_SHIELD, SCATTER_BOLTS, SHOCKWAVE_PUSH,
    SHOCKWAVE_RADIUS, TICKS_PER_SEC,
};
use sim::{Loadout, ShipKind, ShipSheet, Upgrades};

use crate::art::{self, icon_node, ContentImages};
use crate::flow::{self, GameScreen, PauseMenu, SaveSlot, ScreenEntity};
use crate::hints::{Hint, Hints};
use crate::mission::MissionRequest;
use crate::save::LastResult;
use crate::workshop::{self, BuyError, Upgrade, UPGRADES};

/// Width of one room; the deck is four rooms long.
pub const ROOM_W: f32 = 320.0;
pub const DECK_LEN: f32 = ROOM_W * 4.0;
/// A / D walking speed, px/s.
pub const WALK_SPEED: f32 = 120.0;
/// E reaches the nearest hotspot within this many px.
pub const INTERACT_RANGE: f32 = 40.0;
const PILOT_HALF_W: f32 = 10.0;
const PILOT_SPAWN_NEW_GAME: f32 = 110.0;
const PILOT_SPAWN_AFTER_MISSION: f32 = 1060.0;
const CAMERA_Y: f32 = 70.0;
/// Carrier camera zoom (world px per screen px); the deck art reads better
/// close up than at 1:1.
const CAMERA_ZOOM: f32 = 0.7;
/// Room art is fitted to the room width; this much of it hangs below the
/// walking line (the art's lower frame).
const ROOM_ART_SINK: f32 = 34.0;
/// On-screen height of the Pilot and crew sprites, px.
const PILOT_H: f32 = 64.0;
const CREW_H: f32 = 70.0;
/// Pilot walk cycle: a new frame every this many px walked.
const STRIDE: f32 = 12.0;
const PILOT_FRAMES: [&str; 4] = [
    "core.carrier.pilot.walk_1",
    "core.carrier.pilot.walk_2",
    "core.carrier.pilot.walk_3",
    "core.carrier.pilot.walk_4",
];
const PILOT_IDLE: &str = "core.carrier.pilot.idle";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Room {
    Bridge,
    CrewQuarters,
    Workshop,
    Dock,
}

const ROOMS: [(Room, &str, &str); 4] = [
    (Room::Bridge, "Bridge", "core.carrier.room.bridge"),
    (
        Room::CrewQuarters,
        "Crew Quarters",
        "core.carrier.room.crew_quarters",
    ),
    (Room::Workshop, "Workshop", "core.carrier.room.workshop"),
    (Room::Dock, "Dock", "core.carrier.room.dock"),
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

    /// Standing sprite on the deck.
    fn sprite_id(self) -> String {
        format!("core.portraits.{}.normal", self.key())
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
        "Twenty kills clears it. They keep pouring out of void fissures.",
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

/// A battleship in the Dock. Its numbers are read from `sim::ShipSheet`
/// with the current Workshop upgrades, so the Dock shows what the battle
/// will use.
pub struct ShipInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub role: &'static str,
    pub kind: ShipKind,
    /// Battle sprite, also shown on the Dock card.
    pub image_id: &'static str,
}

pub const SHIPS: [ShipInfo; 2] = [
    ShipInfo {
        id: "kite",
        name: "Kite",
        role: "fast, fragile, short cooldowns",
        kind: ShipKind::Kite,
        image_id: crate::render::ids::SHIP_KITE,
    },
    ShipInfo {
        id: "bulwark",
        name: "Bulwark",
        role: "slow, tanky, stronger basic attack",
        kind: ShipKind::Bulwark,
        image_id: crate::render::ids::SHIP_BULWARK,
    },
];

fn sheet(kind: ShipKind, upgrades: Upgrades) -> ShipSheet {
    ShipSheet::new(Loadout {
        ship: kind,
        upgrades,
    })
}

/// `value`, plus how much upgrades added to it.
fn with_bonus(value: i32, base: i32) -> String {
    if value > base {
        format!("{value} (+{})", value - base)
    } else {
        value.to_string()
    }
}

fn secs(ticks: u32) -> String {
    let s = format!("{:.2}", ticks as f32 / TICKS_PER_SEC as f32);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The Dock's stat card for a battleship with Workshop `upgrades` applied.
pub fn ship_card(ship: &ShipInfo, upgrades: Upgrades) -> String {
    let s = sheet(ship.kind, upgrades);
    let b = sheet(ship.kind, Upgrades::default());
    let (q, w) = match ship.kind {
        ShipKind::Kite => (
            format!(
                "Afterburn: dash {AFTERBURN_DISTANCE} px toward the cursor. CD {} s",
                secs(s.q_cooldown)
            ),
            format!(
                "Scatter: {SCATTER_BOLTS} bolts in a 40 deg cone, {} dmg each. CD {} s",
                with_bonus(s.w_damage, b.w_damage),
                secs(s.w_cooldown)
            ),
        ),
        ShipKind::Bulwark => (
            format!(
                "Bastion: shield absorbs the next {BASTION_SHIELD} dmg for {} s. CD {} s",
                secs(BASTION_DURATION),
                secs(s.q_cooldown)
            ),
            format!(
                "Shockwave: {} dmg within {SHOCKWAVE_RADIUS} px, pushes {SHOCKWAVE_PUSH} px. CD {} s",
                with_bonus(s.w_damage, b.w_damage),
                secs(s.w_cooldown)
            ),
        ),
    };
    format!(
        "{}\nHull {}    Speed {} px/s    Radius {} px\nBasic: {} dmg every {} s, range {} px\nQ {q}\nW {w}",
        ship.role,
        with_bonus(s.hull, b.hull),
        with_bonus(s.speed, b.speed),
        s.radius,
        with_bonus(s.basic_damage, b.basic_damage),
        secs(s.basic_interval),
        s.basic_range,
    )
}

/// What buying the next level of `upgrade` changes, one line per battleship.
pub fn upgrade_preview(upgrade: Upgrade, current: Upgrades) -> Vec<String> {
    let mut next = current;
    let slot = match upgrade {
        Upgrade::HullPlating => &mut next.hull,
        Upgrade::WeaponTuning => &mut next.weapon,
        Upgrade::ThrusterTuning => &mut next.thruster,
    };
    if *slot >= sim::tuning::MAX_UPGRADE_LEVEL {
        return vec!["Fully upgraded.".to_string()];
    }
    *slot += 1;
    SHIPS
        .iter()
        .map(|ship| {
            let (a, b) = (sheet(ship.kind, current), sheet(ship.kind, next));
            let change = match upgrade {
                Upgrade::HullPlating => format!("hull {} -> {}", a.hull, b.hull),
                Upgrade::ThrusterTuning => {
                    format!("speed {} -> {} px/s", a.speed, b.speed)
                }
                Upgrade::WeaponTuning => format!(
                    "basic {} -> {} dmg, {} {} -> {} dmg",
                    a.basic_damage,
                    b.basic_damage,
                    if ship.kind == ShipKind::Kite {
                        "Scatter"
                    } else {
                        "Shockwave"
                    },
                    a.w_damage,
                    b.w_damage
                ),
            };
            format!("{:<8} {change}", ship.name)
        })
        .collect()
}

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

pub const LAUNCH_CONSOLE_X: f32 = 1010.0;

/// Interactable things on the deck and their x position.
pub const HOTSPOTS: [(Hotspot, f32); 5] = [
    (Hotspot::Crew(Crew::Gunner), 210.0),
    (Hotspot::Crew(Crew::Researcher), 500.0),
    (Hotspot::Crew(Crew::Engineer), 700.0),
    (Hotspot::UpgradeBench, 840.0),
    (Hotspot::LaunchConsole, LAUNCH_CONSOLE_X),
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
    /// The upgrade shop: the selected row and the last purchase's feedback.
    Workshop {
        index: usize,
        message: String,
    },
}

/// Set by the flow when the Carrier is entered straight from a mission, so the
/// Pilot spawns at the Dock instead of the Bridge.
#[derive(Resource, Default)]
pub struct CarrierArrival {
    pub from_mission: bool,
}

/// The player character. `walked` (px since it last stood still) drives
/// the walk cycle.
#[derive(Component, Default)]
pub(crate) struct Pilot {
    walked: f32,
    last_x: Option<f32>,
}

#[derive(Component)]
struct Prompt;

/// One live value in the Carrier HUD.
#[derive(Component, Clone, Copy, PartialEq)]
enum HudField {
    Credits,
    Crystal,
    Room,
}

#[derive(Component)]
struct OverlayUi;

fn portrait(
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    crew: Crew,
    expression: &str,
) -> Option<Handle<Image>> {
    art.get(
        images,
        &format!("core.portraits.{}.{}", crew.key(), expression),
    )
}

/// Every manifest image the Carrier draws (besides dialogue portraits).
#[cfg(test)]
pub fn image_ids() -> Vec<String> {
    let mut ids: Vec<String> = ROOMS.iter().map(|r| r.2.to_string()).collect();
    ids.extend(PILOT_FRAMES.iter().map(|f| f.to_string()));
    ids.push(PILOT_IDLE.into());
    ids.extend(SHIPS.iter().map(|s| s.image_id.to_string()));
    for crew in [Crew::Gunner, Crew::Researcher, Crew::Engineer] {
        ids.push(crew.sprite_id());
    }
    ids
}

pub struct CarrierPlugin;

impl Plugin for CarrierPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Overlay>()
            .init_resource::<CarrierArrival>()
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
                    animate_pilot,
                    update_prompt,
                    update_hud,
                    carrier_hints,
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

/// A sprite `height` px tall standing with its feet at `pos`.
fn standing(
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    id: &str,
    height: f32,
    pos: Vec3,
) -> impl Bundle {
    let image = art.get(images, id).unwrap_or_default();
    let px = ContentImages::size(images, &image);
    (
        ScreenEntity,
        Sprite {
            image,
            custom_size: Some(Vec2::new(px.x * height / px.y, height)),
            ..default()
        },
        Transform::from_translation(pos + Vec3::Y * height / 2.0),
    )
}

fn spawn_carrier(
    mut commands: Commands,
    arrival: Res<CarrierArrival>,
    mut overlay: ResMut<Overlay>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut camera: Query<&mut Projection, With<Camera2d>>,
) {
    *overlay = Overlay::None;
    if let Ok(mut projection) = camera.single_mut() {
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scale = CAMERA_ZOOM;
        }
    }
    // Rooms: each room's art fitted to its width, its lower frame below the
    // walking line (y = 0).
    for (i, (_, name, id)) in ROOMS.iter().enumerate() {
        let x0 = i as f32 * ROOM_W;
        let image = art.get(&mut images, id).unwrap_or_default();
        let px = ContentImages::size(&images, &image);
        let h = ROOM_W * px.y / px.x;
        commands.spawn((
            ScreenEntity,
            Sprite {
                image,
                custom_size: Some(Vec2::new(ROOM_W, h)),
                ..default()
            },
            Transform::from_xyz(x0 + ROOM_W / 2.0, h / 2.0 - ROOM_ART_SINK, -1.0),
        ));
        commands.spawn(label(
            name,
            16.0,
            Color::srgb(0.75, 0.82, 0.9),
            Vec3::new(x0 + ROOM_W / 2.0, h - ROOM_ART_SINK + 14.0, 1.0),
        ));
    }
    commands.spawn(label(
        "Next: Elimination",
        11.0,
        Color::srgb(0.7, 0.9, 1.0),
        Vec3::new(90.0, 100.0, 0.3),
    ));

    // Crew.
    for (hotspot, x) in HOTSPOTS {
        let Hotspot::Crew(crew) = hotspot else {
            continue;
        };
        commands.spawn(standing(
            &mut art,
            &mut images,
            &crew.sprite_id(),
            CREW_H,
            Vec3::new(x, 0.0, 0.5),
        ));
        commands.spawn(label(
            crew.name(),
            11.0,
            crew.color(),
            Vec3::new(x, CREW_H + 8.0, 0.6),
        ));
    }

    // The Pilot (player character).
    let x = if arrival.from_mission {
        PILOT_SPAWN_AFTER_MISSION
    } else {
        PILOT_SPAWN_NEW_GAME
    };
    commands.spawn((
        standing(
            &mut art,
            &mut images,
            PILOT_IDLE,
            PILOT_H,
            Vec3::new(x, 0.0, 2.0),
        ),
        Pilot::default(),
    ));

    commands.spawn((
        label("", 14.0, Color::WHITE, Vec3::new(0.0, 0.0, 5.0)),
        Prompt,
    ));
    // Wallet and current room, top right.
    let credits = art.get(&mut images, art::ids::ICON_CREDITS);
    let crystal = art.get(&mut images, art::ids::ICON_VOID_CRYSTAL);
    let hud_font = || {
        (
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.9, 0.6)),
        )
    };
    commands
        .spawn((
            ScreenEntity,
            Node {
                position_type: PositionType::Absolute,
                right: px(16),
                top: px(36),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                row_gap: px(2),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            })
            .with_children(|row| {
                for (icon, field) in [(credits, HudField::Credits), (crystal, HudField::Crystal)] {
                    if let Some(icon) = icon {
                        row.spawn(icon_node(icon, 26.0));
                    }
                    row.spawn((field, Text::new(""), hud_font()));
                }
            });
            p.spawn((HudField::Room, Text::new(""), hud_font()));
        });
}

fn leave_carrier(
    mut overlay: ResMut<Overlay>,
    mut arrival: ResMut<CarrierArrival>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    *overlay = Overlay::None;
    arrival.from_mission = false;
    if let Ok((mut cam, mut projection)) = camera.single_mut() {
        cam.translation.x = 0.0;
        cam.translation.y = 0.0;
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scale = 1.0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn carrier_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    pause: Res<PauseMenu>,
    mut save: ResMut<SaveSlot>,
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
                Some((Hotspot::UpgradeBench, _)) => Overlay::Workshop {
                    index: 0,
                    message: String::new(),
                },
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
        Overlay::Workshop { index, message } => {
            let up = keys.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
            let down = keys.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
            let digit = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
                .iter()
                .position(|k| keys.just_pressed(*k));
            if back {
                *overlay = Overlay::None;
            } else if let Some(i) = digit {
                *index = i;
            } else if up {
                *index = (*index + UPGRADES.len() - 1) % UPGRADES.len();
            } else if down {
                *index = (*index + 1) % UPGRADES.len();
            } else if confirm {
                let upgrade = UPGRADES[*index];
                let Some(game) = &mut save.game else {
                    return;
                };
                *message = match workshop::buy(game, upgrade) {
                    Ok(level) => format!("Installed {} {level}.", upgrade.name()),
                    Err(BuyError::Maxed) => format!("{} is fully upgraded.", upgrade.name()),
                    Err(BuyError::TooExpensive(c)) => format!(
                        "Not enough: {} needs {} credits and {} Void Crystal.",
                        upgrade.name(),
                        c.credits,
                        c.void_crystal
                    ),
                };
                // Spec § Save file: saved on every Workshop purchase.
                save.store();
            }
        }
    }
}

/// Carrier tutorial hint triggers (`hints`).
fn carrier_hints(
    overlay: Res<Overlay>,
    pilot: Query<&Transform, With<Pilot>>,
    mut hints: ResMut<Hints>,
    mut save: ResMut<SaveSlot>,
) {
    let Ok(pilot) = pilot.single() else {
        return;
    };
    let x = pilot.translation.x;
    if nearest_hotspot(x).is_some() {
        hints.trigger(&mut save, Hint::CarrierInteract);
    }
    if room_at(x) == Room::Dock {
        hints.trigger(&mut save, Hint::Dock);
    }
    if matches!(*overlay, Overlay::Workshop { .. }) {
        hints.trigger(&mut save, Hint::Workshop);
    }
}

/// Walk-cycle frame for the Pilot after `walked` px; idle when standing.
fn pilot_frame(moving: bool, walked: f32) -> &'static str {
    if !moving {
        return PILOT_IDLE;
    }
    PILOT_FRAMES[(walked / STRIDE) as usize % PILOT_FRAMES.len()]
}

fn animate_pilot(
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut pilot: Query<(&mut Pilot, &Transform, &mut Sprite)>,
) {
    let Ok((mut pilot, tf, mut sprite)) = pilot.single_mut() else {
        return;
    };
    let x = tf.translation.x;
    let dx = x - pilot.last_x.unwrap_or(x);
    pilot.last_x = Some(x);
    let moving = dx.abs() > 0.01;
    if moving {
        pilot.walked += dx.abs();
        // The art faces right.
        sprite.flip_x = dx < 0.0;
    } else {
        pilot.walked = 0.0;
    }
    if let Some(image) = art.get(&mut images, pilot_frame(moving, pilot.walked)) {
        if sprite.image != image {
            sprite.image = image;
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
    let half = windows.single().map(|w| w.width() / 2.0).unwrap_or(500.0) * CAMERA_ZOOM;
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
            tf.translation.y = PILOT_H + 20.0;
            *vis = Visibility::Visible;
        }
        _ => *vis = Visibility::Hidden,
    }
}

fn update_hud(
    save: Res<SaveSlot>,
    pilot: Query<&Transform, With<Pilot>>,
    mut hud: Query<(&mut Text, &HudField)>,
) {
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
    for (mut text, field) in &mut hud {
        let value = match field {
            HudField::Credits => format!("{credits} credits  "),
            HudField::Crystal => format!("{crystal} Void Crystal"),
            HudField::Room => room.to_string(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

fn sync_overlay(
    mut commands: Commands,
    overlay: Res<Overlay>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    save: Res<SaveSlot>,
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
    let game = save.game.clone().unwrap_or_default();
    let levels = workshop::levels(&game);
    match &*overlay {
        Overlay::None => {}
        Overlay::Dialogue {
            lines,
            index,
            briefing,
        } => {
            let line = lines[*index];
            let portrait = portrait(&mut art, &mut images, line.speaker, line.expression);
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
                    p.spawn((
                        Text::new("Dock - pick a battleship"),
                        font(22.0, Color::WHITE),
                    ));
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
                                if let Some(image) = art.get(&mut images, ship.image_id) {
                                    card.spawn((
                                        ImageNode::new(image),
                                        Node {
                                            height: px(72),
                                            align_self: AlignSelf::Center,
                                            ..default()
                                        },
                                    ));
                                }
                                card.spawn((
                                    Text::new(ship_card(ship, levels)),
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
        Overlay::Workshop { index, message } => {
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
                        top: px(96),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(14)),
                        row_gap: px(8),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new(format!(
                            "Workshop - upgrades apply to every battleship\nCredits {}    Void Crystal {}",
                            game.credits,
                            workshop::void_crystal(&game)
                        )),
                        font(20.0, Color::WHITE),
                    ));
                    for (i, upgrade) in UPGRADES.iter().enumerate() {
                        let level = upgrade.level_in(levels);
                        let (price, affordable) = match workshop::next_level(&game, *upgrade) {
                            Some((next, cost)) => (
                                format!(
                                    "Lv {next}: {} cr + {} VC",
                                    cost.credits, cost.void_crystal
                                ),
                                workshop::can_afford(&game, cost),
                            ),
                            None => ("maxed".to_string(), false),
                        };
                        let selected = i == *index;
                        let color = if affordable {
                            Color::srgb(0.75, 1.0, 0.75)
                        } else {
                            Color::srgb(0.7, 0.72, 0.75)
                        };
                        p.spawn((
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
                                padding: UiRect::axes(px(10), px(5)),
                                border: UiRect::all(px(1)),
                                column_gap: px(12),
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            if let Some(icon) = art.get(&mut images, upgrade.icon_id()) {
                                row.spawn(icon_node(icon, 24.0));
                            }
                            let cells = [
                                (format!("{} {}", i + 1, upgrade.name()), 180.0),
                                (
                                    format!("Lv {level}/{}", sim::tuning::MAX_UPGRADE_LEVEL),
                                    60.0,
                                ),
                                (upgrade.effect().to_string(), 350.0),
                                (price, 190.0),
                            ];
                            for (text, width) in cells {
                                row.spawn(Node {
                                    width: px(width),
                                    flex_shrink: 0.0,
                                    ..default()
                                })
                                .with_child((Text::new(text), font(16.0, color)));
                            }
                        });
                    }
                    let upgrade = UPGRADES[*index];
                    p.spawn((
                        Text::new(format!(
                            "{} next level:\n  {}",
                            upgrade.name(),
                            upgrade_preview(upgrade, levels).join("\n  ")
                        )),
                        font(15.0, Color::srgb(0.82, 0.86, 0.9)),
                    ));
                    if !message.is_empty() {
                        p.spawn((Text::new(message.clone()), font(16.0, Color::srgb(1.0, 0.85, 0.4))));
                    }
                    p.spawn((
                        Text::new("[W]/[S] or [1]-[3] Choose    [E] Buy    [X] Close"),
                        font(15.0, Color::srgb(0.6, 0.7, 0.75)),
                    ));
                });
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
    fn pilot_walk_cycle_steps_with_distance_and_idles_when_still() {
        assert_eq!(pilot_frame(false, 50.0), PILOT_IDLE);
        assert_eq!(pilot_frame(true, 0.0), PILOT_FRAMES[0]);
        assert_eq!(pilot_frame(true, STRIDE * 2.5), PILOT_FRAMES[2]);
        assert_eq!(pilot_frame(true, STRIDE * 5.0), PILOT_FRAMES[1]);
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
        let manifest = content::ContentManifest::load(concat!(
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
    fn dock_card_reads_sim_stats_with_upgrades() {
        let base = ship_card(&SHIPS[0], Upgrades::default());
        assert!(base.contains("Hull 60 "), "{base}");
        assert!(base.contains("Speed 220 px/s"), "{base}");
        assert!(
            base.contains("Basic: 4 dmg every 0.35 s, range 220 px"),
            "{base}"
        );
        let up = Upgrades {
            hull: 1,
            weapon: 1,
            thruster: 2,
        };
        let kite = ship_card(&SHIPS[0], up);
        assert!(kite.contains("Hull 75 (+15)"), "{kite}");
        assert!(kite.contains("Speed 286 (+66) px/s"), "{kite}");
        assert!(kite.contains("Basic: 5 (+1) dmg"), "{kite}");
        let bulwark = ship_card(&SHIPS[1], up);
        assert!(bulwark.contains("Hull 175 (+35)"), "{bulwark}");
        assert!(bulwark.contains("Shockwave: 18 (+3) dmg"), "{bulwark}");
    }

    #[test]
    fn upgrade_preview_shows_both_ships() {
        let p = upgrade_preview(Upgrade::WeaponTuning, Upgrades::default());
        assert_eq!(p.len(), 2);
        assert!(p[0].contains("basic 4 -> 5"), "{p:?}");
        assert!(p[1].contains("basic 8 -> 10"), "{p:?}");
        let maxed = Upgrades {
            hull: 2,
            ..Default::default()
        };
        assert_eq!(upgrade_preview(Upgrade::HullPlating, maxed).len(), 1);
    }

    #[test]
    fn unknown_saved_ship_falls_back_to_kite() {
        assert_eq!(ship_index("bulwark"), 1);
        assert_eq!(SHIPS[ship_index("starter-frigate")].id, "kite");
    }
}
