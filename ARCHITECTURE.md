# Architecture

_Code structure only — see `design/CONCEPT.md` and `design/DESIGN.md` for what the game is/plays like._

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

## `spike/bevy-netcode/` (TAKOAI-18)

A stand-alone Rust/Bevy test build for the Love2D → Bevy go/no-go decision. It is
not part of the game and shares no code with `src/`. See its README for layout and
the determinism choices (fixed-point integer simulation, no floats in rollback state).
