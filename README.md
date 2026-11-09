# Oni Spacewar

A Rust/Bevy game: manage a modular carrier ship and its Oni crew
(Oxygen Not Included-style base building), then take a battleship out on
Warframe-style missions with League of Legends-style click-to-move combat.

The First Playable Demo is in: Title → Carrier → Mission station → Battle (Elimination)
→ Result → Carrier → Workshop upgrades → go again. It is a Cargo workspace
under [`crates/`](crates/), separate from the design docs — see
[`ARCHITECTURE.md`](ARCHITECTURE.md).

### Run

 - `cargo run --release --bin oni-spacewar` — the demo (Title screen → New Game / Continue)
   - Carrier: left-click the floor to walk there (WASD / arrows also walk),
     left-click a crew member or station to walk up and use it (or walk up
     and press E), mouse wheel zoom. Use the Command Console on the Bridge
     for mission selection: pick Kite / Bulwark (1 / 2), choose Launch prep,
     then advance the Pilot briefing with E to launch. Docked battleships
     open ship inspection and selection; component refitting follows later
   - Building: B (or the HUD's Build button) opens Build mode: pick a room
     or an item from Rooms / Stations / Furniture / Decor, then left-click
     to start building it (the ghost says why not); Move and Remove (full
     refund) work on what is there; WASD / arrows pan, mouse wheel zooms,
     right click / Esc drops the pick, Esc / B again leaves
   - Battle: left-click to move, auto-fire, Q / W skills, Esc pause;
     destroy the displayed seeded target of 40–100 enemies (they come in packs; Spitters glow and
     show an aim line before they fire; as kills rise, Lancers draw a
     charge line, Bloaters burst a second after they die or touch you,
     Broods release Swarmers and Wardens shield nearby monsters; later
     some packs are led by a named elite with an affix, and it drops
     extra loot under a light beam) and fly over loot to collect it; at
     the kill target the fissures go quiet and the Fissure Guardian comes
     out: fly through the gaps in its shot rings and off its charge line,
     and the mission is won when it dies
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
