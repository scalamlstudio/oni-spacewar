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
use crate::save::{LastResult, SaveGame};
use crate::workshop::{self, BuyError, Upgrade, UPGRADES};

/// Width of one room. Bridge, Crew Quarters and Workshop are one room wide;
/// the Dock is one berth per battleship.
pub const ROOM_W: f32 = 320.0;
/// Width of one Dock berth.
pub const BERTH_W: f32 = ROOM_W;
/// Where the Dock starts: after the three one-room rooms.
pub const DOCK_X: f32 = ROOM_W * 3.0;
pub const DECK_LEN: f32 = DOCK_X + BERTH_W * SHIPS.len() as f32;
/// A / D walking speed, px/s.
pub const WALK_SPEED: f32 = 120.0;
/// E reaches the nearest hotspot within this many px.
pub const INTERACT_RANGE: f32 = 40.0;
/// A docked battleship is big; E reaches it from this far.
pub const BERTH_REACH: f32 = 90.0;
const PILOT_HALF_W: f32 = 10.0;
const PILOT_SPAWN_NEW_GAME: f32 = 110.0;
/// Between the two berths, out of reach of both.
const PILOT_SPAWN_AFTER_MISSION: f32 = DOCK_X + BERTH_W;
const CAMERA_Y: f32 = 70.0;
/// Carrier camera zoom (world px per screen px); the deck art reads better
/// close up than at 1:1.
const CAMERA_ZOOM: f32 = 0.7;
/// Room art is fitted to the room width; this much of it hangs below the
/// walking line (the art's lower frame).
const ROOM_ART_SINK: f32 = 34.0;
/// Height of the Pilot and the crew from head to feet, px. The art has
/// different amounts of empty canvas, so this is measured on the visible
/// pixels (`standing`), not the image size.
const CHARACTER_H: f32 = 48.0;
/// Berth pad size the docked ships' lengths are given for
/// (design/READINESS.md § Dock and berths: 256 px pad).
const BERTH_PAD: f32 = 256.0;
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

/// Rooms left to right: room, name, room art (the Dock draws its berths
/// instead), left edge, width.
const ROOMS: [(Room, &str, Option<&str>, f32, f32); 4] = [
    (
        Room::Bridge,
        "Bridge",
        Some("core.carrier.room.bridge"),
        0.0,
        ROOM_W,
    ),
    (
        Room::CrewQuarters,
        "Crew Quarters",
        Some("core.carrier.room.crew_quarters"),
        ROOM_W,
        ROOM_W,
    ),
    (
        Room::Workshop,
        "Workshop",
        Some("core.carrier.room.workshop"),
        ROOM_W * 2.0,
        ROOM_W,
    ),
    (Room::Dock, "Dock", None, DOCK_X, DECK_LEN - DOCK_X),
];

/// An empty ship berth; the docked battleship is drawn on top.
const BERTH_IMAGE: &str = "core.carrier.dock.berth";

pub fn room_at(x: f32) -> Room {
    ROOMS
        .iter()
        .rev()
        .find(|r| x >= r.3)
        .map_or(Room::Bridge, |r| r.0)
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

/// A battleship in the Dock. Its numbers are read from `sim::ShipSheet`
/// with the current Workshop upgrades, so the Dock shows what the battle
/// will use.
pub struct ShipInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub role: &'static str,
    pub kind: ShipKind,
    /// Battle sprite, also drawn in its Dock berth.
    pub image_id: &'static str,
    /// Longest side of the docked sprite on a [`BERTH_PAD`] pad, px.
    pub berth_len: f32,
}

pub const SHIPS: [ShipInfo; 2] = [
    ShipInfo {
        id: "kite",
        name: "Kite",
        role: "fast, fragile, short cooldowns",
        kind: ShipKind::Kite,
        image_id: crate::render::ids::SHIP_KITE,
        berth_len: 150.0,
    },
    ShipInfo {
        id: "bulwark",
        name: "Bulwark",
        role: "slow, tanky, stronger basic attack",
        kind: ShipKind::Bulwark,
        image_id: crate::render::ids::SHIP_BULWARK,
        berth_len: 210.0,
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
    /// The battleship docked in berth `n` (an index into [`SHIPS`]).
    Berth(usize),
}

impl Hotspot {
    fn verb(self) -> String {
        match self {
            Hotspot::Crew(_) => "Talk".into(),
            Hotspot::UpgradeBench => "Use".into(),
            Hotspot::Berth(i) => format!("Board {}", SHIPS[i].name),
        }
    }

    fn reach(self) -> f32 {
        match self {
            Hotspot::Berth(_) => BERTH_REACH,
            _ => INTERACT_RANGE,
        }
    }
}

/// Centre of Dock berth `i`, where its battleship sits.
pub const fn berth_x(i: usize) -> f32 {
    DOCK_X + (i as f32 + 0.5) * BERTH_W
}

/// Interactable things on the deck and their x position.
pub const HOTSPOTS: [(Hotspot, f32); 6] = [
    (Hotspot::Crew(Crew::Gunner), 210.0),
    (Hotspot::Crew(Crew::Researcher), 500.0),
    (Hotspot::Crew(Crew::Engineer), 700.0),
    (Hotspot::UpgradeBench, 840.0),
    (Hotspot::Berth(0), berth_x(0)),
    (Hotspot::Berth(1), berth_x(1)),
];

/// The hotspot E would use from `x`: the closest one within reach.
pub fn nearest_hotspot(x: f32) -> Option<(Hotspot, f32)> {
    HOTSPOTS
        .iter()
        .copied()
        .map(|(h, hx)| (h, hx, (hx - x).abs()))
        .filter(|&(h, _, d)| d <= h.reach())
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
    /// Dialogue lines; a briefing (`launch` = the boarded ship) continues
    /// into the launch panel.
    Dialogue {
        lines: Vec<Line>,
        index: usize,
        launch: Option<usize>,
    },
    /// Confirm and launch the battleship in [`SHIPS`]`[index]`.
    Launch { index: usize },
    /// The upgrade shop: the selected row and the last purchase's feedback.
    Workshop { index: usize, message: String },
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

/// A Dock berth: one slot holding one battleship (`ship`, an index into
/// [`SHIPS`]). Spawned by [`berth`]; the 2.5D carrier reuses it for more
/// berths.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Berth {
    pub ship: usize,
}

/// The battleship sprite inside a [`Berth`]; clicking it boards the ship.
#[derive(Component, Clone, Copy)]
pub struct DockedShip {
    pub ship: usize,
    /// Half the sprite's on-screen extent (for clicks and the highlight).
    pub half: Vec2,
}

/// "Selected" tag over the docked ship the save will launch.
#[derive(Component)]
struct SelectedTag;

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
    let mut ids: Vec<String> = ROOMS
        .iter()
        .filter_map(|r| r.2.map(str::to_string))
        .collect();
    ids.push(BERTH_IMAGE.into());
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
                    click_ship,
                    carrier_input,
                    follow_pilot,
                    animate_pilot,
                    update_prompt,
                    update_hud,
                    show_selected_ship,
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

/// Share of an image's height above and below its visible pixels
/// (alpha > 0): `(top, bottom)`. Images without pixel data count as full.
fn empty_rows(image: Option<&Image>) -> (f32, f32) {
    let Some((image, data)) = image.and_then(|i| i.data.as_ref().map(|d| (i, d))) else {
        return (0.0, 0.0);
    };
    let (w, h) = (image.width() as usize, image.height() as usize);
    if data.len() < w * h * 4 || h == 0 {
        return (0.0, 0.0);
    }
    let visible = |y: usize| (0..w).any(|x| data[(y * w + x) * 4 + 3] > 0);
    let Some(top) = (0..h).position(visible) else {
        return (0.0, 0.0);
    };
    let bottom = (0..h).rev().position(visible).unwrap_or(0);
    (top as f32 / h as f32, bottom as f32 / h as f32)
}

/// A character `height` px tall from head to feet, feet at `pos`. The
/// sprite is scaled by its visible pixels, so empty canvas above or below
/// the character doesn't change how big it looks.
fn standing(
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    id: &str,
    height: f32,
    pos: Vec3,
) -> impl Bundle {
    let image = art.get(images, id).unwrap_or_default();
    let px = ContentImages::size(images, &image);
    let (top, bottom) = empty_rows(images.get(&image));
    let canvas_h = height / (1.0 - top - bottom).max(0.1);
    (
        ScreenEntity,
        Sprite {
            image,
            custom_size: Some(Vec2::new(px.x * canvas_h / px.y, canvas_h)),
            ..default()
        },
        Transform::from_translation(pos + Vec3::Y * (canvas_h / 2.0 - bottom * canvas_h)),
    )
}

/// A Dock berth at `x0..x0 + BERTH_W` holding `SHIPS[ship]`, drawn with the
/// same sprite as in battle, nose up, sized for the berth's height the way
/// the design sizes it on a 256 px pad.
pub fn berth(
    commands: &mut Commands,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    x0: f32,
    ship: usize,
) {
    let image = art.get(images, BERTH_IMAGE).unwrap_or_default();
    let px = ContentImages::size(images, &image);
    let h = BERTH_W * px.y / px.x;
    let centre = Vec2::new(x0 + BERTH_W / 2.0, h / 2.0 - ROOM_ART_SINK);
    commands.spawn((
        ScreenEntity,
        Berth { ship },
        Sprite {
            image,
            custom_size: Some(Vec2::new(BERTH_W, h)),
            ..default()
        },
        Transform::from_translation(centre.extend(-1.0)),
    ));
    let image = art.get(images, SHIPS[ship].image_id).unwrap_or_default();
    let px = ContentImages::size(images, &image);
    let size = px * (SHIPS[ship].berth_len / BERTH_PAD) * h / px.max_element();
    commands.spawn((
        ScreenEntity,
        DockedShip {
            ship,
            half: size / 2.0,
        },
        Sprite {
            image,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(centre.extend(0.2)),
    ));
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
    for (_, name, id, x0, w) in ROOMS {
        // Rooms are about this tall; the label sits over the art.
        let mut h = ROOM_W * 0.57;
        if let Some(id) = id {
            let image = art.get(&mut images, id).unwrap_or_default();
            let px = ContentImages::size(&images, &image);
            h = w * px.y / px.x;
            commands.spawn((
                ScreenEntity,
                Sprite {
                    image,
                    custom_size: Some(Vec2::new(w, h)),
                    ..default()
                },
                Transform::from_xyz(x0 + w / 2.0, h / 2.0 - ROOM_ART_SINK, -1.0),
            ));
        }
        commands.spawn(label(
            name,
            16.0,
            Color::srgb(0.75, 0.82, 0.9),
            Vec3::new(x0 + w / 2.0, h - ROOM_ART_SINK + 14.0, 1.0),
        ));
    }
    for (i, _) in SHIPS.iter().enumerate() {
        berth(
            &mut commands,
            &mut art,
            &mut images,
            DOCK_X + i as f32 * BERTH_W,
            i,
        );
    }
    commands.spawn((
        label("Selected", 11.0, Color::srgb(0.5, 0.9, 1.0), Vec3::ZERO),
        SelectedTag,
    ));
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
            CHARACTER_H,
            Vec3::new(x, 0.0, 0.5),
        ));
        commands.spawn(label(
            crew.name(),
            11.0,
            crew.color(),
            Vec3::new(x, CHARACTER_H + 10.0, 0.6),
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
            CHARACTER_H,
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
            if let Some((hotspot, _)) = nearest_hotspot(tf.translation.x) {
                interact_with(hotspot, save.game.as_mut(), &mut overlay);
            }
        }
        Overlay::Dialogue {
            lines,
            index,
            launch,
        } => {
            if back {
                *overlay = Overlay::None;
            } else if confirm {
                if *index + 1 < lines.len() {
                    *index += 1;
                } else if let Some(ship) = *launch {
                    *overlay = Overlay::Launch { index: ship };
                } else {
                    *overlay = Overlay::None;
                }
            }
        }
        Overlay::Launch { index } => {
            if back {
                *overlay = Overlay::None;
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

/// Using `hotspot`: talk to crew, open the Workshop, or board a docked
/// battleship, which selects it and starts the briefing before launch.
pub fn interact_with(hotspot: Hotspot, game: Option<&mut SaveGame>, overlay: &mut Overlay) {
    let last = game.as_ref().map(|g| g.last_result).unwrap_or_default();
    *overlay = match hotspot {
        Hotspot::Crew(crew) => Overlay::Dialogue {
            lines: crew_lines(crew, last),
            index: 0,
            launch: None,
        },
        Hotspot::Berth(ship) => {
            if let Some(game) = game {
                game.selected_battleship = SHIPS[ship].id.to_string();
            }
            Overlay::Dialogue {
                lines: crew_lines(Crew::Pilot, last),
                index: 0,
                launch: Some(ship),
            }
        }
        Hotspot::UpgradeBench => Overlay::Workshop {
            index: 0,
            message: String::new(),
        },
    };
}

/// The docked ship under world point `p`, if any.
pub fn ship_at(p: Vec2, ships: impl IntoIterator<Item = (Vec2, DockedShip)>) -> Option<usize> {
    ships
        .into_iter()
        .find(|(centre, s)| (p - *centre).abs().cmple(s.half).all())
        .map(|(_, s)| s.ship)
}

/// Clicking a docked battleship boards it, like walking up and pressing E.
fn click_ship(
    mouse: Res<ButtonInput<MouseButton>>,
    pause: Res<PauseMenu>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ships: Query<(&GlobalTransform, &DockedShip)>,
    mut save: ResMut<SaveSlot>,
    mut overlay: ResMut<Overlay>,
) {
    if pause.open || *overlay != Overlay::None || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), camera.single()) else {
        return;
    };
    let Some(p) = window
        .cursor_position()
        .and_then(|c| camera.viewport_to_world_2d(cam_tf, c).ok())
    else {
        return;
    };
    let docked = ships
        .iter()
        .map(|(tf, s)| (tf.translation().truncate(), *s));
    if let Some(ship) = ship_at(p, docked) {
        interact_with(Hotspot::Berth(ship), save.game.as_mut(), &mut overlay);
    }
}

/// Marks the docked ship the save has selected: a highlight ring and a
/// "Selected" tag; the other ship is dimmed.
fn show_selected_ship(
    save: Res<SaveSlot>,
    mut ships: Query<(&Transform, &DockedShip, &mut Sprite), Without<SelectedTag>>,
    mut tag: Query<&mut Transform, With<SelectedTag>>,
    mut gizmos: Gizmos,
) {
    let selected = save
        .game
        .as_ref()
        .map(|g| ship_index(&g.selected_battleship))
        .unwrap_or(0);
    for (tf, ship, mut sprite) in &mut ships {
        let chosen = ship.ship == selected;
        let color = if chosen {
            Color::WHITE
        } else {
            Color::srgb(0.6, 0.6, 0.65)
        };
        if sprite.color != color {
            sprite.color = color;
        }
        if chosen {
            let centre = tf.translation.truncate();
            let ring = ship.half.max_element() + 6.0;
            for r in [ring, ring + 2.0] {
                gizmos.circle_2d(centre, r, Color::srgb(0.5, 0.9, 1.0));
            }
            if let Ok(mut tag) = tag.single_mut() {
                tag.translation = (centre + Vec2::Y * (ring + 10.0)).extend(1.0);
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
            // Under the docked ship's berth, clear of the ship.
            tf.translation.y = match hotspot {
                Hotspot::Berth(_) => -ROOM_ART_SINK - 12.0,
                _ => CHARACTER_H + 24.0,
            };
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
            launch,
        } => {
            let line = lines[*index];
            let portrait = portrait(&mut art, &mut images, line.speaker, line.expression);
            let more = if *index + 1 < lines.len() {
                "[E] Next"
            } else if launch.is_some() {
                "[E] Launch prep"
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
        Overlay::Launch { index } => {
            let ship = &SHIPS[*index];
            commands
                .spawn((
                    ScreenEntity,
                    OverlayUi,
                    panel_bg,
                    border,
                    Node {
                        position_type: PositionType::Absolute,
                        // Beside the boarded ship, not over it.
                        left: if *index % 2 == 1 { px(30) } else { auto() },
                        right: if *index % 2 == 0 { px(30) } else { auto() },
                        top: px(100),
                        width: px(520),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(14)),
                        row_gap: px(10),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new(format!("Launch the {}?", ship.name)),
                        font(22.0, Color::WHITE),
                    ));
                    p.spawn((
                        Text::new(ship_card(ship, levels)),
                        font(14.0, Color::srgb(0.82, 0.86, 0.9)),
                    ));
                    p.spawn((
                        Text::new("[E] Launch    [X] Back"),
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
            nearest_hotspot(840.0 + INTERACT_RANGE).map(|h| h.0),
            Some(Hotspot::UpgradeBench)
        );
        assert_eq!(nearest_hotspot(840.0 + INTERACT_RANGE + 1.0), None);
        assert_eq!(nearest_hotspot(PILOT_SPAWN_NEW_GAME), None);
        assert_eq!(nearest_hotspot(PILOT_SPAWN_AFTER_MISSION), None);
        assert_eq!(
            nearest_hotspot(berth_x(1) - BERTH_REACH).map(|h| h.0),
            Some(Hotspot::Berth(1))
        );
    }

    #[test]
    fn every_ship_has_one_berth_in_the_dock() {
        let berths: Vec<_> = HOTSPOTS
            .iter()
            .filter_map(|&(h, x)| match h {
                Hotspot::Berth(i) => Some((i, x)),
                _ => None,
            })
            .collect();
        assert_eq!(berths.len(), SHIPS.len());
        for (i, (ship, x)) in berths.into_iter().enumerate() {
            assert_eq!(ship, i);
            assert_eq!(room_at(x), Room::Dock);
        }
        assert_eq!(room_at(DOCK_X - 1.0), Room::Workshop);
        assert_eq!(room_at(DECK_LEN), Room::Dock);
    }

    #[test]
    fn boarding_a_ship_selects_it_and_briefs_before_launch() {
        let mut game = SaveGame::default();
        let mut overlay = Overlay::None;
        interact_with(Hotspot::Berth(1), Some(&mut game), &mut overlay);
        assert_eq!(game.selected_battleship, "bulwark");
        assert!(matches!(
            overlay,
            Overlay::Dialogue {
                launch: Some(1),
                ..
            }
        ));
    }

    #[test]
    fn clicks_hit_the_docked_ship_under_the_cursor() {
        let ships = [
            (
                Vec2::new(100.0, 50.0),
                DockedShip {
                    ship: 0,
                    half: Vec2::new(60.0, 30.0),
                },
            ),
            (
                Vec2::new(400.0, 50.0),
                DockedShip {
                    ship: 1,
                    half: Vec2::new(60.0, 30.0),
                },
            ),
        ];
        assert_eq!(ship_at(Vec2::new(150.0, 70.0), ships), Some(0));
        assert_eq!(ship_at(Vec2::new(345.0, 25.0), ships), Some(1));
        assert_eq!(ship_at(Vec2::new(250.0, 50.0), ships), None);
        assert_eq!(ship_at(Vec2::new(100.0, 90.0), ships), None);
    }

    #[test]
    fn empty_canvas_rows_are_measured() {
        let mut image = Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 2,
                height: 10,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[0, 0, 0, 0],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );
        let data = image.data.as_mut().unwrap();
        for y in 4..9 {
            data[(y * 2) * 4 + 3] = 255;
        }
        assert_eq!(empty_rows(Some(&image)), (0.4, 0.1));
        assert_eq!(empty_rows(None), (0.0, 0.0));
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
