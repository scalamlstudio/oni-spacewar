//! Carrier scene: the 2.5D (3/4 top-down) carrier the Pilot walks around
//! between missions and builds on in Build mode (design/READINESS.md
//! § Demo Spec › Carrier, Revision 3). Client only; nothing here touches
//! the sim.
//!
//! The layout rules (rooms, the tiles they make, items, construction
//! sites, what may go where, the walkable area) are in `crate::layout`;
//! this module draws the save's `layout::Carrier`, walks the Pilot over it,
//! runs the Upgrade Bench panel and Build mode, and advances construction
//! timers while the scene is open. Every image is shipped art loaded by
//! stable content ID (`crate::art`).

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::ui::FocusPolicy;
use bevy::window::PrimaryWindow;

use sim::tuning::{
    AFTERBURN_DISTANCE, BASTION_DURATION, BASTION_SHIELD, SCATTER_BOLTS, SHOCKWAVE_PUSH,
    SHOCKWAVE_RADIUS, TICKS_PER_SEC,
};
use sim::{Loadout, ShipKind, ShipSheet, Upgrades};

use crate::art::{self, icon_node, ContentImages};
use crate::flow::{self, GameScreen, PauseMenu, SaveSlot, ScreenEntity};
use crate::hints::{Hint, Hints};
use crate::input::{self, MovePress};
use crate::layout::{
    self, Carrier, Category, Count, Item, ItemId, Layer, Reason, Rect, Room, RoomSize, Tile,
    TileKind, CATALOGUE, HULL_H, HULL_W, TILE,
};
use crate::mission::MissionRequest;
use crate::save::{LastResult, SaveGame};
use crate::settings::{self, CarrierPalette};
use crate::workshop::{self, BuyError, Cost, Upgrade, UPGRADES};
use crate::zoom::{self, Zoom};

/// Walking speed, px/s (8 directions, diagonals normalised).
pub const WALK_SPEED: f32 = 160.0;
/// E reaches the nearest hotspot within this many px of the Pilot's feet.
pub const INTERACT_RANGE: f32 = 56.0;
/// Build mode's zoom, screen px per world px: Overview, the whole hull in
/// 1280 × 720. The mouse wheel zooms between `zoom::MIN` and `zoom::MAX`
/// about the cursor (`crate::zoom`).
pub const OVERVIEW: f32 = zoom::MIN;
/// Build-mode camera pan, screen px/s.
const PAN_SPEED: f32 = 600.0;
/// The camera never shows more than this far past the hull, world px
/// (§ Grid and view › Camera: 2 tiles).
const CAMERA_MARGIN: f32 = 2.0 * TILE;
/// Height of the Pilot and the crew from head to feet, px. The art has
/// different amounts of empty canvas, so this is measured on the visible
/// pixels (`standing`), not the image size. 56 px is 0.875 of a 64 px
/// Revision 3 tile (TAKOAI-94: characters smaller than a tile); Art draws
/// it at 112 px at 2× (design/READINESS.md § Carrier › Art spec).
pub const CHARACTER_H: f32 = 56.0;
/// Pilot walk cycle: a new frame every this many px walked.
const STRIDE: f32 = 12.0;
/// Walk-cycle frames after the idle one, per drawn direction.
const PILOT_WALK: [&str; 4] = ["walk_1", "walk_2", "walk_3", "walk_4"];
/// Rooms under construction wear this tileable overlay; station front
/// strips show this chevron in Build mode.
const SCAFFOLD: &str = "core.carrier.scaffold";
const FRONT_MARKER: &str = "core.carrier.front_marker";
/// The builder UI kit (design/art/demo-v2/builder-ui, TAKOAI-65), with the
/// 9-slice insets its README gives.
const UI_PANEL: (&str, f32) = ("core.ui.builder.panel.outer_panel", 24.0);
/// The cards slice at 28 px, not the README's 18: the selected card's
/// corner brackets reach past 18 px and stretched along the edges.
const UI_CARD: (&str, f32) = ("core.ui.builder.panel.card", 28.0);
const UI_CARD_SELECTED: (&str, f32) = ("core.ui.builder.panel.selected_card", 28.0);
const UI_CARD_DISABLED: (&str, f32) = ("core.ui.builder.panel.disabled_card", 28.0);
/// Text insets that clear the panel's dark rim and the selected card's
/// gold brackets (TAKOAI-67). Cards draw their corners at
/// `UI_CARD_CORNER_SCALE` so the brackets leave room for the text.
const UI_PANEL_PAD: f32 = 18.0;
const UI_CARD_PAD: f32 = 12.0;
const UI_CARD_CORNER_SCALE: f32 = 0.5;
const GHOST_VALID: &str = "core.ui.builder.ghost.valid";
const GHOST_INVALID: &str = "core.ui.builder.ghost.invalid";
const GRID_OVERLAY: &str = "core.ui.builder.grid_cell_overlay";
/// The kit's category icons Build mode reuses: Rooms, and the demolish
/// icon for Remove.
const ROOMS_ICON: &str = "core.ui.builder.category.rooms";
const REMOVE_ICON: &str = "core.ui.builder.category.demolish";
/// Construction sites draw in the kit's blueprint tint (§ Construction
/// time).
const BLUEPRINT: Color = Color::srgba(0.55, 0.82, 1.0, 0.6);
/// A doorway that stays shut while one of its rooms is being built.
const SHUT: Color = Color::srgb(0.45, 0.45, 0.5);
/// Draw layers (§ Grid and view › Draw order): floor, walls and doorways
/// (the wall art carries its own face), scaffolds, rugs, wall-mounted
/// items; then objects and characters sorted by their feet (`depth`, 1 and
/// up); Build mode's ghost and labels over everything.
const Z_FLOOR: f32 = -20.0;
const Z_WALL: f32 = -19.5;
const Z_SCAFFOLD: f32 = -19.3;
const Z_RUG: f32 = -19.0;
const Z_WALL_ITEM: f32 = -18.8;
const Z_GRID: f32 = -18.5;
const Z_GHOST: f32 = 4.0;
const Z_LABEL: f32 = 5.0;
/// Construction progress bar, world px.
const BAR_H: f32 = 6.0;
/// Behind every deck label (`label`).
const LABEL_BACKING: Color = Color::srgba(0.03, 0.05, 0.08, 0.75);

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

/// Grid px of a tile's centre.
pub fn tile_centre((x, y): Tile) -> Vec2 {
    Vec2::new((x as f32 + 0.5) * TILE, (y as f32 + 0.5) * TILE)
}

/// Grid px centre and world px size of a `w × d`-tile block anchored at
/// (x, y).
fn tile_rect((x, y): Tile, (w, d): (i32, i32)) -> (Vec2, Vec2) {
    let size = Vec2::new(w as f32, d as f32) * TILE;
    (Vec2::new(x as f32, y as f32) * TILE + size / 2.0, size)
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
    /// The Upgrade Bench: the Workshop upgrades.
    UpgradeBench,
    /// The battleship on the berth holding `SHIPS[n]`.
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

/// The autoplay's first-visit Pilot tour (TAKOAI-74): eight straight
/// walks on the Bridge floor from the new-game spawn (the Command
/// Console's use point, (256, 152)), one per facing, each at least 148 px
/// (0.9 s) long so a shot mid-walk is mid-stride. The Bridge's open floor
/// is rows 2–4, so the walks run between its top line (y 152) and bottom
/// line (y 300), clear of the Gunner at tile (5,2). Grid px targets and the
/// facing's name.
pub const PILOT_TOUR: [(Vec2, &str); 8] = [
    (Vec2::new(256.0, 300.0), "s"),
    (Vec2::new(100.0, 300.0), "w"),
    (Vec2::new(248.0, 152.0), "ne"),
    (Vec2::new(396.0, 300.0), "se"),
    (Vec2::new(396.0, 152.0), "n"),
    (Vec2::new(248.0, 300.0), "sw"),
    (Vec2::new(396.0, 300.0), "e"),
    (Vec2::new(248.0, 152.0), "nw"),
];

/// The autoplay's hold-to-move drag (TAKOAI-87), after the tour ends at
/// its last stop: the held cursor sweeps west, south, then east along these
/// Bridge floor points (grid px), one leg per `PILOT_DRAG_LEG` s, faster
/// than the Pilot walks so it trails the cursor.
pub const PILOT_DRAG: [Vec2; 4] = [
    Vec2::new(248.0, 152.0),
    Vec2::new(100.0, 152.0),
    Vec2::new(100.0, 300.0),
    Vec2::new(396.0, 300.0),
];
pub const PILOT_DRAG_LEG: f32 = 0.7;

/// The autoplay drag's cursor `t` s after the press.
pub fn drag_cursor(t: f32) -> Vec2 {
    let legs = (PILOT_DRAG.len() - 1) as f32;
    let at = (t / PILOT_DRAG_LEG).clamp(0.0, legs);
    let i = (at as usize).min(PILOT_DRAG.len() - 2);
    PILOT_DRAG[i].lerp(PILOT_DRAG[i + 1], at - i as f32)
}

/// The crew members with a post (§ Walking and interaction › Crew posts).
const POSTED: [(Crew, layout::Crew); 3] = [
    (Crew::Gunner, layout::Crew::Gunner),
    (Crew::Researcher, layout::Crew::Researcher),
    (Crew::Engineer, layout::Crew::Engineer),
];

/// Every interactable thing on the carrier and where (grid px): each crew
/// member's feet at their post (the post tile's centre), and the use point
/// of every finished station that does something in the demo, the Upgrade
/// Bench and each berth with its ship. Recomputed after every edit, so
/// crew follow their station when it moves.
pub fn hotspots(carrier: &Carrier) -> Vec<(Hotspot, Vec2)> {
    let mut out: Vec<(Hotspot, Vec2)> = POSTED
        .iter()
        .filter_map(|&(crew, post)| {
            let t = carrier.crew_post(post)?;
            Some((Hotspot::Crew(crew), tile_centre(t)))
        })
        .collect();
    for item in carrier.items.iter().filter(|i| i.finished()) {
        if let Some(h) = station_hotspot(item) {
            out.push((h, Vec2::from(item.use_point())));
        }
    }
    out
}

/// The hotspot a station is, if using it does something.
fn station_hotspot(item: &Item) -> Option<Hotspot> {
    match item.id {
        ItemId::UpgradeBench => Some(Hotspot::UpgradeBench),
        ItemId::Berth => Some(Hotspot::Berth(ship_index(item.ship.as_deref()?))),
        _ => None,
    }
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

/// What a click at grid px `p` uses: a crew member drawn there, else the
/// usable station on the tile under it. Station art rises into the tile
/// north of its footprint, so a click there counts for the station below.
pub fn clicked_hotspot(
    hotspots: &[(Hotspot, Vec2)],
    carrier: &Carrier,
    p: Vec2,
) -> Option<Hotspot> {
    let crew = hotspots.iter().find(|(h, at)| {
        matches!(h, Hotspot::Crew(_))
            && (p.x - at.x).abs() <= 24.0
            && p.y <= at.y + 8.0
            && p.y >= at.y - CHARACTER_H
    });
    if let Some(&(h, _)) = crew {
        return Some(h);
    }
    let (x, y) = layout::tile_at(p.x, p.y);
    [(x, y), (x, y + 1)].into_iter().find_map(|t| {
        let item = &carrier.items[carrier.item_at(t, Layer::Object)?];
        let h = station_hotspot(item)?;
        hotspots.iter().any(|(x, _)| *x == h).then_some(h)
    })
}

/// Where the Pilot appears (§ Walking and interaction › Spawn): after a
/// mission, the use point of the launched ship's berth; otherwise (a new
/// game, Continue) the Command Console's use point.
pub fn spawn_point(carrier: &Carrier, launched: Option<&str>) -> Vec2 {
    launched
        .and_then(|ship| carrier.berth_of(ship))
        .or_else(|| carrier.find(ItemId::CommandConsole))
        .map_or(tile_centre((2, 2)), |i| {
            Vec2::from(carrier.items[i].use_point())
        })
}

/// Something Build mode can place: a room shell or a catalogue item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    Room(RoomSize),
    Item(ItemId),
}

impl Entry {
    pub fn name(self) -> &'static str {
        match self {
            Entry::Room(size) => size.name(),
            Entry::Item(id) => id.name(),
        }
    }

    pub fn cost(self) -> Cost {
        match self {
            Entry::Room(size) => size.cost(),
            Entry::Item(id) => id.def().map_or(Cost::default(), |d| d.cost),
        }
    }

    pub fn build_seconds(self) -> u32 {
        match self {
            Entry::Room(size) => size.build_seconds(),
            Entry::Item(id) => id.def().map_or(0, |d| d.build_seconds),
        }
    }

    /// Footprint in tiles; a room's includes its walls.
    pub fn size(self) -> (i32, i32) {
        match self {
            Entry::Room(size) => {
                let (w, d) = size.floor();
                (w + 2, d + 2)
            }
            Entry::Item(id) => item_size(id),
        }
    }
}

/// An item's footprint, w × d tiles.
fn item_size(id: ItemId) -> (i32, i32) {
    id.def().map_or((1, 1), |d| d.size)
}

/// The anchor that puts the middle tile (rounded north-west) of a `size`
/// footprint on the mouse tile (§ Building › Item ghost).
pub fn item_anchor((w, d): (i32, i32), (x, y): Tile) -> Tile {
    (x - (w - 1) / 2, y - (d - 1) / 2)
}

/// The mouse tile that anchors a room of `size` at `anchor`: its middle
/// floor tile (the inverse of `Room::anchor_for`).
pub fn room_mouse(size: RoomSize, (x, y): Tile) -> Tile {
    let (w, d) = size.floor();
    (x + (w - 1) / 2 + 1, y + (d - 1) / 2 + 1)
}

/// What a Build-mode click does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Place(Entry),
    /// Lift a finished item (`None`), or put down the one held (its index
    /// into `Carrier::items`; nothing else edits the layout while it is
    /// held, so the index holds).
    Move(Option<usize>),
    Remove,
}

/// Build mode's category bar: four flyouts of cards, then the Move and
/// Remove tools (§ Building › Screen).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuildCategory {
    Rooms,
    Stations,
    Furniture,
    Decor,
    Move,
    Remove,
}

impl BuildCategory {
    pub const ALL: [BuildCategory; 6] = [
        BuildCategory::Rooms,
        BuildCategory::Stations,
        BuildCategory::Furniture,
        BuildCategory::Decor,
        BuildCategory::Move,
        BuildCategory::Remove,
    ];

    pub fn of(tool: Tool) -> Self {
        match tool {
            Tool::Place(Entry::Room(_)) => BuildCategory::Rooms,
            Tool::Place(Entry::Item(id)) => match id.def().map(|d| d.category) {
                Some(Category::Station) => BuildCategory::Stations,
                Some(Category::Decor) => BuildCategory::Decor,
                _ => BuildCategory::Furniture,
            },
            Tool::Move(_) => BuildCategory::Move,
            Tool::Remove => BuildCategory::Remove,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BuildCategory::Rooms => "Rooms",
            BuildCategory::Stations => "Stations",
            BuildCategory::Furniture => "Furniture",
            BuildCategory::Decor => "Decor",
            BuildCategory::Move => "Move",
            BuildCategory::Remove => "Remove",
        }
    }

    /// Its button's icon: the kit's where there is one, the Command
    /// Console for Stations; the others show their name only.
    fn icon_id(self) -> Option<String> {
        match self {
            BuildCategory::Rooms => Some(ROOMS_ICON.into()),
            BuildCategory::Stations => Some(ItemId::CommandConsole.content_id()),
            BuildCategory::Remove => Some(REMOVE_ICON.into()),
            _ => None,
        }
    }

    /// The cards in its flyout, in catalogue order; none for the tools.
    pub fn entries(self) -> Vec<Entry> {
        let category = match self {
            BuildCategory::Rooms => return RoomSize::ALL.map(Entry::Room).to_vec(),
            BuildCategory::Stations => Category::Station,
            BuildCategory::Furniture => Category::Furniture,
            BuildCategory::Decor => Category::Decor,
            BuildCategory::Move | BuildCategory::Remove => return Vec::new(),
        };
        CATALOGUE
            .iter()
            .filter(|d| d.category == category)
            .map(|d| Entry::Item(d.id))
            .collect()
    }

    /// The tool its button picks straight away (Move, Remove); a flyout
    /// opens with nothing picked.
    pub fn tool(self) -> Option<Tool> {
        match self {
            BuildCategory::Move => Some(Tool::Move(None)),
            BuildCategory::Remove => Some(Tool::Remove),
            _ => None,
        }
    }
}

/// Why `entry`'s card is greyed out, if it is: a fixed-count station
/// (only the starting carrier's), a buy-once station already on board, or
/// the price (§ Cost, refunds and unlocks › Unlocks).
pub fn entry_lock(game: &SaveGame, entry: Entry) -> Option<Reason> {
    let wallet = workshop::wallet(game);
    match entry {
        Entry::Room(size) => layout::afford(size.cost(), wallet).err(),
        Entry::Item(id) => {
            let def = id.def()?;
            if def.count.fixed() {
                Some(Reason::StarterPiece)
            } else if def.count == Count::BuyOnce && game.carrier.find(id).is_some() {
                Some(Reason::AlreadyBuilt)
            } else {
                layout::afford(def.cost, wallet).err()
            }
        }
    }
}

/// Picking `entry` (a card, a number key): the tool, or the feedback line
/// when its card is greyed out.
pub fn pick_entry(game: &SaveGame, entry: Entry) -> Result<Tool, String> {
    match entry_lock(game, entry) {
        Some(r) => Err(format!("{}: {r}.", entry.name())),
        None => Ok(Tool::Place(entry)),
    }
}

/// What a build card and the detail panel show for an entry, from the
/// catalogue.
#[derive(Clone, PartialEq, Debug)]
pub struct EntryCard {
    pub name: &'static str,
    /// The item's sprite as its icon; `None` for a room, whose card draws
    /// its outline at scale instead (`outline`, tiles with walls).
    pub icon_id: Option<String>,
    pub outline: Option<(i32, i32)>,
    pub cost: Cost,
    pub seconds: u32,
    pub size: String,
    pub placement: &'static str,
    pub effect: String,
    /// Why the card is greyed out.
    pub lock: Option<String>,
}

pub fn entry_card(game: &SaveGame, entry: Entry) -> EntryCard {
    let lock = entry_lock(game, entry).map(|r| r.to_string());
    match entry {
        Entry::Room(size) => {
            let (w, d) = size.floor();
            EntryCard {
                name: size.name(),
                icon_id: None,
                outline: Some(entry.size()),
                cost: size.cost(),
                seconds: size.build_seconds(),
                size: format!("{w} x {d} floor"),
                placement: "Shares a wall with a room",
                effect: "Floor space. Opens into every room it shares a wall with".into(),
                lock,
            }
        }
        Entry::Item(id) => {
            let def = id.def();
            let (w, d) = item_size(id);
            EntryCard {
                name: id.name(),
                icon_id: Some(id.content_id()),
                outline: None,
                cost: entry.cost(),
                seconds: entry.build_seconds(),
                size: format!("{w} x {d}"),
                placement: def.map_or("", |d| d.placement.label()),
                effect: match def.map(|d| d.effect) {
                    Some(e) if !e.is_empty() => e.to_string(),
                    _ => "Cosmetic".into(),
                },
                lock,
            }
        }
    }
}

pub fn cost_text(c: Cost) -> String {
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
    /// The Upgrade Bench's Workshop panel: the selected upgrade row and the
    /// last purchase's feedback.
    Workshop { index: usize, message: String },
    /// Build mode: the open category, what is picked (`None`: nothing yet)
    /// and the last action's feedback. The ghost follows the mouse.
    Build {
        category: BuildCategory,
        tool: Option<Tool>,
        message: String,
    },
}

impl Overlay {
    pub fn workshop(index: usize, message: String) -> Self {
        Overlay::Workshop { index, message }
    }

    /// Build mode as B opens it: the Rooms flyout, nothing picked.
    pub fn build() -> Self {
        Overlay::Build {
            category: BuildCategory::Rooms,
            tool: None,
            message: String::new(),
        }
    }

    /// Build mode with `tool` picked in its category.
    pub fn building(tool: Tool) -> Self {
        Overlay::Build {
            category: BuildCategory::of(tool),
            tool: Some(tool),
            message: String::new(),
        }
    }
}

/// Set by the flow when the Carrier is entered straight from a mission, so
/// the Pilot spawns at the launched ship's berth.
#[derive(Resource, Default)]
pub struct CarrierArrival {
    pub from_mission: bool,
}

/// The layout as drawn (its shape, `drawn_shape`), and what is derived
/// from it.
#[derive(Resource, Default)]
pub struct CarrierScene {
    drawn: Option<Carrier>,
    pub walkable: Vec<Rect>,
    pub hotspots: Vec<(Hotspot, Vec2)>,
}

/// Camera state: the wheel zoom and, in Build mode, the panned view centre
/// (world px) and the zoom to go back to.
#[derive(Resource, Default)]
pub struct CarrierView {
    pub zoom: Zoom,
    pub pan: Vec2,
    building: Option<f32>,
}

/// The hull tile under the mouse in Build mode (autoplay sets it directly).
#[derive(Resource, Default)]
pub struct BuildCursor(pub Option<Tile>);

/// Click-to-move: the Pilot's remaining waypoints (grid px), where the
/// move marker is, and what to use on arrival. Empty when standing.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct WalkOrder {
    pub path: Vec<Vec2>,
    pub marker: Option<Vec2>,
    pub interact: Option<Hotspot>,
}

/// Hold-to-move (TAKOAI-87), the same as steering the battleship: after a
/// press on the floor and while the button stays down, the cursor (grid px)
/// the Pilot follows, and where it was last sent. The autoplay sets
/// `cursor` directly.
#[derive(Resource, Default, Clone, Copy, PartialEq, Debug)]
pub struct PilotDrag {
    pub cursor: Option<Vec2>,
    ordered: Option<Vec2>,
}

/// How far (px) the held cursor moves before the Pilot is re-pathed.
const DRAG_REPATH: f32 = 8.0;
/// A held cursor this close (px) to the Pilot's feet stops it, so a cursor
/// resting on the Pilot doesn't flip its facing back and forth.
const DRAG_DEAD_ZONE: f32 = 12.0;

/// What a held drag does this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DragStep {
    /// The cursor is still near where the Pilot was sent.
    Keep,
    /// The cursor is on the Pilot: stand.
    Stop,
    /// Walk to the cursor.
    Walk,
}

pub fn drag_step(feet: Vec2, cursor: Vec2, ordered: Option<Vec2>) -> DragStep {
    if ordered.is_some_and(|o| o.distance(cursor) < DRAG_REPATH) {
        DragStep::Keep
    } else if feet.distance(cursor) <= DRAG_DEAD_ZONE {
        DragStep::Stop
    } else {
        DragStep::Walk
    }
}

/// Where the Pilot stands to talk to a crew member: this far south of
/// their feet. Stations are used from their use point itself.
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
        Some(h) => {
            let at = hotspots.iter().find(|(x, _)| *x == h)?.1;
            match h {
                Hotspot::Crew(_) => at + Vec2::Y * APPROACH,
                _ => at,
            }
        }
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

/// Which way the Pilot faces, snapped to 8 directions. S, SE, E, NE and N
/// are drawn; SW, W and NW are SE, E and NE mirrored.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Facing {
    #[default]
    S,
    SE,
    E,
    NE,
    N,
    NW,
    W,
    SW,
}

impl Facing {
    /// By angle in grid space (y south), clockwise from east.
    const BY_ANGLE: [Facing; 8] = [
        Facing::E,
        Facing::SE,
        Facing::S,
        Facing::SW,
        Facing::W,
        Facing::NW,
        Facing::N,
        Facing::NE,
    ];

    /// The facing for a movement `d` in grid px (y south); `None` when
    /// standing still. Client-only, so a float angle is fine.
    pub fn of(d: Vec2) -> Option<Facing> {
        if d.length_squared() < 1e-6 {
            return None;
        }
        let octant = (d.y.atan2(d.x) / std::f32::consts::FRAC_PI_4).round() as i32;
        Some(Self::BY_ANGLE[octant.rem_euclid(8) as usize])
    }

    /// The drawn direction's ID part and whether to mirror it.
    pub fn art(self) -> (&'static str, bool) {
        match self {
            Facing::S => ("s", false),
            Facing::SE => ("se", false),
            Facing::E => ("e", false),
            Facing::NE => ("ne", false),
            Facing::N => ("n", false),
            Facing::NW => ("ne", true),
            Facing::W => ("e", true),
            Facing::SW => ("se", true),
        }
    }
}

/// The player character. `walked` (px since it last stood still) drives
/// the walk cycle; `facing` is kept while standing.
#[derive(Component, Default)]
pub(crate) struct Pilot {
    walked: f32,
    last: Option<Vec2>,
    pub facing: Facing,
}

#[derive(Component)]
struct Prompt;

/// Everything drawn from the layout; respawned when its shape changes.
#[derive(Component)]
struct LayoutSprite;

/// A sprite belonging to item `n` (its art, ship, label, progress bar):
/// hidden while Build mode holds the item.
#[derive(Component, Clone, Copy)]
struct ItemSprite(usize);

/// A construction site's progress bar fill, and its full width (world px).
#[derive(Component, Clone, Copy)]
struct SiteBar {
    site: Site,
    width: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Site {
    Room(usize),
    Item(usize),
}

/// Build-mode ghost, its marks and reason line; respawned as they change.
#[derive(Component)]
struct BuildUi;

/// Build mode's grid overlay on every hull tile, up while Build mode is.
#[derive(Component)]
struct BuildGrid;

/// An item in the way of a room's new doorway: it flashes.
#[derive(Component)]
struct Flash;

/// The battleship sprite on a berth.
#[derive(Component, Clone, Copy)]
pub struct DockedShip {
    pub ship: usize,
    /// Half the sprite's on-screen extent (for the highlight).
    pub half: Vec2,
}

/// A berth's "Next: Elimination" line; it also reads "Selected" for the
/// ship the save will launch.
#[derive(Component)]
struct DockNext {
    ship: usize,
}

/// One live value in the Carrier HUD.
#[derive(Component, Clone, Copy, PartialEq)]
enum HudField {
    Credits,
    Crystal,
}

#[derive(Component)]
struct OverlayUi;

/// A [`text_button`] that lights up under the mouse.
#[derive(Component)]
struct Hoverable;

const BUTTON_BG: Color = Color::srgb(0.08, 0.16, 0.2);
const BUTTON_HOVER: Color = Color::srgb(0.14, 0.3, 0.36);
/// Build mode's screen layout (screen px; the default 1000 px window fits
/// the category bar, five cards a row and the detail panel, so the 15
/// furniture cards take three rows) and its heading gold.
const BUILD_CATEGORY_BUTTON: (f32, f32) = (62.0, 52.0);
const BUILD_CARD_W: f32 = 106.0;
const BUILD_CARD_H: f32 = 92.0;
const BUILD_DETAIL_W: f32 = 186.0;
/// A room card's outline, screen px per tile.
const ROOM_ICON_SCALE: f32 = 1.8;
/// How much of the screen bottom the build panels cover (three rows of
/// cards); the Build-mode camera may pan that much further south so the
/// last hull row clears them.
const BUILD_UI_H: f32 = 360.0;
const BUILD_GOLD: Color = Color::srgb(1.0, 0.86, 0.55);

/// A clickable part of a Carrier panel: the same action as its key, so the
/// Carrier plays with the mouse alone (TAKOAI-66).
#[derive(Component, Clone, Copy, PartialEq, Debug)]
enum Click {
    /// Dialogue: next line / launch prep / close (E).
    Next,
    /// Leave / Back / Close (X; Esc in Build mode).
    Leave,
    /// The launch confirm's Launch (E).
    Launch,
    /// A Workshop row: buy the upgrade (E on that row).
    Row(usize),
    /// The HUD's Build button (B).
    Build,
    /// A build card (1-9 in Build mode).
    Entry(Entry),
    /// A category or tool button in Build mode.
    Category(BuildCategory),
}

/// `image` 9-sliced with `inset` px corners (the builder kit's panels),
/// filling the whole node: an `ImageNode` draws inside the padding by
/// default, which left the text outside the frame (TAKOAI-67).
fn sliced(image: Handle<Image>, inset: f32, corner_scale: f32) -> ImageNode {
    ImageNode {
        visual_box: VisualBox::BorderBox,
        ..ImageNode::new(image).with_mode(NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(inset),
            max_corner_scale: corner_scale,
            ..default()
        }))
    }
}

/// A builder-kit 9-slice by `(id, inset)`.
fn kit(art: &mut ContentImages, images: &mut Assets<Image>, (id, inset): (&str, f32)) -> ImageNode {
    sliced(art.get(images, id).unwrap_or_default(), inset, 1.0)
}

/// A build card's 9-slice, its corners drawn smaller (see
/// [`UI_CARD_CORNER_SCALE`]).
fn card_kit(
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    (id, inset): (&str, f32),
) -> ImageNode {
    sliced(
        art.get(images, id).unwrap_or_default(),
        inset,
        UI_CARD_CORNER_SCALE,
    )
}

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

/// Every manifest image the Carrier draws, besides dialogue portraits.
#[cfg(test)]
pub fn image_ids() -> Vec<String> {
    let mut ids: Vec<String> = (0..3)
        .map(|v| format!("core.carrier.tile.floor.{v}"))
        .collect();
    for mask in 0..16 {
        let bits: String = (0..4)
            .map(|i| if mask & 1 << i != 0 { '1' } else { '0' })
            .collect();
        ids.push(format!("core.carrier.tile.wall.{bits}"));
    }
    ids.extend(["ns", "ew"].map(|a| format!("core.carrier.tile.doorway.{a}")));
    ids.extend([SCAFFOLD, FRONT_MARKER].map(String::from));
    ids.extend(CATALOGUE.iter().map(|d| d.id.content_id()));
    for facing in Facing::BY_ANGLE {
        ids.push(pilot_frame(facing, false, 0.0).0);
        for i in 0..PILOT_WALK.len() {
            ids.push(pilot_frame(facing, true, i as f32 * STRIDE).0);
        }
    }
    ids.extend(SHIPS.iter().map(|s| s.image_id.to_string()));
    for (id, _) in [UI_PANEL, UI_CARD, UI_CARD_SELECTED, UI_CARD_DISABLED] {
        ids.push(id.into());
    }
    ids.extend([GHOST_VALID, GHOST_INVALID, GRID_OVERLAY].map(String::from));
    ids.extend(BuildCategory::ALL.iter().filter_map(|c| c.icon_id()));
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
            .init_resource::<PilotDrag>()
            .add_systems(
                OnEnter(GameScreen::Carrier),
                spawn_carrier.after(flow::enter_carrier),
            )
            .add_systems(OnExit(GameScreen::Carrier), leave_carrier)
            .add_systems(
                Update,
                (
                    (
                        run_sites,
                        sync_layout,
                        update_site_bars,
                        click_to_move,
                        drag_pilot,
                        carrier_input,
                        build_click,
                        zoom_wheel,
                        build_mode_camera,
                        carrier_camera,
                        track_build_cursor,
                    )
                        .chain(),
                    (
                        build_grid,
                        build_preview,
                        hide_held,
                        flash_blockers,
                        animate_pilot,
                        draw_walk_marker,
                        update_prompt,
                        update_hud,
                        show_selected_ship,
                        carrier_hints,
                        sync_overlay,
                        hover_buttons,
                    )
                        .chain(),
                )
                    .chain()
                    // Esc closes an open panel instead of pausing.
                    .after(flow::toggle_pause_menu)
                    .run_if(in_state(GameScreen::Carrier)),
            );
    }
}

/// Text on the deck, on a dark backing so it reads over any floor art.
fn label(text: &str, size: f32, color: Color, pos: Vec3) -> impl Bundle {
    (
        ScreenEntity,
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextBackgroundColor(LABEL_BACKING),
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
///
/// Size and anchor are measured once, from `id`, and kept when
/// `animate_pilot` swaps frames: every frame of an 8-direction character
/// shares one canvas size, feet baseline and visible height
/// (`crates/content/tests/character_frames.rs`), so one measurement fits
/// them all. Measuring each frame instead would rescale the body to fit a
/// frame whose head is drawn lower. The anchor is centred horizontally, so
/// `flip_x` (W, SW, NW) mirrors the frame about the feet.
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

/// How a deck sprite is pinned: a tile by its centre, an item by the
/// bottom centre of its art, which is the middle of its footprint's front
/// edge (§ Art spec › Canvas).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pin {
    Centre,
    Feet,
}

/// One deck sprite before it is spawned.
#[derive(Clone, PartialEq, Debug)]
pub struct Draw {
    pub id: String,
    /// Grid px of the pinned point.
    pub at: Vec2,
    pub pin: Pin,
    pub z: f32,
    pub tint: Color,
    /// The item it draws (an index into `Carrier::items`).
    pub item: Option<usize>,
}

/// The deck, back to front (§ Grid and view › Draw order). Each room tile
/// gets its floor, and a wall or doorway tile its wall or doorway art on
/// top of that floor, so wall art that leaves floor showing (TAKOAI-89's
/// thinner walls) drops in unchanged; the tile art is picked by
/// `TileMap::art`. Rooms being built get the scaffold over every tile and
/// keep their new doorways shut (dimmed). Items follow: rugs and
/// wall-mounted pieces on their flat layers, objects sorted with the
/// characters by their front edge; sites in the blueprint tint. Hull tiles
/// draw nothing: space shows there.
pub fn deck(carrier: &Carrier) -> Vec<Draw> {
    let map = carrier.tile_map();
    let tile = |id: String, t: Tile, z: f32, tint: Color| Draw {
        id,
        at: tile_centre(t),
        pin: Pin::Centre,
        z,
        tint,
        item: None,
    };
    let mut out = Vec::new();
    for y in 0..HULL_H {
        for x in 0..HULL_W {
            let t = (x, y);
            let Some(floor) = map.floor_art(t) else {
                continue;
            };
            out.push(tile(floor, t, Z_FLOOR, Color::WHITE));
            let kind = map.kind(t);
            if kind == TileKind::Floor {
                continue;
            }
            if let Some(id) = map.art(t) {
                let shut = kind == TileKind::Doorway && !map.is_open_doorway(t);
                out.push(tile(id, t, Z_WALL, if shut { SHUT } else { Color::WHITE }));
            }
        }
    }
    for room in carrier.rooms.iter().filter(|r| !r.finished()) {
        for t in room.tiles().filter(|&t| layout::in_hull(t)) {
            out.push(tile(SCAFFOLD.into(), t, Z_SCAFFOLD, Color::WHITE));
        }
    }
    for (i, item) in carrier.items.iter().enumerate() {
        let at = item_feet(item);
        out.push(Draw {
            id: item.id.content_id(),
            at,
            pin: Pin::Feet,
            z: item_z(item.layer(), at.y),
            tint: if item.finished() {
                Color::WHITE
            } else {
                BLUEPRINT
            },
            item: Some(i),
        });
    }
    out
}

/// Grid px of the middle of an item's front edge, where its art's bottom
/// centre goes. A wall-mounted item's canvas sits on its wall's face, so
/// it ends on the same line: the wall tile's bottom edge.
fn item_feet(item: &Item) -> Vec2 {
    let (w, d) = item_size(item.id);
    Vec2::new(
        (item.x as f32 + w as f32 / 2.0) * TILE,
        (item.y + d) as f32 * TILE,
    )
}

fn item_z(layer: Layer, feet_y: f32) -> f32 {
    match layer {
        Layer::Object => depth(feet_y),
        Layer::Floor => Z_RUG,
        Layer::Wall => Z_WALL_ITEM,
    }
}

/// An item's sprite, pinned at its feet: its art at 1 world px per 2 art
/// px (Art authors at 2×). Every catalogue entry ships one
/// (`tests::catalogue_art_fits_its_footprint`).
fn item_sprite(
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    id: ItemId,
    tint: Color,
) -> Option<Sprite> {
    let image = art.get(images, &id.content_id())?;
    let size = ContentImages::size(images, &image) / 2.0;
    Some(Sprite {
        image,
        custom_size: Some(size),
        color: tint,
        ..default()
    })
}

/// A berth's battleship: `SHIPS[ship]` on the pad centred at grid px
/// `pad`, drawn with its battle sprite, nose north, its longest side
/// `ShipInfo::berth_len` (§ Dock and berths), just in front of the pad
/// (`front`, the pad's front edge, grid y).
#[allow(clippy::too_many_arguments)]
fn docked_ship(
    commands: &mut Commands,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    pad: Vec2,
    front: f32,
    ship: usize,
    item: usize,
) {
    let image = art.get(images, SHIPS[ship].image_id).unwrap_or_default();
    let px = ContentImages::size(images, &image);
    let size = px * (SHIPS[ship].berth_len / px.max_element());
    commands.spawn((
        ScreenEntity,
        LayoutSprite,
        ItemSprite(item),
        DockedShip {
            ship,
            half: size / 2.0,
        },
        Sprite {
            image,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(world(pad).extend(depth(front) + 0.000_1)),
    ));
}

/// A construction site's progress bar: a dark track `width` px wide
/// centred at grid px `at`, and the fill `update_site_bars` grows.
fn site_bar(commands: &mut Commands, at: Vec2, width: f32, site: Site, item: Option<usize>) {
    let track = commands
        .spawn((
            ScreenEntity,
            LayoutSprite,
            Sprite {
                color: Color::srgba(0.02, 0.05, 0.08, 0.85),
                custom_size: Some(Vec2::new(width + 4.0, BAR_H + 4.0)),
                ..default()
            },
            Transform::from_translation(world(at).extend(Z_LABEL)),
        ))
        .id();
    let fill = commands
        .spawn((
            ScreenEntity,
            LayoutSprite,
            SiteBar { site, width },
            Sprite {
                color: Color::srgb(0.5, 0.9, 1.0),
                custom_size: Some(Vec2::new(0.0, BAR_H)),
                ..default()
            },
            Anchor::CENTER_LEFT,
            Transform::from_translation((world(at) - Vec2::X * width / 2.0).extend(Z_LABEL + 0.1)),
        ))
        .id();
    if let Some(i) = item {
        commands.entity(track).insert(ItemSprite(i));
        commands.entity(fill).insert(ItemSprite(i));
    }
}

/// How far along a construction site is, 0 to 1 (1 when finished or
/// gone).
fn site_progress(carrier: &Carrier, site: Site) -> f32 {
    let (left, total) = match site {
        Site::Room(i) => carrier
            .rooms
            .get(i)
            .map(|r| (r.build_left_ms, r.size.build_seconds() * 1000)),
        Site::Item(i) => carrier.items.get(i).map(|it| {
            let secs = it.id.def().map_or(0, |d| d.build_seconds);
            (it.build_left_ms, secs * 1000)
        }),
    }
    .unwrap_or((None, 0));
    match left {
        Some(left) if total > 0 => 1.0 - left.min(total) as f32 / total as f32,
        _ => 1.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_carrier(
    mut commands: Commands,
    arrival: Res<CarrierArrival>,
    save: Res<SaveSlot>,
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
    // Space around the rooms is the nebula sky (`crate::sky`).

    // The Pilot (player character).
    let game = save.game.clone().unwrap_or_default();
    let launched = arrival
        .from_mission
        .then_some(game.selected_battleship.as_str());
    let feet = spawn_point(&game.carrier, launched);
    commands.spawn((
        standing(
            &mut art,
            &mut images,
            &pilot_frame(Facing::default(), false, 0.0).0,
            CHARACTER_H,
            world(feet).extend(depth(feet.y)),
        ),
        Pilot::default(),
    ));

    commands.spawn((
        label("", 16.0, Color::WHITE, Vec3::new(0.0, 0.0, Z_LABEL + 1.0)),
        Prompt,
    ));
    // Wallet, the Build button and the gear (Settings), top right, on a
    // backing panel.
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
                row_gap: px(6),
                padding: UiRect::axes(px(10), px(6)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.03, 0.05, 0.08)),
            GlobalZIndex(5),
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
            p.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            })
            .with_children(|row| {
                sized_button(row, Click::Build, "[B] Build", 15.0);
                row.spawn(settings::gear_button(
                    &mut images,
                    Node {
                        width: px(28),
                        height: px(28),
                        padding: UiRect::all(px(3)),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                ));
            });
        });
}

/// Draws `carrier` (`deck`), with the berths' ships, progress bars on the
/// construction sites, and the crew at their posts.
fn spawn_layout(
    commands: &mut Commands,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    palette: &mut CarrierPalette,
    carrier: &Carrier,
    hotspots: &[(Hotspot, Vec2)],
) {
    for d in deck(carrier) {
        let Some(i) = d.item else {
            // Floor, wall and doorway tiles in the player's colours.
            let Some(image) = palette.image(art, images, &d.id) else {
                continue;
            };
            commands.spawn((
                ScreenEntity,
                LayoutSprite,
                Sprite {
                    image,
                    custom_size: Some(Vec2::splat(TILE)),
                    color: d.tint,
                    ..default()
                },
                Transform::from_translation(world(d.at).extend(d.z)),
            ));
            continue;
        };
        let item = &carrier.items[i];
        let (w, depth_tiles) = item_size(item.id);
        if let Some(sprite) = item_sprite(art, images, item.id, d.tint) {
            commands.spawn((
                ScreenEntity,
                LayoutSprite,
                ItemSprite(i),
                sprite,
                Anchor::BOTTOM_CENTER,
                Transform::from_translation(world(d.at).extend(d.z)),
            ));
        }
        if let (ItemId::Berth, Some(ship)) = (item.id, item.ship.as_deref()) {
            let ship = ship_index(ship);
            let pad = Vec2::new(d.at.x, d.at.y - depth_tiles as f32 * TILE / 2.0);
            docked_ship(commands, art, images, pad, d.at.y, ship, i);
            commands.spawn((
                label(
                    "",
                    13.0,
                    Color::srgb(0.7, 0.9, 1.0),
                    world(Vec2::new(d.at.x, item.y as f32 * TILE - 12.0)).extend(Z_LABEL),
                ),
                LayoutSprite,
                ItemSprite(i),
                DockNext { ship },
            ));
        }
        if !item.finished() {
            let at = d.at - Vec2::Y * 10.0;
            site_bar(commands, at, w as f32 * TILE - 16.0, Site::Item(i), Some(i));
        }
    }
    for (i, room) in carrier.rooms.iter().enumerate() {
        if room.finished() {
            continue;
        }
        let (w, d) = room.size.floor();
        let (centre, _) = tile_rect((room.x + 1, room.y + 1), (w, d));
        site_bar(commands, centre, w as f32 * TILE / 2.0, Site::Room(i), None);
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

/// The layout as far as drawing goes: a site is a site however long it
/// has left, so the timers ticking don't redraw the deck every frame (the
/// bars follow them, `update_site_bars`); a site finishing does.
fn drawn_shape(carrier: &Carrier) -> Carrier {
    let mut c = carrier.clone();
    let sites = c
        .rooms
        .iter_mut()
        .map(|r| &mut r.build_left_ms)
        .chain(c.items.iter_mut().map(|i| &mut i.build_left_ms));
    for left in sites.filter(|l| l.is_some()) {
        *left = Some(1);
    }
    c
}

/// Redraws the carrier whenever the saved layout's shape differs from what
/// is on screen (entering the scene, building, moving, removing, a site
/// finishing), recomputing the walkable area and the hotspots.
fn sync_layout(
    mut commands: Commands,
    save: Res<SaveSlot>,
    mut scene: ResMut<CarrierScene>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut palette: ResMut<CarrierPalette>,
    existing: Query<Entity, With<LayoutSprite>>,
) {
    let carrier = save
        .game
        .as_ref()
        .map(|g| g.carrier.clone())
        .unwrap_or_default();
    let shape = drawn_shape(&carrier);
    if scene.drawn.as_ref() == Some(&shape) {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let hotspots = hotspots(&carrier);
    spawn_layout(
        &mut commands,
        &mut art,
        &mut images,
        &mut palette,
        &carrier,
        &hotspots,
    );
    scene.walkable = carrier.walkable();
    scene.hotspots = hotspots;
    scene.drawn = Some(shape);
}

/// Construction timers run while the Carrier is open and unpaused
/// (§ Construction time), whatever else is on screen; whole ms only, the
/// rest carried to the next frame. A site finishing saves (§ Save file).
fn run_sites(
    time: Res<Time>,
    pause: Res<PauseMenu>,
    mut save: ResMut<SaveSlot>,
    mut carry: Local<f32>,
) {
    let building = save.game.as_ref().is_some_and(|g| {
        g.carrier.rooms.iter().any(|r| !r.finished())
            || g.carrier.items.iter().any(|i| !i.finished())
    });
    if pause.open || !building {
        *carry = 0.0;
        return;
    }
    *carry += time.delta_secs() * 1000.0;
    let ms = carry.floor();
    *carry -= ms;
    let finished = ms >= 1.0
        && save
            .game
            .as_mut()
            .is_some_and(|g| g.carrier.tick(ms as u32));
    if finished {
        save.store();
    }
}

/// Grows each site's progress bar with its timer.
fn update_site_bars(save: Res<SaveSlot>, mut bars: Query<(&SiteBar, &mut Sprite)>) {
    let Some(carrier) = save.game.as_ref().map(|g| &g.carrier) else {
        return;
    };
    for (bar, mut sprite) in &mut bars {
        let size = Some(Vec2::new(
            bar.width * site_progress(carrier, bar.site),
            BAR_H,
        ));
        if sprite.custom_size != size {
            sprite.custom_size = size;
        }
    }
}

fn leave_carrier(
    mut overlay: ResMut<Overlay>,
    mut order: ResMut<WalkOrder>,
    mut drag: ResMut<PilotDrag>,
    mut arrival: ResMut<CarrierArrival>,
    mut scene: ResMut<CarrierScene>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    *overlay = Overlay::None;
    *order = WalkOrder::default();
    *drag = PilotDrag::default();
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

#[allow(clippy::too_many_arguments)]
fn carrier_input(
    keys: Res<ButtonInput<KeyCode>>,
    clicks: Query<(&Interaction, &Click), Changed<Interaction>>,
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
    let build_key = keys.just_pressed(KeyCode::KeyB);
    let dt = time.delta_secs();
    let clicked = clicks
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, c)| *c);
    let leave = clicked == Some(Click::Leave);
    let digit = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .iter()
    .position(|k| keys.just_pressed(*k));

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
            } else if build_key || clicked == Some(Click::Build) {
                // Build mode from anywhere aboard; the Pilot stops.
                *order = WalkOrder::default();
                next = Some(Overlay::build());
            }
        }
        Overlay::Dialogue {
            lines,
            index,
            launch,
        } => {
            if back || leave {
                next = Some(Overlay::None);
            } else if confirm || clicked == Some(Click::Next) {
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
            if back || leave {
                next = Some(Overlay::None);
            } else if confirm || clicked == Some(Click::Launch) {
                launch.write(MissionRequest {
                    battleship_id: SHIPS[*index].id.to_string(),
                });
            }
        }
        Overlay::Workshop { index, message } => {
            let rows = UPGRADES.len();
            let up = keys.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
            let down = keys.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
            let digit = digit.filter(|&i| i < rows);
            let row = match clicked {
                Some(Click::Row(i)) if i < rows => Some(i),
                _ => None,
            };
            if back || leave {
                next = Some(Overlay::None);
            } else if let Some(i) = digit {
                *index = i;
            } else if up {
                *index = (*index + rows - 1) % rows;
            } else if down {
                *index = (*index + 1) % rows;
            } else if confirm || row.is_some() {
                if let Some(i) = row {
                    *index = i;
                }
                let Some(game) = &mut save.game else {
                    return;
                };
                let upgrade = UPGRADES[*index];
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
        Overlay::Build {
            category,
            tool,
            message,
        } => {
            // The Pilot waits; WASD pans the view.
            let zoom = view.zoom.shown;
            view.pan += world(wasd(&keys)) * PAN_SPEED * dt / zoom;
            let game = save.game.clone().unwrap_or_default();
            let pick = match clicked {
                Some(Click::Entry(e)) => Some(e),
                _ => digit.and_then(|i| category.entries().get(i).copied()),
            };
            if let Some(Click::Category(c)) = clicked {
                *category = c;
                *tool = c.tool();
                message.clear();
            } else if let Some(e) = pick {
                match pick_entry(&game, e) {
                    Ok(t) => {
                        *tool = Some(t);
                        message.clear();
                    }
                    Err(e) => *message = e,
                }
            } else if build_key || clicked == Some(Click::Build) {
                next = Some(Overlay::None);
            } else if esc || leave {
                match back_out(*tool) {
                    Some(t) => {
                        *tool = t;
                        message.clear();
                    }
                    None => next = Some(Overlay::None),
                }
            }
        }
    }
    overlay.set_if_neq(next.unwrap_or(current));
}

/// Right click or Esc in Build mode: a held item goes back where it was,
/// else the picked entry or tool is dropped. `None` when nothing is picked
/// (Esc then leaves Build mode; a right click does nothing).
pub fn back_out(tool: Option<Tool>) -> Option<Option<Tool>> {
    match tool {
        Some(Tool::Move(Some(_))) => Some(Some(Tool::Move(None))),
        Some(_) => Some(None),
        None => None,
    }
}

/// What a Build-mode click did: whether the save changed (it is then
/// stored), the feedback line, and what is picked afterwards.
#[derive(Clone, PartialEq, Debug)]
pub struct BuildOutcome {
    pub changed: bool,
    pub message: String,
    pub tool: Option<Tool>,
}

/// " (+60 cr + 1 VC)" for a refund, nothing for none.
fn refund_text(c: Cost) -> String {
    if c == Cost::default() {
        String::new()
    } else {
        format!(" (+{})", cost_text(c))
    }
}

/// What a Build-mode click on tile `mouse` does with `tool` (§ Building):
/// Place pays and starts a construction site (the entry stays picked for
/// another copy; a buy-once station goes back to the flyout); Move lifts a
/// finished item and puts it down, free and instant; Remove takes away an
/// item, a site or an empty room for a full refund. `pilot` is the tile
/// under the Pilot's feet.
pub fn apply_build_click(
    game: &mut SaveGame,
    tool: Tool,
    mouse: Tile,
    pilot: Option<Tile>,
) -> BuildOutcome {
    let done = |message: String, tool: Option<Tool>| BuildOutcome {
        changed: true,
        message,
        tool,
    };
    let refused = |r: Reason| BuildOutcome {
        changed: false,
        message: r.to_string(),
        tool: Some(tool),
    };
    let nothing = BuildOutcome {
        changed: false,
        message: String::new(),
        tool: Some(tool),
    };
    match tool {
        Tool::Place(Entry::Room(size)) => {
            let (x, y) = Room::anchor_for(size, mouse);
            match workshop::build_room(game, size, x, y) {
                Ok(_) => done(
                    format!("Building a {}: {}.", size.name(), cost_text(size.cost())),
                    Some(tool),
                ),
                Err(r) => refused(r),
            }
        }
        Tool::Place(Entry::Item(id)) => {
            let (x, y) = item_anchor(item_size(id), mouse);
            match workshop::build_item(game, id, x, y, pilot) {
                Ok(_) => {
                    let again = id.def().is_some_and(|d| d.count != Count::BuyOnce);
                    done(
                        format!("Building a {}: {}.", id.name(), cost_text(entry_cost(id))),
                        again.then_some(tool),
                    )
                }
                Err(r) => refused(r),
            }
        }
        Tool::Move(None) => match game.carrier.pick_item(mouse) {
            None => nothing,
            Some(i) if !game.carrier.items[i].finished() => refused(Reason::StillBeingBuilt),
            Some(i) => BuildOutcome {
                changed: false,
                message: format!("Moving the {}.", game.carrier.items[i].id.name()),
                tool: Some(Tool::Move(Some(i))),
            },
        },
        Tool::Move(Some(i)) => {
            let Some(id) = game.carrier.items.get(i).map(|it| it.id) else {
                return BuildOutcome {
                    tool: Some(Tool::Move(None)),
                    ..nothing
                };
            };
            let (x, y) = item_anchor(item_size(id), mouse);
            match game.carrier.can_move(i, x, y, pilot) {
                Ok(()) => {
                    game.carrier.move_item(i, x, y);
                    done(format!("Moved the {}.", id.name()), Some(Tool::Move(None)))
                }
                Err(r) => refused(r),
            }
        }
        Tool::Remove => {
            if let Some(i) = game.carrier.pick_item(mouse) {
                let name = game.carrier.items[i].id.name();
                match workshop::remove_item(game, i) {
                    Ok(c) => done(format!("Removed the {name}{}.", refund_text(c)), Some(tool)),
                    Err(r) => refused(r),
                }
            } else if let Some(i) = game.carrier.room_at(mouse) {
                let name = game.carrier.rooms[i].size.name();
                match workshop::remove_room(game, i) {
                    Ok(c) => done(format!("Removed the {name}{}.", refund_text(c)), Some(tool)),
                    Err(r) => refused(r),
                }
            } else {
                nothing
            }
        }
    }
}

fn entry_cost(id: ItemId) -> Cost {
    Entry::Item(id).cost()
}

/// Build mode's mouse: a left click on the deck applies the picked tool at
/// the cursor tile (saving when it changed something); a right click puts
/// a held item back or drops the pick (`back_out`).
#[allow(clippy::too_many_arguments)]
fn build_click(
    mouse: Res<ButtonInput<MouseButton>>,
    pause: Res<PauseMenu>,
    cursor: Res<BuildCursor>,
    ui: Query<&Interaction>,
    pilot: Query<&Transform, With<Pilot>>,
    mut save: ResMut<SaveSlot>,
    mut overlay: ResMut<Overlay>,
) {
    if pause.open {
        return;
    }
    let Overlay::Build { tool, message, .. } = &mut *overlay else {
        return;
    };
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(t) = back_out(*tool) {
            *tool = t;
            message.clear();
        }
        return;
    }
    // A click on the build panels is theirs (`carrier_input`), not the deck's.
    if !mouse.just_pressed(MouseButton::Left) || ui.iter().any(|i| *i != Interaction::None) {
        return;
    }
    let pilot = pilot.single().ok().map(|tf| {
        let feet = grid(tf.translation.truncate());
        layout::tile_at(feet.x, feet.y)
    });
    let (Some(t), Some(at), Some(game)) = (*tool, cursor.0, save.game.as_mut()) else {
        return;
    };
    let out = apply_build_click(game, t, at, pilot);
    if out.changed {
        // Spec § Save file: saved after every place, move and remove.
        save.store();
    }
    *tool = out.tool;
    *message = out.message;
}

/// Using `hotspot`: talk to crew, open the Workshop upgrades at the Upgrade
/// Bench, or board a docked battleship, which selects it and starts the
/// briefing before launch.
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
        Hotspot::UpgradeBench => Overlay::workshop(0, String::new()),
    };
}

fn cursor_world(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), camera.single()) else {
        return None;
    };
    input::cursor_world(window, camera, cam_tf)
}

/// Left button, the same as in battle ([`MovePress`]): a press on the
/// floor walks the Pilot there through the rooms and doorways, and holding
/// it keeps following the cursor (`drag_pilot`); a press on a crew member,
/// the Upgrade Bench or a berth walks up and uses it on arrival (like
/// walking up and pressing E). Presses and drags over the HUD or a panel
/// are the UI's; Build mode handles its own clicks (`build_click`).
#[allow(clippy::too_many_arguments)]
fn click_to_move(
    mouse: Res<ButtonInput<MouseButton>>,
    pause: Res<PauseMenu>,
    overlay: Res<Overlay>,
    scene: Res<CarrierScene>,
    save: Res<SaveSlot>,
    autoplay: Option<Res<flow::Autoplay>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction>,
    pilot: Query<&Transform, With<Pilot>>,
    mut order: ResMut<WalkOrder>,
    mut drag: ResMut<PilotDrag>,
) {
    // The autoplay drives the Pilot itself.
    if autoplay.is_some() {
        return;
    }
    let press = MovePress::read(&mouse);
    if pause.open || *overlay != Overlay::None || !press.held() {
        if *drag != PilotDrag::default() {
            *drag = PilotDrag::default();
        }
        return;
    }
    let over_ui = ui.iter().any(|i| *i != Interaction::None);
    let Some(p) = cursor_world(&windows, &camera).filter(|_| !over_ui) else {
        return;
    };
    if press == MovePress::Hold {
        if drag.cursor.is_some_and(|c| c != grid(p)) {
            drag.cursor = Some(grid(p));
        }
        return;
    }
    let Ok(pilot) = pilot.single() else {
        return;
    };
    let hotspot = save
        .game
        .as_ref()
        .and_then(|g| clicked_hotspot(&scene.hotspots, &g.carrier, grid(p)));
    let feet = grid(pilot.translation.truncate());
    if let Some(o) = order_walk(&scene.walkable, &scene.hotspots, feet, grid(p), hotspot) {
        *order = o;
    }
    // Only a press on the floor drags; one on something to use stays a click.
    *drag = match hotspot {
        None => PilotDrag {
            cursor: Some(grid(p)),
            ordered: Some(grid(p)),
        },
        Some(_) => PilotDrag::default(),
    };
}

/// Hold-to-move: re-paths the Pilot to the held cursor when it has moved,
/// and stops it when the cursor rests on it.
fn drag_pilot(
    pause: Res<PauseMenu>,
    overlay: Res<Overlay>,
    scene: Res<CarrierScene>,
    pilot: Query<&Transform, With<Pilot>>,
    mut order: ResMut<WalkOrder>,
    mut drag: ResMut<PilotDrag>,
) {
    let Some(cursor) = drag.cursor else {
        return;
    };
    if pause.open || *overlay != Overlay::None {
        *drag = PilotDrag::default();
        return;
    }
    let Ok(pilot) = pilot.single() else {
        return;
    };
    let feet = grid(pilot.translation.truncate());
    match drag_step(feet, cursor, drag.ordered) {
        DragStep::Keep => return,
        DragStep::Stop => {
            if *order != WalkOrder::default() {
                *order = WalkOrder::default();
            }
        }
        DragStep::Walk => {
            // Off the floor (deep space): keep walking to the last spot.
            if let Some(o) = order_walk(&scene.walkable, &scene.hotspots, feet, cursor, None) {
                *order = o;
            }
        }
    }
    drag.ordered = Some(cursor);
}

/// The click-to-move destination marker, the same one the battle draws.
fn draw_walk_marker(order: Res<WalkOrder>, mut gizmos: Gizmos) {
    if let Some(at) = order.marker {
        crate::render::move_marker(&mut gizmos, world(at), Crew::Pilot.color());
    }
}

/// The mouse wheel zooms (scroll up: in), about the cursor
/// (`carrier_camera`).
fn zoom_wheel(
    scroll: Res<AccumulatedMouseScroll>,
    pause: Res<PauseMenu>,
    mut view: ResMut<CarrierView>,
) {
    if pause.open || scroll.delta.y == 0.0 {
        return;
    }
    view.zoom.scroll(zoom::notches(&scroll));
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
            view.building = Some(view.zoom.target);
            view.zoom.set(OVERVIEW);
            if let Ok(tf) = pilot.single() {
                view.pan = tf.translation.truncate();
            }
        }
        (false, Some(zoom)) => {
            view.zoom.set(zoom);
            view.building = None;
        }
        _ => {}
    }
}

/// Keeps the camera centre within `CAMERA_MARGIN` (2 tiles) past the hull
/// on each side (plus `below` world px to the south, which Build mode's
/// panels cover), or centred on that area when the view is bigger than it.
pub fn clamp_camera(target: Vec2, half_view: Vec2, below: f32) -> Vec2 {
    let min = Vec2::new(
        -CAMERA_MARGIN,
        -(HULL_H as f32 * TILE + CAMERA_MARGIN) - below,
    );
    let max = Vec2::new(HULL_W as f32 * TILE + CAMERA_MARGIN, CAMERA_MARGIN);
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

/// Follows the Pilot (Build mode: the pan), zoomed about the cursor; the
/// offset that zooming about the cursor leaves keeps the Pilot on screen.
/// Clamped to the hull (`clamp_camera`).
fn carrier_camera(
    time: Res<Time>,
    mut view: ResMut<CarrierView>,
    pilot: Query<&Transform, (With<Pilot>, Without<Camera2d>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let (Ok(pilot), Ok((mut cam, mut projection))) = (pilot.single(), camera.single_mut()) else {
        return;
    };
    let window = windows.single().ok();
    let size = window.map_or(Vec2::new(1280.0, 720.0), Window::size);
    let cursor = window.and_then(Window::cursor_position);
    let building = view.building.is_some();
    // Build mode pans the view itself, so zooming moves the pan.
    let target = if building {
        view.pan
    } else {
        pilot.translation.truncate()
    };
    // Zoom about the cursor from the view on screen now.
    let mut zoom = view.zoom;
    let from = cam.translation.truncate();
    let zoomed = zoom.update(time.delta_secs(), 1.0, from, cursor, size);
    let changed = zoomed != from;
    let scale = 1.0 / zoom.shown;
    let half = size / 2.0 * scale;
    let below = if building { BUILD_UI_H * scale } else { 0.0 };
    let centre = if building {
        zoom.offset = Vec2::ZERO;
        clamp_camera(if changed { zoomed } else { target }, half, below)
    } else {
        if changed {
            zoom.offset = zoomed - target;
        }
        zoom.keep_in_view(half);
        let centre = clamp_camera(target + zoom.offset, half, below);
        // Drop the part of the offset the hull clamp ate, so it doesn't
        // build up off the map.
        if changed {
            zoom.offset = centre - target;
        }
        centre
    };
    if building && view.pan != centre {
        view.pan = centre;
    }
    if view.zoom != zoom {
        view.zoom = zoom;
    }
    if let Projection::Orthographic(ortho) = &mut *projection {
        if ortho.scale != scale {
            ortho.scale = scale;
        }
    }
    cam.translation.x = centre.x;
    cam.translation.y = centre.y;
}

/// In Build mode, the hull tile under the mouse; none over the panels.
fn track_build_cursor(
    overlay: Res<Overlay>,
    autoplay: Option<Res<flow::Autoplay>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction>,
    mut cursor: ResMut<BuildCursor>,
) {
    // Autoplay points the cursor itself.
    if autoplay.is_some() || !matches!(*overlay, Overlay::Build { .. }) {
        return;
    }
    let over_ui = ui.iter().any(|i| *i != Interaction::None);
    let tile = cursor_world(&windows, &camera)
        .filter(|_| !over_ui)
        .map(|p| {
            let g = grid(p);
            layout::tile_at(g.x, g.y)
        });
    if cursor.0 != tile {
        cursor.0 = tile;
    }
}

/// What Build mode shows at the cursor (§ Building › Room ghost, Item
/// ghost).
#[derive(Clone, PartialEq, Debug)]
struct Ghost {
    /// What it draws: an entry's art or a room's outline (Move and Remove
    /// show the piece under the mouse).
    entry: Entry,
    /// Anchor tile and footprint (a room's with its walls).
    at: Tile,
    size: (i32, i32),
    /// Whether a click would succeed.
    ok: bool,
    /// The line over the ghost: the cost, or why not.
    text: String,
    /// The doorways a room would open.
    doorways: Vec<Tile>,
    /// A station's front strip, marked with chevrons.
    front: Vec<Tile>,
    /// Items in the way of a room's new doorways: they flash.
    flash: Vec<usize>,
}

/// The doorway tiles a room of `size` anchored at (x, y) would open.
fn new_doorways(carrier: &Carrier, size: RoomSize, x: i32, y: i32) -> Vec<Tile> {
    let mut after = carrier.clone();
    let i = after.place_room(size, x, y);
    after
        .tile_map()
        .doorways
        .iter()
        .filter(|d| d.rooms.0 == i || d.rooms.1 == i)
        .flat_map(|d| d.tiles)
        .collect()
}

/// The ghost for `tool` with the mouse on tile `mouse` (`pilot`: the tile
/// under the Pilot's feet), or `None` when there is nothing to show (Move
/// and Remove over bare floor or hull).
fn ghost(game: &SaveGame, tool: Tool, mouse: Tile, pilot: Option<Tile>) -> Option<Ghost> {
    let c = &game.carrier;
    let wallet = workshop::wallet(game);
    let price = |e: Entry| {
        format!(
            "{}: {} - {} s",
            e.name(),
            cost_text(e.cost()),
            e.build_seconds()
        )
    };
    let item_ghost = |item: &Item, ok: bool, text: String| Ghost {
        entry: Entry::Item(item.id),
        at: (item.x, item.y),
        size: item_size(item.id),
        ok,
        text,
        doorways: Vec::new(),
        front: item.front_strip(),
        flash: Vec::new(),
    };
    match tool {
        Tool::Place(entry @ Entry::Room(size)) => {
            let (x, y) = Room::anchor_for(size, mouse);
            let result = c.can_place_room(size, x, y, wallet);
            let flash = match &result {
                Err(Reason::ClearDoorway(items)) => items.clone(),
                _ => Vec::new(),
            };
            Some(Ghost {
                entry,
                at: (x, y),
                size: entry.size(),
                ok: result.is_ok(),
                text: result.map_or_else(|r| r.to_string(), |()| price(entry)),
                doorways: new_doorways(c, size, x, y),
                front: Vec::new(),
                flash,
            })
        }
        Tool::Place(entry @ Entry::Item(id)) => {
            let (x, y) = item_anchor(item_size(id), mouse);
            let result = c.can_place(id, x, y, wallet, pilot);
            let text = result
                .clone()
                .map_or_else(|r| r.to_string(), |()| price(entry));
            Some(item_ghost(&Item::new(id, x, y), result.is_ok(), text))
        }
        Tool::Move(None) => {
            let item = &c.items[c.pick_item(mouse)?];
            Some(if item.finished() {
                item_ghost(item, true, format!("Move the {}", item.id.name()))
            } else {
                item_ghost(item, false, Reason::StillBeingBuilt.to_string())
            })
        }
        Tool::Move(Some(i)) => {
            let held = c.items.get(i)?;
            let (x, y) = item_anchor(item_size(held.id), mouse);
            let result = c.can_move(i, x, y, pilot);
            let text = match &result {
                Ok(()) => format!("Put the {} here", held.id.name()),
                Err(r) => r.to_string(),
            };
            let moved = Item {
                x,
                y,
                ..held.clone()
            };
            Some(item_ghost(&moved, result.is_ok(), text))
        }
        Tool::Remove => {
            if let Some(i) = c.pick_item(mouse) {
                let item = &c.items[i];
                let result = c.can_remove(i);
                let text = match &result {
                    Ok(refund) => format!("Remove the {}{}", item.id.name(), refund_text(*refund)),
                    Err(r) => r.to_string(),
                };
                return Some(item_ghost(item, result.is_ok(), text));
            }
            let i = c.room_at(mouse)?;
            let room = &c.rooms[i];
            let result = c.can_remove_room(i);
            let entry = Entry::Room(room.size);
            Some(Ghost {
                entry,
                at: (room.x, room.y),
                size: entry.size(),
                ok: result.is_ok(),
                text: match &result {
                    Ok(refund) => {
                        format!("Remove the {}{}", room.size.name(), refund_text(*refund))
                    }
                    Err(r) => r.to_string(),
                },
                doorways: Vec::new(),
                front: Vec::new(),
                flash: Vec::new(),
            })
        }
    }
}

/// Build mode's grid overlay on every hull tile (§ Building › Grid),
/// spawned when Build mode opens and gone when it closes.
fn build_grid(
    mut commands: Commands,
    overlay: Res<Overlay>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<BuildGrid>>,
) {
    let building = matches!(*overlay, Overlay::Build { .. });
    if building != existing.is_empty() {
        return;
    }
    if !building {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        return;
    }
    let Some(image) = art.get(&mut images, GRID_OVERLAY) else {
        return;
    };
    for y in 0..HULL_H {
        for x in 0..HULL_W {
            commands.spawn((
                ScreenEntity,
                BuildGrid,
                Sprite {
                    image: image.clone(),
                    custom_size: Some(Vec2::splat(TILE)),
                    ..default()
                },
                Transform::from_translation(world(tile_centre((x, y))).extend(Z_GRID)),
            ));
        }
    }
}

/// Build mode's ghost at the cursor: a room as its walls and floor tinted
/// with the kit's valid / invalid swatch and the doorways it would open
/// marked; an item as its art tinted the same, its
/// footprint outlined and its front strip chevroned; and one line over it,
/// the cost or the reason. Items in the way of a refused room flash.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_preview(
    mut commands: Commands,
    overlay: Res<Overlay>,
    cursor: Res<BuildCursor>,
    save: Res<SaveSlot>,
    pilot: Query<&Transform, With<Pilot>>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<BuildUi>>,
    mut drawn: Local<Option<(Overlay, Option<Tile>, Carrier, Cost)>>,
) {
    let game = save.game.clone().unwrap_or_default();
    let key = (
        overlay.clone(),
        cursor.0,
        drawn_shape(&game.carrier),
        workshop::wallet(&game),
    );
    if drawn.as_ref() == Some(&key) {
        return;
    }
    *drawn = Some(key);
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Overlay::Build {
        tool: Some(tool), ..
    } = &*overlay
    else {
        return;
    };
    let pilot = pilot.single().ok().map(|tf| {
        let feet = grid(tf.translation.truncate());
        layout::tile_at(feet.x, feet.y)
    });
    let Some(g) = cursor.0.and_then(|t| ghost(&game, *tool, t, pilot)) else {
        return;
    };
    let (tint, swatch, text_color) = if g.ok {
        (
            Color::srgba(0.85, 0.95, 1.0, 0.75),
            GHOST_VALID,
            Color::srgb(0.88, 0.96, 1.0),
        )
    } else {
        (
            Color::srgba(1.0, 0.55, 0.55, 0.6),
            GHOST_INVALID,
            Color::srgb(1.0, 0.55, 0.5),
        )
    };
    let swatch = art.get(&mut images, swatch).unwrap_or_default();
    let mut tile = |t: Tile, image: Handle<Image>, color: Color, size: f32, z: f32| {
        commands.spawn((
            ScreenEntity,
            BuildUi,
            Sprite {
                image,
                custom_size: Some(Vec2::splat(size)),
                color,
                ..default()
            },
            Transform::from_translation(world(tile_centre(t)).extend(z)),
        ));
    };
    match g.entry {
        Entry::Room(_) => {
            let (x0, y0) = g.at;
            let (x1, y1) = (x0 + g.size.0 - 1, y0 + g.size.1 - 1);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let ring = x == x0 || x == x1 || y == y0 || y == y1;
                    let alpha = if ring { 0.95 } else { 0.5 };
                    tile(
                        (x, y),
                        swatch.clone(),
                        Color::srgba(1.0, 1.0, 1.0, alpha),
                        TILE,
                        Z_GHOST,
                    );
                }
            }
            for &t in &g.doorways {
                let mark = Color::srgba(0.45, 1.0, 0.65, 0.9);
                tile(t, Handle::default(), mark, TILE - 16.0, Z_GHOST + 0.2);
            }
        }
        Entry::Item(id) => {
            for y in g.at.1..g.at.1 + g.size.1 {
                for x in g.at.0..g.at.0 + g.size.0 {
                    let color = Color::srgba(1.0, 1.0, 1.0, 0.6);
                    tile((x, y), swatch.clone(), color, TILE, Z_GHOST + 0.5);
                }
            }
            let marker = art.get(&mut images, FRONT_MARKER).unwrap_or_default();
            for &t in &g.front {
                tile(t, marker.clone(), tint, TILE, Z_GHOST + 0.6);
            }
            let feet = item_feet(&Item::new(id, g.at.0, g.at.1));
            if let Some(sprite) = item_sprite(&mut art, &mut images, id, tint) {
                commands.spawn((
                    ScreenEntity,
                    BuildUi,
                    sprite,
                    Anchor::BOTTOM_CENTER,
                    Transform::from_translation(world(feet).extend(Z_GHOST + 0.4)),
                ));
            }
        }
    }
    for item in g.flash.iter().filter_map(|&i| game.carrier.items.get(i)) {
        let (centre, size) = tile_rect((item.x, item.y), item_size(item.id));
        commands.spawn((
            ScreenEntity,
            BuildUi,
            Flash,
            Sprite {
                color: Color::srgba(1.0, 0.2, 0.2, 0.5),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world(centre).extend(Z_GHOST + 0.7)),
        ));
    }
    let top = Vec2::new(
        (g.at.0 as f32 + g.size.0 as f32 / 2.0) * TILE,
        g.at.1 as f32 * TILE - 22.0,
    );
    commands.spawn((
        label(&g.text, 22.0, text_color, world(top).extend(Z_LABEL + 2.0)),
        BuildUi,
    ));
}

/// The item Build mode holds becomes the ghost: its own sprites hide.
fn hide_held(overlay: Res<Overlay>, mut sprites: Query<(&ItemSprite, &mut Visibility)>) {
    let held = match *overlay {
        Overlay::Build {
            tool: Some(Tool::Move(Some(i))),
            ..
        } => Some(i),
        _ => None,
    };
    for (sprite, mut vis) in &mut sprites {
        let want = if Some(sprite.0) == held {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != want {
            *vis = want;
        }
    }
}

/// Items in the way of a room's doorway pulse ("the blocking items
/// flash").
fn flash_blockers(time: Res<Time>, mut flashing: Query<&mut Sprite, With<Flash>>) {
    let alpha = 0.2 + 0.45 * (time.elapsed_secs() * 6.0).sin().abs();
    for mut sprite in &mut flashing {
        sprite.color.set_alpha(alpha);
    }
}

/// Marks the docked ship the save has selected: a highlight ring and
/// "Selected" in its berth's label; the other ship is dimmed.
fn show_selected_ship(
    save: Res<SaveSlot>,
    mut ships: Query<(&Transform, &DockedShip, &mut Sprite)>,
    mut next: Query<(&mut Text2d, &DockNext)>,
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
        }
    }
    for (mut text, berth) in &mut next {
        let name = SHIPS[berth.ship].name;
        let value = if berth.ship == selected {
            format!("{name} - Selected - Next: Elimination")
        } else {
            format!("{name} - Next: Elimination")
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

/// Carrier tutorial hint triggers (`hints`, design/READINESS.md § Tutorial
/// hints): the refit notice once after a Revision 1 save was refunded,
/// standing at something usable, near a berth, the Workshop and Build mode.
fn carrier_hints(
    overlay: Res<Overlay>,
    scene: Res<CarrierScene>,
    pilot: Query<&Transform, With<Pilot>>,
    mut hints: ResMut<Hints>,
    mut save: ResMut<SaveSlot>,
) {
    if let Some(game) = save.game.as_mut().filter(|g| g.carrier_refit) {
        game.carrier_refit = false;
        if !hints.trigger(&mut save, Hint::CarrierRefit) {
            save.store();
        }
    }
    let Ok(pilot) = pilot.single() else {
        return;
    };
    let near = nearest_hotspot(&scene.hotspots, grid(pilot.translation.truncate()));
    if near.is_some() {
        hints.trigger(&mut save, Hint::CarrierInteract);
    }
    if matches!(near, Some((Hotspot::Berth(_), _))) {
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

/// The Pilot's frame facing `facing` after `walked` px (idle when
/// standing), and whether to mirror it.
fn pilot_frame(facing: Facing, moving: bool, walked: f32) -> (String, bool) {
    let (dir, flip) = facing.art();
    let frame = if moving {
        PILOT_WALK[(walked / STRIDE) as usize % PILOT_WALK.len()]
    } else {
        "idle"
    };
    (format!("core.carrier.pilot.{dir}.{frame}"), flip)
}

/// Picks the Pilot's frame from how it moved this frame (click-to-move and
/// WASD alike), snapped to 8 directions; standing keeps the last facing
/// (§ Walking and interaction › Pilot art).
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
        if let Some(facing) = Facing::of(grid(d)) {
            pilot.facing = facing;
        }
    } else {
        pilot.walked = 0.0;
    }
    let (id, flip) = pilot_frame(pilot.facing, moving, pilot.walked);
    if sprite.flip_x != flip {
        sprite.flip_x = flip;
    }
    if let Some(image) = art.get(&mut images, &id) {
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
            let label = format!("[Click / E] {}", hotspot.verb());
            if text.0 != label {
                text.0 = label;
            }
            // Over a crew member's head; for a station, under the Pilot's
            // feet (the station fills the space above its use point).
            let pos = match hotspot {
                Hotspot::Crew(_) => world(at) + Vec2::Y * (CHARACTER_H + 28.0),
                Hotspot::Berth(_) | Hotspot::UpgradeBench => {
                    pilot.translation.truncate() - Vec2::Y * 16.0
                }
            };
            tf.translation = pos.extend(Z_LABEL + 1.0);
            *vis = Visibility::Visible;
        }
        _ => *vis = Visibility::Hidden,
    }
}

fn update_hud(save: Res<SaveSlot>, mut hud: Query<(&mut Text, &HudField)>) {
    let (credits, crystal) = save
        .game
        .as_ref()
        .map(|g| (g.credits, workshop::void_crystal(g)))
        .unwrap_or_default();
    for (mut text, field) in &mut hud {
        let value = match field {
            HudField::Credits => format!("{credits} credits  "),
            HudField::Crystal => format!("{crystal} Void Crystal"),
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
                "[Click / E] Next"
            } else if launch.is_some() {
                "[Click / E] Launch prep"
            } else {
                "[Click / E] Close"
            };
            commands
                .spawn((
                    ScreenEntity,
                    OverlayUi,
                    // A click anywhere on the box advances it.
                    Button,
                    Click::Next,
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
                        c.spawn(Node {
                            column_gap: px(10),
                            ..default()
                        })
                        .with_children(|row| {
                            text_button(row, Click::Next, more);
                            text_button(row, Click::Leave, "[Click / X] Leave");
                        });
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
                    p.spawn(Node {
                        column_gap: px(10),
                        ..default()
                    })
                    .with_children(|row| {
                        text_button(row, Click::Launch, "[Click / E] Launch");
                        text_button(row, Click::Leave, "[Click / X] Back");
                    });
                });
        }
        Overlay::Workshop { index, message } => {
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
                    .with_children(|title| {
                        title.spawn((Text::new("Workshop upgrades"), font(22.0, Color::WHITE)));
                        text_button(title, Click::Leave, "[Click / X] Close");
                    });
                    p.spawn((
                        Text::new(format!(
                            "Credits {}    Void Crystal {}",
                            game.credits,
                            workshop::void_crystal(&game)
                        )),
                        font(18.0, Color::srgb(0.95, 0.9, 0.6)),
                    ));
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
                            p.spawn((row_bg(i == *index), Button, Click::Row(i)))
                                .with_children(|row| {
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

                    if !message.is_empty() {
                        p.spawn((
                            Text::new(message.clone()),
                            font(16.0, Color::srgb(1.0, 0.85, 0.4)),
                        ));
                    }
                    p.spawn((
                        Text::new(
                            "[Click a row] Buy    [W]/[S] or [1]-[3] Choose    [E] Buy    [X] Close",
                        ),
                        font(15.0, Color::srgb(0.6, 0.7, 0.75)),
                    ));
                });
        }
        Overlay::Build {
            category,
            tool,
            message,
        } => {
            spawn_build_ui(
                &mut commands,
                &mut art,
                &mut images,
                &game,
                *category,
                *tool,
                message,
            );
        }
    }
}

/// Build mode's screen, after the ONI-style mock
/// (design/art/demo-v2/builder-ui) and § Building › Screen: the category
/// bar bottom left (Rooms, Stations, Furniture, Decor, then the Move and
/// Remove tools), the flyout of the open category's cards (sprite or room
/// outline, name, cost chip and build time, greyed with the reason when
/// locked), and the detail panel for what is picked. The ghost's reason
/// line is `build_preview`'s.
#[allow(clippy::too_many_arguments)]
fn spawn_build_ui(
    commands: &mut Commands,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    game: &SaveGame,
    category: BuildCategory,
    tool: Option<Tool>,
    message: &str,
) {
    let credits = art.get(images, art::ids::ICON_CREDITS);
    let crystal = art.get(images, art::ids::ICON_VOID_CRYSTAL);
    let font = |size: f32, color: Color| {
        (
            TextFont {
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
        )
    };
    // A plain pill, not the kit's chip art: that has a coin and a crystal
    // painted in, which landed under the numbers (TAKOAI-67).
    let cost_chip = |p: &mut ChildSpawnerCommands, cost: Cost, color: Color| {
        p.spawn((
            BackgroundColor(Color::srgb(0.09, 0.23, 0.27)),
            BorderColor::all(Color::srgb(0.06, 0.09, 0.12)),
            Node {
                align_items: AlignItems::Center,
                align_self: AlignSelf::Start,
                column_gap: px(2),
                padding: UiRect::axes(px(5), px(1)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
        ))
        .with_children(|c| {
            let parts = [(&credits, cost.credits), (&crystal, cost.void_crystal)];
            for (icon, amount) in parts.into_iter().filter(|(_, a)| *a > 0) {
                if let Some(icon) = icon {
                    c.spawn(icon_node(icon.clone(), 13.0));
                }
                c.spawn((Text::new(amount.to_string()), font(11.0, color)));
            }
        });
    };
    // Panels catch the mouse, so clicks on them don't build underneath.
    let panel = |image: ImageNode| {
        (
            image,
            Interaction::default(),
            FocusPolicy::Block,
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                padding: UiRect::all(px(UI_PANEL_PAD)),
                row_gap: px(5),
                ..default()
            },
        )
    };
    let picked = match tool {
        Some(Tool::Place(e)) => Some(e),
        _ => None,
    };
    let categories: Vec<_> = BuildCategory::ALL
        .iter()
        .map(|&c| {
            let style = if c == category {
                UI_CARD_SELECTED
            } else {
                UI_CARD
            };
            let icon = c.icon_id().and_then(|id| art.get(images, &id));
            (c, icon, kit(art, images, style))
        })
        .collect();
    let cards: Vec<_> = category
        .entries()
        .into_iter()
        .map(|e| {
            let card = entry_card(game, e);
            let icon = card.icon_id.as_deref().and_then(|id| art.get(images, id));
            let style = if card.lock.is_some() {
                UI_CARD_DISABLED
            } else if Some(e) == picked {
                UI_CARD_SELECTED
            } else {
                UI_CARD
            };
            (e, card, icon, card_kit(art, images, style))
        })
        .collect();
    let panels: Vec<_> = (0..3).map(|_| kit(art, images, UI_PANEL)).collect();
    let mut panels = panels.into_iter();
    let detail = picked.map(|e| entry_card(game, e));
    let detail_icon = detail
        .as_ref()
        .and_then(|d| d.icon_id.as_deref())
        .and_then(|id| art.get(images, id));
    // An entry's icon: its sprite or a room's outline at scale.
    let entry_icon = |p: &mut ChildSpawnerCommands,
                      card: &EntryCard,
                      icon: Option<Handle<Image>>,
                      dim: Color| {
        if let Some((w, d)) = card.outline {
            p.spawn((
                BorderColor::all(BUILD_GOLD.with_alpha(dim.alpha())),
                Node {
                    width: px(w as f32 * ROOM_ICON_SCALE),
                    height: px(d as f32 * ROOM_ICON_SCALE),
                    border: UiRect::all(px(ROOM_ICON_SCALE)),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            return;
        }
        let node = Node {
            width: px(26),
            height: px(26),
            flex_shrink: 0.0,
            ..default()
        };
        if let Some(icon) = icon {
            p.spawn((ImageNode::new(icon).with_color(dim), node));
        }
    };

    // One row along the bottom edge: category bar, flyout, detail panel.
    commands
        .spawn((
            ScreenEntity,
            OverlayUi,
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                right: px(12),
                bottom: px(12),
                column_gap: px(8),
                align_items: AlignItems::End,
                ..default()
            },
            GlobalZIndex(5),
        ))
        .with_children(|root| {
            // Category bar: two buttons a row.
            root.spawn(panel(panels.next().unwrap_or_default()))
                .with_children(|p| {
                    p.spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        width: px(2.0 * BUILD_CATEGORY_BUTTON.0 + 4.0),
                        column_gap: px(4),
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|grid| {
                        for (c, icon, button) in categories {
                            grid.spawn((
                                Button,
                                Click::Category(c),
                                button,
                                Node {
                                    width: px(BUILD_CATEGORY_BUTTON.0),
                                    height: px(BUILD_CATEGORY_BUTTON.1),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    row_gap: px(1),
                                    padding: UiRect::all(px(2)),
                                    ..default()
                                },
                            ))
                            .with_children(|b| {
                                if let Some(icon) = icon {
                                    b.spawn((
                                        ImageNode::new(icon),
                                        Node {
                                            width: px(22),
                                            height: px(22),
                                            flex_shrink: 0.0,
                                            ..default()
                                        },
                                    ));
                                }
                                b.spawn((Text::new(c.name()), font(9.0, BUILD_GOLD)));
                            });
                        }
                    });
                });

            // Flyout of build cards (or what the tool does).
            root.spawn(panel(panels.next().unwrap_or_default()))
                .insert(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: px(0),
                    padding: UiRect::all(px(UI_PANEL_PAD)),
                    row_gap: px(5),
                    ..default()
                })
                .with_children(|p| {
                    p.spawn((Text::new(category.name()), font(18.0, BUILD_GOLD)));
                    let how = match category {
                        BuildCategory::Move => Some(
                            "Click a finished item to lift it, then click where it goes. Free and instant. A berth carries its ship.",
                        ),
                        BuildCategory::Remove => Some(
                            "Click an item, a construction site or an empty room you built. Refunded in full.",
                        ),
                        _ => None,
                    };
                    if let Some(how) = how {
                        p.spawn((Text::new(how), font(12.0, Color::WHITE)));
                    }
                    p.spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(6),
                        row_gap: px(6),
                        ..default()
                    })
                    .with_children(|row| {
                        for (n, (e, card, icon, style)) in cards.into_iter().enumerate() {
                            let locked = card.lock.is_some();
                            let (text, dim) = if locked {
                                (Color::srgb(0.55, 0.6, 0.62), Color::srgb(0.45, 0.45, 0.45))
                            } else {
                                (BUILD_GOLD, Color::WHITE)
                            };
                            row.spawn((
                                Button,
                                Click::Entry(e),
                                style,
                                Node {
                                    width: px(BUILD_CARD_W),
                                    height: px(BUILD_CARD_H),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::axes(px(UI_CARD_PAD), px(8)),
                                    row_gap: px(2),
                                    ..default()
                                },
                            ))
                            .with_children(|c| {
                                // The key and name; the icon with the build
                                // time, or why it's locked; the cost pill.
                                // Fixed rows, clipped, so nothing spills out
                                // of the card's rim.
                                let key = if n < 9 {
                                    format!("{} ", n + 1)
                                } else {
                                    String::new()
                                };
                                let clip = |height: f32| Node {
                                    height: px(height),
                                    flex_shrink: 0.0,
                                    overflow: Overflow::clip(),
                                    ..default()
                                };
                                c.spawn(clip(22.0)).with_child((
                                    Text::new(format!("{key}{}", card.name)),
                                    font(9.0, text),
                                ));
                                c.spawn(Node {
                                    column_gap: px(4),
                                    align_items: AlignItems::Center,
                                    ..clip(28.0)
                                })
                                .with_children(|r| {
                                    entry_icon(r, &card, icon, dim);
                                    r.spawn((
                                        Text::new(format!("{} s", card.seconds)),
                                        font(8.0, text),
                                    ));
                                });
                                // Why it's locked gets the card's full width
                                // (two lines), in place of the cost pill.
                                if let Some(lock) = &card.lock {
                                    c.spawn(clip(22.0)).with_child((
                                        Text::new(lock.clone()),
                                        font(8.0, Color::srgb(0.85, 0.85, 0.85)),
                                    ));
                                } else if card.cost != Cost::default() {
                                    c.spawn((
                                        BackgroundColor(Color::srgb(0.09, 0.23, 0.27)),
                                        Node {
                                            align_self: AlignSelf::Start,
                                            padding: UiRect::axes(px(5), px(1)),
                                            border_radius: BorderRadius::all(px(7)),
                                            ..clip(14.0)
                                        },
                                    ))
                                    .with_child((
                                        Text::new(cost_text(card.cost)),
                                        font(8.0, text),
                                    ));
                                }
                            });
                        }
                    });
                    p.spawn((
                        Text::new(
                            "[Click] Build  [1]-[9] Pick  [Right click / Esc] Cancel  [WASD] Pan  [Wheel] Zoom  [B] Close",
                        ),
                        font(11.0, Color::srgb(0.6, 0.7, 0.75)),
                    ));
                });

            // Detail panel for what is picked.
            root.spawn(panel(panels.next().unwrap_or_default()))
                // A fixed width: a shrinking panel measured its wrapped
                // text too short, and the text spilt out of the frame.
                .insert(Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    width: px(BUILD_DETAIL_W),
                    padding: UiRect::all(px(UI_PANEL_PAD)),
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|p| {
                    let info = Color::srgb(0.8, 0.9, 0.92);
                    match (picked, &detail) {
                        (Some(_), Some(card)) => {
                            p.spawn(Node {
                                column_gap: px(6),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|r| {
                                entry_icon(r, card, detail_icon.clone(), Color::WHITE);
                                r.spawn((Text::new(card.name), font(17.0, BUILD_GOLD)));
                            });
                            p.spawn((Text::new(card.effect.clone()), font(12.0, Color::WHITE)));
                            p.spawn((
                                Text::new(format!("Size: {}", card.size)),
                                font(12.0, info),
                            ));
                            p.spawn((
                                Text::new(format!("Placement: {}", card.placement)),
                                font(12.0, info),
                            ));
                            cost_chip(p, card.cost, BUILD_GOLD);
                            p.spawn((
                                Text::new(format!("Build time: {} s", card.seconds)),
                                font(12.0, info),
                            ));
                            if let Some(lock) = &card.lock {
                                p.spawn((
                                    Text::new(format!("Locked: {lock}")),
                                    font(12.0, Color::srgb(1.0, 0.6, 0.55)),
                                ));
                            }
                        }
                        _ => {
                            let (name, text) = match tool {
                                Some(Tool::Move(Some(i))) => (
                                    "Move",
                                    format!(
                                        "Holding the {}. Click to put it down.",
                                        game.carrier.items.get(i).map_or("item", |it| it.id.name())
                                    ),
                                ),
                                Some(Tool::Move(None)) => ("Move", "Pick up an item.".to_string()),
                                Some(Tool::Remove) => ("Remove", "Full refund.".to_string()),
                                _ => (category.name(), "Pick a card to build.".to_string()),
                            };
                            p.spawn((Text::new(name), font(17.0, BUILD_GOLD)));
                            p.spawn((Text::new(text), font(12.0, info)));
                        }
                    }
                    if !message.is_empty() {
                        p.spawn((Text::new(message), font(12.0, Color::srgb(1.0, 0.85, 0.4))));
                    }
                    sized_button(p, Click::Leave, "[Esc] Back", 13.0);
                });
        });
}

/// A small bordered button in a Carrier panel.
fn text_button(p: &mut ChildSpawnerCommands, click: Click, text: &str) {
    sized_button(p, click, text, 15.0);
}

/// A [`text_button`] with `size` px text.
fn sized_button(p: &mut ChildSpawnerCommands, click: Click, text: &str, size: f32) {
    p.spawn((
        Button,
        click,
        Hoverable,
        BackgroundColor(BUTTON_BG),
        BorderColor::all(Color::srgb(0.35, 0.52, 0.58)),
        Node {
            padding: UiRect::axes(px(10), px(4)),
            border: UiRect::all(px(1)),
            align_self: AlignSelf::Start,
            ..default()
        },
    ))
    .with_child((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.9, 0.95)),
    ));
}

/// Hover feedback for [`text_button`]s.
#[allow(clippy::type_complexity)]
fn hover_buttons(
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Hoverable>),
    >,
) {
    for (interaction, mut bg) in &mut buttons {
        bg.0 = match interaction {
            Interaction::None => BUTTON_BG,
            _ => BUTTON_HOVER,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every catalogue entry ships `core.carrier.item.<id>`, drawn at 2× on
    /// a canvas its footprint fits (§ Carrier › Art spec): one tile (128
    /// art px) wide per footprint column; a wall-mounted piece 96 px tall,
    /// anything else its footprint depth plus headroom in steps of 32 px.
    /// `item_sprite` pins the canvas's bottom centre at the item's feet.
    #[test]
    fn catalogue_art_fits_its_footprint() {
        const ART_TILE: u32 = 2 * TILE as u32;
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
        let manifest = content::ContentManifest::load(format!("{root}/manifest.json")).unwrap();
        for d in &CATALOGUE {
            let id = d.id.content_id();
            let path = manifest
                .processed_path(root, &id)
                .unwrap_or_else(|e| panic!("{id}: {e:?}"));
            let (w, h) = image::image_dimensions(&path).unwrap_or_else(|e| panic!("{id}: {e}"));
            let (fw, fd) = (d.size.0 as u32, d.size.1 as u32);
            assert_eq!(w, fw * ART_TILE, "{id}: width for {fw}x{fd}");
            match d.layer {
                Layer::Wall => assert_eq!(h, 96, "{id}: wall piece height"),
                _ => assert!(
                    h >= fd * ART_TILE && (h - fd * ART_TILE).is_multiple_of(32),
                    "{id}: height {h} for depth {fd}"
                ),
            }
        }
    }

    #[test]
    fn pilot_walk_cycle_steps_with_distance_and_idles_when_still() {
        let frame = |moving, walked| pilot_frame(Facing::S, moving, walked).0;
        assert_eq!(frame(false, 50.0), "core.carrier.pilot.s.idle");
        assert_eq!(frame(true, 0.0), "core.carrier.pilot.s.walk_1");
        assert_eq!(frame(true, STRIDE * 2.5), "core.carrier.pilot.s.walk_3");
        assert_eq!(frame(true, STRIDE * 5.0), "core.carrier.pilot.s.walk_2");
    }

    #[test]
    fn pilot_faces_8_directions_mirroring_the_west_ones() {
        let cases = [
            ((1.0, 0.0), Facing::E, "e", false),
            ((1.0, 1.0), Facing::SE, "se", false),
            ((0.0, 1.0), Facing::S, "s", false),
            ((-1.0, 1.0), Facing::SW, "se", true),
            ((-1.0, 0.0), Facing::W, "e", true),
            ((-1.0, -1.0), Facing::NW, "ne", true),
            ((0.0, -1.0), Facing::N, "n", false),
            ((1.0, -1.0), Facing::NE, "ne", false),
            // Snapped to the nearest of the 8: mostly east, a bit south.
            ((5.0, 1.0), Facing::E, "e", false),
            ((1.0, 3.0), Facing::S, "s", false),
        ];
        for ((x, y), facing, dir, flip) in cases {
            let f = Facing::of(Vec2::new(x, y)).unwrap();
            assert_eq!(f, facing, "{x},{y}");
            assert_eq!(f.art(), (dir, flip), "{x},{y}");
            assert_eq!(
                pilot_frame(f, false, 0.0),
                (format!("core.carrier.pilot.{dir}.idle"), flip)
            );
        }
        assert_eq!(Facing::of(Vec2::ZERO), None);
    }

    fn starting() -> (Carrier, Vec<Rect>, Vec<(Hotspot, Vec2)>) {
        let c = Carrier::starting();
        let rects = c.walkable();
        let spots = hotspots(&c);
        (c, rects, spots)
    }

    fn rich(credits: u32, crystal: u32) -> SaveGame {
        let mut game = SaveGame {
            credits,
            ..Default::default()
        };
        game.resources.insert("void_crystal".into(), crystal);
        game
    }

    #[test]
    fn crew_stand_at_their_posts_and_every_hotspot_is_reachable() {
        let (c, rects, spots) = starting();
        let at = |spots: &[(Hotspot, Vec2)], h| spots.iter().find(|s| s.0 == h).map(|s| s.1);
        // § Starting layout: Gunner (5,2), Researcher (19,2), Engineer (17,7).
        assert_eq!(
            at(&spots, Hotspot::Crew(Crew::Gunner)),
            Some(tile_centre((5, 2)))
        );
        assert_eq!(
            at(&spots, Hotspot::Crew(Crew::Researcher)),
            Some(tile_centre((19, 2)))
        );
        assert_eq!(
            at(&spots, Hotspot::Crew(Crew::Engineer)),
            Some(tile_centre((17, 7)))
        );
        // Stations at their use points, 24 px south of the front edge.
        assert_eq!(
            at(&spots, Hotspot::UpgradeBench),
            Some(Vec2::new(16.0 * TILE, 7.0 * TILE + 24.0))
        );
        assert_eq!(
            at(&spots, Hotspot::Berth(0)),
            Some(Vec2::new(4.0 * TILE, 12.0 * TILE + 24.0))
        );
        assert_eq!(
            at(&spots, Hotspot::Berth(1)),
            Some(Vec2::new(10.0 * TILE, 12.0 * TILE + 24.0))
        );
        assert_eq!(spots.len(), 6);
        // Somewhere the Pilot can stand reaches each with E.
        for &(h, p) in &spots {
            let reachable = (-7..=7).any(|i| {
                (-7..=7).any(|j| {
                    let q = p + Vec2::new(i as f32, j as f32) * 8.0;
                    layout::can_stand(&rects, q.x, q.y)
                        && nearest_hotspot(&spots, q).map(|n| n.0) == Some(h)
                })
            });
            assert!(reachable, "{h:?} at {p}");
        }
        // The Gunner follows the console when it moves; a site can't be used.
        let mut moved = c.clone();
        let console = moved.find(ItemId::CommandConsole).unwrap();
        assert_eq!(moved.can_move(console, 4, 1, None), Ok(()));
        moved.move_item(console, 4, 1);
        let spots = hotspots(&moved);
        assert_eq!(
            at(&spots, Hotspot::Crew(Crew::Gunner)),
            Some(tile_centre((6, 2)))
        );
        let bench = moved.find(ItemId::UpgradeBench).unwrap();
        moved.items[bench].build_left_ms = Some(100);
        assert_eq!(at(&hotspots(&moved), Hotspot::UpgradeBench), None);
    }

    #[test]
    fn clicking_a_ship_walks_there_and_boards_it() {
        let (c, rects, spots) = starting();
        let start = spawn_point(&c, None);
        let mut order = order_walk(&rects, &spots, start, Vec2::ZERO, Some(Hotspot::Berth(1)))
            .expect("Bulwark's berth is reachable");
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
        let floor = order_walk(&rects, &spots, start, tile_centre((3, 4)), None).unwrap();
        assert_eq!(floor.interact, None);
        assert!(order_walk(&rects, &spots, start, tile_centre((30, 18)), None).is_none());
    }

    #[test]
    fn clicks_pick_crew_and_usable_stations() {
        let (c, _, spots) = starting();
        // A crew member is clicked on their sprite.
        let gunner = tile_centre((5, 2)) - Vec2::Y * 30.0;
        assert_eq!(
            clicked_hotspot(&spots, &c, gunner),
            Some(Hotspot::Crew(Crew::Gunner))
        );
        // The bench on its tile, or on its art rising into the tile above.
        for t in [(15, 6), (16, 5)] {
            assert_eq!(
                clicked_hotspot(&spots, &c, tile_centre(t)),
                Some(Hotspot::UpgradeBench),
                "{t:?}"
            );
        }
        // A berth's pad, under its ship.
        assert_eq!(
            clicked_hotspot(&spots, &c, tile_centre((10, 10))),
            Some(Hotspot::Berth(1))
        );
        // A station with nothing to use yet, and bare floor.
        assert_eq!(clicked_hotspot(&spots, &c, tile_centre((3, 1))), None);
        assert_eq!(clicked_hotspot(&spots, &c, tile_centre((10, 13))), None);
    }

    #[test]
    fn holding_the_button_keeps_the_pilot_following_the_cursor() {
        let (_, rects, spots) = starting();
        // Where the tour leaves the Pilot, which is where the drag starts.
        let mut feet = PILOT_TOUR[PILOT_TOUR.len() - 1].0;
        assert_eq!(feet, PILOT_DRAG[0]);
        let mut order = WalkOrder::default();
        let mut ordered = None;
        let mut facings = Vec::new();
        let mut walks = 0;
        let end = (PILOT_DRAG.len() - 1) as f32 * PILOT_DRAG_LEG;
        // Held for the sweep, then released once the cursor stops: the
        // Pilot finishes walking to the last cursor spot.
        for frame in 0..60 * 5 {
            let t = frame as f32 / 60.0;
            if t <= end {
                let cursor = drag_cursor(t);
                let step = drag_step(feet, cursor, ordered);
                match step {
                    DragStep::Keep => {}
                    DragStep::Stop => order = WalkOrder::default(),
                    DragStep::Walk => {
                        order = order_walk(&rects, &spots, feet, cursor, None).expect("floor");
                        walks += 1;
                    }
                }
                if step != DragStep::Keep {
                    ordered = Some(cursor);
                }
            }
            let before = feet;
            (feet, _) = follow(&rects, &mut order, feet, WALK_SPEED / 60.0);
            assert!(layout::can_stand(&rects, feet.x, feet.y), "{feet}");
            if let Some(f) = Facing::of(feet - before) {
                if facings.last() != Some(&f) {
                    facings.push(f);
                }
            }
        }
        assert!(feet.distance(PILOT_DRAG[3]) < 0.5, "ends at {feet}");
        // Re-pathed all along the sweep, not once per press.
        assert!(walks > 20, "{walks} walks");
        // West, then turning south, then east, without flip-flopping.
        assert_eq!(facings.first(), Some(&Facing::W), "{facings:?}");
        assert_eq!(facings.last(), Some(&Facing::E), "{facings:?}");
        assert!(facings.len() <= 6, "{facings:?}");
    }

    #[test]
    fn pilot_tour_walks_once_in_every_facing() {
        let (c, rects, spots) = starting();
        let mut feet = spawn_point(&c, None);
        let mut seen = Vec::new();
        for (to, name) in PILOT_TOUR {
            let mut order = order_walk(&rects, &spots, feet, to, None).expect("on the Bridge");
            // One straight leg, so the facing holds for the whole walk.
            assert_eq!(order.path, vec![to], "{name}");
            let facing = Facing::of(to - feet).unwrap();
            assert_eq!(format!("{facing:?}").to_lowercase(), name);
            // Still walking 0.5 s in, when the autoplay shoots; done within
            // its 1.4 s leg.
            assert!(feet.distance(to) > WALK_SPEED * 0.5, "{name}");
            assert!(feet.distance(to) < WALK_SPEED * 1.4, "{name}");
            let mut done = false;
            for _ in 0..60 * 2 {
                (feet, done) = follow(&rects, &mut order, feet, WALK_SPEED / 60.0);
                if done {
                    break;
                }
            }
            assert!(done && feet.distance(to) < 0.5, "{name} ends at {feet}");
            seen.push(facing);
        }
        for facing in Facing::BY_ANGLE {
            assert!(seen.contains(&facing), "{facing:?}");
        }
    }

    #[test]
    fn spawns_at_the_console_or_the_launched_ships_berth() {
        let (c, rects, spots) = starting();
        let new_game = spawn_point(&c, None);
        let console = c.find(ItemId::CommandConsole).unwrap();
        assert_eq!(new_game, Vec2::from(c.items[console].use_point()));
        assert!(layout::can_stand(&rects, new_game.x, new_game.y));
        assert_eq!(nearest_hotspot(&spots, new_game), None);
        for (i, ship) in SHIPS.iter().enumerate() {
            let p = spawn_point(&c, Some(ship.id));
            assert!(layout::can_stand(&rects, p.x, p.y), "{}", ship.id);
            assert_eq!(
                nearest_hotspot(&spots, p).map(|h| h.0),
                Some(Hotspot::Berth(i))
            );
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
    }

    #[test]
    fn camera_stays_within_two_tiles_of_the_hull() {
        let half = Vec2::new(640.0, 360.0);
        // Pilot at the north-west corner: the view stops two tiles out.
        let c = clamp_camera(Vec2::new(0.0, 0.0), half, 0.0);
        assert_eq!(c, Vec2::new(-2.0 * TILE + 640.0, 2.0 * TILE - 360.0));
        // Overview: the whole hull fits 1280 × 720, centred across.
        let (w, h) = (HULL_W as f32 * TILE, HULL_H as f32 * TILE);
        assert!(w <= 1280.0 / OVERVIEW && h <= 720.0 / OVERVIEW);
        let c = clamp_camera(Vec2::ZERO, half / OVERVIEW, 0.0);
        assert_eq!(c.x, w / 2.0);
        // Build mode: the view can go far enough south that the last hull
        // row sits above the panels covering the screen bottom.
        let half = half / OVERVIEW;
        let below = BUILD_UI_H / OVERVIEW;
        let c = clamp_camera(Vec2::new(0.0, -10_000.0), half, below);
        let screen_bottom = c.y - half.y;
        assert!(-h - screen_bottom >= below, "{c}");
        assert_eq!(world(grid(Vec2::new(3.0, -4.0))), Vec2::new(3.0, -4.0));
    }

    #[test]
    fn build_clicks_place_move_and_remove() {
        let mut game = rich(300, 6);
        let vc = |g: &SaveGame| workshop::void_crystal(g);
        // The mouse is a room's middle floor tile: (17,12) anchors the
        // spec's Small Room at (14,10).
        let room = Tool::Place(Entry::Room(RoomSize::Small));
        let mouse = room_mouse(RoomSize::Small, (14, 10));
        assert_eq!(mouse, (17, 12));
        assert_eq!(Room::anchor_for(RoomSize::Small, mouse), (14, 10));
        let out = apply_build_click(&mut game, room, mouse, None);
        assert_eq!(
            out,
            BuildOutcome {
                changed: true,
                message: "Building a Small Room: 60 cr + 1 VC.".into(),
                tool: Some(room),
            }
        );
        assert_eq!((game.credits, vc(&game)), (240, 5));
        let out = apply_build_click(&mut game, room, mouse, None);
        assert_eq!(
            (out.changed, out.message.as_str()),
            (false, "Overlaps a room")
        );
        // The site takes nothing until it finishes.
        let sorter = Tool::Place(Entry::Item(ItemId::SalvageSorter));
        let out = apply_build_click(&mut game, sorter, (19, 11), None);
        assert_eq!(out.message, "Still being built");
        game.carrier.tick(5000);
        // A buy-once station goes back to the flyout once placed.
        let out = apply_build_click(&mut game, sorter, (19, 11), None);
        assert!(out.changed, "{}", out.message);
        assert_eq!(out.tool, None);
        assert_eq!((game.credits, vc(&game)), (120, 2));
        // Furniture stays picked for another copy; never on the Pilot.
        let stool = Tool::Place(Entry::Item(ItemId::Stool));
        let out = apply_build_click(&mut game, stool, (16, 12), Some((16, 12)));
        assert_eq!(out.message, "The Pilot is in the way");
        let out = apply_build_click(&mut game, stool, (16, 12), None);
        assert_eq!((out.changed, out.tool), (true, Some(stool)));
        assert_eq!(game.credits, 110);

        // Move: lift the Upgrade Bench and put it down by the east wall,
        // free; it stays a starter piece and the Engineer follows it.
        let lift = apply_build_click(&mut game, Tool::Move(None), (15, 6), None);
        let bench = game.carrier.find(ItemId::UpgradeBench).unwrap();
        assert_eq!(
            (lift.changed, lift.tool),
            (false, Some(Tool::Move(Some(bench))))
        );
        let put = apply_build_click(&mut game, Tool::Move(Some(bench)), (19, 6), None);
        assert!(put.changed, "{}", put.message);
        assert_eq!(put.tool, Some(Tool::Move(None)));
        let bench = &game.carrier.items[game.carrier.find(ItemId::UpgradeBench).unwrap()];
        assert_eq!((bench.x, bench.y, bench.starter), (19, 6, true));
        assert_eq!(game.credits, 110);
        assert_eq!(
            game.carrier.crew_post(layout::Crew::Engineer),
            Some((18, 7))
        );
        // A berth carries its ship.
        let lift = apply_build_click(&mut game, Tool::Move(None), (3, 9), None);
        let put = apply_build_click(&mut game, lift.tool.unwrap(), (3, 8), None);
        assert!(put.changed, "{}", put.message);
        let kite = &game.carrier.items[game.carrier.berth_of("kite").unwrap()];
        assert_eq!((kite.x, kite.y), (2, 7));
        // A refused drop keeps the item held.
        let out = apply_build_click(&mut game, Tool::Move(Some(0)), (30, 18), None);
        assert_eq!((out.changed, out.tool), (false, Some(Tool::Move(Some(0)))));

        // Remove: furniture and stations for their price, a room once it's
        // empty, never a fixed station or the starting rooms.
        let remove = |game: &mut SaveGame, t: Tile| apply_build_click(game, Tool::Remove, t, None);
        assert_eq!(
            remove(&mut game, (16, 12)).message,
            "Removed the Stool (+10 cr)."
        );
        assert_eq!(remove(&mut game, (20, 6)).message, "Move it instead");
        assert_eq!(remove(&mut game, (16, 13)).message, "Room isn't empty");
        assert_eq!(
            remove(&mut game, (19, 12)).message,
            "Removed the Salvage Sorter (+120 cr + 3 VC)."
        );
        assert_eq!(
            remove(&mut game, (16, 13)).message,
            "Removed the Small Room (+60 cr + 1 VC)."
        );
        assert_eq!((game.credits, vc(&game)), (300, 6));
        assert_eq!(
            remove(&mut game, (2, 3)).message,
            "Part of the starting carrier"
        );
        assert_eq!(remove(&mut game, (6, 1)).message, "Removed the Floor lamp.");
        assert_eq!(remove(&mut game, (30, 18)).message, "");
    }

    #[test]
    fn right_click_and_esc_back_out_one_step() {
        assert_eq!(
            back_out(Some(Tool::Move(Some(3)))),
            Some(Some(Tool::Move(None)))
        );
        assert_eq!(back_out(Some(Tool::Move(None))), Some(None));
        assert_eq!(back_out(Some(Tool::Remove)), Some(None));
        let small = Tool::Place(Entry::Room(RoomSize::Small));
        assert_eq!(back_out(Some(small)), Some(None));
        assert_eq!(back_out(None), None);
    }

    #[test]
    fn build_cards_read_the_catalogue_and_grey_out_when_locked() {
        let counts: Vec<usize> = BuildCategory::ALL
            .iter()
            .map(|c| c.entries().len())
            .collect();
        assert_eq!(counts, [2, 6, 15, 4, 0, 0]);
        assert_eq!(BuildCategory::Move.tool(), Some(Tool::Move(None)));
        assert_eq!(BuildCategory::Remove.tool(), Some(Tool::Remove));
        assert_eq!(BuildCategory::Decor.tool(), None);
        for c in BuildCategory::ALL {
            for e in c.entries() {
                assert_eq!(BuildCategory::of(Tool::Place(e)), c, "{e:?}");
            }
        }

        let mut game = rich(100, 2);
        let small = entry_card(&game, Entry::Room(RoomSize::Small));
        assert_eq!(
            small.cost,
            Cost {
                credits: 60,
                void_crystal: 1
            }
        );
        assert_eq!(
            (small.seconds, small.outline, small.icon_id),
            (5, Some((8, 6)), None)
        );
        assert_eq!((small.size.as_str(), small.lock), ("6 x 4 floor", None));
        let large = entry_card(&game, Entry::Room(RoomSize::Large));
        assert_eq!(large.lock.as_deref(), Some("Need 220 cr + 4 VC"));
        let console = entry_card(&game, Entry::Item(ItemId::CommandConsole));
        assert_eq!(
            console.icon_id.as_deref(),
            Some("core.carrier.item.command_console")
        );
        assert_eq!(console.placement, "Against a wall");
        assert_eq!(
            console.lock.as_deref(),
            Some("Part of the starting carrier")
        );
        let rug = entry_card(&game, Entry::Item(ItemId::Rug));
        assert_eq!(
            (rug.effect.as_str(), rug.seconds, rug.size.as_str()),
            ("Cosmetic", 2, "2 x 2")
        );
        // Picking a greyed-out card explains why.
        let sorter = Entry::Item(ItemId::SalvageSorter);
        assert_eq!(
            pick_entry(&game, sorter),
            Err("Salvage Sorter: Need 120 cr + 3 VC.".into())
        );
        game.credits = 500;
        game.resources.insert("void_crystal".into(), 10);
        assert_eq!(pick_entry(&game, sorter), Ok(Tool::Place(sorter)));
        // Buy-once: a site already counts.
        game.carrier.place(ItemId::SalvageSorter, 12, 9, None);
        assert_eq!(
            entry_card(&game, sorter).lock.as_deref(),
            Some("Already built")
        );
    }

    #[test]
    fn ghosts_show_the_cost_or_why_not() {
        let mut game = rich(60, 1);
        let room = Tool::Place(Entry::Room(RoomSize::Small));
        let g = ghost(&game, room, (17, 12), None).unwrap();
        assert!(g.ok, "{}", g.text);
        assert_eq!((g.at, g.size), ((14, 10), (8, 6)));
        assert_eq!(g.text, "Small Room: 60 cr + 1 VC - 5 s");
        // The doorways it would open (§ Starting layout, example edits).
        let mut doors = g.doorways.clone();
        doors.sort();
        assert_eq!(doors, [(14, 12), (14, 13), (17, 10), (18, 10)]);
        let g = ghost(&game, room, (30, 18), None).unwrap();
        assert_eq!(
            (g.ok, g.text.as_str()),
            (false, "Must share a wall with a room")
        );
        // A stool on the new doorway's clearance: refused, and it flashes.
        let stool = game.carrier.place(ItemId::Stool, 17, 9, None);
        let g = ghost(&game, room, (17, 12), None).unwrap();
        assert_eq!((g.ok, g.text.as_str()), (false, "Clear the doorway first"));
        assert_eq!(g.flash, [stool]);
        // An item: its footprint around the mouse and its front strip.
        game.credits = 500;
        game.resources.insert("void_crystal".into(), 10);
        let sim = Tool::Place(Entry::Item(ItemId::CombatSimulator));
        let g = ghost(&game, sim, (12, 9), None).unwrap();
        assert!(g.ok, "{}", g.text);
        assert_eq!(g.text, "Combat Simulator: 200 cr + 6 VC - 4 s");
        assert_eq!((g.at, g.front), ((12, 9), vec![(12, 11), (13, 11)]));
        let g = ghost(&game, sim, (13, 7), None).unwrap();
        assert_eq!((g.ok, g.text.as_str()), (false, "Keep the doorway clear"));
        // Move and Remove show the piece under the mouse, or nothing.
        let g = ghost(&game, Tool::Remove, (3, 1), None).unwrap();
        assert_eq!(
            (g.at, g.ok, g.text.as_str()),
            ((3, 1), false, "Move it instead")
        );
        let g = ghost(&game, Tool::Move(None), (4, 1), None).unwrap();
        assert_eq!((g.ok, g.text.as_str()), (true, "Move the Command Console"));
        let g = ghost(&game, Tool::Remove, (16, 3), None).unwrap();
        assert_eq!(g.text, "Part of the starting carrier");
        assert_eq!(ghost(&game, Tool::Move(None), (10, 13), None), None);
    }

    fn tiles_at(draws: &[Draw], t: Tile) -> Vec<&Draw> {
        draws
            .iter()
            .filter(|d| d.item.is_none() && d.at == tile_centre(t))
            .collect()
    }

    #[test]
    fn the_deck_draws_floor_under_walls_and_items_by_layer() {
        let mut c = Carrier::starting();
        let draws = deck(&c);
        // Bare hull draws nothing: space shows there.
        assert!(tiles_at(&draws, (30, 18)).is_empty());
        let floor = tiles_at(&draws, (2, 2));
        assert_eq!(floor.len(), 1);
        assert!(floor[0].id.starts_with("core.carrier.tile.floor."));
        // Wall and doorway tiles: a floor, then the tile map's art over it.
        let map = c.tile_map();
        for t in [(0, 0), (7, 1), (7, 2), (3, 15)] {
            let d = tiles_at(&draws, t);
            assert_eq!(d.len(), 2, "{t:?}");
            assert_eq!(Some(d[0].id.clone()), map.floor_art(t));
            assert_eq!(Some(d[1].id.clone()), map.art(t));
            assert!(d[0].z < d[1].z, "{t:?}");
            assert_eq!(d[1].tint, Color::WHITE);
        }
        // Every item once, pinned at its front edge; rugs under wall pieces
        // under objects, and objects sorted by their front edge.
        let items: Vec<&Draw> = draws.iter().filter(|d| d.item.is_some()).collect();
        assert_eq!(items.len(), c.items.len());
        let find = |id: ItemId| *items.iter().find(|d| d.id == id.content_id()).unwrap();
        let console = find(ItemId::CommandConsole);
        assert_eq!(
            (console.at, console.pin),
            (Vec2::new(4.0 * TILE, 2.0 * TILE), Pin::Feet)
        );
        assert!(find(ItemId::Rug).z < find(ItemId::WallScreen).z);
        assert!(find(ItemId::WallScreen).z < find(ItemId::Table).z);
        assert!(find(ItemId::CommandConsole).z < find(ItemId::UpgradeBench).z);
        assert!(find(ItemId::UpgradeBench).z < find(ItemId::Berth).z);

        // A room site: scaffold over its tiles and its doorways shut.
        c.place_room(RoomSize::Small, 14, 10);
        let draws = deck(&c);
        assert!(tiles_at(&draws, (16, 12)).iter().any(|d| d.id == SCAFFOLD));
        let door = tiles_at(&draws, (17, 10));
        assert_eq!(door[1].tint, SHUT);
        // Finished, the doorway opens; an item site draws in blueprint.
        c.tick(5000);
        let i = c.place(ItemId::SalvageSorter, 19, 11, None);
        let draws = deck(&c);
        assert_eq!(tiles_at(&draws, (17, 10))[1].tint, Color::WHITE);
        assert!(!tiles_at(&draws, (16, 12)).iter().any(|d| d.id == SCAFFOLD));
        let site = draws.iter().find(|d| d.item == Some(i)).unwrap();
        assert_eq!(site.tint, BLUEPRINT);
    }

    #[test]
    fn sites_redraw_only_when_they_finish_and_their_bars_fill() {
        let mut c = Carrier::starting();
        c.place_room(RoomSize::Small, 14, 10);
        let room = c.room_at((16, 12)).unwrap();
        let shape = drawn_shape(&c);
        assert_eq!(site_progress(&c, Site::Room(room)), 0.0);
        c.tick(2500);
        assert_eq!(drawn_shape(&c), shape);
        assert_eq!(site_progress(&c, Site::Room(room)), 0.5);
        c.tick(2500);
        assert_ne!(drawn_shape(&c), shape);
        assert_eq!(site_progress(&c, Site::Room(room)), 1.0);
        assert_eq!(site_progress(&c, Site::Item(999)), 1.0);
    }

    /// The scene systems on a bare world: the deck is drawn from the save,
    /// timers run only unpaused, and a site finishing redraws it.
    #[test]
    fn the_scene_follows_the_saved_layout() {
        use bevy::ecs::system::RunSystemOnce;
        use std::time::Duration;

        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        world.insert_resource(ContentImages::repo());
        world.init_resource::<CarrierScene>();
        world.init_resource::<PauseMenu>();
        world.init_resource::<CarrierPalette>();
        world.init_resource::<Time>();
        let mut game = rich(60, 1);
        workshop::build_room(&mut game, RoomSize::Small, 14, 10).unwrap();
        world.insert_resource(SaveSlot::scratch("carrier-scene", game));
        world.run_system_once(sync_layout).unwrap();

        let scene = world.resource::<CarrierScene>();
        assert_eq!(scene.hotspots.len(), 6);
        assert!(!scene.walkable.is_empty());
        let items = Carrier::starting().items.len();
        let mut sprites = world.query::<(&ItemSprite, &Sprite)>();
        let drawn: std::collections::BTreeSet<usize> =
            sprites.iter(&world).map(|(i, _)| i.0).collect();
        assert_eq!(drawn.len(), items);
        assert_eq!(world.query::<&DockedShip>().iter(&world).count(), 2);
        assert_eq!(world.query::<&SiteBar>().iter(&world).count(), 1);
        let count = |world: &mut World| world.query::<&LayoutSprite>().iter(world).count();
        let before = count(&mut world);

        // Paused: the site waits. Unpaused: it runs, the deck stays.
        world.resource_mut::<PauseMenu>().open = true;
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(2500));
        world.run_system_once(run_sites).unwrap();
        let left = |world: &World| {
            world
                .resource::<SaveSlot>()
                .game
                .as_ref()
                .unwrap()
                .carrier
                .rooms[Carrier::starting().rooms.len()]
            .build_left_ms
        };
        assert_eq!(left(&world), Some(5000));
        world.resource_mut::<PauseMenu>().open = false;
        world.run_system_once(run_sites).unwrap();
        assert_eq!(left(&world), Some(2500));
        world.run_system_once(sync_layout).unwrap();
        world.run_system_once(update_site_bars).unwrap();
        assert_eq!(count(&mut world), before);
        let mut bars = world.query::<(&SiteBar, &Sprite)>();
        let (bar, sprite) = bars.single(&world).unwrap();
        assert_eq!(sprite.custom_size, Some(Vec2::new(bar.width * 0.5, BAR_H)));

        // Finished: redrawn without the bar, the room's floor walkable.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(2500));
        world.run_system_once(run_sites).unwrap();
        assert_eq!(left(&world), None);
        world.run_system_once(sync_layout).unwrap();
        assert_eq!(world.query::<&SiteBar>().iter(&world).count(), 0);
        let scene = world.resource::<CarrierScene>();
        let p = tile_centre((17, 12));
        assert!(layout::can_stand(&scene.walkable, p.x, p.y));
    }

    #[test]
    fn a_cursor_resting_on_the_pilot_stands_it_still() {
        let feet = Vec2::new(192.0, 192.0);
        // Held right on the Pilot after a drag: it stops, once.
        assert_eq!(
            drag_step(
                feet,
                feet + Vec2::new(3.0, -2.0),
                Some(feet + Vec2::X * 80.0)
            ),
            DragStep::Stop
        );
        // Small wobbles around it never send it walking (no facing jitter).
        let mut ordered = Some(feet + Vec2::new(3.0, -2.0));
        for d in [
            (5.0, 5.0),
            (-6.0, 4.0),
            (0.0, -11.0),
            (-8.0, -8.0),
            (11.0, 0.0),
        ] {
            let cursor = feet + Vec2::from(d);
            let step = drag_step(feet, cursor, ordered);
            assert_ne!(step, DragStep::Walk, "{d:?}");
            if step != DragStep::Keep {
                ordered = Some(cursor);
            }
        }
        // Moving it away walks again.
        assert_eq!(
            drag_step(feet, feet + Vec2::new(40.0, 0.0), ordered),
            DragStep::Walk
        );
    }

    #[test]
    fn dock_card_shows_combat_simulator_cooldowns() {
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
