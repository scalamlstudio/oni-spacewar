# Readiness

_What "the design is solid enough to start building" means for Oni Spacewar.
Questions themselves — including which ones block this scope and the order to
decide them in — live only in design/OPEN_QUESTIONS.md; this doc doesn't copy
them. Task tracking lives on the Multica board (First Playable Demo: TAKOAI-36)._

## First Playable Scope

_**Confirmed by Wei on 2026-10-03** (logged in design/PROGRESS.md). Mechanics
live in design/DESIGN.md; the numbers live in § Demo Spec below._

The **First Playable Demo** is one complete, repeatable loop:

**Title → Carrier → Dock (pick battleship + briefing) → Battle (Elimination) →
Result → back to Carrier → spend rewards at the Workshop → go again.**

It is single-player and offline. Battle logic still lives in the deterministic
`sim` crate (no Bevy, no clock, no floats) and never assumes there is only one
battleship in a mission instance, so multiplayer missions stay possible later.
Carrier, result, menus and save live in `client`.

**In scope**

- **Title screen:** New Game / Continue / Quit.
- **Carrier scene:** the player walks the main character (the Pilot) freely
  around the carrier interior, talks to crew and enters modules to use them.
  3/4 top-down (2.5D) grid of rooms joined by corridors (Revision 1,
  2026-10-04). Starting rooms: Bridge, Crew Quarters, **Workshop** (spend
  credits and Void Crystal on upgrades, and build) and **Dock** (pick a
  battleship from its berth, hear the briefing, launch).
- **Carrier building:** at the Workshop the player builds corridors and 2
  rooms with a real effect (Salvage Bay, Training Room), dungeon-builder
  style, so the carrier grows between missions.
- **Crew dialogue:** 2–3 short lines per crew member, each with one of the 11
  expression portraits; the Pilot gives the briefing at the Dock; one line per
  crew member reacts to the last mission's result.
- **Battleships:** 2 with different playstyles — **Kite** (fast, fragile, short
  cooldowns) and **Bulwark** (slow, tanky, stronger basic attack). Each has 2
  skills (Q, W).
- **Battle scene:** the existing top-down click-to-move battleship, auto-firing
  its basic attack. One mission type, **Elimination** (kill 40 enemies), in
  open space with no edge, over a procedural "magical night sky with nebula"
  background, with 2 void-monster types (**Void Swarmer**, **Void Spitter**)
  pouring continuously out of 1–2 void fissures.
- **Loot:** enemies drop **Credits** and **Void Crystal**, collected by flying
  over them.
- **Battle HUD:** hull bar, skill cooldowns, kill counter, mission time and
  live enemies, loot collected this mission, and an edge arrow for every
  off-screen fissure.
- **Result scene:** Success or Failed, kills, and the loot kept or lost. On
  failure the mission's loot is lost, following each reward's "survives
  failure" flag (all off in the demo).
- **Workshop upgrades:** 3 upgrades × 2 levels, applied to every battleship.
- **Pause / quit menu** in every scene, and a **save file** (credits, Void
  Crystal, upgrade levels, last result, tutorial hints seen).
- **Tutorial hints** the first time the player does something.
- **Art:** the demo art set, shipped through the content pipeline
  (`assets/source/<pack>/…`, stable IDs, manifest) and following
  `design/ART_GUIDELINES.md`. No audio.

**Out of scope / deferred** (not rejected)

- Manufacturing Module, buildings, conveyors and production chains;
  extraordinary samples.
- Companion task assignment (ONI-style jobs) and companions coming along on
  missions (their auto-trigger skills).
- Research loop: Research points, tech documents, crew skill trees, Lab Module.
- Manufacturing buildings inside rooms, module unlocking, growing the hull
  itself (the demo builds rooms inside a fixed hull).
- Hub, trading, factions, player-to-player anything.
- Multiplayer missions — the design keeps them; the code just mustn't rule
  them out.
- Procedural levels, other objective types, story beyond a title card.

## Demo Spec

_Defaults written by Mika for the demo — Wei can tune any of them; none
blocks the build. Units match `crates/sim/src/tuning.rs`: px, px/second,
seconds. Percent bonuses are integer math, rounded down._

### Carrier

_Revision 1 (TAKOAI-52, 2026-10-04): the one-deck side-view corridor is
replaced by a 2.5D carrier the player expands. Mechanics: design/DESIGN.md
§ Carrier. Wei approved the 3/4 top-down view and building at the Workshop;
everything else below is a default Mika chose and Wei can tune._

#### Grid and view

| Item | Value |
|---|---|
| View | 3/4 top-down, orthographic, axis-aligned square grid, light from the top left |
| Cell | 128 × 128 world px (art authored at 2×: 256 × 256) |
| Hull | 12 × 8 cells (1536 × 1024 world px). Cell (0,0) is the north-west corner; x grows east, y grows south. Fixed size in the demo |
| Back (north) wall | the top 48 px of a room's top row (or of a corridor cell with no north opening) shows the wall face |
| Side walls / front lip | 12 px thick (drawn as wall tops; the front wall is cut away to a lip) |
| Door opening | 64 px wide, centred on the socket cell's edge |
| Draw order | by feet y (larger y drawn later) |
| Zoom (screen px per world px) | Overview 0.65 (the whole hull fits 1280 × 720) · **Normal 1.0** (default; about 10 × 5.6 cells on screen) · Close 1.5. Mouse wheel steps between them |
| Camera | follows the Pilot, clamped so it never shows more than 1 cell past the hull. Build mode: Overview, pan with WASD / arrows at 600 screen px/s |
| Behind the hull | the procedural nebula sky shared with the battle (TAKOAI-58) |

#### Walking and interaction

| Item | Value |
|---|---|
| Keys | **left click** on the floor walks there (the same control as in battle, TAKOAI-60), pathing along rooms, doors and corridors; a click off the floor goes to the nearest floor within 64 px; the battle's move marker shows the destination. Left click on a crew member, the Workshop bench or a docked ship walks up to it (24 px in front) and uses it on arrival. WASD or arrow keys also walk in 8 directions (diagonals normalised) and cancel a click-to-move; E interacts; mouse wheel zooms; Esc pauses. Build mode keeps its own clicks |
| Speed | 160 px/s |
| Pilot collider | circle, radius 12 px, at the sprite's feet |
| Pilot size | same on-screen height as the crew (TAKOAI-54), about 70 px |
| Walkable in a room | its footprint minus the 48 px back-wall band, the 12 px side walls and the 12 px front lip, plus a door gap at each connected socket |
| Walkable in a corridor | a 64 × 68 px centre square (cell-local x 32–96, y 48–116) plus a lane to each open side: north x 32–96, y 0–48 · south x 32–96, y 116–128 · east x 96–128, y 48–116 · west x 0–32, y 48–116 |
| Interact | the nearest hotspot within 56 px of the Pilot's feet, shown as an "E Talk" / "E Use" / "E Board" prompt; a hotspot can also be clicked |
| Spawn | new game: Bridge cell (1,1) · after a mission: Kite Dock walkway cell (5,6) |
| Pilot art | 8 directions (TAKOAI-66): the facing is the Pilot's movement (click-to-move and WASD alike) snapped to the nearest of 8. S, SE, E, NE and N are drawn (`core.carrier.pilot.<dir>.idle` + `walk_1..4`); SW, W and NW mirror SE, E and NE. Standing shows the last direction's idle frame |
| Panels | every Carrier panel also plays with the mouse alone (TAKOAI-66): a click on the dialogue box advances it and its options (Next / Launch prep / Close, Leave) are buttons; the launch confirm has Launch / Back buttons; the Workshop's tabs and rows are clickable (a row buys the upgrade or picks the piece) and it has a Close button. Keys still work, and prompts show both, e.g. `[Click / E]` |

#### Layout model

- Every hull cell is **empty**, a **corridor** cell or part of a **room**.
- A room has a rectangular footprint (w × d cells, anchored at its north-west
  cell) and a fixed list of **door sockets**: a cell on its edge plus the side
  (N, E, S, W) the door faces. Rooms do not rotate.
- A corridor is one cell. Its shape comes from a 4-bit mask: it opens toward
  each orthogonal neighbour that is a corridor cell or a room socket facing
  it. 1 opening = end, 2 = straight or corner, 3 = T, 4 = crossroad.
- A socket with a corridor in front of it is a **door**; an unused socket is
  plain wall. Rooms never connect to rooms directly.
- **Connectivity:** the network is corridor cells plus rooms, linked by
  corridor-to-corridor adjacency and corridor-to-socket doors. Everything must
  be reachable from the Bridge.

#### Building (Workshop bench → Build tab)

| Step | Rule |
|---|---|
| Open | E at the Workshop bench opens the panel with two tabs: **Upgrades** (unchanged) and **Build**. Tab switches tabs |
| Pick | the Build tab lists every piece with its footprint, effect, cost and a lock reason (already built, can't afford). Picking one closes the panel and enters Build mode |
| Build screen | ONI-style, after `design/art/demo-v2/builder-ui/` (TAKOAI-66), along the bottom edge: a **category bar** (Rooms, Corridors, Demolish; a click picks the category's first available card), a **flyout** of build cards (Corridor, Salvage Bay, Training Room, Demolish) each with its icon, name and a cost chip, greyed out with the reason when it is already built or can't be afforded, and a **detail panel** for the picked piece (effect, size, doors, cost, lock, the last action's feedback, a Back button). Click a card or press 1–4 to pick; a greyed-out card explains why instead. The carrier's real room art stays underneath; the camera may pan far enough south that the last hull row clears the panels |
| Ghost | the piece's real sprite, tinted, follows the mouse, snapped so the mouse cell is the room's north-west cell, with the kit's blue-white swatch when a click works and the red one when not, plus one line: the cost, or why not ("Must connect to a corridor", "Blocked", "Outside the hull", "Need 120 cr + 3 VC"). The reason also shows in the error tooltip (top right). Over the panels there is no ghost |
| Grid | the hull's cells show the kit's grid overlay while in Build mode |
| Slots | every empty cell next to the network (beside a corridor or in front of a free socket) shows the build-slot marker while in Build mode |
| Place | left click on a legal spot pays and places it, then saves. A corridor stays selected for the next cell; a room returns to the Build tab |
| Rotate | none. Rooms have fixed sockets; corridors shape themselves |
| Demolish | X, the Demolish card / category or the Demolish row in the Build tab picks demolish mode: clicking a player-built piece removes it for a **full refund**, unless that would disconnect anything (red, "Something would be cut off"). The starting rooms and corridors can't be demolished |
| Cancel | right click or Esc leaves Build mode back to the panel; Esc again closes it |

Placement is legal when the footprint is inside the hull, covers only empty
cells, and connects: a corridor needs an orthogonal neighbour that is a
corridor or a free socket facing it; a room needs at least one of its sockets
facing an existing corridor.

#### Room catalogue

Sockets are given as room-local cell (x,y) plus side. The four starting rooms
are fixed; the two buildable rooms can each be built once.

| Room | Footprint | Sockets | Who / what inside (cell, room-local) | Effect | Cost |
|---|---|---|---|---|---|
| **Bridge** (start) | 3 × 2 | (1,1) S · (2,0) E | Gunner at (0,1); star-map screen on the back wall above (1,0) | — | — |
| **Crew Quarters** (start) | 2 × 2 | (0,1) S · (0,0) W · (1,0) E | Researcher at (1,1); bunks along the back wall | — | — |
| **Workshop** (start) | 3 × 2 | (1,0) N · (2,1) E | Engineer at (0,1); Workshop bench at (2,0) (hotspot: Upgrades + Build) | — | — |
| **Dock** (start, × 2) | 2 × 3 | (0,2) W · (1,2) E · (0,2) S | one berth on cells (0–1, 0–1); walkway row y = 2. The west Dock holds Kite, the east one Bulwark (TAKOAI-60) | launches missions | — |
| **Salvage Bay** | 2 × 2 | (0,0) N · (1,0) E · (1,1) S · (0,1) W | scrap bins, a magnet crane | **+25% Credits and Void Crystal** from every successful mission, success bonus included (rounded down; nothing on failure). ~180 cr + 6 VC → ~225 cr + 7 VC | 120 cr + 3 VC |
| **Training Room** | 2 × 2 | (0,0) N · (1,0) E · (1,1) S · (0,1) W | a simulator pod, a target hologram | **−15% Q and W cooldowns** on every battleship (ticks × 85 / 100, rounded down). Kite Q 4 → 3.4 s, W 6 → 5.1 s; Bulwark Q 12 → 10.2 s, W 9 → 7.65 s | 200 cr + 6 VC |
| **Corridor** | 1 × 1 | shaped by neighbours | — | connects | 10 cr |

- One successful run (~180 cr + 6 VC) buys the Salvage Bay plus its corridor,
  or one Workshop upgrade. The Salvage Bay pays for itself in about three
  more successful runs; the Training Room competes with Weapon Tuning 2.
- Example spots for autoplay and tests: **Salvage Bay** at (9,5) with a corridor
  at (8,6) (the bay's W socket (0,1) faces the Dock's E door).
  **Training Room** at (7,0) with a corridor at (6,1) (off the Crew Quarters'
  E socket; the room's W socket (0,1) faces it).

#### Starting layout

Rooms (anchor = north-west cell): Bridge (0,1), Crew Quarters (4,1),
Workshop (0,4), Dock A / Kite (4,4), Dock B / Bulwark (6,4). Corridors:
(1,3), (2,3), (3,3), (4,3), (3,4), (3,5), (3,6), (4,7), (5,7), (6,7). The
two Docks fill the old two-berth Dock's footprint; the row below joins
their S doors.

```
x →   0  1  2  3  4  5  6  7  8  9 10 11
y0    .  .  .  .  .  .  .  .  .  .  .  .
y1    B  B  B  .  Q  Q  .  .  .  .  .  .
y2    B  B  B  .  Q  Q  .  .  .  .  .  .
y3    .  ├  ─  ┬  ┘  .  .  .  .  .  .  .
y4    W  W  W  │  A  A  K  K  .  .  .  .
y5    W  W  W  ┤  A  A  K  K  .  .  .  .
y6    .  .  .  └  A  A  K  K  .  .  .  .
y7    .  .  .  .  └  ─  ┘  .  .  .  .  .
B Bridge · Q Crew Quarters · W Workshop · A Dock A (Kite) · K Dock B (Bulwark)
```

| Corridor | Opens to | Shape |
|---|---|---|
| (1,3) | N Bridge door · E corridor · S Workshop door | T (`nes`) |
| (2,3) | E · W | straight (`ew`) |
| (3,3) | E · S · W | T (`esw`) |
| (4,3) | N Crew Quarters door · W | corner (`nw`) |
| (3,4) | N · S | straight (`ns`) |
| (3,5) | N · S · W Workshop door | T (`nsw`) |
| (3,6) | N · E Dock A door | corner (`ne`) |
| (4,7) | N Dock A door · E | corner (`ne`) |
| (5,7) | E · W | straight (`ew`) |
| (6,7) | N Dock B door · W | corner (`nw`) |

Free sockets to build from: Bridge E and Crew Quarters W (both face (3,1)),
Crew Quarters E (faces (6,1)), Dock B E (faces (8,6)), plus the open sides
of every corridor. (Dock A's E and Dock B's W sockets face each other and
stay walls.)

#### Dock and berths

- One Dock room per battleship, one berth each (TAKOAI-60); the demo has
  two (Kite west, Bulwark east). Docks are matched to ships in layout order
  (west to east), so the save stores no ship per Dock. Each berth is a
  2 × 2-cell pad (256 × 256 px) and each Dock shows only its own ship.
- The ship is drawn with its combat sprite (`core.battle.ship.kite` /
  `bulwark`), centred on the pad, nose north, scaled so its longest side is
  **150 px (Kite)** and **210 px (Bulwark)**.
- Hotspot: the middle of the pad's front edge (room-local px (128, 256));
  E, or a click on the ship (the Pilot walks up first), selects it and
  opens briefing → launch (TAKOAI-54). The selected ship gets a highlight ring on its pad.
- More berths: a future battleship gets its own Dock (or a **Hangar Bay**)
  when the roster grows past two. Not in the demo (no third ship).

#### Save (version 4)

The save gains one field. Starting pieces are not flagged; they are
recognised by matching the starting layout.

```json
"carrier": {
  "rooms": [
    {"id": "bridge", "x": 0, "y": 1},
    {"id": "crew_quarters", "x": 4, "y": 1},
    {"id": "workshop", "x": 0, "y": 4},
    {"id": "dock", "x": 4, "y": 4},
    {"id": "dock", "x": 6, "y": 4},
    {"id": "salvage_bay", "x": 9, "y": 5}
  ],
  "corridors": [[1,3],[2,3],[3,3],[4,3],[3,4],[3,5],[3,6],[4,7],[5,7],[6,7],[8,6]]
}
```

- Room IDs: `bridge`, `crew_quarters`, `workshop`, `dock`, `salvage_bay`,
  `training_room`. Coordinates are the anchor cell. Rooms are sorted by id and
  corridors by (x, y), so the same layout always writes the same bytes.
- **Migration:** v1 and v2 saves have no `carrier` field; it defaults to the
  starting layout (serde default). In a v3 save (TAKOAI-56) the Dock is the
  old 4 × 3 two-berth room; v4 (TAKOAI-60) replaces it in place with two
  one-berth Docks at (x, y) and (x + 2, y) and adds the starting corridors
  on the row below that join them. A player-built corridor already on one
  of those cells becomes a starting piece and is refunded (10 cr). The
  layout is then validated as below; if the two Docks don't fit or don't
  connect, the v3 rule applies (reset + refund). The version is stamped to
  4. Nothing else changes.
- **Validation on load:** the layout must have only known IDs, fit the hull,
  have no overlaps, contain every starting room and corridor in place, have at
  most one of each buildable room and be fully connected. If any check fails,
  the layout resets to the starting layout and the cost of every non-starting
  piece in it is refunded (unknown IDs refund nothing), with a warning in the
  log.

#### Art list (Stage 7, `design/art/carrier-2_5d/`; detailed set `design/art/carrier-2_5d-v2/`)

Shipped since TAKOAI-69/70: the detailed painted set in
`design/art/carrier-2_5d-v2/` (same files, canvases and grid as below;
`make-carrier-v2.mjs` builds it). The Stage 7 set in
`design/art/carrier-2_5d/` is kept as the previous version.

Style: follow `design/ART_GUIDELINES.md` for painted detail, material rendering,
crew consistency, text-free image assets and cropping checks. Every piece is
authored at 2× (256 px per cell) on a transparent background, canvas exactly
the footprint, so pieces line up on a shared 256 px grid. North walls show a
96 px face (2×) at the top of the top row; side walls and the front lip are
24 px (2×). Rooms are drawn with every socket **closed** (solid wall); the door
overlays open them.

| File | Canvas (2×) | Content |
|---|---|---|
| `room-bridge.png` | 768 × 512 | star-map screen on the back wall above cell (1,0), captain's console in the middle, two side consoles. Leave cell (0,1) clear for the Gunner |
| `room-crew-quarters.png` | 512 × 512 | two bunks along the back wall, a small table, a locker. Leave cell (1,1) clear for the Researcher |
| `room-workshop.png` | 768 × 512 | the bench with tools and a build-planning screen against the back wall at cell (2,0), a tool wall, parts crates. Leave cell (0,1) clear for the Engineer |
| `room-dock.png` | 512 × 768 | one **empty** berth pad (512 × 512, landing markings) on rows 0–1, a bay door in the back wall, walkway row 2. No ships. (The Stage 7 file was the old two-berth Dock, 1024 × 768; the v2 one-berth Dock replaces the mirrored crop TAKOAI-60 shipped) |
| `dock-berth.png` | 480 × 244 | the Dock panel's empty berth (`core.carrier.dock.berth`) |
| `room-salvage-bay.png` | 512 × 512 | scrap bins, a magnet crane, a sorting belt stub (decor only) |
| `room-training-room.png` (v1: `room-training.png`) | 512 × 512 | a simulator pod, a target hologram |
| `door-n.png` | 256 × 96 | a 128 px opening (x 64–192) in the back-wall face, with a door frame and floor threshold |
| `door-s.png` | 256 × 24 | a 128 px gap in the front lip |
| `door-e.png`, `door-w.png` | 24 × 256 | a gap in the side wall from y 96 to 232, with a threshold |
| `corridor-<mask>.png` × 15 | 256 × 256 | one per opening mask, sides named in n-e-s-w order: ends `n`, `e`, `s`, `w`; straights `ns`, `ew`; corners `ne`, `es`, `sw`, `nw`; T `nes`, `esw`, `nsw`, `new`; crossroad `nesw`. Floor lanes 128 px wide (2×) as in § Walking; a back-wall face where north is closed |
| `hull-floor.png` | 256 × 256 | an empty hull cell: dark, unlit plating, tiles seamlessly, clearly not walkable |
| `build-slot.png` | 256 × 256 | a bright dashed outline with a small "+" on transparent, readable on top of `hull-floor` |
| `mockup-starting-layout.png` | 3072 × 2048 | the starting layout above assembled from the pieces (not shipped) |

Crew are not redrawn. The Pilot's painted 8-direction frames come from
`design/art/demo-v3/pilot-8dir/` (TAKOAI-68; `make-pilot-8dir.py` writes
them to `assets/source/core/carrier/pilot/`). Shipped IDs after import:
`core.carrier.room.<id>`, `core.carrier.door.<n|e|s|w>`,
`core.carrier.corridor.<mask>`, `core.carrier.hull_floor`,
`core.carrier.build_slot`.

#### Engineer scope (Stage 8, TAKOAI-56)

- **New pure module `client/src/layout.rs`** (no Bevy): `CarrierLayout`,
  the room catalogue (footprint, sockets, cost, effect), the corridor mask,
  connectivity, `can_place` / `can_demolish` with the reasons above,
  walkable rectangles, `starting()` and `validate()`. Unit tests for the
  connectivity rules, every reason, demolish-would-disconnect, the starting
  layout's masks (table above) and the save validation.
- **`client/src/carrier.rs`:** drop `ROOM_W`, `DECK_LEN`, `room_at(x)`,
  `walk(x)` and the x-only `HOTSPOTS`. Spawn sprites from the layout (rooms,
  doors, corridors, hull floor), place crew and hotspots in 2D, 8-direction
  walking with collision against the walkable rectangles, y-sorting, the
  camera and zoom levels, and Build mode (ghost, slots, place, demolish,
  pan). The HUD's room label comes from the cell the Pilot stands in.
  Reuse the berth piece from TAKOAI-54 for the Dock.
- **`client/src/workshop.rs`:** the Build tab, piece costs and refunds
  (reuse `Cost`), and the "already built / can't afford" locks.
- **`client/src/save.rs`:** version 3, the `carrier` field, migration and
  validation as above, with tests for a v2 save migrating and a broken
  layout resetting with a refund.
- **Room effects:** Training Room → the mission's upgrade IDs get
  `training_room_1`; `mission::upgrade_levels` maps it to a new
  `sim::Upgrades::training` level that `ShipSheet::new` applies to the Q and W
  cooldowns, so the Dock's stat preview shows it and the determinism gate
  covers it. Salvage Bay → `record_mission_return` applies +25% on success,
  and the Result screen shows the bonus as its own line ("Salvage Bay +45 cr
  +1 VC").
- **Hints** (§ Tutorial hints): new text for `carrier_walk`, and two new hints,
  `carrier_build` and `carrier_can_build`.
- **Autoplay:** title → new game → mission → build the corridor at (8,6)
  and the Salvage Bay at (9,5) → second mission → quit → Continue, with
  the layout still there.

Out of scope: manufacturing, conveyors, production chains, hull growth,
companions working in rooms.

### Battleships

Both are available from the start. Stats are before upgrades.

| | Kite | Bulwark |
|---|---|---|
| Role | fast, fragile, short cooldowns | slow, tanky, stronger basic attack |
| Hull (HP) | 60 | 140 |
| Move speed | 220 px/s | 140 px/s |
| Radius | 14 px | 20 px |
| Basic attack | 4 dmg every 0.35 s, range 220 px | 8 dmg every 0.6 s, range 240 px |
| Q | **Afterburn:** dash 160 px toward the cursor. CD 4 s | **Bastion:** shield absorbs the next 40 dmg, lasts 5 s. CD 12 s |
| W | **Scatter:** 5 bolts in a 40° cone toward the cursor, 6 dmg each, 600 px/s, 0.5 s life. CD 6 s | **Shockwave:** 15 dmg to every enemy within 150 px, pushes them 100 px away. CD 9 s |

Basic attacks fire at the nearest enemy in range (design/DESIGN.md § Player).

### Enemies

| | Void Swarmer | Void Spitter |
|---|---|---|
| Behavior | rushes the nearest battleship | keeps 200–320 px from the nearest battleship; fires when it's within 360 px |
| HP | 10 | 36 |
| Move speed | 132 px/s | 72 px/s |
| Radius | 10 px | 14 px |
| Attack | contact: 5 dmg, then 1 s cooldown per swarmer | slow projectile: 8 dmg, 160 px/s, radius 5 px, 3 s life, every 2.5 s |

### Elimination mission

- **Battlefield:** no edge (TAKOAI-58). Nothing clamps battleships, enemies
  or shots; shots end by their lifetime. The starting view is the 1000 × 580
  window at the 1.265 battle zoom, ≈ 1265 × 734 px. The background is a
  procedural "magical night sky with nebula" (deep blue-violet, drifting
  magenta / teal / violet clouds, twinkling stars at 3 parallax depths), kept
  dark so ships and shots read.
- **Kill target:** 40. The HUD shows "Kills x/40". `--autoplay` QA runs use
  10 instead (shorter test battles, TAKOAI-77); `--kill-target N` overrides
  either for manual testing. Reaching the target is Success immediately: remaining enemies vanish and every pickup still on the field is
  collected.
- **Failed:** every battleship in the mission is destroyed (solo: yours). Quit
  Mission from the pause menu also counts as Failed.
- **No time limit.**

Spawn plan (Revision 2, TAKOAI-58: 3× the enemies, 20% faster; Revision 1,
TAKOAI-53, replaced the 3 fixed waves):

- **Void fissures:** 1 or 2 per mission (seeded), on a ring 900–1300 px from
  the start at seeded angles, outside the starting view (centre ≥ 130 px past
  its edge), ≥ 600 px apart. They can't be hit or destroyed, and don't
  rotate (a gentle pulse only). Every off-screen fissure has its own arrow at
  the screen edge pointing at it, in the v2 fissure's rim colours
  (violet-magenta with a teal inner edge); the arrow goes while it's on screen.
- **Spawn director:** one enemy at a time out of a random fissure, on a ring
  60 px from its centre. First spawn 2 s in. The gap between spawns falls
  from 1.33 s to 0.5 s and the Spitter share rises from 10% to 40%, both
  linearly over the first 120 s, then stay. At most 36 enemies alive at once
  (the director waits while the field is full). It keeps spawning until the
  kill target, so the player never has to hunt the last one.
- **Moving enemies:** Swarmers chase the nearest battleship. Spitters close to
  their 200–320 px band, then strafe around the battleship inside it (a random
  way round) while shooting.
- Tuned so a Kite or Bulwark with no upgrades wins in about 1–1.5 minutes
  (52–74 s over 12 seeds with the scripted QA pilot, TAKOAI-58).

### Loot

| Source | Credits | Void Crystal | Survives failure |
|---|---|---|---|
| Void Swarmer | 5 (always) | 1 (15% chance) | off |
| Void Spitter | 15 (always) | 1 (60% chance) | off |
| Mission success bonus | 50 | 2 | — (success only) |

- Drops are rolled with the sim's deterministic RNG.
- Pickups are collected when a battleship comes within 32 px; within 80 px they
  drift toward it. They disappear after 20 s.
- Expected haul from a successful run (~17 Swarmers + 3 Spitters + bonus):
  **~180 credits, ~6 Void Crystal.** A failed run keeps nothing.
- Every reward type carries a `survives_failure` flag, default off. The demo
  turns none on.

### Workshop upgrades

Apply to every battleship, in battle and in the Dock's stat preview. Priced so
one successful mission buys about one upgrade.

| Upgrade | Effect per level | Level 1 cost | Level 2 cost |
|---|---|---|---|
| Hull Plating | +25% hull (Kite 60 → 75 → 90; Bulwark 140 → 175 → 210) | 150 cr + 4 VC | 250 cr + 6 VC |
| Weapon Tuning | +25% damage, basic attack and skills (Kite basic 4 → 5 → 6; Bulwark basic 8 → 10 → 12) | 150 cr + 4 VC | 250 cr + 6 VC |
| Thruster Tuning | +15% move speed (Kite 220 → 253 → 286; Bulwark 140 → 161 → 182) | 150 cr + 4 VC | 250 cr + 6 VC |

All six levels cost 1200 credits + 30 Void Crystal (~6 successful missions). A
new game starts with 0 credits and 0 Void Crystal. Weapon Tuning 1 is the most
visible first buy: Bulwark one-shots Swarmers, Kite needs 2 hits instead of 3.

### Crew dialogue

Talking cycles through the member's lines; the reaction line comes first after
a mission. Portrait expressions are the 11 in `assets/packs/core` (normal,
happy, excited, serious, angry, sad, shocked, surprised, confused, shy,
sleepy).

| Crew | Lines (expression) |
|---|---|
| **Pilot** (main character) — Dock briefing | "Elimination run. Void monsters are nesting in the next segment." (serious) · "Forty kills clears it. They keep pouring out of void fissures." (normal) · "Grab whatever they drop — the Workshop runs on it." (excited) |
| **Gunner** — Bridge | "Swarmers rush you. Keep moving and let the guns work." (serious) · "Spitters hang back. Their shots are slow — sidestep them." (normal) · Reaction — success: "Clean shooting out there." (happy) / failed: "We lost the hull, not the crew. Again." (angry) |
| **Researcher** — Crew Quarters | "Void Crystal hums when you hold it. I'd love a proper lab." (excited) · "Kite or Bulwark? Speed or armor — both are valid." (confused) · Reaction — success: "Fascinating samples! Well, crystals." (happy) / failed: "We'll learn from it… next time." (sad) |
| **Engineer** — Workshop | "Bring me credits and crystals and I'll make her sing." (happy) · "Plating, weapons, thrusters — pick one." (normal) · Reaction — success: "Haul's in. Let's upgrade." (excited) / failed: "Nothing came back with you. Not even scrap." (sleepy) |

Before the first mission, the reaction line is skipped.

### Tutorial hints

Shown once each, then recorded in the save.

| Trigger | Hint |
|---|---|
| First Carrier load | "WASD to walk. E to talk or use." |
| First time near the Dock | "Pick a battleship here to start a mission." |
| First battle start | "Click to move. Your guns fire on their own." |
| First enemy in range | "Q / W use skills. Watch the cooldowns." |
| First loot drop | "Fly over drops to collect them." |
| First return with loot | "Spend credits and Void Crystal at the Workshop." |
| First Carrier arrival with ≥ 120 cr + 3 VC and no room built yet (`carrier_can_build`) | "You can afford a new room. Build it at the Workshop bench." |
| First time in Build mode (`carrier_build`) | "Click a glowing slot to build. Rooms need a corridor at one of their doors." |

### Save file

Saved on every return to the Carrier and every Workshop purchase. Holds:
credits, Void Crystal, upgrade levels, last mission result (none / success /
failed), missions played and won, tutorial hints seen, last battleship picked,
and (version 3) the carrier layout — see § Carrier › Save. Also saved after
every build or demolish. Mission loot is never saved mid-battle.

## Design-Done Checklist

Development on the first playable can start when all of these hold:

1. **Scope confirmed** — Wei has accepted, edited, or replaced § First Playable
   Scope above, and design/PROGRESS.md logs it. _Done 2026-10-03._
2. **Every blocking question is decided** — each design/OPEN_QUESTIONS.md
   entry tagged `Blocks first playable` or `Blocks (partial)` is reflected in the
   relevant design doc and removed from design/OPEN_QUESTIONS.md.
3. **Between-missions controls are written down** — how the player moves
   around the carrier, uses its modules, and launches a mission from it
   (design/DESIGN.md § Player, § Crew / Companions).
4. **One combat round is specified end to end** — what the player presses,
   what the battleship does, what enemies do, and how a mission is won or lost.
5. **One progression round is specified end to end** — which resources come
   back from a mission and what they buy at the Workshop. Concrete names and
   rough numbers, not categories (§ Demo Spec).
6. **Docs are consistent** — no doc contradicts a decision logged in
   design/PROGRESS.md (e.g. design/CONCEPT.md's pitch and reference games still
   describe "the ship" as both base and combat vessel, and cite FTL for combat;
   both predate the carrier/battleship split and the LoL-style control scheme).
7. **Deferred questions are explicitly deferred** — each design/OPEN_QUESTIONS.md
   entry tagged `Deferred` is acknowledged as out of the first playable, so it
   can't silently block implementation.

Not required before development: final numbers/balance, art, audio, story
beyond the opening premise, multiplayer or hub design.

_Open questions — tagged as blocking or deferred against this scope, with their
board issues: design/OPEN_QUESTIONS.md._
