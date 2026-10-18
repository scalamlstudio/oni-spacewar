# Architecture

_Code structure only — see `design/CONCEPT.md` and `design/DESIGN.md` for what the game is/plays like._

The game is a Rust/Bevy Cargo workspace. The earlier LÖVE2D/Lua prototype
(`src/`, `love.sh`) and the TAKOAI-18 netcode spike (`spike/bevy-netcode/`)
were removed in TAKOAI-24; both remain in git history (the spike's final state
is commit `b92126c`, its results are recorded on TAKOAI-18).

| path | what | status |
|---|---|---|
| `Cargo.toml`, `crates/` | Rust/Bevy workspace | foundation (TAKOAI-26) + Elimination mission (TAKOAI-41) |
| `assets/` | source and shipped content packs | modular content-patch foundation (TAKOAI-30) |
| `.github/workflows/`, `ci/` | CI: determinism gate for the sim, content checks; local P2P soak script | see § Verifying determinism, § Netcode |
| `design/` | design docs, art guidelines and concept art | source of truth for gameplay and art direction |

## Rust workspace

```
Cargo.toml            workspace; exact version pins live in [workspace.dependencies]
crates/
  sim/                package `oni-sim`, lib `sim` — deterministic mission simulation, NO Bevy
    src/lib.rs          crate docs + determinism rules
    src/state.rs        SimState (all rolled-back state), step() and its phases
    src/entity.rs       plain-data entities: Ship (+ ShipKind, Loadout, ShipSheet, ShipStats), Enemy, Projectile, Pickup, Fissure
    src/mission.rs      Elimination bookkeeping: spawn-director timer, kills, loot totals, MissionStatus/MissionOutcome
    src/tuning.rs       Demo Spec numbers (design/READINESS.md) + unit conversion (px/s -> sub-px/tick)
    src/collision.rs    integer circle overlap + separation
    src/fixed.rs        fixed-point: SUB (1 px = 256), FxVec2 (+ scale_to), mul_q16
    src/trig.rs         integer sin/cos from a committed Q16 table (from the spike)
    src/rng.rs          SimRng, seeded xorshift64* (part of SimState)
    src/input.rs        NetInput: buttons (move, Q/W/E/R) + cursor target, exchanged per tick
  content/            package `oni-content`, lib `content` — content manifests, stable IDs, patch diffing
    src/lib.rs          manifest types, hash validation, stable-ID resolution, manifest diff
    tests/asset_crop.rs crop check: no opaque pixel of a cut-out under assets/source/core/ touches its canvas edge
    tests/character_frames.rs  8-direction frame contract (canvas, feet baseline, visible height, margin) + processed == source
    tests/carrier_tiles.rs     Revision 3 carrier art: batch A IDs resolve, tiles are one cell, mask-picked walls join seamlessly
    src/bin/content_pipeline.rs        source -> processed asset + zstd bundle + manifest
    src/bin/content_manifest_diff.rs   compare manifests and list packs/assets a patch needs
    src/cutouts.rs                     background keying, sheet slicing, single-sprite crop, resize
    src/bin/sprite_sheet_cutouts.rs    expression sheets -> portraits
    src/bin/demo_art_import.rs         design/art/demo (+ demo-v2 battle art and backdrops) -> battle / title / result / icon sources
    src/bin/carrier_tile_import.rs     design/art/carrier-furniture catalogues (batches A/B, patches, hull edge) -> carrier/tile, carrier/item, scaffold, front marker sources
    hull-wall/build-hull-wall-extra.py every reachable hull-edge case Art's catalogue lacks, drawn by Art's generator -> hull-wall/extra-v1/ (PNGs + catalogue)
  client/             package `oni-client`, bin `oni-spacewar` — everything else
    src/main.rs         CLI (synctest / p2p modes), app + GGRS session setup, ICE (STUN/TURN) config
    src/art.rs          ContentImages: shipped images and fonts by stable content ID (manifest -> processed file), cached; linear sampler + CPU mip chain
    src/flow.rs         first-playable scene state machine, Title / Result UI, --autoplay QA driver: Title → Carrier → Loading → Battle → Result
    src/loading.rs      mission transition screen (GameScreen::Loading): mission name, progress bar, a tip; loads the battle's images, then Battle; error + Back to Carrier
    src/tips.rs         TIPS (transition-screen tip list) + TipDeck (random pick, no back-to-back repeat)
    src/layout.rs       carrier interior Revision 3 rules, no Bevy, no clock: rooms (Small / Large shells) → derived tiles, walls, doorways + clearance; item catalogue; construction sites (`tick(ms)`); can_place_room / can_remove_room / can_place / can_move / can_remove with the spec's reasons; reachability, crew posts, walkable rects, validation, refunds; click-to-move pathing (A*)
    src/carrier.rs      Carrier scene (Revision 3): draws the rooms' tiles and the items by layer, construction sites with progress bars, click-to-move / hold-to-drag + 8-direction WASD walking, crew at their posts, stations as hotspots, berths with their ships (board → briefing → launch), camera / zoom, Workshop upgrades panel, Build mode (B: Rooms / Stations / Furniture / Decor, Move, Remove; builder UI kit), clickable panels
    src/workshop.rs     Workshop rules: upgrade costs / levels / buy(), and Build mode's wallet side: build_room / build_item (pay up front) / remove_room / remove_item (full refund)
    src/save.rs         versioned JSON save data in the OS data directory (override: ONI_SAVE_DIR) + migration
    src/settings.rs     player settings (settings.json beside the save, not in it), the Settings screen and gear button, carrier floor / wall colour presets + contrast check, CarrierPalette (recoloured tile images)
    src/codex.rs        the Codex (from Settings): content browser built from `core.codex` (assets/source/core/codex.json) — category tabs, scrolling thumbnail grid, detail panel; stats read from the catalogue / sim tuning; `discovered` hook
    src/mission.rs      typed client mission config/result handoff into `sim::SimState`; reward rules (survives_failure)
    src/rollback.rs     SimWorld resource, rollback/checksum registration, GgrsSchedule
    src/input.rs        mouse + keyboard / bot → NetInput (ReadInputs)
    src/net.rs          matchbox ↔ ggrs socket adapter + latency/loss emulator (from the spike)
    src/stats.rs        rollback / frame-time / desync measurement + report (from the spike)
    src/pacing.rs       frame-pacing profiler (windowed): main / render / swapchain-acquire split + OS-stall probe
    src/render.rs       battle sprites keyed by stable content IDs + gizmo FX, fissure pointers, camera follow, battle HUD (Update, outside rollback)
    src/zoom.rs         mouse-wheel zoom for the local camera (Carrier and battle): 0.5×–2× target, eased, about the cursor; follow offset kept on screen; picking maths (`world_at`)
    src/sky.rs          procedural nebula sky: SkyMaterial (Material2d, `sky.wgsl`) on a quad that follows the camera; one sky behind every scene
```

Dependency direction is one-way for gameplay: `client → sim`, while content
tooling is a sibling crate used only by the client and tools. `sim` depends only
on `serde`, so it cannot reach assets, the engine, the clock, the renderer or
the network.

## Modular content packs

Shipped game content lives under `assets/` and is split into independently
versioned packs. Source art and design exploration remain in `design/art/`,
with binding art rules in `design/ART_GUIDELINES.md`; that tree is never loaded
directly by the game and is not rewritten by the content pipeline. When art is
ready to ship, an import step copies or exports it into
`assets/source/<pack>/...`, then the pipeline writes processed files and
compressed bundles under `assets/packs/<pack>/...`.

```
assets/
  manifest.json                 versioned manifest for every shipped pack
  source/
    core/...                    editable pack inputs tracked in git
  packs/
    core/
      processed/...             files the client can load by stable asset ID
      bundles/...               compressed patch/download payloads
```

`crates/content` owns the manifest schema and stable-ID lookup. Code must refer
to content as IDs such as `core.ui.hud_status`, not as source or processed file
paths. The first shipped asset is the HUD status text:

```
assets/source/core/ui/hud_status.txt
  -> assets/packs/core/processed/ui/hud_status.txt
  -> assets/packs/core/bundles/ui/hud_status.txt.zst
  -> assets/manifest.json asset id core.ui.hud_status
```

The client loads that processed asset through `ContentManifest::load_text()` and
shows it in the HUD. This proves the path is manifest-driven without putting
asset IO inside rollback.

### Manifest and stable IDs

`assets/manifest.json` is schema-versioned and lists:

- each pack id, semantic-ish pack version, `gameplay_affecting` flag and pack
  content hash;
- each asset's stable ID, kind, source path, processed path, compressed payload
  path, per-asset content hash and `gameplay_affecting` flag;
- the sim/content compatibility version that peers can compare before a
  mission starts.

The pack hash is derived from stable IDs, asset hashes, processed paths,
compressed paths and gameplay-affecting flags. It is not derived from local
filesystem metadata, so two peers with identical content bytes produce the same
manifest identity regardless of checkout path.

### Patch model

Content-only patches update `assets/manifest.json` plus only changed compressed
pack payloads/chunks. `content-manifest-diff OLD NEW` compares two manifests and
prints the packs and stable asset IDs a patch needs, for example:

```sh
cargo run --bin content-manifest-diff -- old_manifest.json assets/manifest.json
```

Code patches ship a new binary and may also ship content packs. A mission lobby
must reject peers unless all players have the same sim binary/protocol version
and the same hashes for every gameplay-affecting pack or asset. Cosmetic-only
packs can differ outside a mission if the client has fallbacks, but any asset or
data table that changes simulation inputs, timing, hitboxes, tuning, spawn
tables, level collision, enemy behaviour or ability rules is
`gameplay_affecting = true` and must match exactly for every peer in the same
rollback session.

`crates/sim` still does not read assets from disk at runtime. If future gameplay
data is content-authored, the client will validate matching manifest hashes in
the lobby and pass a deterministic, already-parsed data value into `SimState`
construction. That value then becomes rolled-back/checksummed state like any
other gameplay input.

### Compression and build-time processing

The target shipped formats are:

- GPU textures: KTX2/Basis Universal, so texture transcodes happen for the
  player's GPU format instead of shipping raw PNGs. Trade-off: slower/offline
  build processing and occasional quality tuning, but much smaller installs and
  less VRAM/upload cost.
- Audio: Ogg Vorbis or Opus for music/voice/ambience, with short latency-critical
  SFX allowed as processed WAV only when profiling shows the decode cost is not
  worth it. Trade-off: compressed audio saves install/update size but adds
  decode work and looping metadata must be tested.
- Pack payloads: zstd-compressed bundles/chunks under `assets/packs/*/bundles`.
  Trade-off: zstd is CPU-cheap and patches well, but the client needs a bundle
  index before random access to large packs.
- Bevy asset processing: use Bevy's asset processor/import settings for
  build-time conversion once the art volume justifies it. The checked-in
  `content-pipeline` is deliberately small: it copies every PNG under
  `assets/source/core/` (stable ID from its path, e.g.
  `battle/ship/kite.png` -> `core.battle.ship.kite`), every TTF font the
  same way (`fonts/russo_one.ttf` -> `core.fonts.russo_one`, kind
  `font/ttf`, licence text next to it), every JSON data file the same way
  (`codex.json` -> `core.codex`, kind `application/json`) plus the HUD
  text, and writes the zstd bundles and manifest. Images ship as PNG for now.

Run the current pipeline after editing pack source files:

```sh
cargo run --bin content-pipeline -- .
```

The pipeline writes processed assets, `.zst` payloads and a validated manifest.
Generated shipped assets are committed because this repo currently has no
external asset CDN; later release automation can upload the bundle directory and
leave only source plus manifest in git if install size becomes a problem.

### Expression portrait cutouts

Expression sprite sheets from `design/art/expressions/*-expression-sheet-v1.png`
are imported with a deterministic Rust tool:

```sh
cargo run --bin sprite-sheet-cutouts -- .
cargo run --bin sprite-sheet-cutouts -- . \
  --extra-sheet-suffix normal-serious-v1 \
  --extra-expressions normal,serious
cargo run --bin content-pipeline -- .
```

The cutout tool samples the flat sheet background from the image border, removes
only border-connected background pixels with a soft alpha edge, finds connected
foreground regions, splits touching poses on the lowest-density vertical seam,
reattaches small floating expression marks to the nearest pose, drops the text
label row, and writes normalized transparent PNGs to
`assets/source/core/portraits/<character>/<expression>.png`. Each character's
nine expressions share a fixed canvas and baseline anchor so swapping portraits
does not jitter.

The expected sheet format is one row of nine poses in this order: Angry, Happy,
Sleepy, Confused, Shocked, Excited, Sad, Surprised, Shy. Sheets need a flat
solid background connected to the border, clear gaps between poses, and no text
labels in the pose area; labels below the ground line are ignored during import.
Additional expression sheets can be imported with `--extra-sheet-suffix` and a
comma-separated `--extra-expressions` list. Extra sheets are extracted with the
same per-character canvas and baseline computed from that character's default
nine-pose sheet. Extra poses that exceed that target layout are scaled down to
fit the established character canvas and baseline; the tool still fails if the
detected pose count differs from the provided expression count.

### Demo art import

The First Playable art set (`design/art/demo/*-v1.png`, with the restyled
battle art from `design/art/demo-v2/` replacing the Swarmer, Spitter, Spitter
shot and loot icons under the same stable IDs, plus the void fissure, and
transparent text-free scene art from `design/art/demo-v3/`) is imported
with:

```sh
cargo run --release --bin demo-art-import -- .
cargo run --release --bin content-pipeline -- .
```

`demo-art-import` keys out each image's flat background (art that already
has a transparent background is only cropped), cuts the icon sheet (5 icons) with the same
sheet slicer as the portraits, scales everything down to its in-game size
(single sprites get a `SPRITE_MARGIN` transparent border after scaling, so
nothing touches the canvas edge) and writes `assets/source/core/{battle,carrier,title,ui/icon}/...`. Restyled
pieces from `design/art/demo-v2/` are rows in its `SPRITES` table. (The
Revision 1 carrier's rooms, doors, corridors, hull floor and build slot were
retired with their `CARRIER_2_5D` rows in TAKOAI-91; the carrier's art now
comes from `carrier-tile-import`, below.) The scene backdrops (`BACKDROPS`) are the transparent demo-v3 title
key art and Result backdrop (`core.title.key_art`, `core.result.backdrop`),
shipped at 2000 × 1160 so they stay crisp up to that window size while the
runtime nebula shader shows through. They follow `design/ART_GUIDELINES.md`:
no painted sky, no in-image text, concept crew only when characters appear, and
composition that leaves calm areas for UI text. The
import also writes `target/demo-art-contact-sheet.png` for a visual check. The
file names and target sizes are tables at the top of the tool; a new art file
means a new table row.

### Carrier Revision 3 art import

Art batches A (TAKOAI-89) and B (TAKOAI-92) for the room-and-item carrier
are imported with:

```sh
cargo run --release --bin carrier-tile-import -- .
cargo run --release --bin content-pipeline -- .
```

`carrier-tile-import` reads the catalogues in its `CATALOGUES` table, in
order: Art's `design/art/carrier-furniture/batch-a-import-v3.json` and
`batch-b-import-v1.json` (selected file, stable ID, destination, size,
alpha bounds; batch B adds footprint, headroom, anchor, layer and
placement); patch catalogues (`orientation-import-patch-v2.json`,
TAKOAI-99, then `orthographic-import-patch-v1.json`, TAKOAI-104, the
top-down redraw), whose rows each redraw a row an earlier catalogue has (same
ID and canvas, a footprint row keeps footprint, headroom, anchor and
layer; anything else is an error) and replace it; and the hull-edge
catalogues (`outer-wall-import-v1.json`, TAKOAI-101, and the generated
`crates/content/hull-wall/extra-v1/catalogue.json`, rows under
`tiles`). It checks each row (the ID decides the destination, the path
the pipeline derives it from; a catalogue `source` that disagrees is
reported and overridden, as Art's hull rows put their tiles in a
`hull_wall/` folder their IDs don't have; size and alpha bounds must match the file, the
alpha bounds counting the pixels above each batch's own threshold: any
alpha for A, the patches and the hull tiles, above 0.02 for B as Art's inspector does; a batch B canvas
must be its footprint's width × 128, and 96 px tall for a wall piece or
depth × 128 plus a multiple-of-32 headroom otherwise, anchored bottom
centre) and copies the file unchanged: the art is already transparent and
authored at 2×. It ships
`core.carrier.tile.floor.<0..2>`, `core.carrier.tile.wall.<nesw>` (16
masks, digits in n-e-s-w order), `core.carrier.tile.doorway.<ns|ew>`,
`core.carrier.item.<id>` for all 25 catalogue entries (six stations, 19
furniture and decor pieces), `core.carrier.scaffold`,
`core.carrier.front_marker` and the 38 hull-edge walls
`core.carrier.tile.hull_wall_<nesw>_x<n e s w ne se sw nw>`. Nothing is
drawn outside the rooms: space shows through (TAKOAI-94). A new Art batch
or patch means a new catalogue file and a new `CATALOGUES` row there.

**Hull-edge tiles (TAKOAI-101, TAKOAI-102).** Art ships the 17 starter
cases; a player-built layout reaches 38. `build-hull-wall-extra.py`
(Python 3 + Pillow) runs Art's deterministic `build-outer-wall-v1.py` on a
scratch copy of its folder (design/art/ is never written), enumerates the
reachable keys (every set of Small and Large rooms round a wall tile that
overlap only on walls, doorways included: a superset of valid layouts)
and renders the 21 missing ones with Art's `render()` into
`crates/content/hull-wall/extra-v1/`. `layout::reachable_hull_keys` (test
only) enumerates the same from the Rust side, and
`every_reachable_hull_case_has_a_shipped_tile` requires the shipped set to
equal it, so a new room size or doorway rule fails until the tiles are
regenerated; `random_layouts_resolve_every_hull_wall_to_a_shipped_tile`
fuzzes real `can_place_room` layouts on top. Every item draws as its art, bottom centre at
the middle of its footprint's front edge, at 1 world px per 2 art px;
there is no placeholder. `carrier::tests::catalogue_art_fits_its_footprint`
checks every `layout::CATALOGUE` entry against its shipped canvas, so a
new catalogue entry needs its art in the same change.

`layout::TileMap::art(tile)` is the one place that picks a tile's image
(`TileMap::floor_art` the floor drawn on or under it), and
`carrier::deck` the one place that turns them into sprites, so a later
recolour (TAKOAI-96) hooks in there:
walls by `wall_mask` (wall neighbours only; a doorway ends a wall, so the
wall beside it shows its end trim and jamb); a wall with space round it
(`exterior_mask`: n e s w ne se sw nw outside every room, a diagonal
cleared when a cardinal beside it is set) by both masks as its hull-edge
tile, which replaces the inner wall and has the floor in it, so
`floor_art` is `None` there and `deck` draws that tile alone; floors cycling the three
variants by `x + y`, doorways by axis (`ew` in a north–south wall), hull
none. `crates/content/tests/carrier_tiles.rs` checks that any two
mask-picked walls that join share their edge pixels (cap always, face when
both show one), apart from the rim of an outer edge one of them exposes at
a corner, so a run of wall tiles draws as one wall.

**Crop check.** `crates/content/tests/asset_crop.rs` (part of `cargo test`)
loads every PNG under `assets/source/core/` and fails if a pixel with
alpha > 8 touches the canvas edge (design/ART_GUIDELINES.md § Cropping).
Art allowed to touch its canvas edge is listed by path in `FULL_BLEED`: the
transparent title and Result scene art, the Revision 3 floor / wall / doorway tiles and scaffold,
and the builder UI's 9-slice panels, module cards and grid overlay. Real
clipped cut-outs waiting for an art fix go in `KNOWN_CLIPPED`, which fails
once the asset is fixed so the entry is removed.

**Character frame contract.** `crates/content/tests/character_frames.rs`
finds every `<character>.<dir>.<frame>` image in the manifest (`<dir>` one of
the 8 compass directions, at least 3 facings) and checks, per character, that
all frames share one canvas size, the feet (lowest row with alpha > 8) sit
within 2 px of the median baseline, the visible height is within 4 % of the
median (so the head is the same size facing away as facing the camera) and
nothing touches the canvas edge. The client relies on this: `standing()`
in `carrier.rs` sizes and anchors a character once, from its S idle frame,
and `animate_pilot` only swaps the image and `flip_x`. Frames waiting for an
art fix go in `KNOWN_BAD`, which fails once they pass. The same file checks
that every shipped PNG's processed copy and manifest hash match its source,
so a source edit without a `content-pipeline` re-run fails instead of
shipping the old art. `.github/workflows/content.yml` runs `oni-content`'s
tests on any `assets/**` or `crates/content/**` change.

**Sampling.** `ContentImages` loads images with `ImageSampler::linear()`
(linear min / mag / mip) and appends a full mip chain built on the CPU
(2×2 box filter on premultiplied alpha, so the black RGB of transparent
pixels doesn't darken edges). Painted sprites are drawn 2–3× smaller than
their canvas on a 1× display (the Pilot's 192 px canvas is ~72 px at
Carrier zoom 1, ~36 px at 0.5); without mips that skips texels and outlines
shimmer while walking. Level 0 stays first in `Image::data`, so CPU readers
of the full-size pixels (e.g. `empty_rows`) are unaffected.

### How the sim plugs into rollback

- The whole simulation is one value, `sim::SimState`. The client wraps it in the
  `SimWorld` resource and registers it with bevy_ggrs by clone (save/load) and
  by `Hash` (checksum). Any new gameplay state must be a field of `SimState` (or
  something it owns), which makes it rolled back and checksummed automatically.
- One system in `GgrsSchedule` collects `PlayerInputs` in handle order and calls
  `SimState::step(&inputs)`. Nothing else mutates `SimWorld`.
- Rendering, HUD, input reading and stats run in normal Bevy schedules and only
  read `SimWorld`.
- Multiplayer is the default shape: `SimState::ships` holds one ship per player
  handle (1..=4, `MAX_PLAYERS`); single-player is `num_players = 1`.
- The first-playable single-player flow launches missions through
  `client::mission::MissionConfig`, which carries a list of ship loadouts even
  when that list currently has one entry, plus the mission's `kill_target`
  (`KILL_TARGET` = 40 in normal play, `AUTOPLAY_KILL_TARGET` = 10 under
  `--autoplay`, or `--kill-target N`; the run's value is the `flow::KillTarget`
  resource). `launch_sim` puts it into the sim's start state with
  `SimState::with_kill_target`; it lives in `Mission::kill_target`, so it is
  rolled back, hashed and identical on every peer, and the sim, HUD and Result
  screen all read it from there. The demo battle path steps the same
  `SimWorld` resource at a fixed rate without opening a network session; the
  `synctest` and `p2p` modes still use `RollbackPlugin` and GGRS.
- **Carrier interior, Revision 3 core** (`layout.rs`, TAKOAI-90;
  design/READINESS.md § Carrier). Pure client data, no Bevy and no clock.
  `layout::Carrier` (the save's `carrier`) is `rooms` (size + north-west wall
  anchor, `starter`, `build_left_ms`) and `items` (catalogue `ItemId` +
  north-west footprint anchor, `ship` for berths, `starter`,
  `build_left_ms`), kept sorted (`normalize`). Nothing else is stored:
  `TileMap::new` derives hull / wall / floor / doorway tiles from the rooms
  (`TileMap::art` names each one's content ID, § Carrier Revision 3 art import),
  one 2-tile doorway per shared straight wall with ≥ 2 facing floor tiles
  (`doorway_between`), its clearance tiles, and which doorways are open (both
  rooms finished). `CATALOGUE` holds every item's footprint, layer (object /
  floor / wall), placement (free / against a wall / on a wall), category,
  count rule, cost and `build_seconds`; everything faces S. The checks
  (`can_place_room`, `can_remove_room`, `can_place`, `can_move`,
  `can_remove`) return a `Reason` whose `Display` is the spec's text; each
  applies the edit to a clone and re-checks reachability (4-neighbour flood
  from the Command Console's front strip over walkable tiles: floor of a
  finished room or an open doorway, no object) and, for rooms, room-graph
  connectivity. Edits (`place_room`, `place`, `move_item`, ...) don't check
  and return the new index after re-sorting. `tick(ms)` advances every
  construction site. `walkable()` merges walkable tiles into rectangles for
  the same `find_path` / `walk` / `can_stand` pathing; `crew_post` and
  `Item::use_point` place crew and use points. `validate()` is the load-time
  check; `refund()` is what a reset gives back. The pathing lives here too.
  Floats appear only in the
  pixel helpers; the rules are integer tile math (this is client code, not
  `sim`).
- **The Carrier scene** (`carrier.rs`, TAKOAI-91; design/READINESS.md
  § Carrier). Plain Bevy in `Update`, gated on `GameScreen::Carrier`; it
  owns no sim state and draws the save's `layout::Carrier`; grid y grows
  south and the scene flips it (`carrier::world` / `grid`). **Drawing:**
  `deck(carrier)` lists the sprites back to front: every room tile gets its
  floor (`TileMap::floor_art`), and every wall and doorway tile its
  `TileMap::art` on top of that floor, so wall art that leaves floor
  showing drops in unchanged; a doorway that is still shut (one of its
  rooms under construction) is dimmed; rooms being built get the scaffold
  over every tile. Items follow by layer: rugs and wall-mounted pieces on
  fixed layers over the floor and walls, objects y-sorted with the
  characters by their front edge (`depth`); each item's art is pinned at
  its bottom centre, the middle of its footprint's front edge. Sites are
  drawn in the blueprint tint with a progress bar (`SiteBar`,
  `update_site_bars`). Hull tiles draw nothing: space (the shared sky)
  shows there. `sync_layout` respawns the deck when the layout's *shape*
  changes (`drawn_shape`: a site counts as a site however long it has
  left, so ticking timers don't redraw every frame; a site finishing does),
  and recomputes the walkable rects (`Carrier::walkable`) and hotspots.
  **Construction timers:** `run_sites` advances `Carrier::tick` by whole
  ms of frame time while the Carrier is open and unpaused (client code, so
  frame time is fine) and saves when a site finishes.
  **Scene:** `hotspots` gives the crew at their posts (`Carrier::crew_post`,
  recomputed after every edit, so crew follow their station) and every
  finished station's use point: the Upgrade Bench (Workshop upgrades) and
  one per berth (`Hotspot::Berth(ship)`, from the berth's `ship`). The
  Command Console and Sample Analyzer are posts only in the demo. The Pilot
  spawns at the Command Console's use point, or after a mission at the
  launched ship's berth (`spawn_point`). **Walking** is unchanged from
  Revision 1 (TAKOAI-60/87), now over the tile rectangles: `find_path`
  runs A* over an 8 px lattice of standable points and string-pulls the
  route (`clear_line`); `carrier::click_to_move` turns a left click into a
  `WalkOrder` (`order_walk`: the floor point, or the `APPROACH` spot in
  front of a clicked ship / crew member / station with the hotspot to use
  on arrival); `carrier_input` follows it (`follow`, through `walk`) and
  interacts on arrival when in reach. Hold-to-move reads the button through
  `input::MovePress` like the battle: a press on the floor starts a
  `PilotDrag` that re-paths to the cursor every `DRAG_REPATH` px and stops
  inside `DRAG_DEAD_ZONE` (`drag_step`); presses over UI nodes are ignored.
  WASD cancels the order. The destination is drawn with the battle's move
  marker. Characters are scaled by their visible pixels (`standing`) to
  `CHARACTER_H` = 56 px (0.875 tile) on a 24 px footprint
  (`layout::PILOT_RADIUS`, capped at `MAX_FOOTPRINT_TILES` = 0.6 tile) and
  anchored at their visible feet. The Pilot faces 8 directions
  (`Facing::of`; S, SE, E, NE, N drawn, SW, W, NW mirrored, `Facing::art`;
  standing keeps the last facing's idle frame, `pilot_frame`). Every panel
  part carries a `Click` component that `carrier_input` reads alongside the
  keys. The camera follows the Pilot, clamped to 2 tiles past the hull
  (`clamp_camera`); the wheel zooms 0.5×–2× about the cursor
  (`crate::zoom`). Deck labels sit on a dark backing (`label`), so they
  read over any floor art. Pure helpers (`deck`, `hotspots`,
  `nearest_hotspot`, `clicked_hotspot`, `spawn_point`, `interact_with`,
  `ship_at`, `apply_build_click`, `ghost`, `entry_card`, `crew_lines`,
  `ship_card`, `upgrade_preview`) hold the rules and are unit tested.
- **Build mode** (`Overlay::Build`, § Building). **B** or the HUD's Build
  button opens it from anywhere aboard (B or Esc with nothing picked
  closes it); the Upgrade Bench has no Build tab any more. The camera goes
  to Overview (`OVERVIEW`, 0.5) and WASD pans it; the wheel still zooms,
  and the cursor tile is picked through the camera (`track_build_cursor`),
  so ghosts and clicks work at every zoom. The screen (`spawn_build_ui`,
  the builder kit) is the category bar (`BuildCategory`: Rooms, Stations,
  Furniture, Decor, then the Move and Remove tools), the flyout of cards
  (`BuildCategory::entries`, catalogue order; an item card shows its
  sprite, a room card its outline at scale; name, cost chip, build time;
  greyed with the reason, `entry_lock`: fixed-count stations, a buy-once
  station already on board, the price) and the detail panel (`entry_card`:
  size, placement rule, effect, cost, build time, lock, last feedback,
  Back). 1–9 pick cards. The panels carry `Interaction`, so deck clicks
  ignore the mouse over them. `ghost` is what shows at the cursor: a room's
  wall-and-floor outline anchored so the mouse is its middle floor tile
  (`Room::anchor_for`), with the doorways it would open marked; an item's
  sprite with its footprint centred on the mouse (`item_anchor`) and its
  front strip marked with the front-marker chevrons; tinted with the kit's
  valid / invalid swatch, with one line over it: the cost and build time
  or the `layout::Reason`. Items that block a new doorway flash. A left
  click runs `apply_build_click`: Place pays and starts a construction
  site (`workshop::build_room` / `build_item`; the entry stays picked, a
  buy-once station goes back to the flyout), Move lifts a finished item
  (hidden while held; its own tiles count as free) and puts it down free
  and instant (a berth carries its ship), Remove takes an item, a site or
  an empty room for a full refund (`workshop::remove_item` /
  `remove_room`); every change saves. Right click / Esc puts a held item
  back or drops the pick (`back_out`). `build_grid` shows the kit's grid
  overlay on every hull tile while Build mode is open.
- **Station effects.** Combat Simulator: once finished, `mission::upgrade_ids`
  adds `combat_simulator_1` to the mission's upgrade IDs, which
  `upgrade_levels` maps to `sim::Upgrades::training` (the sim is untouched);
  `ShipSheet::new` applies × 85 / 100 (integer) to the Q / W cooldowns
  (`tuning::trained`), the ship keeps them in `ShipStats`, and the launch
  card / Workshop preview read the same sheet. The determinism gate's
  fleet gives handles 2–3 `training`. Salvage Sorter: once finished,
  `SaveGame::salvage_bonus` (+25% of what a successful mission kept,
  rounded down) is added in `record_mission_return` and shown as its own
  Result line ("Salvage Sorter +45 cr +1 VC"). A site has no effect yet.
- **Berths.** One `berth` item per battleship (§ Dock and berths); each
  stores its ship, so berths can move. `docked_ship` draws the ship's
  battle sprite on the pad, nose north, its longest side
  `ShipInfo::berth_len` (`DockedShip`). The ship is the interaction: E at
  the berth's use point or a click on the ship calls `interact_with`,
  which writes it to `selected_battleship`, plays the Pilot's briefing and
  ends in the launch confirm panel for that ship. The selected ship gets a
  highlight ring and "Selected" in its berth's label; the other is dimmed.
  Launch writes a `mission::MissionRequest`; the flow stores the picked
  battleship in the save and enters the transition screen, then Battle, where `config_from_save` builds
  the `MissionConfig`.
- **Ship numbers have one source.** `sim::ShipSheet::new(loadout)` gives a
  battleship's stats after upgrades in human units; `ShipStats` (what the
  battle uses) is derived from it, and the Dock card and Workshop preview
  read the same sheet with the save's upgrade levels, so what the player
  reads is what the battle does.
- **Workshop** (`workshop.rs` rules, `carrier.rs` panel): buying spends the
  cost, inserts `<upgrade>_<level>` into `SaveGame::purchased_upgrades` and
  saves. That ID list is the only thing the battle sees: `config_from_save`
  copies it into the `MissionConfig`, `mission::upgrade_levels` maps it to
  `sim::Upgrades` in the `Loadout`, so upgrades change the sim only through
  its deterministic launch input (identical on every peer).
- **Result scene** (`flow.rs`): banner, kills, mission time (sim ticks),
  collected / bonus / kept / Salvage Sorter / lost loot and the wallet before →
  after.
  Continue goes to the Carrier, whose `enter_carrier` books the result into
  the save (`record_mission_return`) and stores it.
- **Mission transition screen** (`loading.rs`, TAKOAI-103): Launch goes
  Carrier → `GameScreen::Loading` → Battle (`flow::transition`; nothing
  else reaches Battle). The screen shows the mission name, a progress bar
  and one tip, and decodes every image the battle draws
  (`render::battle_image_ids`) through `ContentImages`, one per frame. It
  enters Battle the frame they are all in `Assets<Image>`, with no minimum
  time; a missing or broken image stops on an error with Back to Carrier
  (button or Enter). UI and assets only: the `SimWorld` is still created on
  entering Battle. The demo is offline; networked missions will need every
  peer to report its load done before Battle (no ready handshake exists
  yet; synctest / p2p runs don't use the flow). `--autoplay` passes through
  it and shoots `NNb-loading-M.png`.
- **Tips only on the transition screen** (TAKOAI-98): `tips::TIPS` is the
  list (the retired tutorial hints, reworded); `tips::TipDeck` picks one at
  random per transition, never the last one (UI RNG seeded from the clock,
  not the sim's). Neither the Carrier nor the Battle spawns a tip box
  (`flow::tests::no_scene_spawns_a_tip_box`).
- Battle visuals are sprites drawn by stable content ID
  (`core.battle.ship.kite`, `core.battle.enemy.void_swarmer`,
  `core.battle.fx.spit`, loot as `core.ui.icon.*`, ... in `render::ids`)
  from a reused pool of sprite entities, sized from the sim hit radius
  (longest side ≈ 1.2× the hit-circle diameter, `render::SPRITE_SCALE`, so
  what you see is what gets hit). The battle camera zooms out to
  `render::BATTLE_ZOOM` (1.265 ≈ √1.6, i.e. 1.6× the visible area) while a
  battle world exists, and the mouse wheel zooms 0.5×–2× on top of that
  about the cursor (`crate::zoom`, reset each battle; there is no arena
  edge, so the clamp keeps our ship on screen). Local camera only, never
  sim, rollback or network state. HUD text is UI and keeps its screen size;
  move targets go through `Camera::viewport_to_world_2d`, so they land
  under the cursor at any zoom.
  Ship facing is visual-only client state (from the move target / dash).
  Player bolts are a generated soft glow tinted per player; shields, the
  Shockwave ring, hull bars and the move marker stay gizmo effects. The
  Title shows the transparent `core.title.key_art` over the shared nebula
  shader, with the game name drawn over it by
  the game (`flow::title_logo`: "ONI SPACEWAR" in `core.fonts.russo_one`
  in the art's empty upper right, a dark 8-copy outline and a drop shadow,
  sized in window-height units like the art so it fits from 1000 × 580 to
  2000 × 1160; the images themselves carry no text), the Result screen a
  lightly dimmed transparent `core.result.backdrop` (the bridge window over
  the shared nebula shader), and the Workshop rows, Dock berths and Carrier
  wallet show icons / ship art.
- **Fissures and the spawn director** (`state.rs`, numbers in `tuning.rs`):
  `SimState::fissures` is placed once in `with_loadouts` from the seeded RNG
  (rejection sampling: on a ring `FISSURE_RING_MIN..=MAX` px around the
  origin, at a table-trig angle, outside the starting view given by
  `INITIAL_VIEW_HALF_*`, apart from each other). There is no arena edge
  (TAKOAI-58): nothing clamps positions; shots end by `ttl`. `direct_spawns` runs first
  each tick: one enemy per `spawn_interval(t)` on a random fissure's ring,
  Spitter with `spitter_pct(t)`, both integer ramps of the mission tick,
  held while `MAX_LIVE_ENEMIES` are alive. Spitters strafe around their
  target inside their range band (`Enemy::orbit`, picked at spawn). The sim's starting-view size mirrors the client's default window
  at `BATTLE_ZOOM` (`DEFAULT_WINDOW`); a client test keeps them equal. The
  off-screen fissure arrows (`render::fissure_pointers`, one per fissure,
  violet-magenta with a teal inner edge to match the v2 fissure art;
  `render::clear_of` slides one along its edge off the HUD text's corner)
  and the nebula sky (`sky.rs`) are client-only.
- **Nebula sky** (`sky.rs` + `sky.wgsl`, TAKOAI-58). One `SkyMaterial`
  (`Material2d`, shader embedded with `embedded_asset!`) on a unit quad that
  `follow_view` (PostUpdate) moves and scales to cover the camera's view
  each frame, passing the camera centre (parallax) and elapsed time
  (nebula drift, twinkle) as a uniform. One entity, always visible, so every
  scene shares the same live sky and a new scene gets it with no code
  (TAKOAI-83): behind the battle, around the Carrier's rooms, and the Title key art
  and Result backdrop (UI images with a transparent sky, drawn over it; the
  menu screens never move the camera, so there the sky only drifts and
  twinkles, with no parallax). `content/tests/scene_sky.rs` keeps those two
  images' skies transparent (Title corners, Result window panes at alpha 0).
  Added by `RenderPlugin`, so headless runs
  never build it. Floats and wall-clock time are fine here: it is purely
  visual and never reads or writes sim state.
- `oni-spacewar --autoplay [--ship kite|bulwark] [--missions N] [--continue]
  [--abandon] [--shots DIR] [--kill-target N]` is a QA mode for the whole
  demo loop; its battles end at 10 kills unless `--kill-target` says otherwise: New Game
  (or Continue the existing save), fly N battles (default 1) with a scripted
  pilot (input only, like a player). The first Carrier visit shows Build
  mode's flyouts, its grid at 1× and zoomed in (`11b-build-mode-grid-*`,
  to check footprints against the cells, TAKOAI-105) and walks the Pilot in all 8 directions. Between missions
  it builds the spec's example edits when it can afford them: a Small
  Room at (14,10) (mouse tile (17,12)), waits for it, then the Salvage
  Sorter at (19,11) with a few Art batch B pieces around it
  (`flow::EXAMPLE_FURNISHING`: sofa, wall screen, rug, cat tree), and
  waits again; else it buys the first affordable
  Workshop upgrade. Then it click-to-moves to the ship's berth and boards
  it on arrival; return to the Carrier after the last
  one, quit to the Title, reload the save from disk and Continue (a restart
  inside the same process), then quit (saving). `--abandon` quits the last mission from the pause menu 20 s in;
  `--shots` saves a screenshot of every scene (including the Settings
  screen over the first Title and over the last Carrier visit). Use a scratch `ONI_SAVE_DIR`.
  The acceptance loop is `--missions 2` followed by `--continue --missions 0`.
  On macOS, wrap it in `caffeinate -d`: if the display sleeps the window stops
  presenting and screenshots come out black. For the same reason (a fully
  covered window presents nothing), autoplay opens its window always-on-top
  and activates it once winit has created it (`bring_window_to_front`).
- **Settings** (`settings.rs`, TAKOAI-96) are per player and per machine:
  `settings.json` (own `version`, now 1; missing fields take defaults,
  newer versions are refused and the defaults used) next to the save, so
  `ONI_SAVE_DIR` moves both. Not part of `SaveGame` and never read by
  `sim`, so they cannot affect a battle or sync. Stored on every change.
  The Settings screen (`SettingsScreen`) opens from the gear button
  (top right on the Title, beside Build in the Carrier HUD), Esc on the
  Title, and the Carrier pause menu's Settings (S); Esc or Close shuts it
  (Esc is used up there, `settings_keys` runs before
  `flow::toggle_pause_menu`). On the Carrier it holds `PauseMenu::open`
  with the menu hidden, so the scene stays paused and every carrier system
  that checks the pause leaves it alone; closing returns to the pause
  menu if it came from there. The panel is rebuilt from sections on each
  change; the first is Display › Carrier colours: floor and wall rows of
  6 presets plus Reset to default, the chosen pair's WCAG contrast, and a
  warning (never a block) below `MIN_CONTRAST` (3:1). Defaults: Slate
  floor `#3A4350`, Steel walls `#A3B1C0`; the presets include Art's v3
  Office floor and Navy wall. **Recolour:** the floor, wall and doorway
  tiles (`settings::recolourable`, `core.carrier.tile.*`) are drawn from
  `CarrierPalette`'s copies of the shipped images: `recolour_pixel` keeps
  Art's cyan light line, and moves every other pixel by its per-channel
  offset from Art's floor (`FLOOR_REF`, seams) or wall (`WALL_REF`, trim)
  colour, whichever is nearer in luma, onto the chosen colour (Art's own
  pair reproduces the shipped pixels). The hull-edge tiles are drawn in
  exactly Art's v3 floor, wall, trim and cyan, so the same per-pixel
  family split recolours their inside floor with the floor colour and the
  shell with the wall colour; no mask image is needed
  (`hull_tiles_recolour_their_floor_and_shell_apart`). A change rewrites those images in
  place (`apply_palette`), so the deck updates live without a redraw.
  Stations, scaffold, marker and characters keep their art.
- **Codex** (`codex.rs`, TAKOAI-100): Settings › Open Codex opens a
  full-screen view on top (`CodexScreen`, `GlobalZIndex` above the panel,
  `FocusPolicy::Block`); Esc / Back returns to Settings (`codex_keys` runs
  before `settings_keys` and uses the Esc up), and closing Settings closes
  it. Its content is data, not UI code: the shipped `core.codex` JSON
  (`assets/source/core/codex.json`) declares the tabs (`categories`, in
  order) and the entries (`id`, `category`, `image` content ID, `name`,
  `description`, `placeholder`). `CodexData::sections` groups entries into
  tabs, adds a tab for any undeclared category after the declared ones and
  drops empty ones, so new content appears by adding its image and an
  entry. A missing image draws a "No art yet" box. Stats beside an entry
  are only those already in the data (`codex::stats`): footprint /
  placement / cost / effect from `layout::CATALOGUE`, HP and speed from
  `sim::tuning`, hull / speed / basic attack from `sim::ShipSheet`, a crew
  member's post. `codex::discovered` is the single unlock hook (all true
  for now). Tests require an entry, with name, description and shipped
  art, for every enemy / ship / carrier item image in the manifest, every
  catalogue item (same name) and every crew member. Wheel scrolls the
  grid (`ScrollPosition`; the held pause stops the carrier zoom), arrows
  move the selection and keep it in view, Tab / Shift+Tab switch tabs.
- Save data is client-only JSON with an explicit schema version (now 5). It
  stores credits, resources, purchased upgrades, selected battleship,
  missions played and won, the last result and the carrier
  (`carrier`: Revision 3 rooms and items), and is never read by `sim`; the
  client converts it into a deterministic mission config before launch.
  A new game (`SaveGame::default`) starts with `save::STARTING_RESOURCES`,
  currently the test-build values (`TEST_BUILD_STARTING_RESOURCES`: 10,000
  credits, 1,000 Void Crystal, TAKOAI-97); pointing it at
  `RELEASE_STARTING_RESOURCES` (nothing) restores the Demo Spec start.
  `save::from_json` loads any supported version: up to v4 the `carrier`
  field is a Revision 1 layout, so it is taken out of the JSON and handed to
  `save::migrate`, which refunds every player-built room and corridor in it
  at Revision 1's prices (`save::rev1`, the only code left that reads a
  Revision 1 layout; a v3 Dock's corridors that v4 made starting pieces
  count as refunded), and sets the starting carrier; newer versions are refused. `migrate`
  then validates the carrier (`Carrier::validate`: known IDs, rooms inside
  the hull and overlapping only on walls, every starter room, the station
  counts and one berth per ship, connected to the Command Console's room,
  every item's placement rule, layer, doorway clearance and front strip,
  every front strip reachable); a broken one resets to the starting layout
  and refunds every non-starter room and item (unknown IDs nothing), with a
  warning. Early v5 builds also stored a transitional `carrier_rev1` (the
  Revision 1 layout their scene ran on): it is dropped on load and what the
  player built in it is refunded the same way. Saves from before
  TAKOAI-98 also carry `tutorial_seen` and `carrier_refit` (the retired
  hint box's state): serde ignores them on load and the next write drops
  them. Saved on New Game, every return to the Carrier, every
  Workshop purchase, every place, move and remove, a construction site
  finishing, Launch, and Quit.

### Simulation structure

The gameplay in `sim` is the First Playable's Elimination mission
(design/READINESS.md § Demo Spec): click-to-move battleships (Kite or
Bulwark, from a per-player `Loadout` with Workshop upgrade levels) with an
auto-firing basic attack and Q/W skills, Void Swarmers and Void Spitters
spawned continuously from 1–2 void fissures in open space (no arena edge), loot pickups, and a win (40 kills) /
lose (every ship destroyed) result. The structure is what new gameplay should
follow:

- **Entities are plain data** (`entity.rs`), one `Vec` per kind inside
  `SimState`. Removal uses `retain`, so order stays stable. No Bevy entities or
  components in the simulation.
- **`step()` is a fixed list of phases** (`state.rs`): spawn director → apply
  inputs (move target, Q/W skills) → move ships → basic attacks → enemy
  behaviour (chase / strafe in range band, contact damage, shots) → move projectiles
  → hits → separation → kills + loot drops → pickups → win/lose check. Each
  phase is a method over whole collections, so the order of effects is
  explicit and identical on every peer. New mechanics add a phase (or a new
  module with one) rather than hooking into Bevy schedules. Once the mission
  has ended the world is frozen and only `frame` advances.
- **Never one battleship.** Ships stay in their `Vec` slot when destroyed
  (`hull <= 0`), enemies target the nearest *living* ship (spawns come from the
  fissures, not from any one ship), loot totals are mission-wide, and the mission
  fails only when every ship is down. Synctest/p2p runs give even handles
  Kite and odd handles Bulwark so both ships are always exercised.
- **Numbers live in `tuning.rs`** in human units (px, px/s, seconds) and are
  converted at use (`px_per_tick`, `ticks`). They come from the Demo Spec;
  values the spec doesn't give (bolt speed, dash duration, pickup drift
  speed, ...) are marked "engine default" there. Deliberate departures from
  the spec are commented where they're set (the mission success bonus is
  100 cr + 3 VC instead of 50 + 2 so one win buys an upgrade; TAKOAI-42).
- **Input** (`NetInput`) is a button bitmask (move, Q/W/E/R) plus the cursor in
  world pixels: click-to-move sets the ship's target, skills aim at the cursor.
  The basic attack needs no input. Inputs are *levels* (key held = bit set);
  the sim latches each ship's previous buttons (`Ship::prev_buttons`, rolled
  back with the rest of the state) and fires a skill only on the tick its bit
  goes 0 → 1, so a held key fires once and a press during the cooldown is
  spent (TAKOAI-50). Chosen over an edge bit sent by the client because a
  level input survives GGRS prediction (which repeats the last input) and
  dropped/duplicated frames without double-firing; move stays level-
  triggered (hold to keep steering).
- **Mission result.** `SimState::outcome()` returns a `sim::MissionOutcome`
  (success, kills, loot collected, success bonus) once the mission
  ends. Which loot the player keeps is client policy: `client::mission`
  applies each reward's `survives_failure` flag (all off in the demo) and
  turns it into the `MissionResult` the Result scene shows and the save
  records. Quit Mission from the pause menu is `MissionOutcome::Abandoned`
  and counts as failed. The client maps the save's battleship ID (`kite` /
  `bulwark`) and purchased upgrade IDs (`hull_plating_<n>`,
  `weapon_tuning_<n>`, `thruster_tuning_<n>`; highest level wins) into the
  sim `Loadout`.
- **Collision** is integer circle overlap; pairs are checked brute-force in
  `Vec` order, which is fine at current entity counts. A deterministic broad
  phase can be added when counts grow.

### Determinism choices

- **Fixed-point integers, no floats in the simulation.** Positions and
  velocities are `i32` sub-pixels (`SUB = 256`). Aiming at a point rescales
  the integer delta vector (`FxVec2::scale_to`); angle-based directions come from the
  committed Q16 sine table in `sim::trig`; lengths use `i64::isqrt`. This is
  bit-identical across platforms and compilers without depending on libm or FMA
  behaviour. Floats appear only in the client when converting a sim position to
  a `Transform`.
- **Fixed timestep, inputs only.** `step()` takes no time argument; bevy_ggrs
  runs it at `RollbackFrameRate(60)`. The sim never reads wall-clock time, frame
  delta, or unseeded randomness. `SimRng` is seeded from `SimParams::seed`
  (splitmix64-mixed) and lives inside `SimState`.
- **Stable order.** Ships are a `Vec` indexed by player handle; future entity
  collections should also be `Vec`s (or other ordered containers) inside
  `SimState`, not Bevy entities, so their order is part of the state.
- `SimState::checksum()` is an FNV-1a hash of the state for tests/logs; GGRS's
  own desync checksum comes from bevy_ggrs hashing `SimWorld`.

### Verifying determinism

```sh
cargo test                                                          # sim unit tests
cargo run --release -- synctest --headless --minutes 2 --players 4  # expect 0 mismatches
cargo run --release -- synctest --headless --minutes 1 --inject-desync  # negative control: must fail
```

CI runs the same checks: `.github/workflows/determinism.yml` calls
`ci/determinism-gate.sh` on every PR touching `crates/sim`, `crates/client`
(the SyncTest harness) or the Cargo manifests/lock. It fails on any SyncTest
mismatch, on a run that doesn't reach its frame limit, and if the
`--inject-desync` negative control is *not* caught (non-zero exit plus a
reported mismatch), so the checker can't pass vacuously. The job has a timeout
because a swallowed mismatch would otherwise stall the headless run forever.
Run `ci/determinism-gate.sh` locally after `cargo build --release`.

## Netcode

Peers connect through a matchbox signaling server (WebSocket), then talk
directly over WebRTC data channels in a full mesh (1–4 players). GGRS
exchanges inputs over one unreliable channel; `net.rs` adapts it to ggrs 0.13
and can emulate latency/loss on outgoing packets.

```sh
cargo install matchbox_server --version 0.14.0 --locked   # listens on 0.0.0.0:3536
# every peer, same --players / --seed / --room:
cargo run --release -- p2p --players 3 --room 'ws://HOST:3536/oni?next=3'
```

`--room` names the server and the room; `next=N` starts the match once N peers
have joined. Handles are assigned by sorted peer id, so every peer agrees.
Other flags: `--bot`, `--headless`, `--minutes M`, `--input-delay`,
`--max-prediction`, `--desync-interval`, and the emulator's `--delay-ms`,
`--jitter-ms`, `--loss` (applied to each peer's outgoing packets, so the added
RTT is `2 × delay`). A p2p run exits non-zero if any desync was detected or the
signaling server can't be reached.

### Internet play: signaling, STUN, TURN

- **Signaling** must be reachable by every peer: run `matchbox_server` on a
  public host and use `ws://host:3536/...`, or `wss://` behind a TLS proxy
  (matchbox's native client supports TLS).
- **STUN** lets peers behind ordinary NATs find their public address. By
  default matchbox uses Google's public STUN servers. `--ice URL` (repeatable)
  replaces them; `--ice none` disables ICE servers (LAN / same machine only).
- **TURN** relays traffic when a direct path is impossible (symmetric NAT,
  strict firewalls). Pass `--ice turn:host:3478` (plus any STUN URLs) and put
  the credentials in `ONI_ICE_USERNAME` / `ONI_ICE_CREDENTIAL`. They are read
  from the environment so they never land in the repo, shell history or
  process list. matchbox 0.14 accepts a single ICE server entry, so one
  username/credential applies to all URLs (STUN ignores them).
- matchbox 0.14 has no "relay only" switch, so a TURN server can only be
  verified from two networks where a direct path really fails.

### Soak test

`ci/p2p-soak.sh` runs N headless bot peers plus a signaling server on one
machine: 4 peers, 5 minutes, 50 ms ± 5 one-way and 2% loss by default (`PEERS`,
`MINUTES`, `NET` override). It passes when every peer reaches the frame limit
with 0 desyncs. `NEGATIVE=1` makes peer 1 inject a desync and passes only if
every peer catches it. It takes real time, so CI doesn't run it.

### Frame pacing

Windowed synctest/p2p runs append a frame-pacing section to the run report
(`pacing.rs`; the demo flow has no run report and skips it).
Bevy renders pipelined (the main app updates frame N while the render thread
draws N-1), so each frame over 20 ms is attributed to the main update, render
prepare, swapchain acquire (`get_current_texture`, which blocks on vsync and
the compositor), or render graph + submit. A probe thread sleeping 1 ms in a
loop flags OS scheduler stalls, so a hitch caused by the machine can be told
apart from one caused by our code. `--frame-latency N` sets the swapchain's
`desired_maximum_frame_latency` for experiments.

Findings (TAKOAI-23, Apple M2, 60 Hz, same machine as the spike): main update p99
≈ 2.5–3.5 ms and sim + rollback p99 ≈ 1–2 ms, so the netcode and sim are not
the cause. Hitches are almost all late swapchain acquires, i.e. the macOS
compositor handing back a drawable after the vblank. Many coincide with
scheduler stalls on a loaded machine (load average 15–20 on 8 cores during most
runs), and two P2P peers in separate processes hitch at the same sim frames,
which only a system-wide cause can do. The rare long render-prepare spans land
in a different render stage each time, which also points to preemption rather
than one slow system. `--no-vsync` removes nearly all of them;
`--frame-latency 3` on Metal lets frames run uncapped and is not a fix.
Nothing needs fixing on the game side at current content. Re-measure on an
idle machine and with real content before optimising.
