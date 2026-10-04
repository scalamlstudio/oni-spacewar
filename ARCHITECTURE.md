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
| `.github/workflows/`, `ci/` | CI: determinism gate for the sim; local P2P soak script | see § Verifying determinism, § Netcode |
| `design/` | design docs and concept art | source of truth for gameplay |

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
    src/bin/content_pipeline.rs        source -> processed asset + zstd bundle + manifest
    src/bin/content_manifest_diff.rs   compare manifests and list packs/assets a patch needs
    src/cutouts.rs                     background keying, sheet slicing, single-sprite crop, resize
    src/bin/sprite_sheet_cutouts.rs    expression sheets -> portraits
    src/bin/demo_art_import.rs         design/art/demo (+ demo-v2 battle art, carrier-2_5d pieces) -> battle / carrier / title / icon sources
  client/             package `oni-client`, bin `oni-spacewar` — everything else
    src/main.rs         CLI (synctest / p2p modes), app + GGRS session setup, ICE (STUN/TURN) config
    src/art.rs          ContentImages: shipped images by stable content ID (manifest -> processed PNG), cached
    src/flow.rs         first-playable scene state machine, Title / Result UI, --autoplay QA driver: Title → Carrier → Battle → Result
    src/layout.rs       2.5D carrier layout rules, no Bevy: hull grid, room catalogue, corridor masks, connectivity, can_place / can_demolish, walkable rects, save validation
    src/carrier.rs      Carrier scene: draws the layout, 8-direction walking, crew dialogue, Dock berths (board a ship → briefing → launch), camera / zoom, Workshop panel (Upgrades + Build tabs), Build mode
    src/workshop.rs     Workshop rules: upgrade costs / levels / buy(), and Build: build_lock / can_build / build / demolish (full refund)
    src/hints.rs        one-time tutorial hints: triggers, display queue, seen IDs in the save
    src/save.rs         versioned JSON save data in the OS data directory (override: ONI_SAVE_DIR) + migration
    src/mission.rs      typed client mission config/result handoff into `sim::SimState`; reward rules (survives_failure)
    src/rollback.rs     SimWorld resource, rollback/checksum registration, GgrsSchedule
    src/input.rs        mouse + keyboard / bot → NetInput (ReadInputs)
    src/net.rs          matchbox ↔ ggrs socket adapter + latency/loss emulator (from the spike)
    src/stats.rs        rollback / frame-time / desync measurement + report (from the spike)
    src/pacing.rs       frame-pacing profiler (windowed): main / render / swapchain-acquire split + OS-stall probe
    src/render.rs       battle sprites keyed by stable content IDs + gizmo FX, fissure pointers, camera follow, battle HUD (Update, outside rollback)
    src/sky.rs          procedural nebula sky: SkyMaterial (Material2d, `sky.wgsl`) on a quad that follows the camera; battle + Carrier backdrop
```

Dependency direction is one-way for gameplay: `client → sim`, while content
tooling is a sibling crate used only by the client and tools. `sim` depends only
on `serde`, so it cannot reach assets, the engine, the clock, the renderer or
the network.

## Modular content packs

Shipped game content lives under `assets/` and is split into independently
versioned packs. Source art and design exploration remain in `design/art/`;
that tree is never loaded directly by the game and is not rewritten by the
content pipeline. When art is ready to ship, an import step copies or exports it
into `assets/source/<pack>/...`, then the pipeline writes processed files and
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
  `battle/ship/kite.png` -> `core.battle.ship.kite`) plus the HUD text, and
  writes the zstd bundles and manifest. Images ship as PNG for now.

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
shot and loot icons under the same stable IDs, plus the void fissure) is
imported with:

```sh
cargo run --release --bin demo-art-import -- .
cargo run --release --bin content-pipeline -- .
```

`demo-art-import` keys out each image's flat background (art that already
has a transparent background is only cropped), cuts the Pilot walk sheet
(idle + 4 frames, shared canvas) and the icon sheet (5 icons) with the same
sheet slicer as the portraits, scales everything down to its in-game size
and writes `assets/source/core/{battle,carrier,title,ui/icon}/...`. Restyled
pieces from `design/art/demo-v2/` are rows in its `V2_SPRITES` table (so far
the ship-free Dock berth, `core.carrier.dock.berth`, no longer drawn since
the 2.5D Dock room has its own pads). The 2.5D carrier's modular pieces
(`design/art/carrier-2_5d/`, TAKOAI-55) are rows in `CARRIER_2_5D`: they are
already transparent and authored at 2× on a 256 px-per-cell grid, so they are
copied unchanged after a size check, as `core.carrier.room.<id>`,
`core.carrier.door.<n|e|s|w>`, `core.carrier.corridor.<mask>`,
`core.carrier.hull_floor` and `core.carrier.build_slot` (drawn at 1×, 128 px
per cell). It also
writes `target/demo-art-contact-sheet.png` for a visual check. The file
names and target sizes are tables at the top of the tool; a new art file
means a new table row.

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
  when that list currently has one entry. The demo battle path steps the same
  `SimWorld` resource at a fixed rate without opening a network session; the
  `synctest` and `p2p` modes still use `RollbackPlugin` and GGRS.
- **The 2.5D carrier** (`layout.rs` rules, `carrier.rs` scene; design/READINESS.md
  § Carrier). Plain Bevy in `Update`, gated on `GameScreen::Carrier`; it owns
  no sim state. `layout::CarrierLayout` (in the save) is rooms (id + anchor
  cell) and corridor cells on a 12 × 8 hull of 128 px cells; grid y grows
  south and the scene flips it (`carrier::world` / `grid`). Corridors shape
  themselves from a 4-bit mask (`mask`, which picks the corridor art); a
  socket with a corridor in front is a door (door overlay art). Everything
  must stay connected to the Bridge (`connected`, a flood fill over rooms and
  corridors). `can_place` / `can_demolish` return the spec's reasons, which the
  build ghost shows. `walkable()` turns the layout into rectangles (room
  interiors, door gaps, corridor lanes); `can_stand` checks the Pilot's 12 px
  circle (centre + 8 rim points, each inside some rectangle) and `walk`
  slides along walls. `sync_layout` redraws the scene whenever the saved
  layout changes, recomputing the walkable rects and the hotspots (crew, the
  Workshop bench, one per berth). Characters and docked ships are y-sorted
  by their feet (`depth`); floors, doors, slots and the ghost are fixed
  layers. Characters are scaled by their visible pixels (`standing`), so the
  Pilot and the crew are the same height (`CHARACTER_H`). The Pilot keeps the
  side-view walk sheet (flipped for west, facing kept for north / south). The
  camera follows the Pilot, clamped to one cell past the hull
  (`clamp_camera`), with three zoom levels on the mouse wheel (`ZOOMS`); Build
  mode switches to Overview and pans with WASD instead. Pure helpers
  (`hotspots`, `nearest_hotspot`, `clicked_hotspot`, `interact_with`,
  `ship_at`, `apply_build_click`, `ghost`, `crew_lines`, `ship_card`,
  `upgrade_preview`) hold the rules and are unit tested. The Dock's Launch
  writes a `mission::MissionRequest` message; the flow stores the picked
  battleship in the save and enters Battle, where `config_from_save` builds
  the `MissionConfig`.
- **Building** (Workshop bench → Build tab → `Overlay::Build`). The Build tab
  lists `BUILD_TOOLS` (Corridor, Salvage Bay, Training Room, Demolish) with
  `workshop::build_lock` (already built / can't afford). In Build mode the
  ghost (the piece's art, green or red, plus one line) follows `BuildCursor`;
  a left click runs `apply_build_click` → `workshop::build` (pays and places)
  or `workshop::demolish` (full refund) and saves. Right click / Esc go back
  to the tab (Esc on the Carrier closes panels before it opens the pause
  menu: `flow::toggle_pause_menu` skips while an overlay is open).
- **Room effects.** Training Room: `mission::upgrade_ids` adds
  `training_room_1` to the mission's upgrade IDs, which `upgrade_levels`
  maps to `sim::Upgrades::training`; `ShipSheet::new` applies × 85 / 100
  (integer) to the Q / W cooldowns (`tuning::trained`), the ship keeps them
  in `ShipStats`, and the Dock card / Workshop preview read the same sheet.
  The determinism gate's fleet gives handles 2–3 the Training Room. Salvage
  Bay: `SaveGame::salvage_bonus` (+25% of what a successful mission kept,
  rounded down) is added in `record_mission_return` and shown as its own
  Result line.
- **Dock berths.** The Dock room art has the pads; `carrier::berth()` places
  one `Berth { ship }` per entry in `carrier::SHIPS` (2 × 2-cell pads, left to
  right) and the ship's battle sprite on it (`DockedShip`), nose north, its
  longest side `ShipInfo::berth_len` (design/READINESS.md § Dock and
  berths). The ship is the interaction: E near the pad's front edge
  (`Hotspot::Berth`) or a click on the ship calls `interact_with`, which
  writes it to `selected_battleship`, plays the Pilot's briefing and ends in
  the launch confirm panel for that ship. The selected ship gets a highlight
  ring and a "Selected" tag; the other is dimmed.
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
  collected / bonus / kept / Salvage Bay / lost loot and the wallet before →
  after.
  Continue goes to the Carrier, whose `enter_carrier` books the result into
  the save (`record_mission_return`) and stores it.
- **Tutorial hints** (`hints.rs`): `Hints::trigger(save, hint)` queues a hint
  the first time and writes its stable ID into `SaveGame::tutorial_seen`
  (saved immediately). Carrier triggers live in `carrier.rs`, battle triggers
  read `SimWorld` in `Update` (never in the rollback schedule). One box shows
  one hint for 6 s; queued hints from a scene the player has left are
  dropped.
- Battle visuals are sprites drawn by stable content ID
  (`core.battle.ship.kite`, `core.battle.enemy.void_swarmer`,
  `core.battle.fx.spit`, loot as `core.ui.icon.*`, ... in `render::ids`)
  from a reused pool of sprite entities, sized from the sim hit radius
  (longest side ≈ 1.2× the hit-circle diameter, `render::SPRITE_SCALE`, so
  what you see is what gets hit). The battle camera zooms out to
  `render::BATTLE_ZOOM` (1.265 ≈ √1.6, i.e. 1.6× the visible area) while a
  battle world exists; HUD text is UI and keeps its screen size.
  Ship facing is visual-only client state (from the move target / dash).
  Player bolts are a generated soft glow tinted per player; shields, the
  Shockwave ring, hull bars and the move marker stay gizmo effects. The
  Title shows `core.title.key_art`, the Result screen a dimmed
  `core.carrier.interior`, and the Workshop rows, Dock berths and Carrier
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
  off-screen fissure arrows (`render::fissure_pointers`, one per fissure)
  and the nebula sky (`sky.rs`) are client-only.
- **Nebula sky** (`sky.rs` + `sky.wgsl`, TAKOAI-58). One `SkyMaterial`
  (`Material2d`, shader embedded with `embedded_asset!`) on a unit quad that
  `follow_view` (PostUpdate) moves and scales to cover the camera's view
  each frame, passing the camera centre (parallax) and elapsed time
  (nebula drift, twinkle) as a uniform. Visible while a `SimWorld` exists
  (battle, synctest / p2p) or on the Carrier, where it replaces the old
  tiled space behind the hull. Added by `RenderPlugin`, so headless runs
  never build it. Floats and wall-clock time are fine here: it is purely
  visual and never reads or writes sim state.
- `oni-spacewar --autoplay [--ship kite|bulwark] [--missions N] [--continue]
  [--abandon] [--shots DIR]` is a QA mode for the whole demo loop: New Game
  (or Continue the existing save), fly N battles (default 1) with a scripted
  pilot (input only, like a player); between them, build the corridor at
  (8,6) and the Salvage Bay at (9,5) if they're affordable and not built yet,
  else buy the first affordable Workshop upgrade; return to the Carrier
  after the last one and quit
  (saving). `--abandon` quits the last mission from the pause menu 20 s in;
  `--shots` saves a screenshot of every scene. Use a scratch `ONI_SAVE_DIR`.
  The acceptance loop is `--missions 2` followed by `--continue --missions 0`.
  On macOS, wrap it in `caffeinate -d`: if the display sleeps the window stops
  presenting and screenshots come out black.
- Save data is client-only JSON with an explicit schema version (now 3). It
  stores credits, resources, purchased upgrades, selected battleship, tutorial
  hints seen, missions played and won, the last result and the carrier
  layout, and is never read by `sim`; the client converts it into a
  deterministic mission config before launch. Older versions load through
  `save::migrate` (every field added since v1 has a serde default; v1 / v2
  get the starting layout); newer versions are refused. `migrate` then
  validates the layout (`CarrierLayout::validate`: known rooms, inside the
  hull, no overlaps, the starting pieces in place, each buildable room at
  most once, connected); a broken one resets to the starting layout and
  refunds every non-starting piece, with a warning. Rooms are written sorted
  by id and corridors by (x, y). Saved on New Game, every return to the
  Carrier, every Workshop purchase, build and demolish, Launch, the first
  showing of each hint, and Quit.

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
