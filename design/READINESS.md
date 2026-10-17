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
  around the carrier interior, talks to crew and uses work stations.
  3/4 top-down (2.5D) tile grid of rooms, furnished with stations and
  furniture (Revision 3, TAKOAI-88, approved 2026-10-17; Revision 1's
  pre-drawn rooms and corridors are retired). Starting stations: Command
  Console, **Upgrade Bench** (spend credits and Void Crystal on upgrades),
  Sample Analyzer, and one **Berth** per battleship (pick a ship, hear the
  briefing, launch).
- **Carrier building:** from anywhere aboard (B), ONI-style, the player
  builds Small (6 × 4) and Large (13 × 9) rooms that open into their
  neighbours by themselves, and places, moves and removes stations and
  furniture, all facing S. Building takes a few seconds. Two stations have
  a real effect (Salvage Sorter, Combat Simulator); 19 furniture and decor
  pieces are cosmetic.
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
- Manufacturing stations, module unlocking, room bonuses, furniture
  comfort, growing the hull itself (the demo builds inside a fixed hull).
- Hub, trading, factions, player-to-player anything.
- Multiplayer missions — the design keeps them; the code just mustn't rule
  them out.
- Procedural levels, other objective types, story beyond a title card.

## Demo Spec

_Defaults written by Mika for the demo — Wei can tune any of them; none
blocks the build. Units match `crates/sim/src/tuning.rs`: px, px/second,
seconds. Percent bonuses are integer math, rounded down._

### Carrier

_Revision 3 (TAKOAI-88), **approved by Wei on 2026-10-17**. Wei's direction
(2026-10-12): stop drawing whole rooms; draw lots of furniture and work
stations and let the player place them freely. His answers on 2026-10-17:
six stations; building takes a few seconds as a placeholder; everything
faces S; rooms come back as **two buildable shells, 6 × 4 and 13 × 9**, that
open into each other by themselves; one deck. Revision 1's pre-drawn rooms
and corridors (TAKOAI-52/56/60) and Revision 2's painted floor tiles
(2026-10-12) are both retired; their specs are in git history. Until the
follow-up issues land, the code still runs Revision 1. Mechanics:
design/DESIGN.md § Carrier. Wei set the decisions above; every number below
is a default Mika chose, and Wei can tune any of them._

#### Grid and view

| Item | Value |
|---|---|
| View | 3/4 top-down, orthographic, axis-aligned square grid, light from the top left (unchanged) |
| Tile | **64 × 64 world px** (art authored at 2×: 128 × 128). The Pilot is about 70 px tall, so about 1.1 tiles |
| Hull | **36 × 22 tiles** (2304 × 1408 world px), one deck. Tile (0,0) is the north-west corner; x grows east, y grows south. Tiles outside every room are **bare hull**: drawn, not walkable, free for new rooms. Fixed size in the demo |
| Tile kinds | **hull**, **wall**, **floor** or **doorway**, all derived from the rooms (§ Rooms). Nothing is painted tile by tile |
| Wall look | auto-tiled from a 4-bit mask of which neighbours are wall. A wall tile with floor directly south shows its **48 px face** in the lower part of its own tile under a 16 px cap; any other wall shows only its top |
| Draw order | hull and floor, rugs, wall faces, wall-mounted items, then objects and characters sorted by the y of their feet (front edge) |
| Zoom (screen px per world px) | **Overview 0.5** (the whole hull fits 1280 × 720) · **Normal 1.0** (about 20 × 11 tiles on screen) · Close 1.5 |
| Camera | follows the Pilot, clamped so it never shows more than 2 tiles past the hull. Build mode: Overview, pan with WASD / arrows at 600 screen px/s |
| Behind the hull | the procedural nebula sky shared with the battle (TAKOAI-58) |

#### Walking and interaction

| Item | Value |
|---|---|
| Keys | unchanged from Revision 1 (TAKOAI-60/87): left click or press-and-hold on the floor walks there; left click on a crew member, a station or a docked ship walks up to it and uses it on arrival; WASD / arrows walk in 8 directions; E interacts; mouse wheel zooms; Esc pauses. **B** opens Build mode |
| Speed, collider, Pilot size, Pilot art, panels | unchanged (160 px/s; 12 px circle at the feet; about 70 px tall; 8 directions; every panel clickable) |
| Walkable | every floor and doorway tile of a **finished** room that no **object**-layer item covers. A room under construction isn't walkable and its doorways stay shut |
| Pathing | the same A\* over an 8 px lattice, string-pulled (`find_path`), fed by walkable rectangles built from the tiles |
| Use point | 24 px south of a station's front edge, at the middle of its front strip. A click on a station walks there and uses it |
| Interact | the nearest use point or crew member within 56 px of the Pilot's feet: "E Use" / "E Talk" / "E Board" |
| Crew posts | the Gunner stands at the Command Console, the Engineer at the Upgrade Bench and the Researcher at the Sample Analyzer: on the first walkable tile east of the post's front strip, else west of it, else south of its first tile. Recomputed after every edit, so crew follow their station when it moves |
| Spawn | new game: the Command Console's use point · after a mission: the use point of the launched ship's berth |
| HUD | Revision 1's room label goes. Rooms have no names in the demo |

#### Rooms

A room is a rectangle of floor with a one-tile wall ring around it. The
player builds rooms; walls and doorways come with them.

| ID | Name | Floor | With walls | Cost | Build time |
|---|---|---|---|---|---|
| `small_room` | Small Room | **6 × 4** | 8 × 6 | 60 cr + 1 VC | 5 s |
| `large_room` | Large Room | **13 × 9** | 15 × 11 | 220 cr + 4 VC | 8 s |

- **Placing:** the room's anchor is its north-west wall tile. The whole
  room, walls included, must lie inside the hull. It may overlap other
  rooms only on wall tiles, which then become a **shared wall**. Its floor
  can't cover another room's tiles.
- **Connecting:** a new room must share a straight wall with at least one
  existing room, with at least 2 floor tiles facing each other across it.
  Every such shared wall gets **one doorway** automatically: 2 tiles wide, at
  the middle of the facing stretch (rounded north or west). There are no
  doors or gateways to build. Rooms that touch only at a corner or along
  less than 2 tiles don't connect, and that alone isn't enough to place one.
- **Doorway clearance:** the floor tile on each side of every doorway tile
  stays free of objects (rugs are fine). Placing a room whose new doorway
  would land on an object or a wall-mounted item is refused: "Clear the
  doorway first", and the blocking items flash.
- **Removing:** only an empty room (no item on its floor or on its walls'
  faces), only one the player built, and only if every other room still
  connects to the room holding the Command Console. Refunds in full. Rooms
  don't move; to move one, remove it and build it again.
- **Room types and bonuses:** none in the demo. A room is just space; what
  makes it a bunk room or a lab is the furniture the player puts in it.
  Labels and ONI-style room bonuses are deferred to TAKOAI-4 and need no
  save change.
- **More decks:** deferred (design/OPEN_QUESTIONS.md, ship size extension). The
  demo has one deck.

#### Items

- An item is a catalogue ID and an anchor tile (the north-west tile of its
  footprint). **Every item faces S** (toward the viewer), so there is no
  rotation and each item is drawn once.
- **Footprint:** w × d tiles (1 × 1, 2 × 1, 1 × 2, 2 × 2, 4 × 4).
- **Layers.** Each tile holds at most one item per layer:
  - **object**: stations and most furniture. Blocks walking.
  - **floor**: rugs. Walkable; an object may stand on top of it.
  - **wall**: wall-mounted decor (screens, lights, racks). Hangs on a wall
    face. Never blocks.
- **Placement rules**, one per catalogue item:
  - **Free:** every footprint tile is floor of a finished room (not a
    doorway, not a doorway's clearance tile for objects) and the item's
    layer is empty there. A footprint may span a doorway's two rooms only
    if it is on the floor layer.
  - **Against a wall:** free, and every tile directly north of the
    footprint is wall (not a doorway). Since everything faces S, these items
    stand along a room's north wall.
  - **On a wall:** every footprint tile is a wall tile with floor directly
    south of it (a visible face, so a north wall) and an empty wall layer.
- **Front strip:** every station has one, the row of tiles directly south of
  its footprint along its full width. These tiles must be floor or doorway
  and free of objects, and at least one of them must be reachable. No object
  may be placed on any station's front strip.
- **Reachability:** the root is the Command Console's front strip. Every
  station's front strip must be reachable from the root by walking
  (4-neighbour over walkable tiles). An edit that breaks this is refused:
  "Would cut off the Upgrade Bench".

#### Item catalogue

**Work stations** have a gameplay function. Every Revision 1 room maps to a
station. Stations with a fixed count can be moved but not removed or bought.
Six stations is the batch (Wei, 2026-10-17); more would be new content.

| ID | Name | Footprint | Placement | Function in the demo | Count | Cost | Replaces (Rev. 1) |
|---|---|---|---|---|---|---|---|
| `command_console` | Command Console | 2 × 1 | against a wall | the root for reachability, the spawn point, the Gunner's post. Not usable yet (a star map / mission board later) | exactly 1 | starter | Bridge |
| `upgrade_bench` | Upgrade Bench | 2 × 1 | against a wall | E opens the Workshop upgrades (Hull Plating, Weapon Tuning, Thruster Tuning, unchanged). The Engineer's post | exactly 1 | starter | Workshop bench (minus the Build tab) |
| `berth` | Berth | 4 × 4 | free | holds one battleship, drawn with its combat sprite, nose north, longest side 150 px (Kite) / 210 px (Bulwark). E or a click boards it → briefing → launch (TAKOAI-54). Its front strip is the row south of the pad | one per battleship (2) | starter | Dock |
| `sample_analyzer` | Sample Analyzer | 2 × 1 | against a wall | the Researcher's post. No function in the demo (the Lab Module's research, TAKOAI-8) | exactly 1 | starter | Crew Quarters (as the Researcher's spot) |
| `salvage_sorter` | Salvage Sorter | 2 × 2 | free | **+25% Credits and Void Crystal** from every successful mission (the Salvage Bay effect, unchanged numbers). Result line "Salvage Sorter +45 cr +1 VC" | buy once | 120 cr + 3 VC | Salvage Bay |
| `combat_simulator` | Combat Simulator | 2 × 2 | free | **−15% Q and W cooldowns** on every battleship (the Training Room effect, unchanged) | buy once | 200 cr + 6 VC | Training Room |

Deferred stations, placeholders only (no catalogue entry, no art yet):
Fabricator and other manufacturing stations (Manufacturing Module, TAKOAI-8),
Research Bench for studying samples and tech documents (Lab Module, TAKOAI-8),
Mission Board (hub and factions), crew needs (beds that matter, Medbay pod),
power and life support (the ONI webs), and extra berths for a third
battleship. The catalogue takes them without a model change.

**Furniture and decor** are cosmetic only in the demo. A comfort or morale
stat is deferred to TAKOAI-4 (economy). All cost Credits only, so they never
compete with upgrades for Void Crystal.

| ID | Name | Footprint | Placement | Layer | Cost |
|---|---|---|---|---|---|
| `bunk_bed` | Bunk bed | 1 × 2 | against a wall | object | 40 cr |
| `locker` | Locker | 1 × 1 | against a wall | object | 20 cr |
| `table` | Table | 2 × 1 | free | object | 25 cr |
| `stool` | Stool | 1 × 1 | free | object | 10 cr |
| `sofa` | Sofa | 2 × 1 | against a wall | object | 40 cr |
| `cat_bed` | Cat bed | 1 × 1 | free | object | 15 cr |
| `cat_tree` | Cat tree | 1 × 1 | free | object | 30 cr |
| `scratch_post` | Scratching post | 1 × 1 | free | object | 10 cr |
| `food_bowls` | Food and water bowls | 1 × 1 | free | object | 10 cr |
| `crate_stack` | Crate stack | 1 × 1 | free | object | 10 cr |
| `snack_dispenser` | Snack dispenser | 1 × 1 | against a wall | object | 30 cr |
| `aquarium` | Aquarium | 2 × 1 | against a wall | object | 60 cr |
| `potted_plant` | Potted plant | 1 × 1 | free | object | 15 cr |
| `hydro_planter` | Hydroponic planter | 2 × 1 | free | object | 35 cr |
| `floor_lamp` | Floor lamp | 1 × 1 | free | object | 15 cr |
| `rug` | Rug | 2 × 2 | free | floor | 20 cr |
| `wall_screen` | Wall screen (star map) | 2 × 1 | on a wall | wall | 25 cr |
| `wall_light` | Wall light | 1 × 1 | on a wall | wall | 10 cr |
| `tool_rack` | Tool rack | 1 × 1 | on a wall | wall | 15 cr |

#### Cost, refunds and unlocks

- Building pays the full cost up front. **Removing refunds it in full**,
  including cancelling something still under construction. Moving an item
  is free and instant.
- **Starter pieces refund nothing**, so the starting carrier can't be sold:
  starter rooms can't be removed at all, and starter items carry a
  `starter` flag in the save (it stays when they move).
- **Unlocks:** both room sizes and every catalogue item are available from
  the start. The only locks are affordability, free hull space, the
  buy-once stations ("Already built") and the fixed-count stations.
  Unlocking through Ship Modules and crew skill trees comes with TAKOAI-4 /
  TAKOAI-8.
- **Pricing:** a new game starts at 0 cr. One successful run (about 180 cr +
  6 VC) buys a Small Room and furnishes it (5–6 pieces), or buys the
  Salvage Sorter, or one Workshop upgrade. A Large Room is about one and a
  half runs.

#### Construction time

Building takes time, with short placeholder values while we test (Wei,
2026-10-17). One `build_seconds` value per catalogue entry, in the
catalogue data:

| What | Build time |
|---|---|
| Small Room | 5 s |
| Large Room | 8 s |
| Station | 4 s |
| Furniture and decor | 2 s |

- Placing creates a **construction site** at once: the cost is paid, the
  tiles are taken, and the item or room shows as its sprite in the kit's
  blueprint tint with a progress bar. Nobody has to walk there; the timer
  just runs. Any number of sites build at the same time.
- A room under construction draws its floor and walls with a scaffold
  overlay, isn't walkable, keeps its new doorways shut and takes no items
  until it finishes. An item under construction already blocks its tiles
  (object layer) but can't be used, and its effect (Salvage Sorter, Combat
  Simulator) isn't on yet.
- Timers run only while the Carrier scene is open and unpaused. They are
  saved, so a site resumes after a mission or a reload.
- Remove on a site cancels it with a full refund. A site can't be moved.
- Crew carrying out construction (ONI-style jobs) comes with companion task
  assignment.

#### Building (Build mode)

| Step | Rule |
|---|---|
| Open | **B**, or the Build button on the Carrier HUD, from anywhere aboard. No station is needed. The Upgrade Bench loses Revision 1's Build tab |
| Screen | the ONI-style builder kit (TAKOAI-65/66) along the bottom edge. **Category bar:** Rooms (Small Room, Large Room), Stations, Furniture, Decor (rugs and wall-mounted pieces), then two tools: **Move** and **Remove**. **Flyout:** one card per entry, its sprite as the icon (a room card shows its outline at scale), name, cost chip and build time, greyed with the reason when locked. **Detail panel:** name, size, placement rule ("Against a wall", "On a wall"), effect, cost, build time, lock, last feedback, Back button. 1–9 pick cards |
| Room ghost | the room's outline, walls and floor, snapped to the grid, tinted with the kit's valid / invalid swatch; the doorways it would open are marked; one line with the cost or the reason. The mouse tile is the room's middle floor tile (rounded north-west) |
| Item ghost | the item's real sprite, tinted the same way; its footprint outlined and its front strip marked with chevrons on the row south of it; one line with the cost or the reason. The mouse tile is the footprint's middle tile (rounded north-west) |
| Place | left click pays and starts construction, and saves. The entry stays picked for another copy; a buy-once station returns to the flyout |
| Move | items only. Click a finished item to lift it: it becomes the ghost, and its own tiles count as free while held. Click to drop it (free, instant). Right click or Esc puts it back. A berth carries its ship |
| Remove | click an item, a site or an empty room to remove it and refund. Fixed-count stations can't be removed ("Move it instead") |
| Grid | the hull's tiles show the kit's grid overlay in Build mode. No build-slot markers: any bare hull is room space and any free floor is item space |
| Cancel | right click or Esc drops the picked entry or tool; Esc again leaves Build mode |

Reasons, shown on the ghost and in the error tooltip: "Outside the hull" ·
"Overlaps a room" · "Must share a wall with a room" · "Clear the doorway
first" · "Room isn't empty" · "Would cut off a room" · "Needs floor" ·
"Still being built" · "Blocked" · "Keep the doorway clear" · "Needs a wall
behind it" · "Needs a wall face" · "Keep the front clear" (on another
station's front strip, or its own strip blocked) · "Would cut off the
<station>" · "The Pilot is in the way" (an object over the Pilot's
feet) · "Need 120 cr + 3 VC" · "Already built" · "Move it instead" ·
"Part of the starting carrier".

#### Starting layout

Five rooms (four small, one large) and six doorways, in the north-west of
the hull, leaving the east and south free to grow. The names are for this
doc only; the game stores rooms by size and position. Mock:
`design/art/carrier-furniture/mockup-starter-carrier.png`
(`make-starter-mock.mjs` draws it and checks the layout against the rules
above, including the autoplay's expansion).

```
    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1   (x 22–35 bare hull)
y 0 # # # # # # # # # # # # # # # # # # # # # #
y 1 # p . C C . f # . b b . l c # p . A A h h #
y 2 # . . . . . . + . b b . . . + . . . . . . #
y 3 # . . . . . . + . . t t . . + . . . . . . #
y 4 # . . . . . . # . . . . . . # . . . . . . #
y 5 # # # + + # # # # # + + # # # # # + + # # #
y 6 # . . . . . . . . . . . . . # U U . . . . #
y 7 # . . . . . . . . . . . . . + . . . . . . #
y 8 # . B B B B . . B B B B . . + . . . . . . #
y 9 # . B B B B . . B B B B . . # . . . . x x #
y10 # . B B B B . . B B B B . . # # # # # # # #
y11 # . B B B B . . B B B B . . #
y12 # . . . . . . . . . . . . . #
y13 # . . . . . . . . . . . . . #
y14 # . . . . . . . . . . . . . #
y15 # # # # # # # # # # # # # # #                 (y 16–21 bare hull)
# wall · . floor · + doorway · C Command Console · U Upgrade Bench ·
A Sample Analyzer · B berth · b bunk bed · l locker · t table · c cat bed ·
p potted plant · h hydroponic planter · f floor lamp · x crate stack
```

| Room | Size, anchor | Items (anchor) |
|---|---|---|
| Bridge | small (0,0) | Command Console (3,1); wall screen (5,0); potted plant (1,1); floor lamp (6,1). Gunner at (5,2) |
| Quarters | small (7,0) | bunk beds (9,1) and (10,1); locker (12,1); cat bed (13,1); rug (10,3); table (10,3) |
| Lab | small (14,0) | Sample Analyzer (17,1); potted plant (15,1); hydroponic planter (19,1). Researcher at (19,2) |
| Hangar | large (0,5) | berth Kite (2,8) and berth Bulwark (8,8), front strips on row 12; wall lights (6,5) and (13,5) |
| Workshop | small (14,5) | Upgrade Bench (15,6); tool rack (20,5); crate stacks (19,9) and (20,9). Engineer at (17,7) |

Doorways: (7,2)–(7,3) Bridge–Quarters · (14,2)–(14,3) Quarters–Lab ·
(3,5)–(4,5) Bridge–Hangar · (10,5)–(11,5) Quarters–Hangar · (17,5)–(18,5)
Lab–Workshop · (14,7)–(14,8) Hangar–Workshop. Everything is starter, so
none of it refunds.

Example edits for autoplay and tests: a **Small Room** at (14,10), south of
the Workshop, opens doorways (17,10)–(18,10) to the Workshop and
(14,12)–(14,13) to the Hangar; the **Salvage Sorter** at (19,11) inside it
(front strip (19,13)–(20,13)); the **Combat Simulator** at (12,9) in the
Hangar (front strip (12,11)–(13,11)).

#### Dock and berths

- One berth per battleship; the demo has two. Each berth item stores its
  ship (`"ship": "kite"`), so berths can move and the save still knows which
  ship is where. Validation needs exactly one berth per owned ship.
- The ship's sprite, scale, highlight ring for the selected ship and the
  briefing → launch flow are unchanged from Revision 1. The berth's hotspot
  is its use point.
- A third battleship later gets a buyable extra berth. Not in the demo.

#### Save (version 5)

```json
"carrier": {
  "rooms": [
    {"size": "small", "x": 0, "y": 0, "starter": true},
    {"size": "large", "x": 0, "y": 5, "starter": true},
    {"size": "small", "x": 14, "y": 10, "build_left_ms": 3200}
  ],
  "items": [
    {"id": "command_console", "x": 3, "y": 1, "starter": true},
    {"id": "berth", "x": 2, "y": 8, "ship": "kite", "starter": true},
    {"id": "salvage_sorter", "x": 19, "y": 11, "build_left_ms": 4000}
  ]
}
```

- Tiles and doorways aren't saved: they are rebuilt from the rooms. Rooms
  are sorted by (y, x), items by (y, x, layer, id), so the same layout
  always writes the same bytes. `starter` defaults to false, `ship` and
  `build_left_ms` to none (omitted when default; none means finished).
- **Migration from v1–v4:** the old `carrier` field (rooms and corridors) is
  read once, the full cost of every player-built room and corridor in it is
  refunded using Revision 1's prices (Salvage Bay 120 cr + 3 VC, Training
  Room 200 cr + 6 VC, corridor 10 cr; starting pieces nothing), and the
  carrier becomes the starting layout above. Credits, Void Crystal,
  upgrades, hints, stats and the last ship picked are kept. A one-time hint
  explains it (§ Tutorial hints, `carrier_refit`). The version is stamped to
  5. Revision 1's room prices live only in this migration.
- **Validation on load:** only known IDs and room sizes; every room inside
  the hull, overlapping others only on walls, and connected to the room
  holding the Command Console; every item obeys its placement rule and
  sits in a finished room; no two items share a tile on one layer; doorway
  clearance kept; every front strip clear and reachable; exactly one
  Command Console, Upgrade Bench and Sample Analyzer; exactly one berth per
  battleship with distinct ships; at most one Salvage Sorter and one Combat
  Simulator; every starter room present. If any check fails, the carrier
  resets to the starting layout and the cost of every non-starter room and
  item is refunded (unknown IDs refund nothing), with a warning in the log.

#### Art spec and first batch (`design/art/carrier-furniture/`)

Follow `design/ART_GUIDELINES.md` (TAKOAI-75): painted detail at the
battle-ship level, no text in the pixels, crew unchanged, and the alpha
bounding-box check. On top of it:

- **Angle:** the same 3/4 top-down, orthographic view as the rest of the
  carrier, light from the top left. Tops and south-facing fronts are
  visible; heights are drawn upward at full scale (1 px of height = 1 px up),
  as the 48 px wall face and the 70 px Pilot are.
- **One view per item: facing S.** Nothing rotates (Wei, 2026-10-17).
- **Scale (2×: 128 px per tile).** The Pilot is about 140 px tall at 2×.
  Reference heights at 2×: stool seat 36, bed top 44 (bunk's upper bunk
  104), table top 52, console desk 60 with screen up to 140, locker 180, cat
  tree 200, floor lamp 160, Combat Simulator 160, Salvage Sorter 170, berth
  pad 12 (flat).
- **Canvas:** width = footprint width × 128; height = footprint depth × 128
  + headroom (a multiple of 32, as much as the item rises above its
  footprint). The footprint is the bottom d × 128 px of the canvas; the
  item's ground contact stays inside it with at least a 4 px transparent
  margin; everything that rises goes into the headroom. The anchor is the
  bottom centre of the canvas (the middle of the footprint's front edge).
  File: `<id>.png`. On-a-wall items get a `w × 128` by 96 px canvas that
  sits on the wall face.
- **Whole on transparent:** draw each piece whole on a transparent canvas;
  never cut it out of a painted room or patch it. Rooms aren't drawn whole:
  they are assembled from the tiles below.
- **Tiles:** `floor-0..2.png` (128 × 128, three seamless variants of one
  plating), `wall-<mask>.png` × 16 (128 × 128, mask of wall neighbours in
  n-e-s-w order, a 96 px face in the lower part where south is open and a
  32 px cap above it), `doorway-ns.png` and `doorway-ew.png` (128 × 128, an
  open frame and threshold, no door leaf; two side by side make the 2-tile
  doorway), `hull-0..1.png` (128 × 128, dark bare-hull plating, two
  variants), `scaffold.png` (128 × 128, tileable overlay for rooms under
  construction), `front-marker.png` (a small chevron, 128 × 128, for the
  front strip in Build mode).
- **Icons:** none. Build cards show the item's sprite; room cards draw
  their outline in code.

**Art batch A (needed to ship):** the tiles above, plus the six stations
(Command Console, Upgrade Bench, Sample Analyzer, Salvage Sorter, Combat
Simulator, berth pad; one S view each), plus a painted mock of the starting
layout.
**Art batch B:** the 19 furniture and decor pieces in § Item catalogue, one
S view each.

Shipped IDs after import: `core.carrier.tile.floor.<n>`,
`core.carrier.tile.wall.<mask>`, `core.carrier.tile.doorway.<ns|ew>`,
`core.carrier.tile.hull.<n>`, `core.carrier.scaffold`,
`core.carrier.item.<id>`, `core.carrier.front_marker`.
`core.carrier.hull_floor` is retired in favour of the 64 px hull tiles.

#### Retired with Revision 1

The follow-up issues remove these outright rather than keeping them
alongside the new model.

- **Code (`client`):** in `layout.rs`, `RoomId`, `Piece::Room` /
  `Piece::Corridor`, sockets, the corridor `Mask` use, `STARTING_ROOMS` /
  `STARTING_CORRIDORS`, `CORRIDOR_COST`, `split_v3_dock`, the room and
  corridor `walkable()` rectangles, `BACK_WALL`, `LANE_*`, the 128 px
  `CELL` and the 12 × 8 hull. (The A\* pathing, `can_stand`, `walk` and
  `clear_line` stay and take the new rectangles.) In `carrier.rs`, room,
  door and corridor spawning, the room HUD label, build slots, `BUILD_TOOLS`
  and the Rooms / Corridors categories, and the Workshop panel's Build tab.
  In `workshop.rs`, the room `build_lock`. In `mission.rs` the
  `training_room_1` upgrade ID becomes `combat_simulator_1` (`sim` is
  untouched: it still sees `Upgrades::training`). `SaveGame::salvage_bonus`
  checks for a finished Salvage Sorter. The `carrier_build` /
  `carrier_can_build` hint texts and the autoplay's build steps change
  (§ Tutorial hints).
- **Shipped art:** `assets/source/core/carrier/room/*` (6),
  `door/*` (4), `corridor/*` (15), `build_slot.png`, `hull_floor.png` and
  `dock/berth.png`, their manifest entries and the `CARRIER_2_5D` rows in
  `demo_art_import`. Builder kit pieces tied to the old rooms:
  `core.ui.builder.category.corridors` and
  `core.ui.builder.module.corridor` / `salvage_bay` / `training_room`.
  `core.ui.builder.category.rooms` stays for the new Rooms category. The
  rest of the kit (panels, cards, cost chip, ghost swatches, grid overlay,
  the demolish icon reused for Remove) stays.
- **Design art:** `design/art/carrier-2_5d/` and `carrier-2_5d-v2/` stay as
  history, like every superseded concept folder.

#### Engineer scope

- **Layout core** (pure, no Bevy, `client/src/layout.rs` rewritten): rooms
  (sizes, overlap on walls only, shared-wall detection, doorways and their
  clearance), the derived tile map, the catalogue (footprint, layer,
  placement, front strip, count rule, cost, `build_seconds`), construction
  sites and their timers (advanced by an explicit `tick(ms)`, no clock),
  `can_place_room` / `can_remove_room` / `can_place` / `can_move` /
  `can_remove` with the reasons above, reachability, walkable rectangles,
  crew posts, `starting()`, `validate()` and the refund rules. Unit tests
  for every rule and reason, the starting layout, the example edits, and
  save round-trip.
- **Save v5** (`save.rs`): the new `carrier` field, the v1–v4 migration with
  refunds, validation and reset. Tests for each old version and for a
  broken layout.
- **Scene and Build mode** (`carrier.rs`): draw hull, floor, walls (mask),
  doorways, scaffolds and items by layer and y; construction sites with
  progress bars; stations as hotspots; crew at their posts; berths with
  their ships; B opens Build mode with Rooms, Stations, Furniture, Decor,
  Move and Remove; hints; autoplay (new game → mission → build a Small Room
  at (14,10) → wait for it → place a Salvage Sorter at (19,11) → wait →
  second mission → Continue, with the layout still there).

Out of scope: room types and bonuses, comfort, more decks, crew building
things, unlock trees, hull growth, manufacturing.

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
| **Pilot** (main character) — briefing at a berth | "Elimination run. Void monsters are nesting in the next segment." (serious) · "Forty kills clears it. They keep pouring out of void fissures." (normal) · "Grab whatever they drop — the Workshop runs on it." (excited) |
| **Gunner** — Command Console | "Swarmers rush you. Keep moving and let the guns work." (serious) · "Spitters hang back. Their shots are slow — sidestep them." (normal) · Reaction — success: "Clean shooting out there." (happy) / failed: "We lost the hull, not the crew. Again." (angry) |
| **Researcher** — Sample Analyzer | "Void Crystal hums when you hold it. I'd love a proper lab." (excited) · "Kite or Bulwark? Speed or armor — both are valid." (confused) · Reaction — success: "Fascinating samples! Well, crystals." (happy) / failed: "We'll learn from it… next time." (sad) |
| **Engineer** — Upgrade Bench | "Bring me credits and crystals and I'll make her sing." (happy) · "Plating, weapons, thrusters — pick one." (normal) · Reaction — success: "Haul's in. Let's upgrade." (excited) / failed: "Nothing came back with you. Not even scrap." (sleepy) |

Before the first mission, the reaction line is skipped.

### Tutorial hints

Shown once each, then recorded in the save.

| Trigger | Hint |
|---|---|
| First Carrier load | "WASD to walk. E to talk or use." |
| First time near a berth | "Pick a battleship here to start a mission." |
| First battle start | "Click to move. Your guns fire on their own." |
| First enemy in range | "Q / W use skills. Watch the cooldowns." |
| First loot drop | "Fly over drops to collect them." |
| First return with loot | "Spend credits and Void Crystal at the Upgrade Bench." |
| First Carrier arrival with ≥ 120 cr + 3 VC and no Salvage Sorter yet (`carrier_can_build`) | "You can afford a Salvage Sorter. Press B to build." |
| First time in Build mode (`carrier_build`) | "Pick a room or an item, then click to build. Rooms open into their neighbours. Keep the front of a station clear." |
| First Carrier load after a v1–v4 save migrated (`carrier_refit`) | "The carrier has been refitted. Everything you built was refunded. Press B to rebuild it your way." |

### Save file

Saved on every return to the Carrier and every Workshop purchase. Holds:
credits, Void Crystal, upgrade levels, last mission result (none / success /
failed), missions played and won, tutorial hints seen, last battleship picked,
and (version 5) the carrier tiles and items — see § Carrier › Save. Also
saved after every place, move and remove, and when a construction site
finishes. Mission loot is never saved mid-battle.

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
