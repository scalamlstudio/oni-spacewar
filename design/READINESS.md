# Readiness

_What "the design is solid enough to start building" means for Oni Spacewar.
Questions themselves — including which ones block this scope and the order to
decide them in — live only in design/OPEN_QUESTIONS.md; this doc doesn't copy
them. Task tracking lives on the Multica board (First Playable Demo: TAKOAI-36)._

## First Playable Scope

_**Confirmed by Wei on 2026-10-03** (logged in design/PROGRESS.md). Mechanics
live in design/DESIGN.md; the numbers live in § Demo Spec below._

The **First Playable Demo** is one complete, repeatable loop:

**Title → Carrier → Dock (pick battleship + briefing) → Battle (Elimination) →
Result → back to Carrier → spend rewards at the Workshop → go again.**

It is single-player and offline. Battle logic still lives in the deterministic
`sim` crate (no Bevy, no clock, no floats) and never assumes there is only one
battleship in a mission instance, so multiplayer missions stay possible later.
Carrier, result, menus and save live in `client`.

**In scope**

- **Title screen:** New Game / Continue / Quit.
- **Carrier scene:** the player walks the main character (the Pilot) freely
  around the carrier interior, talks to crew and enters modules to use them.
  Side view, ONI-style cross-section, one deck. Modules: **Dock** (pick a
  battleship, hear the briefing, launch) and **Workshop** (spend credits and
  Void Crystal on upgrades), plus crew to talk to.
- **Crew dialogue:** 2–3 short lines per crew member, each with one of the 11
  expression portraits; the Pilot gives the briefing at the Dock; one line per
  crew member reacts to the last mission's result.
- **Battleships:** 2 with different playstyles — **Kite** (fast, fragile, short
  cooldowns) and **Bulwark** (slow, tanky, stronger basic attack). Each has 2
  skills (Q, W).
- **Battle scene:** the existing top-down click-to-move battleship, auto-firing
  its basic attack. One mission type, **Elimination** (kill 20 enemies), in one
  hand-built arena, with 3 enemy waves of 2 void-monster types (**Void Swarmer**,
  **Void Spitter**).
- **Loot:** enemies drop **Credits** and **Void Crystal**, collected by flying
  over them.
- **Battle HUD:** hull bar, skill cooldowns, kill counter, wave banner, loot
  collected this mission.
- **Result scene:** Success or Failed, kills, and the loot kept or lost. On
  failure the mission's loot is lost, following each reward's "survives
  failure" flag (all off in the demo).
- **Workshop upgrades:** 3 upgrades × 2 levels, applied to every battleship.
- **Pause / quit menu** in every scene, and a **save file** (credits, Void
  Crystal, upgrade levels, last result, tutorial hints seen).
- **Tutorial hints** the first time the player does something.
- **Art:** the demo art set, shipped through the content pipeline
  (`assets/source/<pack>/…`, stable IDs, manifest). No audio.

**Out of scope / deferred** (not rejected)

- Manufacturing Module, buildings, conveyors and production chains;
  extraordinary samples.
- Companion task assignment (ONI-style jobs) and companions coming along on
  missions (their auto-trigger skills).
- Research loop: Research points, tech documents, crew skill trees, Lab Module.
- Building placement on the carrier, module unlocking, carrier growth.
- Hub, trading, factions, player-to-player anything.
- Multiplayer missions — the design keeps them; the code just mustn't rule
  them out.
- Procedural levels, other objective types, story beyond a title card.

## Demo Spec

_Defaults written by Mika for the demo — Wei can tune any of them; none
blocks the build. Units match `crates/sim/src/tuning.rs`: px, px/second,
seconds. Percent bonuses are integer math, rounded down._

### Carrier

One deck, laid out left to right. The Pilot spawns at the Dock after a
mission, at the Bridge on a new game.

| Room | Who / what | Interaction (E) |
|---|---|---|
| Bridge | Gunner; star map showing the next mission | Talk |
| Crew Quarters | Researcher | Talk |
| Workshop | Engineer; upgrade bench | Talk; open upgrade panel |
| Dock | Kite and Bulwark in their berths; launch console | Pick battleship → briefing → Launch |

- A / D walk (120 px/s); E interacts with the nearest hotspot within 40 px,
  shown as an "E Talk" / "E Use" prompt. Esc opens the pause menu.

### Battleships

Both are available from the start. Stats are before upgrades.

| | Kite | Bulwark |
|---|---|---|
| Role | fast, fragile, short cooldowns | slow, tanky, stronger basic attack |
| Hull (HP) | 60 | 140 |
| Move speed | 220 px/s | 140 px/s |
| Radius | 14 px | 20 px |
| Basic attack | 4 dmg every 0.35 s, range 220 px | 8 dmg every 0.6 s, range 240 px |
| Q | **Afterburn:** dash 160 px toward the cursor. CD 4 s | **Bastion:** shield absorbs the next 40 dmg, lasts 5 s. CD 12 s |
| W | **Scatter:** 5 bolts in a 40° cone toward the cursor, 6 dmg each, 600 px/s, 0.5 s life. CD 6 s | **Shockwave:** 15 dmg to every enemy within 150 px, pushes them 100 px away. CD 9 s |

Basic attacks fire at the nearest enemy in range (design/DESIGN.md § Player).

### Enemies

| | Void Swarmer | Void Spitter |
|---|---|---|
| Behavior | rushes the nearest battleship | keeps 200–320 px from the nearest battleship; fires when it's within 360 px |
| HP | 10 | 36 |
| Move speed | 110 px/s | 60 px/s |
| Radius | 10 px | 14 px |
| Attack | contact: 5 dmg, then 1 s cooldown per swarmer | slow projectile: 8 dmg, 160 px/s, radius 5 px, 3 s life, every 2.5 s |

### Elimination mission

- **Arena:** one hand-built rectangle, 1600 × 1200 px, edges block movement.
- **Kill target:** 20. The HUD shows "Kills x/20". Reaching 20 is Success
  immediately: remaining enemies vanish and every pickup still on the field is
  collected.
- **Failed:** every battleship in the mission is destroyed (solo: yours). Quit
  Mission from the pause menu also counts as Failed.
- **No time limit.**

| Wave | Spawns | Total spawned |
|---|---|---|
| 1 | 8 Swarmers (2 groups of 4, 2 s apart) | 8 |
| 2 | 6 Swarmers + 2 Spitters | 16 |
| 3 | 6 Swarmers + 2 Spitters | 24 |

- 24 spawn for a target of 20, so the player never has to hunt the last one.
- The next wave comes when the current one is cleared or 25 s after it spawned,
  whichever is first, after a **4 s pause** with a "Wave N" banner. Wave 1 also
  gets the 4 s banner.
- Enemies spawn on a ring 450 px from the centroid of the living battleships,
  clamped into the arena.

### Loot

| Source | Credits | Void Crystal | Survives failure |
|---|---|---|---|
| Void Swarmer | 5 (always) | 1 (15% chance) | off |
| Void Spitter | 15 (always) | 1 (60% chance) | off |
| Mission success bonus | 50 | 2 | — (success only) |

- Drops are rolled with the sim's deterministic RNG.
- Pickups are collected when a battleship comes within 32 px; within 80 px they
  drift toward it. They disappear after 20 s.
- Expected haul from a successful run (~17 Swarmers + 3 Spitters + bonus):
  **~180 credits, ~6 Void Crystal.** A failed run keeps nothing.
- Every reward type carries a `survives_failure` flag, default off. The demo
  turns none on.

### Workshop upgrades

Apply to every battleship, in battle and in the Dock's stat preview. Priced so
one successful mission buys about one upgrade.

| Upgrade | Effect per level | Level 1 cost | Level 2 cost |
|---|---|---|---|
| Hull Plating | +25% hull (Kite 60 → 75 → 90; Bulwark 140 → 175 → 210) | 150 cr + 4 VC | 250 cr + 6 VC |
| Weapon Tuning | +25% damage, basic attack and skills (Kite basic 4 → 5 → 6; Bulwark basic 8 → 10 → 12) | 150 cr + 4 VC | 250 cr + 6 VC |
| Thruster Tuning | +15% move speed (Kite 220 → 253 → 286; Bulwark 140 → 161 → 182) | 150 cr + 4 VC | 250 cr + 6 VC |

All six levels cost 1200 credits + 30 Void Crystal (~6 successful missions). A
new game starts with 0 credits and 0 Void Crystal. Weapon Tuning 1 is the most
visible first buy: Bulwark one-shots Swarmers, Kite needs 2 hits instead of 3.

### Crew dialogue

Talking cycles through the member's lines; the reaction line comes first after
a mission. Portrait expressions are the 11 in `assets/packs/core` (normal,
happy, excited, serious, angry, sad, shocked, surprised, confused, shy,
sleepy).

| Crew | Lines (expression) |
|---|---|
| **Pilot** (main character) — Dock briefing | "Elimination run. Void monsters are nesting in the next segment." (serious) · "Twenty kills clears it. They come in three waves." (normal) · "Grab whatever they drop — the Workshop runs on it." (excited) |
| **Gunner** — Bridge | "Swarmers rush you. Keep moving and let the guns work." (serious) · "Spitters hang back. Their shots are slow — sidestep them." (normal) · Reaction — success: "Clean shooting out there." (happy) / failed: "We lost the hull, not the crew. Again." (angry) |
| **Researcher** — Crew Quarters | "Void Crystal hums when you hold it. I'd love a proper lab." (excited) · "Kite or Bulwark? Speed or armor — both are valid." (confused) · Reaction — success: "Fascinating samples! Well, crystals." (happy) / failed: "We'll learn from it… next time." (sad) |
| **Engineer** — Workshop | "Bring me credits and crystals and I'll make her sing." (happy) · "Plating, weapons, thrusters — pick one." (normal) · Reaction — success: "Haul's in. Let's upgrade." (excited) / failed: "Nothing came back with you. Not even scrap." (sleepy) |

Before the first mission, the reaction line is skipped.

### Tutorial hints

Shown once each, then recorded in the save.

| Trigger | Hint |
|---|---|
| First Carrier load | "A / D to walk. E to talk or use." |
| First time near the Dock | "Pick a battleship here to start a mission." |
| First battle start | "Click to move. Your guns fire on their own." |
| First enemy in range | "Q / W use skills. Watch the cooldowns." |
| First loot drop | "Fly over drops to collect them." |
| First return with loot | "Spend credits and Void Crystal at the Workshop." |

### Save file

Saved on every return to the Carrier and every Workshop purchase. Holds:
credits, Void Crystal, upgrade levels, last mission result (none / success /
failed), missions played and won, tutorial hints seen, last battleship picked.
Mission loot is never saved mid-battle.

## Design-Done Checklist

Development on the first playable can start when all of these hold:

1. **Scope confirmed** — Wei has accepted, edited, or replaced § First Playable
   Scope above, and design/PROGRESS.md logs it. _Done 2026-10-03._
2. **Every blocking question is decided** — each design/OPEN_QUESTIONS.md
   entry tagged `Blocks first playable` or `Blocks (partial)` is reflected in the
   relevant design doc and removed from design/OPEN_QUESTIONS.md.
3. **Between-missions controls are written down** — how the player moves
   around the carrier, uses its modules, and launches a mission from it
   (design/DESIGN.md § Player, § Crew / Companions).
4. **One combat round is specified end to end** — what the player presses,
   what the battleship does, what enemies do, and how a mission is won or lost.
5. **One progression round is specified end to end** — which resources come
   back from a mission and what they buy at the Workshop. Concrete names and
   rough numbers, not categories (§ Demo Spec).
6. **Docs are consistent** — no doc contradicts a decision logged in
   design/PROGRESS.md (e.g. design/CONCEPT.md's pitch and reference games still
   describe "the ship" as both base and combat vessel, and cite FTL for combat;
   both predate the carrier/battleship split and the LoL-style control scheme).
7. **Deferred questions are explicitly deferred** — each design/OPEN_QUESTIONS.md
   entry tagged `Deferred` is acknowledged as out of the first playable, so it
   can't silently block implementation.

Not required before development: final numbers/balance, art, audio, story
beyond the opening premise, multiplayer or hub design.

_Open questions — tagged as blocking or deferred against this scope, with their
board issues: design/OPEN_QUESTIONS.md._
