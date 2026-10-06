# Oni Spacewar

A Rust/Bevy game: manage a modular carrier ship and its Oni crew
(Oxygen Not Included-style base building), then take a battleship out on
Warframe-style missions with League of Legends-style click-to-move combat.

The First Playable Demo is in: Title → Carrier → Dock → Battle (Elimination)
→ Result → Carrier → Workshop upgrades → go again. It is a Cargo workspace
under [`crates/`](crates/), separate from the design docs — see
[`ARCHITECTURE.md`](ARCHITECTURE.md).

### Run

 - `cargo run --release --bin oni-spacewar` — the demo (Title screen → New Game / Continue)
   - Carrier: left-click the floor to walk there (WASD / arrows also walk),
     left-click a crew member, the Workshop bench or a battleship to walk up
     and use it (or walk up and press E), mouse wheel zoom; each battleship
     has its own Dock: board it, then launch
   - Building: Workshop bench → Tab to the Build tab → pick a piece, then
     left-click a spot (green = legal); X toggles demolish (full refund),
     WASD / arrows pan, mouse wheel zooms, right click / Esc goes back
   - Battle: left-click to move, auto-fire, Q / W skills, Esc pause;
     destroy 20 enemies and fly over loot to collect it
   - Save: `~/Library/Application Support/oni-spacewar/save.json` (macOS),
     `%APPDATA%\oni-spacewar\save.json` (Windows); `ONI_SAVE_DIR` overrides
 - `cargo run --release --bin oni-spacewar -- --autoplay --missions 2` — scripted demo run (QA)
 - `cargo run --release --bin oni-spacewar -- synctest --players 1` — rollback sandbox
 - `cargo test` — tests

### Design

 - [`design/CONCEPT.md`](design/CONCEPT.md) — pitch
 - [`design/DESIGN.md`](design/DESIGN.md) — mechanics
 - [`design/STORY.md`](design/STORY.md) — setting and lore
 - [`design/READINESS.md`](design/READINESS.md) — first-playable scope and design-done checklist
 - [`design/OPEN_QUESTIONS.md`](design/OPEN_QUESTIONS.md) — open questions, what blocks the first playable, and what to decide next
 - [`design/PROGRESS.md`](design/PROGRESS.md) — decision log
 - [`design/art/`](design/art/) — concept art (concept-art, ships, carriers, enemies)

Tasks are tracked on the project's Multica board, not in this README.
