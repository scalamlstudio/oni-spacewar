# Bevy rollback netcode spike (TAKOAI-18)

Stand-alone test build that answers one question: can Bevy + `bevy_ggrs` +
`matchbox` run a deterministic bullet-hell mission (2 players, ~1,000 bullets)
with P2P rollback? It is a go/no-go gate for moving from LÖVE2D to Bevy. The Lua
game in `src/` is untouched.

> **Status: not yet compiled.** The code is complete but the first build is
> blocked on a very slow crates.io connection. See the PR/issue for the latest
> results.

## Versions

| crate | version | note |
|---|---|---|
| bevy | `=0.19.1` | pinned exactly |
| bevy_ggrs | `=0.22.0` | needs bevy 0.19, ggrs 0.13 |
| ggrs | `=0.13.0` | |
| matchbox_socket | `=0.14.0` | its bundled GGRS adapter targets ggrs 0.11, so `src/net.rs` implements `ggrs::NonBlockingSocket` itself |
| matchbox_server | 0.14 | signaling server, installed separately |

## Layout

- `src/sim.rs` is the deterministic simulation that runs in `GgrsSchedule`: ships, the Maw ring/spiral emitter,
  Swarmling aimed streams, and collisions.
- `src/trig.rs` holds integer sin/cos from a committed Q16 lookup table.
- `src/net.rs` has the matchbox → GGRS socket adapter and the latency/loss emulator.
- `src/stats.rs` measures rollbacks, frame times and desyncs, and prints the report.
- `src/render.rs` has the placeholder visuals and HUD, outside the rollback schedule.

## Determinism choices

- **Fixed-point integers, no floats in the simulation.** Positions and velocities
  are `i32` sub-pixels (1 px = 256). Directions come from a committed Q16
  sine table. Aim vectors are normalised with `i64::isqrt`. The result is
  bit-identical on every platform and compiler, with no reliance on libm or FMA
  behaviour. Floats are only used when rendering converts positions into a
  `Transform`.
- The only inputs are `PlayerInputs` and a seeded xorshift RNG (`SimRng`). The
  RNG is itself rolled back and checksummed.
- Stable order: ships are sorted by player handle, and all bullets live in one
  `Vec` resource, so their order is part of the state.
- Rolled back and checksummed: `Ship`, `FxPos`, `ShipHealth`, `Bullets`, `SimRng`,
  `SimFrame`, `Emitters`.

## Build

```sh
cd spike/bevy-netcode
cargo build --release
```

## Modes

### SyncTest (criterion 1)

GGRS re-simulates the last `check-distance` frames every frame and compares
checksums. Both players are bots.

```sh
# headless, as fast as the CPU allows: 10 minutes of simulated play
cargo run --release -- synctest --headless --minutes 10
# windowed (player 1 on WASD/arrows + Shift for focus; player 2 is a bot)
cargo run --release -- synctest
```

### P2P over matchbox (criteria 2 and 3)

1. Start the signaling server once:
   ```sh
   cargo install matchbox_server --version 0.14.0
   matchbox_server            # listens on 0.0.0.0:3536
   ```
2. Start two peers. Both must use the same `--room`, `--seed` and `--bullets`.
   ```sh
   # 50 ms one-way each direction (= ~100 ms added RTT), 2% loss per direction
   cargo run --release -- p2p --bot --minutes 5 --delay-ms 50 --jitter-ms 5 --loss 0.02
   cargo run --release -- p2p --bot --minutes 5 --delay-ms 50 --jitter-ms 5 --loss 0.02
   ```
   Leave out `--bot` to fly with the keyboard. Add `--headless` to run without a window.
   For two machines, point `--room` at the server machine, for example
   `--room ws://192.168.1.10:3536/spike?next=2`.

### Latency / packet-loss emulation

This is built into the GGRS socket adapter (`src/net.rs`), so it works the same
on every OS and needs no admin rights. Each outgoing packet is dropped with
probability `--loss`. Otherwise it is held for `--delay-ms ± --jitter-ms`
before being sent. Both peers apply it, so the added round trip is
`2 × delay-ms`.

## Report

Each run prints a `spike report` block on exit (or when `--minutes` is
reached) containing:

- simulated frames
- rollback count and max rollback depth
- average and max bullet count
- synctest mismatches and P2P desyncs
- frame-time p50/p99/p99.9 and the five worst frames
- time spent inside the GGRS update

Frame times exclude the first 3 s (warm-up).

Other options: `--bullets N` sets the target live-bullet count (emitters pause
at the cap), `--input-delay`, `--max-prediction`, `--desync-interval`, and
`--no-vsync` to measure headroom above 60 fps.
