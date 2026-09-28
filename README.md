# Oni Spacewar

A LÖVE2D game: manage a modular carrier ship and its Oni crew
(Oxygen Not Included-style base building), then take a battleship out on
Warframe-style missions with League of Legends-style click-to-move combat.

In the design phase — the code is still the running project shell bootstrapped
from the `space-sandbox` entity/world architecture.

The game code lives in [`src/`](src/), separate from the design docs. The
Rust/Bevy rewrite is a Cargo workspace under [`crates/`](crates/) — see
[`ARCHITECTURE.md`](ARCHITECTURE.md).

### Run Rust/Bevy client

 - `cargo run --release -- synctest --players 1` — placeholder gameplay on the
   foundation: left-click to move (hold to steer), Q to fire toward the cursor
 - `cargo test` — simulation tests

### Run Game

[Install Love2D](https://love2d.org/#download)

 - macOS: `bash love.sh`
 - Other Platforms: `love src` — see [Love2D Wiki - Getting Started](https://love2d.org/wiki/Getting_Started)

### Design

 - [`design/CONCEPT.md`](design/CONCEPT.md) — pitch
 - [`design/DESIGN.md`](design/DESIGN.md) — mechanics
 - [`design/STORY.md`](design/STORY.md) — setting and lore
 - [`design/READINESS.md`](design/READINESS.md) — first-playable scope and design-done checklist
 - [`design/OPEN_QUESTIONS.md`](design/OPEN_QUESTIONS.md) — open questions, what blocks the first playable, and what to decide next
 - [`design/PROGRESS.md`](design/PROGRESS.md) — decision log
 - [`design/art/`](design/art/) — concept art (concept-art, ships, carriers, enemies)

Tasks are tracked on the project's Multica board, not in this README.
