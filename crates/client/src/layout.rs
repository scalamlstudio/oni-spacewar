//! The carrier interior's layout rules, Revision 3 (design/READINESS.md
//! § Demo Spec › Carrier: Rooms, Items, Item catalogue, Cost, Construction
//! time, Starting layout, Save). Pure data, no Bevy and no clock: rooms and
//! the tiles, walls and doorways derived from them, the item catalogue,
//! construction sites advanced by an explicit [`Carrier::tick`], what may
//! be placed, moved or removed where and why, reachability, the walkable
//! area, crew posts, save validation and refunds. Also the click-to-move
//! pathing over walkable rectangles (`find_path`, `walk`, `can_stand`).
//!
//! Grid coordinates: tile (0,0) is the hull's north-west corner, x grows
//! east, y grows south. "Grid px" are world px in the same orientation
//! ([`TILE`] per tile); the Carrier scene flips y for Bevy.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::mission::{BULWARK_ID, KITE_ID};
use crate::workshop::Cost;

/// Hull size in tiles (fixed in the demo, one deck).
pub const HULL_W: i32 = 36;
pub const HULL_H: i32 = 22;
/// One tile in world px.
pub const TILE: f32 = 64.0;
/// A station's use point sits this far south of its front edge, px.
pub const USE_POINT_OFFSET: f32 = 24.0;
/// The battleships the demo owns; one berth each.
pub const SHIPS: [&str; 2] = [KITE_ID, BULWARK_ID];

/// A hull tile, (x, y).
pub type Tile = (i32, i32);

const fn cost(credits: u32, void_crystal: u32) -> Cost {
    Cost {
        credits,
        void_crystal,
    }
}

const FREE: Cost = cost(0, 0);

fn add(total: &mut Cost, c: Cost) {
    total.credits = total.credits.saturating_add(c.credits);
    total.void_crystal = total.void_crystal.saturating_add(c.void_crystal);
}

pub fn in_hull((x, y): Tile) -> bool {
    (0..HULL_W).contains(&x) && (0..HULL_H).contains(&y)
}

/// The tile under grid px (x, y).
pub fn tile_at(x: f32, y: f32) -> Tile {
    ((x / TILE).floor() as i32, (y / TILE).floor() as i32)
}

fn is_false(b: &bool) -> bool {
    !*b
}

// ---------------------------------------------------------------- rooms

/// The two buildable room shells (§ Rooms).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomSize {
    Small,
    Large,
    /// Anything a save names that this build doesn't know. Never valid;
    /// `validate` resets the layout when it sees one.
    #[serde(other)]
    Unknown,
}

impl RoomSize {
    pub const ALL: [RoomSize; 2] = [RoomSize::Small, RoomSize::Large];

    /// Catalogue ID.
    pub fn id(self) -> &'static str {
        match self {
            RoomSize::Small => "small_room",
            RoomSize::Large => "large_room",
            RoomSize::Unknown => "unknown",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RoomSize::Small => "Small Room",
            RoomSize::Large => "Large Room",
            RoomSize::Unknown => "?",
        }
    }

    /// Floor width × depth in tiles; the wall ring adds one tile all round.
    pub fn floor(self) -> (i32, i32) {
        match self {
            RoomSize::Small => (6, 4),
            RoomSize::Large => (13, 9),
            RoomSize::Unknown => (0, 0),
        }
    }

    pub fn cost(self) -> Cost {
        match self {
            RoomSize::Small => cost(60, 1),
            RoomSize::Large => cost(220, 4),
            RoomSize::Unknown => FREE,
        }
    }

    pub fn build_seconds(self) -> u32 {
        match self {
            RoomSize::Small => 5,
            RoomSize::Large => 8,
            RoomSize::Unknown => 0,
        }
    }
}

/// A room: its size and anchor (the north-west wall tile).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Room {
    pub size: RoomSize,
    pub x: i32,
    pub y: i32,
    /// Part of the starting carrier: can't be removed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub starter: bool,
    /// A construction site: ms left to build. `None` means finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_left_ms: Option<u32>,
}

impl Room {
    /// A finished, player-built room.
    pub fn new(size: RoomSize, x: i32, y: i32) -> Self {
        Self {
            size,
            x,
            y,
            starter: false,
            build_left_ms: None,
        }
    }

    fn starter(size: RoomSize, x: i32, y: i32) -> Self {
        Self {
            starter: true,
            ..Self::new(size, x, y)
        }
    }

    pub fn finished(&self) -> bool {
        self.build_left_ms.is_none()
    }

    /// The wall ring's corners, inclusive: (x0, y0, x1, y1).
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        let (w, d) = self.size.floor();
        (self.x, self.y, self.x + w + 1, self.y + d + 1)
    }

    pub fn contains(&self, (x, y): Tile) -> bool {
        let (x0, y0, x1, y1) = self.bounds();
        (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
    }

    pub fn is_floor(&self, (x, y): Tile) -> bool {
        let (x0, y0, x1, y1) = self.bounds();
        x > x0 && x < x1 && y > y0 && y < y1
    }

    pub fn is_ring(&self, t: Tile) -> bool {
        self.contains(t) && !self.is_floor(t)
    }

    /// Every tile, walls included, row by row.
    pub fn tiles(&self) -> impl Iterator<Item = Tile> {
        let (x0, y0, x1, y1) = self.bounds();
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (x, y)))
    }

    /// The anchor for a room whose middle floor tile (rounded north-west)
    /// is `t`: Build mode's mouse tile.
    pub fn anchor_for(size: RoomSize, (x, y): Tile) -> Tile {
        let (w, d) = size.floor();
        (x - (w - 1) / 2 - 1, y - (d - 1) / 2 - 1)
    }
}

/// A doorway between two rooms: two tiles across their shared wall.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Doorway {
    pub tiles: [Tile; 2],
    /// The two rooms (indices into [`Carrier::rooms`]).
    pub rooms: (usize, usize),
    /// True when the wall runs north–south (the doorway opens east–west).
    pub vertical: bool,
}

impl Doorway {
    /// The floor tile on each side of each doorway tile (§ Doorway
    /// clearance).
    pub fn clearance(&self) -> [Tile; 4] {
        let [(ax, ay), (bx, by)] = self.tiles;
        if self.vertical {
            [(ax - 1, ay), (ax + 1, ay), (bx - 1, by), (bx + 1, by)]
        } else {
            [(ax, ay - 1), (ax, ay + 1), (bx, by - 1), (bx, by + 1)]
        }
    }
}

/// The doorway two rooms get from a shared straight wall with at least 2
/// floor tiles facing each other across it, at the middle of the facing
/// stretch (rounded north or west).
fn doorway_between(a: &Room, b: &Room) -> Option<([Tile; 2], bool)> {
    let (ax0, ay0, ax1, ay1) = a.bounds();
    let (bx0, by0, bx1, by1) = b.bounds();
    let (at, lo, hi, vertical) = if ax0 == bx1 || ax1 == bx0 {
        let at = if ax0 == bx1 { ax0 } else { ax1 };
        (at, ay0.max(by0) + 1, ay1.min(by1) - 1, true)
    } else if ay0 == by1 || ay1 == by0 {
        let at = if ay0 == by1 { ay0 } else { ay1 };
        (at, ax0.max(bx0) + 1, ax1.min(bx1) - 1, false)
    } else {
        return None;
    };
    if hi - lo + 1 < 2 {
        return None;
    }
    let s = lo + (hi - lo + 1 - 2) / 2;
    Some(if vertical {
        ([(at, s), (at, s + 1)], true)
    } else {
        ([(s, at), (s + 1, at)], false)
    })
}

/// What a hull tile is, derived from the rooms.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileKind {
    /// Outside every room: not drawn (space shows), not walkable, free for
    /// new rooms.
    Hull,
    Wall,
    Floor,
    Doorway,
}

/// The tile map the rooms make (§ Grid and view › Tile kinds).
#[derive(Clone, Debug)]
pub struct TileMap {
    kinds: Vec<TileKind>,
    /// The room whose floor a tile is.
    floor_of: Vec<Option<usize>>,
    pub doorways: Vec<Doorway>,
    clearance: BTreeSet<Tile>,
    /// Doorway tiles of a finished pair of rooms.
    open: BTreeSet<Tile>,
    finished: Vec<bool>,
}

fn index((x, y): Tile) -> usize {
    (y * HULL_W + x) as usize
}

impl TileMap {
    pub fn new(rooms: &[Room]) -> Self {
        let n = (HULL_W * HULL_H) as usize;
        let mut map = Self {
            kinds: vec![TileKind::Hull; n],
            floor_of: vec![None; n],
            doorways: Vec::new(),
            clearance: BTreeSet::new(),
            open: BTreeSet::new(),
            finished: rooms.iter().map(Room::finished).collect(),
        };
        for (i, room) in rooms.iter().enumerate() {
            for t in room.tiles().filter(|t| in_hull(*t)) {
                let k = index(t);
                if room.is_floor(t) {
                    map.kinds[k] = TileKind::Floor;
                    map.floor_of[k] = Some(i);
                } else if map.kinds[k] == TileKind::Hull {
                    map.kinds[k] = TileKind::Wall;
                }
            }
        }
        for i in 0..rooms.len() {
            for j in i + 1..rooms.len() {
                if let Some((tiles, vertical)) = doorway_between(&rooms[i], &rooms[j]) {
                    let d = Doorway {
                        tiles,
                        rooms: (i, j),
                        vertical,
                    };
                    for t in tiles.into_iter().filter(|t| in_hull(*t)) {
                        map.kinds[index(t)] = TileKind::Doorway;
                        if map.finished[i] && map.finished[j] {
                            map.open.insert(t);
                        }
                    }
                    map.clearance.extend(d.clearance());
                    map.doorways.push(d);
                }
            }
        }
        map
    }

    pub fn kind(&self, t: Tile) -> TileKind {
        if in_hull(t) {
            self.kinds[index(t)]
        } else {
            TileKind::Hull
        }
    }

    /// The room whose floor `t` is.
    pub fn room_at(&self, t: Tile) -> Option<usize> {
        if in_hull(t) {
            self.floor_of[index(t)]
        } else {
            None
        }
    }

    /// Floor of a finished room.
    pub fn is_finished_floor(&self, t: Tile) -> bool {
        self.room_at(t).is_some_and(|r| self.finished[r])
    }

    /// A wall tile with floor directly south: it shows its face.
    pub fn is_wall_face(&self, (x, y): Tile) -> bool {
        self.kind((x, y)) == TileKind::Wall && self.kind((x, y + 1)) == TileKind::Floor
    }

    /// A floor tile beside a doorway, kept free of objects.
    pub fn is_clearance(&self, t: Tile) -> bool {
        self.clearance.contains(&t)
    }

    /// A doorway between two finished rooms (one under construction stays
    /// shut).
    pub fn is_open_doorway(&self, t: Tile) -> bool {
        self.open.contains(&t)
    }

    /// The 4-bit mask of wall neighbours (n-e-s-w: 1, 2, 4, 8) the wall art
    /// is picked by.
    pub fn wall_mask(&self, (x, y): Tile) -> u8 {
        [(0, -1), (1, 0), (0, 1), (-1, 0)]
            .iter()
            .enumerate()
            .filter(|(_, (dx, dy))| self.kind((x + dx, y + dy)) == TileKind::Wall)
            .fold(0, |m, (i, _)| m | 1 << i)
    }

    /// The content ID a tile is drawn with (§ Art spec › Tiles), `None` for
    /// hull (space shows through, TAKOAI-94). Walls use the n-e-s-w mask
    /// tile (`core.carrier.tile.wall.<nesw>`, bits in that order), so joined
    /// walls draw as one wall; floors cycle the three variants by `x + y`;
    /// a doorway tile shows the threshold across its passage.
    pub fn art(&self, (x, y): Tile) -> Option<String> {
        Some(match self.kind((x, y)) {
            TileKind::Hull => return None,
            TileKind::Floor => return self.floor_art((x, y)),
            TileKind::Wall => {
                let mask = self.wall_mask((x, y));
                let bits: String = (0..4)
                    .map(|i| if mask & 1 << i != 0 { '1' } else { '0' })
                    .collect();
                format!("core.carrier.tile.wall.{bits}")
            }
            TileKind::Doorway => {
                let vertical = self
                    .doorways
                    .iter()
                    .find(|d| d.tiles.contains(&(x, y)))
                    .is_some_and(|d| d.vertical);
                let axis = if vertical { "ew" } else { "ns" };
                format!("core.carrier.tile.doorway.{axis}")
            }
        })
    }

    /// The floor drawn on (or under) a room tile: the floor variant for
    /// `x + y`. The scene puts it under every wall and doorway tile too, so
    /// wall art that leaves floor showing (TAKOAI-89) drops in unchanged.
    /// `None` for hull.
    pub fn floor_art(&self, (x, y): Tile) -> Option<String> {
        (self.kind((x, y)) != TileKind::Hull)
            .then(|| format!("core.carrier.tile.floor.{}", (x + y).rem_euclid(3)))
    }
}

// ---------------------------------------------------------------- items

/// Each tile holds at most one item per layer (§ Items).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Layer {
    /// Stations and most furniture. Blocks walking.
    Object,
    /// Rugs. Walkable; an object may stand on top.
    Floor,
    /// Wall-mounted decor on a wall face. Never blocks.
    Wall,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Placement {
    /// Every footprint tile is floor of a finished room.
    Free,
    /// Free, and every tile directly north of the footprint is wall.
    AgainstWall,
    /// Every footprint tile is a wall face.
    OnWall,
}

impl Placement {
    pub fn label(self) -> &'static str {
        match self {
            Placement::Free => "Free",
            Placement::AgainstWall => "Against a wall",
            Placement::OnWall => "On a wall",
        }
    }
}

/// How many of an item the carrier may hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Count {
    /// Exactly one, from the starting carrier: move only.
    Exactly1,
    /// One per battleship, from the starting carrier: move only.
    PerShip,
    /// At most one, bought.
    BuyOnce,
    Any,
}

impl Count {
    /// Fixed-count stations can be moved but not bought or removed.
    pub fn fixed(self) -> bool {
        matches!(self, Count::Exactly1 | Count::PerShip)
    }
}

/// Build mode category.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Station,
    Furniture,
    /// Rugs and wall-mounted pieces.
    Decor,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemId {
    CommandConsole,
    UpgradeBench,
    Berth,
    SampleAnalyzer,
    SalvageSorter,
    CombatSimulator,
    BunkBed,
    Locker,
    Table,
    Stool,
    Sofa,
    CatBed,
    CatTree,
    ScratchPost,
    FoodBowls,
    CrateStack,
    SnackDispenser,
    Aquarium,
    PottedPlant,
    HydroPlanter,
    FloorLamp,
    Rug,
    WallScreen,
    WallLight,
    ToolRack,
    /// Anything a save names that this build doesn't know. Never valid.
    #[serde(other)]
    Unknown,
}

/// One catalogue entry (§ Item catalogue). Every item faces S.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemDef {
    pub id: ItemId,
    /// Stable ID (save file, content IDs).
    pub key: &'static str,
    pub name: &'static str,
    /// Footprint width × depth in tiles.
    pub size: (i32, i32),
    pub layer: Layer,
    pub placement: Placement,
    pub category: Category,
    pub count: Count,
    /// Build cost; starter-only stations cost nothing (they can't be bought).
    pub cost: Cost,
    pub build_seconds: u32,
    pub effect: &'static str,
}

impl ItemDef {
    /// Stations have a front strip, must stay reachable and can be used.
    pub fn station(&self) -> bool {
        self.category == Category::Station
    }
}

const STATION_SECS: u32 = 4;
const FURNITURE_SECS: u32 = 2;

#[allow(clippy::too_many_arguments)]
const fn station(
    id: ItemId,
    key: &'static str,
    name: &'static str,
    size: (i32, i32),
    placement: Placement,
    count: Count,
    cost: Cost,
    effect: &'static str,
) -> ItemDef {
    ItemDef {
        id,
        key,
        name,
        size,
        layer: Layer::Object,
        placement,
        category: Category::Station,
        count,
        cost,
        build_seconds: STATION_SECS,
        effect,
    }
}

const fn furniture(
    id: ItemId,
    key: &'static str,
    name: &'static str,
    size: (i32, i32),
    placement: Placement,
    layer: Layer,
    credits: u32,
) -> ItemDef {
    ItemDef {
        id,
        key,
        name,
        size,
        layer,
        placement,
        category: match layer {
            Layer::Object => Category::Furniture,
            _ => Category::Decor,
        },
        count: Count::Any,
        cost: cost(credits, 0),
        build_seconds: FURNITURE_SECS,
        effect: "",
    }
}

use Placement::{AgainstWall, Free, OnWall};

/// The catalogue, in Build mode order.
pub const CATALOGUE: [ItemDef; 25] = [
    station(
        ItemId::CommandConsole,
        "command_console",
        "Command Console",
        (2, 1),
        AgainstWall,
        Count::Exactly1,
        FREE,
        "The Gunner's post",
    ),
    station(
        ItemId::UpgradeBench,
        "upgrade_bench",
        "Upgrade Bench",
        (2, 1),
        AgainstWall,
        Count::Exactly1,
        FREE,
        "Workshop upgrades. The Engineer's post",
    ),
    station(
        ItemId::Berth,
        "berth",
        "Berth",
        (4, 4),
        Free,
        Count::PerShip,
        FREE,
        "Holds one battleship. Board it to launch a mission",
    ),
    station(
        ItemId::SampleAnalyzer,
        "sample_analyzer",
        "Sample Analyzer",
        (2, 1),
        AgainstWall,
        Count::Exactly1,
        FREE,
        "The Researcher's post",
    ),
    station(
        ItemId::SalvageSorter,
        "salvage_sorter",
        "Salvage Sorter",
        (2, 2),
        Free,
        Count::BuyOnce,
        cost(120, 3),
        "+25% Credits and Void Crystal from every successful mission",
    ),
    station(
        ItemId::CombatSimulator,
        "combat_simulator",
        "Combat Simulator",
        (2, 2),
        Free,
        Count::BuyOnce,
        cost(200, 6),
        "-15% Q and W cooldowns on every battleship",
    ),
    furniture(
        ItemId::BunkBed,
        "bunk_bed",
        "Bunk bed",
        (1, 2),
        AgainstWall,
        Layer::Object,
        40,
    ),
    furniture(
        ItemId::Locker,
        "locker",
        "Locker",
        (1, 1),
        AgainstWall,
        Layer::Object,
        20,
    ),
    furniture(
        ItemId::Table,
        "table",
        "Table",
        (2, 1),
        Free,
        Layer::Object,
        25,
    ),
    furniture(
        ItemId::Stool,
        "stool",
        "Stool",
        (1, 1),
        Free,
        Layer::Object,
        10,
    ),
    furniture(
        ItemId::Sofa,
        "sofa",
        "Sofa",
        (2, 1),
        AgainstWall,
        Layer::Object,
        40,
    ),
    furniture(
        ItemId::CatBed,
        "cat_bed",
        "Cat bed",
        (1, 1),
        Free,
        Layer::Object,
        15,
    ),
    furniture(
        ItemId::CatTree,
        "cat_tree",
        "Cat tree",
        (1, 1),
        Free,
        Layer::Object,
        30,
    ),
    furniture(
        ItemId::ScratchPost,
        "scratch_post",
        "Scratching post",
        (1, 1),
        Free,
        Layer::Object,
        10,
    ),
    furniture(
        ItemId::FoodBowls,
        "food_bowls",
        "Food and water bowls",
        (1, 1),
        Free,
        Layer::Object,
        10,
    ),
    furniture(
        ItemId::CrateStack,
        "crate_stack",
        "Crate stack",
        (1, 1),
        Free,
        Layer::Object,
        10,
    ),
    furniture(
        ItemId::SnackDispenser,
        "snack_dispenser",
        "Snack dispenser",
        (1, 1),
        AgainstWall,
        Layer::Object,
        30,
    ),
    furniture(
        ItemId::Aquarium,
        "aquarium",
        "Aquarium",
        (2, 1),
        AgainstWall,
        Layer::Object,
        60,
    ),
    furniture(
        ItemId::PottedPlant,
        "potted_plant",
        "Potted plant",
        (1, 1),
        Free,
        Layer::Object,
        15,
    ),
    furniture(
        ItemId::HydroPlanter,
        "hydro_planter",
        "Hydroponic planter",
        (2, 1),
        Free,
        Layer::Object,
        35,
    ),
    furniture(
        ItemId::FloorLamp,
        "floor_lamp",
        "Floor lamp",
        (1, 1),
        Free,
        Layer::Object,
        15,
    ),
    furniture(ItemId::Rug, "rug", "Rug", (2, 2), Free, Layer::Floor, 20),
    furniture(
        ItemId::WallScreen,
        "wall_screen",
        "Wall screen (star map)",
        (2, 1),
        OnWall,
        Layer::Wall,
        25,
    ),
    furniture(
        ItemId::WallLight,
        "wall_light",
        "Wall light",
        (1, 1),
        OnWall,
        Layer::Wall,
        10,
    ),
    furniture(
        ItemId::ToolRack,
        "tool_rack",
        "Tool rack",
        (1, 1),
        OnWall,
        Layer::Wall,
        15,
    ),
];

impl ItemId {
    pub fn def(self) -> Option<&'static ItemDef> {
        CATALOGUE.iter().find(|d| d.id == self)
    }

    pub fn key(self) -> &'static str {
        self.def().map_or("unknown", |d| d.key)
    }

    pub fn name(self) -> &'static str {
        self.def().map_or("?", |d| d.name)
    }

    pub fn content_id(self) -> String {
        format!("core.carrier.item.{}", self.key())
    }
}

/// A placed item: a catalogue ID and its anchor (the footprint's
/// north-west tile).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub x: i32,
    pub y: i32,
    /// A berth's battleship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ship: Option<String>,
    /// Part of the starting carrier: refunds nothing. Kept when it moves.
    #[serde(default, skip_serializing_if = "is_false")]
    pub starter: bool,
    /// A construction site: ms left to build. `None` means finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_left_ms: Option<u32>,
}

impl Item {
    /// A finished, player-built item.
    pub fn new(id: ItemId, x: i32, y: i32) -> Self {
        Self {
            id,
            x,
            y,
            ship: None,
            starter: false,
            build_left_ms: None,
        }
    }

    fn starter(id: ItemId, x: i32, y: i32) -> Self {
        Self {
            starter: true,
            ..Self::new(id, x, y)
        }
    }

    pub fn finished(&self) -> bool {
        self.build_left_ms.is_none()
    }

    fn size(&self) -> (i32, i32) {
        self.id.def().map_or((0, 0), |d| d.size)
    }

    pub fn layer(&self) -> Layer {
        self.id.def().map_or(Layer::Object, |d| d.layer)
    }

    pub fn is_station(&self) -> bool {
        self.id.def().is_some_and(ItemDef::station)
    }

    /// Footprint tiles, row by row.
    pub fn tiles(&self) -> impl Iterator<Item = Tile> {
        footprint(self.size(), self.x, self.y)
    }

    /// A station's front strip: the row directly south of its footprint.
    /// Empty for anything else.
    pub fn front_strip(&self) -> Vec<Tile> {
        if !self.is_station() {
            return Vec::new();
        }
        let (w, d) = self.size();
        (self.x..self.x + w).map(|x| (x, self.y + d)).collect()
    }

    /// Where a station is used from, in grid px: 24 px south of its front
    /// edge, at the middle of its front strip.
    pub fn use_point(&self) -> (f32, f32) {
        let (w, d) = self.size();
        (
            (self.x as f32 + w as f32 / 2.0) * TILE,
            (self.y + d) as f32 * TILE + USE_POINT_OFFSET,
        )
    }

    /// What it refunds when removed: nothing for starter pieces or unknown
    /// IDs, else its full cost (sites included).
    pub fn refund(&self) -> Cost {
        match self.id.def() {
            Some(d) if !self.starter => d.cost,
            _ => FREE,
        }
    }

    fn sort_key(&self) -> (i32, i32, Layer, &'static str) {
        (self.y, self.x, self.layer(), self.id.key())
    }
}

fn footprint((w, d): (i32, i32), x: i32, y: i32) -> impl Iterator<Item = Tile> {
    (y..y + d).flat_map(move |ty| (x..x + w).map(move |tx| (tx, ty)))
}

impl Room {
    /// What it refunds when removed (a site included): its full cost,
    /// nothing for a starter room.
    pub fn refund(&self) -> Cost {
        if self.starter {
            FREE
        } else {
            self.size.cost()
        }
    }
}

/// The crew members with a post (§ Walking and interaction › Crew posts).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Crew {
    Gunner,
    Engineer,
    Researcher,
}

impl Crew {
    pub fn post(self) -> ItemId {
        match self {
            Crew::Gunner => ItemId::CommandConsole,
            Crew::Engineer => ItemId::UpgradeBench,
            Crew::Researcher => ItemId::SampleAnalyzer,
        }
    }
}

// ---------------------------------------------------------------- reasons

/// Why an edit is refused: the ghost's line and the error tooltip
/// (§ Building, Reasons).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reason {
    OutsideHull,
    OverlapsRoom,
    MustShareWall,
    /// The new doorways would land on these items (indices into
    /// [`Carrier::items`]); they flash.
    ClearDoorway(Vec<usize>),
    RoomNotEmpty,
    WouldCutOffRoom,
    NeedsFloor,
    StillBeingBuilt,
    Blocked,
    KeepDoorwayClear,
    NeedsWallBehind,
    NeedsWallFace,
    KeepFrontClear,
    /// A station's front strip would be unreachable from the Command
    /// Console.
    WouldCutOff(ItemId),
    PilotInTheWay,
    Need(Cost),
    AlreadyBuilt,
    MoveItInstead,
    StarterPiece,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::OutsideHull => write!(f, "Outside the hull"),
            Reason::OverlapsRoom => write!(f, "Overlaps a room"),
            Reason::MustShareWall => write!(f, "Must share a wall with a room"),
            Reason::ClearDoorway(_) => write!(f, "Clear the doorway first"),
            Reason::RoomNotEmpty => write!(f, "Room isn't empty"),
            Reason::WouldCutOffRoom => write!(f, "Would cut off a room"),
            Reason::NeedsFloor => write!(f, "Needs floor"),
            Reason::StillBeingBuilt => write!(f, "Still being built"),
            Reason::Blocked => write!(f, "Blocked"),
            Reason::KeepDoorwayClear => write!(f, "Keep the doorway clear"),
            Reason::NeedsWallBehind => write!(f, "Needs a wall behind it"),
            Reason::NeedsWallFace => write!(f, "Needs a wall face"),
            Reason::KeepFrontClear => write!(f, "Keep the front clear"),
            Reason::WouldCutOff(id) => write!(f, "Would cut off the {}", id.name()),
            Reason::PilotInTheWay => write!(f, "The Pilot is in the way"),
            Reason::Need(c) if c.void_crystal > 0 => {
                write!(f, "Need {} cr + {} VC", c.credits, c.void_crystal)
            }
            Reason::Need(c) => write!(f, "Need {} cr", c.credits),
            Reason::AlreadyBuilt => write!(f, "Already built"),
            Reason::MoveItInstead => write!(f, "Move it instead"),
            Reason::StarterPiece => write!(f, "Part of the starting carrier"),
        }
    }
}

/// Whether `wallet` covers `price`; the reason names the full price.
pub fn afford(price: Cost, wallet: Cost) -> Result<(), Reason> {
    if wallet.credits >= price.credits && wallet.void_crystal >= price.void_crystal {
        Ok(())
    } else {
        Err(Reason::Need(price))
    }
}

/// Why a saved layout was rejected on load (§ Save › Validation on load).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LayoutError {
    UnknownId,
    /// A room outside the hull or overlapping another off its walls.
    BadRoom(usize),
    /// A room not connected to the Command Console's room.
    Disconnected(usize),
    /// An item breaking its placement rule, a layer clash, doorway
    /// clearance or a front strip.
    BadItem(usize, Reason),
    /// A station's front strip can't be reached.
    Unreachable(usize),
    /// Wrong number of a counted station, or berths not one per ship.
    Count(ItemId),
    MissingStarterRoom,
}

// ---------------------------------------------------------------- carrier

/// The carrier: rooms and items. Stored in the save as-is (§ Save
/// (version 5)); tiles and doorways are rebuilt from the rooms. Rooms are
/// sorted by (y, x), items by (y, x, layer, id), so the same layout always
/// writes the same bytes. Item and room indices change when an edit
/// re-sorts them; the edit functions return the new index.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Carrier {
    pub rooms: Vec<Room>,
    pub items: Vec<Item>,
}

impl Default for Carrier {
    fn default() -> Self {
        Self::starting()
    }
}

/// Bridge, Quarters, Lab, Hangar, Workshop (§ Starting layout).
const STARTING_ROOMS: [(RoomSize, i32, i32); 5] = [
    (RoomSize::Small, 0, 0),
    (RoomSize::Small, 7, 0),
    (RoomSize::Small, 14, 0),
    (RoomSize::Large, 0, 5),
    (RoomSize::Small, 14, 5),
];

const STARTING_ITEMS: [(ItemId, i32, i32); 21] = [
    // Bridge
    (ItemId::CommandConsole, 3, 1),
    (ItemId::WallScreen, 5, 0),
    (ItemId::PottedPlant, 1, 1),
    (ItemId::FloorLamp, 6, 1),
    // Quarters
    (ItemId::BunkBed, 9, 1),
    (ItemId::BunkBed, 10, 1),
    (ItemId::Locker, 12, 1),
    (ItemId::CatBed, 13, 1),
    (ItemId::Rug, 10, 3),
    (ItemId::Table, 10, 3),
    // Lab
    (ItemId::SampleAnalyzer, 17, 1),
    (ItemId::PottedPlant, 15, 1),
    (ItemId::HydroPlanter, 19, 1),
    // Hangar (the berths get their ships below)
    (ItemId::Berth, 2, 8),
    (ItemId::Berth, 8, 8),
    (ItemId::WallLight, 6, 5),
    (ItemId::WallLight, 13, 5),
    // Workshop
    (ItemId::UpgradeBench, 15, 6),
    (ItemId::ToolRack, 20, 5),
    (ItemId::CrateStack, 19, 9),
    (ItemId::CrateStack, 20, 9),
];

/// Tile → item index, per layer.
type Occupancy = BTreeMap<(Layer, Tile), usize>;

impl Carrier {
    /// The starting carrier: five rooms, six doorways, everything starter.
    pub fn starting() -> Self {
        let rooms = STARTING_ROOMS
            .iter()
            .map(|&(s, x, y)| Room::starter(s, x, y))
            .collect();
        let mut ships = SHIPS.iter();
        let items = STARTING_ITEMS
            .iter()
            .map(|&(id, x, y)| Item {
                ship: (id == ItemId::Berth)
                    .then(|| ships.next().map(|s| s.to_string()))
                    .flatten(),
                ..Item::starter(id, x, y)
            })
            .collect();
        let mut c = Self { rooms, items };
        c.normalize();
        c
    }

    /// Canonical order: rooms by (y, x), items by (y, x, layer, id).
    pub fn normalize(&mut self) {
        self.rooms.sort_by(|a, b| {
            (a.y, a.x, a.size)
                .cmp(&(b.y, b.x, b.size))
                .then_with(|| (a.starter, a.build_left_ms).cmp(&(b.starter, b.build_left_ms)))
        });
        self.items.sort_by(|a, b| {
            a.sort_key().cmp(&b.sort_key()).then_with(|| {
                (&a.ship, a.starter, a.build_left_ms).cmp(&(&b.ship, b.starter, b.build_left_ms))
            })
        });
    }

    pub fn tile_map(&self) -> TileMap {
        TileMap::new(&self.rooms)
    }

    fn occupancy(&self, skip: Option<usize>) -> Occupancy {
        let mut occ = Occupancy::new();
        for (i, item) in self.items.iter().enumerate() {
            if Some(i) == skip {
                continue;
            }
            for t in item.tiles() {
                occ.entry((item.layer(), t)).or_insert(i);
            }
        }
        occ
    }

    /// The item on `layer` covering `t`.
    pub fn item_at(&self, t: Tile, layer: Layer) -> Option<usize> {
        self.items
            .iter()
            .position(|i| i.layer() == layer && i.tiles().any(|q| q == t))
    }

    /// What a click on `t` picks: a wall-mounted item, else an object,
    /// else a rug.
    pub fn pick_item(&self, t: Tile) -> Option<usize> {
        [Layer::Wall, Layer::Object, Layer::Floor]
            .into_iter()
            .find_map(|l| self.item_at(t, l))
    }

    /// The room whose floor `t` is.
    pub fn room_at(&self, t: Tile) -> Option<usize> {
        self.rooms.iter().position(|r| r.is_floor(t))
    }

    /// The first `id` on board, finished or not.
    pub fn find(&self, id: ItemId) -> Option<usize> {
        self.items.iter().position(|i| i.id == id)
    }

    /// Whether a finished `id` is on board (an item's effect is off while
    /// it is being built).
    pub fn has_finished(&self, id: ItemId) -> bool {
        self.items.iter().any(|i| i.id == id && i.finished())
    }

    /// The berth holding `ship`.
    pub fn berth_of(&self, ship: &str) -> Option<usize> {
        self.items
            .iter()
            .position(|i| i.id == ItemId::Berth && i.ship.as_deref() == Some(ship))
    }

    /// Walkable tiles (§ Walking and interaction › Walkable): floor of a
    /// finished room or an open doorway, with no object on it.
    fn walkable_tiles(&self, map: &TileMap, occ: &Occupancy) -> BTreeSet<Tile> {
        (0..HULL_H)
            .flat_map(|y| (0..HULL_W).map(move |x| (x, y)))
            .filter(|&t| {
                (map.is_finished_floor(t) || map.is_open_doorway(t))
                    && !occ.contains_key(&(Layer::Object, t))
            })
            .collect()
    }

    /// Tiles reachable by walking (4-neighbour) from the Command Console's
    /// front strip, the root.
    pub fn reachable(&self) -> BTreeSet<Tile> {
        let map = self.tile_map();
        let walk = self.walkable_tiles(&map, &self.occupancy(None));
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        if let Some(c) = self.find(ItemId::CommandConsole) {
            for t in self.items[c].front_strip() {
                if walk.contains(&t) && seen.insert(t) {
                    queue.push_back(t);
                }
            }
        }
        while let Some((x, y)) = queue.pop_front() {
            for n in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if walk.contains(&n) && seen.insert(n) {
                    queue.push_back(n);
                }
            }
        }
        seen
    }

    /// The first station whose front strip can't be reached from the root.
    fn cut_off(&self) -> Option<usize> {
        let seen = self.reachable();
        self.items
            .iter()
            .position(|i| i.is_station() && !i.front_strip().iter().any(|t| seen.contains(t)))
    }

    /// The first room not connected (through doorways) to the room holding
    /// the Command Console; every room when there is no console.
    fn disconnected_room(&self) -> Option<usize> {
        let root = self
            .find(ItemId::CommandConsole)
            .and_then(|c| self.room_at((self.items[c].x, self.items[c].y)));
        let Some(root) = root else {
            return (!self.rooms.is_empty()).then_some(0);
        };
        let mut seen = vec![false; self.rooms.len()];
        seen[root] = true;
        let mut queue = VecDeque::from([root]);
        while let Some(i) = queue.pop_front() {
            for (j, other) in self.rooms.iter().enumerate() {
                if !seen[j] && doorway_between(&self.rooms[i], other).is_some() {
                    seen[j] = true;
                    queue.push_back(j);
                }
            }
        }
        seen.iter().position(|s| !s)
    }

    /// Where a crew member stands: the first walkable tile east of their
    /// post's front strip, else west of it, else south of its first tile.
    pub fn crew_post(&self, crew: Crew) -> Option<Tile> {
        let post = &self.items[self.find(crew.post())?];
        let strip = post.front_strip();
        let (&(lx, y), &(rx, _)) = (strip.first()?, strip.last()?);
        let map = self.tile_map();
        let walk = self.walkable_tiles(&map, &self.occupancy(None));
        [(rx + 1, y), (lx - 1, y), (lx, y + 1)]
            .into_iter()
            .find(|t| walk.contains(t))
    }

    /// Where the Pilot can stand, in grid px: the walkable tiles merged
    /// into rectangles (runs along each row, stacked when rows match).
    pub fn walkable(&self) -> Vec<Rect> {
        let map = self.tile_map();
        let walk = self.walkable_tiles(&map, &self.occupancy(None));
        let mut done: Vec<(i32, i32, i32, i32)> = Vec::new();
        // Open rectangles by (x0, x1): (y0, y1), tiles inclusive.
        let mut open: BTreeMap<(i32, i32), (i32, i32)> = BTreeMap::new();
        for y in 0..HULL_H {
            let mut runs = Vec::new();
            let mut x = 0;
            while x < HULL_W {
                if walk.contains(&(x, y)) {
                    let x0 = x;
                    while x < HULL_W && walk.contains(&(x, y)) {
                        x += 1;
                    }
                    runs.push((x0, x - 1));
                } else {
                    x += 1;
                }
            }
            let mut next = BTreeMap::new();
            for run in runs {
                let span = match open.remove(&run) {
                    Some((y0, _)) => (y0, y),
                    None => (y, y),
                };
                next.insert(run, span);
            }
            done.extend(
                open.into_iter()
                    .map(|((x0, x1), (y0, y1))| (x0, y0, x1, y1)),
            );
            open = next;
        }
        done.extend(
            open.into_iter()
                .map(|((x0, x1), (y0, y1))| (x0, y0, x1, y1)),
        );
        done.sort_by_key(|&(x0, y0, _, _)| (y0, x0));
        done.into_iter()
            .map(|(x0, y0, x1, y1)| Rect {
                x0: x0 as f32 * TILE,
                y0: y0 as f32 * TILE,
                x1: (x1 + 1) as f32 * TILE,
                y1: (y1 + 1) as f32 * TILE,
            })
            .collect()
    }

    // ------------------------------------------------------------ rooms

    /// Whether a room of `size` can be built with its anchor (north-west
    /// wall tile) at (x, y), and paid for from `wallet`.
    pub fn can_place_room(
        &self,
        size: RoomSize,
        x: i32,
        y: i32,
        wallet: Cost,
    ) -> Result<(), Reason> {
        if size == RoomSize::Unknown {
            return Err(Reason::Blocked);
        }
        let room = Room {
            build_left_ms: Some(size.build_seconds() * 1000),
            ..Room::new(size, x, y)
        };
        if !room.tiles().all(in_hull) {
            return Err(Reason::OutsideHull);
        }
        let map = self.tile_map();
        for t in room.tiles() {
            match map.kind(t) {
                TileKind::Hull => {}
                TileKind::Wall if room.is_ring(t) => {}
                _ => return Err(Reason::OverlapsRoom),
            }
        }
        let doors: Vec<Doorway> = self
            .rooms
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                doorway_between(&room, r).map(|(tiles, vertical)| Doorway {
                    tiles,
                    rooms: (i, self.rooms.len()),
                    vertical,
                })
            })
            .collect();
        if doors.is_empty() {
            return Err(Reason::MustShareWall);
        }
        let mut blockers = BTreeSet::new();
        for d in &doors {
            for t in d.tiles {
                blockers.extend(
                    (0..self.items.len()).filter(|&i| self.items[i].tiles().any(|q| q == t)),
                );
            }
            for t in d.clearance() {
                if let Some(i) = self.item_at(t, Layer::Object) {
                    blockers.insert(i);
                }
            }
        }
        if !blockers.is_empty() {
            return Err(Reason::ClearDoorway(blockers.into_iter().collect()));
        }
        // Nothing else should change, but every item must still hold.
        let mut after = self.clone();
        after.rooms.push(room);
        let amap = after.tile_map();
        for (i, item) in self.items.iter().enumerate() {
            if after.check_spot(&amap, item, Some(i), None).is_err() {
                return Err(Reason::ClearDoorway(vec![i]));
            }
        }
        if let Some(i) = after.cut_off() {
            return Err(Reason::WouldCutOff(after.items[i].id));
        }
        afford(size.cost(), wallet)
    }

    /// Starts building a room (no checks; see [`Self::can_place_room`]).
    /// Returns its index.
    pub fn place_room(&mut self, size: RoomSize, x: i32, y: i32) -> usize {
        let ms = size.build_seconds() * 1000;
        self.rooms.push(Room {
            build_left_ms: (ms > 0).then_some(ms),
            ..Room::new(size, x, y)
        });
        self.normalize();
        self.rooms
            .iter()
            .position(|r| (r.size, r.x, r.y) == (size, x, y))
            .unwrap_or(0)
    }

    /// Whether room `i` can be removed; `Ok` holds its refund.
    pub fn can_remove_room(&self, i: usize) -> Result<Cost, Reason> {
        let room = self.rooms.get(i).ok_or(Reason::Blocked)?;
        if room.starter {
            return Err(Reason::StarterPiece);
        }
        let holds = |item: &Item| {
            item.tiles().any(|(x, y)| {
                room.is_floor((x, y))
                    || (item.layer() == Layer::Wall
                        && room.is_ring((x, y))
                        && room.is_floor((x, y + 1)))
            })
        };
        if self.items.iter().any(holds) {
            return Err(Reason::RoomNotEmpty);
        }
        let mut after = self.clone();
        after.rooms.remove(i);
        if after.disconnected_room().is_some() {
            return Err(Reason::WouldCutOffRoom);
        }
        if let Some(j) = after.cut_off() {
            return Err(Reason::WouldCutOff(after.items[j].id));
        }
        Ok(room.refund())
    }

    /// Removes room `i` (no checks) and returns its refund.
    pub fn remove_room(&mut self, i: usize) -> Cost {
        self.rooms.remove(i).refund()
    }

    // ------------------------------------------------------------ items

    /// The placement rules for `item` against everything but item `skip`
    /// (§ Items › Placement rules, Front strip, Doorway clearance).
    fn check_spot(
        &self,
        map: &TileMap,
        item: &Item,
        skip: Option<usize>,
        pilot: Option<Tile>,
    ) -> Result<(), Reason> {
        let def = item.id.def().ok_or(Reason::Blocked)?;
        let occ = self.occupancy(skip);
        for (x, y) in item.tiles() {
            if def.placement == Placement::OnWall {
                if !map.is_wall_face((x, y)) {
                    return Err(Reason::NeedsWallFace);
                }
                if !map.is_finished_floor((x, y + 1)) {
                    return Err(Reason::StillBeingBuilt);
                }
            } else {
                match map.kind((x, y)) {
                    TileKind::Floor => {}
                    TileKind::Doorway if def.layer == Layer::Object => {
                        return Err(Reason::KeepDoorwayClear)
                    }
                    _ => return Err(Reason::NeedsFloor),
                }
                if !map.is_finished_floor((x, y)) {
                    return Err(Reason::StillBeingBuilt);
                }
                if def.layer == Layer::Object && map.is_clearance((x, y)) {
                    return Err(Reason::KeepDoorwayClear);
                }
            }
            if occ.contains_key(&(def.layer, (x, y))) {
                return Err(Reason::Blocked);
            }
        }
        if def.placement == Placement::AgainstWall
            && (item.x..item.x + def.size.0).any(|x| map.kind((x, item.y - 1)) != TileKind::Wall)
        {
            return Err(Reason::NeedsWallBehind);
        }
        if def.layer == Layer::Object {
            let strips: BTreeSet<Tile> = self
                .items
                .iter()
                .enumerate()
                .filter(|(j, _)| Some(*j) != skip)
                .flat_map(|(_, i)| i.front_strip())
                .collect();
            if item.tiles().any(|t| strips.contains(&t)) {
                return Err(Reason::KeepFrontClear);
            }
        }
        for t in item.front_strip() {
            let floor = matches!(map.kind(t), TileKind::Floor | TileKind::Doorway);
            if !floor || occ.contains_key(&(Layer::Object, t)) {
                return Err(Reason::KeepFrontClear);
            }
        }
        if def.layer == Layer::Object && pilot.is_some_and(|p| item.tiles().any(|t| t == p)) {
            return Err(Reason::PilotInTheWay);
        }
        Ok(())
    }

    /// Whether a new `id` can be built anchored at (x, y) and paid for
    /// from `wallet`; `pilot` is the tile under the Pilot's feet.
    pub fn can_place(
        &self,
        id: ItemId,
        x: i32,
        y: i32,
        wallet: Cost,
        pilot: Option<Tile>,
    ) -> Result<(), Reason> {
        let def = id.def().ok_or(Reason::Blocked)?;
        if def.count.fixed() {
            return Err(Reason::StarterPiece);
        }
        if def.count == Count::BuyOnce && self.find(id).is_some() {
            return Err(Reason::AlreadyBuilt);
        }
        let item = Item {
            build_left_ms: Some(def.build_seconds * 1000),
            ..Item::new(id, x, y)
        };
        self.check_spot(&self.tile_map(), &item, None, pilot)?;
        let mut after = self.clone();
        after.items.push(item);
        if let Some(i) = after.cut_off() {
            return Err(Reason::WouldCutOff(after.items[i].id));
        }
        afford(def.cost, wallet)
    }

    /// Starts building `id` at (x, y) (no checks; see [`Self::can_place`]).
    /// Returns its index.
    pub fn place(&mut self, id: ItemId, x: i32, y: i32, ship: Option<String>) -> usize {
        let ms = id.def().map_or(0, |d| d.build_seconds * 1000);
        let layer = Item::new(id, x, y).layer();
        self.items.push(Item {
            ship,
            build_left_ms: (ms > 0).then_some(ms),
            ..Item::new(id, x, y)
        });
        self.normalize();
        self.items
            .iter()
            .position(|i| (i.x, i.y, i.layer(), i.id) == (x, y, layer, id))
            .unwrap_or(0)
    }

    /// Whether finished item `i` can be moved to (x, y). Its own tiles
    /// count as free while it is held.
    pub fn can_move(&self, i: usize, x: i32, y: i32, pilot: Option<Tile>) -> Result<(), Reason> {
        let item = self.items.get(i).ok_or(Reason::Blocked)?;
        if !item.finished() {
            return Err(Reason::StillBeingBuilt);
        }
        let moved = Item {
            x,
            y,
            ..item.clone()
        };
        self.check_spot(&self.tile_map(), &moved, Some(i), pilot)?;
        let mut after = self.clone();
        after.items[i] = moved;
        if let Some(j) = after.cut_off() {
            return Err(Reason::WouldCutOff(after.items[j].id));
        }
        Ok(())
    }

    /// Moves item `i` (free and instant, no checks; `starter` and `ship`
    /// stay). Returns its new index.
    pub fn move_item(&mut self, i: usize, x: i32, y: i32) -> usize {
        let (id, layer) = (self.items[i].id, self.items[i].layer());
        self.items[i].x = x;
        self.items[i].y = y;
        self.normalize();
        self.items
            .iter()
            .position(|it| (it.x, it.y, it.layer(), it.id) == (x, y, layer, id))
            .unwrap_or(0)
    }

    /// Whether item `i` (or site) can be removed; `Ok` holds its refund.
    pub fn can_remove(&self, i: usize) -> Result<Cost, Reason> {
        let item = self.items.get(i).ok_or(Reason::Blocked)?;
        if item.id.def().is_some_and(|d| d.count.fixed()) {
            return Err(Reason::MoveItInstead);
        }
        Ok(item.refund())
    }

    /// Removes item `i` (no checks) and returns its refund.
    pub fn remove(&mut self, i: usize) -> Cost {
        self.items.remove(i).refund()
    }

    /// Advances every construction site by `ms`. Returns whether any
    /// finished (the caller saves then).
    pub fn tick(&mut self, ms: u32) -> bool {
        let mut finished = false;
        let sites = self
            .rooms
            .iter_mut()
            .map(|r| &mut r.build_left_ms)
            .chain(self.items.iter_mut().map(|i| &mut i.build_left_ms));
        for left in sites {
            if let Some(v) = left {
                let rest = v.saturating_sub(ms);
                *left = (rest > 0).then_some(rest);
                finished |= rest == 0;
            }
        }
        finished
    }

    // ------------------------------------------------------------ save

    /// Checks a loaded layout (§ Save › Validation on load).
    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.rooms.iter().any(|r| r.size == RoomSize::Unknown)
            || self.items.iter().any(|i| i.id == ItemId::Unknown)
        {
            return Err(LayoutError::UnknownId);
        }
        // Inside the hull; a tile is floor of at most one room and never
        // both floor and wall.
        let mut floor = BTreeSet::new();
        let mut ring = BTreeSet::new();
        for (i, r) in self.rooms.iter().enumerate() {
            for t in r.tiles() {
                let clash = if r.is_floor(t) {
                    !floor.insert(t) || ring.contains(&t)
                } else {
                    ring.insert(t);
                    floor.contains(&t)
                };
                if !in_hull(t) || clash {
                    return Err(LayoutError::BadRoom(i));
                }
            }
        }
        for (size, x, y) in STARTING_ROOMS {
            if !self
                .rooms
                .iter()
                .any(|r| r.starter && (r.size, r.x, r.y) == (size, x, y))
            {
                return Err(LayoutError::MissingStarterRoom);
            }
        }
        for def in &CATALOGUE {
            let n = self.items.iter().filter(|i| i.id == def.id).count();
            let ok = match def.count {
                Count::Exactly1 => n == 1,
                Count::BuyOnce => n <= 1,
                Count::Any => true,
                Count::PerShip => {
                    let ships: BTreeSet<_> = self
                        .items
                        .iter()
                        .filter(|i| i.id == def.id)
                        .filter_map(|i| i.ship.as_deref())
                        .filter(|s| SHIPS.contains(s))
                        .collect();
                    n == SHIPS.len() && ships.len() == n
                }
            };
            if !ok {
                return Err(LayoutError::Count(def.id));
            }
        }
        if let Some(i) = self.disconnected_room() {
            return Err(LayoutError::Disconnected(i));
        }
        let map = self.tile_map();
        for (i, item) in self.items.iter().enumerate() {
            self.check_spot(&map, item, Some(i), None)
                .map_err(|r| LayoutError::BadItem(i, r))?;
        }
        if let Some(i) = self.cut_off() {
            return Err(LayoutError::Unreachable(i));
        }
        Ok(())
    }

    /// What resetting this layout to the start refunds: the cost of every
    /// non-starter room and item, sites included (unknown IDs refund
    /// nothing).
    pub fn refund(&self) -> Cost {
        let mut total = FREE;
        for r in &self.rooms {
            add(&mut total, r.refund());
        }
        for i in &self.items {
            add(&mut total, i.refund());
        }
        total
    }
}

// ---------------------------------------------------------------- pathing

/// An axis-aligned rectangle in grid px: `x0..x1`, `y0..y1`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

/// Pilot (and crew) collider radius, px: a 24 px footprint, 0.375 of a
/// tile, under the 0.6-tile cap that lets a character stand on one tile and
/// pass 1-tile gaps and 2-tile doorways without snagging (TAKOAI-94).
pub const PILOT_RADIUS: f32 = 12.0;
/// The widest a character's footprint may be, in tiles.
pub const MAX_FOOTPRINT_TILES: f32 = 0.6;
const _: () = assert!(2.0 * PILOT_RADIUS <= MAX_FOOTPRINT_TILES * TILE);

/// Whether a circle of `PILOT_RADIUS` at grid px (x, y) stands on walkable
/// floor: its centre and eight points on its rim are each inside some
/// walkable rectangle (rectangles overlap at every join, so lanes connect).
pub fn can_stand(rects: &[Rect], x: f32, y: f32) -> bool {
    let r = PILOT_RADIUS;
    let d = r * std::f32::consts::FRAC_1_SQRT_2;
    [
        (0.0, 0.0),
        (r, 0.0),
        (-r, 0.0),
        (0.0, r),
        (0.0, -r),
        (d, d),
        (d, -d),
        (-d, d),
        (-d, -d),
    ]
    .iter()
    .all(|&(dx, dy)| rects.iter().any(|q| q.contains(x + dx, y + dy)))
}

/// One step of walking from `pos` by `delta` (grid px): the whole move if
/// the Pilot can stand there, else sliding along one axis, else no move.
pub fn walk(rects: &[Rect], pos: (f32, f32), delta: (f32, f32)) -> (f32, f32) {
    let (x, y) = pos;
    for (nx, ny) in [
        (x + delta.0, y + delta.1),
        (x + delta.0, y),
        (x, y + delta.1),
    ] {
        if (nx, ny) != pos && can_stand(rects, nx, ny) {
            return (nx, ny);
        }
    }
    pos
}

/// Click-to-move lattice spacing, px.
const PATH_STEP: f32 = 8.0;
/// A click off the floor walks to the nearest floor within this many px.
pub const SNAP_RANGE: f32 = 64.0;
const LATTICE_W: i32 = (HULL_W as f32 * TILE / PATH_STEP) as i32 + 1;
const LATTICE_H: i32 = (HULL_H as f32 * TILE / PATH_STEP) as i32 + 1;

/// Whether the Pilot can walk the straight line from `a` to `b` (checked
/// every 4 px).
pub fn clear_line(rects: &[Rect], a: (f32, f32), b: (f32, f32)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let n = ((dx * dx + dy * dy).sqrt() / 4.0).ceil().max(1.0) as i32;
    (0..=n).all(|i| {
        let t = i as f32 / n as f32;
        can_stand(rects, a.0 + dx * t, a.1 + dy * t)
    })
}

/// The standable lattice node nearest to `p` within `range` px for which
/// `ok` holds.
fn nearest_node(
    rects: &[Rect],
    p: (f32, f32),
    range: f32,
    ok: impl Fn((f32, f32)) -> bool,
) -> Option<(i32, i32)> {
    let r = (range / PATH_STEP).ceil() as i32;
    let (cx, cy) = (
        (p.0 / PATH_STEP).round() as i32,
        (p.1 / PATH_STEP).round() as i32,
    );
    let mut best: Option<((i32, i32), f32)> = None;
    for y in cy - r..=cy + r {
        for x in cx - r..=cx + r {
            let q = (x as f32 * PATH_STEP, y as f32 * PATH_STEP);
            let d = (q.0 - p.0).hypot(q.1 - p.1);
            if d > range || best.is_some_and(|(_, b)| b <= d) {
                continue;
            }
            if can_stand(rects, q.0, q.1) && ok(q) {
                best = Some(((x, y), d));
            }
        }
    }
    best.map(|(n, _)| n)
}

/// Click-to-move: a walkable route from `from` to `to` (grid px), as
/// waypoints after `from`. A goal off the floor snaps to the nearest floor
/// within [`SNAP_RANGE`]; `None` if there is none or it can't be reached.
/// A* over an 8 px lattice (8 directions), then shortened to the corners
/// that block a straight line.
pub fn find_path(rects: &[Rect], from: (f32, f32), to: (f32, f32)) -> Option<Vec<(f32, f32)>> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    let px = |(x, y): (i32, i32)| (x as f32 * PATH_STEP, y as f32 * PATH_STEP);
    let start = nearest_node(rects, from, 2.0 * PATH_STEP, |q| clear_line(rects, from, q))?;
    let goal = nearest_node(rects, to, SNAP_RANGE, |_| true)?;
    let end = if can_stand(rects, to.0, to.1) && clear_line(rects, px(goal), to) {
        to
    } else {
        px(goal)
    };
    if clear_line(rects, from, end) {
        return Some(vec![end]);
    }

    let index = |(x, y): (i32, i32)| (y * LATTICE_W + x) as usize;
    let inside = |(x, y): (i32, i32)| (0..LATTICE_W).contains(&x) && (0..LATTICE_H).contains(&y);
    let total = (LATTICE_W * LATTICE_H) as usize;
    // Octile distance in tenths of a step.
    let h = |(x, y): (i32, i32)| {
        let (dx, dy) = ((x - goal.0).unsigned_abs(), (y - goal.1).unsigned_abs());
        10 * dx.max(dy) + 4 * dx.min(dy)
    };
    let mut standable: Vec<Option<bool>> = vec![None; total];
    let mut cost = vec![u32::MAX; total];
    let mut came = vec![usize::MAX; total];
    let mut open = BinaryHeap::new();
    cost[index(start)] = 0;
    open.push(Reverse((h(start), index(start))));
    let mut found = false;
    while let Some(Reverse((_, i))) = open.pop() {
        let node = ((i as i32) % LATTICE_W, (i as i32) / LATTICE_W);
        if node == goal {
            found = true;
            break;
        }
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let next = (node.0 + dx, node.1 + dy);
            if !inside(next) {
                continue;
            }
            let j = index(next);
            let ok = *standable[j].get_or_insert_with(|| {
                let q = px(next);
                can_stand(rects, q.0, q.1)
            });
            if !ok {
                continue;
            }
            let c = cost[i] + if dx != 0 && dy != 0 { 14 } else { 10 };
            if c < cost[j] {
                cost[j] = c;
                came[j] = i;
                open.push(Reverse((c + h(next), j)));
            }
        }
    }
    if !found {
        return None;
    }
    let mut nodes = vec![goal];
    let mut i = index(goal);
    while came[i] != usize::MAX {
        i = came[i];
        nodes.push(((i as i32) % LATTICE_W, (i as i32) / LATTICE_W));
    }
    nodes.reverse();
    let mut points: Vec<(f32, f32)> = nodes.into_iter().map(px).collect();
    if *points.last()? != end {
        points.push(end);
    }
    // String pulling: from each corner, jump to the furthest point still in
    // a straight line.
    let mut path = Vec::new();
    let mut at = from;
    let mut k = 0;
    while k < points.len() {
        let mut far = k;
        while far + 1 < points.len() && clear_line(rects, at, points[far + 1]) {
            far += 1;
        }
        at = points[far];
        path.push(at);
        k = far + 1;
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RICH: Cost = cost(10_000, 1_000);
    const BROKE: Cost = cost(0, 0);

    /// The spec's map: `#` wall, `.` floor, `+` doorway, letters objects
    /// (rugs and wall-mounted items don't show), trailing hull trimmed.
    fn ascii(c: &Carrier) -> Vec<String> {
        let map = c.tile_map();
        (0..HULL_H)
            .map(|y| {
                let row: String = (0..HULL_W)
                    .map(
                        |x| match c.item_at((x, y), Layer::Object).map(|i| c.items[i].id) {
                            Some(ItemId::CommandConsole) => 'C',
                            Some(ItemId::UpgradeBench) => 'U',
                            Some(ItemId::SampleAnalyzer) => 'A',
                            Some(ItemId::Berth) => 'B',
                            Some(ItemId::SalvageSorter) => 'S',
                            Some(ItemId::CombatSimulator) => 'M',
                            Some(ItemId::BunkBed) => 'b',
                            Some(ItemId::Locker) => 'l',
                            Some(ItemId::Table) => 't',
                            Some(ItemId::CatBed) => 'c',
                            Some(ItemId::PottedPlant) => 'p',
                            Some(ItemId::HydroPlanter) => 'h',
                            Some(ItemId::FloorLamp) => 'f',
                            Some(ItemId::CrateStack) => 'x',
                            Some(_) => '?',
                            None => match map.kind((x, y)) {
                                TileKind::Hull => ' ',
                                TileKind::Wall => '#',
                                TileKind::Floor => '.',
                                TileKind::Doorway => '+',
                            },
                        },
                    )
                    .collect();
                row.trim_end().to_string()
            })
            .collect()
    }

    /// Art batch A (TAKOAI-95): every tile of the starting carrier (and of
    /// the autoplay's expansion) and every station draws with a shipped
    /// image, walls pick their tile by the n-e-s-w mask, and both doorway
    /// axes show up.
    #[test]
    fn tiles_and_stations_draw_with_shipped_art() {
        let manifest = content::ContentManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/manifest.json"
        ))
        .unwrap();
        let mut c = Carrier::starting();
        c.place_room(RoomSize::Small, 14, 10);
        let map = c.tile_map();
        let mut axes = BTreeSet::new();
        for y in 0..HULL_H {
            for x in 0..HULL_W {
                let Some(id) = map.art((x, y)) else {
                    assert_eq!(map.kind((x, y)), TileKind::Hull);
                    continue;
                };
                assert!(manifest.asset(&id).is_ok(), "({x},{y}) {id}");
                if let Some(axis) = id.strip_prefix("core.carrier.tile.doorway.") {
                    axes.insert(axis.to_string());
                }
                if map.kind((x, y)) == TileKind::Wall {
                    // n, e, s, w as '0' / '1', each set exactly when that
                    // neighbour is wall.
                    let bits: Vec<char> = id.rsplit('.').next().unwrap().chars().collect();
                    for (bit, (dx, dy)) in bits.iter().zip([(0, -1), (1, 0), (0, 1), (-1, 0)]) {
                        let joined = map.kind((x + dx, y + dy)) == TileKind::Wall;
                        assert_eq!(*bit == '1', joined, "({x},{y}) {id}");
                    }
                }
            }
        }
        assert_eq!(axes, BTreeSet::from(["ew".to_string(), "ns".to_string()]));
        for id in [
            ItemId::CommandConsole,
            ItemId::UpgradeBench,
            ItemId::SampleAnalyzer,
            ItemId::SalvageSorter,
            ItemId::CombatSimulator,
            ItemId::Berth,
        ] {
            assert!(
                manifest.asset(&id.content_id()).is_ok(),
                "{}",
                id.content_id()
            );
        }
    }

    #[test]
    fn starting_layout_matches_the_spec_map() {
        let c = Carrier::starting();
        assert_eq!(c.validate(), Ok(()));
        let expect = [
            "######################",
            "#p.CC.f#.bb.lc#p.AAhh#",
            "#......+.bb...+......#",
            "#......+..tt..+......#",
            "#......#......#......#",
            "###++#####++#####++###",
            "#.............#UU....#",
            "#.............+......#",
            "#.BBBB..BBBB..+......#",
            "#.BBBB..BBBB..#....xx#",
            "#.BBBB..BBBB..########",
            "#.BBBB..BBBB..#",
            "#.............#",
            "#.............#",
            "#.............#",
            "###############",
            "",
            "",
            "",
            "",
            "",
            "",
        ];
        assert_eq!(ascii(&c), expect);
        assert_eq!(c.rooms.len(), 5);
        assert!(c.rooms.iter().all(|r| r.starter && r.finished()));
        assert!(c.items.iter().all(|i| i.starter && i.finished()));
        assert_eq!(c.refund(), BROKE);
        let mut doors: Vec<Tile> = c.tile_map().doorways.iter().flat_map(|d| d.tiles).collect();
        doors.sort();
        let mut expect = vec![
            (7, 2),
            (7, 3),
            (14, 2),
            (14, 3),
            (3, 5),
            (4, 5),
            (10, 5),
            (11, 5),
            (17, 5),
            (18, 5),
            (14, 7),
            (14, 8),
        ];
        expect.sort();
        assert_eq!(doors, expect);
        assert!(expect.iter().all(|t| c.tile_map().is_open_doorway(*t)));
        // The wall-mounted and floor pieces the map doesn't show.
        assert_eq!(
            c.pick_item((5, 0)).map(|i| c.items[i].id),
            Some(ItemId::WallScreen)
        );
        assert_eq!(
            c.pick_item((20, 5)).map(|i| c.items[i].id),
            Some(ItemId::ToolRack)
        );
        assert_eq!(
            c.item_at((11, 4), Layer::Floor).map(|i| c.items[i].id),
            Some(ItemId::Rug)
        );
        let ships: Vec<_> = [KITE_ID, BULWARK_ID]
            .iter()
            .map(|s| c.berth_of(s).map(|i| (c.items[i].x, c.items[i].y)))
            .collect();
        assert_eq!(ships, [Some((2, 8)), Some((8, 8))]);
    }

    #[test]
    fn crew_stand_beside_their_posts() {
        let c = Carrier::starting();
        assert_eq!(c.crew_post(Crew::Gunner), Some((5, 2)));
        assert_eq!(c.crew_post(Crew::Engineer), Some((17, 7)));
        assert_eq!(c.crew_post(Crew::Researcher), Some((19, 2)));
        // A use point is 24 px south of the front edge, mid-strip.
        let console = &c.items[c.find(ItemId::CommandConsole).unwrap()];
        assert_eq!(console.use_point(), (4.0 * TILE, 2.0 * TILE + 24.0));
        assert_eq!(console.front_strip(), [(3, 2), (4, 2)]);
    }

    #[test]
    fn crew_follow_a_moved_station_and_fall_back_west_then_south() {
        let mut c = Carrier::starting();
        let i = c.find(ItemId::CommandConsole).unwrap();
        assert_eq!(c.can_move(i, 2, 1, None), Ok(()));
        let i = c.move_item(i, 2, 1);
        assert_eq!((c.items[i].x, c.items[i].starter), (2, true));
        assert_eq!(c.crew_post(Crew::Gunner), Some((4, 2)));
        // East blocked: west of the strip.
        c.place(ItemId::Stool, 4, 2, None);
        assert_eq!(c.crew_post(Crew::Gunner), Some((1, 2)));
        // East and west blocked: south of the first strip tile.
        c.place(ItemId::Stool, 1, 2, None);
        assert_eq!(c.crew_post(Crew::Gunner), Some((2, 3)));
    }

    #[test]
    fn room_anchor_from_the_mouse_tile() {
        assert_eq!(Room::anchor_for(RoomSize::Small, (17, 12)), (14, 10));
        assert_eq!(Room::anchor_for(RoomSize::Large, (7, 10)), (0, 5));
    }

    #[test]
    fn spec_example_edits_build_and_finish() {
        let mut c = Carrier::starting();
        assert_eq!(
            c.can_place_room(RoomSize::Small, 14, 10, cost(60, 1)),
            Ok(())
        );
        assert_eq!(
            c.can_place_room(RoomSize::Small, 14, 10, cost(60, 0)),
            Err(Reason::Need(cost(60, 1)))
        );
        let r = c.place_room(RoomSize::Small, 14, 10);
        assert_eq!(c.rooms[r].build_left_ms, Some(5000));
        let map = c.tile_map();
        for t in [(17, 10), (18, 10), (14, 12), (14, 13)] {
            assert_eq!(map.kind(t), TileKind::Doorway, "{t:?}");
            // Shut while the room is being built.
            assert!(!map.is_open_doorway(t), "{t:?}");
        }
        assert!(!c.reachable().contains(&(16, 12)));
        assert_eq!(
            c.can_place(ItemId::SalvageSorter, 19, 11, RICH, None),
            Err(Reason::StillBeingBuilt)
        );
        assert!(!c.tick(4999));
        assert!(c.tick(1));
        assert!(c.rooms[r].finished());
        assert!(c.reachable().contains(&(16, 12)));
        assert!(c.tile_map().is_open_doorway((14, 12)));

        assert_eq!(
            c.can_place(ItemId::SalvageSorter, 19, 11, cost(120, 3), None),
            Ok(())
        );
        let s = c.place(ItemId::SalvageSorter, 19, 11, None);
        assert_eq!(c.items[s].front_strip(), [(19, 13), (20, 13)]);
        assert_eq!(c.items[s].build_left_ms, Some(4000));
        assert!(!c.has_finished(ItemId::SalvageSorter));
        assert_eq!(
            c.can_place(ItemId::CombatSimulator, 12, 9, RICH, None),
            Ok(())
        );
        let m = c.place(ItemId::CombatSimulator, 12, 9, None);
        assert_eq!(c.items[m].front_strip(), [(12, 11), (13, 11)]);
        assert_eq!(c.validate(), Ok(()));
        assert!(c.tick(4000));
        assert!(c.has_finished(ItemId::SalvageSorter) && c.has_finished(ItemId::CombatSimulator));
        assert!(!c.tick(1000));
        assert_eq!(c.validate(), Ok(()));
        assert_eq!(c.refund(), cost(60 + 120 + 200, 1 + 3 + 6));
    }

    #[test]
    fn sites_tick_together_and_cancel_with_a_full_refund() {
        let mut c = Carrier::starting();
        c.place_room(RoomSize::Large, 21, 5);
        let s = c.place(ItemId::Stool, 2, 3, None);
        assert_eq!(c.items[s].build_left_ms, Some(2000));
        // An item site already blocks its tile.
        assert_eq!(
            c.can_place(ItemId::Stool, 2, 3, RICH, None),
            Err(Reason::Blocked)
        );
        assert_eq!(c.can_move(s, 2, 2, None), Err(Reason::StillBeingBuilt));
        assert!(c.tick(2000));
        assert_eq!(
            c.rooms
                .iter()
                .find(|r| r.size == RoomSize::Large && !r.starter)
                .unwrap()
                .build_left_ms,
            Some(6000)
        );
        let r = c.rooms.iter().position(|r| (r.x, r.y) == (21, 5)).unwrap();
        assert_eq!(c.can_remove_room(r), Ok(cost(220, 4)));
        assert_eq!(c.remove_room(r), cost(220, 4));
        let s = c.place(ItemId::SalvageSorter, 1, 2, None);
        assert_eq!(c.can_remove(s), Ok(cost(120, 3)));
        assert_eq!(c.remove(s), cost(120, 3));
        assert_eq!(c.validate(), Ok(()));
    }

    #[test]
    fn room_placement_reasons() {
        let c = Carrier::starting();
        assert_eq!(
            c.can_place_room(RoomSize::Small, 30, 0, RICH),
            Err(Reason::OutsideHull)
        );
        assert_eq!(
            c.can_place_room(RoomSize::Large, 0, 12, RICH),
            Err(Reason::OutsideHull)
        );
        assert_eq!(
            c.can_place_room(RoomSize::Small, -1, 16, RICH),
            Err(Reason::OutsideHull)
        );
        assert_eq!(
            c.can_place_room(RoomSize::Small, 1, 1, RICH),
            Err(Reason::OverlapsRoom)
        );
        // Floor over another room's wall.
        assert_eq!(
            c.can_place_room(RoomSize::Small, 13, 10, RICH),
            Err(Reason::OverlapsRoom)
        );
        assert_eq!(
            c.can_place_room(RoomSize::Small, 26, 14, RICH),
            Err(Reason::MustShareWall)
        );
        // Only a corner, or a single facing tile, doesn't connect.
        assert_eq!(
            c.can_place_room(RoomSize::Small, 21, 10, RICH),
            Err(Reason::MustShareWall)
        );
        assert_eq!(
            c.can_place_room(RoomSize::Small, 21, 8, RICH),
            Err(Reason::MustShareWall)
        );
        // East of the Workshop: the doorway at (21,8)-(21,9) would land
        // next to a crate stack; one row up it's clear.
        let crate_stack = c.item_at((20, 9), Layer::Object).unwrap();
        assert_eq!(
            c.can_place_room(RoomSize::Small, 21, 7, RICH),
            Err(Reason::ClearDoorway(vec![crate_stack]))
        );
        assert_eq!(c.can_place_room(RoomSize::Small, 21, 5, RICH), Ok(()));
        assert_eq!(
            c.can_place_room(RoomSize::Large, 21, 5, cost(219, 4))
                .unwrap_err()
                .to_string(),
            "Need 220 cr + 4 VC"
        );
    }

    #[test]
    fn a_new_doorway_must_land_clear() {
        // An object on the far side's clearance tile.
        let mut c = Carrier::starting();
        let stool = c.place(ItemId::Stool, 17, 9, None);
        c.tick(2000);
        assert_eq!(
            c.can_place_room(RoomSize::Small, 14, 10, RICH),
            Err(Reason::ClearDoorway(vec![stool]))
        );
        // A rug there is fine.
        c.remove(stool);
        c.place(ItemId::Rug, 17, 8, None);
        c.tick(2000);
        assert_eq!(c.can_place_room(RoomSize::Small, 14, 10, RICH), Ok(()));

        // A wall-mounted item on the new doorway tile itself: room D east
        // of the Workshop, a wall light on its north wall, then a room
        // north of D whose doorway lands on that light.
        let mut c = Carrier::starting();
        c.place_room(RoomSize::Small, 21, 5);
        assert_eq!(
            c.can_place(ItemId::WallLight, 24, 5, RICH, None),
            Err(Reason::StillBeingBuilt)
        );
        c.tick(5000);
        assert_eq!(c.can_place(ItemId::WallLight, 24, 5, RICH, None), Ok(()));
        let light = c.place(ItemId::WallLight, 24, 5, None);
        assert_eq!(
            c.can_place_room(RoomSize::Small, 21, 0, RICH),
            Err(Reason::ClearDoorway(vec![light]))
        );
        assert_eq!(
            Reason::ClearDoorway(vec![light]).to_string(),
            "Clear the doorway first"
        );
    }

    #[test]
    fn room_removal_reasons() {
        let mut c = Carrier::starting();
        assert_eq!(c.can_remove_room(0), Err(Reason::StarterPiece));
        c.place_room(RoomSize::Small, 21, 5);
        c.place_room(RoomSize::Small, 28, 5);
        c.tick(5000);
        let d = c.rooms.iter().position(|r| (r.x, r.y) == (21, 5)).unwrap();
        let e = c.rooms.iter().position(|r| (r.x, r.y) == (28, 5)).unwrap();
        // D links E to the rest.
        assert_eq!(c.can_remove_room(d), Err(Reason::WouldCutOffRoom));
        assert_eq!(c.can_remove_room(e), Ok(cost(60, 1)));
        // A wall-mounted item on E's north wall face counts as inside.
        let light = c.place(ItemId::WallLight, 30, 5, None);
        assert_eq!(c.can_remove_room(e), Err(Reason::RoomNotEmpty));
        c.remove(light);
        let stool = c.place(ItemId::Stool, 31, 7, None);
        assert_eq!(c.can_remove_room(e), Err(Reason::RoomNotEmpty));
        c.remove(stool);
        assert_eq!(c.remove_room(e), cost(60, 1));
        assert_eq!(c.validate(), Ok(()));
    }

    #[test]
    fn item_placement_reasons() {
        let c = Carrier::starting();
        let id = ItemId::Stool;
        assert_eq!(
            c.can_place(ItemId::CommandConsole, 1, 2, RICH, None),
            Err(Reason::StarterPiece)
        );
        assert_eq!(
            c.can_place(ItemId::Berth, 1, 2, RICH, None),
            Err(Reason::StarterPiece)
        );
        assert_eq!(c.can_place(id, 0, 0, RICH, None), Err(Reason::NeedsFloor));
        assert_eq!(c.can_place(id, 30, 18, RICH, None), Err(Reason::NeedsFloor));
        assert_eq!(
            c.can_place(id, 7, 2, RICH, None),
            Err(Reason::KeepDoorwayClear)
        );
        assert_eq!(
            c.can_place(id, 8, 2, RICH, None),
            Err(Reason::KeepDoorwayClear)
        );
        assert_eq!(c.can_place(id, 10, 3, RICH, None), Err(Reason::Blocked));
        assert_eq!(
            c.can_place(id, 3, 2, RICH, None),
            Err(Reason::KeepFrontClear)
        );
        assert_eq!(
            c.can_place(id, 2, 3, RICH, Some((2, 3))),
            Err(Reason::PilotInTheWay)
        );
        assert_eq!(
            c.can_place(id, 2, 3, BROKE, None),
            Err(Reason::Need(cost(10, 0)))
        );
        assert_eq!(Reason::Need(cost(10, 0)).to_string(), "Need 10 cr");
        assert_eq!(c.can_place(id, 2, 3, RICH, Some((5, 5))), Ok(()));
        // Rugs: floor layer, fine on a doorway's clearance, not on a doorway
        // or another rug.
        assert_eq!(c.can_place(ItemId::Rug, 12, 3, RICH, None), Ok(()));
        assert_eq!(
            c.can_place(ItemId::Rug, 13, 2, RICH, None),
            Err(Reason::NeedsFloor)
        );
        assert_eq!(
            c.can_place(ItemId::Rug, 11, 3, RICH, None),
            Err(Reason::Blocked)
        );
        // Against a wall / on a wall.
        assert_eq!(
            c.can_place(ItemId::Locker, 2, 2, RICH, None),
            Err(Reason::NeedsWallBehind)
        );
        assert_eq!(c.can_place(ItemId::Locker, 2, 1, RICH, None), Ok(()));
        assert_eq!(
            c.can_place(ItemId::WallLight, 3, 3, RICH, None),
            Err(Reason::NeedsWallFace)
        );
        assert_eq!(
            c.can_place(ItemId::WallLight, 0, 2, RICH, None),
            Err(Reason::NeedsWallFace)
        );
        assert_eq!(
            c.can_place(ItemId::WallLight, 5, 0, RICH, None),
            Err(Reason::Blocked)
        );
        assert_eq!(c.can_place(ItemId::WallLight, 1, 5, RICH, None), Ok(()));
        // A station's own front strip: blocked, or on a wall.
        let mut d = c.clone();
        d.place(id, 1, 4, None);
        assert_eq!(
            d.can_place(ItemId::SalvageSorter, 1, 2, RICH, None),
            Err(Reason::KeepFrontClear)
        );
        assert_eq!(
            c.can_place(ItemId::SalvageSorter, 1, 3, RICH, None),
            Err(Reason::KeepFrontClear)
        );
        assert_eq!(c.can_place(ItemId::SalvageSorter, 1, 2, RICH, None), Ok(()));
        d.place(ItemId::SalvageSorter, 9, 12, None);
        assert_eq!(
            d.can_place(ItemId::SalvageSorter, 1, 2, RICH, None),
            Err(Reason::AlreadyBuilt)
        );
    }

    #[test]
    fn edits_that_cut_off_a_station_are_refused() {
        // Box the Command Console's front strip in.
        let mut c = Carrier::starting();
        for (x, y) in [(2, 2), (5, 2), (3, 3)] {
            assert_eq!(c.can_place(ItemId::Stool, x, y, RICH, None), Ok(()));
            c.place(ItemId::Stool, x, y, None);
        }
        c.tick(2000);
        let reason = c.can_place(ItemId::Stool, 4, 3, RICH, None).unwrap_err();
        assert_eq!(reason, Reason::WouldCutOff(ItemId::SampleAnalyzer));
        assert_eq!(reason.to_string(), "Would cut off the Sample Analyzer");
        // Moving a stool there is refused the same way.
        let s = c.item_at((3, 3), Layer::Object).unwrap();
        assert_eq!(c.can_move(s, 4, 3, None), Ok(()));
        let lamp = c.item_at((6, 1), Layer::Object).unwrap();
        assert_eq!(
            c.can_move(lamp, 4, 3, None),
            Err(Reason::WouldCutOff(ItemId::SampleAnalyzer))
        );

        // A saved stool on a doorway's clearance fails validation.
        let mut c = Carrier::starting();
        for (x, y) in [(17, 6), (17, 7)] {
            c.place(ItemId::Stool, x, y, None);
        }
        assert_eq!(
            c.validate(),
            Err(LayoutError::BadItem(
                c.item_at((17, 6), Layer::Object).unwrap(),
                Reason::KeepDoorwayClear
            ))
        );
    }

    #[test]
    fn moving_and_removing_items() {
        let mut c = Carrier::starting();
        let console = c.find(ItemId::CommandConsole).unwrap();
        assert_eq!(c.can_remove(console), Err(Reason::MoveItInstead));
        assert_eq!(Reason::MoveItInstead.to_string(), "Move it instead");
        assert_eq!(c.can_move(console, 1, 1, None), Err(Reason::Blocked));
        // Its own tiles count as free while held.
        assert_eq!(c.can_move(console, 4, 1, None), Ok(()));
        assert_eq!(
            c.can_move(console, 2, 1, Some((2, 1))),
            Err(Reason::PilotInTheWay)
        );
        let berth = c.berth_of(KITE_ID).unwrap();
        assert_eq!(c.can_remove(berth), Err(Reason::MoveItInstead));
        assert_eq!(c.can_move(berth, 2, 7, None), Ok(()));
        let berth = c.move_item(berth, 2, 7);
        assert_eq!(c.items[berth].ship.as_deref(), Some(KITE_ID));
        assert!(c.items[berth].starter);
        assert_eq!(c.validate(), Ok(()));
        // Starter furniture refunds nothing; bought pieces refund in full.
        let plant = c.item_at((1, 1), Layer::Object).unwrap();
        assert_eq!(c.can_remove(plant), Ok(BROKE));
        let lamp = c.place(ItemId::FloorLamp, 2, 3, None);
        c.tick(2000);
        let lamp = c.move_item(lamp, 5, 4);
        assert_eq!(c.can_remove(lamp), Ok(cost(15, 0)));
    }

    #[test]
    fn walkable_rects_cover_the_walkable_tiles() {
        let c = Carrier::starting();
        let rects = c.walkable();
        let area: f32 = rects.iter().map(|r| (r.x1 - r.x0) * (r.y1 - r.y0)).sum();
        let map = c.tile_map();
        let walk = c.walkable_tiles(&map, &c.occupancy(None));
        assert_eq!(area, walk.len() as f32 * TILE * TILE);
        assert!(rects.len() < walk.len() / 2, "{}", rects.len());
        let centre = |(x, y): Tile| ((x as f32 + 0.5) * TILE, (y as f32 + 0.5) * TILE);
        let console = &c.items[c.find(ItemId::CommandConsole).unwrap()];
        let from = console.use_point();
        assert!(can_stand(&rects, from.0, from.1));
        // Not on a berth, a wall or bare hull.
        for t in [(3, 9), (0, 3), (30, 18)] {
            let p = centre(t);
            assert!(!can_stand(&rects, p.0, p.1), "{t:?}");
        }
        // Through two doorways to the Engineer.
        let to = centre(c.crew_post(Crew::Engineer).unwrap());
        let path = find_path(&rects, from, to).expect("reachable");
        assert_eq!(*path.last().unwrap(), to);
        // A room under construction isn't walkable yet.
        let mut c = c;
        c.place_room(RoomSize::Small, 14, 10);
        let p = centre((16, 12));
        assert!(!can_stand(&c.walkable(), p.0, p.1));
        c.tick(5000);
        assert!(can_stand(&c.walkable(), p.0, p.1));
    }

    #[test]
    fn characters_are_smaller_than_a_tile() {
        const { assert!(2.0 * PILOT_RADIUS <= MAX_FOOTPRINT_TILES * TILE) };
        // Standing in the middle of one tile, the footprint stays on it.
        let tile = [Rect {
            x0: 0.0,
            y0: 0.0,
            x1: TILE,
            y1: TILE,
        }];
        assert!(can_stand(&tile, TILE / 2.0, TILE / 2.0));
        // Through a 1-tile gap between two rooms, by path and straight.
        let room = |x0: f32| Rect {
            x0,
            y0: 0.0,
            x1: x0 + 4.0 * TILE,
            y1: 4.0 * TILE,
        };
        let gap = Rect {
            x0: 4.0 * TILE,
            y0: 2.0 * TILE,
            x1: 5.0 * TILE,
            y1: 3.0 * TILE,
        };
        let rects = [room(0.0), gap, room(5.0 * TILE)];
        let (a, b) = ((TILE, 2.5 * TILE), (8.0 * TILE, 2.5 * TILE));
        assert!(clear_line(&rects, a, b));
        assert_eq!(
            *find_path(&rects, (TILE, TILE), b).unwrap().last().unwrap(),
            b
        );
    }

    #[test]
    fn characters_walk_through_a_two_tile_doorway() {
        // The spec example's Small room south of the Workshop, joined by the
        // 2-tile doorway (17,10)-(18,10).
        let mut c = Carrier::starting();
        c.place_room(RoomSize::Small, 14, 10);
        c.tick(5000);
        let map = c.tile_map();
        assert!(map.is_open_doorway((17, 10)) && map.is_open_doorway((18, 10)));
        let rects = c.walkable();
        let (north, south) = ((18.0 * TILE, 9.5 * TILE), (18.0 * TILE, 11.5 * TILE));
        assert!(can_stand(&rects, north.0, north.1));
        assert!(can_stand(&rects, south.0, south.1));
        // Straight through the middle, and off-centre within either tile.
        assert!(clear_line(&rects, north, south));
        for x in [17.0 * TILE + PILOT_RADIUS, 19.0 * TILE - PILOT_RADIUS] {
            assert!(clear_line(&rects, (x, 9.5 * TILE), (x, 11.5 * TILE)), "{x}");
        }
        // And by click-to-move from deep in the new room.
        let from = (15.5 * TILE, 12.5 * TILE);
        let path = find_path(&rects, from, north).expect("through the doorway");
        assert_eq!(*path.last().unwrap(), north);
    }

    #[test]
    fn validation_catches_broken_layouts() {
        let ok = Carrier::starting();
        let mut c = ok.clone();
        c.items.push(Item::new(ItemId::Unknown, 2, 3));
        assert_eq!(c.validate(), Err(LayoutError::UnknownId));
        let mut c = ok.clone();
        c.rooms.push(Room::new(RoomSize::Small, 2, 2));
        assert!(matches!(c.validate(), Err(LayoutError::BadRoom(_))));
        let mut c = ok.clone();
        c.rooms.push(Room::new(RoomSize::Small, 30, 18));
        assert!(matches!(c.validate(), Err(LayoutError::BadRoom(_))));
        let mut c = ok.clone();
        c.rooms.push(Room::new(RoomSize::Small, 26, 14));
        c.normalize();
        assert!(matches!(c.validate(), Err(LayoutError::Disconnected(_))));
        let mut c = ok.clone();
        c.rooms[0].starter = false;
        assert_eq!(c.validate(), Err(LayoutError::MissingStarterRoom));
        let mut c = ok.clone();
        c.items.push(Item::starter(ItemId::UpgradeBench, 1, 1));
        assert_eq!(c.validate(), Err(LayoutError::Count(ItemId::UpgradeBench)));
        let mut c = ok.clone();
        let b = c.berth_of(BULWARK_ID).unwrap();
        c.items[b].ship = Some(KITE_ID.into());
        assert_eq!(c.validate(), Err(LayoutError::Count(ItemId::Berth)));
        c.items[b].ship = Some("dreadnought".into());
        assert_eq!(c.validate(), Err(LayoutError::Count(ItemId::Berth)));
        let mut c = ok.clone();
        c.items.push(Item::new(ItemId::SalvageSorter, 1, 2));
        c.items.push(Item::new(ItemId::SalvageSorter, 9, 12));
        assert_eq!(c.validate(), Err(LayoutError::Count(ItemId::SalvageSorter)));
        // Two items on one tile of one layer.
        let mut c = ok.clone();
        c.items.push(Item::new(ItemId::Stool, 13, 1));
        c.normalize();
        assert!(matches!(
            c.validate(),
            Err(LayoutError::BadItem(_, Reason::Blocked))
        ));
        // An item in a room that is still being built.
        let mut c = ok.clone();
        c.place_room(RoomSize::Small, 14, 10);
        c.items.push(Item::new(ItemId::Stool, 16, 12));
        c.normalize();
        assert!(matches!(
            c.validate(),
            Err(LayoutError::BadItem(_, Reason::StillBeingBuilt))
        ));
        // Unreachable: a sealed-off Upgrade Bench (stools on the bench's
        // strip neighbours and across the Workshop).
        let mut c = ok.clone();
        for x in 15..=20 {
            c.items.push(Item::new(ItemId::Stool, x, 8));
        }
        c.items.push(Item::new(ItemId::Stool, 17, 7));
        c.normalize();
        // (15,8) is the Hangar doorway's clearance tile.
        assert!(matches!(
            c.validate(),
            Err(LayoutError::BadItem(_, Reason::KeepDoorwayClear))
        ));
    }

    #[test]
    fn unreachable_station_fails_validation() {
        // Fence the Sample Analyzer off: its strip (17,2)-(18,2), neighbours
        // (16,2), (19,2) and the row below. Hydro planter at (19,1).
        let mut c = Carrier::starting();
        for t in [(16, 2), (19, 2), (17, 3), (18, 3)] {
            c.items.push(Item::new(ItemId::Stool, t.0, t.1));
        }
        c.normalize();
        let a = c.find(ItemId::SampleAnalyzer).unwrap();
        assert_eq!(c.validate(), Err(LayoutError::Unreachable(a)));
    }

    #[test]
    fn catalogue_matches_the_spec() {
        assert_eq!(CATALOGUE.len(), 25);
        let keys: BTreeSet<_> = CATALOGUE.iter().map(|d| d.key).collect();
        assert_eq!(keys.len(), 25);
        for d in &CATALOGUE {
            // The serde name is the key.
            assert_eq!(
                serde_json::to_string(&d.id).unwrap(),
                format!("\"{}\"", d.key)
            );
            let secs = if d.station() { 4 } else { 2 };
            assert_eq!(d.build_seconds, secs, "{}", d.key);
            assert_eq!(
                d.layer == Layer::Wall,
                d.placement == Placement::OnWall,
                "{}",
                d.key
            );
            if !d.station() {
                assert_eq!(d.cost.void_crystal, 0, "{}", d.key);
            }
        }
        assert_eq!(ItemId::SalvageSorter.def().unwrap().cost, cost(120, 3));
        assert_eq!(ItemId::CombatSimulator.def().unwrap().cost, cost(200, 6));
        assert_eq!(ItemId::Berth.def().unwrap().size, (4, 4));
        assert_eq!(ItemId::Rug.def().unwrap().category, Category::Decor);
        assert_eq!(ItemId::WallLight.def().unwrap().category, Category::Decor);
        assert_eq!(ItemId::Locker.def().unwrap().category, Category::Furniture);
        assert_eq!(RoomSize::Small.cost(), cost(60, 1));
        assert_eq!(RoomSize::Large.cost(), cost(220, 4));
        let unknown: ItemId = serde_json::from_str("\"jacuzzi\"").unwrap();
        assert_eq!(unknown, ItemId::Unknown);
    }
}
