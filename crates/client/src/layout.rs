//! The 2.5D carrier's layout rules (design/READINESS.md § Carrier › Layout
//! model, Building, Room catalogue, Starting layout, Save). Pure data, no
//! Bevy: the hull grid, the room catalogue, corridor masks, connectivity,
//! what may be built or demolished where and why, and the walkable area.
//!
//! Grid coordinates: cell (0,0) is the hull's north-west corner, x grows
//! east, y grows south. "Grid px" are world px in the same orientation
//! (`CELL` per cell); the Carrier scene flips y for Bevy.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::workshop::Cost;

/// Hull size in cells (fixed in the demo).
pub const HULL_W: i32 = 12;
pub const HULL_H: i32 = 8;
/// One cell in world px.
pub const CELL: f32 = 128.0;
/// The back-wall face at the top of a room's top row.
pub const BACK_WALL: f32 = 48.0;
/// Side walls and the front lip.
pub const WALL: f32 = 12.0;
/// Door and corridor lane width, centred on the cell edge.
pub const LANE_X0: f32 = 32.0;
pub const LANE_X1: f32 = 96.0;
/// A corridor's centre square, cell-local y (and the E / W lanes' span).
pub const LANE_Y0: f32 = 48.0;
pub const LANE_Y1: f32 = 116.0;

/// The side of a cell a door or corridor opening faces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    N,
    E,
    S,
    W,
}

impl Side {
    /// n-e-s-w, the order corridor masks are named in.
    pub const ALL: [Side; 4] = [Side::N, Side::E, Side::S, Side::W];

    pub fn delta(self) -> (i32, i32) {
        match self {
            Side::N => (0, -1),
            Side::E => (1, 0),
            Side::S => (0, 1),
            Side::W => (-1, 0),
        }
    }

    pub fn opposite(self) -> Side {
        match self {
            Side::N => Side::S,
            Side::E => Side::W,
            Side::S => Side::N,
            Side::W => Side::E,
        }
    }

    pub fn letter(self) -> char {
        match self {
            Side::N => 'n',
            Side::E => 'e',
            Side::S => 's',
            Side::W => 'w',
        }
    }

    fn bit(self) -> u8 {
        match self {
            Side::N => 1,
            Side::E => 2,
            Side::S => 4,
            Side::W => 8,
        }
    }
}

/// A corridor's openings, one bit per [`Side`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Mask(u8);

impl Mask {
    pub fn has(self, side: Side) -> bool {
        self.0 & side.bit() != 0
    }

    /// Sides in n-e-s-w order, e.g. `nes`; the corridor art's name.
    pub fn name(self) -> String {
        Side::ALL
            .iter()
            .filter(|s| self.has(**s))
            .map(|s| s.letter())
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomId {
    Bridge,
    CrewQuarters,
    Workshop,
    Dock,
    SalvageBay,
    TrainingRoom,
    /// Anything a save names that this build doesn't know. Never valid in a
    /// layout; `validate` resets the layout when it sees one.
    #[serde(other)]
    Unknown,
}

/// A door socket: a cell on the room's edge (room-local) and the side the
/// door faces.
pub type Socket = (i32, i32, Side);

impl RoomId {
    pub const BUILDABLE: [RoomId; 2] = [RoomId::SalvageBay, RoomId::TrainingRoom];

    /// Stable ID (save file, content IDs).
    pub fn id(self) -> &'static str {
        match self {
            RoomId::Bridge => "bridge",
            RoomId::CrewQuarters => "crew_quarters",
            RoomId::Workshop => "workshop",
            RoomId::Dock => "dock",
            RoomId::SalvageBay => "salvage_bay",
            RoomId::TrainingRoom => "training_room",
            RoomId::Unknown => "unknown",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RoomId::Bridge => "Bridge",
            RoomId::CrewQuarters => "Crew Quarters",
            RoomId::Workshop => "Workshop",
            RoomId::Dock => "Dock",
            RoomId::SalvageBay => "Salvage Bay",
            RoomId::TrainingRoom => "Training Room",
            RoomId::Unknown => "?",
        }
    }

    /// Width × depth in cells.
    pub fn footprint(self) -> (i32, i32) {
        match self {
            RoomId::Bridge | RoomId::Workshop => (3, 2),
            RoomId::Dock => (4, 3),
            RoomId::CrewQuarters | RoomId::SalvageBay | RoomId::TrainingRoom => (2, 2),
            RoomId::Unknown => (0, 0),
        }
    }

    pub fn sockets(self) -> &'static [Socket] {
        use Side::*;
        match self {
            RoomId::Bridge => &[(1, 1, S), (2, 0, E)],
            RoomId::CrewQuarters => &[(0, 1, S), (0, 0, W), (1, 0, E)],
            RoomId::Workshop => &[(1, 0, N), (2, 1, E)],
            RoomId::Dock => &[(0, 2, W), (3, 2, E)],
            RoomId::SalvageBay | RoomId::TrainingRoom => {
                &[(0, 0, N), (1, 0, E), (1, 1, S), (0, 1, W)]
            }
            RoomId::Unknown => &[],
        }
    }

    /// Build cost; `None` for the starting rooms.
    pub fn cost(self) -> Option<Cost> {
        match self {
            RoomId::SalvageBay => Some(Cost {
                credits: 120,
                void_crystal: 3,
            }),
            RoomId::TrainingRoom => Some(Cost {
                credits: 200,
                void_crystal: 6,
            }),
            _ => None,
        }
    }

    pub fn effect(self) -> &'static str {
        match self {
            RoomId::SalvageBay => "+25% credits and Void Crystal from every successful mission",
            RoomId::TrainingRoom => "-15% Q and W cooldowns on every battleship",
            RoomId::Dock => "launches missions",
            _ => "",
        }
    }

    pub fn content_id(self) -> String {
        format!("core.carrier.room.{}", self.id())
    }
}

pub const CORRIDOR_COST: Cost = Cost {
    credits: 10,
    void_crystal: 0,
};

/// Something the Build tab can place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Corridor,
    Room(RoomId),
}

impl Piece {
    /// Build tab order: corridor first, then the buildable rooms.
    pub const BUILDABLE: [Piece; 3] = [
        Piece::Corridor,
        Piece::Room(RoomId::SalvageBay),
        Piece::Room(RoomId::TrainingRoom),
    ];

    pub fn name(self) -> &'static str {
        match self {
            Piece::Corridor => "Corridor",
            Piece::Room(r) => r.name(),
        }
    }

    pub fn footprint(self) -> (i32, i32) {
        match self {
            Piece::Corridor => (1, 1),
            Piece::Room(r) => r.footprint(),
        }
    }

    pub fn cost(self) -> Cost {
        match self {
            Piece::Corridor => CORRIDOR_COST,
            Piece::Room(r) => r.cost().unwrap_or(Cost {
                credits: 0,
                void_crystal: 0,
            }),
        }
    }

    pub fn effect(self) -> &'static str {
        match self {
            Piece::Corridor => "connects rooms",
            Piece::Room(r) => r.effect(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PlacedRoom {
    pub id: RoomId,
    /// Anchor: the room's north-west cell.
    pub x: i32,
    pub y: i32,
}

impl PlacedRoom {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let (w, d) = self.id.footprint();
        x >= self.x && x < self.x + w && y >= self.y && y < self.y + d
    }

    /// Sockets in hull cells: the socket cell and the side it faces.
    pub fn sockets(&self) -> impl Iterator<Item = (i32, i32, Side)> + '_ {
        self.id
            .sockets()
            .iter()
            .map(|&(sx, sy, side)| (self.x + sx, self.y + sy, side))
    }

    /// The cell each socket faces (where its corridor would go).
    pub fn socket_fronts(&self) -> impl Iterator<Item = (i32, i32, Side)> + '_ {
        self.sockets().map(|(x, y, side)| {
            let (dx, dy) = side.delta();
            (x + dx, y + dy, side)
        })
    }

    fn cells(&self) -> impl Iterator<Item = (i32, i32)> {
        let (w, d) = self.id.footprint();
        let (x0, y0) = (self.x, self.y);
        (y0..y0 + d).flat_map(move |y| (x0..x0 + w).map(move |x| (x, y)))
    }
}

/// What occupies a hull cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Empty,
    Corridor,
    /// Index into [`CarrierLayout::rooms`].
    Room(usize),
}

/// Why a piece can't go somewhere (the build ghost's red line).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlaceError {
    OutsideHull,
    Blocked,
    AlreadyBuilt,
    MustConnect,
    CantAfford(Cost),
}

impl fmt::Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaceError::OutsideHull => write!(f, "Outside the hull"),
            PlaceError::Blocked => write!(f, "Blocked"),
            PlaceError::AlreadyBuilt => write!(f, "Already built"),
            PlaceError::MustConnect => write!(f, "Must connect to a corridor"),
            PlaceError::CantAfford(c) if c.void_crystal > 0 => {
                write!(f, "Need {} cr + {} VC", c.credits, c.void_crystal)
            }
            PlaceError::CantAfford(c) => write!(f, "Need {} cr", c.credits),
        }
    }
}

/// Why a piece can't be demolished.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DemolishError {
    Nothing,
    Starting,
    WouldDisconnect,
}

impl fmt::Display for DemolishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DemolishError::Nothing => write!(f, "Nothing to demolish"),
            DemolishError::Starting => write!(f, "Part of the original carrier"),
            DemolishError::WouldDisconnect => write!(f, "Something would be cut off"),
        }
    }
}

/// Why a saved layout was rejected on load.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayoutError {
    UnknownRoom,
    OutsideHull,
    Overlap,
    MissingStart,
    Duplicate,
    Disconnected,
}

/// The carrier: rooms and corridor cells on the hull grid. Stored in the
/// save as-is; rooms sorted by id and corridors by (x, y), so the same
/// layout always writes the same bytes.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CarrierLayout {
    pub rooms: Vec<PlacedRoom>,
    pub corridors: Vec<(i32, i32)>,
}

impl Default for CarrierLayout {
    fn default() -> Self {
        Self::starting()
    }
}

const STARTING_ROOMS: [PlacedRoom; 4] = [
    PlacedRoom {
        id: RoomId::Bridge,
        x: 0,
        y: 1,
    },
    PlacedRoom {
        id: RoomId::CrewQuarters,
        x: 4,
        y: 1,
    },
    PlacedRoom {
        id: RoomId::Workshop,
        x: 0,
        y: 4,
    },
    PlacedRoom {
        id: RoomId::Dock,
        x: 4,
        y: 4,
    },
];

const STARTING_CORRIDORS: [(i32, i32); 7] =
    [(1, 3), (2, 3), (3, 3), (4, 3), (3, 4), (3, 5), (3, 6)];

pub fn in_hull(x: i32, y: i32) -> bool {
    (0..HULL_W).contains(&x) && (0..HULL_H).contains(&y)
}

/// An axis-aligned rectangle in grid px: `x0..x1`, `y0..y1`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    fn cell(cx: i32, cy: i32, x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        let (ox, oy) = (cx as f32 * CELL, cy as f32 * CELL);
        Self {
            x0: ox + x0,
            y0: oy + y0,
            x1: ox + x1,
            y1: oy + y1,
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

impl CarrierLayout {
    pub fn starting() -> Self {
        let mut layout = Self {
            rooms: STARTING_ROOMS.to_vec(),
            corridors: STARTING_CORRIDORS.to_vec(),
        };
        layout.normalize();
        layout
    }

    /// Canonical order: rooms by id, corridors by (x, y).
    pub fn normalize(&mut self) {
        self.rooms.sort_by_key(|r| (r.id.id(), r.x, r.y));
        self.corridors.sort_unstable();
        self.corridors.dedup();
    }

    pub fn has_room(&self, id: RoomId) -> bool {
        self.rooms.iter().any(|r| r.id == id)
    }

    #[cfg(test)]
    pub fn room(&self, id: RoomId) -> Option<&PlacedRoom> {
        self.rooms.iter().find(|r| r.id == id)
    }

    /// Whether any room beyond the starting four has been built.
    pub fn has_built_room(&self) -> bool {
        self.rooms.iter().any(|r| r.id.cost().is_some())
    }

    pub fn cell(&self, x: i32, y: i32) -> Cell {
        if self.corridors.contains(&(x, y)) {
            return Cell::Corridor;
        }
        self.rooms
            .iter()
            .position(|r| r.contains(x, y))
            .map_or(Cell::Empty, Cell::Room)
    }

    fn is_corridor(&self, x: i32, y: i32) -> bool {
        self.corridors.contains(&(x, y))
    }

    /// A room socket on cell (x, y) facing `side`, if there is one.
    fn socket_at(&self, x: i32, y: i32, side: Side) -> Option<usize> {
        self.rooms
            .iter()
            .position(|r| r.sockets().any(|s| s == (x, y, side)))
    }

    /// The openings of a corridor at (x, y): toward each neighbour that is a
    /// corridor or a room socket facing it.
    pub fn mask(&self, x: i32, y: i32) -> Mask {
        let mut m = 0;
        for side in Side::ALL {
            let (dx, dy) = side.delta();
            let (nx, ny) = (x + dx, y + dy);
            if self.is_corridor(nx, ny) || self.socket_at(nx, ny, side.opposite()).is_some() {
                m |= side.bit();
            }
        }
        Mask(m)
    }

    /// A room's sockets that have a corridor in front of them (doors).
    pub fn doors<'a>(
        &'a self,
        room: &'a PlacedRoom,
    ) -> impl Iterator<Item = (i32, i32, Side)> + 'a {
        room.sockets().filter(move |&(x, y, side)| {
            let (dx, dy) = side.delta();
            self.is_corridor(x + dx, y + dy)
        })
    }

    /// Whether every room and corridor is reachable from the Bridge.
    pub fn connected(&self) -> bool {
        let Some(bridge) = self.rooms.iter().position(|r| r.id == RoomId::Bridge) else {
            return false;
        };
        // Nodes: rooms 0..n, then corridors n..n+m.
        let n = self.rooms.len();
        let total = n + self.corridors.len();
        let corridor_node = |x: i32, y: i32| {
            self.corridors
                .iter()
                .position(|&c| c == (x, y))
                .map(|i| n + i)
        };
        let mut seen = vec![false; total];
        let mut stack = vec![bridge];
        seen[bridge] = true;
        while let Some(node) = stack.pop() {
            let mut next = Vec::new();
            if node < n {
                for (fx, fy, _) in self.rooms[node].socket_fronts() {
                    next.extend(corridor_node(fx, fy));
                }
            } else {
                let (x, y) = self.corridors[node - n];
                for side in Side::ALL {
                    let (dx, dy) = side.delta();
                    let (nx, ny) = (x + dx, y + dy);
                    next.extend(corridor_node(nx, ny));
                    next.extend(self.socket_at(nx, ny, side.opposite()));
                }
            }
            for m in next {
                if !seen[m] {
                    seen[m] = true;
                    stack.push(m);
                }
            }
        }
        seen.into_iter().all(|s| s)
    }

    /// Whether `piece` can go at (x, y) (a room's north-west cell), wallet
    /// aside: inside the hull, on empty cells, not built twice, connected.
    pub fn can_place(&self, piece: Piece, x: i32, y: i32) -> Result<(), PlaceError> {
        let (w, d) = piece.footprint();
        if !in_hull(x, y) || !in_hull(x + w - 1, y + d - 1) {
            return Err(PlaceError::OutsideHull);
        }
        let blocked = (y..y + d).any(|cy| (x..x + w).any(|cx| self.cell(cx, cy) != Cell::Empty));
        if blocked {
            return Err(PlaceError::Blocked);
        }
        let connects = match piece {
            Piece::Corridor => Side::ALL.iter().any(|&side| {
                let (dx, dy) = side.delta();
                let (nx, ny) = (x + dx, y + dy);
                self.is_corridor(nx, ny) || self.socket_at(nx, ny, side.opposite()).is_some()
            }),
            Piece::Room(id) => {
                if id.cost().is_none() || id == RoomId::Unknown {
                    return Err(PlaceError::AlreadyBuilt);
                }
                if self.has_room(id) {
                    return Err(PlaceError::AlreadyBuilt);
                }
                PlacedRoom { id, x, y }
                    .socket_fronts()
                    .any(|(fx, fy, _)| self.is_corridor(fx, fy))
            }
        };
        if connects {
            Ok(())
        } else {
            Err(PlaceError::MustConnect)
        }
    }

    /// Places `piece` (already checked with [`Self::can_place`]).
    pub fn place(&mut self, piece: Piece, x: i32, y: i32) {
        match piece {
            Piece::Corridor => self.corridors.push((x, y)),
            Piece::Room(id) => self.rooms.push(PlacedRoom { id, x, y }),
        }
        self.normalize();
    }

    /// Whether the piece on cell (x, y) is one of the starting pieces.
    fn is_starting(&self, cell: Cell, x: i32, y: i32) -> bool {
        match cell {
            Cell::Corridor => STARTING_CORRIDORS.contains(&(x, y)),
            Cell::Room(i) => STARTING_ROOMS.contains(&self.rooms[i]),
            Cell::Empty => false,
        }
    }

    /// The layout without the piece on cell (x, y), and that piece.
    fn without(&self, x: i32, y: i32) -> Option<(CarrierLayout, Piece)> {
        let mut rest = self.clone();
        let piece = match self.cell(x, y) {
            Cell::Empty => return None,
            Cell::Corridor => {
                rest.corridors.retain(|&c| c != (x, y));
                Piece::Corridor
            }
            Cell::Room(i) => Piece::Room(rest.rooms.remove(i).id),
        };
        Some((rest, piece))
    }

    /// Whether the piece on cell (x, y) can be demolished: player-built and
    /// nothing gets cut off without it. Returns the piece.
    pub fn can_demolish(&self, x: i32, y: i32) -> Result<Piece, DemolishError> {
        let cell = self.cell(x, y);
        if cell == Cell::Empty {
            return Err(DemolishError::Nothing);
        }
        if self.is_starting(cell, x, y) {
            return Err(DemolishError::Starting);
        }
        let (rest, piece) = self.without(x, y).ok_or(DemolishError::Nothing)?;
        if !rest.connected() {
            return Err(DemolishError::WouldDisconnect);
        }
        Ok(piece)
    }

    /// Removes the piece on (x, y) (already checked with
    /// [`Self::can_demolish`]) and returns it.
    pub fn demolish(&mut self, x: i32, y: i32) -> Option<Piece> {
        let (rest, piece) = self.without(x, y)?;
        *self = rest;
        Some(piece)
    }

    /// The top-left cell of the piece on (x, y): a room's anchor, or the
    /// corridor cell itself.
    pub fn piece_origin(&self, x: i32, y: i32) -> Option<(Piece, i32, i32)> {
        match self.cell(x, y) {
            Cell::Empty => None,
            Cell::Corridor => Some((Piece::Corridor, x, y)),
            Cell::Room(i) => {
                let r = self.rooms[i];
                Some((Piece::Room(r.id), r.x, r.y))
            }
        }
    }

    /// Empty cells next to the network: beside a corridor or in front of a
    /// free socket (the build-slot markers).
    pub fn build_slots(&self) -> Vec<(i32, i32)> {
        let mut slots = BTreeSet::new();
        for &(x, y) in &self.corridors {
            for side in Side::ALL {
                let (dx, dy) = side.delta();
                slots.insert((y + dy, x + dx));
            }
        }
        for room in &self.rooms {
            for (fx, fy, _) in room.socket_fronts() {
                slots.insert((fy, fx));
            }
        }
        slots
            .into_iter()
            .map(|(y, x)| (x, y))
            .filter(|&(x, y)| in_hull(x, y) && self.cell(x, y) == Cell::Empty)
            .collect()
    }

    /// Checks a loaded layout (§ Save › Validation on load).
    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.rooms.iter().any(|r| r.id == RoomId::Unknown) {
            return Err(LayoutError::UnknownRoom);
        }
        let mut cells = BTreeSet::new();
        for r in &self.rooms {
            for c in r.cells() {
                if !in_hull(c.0, c.1) {
                    return Err(LayoutError::OutsideHull);
                }
                if !cells.insert(c) {
                    return Err(LayoutError::Overlap);
                }
            }
        }
        for &c in &self.corridors {
            if !in_hull(c.0, c.1) {
                return Err(LayoutError::OutsideHull);
            }
            if !cells.insert(c) {
                return Err(LayoutError::Overlap);
            }
        }
        if STARTING_ROOMS.iter().any(|r| !self.rooms.contains(r))
            || STARTING_CORRIDORS
                .iter()
                .any(|c| !self.corridors.contains(c))
        {
            return Err(LayoutError::MissingStart);
        }
        for id in RoomId::BUILDABLE {
            if self.rooms.iter().filter(|r| r.id == id).count() > 1 {
                return Err(LayoutError::Duplicate);
            }
        }
        if self.rooms.iter().filter(|r| r.id.cost().is_none()).count() != STARTING_ROOMS.len() {
            return Err(LayoutError::Duplicate);
        }
        if !self.connected() {
            return Err(LayoutError::Disconnected);
        }
        Ok(())
    }

    /// What resetting this layout to the start refunds: the cost of every
    /// non-starting piece (unknown rooms refund nothing).
    pub fn refund(&self) -> Cost {
        let mut total = Cost {
            credits: 0,
            void_crystal: 0,
        };
        let mut add = |c: Cost| {
            total.credits = total.credits.saturating_add(c.credits);
            total.void_crystal = total.void_crystal.saturating_add(c.void_crystal);
        };
        for r in &self.rooms {
            if !STARTING_ROOMS.contains(r) {
                if let Some(c) = r.id.cost() {
                    add(c);
                }
            }
        }
        for c in &self.corridors {
            if !STARTING_CORRIDORS.contains(c) {
                add(CORRIDOR_COST);
            }
        }
        total
    }

    /// Where the Pilot can stand, in grid px (§ Walking and interaction).
    pub fn walkable(&self) -> Vec<Rect> {
        let mut rects = Vec::new();
        for room in &self.rooms {
            let (w, d) = room.id.footprint();
            rects.push(Rect {
                x0: room.x as f32 * CELL + WALL,
                y0: room.y as f32 * CELL + BACK_WALL,
                x1: (room.x + w) as f32 * CELL - WALL,
                y1: (room.y + d) as f32 * CELL - WALL,
            });
            for (x, y, side) in self.doors(room) {
                rects.push(match side {
                    Side::N => Rect::cell(x, y, LANE_X0, 0.0, LANE_X1, BACK_WALL),
                    Side::S => Rect::cell(x, y, LANE_X0, CELL - WALL, LANE_X1, CELL),
                    Side::E => Rect::cell(x, y, CELL - WALL, LANE_Y0, CELL, LANE_Y1),
                    Side::W => Rect::cell(x, y, 0.0, LANE_Y0, WALL, LANE_Y1),
                });
            }
        }
        for &(x, y) in &self.corridors {
            let mask = self.mask(x, y);
            rects.push(Rect::cell(x, y, LANE_X0, LANE_Y0, LANE_X1, LANE_Y1));
            for side in Side::ALL.into_iter().filter(|s| mask.has(*s)) {
                rects.push(match side {
                    Side::N => Rect::cell(x, y, LANE_X0, 0.0, LANE_X1, LANE_Y0),
                    Side::S => Rect::cell(x, y, LANE_X0, LANE_Y1, LANE_X1, CELL),
                    Side::E => Rect::cell(x, y, LANE_X1, LANE_Y0, CELL, LANE_Y1),
                    Side::W => Rect::cell(x, y, 0.0, LANE_Y0, LANE_X0, LANE_Y1),
                });
            }
        }
        rects
    }
}

/// Pilot collider radius, px.
pub const PILOT_RADIUS: f32 = 12.0;

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

/// The hull cell under grid px (x, y).
pub fn cell_at(x: f32, y: f32) -> (i32, i32) {
    ((x / CELL).floor() as i32, (y / CELL).floor() as i32)
}

/// Grid px of a point given in a room's local cells and cell-local px.
pub fn room_px(room: &PlacedRoom, local: (f32, f32)) -> (f32, f32) {
    (
        room.x as f32 * CELL + local.0,
        room.y as f32 * CELL + local.1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cost(credits: u32, void_crystal: u32) -> Cost {
        Cost {
            credits,
            void_crystal,
        }
    }

    #[test]
    fn starting_layout_masks_match_the_spec_table() {
        let l = CarrierLayout::starting();
        let masks: Vec<_> = l
            .corridors
            .iter()
            .map(|&(x, y)| ((x, y), l.mask(x, y).name()))
            .collect();
        assert_eq!(
            masks,
            [
                ((1, 3), "nes".to_string()),
                ((2, 3), "ew".to_string()),
                ((3, 3), "esw".to_string()),
                ((3, 4), "ns".to_string()),
                ((3, 5), "nsw".to_string()),
                ((3, 6), "ne".to_string()),
                ((4, 3), "nw".to_string()),
            ]
        );
        assert!(l.connected());
        assert_eq!(l.validate(), Ok(()));
    }

    #[test]
    fn starting_layout_doors_and_free_sockets() {
        let l = CarrierLayout::starting();
        let doors = |id| {
            let r = *l.room(id).unwrap();
            l.doors(&r).collect::<Vec<_>>()
        };
        assert_eq!(doors(RoomId::Bridge), [(1, 2, Side::S)]);
        assert_eq!(doors(RoomId::CrewQuarters), [(4, 2, Side::S)]);
        assert_eq!(doors(RoomId::Workshop), [(1, 4, Side::N), (2, 5, Side::E)]);
        assert_eq!(doors(RoomId::Dock), [(4, 6, Side::W)]);
        let slots = l.build_slots();
        // Bridge E and Crew Quarters W both face (3,1); Crew Quarters E
        // faces (6,1); the Dock's E door faces (8,6).
        for s in [(3, 1), (6, 1), (8, 6), (3, 7), (0, 3), (3, 2)] {
            assert!(slots.contains(&s), "{s:?} in {slots:?}");
        }
        assert!(!slots.contains(&(1, 3)));
        assert!(slots.iter().all(|&(x, y)| l.cell(x, y) == Cell::Empty));
    }

    #[test]
    fn spec_example_spots_are_legal() {
        let mut l = CarrierLayout::starting();
        // The Salvage Bay at (9,5) needs its corridor at (8,6) first.
        assert_eq!(
            l.can_place(Piece::Room(RoomId::SalvageBay), 9, 5),
            Err(PlaceError::MustConnect)
        );
        assert_eq!(l.can_place(Piece::Corridor, 8, 6), Ok(()));
        l.place(Piece::Corridor, 8, 6);
        // Opens west to the Dock's E door; east once the bay is there.
        assert_eq!(l.mask(8, 6).name(), "w");
        assert_eq!(l.can_place(Piece::Room(RoomId::SalvageBay), 9, 5), Ok(()));
        l.place(Piece::Room(RoomId::SalvageBay), 9, 5);
        assert_eq!(l.mask(8, 6).name(), "ew");
        assert!(l.connected());
        // Training Room at (7,0) off a corridor at (6,1).
        l.place(Piece::Corridor, 6, 1);
        assert_eq!(l.mask(6, 1).name(), "w");
        assert_eq!(l.can_place(Piece::Room(RoomId::TrainingRoom), 7, 0), Ok(()));
        l.place(Piece::Room(RoomId::TrainingRoom), 7, 0);
        assert_eq!(l.mask(6, 1).name(), "ew");
        assert_eq!(l.validate(), Ok(()));
    }

    #[test]
    fn every_place_reason() {
        let mut l = CarrierLayout::starting();
        assert_eq!(
            l.can_place(Piece::Corridor, -1, 3),
            Err(PlaceError::OutsideHull)
        );
        assert_eq!(
            l.can_place(Piece::Room(RoomId::SalvageBay), 11, 6),
            Err(PlaceError::OutsideHull)
        );
        assert_eq!(l.can_place(Piece::Corridor, 2, 3), Err(PlaceError::Blocked));
        assert_eq!(l.can_place(Piece::Corridor, 5, 5), Err(PlaceError::Blocked));
        assert_eq!(
            l.can_place(Piece::Room(RoomId::SalvageBay), 3, 1),
            Err(PlaceError::Blocked)
        );
        // Next to a room wall without a socket: no connection.
        assert_eq!(
            l.can_place(Piece::Corridor, 8, 4),
            Err(PlaceError::MustConnect)
        );
        assert_eq!(
            l.can_place(Piece::Corridor, 10, 0),
            Err(PlaceError::MustConnect)
        );
        // In front of a free socket: connects.
        assert_eq!(l.can_place(Piece::Corridor, 3, 1), Ok(()));
        // Beside a corridor: connects.
        assert_eq!(l.can_place(Piece::Corridor, 0, 3), Ok(()));
        l.place(Piece::Corridor, 8, 6);
        l.place(Piece::Room(RoomId::SalvageBay), 9, 5);
        l.place(Piece::Corridor, 6, 1);
        assert_eq!(
            l.can_place(Piece::Room(RoomId::SalvageBay), 7, 0),
            Err(PlaceError::AlreadyBuilt)
        );
        assert_eq!(
            l.can_place(Piece::Room(RoomId::Dock), 7, 0),
            Err(PlaceError::AlreadyBuilt)
        );
        assert_eq!(
            PlaceError::CantAfford(cost(120, 3)).to_string(),
            "Need 120 cr + 3 VC"
        );
        assert_eq!(
            PlaceError::MustConnect.to_string(),
            "Must connect to a corridor"
        );
    }

    #[test]
    fn a_room_connects_only_through_a_socket() {
        let mut l = CarrierLayout::starting();
        l.place(Piece::Corridor, 8, 6);
        // (9,6) would put the bay's W socket (0,1) at (9,7) facing (8,7):
        // no corridor there, and its N socket faces (9,5): empty.
        assert_eq!(
            l.can_place(Piece::Room(RoomId::SalvageBay), 9, 6),
            Err(PlaceError::MustConnect)
        );
        // (9,3): sockets face (9,2), (11,3), (10,5), (8,4): none a corridor,
        // even though (8,6) is near.
        assert_eq!(
            l.can_place(Piece::Room(RoomId::SalvageBay), 9, 3),
            Err(PlaceError::MustConnect)
        );
    }

    #[test]
    fn demolish_refuses_starting_pieces_and_cutting_things_off() {
        let mut l = CarrierLayout::starting();
        assert_eq!(l.can_demolish(3, 4), Err(DemolishError::Starting));
        assert_eq!(l.can_demolish(5, 5), Err(DemolishError::Starting));
        assert_eq!(l.can_demolish(10, 0), Err(DemolishError::Nothing));
        l.place(Piece::Corridor, 8, 6);
        l.place(Piece::Room(RoomId::SalvageBay), 9, 5);
        // The corridor holds the bay on.
        assert_eq!(l.can_demolish(8, 6), Err(DemolishError::WouldDisconnect));
        assert_eq!(
            DemolishError::WouldDisconnect.to_string(),
            "Something would be cut off"
        );
        // Any cell of the room demolishes the room.
        assert_eq!(l.can_demolish(10, 6), Ok(Piece::Room(RoomId::SalvageBay)));
        assert_eq!(l.demolish(10, 6), Some(Piece::Room(RoomId::SalvageBay)));
        assert_eq!(l.can_demolish(8, 6), Ok(Piece::Corridor));
        l.demolish(8, 6);
        assert_eq!(l, CarrierLayout::starting());
    }

    #[test]
    fn a_dead_end_corridor_chain_demolishes_from_the_end() {
        let mut l = CarrierLayout::starting();
        l.place(Piece::Corridor, 3, 7);
        l.place(Piece::Corridor, 4, 7);
        assert_eq!(l.can_demolish(3, 7), Err(DemolishError::WouldDisconnect));
        assert_eq!(l.can_demolish(4, 7), Ok(Piece::Corridor));
    }

    #[test]
    fn validation_catches_every_broken_layout() {
        let start = CarrierLayout::starting();
        let with = |f: &dyn Fn(&mut CarrierLayout)| {
            let mut l = start.clone();
            f(&mut l);
            l.validate()
        };
        assert_eq!(
            with(&|l| l.rooms.push(PlacedRoom {
                id: RoomId::Unknown,
                x: 9,
                y: 0
            })),
            Err(LayoutError::UnknownRoom)
        );
        assert_eq!(
            with(&|l| l.corridors.push((12, 0))),
            Err(LayoutError::OutsideHull)
        );
        assert_eq!(
            with(&|l| l.rooms.push(PlacedRoom {
                id: RoomId::SalvageBay,
                x: 4,
                y: 4
            })),
            Err(LayoutError::Overlap)
        );
        assert_eq!(
            with(&|l| l.corridors.retain(|&c| c != (2, 3))),
            Err(LayoutError::MissingStart)
        );
        assert_eq!(
            with(&|l| {
                l.corridors.extend([(8, 6), (8, 5), (8, 4), (8, 3)]);
                l.rooms.push(PlacedRoom {
                    id: RoomId::SalvageBay,
                    x: 9,
                    y: 5,
                });
                l.rooms.push(PlacedRoom {
                    id: RoomId::SalvageBay,
                    x: 9,
                    y: 2,
                });
            }),
            Err(LayoutError::Duplicate)
        );
        assert_eq!(
            with(&|l| l.corridors.push((10, 0))),
            Err(LayoutError::Disconnected)
        );
    }

    #[test]
    fn refund_counts_non_starting_pieces_only() {
        let mut l = CarrierLayout::starting();
        assert_eq!(l.refund(), cost(0, 0));
        l.corridors.push((8, 6));
        l.rooms.push(PlacedRoom {
            id: RoomId::SalvageBay,
            x: 9,
            y: 5,
        });
        l.rooms.push(PlacedRoom {
            id: RoomId::Unknown,
            x: 0,
            y: 0,
        });
        assert_eq!(l.refund(), cost(130, 3));
    }

    #[test]
    fn layout_serializes_like_the_spec() {
        let mut l = CarrierLayout::starting();
        l.place(Piece::Corridor, 8, 6);
        l.place(Piece::Room(RoomId::SalvageBay), 9, 5);
        let json = serde_json::to_string(&l).unwrap();
        assert_eq!(
            json,
            r#"{"rooms":[{"id":"bridge","x":0,"y":1},{"id":"crew_quarters","x":4,"y":1},{"id":"dock","x":4,"y":4},{"id":"salvage_bay","x":9,"y":5},{"id":"workshop","x":0,"y":4}],"corridors":[[1,3],[2,3],[3,3],[3,4],[3,5],[3,6],[4,3],[8,6]]}"#
        );
        let back: CarrierLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(back, l);
        let odd: CarrierLayout =
            serde_json::from_str(r#"{"rooms":[{"id":"hangar_bay","x":1,"y":1}],"corridors":[]}"#)
                .unwrap();
        assert_eq!(odd.rooms[0].id, RoomId::Unknown);
    }

    /// Every cell of the network can be walked to from the Bridge spawn.
    #[test]
    fn the_pilot_can_walk_everywhere_in_the_network() {
        let mut l = CarrierLayout::starting();
        l.place(Piece::Corridor, 8, 6);
        l.place(Piece::Room(RoomId::SalvageBay), 9, 5);
        let rects = l.walkable();
        // Flood fill on an 8 px lattice with 8 px steps.
        let step = 8.0;
        let start = (1.5 * CELL, 1.5 * CELL + 16.0);
        assert!(can_stand(&rects, start.0, start.1));
        let key = |p: (f32, f32)| ((p.0 / step).round() as i32, (p.1 / step).round() as i32);
        let mut seen = BTreeSet::from([key(start)]);
        let mut stack = vec![start];
        let mut cells = BTreeSet::new();
        while let Some(p) = stack.pop() {
            cells.insert(cell_at(p.0, p.1));
            for d in [(step, 0.0), (-step, 0.0), (0.0, step), (0.0, -step)] {
                let q = (p.0 + d.0, p.1 + d.1);
                if can_stand(&rects, q.0, q.1) && seen.insert(key(q)) {
                    stack.push(q);
                }
            }
        }
        for &(x, y) in &l.corridors {
            assert!(cells.contains(&(x, y)), "corridor {x},{y} unreachable");
        }
        for r in &l.rooms {
            assert!(cells.contains(&(r.x, r.y + 1)), "{:?} unreachable", r.id);
        }
        // Never outside the network.
        assert!(cells.iter().all(|&(x, y)| l.cell(x, y) != Cell::Empty));
    }

    #[test]
    fn walls_stop_walking_and_the_pilot_slides_along_them() {
        let l = CarrierLayout::starting();
        let rects = l.walkable();
        // In the Bridge, walking north into the back wall.
        let p = (1.5 * CELL, 1.0 * CELL + BACK_WALL + PILOT_RADIUS);
        assert_eq!(walk(&rects, p, (0.0, -5.0)), p);
        // Diagonally into it: slides east.
        assert_eq!(walk(&rects, p, (4.0, -4.0)), (p.0 + 4.0, p.1));
    }
}
