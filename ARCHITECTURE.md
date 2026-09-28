# Architecture

_Code structure only — see `design/CONCEPT.md` and `design/DESIGN.md` for what the game is/plays like._

The repo currently holds two codebases side by side while the game moves from
LÖVE2D to Rust/Bevy:

| path | what | status |
|---|---|---|
| `Cargo.toml`, `crates/` | Rust/Bevy workspace (the game going forward) | foundation + placeholder gameplay (TAKOAI-26) |
| `src/` | LÖVE2D/Lua game shell | prototype from another project, not ported; deleted in the final port stage |
| `spike/bevy-netcode/` | TAKOAI-18 rollback spike (own `Cargo.toml`, excluded from the workspace) | reference only; delete once fully migrated |

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
  client/             package `oni-client`, bin `oni-spacewar` — everything else
    src/main.rs         CLI (synctest / p2p modes), app + GGRS session setup
    src/rollback.rs     SimWorld resource, rollback/checksum registration, GgrsSchedule
    src/input.rs        mouse + keyboard / bot → NetInput (ReadInputs)
    src/net.rs          matchbox ↔ ggrs socket adapter + latency/loss emulator (from the spike)
    src/stats.rs        rollback / frame-time / desync measurement + report (from the spike)
    src/render.rs       placeholder shapes, camera follow, HUD (Update, outside rollback)
```

Dependency direction is one-way: `client → sim`. `sim` depends only on `serde`,
so it cannot reach the engine, the clock, the renderer or the network.

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

P2P needs a matchbox signaling server (`cargo install matchbox_server --version 0.14.0`);
see `spike/bevy-netcode/README.md` for the p2p flags, which are unchanged.

## LÖVE2D shell (`src/`)

The code lives under [`src/`](src/), with `src/main.lua` as the entry point.

```
Agent ------+-- Object --+-- Circle
            |     |      |
Enemy ------+     |      +-- Square
            |   Config   |
Obstacle ---+            +-- Triangle
            |            |
Item -------+            +-- Hexagon
            |
Projectile -+
            |
Portal -----+
```
