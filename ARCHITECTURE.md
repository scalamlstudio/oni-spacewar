# Architecture

_Code structure only — see `design/CONCEPT.md` and `design/DESIGN.md` for what the game is/plays like._

The repo currently holds two codebases side by side while the game moves from
LÖVE2D to Rust/Bevy:

| path | what | status |
|---|---|---|
| `Cargo.toml`, `crates/` | Rust/Bevy workspace (the game going forward) | mission prototype ported (TAKOAI-26) |
| `src/` | LÖVE2D/Lua game shell | mission content ported to `crates/`; kept until the final port stage deletes it |
| `spike/bevy-netcode/` | TAKOAI-18 rollback spike (own `Cargo.toml`, excluded from the workspace) | reference only; delete once fully migrated |

## Rust workspace

```
Cargo.toml            workspace; exact version pins live in [workspace.dependencies]
crates/
  sim/                package `oni-sim`, lib `sim` — deterministic mission simulation, NO Bevy
    src/lib.rs          crate docs + determinism rules
    src/state.rs        SimState (all rolled-back state), entities, step(), collision pass
    src/content.rs      gameplay data ported from src/ (e1/e2, p1/p2, obstacles, portal, levels w1/w2)
    src/collision.rs    integer circle / square overlap + push-out (replaces Box2D)
    src/fixed.rs        fixed-point: SUB (1 px = 256), FxVec2, mul_q16
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
  handle (1..=4, `MAX_PLAYERS`); single-player is `num_players = 1`. Dead ships
  stay in the `Vec` (`alive = false`) so indices keep matching handles.

### Mission simulation (port of `src/`)

Each entity kind is a plain struct in its own `Vec` inside `SimState`
(`ships`, `enemies`, `projectiles`, `obstacles`, `portals`, `items`). Kinds
(`EnemyKind`, `ProjectileKind`, `ObstacleKind`) index static tables in
`content.rs`, which keep the prototype's units (px/s, seconds) and convert at
use (`px_per_tick`, `ticks`).

`step()` follows `love.update`: apply a pending level switch → ships (timed
effects, click-to-move target, Q fires p1) → enemies (chase nearest living
ship, e2 fires p2 with 2%/tick from `SimRng`) → movement → collisions → remove
dead/expired.

Collisions replace Box2D with the subset the prototype relies on:
- Shapes are circles and axis-aligned squares (obstacles), in sub-pixels.
- Broad phase is sort-and-sweep on x with a total sort key `(min_x, body)`, so
  the pair order is identical on every peer; narrow phase is integer-only.
- Bodies on the same `Side` never touch (the prototype's category/mask
  `alias`); static/static pairs are skipped.
- A projectile touching anything loses 1 hp (so it is removed) and applies its
  effects (`tmat`: `stat = stat * mul + add`, instant or timed). Only
  battleships keep timed effects, as in `agent.lua`.
- Ships and enemies are pushed out of obstacles/portals, and apart from each
  other by mass (ship 5, enemy 1). Projectiles don't push; there is no
  bounce, rotation or impulse.
- A living ship touching the portal queues a switch to a random level; the
  next tick clears the level, spawns the new one and resets every ship to its
  spawn point.

Things the port does not carry over: the vignette shader (`src/shader/s1.lua`),
camera vibration (never triggered), the event-log panel, and the player/char
save stub (`player.lua`, `char.lua` beyond the battleship stats).

### Determinism choices

- **Fixed-point integers, no floats in the simulation.** Positions and
  velocities are `i32` sub-pixels (`SUB = 256`). Aiming at a point rescales
  the integer delta vector (`collision::scale_to`); angle-based directions come from the
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
