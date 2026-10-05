//! Carrier scene: the 2.5D (3/4 top-down) carrier the Pilot walks around
//! between missions and expands at the Workshop bench (design/READINESS.md
//! § Demo Spec › Carrier). Client only; nothing here touches the sim.
//!
//! The layout rules (grid, rooms, corridors, connectivity, walkable area)
//! are in `crate::layout`; this module draws a `CarrierLayout` from the
//! save, walks the Pilot over it, and runs the Workshop panel and Build
//! mode. Every image is shipped art loaded by stable content ID
//! (`crate::art`).

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use sim::tuning::{
    AFTERBURN_DISTANCE, BASTION_DURATION, BASTION_SHIELD, SCATTER_BOLTS, SHOCKWAVE_PUSH,
    SHOCKWAVE_RADIUS, TICKS_PER_SEC,
};
use sim::{Loadout, ShipKind, ShipSheet, Upgrades};

use crate::art::{self, icon_node, ContentImages};
use crate::flow::{self, GameScreen, PauseMenu, SaveSlot, ScreenEntity};
use crate::hints::{Hint, Hints};
use crate::layout::{
    self, room_px, CarrierLayout, Cell, Piece, PlaceError, PlacedRoom, Rect, RoomId, Side, CELL,
    HULL_H, HULL_W,
};
use crate::mission::MissionRequest;
use crate::save::{LastResult, SaveGame};
use crate::workshop::{self, BuyError, Upgrade, UPGRADES};

/// Walking speed, px/s (8 directions, diagonals normalised).
pub const WALK_SPEED: f32 = 160.0;
/// E reaches the nearest hotspot within this many px of the Pilot's feet.
pub const INTERACT_RANGE: f32 = 56.0;
/// Spawn cells: a new game on the Bridge, after a mission on the Dock
/// walkway (§ Walking and interaction).
const SPAWN_NEW_GAME: (i32, i32) = (1, 1);
const SPAWN_AFTER_MISSION: (i32, i32) = (5, 6);
/// Zoom levels, screen px per world px: Overview (the whole hull fits
/// 1280 × 720), Normal, Close. The mouse wheel steps between them.
pub const ZOOMS: [f32; 3] = [0.65, 1.0, 1.5];
const OVERVIEW: usize = 0;
const NORMAL: usize = 1;
/// Build-mode camera pan, screen px/s.
const PAN_SPEED: f32 = 600.0;
/// Height of the Pilot and the crew from head to feet, px. The art has
/// different amounts of empty canvas, so this is measured on the visible
/// pixels (`standing`), not the image size.
const CHARACTER_H: f32 = 70.0;
/// Berth pad size the docked ships' lengths are given for
/// (design/READINESS.md § Dock and berths: a 2 × 2-cell, 256 px pad).
pub const BERTH_PAD: f32 = 2.0 * CELL;
/// Pilot walk cycle: a new frame every this many px walked.
const STRIDE: f32 = 12.0;
const PILOT_FRAMES: [&str; 4] = [
    "core.carrier.pilot.walk_1",
    "core.carrier.pilot.walk_2",
    "core.carrier.pilot.walk_3",
    "core.carrier.pilot.walk_4",
];
const PILOT_IDLE: &str = "core.carrier.pilot.idle";
const HULL_FLOOR: &str = "core.carrier.hull_floor";
const BUILD_SLOT: &str = "core.carrier.build_slot";
/// Draw layers. Floors are flat; characters and docked ships are sorted by
/// their feet (`depth`).
const Z_HULL: f32 = -30.0;
const Z_FLOOR: f32 = -20.0;
const Z_DOOR: f32 = -19.0;
const Z_SLOT: f32 = -18.0;
const Z_GHOST: f32 = -17.0;
const Z_LABEL: f32 = 5.0;

/// World position (Bevy, y up) of grid px (y down).
pub fn world(p: Vec2) -> Vec2 {
    Vec2::new(p.x, -p.y)
}

/// Grid px of a world position.
pub fn grid(p: Vec2) -> Vec2 {
    Vec2::new(p.x, -p.y)
}

/// Draw depth for something standing at grid y: further south is in front.
fn depth(grid_y: f32) -> f32 {
    1.0 + grid_y / 10_000.0
}

fn cell_centre((x, y): (i32, i32)) -> Vec2 {
    Vec2::new((x as f32 + 0.5) * CELL, (y as f32 + 0.5) * CELL)
}

/// World-space centre and size of a `w × d`-cell piece anchored at (x, y).
fn piece_rect(x: i32, y: i32, (w, d): (i32, i32)) -> (Vec2, Vec2) {
    let size = Vec2::new(w as f32, d as f32) * CELL;
    let centre = Vec2::new(x as f32 * CELL, y as f32 * CELL) + size / 2.0;
    (world(centre), size)
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
        "Forty kills clears it. They keep pouring out of void fissures.",
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
    /// The Workshop bench: Upgrades and Build.
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
}

/// Room-local spot (cell, then px inside it) of a crew member's feet:
/// design/READINESS.md § Room catalogue.
const CREW_SPOTS: [(RoomId, Crew, (i32, i32)); 3] = [
    (RoomId::Bridge, Crew::Gunner, (0, 1)),
    (RoomId::CrewQuarters, Crew::Researcher, (1, 1)),
    (RoomId::Workshop, Crew::Engineer, (0, 1)),
];
/// Feet inside a crew member's cell, cell-local px.
const CREW_FEET: (f32, f32) = (64.0, 80.0);
/// The Workshop bench: cell (2,0), in front of the back wall.
const BENCH_SPOT: (i32, i32, f32, f32) = (2, 0, 64.0, 72.0);

/// Centre of a Dock's berth pad, room-local px: the pad is the 2 × 2 cells
/// on rows 0–1, the walkway is row 2.
const BERTH_PAD_CENTRE: (f32, f32) = (BERTH_PAD / 2.0, BERTH_PAD / 2.0);

/// The battleship in each Dock: Dock `i` (west to east) holds `SHIPS[i]`.
fn docked_ship(layout: &CarrierLayout, room: &PlacedRoom) -> Option<usize> {
    layout
        .docks()
        .position(|d| d == room)
        .filter(|&i| i < SHIPS.len())
}

/// Every interactable thing on the carrier and where (grid px; for crew,
/// their feet). A Dock's berth hotspot is the middle of its pad's front
/// edge.
pub fn hotspots(layout: &CarrierLayout) -> Vec<(Hotspot, Vec2)> {
    let mut out = Vec::new();
    let at = |room: &PlacedRoom, (cx, cy): (i32, i32), (x, y): (f32, f32)| {
        Vec2::from(room_px(room, (cx as f32 * CELL + x, cy as f32 * CELL + y)))
    };
    for room in &layout.rooms {
        for (id, crew, cell) in CREW_SPOTS {
            if room.id == id {
                out.push((Hotspot::Crew(crew), at(room, cell, CREW_FEET)));
            }
        }
        match room.id {
            RoomId::Workshop => {
                let (cx, cy, x, y) = BENCH_SPOT;
                out.push((Hotspot::UpgradeBench, at(room, (cx, cy), (x, y))));
            }
            RoomId::Dock => {
                if let Some(ship) = docked_ship(layout, room) {
                    let (x, _) = BERTH_PAD_CENTRE;
                    out.push((
                        Hotspot::Berth(ship),
                        Vec2::from(room_px(room, (x, BERTH_PAD))),
                    ));
                }
            }
            _ => {}
        }
    }
    out
}

/// The hotspot E would use from grid px `p`: the closest one within reach.
pub fn nearest_hotspot(hotspots: &[(Hotspot, Vec2)], p: Vec2) -> Option<(Hotspot, Vec2)> {
    hotspots
        .iter()
        .map(|&(h, at)| (h, at, at.distance(p)))
        .filter(|&(_, _, d)| d <= INTERACT_RANGE)
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(h, at, _)| (h, at))
}

/// The crew member or bench drawn under grid px `p` (clicked), if any.
/// Docked ships are hit-tested on their sprites ([`ship_at`]).
pub fn clicked_hotspot(hotspots: &[(Hotspot, Vec2)], p: Vec2) -> Option<Hotspot> {
    hotspots
        .iter()
        .find(|(h, at)| {
            !matches!(h, Hotspot::Berth(_))
                && (p.x - at.x).abs() <= 24.0
                && p.y <= at.y + 8.0
                && p.y >= at.y - CHARACTER_H
        })
        .map(|(h, _)| *h)
}

/// The Workshop panel's two tabs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WorkshopTab {
    #[default]
    Upgrades,
    Build,
}

/// What Build mode does on a click.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Place(Piece),
    Demolish,
}

impl Tool {
    fn label(self) -> String {
        match self {
            Tool::Place(p) => format!("{} ({})", p.name(), cost_text(p.cost())),
            Tool::Demolish => "Demolish (full refund)".into(),
        }
    }
}

/// The Build tab's rows: every piece, then Demolish.
pub const BUILD_TOOLS: [Tool; 4] = [
    Tool::Place(Piece::BUILDABLE[0]),
    Tool::Place(Piece::BUILDABLE[1]),
    Tool::Place(Piece::BUILDABLE[2]),
    Tool::Demolish,
];

pub fn cost_text(c: workshop::Cost) -> String {
    if c.void_crystal > 0 {
        format!("{} cr + {} VC", c.credits, c.void_crystal)
    } else {
        format!("{} cr", c.credits)
    }
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
    /// The Workshop bench panel: its tab, the selected row and the last
    /// action's feedback.
    Workshop {
        tab: WorkshopTab,
        index: usize,
        message: String,
    },
    /// Build mode: the ghost follows the mouse, a click places or
    /// demolishes.
    Build { tool: Tool, message: String },
}

impl Overlay {
    pub fn workshop(tab: WorkshopTab, index: usize, message: String) -> Self {
        Overlay::Workshop {
            tab,
            index,
            message,
        }
    }
}

/// Set by the flow when the Carrier is entered straight from a mission, so the
/// Pilot spawns at the Dock instead of the Bridge.
#[derive(Resource, Default)]
pub struct CarrierArrival {
    pub from_mission: bool,
}

/// The layout as drawn, and what is derived from it.
#[derive(Resource, Default)]
pub struct CarrierScene {
    drawn: Option<CarrierLayout>,
    pub walkable: Vec<Rect>,
    pub hotspots: Vec<(Hotspot, Vec2)>,
}

/// Camera state: the zoom level (index into [`ZOOMS`]) and, in Build mode,
/// the panned view centre (world px) and the zoom to go back to.
#[derive(Resource)]
pub struct CarrierView {
    pub zoom: usize,
    pub pan: Vec2,
    building: Option<usize>,
}

impl Default for CarrierView {
    fn default() -> Self {
        Self {
            zoom: NORMAL,
            pan: Vec2::ZERO,
            building: None,
        }
    }
}

/// The hull cell under the mouse in Build mode (autoplay sets it directly).
#[derive(Resource, Default)]
pub struct BuildCursor(pub Option<(i32, i32)>);

/// Click-to-move: the Pilot's remaining waypoints (grid px), where the
/// move marker is, and what to use on arrival. Empty when standing.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct WalkOrder {
    pub path: Vec<Vec2>,
    pub marker: Option<Vec2>,
    pub interact: Option<Hotspot>,
}

/// Where the Pilot stands to use a hotspot: in front of (south of) a crew
/// member or the bench, on the walkway edge of a berth pad.
const APPROACH: f32 = 24.0;

/// A walk order from the Pilot's feet `from` (grid px) to the floor at
/// `to`, or up to `hotspot` (using it on arrival). `None` when no floor
/// can be reached there.
pub fn order_walk(
    walkable: &[Rect],
    hotspots: &[(Hotspot, Vec2)],
    from: Vec2,
    to: Vec2,
    hotspot: Option<Hotspot>,
) -> Option<WalkOrder> {
    let goal = match hotspot {
        Some(h) => hotspots.iter().find(|(x, _)| *x == h)?.1 + Vec2::Y * APPROACH,
        None => to,
    };
    let path: Vec<Vec2> = layout::find_path(walkable, from.into(), goal.into())?
        .into_iter()
        .map(Vec2::from)
        .collect();
    Some(WalkOrder {
        marker: path.last().copied(),
        path,
        interact: hotspot,
    })
}

/// Moves `feet` up to `dist` px along `order`'s path, dropping reached
/// waypoints. Returns the new feet and whether the path is finished.
pub fn follow(
    walkable: &[Rect],
    order: &mut WalkOrder,
    mut feet: Vec2,
    mut dist: f32,
) -> (Vec2, bool) {
    while let Some(&next) = order.path.first() {
        let to = next - feet;
        let len = to.length();
        let step = if len <= dist { to } else { to * (dist / len) };
        let (x, y) = layout::walk(walkable, feet.into(), step.into());
        let moved = Vec2::new(x, y);
        if moved == feet && step.length() > 0.01 {
            // Blocked (the layout changed under the path): give up.
            order.path.clear();
            break;
        }
        dist -= moved.distance(feet);
        feet = moved;
        if feet.distance(next) < 0.5 {
            order.path.remove(0);
        }
        if dist <= 0.01 {
            break;
        }
    }
    (feet, order.path.is_empty())
}

/// The player character. `walked` (px since it last stood still) drives
/// the walk cycle.
#[derive(Component, Default)]
pub(crate) struct Pilot {
    walked: f32,
    last: Option<Vec2>,
}

#[derive(Component)]
struct Prompt;

/// Everything drawn from the layout; respawned when it changes.
#[derive(Component)]
struct LayoutSprite;

/// Build-mode ghost, slots and reason line; respawned as they change.
#[derive(Component)]
struct BuildUi;

/// A Dock berth: one pad holding one battleship (`ship`, an index into
/// [`SHIPS`]). Spawned by [`berth`] for each ship; the pad itself is part of
/// the Dock room art.
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

/// The corridor art for an opening mask, e.g. `core.carrier.corridor.nes`.
fn corridor_id(mask: layout::Mask) -> String {
    let name = mask.name();
    let name = if name.is_empty() { "nesw".into() } else { name };
    format!("core.carrier.corridor.{name}")
}

fn door_id(side: Side) -> String {
    format!("core.carrier.door.{}", side.letter())
}

/// Every manifest image the Carrier draws (besides dialogue portraits).
#[cfg(test)]
pub fn image_ids() -> Vec<String> {
    let mut ids: Vec<String> = [
        RoomId::Bridge,
        RoomId::CrewQuarters,
        RoomId::Workshop,
        RoomId::Dock,
        RoomId::SalvageBay,
        RoomId::TrainingRoom,
    ]
    .iter()
    .map(|r| r.content_id())
    .collect();
    for mask in [
        "n", "e", "s", "w", "ns", "ew", "ne", "es", "sw", "nw", "nes", "esw", "nsw", "new", "nesw",
    ] {
        ids.push(format!("core.carrier.corridor.{mask}"));
    }
    ids.extend(layout::Side::ALL.iter().map(|s| door_id(*s)));
    ids.extend([HULL_FLOOR.into(), BUILD_SLOT.into()]);
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
            .init_resource::<CarrierScene>()
            .init_resource::<CarrierView>()
            .init_resource::<BuildCursor>()
            .init_resource::<WalkOrder>()
            .add_systems(
                OnEnter(GameScreen::Carrier),
                spawn_carrier.after(flow::enter_carrier),
            )
            .add_systems(OnExit(GameScreen::Carrier), leave_carrier)
            .add_systems(
                Update,
                (
                    sync_layout,
                    click_to_move,
                    carrier_input,
                    build_click,
                    zoom_wheel,
                    build_mode_camera,
                    carrier_camera,
                    track_build_cursor,
                    build_preview,
                    animate_pilot,
                    draw_walk_marker,
                    update_prompt,
                    update_hud,
                    show_selected_ship,
                    carrier_hints,
                    sync_overlay,
                )
                    .chain()
                    // Esc closes an open panel instead of pausing.
                    .after(flow::toggle_pause_menu)
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
/// the character doesn't change how big it looks, and anchored at its
/// visible feet, so the entity's translation is where it stands (walking
/// and collision read the Pilot's translation as its feet).
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
        Anchor(Vec2::new(0.0, bottom - 0.5)),
        Transform::from_translation(pos),
    )
}

/// Dock berth: `SHIPS[ship]` on the pad centred at grid px `pad`, drawn
/// with the same sprite as in battle, nose north, its longest side
/// `ShipInfo::berth_len` on the 256 px pad (design/READINESS.md § Dock and
/// berths). The pad itself is part of the Dock room art.
pub fn berth(
    commands: &mut Commands,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    pad: Vec2,
    ship: usize,
) {
    let centre = world(pad);
    let feet = depth(pad.y + BERTH_PAD / 2.0);
    commands.spawn((
        ScreenEntity,
        LayoutSprite,
        Berth { ship },
        Transform::from_translation(centre.extend(Z_FLOOR)),
    ));
    let image = art.get(images, SHIPS[ship].image_id).unwrap_or_default();
    let px = ContentImages::size(images, &image);
    let size = px * (SHIPS[ship].berth_len / px.max_element());
    commands.spawn((
        ScreenEntity,
        LayoutSprite,
        DockedShip {
            ship,
            half: size / 2.0,
        },
        Sprite {
            image,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(centre.extend(feet)),
    ));
}

/// A flat floor piece: `id` stretched over `size` world px centred at
/// `centre`.
fn floor(
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    id: &str,
    centre: Vec2,
    size: Vec2,
    z: f32,
) -> impl Bundle {
    (
        ScreenEntity,
        Sprite {
            image: art.get(images, id).unwrap_or_default(),
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(centre.extend(z)),
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_carrier(
    mut commands: Commands,
    arrival: Res<CarrierArrival>,
    mut overlay: ResMut<Overlay>,
    mut scene: ResMut<CarrierScene>,
    mut view: ResMut<CarrierView>,
    mut order: ResMut<WalkOrder>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
) {
    *overlay = Overlay::None;
    *order = WalkOrder::default();
    // Drawn from the save by `sync_layout`.
    scene.drawn = None;
    *view = CarrierView::default();
    // Space behind the hull is the nebula sky (`crate::sky`).
    commands.spawn((
        label("Selected", 13.0, Color::srgb(0.5, 0.9, 1.0), Vec3::ZERO),
        SelectedTag,
    ));

    // The Pilot (player character).
    let spawn = if arrival.from_mission {
        SPAWN_AFTER_MISSION
    } else {
        SPAWN_NEW_GAME
    };
    let feet = cell_centre(spawn);
    commands.spawn((
        standing(
            &mut art,
            &mut images,
            PILOT_IDLE,
            CHARACTER_H,
            world(feet).extend(depth(feet.y)),
        ),
        Pilot::default(),
    ));

    commands.spawn((
        label("", 16.0, Color::WHITE, Vec3::new(0.0, 0.0, Z_LABEL + 1.0)),
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

/// Draws `layout`: hull floor, rooms, corridors, doors, room labels, crew
/// and the docked battleships.
fn spawn_layout(
    commands: &mut Commands,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    layout: &CarrierLayout,
    hotspots: &[(Hotspot, Vec2)],
) {
    for y in 0..HULL_H {
        for x in 0..HULL_W {
            let (centre, size) = piece_rect(x, y, (1, 1));
            commands.spawn((
                floor(art, images, HULL_FLOOR, centre, size, Z_HULL),
                LayoutSprite,
            ));
        }
    }
    for room in &layout.rooms {
        let (centre, size) = piece_rect(room.x, room.y, room.id.footprint());
        commands.spawn((
            floor(art, images, &room.id.content_id(), centre, size, Z_FLOOR),
            LayoutSprite,
        ));
        let ship = docked_ship(layout, room);
        let name = match ship {
            Some(i) => format!("{} Dock", SHIPS[i].name),
            None => room.id.name().to_string(),
        };
        commands.spawn((
            label(
                &name,
                15.0,
                Color::srgb(0.8, 0.88, 0.95),
                Vec3::new(centre.x, centre.y + size.y / 2.0 - 16.0, Z_LABEL),
            ),
            LayoutSprite,
        ));
        for (x, y, side) in layout.doors(room) {
            let (cell, _) = piece_rect(x, y, (1, 1));
            let half = CELL / 2.0;
            let (offset, size) = match side {
                Side::N => (Vec2::new(0.0, half - 24.0), Vec2::new(CELL, 48.0)),
                Side::S => (Vec2::new(0.0, -half + 6.0), Vec2::new(CELL, 12.0)),
                Side::E => (Vec2::new(half - 6.0, 0.0), Vec2::new(12.0, CELL)),
                Side::W => (Vec2::new(-half + 6.0, 0.0), Vec2::new(12.0, CELL)),
            };
            commands.spawn((
                floor(art, images, &door_id(side), cell + offset, size, Z_DOOR),
                LayoutSprite,
            ));
        }
        if let Some(ship) = ship {
            let pad = Vec2::from(room_px(room, BERTH_PAD_CENTRE));
            berth(commands, art, images, pad, ship);
            commands.spawn((
                label(
                    "Next: Elimination",
                    13.0,
                    Color::srgb(0.7, 0.9, 1.0),
                    Vec3::new(centre.x, centre.y + size.y / 2.0 - 34.0, Z_LABEL),
                ),
                LayoutSprite,
            ));
        }
    }
    for &(x, y) in &layout.corridors {
        let (centre, size) = piece_rect(x, y, (1, 1));
        commands.spawn((
            floor(
                art,
                images,
                &corridor_id(layout.mask(x, y)),
                centre,
                size,
                Z_FLOOR,
            ),
            LayoutSprite,
        ));
    }
    for &(hotspot, at) in hotspots {
        let Hotspot::Crew(crew) = hotspot else {
            continue;
        };
        commands.spawn((
            standing(
                art,
                images,
                &crew.sprite_id(),
                CHARACTER_H,
                world(at).extend(depth(at.y)),
            ),
            LayoutSprite,
        ));
        commands.spawn((
            label(
                crew.name(),
                13.0,
                crew.color(),
                (world(at) + Vec2::Y * (CHARACTER_H + 10.0)).extend(Z_LABEL),
            ),
            LayoutSprite,
        ));
    }
}

/// Redraws the carrier whenever the saved layout differs from what is on
/// screen (entering the scene, building, demolishing).
fn sync_layout(
    mut commands: Commands,
    save: Res<SaveSlot>,
    mut scene: ResMut<CarrierScene>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<LayoutSprite>>,
) {
    let layout = save
        .game
        .as_ref()
        .map(|g| g.carrier.clone())
        .unwrap_or_default();
    if scene.drawn.as_ref() == Some(&layout) {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let hotspots = hotspots(&layout);
    spawn_layout(&mut commands, &mut art, &mut images, &layout, &hotspots);
    scene.walkable = layout.walkable();
    scene.hotspots = hotspots;
    scene.drawn = Some(layout);
}

fn leave_carrier(
    mut overlay: ResMut<Overlay>,
    mut order: ResMut<WalkOrder>,
    mut arrival: ResMut<CarrierArrival>,
    mut scene: ResMut<CarrierScene>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    *overlay = Overlay::None;
    *order = WalkOrder::default();
    arrival.from_mission = false;
    scene.drawn = None;
    if let Ok((mut cam, mut projection)) = camera.single_mut() {
        cam.translation.x = 0.0;
        cam.translation.y = 0.0;
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scale = 1.0;
        }
    }
}

/// WASD / arrows as a direction in grid space (y south), normalised.
fn wasd(keys: &ButtonInput<KeyCode>) -> Vec2 {
    let mut dir = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        dir.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        dir.x += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        dir.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        dir.y += 1.0;
    }
    dir.normalize_or_zero()
}

/// The Build tab row a tool is on.
fn tool_row(tool: Tool) -> usize {
    BUILD_TOOLS.iter().position(|&t| t == tool).unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
fn carrier_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    pause: Res<PauseMenu>,
    scene: Res<CarrierScene>,
    mut view: ResMut<CarrierView>,
    mut save: ResMut<SaveSlot>,
    mut overlay: ResMut<Overlay>,
    mut order: ResMut<WalkOrder>,
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
    let esc = keys.just_pressed(KeyCode::Escape);
    let back = esc || keys.just_pressed(KeyCode::KeyX) || keys.just_pressed(KeyCode::Backspace);
    let dt = time.delta_secs();

    // Edit a copy so the overlay only reads as changed when it did.
    let mut current = overlay.clone();
    let mut next = None;
    match &mut current {
        Overlay::None => {
            let feet = grid(tf.translation.truncate());
            let dir = wasd(&keys);
            let feet = if dir != Vec2::ZERO || order.path.is_empty() {
                // WASD walks directly and cancels a click-to-move.
                if *order != WalkOrder::default() {
                    *order = WalkOrder::default();
                }
                let step = dir * WALK_SPEED * dt;
                let (x, y) = layout::walk(&scene.walkable, (feet.x, feet.y), (step.x, step.y));
                Vec2::new(x, y)
            } else {
                let (feet, done) = follow(&scene.walkable, &mut order, feet, WALK_SPEED * dt);
                if done {
                    // Arrived: use what was clicked, if it is in reach.
                    let target = order.interact.and_then(|h| {
                        scene
                            .hotspots
                            .iter()
                            .find(|(x, at)| *x == h && at.distance(feet) <= INTERACT_RANGE)
                    });
                    if let Some(&(hotspot, _)) = target {
                        let mut o = Overlay::None;
                        interact_with(hotspot, save.game.as_mut(), &mut o);
                        next = Some(o);
                    }
                    *order = WalkOrder::default();
                }
                feet
            };
            tf.translation = world(feet).extend(depth(feet.y));
            if interact {
                *order = WalkOrder::default();
                if let Some((hotspot, _)) = nearest_hotspot(&scene.hotspots, feet) {
                    let mut o = Overlay::None;
                    interact_with(hotspot, save.game.as_mut(), &mut o);
                    next = Some(o);
                }
            }
        }
        Overlay::Dialogue {
            lines,
            index,
            launch,
        } => {
            if back {
                next = Some(Overlay::None);
            } else if confirm {
                if *index + 1 < lines.len() {
                    *index += 1;
                } else if let Some(ship) = *launch {
                    next = Some(Overlay::Launch { index: ship });
                } else {
                    next = Some(Overlay::None);
                }
            }
        }
        Overlay::Launch { index } => {
            if back {
                next = Some(Overlay::None);
            } else if confirm {
                launch.write(MissionRequest {
                    battleship_id: SHIPS[*index].id.to_string(),
                });
            }
        }
        Overlay::Workshop {
            tab,
            index,
            message,
        } => {
            let rows = match tab {
                WorkshopTab::Upgrades => UPGRADES.len(),
                WorkshopTab::Build => BUILD_TOOLS.len(),
            };
            let up = keys.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
            let down = keys.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
            let digit = [
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
            ]
            .iter()
            .position(|k| keys.just_pressed(*k))
            .filter(|&i| i < rows);
            if keys.just_pressed(KeyCode::Tab) {
                *tab = match tab {
                    WorkshopTab::Upgrades => WorkshopTab::Build,
                    WorkshopTab::Build => WorkshopTab::Upgrades,
                };
                *index = 0;
                message.clear();
            } else if back {
                next = Some(Overlay::None);
            } else if let Some(i) = digit {
                *index = i;
            } else if up {
                *index = (*index + rows - 1) % rows;
            } else if down {
                *index = (*index + 1) % rows;
            } else if confirm {
                let Some(game) = &mut save.game else {
                    return;
                };
                match tab {
                    WorkshopTab::Upgrades => {
                        let upgrade = UPGRADES[*index];
                        *message = match workshop::buy(game, upgrade) {
                            Ok(level) => format!("Installed {} {level}.", upgrade.name()),
                            Err(BuyError::Maxed) => {
                                format!("{} is fully upgraded.", upgrade.name())
                            }
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
                    WorkshopTab::Build => {
                        let tool = BUILD_TOOLS[*index];
                        let lock = match tool {
                            Tool::Place(piece) => workshop::build_lock(game, piece),
                            Tool::Demolish => None,
                        };
                        match lock {
                            Some(e) => *message = format!("{}: {e}.", tool_name(tool)),
                            None => {
                                next = Some(Overlay::Build {
                                    tool,
                                    message: String::new(),
                                })
                            }
                        }
                    }
                }
            }
        }
        Overlay::Build { tool, message } => {
            // The Pilot waits; WASD pans the view.
            let zoom = ZOOMS[view.zoom];
            view.pan += world(wasd(&keys)) * PAN_SPEED * dt / zoom;
            if keys.just_pressed(KeyCode::KeyX) {
                *tool = match tool {
                    Tool::Demolish => Tool::Place(Piece::Corridor),
                    Tool::Place(_) => Tool::Demolish,
                };
                message.clear();
            } else if esc {
                next = Some(Overlay::workshop(
                    WorkshopTab::Build,
                    tool_row(*tool),
                    std::mem::take(message),
                ));
            }
        }
    }
    overlay.set_if_neq(next.unwrap_or(current));
}

fn tool_name(tool: Tool) -> &'static str {
    match tool {
        Tool::Place(p) => p.name(),
        Tool::Demolish => "Demolish",
    }
}

/// What a Build-mode click on `cell` does to the save, and the feedback
/// line. A built room returns to the Build tab; a corridor stays selected.
pub fn apply_build_click(
    game: &mut SaveGame,
    tool: Tool,
    (x, y): (i32, i32),
) -> (bool, String, bool) {
    match tool {
        Tool::Place(piece) => match workshop::build(game, piece, x, y) {
            Ok(()) => (
                true,
                format!("Built {} for {}.", piece.name(), cost_text(piece.cost())),
                matches!(piece, Piece::Room(_)),
            ),
            Err(e) => (false, e.to_string(), false),
        },
        Tool::Demolish => match workshop::demolish(game, x, y) {
            Ok(piece) => (
                true,
                format!(
                    "Demolished {}: {} refunded.",
                    piece.name(),
                    cost_text(piece.cost())
                ),
                false,
            ),
            Err(e) => (false, e.to_string(), false),
        },
    }
}

/// Build mode's mouse: left click places or demolishes at the cursor cell
/// (saving on success), right click goes back to the Build tab.
fn build_click(
    mouse: Res<ButtonInput<MouseButton>>,
    pause: Res<PauseMenu>,
    cursor: Res<BuildCursor>,
    mut save: ResMut<SaveSlot>,
    mut overlay: ResMut<Overlay>,
) {
    if pause.open {
        return;
    }
    let Overlay::Build { tool, message } = &mut *overlay else {
        return;
    };
    if mouse.just_pressed(MouseButton::Right) {
        let back = Overlay::workshop(WorkshopTab::Build, tool_row(*tool), std::mem::take(message));
        *overlay = back;
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let (Some(cell), Some(game)) = (cursor.0, save.game.as_mut()) else {
        return;
    };
    let (changed, text, to_tab) = apply_build_click(game, *tool, cell);
    let tool = *tool;
    if changed {
        // Spec § Save file: saved after every build or demolish.
        save.store();
    }
    *overlay = if to_tab {
        Overlay::workshop(WorkshopTab::Build, tool_row(tool), text)
    } else {
        Overlay::Build {
            tool,
            message: text,
        }
    };
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
        Hotspot::UpgradeBench => Overlay::workshop(WorkshopTab::Upgrades, 0, String::new()),
    };
}

/// The docked ship under world point `p`, if any.
pub fn ship_at(p: Vec2, ships: impl IntoIterator<Item = (Vec2, DockedShip)>) -> Option<usize> {
    ships
        .into_iter()
        .find(|(centre, s)| (p - *centre).abs().cmple(s.half).all())
        .map(|(_, s)| s.ship)
}

fn cursor_world(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), camera.single()) else {
        return None;
    };
    window
        .cursor_position()
        .and_then(|c| camera.viewport_to_world_2d(cam_tf, c).ok())
}

/// Left click, the same as in battle: on the floor, the Pilot walks there
/// along the corridors and rooms; on a docked battleship, a crew member or
/// the bench, it walks up and uses it on arrival (like walking up and
/// pressing E). Build mode handles its own clicks (`build_click`).
#[allow(clippy::too_many_arguments)]
fn click_to_move(
    mouse: Res<ButtonInput<MouseButton>>,
    pause: Res<PauseMenu>,
    overlay: Res<Overlay>,
    scene: Res<CarrierScene>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ships: Query<(&GlobalTransform, &DockedShip)>,
    pilot: Query<&Transform, With<Pilot>>,
    mut order: ResMut<WalkOrder>,
) {
    if pause.open || *overlay != Overlay::None || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let (Some(p), Ok(pilot)) = (cursor_world(&windows, &camera), pilot.single()) else {
        return;
    };
    let docked = ships
        .iter()
        .map(|(tf, s)| (tf.translation().truncate(), *s));
    let hotspot = ship_at(p, docked)
        .map(Hotspot::Berth)
        .or_else(|| clicked_hotspot(&scene.hotspots, grid(p)));
    let feet = grid(pilot.translation.truncate());
    if let Some(o) = order_walk(&scene.walkable, &scene.hotspots, feet, grid(p), hotspot) {
        *order = o;
    }
}

/// The click-to-move destination marker, the same one the battle draws.
fn draw_walk_marker(order: Res<WalkOrder>, mut gizmos: Gizmos) {
    if let Some(at) = order.marker {
        crate::render::move_marker(&mut gizmos, world(at), Crew::Pilot.color());
    }
}

/// Mouse wheel steps between the zoom levels.
fn zoom_wheel(
    scroll: Res<AccumulatedMouseScroll>,
    pause: Res<PauseMenu>,
    mut view: ResMut<CarrierView>,
) {
    if pause.open || scroll.delta.y == 0.0 {
        return;
    }
    view.zoom = if scroll.delta.y > 0.0 {
        (view.zoom + 1).min(ZOOMS.len() - 1)
    } else {
        view.zoom.saturating_sub(1)
    };
}

/// Entering Build mode switches to Overview and pans from where the Pilot
/// stands; leaving it restores the zoom.
fn build_mode_camera(
    overlay: Res<Overlay>,
    mut view: ResMut<CarrierView>,
    pilot: Query<&Transform, With<Pilot>>,
) {
    let building = matches!(*overlay, Overlay::Build { .. });
    match (building, view.building) {
        (true, None) => {
            view.building = Some(view.zoom);
            view.zoom = OVERVIEW;
            if let Ok(tf) = pilot.single() {
                view.pan = tf.translation.truncate();
            }
        }
        (false, Some(zoom)) => {
            view.zoom = zoom;
            view.building = None;
        }
        _ => {}
    }
}

/// Keeps the camera centre within one cell past the hull on each side, or
/// centred on the hull when the view is bigger than that.
pub fn clamp_camera(target: Vec2, half_view: Vec2) -> Vec2 {
    let min = Vec2::new(-CELL, -(HULL_H as f32 + 1.0) * CELL);
    let max = Vec2::new((HULL_W as f32 + 1.0) * CELL, CELL);
    let axis = |t: f32, lo: f32, hi: f32, half: f32| {
        if hi - lo <= 2.0 * half {
            (lo + hi) / 2.0
        } else {
            t.clamp(lo + half, hi - half)
        }
    };
    Vec2::new(
        axis(target.x, min.x, max.x, half_view.x),
        axis(target.y, min.y, max.y, half_view.y),
    )
}

fn carrier_camera(
    mut view: ResMut<CarrierView>,
    pilot: Query<&Transform, (With<Pilot>, Without<Camera2d>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let (Ok(pilot), Ok((mut cam, mut projection))) = (pilot.single(), camera.single_mut()) else {
        return;
    };
    let scale = 1.0 / ZOOMS[view.zoom];
    if let Projection::Orthographic(ortho) = &mut *projection {
        if ortho.scale != scale {
            ortho.scale = scale;
        }
    }
    let size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1280.0, 720.0));
    let half = size / 2.0 * scale;
    let target = if view.building.is_some() {
        view.pan
    } else {
        pilot.translation.truncate()
    };
    let centre = clamp_camera(target, half);
    if view.building.is_some() && view.pan != centre {
        view.pan = centre;
    }
    cam.translation.x = centre.x;
    cam.translation.y = centre.y;
}

/// In Build mode, the hull cell under the mouse.
fn track_build_cursor(
    overlay: Res<Overlay>,
    autoplay: Option<Res<flow::Autoplay>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut cursor: ResMut<BuildCursor>,
) {
    // Autoplay points the cursor itself.
    if autoplay.is_some() || !matches!(*overlay, Overlay::Build { .. }) {
        return;
    }
    if let Some(p) = cursor_world(&windows, &camera) {
        let g = grid(p);
        let cell = Some(layout::cell_at(g.x, g.y));
        if cursor.0 != cell {
            cursor.0 = cell;
        }
    }
}

/// What Build mode shows at the cursor.
#[derive(Clone, PartialEq, Debug)]
struct Ghost {
    image: String,
    /// Anchor cell and footprint.
    at: (i32, i32),
    size: (i32, i32),
    /// Whether a click would succeed.
    ok: bool,
    /// The line over the ghost: the cost, or why not.
    text: String,
}

/// The ghost for `tool` at `cell`.
fn ghost(game: &SaveGame, tool: Tool, (x, y): (i32, i32)) -> Option<Ghost> {
    let layout = &game.carrier;
    let image = |piece: Piece, x: i32, y: i32| match piece {
        Piece::Room(id) => id.content_id(),
        Piece::Corridor => {
            let mut after = layout.clone();
            if layout.cell(x, y) == Cell::Empty {
                after.place(Piece::Corridor, x, y);
            }
            corridor_id(after.mask(x, y))
        }
    };
    match tool {
        Tool::Place(piece) => {
            let result = workshop::can_build(game, piece, x, y);
            let text = match result {
                Ok(()) => format!("{}: {}", piece.name(), cost_text(piece.cost())),
                Err(e) => e.to_string(),
            };
            Some(Ghost {
                image: image(piece, x, y),
                at: (x, y),
                size: piece.footprint(),
                ok: result.is_ok(),
                text,
            })
        }
        Tool::Demolish => {
            let (piece, ox, oy) = layout.piece_origin(x, y)?;
            let result = layout.can_demolish(x, y);
            let text = match result {
                Ok(p) => format!("Demolish {}: +{}", p.name(), cost_text(p.cost())),
                Err(e) => e.to_string(),
            };
            Some(Ghost {
                image: image(piece, ox, oy),
                at: (ox, oy),
                size: piece.footprint(),
                ok: result.is_ok(),
                text,
            })
        }
    }
}

/// Build mode visuals: a slot marker on every empty cell next to the
/// network, and the ghost at the cursor (green = legal, red + why).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_preview(
    mut commands: Commands,
    overlay: Res<Overlay>,
    cursor: Res<BuildCursor>,
    save: Res<SaveSlot>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<BuildUi>>,
    mut drawn: Local<Option<(Overlay, Option<(i32, i32)>, CarrierLayout)>>,
) {
    let game = save.game.clone().unwrap_or_default();
    let key = (overlay.clone(), cursor.0, game.carrier.clone());
    if drawn.as_ref() == Some(&key) {
        return;
    }
    *drawn = Some(key);
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Overlay::Build { tool, .. } = &*overlay else {
        return;
    };
    let preview = cursor.0.and_then(|cell| ghost(&game, *tool, cell));
    let under_ghost = |x: i32, y: i32| {
        preview.as_ref().is_some_and(|g| {
            (g.at.0..g.at.0 + g.size.0).contains(&x) && (g.at.1..g.at.1 + g.size.1).contains(&y)
        })
    };
    if matches!(tool, Tool::Place(_)) {
        for (x, y) in game
            .carrier
            .build_slots()
            .into_iter()
            .filter(|&(x, y)| !under_ghost(x, y))
        {
            let (centre, size) = piece_rect(x, y, (1, 1));
            commands.spawn((
                floor(&mut art, &mut images, BUILD_SLOT, centre, size, Z_SLOT),
                BuildUi,
            ));
        }
    }
    let Some(Ghost {
        image,
        at: (x, y),
        size: footprint,
        ok,
        text,
    }) = preview
    else {
        return;
    };
    let (centre, size) = piece_rect(x, y, footprint);
    let tint = match (ok, tool) {
        (true, Tool::Place(_)) => Color::srgba(0.55, 1.0, 0.55, 0.5),
        (true, Tool::Demolish) => Color::srgba(1.0, 0.8, 0.3, 0.6),
        (false, _) => Color::srgba(1.0, 0.35, 0.35, 0.5),
    };
    commands.spawn((
        ScreenEntity,
        BuildUi,
        Sprite {
            image: art.get(&mut images, &image).unwrap_or_default(),
            custom_size: Some(size),
            color: tint,
            ..default()
        },
        Transform::from_translation(centre.extend(Z_GHOST)),
    ));
    let color = if ok {
        Color::srgb(0.6, 1.0, 0.6)
    } else {
        Color::srgb(1.0, 0.5, 0.45)
    };
    commands.spawn((
        label(
            &text,
            22.0,
            color,
            (centre + Vec2::Y * (size.y / 2.0 + 18.0)).extend(Z_LABEL + 2.0),
        ),
        BuildUi,
    ));
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
    scene: Res<CarrierScene>,
    pilot: Query<&Transform, With<Pilot>>,
    mut hints: ResMut<Hints>,
    mut save: ResMut<SaveSlot>,
) {
    let Ok(pilot) = pilot.single() else {
        return;
    };
    let feet = grid(pilot.translation.truncate());
    if nearest_hotspot(&scene.hotspots, feet).is_some() {
        hints.trigger(&mut save, Hint::CarrierInteract);
    }
    if room_under(&save, feet) == Some(RoomId::Dock) {
        hints.trigger(&mut save, Hint::Dock);
    }
    match *overlay {
        Overlay::Workshop { .. } => {
            hints.trigger(&mut save, Hint::Workshop);
        }
        Overlay::Build { .. } => {
            hints.trigger(&mut save, Hint::CarrierBuild);
        }
        _ => {}
    }
}

/// The room the grid point `p` is in, if any.
fn room_under(save: &SaveSlot, p: Vec2) -> Option<RoomId> {
    let layout = &save.game.as_ref()?.carrier;
    let (x, y) = layout::cell_at(p.x, p.y);
    match layout.cell(x, y) {
        Cell::Room(i) => Some(layout.rooms[i].id),
        _ => None,
    }
}

/// Walk-cycle frame for the Pilot after `walked` px; idle when standing.
fn pilot_frame(moving: bool, walked: f32) -> &'static str {
    if !moving {
        return PILOT_IDLE;
    }
    PILOT_FRAMES[(walked / STRIDE) as usize % PILOT_FRAMES.len()]
}

/// The side-view walk sheet: flipped when walking west, the last facing
/// kept for north and south (§ Walking and interaction › Pilot art).
fn animate_pilot(
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut pilot: Query<(&mut Pilot, &Transform, &mut Sprite)>,
) {
    let Ok((mut pilot, tf, mut sprite)) = pilot.single_mut() else {
        return;
    };
    let p = tf.translation.truncate();
    let d = p - pilot.last.unwrap_or(p);
    pilot.last = Some(p);
    let moving = d.length() > 0.01;
    if moving {
        pilot.walked += d.length();
        // The art faces right.
        if d.x.abs() > 0.01 {
            sprite.flip_x = d.x < 0.0;
        }
    } else {
        pilot.walked = 0.0;
    }
    if let Some(image) = art.get(&mut images, pilot_frame(moving, pilot.walked)) {
        if sprite.image != image {
            sprite.image = image;
        }
    }
}

fn update_prompt(
    overlay: Res<Overlay>,
    scene: Res<CarrierScene>,
    pilot: Query<&Transform, (With<Pilot>, Without<Prompt>)>,
    mut prompt: Query<(&mut Text2d, &mut Transform, &mut Visibility), With<Prompt>>,
) {
    let (Ok(pilot), Ok((mut text, mut tf, mut vis))) = (pilot.single(), prompt.single_mut()) else {
        return;
    };
    let near = nearest_hotspot(&scene.hotspots, grid(pilot.translation.truncate()));
    match near {
        Some((hotspot, at)) if *overlay == Overlay::None => {
            let label = format!("[E] {}", hotspot.verb());
            if text.0 != label {
                text.0 = label;
            }
            // Over the crew member or bench; for a docked ship, under the
            // Pilot's feet (the ship fills the pad above the hotspot).
            let pos = match hotspot {
                Hotspot::Berth(_) => pilot.translation.truncate() - Vec2::Y * 16.0,
                Hotspot::UpgradeBench => world(at) + Vec2::Y * 40.0,
                Hotspot::Crew(_) => world(at) + Vec2::Y * (CHARACTER_H + 28.0),
            };
            tf.translation = pos.extend(Z_LABEL + 1.0);
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
        .map(|g| (g.credits, workshop::void_crystal(g)))
        .unwrap_or_default();
    let room = pilot
        .single()
        .ok()
        .and_then(|tf| {
            let feet = grid(tf.translation.truncate());
            let layout = &save.game.as_ref()?.carrier;
            let (x, y) = layout::cell_at(feet.x, feet.y);
            Some(match layout.cell(x, y) {
                Cell::Room(i) => layout.rooms[i].id.name(),
                Cell::Corridor => "Corridor",
                Cell::Empty => "",
            })
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
        Overlay::Workshop {
            tab,
            index,
            message,
        } => {
            let tab_color = |t: WorkshopTab| {
                if t == *tab {
                    Color::srgb(0.5, 0.9, 1.0)
                } else {
                    Color::srgb(0.45, 0.5, 0.55)
                }
            };
            let row_bg = |selected: bool| {
                (
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
                )
            };
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
                    p.spawn(Node {
                        column_gap: px(24),
                        ..default()
                    })
                    .with_children(|tabs| {
                        tabs.spawn((Text::new("Workshop"), font(22.0, Color::WHITE)));
                        tabs.spawn((
                            Text::new("Upgrades"),
                            font(20.0, tab_color(WorkshopTab::Upgrades)),
                        ));
                        tabs.spawn((
                            Text::new("Build"),
                            font(20.0, tab_color(WorkshopTab::Build)),
                        ));
                        tabs.spawn((
                            Text::new("[Tab] switch"),
                            font(15.0, Color::srgb(0.6, 0.7, 0.75)),
                        ));
                    });
                    p.spawn((
                        Text::new(format!(
                            "Credits {}    Void Crystal {}",
                            game.credits,
                            workshop::void_crystal(&game)
                        )),
                        font(18.0, Color::srgb(0.95, 0.9, 0.6)),
                    ));
                    match tab {
                        WorkshopTab::Upgrades => {
                            p.spawn((
                                Text::new("Upgrades apply to every battleship."),
                                font(15.0, Color::srgb(0.82, 0.86, 0.9)),
                            ));
                            for (i, upgrade) in UPGRADES.iter().enumerate() {
                                let level = upgrade.level_in(levels);
                                let (price, affordable) =
                                    match workshop::next_level(&game, *upgrade) {
                                        Some((next, cost)) => (
                                            format!(
                                                "Lv {next}: {} cr + {} VC",
                                                cost.credits, cost.void_crystal
                                            ),
                                            workshop::can_afford(&game, cost),
                                        ),
                                        None => ("maxed".to_string(), false),
                                    };
                                let color = if affordable {
                                    Color::srgb(0.75, 1.0, 0.75)
                                } else {
                                    Color::srgb(0.7, 0.72, 0.75)
                                };
                                p.spawn(row_bg(i == *index)).with_children(|row| {
                                    if let Some(icon) = art.get(&mut images, upgrade.icon_id()) {
                                        row.spawn(icon_node(icon, 24.0));
                                    }
                                    let cells = [
                                        (format!("{} {}", i + 1, upgrade.name()), 180.0),
                                        (
                                            format!(
                                                "Lv {level}/{}",
                                                sim::tuning::MAX_UPGRADE_LEVEL
                                            ),
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
                            let upgrade = UPGRADES[(*index).min(UPGRADES.len() - 1)];
                            p.spawn((
                                Text::new(format!(
                                    "{} next level:\n  {}",
                                    upgrade.name(),
                                    upgrade_preview(upgrade, levels).join("\n  ")
                                )),
                                font(15.0, Color::srgb(0.82, 0.86, 0.9)),
                            ));
                        }
                        WorkshopTab::Build => {
                            p.spawn((
                                Text::new(
                                    "Rooms connect through a corridor at one of their doors.",
                                ),
                                font(15.0, Color::srgb(0.82, 0.86, 0.9)),
                            ));
                            for (i, tool) in BUILD_TOOLS.iter().enumerate() {
                                let (size, effect, cost, lock) = match tool {
                                    Tool::Place(piece) => {
                                        let (w, d) = piece.footprint();
                                        (
                                            format!("{w} x {d}"),
                                            piece.effect().to_string(),
                                            cost_text(piece.cost()),
                                            match workshop::build_lock(&game, *piece) {
                                                Some(PlaceError::AlreadyBuilt) => {
                                                    "already built".into()
                                                }
                                                Some(_) => "can't afford".into(),
                                                None => String::new(),
                                            },
                                        )
                                    }
                                    Tool::Demolish => (
                                        String::new(),
                                        "remove a piece you built ([X] in Build mode)".into(),
                                        "full refund".into(),
                                        String::new(),
                                    ),
                                };
                                let color = if lock.is_empty() {
                                    Color::srgb(0.75, 1.0, 0.75)
                                } else {
                                    Color::srgb(0.7, 0.72, 0.75)
                                };
                                p.spawn(row_bg(i == *index)).with_children(|row| {
                                    let cells = [
                                        (format!("{} {}", i + 1, tool_name(*tool)), 170.0),
                                        (size, 50.0),
                                        (effect, 330.0),
                                        (cost, 130.0),
                                        (lock, 130.0),
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
                        }
                    }
                    if !message.is_empty() {
                        p.spawn((
                            Text::new(message.clone()),
                            font(16.0, Color::srgb(1.0, 0.85, 0.4)),
                        ));
                    }
                    let keys = match tab {
                        WorkshopTab::Upgrades => {
                            "[W]/[S] or [1]-[3] Choose    [E] Buy    [Tab] Build    [X] Close"
                        }
                        WorkshopTab::Build => {
                            "[W]/[S] or [1]-[4] Choose    [E] Pick    [Tab] Upgrades    [X] Close"
                        }
                    };
                    p.spawn((Text::new(keys), font(15.0, Color::srgb(0.6, 0.7, 0.75))));
                });
        }
        Overlay::Build { tool, message } => {
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
                        bottom: px(14),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(px(12), px(8)),
                        row_gap: px(4),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new(format!(
                            "Build mode: {}    Credits {}    Void Crystal {}",
                            tool.label(),
                            game.credits,
                            workshop::void_crystal(&game)
                        )),
                        font(18.0, Color::WHITE),
                    ));
                    if !message.is_empty() {
                        p.spawn((
                            Text::new(message.clone()),
                            font(16.0, Color::srgb(1.0, 0.85, 0.4)),
                        ));
                    }
                    p.spawn((
                        Text::new(
                            "[Left click] Place    [X] Demolish mode    [WASD] Pan    [Wheel] Zoom    [Right click]/[Esc] Back",
                        ),
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
    fn pilot_walk_cycle_steps_with_distance_and_idles_when_still() {
        assert_eq!(pilot_frame(false, 50.0), PILOT_IDLE);
        assert_eq!(pilot_frame(true, 0.0), PILOT_FRAMES[0]);
        assert_eq!(pilot_frame(true, STRIDE * 2.5), PILOT_FRAMES[2]);
        assert_eq!(pilot_frame(true, STRIDE * 5.0), PILOT_FRAMES[1]);
    }

    #[test]
    fn crew_bench_and_berths_sit_in_their_rooms_and_are_reachable() {
        let l = CarrierLayout::starting();
        let spots = hotspots(&l);
        let room_of = |p: Vec2| {
            let (x, y) = layout::cell_at(p.x, p.y - 1.0);
            match l.cell(x, y) {
                Cell::Room(i) => Some(l.rooms[i].id),
                _ => None,
            }
        };
        let expect = [
            (Hotspot::Crew(Crew::Gunner), RoomId::Bridge),
            (Hotspot::Crew(Crew::Researcher), RoomId::CrewQuarters),
            (Hotspot::Crew(Crew::Engineer), RoomId::Workshop),
            (Hotspot::UpgradeBench, RoomId::Workshop),
            (Hotspot::Berth(0), RoomId::Dock),
            (Hotspot::Berth(1), RoomId::Dock),
        ];
        assert_eq!(spots.len(), expect.len());
        let rects = l.walkable();
        for (h, room) in expect {
            let at = spots.iter().find(|s| s.0 == h).unwrap().1;
            assert_eq!(room_of(at), Some(room), "{h:?}");
            // Somewhere the Pilot can stand reaches it with E.
            let reachable = (-7..=7).any(|i| {
                (-7..=7).any(|j| {
                    let p = at + Vec2::new(i as f32, j as f32) * 8.0;
                    layout::can_stand(&rects, p.x, p.y)
                        && nearest_hotspot(&spots, p).map(|n| n.0) == Some(h)
                })
            });
            assert!(reachable, "{h:?} at {at}");
        }
        // One ship per Dock, west to east; the hotspot is the middle of the
        // pad's front edge.
        for (i, dock) in l.docks().enumerate() {
            assert_eq!(
                spots.iter().find(|s| s.0 == Hotspot::Berth(i)).unwrap().1,
                Vec2::from(room_px(dock, (128.0, 256.0)))
            );
        }
    }

    #[test]
    fn clicking_a_ship_walks_there_and_boards_it() {
        let l = CarrierLayout::starting();
        let rects = l.walkable();
        let spots = hotspots(&l);
        let start = cell_centre(SPAWN_NEW_GAME);
        let mut order = order_walk(&rects, &spots, start, Vec2::ZERO, Some(Hotspot::Berth(1)))
            .expect("Bulwark's Dock is reachable");
        assert_eq!(order.marker, order.path.last().copied());
        // Walk it at 60 fps.
        let mut feet = start;
        let mut done = false;
        for _ in 0..60 * 30 {
            (feet, done) = follow(&rects, &mut order, feet, WALK_SPEED / 60.0);
            assert!(layout::can_stand(&rects, feet.x, feet.y), "{feet}");
            if done {
                break;
            }
        }
        assert!(done);
        assert_eq!(
            nearest_hotspot(&spots, feet).map(|h| h.0),
            Some(Hotspot::Berth(1))
        );
        // A floor click has no hotspot; deep space is not a destination.
        let floor = order_walk(&rects, &spots, start, cell_centre((3, 4)), None).unwrap();
        assert_eq!(floor.interact, None);
        assert!(order_walk(&rects, &spots, start, cell_centre((10, 0)), None).is_none());
    }

    #[test]
    fn spawns_stand_on_the_floor_away_from_hotspots() {
        let l = CarrierLayout::starting();
        let rects = l.walkable();
        let spots = hotspots(&l);
        for cell in [SPAWN_NEW_GAME, SPAWN_AFTER_MISSION] {
            let p = cell_centre(cell);
            assert!(layout::can_stand(&rects, p.x, p.y), "{cell:?}");
            assert_eq!(nearest_hotspot(&spots, p), None, "{cell:?}");
        }
    }

    #[test]
    fn nearest_hotspot_respects_range() {
        let spots = [(Hotspot::UpgradeBench, Vec2::new(100.0, 100.0))];
        assert_eq!(
            nearest_hotspot(&spots, Vec2::new(100.0 + INTERACT_RANGE, 100.0)).map(|h| h.0),
            Some(Hotspot::UpgradeBench)
        );
        assert_eq!(nearest_hotspot(&spots, Vec2::new(140.0, 141.0)), None);
        assert_eq!(
            clicked_hotspot(&spots, Vec2::new(110.0, 60.0)),
            Some(Hotspot::UpgradeBench)
        );
        assert_eq!(clicked_hotspot(&spots, Vec2::new(130.0, 60.0)), None);
    }

    #[test]
    fn camera_stays_within_a_cell_of_the_hull() {
        let half = Vec2::new(640.0, 360.0);
        // Pilot at the north-west corner: the view stops one cell out.
        let c = clamp_camera(Vec2::new(0.0, 0.0), half);
        assert_eq!(c, Vec2::new(-CELL + 640.0, CELL - 360.0));
        // Overview (1 / 0.65 scale): the whole hull fits, so it's centred.
        let c = clamp_camera(Vec2::ZERO, half / 0.65);
        assert_eq!(c.x, HULL_W as f32 * CELL / 2.0);
        assert_eq!(world(grid(Vec2::new(3.0, -4.0))), Vec2::new(3.0, -4.0));
    }

    #[test]
    fn build_clicks_pay_place_and_refund() {
        let mut game = SaveGame {
            credits: 300,
            ..Default::default()
        };
        game.resources.insert("void_crystal".into(), 6);
        let (ok, text, to_tab) = apply_build_click(&mut game, Tool::Place(Piece::Corridor), (8, 6));
        assert!(ok && !to_tab, "{text}");
        assert_eq!(text, "Built Corridor for 10 cr.");
        let bay = Tool::Place(Piece::Room(RoomId::SalvageBay));
        let (ok, text, _) = apply_build_click(&mut game, bay, (9, 4));
        assert!(!ok);
        assert_eq!(text, "Must connect to a corridor");
        let (ok, _, to_tab) = apply_build_click(&mut game, bay, (9, 5));
        assert!(ok && to_tab);
        assert_eq!(game.credits, 170);
        let (ok, text, _) = apply_build_click(&mut game, Tool::Demolish, (8, 6));
        assert!(!ok);
        assert_eq!(text, "Something would be cut off");
        let (ok, text, _) = apply_build_click(&mut game, Tool::Demolish, (10, 6));
        assert!(ok, "{text}");
        assert_eq!(game.credits, 290);
    }

    #[test]
    fn ghost_shows_the_shaped_corridor_and_why_not() {
        let game = SaveGame::default();
        let g = ghost(&game, Tool::Place(Piece::Corridor), (8, 6)).unwrap();
        assert_eq!(g.image, "core.carrier.corridor.w");
        assert_eq!((g.at, g.size, g.ok), ((8, 6), (1, 1), false));
        assert_eq!(g.text, "Need 10 cr");
        let g = ghost(&game, Tool::Demolish, (3, 4)).unwrap();
        assert!(!g.ok);
        assert_eq!(g.text, "Part of the original carrier");
        assert!(ghost(&game, Tool::Demolish, (10, 0)).is_none());
        // A room's ghost is anchored at the mouse cell.
        let g = ghost(
            &game,
            Tool::Place(Piece::Room(RoomId::TrainingRoom)),
            (7, 0),
        )
        .unwrap();
        assert_eq!(g.image, "core.carrier.room.training_room");
        assert_eq!((g.at, g.size), ((7, 0), (2, 2)));
    }

    #[test]
    fn dock_card_shows_training_room_cooldowns() {
        let up = Upgrades {
            training: 1,
            ..Default::default()
        };
        let kite = ship_card(&SHIPS[0], up);
        assert!(kite.contains("CD 3.4 s"), "{kite}");
        assert!(kite.contains("CD 5.1 s"), "{kite}");
        let bulwark = ship_card(&SHIPS[1], up);
        assert!(bulwark.contains("CD 10.2 s"), "{bulwark}");
        assert!(bulwark.contains("CD 7.65 s"), "{bulwark}");
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
            training: 0,
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
