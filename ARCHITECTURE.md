# Architecture

_Code structure only — see `design/CONCEPT.md` and `design/DESIGN.md` for what the game is/plays like._

The game is a Rust/Bevy Cargo workspace. The earlier LÖVE2D/Lua prototype
(`src/`, `love.sh`) and the TAKOAI-18 netcode spike (`spike/bevy-netcode/`)
were removed in TAKOAI-24; both remain in git history (the spike's final state
is commit `b92126c`, its results are recorded on TAKOAI-18).

| path | what | status |
|---|---|---|
| `Cargo.toml`, `crates/` | Rust/Bevy workspace | foundation + placeholder gameplay (TAKOAI-26) |
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
    src/entity.rs       plain-data entities: Ship, Enemy, Projectile
    src/tuning.rs       placeholder gameplay numbers + unit conversion (px/s -> sub-px/tick)
    src/collision.rs    integer circle overlap + separation
    src/fixed.rs        fixed-point: SUB (1 px = 256), FxVec2 (+ scale_to), mul_q16
    src/trig.rs         integer sin/cos from a committed Q16 table (from the spike)
    src/rng.rs          SimRng, seeded xorshift64* (part of SimState)
    src/input.rs        NetInput: buttons (move, Q/W/E/R) + cursor target, exchanged per tick
  content/            package `oni-content`, lib `content` — content manifests, stable IDs, patch diffing
    src/lib.rs          manifest types, hash validation, stable-ID resolution, manifest diff
    src/bin/content_pipeline.rs        source -> processed asset + zstd bundle + manifest
    src/bin/content_manifest_diff.rs   compare manifests and list packs/assets a patch needs
  client/             package `oni-client`, bin `oni-spacewar` — everything else
    src/main.rs         CLI (synctest / p2p modes), app + GGRS session setup, ICE (STUN/TURN) config
    src/flow.rs         first-playable scene state machine and placeholder UI: Title → Carrier → Battle → Result
    src/save.rs         versioned JSON save data in the OS data directory (override: ONI_SAVE_DIR)
    src/mission.rs      typed client mission config/result handoff into `sim::SimState`
    src/rollback.rs     SimWorld resource, rollback/checksum registration, GgrsSchedule
    src/input.rs        mouse + keyboard / bot → NetInput (ReadInputs)
    src/net.rs          matchbox ↔ ggrs socket adapter + latency/loss emulator (from the spike)
    src/stats.rs        rollback / frame-time / desync measurement + report (from the spike)
    src/pacing.rs       frame-pacing profiler (windowed): main / render / swapchain-acquire split + OS-stall probe
    src/render.rs       placeholder shapes, camera follow, HUD (Update, outside rollback)
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
  build-time conversion once real sprites/audio land. The checked-in
  `content-pipeline` is deliberately small; it establishes the manifest and
  zstd packaging contract while the art pipeline is still placeholder-scale.

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
- Save data is client-only JSON with an explicit schema version. It stores
  credits, resources, purchased upgrades, selected battleship, tutorial flags
  and mission count, and is never read by `sim`; the client converts it into a
  deterministic mission config before launch.

### Simulation structure

The gameplay in `sim` is a placeholder (click-to-move battleships, a Q shot on
a cooldown, waves of enemies that chase the nearest ship). It exists to
exercise the foundation and will be replaced; the structure is what new
gameplay should follow:

- **Entities are plain data** (`entity.rs`), one `Vec` per kind inside
  `SimState`. Removal uses `retain`, so order stays stable. No Bevy entities or
  components in the simulation.
- **`step()` is a fixed list of phases** (`state.rs`): spawn → apply inputs →
  move → hits → separation → remove dead. Each phase is a method over whole
  collections, so the order of effects is explicit and identical on every
  peer. New mechanics add a phase (or a new module with one) rather than
  hooking into Bevy schedules.
- **Numbers live in `tuning.rs`** in human units (px, px/s, seconds) and are
  converted at use (`px_per_tick`, `ticks`). They are placeholders, not design
  values.
- **Input** (`NetInput`) is a button bitmask (move, Q/W/E/R) plus the cursor in
  world pixels: click-to-move sets the ship's target, skills aim at the cursor.
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

Windowed runs append a frame-pacing section to the run report (`pacing.rs`).
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
